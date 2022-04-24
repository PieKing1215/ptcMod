use winapi::um::winuser;

use crate::{
    patch::Patch,
    ptc::{addr, PTCVersion},
    winutil::{self, Menus},
};

use super::Feature;

lazy_static::lazy_static! {
    static ref M_FPS_UNLOCK_ID: u16 = winutil::next_id();
}

pub struct FPSUnlock {
    patch: Vec<Patch>,
}

impl FPSUnlock {
    pub fn new<PTC: PTCVersion>() -> Self {
        // the original source is like
        // do {
        //     Sleep(1);
        // } while(not enough time passed);

        // push 1 -> push 0
        let sleep_patch = Patch::new(0x167f3, vec![0x01], vec![0x00]).unwrap();

        // jc (label) -> nop nop
        let loop_patch = Patch::new(0x16808, vec![0x72, 0xe8], vec![0x90, 0x90]).unwrap();

        Self { patch: vec![sleep_patch, loop_patch] }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for FPSUnlock {
    fn init(&mut self, menus: &mut Menus) {
        winutil::add_menu_toggle(
            menus.get_default::<PTC>(),
            "FPS Unlock",
            *M_FPS_UNLOCK_ID,
            false,
            true,
        );
    }

    fn cleanup(&mut self) {
        unsafe {
            for p in &self.patch {
                if let Err(e) = p.unapply() {
                    log::warn!("note_rect_hook_patch: {:?}", e);
                }
            }

            // set wait time to tick time (otherwise it waits for it to catch up)
            *(addr(0xa6fa4) as *mut u32) = *(addr(0xa6fa0) as *mut u32);
        }
    }

    fn win_msg(&mut self, msg: &winapi::um::winuser::MSG) {
        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_FPS_UNLOCK_ID {
                    if winutil::menu_toggle(msg.hwnd, *M_FPS_UNLOCK_ID) {
                        for p in &self.patch {
                            unsafe { p.apply() }.unwrap();
                        }
                    } else {
                        for p in &self.patch {
                            unsafe { p.unapply() }.unwrap();
                        }

                        unsafe {
                            // set wait time to tick time (otherwise it waits for it to catch up)
                            *(addr(0xa6fa4) as *mut u32) = *(addr(0xa6fa0) as *mut u32);
                        }
                    }
                }
            }
        }
    }
}
