use std::{ffi::CString, slice};

use crate::{
    feature::{
        custom_note_rendering::{self, CustomNoteRendering},
        drag_and_drop::DragAndDrop,
        fps_display_fix::FPSDisplayFix,
        fps_unlock::FPSUnlock,
        playhead::{self, Playhead},
        scroll_hook::{self, Scroll},
        volume_muliply::VolumeAdjuster,
        Feature,
    },
    patch::{hook_post_ret_new, hook_pre_ret_new, replace, Patch},
};
use winapi::{shared::{minwindef::HINSTANCE, windef::{HWND}}};

use super::{
    addr,
    events::{Event, EventList},
    PTCVersion, Selection,
};

pub struct PTC0925;

impl PTCVersion for PTC0925 {
    fn get_features() -> Vec<Box<dyn Feature<Self>>> {
        // callbacks involving extern fn are hard to make generic
        // I've tried like 4 different ways of handling this as well as possible
        // ideally, I would like to be able to generate a function here that takes a
        //   callback and outputs a Patch, and then pass this generator fn to the feature
        // like:
        // ```
        // fn gen(callback: fn()) {
        //     unsafe extern "stdcall" fn unit_clear_hook<const f: usize>() {
        //         let unit_clear: unsafe extern "stdcall" fn() =
        //             std::mem::transmute(addr(0x16440) as *const ());
        //         (unit_clear)();
        //         (callback)();
        //     }
        //     patch::call_patch(0x165e8, 0x16440, unit_clear_hook)
        // }
        // let feat = SomeFeature::new(gen);
        // ```
        // that doesn't work because a fn can't capture environment, and it cant be a closure because it needs to be extern
        // I also tried with const generics, but it doesn't work since you can't use a fn as const generic param
        //   you can't do `<const f: fn()>`
        //   you can't do `<const f: *const ()>`
        //   you can do `<const f: usize>`, but you can't do `const p: usize = my_func as usize`
        // if any of the above worked, it would be possible
        // as far as I can tell, this is just impossible right now, even on nightly

        // const fn gen<const f: fn()>() {
        //     unsafe extern "stdcall" fn draw_unitkb_top_hook<const f: usize>() {
        //         let unit_clear: unsafe extern "stdcall" fn() =
        //             std::mem::transmute(addr(0x16440) as *const ());
        //         (unit_clear)();

        //         let myf: fn() =
        //             std::mem::transmute(f as *const ());
        //         (myf)();
        //     }
        // }
        // // pass gen into feature and it could do:
        // let patch = gen::<my_func::<PTC>>();

        // scroll hook

        let unit_clear_hook_patch = hook_post_ret_new!(
            0x165e8,
            0x16440,
            "stdcall",
            fn(),
            scroll_hook::unit_clear::<PTC0925>
        );

        let f_scroll_hook = Scroll::new::<Self>(unit_clear_hook_patch);

        // custom note rendering

        // let note_rect_push_ebp = Patch::new(0x1469a, vec![0x52], vec![0x55]).unwrap();
        // let note_rect_hook_patch = hook!(
        //     0x1469f,
        //     0x1c0e0,
        //     "cdecl",
        //     fn(rect: *const i32, ebp: u32),
        //     |_old_fn, rect, ebp| {
        //         let not_focused = *((ebp - 0x7c) as *mut u32) != 0;
        //         let unit = *((ebp - 0x80) as *mut u32);
        //         custom_note_rendering::draw_unit_note_rect::<PTC0925>(rect, unit, not_focused);
        //     }
        // );

        // // first set here changes the spritesheet to empty, second NOPs the draw_image call completely
        // // let note_disable_left_edge = Patch::new(0x146b8, vec![0x03], vec![0x00]).unwrap();
        // // let note_disable_right_edge = Patch::new(0x146e9, vec![0x03], vec![0x00]).unwrap();
        // let note_disable_left_edge = Patch::new(
        //     0x146ca,
        //     vec![0xe8, 0xa1, 0x77, 0x00, 0x00],
        //     vec![0x90, 0x90, 0x90, 0x90, 0x90],
        // )
        // .unwrap();
        // let note_disable_right_edge = Patch::new(
        //     0x146f9,
        //     vec![0xe8, 0x72, 0x77, 0x00, 0x00],
        //     vec![0x90, 0x90, 0x90, 0x90, 0x90],
        // )
        // .unwrap();

        // let f_custom_note_rendering = CustomNoteRendering::new::<Self>(
        //     note_rect_push_ebp,
        //     note_rect_hook_patch,
        //     note_disable_left_edge,
        //     note_disable_right_edge,
        // );

        let draw_unit_notes = replace!(
            0x166bb,
            0x14480,
            "stdcall",
            fn(),
            custom_note_rendering::draw_unit_notes::<PTC0925>
        );

        let f_custom_note_rendering = CustomNoteRendering::new::<Self>(draw_unit_notes);

        // playhead

        let draw_unitkb_top_patch = hook_pre_ret_new!(
            0x166c0,
            0x9f80,
            "stdcall",
            fn(),
            playhead::draw_unitkb_top::<PTC0925>
        );

        let f_playhead = Playhead::new::<Self>(draw_unitkb_top_patch);

        // fps display fix

        let digit_patch = Patch::new(0x167d1, vec![0x2], vec![0x3]).unwrap();

        let number_x_patch =
            Patch::new(0x167d7, vec![-0x14_i8 as u8], vec![(-0x14_i8 - 8) as u8]).unwrap();

        let label_x_patch =
            Patch::new(0x167c0, vec![-0x2c_i8 as u8], vec![(-0x2c_i8 - 8) as u8]).unwrap();

        let f_fps_display_fix =
            FPSDisplayFix::new::<Self>(digit_patch, number_x_patch, label_x_patch);

        vec![
            Box::new(FPSUnlock::new::<Self>()),
            Box::new(f_scroll_hook),
            Box::new(f_custom_note_rendering),
            Box::new(f_playhead),
            Box::new(DragAndDrop::new::<Self>()),
            Box::new(f_fps_display_fix),
            Box::new(VolumeAdjuster::new()),
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

    fn get_tab() -> &'static mut u32 {
        unsafe { &mut *(addr(0xa6ca8) as *mut u32) }
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
        .max(0)
    }

    fn get_unit_rect() -> [i32; 4] {
        // unsafe { *(addr(0xa693c) as *const [i32; 4]) }
        unsafe { *(addr(0xa6cbc) as *const [i32; 4]) }
    }

    fn get_event_list() -> &'static mut super::events::EventList {
        unsafe { &mut **((*(addr(0xa4430) as *mut usize) + 160) as *mut *mut EventList) }
    }

