#![deny(clippy::all)]
#![allow(clippy::cargo)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::missing_safety_doc)]
// __thiscall calling convention is nightly only
// Tracking issue: https://github.com/rust-lang/rust/issues/42202
#![feature(abi_thiscall)]

use std::convert::TryInto;

use winapi::{
    shared::minwindef::{DWORD, HINSTANCE, LPVOID},
    um::{
        libloaderapi::{DisableThreadLibraryCalls, FreeLibraryAndExitThread, GetModuleFileNameA},
        processthreadsapi::CreateThread,
        winuser,
        winver::{GetFileVersionInfoA, GetFileVersionInfoSizeA, VerQueryValueA},
    },
};

#[cfg(not(target_os = "windows"))]
compile_error!("this is extremely windows dependent");

mod feature;
mod patch;
mod ptc;
mod runtime;
mod winutil;

#[allow(clippy::too_many_lines)] // TODO
fn attach() -> anyhow::Result<()> {
    unsafe {
        // this makes stdout work (eg println!)
        winapi::um::wincon::AttachConsole(winapi::um::wincon::ATTACH_PARENT_PROCESS);

        println!("attach");

        // need to get ptc version without depending on memory addresses since addresses change on different versions
        // unfortunately the code to get exe version with winapi is terrible

        let mut lptstr_filename = [0_i8; 260];
        lptstr_filename[0] = '\0' as i8;
        GetModuleFileNameA(
            std::ptr::null_mut(),
            lptstr_filename.as_mut_ptr(),
            lptstr_filename.len().try_into().unwrap(),
        );
        let mut dw_handle: DWORD = 0;
        let dw_size = GetFileVersionInfoSizeA(lptstr_filename.as_ptr(), &mut dw_handle);

        if dw_size > 0 {
            let mut buf = Vec::new();
            buf.resize(dw_size.try_into().unwrap(), 0);

            if GetFileVersionInfoA(
                lptstr_filename.as_ptr(),
                dw_handle,
                dw_size,
                buf.as_mut_ptr().cast(),
            ) > 0
            {
                let mut pu_len = 0;
                let mut lplp_buffer: *mut libc::c_void = std::ptr::null_mut();
                if VerQueryValueA(
                    buf.as_mut_ptr().cast(),
                    "\\\0".bytes().collect::<Vec<u8>>().as_ptr().cast::<i8>(),
                    &mut lplp_buffer,
                    &mut pu_len,
                ) > 0
                {
                    let v = std::slice::from_raw_parts(
                        lplp_buffer as *const u16,
                        pu_len as usize / std::mem::size_of::<u16>(),
                    );
                    let check = (u32::from(v[1]) << 16) | (u32::from(v[0]) & 0xffff);

                    // https://docs.microsoft.com/en-us/windows/win32/api/verrsrc/ns-verrsrc-vs_fixedfileinfo#members
                    if check == 0xFEEF04BD {
                        // run the mod
                        if let Some(res) = runtime::try_run_version((v[5], v[4], v[7], v[6])) {
                            return res;
                        }

                        println!(
                            "Unsupported PTC version: {}.{}.{}.{}\0",
                            v[5], v[4], v[7], v[6]
                        );
                        let l_msg: Vec<u16> = format!(
                            "Unsupported PTC version: {}.{}.{}.{}\0",
                            v[5], v[4], v[7], v[6]
                        )
                        .encode_utf16()
                        .collect();
                        let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                        winuser::MessageBoxW(
                            std::ptr::null_mut(),
                            l_msg.as_ptr(),
                            l_title.as_ptr(),
                            winuser::MB_OK | winuser::MB_ICONERROR,
                        );
                    } else {
                        println!("Failed to fetch version, unsafe to continue: check");
                        let l_msg: Vec<u16> =
                            "Failed to fetch version, unsafe to continue.\n(check)\0"
                                .encode_utf16()
                                .collect();
                        let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                        winuser::MessageBoxW(
                            std::ptr::null_mut(),
                            l_msg.as_ptr(),
                            l_title.as_ptr(),
                            winuser::MB_OK | winuser::MB_ICONERROR,
                        );
                    }
                } else {
                    println!("Failed to fetch version, unsafe to continue: VerQueryValueA");
                    let l_msg: Vec<u16> =
                        "Failed to fetch version, unsafe to continue.\n(VerQueryValueA)\0"
                            .encode_utf16()
                            .collect();
                    let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                    winuser::MessageBoxW(
                        std::ptr::null_mut(),
                        l_msg.as_ptr(),
                        l_title.as_ptr(),
                        winuser::MB_OK | winuser::MB_ICONERROR,
                    );
                }
            } else {
                println!("Failed to fetch version, unsafe to continue: GetFileVersionInfoA");
                let l_msg: Vec<u16> =
                    "Failed to fetch version, unsafe to continue.\n(GetFileVersionInfoA)\0"
                        .encode_utf16()
                        .collect();
                let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                winuser::MessageBoxW(
                    std::ptr::null_mut(),
                    l_msg.as_ptr(),
                    l_title.as_ptr(),
                    winuser::MB_OK | winuser::MB_ICONERROR,
                );
            }
        } else {
            println!("Failed to fetch version, unsafe to continue: GetFileVersionInfoSizeA");
            let l_msg: Vec<u16> =
                "Failed to fetch version, unsafe to continue.\n(GetFileVersionInfoSizeA)\0"
                    .encode_utf16()
                    .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            winuser::MessageBoxW(
                std::ptr::null_mut(),
                l_msg.as_ptr(),
                l_title.as_ptr(),
                winuser::MB_OK | winuser::MB_ICONERROR,
            );
        }

        Ok(())
    }
}

