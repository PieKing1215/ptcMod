pub mod custom_note_rendering;
pub mod fps_unlock;
pub mod scroll;

use winapi::{um::winuser::MSG, shared::windef::HMENU};

use crate::ptc::PTCVersion;

pub trait Feature<PTC: PTCVersion> {
    fn init(&mut self, menu: HMENU);
    fn cleanup(&mut self);
    fn win_msg(&mut self, msg: &MSG) -> bool;
}