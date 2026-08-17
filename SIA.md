# Scalable Instruction Architecture (SIA)

## Integer Architecture — Draft v0.2

SIA is a compact, scalable 32-bit RISC instruction set architecture built around **mixed mandatory 16-bit and 32-bit instructions**.

The first architecture defined here is the integer-only `SIA32-I` base. It has 16 architectural integer registers, no condition-code register, no branch delay slots, and no floating-point or vector state.

SIA has two equally architectural instruction lengths:

- **short** — 16-bit instructions for common operations and maximum code density;
- **wide** — 32-bit instructions for richer operand forms, larger immediates, generalized address generation, and operations that cannot be encoded cleanly in 16 bits.

Both lengths are mandatory. There is no compressed mode, wide mode, compatibility mode, or ISA-mode switch. A conforming `SIA32-I` processor executes arbitrary interleaved 16-bit and 32-bit instructions.

The design objective is an extended 16-bit RISC: the common path should have SuperH-like density, while the architecture should not distort itself merely to force every useful operation into 16 bits.

Numeric opcode assignments in this draft remain provisional until assembler, compiler, and code-density experiments are available. Instruction semantics and the short/wide division are intended to be much more stable.

---

# 1. Design goals

`SIA32-I` targets the following properties:

- 32-bit integer and address model.
- Exactly 16 architectural integer registers.
- Four-bit register identifiers everywhere.
- Native 16-bit instructions for common operations.
- Native 32-bit instructions as a mandatory part of the same base ISA.
- The first halfword identifies instruction length without mode state.
- No condition-code or arithmetic flags register.
- Compare instructions produce integer masks.
- Conditional move and full conditional select are architectural operations.
- Common pointer and stack walks have update addressing.
- Register-indexed and scaled-array memory operations are architectural.
- Bitfield extract and insert are architectural.
- High-value bit manipulation is architectural rather than left to large optional extensions.
- No branch delay slots.
- PC-relative address and literal formation are first-class operations.
- Precise exceptions and a simple compiler-visible execution model take priority over clever hidden state.
- Future architecture growth must preserve all old encodings.

SIA is deliberately not a 32-bit ISA with a compressed side ISA. The 16-bit and 32-bit forms are parts of one instruction set.

---

# 2. Architectural state

## 2.1 Integer width

`XLEN = 32` in `SIA32-I`.

Integer arithmetic is performed modulo 2^32 unless an instruction explicitly specifies checked arithmetic.

Addresses are 32 bits and memory is byte addressed.

## 2.2 Integer registers

SIA defines 16 integer registers:

| Register | Architectural / ABI role |
|---|---|
| `r0` | constant zero |
| `r1-r12` | general-purpose integer registers |
| `r13` | stack pointer, `sp`, by ABI |
| `r14` | link register, `lr`, by ABI |
| `r15` | frame pointer, `fp`, by ABI; otherwise general-purpose |

Reads from `r0` return zero. Writes to `r0` are discarded except where an encoding explicitly reuses an otherwise useless destination-zero pattern as an escape.

Suggested initial ABI:

- `r1-r6`: arguments and return values
- `r7-r8`: caller-saved temporaries
- `r9-r12`: callee-saved
- `r13`: stack pointer
- `r14`: link register
- `r15`: frame pointer when required

The ABI is separate from core instruction semantics.

## 2.3 Program counter

The PC addresses bytes.

A short instruction advances the PC by 2 bytes.

A wide instruction advances the PC by 4 bytes.

Every valid instruction address is at least 2-byte aligned.

There are no architecturally visible branch delay slots.

---

# 3. Instruction length encoding

Instructions are fetched as 16-bit halfwords.

For the first halfword `H0`:

```text
H0[15] = 0    16-bit short instruction
H0[15] = 1    32-bit wide instruction; consume H0 and following H1
```

A 32-bit instruction may begin at any 2-byte-aligned address. It is not required to be 4-byte aligned.

In memory, `H0` is always the lower-addressed halfword. `H1` immediately follows it.

This makes instruction length locally decodable from the first halfword and allows short and wide instructions to be freely interleaved.

There is no instruction-set mode bit in the PC or status register.

---

# 4. Why SIA uses both 16 and 32 bits

After reserving the length bit, a short instruction has 15 remaining bits.

