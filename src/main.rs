use std::path::Path;

use process_memory::{Pid, ProcessHandle, TryIntoProcessHandle};
use sysinfo::{ProcessExt, System, SystemExt, PidExt};

mod inject;

fn main() {
    println!("PTC Mod Injector");

    if let Some(handle) = get_ptc_handle() {
        inject::inject_dll(
            handle.0,
            Path::new("target/i686-pc-windows-gnu/debug/ptc_mod.dll"), // TODO: hardcoded, make a build.rs to use OUT_DIR
        )
        .unwrap();
    }
}

fn get_ptc_handle() -> Option<ProcessHandle> {
    let mut s = System::new();
    s.refresh_processes();
    for (pid, process) in s.processes() {
        // println!("{} {}", pid, process.name());
        if process.name() == "ptCollage.exe" {
            println!("Found {} with PID = {}", process.name(), pid);
            let pp = pid.as_u32() as Pid;
            let ph = pp.try_into_process_handle();
            if let Ok(ph) = ph {
                return Some(ph);
            }
        }
    }

    None
}
