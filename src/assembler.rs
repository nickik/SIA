use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    pub bytes: Vec<u8>,
    pub symbols: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembleError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for AssembleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 { write!(f, "{}", self.message) }
        else { write!(f, "line {}: {}", self.line, self.message) }
    }
}

impl std::error::Error for AssembleError {}
type AResult<T> = Result<T, AssembleError>;

/// Assemble the executable SIA32-I reference syntax into a raw little-endian image.
/// Only instructions with assigned encodings in `isa.rs` are accepted.
pub fn assemble(source: &str) -> AResult<AssembledProgram> {
    let mut symbols = BTreeMap::new();
    let mut pc = 0u32;

    // Pass 1: collect symbols and determine layout.
    for (idx, raw) in source.lines().enumerate() {
        let line_no = idx + 1;
        let mut line = strip_comment(raw).trim();
        if line.is_empty() { continue; }
        loop {
            let Some((label, rest)) = split_leading_label(line) else { break; };
            validate_label(label).map_err(|m| err(line_no, m))?;
            if symbols.insert(label.to_string(), pc).is_some() {
                return Err(err(line_no, format!("duplicate label `{label}`")));
            }
            line = rest.trim();
            if line.is_empty() { break; }
        }
        if line.is_empty() { continue; }
        pc = pc.checked_add(statement_size(line, pc, line_no)?)
            .ok_or_else(|| err(line_no, "assembly image exceeds 32-bit address space"))?;
    }

    // Pass 2: encode instructions and data using the completed symbol table.
    let mut bytes = Vec::with_capacity(pc as usize);
    pc = 0;
    for (idx, raw) in source.lines().enumerate() {
        let line_no = idx + 1;
        let mut line = strip_comment(raw).trim();
        if line.is_empty() { continue; }
        while let Some((_label, rest)) = split_leading_label(line) {
            line = rest.trim();
            if line.is_empty() { break; }
        }
        if line.is_empty() { continue; }
        emit_statement(line, pc, line_no, &symbols, &mut bytes)?;
        pc = bytes.len() as u32;
    }

    Ok(AssembledProgram { bytes, symbols })
}

fn err(line: usize, message: impl Into<String>) -> AssembleError {
    AssembleError { line, message: message.into() }
}

fn strip_comment(line: &str) -> &str {
    let semi = line.find(';');
    let slash = line.find("//");
    match (semi, slash) {
        (Some(a), Some(b)) => &line[..a.min(b)],
        (Some(a), None) => &line[..a],
        (None, Some(b)) => &line[..b],
        (None, None) => line,
    }
}

fn split_leading_label(line: &str) -> Option<(&str, &str)> {
    let colon = line.find(':')?;
    let before = &line[..colon];
    if before.is_empty() || before.chars().any(|c| c.is_whitespace() || matches!(c, ',' | '[' | ']')) {
        return None;
    }
    Some((before, &line[colon + 1..]))
}

fn validate_label(label: &str) -> Result<(), String> {
    let mut chars = label.chars();
    let Some(first) = chars.next() else { return Err("empty label".into()); };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '.') {
        return Err(format!("invalid label `{label}`"));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.')) {
        return Err(format!("invalid label `{label}`"));
    }
    Ok(())
}

fn statement_size(line: &str, pc: u32, line_no: usize) -> AResult<u32> {
    if !line.starts_with('.') {
        if pc & 1 != 0 {
            return Err(err(line_no, format!("instruction at odd address 0x{pc:08x}; use `.align 2`")));
        }
        return Ok(2);
    }
    let (name, args) = split_head(line);
    match name.to_ascii_lowercase().as_str() {
        ".byte" => Ok(split_operands(args).len() as u32),
        ".half" | ".hword" => Ok((split_operands(args).len() * 2) as u32),
        ".word" => Ok((split_operands(args).len() * 4) as u32),
        ".zero" => parse_u32_literal(args.trim()).map_err(|m| err(line_no, m)),
        ".align" => {
            let align = parse_alignment(args.trim(), line_no)?;
            Ok(padding(pc, align))
        }
        _ => Err(err(line_no, format!("unknown directive `{name}`"))),
    }
}

