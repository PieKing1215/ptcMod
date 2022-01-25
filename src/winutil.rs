#![allow(dead_code)]

use std::{cell::Cell, ffi::CString};

use winapi::{
    shared::windef::{HMENU, HWND},
    um::winuser,
};

// system for assigning globally unique menu ids without hardcoded constants
static mut MENU_ID_COUNTER: Cell<u16> = Cell::new(1000);

pub(crate) fn next_id() -> u16 {
    unsafe {
        MENU_ID_COUNTER.set(MENU_ID_COUNTER.get() + 1);
        MENU_ID_COUNTER.get()
    }
}

// utility type for accepting either a direct HMENU or taking an HWND and using winuser::GetMenu to get its HMENU

pub(crate) trait GetHMENU {
    fn get_hmenu(self) -> HMENU;
}

impl GetHMENU for HWND {
    fn get_hmenu(self) -> HMENU {
        unsafe { winuser::GetMenu(self) }
    }
}

impl GetHMENU for HMENU {
    fn get_hmenu(self) -> HMENU {
        self
    }
}

// utility functions for getting/setting menu item properties

pub(crate) fn get_menu_checked(menu: impl GetHMENU, id: impl Into<u32>) -> bool {
    let id = id.into();
    unsafe {
        winuser::GetMenuState(menu.get_hmenu(), id, winuser::MF_BYCOMMAND) & winuser::MF_CHECKED > 0
    }
}

pub(crate) fn set_menu_checked(menu: impl GetHMENU, id: impl Into<u32>, checked: bool) {
    let id = id.into();
    unsafe {
        winuser::CheckMenuItem(
            menu.get_hmenu(),
            id,
            winuser::MF_BYCOMMAND
                | if checked {
                    winuser::MF_CHECKED
                } else {
                    winuser::MF_UNCHECKED
                },
        );
    }
}

pub(crate) fn get_menu_enabled(menu: impl GetHMENU, id: impl Into<u32>) -> bool {
    let id = id.into();
    unsafe {
        winuser::GetMenuState(menu.get_hmenu(), id, winuser::MF_BYCOMMAND) & winuser::MF_GRAYED == 0
    }
}

pub(crate) fn set_menu_enabled(menu: impl GetHMENU, id: impl Into<u32>, enabled: bool) {
    let id = id.into();
    unsafe {
        winuser::EnableMenuItem(
            menu.get_hmenu(),
            id,
            winuser::MF_BYCOMMAND
                | if enabled {
                    winuser::MF_ENABLED
                } else {
                    winuser::MF_GRAYED
                },
        );
    }
}

/// Handles toggling the state of a menu toggle
/// Returns true if the menu is now checked
pub(crate) fn menu_toggle(menu: impl GetHMENU + Copy, id: impl Into<u32>) -> bool {
    let id = id.into();
    let was_checked = get_menu_checked(menu, id);
    set_menu_checked(menu, id, !was_checked);
    !was_checked
}

/// Appends a new toggleable menu item to the given HMENU
pub(crate) fn add_menu_toggle(
    menu: HMENU,
    name: impl Into<String>,
    id: impl Into<usize>,
    checked: bool,
    enabled: bool,
) {
    let id = id.into();
    let l_title = CString::new(name.into()).unwrap();
    unsafe {
        winuser::AppendMenuA(menu, winuser::MF_CHECKED, id, l_title.as_ptr().cast::<i8>());
    }

    set_menu_checked(menu, id as u32, checked);
    set_menu_enabled(menu, id as u32, enabled);
}
