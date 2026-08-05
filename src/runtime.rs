use std::{
    convert::TryInto,
    sync::{LazyLock, OnceLock, mpsc::Sender},
};

use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleA,
            Threading::{GetCurrentProcess, GetProcessId, Sleep},
        },
        UI::WindowsAndMessaging::{
            AppendMenuA, CallNextHookEx, DialogBoxParamA, DispatchMessageA, DrawMenuBar, EndDialog,
            GetWindowThreadProcessId, MB_ICONINFORMATION, MB_OK, MENU_ITEM_FLAGS, MSG, MessageBoxW,
            PM_REMOVE, PeekMessageA, SetDlgItemTextA, SetWindowsHookExW, TranslateMessage,
            UnhookWindowsHookEx, WH_GETMESSAGE, WM_COMMAND, WM_INITDIALOG,
        },
    },
    core::{PCSTR, PCWSTR},
};

// TODO: maybe use https://crates.io/crates/built or something to make this more detailed (git hash, etc.)
const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

use crate::{
    feature::Feature,
    ptc::PTCVersion,
    winutil::{self, Menus, hiword, loword},
};

static M_ABOUT_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);
static M_UNINJECT_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);

enum MsgType {
    Uninject,
    WinMsg(MSG),
}

unsafe impl Send for MsgType {}

static SENDER: OnceLock<Sender<MsgType>> = OnceLock::new();

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
        Self { features: PTC::get_features() }
    }

    #[allow(clippy::too_many_lines)] // TODO
    #[allow(clippy::unnecessary_wraps)]
    pub fn main(&mut self) -> anyhow::Result<()> {
        CombinedLogger::init(vec![TermLogger::new(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )])
        .unwrap();

        let pid = unsafe { GetProcessId(GetCurrentProcess()) };
        log::info!("PTC Mod starting...");
        log::info!("mod version = {}", VERSION.unwrap_or("unknown"));
        log::info!("PID = {pid}");

        unsafe {
            let hwnd = PTC::get_hwnd();

            let base = GetModuleHandleA(PCSTR(
                "ptCollage.exe\0".bytes().collect::<Vec<u8>>().as_ptr(),
            ))
            .unwrap()
            .0;

            log::debug!("Base address (allocation address) = {base:x?}");

            let (v1, v2, v3, v4) = PTC::get_version();
            log::info!("ptc version = {v1}.{v2}.{v3}.{v4}");

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
            MessageBoxW(
                Some(*hwnd),
                PCWSTR::from_raw(l_msg.as_ptr()),
                PCWSTR::from_raw(l_title.as_ptr()),
                MB_OK | MB_ICONINFORMATION,
            );

            let mut menus = Menus::new();

            for feat in &mut self.features {
                feat.init(&mut menus);
            }

            let base = menus.get_or_create::<PTC>("PTC Mod");

            let l_title: Vec<u8> = "About\0".bytes().collect();
            AppendMenuA(
                base,
                MENU_ITEM_FLAGS::default(),
                *M_ABOUT_ID as usize,
                PCSTR(l_title.as_ptr()),
            )
            .unwrap();

            let l_title: Vec<u8> = "Uninject\0".bytes().collect();
            AppendMenuA(
                base,
                MENU_ITEM_FLAGS::default(),
                *M_UNINJECT_ID as usize,
                PCSTR(l_title.as_ptr()),
            )
            .unwrap();

            DrawMenuBar(*hwnd).unwrap();

            let window_thread = GetWindowThreadProcessId(*hwnd, None);
            let (tx, rx) = std::sync::mpsc::channel::<MsgType>();
            SENDER.set(tx).unwrap();
            let event_hook =
                SetWindowsHookExW(WH_GETMESSAGE, Some(hook_ex), None, window_thread).unwrap();

            // block for signals from windows
            loop {
                let mut did_something = false;

                if let Ok(v) = rx.try_recv() {
                    did_something = true;
                    match v {
                        MsgType::Uninject => break,
                        MsgType::WinMsg(msg) => {
                            self.on_win_msg(msg);
                        },
                    }
                }

                // we need to pump on our thread since drag/drop needs to be done on this thread
                let mut msg = MSG::default();
                if PeekMessageA(&raw mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    did_something = true;
                    // TranslateMessage's BOOL return does not represent an error and we don't care about it
                    let _ = TranslateMessage(&raw const msg);
                    DispatchMessageA(&raw const msg);
                }

                if !did_something {
                    Sleep(5);
                }
            }

            // cleanup

            for feat in &mut self.features {
                feat.cleanup();
            }

            menus.cleanup::<PTC>();

            DrawMenuBar(*hwnd).unwrap();

            UnhookWindowsHookEx(event_hook).unwrap();
        }

        Ok(())
    }

    unsafe fn on_win_msg(&mut self, msg: MSG) {
        self.features.iter_mut().for_each(|f| f.win_msg(&msg));

        if msg.message == WM_COMMAND {
            let high = hiword(msg.wParam.0.try_into().unwrap());
            let low = loword(msg.wParam.0.try_into().unwrap());

            if high == 0 {
                // can't match against statics
                if low == *M_ABOUT_ID {
                    let l_template: Vec<u8> = "DLG_ABOUT\0".bytes().collect();
                    unsafe {
                        DialogBoxParamA(
                            Some(*PTC::get_hinstance()),
                            PCSTR(l_template.as_ptr()),
                            Some(msg.hwnd),
                            Some(PTC::get_fill_about_dialog()),
                            LPARAM(0),
                        )
                    };
                } else if low == *M_UNINJECT_ID {
                    SENDER.get().unwrap().send(MsgType::Uninject).unwrap();
                }
            }
        }
    }
}

