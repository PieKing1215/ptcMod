use std::mem::ManuallyDrop;

use windows::Win32::{
    Foundation::HANDLE,
    Graphics::DirectDraw::{
        DDBLT_COLORFILL, DDBLT_DDFX, DDBLT_WAIT, DDBLTBATCH, DDLOCK_WAIT, DDSURFACEDESC,
        IDirectDrawSurface,
    },
};
use windows_core::Interface;

use super::{Draw, Rect, color::Color};

impl Draw for IDirectDrawSurface {
    unsafe fn fill_rect(&self, rect: &Rect<i32>, color: Color) {
        if color.a != u8::MAX {
            // ddraw can't do alpha blending so need to do it manually
            self.set_pixels(|set_pixel| {
                for x in rect.left..rect.right {
                    for y in rect.top..rect.bottom {
                        set_pixel(x, y, color);
                    }
                }
            });
            return;
        }

        let mut ddbltfx = [0_u32; 25];
        ddbltfx[0] = 100;
        ddbltfx[20] = color.into_argb();
        let mut rect = *rect;
        let _ = self.Blt(
            rect.as_lprect(),
            None,
            std::ptr::null_mut(),
            (DDBLT_COLORFILL | DDBLT_WAIT | DDBLT_DDFX) as _,
            ddbltfx.as_mut_ptr().cast(),
        );
    }

    unsafe fn fill_rect_batch(&self, rects: Vec<Rect<i32>>, color: Color) {
        let mut ddbltfx = [0_u32; 25];
        ddbltfx[0] = 100;
        ddbltfx[20] = color.into_argb();

        // BltBatch is unimplemented in ddraw.dll so this doesn't do anything

        let mut batches: Vec<DDBLTBATCH> = (0..rects.len())
            .into_iter()
            .map(|i| DDBLTBATCH {
                lprDest: std::ptr::addr_of!(rects[i]) as *mut _,
                lpDDSSrc: ManuallyDrop::new(None),
                lprSrc: std::ptr::null_mut(),
                dwFlags: 0x1000400,
                lpDDBltFx: ddbltfx.as_mut_ptr().cast(),
            })
            .collect();

        let _ = self.BltBatch(batches.as_mut_ptr(), batches.len() as u32, 0);
    }

    fn set_pixels(&self, modify: impl FnOnce(&mut dyn FnMut(i32, i32, Color))) {
        unsafe {
            let mut desc = DDSURFACEDESC {
                dwSize: std::mem::size_of::<DDSURFACEDESC>() as u32,
                ..Default::default()
            };

            self.Lock(
                std::ptr::null_mut(),
                &raw mut desc,
                DDLOCK_WAIT as _,
                HANDLE(std::ptr::null_mut()),
            )
            .unwrap();

            let pf = &desc.ddpfPixelFormat;
            assert_eq!(pf.Anonymous1.dwRGBBitCount, 32);
            assert_eq!(pf.Anonymous2.dwRBitMask, 0x00FF0000);
            assert_eq!(pf.Anonymous3.dwGBitMask, 0x0000FF00);
            assert_eq!(pf.Anonymous4.dwBBitMask, 0x000000FF);
            assert_eq!(pf.Anonymous5.dwRGBAlphaBitMask, 0x00000000);

            let pixels = desc.lpSurface.cast::<u8>();
            let pitch = desc.Anonymous1.lPitch as isize;

            let mut set_pixel = |x, y, color: Color| {
                let pixel = pixels
                    .offset(y as isize * pitch + x as isize * 4)
                    .cast::<u32>();
                let dst = Color::from_argb(*pixel);
                *pixel = dst.blend(color, color.a_f32()).with_a(u8::MAX).into_argb();
            };

            modify(&mut set_pixel);

            self.Unlock(std::ptr::null_mut()).unwrap();
        }
    }
}

pub unsafe fn create_surface(
    ddraw: *mut libc::c_void,
    width: i32,
    height: i32,
) -> IDirectDrawSurface {
    #[allow(clippy::ptr_as_ptr)]
    let raw_fn: unsafe extern "stdcall" fn(
        this: *mut libc::c_void,
        surface_desc: *mut libc::c_void,
        out_surf: *mut *mut libc::c_void,
        unused: *mut libc::c_void,
    ) = std::mem::transmute(*((*(ddraw as *mut usize) + 0x18) as *const *const ()));

    let mut surface_desc = [0_i32; 0x6c / 4];
    surface_desc[0] = 0x6c; // dwSize
    surface_desc[1] = 0x00000001 | 0x00000002 | 0x00000004; // dwFlags = DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH
    surface_desc[2] = height; // dwHeight
    surface_desc[3] = width; // dwWidth

    let mut out_surf = std::ptr::null_mut();
    (raw_fn)(
        ddraw,
        surface_desc.as_mut_ptr().cast(),
        &raw mut out_surf,
        std::ptr::null_mut(),
    );
    IDirectDrawSurface::from_raw(out_surf)
}
