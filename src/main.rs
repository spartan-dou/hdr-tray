// HDR Tray : icône de zone de notification qui suit l'état du HDR.
// Clic gauche : bascule. Clic droit : menu. `HDRTray.exe --quit` ferme l'instance en cours.
#![windows_subsystem = "windows"]

mod apps;
mod display;
mod i18n;
mod icon;
mod settings;
mod update;

use i18n::tr;
use settings::UpdateMode;

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, Ordering::Relaxed};

type Handle = *mut c_void;
type WndProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: Point,
    private: u32,
}

#[repr(C)]
struct WndClass {
    style: u32,
    wndproc: WndProc,
    cls_extra: i32,
    wnd_extra: i32,
    instance: Handle,
    icon: Handle,
    cursor: Handle,
    background: Handle,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}

#[repr(C)]
struct NotifyIconData {
    size: u32,
    hwnd: Handle,
    id: u32,
    flags: u32,
    callback: u32,
    icon: Handle,
    tip: [u16; 128],
    state: u32,
    state_mask: u32,
    info: [u16; 256],
    version: u32,
    info_title: [u16; 64],
    info_flags: u32,
    guid: Guid,
    balloon_icon: Handle,
}

#[repr(C)]
struct IconInfo {
    is_icon: i32,
    x_hotspot: u32,
    y_hotspot: u32,
    mask: Handle,
    color: Handle,
}

