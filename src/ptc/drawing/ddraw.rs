use winapi::shared::{minwindef::DWORD, windef::LPRECT};

use super::{color::Color, Draw, Rect};

pub struct IDirectDrawSurface {
    raw: *mut libc::c_void,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(clippy::unused_self)]
impl IDirectDrawSurface {
    pub unsafe fn wrap(raw: *mut libc::c_void) -> Self {
        Self { raw }
    }

    pub unsafe fn QueryInterface(&self) {}

    pub unsafe fn AddRef(&self) {}

    pub unsafe fn Release(&self) {}

    pub unsafe fn AddAttachedSurface(&self) {}

    pub unsafe fn AddOverlayDirtyRect(&self) {}

    pub unsafe fn blt(
        &self,
        dst: LPRECT,
        unknown: *mut libc::c_void,
        src: LPRECT,
        flags: DWORD,
        bltfx: *mut DDBLTFX,
    ) {
        #[allow(clippy::ptr_as_ptr)]
        let raw_fn: unsafe extern "stdcall" fn(
            this: *mut libc::c_void,
            dst: LPRECT,
            unknown: *mut libc::c_void,
            src: LPRECT,
            flags: u32,
            ddbltfx: *mut DDBLTFX,
        ) = std::mem::transmute(*((*(self.raw as *mut usize) + 0x14) as *const *const ()));
        (raw_fn)(self.raw, dst, unknown, src, flags, bltfx);
    }

    pub unsafe fn blt_batch(&self, batch_array: *const DDBLTBATCH, batch_size: DWORD) {
        // BltBatch is unimplemented in ddraw.dll so this doesn't do anything

        #[allow(clippy::ptr_as_ptr)]
        let raw_fn: unsafe extern "stdcall" fn(
            this: *mut libc::c_void,
            batch_array: *const DDBLTBATCH,
            batch_size: DWORD,
            unused_zero: DWORD,
        ) = std::mem::transmute(*((*(self.raw as *mut usize) + 0x18) as *const *const ()));
        (raw_fn)(self.raw, batch_array, batch_size, 0);
    }

    pub unsafe fn blt_fast(
        &self,
        x: DWORD,
        y: DWORD,
        unknown: *mut libc::c_void,
        src: LPRECT,
        blt_type: DWORD,
    ) {
        #[allow(clippy::ptr_as_ptr)]
        let raw_fn: unsafe extern "stdcall" fn(
            this: *mut libc::c_void,
            x: DWORD,
            y: DWORD,
            unknown: *mut libc::c_void,
            src: LPRECT,
            blt_type: DWORD,
        ) = std::mem::transmute(*((*(self.raw as *mut usize) + 0x1c) as *const *const ()));
        (raw_fn)(self.raw, x, y, unknown, src, blt_type);
    }
}

#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
pub struct DDBLTFX;

#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_snake_case)]
pub struct DDBLTBATCH {
    pub lprDest: LPRECT,
    pub lpDDSSrc: *mut libc::c_void,
    pub lprSrc: LPRECT,
    pub dwFlags: DWORD,
    pub lpDDBltFx: *const DDBLTFX,
}

impl Draw for IDirectDrawSurface {
    unsafe fn fill_rect(&mut self, rect: Rect<i32>, color: Color) {
        let mut ddbltfx = [0_u32; 25];
        ddbltfx[0] = 100;
        ddbltfx[20] = color.into_argb();
        self.blt(
            std::ptr::addr_of!(rect) as *mut _,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0x1000400,
            ddbltfx.as_mut_ptr().cast(),
        );
    }

    unsafe fn fill_rect_batch(&mut self, rects: Vec<Rect<i32>>, color: Color) {
        let mut ddbltfx = [0_u32; 25];
        ddbltfx[0] = 100;
        ddbltfx[20] = color.into_argb();

        // BltBatch is unimplemented in ddraw.dll so this doesn't do anything

        let batches: Vec<DDBLTBATCH> = (0..rects.len())
            .into_iter()
            .map(|i| DDBLTBATCH {
                lprDest: std::ptr::addr_of!(rects[i]) as *mut _,
                lpDDSSrc: std::ptr::null_mut(),
                lprSrc: std::ptr::null_mut(),
                dwFlags: 0x1000400,
                lpDDBltFx: ddbltfx.as_mut_ptr().cast(),
            })
            .collect();

        self.blt_batch(batches.as_ptr(), batches.len() as DWORD);
    }
}
