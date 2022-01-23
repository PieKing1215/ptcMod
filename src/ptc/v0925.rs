use crate::feature::{
    custom_note_rendering::CustomNoteRendering, fps_unlock::FPSUnlock, playhead::Playhead,
    scroll_hook::Scroll, Feature,
};
use winapi::shared::{minwindef::HINSTANCE, windef::HWND};

use super::{addr, PTCVersion};

pub struct PTC0925;

impl PTCVersion for PTC0925 {
    fn get_features() -> Vec<Box<dyn Feature<Self>>> {
        // it is actually possible to move these extern fn definitions into the constructors using generics
        // however keeping them here is better since the signatures of the hooked functions can change between ptc versions

        unsafe extern "stdcall" fn unit_clear_hook() {
            let unit_clear: unsafe extern "stdcall" fn() =
                std::mem::transmute(addr(0x16440) as *const ());
            (unit_clear)();
            crate::feature::scroll_hook::unit_clear::<PTC0925>();
        }

        unsafe extern "cdecl" fn draw_unit_note_rect(
            rect: *const libc::c_int,
            color: libc::c_uint,
        ) {
            crate::feature::custom_note_rendering::draw_unit_note_rect::<PTC0925>(rect, color);
        }

        unsafe extern "stdcall" fn draw_unitkb_top_hook() {
            crate::feature::playhead::draw_unitkb_top::<PTC0925>();

            let fun_00009f80: unsafe extern "stdcall" fn() =
                std::mem::transmute(addr(0x9f80) as *const ());
            (fun_00009f80)();
        }

        vec![
            Box::new(FPSUnlock::new::<Self>()),
            Box::new(Scroll::new::<Self>(
                unit_clear_hook as *const (),
                0x165e8,
                0x16440,
            )),
            Box::new(CustomNoteRendering::new::<Self>(draw_unit_note_rect)),
            Box::new(Playhead::new::<Self>(draw_unitkb_top_hook)),
        ]
    }

    fn get_hwnd() -> &'static mut HWND {
        unsafe { &mut *(addr(0xDD4440 - 0xd30000) as *mut HWND) }
    }

    fn get_hinstance() -> &'static mut winapi::shared::minwindef::HINSTANCE {
        unsafe { &mut *(addr(0x00dd431c - 0xd30000) as *mut HINSTANCE) }
    }

    fn start_play() {
        unsafe {
            let start_play: unsafe extern "fastcall" fn(
                param_1: libc::c_int,
                param_2: libc::c_int,
            ) = std::mem::transmute(addr(0x00d41ae0 - 0xd30000) as *const ());
            (start_play)(0, 0x208); // idk what these magic numbers are but this is what ecx and edx are when the function is called normally
        }
    }

    fn is_playing() -> bool {
        unsafe { *(addr(0xdd81ec - 0xd30000) as *mut u8) == 1 }
    }

    fn get_volume() -> &'static mut f32 {
        unsafe { &mut *(addr(0xDD6BA0 - 0xd30000) as *mut f32) }
    }

    fn get_version() -> (u32, u32, u32, u32) {
        // let mut v1 = 0;
        // let mut v2 = 0;
        // let mut v3 = 0;
        // let mut v4 = 0;
        // unsafe {
        //     let fill_version: unsafe extern "cdecl" fn(
        //         param_1: *mut libc::c_uint,
        //         param_2: *mut libc::c_uint,
        //         param_3: *mut libc::c_uint,
        //         param_4: *mut libc::c_uint,
        //     ) = std::mem::transmute(0x00d31380 as *const ());
        //     (fill_version)(&mut v1, &mut v2, &mut v3, &mut v4);
        // }
        // (v1, v2, v3, v4)
        (0, 9, 2, 5)
    }

    fn get_beat_num() -> &'static mut u32 {
        unsafe {
            &mut *((*((*(addr(0xdd4430 - 0xd30000) as *mut usize) + 0x98) as *mut usize) + 0x10)
                as *mut u32)
        }
    }

    fn get_tempo() -> &'static mut f32 {
        unsafe {
            &mut *((*((*(addr(0xdd4430 - 0xd30000) as *mut usize) + 0x98) as *mut usize)
                + 0x10
                + 0x4) as *mut f32)
        }
    }

    fn get_beat_clock() -> u32 {
        unsafe {
            *((*((*(addr(0xdd4430 - 0xd30000) as *mut usize) + 0x98) as *mut usize) + 0x10 + 0x8)
                as *mut u32)
        }
    }

    fn get_measure_width() -> &'static mut u32 {
        unsafe { &mut *(addr(0xdd694c - 0xd30000) as *mut u32) }
    }

    fn get_sample_rate() -> u32 {
        unsafe { *((*(addr(0xdd4430 - 0xd30000) as *mut usize) + 0x8) as *mut u32) }
    }

    fn get_buffer_size() -> u32 {
        unsafe { *((*(addr(0xdd4434 - 0xd30000) as *mut usize) + 0x20) as *mut u32) }
    }

    fn get_play_pos() -> &'static mut u32 {
        unsafe { &mut *((*(addr(0xdd4430 - 0xd30000) as *mut usize) + 0x54) as *mut u32) }
    }

    fn get_scroll() -> &'static mut i32 {
        unsafe { &mut *((addr(0xdd6d70 - 0xd30000) + 0x14) as *mut i32) }
    }

    fn get_scroll_max() -> i32 {
        unsafe {
            *(addr(0xa6d70 + 0x10) as *mut i32)
                - (*(addr(0xa6d70 + 0x64) as *mut i32) - *(addr(0xa6d70 + 0x5c) as *mut i32))
        }
    }

    fn get_unit_rect() -> [i32; 4] {
        unsafe { *(addr(0xa693c) as *const [i32; 4]) }
    }

    fn get_fill_about_dialog(
    ) -> unsafe extern "system" fn(hwnd: HWND, msg: u32, w_param: usize, l_param: isize) -> isize
    {
        unsafe extern "system" fn fill_about_dialog(
            hwnd: HWND,
            msg: u32,
            w_param: usize,
            l_param: isize,
        ) -> isize {
            crate::runtime::fill_about_dialog::<PTC0925>(hwnd, msg, w_param, l_param)
        }
        fill_about_dialog
    }

    fn center_window(hwnd: HWND) {
        unsafe {
            let center_window: unsafe extern "cdecl" fn(param_1: HWND) =
                std::mem::transmute(addr(0x1e550) as *const ());
            (center_window)(hwnd);
        }
    }

    fn about_dlg_fn_2(hwnd: HWND) {
        unsafe {
            let fn_2: unsafe extern "cdecl" fn(param_1: HWND) =
                std::mem::transmute(addr(0x1d310) as *const ());
            (fn_2)(hwnd);
        }
    }

    fn get_about_dialog_text_ids() -> (i32, i32, i32, i32) {
        (0x3f6, 0x43a, 0x40c, 0x3ea)
    }
}