    fn is_unit_highlighted(unit_no: i32) -> bool {
        unsafe {
            let is_unit_highlighted: unsafe extern "cdecl" fn(unit: libc::c_int) -> bool =
                std::mem::transmute(addr(0x71a0) as *const ());
            (is_unit_highlighted)(unit_no)
        }
    }

    fn get_selected_range() -> Selection {
        unsafe {
            let get_selected_range: unsafe extern "cdecl" fn(
                meas_min: *mut i32,
                meas_max: *mut i32,
                beat_min: *mut i32,
                beat_max: *mut i32,
                clock_min: *mut i32,
                clock_max: *mut i32,
            ) -> bool = std::mem::transmute(addr(0x12940) as *const ());

            let mut meas_min = 0;
            let mut meas_max = 0;
            let mut beat_min = 0;
            let mut beat_max = 0;
            let mut clock_min = 0;
            let mut clock_max = 0;

            (get_selected_range)(
                &mut meas_min,
                &mut beat_min,
                &mut clock_min,
                &mut meas_max,
                &mut beat_max,
                &mut clock_max,
            );

            Selection {
                meas_min,
                meas_max,
                beat_min,
                beat_max,
                clock_min,
                clock_max,
            }
        }
    }

    fn get_unit_scroll_ofs_x() -> &'static i32 {
        unsafe { &*(addr(0xa6d70 + 0x14) as *mut i32) }
    }

    fn get_unit_scroll_ofs_y() -> &'static i32 {
        unsafe { &*(addr(0xa6ec0 + 0x14) as *mut i32) }
    }

    fn get_unit_num() -> i32 {
        unsafe { *((*(addr(0xa4430) as *mut usize) + 60) as *mut i32) }
    }

    fn get_events_for_unit(unit_no: i32) -> &'static [super::events::Event] {
        unsafe {
            // reset lock
            *(addr(0xa696c) as *mut bool) = false;

            let mut raw_events = std::ptr::null_mut();
            // log::debug!("{unit_no} {raw_events:?}");
            let fill_events_for_unit: unsafe extern "cdecl" fn(
                unit_no: u32,
                events_ptr: *mut *mut Event,
            ) -> i32 = std::mem::transmute(addr(0x8cd0) as *const ());
            let count = (fill_events_for_unit)(unit_no as u32, &mut raw_events);

            // log::debug!("{count} {raw_events:?}");

            // reset lock again
            *(addr(0xa696c) as *mut bool) = false;

            slice::from_raw_parts(raw_events, count as usize)
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

    fn draw_rect(rect: [i32; 4], color: u32) {
        unsafe {
            let draw_rect: unsafe extern "cdecl" fn(rect: *const libc::c_int, color: libc::c_uint) =
                std::mem::transmute(addr(0x1c0e0) as *const ());
            (draw_rect)(rect.as_ptr(), color);
        }
    }

    fn get_base_note_colors_argb() -> [u32; 2] {
        unsafe { *(addr(0xa6cb4) as *mut [u32; 2]) }
    }

    fn get_event_value_at_screen_pos(pos_x: i32, unit_no: i32, ev_type: i32) -> i32 {
        unsafe {
            let get_event_value: unsafe extern "cdecl" fn(
                pos_x: i32,
                unit_no: i32,
                ev_type: i32,
            ) -> i32 = std::mem::transmute(addr(0x8f80) as *const ());
            (get_event_value)(pos_x, unit_no, ev_type)
        }
    }

    fn load_file_no_history(path: std::path::PathBuf) {
        unsafe {
            log::debug!("load_file_no_history({path:?})");

            // if we don't do this, it crashes with scroll hook
            log::debug!("stop_playing()");
            let stop_playing: unsafe extern "stdcall" fn() -> bool =
                std::mem::transmute(addr(0x3ca0) as *const ());
            let r = (stop_playing)();
            log::debug!("-> {r}");

            // fopen

            let cstr = path.to_str().and_then(|p| CString::new(p).ok()).unwrap();
            let mode = CString::new("rb").unwrap();

            let fopen: unsafe extern "cdecl" fn(
                fname: *const libc::c_char,
                mode: *const libc::c_char,
            ) -> *mut libc::FILE = std::mem::transmute(addr(0x3688e) as *const ());

            log::debug!("fopen({cstr:?}, {mode:?})");
            let mut file = (fopen)(cstr.as_ptr(), mode.as_ptr());
            log::debug!("-> {file:?}");

            // read file

            let read_file: unsafe extern "thiscall" fn(
                this: *mut libc::c_void,
                file: *mut libc::c_void,
                unk: u8,
            ) -> u8 = std::mem::transmute(addr(0x25ef0) as *const ());

            let ptr_2: *mut *mut libc::FILE = &mut file;

            log::debug!("read_file(...)");
            let r = (read_file)(*(addr(0xa4430) as *mut usize) as *mut _, ptr_2.cast(), 0);
            log::debug!("-> {r}");

            // fclose

            let fclose: unsafe extern "cdecl" fn(mode: *mut libc::FILE) -> libc::c_int =
                std::mem::transmute(addr(0x365c8) as *const ());

            log::debug!("fclose({file:?})");
            let r = (fclose)(file);
            log::debug!("-> {r}");

            // extra functions that need to be called

            let fn_1: unsafe extern "fastcall" fn(this: *mut libc::c_void) -> u8 =
                std::mem::transmute(addr(0x22c90) as *const ());

            log::debug!("fn_1(...)");
            let r = (fn_1)(*(addr(0xa4430) as *mut usize) as *mut _);
            log::debug!("-> {r}");

            log::debug!("fix_ui()");
            let fix_ui: unsafe extern "stdcall" fn() =
                std::mem::transmute(addr(0x1940) as *const ());
            (fix_ui)();

            // clear save path + update window title

            log::debug!("clear_save_path()");
            let clear_save_path: unsafe extern "fastcall" fn(this: *mut libc::c_void) =
                std::mem::transmute(addr(0x1d0d0) as *const ());
            (clear_save_path)(addr(0xa3598) as *mut _);

            log::debug!("set_window_title_path({cstr:?})");
            let set_window_title_path: unsafe extern "cdecl" fn(path: winapi::um::winnt::LPCSTR) =
                std::mem::transmute(addr(0x3ad0) as *const ());
            (set_window_title_path)(cstr.as_ptr());

            log::debug!("done.");

            // // alt: read from path, adds to history
            // let read_file2: unsafe extern "cdecl" fn(
            //     hwnd: HWND,
            //     path: *const libc::c_char,
            //     a: *mut bool,
            //     b: *mut bool,
            // ) -> u32 = std::mem::transmute(addr(0x2590) as *const ());

            // let cstr = path.to_str().and_then(|p| CString::new(p).ok()).unwrap();
            // let mut a = false;
            // let mut b = false;
            // let _r = (read_file2)(*Self::get_hwnd(), cstr.as_ptr(), &mut a, &mut b);
            // log::debug!("read_file2 => {r}");
        }
    }

    fn volume_adjust_fill_selected_units(hwnd: HWND) -> bool {
        unsafe {
            let fill_selected_units: unsafe extern "cdecl" fn(hwnd: HWND) -> bool =
                std::mem::transmute(addr(0x190b0) as *const ());
            (fill_selected_units)(hwnd)
        }
    }
}
