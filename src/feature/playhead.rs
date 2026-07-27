use std::sync::LazyLock;

use windows::Win32::UI::WindowsAndMessaging::{MSG, WM_COMMAND};

use crate::{
    feature::scroll_hook,
    patch::Patch,
    ptc::PTCVersion,
    winutil::{self, Menus, hiword, loword},
};

use super::Feature;

static M_PLAYHEAD_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);

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
                    log::warn!("note_rect_hook_patch: {e:?}");
                }
            }
        }
    }

    fn win_msg(&mut self, msg: &MSG) {
        if msg.message == WM_COMMAND {
            let high = hiword(msg.wParam.0.try_into().unwrap());
            let low = loword(msg.wParam.0.try_into().unwrap());

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
        let kb_rect = PTC::get_kb_rect();

        let x = crate::feature::scroll_hook::LAST_PLAYHEAD_POS;

        let rect = [
            x,
            unit_rect.top.min(kb_rect.top),
            x + 2,
            unit_rect.bottom.max(kb_rect.bottom),
        ];
        PTC::draw_rect(rect, 0xffcccccc);
    }
}