impl<PTC: PTCVersion> Default for Runtime<PTC> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe extern "system" fn hook_ex(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if code >= 0 {
        // need to copy since we handle this on the main thread, so the pointer will be gone
        // (not sure if this is really safe or not)
        let msg = unsafe { *(l_param.0 as *const MSG) };
        let _ = SENDER.get().unwrap().send(MsgType::WinMsg(msg));
    }

    unsafe { CallNextHookEx(None, code, w_param, l_param) }
}

pub unsafe fn fill_about_dialog<PTC: PTCVersion>(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> isize {
    if msg == WM_INITDIALOG {
        let ids = PTC::get_about_dialog_text_ids();
        unsafe {
            let msg_1: Vec<u8> = "PTC Mod\0".bytes().collect();
            SetDlgItemTextA(hwnd, ids.0, PCSTR(msg_1.as_ptr())).unwrap();
            let msg_2: Vec<u8> = "PieKing1215\0".bytes().collect();
            SetDlgItemTextA(hwnd, ids.1, PCSTR(msg_2.as_ptr())).unwrap();
            let msg_3: Vec<u8> = format!("version.{}\0", VERSION.unwrap_or("unknown"))
                .bytes()
                .collect();
            SetDlgItemTextA(hwnd, ids.2, PCSTR(msg_3.as_ptr())).unwrap();
            let msg_4: Vec<u8> = "alpha test\0".bytes().collect();
            SetDlgItemTextA(hwnd, ids.3, PCSTR(msg_4.as_ptr())).unwrap();
        }

        PTC::center_window(hwnd);
        PTC::about_dlg_fn_2(hwnd);
    } else if msg == WM_COMMAND {
        let high = hiword(w_param.0.try_into().unwrap());
        let low = loword(w_param.0.try_into().unwrap());

        if high == 0 {
            if low == 1 {
                // click "OK"
                unsafe { EndDialog(hwnd, 1).unwrap() };
            } else if l_param.0 == 2 {
                // ESC key
                unsafe { EndDialog(hwnd, 0).unwrap() };
            }
        }
    }

    0
}
