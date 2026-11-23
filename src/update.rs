// Mise à jour automatique depuis les releases GitHub.
// La release contient version.txt et un installeur par architecture ; il est lancé en silencieux (/S).
use crate::i18n::tr;
use crate::settings::{self, UpdateMode};
use std::ffi::c_void;
use std::path::Path;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs, process::Command, thread};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = "spartan-dou/hdr-tray";
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(60);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 3600);

#[link(name = "urlmon")]
extern "system" {
    fn URLDownloadToFileW(caller: *mut c_void, url: *const u16, file: *const u16, reserved: u32, callback: *mut c_void) -> i32;
}

#[link(name = "user32")]
extern "system" {
    fn PostMessageW(hwnd: *mut c_void, msg: u32, wp: usize, lp: isize) -> i32;
    fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
}

const MB_ICONINFORMATION: u32 = 0x40;
const MB_ICONWARNING: u32 = 0x30;
const MB_YESNO: u32 = 0x4;
const IDYES: i32 = 6;

/// Message posté à la fenêtre principale quand une nouvelle version est détectée (mode « prévenir »).
pub const WM_UPDATE_AVAILABLE: u32 = 0x8000 + 2;
static MAIN_HWND: AtomicUsize = AtomicUsize::new(0);
/// Dernière version trouvée mais pas encore installée.
static PENDING: Mutex<Option<String>> = Mutex::new(None);
static NOTIFIED: Mutex<Option<String>> = Mutex::new(None);

pub fn pending() -> Option<String> {
    PENDING.lock().ok().and_then(|p| p.clone())
}

/// Vrai si cette copie peut être mise à jour par l'installeur (et pas une copie portable).
pub fn can_install() -> bool {
    is_installed_copy()
}

pub enum Check {
    UpToDate,
    Updating(String),
    /// Nouvelle version disponible, mais cette copie est portable : on ne l'installe pas toute seule.
    Available(String),
    Failed,
}

/// Une seule vérification à la fois (thread d'arrière-plan et clic manuel).
static BUSY: AtomicBool = AtomicBool::new(false);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn nonce() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

fn download(url: &str, path: &Path) -> bool {
    // Le paramètre unique évite de récupérer une version en cache
    let url = wide(&format!("{url}?t={}", nonce()));
    let path = wide(&path.to_string_lossy());
    unsafe { URLDownloadToFileW(null_mut(), url.as_ptr(), path.as_ptr(), 0, null_mut()) == 0 }
}

fn parse(v: &str) -> [u32; 3] {
    // "1.2" == "1.2.0" ; un suffixe ("-beta") est ignoré
    let core = v.trim().trim_start_matches('v').split(['-', '+']).next().unwrap_or("");
    let mut out = [0; 3];
    for (o, p) in out.iter_mut().zip(core.split('.')) {
        *o = p.parse().unwrap_or(0);
    }
    out
}

/// SHA-256 (FIPS 180-4), pour vérifier l'installeur téléchargé sans dépendance externe.
fn sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, b) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// Cherche le hash de `name` dans un fichier au format `sha256sum` ("<hash>  <nom>").
fn expected_hash(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let (hash, file) = l.split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name).then(|| hash.to_lowercase())
    })
}

/// Vrai si l'exe courant est celui posé par l'installeur (et non une copie portable).
fn is_installed_copy() -> bool {
    let (Ok(exe), Some(local)) = (env::current_exe(), env::var_os("LOCALAPPDATA")) else { return false };
    let dir = Path::new(&local).join("Programs").join("HDRTray");
    exe.parent().map(|p| p.to_string_lossy().eq_ignore_ascii_case(&dir.to_string_lossy())).unwrap_or(false)
}

fn asset_url(name: &str) -> String {
    format!("https://github.com/{REPO}/releases/latest/download/{name}")
}

/// Cherche une nouvelle version. `auto_install` : l'installer tout de suite si possible.
pub fn check(auto_install: bool) -> Check {
    if BUSY.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return Check::Failed; // une vérification est déjà en cours
    }
    let result = check_inner(auto_install);
    BUSY.store(false, Ordering::Release);
    result
}

/// Installe la version déjà trouvée par `check` (action du menu en mode « prévenir »).
pub fn install_pending() -> Check {
    let Some(latest) = pending() else { return Check::UpToDate };
    if BUSY.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return Check::Failed;
    }
    let result = if is_installed_copy() { install(&latest) } else { Check::Available(latest) };
    BUSY.store(false, Ordering::Release);
    result
}

fn check_inner(auto_install: bool) -> Check {
    let dir = env::temp_dir();
    let tag = format!("{}-{}", std::process::id(), nonce());
    let version_file = dir.join(format!("hdrtray-version-{tag}.txt"));
    if !download(&asset_url("version.txt"), &version_file) {
        return Check::Failed;
    }
    let latest = fs::read_to_string(&version_file).unwrap_or_default().trim().to_string();
    let _ = fs::remove_file(&version_file);
    if latest.is_empty() {
        return Check::Failed;
    }
    if parse(&latest) <= parse(VERSION) {
        if let Ok(mut p) = PENDING.lock() {
            *p = None;
        }
        return Check::UpToDate;
    }
    if let Ok(mut p) = PENDING.lock() {
        *p = Some(latest.clone());
    }
    // Pas d'installation automatique en mode « prévenir » ni sur une copie portable
    if !auto_install || !is_installed_copy() {
        return Check::Available(latest);
    }
    install(&latest)
}

