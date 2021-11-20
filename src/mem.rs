pub const MEMORY_SIZE: usize = 16777216; // 16mb

pub struct Mem {
    pub mem: Box<[u8; MEMORY_SIZE]>,
}

impl Default for Mem {
    fn default() -> Self {
        Self {
            mem: Box::new([0; MEMORY_SIZE]),
        }
    }
}

impl Mem {
    pub fn lb(&self, addr: u32) -> u8 {
        if addr & 0x80000000 != 0 {
            self.mem[addr as usize & 0x7FFFFFFF]
        } else {
            0
        }
    }

    pub fn sb(&mut self, addr: u32, val: u8) {
        if addr & 0x80000000 != 0 {
            self.mem[addr as usize & 0x7FFFFFFF] = val;
        }
    }

    pub fn lh(&self, addr: u32) -> u16 {
        ((self.lb(addr + 1) as u16) << 8) | (self.lb(addr) as u16)
    }

    pub fn lw(&self, addr: u32) -> u32 {
        ((self.lb(addr + 3) as u32) << 24)
            | ((self.lb(addr + 2) as u32) << 16)
            | ((self.lb(addr + 1) as u32) << 8)
            | (self.lb(addr) as u32)
    }

    pub fn sh(&mut self, addr: u32, val: u16) {
        self.sb(addr, val as u8);
        self.sb(addr + 1, (val >> 8) as u8);
    }

    pub fn sw(&mut self, addr: u32, val: u32) {
        self.sb(addr, val as u8);
        self.sb(addr + 1, (val >> 8) as u8);
        self.sb(addr + 2, (val >> 16) as u8);
        self.sb(addr + 3, (val >> 24) as u8);
    }
}