Three unrestricted registers require:

```text
4 + 4 + 4 = 12 bits
```

That leaves only three bits for identifying the operation.

Four unrestricted registers require all 16 bits before an opcode is encoded at all.

Therefore SIA uses 16-bit encodings only where they provide exceptional density value. Richer forms use the mandatory wide encoding rather than contorting the architecture around tiny fields, implicit registers, or hidden condition state.

The intended compiler policy is:

1. Prefer a short instruction whenever it expresses the desired operation directly.
2. Use a wide instruction when it avoids extra moves, extra address-generation operations, or destructive two-address constraints.
3. Optimize for both **bytes** and **instruction path length**, not code size alone.

---

# 5. Conditional model

SIA has no arithmetic flags register.

Scalar comparisons write a normal integer register.

True is represented as:

```text
0xFFFFFFFF
```

False is represented as:

```text
0x00000000
```

This gives comparison results useful semantics as both Boolean conditions and full-register masks.

Example:

```asm
CMPLT   r7, r1, r2
SEL     r3, r1, r2, r7
```

Conceptually:

```c
r7 = ((int32_t)r1 < (int32_t)r2) ? 0xFFFFFFFFu : 0;
r3 = (r7 != 0) ? r1 : r2;
```

The short ISA also contains destructive `CMOV` for cases where a full four-register select is unnecessary.

---

# 6. Byte order and alignment

The initial SIA machine is little-endian for instructions and data.

Natural data alignment is required:

- byte access: any byte address
- halfword access: address divisible by 2
- word access: address divisible by 4

A misaligned halfword or word access traps.

The base architecture does not include MIPS-style partial-word unaligned load/store instructions.

---

# 7. Short 16-bit instruction space

## 7.1 Primary map

```text
15   14:12                         0
+--+--------+-----------------------+
|0 | primary|        payload        |
+--+--------+-----------------------+
```

Initial assignments:

| `H0[14:12]` | Format / instruction |
|---|---|
| `000` | `ADD rd, ra, rb` |
| `001` | `CMOV rd, rs, rc` |
| `010` | `LDX.W rd, [rb + ri]` |
| `011` | `LDA.W rd, [rb + (ri << 2)]` |
| `100` | two-register ALU group |
| `101` | 7-bit immediate group |
| `110` | scalar memory group |
| `111` | control-flow / system group |

The first four groups are intentionally expensive unrestricted three-register formats.

## 7.2 Short three-register operations

```text
15  14:12  11:8  7:4  3:0
+--+------+-----+----+----+
|0 | op   | rd  | ra | rb |
+--+------+-----+----+----+
```

### `ADD rd, ra, rb`

```c
rd = ra + rb;
```

`MOV rd, rs` is encoded as:

```asm
ADD rd, r0, rs
```

### `CMOV rd, rs, rc`

```c
if (rc != 0)
    rd = rs;
```

If `rc == 0`, `rd` is unchanged.

### `LDX.W rd, [rb + ri]`

```c
rd = load_u32(rb + ri);
```

The index is an unscaled byte offset.

### `LDA.W rd, [rb + (ri << 2)]`

```c
rd = load_u32(rb + (ri << 2));
```

This is the short fast path for a 32-bit array load.

## 7.3 Destination-zero short escapes

The otherwise useless result-to-`r0` patterns of the four three-register groups are reclaimed.

| Parent slot with `rd=r0` | Escape instruction |
|---|---|
| `ADD` | `CLZ rd, rs` |
| `CMOV` | `CTZ rd, rs` |
| `LDX.W` | `CPOP rd, rs` |
| `LDA.W` | `NOT rd, rs` |

---

# 8. Short integer ALU group

```text
15 14:12 11:8 7:4 3:0
+--+-----+----+---+----+
|0 | 100 | rd |rs | fn |
+--+-----+----+---+----+
```

These are destructive two-address forms: the old value of `rd` is the first source.