fn emit_statement(line: &str, pc: u32, line_no: usize, symbols: &BTreeMap<String, u32>, out: &mut Vec<u8>) -> AResult<()> {
    if line.starts_with('.') { emit_directive(line, pc, line_no, symbols, out) }
    else {
        out.extend_from_slice(&encode_instruction(line, pc, line_no, symbols)?.to_le_bytes());
        Ok(())
    }
}

fn emit_directive(line: &str, pc: u32, line_no: usize, symbols: &BTreeMap<String, u32>, out: &mut Vec<u8>) -> AResult<()> {
    let (name, args) = split_head(line);
    match name.to_ascii_lowercase().as_str() {
        ".byte" => for op in require_operands(args, line_no)? {
            let v = eval_expr(op, symbols).map_err(|m| err(line_no, m))?;
            if !(-128..=255).contains(&v) { return Err(err(line_no, format!("byte value out of range: {v}"))); }
            out.push(v as u8);
        },
        ".half" | ".hword" => for op in require_operands(args, line_no)? {
            let v = eval_expr(op, symbols).map_err(|m| err(line_no, m))?;
            if !(-32768..=65535).contains(&v) { return Err(err(line_no, format!("halfword value out of range: {v}"))); }
            out.extend_from_slice(&(v as u16).to_le_bytes());
        },
        ".word" => for op in require_operands(args, line_no)? {
            let v = eval_expr(op, symbols).map_err(|m| err(line_no, m))?;
            if !(-2147483648..=0xffff_ffff).contains(&v) { return Err(err(line_no, format!("word value out of range: {v}"))); }
            out.extend_from_slice(&(v as u32).to_le_bytes());
        },
        ".zero" => {
            let n = parse_u32_literal(args.trim()).map_err(|m| err(line_no, m))?;
            out.resize(out.len() + n as usize, 0);
        }
        ".align" => {
            let align = parse_alignment(args.trim(), line_no)?;
            out.resize(out.len() + padding(pc, align) as usize, 0);
        }
        _ => return Err(err(line_no, format!("unknown directive `{name}`"))),
    }
    Ok(())
}

fn parse_alignment(text: &str, line_no: usize) -> AResult<u32> {
    let align = parse_u32_literal(text).map_err(|m| err(line_no, m))?;
    if align == 0 || !align.is_power_of_two() {
        return Err(err(line_no, "`.align` requires a non-zero power-of-two byte alignment"));
    }
    Ok(align)
}

fn padding(pc: u32, align: u32) -> u32 { (align - (pc & (align - 1))) & (align - 1) }

fn split_head(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(i) => (&line[..i], line[i..].trim()),
        None => (line, ""),
    }
}

fn split_operands(args: &str) -> Vec<&str> {
    if args.trim().is_empty() { return Vec::new(); }
    args.split(',').map(str::trim).filter(|s| !s.is_empty()).collect()
}

fn require_operands(args: &str, line_no: usize) -> AResult<Vec<&str>> {
    let ops = split_operands(args);
    if ops.is_empty() { Err(err(line_no, "directive requires at least one operand")) } else { Ok(ops) }
}

