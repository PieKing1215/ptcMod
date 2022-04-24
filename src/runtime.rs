use std::{convert::TryInto, mem::MaybeUninit, sync::mpsc::Sender};

use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use winapi::{
    shared::windef::HWND,
    um::{
        libloaderapi::GetModuleHandleA,
        synchapi::Sleep,
        winuser::{self, DispatchMessageA, PeekMessageA, TranslateMessage},
    },
};

// TODO: maybe use https://crates.io/crates/built or something to make this more detailed (git hash, etc.)
const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

use crate::{
    feature::Feature,
    ptc::PTCVersion,
    winutil::{self, Menus},
};

lazy_static::lazy_static! {
    static ref M_ABOUT_ID: u16 = winutil::next_id();
    static ref M_UNINJECT_ID: u16 = winutil::next_id();
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

            let mut menus = Menus::new();

            for feat in &mut self.features {
                feat.init(&mut menus);
            }

            let base = menus.get_or_create::<PTC>("PTC Mod");

            let l_title: Vec<u8> = "About\0".bytes().collect();
            winuser::AppendMenuA(base, 0, *M_ABOUT_ID as usize, l_title.as_ptr().cast::<i8>());

            let l_title: Vec<u8> = "Uninject\0".bytes().collect();
            winuser::AppendMenuA(
                base,
                0,
                *M_UNINJECT_ID as usize,
                l_title.as_ptr().cast::<i8>(),
            );

            winuser::DrawMenuBar(*hwnd);

            let window_thread = winuser::GetWindowThreadProcessId(*hwnd, std::ptr::null_mut());
            let (tx, rx) = std::sync::mpsc::channel::<MsgType>();
            SENDER = Some(tx);
            let event_hook = winuser::SetWindowsHookExW(
                winuser::WH_GETMESSAGE,
                Some(hook_ex),
                std::ptr::null_mut(),
                window_thread,
            );

            // block for signals from windows
            loop {
                let mut did_something = false;

                if let Ok(v) = rx.try_recv() {
                    did_something = true;
                    match v {
                        MsgType::Uninject => break,
                        MsgType::WinMsg(msg) => {
                            self.on_win_msg(msg);
                        }
                    }
                }

                // we need to pump on our thread since drag/drop needs to be done on this thread
                let mut msg = MaybeUninit::<winuser::MSG>::uninit();
                if PeekMessageA(
                    msg.as_mut_ptr(),
                    std::ptr::null_mut(),
                    0,
                    0,
                    winuser::PM_REMOVE,
                ) != 0
                {
                    did_something = true;
                    TranslateMessage(msg.as_ptr());
                    DispatchMessageA(msg.as_ptr());
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

            winuser::DrawMenuBar(*hwnd);

            winuser::UnhookWindowsHookEx(event_hook);
        }

        Ok(())
    }

    unsafe fn on_win_msg(&mut self, msg: winuser::MSG) {
        self.features.iter_mut().for_each(|f| f.win_msg(&msg));

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
                        Some(PTC::get_fill_about_dialog()),
                        0,
                    );
                } else if low == *M_UNINJECT_ID {
                    SENDER.as_mut().unwrap().send(MsgType::Uninject).unwrap();
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

unsafe extern "system" fn hook_ex(code: i32, w_param: usize, l_param: isize) -> isize {
    if code >= 0 {
        // need to copy since we handle this on the main thread, so the pointer will be gone
        // (not sure if this is really safe or not)
        let msg = *(l_param as *const winuser::MSG);
        SENDER.as_mut().unwrap().send(MsgType::WinMsg(msg)).unwrap();
    }

    winuser::CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param)
}

pub unsafe fn fill_about_dialog<PTC: PTCVersion>(
    hwnd: HWND,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if msg == winuser::WM_INITDIALOG {
        let ids = PTC::get_about_dialog_text_ids();
        let msg_1: Vec<u8> = "PTC Mod\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, ids.0, msg_1.as_ptr().cast::<i8>());
        let msg_2: Vec<u8> = "PieKing1215\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, ids.1, msg_2.as_ptr().cast::<i8>());
        let msg_3: Vec<u8> = format!("version.{}\0", VERSION.unwrap_or("unknown"))
            .bytes()
            .collect();
        winuser::SetDlgItemTextA(hwnd, ids.2, msg_3.as_ptr().cast::<i8>());
        let msg_4: Vec<u8> = "alpha test\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, ids.3, msg_4.as_ptr().cast::<i8>());

        PTC::center_window(hwnd);
        PTC::about_dlg_fn_2(hwnd);
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
