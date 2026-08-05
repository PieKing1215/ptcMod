use std::{
    ffi::CString,
    fs::File,
    path::PathBuf,
    ptr,
    string::ToString,
    sync::{
        LazyLock,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use regex::Regex;
use windows::{
    Win32::{
        Foundation::{HWND, POINTL},
        Graphics::Gdi::InvalidateRect,
        System::{
            Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL},
            Memory::{GlobalLock, GlobalUnlock},
            Ole::{
                CF_TEXT, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, IDropTarget,
                IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium,
                RevokeDragDrop,
            },
        },
        UI::WindowsAndMessaging::{MSG, WM_COMMAND},
    },
    core::implement,
};

use crate::{
    ptc::PTCVersion,
    winutil::{self, Menus, hiword, loword},
};

use super::Feature;

static M_DRAGDROP_ID: LazyLock<u16> = LazyLock::new(winutil::next_id);

#[expect(clippy::ref_as_ptr, reason = "macro")]
#[expect(clippy::inline_always, reason = "macro")]
mod data {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[implement(IDropTarget)]
    pub struct DropHandlerData {
        pub load_file_fn: fn(PathBuf),
        pub get_hwnd_fn: fn() -> &'static mut HWND,
        pub state: AtomicU32,
    }
}
#[allow(clippy::wildcard_imports)]
use data::*;

impl Clone for DropHandlerData {
    fn clone(&self) -> Self {
        Self {
            load_file_fn: self.load_file_fn,
            get_hwnd_fn: self.get_hwnd_fn,
            state: self.state.load(Ordering::Relaxed).into(),
        }
    }
}

pub struct DragAndDrop {
    pub data: DropHandlerData,
}

impl DragAndDrop {
    pub fn new<PTC: PTCVersion>() -> Self {
        let data = DropHandlerData {
            load_file_fn: PTC::load_file_no_history,
            get_hwnd_fn: PTC::get_hwnd,
            state: DROPEFFECT_NONE.0.into(),
        };

        // needed since we're using reqwest's "rustls-no-provider" feature to avoid aws-lc (since it complicates building on linux)
        rustls::crypto::ring::default_provider()
            .install_default()
            .unwrap();

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
            let _ = RevokeDragDrop(*PTC::get_hwnd());
        }
    }

    fn win_msg(&mut self, msg: &MSG) {
        if msg.message == WM_COMMAND {
            let high = hiword(msg.wParam.0.try_into().unwrap());
            let low = loword(msg.wParam.0.try_into().unwrap());

            #[allow(clippy::collapsible_if)]
            if high == 0 {
                if low == *M_DRAGDROP_ID {
                    if winutil::menu_toggle(msg.hwnd, *M_DRAGDROP_ID) {
                        unsafe {
                            let r = OleInitialize(None);
                            if let Err(e) = r {
                                log::error!("OleInitialize failed: {e}");
                                return;
                            }
                            RegisterDragDrop(
                                *PTC::get_hwnd(),
                                &IDropTarget::from(self.data.clone()),
                            )
                            .unwrap();
                        }
                    } else {
                        unsafe {
                            RevokeDragDrop(*PTC::get_hwnd()).unwrap();
                        }
                    }
                }
            }
        }
    }
}

// IDropTarget

impl IDropTarget_Impl for DropHandlerData_Impl {
    fn DragEnter(
        &self,
        pdataobj: windows::core::Ref<windows::Win32::System::Com::IDataObject>,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        let text = unsafe { get_text(pdataobj.unwrap()) };

        if text.is_some() {
            unsafe {
                *pdweffect = DROPEFFECT_COPY;
            }
            self.state.store(DROPEFFECT_COPY.0, Ordering::Relaxed);
        }

        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            *pdweffect = DROPEFFECT(self.state.load(Ordering::Relaxed));
        }

        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: windows::core::Ref<windows::Win32::System::Com::IDataObject>,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        _pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        // if the dropped item is text and is a ptweb url, get the id
        // (the capture group for id explicitly allows '/' so it can match private urls)
        let re = Regex::new(r"^https?://(?:www\.)?ptweb\.me/(?:play|get|full)/([a-zA-Z0-9/]+)$")
            .unwrap();
        let var_name = unsafe { get_text(pdataobj.unwrap()) };
        let id = var_name.as_ref().and_then(|txt| {
            re.captures(txt.as_str())
                .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        });
        log::debug!("Drop {var_name:?} -> {id:?}");