fn encode_instruction(line: &str, pc: u32, line_no: usize, symbols: &BTreeMap<String, u32>) -> AResult<u16> {
    let (mnemonic_raw, args) = split_head(line);
    let mnemonic = mnemonic_raw.to_ascii_uppercase();
    let ops = split_operands(args);

    macro_rules! preg { ($idx:expr) => { parse_reg(ops[$idx]).map_err(|m| err(line_no, m))? }; }
    macro_rules! two_reg {
        ($fun:ident) => {{
            expect_n(&ops, 2, line_no, &mnemonic)?;
            super::$fun(preg!(0), preg!(1))
        }};
    }

    let word = match mnemonic.as_str() {
        "ADD" => {
            expect_n(&ops, 3, line_no, &mnemonic)?;
            let rd = preg!(0);
            if rd == 0 { return Err(err(line_no, "`ADD r0,...` aliases CLZ in the reference encoding; use an explicit destination")); }
            super::add(rd, preg!(1), preg!(2))
        }
        "MOV" => { expect_n(&ops, 2, line_no, &mnemonic)?; super::mov(preg!(0), preg!(1)) }
        "CLZ" => two_reg!(clz),
        "CMOV" => {
            expect_n(&ops, 3, line_no, &mnemonic)?;
            let rd = preg!(0);
            if rd == 0 { return Err(err(line_no, "`CMOV r0,...` aliases CTZ in the reference encoding")); }
            super::cmov(rd, preg!(1), preg!(2))
        }
        "CTZ" => two_reg!(ctz),
        "CPOP" => two_reg!(cpop),
        "LDA.W" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let rd = preg!(0);
            if rd == 0 { return Err(err(line_no, "`LDA.W r0,...` aliases CPOP in the reference encoding")); }
            let (rb, ri) = parse_scaled_address(ops[1]).map_err(|m| err(line_no, m))?;
            super::lda_w(rd, rb, ri)
        }
        "STA.W" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let (rb, ri) = parse_scaled_address(ops[1]).map_err(|m| err(line_no, m))?;
            super::sta_w(preg!(0), rb, ri)
        }

        "SUB" => two_reg!(sub), "ADDO" => two_reg!(addo), "SUBO" => two_reg!(subo),
        "CMPEQ" => two_reg!(cmpeq), "CMPLT" => two_reg!(cmplt), "CMPLTU" => two_reg!(cmpltu),
        "MIN" => two_reg!(min), "MINU" => two_reg!(minu), "MAX" => two_reg!(max), "MAXU" => two_reg!(maxu),
        "AND" => two_reg!(and), "OR" => two_reg!(or), "XOR" => two_reg!(xor),
        "SHL" => two_reg!(shl), "SHR" => two_reg!(shr), "SAR" => two_reg!(sar),

        "SHLI" | "SHRI" | "SARI" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let n = eval_expr(ops[1], symbols).map_err(|m| err(line_no, m))?;
            if !(0..=31).contains(&n) { return Err(err(line_no, format!("{mnemonic} shift amount must be 0..31"))); }
            match mnemonic.as_str() {
                "SHLI" => super::shli(preg!(0), n as u8),
                "SHRI" => super::shri(preg!(0), n as u8),
                "SARI" => super::sari(preg!(0), n as u8),
                _ => unreachable!(),
            }
        }
        "LI" | "ADDI" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let imm = eval_expr(ops[1], symbols).map_err(|m| err(line_no, m))?;
            if !(-64..=63).contains(&imm) { return Err(err(line_no, format!("{mnemonic} immediate must be -64..63"))); }
            if mnemonic == "LI" { super::li(preg!(0), imm as i8) } else { super::addi(preg!(0), imm as i8) }
        }

        "LB" | "LBU" | "LH" | "LHU" | "LW" | "SB" | "SH" | "SW" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let rv = preg!(0);
            let (rb, update) = parse_simple_address(ops[1]).map_err(|m| err(line_no, m))?;
            match (mnemonic.as_str(), update) {
                ("LB", AddrUpdate::None) => super::lb(rv, rb),
                ("LBU", AddrUpdate::None) => super::lbu(rv, rb),
                ("LH", AddrUpdate::None) => super::lh(rv, rb),
                ("LHU", AddrUpdate::None) => super::lhu(rv, rb),
                ("LW", AddrUpdate::None) => super::lw(rv, rb),
                ("SB", AddrUpdate::None) => super::sb(rv, rb),
                ("SH", AddrUpdate::None) => super::sh(rv, rb),
                ("SW", AddrUpdate::None) => super::sw(rv, rb),
                ("LW", AddrUpdate::Post) => super::lw_post(rv, rb),
                ("SW", AddrUpdate::Post) => super::sw_post(rv, rb),
                ("LW", AddrUpdate::Pre) => super::lw_pre(rv, rb),
                ("SW", AddrUpdate::Pre) => super::sw_pre(rv, rb),
                (_, AddrUpdate::Post | AddrUpdate::Pre) => {
                    return Err(err(line_no, format!("{mnemonic} does not have an update-addressing form")));
                }
                _ => unreachable!("scalar mnemonic pre-filtered by outer match"),
            }
        }

        "LDP" | "STP" | "LD4" | "ST4" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let count = if mnemonic.ends_with('4') { 4 } else { 2 };
            let first = parse_group(ops[0], count).map_err(|m| err(line_no, m))?;
            let (rb, update) = parse_simple_address(ops[1]).map_err(|m| err(line_no, m))?;
            match (mnemonic.as_str(), update) {
                ("LDP", AddrUpdate::None) => super::ldp(first, rb),
                ("LDP", AddrUpdate::Post) => super::ldp_post(first, rb),
                ("STP", AddrUpdate::None) => super::stp(first, rb),
                ("STP", AddrUpdate::Post) => super::stp_post(first, rb),
                ("STP", AddrUpdate::Pre) => super::stp_pre(first, rb),
                ("LD4", AddrUpdate::None) => super::ld4(first, rb),
                ("LD4", AddrUpdate::Post) => super::ld4_post(first, rb),
                ("ST4", AddrUpdate::None) => super::st4(first, rb),
                ("ST4", AddrUpdate::Post) => super::st4_post(first, rb),
                ("ST4", AddrUpdate::Pre) => super::st4_pre(first, rb),
                ("LDP" | "LD4", AddrUpdate::Pre) => return Err(err(line_no, format!("{mnemonic} has no pre-decrement encoding"))),
                _ => unreachable!("multi-transfer mnemonic pre-filtered by outer match"),
            }
        }

        "LDPC.W" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let target = eval_expr(ops[1], symbols).map_err(|m| err(line_no, m))?;
            let base = ((pc as i64) + 4) & !3;
            let delta = target - base;
            if delta % 4 != 0 { return Err(err(line_no, "LDPC.W target must be word-aligned relative to aligned PC+4 base")); }
            let disp = delta / 4;
            if !(-128..=127).contains(&disp) { return Err(err(line_no, format!("LDPC.W target out of range ({disp} words)"))); }
            super::ldpc_w(preg!(0), disp as i16)
        }
        "BNZ" | "DBNZ" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let disp = branch_disp(pc, ops[1], symbols, -64, 63).map_err(|m| err(line_no, m))?;
            if mnemonic == "BNZ" { super::bnz(preg!(0), disp as i16) } else { super::dbnz(preg!(0), disp as i16) }
        }
        "B" | "BL" => {
            expect_n(&ops, 1, line_no, &mnemonic)?;
            let disp = branch_disp(pc, ops[0], symbols, -1024, 1023).map_err(|m| err(line_no, m))?;
            if mnemonic == "B" { super::b(disp as i16) } else { super::bl(disp as i16) }
        }

        "JALR" => { expect_n(&ops, 2, line_no, &mnemonic)?; super::jalr(preg!(0), preg!(1)) }
        "JR" => { expect_n(&ops, 1, line_no, &mnemonic)?; super::jr(preg!(0)) }
        "CALLR" => { expect_n(&ops, 1, line_no, &mnemonic)?; super::callr(preg!(0)) }
        "RET" => { expect_n(&ops, 0, line_no, &mnemonic)?; super::ret() }
        "BSET" => two_reg!(bset), "BCLR" => two_reg!(bclr), "BINV" => two_reg!(binv), "BEXT" => two_reg!(bext),
        "REV8" => two_reg!(rev8),
        "MUL" => two_reg!(mul), "MULH" => two_reg!(mulh), "MULHU" => two_reg!(mulhu), "MULHSU" => two_reg!(mulhsu), "MULO" => two_reg!(mulo),
        "DIV" => two_reg!(div), "DIVU" => two_reg!(divu), "REM" => two_reg!(rem), "REMU" => two_reg!(remu),
        "ADC" | "SBB" => {
            expect_n(&ops, 3, line_no, &mnemonic)?;
            let rd = preg!(0); let rs = preg!(1); let rc = preg!(2);
            if rd == rc { return Err(err(line_no, format!("{mnemonic} requires rd != rc because both are outputs"))); }
            if mnemonic == "ADC" { super::adc(rd, rs, rc) } else { super::sbb(rd, rs, rc) }
        }
        "SREAD" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let sr = parse_sysreg(ops[1]).map_err(|m| err(line_no, m))?;
            super::sread(preg!(0), sr)
        }
        "SWRITE" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let sr = parse_sysreg(ops[0]).map_err(|m| err(line_no, m))?;
            super::swrite(preg!(1), sr)
        }
        "SSWAP" => {
            expect_n(&ops, 2, line_no, &mnemonic)?;
            let sr = parse_sysreg(ops[1]).map_err(|m| err(line_no, m))?;
            if sr != super::SYSREG_SCRATCH { return Err(err(line_no, "SSWAP only permits SCRATCH")); }
            super::sswap_scratch(preg!(0))
        }
        "SRET" => { expect_n(&ops, 0, line_no, &mnemonic)?; super::sret() }
        "SRETCTX" => { expect_n(&ops, 1, line_no, &mnemonic)?; super::sretctx(preg!(0)) }
        "TLBFENCE" => { expect_n(&ops, 0, line_no, &mnemonic)?; super::tlbfence() }
        "TLBFENCE.VA" => { expect_n(&ops, 1, line_no, &mnemonic)?; super::tlbfence_va(preg!(0)) }
        "TLBFENCE.ASID" => { expect_n(&ops, 1, line_no, &mnemonic)?; super::tlbfence_asid(preg!(0)) }
        "WFI" => { expect_n(&ops, 0, line_no, &mnemonic)?; super::wfi() }
        "SYNC.I" => { expect_n(&ops, 0, line_no, &mnemonic)?; super::sync_i() }
        "FENCE" => { expect_n(&ops, 0, line_no, &mnemonic)?; super::fence() }

        "TRAP" => {
            expect_n(&ops, 1, line_no, &mnemonic)?;
            let imm = eval_expr(ops[0], symbols).map_err(|m| err(line_no, m))?;
            if !(0..=253).contains(&imm) { return Err(err(line_no, "TRAP immediate 254/255 is reserved for BREAK/NOP in the reference encoding")); }
            super::trap(imm as u8)
        }
        "BREAK" => { expect_n(&ops, 0, line_no, &mnemonic)?; super::BREAK }
        "NOP" => { expect_n(&ops, 0, line_no, &mnemonic)?; super::NOP }
        "BZ" => return Err(err(line_no, "BZ is not assigned in the current SIA32-I reference encoding; compare/invert + BNZ instead")),
        _ => return Err(err(line_no, format!("unknown or unassigned instruction `{mnemonic_raw}`"))),
    };
    Ok(word)
}