| `fn` | Instruction | Semantics |
|---|---|---|
| `0` | `SUB rd, rs` | `rd = rd - rs` |
| `1` | `ADDC rd, rs` | checked signed add |
| `2` | `SUBC rd, rs` | checked signed subtract |
| `3` | `AND rd, rs` | `rd &= rs` |
| `4` | `OR rd, rs` | `rd |= rs` |
| `5` | `XOR rd, rs` | `rd ^= rs` |
| `6` | `SHL rd, rs` | logical left shift |
| `7` | `SHR rd, rs` | logical right shift |
| `8` | `SAR rd, rs` | arithmetic right shift |
| `9` | `CMPEQ rd, rs` | equality mask |
| `A` | `CMPLT rd, rs` | signed less-than mask |
| `B` | `CMPLTU rd, rs` | unsigned less-than mask |
| `C` | `MIN rd, rs` | signed minimum |
| `D` | `MINU rd, rs` | unsigned minimum |
| `E` | `MAX rd, rs` | signed maximum |
| `F` | `MAXU rd, rs` | unsigned maximum |

`C` in `ADDC` and `SUBC` means **checked**, not carry. These instructions trap on signed overflow.

There is no architectural carry flag.

Register shift counts use the low five bits of the shift-count register.

---

# 9. Short immediate group

```text
15 14:12 11 10:7 6:0
+--+-----+--+----+-------+
|0 | 101 |op| rd | imm7  |
+--+-----+--+----+-------+
```

`imm7` is signed in the range `-64..+63`.

| `op` | Instruction |
|---|---|
| `0` | `LI rd, imm7` |
| `1` | `ADDI rd, imm7` |

```c
LI:   rd = sign_extend_7(imm7);
ADDI: rd = rd + sign_extend_7(imm7);
```

---

# 10. Short scalar memory group

```text
15 14:12 11:8 7:4 3:0
+--+-----+----+----+------+
|0 | 110 | rv | rb | mode |
+--+-----+----+----+------+
```

| `mode` | Instruction |
|---|---|
| `0` | `LB rv, [rb]` |
| `1` | `LBU rv, [rb]` |
| `2` | `LH rv, [rb]` |
| `3` | `LHU rv, [rb]` |
| `4` | `LW rv, [rb]` |
| `5` | `SB rv, [rb]` |
| `6` | `SH rv, [rb]` |
| `7` | `SW rv, [rb]` |
| `8` | `LW rv, [rb + 4]` |
| `9` | `SW rv, [rb + 4]` |
| `A` | `LW rv, [rb + 8]` |
| `B` | `SW rv, [rb + 8]` |
| `C` | `LW rv, [rb]+` |
| `D` | `SW rv, [rb]+` |
| `E` | `LW rv, -[rb]` |
| `F` | `SW rv, -[rb]` |

The update forms are defined as:

```text
LW rv,[rb]+     load from rb, then rb += 4
SW rv,[rb]+     store to rb, then rb += 4
LW rv,-[rb]     rb -= 4, then load from rb
SW rv,-[rb]     rb -= 4, then store to rb
```

This deliberately provides both forward pointer walking and natural stack behavior:

```asm
SW r4, -[sp]     ; push
LW r4, [sp]+     ; pop
```

For an updating load, `rv == rb` is illegal because the load result and base update would target the same architectural register.

---

# 11. Short control flow

There are no delay slots.

## 11.1 Branch on nonzero

```text
15 14:11 10:7 6:0
+--+-----+----+-------+
|0 |1110 | rs | disp7 |
+--+-----+----+-------+
```

```asm
BNZ rs, target
```

```c
if (rs != 0)
    pc = next_pc + sign_extend_7(disp7) * 2;
```

`BT` may be accepted as an alias when the source is known to contain a comparison mask.

## 11.2 Direct branch and call

```text
15 14:11 10 9:0
+--+-----+--+----------+
|0 |1111 | L| disp10   |
+--+-----+--+----------+
```

`L=0` is `B target`.

`L=1` is `BL target`, which writes the address of the following instruction to `r14`.

## 11.3 Register-control escape

The impossible `BNZ r0,...` case is reclaimed for control/system instructions.

Initial assignments include:

```text
JR rs
JALR rs
TRAP imm4
BREAK
NOP
```

`RET` is an alias for `JR r14`.

---

# 12. Mandatory 32-bit wide encoding

Every `SIA32-I` implementation must implement the wide instruction format.

A wide instruction consists of `H0` and `H1`:

```text
H0:
15  14:8   7:4  3:0
+--+------+----+----+
|1 | xop  | A  | B  |
+--+------+----+----+

H1:
15                     0
+-----------------------+
|      operation data   |
+-----------------------+
```

