use winapi::shared::{minwindef::HINSTANCE, windef::HWND};

use crate::{
    feature::{
        playhead::{self, Playhead},
        scroll_hook::{self, Scroll},
    },
    patch::{hook, hook_pre_ret_new},
};

use super::{addr, PTCVersion};

pub struct PTC09454;

impl PTCVersion for PTC09454 {
    fn get_features() -> Vec<Box<dyn crate::feature::Feature<Self>>> {
        // scroll hook

        let unit_clear_hook_patch =
            hook!(0x7920a, 0x78a60, "cdecl", fn(a: *mut f32), |_old_fn, _a| {
                scroll_hook::unit_clear::<PTC09454>();
            });

        let f_scroll_hook = Scroll::new::<Self>(unit_clear_hook_patch);

        // playhead

        let draw_unitkb_top_patch = hook_pre_ret_new!(
            0x79331,
            0x62080,
            "stdcall",
            fn(),
            playhead::draw_unitkb_top::<PTC09454>
        );

        let f_playhead = Playhead::new::<Self>(draw_unitkb_top_patch);

        vec![Box::new(f_scroll_hook), Box::new(f_playhead)]
    }

    fn get_hwnd() -> &'static mut HWND {
        unsafe { &mut *(addr(0xbddd0) as *mut HWND) }
    }

    fn get_hinstance() -> &'static mut winapi::shared::minwindef::HINSTANCE {
        unsafe { &mut *(addr(0xbddcc) as *mut HINSTANCE) }
    }

    fn start_play() {
        unsafe {
            let start_play: unsafe extern "fastcall" fn() =
                std::mem::transmute(addr(0x6fbf0) as *const ());
            (start_play)();
        }
    }

    fn is_playing() -> bool {
        unsafe { *((*(addr(0xBE028) as *mut usize) + 0x94) as *mut u8) == 1 }
    }

    fn get_volume() -> &'static mut f32 {
        unsafe { &mut *(addr(0xC00E0) as *mut f32) }
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
        (0, 9, 4, 54)
    }

    fn get_beat_num() -> &'static mut u32 {
        unsafe {
            &mut *((*((*(addr(0xBE020) as *mut usize) + 0xC0) as *mut usize) + 0x18) as *mut u32)
        }
    }

    fn get_tempo() -> &'static mut f32 {
        unsafe {
            &mut *((*((*(addr(0xBE020) as *mut usize) + 0xC0) as *mut usize) + 0x18 + 0x4)
                as *mut f32)
        }
    }

    fn get_beat_clock() -> u32 {
        unsafe {
            *((*((*(addr(0xBE020) as *mut usize) + 0xC0) as *mut usize) + 0x18 + 0x8) as *mut u32)
        }
    }

    fn get_measure_width() -> &'static mut u32 {
        unsafe { &mut *(addr(0xBFE3C) as *mut u32) }
    }

    fn get_sample_rate() -> u32 {
        unsafe { *((*(addr(0xBE020) as *mut usize) + 0x20) as *mut u32) }
    }

    fn get_buffer_size() -> u32 {
        unsafe {
            // *((*(0xdd4434 as *mut usize) + 0x20) as *mut u32)
            let buf_size_float = *((*(addr(0xBE028) as *mut usize) + 0x14) as *mut f32);
            (buf_size_float * Self::get_sample_rate() as f32) as u32
        }
    }

    fn get_play_pos() -> &'static mut u32 {
        unsafe { &mut *((*(addr(0xBE020) as *mut usize) + 0x70) as *mut u32) }
    }

    fn get_scroll() -> &'static mut i32 {
        unsafe { &mut *(addr(0xC02F8) as *mut i32) }
    }

    fn get_scroll_max() -> i32 {
        unsafe {
            *(addr(0xc02e0 + 0x14) as *mut i32)
                - (*(addr(0xc02e0 + 0x68) as *mut f32) - *(addr(0xc02e0 + 0x60) as *mut f32)) as i32
        }
    }

    fn get_unit_rect() -> [i32; 4] {
        // this version's rectangles are floats while 0.9.2.5 is ints
        unsafe {
            [
                *(addr(0xbfe48) as *const f32) as i32,
                *(addr(0xbfe48 + 0x04) as *const f32) as i32,
                *(addr(0xbfe48 + 0x08) as *const f32) as i32,
                *(addr(0xbfe48 + 0x0c) as *const f32) as i32,
            ]
        }
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
            crate::runtime::fill_about_dialog::<PTC09454>(hwnd, msg, w_param, l_param)
        }
        fill_about_dialog
    }

    fn center_window(hwnd: HWND) {
        unsafe {
            let center_window: unsafe extern "cdecl" fn(param_1: HWND) =
                std::mem::transmute(addr(0x24e0) as *const ());
            (center_window)(hwnd);
        }
    }

    fn about_dlg_fn_2(hwnd: HWND) {
        unsafe {
            let fn_2: unsafe extern "cdecl" fn(param_1: HWND) =
                std::mem::transmute(addr(0x815e0) as *const ());
            (fn_2)(hwnd);
        }
    }

    fn get_about_dialog_text_ids() -> (i32, i32, i32, i32) {
        (0x3f6, 0x439, 0x40b, 0x3ea)
    }

    fn draw_rect(rect: [i32; 4], color: u32) {
        unsafe {
            let draw_rect: unsafe extern "thiscall" fn(
                this: *mut (),
                rect: *const f32,
                color: libc::c_uint,
            ) = std::mem::transmute(addr(0x7570) as *const ());
            let f_rect = [
                rect[0] as f32,
                rect[1] as f32,
                rect[2] as f32,
                rect[3] as f32,
            ];
            (draw_rect)(
                *(addr(0xbe03c) as *mut usize) as *mut (),
                f_rect.as_ptr(),
                color,
            );
        }
    }
}