fn expect_n(ops: &[&str], n: usize, line_no: usize, mnemonic: &str) -> AResult<()> {
    if ops.len() == n { Ok(()) }
    else { Err(err(line_no, format!("{mnemonic} expects {n} operand(s), got {}", ops.len()))) }
}

fn branch_disp(pc: u32, expr: &str, symbols: &BTreeMap<String, u32>, min: i64, max: i64) -> Result<i64, String> {
    let target = eval_expr(expr, symbols)?;
    let delta = target - (pc as i64 + 2);
    if delta % 2 != 0 { return Err("branch target must be 2-byte aligned".into()); }
    let disp = delta / 2;
    if disp < min || disp > max { return Err(format!("branch target out of range ({disp} halfwords; valid {min}..{max})")); }
    Ok(disp)
}

fn parse_reg(text: &str) -> Result<u8, String> {
    let s = text.trim().to_ascii_lowercase();
    match s.as_str() { "sp" => return Ok(13), "lr" => return Ok(14), _ => {} }
    let Some(rest) = s.strip_prefix('r') else { return Err(format!("expected register, got `{text}`")); };
    let n: u8 = rest.parse().map_err(|_| format!("invalid register `{text}`"))?;
    if n > 15 { return Err(format!("register out of range `{text}`")); }
    Ok(n)
}

