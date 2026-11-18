// Réglages persistants dans HKCU\Software\HDRTray.
use std::ffi::c_void;
use std::ptr::null_mut;

#[link(name = "advapi32")]
extern "system" {
    fn RegCreateKeyExW(
        key: *mut c_void,
        sub: *const u16,
        reserved: u32,
        class: *const u16,
        options: u32,
        sam: u32,
        attrs: *const c_void,
        out: *mut *mut c_void,
        disp: *mut u32,
    ) -> i32;
    fn RegOpenKeyExW(key: *mut c_void, sub: *const u16, options: u32, sam: u32, out: *mut *mut c_void) -> i32;
    fn RegQueryValueExW(key: *mut c_void, name: *const u16, reserved: *mut u32, kind: *mut u32, data: *mut u8, len: *mut u32) -> i32;
    fn RegSetValueExW(key: *mut c_void, name: *const u16, reserved: u32, kind: u32, data: *const u8, len: u32) -> i32;
    fn RegCloseKey(key: *mut c_void) -> i32;
}

const KEY: &str = "Software\\HDRTray";
const KEY_QUERY_VALUE: u32 = 0x1;
const KEY_SET_VALUE: u32 = 0x2;
const REG_SZ: u32 = 1;
const REG_DWORD: u32 = 4;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn hkcu() -> *mut c_void {
    0x8000_0001u32 as i32 as isize as *mut c_void
}

pub fn get_dword_at(path: &str, name: &str) -> Option<u32> {
    unsafe {
        let mut key = null_mut();
        if RegOpenKeyExW(hkcu(), wide(path).as_ptr(), 0, KEY_QUERY_VALUE, &mut key) != 0 {
            return None;
        }
        let (mut v, mut len, mut kind) = (0u32, 4u32, 0u32);
        let rc = RegQueryValueExW(key, wide(name).as_ptr(), null_mut(), &mut kind, &mut v as *mut u32 as *mut u8, &mut len);
        RegCloseKey(key);
        (rc == 0 && kind == REG_DWORD).then_some(v)
    }
}

pub fn get_dword(name: &str) -> Option<u32> {
    get_dword_at(KEY, name)
}

unsafe fn open_write() -> *mut c_void {
    let mut key = null_mut();
    if RegCreateKeyExW(hkcu(), wide(KEY).as_ptr(), 0, std::ptr::null(), 0, KEY_SET_VALUE, std::ptr::null(), &mut key, null_mut()) != 0 {
        return null_mut();
    }
    key
}

pub fn set_dword(name: &str, value: u32) {
    unsafe {
        let key = open_write();
        if !key.is_null() {
            RegSetValueExW(key, wide(name).as_ptr(), 0, REG_DWORD, &value as *const u32 as *const u8, 4);
            RegCloseKey(key);
        }
    }
}

pub fn get_string(name: &str) -> Option<String> {
    unsafe {
        let mut key = null_mut();
        if RegOpenKeyExW(hkcu(), wide(KEY).as_ptr(), 0, KEY_QUERY_VALUE, &mut key) != 0 {
            return None;
        }
        let mut buf = [0u16; 128];
        let (mut len, mut kind) = (std::mem::size_of_val(&buf) as u32, 0u32);
        let rc = RegQueryValueExW(key, wide(name).as_ptr(), null_mut(), &mut kind, buf.as_mut_ptr() as *mut u8, &mut len);
        RegCloseKey(key);
        if rc != 0 || kind != REG_SZ {
            return None;
        }
        let n = (len as usize / 2).min(buf.len());
        let s = String::from_utf16_lossy(&buf[..n]);
        Some(s.trim_end_matches('\0').to_string())
    }
}

pub fn set_string(name: &str, value: &str) {
    unsafe {
        let key = open_write();
        if !key.is_null() {
            let v = wide(value);
            RegSetValueExW(key, wide(name).as_ptr(), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32);
            RegCloseKey(key);
        }
    }
}

/// Mode de mise à jour : automatique (par défaut), prévenir seulement, désactivée.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UpdateMode {
    Auto,
    Notify,
    Off,
}

impl UpdateMode {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Notify,
            2 => Self::Off,
            _ => Self::Auto,
        }
    }
    pub fn to_u32(self) -> u32 {
        self as u32
    }
}

pub fn update_mode() -> UpdateMode {
    UpdateMode::from_u32(get_dword("UpdateMode").unwrap_or(0))
}

/// Vrai si la barre des tâches utilise le thème clair.
pub fn taskbar_light() -> bool {
    get_dword_at("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize", "SystemUsesLightTheme") == Some(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_mode_roundtrip() {
        for m in [UpdateMode::Auto, UpdateMode::Notify, UpdateMode::Off] {
            assert_eq!(UpdateMode::from_u32(m.to_u32()), m);
        }
        assert_eq!(UpdateMode::from_u32(99), UpdateMode::Auto);
    }
}
