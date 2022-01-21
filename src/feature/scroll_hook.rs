use std::time::Instant;

use winapi::{shared::windef::HMENU, um::winuser};

use crate::{
    patch::Patch,
    ptc::{addr, PTCVersion},
    runtime::{menu_toggle, next_id},
};

use super::Feature;

lazy_static::lazy_static! {
    pub(crate) static ref M_SCROLL_HOOK_ID: u16 = next_id();
    pub(crate) static ref M_SMOOTH_SCROLL_ID: u16 = next_id();
}

pub(crate) static mut ENABLED: bool = false;

pub(crate) static mut LAST_PLAY_POS: u32 = 0;
pub(crate) static mut LAST_PLAY_POS_TIME: Option<Instant> = None;
pub(crate) static mut LAST_SCROLL: i32 = 0;
pub(crate) static mut LAST_PLAYHEAD_POS: i32 = 0;

pub struct Scroll {
    patch: Vec<Patch>,
}

impl Scroll {
    pub fn new<PTC: PTCVersion>(unit_clear_hook: unsafe extern "stdcall" fn()) -> Self {
        let old_bytes = i32::to_le_bytes(0x16440 - (0x165e8 + 0x5));

        let new_bytes = i32::to_le_bytes(
            (unit_clear_hook as *const () as i64 - (addr(0x165e8) + 0x5) as i64) as i32,
        );

        let clear_notes_hook_patch = Patch::new(
            0x165e8,
            vec![0xe8, old_bytes[0], old_bytes[1], old_bytes[2], old_bytes[3]],
            vec![0xe8, new_bytes[0], new_bytes[1], new_bytes[2], new_bytes[3]],
        )
        .unwrap();

        Self { patch: vec![clear_notes_hook_patch] }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for Scroll {
    fn init(&mut self, menu: HMENU) {
        unsafe {
            let l_title: Vec<u8> = "Scroll Hook\0".bytes().collect();
            winuser::AppendMenuA(
                menu,
                winuser::MF_CHECKED,
                *M_SCROLL_HOOK_ID as usize,
                l_title.as_ptr().cast::<i8>(),
            );

            winuser::CheckMenuItem(
                menu,
                *M_SCROLL_HOOK_ID as u32,
                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
            );

            let l_title: Vec<u8> = "Smooth Scroll\0".bytes().collect();
            winuser::AppendMenuA(
                menu,
                winuser::MF_CHECKED,
                *M_SMOOTH_SCROLL_ID as usize,
                l_title.as_ptr().cast::<i8>(),
            );

            winuser::CheckMenuItem(
                menu,
                *M_SMOOTH_SCROLL_ID as u32,
                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
            );

            winuser::EnableMenuItem(
                menu,
                *M_SMOOTH_SCROLL_ID as u32,
                winuser::MF_BYCOMMAND | winuser::MF_GRAYED,
            );
        }
    }

    fn cleanup(&mut self) {
        unsafe {
            for p in &self.patch {
                if let Err(e) = p.unapply() {
                    log::warn!("note_rect_hook_patch: {:?}", e);
                }
            }
        }
    }

    fn win_msg(&mut self, msg: &winapi::um::winuser::MSG) {
        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_SCROLL_HOOK_ID {
                    if menu_toggle(msg.hwnd, *M_SCROLL_HOOK_ID) {
                        for p in &self.patch {
                            unsafe { p.apply() }.unwrap();
                        }

                        unsafe {
                            winuser::EnableMenuItem(
                                winuser::GetMenu(msg.hwnd),
                                *M_SMOOTH_SCROLL_ID as u32,
                                winuser::MF_BYCOMMAND | winuser::MF_ENABLED,
                            );

                            ENABLED = true;
                            
                            winuser::InvalidateRect(*PTC::get_hwnd(), std::ptr::null(), 0);
                        }
                    } else {
                        for p in &self.patch {
                            unsafe { p.unapply() }.unwrap();
                        }

                        unsafe {
                            winuser::EnableMenuItem(
                                winuser::GetMenu(msg.hwnd),
                                *M_SMOOTH_SCROLL_ID as u32,
                                winuser::MF_BYCOMMAND | winuser::MF_GRAYED,
                            );

                            ENABLED = false;
                        }
                    }
                } else if low == *M_SMOOTH_SCROLL_ID {
                    menu_toggle(msg.hwnd, *M_SMOOTH_SCROLL_ID);
                }
            }
        }
    }
}

pub(crate) unsafe fn unit_clear<PTC: PTCVersion>() {
    // println!("draw_unitkb_top called");

    if PTC::is_playing() {
        {
            let smooth = winuser::GetMenuState(
                winuser::GetMenu(*PTC::get_hwnd()),
                (*M_SMOOTH_SCROLL_ID).try_into().unwrap(),
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

            *PTC::get_scroll() = des_scroll.clamp(0, PTC::get_scroll_max());
        }

        let unit_rect = PTC::get_unit_rect();

        let x = unit_rect[0] - *PTC::get_scroll() + LAST_SCROLL;
        LAST_PLAYHEAD_POS = x;

        winuser::InvalidateRect(*PTC::get_hwnd(), std::ptr::null(), 0);
    }
}