fn parse_group(text: &str, count: usize) -> Result<u8, String> {
    if let Some((a, b)) = text.split_once(':') {
        let first = parse_reg(a)?; let last = parse_reg(b)?;
        let expected = first as usize + count - 1;
        if expected > 15 || last as usize != expected {
            return Err(format!("register group `{text}` must contain exactly {count} consecutive registers"));
        }
        Ok(first)
    } else {
        let first = parse_reg(text)?;
        if first as usize + count > 16 { return Err(format!("register group starting at `{text}` wraps beyond r15")); }
        Ok(first)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddrUpdate { None, Post, Pre }

fn parse_simple_address(text: &str) -> Result<(u8, AddrUpdate), String> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let (body, update) = if let Some(body) = compact.strip_prefix("-[").and_then(|s| s.strip_suffix(']')) {
        (body, AddrUpdate::Pre)
    } else if let Some(body) = compact.strip_prefix('[').and_then(|s| s.strip_suffix("]+")) {
        (body, AddrUpdate::Post)
    } else if let Some(body) = compact.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        (body, AddrUpdate::None)
    } else {
        return Err(format!("expected address `[rb]`, `[rb]+`, or `-[rb]`, got `{text}`"));
    };
    Ok((parse_reg(body)?, update))
}

fn parse_scaled_address(text: &str) -> Result<(u8, u8), String> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let inner = compact.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| format!("expected scaled address `[rb+ri*4]`, got `{text}`"))?;
    let (base, index_scale) = inner.split_once('+')
        .ok_or_else(|| format!("expected scaled address `[rb+ri*4]`, got `{text}`"))?;
    let (index, scale) = index_scale.split_once('*')
        .ok_or_else(|| format!("expected scaled address `[rb+ri*4]`, got `{text}`"))?;
    if scale != "4" { return Err(format!("scaled SIA word address requires *4, got `*{scale}`")); }
    Ok((parse_reg(base)?, parse_reg(index)?))
}

