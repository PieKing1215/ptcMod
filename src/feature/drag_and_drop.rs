use std::{ffi::CString, fs::File, intrinsics::transmute, path::PathBuf, ptr, string::ToString};

use regex::Regex;
use winapi::{
    shared::{
        guiddef::REFIID,
        minwindef::{DWORD, ULONG},
        windef::POINTL,
        wtypes::DVASPECT_CONTENT,
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

use crate::{
    ptc::PTCVersion,
    winutil::{self, Menus},
};

use super::Feature;

lazy_static::lazy_static! {
    static ref M_DRAGDROP_ID: u16 = winutil::next_id();
}

static mut VTABLE: Option<IDropTargetVtbl> = None;

pub struct DropHandlerData {
    drop_target: IDropTarget,
    state: DWORD,
}

pub struct DragAndDrop {
    pub data: DropHandlerData,
}

impl DragAndDrop {
    pub fn new<PTC: PTCVersion>() -> Self {
        unsafe {
            // the transmutes here are because the definitions in IDropTargetVtbl incorrectly
            //   use `*const POINTL` instead of `POINTL` which messes up pdw_effect
            // my functions correctly use POINTL so they need to be forced into place
            VTABLE = Some(IDropTargetVtbl {
                parent: IUnknownVtbl {
                    QueryInterface: query_interface,
                    AddRef: add_ref,
                    Release: release,
                },
                DragEnter: transmute::<*const (), _>(drag_enter as *const ()),
                DragOver: transmute::<*const (), _>(drag_over as *const ()),
                DragLeave: drag_leave,
                Drop: transmute::<*const (), _>(drop::<PTC> as *const ()),
            });
        }

        let data = DropHandlerData {
            drop_target: IDropTarget {
                lpVtbl: unsafe { VTABLE.as_ref() }.unwrap() as *const IDropTargetVtbl,
            },
            state: DROPEFFECT_NONE,
        };

        Self { data }
    }
}

impl<PTC: PTCVersion> Feature<PTC> for DragAndDrop {
    fn init(&mut self, menus: &mut Menus) {
        winutil::add_menu_toggle(
            menus.get_default::<PTC>(),
            "Drop URLs",
            *M_DRAGDROP_ID,
            false,
            true,
        );
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

unsafe extern "system" fn drop<PTC: PTCVersion>(
    _this: *mut IDropTarget,
    p_data_obj: *const IDataObject,
    _grf_key_state: DWORD,
    _pt: POINTL,
    _pdw_effect: *mut DWORD,
) -> HRESULT {
    // if the dropped item is text and is a ptweb url, get the id
    // (the capture group for id explicitly allows '/' so it can match private urls)
    let re = Regex::new(r"^https?://www\.ptweb\.me/(?:play|get|full)/([a-zA-Z0-9/]+)$").unwrap();
    let id = get_text(p_data_obj).as_ref().and_then(|txt| {
        re.captures(txt.as_str())
            .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
    });

    if let Some(id) = id {
        // format url for download
        let url = format!("https://www.ptweb.me/get/{id}");

        log::info!("GET {url}");

        let res = reqwest::blocking::get(url)
            .map_err(|e| format!("GET request failed: {e:?}"))
            .and_then(|resp| {
                // got a response, check if 200
                log::debug!("{resp:?}");

                if resp.status() == reqwest::StatusCode::OK {
                    Ok(resp)
                } else {
                    Err(format!("Response was: {}", resp.status()))
                }
            })
            .and_then(|resp| {
                // make temp folder
                let mut pb = PathBuf::new();
                pb.push("ptweb/");
                std::fs::create_dir_all(pb.clone())
                    .map_err(|e| format!("Failed to create dirs: {e:?}"))
                    .map(|_| (resp, pb))
            })
            .and_then(|(resp, mut pb)| {
                // try to extract filename from headers, otherwise use {id}.ptcop
                let fname = resp
                    .headers()
                    .get("content-disposition")
                    .and_then(|v| {
                        v.to_str().ok().and_then(|s| {
                            s.strip_prefix("attachment; filename=\"")
                                .and_then(|s| s.strip_suffix('\"'))
                        })
                    })
                    .map(ToString::to_string)
                    .unwrap_or(format!("{id}.ptcop"));

                // make file in the temp folder
                pb.push(fname);
                File::create(pb.clone())
                    .map_err(|e| format!("Failed to create file: {e:?}"))
                    .map(|f| (resp, f, pb))
            })
            .and_then(|(resp, mut f, pb)| {
                // copy payload bytes into file
                std::io::copy(&mut resp.bytes().unwrap().as_ref(), &mut f)
                    .map_err(|e| format!("Failed to write file: {e:?}"))
                    .map(|_| pb)
            })
            .and_then(|pb| {
                log::info!("Downloaded file!");
                if pb.exists() {
                    Ok(pb)
                } else {
                    Err(format!("File still doesn't exist: {pb:?}"))
                }
            });

        match res {
            Err(msg) => log::error!("{msg}"),
            Ok(pb) => {
                // load the file into ptCollage
                log::info!("Loading file...");
                PTC::load_file_no_history(pb.clone());
                winuser::InvalidateRect(*PTC::get_hwnd(), std::ptr::null(), 0);
                log::info!("Loaded.");
                // remove the temp file
                log::info!("remove_file: {:?}", std::fs::remove_file(pb));
                log::info!("Deleted tempfile.");
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
