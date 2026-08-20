use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

const DEFAULT_MEMORY_SIZE: usize = 1024 * 1024;
const DEFAULT_MAX_STEPS: u64 = 100_000_000;
const SP: usize = 13;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    EndOfImage,
    Exit(u32),
}

struct Cpu {
    regs: [u32; 16],
    pc: u32,
    memory: Vec<u8>,
    image_len: u32,
    trace: bool,
    steps: u64,
    max_steps: u64,
}

impl Cpu {
    fn new(image: &[u8], memory_size: usize, trace: bool, max_steps: u64) -> Result<Self, String> {
        if image.len() % 2 != 0 {
            return Err("binary length must be a multiple of 2 bytes".into());
        }
        if image.len() > u32::MAX as usize {
            return Err("binary is too large for the 32-bit SIA address space".into());
        }
        let memory_size = memory_size.max(image.len()).max(16);
        if memory_size > u32::MAX as usize {
            return Err("memory size exceeds the 32-bit SIA address space".into());
        }

        let mut memory = vec![0u8; memory_size];
        memory[..image.len()].copy_from_slice(image);

        let mut regs = [0u32; 16];
        // The standard ABI requires a 16-byte-aligned call-boundary stack.
        regs[SP] = (memory_size as u32) & !15;

        Ok(Self {
            regs,
            pc: 0,
            memory,
            image_len: image.len() as u32,
            trace,
            steps: 0,
            max_steps,
        })
    }

    fn read_reg(&self, reg: usize) -> u32 {
        if reg == 0 {
            0
        } else {
            self.regs[reg]
        }
    }

    fn write_reg(&mut self, reg: usize, value: u32) {
        if reg != 0 {
            self.regs[reg] = value;
        }
    }

    fn run(&mut self) -> Result<Stop, String> {
        loop {
            if self.pc == self.image_len {
                return Ok(Stop::EndOfImage);
            }
            if self.pc > self.image_len
                || self
                    .pc
                    .checked_add(2)
                    .map_or(true, |end| end > self.image_len)
            {
                return Err(format!(
                    "PC 0x{:08x} is outside the executable image",
                    self.pc
                ));
            }
            if self.pc & 1 != 0 {
                return Err(format!("misaligned instruction PC 0x{:08x}", self.pc));
            }
            if self.steps >= self.max_steps {
                return Err(format!(
                    "maximum step count ({}) exceeded",
                    self.max_steps
                ));
            }
            self.steps += 1;

            if let Some(stop) = self.step()? {
                return Ok(stop);
            }
        }
    }

    fn step(&mut self) -> Result<Option<Stop>, String> {
        let pc = self.pc;
        let insn = self.fetch_u16(pc)?;
        let next_pc = pc.wrapping_add(2);
        self.pc = next_pc;

        if self.trace {
            eprintln!(
                "pc={pc:08x} insn={insn:04x} {}",
                self.disassemble(insn)
            );
        }

        let primary = (insn >> 12) as u8;
        match primary {
            0x0 => self.exec_add_or_clz(insn),
            0x1 => self.exec_cmov_or_ctz(insn),
            0x2 => self.exec_scaled_load_or_cpop(insn),
            0x3 => self.exec_scaled_store(insn),
            0x4 => self.exec_arithmetic(insn),
            0x5 => self.exec_logic_shift(insn),
            0x6 => self.exec_immediate(insn),
            0x7 => self.exec_scalar_memory(insn),
            0x8 => self.exec_multi_memory(insn),
            0x9 => self.exec_ldpc(insn, pc),
            0xA => self.exec_cond_branch(insn, next_pc),
            0xB => self.exec_direct_branch(insn, next_pc),
            0xC => return self.exec_misc(insn, next_pc),
            0xD => self.exec_adc(insn),
            0xE => self.exec_sbb(insn),
            0xF => Err("reserved EXT prefix executed".into()),
            _ => unreachable!(),
        }?;

        Ok(None)
    }

    fn exec_add_or_clz(&mut self, insn: u16) -> Result<(), String> {
        let rd = nibble(insn, 8);
        let ra = nibble(insn, 4);
        let rb = nibble(insn, 0);
        if rd == 0 {
            self.write_reg(ra, self.read_reg(rb).leading_zeros());
        } else {
            self.write_reg(rd, self.read_reg(ra).wrapping_add(self.read_reg(rb)));
        }
        Ok(())
    }