fn eval_expr(text: &str, symbols: &BTreeMap<String, u32>) -> Result<i64, String> {
    let compact: String = text.trim().chars().filter(|c| !c.is_whitespace()).collect();
    let s = compact.strip_prefix('#').unwrap_or(&compact);
    if s.is_empty() { return Err("empty expression".into()); }
    if let Ok(v) = parse_i64_literal(s) { return Ok(v); }
    for (i, ch) in s.char_indices().skip(1) {
        if ch == '+' || ch == '-' {
            let base = symbol_or_number(&s[..i], symbols)?;
            let offset = parse_i64_literal(&s[i + 1..])?;
            return Ok(if ch == '+' { base + offset } else { base - offset });
        }
    }
    symbol_or_number(s, symbols)
}

fn symbol_or_number(text: &str, symbols: &BTreeMap<String, u32>) -> Result<i64, String> {
    if let Ok(v) = parse_i64_literal(text) { return Ok(v); }
    symbols.get(text).map(|&v| v as i64).ok_or_else(|| format!("unknown symbol or number `{text}`"))
}

fn parse_u32_literal(text: &str) -> Result<u32, String> {
    let v = parse_i64_literal(text)?;
    if !(0..=u32::MAX as i64).contains(&v) { return Err(format!("value out of u32 range: {v}")); }
    Ok(v as u32)
}

fn parse_i64_literal(text: &str) -> Result<i64, String> {
    let s: String = text.trim().chars().filter(|c| *c != '_').collect();
    let (neg, body) = if let Some(v) = s.strip_prefix('-') { (true, v) }
        else if let Some(v) = s.strip_prefix('+') { (false, v) }
        else { (false, s.as_str()) };
    let (radix, digits) = if let Some(v) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) { (16, v) }
        else if let Some(v) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) { (2, v) }
        else if let Some(v) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) { (8, v) }
        else { (10, body) };
    if digits.is_empty() { return Err(format!("invalid number `{text}`")); }
    let mag = i64::from_str_radix(digits, radix).map_err(|_| format!("invalid number `{text}`"))?;
    Ok(if neg { -mag } else { mag })
}

fn parse_sysreg(text: &str) -> Result<u8, String> {
    match text.trim().to_ascii_uppercase().as_str() {
        "STATUS" => Ok(super::SYSREG_STATUS),
        "EPC" => Ok(super::SYSREG_EPC),
        "CAUSE" => Ok(super::SYSREG_CAUSE),
        "BADADDR" => Ok(super::SYSREG_BADADDR),
        "SCRATCH" => Ok(super::SYSREG_SCRATCH),
        "VMCTX" => Ok(super::SYSREG_VMCTX),
        other => {
            let value = parse_i64_literal(other)?;
            if !(0..=15).contains(&value) { return Err(format!("system register selector out of range: {value}")); }
            Ok(value as u8)
        }
    }
}
