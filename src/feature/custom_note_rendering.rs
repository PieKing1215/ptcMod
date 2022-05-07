use colorsys::ColorTransform;
use winapi::um::winuser;

use crate::{
    patch::Patch,
    ptc::{
        addr,
        drawing::{color::Color, ddraw, Draw, Rect},
        events::EventType,
        PTCVersion,
    },
    winutil::{self, Menus},
};

use super::{scroll_hook, Feature};

lazy_static::lazy_static! {
    static ref M_CUSTOM_RENDERING_ENABLED_ID: u16 = winutil::next_id();
    static ref M_NOTE_PULSE_ID: u16 = winutil::next_id();
    static ref M_VOLUME_FADE_ID: u16 = winutil::next_id();
    static ref M_COLORED_UNITS_ID: u16 = winutil::next_id();
}

// store our own values instead since calling winapi in the draw loop would be slow
static mut NOTE_PULSE: bool = true;
static mut VOLUME_FADE: bool = true;
static mut COLORED_UNITS: bool = true;

pub struct CustomNoteRendering {
    draw_unit_notes_patch: Patch,
}

impl CustomNoteRendering {
    pub fn new<PTC: PTCVersion>(draw_unit_notes_patch: Patch) -> Self {
        Self { draw_unit_notes_patch }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for CustomNoteRendering {
    fn init(&mut self, menus: &mut Menus) {
        unsafe {
            let menu = menus.get_or_create::<PTC>("Rendering");

            winutil::add_menu_toggle(
                menu,
                "Note Render Hook",
                *M_CUSTOM_RENDERING_ENABLED_ID,
                false,
                true,
            );
            winutil::add_menu_toggle(
                menu,
                "Colored Units",
                *M_COLORED_UNITS_ID,
                COLORED_UNITS,
                false,
            );
            winutil::add_menu_toggle(menu, "Volume Fade", *M_VOLUME_FADE_ID, VOLUME_FADE, false);
            winutil::add_menu_toggle(menu, "Note Pulse", *M_NOTE_PULSE_ID, NOTE_PULSE, false);
        }
    }

    fn cleanup(&mut self) {
        unsafe {
            if let Err(e) = self.draw_unit_notes_patch.unapply() {
                log::warn!("draw_unit_notes_patch: {:?}", e);
            }
        }
    }

    fn win_msg(&mut self, msg: &winuser::MSG) {
        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_CUSTOM_RENDERING_ENABLED_ID {
                    if winutil::menu_toggle(msg.hwnd, *M_CUSTOM_RENDERING_ENABLED_ID) {
                        unsafe { self.draw_unit_notes_patch.apply() }.unwrap();

                        unsafe {
                            winutil::set_menu_enabled(
                                msg.hwnd,
                                *M_NOTE_PULSE_ID,
                                scroll_hook::ENABLED,
                            );
                        }
                        winutil::set_menu_enabled(msg.hwnd, *M_VOLUME_FADE_ID, true);
                        winutil::set_menu_enabled(msg.hwnd, *M_COLORED_UNITS_ID, true);
                    } else {
                        unsafe { self.draw_unit_notes_patch.unapply() }.unwrap();

                        winutil::set_menu_enabled(msg.hwnd, *M_NOTE_PULSE_ID, false);
                        winutil::set_menu_enabled(msg.hwnd, *M_VOLUME_FADE_ID, false);
                        winutil::set_menu_enabled(msg.hwnd, *M_COLORED_UNITS_ID, false);
                    }
                } else if low == *M_NOTE_PULSE_ID {
                    unsafe {
                        NOTE_PULSE = winutil::menu_toggle(msg.hwnd, *M_NOTE_PULSE_ID);
                    }
                } else if low == *M_VOLUME_FADE_ID {
                    unsafe {
                        VOLUME_FADE = winutil::menu_toggle(msg.hwnd, *M_VOLUME_FADE_ID);
                    }
                } else if low == *M_COLORED_UNITS_ID {
                    unsafe {
                        COLORED_UNITS = winutil::menu_toggle(msg.hwnd, *M_COLORED_UNITS_ID);
                    }
                } else if low == *scroll_hook::M_SCROLL_HOOK_ID {
                    let scroll_hook_enabled =
                        winutil::get_menu_checked(*PTC::get_hwnd(), *scroll_hook::M_SCROLL_HOOK_ID);
                    let custom_rendering_enabled =
                        winutil::get_menu_checked(*PTC::get_hwnd(), *M_CUSTOM_RENDERING_ENABLED_ID);
                    winutil::set_menu_enabled(
                        msg.hwnd,
                        *M_NOTE_PULSE_ID,
                        scroll_hook_enabled && custom_rendering_enabled,
                    );
                }
            }
        }
    }
}