    fn exec_cmov_or_ctz(&mut self, insn: u16) -> Result<(), String> {
        let rd = nibble(insn, 8);
        let rs = nibble(insn, 4);
        let rc = nibble(insn, 0);
        if rd == 0 {
            self.write_reg(rs, self.read_reg(rc).trailing_zeros());
        } else if self.read_reg(rc) != 0 {
            self.write_reg(rd, self.read_reg(rs));
        }
        Ok(())
    }

    fn exec_scaled_load_or_cpop(&mut self, insn: u16) -> Result<(), String> {
        let rd = nibble(insn, 8);
        let rb = nibble(insn, 4);
        let ri = nibble(insn, 0);
        if rd == 0 {
            self.write_reg(rb, self.read_reg(ri).count_ones());
        } else {
            let addr = self
                .read_reg(rb)
                .wrapping_add(self.read_reg(ri).wrapping_shl(2));
            let value = self.load_u32(addr)?;
            self.write_reg(rd, value);
        }
        Ok(())
    }

    fn exec_scaled_store(&mut self, insn: u16) -> Result<(), String> {
        let rs = nibble(insn, 8);
        let rb = nibble(insn, 4);
        let ri = nibble(insn, 0);
        let addr = self
            .read_reg(rb)
            .wrapping_add(self.read_reg(ri).wrapping_shl(2));
        self.store_u32(addr, self.read_reg(rs))
    }

    fn exec_arithmetic(&mut self, insn: u16) -> Result<(), String> {
        let rd = nibble(insn, 8);
        let rs = nibble(insn, 4);
        let func = (insn & 0xF) as u8;
        let a = self.read_reg(rd);
        let b = self.read_reg(rs);
        let value = match func {
            0x0 => a.wrapping_sub(b),
            0x1 => {
                let (v, overflow) = (a as i32).overflowing_add(b as i32);
                if overflow {
                    return Err("ADDO signed overflow".into());
                }
                v as u32
            }
            0x2 => {
                let (v, overflow) = (a as i32).overflowing_sub(b as i32);
                if overflow {
                    return Err("SUBO signed overflow".into());
                }
                v as u32
            }
            0x3 => bool_mask(a == b),
            0x4 => bool_mask((a as i32) < (b as i32)),
            0x5 => bool_mask(a < b),
            0x6 => (a as i32).min(b as i32) as u32,
            0x7 => a.min(b),
            0x8 => (a as i32).max(b as i32) as u32,
            0x9 => a.max(b),
            _ => return Err(format!("reserved arithmetic function {func:#x}")),
        };
        self.write_reg(rd, value);
        Ok(())
    }

    fn exec_logic_shift(&mut self, insn: u16) -> Result<(), String> {
        let rd = nibble(insn, 8);
        let rs_or_imm4 = nibble(insn, 4);
        let func = (insn & 0xF) as u8;
        let a = self.read_reg(rd);
        let value = match func {
            0x0 => a & self.read_reg(rs_or_imm4),
            0x1 => a | self.read_reg(rs_or_imm4),
            0x2 => a ^ self.read_reg(rs_or_imm4),
            0x3 => a.wrapping_shl(self.read_reg(rs_or_imm4) & 31),
            0x4 => a.wrapping_shr(self.read_reg(rs_or_imm4) & 31),
            0x5 => ((a as i32) >> (self.read_reg(rs_or_imm4) & 31)) as u32,
            0x6 => a.wrapping_shl(rs_or_imm4 as u32),
            0x7 => a.wrapping_shl(rs_or_imm4 as u32 + 16),
            0x8 => a.wrapping_shr(rs_or_imm4 as u32),
            0x9 => a.wrapping_shr(rs_or_imm4 as u32 + 16),
            0xA => ((a as i32) >> (rs_or_imm4 as u32)) as u32,
            0xB => ((a as i32) >> (rs_or_imm4 as u32 + 16)) as u32,
            _ => return Err(format!("reserved logic/shift function {func:#x}")),
        };
        self.write_reg(rd, value);
        Ok(())
    }

    fn exec_immediate(&mut self, insn: u16) -> Result<(), String> {
        let op = (insn >> 11) & 1;
        let rd = ((insn >> 7) & 0xF) as usize;
        let imm = sign_extend((insn & 0x7F) as u32, 7) as u32;
        match op {
            0 => self.write_reg(rd, imm),
            1 => self.write_reg(rd, self.read_reg(rd).wrapping_add(imm)),
            _ => unreachable!(),
        }
        Ok(())
    }

