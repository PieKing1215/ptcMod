use std::{ffi::CString, intrinsics::transmute, ptr};

use regex::Regex;
use winapi::{
    shared::{
        guiddef::REFIID,
        minwindef::{DWORD, ULONG},
        windef::POINTL,
        wtypes::{CLIPFORMAT, DVASPECT_CONTENT},
    },
    um::{
        objidl::{IDataObject, FORMATETC, TYMED_HGLOBAL},
        ole2::{OleInitialize, RegisterDragDrop, RevokeDragDrop},
        oleidl::{IDropTarget, IDropTargetVtbl, DROPEFFECT_COPY, DROPEFFECT_NONE},
        unknwnbase::{IUnknown, IUnknownVtbl},
        winnt::HRESULT,
        winuser::{self, CF_TEXT},
    },
};

use crate::{ptc::PTCVersion, winutil};

use super::Feature;

lazy_static::lazy_static! {
    static ref M_DRAGDROP_ID: u16 = winutil::next_id();
}

// the transmutes here are because the definitions in IDropTargetVtbl incorrectly
//   use `*const POINTL` instead of `POINTL` which messes up pdw_effect
// my functions correctly use POINTL so they need to be forced into place
static VTABLE: IDropTargetVtbl = IDropTargetVtbl {
    parent: IUnknownVtbl {
        QueryInterface: query_interface,
        AddRef: add_ref,
        Release: release,
    },
    DragEnter: unsafe { transmute::<*const (), _>(drag_enter as *const ()) },
    DragOver: unsafe { transmute::<*const (), _>(drag_over as *const ()) },
    DragLeave: drag_leave,
    Drop: unsafe { transmute::<*const (), _>(drop as *const ()) },
};

pub struct DropHandlerData {
    drop_target: IDropTarget,
    state: DWORD,
}

pub struct DragAndDrop {
    pub data: DropHandlerData,
}

impl DragAndDrop {
    pub fn new<PTC: PTCVersion>() -> Self {
        let data = DropHandlerData {
            drop_target: IDropTarget { lpVtbl: std::ptr::addr_of!(VTABLE) },
            state: DROPEFFECT_NONE,
        };

        Self { data }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for DragAndDrop {
    fn init(&mut self, menu: winapi::shared::windef::HMENU) {
        winutil::add_menu_toggle(menu, "Drop URLs", *M_DRAGDROP_ID, false, true);
    }

    fn cleanup(&mut self) {
        unsafe {
            RevokeDragDrop(*PTC::get_hwnd());
        }
    }

    fn win_msg(&mut self, msg: &winapi::um::winuser::MSG) {
        if msg.message == winuser::WM_COMMAND {
            let high = winapi::shared::minwindef::HIWORD(msg.wParam.try_into().unwrap());
            let low = winapi::shared::minwindef::LOWORD(msg.wParam.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_DRAGDROP_ID {
                    if winutil::menu_toggle(msg.hwnd, *M_DRAGDROP_ID) {
                        unsafe {
                            let r = OleInitialize(ptr::null_mut());
                            if r >= 0 {
                                RegisterDragDrop(*PTC::get_hwnd(), &mut self.data.drop_target);
                            } else {
                                log::error!("OleInitialize failed: {}", r);
                            }
                        }
                    } else {
                        unsafe {
                            RevokeDragDrop(*PTC::get_hwnd());
                        }
                    }
                }
            }
        }
    }
}

// IUnknown

unsafe extern "system" fn query_interface(
    _this: *mut IUnknown,
    _riid: REFIID,
    _ppv_object: *mut *mut libc::c_void,
) -> HRESULT {
    unimplemented!();
}

unsafe extern "system" fn add_ref(_this: *mut IUnknown) -> ULONG {
    // not really sure how bad this is but it seems unnecessary when I'm the one handling the memory
    1
}

unsafe extern "system" fn release(_this: *mut IUnknown) -> ULONG {
    // not really sure how bad this is but it seems unnecessary when I'm the one handling the memory
    1
}

// IDropTarget

unsafe extern "system" fn drag_enter(
    this: *mut IDropTarget,
    p_data_obj: *const IDataObject,
    _grf_key_state: DWORD,
    _pt: POINTL,
    pdw_effect: *mut DWORD,
) -> HRESULT {
    let data = &mut *this.cast::<DropHandlerData>();

    let text = get_text(p_data_obj);

    if text.is_some() {
        *pdw_effect = DROPEFFECT_COPY;
        data.state = DROPEFFECT_COPY;
    }

    0
}

unsafe extern "system" fn drag_over(
    this: *mut IDropTarget,
    _grf_key_state: DWORD,
    _pt: POINTL,
    pdw_effect: *mut DWORD,
) -> HRESULT {
    let data = &mut *this.cast::<DropHandlerData>();

    *pdw_effect = data.state;

    0
}

unsafe extern "system" fn drag_leave(_this: *mut IDropTarget) -> HRESULT {
    0
}

unsafe extern "system" fn drop(
    _this: *mut IDropTarget,
    p_data_obj: *const IDataObject,
    _grf_key_state: DWORD,
    _pt: POINTL,
    _pdw_effect: *mut DWORD,
) -> HRESULT {
    if let Some(txt) = get_text(p_data_obj) {
        let re =
            Regex::new(r"^https?://www\.ptweb\.me/(?:play|get|full)/([a-zA-z0-9/]+)$").unwrap();
        if let Some(cap) = re.captures(txt.as_str()) {
            if let Some(id) = cap.get(1).map(|m| m.as_str()) {
                let url = format!("https://www.ptweb.me/get/{id}");
                log::info!("GET {url}");
            }
        }
    }

    0
}

unsafe fn get_text(p_data_obj: *const IDataObject) -> Option<String> {
    let format = FORMATETC {
        cfFormat: CF_TEXT as u16,
        ptd: ptr::null(),
        dwAspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed: TYMED_HGLOBAL,
    };

    let mut storage = std::mem::zeroed();
    let r = (*p_data_obj).GetData(&format, &mut storage);
    if r >= 0 {
        let data = (*storage.u).hGlobal();
        let txt = CString::from_raw((*data).cast::<i8>());
        let str = txt.to_str().unwrap().to_string();
        Some(str)
    } else {
        None
    }
}
