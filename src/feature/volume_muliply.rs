use std::sync::LazyLock;

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            DialogBoxParamA, EndDialog, GetDlgItemInt, GetDlgItemTextA, MSG, SetDlgItemInt,
            SetDlgItemTextA, WM_COMMAND, WM_INITDIALOG,
        },
    },
    core::PCSTR,
};

use crate::{
    feature::dialog_input_width,
    ptc::{PTCVersion, events::EventType},
    winutil::{self, Menus, hiword, loword},
};

use super::Feature;

static M_VOLUME_MULTIPLY_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);

pub struct VolumeAdjuster {}

impl VolumeAdjuster {
    pub fn new() -> Self {
        Self {}
    }
}

impl<PTC: PTCVersion> Feature<PTC> for VolumeAdjuster {
    fn init(&mut self, menus: &mut Menus) {
        let menu = menus.get_or_create::<PTC>("Tools");
        winutil::add_menu_button(menu, "Volume Multiply", *M_VOLUME_MULTIPLY_ID, true);
    }

    fn cleanup(&mut self) {}

    fn win_msg(&mut self, msg: &MSG) {
        if msg.message == WM_COMMAND {
            let high = hiword(msg.wParam.0.try_into().unwrap());
            let low = loword(msg.wParam.0.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_VOLUME_MULTIPLY_ID {
                    unsafe {
                        let l_template: Vec<u8> = "DLG_EVENTVOLUME\0".bytes().collect();
                        DialogBoxParamA(
                            Some(*PTC::get_hinstance()),
                            PCSTR(l_template.as_ptr()),
                            Some(msg.hwnd),
                            Some(fill_dialog::<PTC>),
                            LPARAM(0),
                        );
                    }
                }
            }
        }
    }
}

unsafe extern "system" fn fill_dialog<PTC: PTCVersion>(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    _l_param: LPARAM,
) -> isize {
    if msg == WM_INITDIALOG {
        let title: Vec<u8> = "== Volume Multiply ==\0".bytes().collect();
        SetDlgItemTextA(hwnd, 0x451, PCSTR(title.as_ptr())).unwrap();

        PTC::volume_adjust_fill_selected_units(hwnd);

        let selection = PTC::get_selected_range();

        SetDlgItemInt(hwnd, 0x403, selection.meas_min as u32, true).unwrap();
        SetDlgItemInt(hwnd, 0x404, selection.meas_max as u32, true).unwrap();
        SetDlgItemInt(hwnd, 0x446, selection.beat_min as u32, true).unwrap();
        SetDlgItemInt(hwnd, 0x448, selection.beat_max as u32, true).unwrap();
        SetDlgItemInt(hwnd, 0x447, (selection.clock_min / 10) as u32, true).unwrap();
        SetDlgItemInt(hwnd, 0x449, (selection.clock_max / 10) as u32, true).unwrap();
        SetDlgItemInt(hwnd, 0x467, 0, true).unwrap();
        SetDlgItemInt(hwnd, 0x40a, PTC::get_beat_clock() / 10, true).unwrap();

        dialog_input_width::modify_dialog(hwnd.0);

        PTC::center_window(hwnd);
    } else if msg == WM_COMMAND {
        let high = hiword(w_param.0.try_into().unwrap());
        let low = loword(w_param.0.try_into().unwrap());

        if high == 0 {
            if low == 1 {
                // click "OK"

                let meas_min = GetDlgItemInt(hwnd, 0x403, None, true) as i32;
                let meas_max = GetDlgItemInt(hwnd, 0x404, None, true) as i32;
                let beat_min = GetDlgItemInt(hwnd, 0x446, None, true) as i32;
                let beat_max = GetDlgItemInt(hwnd, 0x448, None, true) as i32;
                let clock_min = GetDlgItemInt(hwnd, 0x447, None, true) as i32;
                let clock_max = GetDlgItemInt(hwnd, 0x449, None, true) as i32;

                let mut buf = [0_u8; 16];
                let len = GetDlgItemTextA(hwnd, 0x469, &mut buf);
                let factor = std::str::from_utf8(&buf[..len as usize])
                    .map_or(1.0, |s| s.parse::<f32>().unwrap_or(1.0));

                let evel = PTC::get_event_list();

                let start_pos = PTC::calc_clock_pos(meas_min, beat_min, clock_min * 10);
                let end_pos = PTC::calc_clock_pos(meas_max, beat_max, clock_max * 10);

                let (start_pos, end_pos) = (start_pos.min(end_pos), start_pos.max(end_pos));

                let mut cur = evel.start;
                loop {
                    if cur.is_null() {
                        break;
                    }

                    if (*cur).kind == EventType::Volume
                        && (*cur).clock >= start_pos
                        && (end_pos == -1 || (*cur).clock < end_pos)
                        && PTC::is_unit_highlighted((*cur).unit as i32)
                    {
                        (*cur).value = (((*cur).value as f32 * factor) as i32).clamp(0, 0x80);
                    }

                    cur = (*cur).next;
                }

                EndDialog(hwnd, 1).unwrap();
            } else {
                // esc or cancel
                EndDialog(hwnd, 0).unwrap();
            }
        }
    }

    0
}
