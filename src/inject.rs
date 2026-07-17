use std::ffi::CString;
use std::io;
use std::mem;
use std::path::Path;

use windows::core::PCSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::LibraryLoader::GetProcAddress;
use windows::Win32::System::Memory::VirtualAllocEx;
use windows::Win32::System::Memory::MEM_COMMIT;
use windows::Win32::System::Memory::MEM_RESERVE;
use windows::Win32::System::Memory::PAGE_READWRITE;
use windows::Win32::System::Threading::CreateRemoteThread;

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
            None,
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
            dll_path.as_ptr().cast(),
            path_size,
            None,
        )
    };

    if let Err(err) = suc {
        eprintln!("WriteProcessMemory failed: {:?}", err);
        return Err(err.into());
    }

    // find LoadLibraryA address
    println!("Looking for LoadLibraryA...");
    let load_library_a = unsafe {
        let kernel = GetModuleHandleA(PCSTR(c"Kernel32.dll".as_ptr().cast())).unwrap();
        GetProcAddress(kernel, PCSTR(c"LoadLibraryA".as_ptr().cast())).unwrap()
    };
    println!("-> {:#p}", load_library_a);

    // spawn thread in the target process that calls LoadLibraryA with the dll path as argument
    println!("Spawning thread for LoadLibraryA...");
    let thread_handle = unsafe {
        CreateRemoteThread(
            process,
            None,
            0,
            Some(mem::transmute::<
                unsafe extern "system" fn() -> isize,
                unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
            >(load_library_a)),
            Some(path_addr),
            0,
            None,
        )
    };

    let thread_handle = match thread_handle {
        Ok(thread_handle) => thread_handle,
        Err(err) => {
            eprintln!("CreateRemoteThread failed: {:?}", err);
            return Err(err.into());
        },
    };
    println!("-> {:#p}", thread_handle.0);

    // don't care about this thread anymore
    unsafe {
        CloseHandle(thread_handle).unwrap();
    }

    println!("Done.");

    Ok(())
}