#[repr(C)]
struct BitmapInfo {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_ppm: i32,
    y_ppm: i32,
    clr_used: u32,
    clr_important: u32,
    colors: [u32; 1],
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassW(wc: *const WndClass) -> u16;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: Handle,
        menu: Handle,
        inst: Handle,
        param: *const c_void,
    ) -> Handle;
    fn DefWindowProcW(hwnd: Handle, msg: u32, wp: usize, lp: isize) -> isize;
    fn GetMessageW(msg: *mut Msg, hwnd: Handle, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> isize;
    fn PostQuitMessage(code: i32);
    fn DestroyWindow(hwnd: Handle) -> i32;
    fn PostMessageW(hwnd: Handle, msg: u32, wp: usize, lp: isize) -> i32;
    fn FindWindowW(class: *const u16, name: *const u16) -> Handle;
    fn SetTimer(hwnd: Handle, id: usize, ms: u32, func: *const c_void) -> usize;
    fn RegisterWindowMessageW(name: *const u16) -> u32;
    fn CreatePopupMenu() -> Handle;
    fn AppendMenuW(menu: Handle, flags: u32, id: usize, text: *const u16) -> i32;
    fn TrackPopupMenu(menu: Handle, flags: u32, x: i32, y: i32, reserved: i32, hwnd: Handle, rect: *const c_void) -> i32;
    fn DestroyMenu(menu: Handle) -> i32;
    fn GetCursorPos(pt: *mut Point) -> i32;
    fn SetForegroundWindow(hwnd: Handle) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn CreateIconIndirect(info: *const IconInfo) -> Handle;
    fn DestroyIcon(icon: Handle) -> i32;
    fn SetProcessDPIAware() -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(hwnd: Handle, op: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> Handle;
    fn Shell_NotifyIconW(msg: u32, data: *mut NotifyIconData) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateDIBSection(hdc: Handle, info: *const BitmapInfo, usage: u32, bits: *mut *mut c_void, section: Handle, offset: u32) -> Handle;
    fn CreateBitmap(w: i32, h: i32, planes: u32, bpp: u32, bits: *const c_void) -> Handle;
    fn DeleteObject(obj: Handle) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(key: Handle, sub: *const u16, options: u32, sam: u32, out: *mut Handle) -> i32;
    fn RegQueryValueExW(key: Handle, name: *const u16, reserved: *mut u32, kind: *mut u32, data: *mut u8, len: *mut u32) -> i32;
    fn RegSetValueExW(key: Handle, name: *const u16, reserved: u32, kind: u32, data: *const u8, len: u32) -> i32;
    fn RegDeleteValueW(key: Handle, name: *const u16) -> i32;
    fn RegCloseKey(key: Handle) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
    fn GetModuleFileNameW(module: Handle, buf: *mut u16, len: u32) -> u32;
    fn CreateMutexW(attrs: *const c_void, owner: i32, name: *const u16) -> Handle;
    fn GetLastError() -> u32;
    fn Sleep(ms: u32);
}

const WM_CLOSE: u32 = 0x0010;
const WM_DESTROY: u32 = 0x0002;
const WM_DISPLAYCHANGE: u32 = 0x007E;
const WM_TIMER: u32 = 0x0113;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_NULL: u32 = 0;
const WM_TRAY: u32 = 0x8000 + 1;
const NIM_ADD: u32 = 0;
const NIM_MODIFY: u32 = 1;
const NIM_DELETE: u32 = 2;
const NIF_MESSAGE: u32 = 1;
const NIF_ICON: u32 = 2;
const NIF_TIP: u32 = 4;
const NIF_INFO: u32 = 0x10;
const NIIF_INFO: u32 = 1;
const MF_POPUP: u32 = 0x10;
const MF_STRING: u32 = 0;
const MF_GRAYED: u32 = 1;
const MF_CHECKED: u32 = 8;
const MF_SEPARATOR: u32 = 0x800;
const TPM_RIGHTBUTTON: u32 = 0x2;
const TPM_BOTTOMALIGN: u32 = 0x20;
const TPM_NONOTIFY: u32 = 0x80;
const TPM_RETURNCMD: u32 = 0x100;
const SM_CXSMICON: i32 = 49;
const KEY_QUERY_VALUE: u32 = 0x1;
const KEY_SET_VALUE: u32 = 0x2;
const REG_SZ: u32 = 1;
const ERROR_ALREADY_EXISTS: u32 = 183;
const CHECK_INTERVAL_MS: u32 = 2000;
const SCREEN_CMD: usize = 100;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_NAME: &str = "HDRTray";
const CLASS_NAME: &str = "HDRTrayWindow";

static HWND: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static ICON: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static STATE: AtomicI32 = AtomicI32::new(-2);
static ADDED: AtomicBool = AtomicBool::new(false);
static LIGHT: AtomicBool = AtomicBool::new(false);
static UPDATED_NOTICE: AtomicBool = AtomicBool::new(false);
static AUTO_ACTIVE: AtomicBool = AtomicBool::new(false);
static AUTO_RESTORE_OFF: AtomicBool = AtomicBool::new(false);
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn hkcu() -> Handle {
    // HKEY_CURRENT_USER, étendu en signe comme dans les en-têtes Windows
    0x8000_0001u32 as i32 as isize as Handle
}

unsafe fn make_icon(state: i32) -> Handle {
    let size = GetSystemMetrics(SM_CXSMICON).max(16);
    let pixels = icon::render_themed(size as u32, state, LIGHT.load(Relaxed));
    let info = BitmapInfo {
        size: 40,
        width: size,
        height: -size,
        planes: 1,
        bit_count: 32,
        compression: 0,
        size_image: 0,
        x_ppm: 0,
        y_ppm: 0,
        clr_used: 0,
        clr_important: 0,
        colors: [0],
    };
    let mut bits = null_mut();
    let color = CreateDIBSection(null_mut(), &info, 0, &mut bits, null_mut(), 0);
    if color.is_null() || bits.is_null() {
        return null_mut();
    }
    let dst = std::slice::from_raw_parts_mut(bits as *mut u8, pixels.len() * 4);
    for (d, [r, g, b, a]) in dst.as_chunks_mut::<4>().0.iter_mut().zip(pixels) {
        d.copy_from_slice(&[b, g, r, a]);
    }
    let mask_bits = vec![0u8; ((size as usize).div_ceil(16) * 2) * size as usize];
    let mask = CreateBitmap(size, size, 1, 1, mask_bits.as_ptr() as *const c_void);
    let icon = CreateIconIndirect(&IconInfo { is_icon: 1, x_hotspot: 0, y_hotspot: 0, mask, color });
    DeleteObject(mask);
    DeleteObject(color);
    icon
}

unsafe fn update_tray(add: bool) {
    let state = STATE.load(Relaxed);
    let icon = make_icon(state);
    let mut nid: NotifyIconData = zeroed();
    nid.size = size_of::<NotifyIconData>() as u32;
    nid.hwnd = HWND.load(Relaxed);
    nid.id = 1;
    nid.flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.callback = WM_TRAY;
    nid.icon = icon;
    let tip = match state {
        1 => tr("HDR activé — clic pour désactiver", "HDR on — click to turn off"),
        0 => tr("HDR désactivé — clic pour activer", "HDR off — click to turn on"),
        _ => tr("Aucun écran HDR détecté", "No HDR display detected"),
    };
    for (d, c) in nid.tip.iter_mut().zip(tip.encode_utf16().take(127)) {
        *d = c;
    }
    // Si l'Explorateur n'est pas encore prêt (ouverture de session), on réessaie au prochain tick
    ADDED.store(Shell_NotifyIconW(if add { NIM_ADD } else { NIM_MODIFY }, &mut nid) != 0, Relaxed);
    let old = ICON.swap(icon, Relaxed);
    if !old.is_null() {
        DestroyIcon(old);
    }
}

unsafe fn balloon(title: &str, text: &str) {
    if !ADDED.load(Relaxed) {
        return;
    }
    let mut nid: NotifyIconData = zeroed();
    nid.size = size_of::<NotifyIconData>() as u32;
    nid.hwnd = HWND.load(Relaxed);
    nid.id = 1;
    nid.flags = NIF_INFO;
    nid.info_flags = NIIF_INFO;
    for (d, c) in nid.info.iter_mut().zip(text.encode_utf16().take(255)) {
        *d = c;
    }
    for (d, c) in nid.info_title.iter_mut().zip(title.encode_utf16().take(63)) {
        *d = c;
    }
    Shell_NotifyIconW(NIM_MODIFY, &mut nid);
}

unsafe fn refresh(force: bool) {
    let state = display::state();
    let light = settings::taskbar_light();
    // L'icône est redessinée aussi quand le thème de la barre des tâches change
    let changed = STATE.swap(state, Relaxed) != state || LIGHT.swap(light, Relaxed) != light;
    if !ADDED.load(Relaxed) {
        update_tray(true);
    } else if changed || force {
        update_tray(false);
    }
    if ADDED.load(Relaxed) && UPDATED_NOTICE.swap(false, Relaxed) {
        balloon("HDR Tray", &format!("{} {}", tr("Mis à jour vers la version", "Updated to version"), update::VERSION));
    }
}

/// HDR automatique : activé tant qu'une application de la liste tourne, puis remis dans son état initial.
unsafe fn auto_apps() {
    let enabled = settings::get_dword("AutoApps") == Some(1);
    let running = enabled && apps::any_running(&apps::load_list());
    let active = AUTO_ACTIVE.load(Relaxed);
    if running && !active {
        AUTO_ACTIVE.store(true, Relaxed);
        let was_on = display::state() == 1;
        AUTO_RESTORE_OFF.store(!was_on, Relaxed);
        if !was_on {
            display::set_all(true);
            Sleep(250);
            refresh(false);
        }
    } else if !running && active {
        AUTO_ACTIVE.store(false, Relaxed);
        if AUTO_RESTORE_OFF.swap(false, Relaxed) && display::state() == 1 {
            display::set_all(false);
            Sleep(250);
            refresh(false);
        }
    }
}

unsafe fn toggle() {
    display::toggle();
    Sleep(250);
    refresh(false);
}

unsafe fn startup_enabled() -> bool {
    let mut key = null_mut();
    if RegOpenKeyExW(hkcu(), wide(RUN_KEY).as_ptr(), 0, KEY_QUERY_VALUE, &mut key) != 0 {
        return false;
    }
    let found = RegQueryValueExW(key, wide(RUN_NAME).as_ptr(), null_mut(), null_mut(), null_mut(), null_mut()) == 0;
    RegCloseKey(key);
    found
}

unsafe fn set_startup(on: bool) {
    let mut key = null_mut();
    if RegOpenKeyExW(hkcu(), wide(RUN_KEY).as_ptr(), 0, KEY_SET_VALUE, &mut key) != 0 {
        return;
    }
    let name = wide(RUN_NAME);
    if on {
        let mut buf = [0u16; 1024];
        let len = GetModuleFileNameW(null_mut(), buf.as_mut_ptr(), buf.len() as u32) as usize;
        let value = wide(&format!("\"{}\"", String::from_utf16_lossy(&buf[..len])));
        RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, value.as_ptr() as *const u8, (value.len() * 2) as u32);
    } else {
        RegDeleteValueW(key, name.as_ptr());
    }
    RegCloseKey(key);
}