`xop` is a seven-bit wide-operation code.

`A` and `B` are usually register fields but may become immediate bits for formats that do not need two registers.

The 16-bit `H1` payload is interpreted by the selected `xop`.

This gives SIA 128 wide operation families while retaining four-bit register identifiers.

Wide opcode allocation is grouped as follows:

| `xop` range | Use |
|---|---|
| `00-0F` | three-register arithmetic and logic |
| `10-1F` | compare, select, bitfield |
| `20-2F` | immediate and constant formation |
| `30-3F` | generalized memory and address generation |
| `40-4F` | control flow |
| `50-5F` | multiply/divide and integer helpers |
| `60-6F` | bit manipulation and byte operations |
| `70-7F` | reserved for future base-architecture growth |

Exact `xop` values inside these ranges are provisional.

---

# 13. Wide three-register arithmetic and logic

The wide three-register form removes the destructive destination constraint of the short ALU group.

Conceptual format:

```text
H0:  1 | xop | rd | ra
H1:  rb | reserved(12)
```

Required operations:

```text
SUB     rd, ra, rb
ADDC    rd, ra, rb
SUBC    rd, ra, rb

AND     rd, ra, rb
OR      rd, ra, rb
XOR     rd, ra, rb
ANDN    rd, ra, rb
ORN     rd, ra, rb
XNOR    rd, ra, rb

SHL     rd, ra, rb
SHR     rd, ra, rb
SAR     rd, ra, rb
ROL     rd, ra, rb
ROR     rd, ra, rb

MIN     rd, ra, rb
MINU    rd, ra, rb
MAX     rd, ra, rb
MAXU    rd, ra, rb
```

`ADD rd,ra,rb` already has a short unrestricted three-register encoding and therefore does not require a distinct wide encoding.

Boolean-not combinations are defined as:

```c
ANDN: rd = ra & ~rb;
ORN:  rd = ra | ~rb;
XNOR: rd = ~(ra ^ rb);
```

Rotates use the low five bits of `rb` as the count.

---

# 14. Features intentionally absent from the 16-bit form but mandatory in the 32-bit base

These are **not optional extensions**. They are required parts of `SIA32-I`; only their compact 16-bit encodings are absent.

## 14.1 Full immediate bitfields

```text
BFEXTU rd, rs, pos, len
BFEXTS rd, rs, pos, len
BFINS  rd, ra, rb, pos, len
```

`pos` is a five-bit bit position `0..31`.

`len` is encoded as `len-1`, permitting lengths `1..32`.

### `BFEXTU`

```c
rd = (rs >> pos) & mask(len);
```

### `BFEXTS`

```c
rd = sign_extend((rs >> pos) & mask(len), len);
```

### `BFINS`

```c
rd = (ra & ~(mask(len) << pos)) |
     ((rb & mask(len)) << pos);
```

An encoding whose requested field extends past bit 31 is illegal rather than wrapping.

These instructions provide shifts, masks, field extraction, register packing, device-register manipulation, and language-runtime tag manipulation without creating a large addressing-mode-like family of special cases.

## 14.2 Full four-register `SEL`

```text
SEL rd, rt, rf, rc
```

Semantics:

```c
rd = (rc != 0) ? rt : rf;
```

Conceptual encoding:

```text
H0: 1 | xop | rd | rt
H1: rf(4) | rc(4) | reserved(8)
```

The 16-bit `CMOV` remains useful because it can often perform the same job in half the bytes when one alternative is already in the destination.

## 14.3 Fully general scaled memory operations

Wide indexed memory instructions support:

- byte, halfword, and word accesses;
- signed and unsigned byte/halfword loads;
- loads and stores;
- index scales x1, x2, x4, x8;
- a signed 7-bit byte displacement;
- no update, pre-update, or post-update of the base register.

Syntax:

```text
LBX   rd, [rb + ri*scale + disp], update
LBUX  rd, [rb + ri*scale + disp], update
LHX   rd, [rb + ri*scale + disp], update
LHUX  rd, [rb + ri*scale + disp], update
LWX   rd, [rb + ri*scale + disp], update

SBX   rs, [rb + ri*scale + disp], update
SHX   rs, [rb + ri*scale + disp], update
SWX   rs, [rb + ri*scale + disp], update
```

