use winapi::um::{memoryapi::VirtualProtect, winnt::PAGE_EXECUTE_READWRITE};

use crate::ptc::addr;

#[derive(Clone)]
pub struct Patch {
    addr: usize,
    old: Vec<u8>,
    new: Vec<u8>,
}

impl Patch {
    pub fn new(addr: usize, old: Vec<u8>, new: Vec<u8>) -> anyhow::Result<Self> {
        if old.len() == new.len() {
            Ok(Self { addr, old, new })
        } else {
            Err(anyhow::anyhow!(""))
        }
    }

    pub unsafe fn apply(&self) -> anyhow::Result<()> {
        let mem = std::slice::from_raw_parts_mut(addr(self.addr) as *mut u8, self.old.len());

        log::debug!(
            "Patching @ {:#x} (apply). Expect {:x?} found {:x?}",
            self.addr,
            self.old,
            mem
        );
        if self.old == mem {
            let mut lpfl_old_protect_1: winapi::shared::minwindef::DWORD = 0;
            VirtualProtect(
                addr(self.addr) as *mut libc::c_void,
                mem.len(),
                PAGE_EXECUTE_READWRITE,
                &mut lpfl_old_protect_1,
            );

            mem.copy_from_slice(&self.new);

            VirtualProtect(
                addr(self.addr) as *mut libc::c_void,
                mem.len(),
                lpfl_old_protect_1,
                &mut lpfl_old_protect_1,
            );

            log::debug!("-> {:x?}", mem);
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Patch at {:#x} found wrong bytes for apply. Expected {:x?} found {:x?}",
                self.addr,
                self.old,
                mem
            ))
        }
    }

    pub unsafe fn unapply(&self) -> anyhow::Result<()> {
        let mem = std::slice::from_raw_parts_mut(addr(self.addr) as *mut u8, self.new.len());

        log::debug!(
            "Patching @ {:#x} (unapply). Expect {:x?} found {:x?}",
            self.addr,
            self.old,
            mem
        );
        if self.new == mem {
            let mut lpfl_old_protect: winapi::shared::minwindef::DWORD = 0;
            VirtualProtect(
                addr(self.addr) as *mut libc::c_void,
                mem.len(),
                PAGE_EXECUTE_READWRITE,
                &mut lpfl_old_protect,
            );

            mem.copy_from_slice(&self.old);

            VirtualProtect(
                addr(self.addr) as *mut libc::c_void,
                mem.len(),
                lpfl_old_protect,
                &mut lpfl_old_protect,
            );

            log::debug!("-> {:x?}", mem);

            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Patch at {:#x} found wrong bytes for unapply. Expected {:x?} found {:x?}",
                self.addr,
                self.new,
                mem
            ))
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub fn call_patch(call_addr: usize, old_fn_addr: usize, new_fn: *const ()) -> Patch {
    let old_bytes = i32::to_le_bytes(old_fn_addr as i32 - (call_addr + 0x5) as i32);

    let new_bytes = i32::to_le_bytes((new_fn as i64 - (addr(call_addr) + 0x5) as i64) as i32);

    Patch::new(
        call_addr,
        vec![0xe8, old_bytes[0], old_bytes[1], old_bytes[2], old_bytes[3]],
        vec![0xe8, new_bytes[0], new_bytes[1], new_bytes[2], new_bytes[3]],
    )
    .unwrap()
}

// A bunch of macros that help reduce boilerplate of extern fns

/// Returns a [Patch] which replaces the function call at the specified address with a wrapper.
///
/// This variant of the macro calls completely replaces the original fn with the new fn
#[allow(unused_macros)]
macro_rules! replace {
    ( $call_addr:expr, $fn_addr:expr, $cc:expr, fn($($p_name:ident: $p_type:ty),*)$( -> $ret:ty)?, $my_fn:expr ) => {
        {
            unsafe extern $cc fn func($($p_name: $p_type),*)$( -> $ret)* {
                $my_fn($($p_name),*)
            }
            crate::patch::call_patch($call_addr, $fn_addr, func as *const ())
        }
    };
}
#[allow(unused_imports)]
pub(crate) use replace;

/// Returns a [Patch] which replaces the function call at the specified address with a wrapper.
///
/// This variant of the macro calls the new fn, then the original fn, and returns the new fn's value.
#[allow(unused_macros)]
macro_rules! hook_pre_ret_new {
    ( $call_addr:expr, $fn_addr:expr, $cc:expr, fn($($p_name:ident: $p_type:ty),*)$( -> $ret:ty)?, $my_fn:expr ) => {
        {
            unsafe extern $cc fn func($($p_name: $p_type,)*)$( -> $ret)* {
                let ret = $my_fn($($p_name),*);
                let raw_fn: unsafe extern $cc fn($($p_name: $p_type),*)$( -> $ret)* =
                    std::mem::transmute(addr($fn_addr) as *const ());
                (raw_fn)($($p_name),*);
                ret
            }
            crate::patch::call_patch($call_addr, $fn_addr, func as *const ())
        }
    };
}
#[allow(unused_imports)]
pub(crate) use hook_pre_ret_new;

/// Returns a [Patch] which replaces the function call at the specified address with a wrapper.
///
/// This variant of the macro calls the original fn, then the new fn, and returns the new fn's value.
#[allow(unused_macros)]
macro_rules! hook_post_ret_new {
    ( $call_addr:expr, $fn_addr:expr, $cc:expr, fn($($p_name:ident: $p_type:ty),*)$( -> $ret:ty)?, $my_fn:expr ) => {
        {
            unsafe extern $cc fn func($($p_name: $p_type),*)$( -> $ret)* {
                let raw_fn: unsafe extern $cc fn($($p_name: $p_type),*)$( -> $ret)* =
                    std::mem::transmute(addr($fn_addr) as *const ());
                (raw_fn)($($p_name),*);
                $my_fn($($p_name),*)
            }
            crate::patch::call_patch($call_addr, $fn_addr, func as *const ())
        }
    };
}
#[allow(unused_imports)]
pub(crate) use hook_post_ret_new;

/// Returns a [Patch] which replaces the function call at the specified address with a wrapper.
///
/// This variant of the macro calls the original fn, then the new fn, and returns the original fn's value.
#[allow(unused_macros)]
macro_rules! hook_pre_ret_old {
    ( $call_addr:expr, $fn_addr:expr, $cc:expr, fn($($p_name:ident: $p_type:ty),*)$( -> $ret:ty)?, $my_fn:expr ) => {
        {
            unsafe extern $cc fn func($($p_name: $p_type,)*)$( -> $ret)* {
                $my_fn($($p_name),*);
                let raw_fn: unsafe extern $cc fn($($p_name: $p_type),*)$( -> $ret)* =
                    std::mem::transmute(addr($fn_addr) as *const ());
                (raw_fn)($($p_name),*)
            }
            crate::patch::call_patch($call_addr, $fn_addr, func as *const ())
        }
    };
}
#[allow(unused_imports)]
pub(crate) use hook_pre_ret_old;

/// Returns a [Patch] which replaces the function call at the specified address with a wrapper.
///
/// This variant of the macro calls the new fn, then the original fn, and returns the original fn's value.
#[allow(unused_macros)]
macro_rules! hook_post_ret_old {
    ( $call_addr:expr, $fn_addr:expr, $cc:expr, fn($($p_name:ident: $p_type:ty),*)$( -> $ret:ty)?, $my_fn:expr ) => {
        {
            unsafe extern $cc fn func($($p_name: $p_type),*)$( -> $ret)* {
                let raw_fn: unsafe extern $cc fn($($p_name: $p_type),*)$( -> $ret)* =
                    std::mem::transmute(addr($fn_addr) as *const ());
                (raw_fn)($($p_name),*);
                $my_fn($($p_name),*)
            }
            crate::patch::call_patch($call_addr, $fn_addr, func as *const ())
        }
    };
}
#[allow(unused_imports)]
pub(crate) use hook_post_ret_old;

/// Returns a [Patch] which replaces the function call at the specified address with a wrapper.
///
/// This variant of the macro takes a function or closure which is passed the old function and the parameters
#[allow(unused_macros)]
macro_rules! hook {
    ( $call_addr:expr, $fn_addr:expr, $cc:expr, fn($($p_name:ident: $p_type:ty),*)$( -> $ret:ty)?, $my_fn:expr ) => {
        {
            unsafe extern $cc fn func($($p_name: $p_type),*)$( -> $ret)* {
                let raw_fn: unsafe extern $cc fn($($p_name: $p_type),*)$( -> $ret)* =
                    std::mem::transmute(addr($fn_addr) as *const ());
                (raw_fn)($($p_name),*);
                $my_fn(raw_fn, $($p_name),*)
            }
            crate::patch::call_patch($call_addr, $fn_addr, func as *const ())
        }
    };
}
#[allow(unused_imports)]
pub(crate) use hook;

#[allow(clippy::all)]
#[test]
fn macro_usage() {
    use crate::ptc::v0925::PTC0925;

    unsafe fn test_f<PTC: crate::ptc::PTCVersion>(a: i32, b: usize) -> i32 {
        a + b as i32
    }

    unsafe fn test_f2<PTC: crate::ptc::PTCVersion>() {}

    replace!(
        0xAAAA,
        0xBBBB,
        "stdcall",
        fn(a: i32, b: usize) -> i32,
        test_f::<PTC0925>
    );
    hook_pre_ret_new!(
        0xAAAA,
        0xBBBB,
        "stdcall",
        fn(a: i32, b: usize) -> i32,
        test_f::<PTC0925>
    );
    hook_post_ret_new!(
        0xAAAA,
        0xBBBB,
        "stdcall",
        fn(a: i32, b: usize) -> i32,
        test_f::<PTC0925>
    );
    hook_pre_ret_old!(
        0xAAAA,
        0xBBBB,
        "stdcall",
        fn(a: i32, b: usize) -> i32,
        test_f::<PTC0925>
    );
    hook_post_ret_old!(
        0xAAAA,
        0xBBBB,
        "stdcall",
        fn(a: i32, b: usize) -> i32,
        test_f::<PTC0925>
    );

    replace!(0xAAAA, 0xBBBB, "stdcall", fn(), test_f2::<PTC0925>);
    hook_pre_ret_new!(0xAAAA, 0xBBBB, "stdcall", fn(), test_f2::<PTC0925>);
    hook_post_ret_new!(0xAAAA, 0xBBBB, "stdcall", fn(), test_f2::<PTC0925>);

    replace!(0xAAAA, 0xBBBB, "stdcall", fn(), || println!("hi"));
    hook_pre_ret_new!(0xAAAA, 0xBBBB, "stdcall", fn(), || println!("hi"));
    hook_post_ret_new!(0xAAAA, 0xBBBB, "stdcall", fn(), || println!("hi"));

    hook!(
        0xAAAA,
        0xBBBB,
        "stdcall",
        fn(a: i32, b: usize) -> i32,
        |func: unsafe extern "stdcall" fn(a: i32, b: usize) -> i32, a, b| {
            let r = func(a, b);
            test_f::<PTC0925>(r, b)
        }
    );
}
