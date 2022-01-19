#[derive(Clone)]
pub struct Patch {
    bytes: Vec<PatchByte>,
}

#[derive(Clone)]
pub struct PatchByte {
    addr: usize,
    old: u8,
    new: u8,
}

impl Patch {
    pub fn new(bytes: Vec<PatchByte>) -> Self {
        Self { bytes }
    }
}

impl PatchByte {
    pub fn new(addr: usize, old: u8, new: u8) -> Self {
        Self { addr, old, new }
    }
}
