pub mod custom_note_rendering;
pub mod custom_scroll;
pub mod fps_unlock;

use winapi::{shared::windef::HMENU, um::winuser::MSG};

use crate::ptc::PTCVersion;

pub trait Feature<PTC: PTCVersion> {
    fn init(&mut self, menu: HMENU);
    fn cleanup(&mut self);
    fn win_msg(&mut self, msg: &MSG) -> bool;
}
