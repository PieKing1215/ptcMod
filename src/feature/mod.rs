pub mod custom_note_rendering;

use winapi::um::winuser::MSG;

use crate::ptc::PTCVersion;

pub trait Feature<PTC: PTCVersion> {
    fn init(&mut self);
    fn cleanup(&mut self);
    fn win_msg(&mut self, msg: &MSG) -> bool;
}
