use widestring::U16CString;
use winapi::{shared::{minwindef::HINSTANCE, windef::{HWND}}};

use crate::{
    feature::{
        drag_and_drop::DragAndDrop,
        fps_display_fix::FPSDisplayFix,
        playhead::{self, Playhead},
        scroll_hook::{self, Scroll},
    },
    patch::{hook, hook_pre_ret_new, Patch},
};

use super::{addr, color_abgr_to_argb, color_argb_to_abgr, PTCVersion, Selection};

pub struct PTC09454;

impl PTCVersion for PTC09454 {
    fn get_features() -> Vec<Box<dyn crate::feature::Feature<Self>>> {
        // scroll hook

        let unit_clear_hook_patch =
            hook!(0x7920a, 0x78a60, "cdecl", fn(a: *mut f32), |_old_fn, _a| {
                scroll_hook::unit_clear::<PTC09454>();
            });

        let f_scroll_hook = Scroll::new::<Self>(unit_clear_hook_patch);

        // custom note rendering

        // let note_rect_push_ebp = Patch::new(0x74cb7, vec![0x51], vec![0x55]).unwrap();
        // let note_rect_hook_patch = hook!(
        //     0x74cc5,
        //     0x7570,
        //     "thiscall",
        //     fn(this: *mut (), rect: *const [f32; 4], ebp: u32),
        //     |_old_fn, _this, rect: *const [f32; 4], ebp| {
        //         let not_focused = *((ebp - 0xbc) as *mut u32) != 0;
        //         let unit = *((ebp - 0xcc) as *mut u32);
        //         // let not_focused = false;
        //         // let unit = 3;
        //         let rect = [
        //             (*rect)[0] as i32,
        //             (*rect)[1] as i32,
        //             (*rect)[2] as i32,
        //             (*rect)[3] as i32,
        //         ];
        //         custom_note_rendering::draw_unit_note_rect::<PTC09454>(
        //             rect.as_ptr(),
        //             unit,
        //             not_focused,
        //         );
        //     }
        // );

        // // for some reason NOPing the draw_image call directly crashes unlike in 0.9.2.5
        // // instead, this changes the conditional jumps around the draw_image calls into unconditional jumps
        // // so the draw_image is skipped
        // let note_disable_left_edge =
        //     Patch::new(0x74ce2, vec![0x72, 0x3e], vec![0xeb, 0x3e]).unwrap();
        // let note_disable_right_edge =
        //     Patch::new(0x74d31, vec![0x76, 0x41], vec![0xeb, 0x41]).unwrap();

        // let f_custom_note_rendering = CustomNoteRendering::new::<Self>(
        //     note_rect_push_ebp,
        //     note_rect_hook_patch,
        //     note_disable_left_edge,
        //     note_disable_right_edge,
        // );

        // playhead

        let draw_unitkb_top_patch = hook_pre_ret_new!(
            0x79331,
            0x62080,
            "stdcall",
            fn(),
            playhead::draw_unitkb_top::<PTC09454>
        );

        let f_playhead = Playhead::new::<Self>(draw_unitkb_top_patch);

        // fps display fix

        let digit_patch = Patch::new(0x794f0, vec![0x2], vec![0x3]).unwrap();

        let number_x_patch = hook!(
            0x79524,
            0x82e90,
            "cdecl",
            fn(x: f32, y: f32, num: i32, digit: u32),
            |func: unsafe extern "cdecl" fn(x: f32, y: f32, num: i32, digit: u32),
             x,
             y,
             num,
             digit| {
                func(x - 6.0, y, num, digit);
            }
        );

        let label_x_patch = hook!(
            0x794ea,
            0x6fc0,
            "thiscall",
            fn(this: *mut libc::c_void, x: f32, y: f32, p3: *mut libc::c_void, p4: u32),
            |func: unsafe extern "thiscall" fn(
                this: *mut libc::c_void,
                x: f32,
                y: f32,
                p3: *mut libc::c_void,
                p4: u32,
            ),
             this,
             x,
             y,
             p3,
             p4| {
                func(this, x - 6.0, y, p3, p4);
            }
        );

        let f_fps_display_fix =
            FPSDisplayFix::new::<Self>(digit_patch, number_x_patch, label_x_patch);

        vec![
            Box::new(f_scroll_hook),
            // Box::new(f_custom_note_rendering),
            Box::new(f_playhead),
            Box::new(DragAndDrop::new::<Self>()),
            Box::new(f_fps_display_fix),
        ]
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

    fn get_tab() -> &'static mut u32 {
        unsafe { &mut *(addr(0xc0210) as *mut u32) }
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
        .max(0)
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

    fn get_event_list() -> &'static mut super::events::EventList {
        todo!()
    }

    fn is_unit_highlighted(_unit_no: i32) -> bool {
        todo!()
    }

