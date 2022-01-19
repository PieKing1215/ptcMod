use std::ffi::CString;
use std::io;
use std::mem;
use std::path::Path;
use std::ptr;

use winapi::shared::minwindef::LPCVOID;
use winapi::um::handleapi::CloseHandle;
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::libloaderapi::GetProcAddress;
use winapi::um::memoryapi::VirtualAllocEx;
use winapi::um::memoryapi::WriteProcessMemory;
use winapi::um::processthreadsapi::CreateRemoteThread;
use winapi::um::winnt::HANDLE;
use winapi::um::winnt::PAGE_READWRITE;
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE};

/// Injects the dll at `dll_path` into the given process
pub fn inject_dll(process: HANDLE, dll_path: &Path) -> io::Result<()> {
    let dll_path = dll_path.canonicalize()?;

    println!("dll_path = {:?}", dll_path);

    let dll_path = CString::new(dll_path.to_str().expect("Invalid dll path (to_str)"))
        .expect("Invalid dll path (CString::new)");

    let path_size = dll_path.as_bytes_with_nul().len();

    // alloc space for dll path
    println!("Allocating {} bytes in target process...", path_size);
    let path_addr = unsafe {
        VirtualAllocEx(
            process,
            ptr::null_mut(),
            path_size,
            MEM_RESERVE | MEM_COMMIT,
            PAGE_READWRITE,
        )
    };

    if path_addr.is_null() {
        let err = io::Error::last_os_error();
        eprintln!("VirtualAllocEx failed: {:?}", err);
        return Err(err);
    }
    println!("-> {:#p}", path_addr);

    // write dll path
    println!("Writing dll path...");
    let suc = unsafe {
        WriteProcessMemory(
            process,
            path_addr,
            dll_path.as_ptr() as LPCVOID,
            path_size,
            ptr::null_mut(),
        )
    };

    if suc == 0 {
        let err = io::Error::last_os_error();
        eprintln!("WriteProcessMemory failed: {:?}", err);
        return Err(err);
    }

    // find LoadLibraryA address
    println!("Looking for LoadLibraryA...");
    let load_library_a = unsafe {
        let kernel = GetModuleHandleA(b"Kernel32.dll\0".as_ptr() as *const _);
        GetProcAddress(kernel, b"LoadLibraryA\0".as_ptr() as *const _)
    };
    println!("-> {:#p}", load_library_a);

    // spawn thread in the target process that calls LoadLibraryA with the dll path as argument
    println!("Spawning thread for LoadLibraryA...");
    let thread_handle = unsafe {
        CreateRemoteThread(
            process,
            ptr::null_mut(),
            0,
            Some(mem::transmute(load_library_a)),
            path_addr,
            0,
            ptr::null_mut(),
        )
    };

    if thread_handle.is_null() {
        let err = io::Error::last_os_error();
        eprintln!("CreateRemoteThread failed: {:?}", err);
        return Err(err);
    }
    println!("-> {:#p}", thread_handle);

    // don't care about this thread anymore
    unsafe {
        CloseHandle(thread_handle);
    }

    println!("Done.");

    Ok(())
}
