use std::{
    collections::VecDeque,
    sync::{LazyLock, RwLock},
};

use colorsys::ColorTransform;
use windows::Win32::{
    Foundation::RECT,
    Graphics::{
        DirectDraw::{DDBLTFAST_SRCCOLORKEY, IDirectDrawSurface},
        Gdi::InvalidateRect,
    },
    UI::WindowsAndMessaging::{MSG, WM_COMMAND},
};
use windows_core::Interface;

use crate::{
    patch::Patch,
    ptc::{
        PTCVersion, addr,
        drawing::{Draw, Rect, color::Color, ddraw},
        events::{Event, EventType},
    },
    winutil::{self, Menus, hiword, loword},
};

use super::{Feature, scroll_hook};

static M_CUSTOM_RENDERING_ENABLED_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);
static M_NOTE_PULSE_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);
static M_VOLUME_FADE_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);
static M_COLORED_UNITS_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);
static M_SEPARATE_SURFACE_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);
static M_KEY_NOTCHES_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);
static M_PORTA_PREVIEW_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);

// store our own values instead since calling winapi in the draw loop would be slow
static mut NOTE_PULSE: bool = true;
static mut VOLUME_FADE: bool = true;
static mut COLORED_UNITS: bool = true;
/// can perform better in some scenarios<br/>
/// eg. true was better at the time I added it on windows<br/>
/// but false is better for me on wine
static mut USE_SEPARATE_SURFACE: bool = false;
static mut KEY_NOTCHES: bool = true;
static mut PORTA_VIEW: bool = true;

// static mut DCRT: Option<&'static mut ID2D1DCRenderTarget> = None;
static SURF: RwLock<Option<(Rect<i32>, ForceSendSync<IDirectDrawSurface>)>> = RwLock::new(None);
static SURF_KB: RwLock<Option<(Rect<i32>, ForceSendSync<IDirectDrawSurface>)>> = RwLock::new(None);

struct ForceSendSync<T>(T);

unsafe impl<T> Send for ForceSendSync<T> {}
unsafe impl<T> Sync for ForceSendSync<T> {}

pub struct CustomNoteRendering {
    draw_unit_notes_patch: Patch,
    draw_kb_notes_patch: Patch,
}

impl CustomNoteRendering {
    pub fn new<PTC: PTCVersion>(draw_unit_notes_patch: Patch, draw_kb_notes_patch: Patch) -> Self {
        Self { draw_unit_notes_patch, draw_kb_notes_patch }
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
                "Use Separate Surface",
                *M_SEPARATE_SURFACE_ID,
                USE_SEPARATE_SURFACE,
                false,
            );
            winutil::add_menu_toggle(menu, "Key Notches", *M_KEY_NOTCHES_ID, KEY_NOTCHES, false);
            winutil::add_menu_toggle(menu, "Porta View", *M_PORTA_PREVIEW_ID, PORTA_VIEW, false);
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
                log::warn!("draw_unit_notes_patch: {e:?}");
            }

            if let Err(e) = self.draw_kb_notes_patch.unapply() {
                log::warn!("draw_kb_notes_patch: {e:?}");
            }

            let draw =
                IDirectDrawSurface::from_raw_borrowed(&*(addr(0xa7b28) as *mut *mut libc::c_void))
                    .unwrap();
            if let Some((_size, ForceSendSync(surf))) = SURF.try_write().unwrap().take() {
                let _ = draw.DeleteAttachedSurface(0, &surf);
            }
            if let Some((_size, ForceSendSync(surf))) = SURF_KB.try_write().unwrap().take() {
                let _ = draw.DeleteAttachedSurface(0, &surf);
            }
        }
    }

    fn win_msg(&mut self, msg: &MSG) {
        if msg.message == WM_COMMAND {
            let high = hiword(msg.wParam.0.try_into().unwrap());
            let low = loword(msg.wParam.0.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_CUSTOM_RENDERING_ENABLED_ID {
                    if winutil::menu_toggle(msg.hwnd, *M_CUSTOM_RENDERING_ENABLED_ID) {
                        unsafe { self.draw_unit_notes_patch.apply() }.unwrap();
                        unsafe { self.draw_kb_notes_patch.apply() }.unwrap();

                        unsafe {
                            winutil::set_menu_enabled(
                                msg.hwnd,
                                *M_NOTE_PULSE_ID,
                                scroll_hook::ENABLED,
                            );
                        }
                        winutil::set_menu_enabled(msg.hwnd, *M_VOLUME_FADE_ID, true);
                        winutil::set_menu_enabled(msg.hwnd, *M_COLORED_UNITS_ID, true);
                        winutil::set_menu_enabled(msg.hwnd, *M_SEPARATE_SURFACE_ID, true);
                        winutil::set_menu_enabled(msg.hwnd, *M_KEY_NOTCHES_ID, true);
                        winutil::set_menu_enabled(msg.hwnd, *M_PORTA_PREVIEW_ID, true);
                    } else {
                        unsafe { self.draw_unit_notes_patch.unapply() }.unwrap();
                        unsafe { self.draw_kb_notes_patch.unapply() }.unwrap();

                        winutil::set_menu_enabled(msg.hwnd, *M_NOTE_PULSE_ID, false);
                        winutil::set_menu_enabled(msg.hwnd, *M_VOLUME_FADE_ID, false);
                        winutil::set_menu_enabled(msg.hwnd, *M_COLORED_UNITS_ID, false);
                        winutil::set_menu_enabled(msg.hwnd, *M_SEPARATE_SURFACE_ID, false);
                        winutil::set_menu_enabled(msg.hwnd, *M_KEY_NOTCHES_ID, false);
                        winutil::set_menu_enabled(msg.hwnd, *M_PORTA_PREVIEW_ID, false);
                    }
                    unsafe {
                        InvalidateRect(Some(*PTC::get_hwnd()), None, false).unwrap();
                    }
                } else if low == *M_NOTE_PULSE_ID {
                    unsafe {
                        NOTE_PULSE = winutil::menu_toggle(msg.hwnd, *M_NOTE_PULSE_ID);
                        InvalidateRect(Some(*PTC::get_hwnd()), None, false).unwrap();
                    }
                } else if low == *M_VOLUME_FADE_ID {
                    unsafe {
                        VOLUME_FADE = winutil::menu_toggle(msg.hwnd, *M_VOLUME_FADE_ID);
                        InvalidateRect(Some(*PTC::get_hwnd()), None, false).unwrap();
                    }
                } else if low == *M_COLORED_UNITS_ID {
                    unsafe {
                        COLORED_UNITS = winutil::menu_toggle(msg.hwnd, *M_COLORED_UNITS_ID);
                        InvalidateRect(Some(*PTC::get_hwnd()), None, false).unwrap();
                    }
                } else if low == *M_SEPARATE_SURFACE_ID {
                    unsafe {
                        USE_SEPARATE_SURFACE =
                            winutil::menu_toggle(msg.hwnd, *M_SEPARATE_SURFACE_ID);
                        InvalidateRect(Some(*PTC::get_hwnd()), None, false).unwrap();
                    }
                } else if low == *M_KEY_NOTCHES_ID {
                    unsafe {
                        KEY_NOTCHES = winutil::menu_toggle(msg.hwnd, *M_KEY_NOTCHES_ID);
                        InvalidateRect(Some(*PTC::get_hwnd()), None, false).unwrap();
                    }
                } else if low == *M_PORTA_PREVIEW_ID {
                    unsafe {
                        PORTA_VIEW = winutil::menu_toggle(msg.hwnd, *M_PORTA_PREVIEW_ID);
                        InvalidateRect(Some(*PTC::get_hwnd()), None, false).unwrap();
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
                    unsafe {
                        InvalidateRect(Some(*PTC::get_hwnd()), None, false).unwrap();
                    }
                }
            }
        }
    }
}

