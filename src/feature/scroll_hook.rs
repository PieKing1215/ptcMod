use std::time::Instant;

use winapi::{shared::windef::HMENU, um::winuser};

use crate::{patch::Patch, ptc::PTCVersion, winutil};

use super::Feature;

lazy_static::lazy_static! {
    pub(crate) static ref M_SCROLL_HOOK_ID: u16 = winutil::next_id();
    pub(crate) static ref M_SMOOTH_SCROLL_ID: u16 = winutil::next_id();
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
    pub fn new<PTC: PTCVersion>(unit_clear_hook_patch: Patch) -> Self {
        Self { patch: vec![unit_clear_hook_patch] }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for Scroll {
    fn init(&mut self, menu: HMENU) {
        winutil::add_menu_toggle(menu, "Scroll Hook", *M_SCROLL_HOOK_ID, false, true);
        winutil::add_menu_toggle(menu, "Smooth Scroll", *M_SMOOTH_SCROLL_ID, false, false);
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
                    if winutil::menu_toggle(msg.hwnd, *M_SCROLL_HOOK_ID) {
                        for p in &self.patch {
                            unsafe { p.apply() }.unwrap();
                        }

                        winutil::set_menu_enabled(*PTC::get_hwnd(), *M_SMOOTH_SCROLL_ID, true);

                        unsafe {
                            ENABLED = true;

                            winuser::InvalidateRect(*PTC::get_hwnd(), std::ptr::null(), 0);
                        }
                    } else {
                        for p in &self.patch {
                            unsafe { p.unapply() }.unwrap();
                        }

                        winutil::set_menu_enabled(*PTC::get_hwnd(), *M_SMOOTH_SCROLL_ID, false);
                        unsafe {
                            ENABLED = false;
                        }
                    }
                } else if low == *M_SMOOTH_SCROLL_ID {
                    winutil::menu_toggle(msg.hwnd, *M_SMOOTH_SCROLL_ID);
                }
            }
        }
    }
}

pub(crate) unsafe fn unit_clear<PTC: PTCVersion>() {
    if PTC::is_playing() && *PTC::get_tab() > 0 {
        {
            let smooth = winutil::get_menu_checked(*PTC::get_hwnd(), *M_SMOOTH_SCROLL_ID);

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
