use crate::mem::{Mem, MEMORY_SIZE};

fn read_imm_i(inst: u32) -> u32 {
    (inst as i32 >> 20) as u32
}

fn read_imm_s(inst: u32) -> u32 {
    (read_imm_i(inst) & !0b11111) | ((inst >> 7) & 0b11111)
}

fn read_imm_b(inst: u32) -> u32 {
    let low = (inst >> 7) & 0b11111 & !1;
    let mid = (inst << 4) & (1 << 11);
    let high = read_imm_i(inst) & !0b11111 & !(1 << 11);
    low | mid | high
}

fn read_imm_u(inst: u32) -> u32 {
    inst & !(0xFFF)
}

fn read_imm_j(inst: u32) -> u32 {
    let a = read_imm_i(inst) & 0xFFF007FE;
    let b = inst & 0x000FF000;
    let c = (inst & (1 << 20)) >> 9;
    a | b | c
}

#[derive(Debug)]
enum Opcode {
    OpImm,
    Lui,
    Auipc,
    Op,
    Jal,
    Jalr,
    Branch,
    Load,
    Store,
    MiscMem,
    System,
    Amo,
}

pub struct Core {
    pub mem: Mem,
    pub csrs: [u32; 4096],
    pub reg: [u32; 32],
    pub pc: u32,
    pub cycle_count: usize,
}

