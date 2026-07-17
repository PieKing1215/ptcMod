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

use std::{convert::TryInto, ffi::c_void};

use windows::{
    core::{PCSTR, PCWSTR},
    Win32::{
        Foundation::{HINSTANCE, HMODULE},
        Storage::FileSystem::{GetFileVersionInfoA, GetFileVersionInfoSizeA, VerQueryValueA},
        System::{
            Console::{AttachConsole, ATTACH_PARENT_PROCESS},
            LibraryLoader::{
                DisableThreadLibraryCalls, FreeLibraryAndExitThread, GetModuleFileNameA,
            },
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
            Threading::{CreateThread, THREAD_CREATION_FLAGS},
        },
        UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK},
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
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);

        println!("attach");

        // need to get ptc version without depending on memory addresses since addresses change on different versions
        // unfortunately the code to get exe version with winapi is terrible

        let mut lptstr_filename = [0_u8; 260];
        lptstr_filename[0] = b'\0';
        GetModuleFileNameA(None, &mut lptstr_filename);
        let mut dw_handle: u32 = 0;
        let lptstr_filename = PCSTR(lptstr_filename.as_ptr());
        let dw_size = GetFileVersionInfoSizeA(lptstr_filename, Some(&raw mut dw_handle));

        if dw_size > 0 {
            let mut buf = vec![0; dw_size.try_into().unwrap()];

            if let Err(_e) = GetFileVersionInfoA(
                lptstr_filename,
                Some(dw_handle),
                dw_size,
                buf.as_mut_ptr().cast(),
            ) {
                println!("Failed to fetch version, unsafe to continue: GetFileVersionInfoA");
                let l_msg: Vec<u16> =
                    "Failed to fetch version, unsafe to continue.\n(GetFileVersionInfoA)\0"
                        .encode_utf16()
                        .collect();
                let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                MessageBoxW(
                    None,
                    PCWSTR::from_raw(l_msg.as_ptr()),
                    PCWSTR::from_raw(l_title.as_ptr()),
                    MB_OK | MB_ICONERROR,
                );
                return Ok(());
            }
            let mut pu_len = 0;
            let mut lplp_buffer: *mut libc::c_void = std::ptr::null_mut();
            if VerQueryValueA(
                buf.as_mut_ptr().cast(),
                PCSTR("\\\0".bytes().collect::<Vec<u8>>().as_ptr()),
                &raw mut lplp_buffer,
                &raw mut pu_len,
            )
            .as_bool()
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
                    MessageBoxW(
                        None,
                        PCWSTR::from_raw(l_msg.as_ptr()),
                        PCWSTR::from_raw(l_title.as_ptr()),
                        MB_OK | MB_ICONERROR,
                    );
                } else {
                    println!("Failed to fetch version, unsafe to continue: check");
                    let l_msg: Vec<u16> = "Failed to fetch version, unsafe to continue.\n(check)\0"
                        .encode_utf16()
                        .collect();
                    let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                    MessageBoxW(
                        None,
                        PCWSTR::from_raw(l_msg.as_ptr()),
                        PCWSTR::from_raw(l_title.as_ptr()),
                        MB_OK | MB_ICONERROR,
                    );
                }
            } else {
                println!("Failed to fetch version, unsafe to continue: VerQueryValueA");
                let l_msg: Vec<u16> =
                    "Failed to fetch version, unsafe to continue.\n(VerQueryValueA)\0"
                        .encode_utf16()
                        .collect();
                let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                MessageBoxW(
                    None,
                    PCWSTR::from_raw(l_msg.as_ptr()),
                    PCWSTR::from_raw(l_title.as_ptr()),
                    MB_OK | MB_ICONERROR,
                );
            }
        } else {
            println!("Failed to fetch version, unsafe to continue: GetFileVersionInfoSizeA");
            let l_msg: Vec<u16> =
                "Failed to fetch version, unsafe to continue.\n(GetFileVersionInfoSizeA)\0"
                    .encode_utf16()
                    .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            MessageBoxW(
                None,
                PCWSTR::from_raw(l_msg.as_ptr()),
                PCWSTR::from_raw(l_title.as_ptr()),
                MB_OK | MB_ICONERROR,
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

unsafe extern "system" fn attach_wrapper(base: *mut c_void) -> u32 {
    match std::panic::catch_unwind(attach) {
        Err(err) => {
            let msg = match err.downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match err.downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<dyn Any>",
                },
            };
            println!("attach panicked: {msg}");

            let l_msg: Vec<u16> = format!("attach panicked: {msg:?}\0")
                .encode_utf16()
                .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            MessageBoxW(
                None,
                PCWSTR::from_raw(l_msg.as_ptr()),
                PCWSTR::from_raw(l_title.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        },
        Ok(Err(err)) => {
            let l_msg: Vec<u16> = format!("attach exited with an Err: {err:?}\0")
                .encode_utf16()
                .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            MessageBoxW(
                None,
                PCWSTR::from_raw(l_msg.as_ptr()),
                PCWSTR::from_raw(l_title.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        },
        Ok(Ok(())) => {},
    }

    match std::panic::catch_unwind(detach) {
        Err(err) => {
            let l_msg: Vec<u16> = format!("detach panicked: {err:?}\0")
                .encode_utf16()
                .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            MessageBoxW(
                None,
                PCWSTR::from_raw(l_msg.as_ptr()),
                PCWSTR::from_raw(l_title.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        },
        Ok(Err(err)) => {
            let l_msg: Vec<u16> = format!("detach exited with an Err: {err:?}\0")
                .encode_utf16()
                .collect();
            let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
            MessageBoxW(
                None,
                PCWSTR::from_raw(l_msg.as_ptr()),
                PCWSTR::from_raw(l_title.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        },
        Ok(Ok(())) => {},
    }

    FreeLibraryAndExitThread(HMODULE(base), 1);
}

#[no_mangle]
pub unsafe extern "stdcall" fn DllMain(
    hinst_dll: HINSTANCE,
    fdw_reason: u32,
    lp_reserved: *mut c_void,
) -> i32 {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            let _ = DisableThreadLibraryCalls(hinst_dll.into());
            CreateThread(
                None,
                0,
                Some(attach_wrapper),
                Some(hinst_dll.0),
                THREAD_CREATION_FLAGS::default(),
                None,
            )
            .unwrap();
        },
        DLL_PROCESS_DETACH if !lp_reserved.is_null() => match std::panic::catch_unwind(detach) {
            Err(err) => {
                let l_msg: Vec<u16> = format!("detach panicked: {err:?}\0")
                    .encode_utf16()
                    .collect();
                let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                MessageBoxW(
                    None,
                    PCWSTR::from_raw(l_msg.as_ptr()),
                    PCWSTR::from_raw(l_title.as_ptr()),
                    MB_OK | MB_ICONERROR,
                );
            },
            Ok(Err(err)) => {
                let l_msg: Vec<u16> = format!("detach exited with an Err: {err:?}\0")
                    .encode_utf16()
                    .collect();
                let l_title: Vec<u16> = "PTC Mod\0".encode_utf16().collect();
                MessageBoxW(
                    None,
                    PCWSTR::from_raw(l_msg.as_ptr()),
                    PCWSTR::from_raw(l_title.as_ptr()),
                    MB_OK | MB_ICONERROR,
                );
            },
            Ok(Ok(())) => {},
        },
        _ => {},
    }

    1
}