    fn exec_scalar_memory(&mut self, insn: u16) -> Result<(), String> {
        // Modes 8..B (mode[3:2] == 10) are the v0.5 implicit-SP
        // subformat. Bits normally used for rb become S + immediate bits.
        if (insn & 0xC) == 0x8 {
            return self.exec_stack_memory(insn);
        }

        let rv = nibble(insn, 8);
        let rb = nibble(insn, 4);
        let mode = (insn & 0xF) as u8;
        let base = self.read_reg(rb);
        match mode {
            0x0 => {
                let v = self.load_u8(base)? as i8 as i32 as u32;
                self.write_reg(rv, v);
            }
            0x1 => {
                let v = self.load_u8(base)? as u32;
                self.write_reg(rv, v);
            }
            0x2 => {
                let v = self.load_u16(base)? as i16 as i32 as u32;
                self.write_reg(rv, v);
            }
            0x3 => {
                let v = self.load_u16(base)? as u32;
                self.write_reg(rv, v);
            }
            0x4 => {
                let v = self.load_u32(base)?;
                self.write_reg(rv, v);
            }
            0x5 => self.store_u8(base, self.read_reg(rv) as u8)?,
            0x6 => self.store_u16(base, self.read_reg(rv) as u16)?,
            0x7 => self.store_u32(base, self.read_reg(rv))?,
            0xC => {
                if rv == rb {
                    return Err("updating load may not use destination == base".into());
                }
                let v = self.load_u32(base)?;
                self.write_reg(rv, v);
                self.write_reg(rb, base.wrapping_add(4));
            }
            0xD => {
                self.store_u32(base, self.read_reg(rv))?;
                self.write_reg(rb, base.wrapping_add(4));
            }
            0xE => {
                if rv == rb {
                    return Err("updating load may not use destination == base".into());
                }
                let addr = base.wrapping_sub(4);
                let v = self.load_u32(addr)?;
                self.write_reg(rb, addr);
                self.write_reg(rv, v);
            }
            0xF => {
                let addr = base.wrapping_sub(4);
                let value = self.read_reg(rv);
                self.check_aligned(addr, 4)?;
                self.check_range(addr, 4)?;
                self.write_reg(rb, addr);
                self.store_u32(addr, value)?;
            }
            _ => unreachable!("stack-relative modes handled above"),
        }
        Ok(())
    }

    fn exec_stack_memory(&mut self, insn: u16) -> Result<(), String> {
        let rv = nibble(insn, 8);
        let store = ((insn >> 7) & 1) != 0;
        let imm5 = (((insn >> 4) & 0x7) << 2) | (insn & 0x3);

        if !store && rv == 0 {
            // LWSP r0 is architecturally reclaimed as ADJSP simm5*16.
            let simm5 = sign_extend(imm5 as u32, 5);
            let delta = (simm5 as u32).wrapping_mul(16);
            self.write_reg(SP, self.read_reg(SP).wrapping_add(delta));
            return Ok(());
        }

        let offset = (imm5 as u32) * 4;
        let addr = self.read_reg(SP).wrapping_add(offset);
        if store {
            self.store_u32(addr, self.read_reg(rv))
        } else {
            let value = self.load_u32(addr)?;
            self.write_reg(rv, value);
            Ok(())
        }
    }

    fn exec_multi_memory(&mut self, insn: u16) -> Result<(), String> {
        let first = nibble(insn, 8);
        let base_reg = nibble(insn, 4);
        let mode = (insn & 0xF) as u8;
        match mode {
            0x0 => self.load_group(first, 2, base_reg, Update::None),
            0x1 => self.load_group(first, 2, base_reg, Update::Post),
            0x2 => self.store_group(first, 2, base_reg, Update::None),
            0x3 => self.store_group(first, 2, base_reg, Update::Post),
            0x4 => self.store_group(first, 2, base_reg, Update::Pre),
            0x5 => self.load_group(first, 4, base_reg, Update::None),
            0x6 => self.load_group(first, 4, base_reg, Update::Post),
            0x7 => self.store_group(first, 4, base_reg, Update::None),
            0x8 => self.store_group(first, 4, base_reg, Update::Post),
            0x9 => self.store_group(first, 4, base_reg, Update::Pre),
            _ => Err(format!("reserved pair/quad-memory mode {mode:#x}")),
        }
    }

