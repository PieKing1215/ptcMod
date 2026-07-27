use std::ffi::c_void;

use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::MapWindowPoints,
    UI::WindowsAndMessaging::{GetDlgItem, GetWindowRect, MSG, SWP_NOZORDER, SetWindowPos},
};

use crate::{patch::Patch, ptc::PTCVersion, winutil::Menus};

use super::Feature;

pub struct DialogInputWidth {
    patch: Patch,
}

impl DialogInputWidth {
    pub fn new<PTC: PTCVersion>(patch: Patch) -> Self {
        Self { patch }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for DialogInputWidth {
    fn init(&mut self, _menus: &mut Menus) {
        unsafe {
            if let Err(e) = self.patch.apply() {
                log::warn!("dialog input width patch: {e:?}");
            }
        }
    }

    fn cleanup(&mut self) {
        unsafe {
            if let Err(e) = self.patch.unapply() {
                log::warn!("dialog input width patch: {e:?}");
            }
        }
    }

    fn win_msg(&mut self, _msg: &MSG) {}
}

// (the edit field is called volume but is used for all of the bulk velocity/pan/volume/transpose menus)
const IDC_VOLUME: i32 = 1129;
const IDC_SPIN_VALUE: i32 = 1107;

pub(crate) unsafe fn modify_dialog(h_dlg: *mut c_void) {
    let h_dlg = HWND(h_dlg);

    // grow the edit field
    let h_edit = GetDlgItem(Some(h_dlg), IDC_VOLUME).unwrap();

    let mut rc = RECT::default();
    GetWindowRect(h_edit, &raw mut rc).unwrap();

    MapWindowPoints(
        None,
        Some(h_dlg),
        std::slice::from_raw_parts_mut((&raw mut rc).cast::<POINT>(), 2),
    );

    let grow = 4;
    SetWindowPos(
        h_edit,
        None,
        rc.left - grow,
        rc.top,
        rc.right - rc.left + grow * 2,
        rc.bottom - rc.top,
        SWP_NOZORDER,
    )
    .unwrap();

    // reposition the up/down spinner
    let h_spin = GetDlgItem(Some(h_dlg), IDC_SPIN_VALUE).unwrap();

    GetWindowRect(h_spin, &raw mut rc).unwrap();

    MapWindowPoints(
        None,
        Some(h_dlg),
        std::slice::from_raw_parts_mut((&raw mut rc).cast::<POINT>(), 2),
    );

    SetWindowPos(
        h_spin,
        None,
        rc.left + grow,
        rc.top,
        rc.right - rc.left,
        rc.bottom - rc.top,
        SWP_NOZORDER,
    )
    .unwrap();
}