pub unsafe fn get_value_at(
    now_pos: i32,
    kind: EventType,
    unit: i32,
    start: *mut Event,
    initial: i32,
) -> i32 {
    let mut val = initial;
    let mut eve_raw = start;
    while !eve_raw.is_null() {
        let eve = unsafe { &mut *eve_raw };

        if eve.clock > now_pos {
            break;
        }

        if eve.unit == unit as u8 && eve.kind == kind {
            val = eve.value;
        }

        eve_raw = eve.next;
    }

    val
}

pub unsafe fn get_event_at(
    now_pos: i32,
    kind: EventType,
    unit: i32,
    start: *mut Event,
) -> Option<&'static Event> {
    let mut val = None;
    let mut eve_raw = start;
    while !eve_raw.is_null() {
        let eve = unsafe { &mut *eve_raw };

        if eve.clock > now_pos {
            break;
        }

        if eve.unit == unit as u8 && eve.kind == kind {
            val = Some(&*eve);
        }

        eve_raw = eve.next;
    }

    val
}

pub unsafe fn get_next_event(
    cur_pos: i32,
    max_pos: i32,
    kind: EventType,
    unit: i32,
    start: *mut Event,
) -> Option<&'static Event> {
    let mut eve_raw = start;
    while !eve_raw.is_null() {
        let eve = unsafe { &mut *eve_raw };

        if eve.clock > max_pos {
            break;
        }

        if eve.clock > cur_pos && eve.unit == unit as u8 && eve.kind == kind {
            return Some(&*eve);
        }

        eve_raw = eve.next;
    }

    None
}

