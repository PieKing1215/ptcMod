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
