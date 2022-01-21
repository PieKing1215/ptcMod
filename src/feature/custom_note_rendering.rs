use colorsys::ColorTransform;
use winapi::{shared::windef::HMENU, um::winuser};

use crate::{
    patch::Patch,
    ptc::{addr, PTCVersion},
    runtime::{menu_toggle, next_id},
};

use super::{Feature, custom_scroll};

lazy_static::lazy_static! {
    static ref M_CUSTOM_RENDERING_ENABLED_ID: u16 = next_id();
}

pub struct CustomNoteRendering {
    note_draw_patch: Vec<Patch>,
}

impl CustomNoteRendering {
    pub fn new<PTC: PTCVersion>(
        draw_unit_note_rect_hook: unsafe extern "cdecl" fn(
            rect: *const libc::c_int,
            color: libc::c_uint,
        ),
    ) -> Self {
        let old_bytes = i32::to_le_bytes(0x1c0e0 - (0x1469f + 0x5));

        let new_bytes = i32::to_le_bytes(
            (draw_unit_note_rect_hook as *const () as i64 - (addr(0x1469f) + 0x5) as i64) as i32,
        );

        let note_rect_hook_patch = Patch::new(
            0x1469f,
            vec![0xe8, old_bytes[0], old_bytes[1], old_bytes[2], old_bytes[3]],
            vec![0xe8, new_bytes[0], new_bytes[1], new_bytes[2], new_bytes[3]],
        )
        .unwrap();

        let note_rect_push_ebp = Patch::new(0x1469a, vec![0x52], vec![0x55]).unwrap();

        let note_disable_left_edge = Patch::new(0x146b8, vec![0x03], vec![0x00]).unwrap();

        let note_disable_right_edge = Patch::new(0x146e9, vec![0x03], vec![0x00]).unwrap();

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
    fn init(&mut self, _menu: HMENU) {
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
            winuser::RemoveMenu(
                winuser::GetMenu(*PTC::get_hwnd()),
                4,
                winuser::MF_BYPOSITION,
            );

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

// the second parameter here would normally be color, but an asm patch is used to change it to push the ebp register instead
//      which can be used to get the unit and focus state (which could be used to get the original color anyway)
#[allow(clippy::too_many_lines)] // TODO
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
        if custom_scroll::ENABLED && rect[0] <= custom_scroll::LAST_PLAYHEAD_POS {
            // TODO: clean up this logic
            let flash_strength = if not_focused { 0.5 } else { 0.95 };
            if rect[2] >= custom_scroll::LAST_PLAYHEAD_POS {
                let get_event_value: unsafe extern "cdecl" fn(
                    pos_x: i32,
                    unit_no: i32,
                    ev_type: i32,
                ) -> i32 = std::mem::transmute(addr(0x8f80) as *const ());

                let volume: f32 =
                    (get_event_value)(custom_scroll::LAST_PLAYHEAD_POS, unit as i32, 0x5)
                        as f32
                        / 128.0;
                let velocity: f32 =
                    (get_event_value)(custom_scroll::LAST_PLAYHEAD_POS, unit as i32, 0x5)
                        as f32
                        / 128.0;

                let factor = volume * velocity;
                let factor = factor.powf(0.25);

                let mix = flash_strength as f64;
                rgb.set_red(rgb.red() + (255.0 - rgb.red()) * mix);
                rgb.set_green(rgb.green() + (255.0 - rgb.green()) * mix);
                rgb.set_blue(rgb.blue() + (255.0 - rgb.blue()) * mix);

                let fade_color: [u8; 4] = if not_focused {
                    0xff200040_u32
                } else {
                    0xff400070
                }
                .to_be_bytes();
                let mix = 1.0 - factor as f64 * 0.8;
                rgb.set_red(rgb.red() + (fade_color[1] as f64 - rgb.red()) * mix);
                rgb.set_green(rgb.green() + (fade_color[2] as f64 - rgb.green()) * mix);
                rgb.set_blue(rgb.blue() + (fade_color[3] as f64 - rgb.blue()) * mix);
            } else {
                let fade_size = *PTC::get_measure_width() as i32 / 4;
                let fade_pt = custom_scroll::LAST_PLAYHEAD_POS - fade_size;

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
                    0xff200040_u32
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
                0xff200040_u32
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
