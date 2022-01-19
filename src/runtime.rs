use std::{convert::TryInto, marker::PhantomData, sync::mpsc::Sender, time::Instant};

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

use crate::ptc::{addr, PTCVersion};

const M_SMOOTH_SCROLL_ID: usize = 1001;
const M_FPS_UNLOCK: usize = 1002;
const M_FRAME_HOOK: usize = 1003;
const M_ABOUT_ID: usize = 1004;
const M_UNINJECT_ID: usize = 1005;

static mut SENDER: Option<Sender<()>> = None;

pub struct Runtime<PTC: PTCVersion + ?Sized> {
    _phantom: PhantomData<PTC>,
}

pub fn try_run_version(version: (u16, u16, u16, u16)) -> Option<anyhow::Result<()>> {
    match version {
        (0, 9, 2, 5) => Some(Runtime::<crate::ptc::v0925::PTC0925>::new().main()),
        (0, 9, 4, 54) => Some(Runtime::<crate::ptc::v09454::PTC09454>::new().main()),
        _ => None,
    }
}

impl<PTC: PTCVersion> Runtime<PTC> {
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }

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
                "ptCollage.exe\0".bytes().collect::<Vec<u8>>().as_ptr() as *const i8,
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
                l_title.as_ptr() as *const i8,
            );

            let l_title: Vec<u8> = "Smooth Scroll\0".bytes().collect();
            winuser::AppendMenuA(
                base,
                winuser::MF_CHECKED,
                M_SMOOTH_SCROLL_ID,
                l_title.as_ptr() as *const i8,
            );

            let l_title: Vec<u8> = "FPS Unlock\0".bytes().collect();
            winuser::AppendMenuA(
                base,
                winuser::MF_CHECKED,
                M_FPS_UNLOCK,
                l_title.as_ptr() as *const i8,
            );

            winuser::CheckMenuItem(
                base,
                M_FPS_UNLOCK.try_into().unwrap(),
                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
            );

            let l_title: Vec<u8> = "dbg frame hook\0".bytes().collect();
            winuser::AppendMenuA(
                base,
                winuser::MF_CHECKED,
                M_FRAME_HOOK,
                l_title.as_ptr() as *const i8,
            );

            winuser::CheckMenuItem(
                base,
                M_FRAME_HOOK.try_into().unwrap(),
                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
            );

            let l_title: Vec<u8> = "About\0".bytes().collect();
            winuser::AppendMenuA(base, 0, M_ABOUT_ID, l_title.as_ptr() as *const i8);

            let l_title: Vec<u8> = "Uninject\0".bytes().collect();
            winuser::AppendMenuA(base, 0, M_UNINJECT_ID, l_title.as_ptr() as *const i8);

            winuser::DrawMenuBar(*hwnd);

            // let event_thread = winapi::um::processthreadsapi::CreateThread(
            //     std::ptr::null_mut(),
            //     0,
            //     Some(event_thread),
            //     std::ptr::null_mut(),
            //     0,
            //     std::ptr::null_mut(),
            // );

            let window_thread = winuser::GetWindowThreadProcessId(*hwnd, 0 as *mut u32);
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            SENDER = Some(tx);
            let event_hook = winuser::SetWindowsHookExW(
                winuser::WH_GETMESSAGE,
                Some(PTC::get_hook()),
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
            rx.recv().unwrap();

            // cleanup

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

            // unit notes

            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
            VirtualProtect(
                crate::ptc::addr(0x1469f) as *mut libc::c_void,
                0x5,
                PAGE_EXECUTE_READWRITE,
                &mut lpfl_old_protect_1,
            );

            // call ptCollage.exe+1c0e0
            let bytes = i32::to_le_bytes(0x1c0e0 - (0x1469f + 0x5));
            *(crate::ptc::addr(0x1469f) as *mut [u8; 5]) =
                [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

            VirtualProtect(
                crate::ptc::addr(0x1469f) as *mut libc::c_void,
                0x5,
                lpfl_old_protect_1,
                &mut lpfl_old_protect_1,
            );

            // (disable note left edge)
            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
            VirtualProtect(
                crate::ptc::addr(0x146b8) as *mut libc::c_void,
                0x1,
                PAGE_EXECUTE_READWRITE,
                &mut lpfl_old_protect_1,
            );

            // push 03
            *(crate::ptc::addr(0x146b8) as *mut u8) = 3;

            VirtualProtect(
                crate::ptc::addr(0x146b8) as *mut libc::c_void,
                0x1,
                lpfl_old_protect_1,
                &mut lpfl_old_protect_1,
            );

            // (disable note right edge)
            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
            VirtualProtect(
                crate::ptc::addr(0x146e9) as *mut libc::c_void,
                0x1,
                PAGE_EXECUTE_READWRITE,
                &mut lpfl_old_protect_1,
            );

            // push 03
            *(crate::ptc::addr(0x146e9) as *mut u8) = 3;

            VirtualProtect(
                crate::ptc::addr(0x146e9) as *mut libc::c_void,
                0x1,
                lpfl_old_protect_1,
                &mut lpfl_old_protect_1,
            );

            // (push ebp instead of note color)
            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
            VirtualProtect(
                crate::ptc::addr(0x1469a) as *mut libc::c_void,
                0x1,
                PAGE_EXECUTE_READWRITE,
                &mut lpfl_old_protect_1,
            );

            // push edx
            *(crate::ptc::addr(0x1469a) as *mut u8) = 0x52;

            VirtualProtect(
                crate::ptc::addr(0x1469a) as *mut libc::c_void,
                0x1,
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
}

static mut LAST_PLAY_POS: u32 = 0;
static mut LAST_PLAY_POS_TIME: Option<Instant> = None;
static mut LAST_SCROLL: i32 = 0;
static mut LAST_PLAYHEAD_POS: i32 = 0;

// the second parameter here would normally be color, but an asm patch is used to change it to push the ebp register instead
//      which can be used to get the unit and focus state (which could be used to get the original color anyway)
pub(crate) unsafe fn draw_unit_note_rect<PTC: PTCVersion>(
    rect: *const libc::c_int,
    ebp: libc::c_uint,
) {
    // color = 0x0094FF;

    let not_focused = *((ebp - 0x7c) as *mut u32) != 0;

    let color = *(addr(0xa6cb4 + if not_focused { 0x4 } else { 0 }) as *mut u32);
    let raw_argb = color.to_be_bytes();
    let mut rgb = colorsys::Rgb::from([raw_argb[1], raw_argb[2], raw_argb[3]]);

    let unit = *((ebp - 0x80) as *mut u32);

    rgb.adjust_hue(unit as f64 * 25.0);

    let rect = std::slice::from_raw_parts(rect, 4);

    if PTC::is_playing() {
        if rect[0] <= LAST_PLAYHEAD_POS {
            // TODO: clean up this logic
            let flash_strength = if not_focused { 0.5 } else { 0.95 };
            if rect[2] >= LAST_PLAYHEAD_POS {
                let get_event_value: unsafe extern "cdecl" fn(
                    pos_x: i32,
                    unit_no: i32,
                    ev_type: i32,
                ) -> i32 = std::mem::transmute(addr(0x8f80) as *const ());

                let volume: f32 =
                    (get_event_value)(LAST_PLAYHEAD_POS, unit as i32, 0x5) as f32 / 128.0;
                let velocity: f32 =
                    (get_event_value)(LAST_PLAYHEAD_POS, unit as i32, 0x5) as f32 / 128.0;

                let factor = volume * velocity;
                let factor = factor.powf(0.25);

                let mix = flash_strength as f64;
                rgb.set_red(rgb.red() + (255.0 - rgb.red()) * mix);
                rgb.set_green(rgb.green() + (255.0 - rgb.green()) * mix);
                rgb.set_blue(rgb.blue() + (255.0 - rgb.blue()) * mix);

                let fade_color: [u8; 4] = if not_focused {
                    0xff200040u32
                } else {
                    0xff400070
                }
                .to_be_bytes();
                let mix = 1.0 - (factor as f64) * 0.8;
                rgb.set_red(rgb.red() + (fade_color[1] as f64 - rgb.red()) * mix);
                rgb.set_green(rgb.green() + (fade_color[2] as f64 - rgb.green()) * mix);
                rgb.set_blue(rgb.blue() + (fade_color[3] as f64 - rgb.blue()) * mix);
            } else {
                let fade_size = *PTC::get_measure_width() as i32 / 4;
                let fade_pt = LAST_PLAYHEAD_POS - fade_size;

                let get_event_value: unsafe extern "cdecl" fn(
                    pos_x: i32,
                    unit_no: i32,
                    ev_type: i32,
                ) -> i32 = std::mem::transmute(addr(0x8f80) as *const ());

                let volume: f32 = (get_event_value)(rect[2], unit as i32, 0x5) as f32 / 128.0;
                let velocity: f32 = (get_event_value)(rect[2], unit as i32, 0x5) as f32 / 128.0;

                let factor = volume * velocity;
                let factor = factor.powf(0.25);

                if rect[2] >= fade_pt {
                    let thru = (rect[2] - fade_pt) as f32 / fade_size as f32;

                    let mix = thru as f64 * flash_strength as f64;
                    rgb.set_red(rgb.red() + (255.0 - rgb.red()) * mix);
                    rgb.set_green(rgb.green() + (255.0 - rgb.green()) * mix);
                    rgb.set_blue(rgb.blue() + (255.0 - rgb.blue()) * mix);
                }

                let fade_color: [u8; 4] = if not_focused {
                    0xff200040u32
                } else {
                    0xff400070
                }
                .to_be_bytes();
                let mix = 1.0 - (factor as f64) * 0.8;
                rgb.set_red(rgb.red() + (fade_color[1] as f64 - rgb.red()) * mix);
                rgb.set_green(rgb.green() + (fade_color[2] as f64 - rgb.green()) * mix);
                rgb.set_blue(rgb.blue() + (fade_color[3] as f64 - rgb.blue()) * mix);
            }
        } else {
            let fade_color: [u8; 4] = if not_focused {
                0xff200040u32
            } else {
                0xff400070
            }
            .to_be_bytes();

            let get_event_value: unsafe extern "cdecl" fn(
                pos_x: i32,
                unit_no: i32,
                ev_type: i32,
            ) -> i32 = std::mem::transmute(addr(0x8f80) as *const ());

            let volume: f32 = (get_event_value)(rect[0], unit as i32, 0x5) as f32 / 128.0;
            let velocity: f32 = (get_event_value)(rect[0], unit as i32, 0x5) as f32 / 128.0;

            let factor = volume * velocity;
            let factor = factor.powf(0.25);

            let mix = 1.0 - (factor as f64) * 0.8;
            rgb.set_red(rgb.red() + (fade_color[1] as f64 - rgb.red()) * mix);
            rgb.set_green(rgb.green() + (fade_color[2] as f64 - rgb.green()) * mix);
            rgb.set_blue(rgb.blue() + (fade_color[3] as f64 - rgb.blue()) * mix);
        }
    }

    let rgb_arr: [u8; 3] = rgb.into();

    let color = u32::from_be_bytes([0, rgb_arr[0], rgb_arr[1], rgb_arr[2]]);

    let draw_rect: unsafe extern "cdecl" fn(rect: *const libc::c_int, color: libc::c_uint) =
        std::mem::transmute(addr(0x1c0e0) as *const ());

    // left edge
    (draw_rect)(rect.as_ptr(), color);

    // main
    (draw_rect)(
        [rect[0] - 1, rect[1] - 1, rect[0], rect[3] + 1].as_ptr(),
        color,
    );
    (draw_rect)(
        [rect[0] - 2, rect[1] - 3, rect[0] - 1, rect[3] + 3].as_ptr(),
        color,
    );

    // right edge
    (draw_rect)([rect[2], rect[1], rect[2] + 1, rect[3]].as_ptr(), color);
    (draw_rect)(
        [rect[2] + 1, rect[1] + 1, rect[2] + 2, rect[3] - 1].as_ptr(),
        color,
    );

    // let get_event_value: unsafe extern "cdecl" fn(pos_x: i32, unit_no: i32, ev_type: i32) -> i32 =
    // std::mem::transmute(addr(0x8f80) as *const ());
    // for x in 0..600 {
    //     let volume = (get_event_value)(x, unit as i32, 0x5);
    //     (draw_rect)([x, 256 - volume, x + 1, 256].as_ptr(), 0xff0000);
    // }
}

pub(crate) unsafe fn draw_unitkb_bg<PTC: PTCVersion>() {
    // println!("draw_unitkb_bg called");

    let mut play_pos = LAST_PLAY_POS;
    if let Some(i) = LAST_PLAY_POS_TIME {
        play_pos += (44100.0
            * Instant::now()
                .saturating_duration_since(i)
                .as_secs_f32()
                .clamp(0.0, 0.5)) as u32;
    }

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
        {
            let smooth = winuser::GetMenuState(
                winuser::GetMenu(*PTC::get_hwnd()),
                M_SMOOTH_SCROLL_ID.try_into().unwrap(),
                winuser::MF_BYCOMMAND,
            ) & winuser::MF_CHECKED
                > 0;

            let mut play_pos =
                *PTC::get_play_pos() / PTC::get_buffer_size() * PTC::get_buffer_size();
            if play_pos != LAST_PLAY_POS {
                LAST_PLAY_POS_TIME = Some(Instant::now());
                LAST_PLAY_POS = play_pos;
            } else if let Some(i) = LAST_PLAY_POS_TIME {
                play_pos += (44100.0
                    * Instant::now()
                        .saturating_duration_since(i)
                        .as_secs_f32()
                        .clamp(0.0, 0.5)) as u32;
            }
            // *((0xdd6d70 + 0x14) as *mut i32) = (((msg.time as f32) / 500.0).sin() * 100.0 + 300.0) as i32;
            let mut des_scroll = (((play_pos as f32
                * *PTC::get_tempo()
                * 4.0
                // * *PTC::get_beat_num() as f32
                * *PTC::get_measure_width() as f32)
                / (PTC::get_beat_clock() as f32))
                / 22050.0) as i32;

            LAST_SCROLL = des_scroll;

            if smooth {
                // let view_rect = PTC::get_unit_rect();
                // des_scroll -= (view_rect[2] - view_rect[0]) / 2;
                des_scroll -= (*PTC::get_measure_width() * 4) as i32;
            } else {
                des_scroll = des_scroll / (*PTC::get_measure_width() * 4) as i32
                    * (*PTC::get_measure_width() * 4) as i32;
            }

            let old_scroll = *PTC::get_scroll();
            *PTC::get_scroll() = des_scroll.clamp(0, PTC::get_scroll_max());
        }

        let mut play_pos = LAST_PLAY_POS;
        /*if play_pos != LAST_PLAY_POS {
            LAST_PLAY_POS_TIME = Some(Instant::now());
            LAST_PLAY_POS = play_pos;
        } else */
        if let Some(i) = LAST_PLAY_POS_TIME {
            play_pos += (44100.0
                * Instant::now()
                    .saturating_duration_since(i)
                    .as_secs_f32()
                    .clamp(0.0, 0.5)) as u32;
        }
        // *((0xdd6d70 + 0x14) as *mut i32) = (((msg.time as f32) / 500.0).sin() * 100.0 + 300.0) as i32;
        let des_scroll = (((play_pos as f32
            * *PTC::get_tempo()
            * 4.0
            // * *PTC::get_beat_num() as f32
            * *PTC::get_measure_width() as f32)
            / (PTC::get_beat_clock() as f32))
            / 22050.0) as i32;

        let unit_rect = PTC::get_unit_rect();

        let x = unit_rect[0] - *PTC::get_scroll() + LAST_SCROLL;
        LAST_PLAYHEAD_POS = x;

        let rect = [x, unit_rect[1], x + 2, unit_rect[3]];
        let draw_rect: unsafe extern "cdecl" fn(rect: *const libc::c_int, color: libc::c_uint) =
            std::mem::transmute(addr(0x1c0e0) as *const ());
        (draw_rect)(rect.as_ptr(), 0xcccccc);

        // winuser::InvalidateRect(
        //     *PTC::get_hwnd(),
        //     0 as *const winapi::shared::windef::RECT,
        //     0,
        // );
        winuser::RedrawWindow(
            *PTC::get_hwnd(),
            0 as *const _,
            0 as *mut _,
            winuser::RDW_INTERNALPAINT,
        );
    }

    let fun_00009f80: unsafe extern "stdcall" fn() = std::mem::transmute(addr(0x9f80) as *const ());
    (fun_00009f80)();
}

pub(crate) unsafe fn hook_ex<PTC: PTCVersion>(code: i32, w_param: usize, l_param: isize) -> isize {
    if code < 0 {
        winuser::CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param)
    } else {
        let msg = &*(l_param as *const winuser::MSG);

        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            if high == 0 {
                match low as usize {
                    M_ABOUT_ID => {
                        let l_template: Vec<u8> = "DLG_ABOUT\0".bytes().collect();
                        winuser::DialogBoxParamA(
                            *PTC::get_hinstance(),
                            l_template.as_ptr() as *const i8,
                            msg.hwnd,
                            Some(fill_about_dialog),
                            0,
                        );
                    }
                    M_UNINJECT_ID => {
                        SENDER.as_mut().unwrap().send(()).unwrap();
                    }
                    M_SMOOTH_SCROLL_ID => {
                        if winuser::GetMenuState(
                            winuser::GetMenu(msg.hwnd),
                            M_SMOOTH_SCROLL_ID.try_into().unwrap(),
                            winuser::MF_BYCOMMAND,
                        ) & winuser::MF_CHECKED
                            > 0
                        {
                            winuser::CheckMenuItem(
                                winuser::GetMenu(msg.hwnd),
                                M_SMOOTH_SCROLL_ID.try_into().unwrap(),
                                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
                            );
                        } else {
                            winuser::CheckMenuItem(
                                winuser::GetMenu(msg.hwnd),
                                M_SMOOTH_SCROLL_ID.try_into().unwrap(),
                                winuser::MF_BYCOMMAND | winuser::MF_CHECKED,
                            );
                        }
                    }
                    M_FPS_UNLOCK => {
                        if winuser::GetMenuState(
                            winuser::GetMenu(msg.hwnd),
                            M_FPS_UNLOCK.try_into().unwrap(),
                            winuser::MF_BYCOMMAND,
                        ) & winuser::MF_CHECKED
                            > 0
                        {
                            winuser::CheckMenuItem(
                                winuser::GetMenu(msg.hwnd),
                                M_FPS_UNLOCK.try_into().unwrap(),
                                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
                            );

                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x00D467f3 - 0xd30000) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );
                            *(crate::ptc::addr(0x00D467f3 - 0xd30000) as *mut u8) = 0x01;
                            VirtualProtect(
                                crate::ptc::addr(0x00D467f3 - 0xd30000) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );

                            let mut lpfl_old_protect_2: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x00D46808 - 0xd30000) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_2,
                            );
                            *(crate::ptc::addr(0x00D46808 - 0xd30000) as *mut u8) = 0x72;
                            VirtualProtect(
                                crate::ptc::addr(0x00D46808 - 0xd30000) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_2,
                                &mut lpfl_old_protect_2,
                            );

                            let mut lpfl_old_protect_3: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x00D46809 - 0xd30000) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_3,
                            );
                            *(crate::ptc::addr(0x00D46809 - 0xd30000) as *mut u8) = 0xe8;
                            VirtualProtect(
                                crate::ptc::addr(0x00D46809 - 0xd30000) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_3,
                                &mut lpfl_old_protect_3,
                            );
                        } else {
                            winuser::CheckMenuItem(
                                winuser::GetMenu(msg.hwnd),
                                M_FPS_UNLOCK.try_into().unwrap(),
                                winuser::MF_BYCOMMAND | winuser::MF_CHECKED,
                            );

                            // let fps_patch_old = (*(0x00D467f3 as *mut u8), *(0x00D46808 as *mut u16));

                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x00D467f3 - 0xd30000) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );
                            *(crate::ptc::addr(0x00D467f3 - 0xd30000) as *mut u8) = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x00D467f3 - 0xd30000) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );

                            let mut lpfl_old_protect_2: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x00D46808 - 0xd30000) as *mut libc::c_void,
                                0x2,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_2,
                            );
                            *(crate::ptc::addr(0x00D46808 - 0xd30000) as *mut u16) = 0x9090;
                            VirtualProtect(
                                crate::ptc::addr(0x00D46808 - 0xd30000) as *mut libc::c_void,
                                0x2,
                                lpfl_old_protect_2,
                                &mut lpfl_old_protect_2,
                            );
                        }
                    }
                    M_FRAME_HOOK => {
                        if winuser::GetMenuState(
                            winuser::GetMenu(msg.hwnd),
                            M_FRAME_HOOK.try_into().unwrap(),
                            winuser::MF_BYCOMMAND,
                        ) & winuser::MF_CHECKED
                            > 0
                        {
                            winuser::CheckMenuItem(
                                winuser::GetMenu(msg.hwnd),
                                M_FRAME_HOOK.try_into().unwrap(),
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

                            // unit notes

                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x1469f) as *mut libc::c_void,
                                0x5,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );

                            // call ptCollage.exe+1c0e0
                            let bytes = i32::to_le_bytes(0x1c0e0 - (0x1469f + 0x5));
                            *(crate::ptc::addr(0x1469f) as *mut [u8; 5]) =
                                [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

                            VirtualProtect(
                                crate::ptc::addr(0x1469f) as *mut libc::c_void,
                                0x5,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );

                            // (enable note left edge)
                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x146b8) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );

                            // push 03
                            *(crate::ptc::addr(0x146b8) as *mut u8) = 3;

                            VirtualProtect(
                                crate::ptc::addr(0x146b8) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );

                            // (enable note right edge)
                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x146e9) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );

                            // push 03
                            *(crate::ptc::addr(0x146e9) as *mut u8) = 3;

                            VirtualProtect(
                                crate::ptc::addr(0x146e9) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );

                            // (push ebp instead of note color)
                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x1469a) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );

                            // push edx
                            *(crate::ptc::addr(0x1469a) as *mut u8) = 0x52;

                            VirtualProtect(
                                crate::ptc::addr(0x1469a) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );
                        } else {
                            winuser::CheckMenuItem(
                                winuser::GetMenu(msg.hwnd),
                                M_FRAME_HOOK.try_into().unwrap(),
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

                            // unit notes

                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x1469f) as *mut libc::c_void,
                                0x5,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );

                            // call ptCollage.exe+1c0e0
                            let target_addr =
                                PTC::get_hook_draw_unit_note_rect() as *const () as usize;
                            let bytes = i32::to_le_bytes(
                                (target_addr as i64 - (addr(0x1469f) + 0x5) as i64) as i32,
                            );
                            *(crate::ptc::addr(0x1469f) as *mut [u8; 5]) =
                                [0xe8, bytes[0], bytes[1], bytes[2], bytes[3]];

                            VirtualProtect(
                                crate::ptc::addr(0x1469f) as *mut libc::c_void,
                                0x5,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );

                            // (disable note left edge)
                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x146b8) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );

                            // push 0
                            *(crate::ptc::addr(0x146b8) as *mut u8) = 0;

                            VirtualProtect(
                                crate::ptc::addr(0x146b8) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );

                            // (disable note right edge)
                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x146e9) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );

                            // push 0
                            *(crate::ptc::addr(0x146e9) as *mut u8) = 0;

                            VirtualProtect(
                                crate::ptc::addr(0x146e9) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );

                            // (push ebp instead of note color)
                            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
                            VirtualProtect(
                                crate::ptc::addr(0x1469a) as *mut libc::c_void,
                                0x1,
                                PAGE_EXECUTE_READWRITE,
                                &mut lpfl_old_protect_1,
                            );

                            // push ebp
                            *(crate::ptc::addr(0x1469a) as *mut u8) = 0x55;

                            VirtualProtect(
                                crate::ptc::addr(0x1469a) as *mut libc::c_void,
                                0x1,
                                lpfl_old_protect_1,
                                &mut lpfl_old_protect_1,
                            );
                        }
                    }
                    _ => {}
                }
            }
        } else if msg.message == winuser::WM_TIMER {
            // let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            // let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());
            // let l_msg: Vec<u16> = format!("message = {}\n{} high = {}\nlow = {}\0", msg.wParam, msg.message, high, low).encode_utf16().collect();
            // let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            // winuser::MessageBoxW(msg.hwnd, l_msg.as_ptr(), l_title.as_ptr(), winuser::MB_OK | winuser::MB_ICONINFORMATION);

            match msg.wParam {
                _ => {}
            }
        }

        winuser::CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param)
    }
}

