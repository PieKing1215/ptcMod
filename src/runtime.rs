use std::{convert::TryInto, marker::PhantomData, sync::mpsc::Sender, time::Instant, cell::Cell};

use colorsys::ColorTransform;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use winapi::{
    shared::{minwindef::LPVOID, windef::HWND},
    um::{
        libloaderapi::GetModuleHandleA, memoryapi::VirtualProtect, winnt::PAGE_EXECUTE_READWRITE,
        winuser,
    },
};

// TODO: maybe use https://crates.io/crates/built or something to make this more detailed (git hash, etc.)
const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

use crate::{ptc::{addr, PTCVersion}, patch::Patch, feature::Feature};

// system for assigning globally unique menu ids without hardcoded constants
static mut MENU_ID_COUNTER: Cell<u16> = Cell::new(1000);

pub(crate) fn next_id() -> u16 {
    unsafe {
        MENU_ID_COUNTER.set(MENU_ID_COUNTER.get() + 1);
        MENU_ID_COUNTER.get()
    }
}

lazy_static::lazy_static! {
    static ref M_PLAYHEAD_ID: u16 = next_id();
    static ref M_ABOUT_ID: u16 = next_id();
    static ref M_UNINJECT_ID: u16 = next_id();
}

/// Handles toggling the state of a menu toggle
/// Returns true if the menu is now checked
pub(crate) fn menu_toggle(hwnd: HWND, id: impl Into<u32>) -> bool {
    let id = id.into();
    unsafe {
        if winuser::GetMenuState(
            winuser::GetMenu(hwnd),
            id,
            winuser::MF_BYCOMMAND,
        ) & winuser::MF_CHECKED
            > 0
        {
            winuser::CheckMenuItem(
                winuser::GetMenu(hwnd),
                id,
                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
            );

            false
        } else {
            winuser::CheckMenuItem(
                winuser::GetMenu(hwnd),
                id,
                winuser::MF_BYCOMMAND | winuser::MF_CHECKED,
            );

            true
        }
    }
}

enum MsgType {
    Uninject,
    WinMsg(winuser::MSG),
}

static mut SENDER: Option<Sender<MsgType>> = None;

pub struct Runtime<PTC: PTCVersion + ?Sized> {
    features: Vec<Box<dyn Feature<PTC>>>,
}

#[must_use]
pub fn try_run_version(version: (u16, u16, u16, u16)) -> Option<anyhow::Result<()>> {
    match version {
        (0, 9, 2, 5) => Some(Runtime::<crate::ptc::v0925::PTC0925>::new().main()),
        (0, 9, 4, 54) => Some(Runtime::<crate::ptc::v09454::PTC09454>::new().main()),
        _ => None,
    }
}

