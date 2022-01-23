pub mod v0925;
pub mod v09454;

use winapi::{
    shared::{minwindef::HINSTANCE, windef::HWND},
    um::libloaderapi::GetModuleHandleA,
};

use crate::feature::Feature;

pub trait PTCVersion {
    fn get_features() -> Vec<Box<dyn Feature<Self>>>;
    fn get_hwnd() -> &'static mut HWND;
    fn get_hinstance() -> &'static mut HINSTANCE;
    fn start_play();
    fn is_playing() -> bool;
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
    fn get_fill_about_dialog(
    ) -> unsafe extern "system" fn(hwnd: HWND, msg: u32, w_param: usize, l_param: isize) -> isize;
    fn center_window(hwnd: HWND);
    fn about_dlg_fn_2(hwnd: HWND);
    fn get_about_dialog_text_ids() -> (i32, i32, i32, i32);
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
