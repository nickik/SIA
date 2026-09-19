//! Exact executable encoding used by the canonical SIA32 architecture tooling.
//!
//! SIA32-I is fixed-width: every instruction is one little-endian 16-bit halfword.
//! This crate is the canonical executable encoding/assembly/disassembly reference shared by simulators and toolchains.

pub const BREAK:u16=0xCFEF;pub const NOP:u16=0xCFFF;pub const EXT_RESERVED:u16=0xF000;
#[inline] const fn r3(p:u16,a:u8,b:u8,c:u8)->u16{(p<<12)|((a as u16)<<8)|((b as u16)<<4)|c as u16}
#[inline] const fn r2f(p:u16,a:u8,b:u8,f:u8)->u16{(p<<12)|((a as u16)<<8)|((b as u16)<<4)|f as u16}
pub const fn add(rd:u8,ra:u8,rb:u8)->u16{r3(0x0,rd,ra,rb)} pub const fn mov(rd:u8,rs:u8)->u16{add(rd,0,rs)} pub const fn clz(rd:u8,rs:u8)->u16{r3(0,0,rd,rs)}
pub const fn cmov(rd:u8,rs:u8,rc:u8)->u16{r3(1,rd,rs,rc)} pub const fn ctz(rd:u8,rs:u8)->u16{r3(1,0,rd,rs)} pub const fn lda_w(rd:u8,rb:u8,ri:u8)->u16{r3(2,rd,rb,ri)} pub const fn cpop(rd:u8,rs:u8)->u16{r3(2,0,rd,rs)} pub const fn sta_w(rs:u8,rb:u8,ri:u8)->u16{r3(3,rs,rb,ri)}
pub const fn sub(rd:u8,rs:u8)->u16{r2f(4,rd,rs,0)} pub const fn addo(rd:u8,rs:u8)->u16{r2f(4,rd,rs,1)} pub const fn subo(rd:u8,rs:u8)->u16{r2f(4,rd,rs,2)} pub const fn cmpeq(rd:u8,rs:u8)->u16{r2f(4,rd,rs,3)} pub const fn cmplt(rd:u8,rs:u8)->u16{r2f(4,rd,rs,4)} pub const fn cmpltu(rd:u8,rs:u8)->u16{r2f(4,rd,rs,5)} pub const fn min(rd:u8,rs:u8)->u16{r2f(4,rd,rs,6)} pub const fn minu(rd:u8,rs:u8)->u16{r2f(4,rd,rs,7)} pub const fn max(rd:u8,rs:u8)->u16{r2f(4,rd,rs,8)} pub const fn maxu(rd:u8,rs:u8)->u16{r2f(4,rd,rs,9)}
pub const fn and(rd:u8,rs:u8)->u16{r2f(5,rd,rs,0)} pub const fn or(rd:u8,rs:u8)->u16{r2f(5,rd,rs,1)} pub const fn xor(rd:u8,rs:u8)->u16{r2f(5,rd,rs,2)} pub const fn shl(rd:u8,rs:u8)->u16{r2f(5,rd,rs,3)} pub const fn shr(rd:u8,rs:u8)->u16{r2f(5,rd,rs,4)} pub const fn sar(rd:u8,rs:u8)->u16{r2f(5,rd,rs,5)}
fn shift_imm(rd:u8,a:u8,lo:u8,hi:u8)->u16{assert!(a<32);let(i,f)=if a<16{(a,lo)}else{(a-16,hi)};r2f(5,rd,i,f)} pub fn shli(rd:u8,a:u8)->u16{shift_imm(rd,a,6,7)} pub fn shri(rd:u8,a:u8)->u16{shift_imm(rd,a,8,9)} pub fn sari(rd:u8,a:u8)->u16{shift_imm(rd,a,10,11)}
pub fn li(rd:u8,i:i8)->u16{assert!((-64..=63).contains(&i));0x6000|((rd as u16)<<7)|((i as u8 as u16)&0x7f)} pub fn addi(rd:u8,i:i8)->u16{assert!((-64..=63).contains(&i));0x6800|((rd as u16)<<7)|((i as u8 as u16)&0x7f)}
pub const fn lb(a:u8,b:u8)->u16{r2f(7,a,b,0)} pub const fn lbu(a:u8,b:u8)->u16{r2f(7,a,b,1)} pub const fn lh(a:u8,b:u8)->u16{r2f(7,a,b,2)} pub const fn lhu(a:u8,b:u8)->u16{r2f(7,a,b,3)} pub const fn lw(a:u8,b:u8)->u16{r2f(7,a,b,4)} pub const fn sb(a:u8,b:u8)->u16{r2f(7,a,b,5)} pub const fn sh(a:u8,b:u8)->u16{r2f(7,a,b,6)} pub const fn sw(a:u8,b:u8)->u16{r2f(7,a,b,7)} pub const fn lw_post(a:u8,b:u8)->u16{r2f(7,a,b,12)} pub const fn sw_post(a:u8,b:u8)->u16{r2f(7,a,b,13)} pub const fn lw_pre(a:u8,b:u8)->u16{r2f(7,a,b,14)} pub const fn sw_pre(a:u8,b:u8)->u16{r2f(7,a,b,15)}
pub const fn ldp(a:u8,b:u8)->u16{r2f(8,a,b,0)} pub const fn ldp_post(a:u8,b:u8)->u16{r2f(8,a,b,1)} pub const fn stp(a:u8,b:u8)->u16{r2f(8,a,b,2)} pub const fn stp_post(a:u8,b:u8)->u16{r2f(8,a,b,3)} pub const fn stp_pre(a:u8,b:u8)->u16{r2f(8,a,b,4)} pub const fn ld4(a:u8,b:u8)->u16{r2f(8,a,b,5)} pub const fn ld4_post(a:u8,b:u8)->u16{r2f(8,a,b,6)} pub const fn st4(a:u8,b:u8)->u16{r2f(8,a,b,7)} pub const fn st4_post(a:u8,b:u8)->u16{r2f(8,a,b,8)} pub const fn st4_pre(a:u8,b:u8)->u16{r2f(8,a,b,9)}
pub fn ldpc_w(rd:u8,d:i16)->u16{assert!((-128..=127).contains(&d));0x9000|((rd as u16)<<8)|((d as u16)&0xff)} pub fn bnz(rs:u8,d:i16)->u16{assert!((-64..=63).contains(&d));0xa000|((rs as u16)<<7)|((d as u16)&0x7f)} pub fn dbnz(rs:u8,d:i16)->u16{assert!((-64..=63).contains(&d));0xa800|((rs as u16)<<7)|((d as u16)&0x7f)} pub fn b(d:i16)->u16{assert!((-1024..=1023).contains(&d));0xb000|((d as u16)&0x7ff)} pub fn bl(d:i16)->u16{assert!((-1024..=1023).contains(&d));0xb800|((d as u16)&0x7ff)}
pub const fn jalr(a:u8,b:u8)->u16{r2f(12,a,b,0)} pub const fn jr(b:u8)->u16{jalr(0,b)} pub const fn callr(b:u8)->u16{jalr(14,b)} pub const fn ret()->u16{jalr(0,14)} pub const fn bset(a:u8,b:u8)->u16{r2f(12,a,b,1)} pub const fn bclr(a:u8,b:u8)->u16{r2f(12,a,b,2)} pub const fn binv(a:u8,b:u8)->u16{r2f(12,a,b,3)} pub const fn bext(a:u8,b:u8)->u16{r2f(12,a,b,4)} pub const fn mul(a:u8,b:u8)->u16{r2f(12,a,b,5)} pub const fn mulh(a:u8,b:u8)->u16{r2f(12,a,b,6)} pub const fn mulhu(a:u8,b:u8)->u16{r2f(12,a,b,7)} pub const fn mulhsu(a:u8,b:u8)->u16{r2f(12,a,b,8)} pub const fn mulo(a:u8,b:u8)->u16{r2f(12,a,b,9)} pub const fn div(a:u8,b:u8)->u16{r2f(12,a,b,10)} pub const fn divu(a:u8,b:u8)->u16{r2f(12,a,b,11)} pub const fn rem(a:u8,b:u8)->u16{r2f(12,a,b,12)} pub const fn remu(a:u8,b:u8)->u16{r2f(12,a,b,13)} pub const fn rev8(a:u8,b:u8)->u16{r2f(12,a,b,14)} pub const fn trap(i:u8)->u16{0xc00f|((i as u16)<<4)} pub const fn adc(a:u8,b:u8,c:u8)->u16{r3(13,a,b,c)} pub const fn sbb(a:u8,b:u8,c:u8)->u16{r3(14,a,b,c)}