Conceptual `H1` layout:

```text
15:12  11:10  9:7     6:0
+------+-----+-------+-------+
|  ri  |scale| update| disp7 |
+------+-----+-------+-------+
```

Scale encoding:

```text
00 = x1
01 = x2
10 = x4
11 = x8
```

Update encoding:

```text
000 = none
001 = post-increment base by access size
010 = post-decrement base by access size
011 = pre-increment base by access size
100 = pre-decrement base by access size
101-111 = reserved
```

`ri = r0` means no index.

The displacement is always a byte displacement; the scale applies only to the register index.

The effective address without pre-update is:

```c
EA = rb + (ri << scale) + sign_extend_7(disp7);
```

For a pre-update form, `rb` is first changed by the access size and the updated value participates in the effective-address calculation.

For a post-update form, the memory access uses the old base and the base is updated only after a successful access.

An updating load with `rd == rb` is illegal.

## 14.4 Additional Boolean operations

The following are mandatory wide operations:

```text
ANDN rd, ra, rb
ORN  rd, ra, rb
XNOR rd, ra, rb
```

They are useful in mask-heavy systems code and avoid an otherwise unnecessary `NOT` temporary.

## 14.5 Single-bit operations

The wide base includes register-indexed and immediate forms:

```text
BSET   rd, rs, rb
BCLR   rd, rs, rb
BINV   rd, rs, rb
BEXT   rd, rs, rb

BSETI  rd, rs, bit
BCLRI  rd, rs, bit
BINVI  rd, rs, bit
BEXTI  rd, rs, bit
```

Semantics:

```c
BSET: rd = rs |  (1u << bit);
BCLR: rd = rs & ~(1u << bit);
BINV: rd = rs ^  (1u << bit);
BEXT: rd = (rs >> bit) & 1u;
```

Register-indexed forms use the low five bits of the bit-index register.

---

# 15. Wide compare operations

Wide compares are non-destructive three-register operations:

```text
CMPEQ   rd, ra, rb
CMPNE   rd, ra, rb
CMPLT   rd, ra, rb
CMPLTU  rd, ra, rb
CMPLE   rd, ra, rb
CMPLEU  rd, ra, rb
```

Every result is either zero or `0xFFFFFFFF`.

Immediate forms are also required:

```text
CMPEQI  rd, ra, imm16
CMPLTI  rd, ra, imm16
CMPLTUI rd, ra, uimm16
```

The immediate signedness follows the comparison.

---

# 16. Wide immediate arithmetic and constants

Wide immediate ALU operations use a 16-bit immediate:

```text
ADDI rd, ra, imm16
ANDI rd, ra, imm16
ORI  rd, ra, imm16
XORI rd, ra, imm16
```

The arithmetic immediate is sign extended. Logical immediates are zero extended.

Required constant-building instructions:

```text
MOVI rd, simm20
LUI  rd, uimm20
```

`MOVI` sign extends a 20-bit constant.

`LUI` writes:

```c
rd = uimm20 << 12;
```

A general 32-bit constant can therefore be formed using `LUI` plus a low-part operation, while common small constants use short `LI`, wide `MOVI`, or a PC-relative literal load.

---

# 17. Address-generation instructions

## 17.1 `LEA`

The wide base includes a general integer address generator:

```text
LEA rd, [rb + ri*scale + disp10]
```

Semantics:

```c
rd = rb + (ri << scale) + sign_extend_10(disp10);
```

`ri = r0` removes the index.

Scale is x1/x2/x4/x8.

This is arithmetic only; it does not access memory.

The following are assembler aliases:

```text
SH1ADD rd, ra, rb    == LEA rd, [rb + ra*2]
SH2ADD rd, ra, rb    == LEA rd, [rb + ra*4]
SH3ADD rd, ra, rb    == LEA rd, [rb + ra*8]
```

This directly captures the useful shift-and-add idea without requiring three separate architectural datapaths.

## 17.2 `ADR`

```text
ADR rd, target
```

`ADR` forms a PC-relative address using a signed 20-bit halfword displacement:

```c
rd = next_pc + sign_extend_20(disp20) * 2;
```

