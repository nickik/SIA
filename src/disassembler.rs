use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub pc: u32,
    pub word: u16,
    pub text: String,
}

fn r(n:u16)->String{match n{13=>"sp".into(),14=>"lr".into(),_=>format!("r{n}")}}
fn sx(v:u16,bits:u32)->i32{let s=32-bits;(((v as u32)<<s) as i32)>>s}
fn target(pc:u32,disp:i32)->u32{pc.wrapping_add(2).wrapping_add((disp as u32).wrapping_mul(2))}
fn sysreg(n:u16)->String{match n{0=>"STATUS".into(),1=>"EPC".into(),2=>"CAUSE".into(),3=>"BADADDR".into(),4=>"SCRATCH".into(),5=>"VMCTX".into(),_=>format!("sr{n}")}}

pub fn decode_word(word:u16,pc:u32)->DecodedInstruction{
    let op=(word>>12)&0xf; let a=(word>>8)&0xf; let b=(word>>4)&0xf; let c=word&0xf;
    let text=match op{
        0x0=>if a==0{format!("CLZ {}, {}",r(b),r(c))}else{format!("ADD {}, {}, {}",r(a),r(b),r(c))},
        0x1=>if a==0{format!("CTZ {}, {}",r(b),r(c))}else{format!("CMOV {}, {}, {}",r(a),r(b),r(c))},
        0x2=>if a==0{format!("CPOP {}, {}",r(b),r(c))}else{format!("LDA.W {}, [{}+{}*4]",r(a),r(b),r(c))},
        0x3=>format!("STA.W {}, [{}+{}*4]",r(a),r(b),r(c)),
        0x4=>{let m=["SUB","ADDO","SUBO","CMPEQ","CMPLT","CMPLTU","MIN","MINU","MAX","MAXU","?4a","?4b","?4c","?4d","?4e","?4f"][c as usize];format!("{m} {}, {}",r(a),r(b))},
        0x5=>match c{
            0..=5=>{let m=["AND","OR","XOR","SHL","SHR","SAR"][c as usize];format!("{m} {}, {}",r(a),r(b))},
            6|7=>format!("SHLI {}, {}",r(a),b+if c==7{16}else{0}),
            8|9=>format!("SHRI {}, {}",r(a),b+if c==9{16}else{0}),
            10|11=>format!("SARI {}, {}",r(a),b+if c==11{16}else{0}),
            _=>format!(".hword 0x{word:04x}"),
        },
        0x6=>{let rd=(word>>7)&0xf;let imm=sx(word&0x7f,7);if word&0x0800==0{format!("LI {}, {imm}",r(rd))}else{format!("ADDI {}, {imm}",r(rd))}},
        0x7=>{let m=match c{0=>"LB",1=>"LBU",2=>"LH",3=>"LHU",4=>"LW",5=>"SB",6=>"SH",7=>"SW",12=>"LW",13=>"SW",14=>"LW",15=>"SW",_=>"?"};let addr=match c{12|13=>format!("[{}]+",r(b)),14|15=>format!("-[{}]",r(b)),_=>format!("[{}]",r(b))};format!("{m} {}, {addr}",r(a))},
        0x8=>{let m=match c{0=>"LDP",1=>"LDP",2=>"STP",3=>"STP",4=>"STP",5=>"LD4",6=>"LD4",7=>"ST4",8=>"ST4",9=>"ST4",_=>"?"};let addr=match c{1|3|6|8=>format!("[{}]+",r(b)),4|9=>format!("-[{}]",r(b)),_=>format!("[{}]",r(b))};format!("{m} {}, {addr}",r(a))},
        0x9=>{let d=sx(word&0xff,8);let base=pc.wrapping_add(4)&!3;let t=base.wrapping_add((d as u32).wrapping_mul(4));format!("LDPC.W {}, 0x{t:08x}",r(a))},
        0xa=>{let rs=(word>>7)&0xf;let d=sx(word&0x7f,7);let t=target(pc,d);if word&0x0800==0{format!("BNZ {}, 0x{t:08x}",r(rs))}else{format!("DBNZ {}, 0x{t:08x}",r(rs))}},
        0xb=>{let d=sx(word&0x7ff,11);let t=target(pc,d);if word&0x0800==0{format!("B 0x{t:08x}")}else{format!("BL 0x{t:08x}")}},
        0xc=>{
            if c==0xf{let imm=((word>>4)&0xff) as u8;match imm{0xfe=>"BREAK".into(),0xff=>"NOP".into(),_=>format!("TRAP 0x{imm:02x}")}}
            else{match c{
                0=>match(a,b){(0,14)=>"RET".into(),(0,_)=>format!("JR {}",r(b)),(14,_)=>format!("CALLR {}",r(b)),_=>format!("JALR {}, {}",r(a),r(b))},
                1=>format!("BSET {}, {}",r(a),r(b)),2=>format!("BCLR {}, {}",r(a),r(b)),3=>format!("BINV {}, {}",r(a),r(b)),4=>format!("BEXT {}, {}",r(a),r(b)),
                5=>format!("MUL {}, {}",r(a),r(b)),6=>format!("MULH {}, {}",r(a),r(b)),7=>format!("MULHU {}, {}",r(a),r(b)),8=>format!("MULHSU {}, {}",r(a),r(b)),9=>format!("MULO {}, {}",r(a),r(b)),
                10=>format!("DIV {}, {}",r(a),r(b)),11=>format!("DIVU {}, {}",r(a),r(b)),12=>format!("REM {}, {}",r(a),r(b)),13=>format!("REMU {}, {}",r(a),r(b)),14=>format!("REV8 {}, {}",r(a),r(b)),
                _=>format!(".hword 0x{word:04x}")}}
        },
        0xd=>format!("ADC {}, {}, {}",r(a),r(b),r(c)),
        0xe=>format!("SBB {}, {}, {}",r(a),r(b),r(c)),
        0xf=>match a{
            0=>format!("SREAD {}, {}",r(b),sysreg(c)),
            1=>format!("SWRITE {}, {}",sysreg(c),r(b)),
            2=>if c==4{format!("SSWAP {}, SCRATCH",r(b))}else{format!(".hword 0x{word:04x}")},
            3=>match (b,c){(0,0)=>"SRET".into(),(_,1)=>format!("SRETCTX {}",r(b)),_=>format!(".hword 0x{word:04x}")},
            4=>match (b,c){(0,0)=>"TLBFENCE".into(),(_,1)=>format!("TLBFENCE.VA {}",r(b)),(_,2)=>format!("TLBFENCE.ASID {}",r(b)),_=>format!(".hword 0x{word:04x}")},
            5=>if b==0&&c==0{"WFI".into()}else{format!(".hword 0x{word:04x}")},
            6=>if b==0&&c==0{"SYNC.I".into()}else{format!(".hword 0x{word:04x}")},
            7=>if b==0&&c==0{"FENCE".into()}else{format!(".hword 0x{word:04x}")},
            _=>format!(".hword 0x{word:04x}"),
        },
        _=>unreachable!(),
    };
    DecodedInstruction{pc,word,text}
}

pub fn disassemble_word(word:u16,pc:u32)->String{decode_word(word,pc).text}

pub fn disassemble_bytes(bytes:&[u8],base:u32)->String{
    let mut out=String::new();
    let mut off=0usize;
    while off+1<bytes.len(){
        let word=u16::from_le_bytes([bytes[off],bytes[off+1]]);
        let pc=base.wrapping_add(off as u32);
        let d=decode_word(word,pc);
        let _=writeln!(&mut out,"0x{pc:08x}: {word:04x}  {}",d.text);
        off+=2;
    }
    if off<bytes.len(){let _=writeln!(&mut out,"0x{:08x}: {:02x}    .byte 0x{:02x}",base.wrapping_add(off as u32),bytes[off],bytes[off]);}
    out
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn decodes_callr(){assert_eq!(disassemble_word(crate::callr(12),0x1000),"CALLR r12");}
    #[test] fn decodes_ldpc_target(){let w=crate::ldpc_w(12,2);assert_eq!(disassemble_word(w,0x1000),"LDPC.W r12, 0x0000100c");}
    #[test] fn decodes_system(){assert_eq!(disassemble_word(crate::swrite(3,crate::SYSREG_VMCTX),0),"SWRITE VMCTX, r3");}
}
