pub mod events;
pub mod v0925;
pub mod v09454;

use std::path::PathBuf;

use winapi::{
    shared::{minwindef::HINSTANCE, windef::HWND},
    um::libloaderapi::GetModuleHandleA,
};

use crate::feature::Feature;

use self::events::{Event, EventList};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selection {
    pub meas_min: i32,
    pub meas_max: i32,
    pub beat_min: i32,
    pub beat_max: i32,
    pub clock_min: i32,
    pub clock_max: i32,
}

pub trait PTCVersion {
    fn get_features() -> Vec<Box<dyn Feature<Self>>>;
    fn get_hwnd() -> &'static mut HWND;
    fn get_hinstance() -> &'static mut HINSTANCE;
    fn start_play();
    fn is_playing() -> bool;
    fn get_tab() -> &'static mut u32;
    fn get_volume() -> &'static mut f32;
    fn get_version() -> (u32, u32, u32, u32);
    fn get_beat_num() -> &'static mut u32;
    fn get_tempo() -> &'static mut f32;
    fn get_beat_clock() -> u32;
    fn get_measure_width() -> &'static mut u32;
    fn get_sample_rate() -> u32;
    fn get_buffer_size() -> u32;
    fn get_play_pos() -> &'static mut u32;
    fn get_scroll() -> &'static mut i32;
    fn get_scroll_max() -> i32;
    fn get_unit_rect() -> [i32; 4];
    fn get_event_list() -> &'static mut EventList;
    fn is_unit_highlighted(unit_no: i32) -> bool;
    fn get_selected_range() -> Selection;
    fn get_unit_scroll_ofs_x() -> &'static i32;
    fn get_unit_scroll_ofs_y() -> &'static i32;
    fn get_unit_num() -> i32;
    fn get_events_for_unit(unit_no: i32) -> &'static [Event];

    fn calc_clock_pos(meas: i32, beat: i32, clock: i32) -> i32 {
        *Self::get_beat_num() as i32 * Self::get_beat_clock() as i32 * meas
            + Self::get_beat_clock() as i32 * beat
            + clock
    }

    fn get_fill_about_dialog(
    ) -> unsafe extern "system" fn(hwnd: HWND, msg: u32, w_param: usize, l_param: isize) -> isize;
    fn center_window(hwnd: HWND);
    fn about_dlg_fn_2(hwnd: HWND);
    fn get_about_dialog_text_ids() -> (i32, i32, i32, i32);
    fn draw_rect(rect: [i32; 4], color: u32);
    fn get_base_note_colors_argb() -> [u32; 2];
    fn get_event_value_at_screen_pos(pos_x: i32, unit_no: i32, ev_type: i32) -> i32;
    fn load_file_no_history(path: PathBuf);
    fn volume_adjust_fill_selected_units(hwnd: HWND) -> bool;
}

pub fn addr(relative: usize) -> usize {
    unsafe {
        let base = GetModuleHandleA(
            "ptCollage.exe\0"
                .bytes()
                .collect::<Vec<u8>>()
                .as_ptr()
                .cast::<i8>(),
        ) as usize;
        base + relative
    }
}

// these two fns are functionally identical, but it's probably better to be explicit with the conversion

pub fn color_argb_to_abgr(abgr: u32) -> u32 {
    let a = (abgr >> 24) & 0xff;
    let r = (abgr >> 16) & 0xff;
    let g = (abgr >> 8) & 0xff;
    let b = abgr & 0xff;

    (a << 24) | (b << 16) | (g << 8) | r
}

pub fn color_abgr_to_argb(abgr: u32) -> u32 {
    let a = (abgr >> 24) & 0xff;
    let b = (abgr >> 16) & 0xff;
    let g = (abgr >> 8) & 0xff;
    let r = abgr & 0xff;

    (a << 24) | (r << 16) | (g << 8) | b
}