This is the normal position-independent way to obtain nearby code or data addresses.

## 17.3 PC-relative literal load

```text
LDLIT.W rd, target
```

The effective address is relative to the next instruction, rounded down to a 4-byte boundary:

```c
base = next_pc & ~3u;
EA   = base + sign_extend_20(disp20) * 4;
rd   = load_u32(EA);
```

This gives assemblers an efficient single-instruction path for arbitrary 32-bit constants and addresses stored in nearby literal pools.

---

# 18. Wide control flow

## 18.1 Direct compare-and-branch

The wide base includes direct two-register branches:

```text
BEQ   ra, rb, target
BNE   ra, rb, target
BLT   ra, rb, target
BGE   ra, rb, target
BLTU  ra, rb, target
BGEU  ra, rb, target
```

These use a signed 16-bit halfword displacement relative to `next_pc`.

They exist because producing a mask and then branching is excellent when the mask is also consumed by dataflow, but wasteful when the comparison exists only for control flow.

## 18.2 Decrement and branch

```text
DBNZ rs, target
```

Semantics:

```c
rs = rs - 1;
if (rs != 0)
    pc = target;
```

The subtraction wraps modulo 2^32.

This is intended for compact counted loops and removes a dependency-visible `ADDI -1` plus `BNZ` pair.

## 18.3 Long direct branch and call

Wide direct branches use a signed 24-bit halfword displacement:

```text
B.W  target
BL.W target
```

`BL.W` writes `next_pc` to `r14`.

## 18.4 Register jump with displacement

```text
JALR rd, rb, imm16
```

Semantics:

```c
target = (rb + sign_extend_16(imm16)) & ~1u;
rd = next_pc;
pc = target;
```

Clearing bit zero is intentional. Since SIA instructions are at least halfword aligned, the low pointer bit can be used by software as metadata without requiring an explicit clear before an indirect call.

Aliases:

```text
JR    rb          == JALR r0, rb, 0
CALLR rb          == JALR r14, rb, 0
RET               == JALR r0, r14, 0
```

---

# 19. Multiply and divide

Ordinary integer multiplication and division are mandatory wide operations rather than optional extension families.

```text
MUL     rd, ra, rb
MULH    rd, ra, rb
MULHU   rd, ra, rb
MULHSU  rd, ra, rb
MULC    rd, ra, rb

DIV     rd, ra, rb
DIVU    rd, ra, rb
REM     rd, ra, rb
REMU    rd, ra, rb
```

`MUL` returns the low 32 bits of the product.

`MULH`, `MULHU`, and `MULHSU` return the high 32 bits of signed×signed, unsigned×unsigned, and signed×unsigned 64-bit products respectively.

`MULC` performs signed multiplication and traps if the mathematical result is not representable in signed 32 bits.

`DIV` and `DIVU` produce quotient only; `REM` and `REMU` produce remainder. There are no special HI/LO result registers.

Division by zero traps.

Signed division of `0x80000000 / -1` traps as signed overflow.

---

# 20. Additional required integer helpers

The following wide operations are part of the integer architecture:

```text
SEXT.B  rd, rs
SEXT.H  rd, rs
ZEXT.B  rd, rs
ZEXT.H  rd, rs
REV8    rd, rs
```

`REV8` reverses byte order within the 32-bit register:

```text
0x11223344 -> 0x44332211
```

`CLZ`, `CTZ`, `CPOP`, and `NOT` already have compact 16-bit encodings and therefore need no mandatory duplicate wide encoding.

---

# 21. SuperH ideas deliberately carried into SIA

SIA takes several density lessons from SuperH while avoiding its architectural condition bit and delay-slot model.

## 21.1 Native 16-bit common path

The first priority is that ordinary integer, memory, and branch sequences remain dense without entering a separate compressed ISA mode.

## 21.2 PC-relative literal pools

Large constants do not need to force large immediate fields into every instruction. `LDLIT.W` provides a defined PC-relative constant-pool path.

## 21.3 Generalized `MOVA` idea

SIA `ADR` is the general form of the PC-relative address-generation idea: it can target any integer register rather than a single implicit register.

## 21.4 Post-increment and pre-decrement

The short forms:

```text
LW rd,[rb]+
SW rs,-[rb]
```

make pointer walks, pushes, and pops compact.

