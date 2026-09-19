use sia32_tools::{assemble, disassemble_word, callr, ldpc_w, swrite, SYSREG_VMCTX};

#[test]
fn canonical_words_disassemble_readably() {
    assert_eq!(disassemble_word(callr(12), 0x1000), "CALLR r12");
    assert_eq!(disassemble_word(ldpc_w(12, 2), 0x1000), "LDPC.W r12, 0x0000100c");
    assert_eq!(disassemble_word(swrite(3, SYSREG_VMCTX), 0x1000), "SWRITE VMCTX, r3");
}

#[test]
fn assembler_and_disassembler_agree_on_privileged_syntax() {
    let p = assemble("LI r1, 1\nSWRITE VMCTX, r1\nTLBFENCE\nCALLR r12\n").unwrap();
    assert_eq!(p.bytes.len(), 8);
    let words = p.bytes.chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    assert_eq!(disassemble_word(words[1], 2), "SWRITE VMCTX, r1");
    assert_eq!(disassemble_word(words[2], 4), "TLBFENCE");
    assert_eq!(disassemble_word(words[3], 6), "CALLR r12");
}
