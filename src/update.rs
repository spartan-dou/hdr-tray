// Mise à jour automatique depuis les releases GitHub.
// La release contient version.txt et un installeur par architecture ; il est lancé en silencieux (/S).
use std::ffi::c_void;
use std::path::Path;
use std::ptr::null_mut;
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
    fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
}

const MB_ICONINFORMATION: u32 = 0x40;
const MB_ICONWARNING: u32 = 0x30;

pub enum Check {
    UpToDate,
    Updating(String),
    Failed,
}

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

fn parse(v: &str) -> Vec<u32> {
    v.trim().trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

fn asset_url(name: &str) -> String {
    format!("https://github.com/{REPO}/releases/latest/download/{name}")
}

pub fn check_and_install() -> Check {
    let dir = env::temp_dir();
    let version_file = dir.join("hdrtray-version.txt");
    if !download(&asset_url("version.txt"), &version_file) {
        return Check::Failed;
    }
    let latest = fs::read_to_string(&version_file).unwrap_or_default().trim().to_string();
    let _ = fs::remove_file(&version_file);
    if latest.is_empty() {
        return Check::Failed;
    }
    if parse(&latest) <= parse(VERSION) {
        return Check::UpToDate;
    }
    let setup = dir.join(format!("HDRTray-Setup-{latest}.exe"));
    let setup_name = if cfg!(target_arch = "aarch64") { "HDRTray-Setup-arm64.exe" } else { "HDRTray-Setup.exe" };
    if !download(&asset_url(setup_name), &setup) {
        return Check::Failed;
    }
    // L'installeur ferme cette instance, remplace l'exe puis relance la nouvelle version
    match Command::new(&setup).arg("/S").spawn() {
        Ok(_) => Check::Updating(latest),
        Err(_) => Check::Failed,
    }
}

pub fn start_background() {
    thread::spawn(|| {
        thread::sleep(FIRST_CHECK_DELAY);
        loop {
            check_and_install();
            thread::sleep(CHECK_INTERVAL);
        }
    });
}

pub fn check_manually() {
    thread::spawn(|| {
        let (text, icon) = match check_and_install() {
            Check::UpToDate => (format!("HDR Tray est à jour (version {VERSION})."), MB_ICONINFORMATION),
            Check::Updating(v) => (format!("Installation de la version {v}…"), MB_ICONINFORMATION),
            Check::Failed => ("Impossible de vérifier les mises à jour.\nVérifiez votre connexion Internet.".to_string(), MB_ICONWARNING),
        };
        unsafe { MessageBoxW(null_mut(), wide(&text).as_ptr(), wide("HDR Tray").as_ptr(), icon) };
    });
}
