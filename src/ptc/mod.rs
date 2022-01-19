pub mod v0925;
pub mod v09454;

use winapi::{
    shared::{minwindef::HINSTANCE, windef::HWND},
    um::libloaderapi::GetModuleHandleA,
};

use crate::patch::Patch;

pub trait PTCVersion {
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
    fn get_hook() -> unsafe extern "system" fn(code: i32, w_param: usize, l_param: isize) -> isize;
    fn get_frame_thread_wrapper(
    ) -> unsafe extern "system" fn(base: winapi::shared::minwindef::LPVOID) -> u32;
    fn get_play_pos() -> &'static mut u32;
    fn get_scroll() -> &'static mut i32;
    fn get_scroll_max() -> i32;
    fn get_unit_rect() -> &'static [i32; 4];
    fn get_patches() -> Vec<Patch>;
    fn get_hook_draw_unitkb_top() -> unsafe extern "stdcall" fn();
    fn get_hook_draw_unitkb_bg() -> unsafe extern "stdcall" fn();
    fn get_hook_draw_unit_note_rect(
    ) -> unsafe extern "cdecl" fn(rect: *const libc::c_int, color: libc::c_uint);
}

pub fn addr(relative: usize) -> usize {
    unsafe {
        let base =
            GetModuleHandleA("ptCollage.exe\0".bytes().collect::<Vec<u8>>().as_ptr() as *const i8)
                as usize;
        base + relative
    }
}