// complete replacement for the vanilla unit notes drawing function
// this allows for much easier modification
#[allow(clippy::too_many_lines)]
#[allow(clippy::field_reassign_with_default)]
pub(crate) unsafe fn draw_unit_notes<PTC: PTCVersion>() {
    let meas_width = PTC::get_measure_width();
    let ofs_x = PTC::get_unit_scroll_ofs_x();
    let ofs_y = PTC::get_unit_scroll_ofs_y();

    let unit_area = PTC::get_unit_rect();
    let bounds = Rect::<i32>::new(0, 0, unit_area.width(), unit_area.height());

    if bounds.width() <= 0 || bounds.height() <= 0 {
        return;
    }

    let beat_clock = PTC::get_beat_clock();
    let unit_num = PTC::get_unit_num();

    let unit_height = 16;

    let real_draw = unsafe {
        IDirectDrawSurface::from_raw_borrowed(&*(addr(0xa7b28) as *mut *mut libc::c_void)).unwrap()
    };

    let mut surf = SURF.try_write().unwrap();
    if let Some((surf_size, _surf)) = surf.as_ref() {
        if *surf_size != unit_area {
            log::debug!("Unit area resized");
            let _ = unsafe { real_draw.DeleteAttachedSurface(0, &surf.take().unwrap().1.0) };
            *surf = Some((
                unit_area,
                ForceSendSync(unsafe {
                    ddraw::create_surface(
                        *(addr(0xa7b20) as *mut *mut libc::c_void),
                        unit_area.width(),
                        unit_area.height(),
                    )
                }),
            ));
        }
    } else {
        log::debug!("Creating unit area surface");
        *surf = Some((
            unit_area,
            ForceSendSync(unsafe {
                ddraw::create_surface(
                    *(addr(0xa7b20) as *mut *mut libc::c_void),
                    unit_area.width(),
                    unit_area.height(),
                )
            }),
        ));
    }

    let use_separate_surface = unsafe { USE_SEPARATE_SURFACE };
    let colored_units = unsafe { COLORED_UNITS };
    let note_pulse = unsafe { NOTE_PULSE };
    let volume_fade = unsafe { VOLUME_FADE };
    let key_notches = unsafe { KEY_NOTCHES };

    let draw = if use_separate_surface {
        &surf.as_mut().unwrap().1.0
    } else {
        real_draw
    };
    let draw = draw.offset(
        if use_separate_surface {
            0
        } else {
            unit_area.left
        },
        if use_separate_surface {
            0
        } else {
            unit_area.top
        },
    );

    let colors = PTC::get_base_note_colors_argb().map(|c| Color::from_argb(c).with_a(1.0));

    let highlighted = (0..unit_num)
        .into_iter()
        .map(|u| PTC::is_unit_highlighted(u))
        .collect::<Vec<_>>();
    let mut cur_volume = (0..unit_num).into_iter().map(|_u| 104).collect::<Vec<_>>();
    let mut cur_velocity = (0..unit_num).into_iter().map(|_u| 104).collect::<Vec<_>>();

    let events_list = PTC::get_event_list();

    let mut batch_a: Vec<(Rect<i32>, Color)> = Vec::new();

    // TODO: this is stupid
    let do_batching = false;

    if use_separate_surface {
        unsafe { draw.fill_rect(&bounds, Color::from_argb(0xff000000)) };
    }

    let mut count = 0;
    let mut culled = 0;

    let mut eve_raw = events_list.start;
    while !eve_raw.is_null() {
        let eve = unsafe { &mut *eve_raw };

        let x = (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x + bounds.left;

        if x > bounds.right {
            break;
        }

        let u = eve.unit as i32;

        let dim = !highlighted[u as usize];
        let y = bounds.top + u * unit_height + unit_height / 2 - ofs_y;

        // y is the center of the unit so this culls when the center goes offscreen
        if y < bounds.top || y > bounds.bottom {
            culled += 1;
            eve_raw = eve.next;
            continue;
        }

        match eve.kind {
            EventType::Volume => {
                cur_volume[u as usize] = eve.value;
            },
            EventType::Velocity => {
                cur_velocity[u as usize] = eve.value;
            },
            EventType::On => {
                #[allow(clippy::bool_to_int_with_if)]
                let mut color = colors[if dim { 1 } else { 0 }];
                #[allow(clippy::bool_to_int_with_if)]
                let mut highlight_color = colors[if dim { 1 } else { 0 }];

                if colored_units {
                    color = color.rotate_hue(u as f64 * 25.0);
                }

                let x =
                    (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x + bounds.left;
                let x2 = ((eve.clock + eve.value) * (*meas_width as i32) / beat_clock as i32)
                    - ofs_x
                    + bounds.left;

                let note_rect = Rect::<i32>::new(
                    (x + 2).max(bounds.left),
                    (y - 2).max(bounds.top),
                    (x2 - 1).min(bounds.right),
                    (y + 2).min(bounds.bottom),
                );

                if note_rect.right < bounds.left || note_rect.left > bounds.right + 1 {
                    culled += 1;
                    eve_raw = eve.next;
                    continue;
                }

                let mut highlight_rect = None;
                if PTC::is_playing() && (note_pulse || volume_fade) {
                    let last_playhead_pos = unsafe { scroll_hook::LAST_PLAYHEAD_POS };
                    if unsafe { scroll_hook::ENABLED } && x + unit_area.left <= last_playhead_pos {
                        // left of note is to the left of the playhead

                        // TODO: clean up this logic
                        let flash_strength = if dim { 0.4 } else { 0.8 };
                        if x2 + unit_area.left >= last_playhead_pos {
                            // right of note is to the right of the playhead (playhead is on the note)

                            if note_pulse {
                                let clock = (ofs_x + last_playhead_pos - unit_area.left)
                                    * beat_clock as i32
                                    / *meas_width as i32;

                                highlight_color = color.blend(Color::WHITE, flash_strength);
                                color = color.blend(Color::WHITE, flash_strength * 0.75);

                                let prev_eve_key_clock = unsafe {
                                    get_event_at(clock, EventType::Key, u, eve_raw)
                                        .map_or(eve.clock, |key| key.clock)
                                };

                                let next_eve_key_clock = unsafe {
                                    get_next_event(
                                        clock,
                                        eve.clock + eve.value,
                                        EventType::Key,
                                        u,
                                        eve_raw,
                                    )
                                    .map_or(eve.clock + eve.value, |key| key.clock)
                                };

                                let x = (prev_eve_key_clock * (*meas_width as i32)
                                    / beat_clock as i32)
                                    - ofs_x
                                    + bounds.left;
                                let x2 = (next_eve_key_clock * (*meas_width as i32)
                                    / beat_clock as i32)
                                    - ofs_x
                                    + bounds.left;

                                let note_rect = Rect::<i32>::new(
                                    (x).max(bounds.left).max(note_rect.left),
                                    (y - 2).max(bounds.top),
                                    (x2 - 1).min(bounds.right),
                                    (y + 2).min(bounds.bottom),
                                );

                                highlight_rect = Some(note_rect);
                            }

                            if volume_fade {
                                let clock = (ofs_x + last_playhead_pos - unit_area.left)
                                    * beat_clock as i32
                                    / *meas_width as i32;
                                let volume: f32 = unsafe {
                                    get_value_at(
                                        clock,
                                        EventType::Volume,
                                        u,
                                        eve_raw,
                                        cur_volume[u as usize],
                                    )
                                } as f32
                                    / 104.0;
                                let velocity: f32 = unsafe {
                                    get_value_at(
                                        clock,
                                        EventType::Velocity,
                                        u,
                                        eve_raw,
                                        cur_velocity[u as usize],
                                    )
                                } as f32
                                    / 104.0;

                                let factor = volume * velocity;
                                let factor = factor.powf(0.25);

                                let fade_color = if dim {
                                    Color::from_argb(0xff200040)
                                } else {
                                    Color::from_argb(0xff400070)
                                };
                                let mix = (1.0 - factor * 0.8 - 0.2).clamp(0.0, 1.0);
                                color = color.blend(fade_color, mix);
                                highlight_color = highlight_color.blend(fade_color, mix);
                            }
                        } else {
                            // right of note is to the left of the playhead (playhead is past the note)

                            let fade_size = *PTC::get_measure_width() as i32 / 4;
                            let fade_pt = last_playhead_pos - fade_size;

                            if note_pulse && x2 + unit_area.left >= fade_pt {
                                let thru =
                                    (x2 + unit_area.left - fade_pt) as f32 / fade_size as f32;

                                color = color.blend(Color::WHITE, thru * flash_strength * 0.75);
                            }

                            if volume_fade {
                                let clock = (ofs_x + x2) * beat_clock as i32 / *meas_width as i32;
                                let volume: f32 = unsafe {
                                    get_value_at(
                                        clock - 1,
                                        EventType::Volume,
                                        u,
                                        eve_raw,
                                        cur_volume[u as usize],
                                    )
                                } as f32
                                    / 104.0;
                                let velocity: f32 = unsafe {
                                    get_value_at(
                                        clock - 1,
                                        EventType::Velocity,
                                        u,
                                        eve_raw,
                                        cur_velocity[u as usize],
                                    )
                                } as f32
                                    / 104.0;

                                let factor = volume * velocity;
                                let factor = factor.powf(0.25);

                                let fade_color = if dim {
                                    Color::from_argb(0xff200040)
                                } else {
                                    Color::from_argb(0xff400070)
                                };
                                let mix = (1.0 - factor * 0.8 - 0.2).clamp(0.0, 1.0);
                                color = color.blend(fade_color, mix);
                            }
                        }
                    } else if volume_fade {
                        // left of note is to the right of the playhead (note not played yet)

                        let fade_color = if dim {
                            Color::from_argb(0xff200040)
                        } else {
                            Color::from_argb(0xff400070)
                        };

                        let clock = (ofs_x + x) * beat_clock as i32 / *meas_width as i32;
                        let volume: f32 = unsafe {
                            get_value_at(
                                clock,
                                EventType::Volume,
                                u,
                                eve_raw,
                                cur_volume[u as usize],
                            )
                        } as f32
                            / 104.0;
                        let velocity: f32 = unsafe {
                            get_value_at(
                                clock,
                                EventType::Velocity,
                                u,
                                eve_raw,
                                cur_velocity[u as usize],
                            )
                        } as f32
                            / 104.0;

                        let factor = volume * velocity;
                        let factor = factor.powf(0.25);

                        let mix = (1.0 - factor * 0.8 - 0.2).clamp(0.0, 1.0);
                        color = color.blend(fade_color, mix);
                    }
                }

                if highlight_rect != Some(note_rect) {
                    if do_batching {
                        batch_a.push((note_rect, color));
                    } else {
                        unsafe { draw.fill_rect(&note_rect, color) };
                        count += 1;
                    }
                }

                if let Some(hl) = highlight_rect {
                    if do_batching {
                        batch_a.push((hl, highlight_color));
                    } else {
                        unsafe { draw.fill_rect(&hl, highlight_color) };
                        count += 1;
                    }
                }

                if x > bounds.left - 2 {
                    // left edge
                    if do_batching {
                        batch_a.push((
                            Rect::<i32>::new(
                                note_rect.left - 1,
                                note_rect.top - 1,
                                note_rect.left,
                                note_rect.bottom + 1,
                            ),
                            color,
                        ));
                    } else {
                        unsafe {
                            draw.fill_rect(
                                &Rect::<i32>::new(
                                    note_rect.left - 1,
                                    note_rect.top - 1,
                                    note_rect.left,
                                    note_rect.bottom + 1,
                                ),
                                color,
                            );
                        };
                        count += 1;
                    }
                }

                if x > bounds.left - 1 {
                    if do_batching {
                        batch_a.push((
                            Rect::<i32>::new(
                                note_rect.left - 2,
                                note_rect.top - 3,
                                note_rect.left - 1,
                                note_rect.bottom + 3,
                            ),
                            color,
                        ));
                    } else {
                        unsafe {
                            draw.fill_rect(
                                &Rect::<i32>::new(
                                    note_rect.left - 2,
                                    note_rect.top - 3,
                                    note_rect.left - 1,
                                    note_rect.bottom + 3,
                                ),
                                color,
                            );
                        };
                        count += 1;
                    }
                }

                if note_rect.right > bounds.left - 1 && note_rect.right < bounds.right {
                    // right edge
                    // if do_batching {
                    //     batch_a.push((
                    //         Rect::<i32>::new(
                    //             note_rect.right,
                    //             note_rect.top,
                    //             note_rect.right + 1,
                    //             note_rect.bottom,
                    //         ),
                    //         color,
                    //     ));
                    // } else {
                    //     draw.fill_rect(
                    //         &Rect::<i32>::new(
                    //             note_rect.right,
                    //             note_rect.top,
                    //             note_rect.right + 1,
                    //             note_rect.bottom,
                    //         ),
                    //         color,
                    //     );
                    //     count += 1;
                    // }

                    if do_batching {
                        batch_a.push((
                            Rect::<i32>::new(
                                note_rect.right,
                                note_rect.top + 1,
                                note_rect.right + 1,
                                note_rect.bottom - 1,
                            ),
                            color,
                        ));
                    } else {
                        unsafe {
                            draw.fill_rect(
                                &Rect::<i32>::new(
                                    note_rect.right,
                                    note_rect.top + 1,
                                    note_rect.right + 1,
                                    note_rect.bottom - 1,
                                ),
                                color,
                            );
                        };
                        count += 1;
                    }
                }
            },
            EventType::Key => {
                if key_notches {
                    let fade_color = if dim {
                        Color::from_argb(0xff200040)
                    } else {
                        Color::from_argb(0xff400070)
                    };

                    let x = (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x
                        + bounds.left;
                    if x > bounds.left - 1 && x < bounds.right {
                        if do_batching {
                            batch_a.push((Rect::<i32>::new(x, y + 1, x + 1, y + 2), fade_color));
                            batch_a.push((Rect::<i32>::new(x, y - 2, x + 1, y - 1), fade_color));
                        } else {
                            unsafe {
                                draw.fill_rect(
                                    &Rect::<i32>::new(x, y + 1, x + 1, y + 2),
                                    fade_color,
                                );
                            };
                            unsafe {
                                draw.fill_rect(
                                    &Rect::<i32>::new(x, y - 2, x + 1, y - 1),
                                    fade_color,
                                );
                            };
                            count += 2;
                        }
                    }
                }
            },
            _ => {
                let x =
                    (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x + bounds.left;
                if x > bounds.left - 2 && x < bounds.right - 1 {
                    #[allow(clippy::bool_to_int_with_if)]
                    let color = Color::from_argb([0xff00f080, 0x007840][if dim { 1 } else { 0 }]);
                    if do_batching {
                        batch_a.push((Rect::<i32>::new(x, y + 4, x + 2, y + 6), color));
                    } else {
                        unsafe { draw.fill_rect(&Rect::<i32>::new(x, y + 4, x + 2, y + 6), color) };
                        count += 1;
                    }
                }
            },
        }

        eve_raw = eve.next;
    }

    if false {
        log::debug!("rects = {count} / {culled}");
    }

    if do_batching {
        for (rect, color) in batch_a {
            unsafe { draw.fill_rect(&rect, color) };
        }
    }

    // let mut ddbltfx = [0_u32; 25];
    // ddbltfx[0] = 100;
    // ddbltfx[23] = 0;
    // ddbltfx[24] = 0;

    // real_draw.blt(
    //     PTC::get_unit_rect().as_mut_ptr().cast(),
    //     SURF.as_mut().unwrap().1,
    //     std::ptr::null_mut(),
    //     0x00010000 | 0x1000000,
    //     ddbltfx.as_mut_ptr().cast(),
    // );

    drop(draw);

    if use_separate_surface {
        let mut unit_rect = PTC::get_unit_rect();
        let dst_rect = unit_rect.as_lprect();
        let mut src_rect = RECT {
            left: 0,
            top: 0,
            right: bounds.width(),
            bottom: bounds.height(),
        };

        unsafe {
            real_draw
                .BltFast(
                    (*dst_rect).left as u32,
                    (*dst_rect).top as u32,
                    &surf.as_mut().unwrap().1.0,
                    &raw mut src_rect,
                    DDBLTFAST_SRCCOLORKEY,
                )
                .unwrap();
        };
    }
}

/// Old function to override drawing individual notes
// the second parameter here would normally be color, but an asm patch is used to change it to push the ebp register instead
//      which can be used to get the unit and focus state (which could be used to get the original color anyway)
#[allow(clippy::too_many_lines)] // TODO
#[allow(unused)]
pub(crate) unsafe fn draw_unit_note_rect<PTC: PTCVersion>(
    rect: *const libc::c_int,
    unit: u32,
    not_focused: bool,
) {
    // color = 0x0094FF;

    #[allow(clippy::bool_to_int_with_if)]
    let color = PTC::get_base_note_colors_argb()[if not_focused { 1 } else { 0 }];
    let raw_argb = color.to_be_bytes();
    let mut rgb = colorsys::Rgb::from([raw_argb[1], raw_argb[2], raw_argb[3]]);

    let colored_units = unsafe { COLORED_UNITS };
    let note_pulse = unsafe { NOTE_PULSE };
    let volume_fade = unsafe { VOLUME_FADE };

    if colored_units {
        rgb.adjust_hue(unit as f64 * 25.0);
    }

    let rect = unsafe { std::slice::from_raw_parts(rect, 4) };

    if PTC::is_playing() && (note_pulse || volume_fade) {
        let last_playhead_pos = unsafe { scroll_hook::LAST_PLAYHEAD_POS };
        if unsafe { scroll_hook::ENABLED } && rect[0] <= last_playhead_pos {
            // left of note is to the left of the playhead

            // TODO: clean up this logic
            let flash_strength = if not_focused { 0.4 } else { 0.8 };
            if rect[2] >= last_playhead_pos {
                // right of note is to the right of the playhead (playhead is on the note)

                if note_pulse {
                    let mix = flash_strength;
                    rgb.set_red(rgb.red() + (255.0 - rgb.red()) * mix);
                    rgb.set_green(rgb.green() + (255.0 - rgb.green()) * mix);
                    rgb.set_blue(rgb.blue() + (255.0 - rgb.blue()) * mix);
                }

                if volume_fade {
                    let volume: f32 =
                        PTC::get_event_value_at_screen_pos(last_playhead_pos, unit as i32, 0x5)
                            as f32
                            / 104.0;
                    let velocity: f32 =
                        PTC::get_event_value_at_screen_pos(last_playhead_pos, unit as i32, 0x5)
                            as f32
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
                let fade_pt = last_playhead_pos - fade_size;

                if note_pulse && rect[2] >= fade_pt {
                    let thru = (rect[2] - fade_pt) as f32 / fade_size as f32;

                    let mix = thru as f64 * flash_strength;
                    rgb.set_red(rgb.red() + (255.0 - rgb.red()) * mix);
                    rgb.set_green(rgb.green() + (255.0 - rgb.green()) * mix);
                    rgb.set_blue(rgb.blue() + (255.0 - rgb.blue()) * mix);
                }

                if volume_fade {
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
        } else if volume_fade {
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

    if rect[0] > PTC::get_unit_rect().left {
        // left edge
        PTC::draw_rect([rect[0] - 1, rect[1] - 1, rect[0], rect[3] + 1], color);
        PTC::draw_rect([rect[0] - 2, rect[1] - 3, rect[0] - 1, rect[3] + 3], color);
    }

    if rect[2] > PTC::get_unit_rect().left {
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

// complete replacement for the vanilla keyboard notes drawing function
// this allows for much easier modification
#[allow(clippy::too_many_lines)]
#[allow(clippy::field_reassign_with_default)]
pub(crate) unsafe fn draw_kb_notes<PTC: PTCVersion>() {
    let meas_width = PTC::get_measure_width();
    let ofs_x = PTC::get_kb_scroll_ofs_x();
    let ofs_y = PTC::get_kb_scroll_ofs_y();

    let mut unit_area = PTC::get_kb_rect();
    let bounds = Rect::<i32>::new(0, 0, unit_area.width(), unit_area.height());

    if bounds.width() <= 0 || bounds.height() <= 0 {
        return;
    }

    let beat_clock = PTC::get_beat_clock();
    let unit_num = PTC::get_unit_num();

    let unit_height = 16;

    let real_draw = unsafe {
        IDirectDrawSurface::from_raw_borrowed(&*(addr(0xa7b28) as *mut *mut libc::c_void)).unwrap()
    };

    let mut surf = SURF_KB.try_write().unwrap();
    if let Some((surf_size, _surf)) = surf.as_ref() {
        if *surf_size != unit_area {
            let _ = unsafe { real_draw.DeleteAttachedSurface(0, &surf.take().unwrap().1.0) };
            *surf = Some((
                unit_area,
                ForceSendSync(unsafe {
                    ddraw::create_surface(
                        *(addr(0xa7b20) as *mut *mut libc::c_void),
                        unit_area.width(),
                        unit_area.height(),
                    )
                }),
            ));
        }
    } else {
        *surf = Some((
            unit_area,
            ForceSendSync(unsafe {
                ddraw::create_surface(
                    *(addr(0xa7b20) as *mut *mut libc::c_void),
                    unit_area.width(),
                    unit_area.height(),
                )
            }),
        ));
    }

    let use_separate_surface = unsafe { USE_SEPARATE_SURFACE };
    let colored_units = unsafe { COLORED_UNITS };
    let note_pulse = unsafe { NOTE_PULSE };
    let volume_fade = unsafe { VOLUME_FADE };
    let porta_view = unsafe { PORTA_VIEW };

    let draw = if use_separate_surface {
        &surf.as_mut().unwrap().1.0
    } else {
        real_draw
    };

    let draw = draw.offset(
        if use_separate_surface {
            0
        } else {
            unit_area.left
        },
        if use_separate_surface {
            0
        } else {
            unit_area.top
        },
    );

    let colors = PTC::get_base_note_colors_argb().map(|c| Color::from_argb(c).with_a(1.0));

    let highlighted = (0..unit_num)
        .into_iter()
        .map(|u| PTC::is_unit_highlighted(u))
        .collect::<Vec<_>>();
    let mut cur_volume = (0..unit_num).into_iter().map(|_u| 104).collect::<Vec<_>>();
    let mut cur_velocity = (0..unit_num).into_iter().map(|_u| 104).collect::<Vec<_>>();

    let events_list = PTC::get_event_list();

    let mut batch_a: Vec<(Rect<i32>, Color)> = Vec::new();

    // TODO: this is stupid
    let do_batching = false;

    if use_separate_surface {
        unsafe { draw.fill_rect(&bounds, Color::from_argb(0xff000000)) };
    }

    let mut cur_y = (0..unit_num)
        .into_iter()
        .map(|_u| (0x6C00 - 0x4500) * unit_height / 0x100 + unit_height / 2)
        .collect::<Vec<_>>();

    let mut last_key_eves = (0..unit_num).into_iter().map(|_u| None).collect::<Vec<_>>();

    let mut last_porta_eves = (0..unit_num).into_iter().map(|_u| None).collect::<Vec<_>>();

    let mut eve_raw = events_list.start;
    while !eve_raw.is_null() {
        let eve = unsafe { &mut *eve_raw };

        let x = (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x + bounds.left;

        if x > bounds.right {
            break;
        }

        let u = eve.unit as i32;

        let dim = !highlighted[u as usize];

        if dim {
            eve_raw = eve.next;
            continue;
        }

        let y = bounds.top + cur_y[u as usize] - ofs_y;

        match eve.kind {
            EventType::Volume => {
                cur_volume[u as usize] = eve.value;
            },
            EventType::Velocity => {
                cur_velocity[u as usize] = eve.value;
            },
            EventType::On => {
                #[allow(clippy::bool_to_int_with_if)]
                let mut color = colors[if dim { 1 } else { 0 }];
                #[allow(clippy::bool_to_int_with_if)]
                let mut highlight_color = colors[if dim { 1 } else { 0 }];

                if colored_units {
                    color = color.rotate_hue(u as f64 * 25.0);
                }

                let x =
                    (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x + bounds.left;
                let x2 = ((eve.clock + eve.value) * (*meas_width as i32) / beat_clock as i32)
                    - ofs_x
                    + bounds.left;

                let note_rect = Rect::<i32>::new(
                    (x).max(bounds.left),
                    (y - 4).max(bounds.top),
                    (x2).min(bounds.right),
                    (y + 4).min(bounds.bottom),
                );

                if note_rect.right < bounds.left || note_rect.left > bounds.right + 1 {
                    eve_raw = eve.next;
                    continue;
                }

                let mut highlight_rect = None;
                if PTC::is_playing() && (note_pulse || volume_fade) {
                    let last_playhead_pos = unsafe { scroll_hook::LAST_PLAYHEAD_POS };
                    if unsafe { scroll_hook::ENABLED } && x + unit_area.left <= last_playhead_pos {
                        // left of note is to the left of the playhead

                        // TODO: clean up this logic
                        let flash_strength = if dim { 0.4 } else { 0.8 };
                        if x2 + unit_area.left >= last_playhead_pos {
                            // right of note is to the right of the playhead (playhead is on the note)

                            if note_pulse {
                                let clock = (ofs_x + last_playhead_pos - unit_area.left)
                                    * beat_clock as i32
                                    / *meas_width as i32;

                                highlight_color = color.blend(Color::WHITE, flash_strength);
                                color = color.blend(Color::WHITE, flash_strength * 0.75);

                                let (prev_eve_key_clock, prev_eve_key_value) = unsafe {
                                    get_event_at(clock, EventType::Key, u, eve_raw)
                                        .map_or((eve.clock, eve.value), |key| {
                                            (key.clock, key.value)
                                        })
                                };

                                let next_eve_key_clock = unsafe {
                                    get_next_event(
                                        clock,
                                        eve.clock + eve.value,
                                        EventType::Key,
                                        u,
                                        eve_raw,
                                    )
                                    .map_or(eve.clock + eve.value, |key| key.clock)
                                };

                                let x = (prev_eve_key_clock * (*meas_width as i32)
                                    / beat_clock as i32)
                                    - ofs_x
                                    + bounds.left;
                                let x2 = (next_eve_key_clock * (*meas_width as i32)
                                    / beat_clock as i32)
                                    - ofs_x
                                    + bounds.left;
                                let note = (0x6C00 - 0x4500 - (prev_eve_key_value - 0x6000))
                                    / 0x100
                                    * unit_height
                                    + unit_height / 2;
                                let y = bounds.top + note - ofs_y;

                                let note_rect = Rect::<i32>::new(
                                    (x).max(bounds.left),
                                    (y - 4).max(bounds.top),
                                    (x2).min(bounds.right),
                                    (y + 4).min(bounds.bottom),
                                );

                                highlight_rect = Some(note_rect);
                            }

                            if volume_fade {
                                let clock = (ofs_x + last_playhead_pos - unit_area.left)
                                    * beat_clock as i32
                                    / *meas_width as i32;
                                let volume: f32 = unsafe {
                                    get_value_at(
                                        clock,
                                        EventType::Volume,
                                        u,
                                        eve_raw,
                                        cur_volume[u as usize],
                                    )
                                } as f32
                                    / 104.0;
                                let velocity: f32 = unsafe {
                                    get_value_at(
                                        clock,
                                        EventType::Velocity,
                                        u,
                                        eve_raw,
                                        cur_velocity[u as usize],
                                    )
                                } as f32
                                    / 104.0;

                                let factor = volume * velocity;
                                let factor = factor.powf(0.25);

                                let fade_color = if dim {
                                    Color::from_argb(0xff200040)
                                } else {
                                    Color::from_argb(0xff400070)
                                };
                                let mix = (1.0 - factor * 0.8 - 0.2).clamp(0.0, 1.0);
                                color = color.blend(fade_color, mix);
                                highlight_color = highlight_color.blend(fade_color, mix);
                            }
                        } else {
                            // right of note is to the left of the playhead (playhead is past the note)

                            let fade_size = *PTC::get_measure_width() as i32 / 4;
                            let fade_pt = last_playhead_pos - fade_size;

                            if note_pulse && x2 + unit_area.left >= fade_pt {
                                let thru =
                                    (x2 + unit_area.left - fade_pt) as f32 / fade_size as f32;

                                color = color.blend(Color::WHITE, thru * flash_strength * 0.75);
                            }

                            if volume_fade {
                                let clock = (ofs_x + x2) * beat_clock as i32 / *meas_width as i32;
                                let volume: f32 = unsafe {
                                    get_value_at(
                                        clock - 1,
                                        EventType::Volume,
                                        u,
                                        eve_raw,
                                        cur_volume[u as usize],
                                    )
                                } as f32
                                    / 104.0;
                                let velocity: f32 = unsafe {
                                    get_value_at(
                                        clock - 1,
                                        EventType::Velocity,
                                        u,
                                        eve_raw,
                                        cur_velocity[u as usize],
                                    )
                                } as f32
                                    / 104.0;

                                let factor = volume * velocity;
                                let factor = factor.powf(0.25);

                                let fade_color = if dim {
                                    Color::from_argb(0xff200040)
                                } else {
                                    Color::from_argb(0xff400070)
                                };
                                let mix = (1.0 - factor * 0.8 - 0.2).clamp(0.0, 1.0);
                                color = color.blend(fade_color, mix);
                            }
                        }
                    } else if volume_fade {
                        // left of note is to the right of the playhead (note not played yet)

                        let fade_color = if dim {
                            Color::from_argb(0x00000001)
                        } else {
                            Color::from_argb(0x00000000)
                        };

                        let clock = (ofs_x + x) * beat_clock as i32 / *meas_width as i32;
                        let volume: f32 = unsafe {
                            get_value_at(
                                clock,
                                EventType::Volume,
                                u,
                                eve_raw,
                                cur_volume[u as usize],
                            )
                        } as f32
                            / 104.0;
                        let velocity: f32 = unsafe {
                            get_value_at(
                                clock,
                                EventType::Velocity,
                                u,
                                eve_raw,
                                cur_velocity[u as usize],
                            )
                        } as f32
                            / 104.0;

                        let factor = volume * velocity;
                        let factor = factor.powf(0.25);

                        let mix = (1.0 - factor * 0.8 - 0.2).clamp(0.0, 1.0);
                        color = color.blend(fade_color, mix);
                    }
                }

                if !PTC::is_playing() && u != PTC::get_focused_unit() {
                    color = color.with_a(color.a_f32() * 0.25);
                }

                if do_batching {
                    batch_a.push((note_rect, color));
                } else {
                    let mut cur_eve: &Event = eve;
                    let mut next_key = cur_y[u as usize];
                    let mut key_events = VecDeque::new();
                    let mut rects = vec![];
                    #[allow(clippy::while_let_loop)]
                    loop {
                        if let Some(next_eve_key) = unsafe {
                            get_next_event(
                                cur_eve.clock,
                                eve.clock + eve.value,
                                EventType::Key,
                                u,
                                eve_raw,
                            )
                        } {
                            let x = (cur_eve.clock * (*meas_width as i32) / beat_clock as i32)
                                - ofs_x
                                + bounds.left;
                            let x2 = ((next_eve_key.clock) * (*meas_width as i32)
                                / beat_clock as i32)
                                - ofs_x
                                + bounds.left;

                            let y = bounds.top + next_key - ofs_y;

                            let note_rect = Rect::<i32>::new(
                                (x).max(bounds.left),
                                (y - 4).max(bounds.top),
                                (x2).min(bounds.right),
                                (y + 4).min(bounds.bottom),
                            );
                            rects.push(note_rect);

                            next_key = (0x6C00 - 0x4500 - (next_eve_key.value - 0x6000)) / 0x100
                                * unit_height
                                + unit_height / 2;
                            cur_eve = next_eve_key;
                            key_events.push_back(next_eve_key);
                        } else {
                            let x = (cur_eve.clock * (*meas_width as i32) / beat_clock as i32)
                                - ofs_x
                                + bounds.left;
                            let x2 = ((eve.clock + eve.value) * (*meas_width as i32)
                                / beat_clock as i32)
                                - ofs_x
                                + bounds.left;

                            let y = bounds.top + next_key - ofs_y;

                            let note_rect = Rect::<i32>::new(
                                (x).max(bounds.left),
                                (y - 4).max(bounds.top),
                                (x2).min(bounds.right),
                                (y + 4).min(bounds.bottom),
                            );
                            rects.push(note_rect);
                            break;
                        }
                    }

                    if porta_view {
                        draw.set_pixels(|set_pixel| {
                            let on_eve = &eve;

                            let initial_key_eve: Option<&Event> = last_key_eves[u as usize];
                            let initial_porta_eve: Option<&Event> = last_porta_eves[u as usize];

                            let mut last_key = initial_key_eve.map_or(0x6000, |ke| ke.value);
                            let mut tgt_key_clock = initial_key_eve.map_or(0, |ke| ke.clock);
                            let mut tgt_key = last_key;

                            let mut last_porta = initial_porta_eve.map_or(0, |pe| pe.value);

                            let mut next_porta_eve = unsafe {
                                get_next_event(
                                    on_eve.clock,
                                    on_eve.clock + on_eve.value,
                                    EventType::Portament,
                                    u,
                                    initial_porta_eve.map_or(on_eve.next, |pe| pe.next),
                                )
                            };

                            for px in x..note_rect.right {
                                let clock = (px - bounds.left + ofs_x) * beat_clock as i32
                                    / (*meas_width as i32);

                                loop {
                                    let mut did_something = false;

                                    let mut process_events = vec![];

                                    if let Some(ke) =
                                        key_events.pop_front_if(|ke| clock >= ke.clock)
                                    {
                                        process_events.push(ke);
                                    }

                                    if let Some(pe) = next_porta_eve
                                        && clock >= pe.clock
                                    {
                                        process_events.push(pe);
                                    }

                                    // since we step 1 pixel at a time (which is >1 clock tick) make sure we don't process events out of order
                                    process_events.sort_by_key(|ev| ev.clock);

                                    for ev in process_events {
                                        match ev.kind {
                                            EventType::Key => {
                                                let porta_thru = if last_porta > 0 {
                                                    ((ev.clock - tgt_key_clock) as f32
                                                        / last_porta as f32)
                                                        .clamp(0.0, 1.0)
                                                } else {
                                                    1.0
                                                };
                                                let key = (last_key as f32
                                                    + (tgt_key - last_key) as f32 * porta_thru)
                                                    as i32;
                                                last_key = key;

                                                tgt_key = ev.value;
                                                tgt_key_clock = ev.clock;
                                                did_something = true;
                                            },
                                            EventType::Portament => {
                                                // changing porta after the current slide ends updates the starting key
                                                if ev.clock != tgt_key_clock
                                                    && clock >= tgt_key_clock + last_porta
                                                {
                                                    last_key = tgt_key;
                                                } else {
                                                    // when porta changes *mid* slide it doesn't recalculate the starting key (unless you put a key event)
                                                    // causes a discontinuity but is accurate
                                                }
                                                last_porta = ev.value;
                                                next_porta_eve = unsafe {
                                                    get_next_event(
                                                        on_eve.clock,
                                                        on_eve.clock + on_eve.value,
                                                        EventType::Portament,
                                                        u,
                                                        ev.next,
                                                    )
                                                };
                                                did_something = true;
                                            },
                                            _ => unimplemented!(),
                                        }
                                    }
                                    if !did_something {
                                        break;
                                    }
                                }

                                let porta_thru = if last_porta > 0 {
                                    ((clock - tgt_key_clock) as f32 / last_porta as f32)
                                        .clamp(0.0, 1.0)
                                } else {
                                    1.0
                                };
                                let key = (last_key as f32
                                    + (tgt_key - last_key) as f32 * porta_thru)
                                    as i32;

                                let key_y = (0x6C00 - 0x4500 - (key - 0x6000)) * unit_height
                                    / 0x100
                                    + unit_height / 2;

                                let y = bounds.top + key_y - ofs_y;
                                for py in (y - 4)..(y + 4) {
                                    if px < bounds.left
                                        || px >= bounds.right
                                        || py < bounds.top
                                        || py >= bounds.bottom
                                    {
                                        continue;
                                    }
                                    let mut color = color;

                                    if let Some(hl) = highlight_rect
                                        && px >= hl.left
                                        && px < hl.right
                                    {
                                        color = highlight_color;
                                    }

                                    set_pixel(
                                        px,
                                        py,
                                        color.with_a(
                                            color.a_f32()
                                                * if PTC::is_playing() { 1.0 } else { 0.5 },
                                        ),
                                    );
                                }
                            }
                        });

                        color = color
                            .with_a(color.a_f32() * if PTC::is_playing() { 0.25 } else { 0.75 });
                    }

                    for rect in rects {
                        unsafe { draw.fill_rect(&rect, color) };
                    }
                }

                if let Some(hl) = highlight_rect
                    && !porta_view
                {
                    if do_batching {
                        batch_a.push((hl, highlight_color));
                    } else {
                        unsafe { draw.fill_rect(&hl, highlight_color) };
                    }
                }

                // if x > bounds.left - 2 {
                //     if do_batching {
                //         batch_a.push((
                //             Rect::<i32>::new(
                //                 note_rect.left - 1,
                //                 note_rect.top - 1,
                //                 note_rect.left,
                //                 note_rect.bottom + 1,
                //             ),
                //             color,
                //         ));
                //     } else {
                //         draw.fill_rect(
                //             &Rect::<i32>::new(
                //                 note_rect.left - 1,
                //                 note_rect.top - 1,
                //                 note_rect.left,
                //                 note_rect.bottom + 1,
                //             ),
                //             color,
                //         );
                //     }

                //     // left edge
                //     if do_batching {
                //         batch_a.push((
                //             Rect::<i32>::new(
                //                 note_rect.left - 1,
                //                 note_rect.top - 1,
                //                 note_rect.left,
                //                 note_rect.bottom + 1,
                //             ),
                //             color,
                //         ));
                //     } else {
                //         draw.fill_rect(
                //             &Rect::<i32>::new(
                //                 note_rect.left - 1,
                //                 note_rect.top - 1,
                //                 note_rect.left,
                //                 note_rect.bottom + 1,
                //             ),
                //             color,
                //         );
                //     }

                //     if do_batching {
                //         batch_a.push((
                //             Rect::<i32>::new(
                //                 note_rect.left - 2,
                //                 note_rect.top - 3,
                //                 note_rect.left - 1,
                //                 note_rect.bottom + 3,
                //             ),
                //             color,
                //         ));
                //     } else {
                //         draw.fill_rect(
                //             &Rect::<i32>::new(
                //                 note_rect.left - 2,
                //                 note_rect.top - 3,
                //                 note_rect.left - 1,
                //                 note_rect.bottom + 3,
                //             ),
                //             color,
                //         );
                //     }
                // }

                // if note_rect.right > bounds.left {
                //     // right edge
                //     if do_batching {
                //         batch_a.push((
                //             Rect::<i32>::new(
                //                 note_rect.right,
                //                 note_rect.top,
                //                 note_rect.right + 1,
                //                 note_rect.bottom,
                //             ),
                //             color,
                //         ));
                //     } else {
                //         draw.fill_rect(
                //             &Rect::<i32>::new(
                //                 note_rect.right,
                //                 note_rect.top,
                //                 note_rect.right + 1,
                //                 note_rect.bottom,
                //             ),
                //             color,
                //         );
                //     }

                //     if do_batching {
                //         batch_a.push((
                //             Rect::<i32>::new(
                //                 note_rect.right + 1,
                //                 note_rect.top + 1,
                //                 note_rect.right + 2,
                //                 note_rect.bottom - 1,
                //             ),
                //             color,
                //         ));
                //     } else {
                //         draw.fill_rect(
                //             &Rect::<i32>::new(
                //                 note_rect.right + 1,
                //                 note_rect.top + 1,
                //                 note_rect.right + 2,
                //                 note_rect.bottom - 1,
                //             ),
                //             color,
                //         );
                //     }
                // }
            },
            EventType::Key => {
                cur_y[u as usize] = (0x6C00 - 0x4500 - (eve.value - 0x6000)) / 0x100 * unit_height
                    + unit_height / 2;
                last_key_eves[u as usize] = Some(eve);

                // let fade_color = if dim {
                //     Color::from_argb(0xff200040)
                // } else {
                //     Color::from_argb(0xff400070)
                // };

                // let x =
                //     (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x + bounds.left;
                // if x > bounds.left - 2 {
                //     if do_batching {
                //         batch_a.push((Rect::<i32>::new(x, y + 1, x + 1, y + 2), fade_color));
                //         batch_a.push((Rect::<i32>::new(x, y - 2, x + 1, y - 1), fade_color));
                //     } else {
                //         draw.fill_rect(&Rect::<i32>::new(x, y + 1, x + 1, y + 2), fade_color);
                //         draw.fill_rect(&Rect::<i32>::new(x, y - 2, x + 1, y - 1), fade_color);
                //     }
                // }
            },
            EventType::Portament => {
                last_porta_eves[u as usize] = Some(eve);
            },
            _ => {
                // let x =
                //     (eve.clock * (*meas_width as i32) / beat_clock as i32) - ofs_x + bounds.left;
                // if x > bounds.left - 2 {
                //     let color = Color::from_argb([0xff00f080, 0x007840][if dim { 1 } else { 0 }]);
                //     if do_batching {
                //         batch_a.push((Rect::<i32>::new(x, y + 4, x + 2, y + 6), color));
                //     } else {
                //         draw.fill_rect(&Rect::<i32>::new(x, y + 4, x + 2, y + 6), color);
                //     }
                // }
            },
        }

        eve_raw = eve.next;
    }

    if do_batching {
        for (rect, color) in batch_a {
            unsafe { draw.fill_rect(&rect, color) };
        }
    }

    drop(draw);

    if use_separate_surface {
        let mut ddbltfx = [0_u32; 25];
        ddbltfx[0] = 100;
        ddbltfx[23] = 0;
        ddbltfx[24] = 0;

        unsafe {
            real_draw
                .Blt(
                    unit_area.as_lprect(),
                    &surf.as_mut().unwrap().1.0,
                    std::ptr::null_mut(),
                    0x00010000 | 0x1000000,
                    ddbltfx.as_mut_ptr().cast(),
                )
                .unwrap();
        };
    }
}