unsafe extern "system" fn fill_about_dialog(
    hwnd: HWND,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if msg == winuser::WM_INITDIALOG {
        let msg_1: Vec<u8> = "PTC Mod\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, 0x3f6, msg_1.as_ptr() as *const i8);
        let msg_2: Vec<u8> = "PieKing1215\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, 0x43a, msg_2.as_ptr() as *const i8);
        let msg_3: Vec<u8> = format!("version.{}\0", VERSION.unwrap_or("unknown"))
            .bytes()
            .collect();
        winuser::SetDlgItemTextA(hwnd, 0x40c, msg_3.as_ptr() as *const i8);
        let msg_4: Vec<u8> = "alpha test\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, 0x3ea, msg_4.as_ptr() as *const i8);

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
                // let smooth = winuser::GetMenuState(
                //     winuser::GetMenu(*PTC::get_hwnd()),
                //     M_SMOOTH_SCROLL_ID.try_into().unwrap(),
                //     winuser::MF_BYCOMMAND,
                // ) & winuser::MF_CHECKED
                //     > 0;

                // let mut play_pos =
                //     *PTC::get_play_pos() / PTC::get_buffer_size() * PTC::get_buffer_size();
                // if play_pos != LAST_PLAY_POS {
                //     LAST_PLAY_POS_TIME = Some(Instant::now());
                //     LAST_PLAY_POS = play_pos;
                // } else if let Some(i) = LAST_PLAY_POS_TIME {
                //     play_pos += (44100.0
                //         * Instant::now()
                //             .saturating_duration_since(i)
                //             .as_secs_f32()
                //             .clamp(0.0, 0.5)) as u32;
                // }
                // // *((0xdd6d70 + 0x14) as *mut i32) = (((msg.time as f32) / 500.0).sin() * 100.0 + 300.0) as i32;
                // let mut des_scroll = (((play_pos as f32
                //     * *PTC::get_tempo()
                //     * 4.0
                //     // * *PTC::get_beat_num() as f32
                //     * *PTC::get_measure_width() as f32)
                //     / (PTC::get_beat_clock() as f32))
                //     / 22050.0) as i32;

                // if smooth {
                //     // let view_rect = PTC::get_unit_rect();
                //     // des_scroll -= (view_rect[2] - view_rect[0]) / 2;
                //     des_scroll -= (*PTC::get_measure_width() * 4) as i32;
                // } else {
                //     des_scroll = des_scroll / (*PTC::get_measure_width() * 4) as i32 * (*PTC::get_measure_width() * 4) as i32;
                // }

                // let old_scroll = *PTC::get_scroll();
                // *PTC::get_scroll() += des_scroll - old_scroll;
                winuser::InvalidateRect(
                    *PTC::get_hwnd(),
                    0 as *const winapi::shared::windef::RECT,
                    0,
                );
                // winuser::UpdateWindow(*PTC::get_hwnd());
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
