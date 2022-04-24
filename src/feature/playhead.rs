use winapi::um::winuser;

use crate::{
    feature::scroll_hook,
    patch::Patch,
    ptc::PTCVersion,
    winutil::{self, Menus},
};

use super::Feature;

lazy_static::lazy_static! {
    static ref M_PLAYHEAD_ID: u16 = winutil::next_id();
}

pub struct Playhead {
    patch: Vec<Patch>,
}

impl Playhead {
    pub fn new<PTC: PTCVersion>(draw_unitkb_top_patch: Patch) -> Self {
        Self { patch: vec![draw_unitkb_top_patch] }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for Playhead {
    fn init(&mut self, menus: &mut Menus) {
        winutil::add_menu_toggle(
            menus.get_or_create::<PTC>("Rendering"),
            "Playhead",
            *M_PLAYHEAD_ID,
            false,
            false,
        );
    }

    fn cleanup(&mut self) {
        unsafe {
            for p in &self.patch {
                if let Err(e) = p.unapply() {
                    log::warn!("note_rect_hook_patch: {:?}", e);
                }
            }
        }
    }

    fn win_msg(&mut self, msg: &winapi::um::winuser::MSG) {
        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_PLAYHEAD_ID {
                    if winutil::menu_toggle(msg.hwnd, *M_PLAYHEAD_ID) {
                        for p in &self.patch {
                            unsafe { p.apply() }.unwrap();
                        }
                    } else {
                        for p in &self.patch {
                            unsafe { p.unapply() }.unwrap();
                        }
                    }
                } else if low == *scroll_hook::M_SCROLL_HOOK_ID {
                    let scroll_hook_enabled =
                        winutil::get_menu_checked(*PTC::get_hwnd(), *scroll_hook::M_SCROLL_HOOK_ID);
                    winutil::set_menu_enabled(
                        *PTC::get_hwnd(),
                        *M_PLAYHEAD_ID,
                        scroll_hook_enabled,
                    );
                }
            }
        }
    }
}

pub(crate) unsafe fn draw_unitkb_top<PTC: PTCVersion>() {
    if scroll_hook::ENABLED && PTC::is_playing() && *PTC::get_tab() > 0 {
        let unit_rect = PTC::get_unit_rect();

        let x = crate::feature::scroll_hook::LAST_PLAYHEAD_POS;

        let rect = [x, unit_rect[1], x + 2, unit_rect[3]];
        PTC::draw_rect(rect, 0xffcccccc);
    }
}