unsafe fn show_menu(hwnd: Handle) {
    let menu = CreatePopupMenu();
    let state = STATE.load(Relaxed);
    let toggle_label = if state == 1 { tr("Désactiver le HDR", "Turn HDR off") } else { tr("Activer le HDR", "Turn HDR on") };
    AppendMenuW(menu, if state == -1 { MF_GRAYED } else { MF_STRING }, 1, wide(toggle_label).as_ptr());

    // Un sous-menu par écran quand il y en a plusieurs
    let screens = display::screens();
    if screens.len() > 1 {
        let sub = CreatePopupMenu();
        for (i, sc) in screens.iter().enumerate() {
            let flags = MF_STRING | if sc.enabled { MF_CHECKED } else { 0 };
            AppendMenuW(sub, flags, SCREEN_CMD + i, wide(&sc.name).as_ptr());
        }
        AppendMenuW(menu, MF_POPUP, sub as usize, wide(tr("Écrans", "Displays")).as_ptr());
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, null());

    let startup_flags = MF_STRING | if startup_enabled() { MF_CHECKED } else { 0 };
    AppendMenuW(menu, startup_flags, 2, wide(tr("Lancer au démarrage de Windows", "Start with Windows")).as_ptr());

    // HDR automatique par application
    let auto = apps_sub();
    AppendMenuW(menu, MF_POPUP, auto as usize, wide(tr("HDR automatique par application", "Automatic HDR per app")).as_ptr());
    AppendMenuW(menu, MF_STRING, 6, wide(tr("Paramètres HDR de Windows…", "Windows HDR settings…")).as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, null());

    // Mises à jour
    let mode = settings::update_mode();
    let sub = CreatePopupMenu();
    for (cmd, m, label) in [
        (10, UpdateMode::Auto, tr("Installer automatiquement", "Install automatically")),
        (11, UpdateMode::Notify, tr("Prévenir seulement", "Notify only")),
        (12, UpdateMode::Off, tr("Désactivées", "Disabled")),
    ] {
        AppendMenuW(sub, MF_STRING | if mode == m { MF_CHECKED } else { 0 }, cmd, wide(label).as_ptr());
    }
    AppendMenuW(menu, MF_POPUP, sub as usize, wide(tr("Mises à jour", "Updates")).as_ptr());
    if let Some(v) = update::pending() {
        if update::can_install() {
            AppendMenuW(menu, MF_STRING, 5, wide(&format!("{} {v}", tr("Installer la version", "Install version"))).as_ptr());
        }
    }
    AppendMenuW(menu, MF_STRING, 4, wide(tr("Rechercher une mise à jour", "Check for updates")).as_ptr());
    AppendMenuW(menu, MF_GRAYED, 0, wide(&format!("Version {}", update::VERSION)).as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, 3, wide(tr("Quitter", "Quit")).as_ptr());

    let mut pt = Point { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY | TPM_BOTTOMALIGN;
    let cmd = TrackPopupMenu(menu, flags, pt.x, pt.y, 0, hwnd, null()) as usize;
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu); // détruit aussi les sous-menus rattachés
    match cmd {
        1 => toggle(),
        2 => set_startup(!startup_enabled()),
        3 => {
            DestroyWindow(hwnd);
        }
        4 => update::check_manually(),
        5 => update::install_now(),
        6 => {
            ShellExecuteW(null_mut(), wide("open").as_ptr(), wide("ms-settings:display-advancedcolor").as_ptr(), null(), null(), 1);
        }
        10 => settings::set_dword("UpdateMode", UpdateMode::Auto.to_u32()),
        11 => settings::set_dword("UpdateMode", UpdateMode::Notify.to_u32()),
        12 => settings::set_dword("UpdateMode", UpdateMode::Off.to_u32()),
        20 => settings::set_dword("AutoApps", (settings::get_dword("AutoApps") != Some(1)) as u32),
        21 => open_apps_list(),
        c if c >= SCREEN_CMD => {
            if let Some(sc) = screens.get(c - SCREEN_CMD) {
                display::toggle_screen(sc.key);
                Sleep(250);
                refresh(false);
            }
        }
        _ => {}
    }
}