// complete replacement for the vanilla unit notes drawing function
// this allows for much easier modification
#[allow(clippy::too_many_lines)]
#[allow(clippy::field_reassign_with_default)]
pub(crate) unsafe fn draw_unit_notes<PTC: PTCVersion>() {
    let meas_width = PTC::get_measure_width();
    let ofs_x = PTC::get_unit_scroll_ofs_x();
    let ofs_y = PTC::get_unit_scroll_ofs_y();

    let bounds = &PTC::get_unit_rect();

    let beat_clock = PTC::get_beat_clock();
    let unit_num = PTC::get_unit_num();

    let unit_height = 16;

    let mut draw = ddraw::IDirectDrawSurface::wrap(*(addr(0xa7b28) as *mut *mut libc::c_void));

    for u in 0..unit_num {
        if u * unit_height + unit_height >= *ofs_y
            && u * unit_height < *ofs_y + (bounds[3] - bounds[1])
        {
            let y = bounds[1] + u * unit_height + unit_height / 2 - ofs_y;

            let events = PTC::get_events_for_unit(u);

            let dim = !PTC::is_unit_highlighted(u);

            let color = Color::from_argb(PTC::get_base_note_colors_argb()[if dim { 1 } else { 0 }]);

            // let mut batch_a = Vec::new();

            for eve in events {
                // should always be true, but vanilla checks it so we will too
                if i32::from(eve.unit) == u {
                    match eve.kind {
                        EventType::On => {
                            let x = (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x
                                + bounds[0];
                            let x2 = ((eve.clock + eve.value) * (*meas_width as i32)
                                / beat_clock as i32)
                                - ofs_x
                                + bounds[0];

                            let note_rect = Rect::<i32>::new(
                                (x + 2).max(bounds[0]),
                                (y - 2).max(bounds[1]),
                                (x2 - 2).min(bounds[2]),
                                (y + 2).min(bounds[3]),
                            );

                            draw.fill_rect(note_rect, color);
                            // batch_a.push(note_rect);

                            if x > bounds[0] - 2 {
                                draw.fill_rect(
                                    Rect::<i32>::new(
                                        note_rect.left - 1,
                                        note_rect.top - 1,
                                        note_rect.left,
                                        note_rect.bottom + 1,
                                    ),
                                    color,
                                );
                                // batch_a.push(Rect::<i32>::new(
                                //     note_rect.left - 1,
                                //     note_rect.top - 1,
                                //     note_rect.left,
                                //     note_rect.bottom + 1,
                                // ));

                                // left edge
                                draw.fill_rect(
                                    Rect::<i32>::new(
                                        note_rect.left - 1,
                                        note_rect.top - 1,
                                        note_rect.left,
                                        note_rect.bottom + 1,
                                    ),
                                    color,
                                );
                                // batch_a.push(Rect::<i32>::new(
                                //     note_rect.left - 1,
                                //     note_rect.top - 1,
                                //     note_rect.left,
                                //     note_rect.bottom + 1,
                                // ));

                                draw.fill_rect(
                                    Rect::<i32>::new(
                                        note_rect.left - 2,
                                        note_rect.top - 3,
                                        note_rect.left - 1,
                                        note_rect.bottom + 3,
                                    ),
                                    color,
                                );
                                // batch_a.push(Rect::<i32>::new(
                                //     note_rect.left - 2,
                                //     note_rect.top - 3,
                                //     note_rect.left - 1,
                                //     note_rect.bottom + 3,
                                // ));
                            }

                            if note_rect.right < bounds[2] {
                                // right edge
                                draw.fill_rect(
                                    Rect::<i32>::new(
                                        note_rect.right,
                                        note_rect.top,
                                        note_rect.right + 1,
                                        note_rect.bottom,
                                    ),
                                    color,
                                );
                                // batch_a.push(Rect::<i32>::new(
                                //     note_rect.right,
                                //     note_rect.top,
                                //     note_rect.right + 1,
                                //     note_rect.bottom,
                                // ));
                                draw.fill_rect(
                                    Rect::<i32>::new(
                                        note_rect.right + 1,
                                        note_rect.top + 1,
                                        note_rect.right + 2,
                                        note_rect.bottom - 1,
                                    ),
                                    color,
                                );
                                // batch_a.push(Rect::<i32>::new(
                                //     note_rect.right + 1,
                                //     note_rect.top + 1,
                                //     note_rect.right + 2,
                                //     note_rect.bottom - 1,
                                // ));
                            }
                        }
                        EventType::Velocity | EventType::Key => {}
                        _ => {
                            let x = (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x
                                + bounds[0];
                            if x > bounds[0] - 2 {
                                let color = Color::from_argb(
                                    [0xff00f080, 0x007840][if dim { 1 } else { 0 }],
                                );
                                draw.fill_rect(Rect::<i32>::new(x, y + 4, x + 2, y + 6), color);
                            }
                        }
                    }
                }
            }

            // draw.fill_rect_batch(batch_a, color);
        }
    }
}

// the second parameter here would normally be color, but an asm patch is used to change it to push the ebp register instead
//      which can be used to get the unit and focus state (which could be used to get the original color anyway)
#[allow(clippy::too_many_lines)] // TODO
pub(crate) unsafe fn draw_unit_note_rect<PTC: PTCVersion>(
    rect: *const libc::c_int,
    unit: u32,
    not_focused: bool,
) {
    // color = 0x0094FF;

    let color = PTC::get_base_note_colors_argb()[if not_focused { 1 } else { 0 }];
    let raw_argb = color.to_be_bytes();
    let mut rgb = colorsys::Rgb::from([raw_argb[1], raw_argb[2], raw_argb[3]]);

    if COLORED_UNITS {
        rgb.adjust_hue(unit as f64 * 25.0);
    }

    let rect = std::slice::from_raw_parts(rect, 4);

    if PTC::is_playing() && (NOTE_PULSE || VOLUME_FADE) {
        if scroll_hook::ENABLED && rect[0] <= scroll_hook::LAST_PLAYHEAD_POS {
            // left of note is to the left of the playhead

            // TODO: clean up this logic
            let flash_strength = if not_focused { 0.4 } else { 0.8 };
            if rect[2] >= scroll_hook::LAST_PLAYHEAD_POS {
                // right of note is to the right of the playhead (playhead is on the note)

                if NOTE_PULSE {
                    let mix = flash_strength as f64;
                    rgb.set_red(rgb.red() + (255.0 - rgb.red()) * mix);
                    rgb.set_green(rgb.green() + (255.0 - rgb.green()) * mix);
                    rgb.set_blue(rgb.blue() + (255.0 - rgb.blue()) * mix);
                }

                if VOLUME_FADE {
                    let volume: f32 = PTC::get_event_value_at_screen_pos(
                        scroll_hook::LAST_PLAYHEAD_POS,
                        unit as i32,
                        0x5,
                    ) as f32
                        / 104.0;
                    let velocity: f32 = PTC::get_event_value_at_screen_pos(
                        scroll_hook::LAST_PLAYHEAD_POS,
                        unit as i32,
                        0x5,
                    ) as f32
                        / 104.0;

                    let factor = volume * velocity;
                    let factor = factor.powf(0.25);

                    let fade_color: [u8; 4] = if not_focused {
                        0xff200040_u32
                    } else {
                        0xff400070
                    }
                    .to_be_bytes();
                    let mix = (1.0 - factor as f64 * 0.8 - 0.2).clamp(0.0, 1.0);
                    rgb.set_red(rgb.red() + (fade_color[1] as f64 - rgb.red()) * mix);
                    rgb.set_green(rgb.green() + (fade_color[2] as f64 - rgb.green()) * mix);
                    rgb.set_blue(rgb.blue() + (fade_color[3] as f64 - rgb.blue()) * mix);
                }
            } else {
                // right of note is to the left of the playhead (playhead is past the note)

                let fade_size = *PTC::get_measure_width() as i32 / 4;
                let fade_pt = scroll_hook::LAST_PLAYHEAD_POS - fade_size;

                if NOTE_PULSE && rect[2] >= fade_pt {
                    let thru = (rect[2] - fade_pt) as f32 / fade_size as f32;

                    let mix = thru as f64 * flash_strength as f64;
                    rgb.set_red(rgb.red() + (255.0 - rgb.red()) * mix);
                    rgb.set_green(rgb.green() + (255.0 - rgb.green()) * mix);
                    rgb.set_blue(rgb.blue() + (255.0 - rgb.blue()) * mix);
                }

                if VOLUME_FADE {
                    let volume: f32 = PTC::get_event_value_at_screen_pos(rect[2], unit as i32, 0x5)
                        as f32
                        / 104.0;
                    let velocity: f32 =
                        PTC::get_event_value_at_screen_pos(rect[2], unit as i32, 0x5) as f32
                            / 104.0;

                    let factor = volume * velocity;
                    let factor = factor.powf(0.25);

                    let fade_color: [u8; 4] = if not_focused {
                        0xff200040_u32
                    } else {
                        0xff400070
                    }
                    .to_be_bytes();
                    let mix = (1.0 - (factor as f64) * 0.8 - 0.2).clamp(0.0, 1.0);
                    rgb.set_red(rgb.red() + (fade_color[1] as f64 - rgb.red()) * mix);
                    rgb.set_green(rgb.green() + (fade_color[2] as f64 - rgb.green()) * mix);
                    rgb.set_blue(rgb.blue() + (fade_color[3] as f64 - rgb.blue()) * mix);
                }
            }
        } else if VOLUME_FADE {
            // left of note is to the right of the playhead (note not played yet)

            let fade_color: [u8; 4] = if not_focused {
                0xff200040_u32
            } else {
                0xff400070
            }
            .to_be_bytes();

            let volume: f32 =
                PTC::get_event_value_at_screen_pos(rect[0], unit as i32, 0x5) as f32 / 104.0;
            let velocity: f32 =
                PTC::get_event_value_at_screen_pos(rect[0], unit as i32, 0x5) as f32 / 104.0;

            let factor = volume * velocity;
            let factor = factor.powf(0.25);

            let mix = (1.0 - (factor as f64) * 0.8 - 0.2).clamp(0.0, 1.0);
            rgb.set_red(rgb.red() + (fade_color[1] as f64 - rgb.red()) * mix);
            rgb.set_green(rgb.green() + (fade_color[2] as f64 - rgb.green()) * mix);
            rgb.set_blue(rgb.blue() + (fade_color[3] as f64 - rgb.blue()) * mix);
        }
    }

    let rgb_arr: [u8; 3] = rgb.into();

    let color = u32::from_be_bytes([0xff, rgb_arr[0], rgb_arr[1], rgb_arr[2]]);

    // main
    PTC::draw_rect([rect[0], rect[1], rect[2], rect[3]], color);

    if rect[0] > PTC::get_unit_rect()[0] {
        // left edge
        PTC::draw_rect([rect[0] - 1, rect[1] - 1, rect[0], rect[3] + 1], color);
        PTC::draw_rect([rect[0] - 2, rect[1] - 3, rect[0] - 1, rect[3] + 3], color);
    }

    if rect[2] > PTC::get_unit_rect()[0] {
        // right edge
        PTC::draw_rect([rect[2], rect[1], rect[2] + 1, rect[3]], color);
        PTC::draw_rect([rect[2] + 1, rect[1] + 1, rect[2] + 2, rect[3] - 1], color);
    }

    // let get_event_value: unsafe extern "cdecl" fn(pos_x: i32, unit_no: i32, ev_type: i32) -> i32 =
    // std::mem::transmute(addr(0x8f80) as *const ());
    // for x in 0..600 {
    //     let volume = (get_event_value)(x, unit as i32, 0x5);
    //     (draw_rect)([x, 256 - volume, x + 1, 256].as_ptr(), 0xff0000);
    // }
}