    fn load_group(
        &mut self,
        first: usize,
        count: usize,
        base_reg: usize,
        update: Update,
    ) -> Result<(), String> {
        if first + count > 16 {
            return Err("register group wraps beyond r15".into());
        }
        if update != Update::None && (first..first + count).contains(&base_reg) {
            return Err("updating multi-load base overlaps destination group".into());
        }
        let bytes = (count * 4) as u32;
        let base = self.read_reg(base_reg);
        let addr = match update {
            Update::Pre => base.wrapping_sub(bytes),
            _ => base,
        };
        self.check_aligned(addr, 4)?;
        self.check_range(addr, bytes as usize)?;

        let mut values = Vec::with_capacity(count);
        for i in 0..count {
            values.push(self.load_u32(addr.wrapping_add((i * 4) as u32))?);
        }
        for (i, value) in values.into_iter().enumerate() {
            self.write_reg(first + i, value);
        }
        match update {
            Update::Post => self.write_reg(base_reg, base.wrapping_add(bytes)),
            Update::Pre => self.write_reg(base_reg, addr),
            Update::None => {}
        }
        Ok(())
    }

    fn store_group(
        &mut self,
        first: usize,
        count: usize,
        base_reg: usize,
        update: Update,
    ) -> Result<(), String> {
        if first + count > 16 {
            return Err("register group wraps beyond r15".into());
        }
        let bytes = (count * 4) as u32;
        let base = self.read_reg(base_reg);
        let addr = match update {
            Update::Pre => base.wrapping_sub(bytes),
            _ => base,
        };
        self.check_aligned(addr, 4)?;
        self.check_range(addr, bytes as usize)?;

        let values: Vec<u32> = (0..count).map(|i| self.read_reg(first + i)).collect();
        if update == Update::Pre {
            self.write_reg(base_reg, addr);
        }
        for (i, value) in values.into_iter().enumerate() {
            self.store_u32(addr.wrapping_add((i * 4) as u32), value)?;
        }
        if update == Update::Post {
            self.write_reg(base_reg, base.wrapping_add(bytes));
        }
        Ok(())
    }

    fn exec_ldpc(&mut self, insn: u16, pc: u32) -> Result<(), String> {
        let rd = nibble(insn, 8);
        let disp = sign_extend((insn & 0xFF) as u32, 8);
        // Prototype choice: use a word-aligned PC+4 base. The exact
        // architectural PC-relative base remains open for measurement.
        let base = pc.wrapping_add(4) & !3;
        let addr = base.wrapping_add((disp as u32).wrapping_mul(4));
        let value = self.load_u32(addr)?;
        self.write_reg(rd, value);
        Ok(())
    }

