pub mod custom_note_rendering;
pub mod drag_and_drop;
pub mod fps_display_fix;
pub mod fps_unlock;
pub mod playhead;
pub mod scroll_hook;

use winapi::um::winuser::MSG;

use crate::{ptc::PTCVersion, winutil::Menus};

pub trait Feature<PTC: PTCVersion> {
    fn init(&mut self, menus: &mut Menus);
    fn cleanup(&mut self);
    fn win_msg(&mut self, msg: &MSG);
}