Wide indexed memory generalizes update addressing where needed.

## 21.5 Decrement-and-test loop support

SuperH-style decrement/test density is represented by `DBNZ`, but SIA writes no hidden T bit.

## 21.6 What SIA does not copy

SIA does not adopt:

- a global T condition bit;
- branch delay slots;
- delayed-branch annul semantics;
- an architecturally special global-base register;
- multiply-accumulate or DSP-style instruction families in the integer base.

An ABI may dedicate a general register as a global pointer without making it special in the ISA.

---

# 22. Useful ideas incorporated from other RISC architectures

## 22.1 Alpha

Useful Alpha-like ideas for SIA are:

- condition-dependent data movement without a conventional flags register;
- a hard zero register;
- high-half multiply as a normal general-register result;
- explicit memory-ordering operations in the future memory-model specification;
- selected byte-manipulation operations as possible future additions if compiler data justifies them.

Alpha-style `ZAP`/`ZAPNOT`, which clear selected bytes under a byte mask, are interesting but are **not** in `SIA32-I` v0.2. They should be measured against general bitfields and `AND` masks first.

SIA also deliberately avoids Alpha's architecture-specific PAL mechanism in the integer ISA. Privileged software interfaces belong in a later privileged-architecture document.

## 22.2 RISC-V

SIA adopts or converges on several useful ideas also found in modern RISC-V integer work:

- `SH1ADD`/`SH2ADD`/`SH3ADD` semantics, represented through `LEA`;
- `ANDN`, `ORN`, and `XNOR`;
- `CLZ`, `CTZ`, population count, min/max, rotates, and byte reverse;
- single-bit set/clear/invert/extract operations;
- conditional dataflow without requiring a flags register;
- clearing bit zero on indirect jump targets.

SIA does **not** need a separate conditional-zero instruction family because short `CMOV` plus wide four-register `SEL` already provide direct conditional data movement.

## 22.3 MIPS

Useful MIPS lessons incorporated into SIA include:

- bitfield extract/insert;
- sign/zero-extension helpers;
- conditional move as a normal integer operation;
- upper-immediate constant construction;
- count-leading-zero and rotate helpers.

SIA deliberately does not adopt:

- HI/LO multiply/divide registers;
- branch delay slots;
- ISA-mode bits for switching between normal and compressed encodings;
- partial-word unaligned load/store pairs.

## 22.4 PA-RISC

PA-RISC provides particularly strong precedent for two SIA choices:

- shift-and-add integer addressing operations;
- first-class extract/deposit bitfield operations.

SIA also uses direct compare-and-branch instructions for cases where materializing a Boolean mask would only increase path length.

PA-RISC-style instruction nullification or 'skip the following instruction if condition' is interesting for code density but is **not** currently part of SIA. With mixed 16/32-bit instructions it would require the machine to determine and suppress the complete next instruction, complicating fetch, prediction, debugging, and precise-exception reasoning for a relatively narrow gain.

---

# 23. Architectural instruction summary

## 23.1 Short 16-bit core

```text
ADD     rd, ra, rb
CMOV    rd, rs, rc

SUB     rd, rs
ADDC    rd, rs
SUBC    rd, rs
AND     rd, rs
OR      rd, rs
XOR     rd, rs
SHL     rd, rs
SHR     rd, rs
SAR     rd, rs
CMPEQ   rd, rs
CMPLT   rd, rs
CMPLTU  rd, rs
MIN     rd, rs
MINU    rd, rs
MAX     rd, rs
MAXU    rd, rs

CLZ     rd, rs
CTZ     rd, rs
CPOP    rd, rs
NOT     rd, rs

LI      rd, imm7
ADDI    rd, imm7

LB      rd, [rb]
LBU     rd, [rb]
LH      rd, [rb]
LHU     rd, [rb]
LW      rd, [rb]
SB      rs, [rb]
SH      rs, [rb]
SW      rs, [rb]
LW/SW   [rb + 4]
LW/SW   [rb + 8]
LW/SW   [rb]+
LW/SW   -[rb]

LDX.W   rd, [rb + ri]
LDA.W   rd, [rb + ri*4]

BNZ
B
BL
JR
JALR
RET
TRAP
BREAK
NOP
```

## 23.2 Mandatory wide 32-bit core

