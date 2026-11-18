// HDR automatique par application : liste d'exécutables (un par ligne) et détection des processus.
use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;
use std::{env, fs};

#[repr(C)]
struct ProcessEntry {
    size: u32,
    usage: u32,
    pid: u32,
    heap: usize,
    module: u32,
    threads: u32,
    parent: u32,
    pri_class: i32,
    flags: u32,
    exe: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
    fn Process32FirstW(snap: *mut c_void, entry: *mut ProcessEntry) -> i32;
    fn Process32NextW(snap: *mut c_void, entry: *mut ProcessEntry) -> i32;
    fn CloseHandle(h: *mut c_void) -> i32;
}

const TH32CS_SNAPPROCESS: u32 = 2;

const HEADER: &str = "# HDR Tray - applications qui activent le HDR automatiquement.\n\
# Un nom d'executable par ligne (ex. game.exe). Les lignes commençant par # sont ignorees.\n\
# Le HDR repasse a son etat initial quand plus aucune de ces applications ne tourne.\n";

pub fn list_path() -> PathBuf {
    let base = env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(env::temp_dir);
    base.join("HDRTray").join("apps.txt")
}

/// Crée le fichier avec son en-tête explicatif s'il n'existe pas.
pub fn ensure_list() -> PathBuf {
    let path = list_path();
    if !path.exists() {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&path, HEADER);
    }
    path
}

pub fn parse_list(text: &str) -> Vec<String> {
    text.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty() && !l.starts_with('#')).collect()
}

pub fn load_list() -> Vec<String> {
    fs::read_to_string(list_path()).map(|t| parse_list(&t)).unwrap_or_default()
}

/// Noms (en minuscules) des exécutables en cours d'exécution.
fn running() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap.is_null() || snap as isize == -1 {
            return out;
        }
        let mut e: ProcessEntry = std::mem::zeroed();
        e.size = size_of::<ProcessEntry>() as u32;
        let mut ok = Process32FirstW(snap, &mut e) != 0;
        while ok {
            let len = e.exe.iter().position(|&c| c == 0).unwrap_or(e.exe.len());
            out.push(String::from_utf16_lossy(&e.exe[..len]).to_lowercase());
            ok = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
    }
    out
}

/// Vrai si une des applications de la liste tourne.
pub fn any_running(list: &[String]) -> bool {
    !list.is_empty() && {
        let procs = running();
        list.iter().any(|a| procs.contains(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list() {
        let l = parse_list("# commentaire\n\n  Game.EXE \nother.exe\n#x.exe\n");
        assert_eq!(l, vec!["game.exe", "other.exe"]);
    }

    #[test]
    fn process_entry_size() {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(size_of::<ProcessEntry>(), 568);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(size_of::<ProcessEntry>(), 556);
    }
}