unsafe fn apps_sub() -> Handle {
    let sub = CreatePopupMenu();
    let on = settings::get_dword("AutoApps") == Some(1);
    AppendMenuW(sub, MF_STRING | if on { MF_CHECKED } else { 0 }, 20, wide(tr("Activer", "Enabled")).as_ptr());
    AppendMenuW(sub, MF_STRING, 21, wide(tr("Modifier la liste des applications…", "Edit the app list…")).as_ptr());
    sub
}

unsafe fn open_apps_list() {
    let path = apps::ensure_list();
    ShellExecuteW(null_mut(), wide("open").as_ptr(), wide("notepad.exe").as_ptr(), wide(&format!("\"{}\"", path.display())).as_ptr(), null(), 1);
}

unsafe extern "system" fn wndproc(hwnd: Handle, msg: u32, wp: usize, lp: isize) -> isize {
    match msg {
        WM_TRAY => {
            match lp as u32 {
                WM_LBUTTONUP => toggle(),
                WM_RBUTTONUP => show_menu(hwnd),
                _ => {}
            }
            0
        }
        WM_TIMER | WM_DISPLAYCHANGE => {
            refresh(false);
            if msg == WM_TIMER {
                auto_apps();
            }
            0
        }
        update::WM_UPDATE_AVAILABLE => {
            if let Some(v) = update::pending() {
                let text =
                    format!("{} {v} {}", tr("La version", "Version"), tr("est disponible (clic droit sur l'icône).", "is available (right-click the icon)."));
                balloon("HDR Tray", &text);
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            let mut nid: NotifyIconData = zeroed();
            nid.size = size_of::<NotifyIconData>() as u32;
            nid.hwnd = hwnd;
            nid.id = 1;
            Shell_NotifyIconW(NIM_DELETE, &mut nid);
            PostQuitMessage(0);
            0
        }
        m if m != 0 && m == TASKBAR_CREATED.load(Relaxed) => {
            // L'Explorateur a redémarré : il faut remettre l'icône
            ADDED.store(false, Relaxed);
            refresh(true);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn main() {
    unsafe {
        let class = wide(CLASS_NAME);
        if std::env::args().any(|a| a == "--quit") {
            let hwnd = FindWindowW(class.as_ptr(), null());
            if !hwnd.is_null() {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
            return;
        }

        CreateMutexW(null(), 0, wide("Local\\HDRTray-single-instance-rs").as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }

        SetProcessDPIAware();
        let instance = GetModuleHandleW(null());
        let wc = WndClass {
            style: 0,
            wndproc,
            cls_extra: 0,
            wnd_extra: 0,
            instance,
            icon: null_mut(),
            cursor: null_mut(),
            background: null_mut(),
            menu_name: null(),
            class_name: class.as_ptr(),
        };
        RegisterClassW(&wc);
        TASKBAR_CREATED.store(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()), Relaxed);

        // Fenêtre invisible (pas « message-only », pour recevoir TaskbarCreated et WM_DISPLAYCHANGE)
        let hwnd = CreateWindowExW(0, class.as_ptr(), wide("HDR Tray").as_ptr(), 0, 0, 0, 0, 0, null_mut(), null_mut(), instance, null());
        HWND.store(hwnd, Relaxed);
        refresh(true);
        // Suit aussi les changements faits ailleurs (Win+Alt+B, Paramètres, jeux)
        SetTimer(hwnd, 1, CHECK_INTERVAL_MS, null());
        // Après une mise à jour, prévient une fois (la version stockée change)
        if let Some(prev) = settings::get_string("LastVersion") {
            UPDATED_NOTICE.store(prev != update::VERSION, Relaxed);
        }
        settings::set_string("LastVersion", update::VERSION);
        update::start_background(hwnd);

        let mut msg: Msg = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
