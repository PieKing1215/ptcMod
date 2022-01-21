use winapi::{shared::windef::HMENU, um::winuser};

use crate::{
    feature::scroll_mod,
    patch::Patch,
    ptc::{addr, PTCVersion},
    runtime::{menu_toggle, next_id},
};

use super::Feature;

lazy_static::lazy_static! {
    static ref M_PLAYHEAD_ID: u16 = next_id();
}

pub struct Playhead {
    patch: Vec<Patch>,
}

impl Playhead {
    pub fn new<PTC: PTCVersion>(draw_unitkb_top_hook: unsafe extern "stdcall" fn()) -> Self {
        let old_bytes = i32::to_le_bytes(0x9f80 - (0x166c0 + 0x5));

        let new_bytes = i32::to_le_bytes(
            (draw_unitkb_top_hook as *const () as i64 - (addr(0x166c0) + 0x5) as i64) as i32,
        );

        let draw_unitkb_top_patch = Patch::new(
            0x166c0,
            vec![0xe8, old_bytes[0], old_bytes[1], old_bytes[2], old_bytes[3]],
            vec![0xe8, new_bytes[0], new_bytes[1], new_bytes[2], new_bytes[3]],
        )
        .unwrap();

        Self { patch: vec![draw_unitkb_top_patch] }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for Playhead {
    fn init(&mut self, menu: HMENU) {
        unsafe {
            let l_title: Vec<u8> = "Playhead\0".bytes().collect();
            winuser::AppendMenuA(
                menu,
                winuser::MF_CHECKED,
                *M_PLAYHEAD_ID as usize,
                l_title.as_ptr().cast::<i8>(),
            );

            winuser::CheckMenuItem(
                menu,
                *M_PLAYHEAD_ID as u32,
                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
            );

            winuser::EnableMenuItem(
                menu,
                *M_PLAYHEAD_ID as u32,
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

            // set wait time to tick time (otherwise it waits for it to catch up)
            *(addr(0xa6fa4) as *mut u32) = *(addr(0xa6fa0) as *mut u32);
        }
    }

    fn win_msg(&mut self, msg: &winapi::um::winuser::MSG) {
        println!("{} {} {}", msg.message, msg.wParam, msg.lParam);
        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_PLAYHEAD_ID {
                    if menu_toggle(msg.hwnd, *M_PLAYHEAD_ID) {
                        for p in &self.patch {
                            unsafe { p.apply() }.unwrap();
                        }
                    } else {
                        for p in &self.patch {
                            unsafe { p.unapply() }.unwrap();
                        }
                    }
                } else if low == *scroll_mod::M_SCROLL_HOOK_ID {
                    unsafe {
                        let enabled = winuser::GetMenuState(
                            winuser::GetMenu(*PTC::get_hwnd()),
                            (*scroll_mod::M_SCROLL_HOOK_ID).try_into().unwrap(),
                            winuser::MF_BYCOMMAND,
                        ) & winuser::MF_CHECKED
                            > 0;

                        winuser::EnableMenuItem(
                            winuser::GetMenu(*PTC::get_hwnd()),
                            *M_PLAYHEAD_ID as u32,
                            winuser::MF_BYCOMMAND
                                | if enabled {
                                    winuser::MF_ENABLED
                                } else {
                                    winuser::MF_GRAYED
                                },
                        );
                    }
                }
            }
        }
    }
}

pub(crate) unsafe fn draw_unitkb_top<PTC: PTCVersion>() {
    // println!("draw_unitkb_top called");

    if scroll_mod::ENABLED && PTC::is_playing() {
        let unit_rect = PTC::get_unit_rect();

        let x = crate::feature::scroll_mod::LAST_PLAYHEAD_POS;

        let rect = [x, unit_rect[1], x + 2, unit_rect[3]];
        let draw_rect: unsafe extern "cdecl" fn(rect: *const libc::c_int, color: libc::c_uint) =
            std::mem::transmute(addr(0x1c0e0) as *const ());
        (draw_rect)(rect.as_ptr(), 0xcccccc);
    }
}
