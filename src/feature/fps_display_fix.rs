use crate::{patch::Patch, ptc::PTCVersion, winutil::Menus};

use super::Feature;

pub struct FPSDisplayFix {
    patch: Vec<Patch>,
}

impl FPSDisplayFix {
    pub fn new<PTC: PTCVersion>(
        digit_patch: Patch,
        number_x_patch: Patch,
        label_x_patch: Patch,
    ) -> Self {
        Self {
            patch: vec![digit_patch, number_x_patch, label_x_patch],
        }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for FPSDisplayFix {
    fn init(&mut self, _menus: &mut Menus) {
        unsafe {
            for p in &self.patch {
                if let Err(e) = p.apply() {
                    log::warn!("fps display patch: {:?}", e);
                }
            }
        }
    }

    fn cleanup(&mut self) {
        unsafe {
            for p in &self.patch {
                if let Err(e) = p.unapply() {
                    log::warn!("fps display patch: {:?}", e);
                }
            }
        }
    }

    fn win_msg(&mut self, _msg: &winapi::um::winuser::MSG) {}
}
