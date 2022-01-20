use winapi::um::winuser;

use crate::{ptc::{PTCVersion, addr}, patch::Patch, runtime::{next_id, menu_toggle}};

use super::Feature;

lazy_static::lazy_static! {
    static ref M_CUSTOM_RENDERING_ENABLED_ID: u16 = next_id();
}

pub struct CustomNoteRendering {
    note_draw_patch: Vec<Patch>,
}

impl CustomNoteRendering {
    pub fn new<PTC: PTCVersion>() -> Self {
        let old_bytes = i32::to_le_bytes(0x1c0e0 - (0x1469f + 0x5));
        
        let new_bytes = i32::to_le_bytes(
            (PTC::get_hook_draw_unit_note_rect() as *const () as i64 - (addr(0x1469f) + 0x5) as i64) as i32,
        );

        let note_rect_hook_patch = Patch::new(0x1469f, 
            vec![0xe8, old_bytes[0], old_bytes[1], old_bytes[2], old_bytes[3]],
            vec![0xe8, new_bytes[0], new_bytes[1], new_bytes[2], new_bytes[3]]).unwrap();

        let note_rect_push_ebp = Patch::new(0x1469a,
            vec![0x52],
            vec![0x55]).unwrap();

        let note_disable_left_edge = Patch::new(0x146b8,
            vec![0x03],
            vec![0x00]).unwrap();

        let note_disable_right_edge = Patch::new(0x146e9,
            vec![0x03],
            vec![0x00]).unwrap();

        Self {
            note_draw_patch: vec![
                note_rect_push_ebp, 
                note_rect_hook_patch,
                note_disable_left_edge,
                note_disable_right_edge,
            ],
        }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for CustomNoteRendering {
    fn init(&mut self) {
        unsafe {
            let h_menu = winuser::GetMenu(*PTC::get_hwnd());
            let base = winuser::CreateMenu();
            let l_title: Vec<u8> = "Rendering\0".bytes().collect();
            winuser::AppendMenuA(
                h_menu,
                winuser::MF_POPUP,
                base as usize,
                l_title.as_ptr().cast::<i8>(),
            );

            let l_title: Vec<u8> = "Enabled\0".bytes().collect();
            winuser::AppendMenuA(
                base,
                winuser::MF_CHECKED,
                *M_CUSTOM_RENDERING_ENABLED_ID as usize,
                l_title.as_ptr().cast::<i8>(),
            );

            winuser::CheckMenuItem(
                base,
                *M_CUSTOM_RENDERING_ENABLED_ID as u32,
                winuser::MF_BYCOMMAND | winuser::MF_UNCHECKED,
            );
        }
    }

    fn cleanup(&mut self) {
        unsafe {
            winuser::RemoveMenu(winuser::GetMenu(*PTC::get_hwnd()), 4, winuser::MF_BYPOSITION);

            for p in &self.note_draw_patch {
                if let Err(e) = p.unapply() {
                    log::warn!("note_rect_hook_patch: {:?}", e);
                }
            }
        }
    }

    fn win_msg(&mut self, msg: &winuser::MSG) -> bool {
        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_CUSTOM_RENDERING_ENABLED_ID {
                    if menu_toggle(msg.hwnd, *M_CUSTOM_RENDERING_ENABLED_ID) {
                        for p in &self.note_draw_patch {
                            unsafe { p.apply() }.unwrap();
                        }
                    } else {
                        for p in &self.note_draw_patch {
                            unsafe { p.unapply() }.unwrap();
                        }
                    }

                    return true;
                }
            }
        }

        false
    }
    
}