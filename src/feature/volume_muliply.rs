use winapi::{
    shared::windef::HWND,
    um::winuser::{self, MSG},
};

use crate::{
    ptc::{events::EventType, PTCVersion},
    winutil::{self, Menus},
};

use super::Feature;

lazy_static::lazy_static! {
    static ref M_VOLUME_MULTIPLY_ID: u16 = winutil::next_id();
}

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
        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_VOLUME_MULTIPLY_ID {
                    unsafe {
                        let l_template: Vec<u8> = "DLG_EVENTVOLUME\0".bytes().collect();
                        winuser::DialogBoxParamA(
                            *PTC::get_hinstance(),
                            l_template.as_ptr().cast::<i8>(),
                            msg.hwnd,
                            Some(fill_dialog::<PTC>),
                            0,
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
    w_param: usize,
    _l_param: isize,
) -> isize {
    if msg == winuser::WM_INITDIALOG {
        let title: Vec<u8> = "== Volume Multiply ==\0".bytes().collect();
        winuser::SetDlgItemTextA(hwnd, 0x451, title.as_ptr().cast::<i8>());

        PTC::volume_adjust_fill_selected_units(hwnd);

        let selection = PTC::get_selected_range();

        winuser::SetDlgItemInt(hwnd, 0x403, selection.meas_min as u32, 1);
        winuser::SetDlgItemInt(hwnd, 0x404, selection.meas_max as u32, 1);
        winuser::SetDlgItemInt(hwnd, 0x446, selection.beat_min as u32, 1);
        winuser::SetDlgItemInt(hwnd, 0x448, selection.beat_max as u32, 1);
        winuser::SetDlgItemInt(hwnd, 0x447, (selection.clock_min / 10) as u32, 1);
        winuser::SetDlgItemInt(hwnd, 0x449, (selection.clock_max / 10) as u32, 1);
        winuser::SetDlgItemInt(hwnd, 0x467, 0, 1);
        winuser::SetDlgItemInt(hwnd, 0x40a, PTC::get_beat_clock() / 10, 1);

        PTC::center_window(hwnd);
    } else if msg == winuser::WM_COMMAND {
        let high = winapi::shared::minwindef::HIWORD(w_param.try_into().unwrap());
        let low = winapi::shared::minwindef::LOWORD(w_param.try_into().unwrap());

        if high == 0 {
            if low == 1 {
                // click "OK"

                let meas_min = winuser::GetDlgItemInt(hwnd, 0x403, std::ptr::null_mut(), 1) as i32;
                let meas_max = winuser::GetDlgItemInt(hwnd, 0x404, std::ptr::null_mut(), 1) as i32;
                let beat_min = winuser::GetDlgItemInt(hwnd, 0x446, std::ptr::null_mut(), 1) as i32;
                let beat_max = winuser::GetDlgItemInt(hwnd, 0x448, std::ptr::null_mut(), 1) as i32;
                let clock_min = winuser::GetDlgItemInt(hwnd, 0x447, std::ptr::null_mut(), 1) as i32;
                let clock_max = winuser::GetDlgItemInt(hwnd, 0x449, std::ptr::null_mut(), 1) as i32;

                let mut buf = [0_u8; 16];
                let len = winuser::GetDlgItemTextA(
                    hwnd,
                    0x469,
                    buf.as_mut_ptr().cast::<i8>(),
                    buf.len() as libc::c_int,
                );
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

                winuser::EndDialog(hwnd, 1);
            } else {
                // esc or cancel
                winuser::EndDialog(hwnd, 0);
            }
        }
    }

    0
}