```text
three-register non-destructive ALU
ANDN / ORN / XNOR
ROL / ROR
full three-register compares
full four-register SEL

BFEXTU / BFEXTS / BFINS
BSET / BCLR / BINV / BEXT
BSETI / BCLRI / BINVI / BEXTI

ADDI16 / ANDI16 / ORI16 / XORI16
MOVI20 / LUI20

LEA with base + scaled index + displacement
ADR
LDLIT.W

scaled/indexed byte/halfword/word loads and stores
x1/x2/x4/x8 index scaling
pre/post increment/decrement forms

BEQ / BNE / BLT / BGE / BLTU / BGEU
DBNZ
B.W / BL.W
JALR rd,rb,imm16

MUL / MULH / MULHU / MULHSU / MULC
DIV / DIVU / REM / REMU

SEXT.B / SEXT.H / ZEXT.B / ZEXT.H
REV8
```

---

# 24. Assembler selection rules

The assembler uses the same mnemonic for short and wide forms where semantics are identical.

Examples:

```asm
SUB r1, r2          ; 16-bit destructive form
SUB r1, r2, r3      ; 32-bit non-destructive form

ADDI r4, -1         ; 16-bit if immediate fits
ADDI r4, r5, 1200   ; 32-bit

CMPLT r1, r2        ; 16-bit: r1 = mask(r1 < r2)
CMPLT r1, r2, r3    ; 32-bit: r1 = mask(r2 < r3)
```

Assemblers may automatically choose an equivalent short encoding when it does not alter register semantics.

A disassembler should display the architectural operation rather than force a `.W` suffix except where instruction length is itself important to debugging or relocation.

---

# 25. Deliberately deferred topics

This document defines the integer execution architecture only.

The following require separate architectural work rather than being silently treated as implementation details:

- atomics and the memory consistency model;
- memory fences and instruction-cache synchronization;
- privileged state and exception-vector architecture;
- MMU and protection;
- floating point;
- vectors/SIMD/media operations;
- virtualization;
- debug architecture.

These later specifications may use reserved wide opcode space but may not change existing `SIA32-I` semantics.

---

# 26. Open decisions before v1.0

The largest remaining questions are:

1. Whether the four expensive short three-register primary regions are all justified after real code-density measurements.
2. Whether both short `LDX.W` and `LDA.W` survive once wide generalized loads exist.
3. Whether `DBNZ` merits a dedicated 16-bit encoding in addition to its mandatory wide form.
4. Whether `BNEZ` deserves a separate short encoding or remains an alias/sequence around the current control map.
5. Whether the wide generalized-memory displacement should remain 7 byte bits or sacrifice one update mode for a larger displacement.
6. Whether wide `LEA` should use a 10-bit or larger displacement.
7. Whether `ZAP`/`ZAPNOT`-style byte masking earns a place in the integer base.
8. Whether a compact register-list `PUSHM`/`POPM` operation is worth the precise-exception and restart complexity.
9. Whether `REV16` or bit-reverse operations have enough general systems value for the base.
10. The exact opcode numbers and relocation encodings.

The correct way to freeze these decisions is to build an assembler, emulator, and compiler backend, compile representative systems and application code, and measure both bytes and dynamic instruction count.

---

# 27. Current design position

SIA32-I v0.2 makes the following architectural commitments:

- **16 registers** and four-bit register identifiers.
- **16-bit and 32-bit instructions are both mandatory.**
- **No ISA mode switch.**
- **The first halfword determines instruction length.**
- **No arithmetic flags register.**
- **Comparisons produce full-register masks.**
- **Short `CMOV` and wide full `SEL` are both architectural.**
- **Bitfield extract/insert is mandatory.**
- **Scaled/indexed loads and stores are mandatory.**
- **Pre/post base update is architectural.**
- **Shift-and-add address generation is architectural through `LEA`.**
- **PC-relative address and literal formation are architectural.**
- **Multiply/divide uses ordinary GPR results, with no hidden HI/LO state.**
- **No delay slots.**
- **No media/SIMD family is defined in the integer base.**

The architecture is therefore best described as a **density-first mixed-width RISC**: a small and aggressive 16-bit common path backed by a mandatory 32-bit form that prevents code-density goals from impoverishing the ISA.