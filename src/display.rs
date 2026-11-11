// Lecture et bascule du HDR via l'API DisplayConfig de Windows.
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::null_mut;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Luid {
    low: u32,
    high: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PathSource {
    adapter: Luid,
    id: u32,
    mode_idx: u32,
    status: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PathTarget {
    adapter: Luid,
    id: u32,
    mode_idx: u32,
    output_tech: u32,
    rotation: u32,
    scaling: u32,
    refresh_num: u32,
    refresh_den: u32,
    scanline: u32,
    available: i32,
    status: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PathInfo {
    source: PathSource,
    target: PathTarget,
    flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ModeInfo {
    _data: [u64; 8],
}

#[repr(C)]
struct Header {
    kind: u32,
    size: u32,
    adapter: Luid,
    id: u32,
}

#[repr(C)]
struct ColorInfo {
    h: Header,
    value: u32,
    encoding: u32,
    bpc: u32,
}

#[repr(C)]
struct ColorInfo2 {
    h: Header,
    value: u32,
    encoding: u32,
    bpc: u32,
    mode: u32,
}

#[repr(C)]
struct SetState {
    h: Header,
    value: u32,
}

#[link(name = "user32")]
extern "system" {
    fn GetDisplayConfigBufferSizes(flags: u32, n_path: *mut u32, n_mode: *mut u32) -> i32;
    fn QueryDisplayConfig(flags: u32, n_path: *mut u32, paths: *mut PathInfo, n_mode: *mut u32, modes: *mut ModeInfo, topology: *mut c_void) -> i32;
    fn DisplayConfigGetDeviceInfo(packet: *mut Header) -> i32;
    fn DisplayConfigSetDeviceInfo(packet: *mut Header) -> i32;
}

const QDC_ONLY_ACTIVE_PATHS: u32 = 2;
const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
const GET_ADVANCED_COLOR_INFO: u32 = 9;
const SET_ADVANCED_COLOR_STATE: u32 = 10;
const GET_ADVANCED_COLOR_INFO_2: u32 = 15;
const SET_HDR_STATE: u32 = 16;
const COLOR_MODE_HDR: u32 = 2;

struct Target {
    adapter: Luid,
    id: u32,
    enabled: bool,
    new_api: bool,
}

fn header<T>(kind: u32, adapter: Luid, id: u32) -> Header {
    Header { kind, size: size_of::<T>() as u32, adapter, id }
}

unsafe fn probe(adapter: Luid, id: u32) -> Option<Target> {
    // Windows 11 24H2+ : distingue le vrai HDR de la gestion auto des couleurs
    let mut c2 = ColorInfo2 { h: header::<ColorInfo2>(GET_ADVANCED_COLOR_INFO_2, adapter, id), value: 0, encoding: 0, bpc: 0, mode: 0 };
    if DisplayConfigGetDeviceInfo(&mut c2 as *mut ColorInfo2 as *mut Header) == 0 {
        let supported = c2.value & (1 << 4) != 0;
        return supported.then(|| Target { adapter, id, enabled: c2.mode == COLOR_MODE_HDR, new_api: true });
    }
    let mut c = ColorInfo { h: header::<ColorInfo>(GET_ADVANCED_COLOR_INFO, adapter, id), value: 0, encoding: 0, bpc: 0 };
    if DisplayConfigGetDeviceInfo(&mut c as *mut ColorInfo as *mut Header) != 0 {
        return None;
    }
    (c.value & 1 != 0).then(|| Target { adapter, id, enabled: c.value & 2 != 0, new_api: false })
}

fn targets() -> Vec<Target> {
    unsafe {
        for _ in 0..3 {
            let (mut np, mut nm) = (0u32, 0u32);
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != 0 {
                return Vec::new();
            }
            let mut paths = vec![PathInfo::default(); np as usize];
            let mut modes = vec![ModeInfo { _data: [0; 8] }; nm as usize];
            let rc = QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), null_mut());
            if rc == ERROR_INSUFFICIENT_BUFFER {
                continue;
            }
            if rc != 0 {
                return Vec::new();
            }
            paths.truncate(np as usize);
            return paths.iter().filter_map(|p| probe(p.target.adapter, p.target.id)).collect();
        }
        Vec::new()
    }
}

/// 1 = HDR actif, 0 = inactif, -1 = aucun écran compatible
pub fn state() -> i32 {
    let t = targets();
    if t.is_empty() {
        -1
    } else if t.iter().any(|t| t.enabled) {
        1
    } else {
        0
    }
}

pub fn toggle() {
    let t = targets();
    let enable = !t.iter().any(|t| t.enabled);
    for d in &t {
        let kind = if d.new_api { SET_HDR_STATE } else { SET_ADVANCED_COLOR_STATE };
        let mut s = SetState { h: header::<SetState>(kind, d.adapter, d.id), value: enable as u32 };
        unsafe { DisplayConfigSetDeviceInfo(&mut s as *mut SetState as *mut Header) };
    }
}