pub const SYSREG_STATUS:u8=0;
pub const SYSREG_EPC:u8=1;
pub const SYSREG_CAUSE:u8=2;
pub const SYSREG_BADADDR:u8=3;
pub const SYSREG_SCRATCH:u8=4;
pub const SYSREG_VMCTX:u8=5;

const fn system(sysop:u8,reg:u8,selector:u8)->u16{0xF000|((sysop as u16)<<8)|((reg as u16)<<4)|selector as u16}
pub const fn sread(rd:u8,selector:u8)->u16{system(0,rd,selector)}
pub const fn swrite(rs:u8,selector:u8)->u16{system(1,rs,selector)}
pub const fn sswap_scratch(r:u8)->u16{system(2,r,SYSREG_SCRATCH)}
pub const fn sret()->u16{system(3,0,0)}
pub const fn sretctx(rs:u8)->u16{system(3,rs,1)}
pub const fn tlbfence()->u16{system(4,0,0)}
pub const fn tlbfence_va(rs:u8)->u16{system(4,rs,1)}
pub const fn tlbfence_asid(rs:u8)->u16{system(4,rs,2)}
pub const fn wfi()->u16{system(5,0,0)}
pub const fn sync_i()->u16{system(6,0,0)}
pub const fn fence()->u16{system(7,0,0)}

pub mod assembler;
pub mod disassembler;
pub use assembler::{assemble, AssembleError, AssembledProgram};
pub use disassembler::{decode_word, disassemble_bytes, disassemble_word, DecodedInstruction};