impl<PTC: PTCVersion> Runtime<PTC> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            features: PTC::get_features(),
        }
    }

    #[allow(clippy::too_many_lines)] // TODO
    pub fn main(&mut self) -> anyhow::Result<()> {
        CombinedLogger::init(vec![TermLogger::new(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )])
        .unwrap();

        let pid = unsafe {
            winapi::um::processthreadsapi::GetProcessId(
                winapi::um::processthreadsapi::GetCurrentProcess(),
            )
        };
        log::info!("PTC Mod starting...");
        log::info!("mod version = {}", VERSION.unwrap_or("unknown"));
        log::info!("PID = {}", pid);

        unsafe {
            let hwnd = PTC::get_hwnd();

            let base: usize = GetModuleHandleA(
                "ptCollage.exe\0"
                    .bytes()
                    .collect::<Vec<u8>>()
                    .as_ptr()
                    .cast::<i8>(),
            ) as usize;

            log::debug!("Base address (allocation address) = {}", base);

            let (v1, v2, v3, v4) = PTC::get_version();
            log::info!("ptc version = {}.{}.{}.{}", v1, v2, v3, v4);

            let msg = format!(
                "Injected!\nPID = {}\nptc version = {}.{}.{}.{}\nmod version = {}\0",
                pid,
                v1,
                v2,
                v3,
                v4,
                VERSION.unwrap_or("unknown"),
            );
            let l_msg: Vec<u16> = msg.encode_utf16().collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            winuser::MessageBoxW(
                *hwnd,
                l_msg.as_ptr(),
                l_title.as_ptr(),
                winuser::MB_OK | winuser::MB_ICONINFORMATION,
            );

            // let log_debug: unsafe extern "fastcall" fn(
            //     param_1: libc::c_int,
            //     param_2: libc::c_int,
            //     text: *const libc::c_char,
            // ) = std::mem::transmute(0x00d4ddf0 as *const ());
            // (log_debug)(0, 0x208, msg.as_ptr() as *const libc::c_char);
            // (log_debug)(*(0x00dd4434 as *const libc::c_int), 123, CString::new("PTC Mod %d").unwrap().into_raw());
            // (log_debug)(*(0x00dd4434 as *const libc::c_int), *(0x00dd4424 as *const libc::c_int), CString::new("PTC Mod2").unwrap().into_raw());

            // let log_debug = &*(0x00d4ddf0 as *const LogDebugFn);
            // log_debug(0, 0, CString::new("PTC Mod").unwrap().into_raw());

            let h_menu = winuser::GetMenu(*hwnd);
            let base = winuser::CreateMenu();
            let l_title: Vec<u8> = "PTC Mod\0".bytes().collect();
            winuser::AppendMenuA(
                h_menu,
                winuser::MF_POPUP,
                base as usize,
                l_title.as_ptr().cast::<i8>(),
            );

            let l_title: Vec<u8> = "Playhead\0".bytes().collect();
            winuser::AppendMenuA(
                base,
                winuser::MF_CHECKED,
                *M_PLAYHEAD_ID as usize,
                l_title.as_ptr().cast::<i8>(),
            );

            winuser::CheckMenuItem(
                base,
                *M_PLAYHEAD_ID as u32,
                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
            );

            for feat in &mut self.features {
                feat.init(base);
            }

            let l_title: Vec<u8> = "About\0".bytes().collect();
            winuser::AppendMenuA(base, 0, *M_ABOUT_ID as usize, l_title.as_ptr().cast::<i8>());

            let l_title: Vec<u8> = "Uninject\0".bytes().collect();
            winuser::AppendMenuA(base, 0, *M_UNINJECT_ID as usize, l_title.as_ptr().cast::<i8>());

            winuser::DrawMenuBar(*hwnd);

            // let event_thread = winapi::um::processthreadsapi::CreateThread(
            //     std::ptr::null_mut(),
            //     0,
            //     Some(event_thread),
            //     std::ptr::null_mut(),
            //     0,
            //     std::ptr::null_mut(),
            // );

            let window_thread = winuser::GetWindowThreadProcessId(*hwnd, std::ptr::null_mut());
            let (tx, rx) = std::sync::mpsc::channel::<MsgType>();
            SENDER = Some(tx);
            let event_hook = winuser::SetWindowsHookExW(
                winuser::WH_GETMESSAGE,
                Some(hook_ex),
                std::ptr::null_mut(),
                window_thread,
            );

            let frame_thread = winapi::um::processthreadsapi::CreateThread(
                // make a thread to live in
                std::ptr::null_mut(),
                0,
                Some(PTC::get_frame_thread_wrapper()),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            );

            // let timer = winuser::SetTimer(*hwnd, SMOOTH_SCROLL_TIMER_ID, 1, None);

            // PTC::start_play();

            // wait for uninject signal
            loop {
                let v = rx.recv().unwrap();
                match v {
                    MsgType::Uninject => break,
                    MsgType::WinMsg(msg) => {
                        self.on_win_msg(msg);
                    }
                    _ => {},
                }
            }

            // cleanup

            for feat in &mut self.features {
                feat.cleanup();
            }

            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
            VirtualProtect(
                crate::ptc::addr(0x16625) as *mut libc::c_void,
                0x5,
                PAGE_EXECUTE_READWRITE,
                &mut lpfl_old_protect_1,
            );

            // call ptCollage.exe+87A0
            let bytes = i32::to_le_bytes(0x87a0 - (0x16625 + 0x5));
            *(crate::ptc::addr(0x16625) as *mut [u8; 5]) =
                [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

            VirtualProtect(
                crate::ptc::addr(0x16625) as *mut libc::c_void,
                0x5,
                lpfl_old_protect_1,
                &mut lpfl_old_protect_1,
            );

            // top

            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
            VirtualProtect(
                crate::ptc::addr(0x166c0) as *mut libc::c_void,
                0x5,
                PAGE_EXECUTE_READWRITE,
                &mut lpfl_old_protect_1,
            );

            // call ptCollage.exe+87A0
            let bytes = i32::to_le_bytes(0x9f80 - (0x166c0 + 0x5));
            *(crate::ptc::addr(0x166c0) as *mut [u8; 5]) =
                [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

            VirtualProtect(
                crate::ptc::addr(0x166c0) as *mut libc::c_void,
                0x5,
                lpfl_old_protect_1,
                &mut lpfl_old_protect_1,
            );

            winapi::um::processthreadsapi::TerminateThread(frame_thread, 0);
            // winuser::KillTimer(*hwnd, timer);

            winuser::RemoveMenu(h_menu, 4, winuser::MF_BYPOSITION);
            winuser::DrawMenuBar(*hwnd);

            winuser::UnhookWindowsHookEx(event_hook);
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)] // TODO
    unsafe fn on_win_msg(&mut self, msg: winuser::MSG) {

        if self.features.iter_mut().any(|f| f.win_msg(&msg)) {
            return;
        }

        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            if high == 0 {

                // can't match against statics
                if low == *M_ABOUT_ID {
                    let l_template: Vec<u8> = "DLG_ABOUT\0".bytes().collect();
                    winuser::DialogBoxParamA(
                        *PTC::get_hinstance(),
                        l_template.as_ptr().cast::<i8>(),
                        msg.hwnd,
                        Some(fill_about_dialog),
                        0,
                    );
                } else if low == *M_UNINJECT_ID {
                    SENDER.as_mut().unwrap().send(MsgType::Uninject).unwrap();
                } else if low == *M_PLAYHEAD_ID {
                    if winuser::GetMenuState(
                        winuser::GetMenu(msg.hwnd),
                        (*M_PLAYHEAD_ID).try_into().unwrap(),
                        winuser::MF_BYCOMMAND,
                    ) & winuser::MF_CHECKED
                        > 0
                    {
                        winuser::CheckMenuItem(
                            winuser::GetMenu(msg.hwnd),
                            (*M_PLAYHEAD_ID).try_into().unwrap(),
                            winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
                        );

                        let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                        VirtualProtect(
                            crate::ptc::addr(0x16625) as *mut libc::c_void,
                            0x5,
                            PAGE_EXECUTE_READWRITE,
                            &mut lpfl_old_protect_1,
                        );

                        // call ptCollage.exe+87A0
                        let bytes = i32::to_le_bytes(0x87a0 - (0x16625 + 0x5));
                        *(crate::ptc::addr(0x16625) as *mut [u8; 5]) =
                            [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

                        VirtualProtect(
                            crate::ptc::addr(0x16625) as *mut libc::c_void,
                            0x5,
                            lpfl_old_protect_1,
                            &mut lpfl_old_protect_1,
                        );

                        // top

                        let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                        VirtualProtect(
                            crate::ptc::addr(0x166c0) as *mut libc::c_void,
                            0x5,
                            PAGE_EXECUTE_READWRITE,
                            &mut lpfl_old_protect_1,
                        );

                        // call ptCollage.exe+9f80
                        let bytes = i32::to_le_bytes(0x9f80 - (0x166c0 + 0x5));
                        *(crate::ptc::addr(0x166c0) as *mut [u8; 5]) =
                            [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

                        VirtualProtect(
                            crate::ptc::addr(0x166c0) as *mut libc::c_void,
                            0x5,
                            lpfl_old_protect_1,
                            &mut lpfl_old_protect_1,
                        );
                    } else {
                        winuser::CheckMenuItem(
                            winuser::GetMenu(msg.hwnd),
                            (*M_PLAYHEAD_ID).try_into().unwrap(),
                            winuser::MF_BYCOMMAND | winuser::MF_CHECKED,
                        );

                        let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                        VirtualProtect(
                            crate::ptc::addr(0x16625) as *mut libc::c_void,
                            0x5,
                            PAGE_EXECUTE_READWRITE,
                            &mut lpfl_old_protect_1,
                        );

                        let target_addr = PTC::get_hook_draw_unitkb_bg() as *const () as usize;
                        // println!("target_addr = {}", target_addr);
                        let bytes = i32::to_le_bytes(
                            (target_addr as i64 - (addr(0x16625) + 0x5) as i64) as i32,
                        );
                        // println!("bytes = {:?}", bytes);
                        *(crate::ptc::addr(0x16625) as *mut [u8; 5]) =
                            [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

                        VirtualProtect(
                            crate::ptc::addr(0x16625) as *mut libc::c_void,
                            0x5,
                            lpfl_old_protect_1,
                            &mut lpfl_old_protect_1,
                        );

                        // top

                        let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                        VirtualProtect(
                            crate::ptc::addr(0x166c0) as *mut libc::c_void,
                            0x5,
                            PAGE_EXECUTE_READWRITE,
                            &mut lpfl_old_protect_1,
                        );

                        let target_addr = PTC::get_hook_draw_unitkb_top() as *const () as usize;
                        let bytes = i32::to_le_bytes(
                            (target_addr as i64 - (addr(0x166c0) + 0x5) as i64) as i32,
                        );
                        *(crate::ptc::addr(0x166c0) as *mut [u8; 5]) =
                            [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

                        VirtualProtect(
                            crate::ptc::addr(0x166c0) as *mut libc::c_void,
                            0x5,
                            lpfl_old_protect_1,
                            &mut lpfl_old_protect_1,
                        );
                    }
                }
            }
        } else if msg.message == winuser::WM_TIMER {
            // let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            // let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());
            // let l_msg: Vec<u16> = format!("message = {}\n{} high = {}\nlow = {}\0", msg.wParam, msg.message, high, low).encode_utf16().collect();
            // let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            // winuser::MessageBoxW(msg.hwnd, l_msg.as_ptr(), l_title.as_ptr(), winuser::MB_OK | winuser::MB_ICONINFORMATION);

            // match msg.wParam {
            //     _ => {}
            // }
        }
    }
}

impl<PTC: PTCVersion> Default for Runtime<PTC> {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) unsafe fn draw_unitkb_bg<PTC: PTCVersion>() {
    // println!("draw_unitkb_bg called");

    // let mut play_pos = LAST_PLAY_POS;
    // if let Some(i) = LAST_PLAY_POS_TIME {
    //     play_pos += (44100.0
    //         * Instant::now()
    //             .saturating_duration_since(i)
    //             .as_secs_f32()
    //             .clamp(0.0, 0.5)) as u32;
    // }

    // let x = (play_pos as f32 / 10000.0).sin() * 50.0 + 200.0;
    // let y = (play_pos as f32 / 10000.0).cos() * 50.0 + 250.0;
    // let rect = [x as i32, y as i32, x as i32 + 20, y as i32 + 20];
    // let draw_rect: unsafe extern "cdecl" fn(rect: *const libc::c_int, color: libc::c_uint) =
    //     std::mem::transmute(addr(0x1c0e0) as *const ());
    // (draw_rect)(rect.as_ptr(), 0x00ff00);

    let fun_000087a0: unsafe extern "stdcall" fn() = std::mem::transmute(addr(0x87a0) as *const ());
    (fun_000087a0)();
}

pub(crate) unsafe fn draw_unitkb_top<PTC: PTCVersion>() {
    // println!("draw_unitkb_top called");

    if PTC::is_playing() {
        let unit_rect = PTC::get_unit_rect();

        let x = crate::feature::scroll::LAST_PLAYHEAD_POS;

        let rect = [x, unit_rect[1], x + 2, unit_rect[3]];
        let draw_rect: unsafe extern "cdecl" fn(rect: *const libc::c_int, color: libc::c_uint) =
            std::mem::transmute(addr(0x1c0e0) as *const ());
        (draw_rect)(rect.as_ptr(), 0xcccccc);
    }

    let fun_00009f80: unsafe extern "stdcall" fn() = std::mem::transmute(addr(0x9f80) as *const ());
    (fun_00009f80)();
}


unsafe extern "system" fn hook_ex(code: i32, w_param: usize, l_param: isize) -> isize {
    if code >= 0 {
        let msg = *(l_param as *const winuser::MSG);
        SENDER.as_mut().unwrap().send(MsgType::WinMsg(msg)).unwrap();
    }

    winuser::CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param)
}

unsafe extern "system" fn fill_about_dialog(
    hwnd: HWND,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if msg == winuser::WM_INITDIALOG {
        let msg_1: Vec<u8> = "PTC Mod\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, 0x3f6, msg_1.as_ptr().cast::<i8>());
        let msg_2: Vec<u8> = "PieKing1215\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, 0x43a, msg_2.as_ptr().cast::<i8>());
        let msg_3: Vec<u8> = format!("version.{}\0", VERSION.unwrap_or("unknown"))
            .bytes()
            .collect();
        winuser::SetDlgItemTextA(hwnd, 0x40c, msg_3.as_ptr().cast::<i8>());
        let msg_4: Vec<u8> = "alpha test\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, 0x3ea, msg_4.as_ptr().cast::<i8>());

        let fn_1: unsafe extern "cdecl" fn(param_1: HWND) =
            std::mem::transmute(addr(0x1e550) as *const ());
        (fn_1)(hwnd);

        let fn_2: unsafe extern "cdecl" fn(param_1: HWND) =
            std::mem::transmute(addr(0x1d310) as *const ());
        (fn_2)(hwnd);
    } else if msg == winuser::WM_COMMAND {
        let high = winapi::shared::minwindef::HIWORD(w_param.try_into().unwrap());
        let low = winapi::shared::minwindef::LOWORD(w_param.try_into().unwrap());

        if high == 0 {
            if low == 1 {
                // click "OK"
                winuser::EndDialog(hwnd, 1);
            } else if l_param == 2 {
                // ESC key
                winuser::EndDialog(hwnd, 0);
            }
        }
    }

    0
}

fn frame_thread<PTC: PTCVersion>(_base: LPVOID) -> anyhow::Result<()> {
    unsafe {
        loop {
            if PTC::is_playing() {
                winuser::InvalidateRect(*PTC::get_hwnd(), std::ptr::null(), 0);
            }

            winapi::um::synchapi::Sleep(2);
        }
    }
}

pub(crate) unsafe fn frame_thread_wrapper_ex<PTC: PTCVersion>(base: LPVOID) -> u32 {
    if let Err(err) = frame_thread::<PTC>(base) {
        let l_msg: Vec<u16> = format!("frame_thread exited with an Err: {:?}\0", err)
            .encode_utf16()
            .collect();
        let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
        winuser::MessageBoxW(
            *PTC::get_hwnd(),
            l_msg.as_ptr(),
            l_title.as_ptr(),
            winuser::MB_OK | winuser::MB_ICONERROR,
        );
    }

    0
}
