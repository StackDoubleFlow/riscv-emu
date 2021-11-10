use std::convert::TryInto;
use byteorder::{LittleEndian, ByteOrder};

const MEMORY_SIZE: usize = 32768; // 32kb

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
}

struct Core {
    mem: [u8; MEMORY_SIZE],
    reg: [u32; 32],
    pc: u32,
    cycle_count: usize,
}

impl Core {
    fn new() -> Core {
        Core {
            mem: [0; MEMORY_SIZE],
            reg: [0; 32],
            pc: 0,
            cycle_count: 0,
        }
    }

    fn reset(&mut self) {
        self.reg = [0; 32];
        self.pc = 0;
    }

    fn load_image(&mut self, mut data: Vec<u8>) {
        assert!(data.len() <= MEMORY_SIZE, "Image does not fit in memory!");
        
        self.reset();
        data.extend(std::iter::repeat(0).take(MEMORY_SIZE - data.len()));
        self.mem.clone_from(&data.try_into().unwrap());
    }

    fn run(&mut self) {
        loop {
            self.step();
        }
    }

    fn step(&mut self) {
        let pc = self.pc as usize;
        let inst = LittleEndian::read_u32(&self.mem[pc..pc + 4]);

        let rs1 = (inst >> 15) & 0b11111;
        let rs2 = (inst >> 20) & 0b11111;
        let rd = (inst >> 7) & 0b11111;
        let funct3 = (inst >> 12) & 0b111;
        let funct7 = (inst >> 25) & 0b1111111;

        let rs1 = self.reg[rs1 as usize];
        let rs2 = self.reg[rs2 as usize];
        let rd = &mut self.reg[rd as usize];

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
            x => {
                println!("Hit invalid opcode: {:05b}", x);
                return;
            },
        };

        println!("pc: {:08x}, Opcode: {:?}", pc, opcode);

        match opcode {
            Opcode::Load => {
                let addr = (rs1 + read_imm_i(inst)) as usize;
                *rd = match funct3 {
                    0 => self.mem[addr] as i8 as i32 as u32,
                    1 => LittleEndian::read_i16(&self.mem[addr..addr + 2]) as i32 as u32,
                    2 => LittleEndian::read_u32(&self.mem[addr..addr + 4]),
                    4 => self.mem[addr] as u32,
                    5 => LittleEndian::read_u16(&self.mem[addr..addr + 2]) as u32,
                    x => {
                        println!("Invalid load width: {}", x);
                        return;
                    },
                };
                self.pc += 4;
            }
            Opcode::Store => {
                let addr = (rs1 + read_imm_s(inst)) as usize;
                match funct3 {
                    0 => self.mem[addr] = rs2 as u8,
                    1 => LittleEndian::write_u16(&mut self.mem[addr..addr + 2], rs2 as u16),
                    2 => LittleEndian::write_u32(&mut self.mem[addr..addr + 4], rs2 as u32),
                    x => {
                        println!("Invalid store width: {}", x);
                        return;
                    },
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
                    _ => unreachable!()
                };
                self.pc += 4;
            }
            Opcode::Op => {
                *rd = match funct3 {
                    0 => if (funct7 & (1 << 5)) == 0 { rs1 + rs2 } else { rs1 - rs2 },
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
                    _ => unreachable!()
                };
                self.pc += 4;
            }
            Opcode::System => {
                let funct12 = inst >> 20;
                match funct12 {
                    0 => todo!("ECALL"),
                    1 => {
                        println!("Hit EBREAK");
                        println!("Cycle count: {}", self.cycle_count);
                        println!("Register state:");
                        for (i, &val) in self.reg.iter().enumerate() {
                            println!(" x{}: {:x} ({})", i, val, val);
                        }
                        std::process::exit(0);
                    }
                    _ => unimplemented!()
                };
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
        }

        self.cycle_count += 1;
        // writes to x0 are discarded
        self.reg[0] = 0;
    }
}

fn main() {
    let mut core = Core::new();
    core.load_image(std::fs::read("test/image.bin").unwrap());
    core.run();
    // for _ in 0..100 {
    //     core.step();
    // }
}
