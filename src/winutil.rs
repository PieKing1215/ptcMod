#![allow(dead_code)]

use std::{collections::HashMap, ffi::CString, sync::atomic::AtomicU16};

use windows::{
    Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            AppendMenuA, CheckMenuItem, CreateMenu, EnableMenuItem, GetMenu, GetMenuState, HMENU,
            MENU_ITEM_FLAGS, MF_BYCOMMAND, MF_BYPOSITION, MF_CHECKED, MF_ENABLED, MF_GRAYED,
            MF_POPUP, MF_UNCHECKED, RemoveMenu,
        },
    },
    core::PCSTR,
};

use crate::ptc::PTCVersion;

// system for assigning globally unique menu ids without hardcoded constants
static MENU_ID_COUNTER: AtomicU16 = AtomicU16::new(1000);

pub(crate) fn next_id() -> u16 {
    MENU_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

pub struct Menus {
    map: HashMap<String, HMENU>,
}

impl Menus {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn get_or_create<PTC: PTCVersion>(&mut self, display: &str) -> HMENU {
        *self
            .map
            .entry(display.to_string())
            .or_insert_with(|| unsafe {
                let h_menu = GetMenu(*PTC::get_hwnd());
                let menu = CreateMenu().unwrap();
                let l_title: Vec<u8> = format!("{display}\0").bytes().collect();
                AppendMenuA(h_menu, MF_POPUP, menu.0 as usize, PCSTR(l_title.as_ptr())).unwrap();
                menu
            })
    }

    pub fn get_default<PTC: PTCVersion>(&mut self) -> HMENU {
        self.get_or_create::<PTC>("PTC Mod")
    }

    pub fn cleanup<PTC: PTCVersion>(self) {
        for _ in self.map.keys() {
            unsafe {
                RemoveMenu(GetMenu(*PTC::get_hwnd()), 4, MF_BYPOSITION).unwrap();
            }
        }
    }
}

// utility type for accepting either a direct HMENU or taking an HWND and using GetMenu to get its HMENU

pub(crate) trait GetHMENU {
    fn get_hmenu(self) -> HMENU;
}

impl GetHMENU for HWND {
    fn get_hmenu(self) -> HMENU {
        unsafe { GetMenu(self) }
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
        MENU_ITEM_FLAGS(GetMenuState(menu.get_hmenu(), id, MF_BYCOMMAND)).contains(MF_CHECKED)
    }
}

pub(crate) fn set_menu_checked(menu: impl GetHMENU, id: impl Into<u32>, checked: bool) {
    let id = id.into();
    unsafe {
        CheckMenuItem(
            menu.get_hmenu(),
            id,
            (MF_BYCOMMAND | if checked { MF_CHECKED } else { MF_UNCHECKED }).0,
        );
    }
}

pub(crate) fn get_menu_enabled(menu: impl GetHMENU, id: impl Into<u32>) -> bool {
    let id = id.into();
    unsafe { MENU_ITEM_FLAGS(GetMenuState(menu.get_hmenu(), id, MF_BYCOMMAND)).contains(MF_GRAYED) }
}

pub(crate) fn set_menu_enabled(menu: impl GetHMENU, id: impl Into<u32>, enabled: bool) {
    let id = id.into();
    unsafe {
        let _ = EnableMenuItem(
            menu.get_hmenu(),
            id,
            MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_GRAYED },
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
        AppendMenuA(menu, MF_CHECKED, id, PCSTR(l_title.as_ptr().cast())).unwrap();
    }

    set_menu_checked(menu, id as u32, checked);
    set_menu_enabled(menu, id as u32, enabled);
}

/// Appends a new buttom menu item to the given HMENU
pub(crate) fn add_menu_button(
    menu: HMENU,
    name: impl Into<String>,
    id: impl Into<usize>,
    enabled: bool,
) {
    let id = id.into();
    let l_title = CString::new(name.into()).unwrap();
    unsafe {
        AppendMenuA(
            menu,
            MENU_ITEM_FLAGS::default(),
            id,
            PCSTR(l_title.as_ptr().cast()),
        )
        .unwrap();
    }

    set_menu_enabled(menu, id as u32, enabled);
}

#[inline]
pub fn loword(dw: u32) -> u16 {
    (dw & 0xffff) as u16
}

#[inline]
pub fn hiword(dw: u32) -> u16 {
    ((dw >> 16) & 0xffff) as u16
}