fn install(latest: &str) -> Check {
    let dir = env::temp_dir();
    let tag = format!("{}-{}", std::process::id(), nonce());
    let setup_name = if cfg!(target_arch = "aarch64") { "HDRTray-Setup-arm64.exe" } else { "HDRTray-Setup.exe" };
    let sums_file = dir.join(format!("hdrtray-sums-{tag}.txt"));
    if !download(&asset_url("SHA256SUMS"), &sums_file) {
        return Check::Failed;
    }
    let sums = fs::read_to_string(&sums_file).unwrap_or_default();
    let _ = fs::remove_file(&sums_file);
    let Some(expected) = expected_hash(&sums, setup_name) else { return Check::Failed };
    let setup = dir.join(format!("HDRTray-Setup-{latest}-{tag}.exe"));
    if !download(&asset_url(setup_name), &setup) {
        return Check::Failed;
    }
    // L'installeur n'est lancé que si son empreinte correspond à celle publiée
    if fs::read(&setup).map(|b| sha256(&b) != expected).unwrap_or(true) {
        let _ = fs::remove_file(&setup);
        return Check::Failed;
    }
    // L'installeur ferme cette instance, remplace l'exe puis relance la nouvelle version
    match Command::new(&setup).arg("/S").spawn() {
        Ok(_) => Check::Updating(latest.to_string()),
        Err(_) => Check::Failed,
    }
}

pub fn start_background(hwnd: *mut c_void) {
    MAIN_HWND.store(hwnd as usize, Ordering::Relaxed);
    thread::spawn(|| {
        thread::sleep(FIRST_CHECK_DELAY);
        loop {
            let mode = settings::update_mode();
            if mode != UpdateMode::Off {
                if let Check::Available(v) = check(mode == UpdateMode::Auto) {
                    // Ne prévient qu'une fois par version
                    let fresh = NOTIFIED.lock().map(|mut n| n.replace(v.clone()).as_deref() != Some(v.as_str())).unwrap_or(false);
                    if fresh {
                        unsafe { PostMessageW(MAIN_HWND.load(Ordering::Relaxed) as *mut c_void, WM_UPDATE_AVAILABLE, 0, 0) };
                    }
                }
            }
            thread::sleep(CHECK_INTERVAL);
        }
    });
}

fn message(text: &str, kind: u32) -> i32 {
    unsafe { MessageBoxW(null_mut(), wide(text).as_ptr(), wide("HDR Tray").as_ptr(), kind) }
}

fn updating_message(v: &str) {
    message(&format!("{} {v}…", tr("Installation de la version", "Installing version")), MB_ICONINFORMATION);
}

/// Installe en arrière-plan la version déjà détectée (menu en mode « prévenir »).
pub fn install_now() {
    thread::spawn(|| match install_pending() {
        Check::Updating(v) => updating_message(&v),
        _ => {
            message(tr("Échec de l'installation de la mise à jour.", "The update could not be installed."), MB_ICONWARNING);
        }
    });
}

pub fn check_manually() {
    thread::spawn(|| {
        match check(settings::update_mode() == UpdateMode::Auto) {
            Check::UpToDate => {
                let t = format!("{} {VERSION}).", tr("HDR Tray est à jour (version", "HDR Tray is up to date (version"));
                message(&t, MB_ICONINFORMATION);
            }
            Check::Updating(v) => updating_message(&v),
            Check::Available(v) if can_install() => {
                let t = format!("{} {v} {}", tr("La version", "Version"), tr("est disponible.\nL'installer maintenant ?", "is available.\nInstall it now?"));
                if message(&t, MB_YESNO | MB_ICONINFORMATION) == IDYES {
                    match install_pending() {
                        Check::Updating(v) => updating_message(&v),
                        _ => {
                            message(tr("Échec de l'installation de la mise à jour.", "The update could not be installed."), MB_ICONWARNING);
                        }
                    }
                }
            }
            Check::Available(v) => {
                let t = format!(
                    "{} {v} {}\nhttps://github.com/{REPO}/releases/latest",
                    tr("La version", "Version"),
                    tr(
                        "est disponible.\nCette copie ne se met pas à jour toute seule (portable ou MSI) : téléchargez-la sur",
                        "is available.\nThis copy does not update itself (portable or MSI): download it from"
                    )
                );
                message(&t, MB_ICONINFORMATION);
            }
            Check::Failed => {
                message(
                    tr(
                        "Impossible de vérifier les mises à jour.\nVérifiez votre connexion Internet.",
                        "Unable to check for updates.\nCheck your Internet connection.",
                    ),
                    MB_ICONWARNING,
                );
            }
        };
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_versions() {
        assert_eq!(parse("1.2"), parse("1.2.0"));
        assert_eq!(parse("v1.2.3\n"), [1, 2, 3]);
        assert_eq!(parse("1.2.0-beta"), [1, 2, 0]);
        assert!(parse("1.10.0") > parse("1.9.9"));
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(sha256(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn find_hash() {
        let sums = "AB12  HDRTray-Setup.exe\ncd34 *HDRTray-Setup-arm64.exe\n";
        assert_eq!(expected_hash(sums, "HDRTray-Setup.exe").as_deref(), Some("ab12"));
        assert_eq!(expected_hash(sums, "HDRTray-Setup-arm64.exe").as_deref(), Some("cd34"));
        assert_eq!(expected_hash(sums, "x.exe"), None);
    }
}