    fn get_selected_range() -> Selection {
        todo!()
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
            let color = color_argb_to_abgr(color);
            (draw_rect)(
                *(addr(0xbe03c) as *mut usize) as *mut (),
                f_rect.as_ptr(),
                color,
            );
        }
    }

    fn get_base_note_colors_argb() -> [u32; 2] {
        unsafe {
            let abgr = *(addr(0xbd6b0) as *mut [u32; 2]);
            [color_abgr_to_argb(abgr[0]), color_abgr_to_argb(abgr[1])]
        }
    }

    fn get_event_value_at_screen_pos(pos_x: i32, unit_no: i32, ev_type: i32) -> i32 {
        unsafe {
            let get_event_value: unsafe extern "cdecl" fn(
                pos_x: f32,
                unit_no: i32,
                ev_type: i32,
            ) -> i32 = std::mem::transmute(addr(0x601c0) as *const ());
            (get_event_value)(pos_x as f32, unit_no, ev_type)
        }
    }

    fn load_file_no_history(path: std::path::PathBuf) {
        unsafe {
            log::debug!("load_file_no_history({path:?})");

            // if we don't do this, it crashes with scroll hook
            log::debug!("stop_playing()");
            let stop_playing: unsafe extern "stdcall" fn() -> bool =
                std::mem::transmute(addr(0x59a00) as *const ());
            let r = (stop_playing)();
            log::debug!("-> {r}");

            // wfopen (0.9.4.54 uses wide strings)

            let mut cstr = U16CString::from_str(path.to_str().unwrap()).unwrap();
            let mut mode = U16CString::from_str("rb").unwrap();

            let wfopen: unsafe extern "cdecl" fn(
                fname: *mut libc::wchar_t,
                mode: *mut libc::wchar_t,
            ) -> *mut libc::FILE =
                std::mem::transmute(*(addr(0x9a20c) as *const usize) as *const ());

            log::debug!("wfopen({cstr:?}, {mode:?})");
            let file = (wfopen)(cstr.as_mut_ptr(), mode.as_mut_ptr());
            log::debug!("-> {file:?}");

            // read file

            let read_file: unsafe extern "thiscall" fn(
                this: *mut libc::c_void,
                file: *mut libc::FILE,
            ) -> u32 = std::mem::transmute(addr(0x8ad90) as *const ());

            log::debug!("read_file(...)");
            let r = (read_file)(*(addr(0xbe020) as *mut usize) as *mut _, file);
            log::debug!("-> {r}");

            // fclose

            let fclose: unsafe extern "cdecl" fn(mode: *mut libc::FILE) -> libc::c_int =
                std::mem::transmute(*(addr(0x9a204) as *const usize) as *const ());

            log::debug!("fclose({file:?})");
            let r = (fclose)(file);
            log::debug!("-> {r}");

            let fn_1: unsafe extern "fastcall" fn(this: *mut libc::c_void) -> u32 =
                std::mem::transmute(addr(0x8b010) as *const ());

            // extra functions that need to be called

            log::debug!("fn_1(...)");
            let r = (fn_1)(*(addr(0xbe020) as *mut usize) as *mut _);
            log::debug!("-> {r}");

            log::debug!("fix_ui()");
            let fix_ui: unsafe extern "stdcall" fn() =
                std::mem::transmute(addr(0x56170) as *const ());
            (fix_ui)();

            // clear save path + update window title

            let clear_save_path: unsafe extern "fastcall" fn(this: *mut libc::c_void) =
                std::mem::transmute(addr(0x2160) as *const ());

            // some places in the decomp call this function with different parameters (only 0.9.4.54)
            // the second one clears the save path

            // log::debug!("clear_save_path(1)");
            // (clear_save_path)(addr(0xbe040) as *mut _);
            log::debug!("clear_save_path(2)");
            (clear_save_path)(*(addr(0xbe044) as *mut usize) as *mut _);
            // log::debug!("clear_save_path(3)");
            // (clear_save_path)(addr(0xbe048) as *mut _);
            // log::debug!("clear_save_path(4)");
            // (clear_save_path)(addr(0xbe04c) as *mut _);

            log::debug!("set_window_title_path({cstr:?})");
            let set_window_title_path: unsafe extern "cdecl" fn(path: winapi::um::winnt::LPCWSTR) =
                std::mem::transmute(addr(0x59650) as *const ());
            (set_window_title_path)(cstr.as_ptr());

            log::debug!("done.");

            // // alt: read from path, adds to history
            // let read_file2: unsafe extern "cdecl" fn(
            //     hwnd: HWND,
            //     path: *const libc::wchar_t,
            //     a: *mut bool,
            //     b: *mut bool,
            // ) -> u32 = std::mem::transmute(addr(0x56540) as *const ());

            // let path = path.as_ref();
            // let cstr = path.to_str().and_then(|p| U16CString::from_str(p).ok()).unwrap();
            // let mut a = false;
            // let mut b = false;
            // let _r = (read_file2)(*Self::get_hwnd(), cstr.as_ptr(), &mut a, &mut b);
            // log::debug!("read_file2 => {r}");
        }
    }

    fn volume_adjust_fill_selected_units(_hwnd: HWND) -> bool {
        todo!()
    }

    fn get_unit_scroll_ofs_x() -> &'static i32 {
        todo!()
    }

    fn get_unit_scroll_ofs_y() -> &'static i32 {
        todo!()
    }

    fn get_unit_num() -> i32 {
        todo!()
    }

    fn get_events_for_unit(_unit_no: i32) -> &'static [super::events::Event] {
        todo!()
    }
}