#[allow(clippy::unnecessary_wraps)]
fn detach() -> anyhow::Result<()> {
    println!("detach");

    Ok(())
}

unsafe extern "system" fn attach_wrapper(base: LPVOID) -> u32 {
    match std::panic::catch_unwind(attach) {
        Err(err) => {
            let msg = match err.downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match err.downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<dyn Any>",
                },
            };
            println!("attach panicked: {}", msg);

            let l_msg: Vec<u16> = format!("attach panicked: {:?}\0", msg)
                .encode_utf16()
                .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            winuser::MessageBoxW(
                std::ptr::null_mut(),
                l_msg.as_ptr(),
                l_title.as_ptr(),
                winuser::MB_OK | winuser::MB_ICONERROR,
            );
        }
        Ok(Err(err)) => {
            let l_msg: Vec<u16> = format!("attach exited with an Err: {:?}\0", err)
                .encode_utf16()
                .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            winuser::MessageBoxW(
                std::ptr::null_mut(),
                l_msg.as_ptr(),
                l_title.as_ptr(),
                winuser::MB_OK | winuser::MB_ICONERROR,
            );
        }
        Ok(Ok(())) => {}
    }

    match std::panic::catch_unwind(detach) {
        Err(err) => {
            let l_msg: Vec<u16> = format!("detach panicked: {:?}\0", err)
                .encode_utf16()
                .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            winuser::MessageBoxW(
                std::ptr::null_mut(),
                l_msg.as_ptr(),
                l_title.as_ptr(),
                winuser::MB_OK | winuser::MB_ICONERROR,
            );
        }
        Ok(Err(err)) => {
            let l_msg: Vec<u16> = format!("detach exited with an Err: {:?}\0", err)
                .encode_utf16()
                .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            winuser::MessageBoxW(
                std::ptr::null_mut(),
                l_msg.as_ptr(),
                l_title.as_ptr(),
                winuser::MB_OK | winuser::MB_ICONERROR,
            );
        }
        Ok(Ok(())) => {}
    }

    FreeLibraryAndExitThread(base.cast(), 1);
    unreachable!()
}

#[no_mangle]
pub unsafe extern "stdcall" fn DllMain(
    hinst_dll: HINSTANCE,
    fdw_reason: DWORD,
    lp_reserved: LPVOID,
) -> i32 {
    match fdw_reason {
        winapi::um::winnt::DLL_PROCESS_ATTACH => {
            DisableThreadLibraryCalls(hinst_dll);
            CreateThread(
                std::ptr::null_mut(),
                0,
                Some(attach_wrapper),
                hinst_dll.cast(),
                0,
                std::ptr::null_mut(),
            );
        }
        winapi::um::winnt::DLL_PROCESS_DETACH => {
            if !lp_reserved.is_null() {
                match std::panic::catch_unwind(detach) {
                    Err(err) => {
                        let l_msg: Vec<u16> = format!("detach panicked: {:?}\0", err)
                            .encode_utf16()
                            .collect();
                        let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                        winuser::MessageBoxW(
                            std::ptr::null_mut(),
                            l_msg.as_ptr(),
                            l_title.as_ptr(),
                            winuser::MB_OK | winuser::MB_ICONERROR,
                        );
                    }
                    Ok(Err(err)) => {
                        let l_msg: Vec<u16> = format!("detach exited with an Err: {:?}\0", err)
                            .encode_utf16()
                            .collect();
                        let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                        winuser::MessageBoxW(
                            std::ptr::null_mut(),
                            l_msg.as_ptr(),
                            l_title.as_ptr(),
                            winuser::MB_OK | winuser::MB_ICONERROR,
                        );
                    }
                    Ok(Ok(())) => {}
                }
            }
        }
        _ => {}
    }

    1
}