impl Core {
    pub fn new() -> Core {
        Core {
            mem: Default::default(),
            csrs: [0; 4096],
            reg: [0; 32],
            pc: 0x80000000,
            cycle_count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.reg = [0; 32];
        self.pc = 0x80000000;
    }

    pub fn load_image(&mut self, mut data: Vec<u8>) {
        assert!(data.len() <= MEMORY_SIZE, "Image does not fit in memory!");

        self.reset();
        data.extend(std::iter::repeat(0).take(MEMORY_SIZE - data.len()));
        self.mem.mem.clone_from_slice(data.as_slice());
    }

    pub fn run(&mut self) {
        loop {
            self.step();
        }
    }

    pub fn step(&mut self) {
        let inst = self.mem.lw(self.pc);

        let rs1_raw = (inst >> 15) & 0b11111;
        let rs2_raw = (inst >> 20) & 0b11111;
        let rd_raw = (inst >> 7) & 0b11111;
        let funct3 = (inst >> 12) & 0b111;
        let funct7 = (inst >> 25) & 0b1111111;

        let rs1 = self.reg[rs1_raw as usize];
        let rs2 = self.reg[rs2_raw as usize];
        let rd = &mut self.reg[rd_raw as usize];

        let opcode = match (inst & 0b1111100) >> 2 {
            0b00000 => Opcode::Load,
            0b01000 => Opcode::Store,
            0b11000 => Opcode::Branch,
            0b11001 => Opcode::Jalr,
            0b11011 => Opcode::Jal,
            0b00011 => Opcode::MiscMem,
            0b00100 => Opcode::OpImm,
            0b01100 => Opcode::Op,
            0b11100 => Opcode::System,
            0b00101 => Opcode::Auipc,
            0b01101 => Opcode::Lui,
            0b01011 => Opcode::Amo,
            x => panic!("Hit invalid opcode: {:05b}", x),
        };

        println!(
            "Inst: {:032b}, pc: {:08x}, Opcode: {:?}",
            inst, self.pc, opcode
        );

        match opcode {
            Opcode::Load => {
                let addr = rs1 + read_imm_i(inst);
                *rd = match funct3 {
                    0 => self.mem.lb(addr) as i8 as i32 as u32,
                    1 => self.mem.lh(addr) as i32 as u32,
                    2 => self.mem.lw(addr),
                    4 => self.mem.lb(addr) as u32,
                    5 => self.mem.lh(addr) as u32,
                    x => {
                        println!("Invalid load width: {}", x);
                        return;
                    }
                };
                self.pc += 4;
            }
            Opcode::Store => {
                let addr = rs1 + read_imm_s(inst);
                match funct3 {
                    0 => self.mem.sb(addr, rs2 as u8),
                    1 => self.mem.sh(addr, rs2 as u16),
                    2 => self.mem.sw(addr, rs2 as u32),
                    x => {
                        println!("Invalid store width: {}", x);
                        return;
                    }
                };
                self.pc += 4;
            }
            Opcode::Branch => {
                let cond = match funct3 {
                    0 => rs1 == rs2,
                    1 => rs1 != rs2,
                    4 => (rs1 as i32) < (rs2 as i32),
                    5 => (rs1 as i32) >= (rs2 as i32),
                    6 => rs1 < rs2,
                    7 => rs1 >= rs2,
                    x => {
                        println!("Invalid branch condition: {}", x);
                        return;
                    }
                };
                if cond {
                    self.pc += read_imm_b(inst);
                } else {
                    self.pc += 4;
                }
            }
            Opcode::Jalr => {
                *rd = self.pc + 4;
                self.pc = rs1 + (read_imm_i(inst) & !1);
            }
            Opcode::Jal => {
                *rd = self.pc + 4;
                self.pc += read_imm_j(inst);
            }
            Opcode::MiscMem => {
                // No need for fencing: only one hart
                self.pc += 4;
            }
            Opcode::OpImm => {
                let imm = read_imm_i(inst);
                *rd = match funct3 {
                    0 => rs1 + imm,
                    2 => ((rs1 as i32) < (imm as i32)) as u32,
                    3 => (rs1 < imm) as u32,
                    4 => rs1 ^ imm,
                    6 => rs1 | imm,
                    7 => rs1 & imm,
                    1 => rs1 << (imm & 0b11111),
                    5 => {
                        let shamt = imm & 0b11111;
                        if (imm & (1 << 10)) == 0 {
                            rs1 >> shamt
                        } else {
                            ((rs1 as i32) >> shamt) as u32
                        }
                    }
                    _ => unreachable!(),
                };
                self.pc += 4;
            }
            Opcode::Op => {
                *rd = match funct3 {
                    0 => {
                        if (funct7 & (1 << 5)) == 0 {
                            rs1 + rs2
                        } else {
                            rs1 - rs2
                        }
                    }
                    2 => ((rs1 as i32) < (rs2 as i32)) as u32,
                    3 => (rs1 < rs2) as u32,
                    4 => rs1 ^ rs2,
                    6 => rs1 | rs2,
                    7 => rs1 & rs2,
                    1 => rs1 << (rs2 & 0b11111),
                    5 => {
                        let shamt = rs2 & 0b11111;
                        if (funct7 & (1 << 5)) == 0 {
                            rs1 >> shamt
                        } else {
                            ((rs1 as i32) >> shamt) as u32
                        }
                    }
                    _ => unreachable!(),
                };
                self.pc += 4;
            }
            Opcode::System => {
                let funct12 = inst >> 20;
                match funct3 {
                    0b000 => {
                        if funct12 == 0 {
                            todo!("ECALL");
                        } else if funct12 == 1 {
                            println!("Hit EBREAK");
                            println!("Cycle count: {}", self.cycle_count);
                            println!("Register state:");
                            for (i, &val) in self.reg.iter().enumerate() {
                                println!(" x{}: {:x} ({})", i, val, val);
                            }
                            std::process::exit(0);
                        } else {
                            panic!();
                        }
                    }
                    0b001 => {
                        let temp = self.csrs[funct12 as usize];
                        self.csrs[funct12 as usize] = rs1;
                        *rd = temp;
                    }
                    0b010 => {
                        let temp = self.csrs[funct12 as usize];
                        self.csrs[funct12 as usize] |= rs1;
                        *rd = temp;
                    }
                    0b011 => {
                        let temp = self.csrs[funct12 as usize];
                        self.csrs[funct12 as usize] = temp & !rs1;
                        *rd = temp;
                    }
                    0b101 => {
                        let temp = self.csrs[funct12 as usize];
                        self.csrs[funct12 as usize] = rs1_raw;
                        *rd = temp;
                    }
                    0b110 => {
                        let temp = self.csrs[funct12 as usize];
                        self.csrs[funct12 as usize] |= rs1_raw;
                        *rd = temp;
                    }
                    0b111 => {
                        let temp = self.csrs[funct12 as usize];
                        self.csrs[funct12 as usize] = temp & !rs1_raw;
                        *rd = temp;
                    }
                    _ => panic!()
                }
                self.pc += 4;
            }
            Opcode::Auipc => {
                *rd = self.pc + read_imm_u(inst);
                self.pc += 4;
            }
            Opcode::Lui => {
                *rd = read_imm_u(inst);
                self.pc += 4;
            }
            Opcode::Amo => {
                assert!(funct3 == 0b010, "Invalid width for AMO");
                match funct7 >> 2 {
                    0b00010 => {
                        *rd = self.mem.lw(rs1);
                    }
                    0b00011 => {
                        self.mem.sw(rs1, rs2);
                        *rd = 0;
                    }
                    0b00001 => {
                        let temp = self.mem.lw(rs1);
                        self.mem.sw(rs1, rs2);
                        *rd = temp;
                    }
                    x => {
                        let temp = self.mem.lw(rs1);
                        *rd = temp;
                        let new = match x {
                            0b00000 => temp + rs2,
                            0b00100 => temp ^ rs2,
                            0b01100 => temp & rs2,
                            0b01000 => temp | rs2,
                            0b10000 => (temp as i32).min(rs2 as i32) as u32,
                            0b10100 => (temp as i32).max(rs2 as i32) as u32,
                            0b11000 => temp.min(rs2),
                            0b11100 => temp.max(rs2),
                            x => panic!("{}", x),
                        };
                        self.mem.sw(rs1, new);
                    }
                }
                self.pc += 4;
            }
        }

        self.cycle_count += 1;
        // writes to x0 are discarded
        self.reg[0] = 0;
    }
}