    fn exec_cond_branch(&mut self, insn: u16, next_pc: u32) -> Result<(), String> {
        let op = ((insn >> 11) & 1) as u8;
        let rs = ((insn >> 7) & 0xF) as usize;
        let disp = sign_extend((insn & 0x7F) as u32, 7);
        match op {
            0 => {
                if self.read_reg(rs) != 0 {
                    self.pc = branch_target(next_pc, disp, 2);
                }
            }
            1 => {
                let value = self.read_reg(rs).wrapping_sub(1);
                self.write_reg(rs, value);
                if value != 0 {
                    self.pc = branch_target(next_pc, disp, 2);
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn exec_direct_branch(&mut self, insn: u16, next_pc: u32) -> Result<(), String> {
        let link = ((insn >> 11) & 1) != 0;
        let disp = sign_extend((insn & 0x7FF) as u32, 11);
        if link {
            self.write_reg(14, next_pc);
        }
        self.pc = branch_target(next_pc, disp, 2);
        Ok(())
    }

    fn exec_misc(&mut self, insn: u16, next_pc: u32) -> Result<Option<Stop>, String> {
        let rd = nibble(insn, 8);
        let rs = nibble(insn, 4);
        let func = (insn & 0xF) as u8;
        let a = self.read_reg(rd);
        let b = self.read_reg(rs);
        match func {
            0x0 => {
                let target = b & !1;
                self.write_reg(rd, next_pc);
                self.pc = target;
            }
            0x1 => self.write_reg(rd, a | (1u32 << (b & 31))),
            0x2 => self.write_reg(rd, a & !(1u32 << (b & 31))),
            0x3 => self.write_reg(rd, a ^ (1u32 << (b & 31))),
            0x4 => self.write_reg(rd, (a >> (b & 31)) & 1),
            0x5 => self.write_reg(rd, a.wrapping_mul(b)),
            0x6 => {
                let product = (a as i32 as i64) * (b as i32 as i64);
                self.write_reg(rd, (product >> 32) as u32);
            }
            0x7 => {
                let product = (a as u64) * (b as u64);
                self.write_reg(rd, (product >> 32) as u32);
            }
            0x8 => {
                let product = (a as i32 as i64) * (b as u64 as i64);
                self.write_reg(rd, (product >> 32) as u32);
            }
            0x9 => {
                let product = (a as i32 as i64) * (b as i32 as i64);
                if product < i32::MIN as i64 || product > i32::MAX as i64 {
                    return Err("MULO signed overflow".into());
                }
                self.write_reg(rd, product as u32);
            }
            0xA => {
                let divisor = b as i32;
                let dividend = a as i32;
                if divisor == 0 {
                    return Err("DIV by zero".into());
                }
                if dividend == i32::MIN && divisor == -1 {
                    return Err("DIV signed overflow".into());
                }
                self.write_reg(rd, (dividend / divisor) as u32);
            }
            0xB => {
                if b == 0 {
                    return Err("DIVU by zero".into());
                }
                self.write_reg(rd, a / b);
            }
            0xC => {
                let divisor = b as i32;
                let dividend = a as i32;
                if divisor == 0 {
                    return Err("REM by zero".into());
                }
                if dividend == i32::MIN && divisor == -1 {
                    self.write_reg(rd, 0);
                } else {
                    self.write_reg(rd, (dividend % divisor) as u32);
                }
            }
            0xD => {
                if b == 0 {
                    return Err("REMU by zero".into());
                }
                self.write_reg(rd, a % b);
            }
            0xE => self.write_reg(rd, b.swap_bytes()),
            0xF => {
                let imm8 = ((insn >> 4) & 0xFF) as u8;
                return self.exec_system(imm8);
            }
            _ => unreachable!(),
        }
        Ok(None)
    }

    fn exec_system(&mut self, imm8: u8) -> Result<Option<Stop>, String> {
        match imm8 {
            0xFE => Err("BREAK instruction executed".into()),
            0xFF => Ok(None),
            trap => self.exec_trap(trap),
        }
    }

    fn exec_trap(&mut self, trap: u8) -> Result<Option<Stop>, String> {
        match trap {
            0 => Ok(Some(Stop::Exit(self.read_reg(1)))),
            1 => {
                let fd = self.read_reg(1);
                let addr = self.read_reg(2);
                let len = self.read_reg(3) as usize;
                self.check_range(addr, len)?;
                let start = addr as usize;
                let bytes = &self.memory[start..start + len];
                match fd {
                    1 => {
                        let mut out = io::stdout().lock();
                        out.write_all(bytes)
                            .map_err(|e| format!("stdout write failed: {e}"))?;
                        out.flush()
                            .map_err(|e| format!("stdout flush failed: {e}"))?;
                    }
                    2 => {
                        let mut out = io::stderr().lock();
                        out.write_all(bytes)
                            .map_err(|e| format!("stderr write failed: {e}"))?;
                        out.flush()
                            .map_err(|e| format!("stderr flush failed: {e}"))?;
                    }
                    _ => {
                        return Err(format!(
                            "TRAP 1 write: unsupported fd {fd}; use 1 or 2"
                        ))
                    }
                }
                self.write_reg(1, len as u32);
                Ok(None)
            }
            2 => {
                let byte = (self.read_reg(1) & 0xFF) as u8;
                let mut out = io::stdout().lock();
                out.write_all(&[byte])
                    .map_err(|e| format!("stdout write failed: {e}"))?;
                out.flush()
                    .map_err(|e| format!("stdout flush failed: {e}"))?;
                Ok(None)
            }
            _ => Err(format!("unsupported emulator semihosting TRAP {trap}")),
        }
    }

    fn exec_adc(&mut self, insn: u16) -> Result<(), String> {
        let rd = nibble(insn, 8);
        let rs = nibble(insn, 4);
        let rc = nibble(insn, 0);
        let cin = u64::from(self.read_reg(rc) != 0);
        let sum = self.read_reg(rd) as u64 + self.read_reg(rs) as u64 + cin;
        self.write_reg(rd, sum as u32);
        self.write_reg(rc, bool_mask((sum >> 32) != 0));
        Ok(())
    }

    fn exec_sbb(&mut self, insn: u16) -> Result<(), String> {
        let rd = nibble(insn, 8);
        let rs = nibble(insn, 4);
        let rc = nibble(insn, 0);
        let bin = u64::from(self.read_reg(rc) != 0);
        let rhs = self.read_reg(rs) as u64 + bin;
        let lhs = self.read_reg(rd) as u64;
        self.write_reg(rd, lhs.wrapping_sub(rhs) as u32);
        self.write_reg(rc, bool_mask(lhs < rhs));
        Ok(())
    }

    fn fetch_u16(&self, addr: u32) -> Result<u16, String> {
        self.check_aligned(addr, 2)?;
        self.check_range(addr, 2)?;
        let a = addr as usize;
        Ok(u16::from_le_bytes([self.memory[a], self.memory[a + 1]]))
    }

    fn load_u8(&self, addr: u32) -> Result<u8, String> {
        self.check_range(addr, 1)?;
        Ok(self.memory[addr as usize])
    }

    fn load_u16(&self, addr: u32) -> Result<u16, String> {
        self.check_aligned(addr, 2)?;
        self.check_range(addr, 2)?;
        let a = addr as usize;
        Ok(u16::from_le_bytes([
            self.memory[a],
            self.memory[a + 1],
        ]))
    }

    fn load_u32(&self, addr: u32) -> Result<u32, String> {
        self.check_aligned(addr, 4)?;
        self.check_range(addr, 4)?;
        let a = addr as usize;
        Ok(u32::from_le_bytes([
            self.memory[a],
            self.memory[a + 1],
            self.memory[a + 2],
            self.memory[a + 3],
        ]))
    }

    fn store_u8(&mut self, addr: u32, value: u8) -> Result<(), String> {
        self.check_range(addr, 1)?;
        self.memory[addr as usize] = value;
        Ok(())
    }

    fn store_u16(&mut self, addr: u32, value: u16) -> Result<(), String> {
        self.check_aligned(addr, 2)?;
        self.check_range(addr, 2)?;
        let a = addr as usize;
        self.memory[a..a + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn store_u32(&mut self, addr: u32, value: u32) -> Result<(), String> {
        self.check_aligned(addr, 4)?;
        self.check_range(addr, 4)?;
        let a = addr as usize;
        self.memory[a..a + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn check_aligned(&self, addr: u32, align: u32) -> Result<(), String> {
        if addr % align != 0 {
            Err(format!(
                "misaligned {align}-byte access at 0x{addr:08x}"
            ))
        } else {
            Ok(())
        }
    }

    fn check_range(&self, addr: u32, len: usize) -> Result<(), String> {
        let start = addr as usize;
        let end = start
            .checked_add(len)
            .ok_or_else(|| "address overflow".to_string())?;
        if end > self.memory.len() {
            Err(format!(
                "memory access 0x{addr:08x}..0x{end:08x} outside {} bytes of RAM",
                self.memory.len()
            ))
        } else {
            Ok(())
        }
    }

    fn disassemble(&self, insn: u16) -> String {
        let p = insn >> 12;
        match p {
            0x0 => format!(
                "ADD/CLZ r{},r{},r{}",
                nibble(insn, 8),
                nibble(insn, 4),
                nibble(insn, 0)
            ),
            0x1 => format!(
                "CMOV/CTZ r{},r{},r{}",
                nibble(insn, 8),
                nibble(insn, 4),
                nibble(insn, 0)
            ),
            0x2 => format!(
                "LDA/CPOP r{},r{},r{}",
                nibble(insn, 8),
                nibble(insn, 4),
                nibble(insn, 0)
            ),
            0x3 => format!(
                "STA r{},r{},r{}",
                nibble(insn, 8),
                nibble(insn, 4),
                nibble(insn, 0)
            ),
            0x4 => format!(
                "ARITH r{},r{},fn={:x}",
                nibble(insn, 8),
                nibble(insn, 4),
                insn & 0xf
            ),
            0x5 => format!(
                "LOGIC r{},r{},fn={:x}",
                nibble(insn, 8),
                nibble(insn, 4),
                insn & 0xf
            ),
            0x6 => format!(
                "IMM op={} r{} imm={}",
                (insn >> 11) & 1,
                (insn >> 7) & 0xf,
                sign_extend((insn & 0x7f) as u32, 7)
            ),
            0x7 => self.disassemble_scalar_memory(insn),
            0x8 => format!(
                "MULTI r{},r{},mode={:x}",
                nibble(insn, 8),
                nibble(insn, 4),
                insn & 0xf
            ),
            0x9 => format!(
                "LDPC r{},disp={}",
                nibble(insn, 8),
                sign_extend((insn & 0xff) as u32, 8)
            ),
            0xA => format!(
                "{} r{},disp={}",
                if (insn >> 11) & 1 == 0 {
                    "BNZ"
                } else {
                    "DBNZ"
                },
                (insn >> 7) & 0xf,
                sign_extend((insn & 0x7f) as u32, 7)
            ),
            0xB => format!(
                "{} disp={}",
                if (insn >> 11) & 1 == 0 { "B" } else { "BL" },
                sign_extend((insn & 0x7ff) as u32, 11)
            ),
            0xC => format!(
                "MISC r{},r{},fn={:x}",
                nibble(insn, 8),
                nibble(insn, 4),
                insn & 0xf
            ),
            0xD => format!(
                "ADC r{},r{},r{}",
                nibble(insn, 8),
                nibble(insn, 4),
                nibble(insn, 0)
            ),
            0xE => format!(
                "SBB r{},r{},r{}",
                nibble(insn, 8),
                nibble(insn, 4),
                nibble(insn, 0)
            ),
            0xF => "EXT(reserved)".into(),
            _ => unreachable!(),
        }
    }

    fn disassemble_scalar_memory(&self, insn: u16) -> String {
        if (insn & 0xC) == 0x8 {
            let rv = nibble(insn, 8);
            let store = ((insn >> 7) & 1) != 0;
            let imm5 = (((insn >> 4) & 0x7) << 2) | (insn & 0x3);
            if !store && rv == 0 {
                let simm5 = sign_extend(imm5 as u32, 5);
                return format!("ADJSP {}", simm5 * 16);
            }
            return format!(
                "{} r{},{}",
                if store { "SWSP" } else { "LWSP" },
                rv,
                imm5 * 4
            );
        }

        format!(
            "MEM r{},r{},mode={:x}",
            nibble(insn, 8),
            nibble(insn, 4),
            insn & 0xf
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Update {
    None,
    Post,
    Pre,
}

fn nibble(word: u16, shift: u16) -> usize {
    ((word >> shift) & 0xF) as usize
}

fn bool_mask(value: bool) -> u32 {
    if value {
        u32::MAX
    } else {
        0
    }
}

fn sign_extend(value: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((value << shift) as i32) >> shift
}

fn branch_target(next_pc: u32, displacement: i32, scale: u32) -> u32 {
    next_pc.wrapping_add((displacement as u32).wrapping_mul(scale))
}

struct Options {
    path: String,
    memory_size: usize,
    trace: bool,
    max_steps: u64,
}

fn parse_options() -> Result<Options, String> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(usage)?;
    if path == "-h" || path == "--help" {
        return Err(usage());
    }

    let mut memory_size = DEFAULT_MEMORY_SIZE;
    let mut trace = false;
    let mut max_steps = DEFAULT_MAX_STEPS;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trace" => trace = true,
            "--memory" => {
                let value = args.next().ok_or("--memory requires a byte count")?;
                memory_size = value
                    .parse()
                    .map_err(|_| format!("invalid --memory value: {value}"))?;
            }
            "--max-steps" => {
                let value = args.next().ok_or("--max-steps requires a count")?;
                max_steps = value
                    .parse()
                    .map_err(|_| format!("invalid --max-steps value: {value}"))?;
            }
            _ => return Err(format!("unknown argument: {arg}\n\n{}", usage())),
        }
    }

    Ok(Options {
        path,
        memory_size,
        trace,
        max_steps,
    })
}

fn usage() -> String {
    "usage: siaemu <program.bin> [--memory BYTES] [--max-steps N] [--trace]\n\n\
     The raw binary is loaded at address 0 and execution starts at PC=0.\n\
     Execution stops when PC reaches the end of the image or TRAP 0 exits.\n\
     r13 (sp) starts 16-byte aligned at the top of RAM. Default RAM: 1 MiB."
        .into()
}

fn main() {
    let options = match parse_options() {
        Ok(v) => v,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    let image = match fs::read(&options.path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to read {}: {e}", options.path);
            process::exit(2);
        }
    };

    let mut cpu = match Cpu::new(
        &image,
        options.memory_size,
        options.trace,
        options.max_steps,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("load error: {e}");
            process::exit(2);
        }
    };

    match cpu.run() {
        Ok(Stop::EndOfImage) => process::exit(0),
        Ok(Stop::Exit(code)) => process::exit((code & 0xFF) as i32),
        Err(e) => {
            eprintln!(
                "SIA fault after {} steps at PC=0x{:08x}: {e}",
                cpu.steps, cpu.pc
            );
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    fn li(rd: u16, imm: i8) -> u16 {
        assert!((-64..=63).contains(&imm));
        0x6000 | (rd << 7) | ((imm as u8 as u16) & 0x7f)
    }

    fn add(rd: u16, ra: u16, rb: u16) -> u16 {
        (rd << 8) | (ra << 4) | rb
    }

    fn stack_mem(reg: u16, imm5: u16, store: bool) -> u16 {
        assert!(reg < 16);
        assert!(imm5 < 32);
        0x7000
            | (reg << 8)
            | ((store as u16) << 7)
            | (((imm5 >> 2) & 0x7) << 4)
            | 0x8
            | (imm5 & 0x3)
    }

    fn lwsp(reg: u16, byte_offset: u16) -> u16 {
        assert_eq!(byte_offset % 4, 0);
        stack_mem(reg, byte_offset / 4, false)
    }

    fn swsp(reg: u16, byte_offset: u16) -> u16 {
        assert_eq!(byte_offset % 4, 0);
        stack_mem(reg, byte_offset / 4, true)
    }

    fn adjsp(byte_delta: i16) -> u16 {
        assert_eq!(byte_delta % 16, 0);
        let units = byte_delta / 16;
        assert!((-16..=15).contains(&units));
        stack_mem(0, (units as u16) & 0x1f, false)
    }

    #[test]
    fn li_and_add() {
        let image = words(&[li(1, 40), li(2, 2), add(3, 1, 2)]);
        let mut cpu = Cpu::new(&image, 1024, false, 100).unwrap();
        assert_eq!(cpu.run().unwrap(), Stop::EndOfImage);
        assert_eq!(cpu.read_reg(3), 42);
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn post_increment_store_and_load() {
        let image = words(&[
            li(1, 63),
            0x6881,
            li(2, 42),
            0x7000 | (2 << 8) | (1 << 4) | 0xD,
            0x6800 | (1 << 7) | 0x7C,
            0x7000 | (3 << 8) | (1 << 4) | 0xC,
        ]);
        let mut cpu = Cpu::new(&image, 1024, false, 100).unwrap();
        cpu.run().unwrap();
        assert_eq!(cpu.read_reg(3), 42);
        assert_eq!(cpu.read_reg(1), 68);
    }

    #[test]
    fn stack_relative_spill_reload_and_adjust() {
        let image = words(&[
            adjsp(-64),
            li(4, 42),
            swsp(4, 20),
            li(4, 0),
            lwsp(4, 20),
            adjsp(64),
        ]);
        let mut cpu = Cpu::new(&image, 1024, false, 100).unwrap();
        assert_eq!(cpu.read_reg(SP), 1024);
        cpu.run().unwrap();
        assert_eq!(cpu.read_reg(4), 42);
        assert_eq!(cpu.read_reg(SP), 1024);
    }

    #[test]
    fn swsp_r0_stores_zero() {
        let image = words(&[adjsp(-32), li(4, 17), swsp(4, 0), swsp(0, 0), lwsp(5, 0)]);
        let mut cpu = Cpu::new(&image, 1024, false, 100).unwrap();
        cpu.run().unwrap();
        assert_eq!(cpu.read_reg(5), 0);
    }

    #[test]
    fn dbnz_loops() {
        let image = words(&[
            li(1, 3),
            li(2, 0),
            0x6800 | (2 << 7) | 1,
            0xA800 | (1 << 7) | 0x7E,
        ]);
        let mut cpu = Cpu::new(&image, 1024, false, 100).unwrap();
        cpu.run().unwrap();
        assert_eq!(cpu.read_reg(2), 3);
        assert_eq!(cpu.read_reg(1), 0);
    }

    #[test]
    fn pair_store_and_load() {
        let image = words(&[
            li(1, 63),
            0x6881,
            li(4, 11),
            li(5, 22),
            0x8000 | (4 << 8) | (1 << 4) | 0x3,
            0x6800 | (1 << 7) | 0x78,
            0x8000 | (6 << 8) | (1 << 4) | 0x1,
        ]);
        let mut cpu = Cpu::new(&image, 1024, false, 100).unwrap();
        cpu.run().unwrap();
        assert_eq!(cpu.read_reg(6), 11);
        assert_eq!(cpu.read_reg(7), 22);
    }
}
