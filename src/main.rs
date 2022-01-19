use std::path::{Path, PathBuf};

use path_absolutize::Absolutize;
use process_memory::{Pid, ProcessHandle, TryIntoProcessHandle};
use sysinfo::{PidExt, ProcessExt, System, SystemExt};

mod inject;

fn main() {
    println!("PTC Mod Injector");

    // basically if there's a ptc_mod.dll in the working directory, use it.
    // otherwise if running using cargo, look in the right target/ folder for it

    let mut try_paths = vec![
        PathBuf::from("./ptc_mod.dll"),
    ];
    println!("{:?}", option_env!("OUT_DIR"));

    if let Some(p) = option_env!("OUT_DIR") {
        let p = PathBuf::from(p);
        // eg navigate from
        // target/i686-pc-windows-gnu/debug/build/ptc-mod-xxxxxxxx/out/
        // to
        // target/i686-pc-windows-gnu/debug/ptc_mod.dll
        let p = p
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(|p| p.to_path_buf());
        if let Some(mut p) = p {
            p.push("ptc_mod.dll");
            try_paths.push(p);
        }
    }

    if let Some(handle) = get_ptc_handle() {
        for path in try_paths {
            if path.exists() {
                let path = path
                    .absolutize()
                    .map_or(path.clone(), |abs| abs.to_path_buf());
                println!("Attempting to inject ptc_mod.dll @ {:?}", path);

                let res = inject::inject_dll(handle.0, path.as_path());

                if let Err(e) = res {
                    eprintln!("{:?}", e);
                } else {
                    break;
                }
            } else {
                let path = path
                    .absolutize()
                    .map_or(path.clone(), |abs| abs.to_path_buf());
                println!("Missing ptc_mod.dll @ {:?}", path);
            }
        }
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