        if let Some(id) = id {
            // format url for download
            let url = format!("https://www.ptweb.me/get/{id}");

            log::info!("GET {url}");

            let res = reqwest::blocking::Client::builder()
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION"),
                ))
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(|e| format!("Failed to build http client: {e:?}"))
                .and_then(|client| {
                    client
                        .get(url)
                        .send()
                        .map_err(|e| format!("GET request failed: {e:?}"))
                })
                .and_then(|resp: reqwest::blocking::Response| {
                    // got a response, check if 200
                    if resp.status() == reqwest::StatusCode::OK {
                        Ok(resp)
                    } else {
                        Err(format!("Response was: {resp:#?}"))
                    }
                })
                .and_then(|resp| {
                    log::debug!("Creating temp folder...");

                    // make temp folder
                    let mut pb = PathBuf::new();
                    pb.push("ptweb/");
                    std::fs::create_dir_all(pb.clone())
                        .map_err(|e| format!("Failed to create dirs: {e:?}"))
                        .map(|()| (resp, pb))
                })
                .and_then(|(resp, mut pb)| {
                    log::debug!("Getting file name...");

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

                    log::debug!("Creating temp file {fname}...");

                    // make file in the temp folder
                    pb.push(fname);
                    File::create(pb.clone())
                        .map_err(|e| format!("Failed to create file: {e:?}"))
                        .map(|f| (resp, f, pb))
                })
                .and_then(|(resp, mut f, pb)| {
                    log::debug!("Reading response bytes...");

                    let bytes = &mut resp
                        .bytes()
                        .map_err(|e| format!("Failed to get response bytes: {e:?}"))?;

                    log::debug!("Writing temp file...");

                    // copy payload bytes into file
                    std::io::copy(&mut bytes.as_ref(), &mut f)
                        .map_err(|e| format!("Failed to write file: {e:?}"))
                        .map(|_| pb)
                })
                .and_then(|pb| {
                    log::info!("Downloaded file!");
                    if pb.exists() {
                        Ok(pb)
                    } else {
                        #[expect(clippy::unnecessary_debug_formatting, reason = "false positive")]
                        Err(format!("File still doesn't exist: {pb:?}"))
                    }
                });

            match res {
                Err(msg) => log::error!("{msg}"),
                Ok(pb) => {
                    // load the file into ptCollage
                    log::info!("Loading file...");
                    (self.load_file_fn)(pb.clone());
                    unsafe {
                        InvalidateRect(Some(*(self.get_hwnd_fn)()), None, false).unwrap();
                    }
                    log::info!("Loaded.");
                    // remove the temp file
                    log::info!("remove_file: {:?}", std::fs::remove_file(pb));
                    log::info!("Deleted tempfile.");
                },
            }
        }

        Ok(())
    }
}

unsafe fn get_text(p_data_obj: &IDataObject) -> Option<String> {
    // try get unicode text first (eg. wine seems to only implement unicode text?)
    let format = FORMATETC {
        cfFormat: CF_UNICODETEXT.0,
        ptd: ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as _,
    };

    unsafe {
        let r = p_data_obj.GetData(&raw const format);
        if let Ok(mut storage) = r {
            let ptr = GlobalLock(storage.u.hGlobal) as *const u16;
            let txt = widestring::U16CStr::from_ptr_str(ptr);
            let str = txt.to_string_lossy();

            let _ = GlobalUnlock(storage.u.hGlobal);
            ReleaseStgMedium(&raw mut storage);
            Some(str)
        } else {
            // try get as plain text as fallback

            let format = FORMATETC { cfFormat: CF_TEXT.0, ..format };

            let r = p_data_obj.GetData(&raw const format);
            if let Ok(mut storage) = r {
                let ptr = GlobalLock(storage.u.hGlobal).cast::<i8>();
                let txt = CString::from_raw(ptr);
                let str = txt.to_str().unwrap().to_string();

                let _ = GlobalUnlock(storage.u.hGlobal);
                ReleaseStgMedium(&raw mut storage);
                Some(str)
            } else {
                None
            }
        }
    }
}
