# Scalable Instruction Architecture (SIA)

## Integer Base Architecture — Draft v0.1

SIA is a compact, scalable 32-bit RISC instruction set architecture designed around a 16-bit base instruction word.

The first architecture defined here is the integer-only `SIA32-I` base. It has 16 architectural integer registers, no condition flags, and no floating-point or vector state.

The central design goal is to obtain SuperH-like instruction density without inheriting a flag-oriented execution model. SIA therefore keeps the register-mask conditional model from the Lighting/MIA work, spends encoding space on high-value address-generation operations, and reserves a clean path to future 32-bit instructions.

This document is a working specification. Instruction semantics are intended to be stable enough for an assembler, emulator, and compiler prototype. Numeric opcode assignments are still provisional until code-density experiments are performed.

---

## 1. Design goals

`SIA32-I` targets the following properties:

- 32-bit integer and address model.
- Exactly 16 architectural integer registers.
- Every instruction in the initial ISA is exactly 16 bits.
- One bit of every first instruction halfword is reserved to distinguish future 32-bit instructions.
- No condition-code or flags register.
- Compare instructions produce integer masks.
- Conditional move operates directly from a register condition.
- Common pointer and stack walks have auto-increment/decrement memory forms.
- Register-indexed and scaled-array word loads are directly encoded.
- Common integer operations remain simple to decode and pipeline.
- No branch delay slots.
- The ISA can later grow through defined extensions without changing old encodings.

The architecture is deliberately not a compressed encoding of a separate 32-bit ISA. The 16-bit form is the native base ISA.

---

## 2. Architectural state

### 2.1 Integer width

`XLEN = 32` in `SIA32-I`.

Integer arithmetic is performed modulo 2^32 unless an instruction explicitly specifies checked arithmetic.

Addresses are 32 bits and memory is byte addressed.

### 2.2 Integer registers

SIA defines 16 integer registers, encoded using four bits.

| Register | Architectural / ABI role |
|---|---|
| `r0` | Constant zero |
| `r1-r12` | General-purpose integer registers |
| `r13` | Stack pointer, `sp`, by ABI |
| `r14` | Link register, `lr` |
| `r15` | Frame pointer, `fp`, by ABI; otherwise general-purpose |

Writes to `r0` are discarded and reads from `r0` return zero, except where an encoding explicitly reuses an otherwise useless `rd = r0` pattern as an instruction escape.

Suggested initial ABI:

- `r1-r6`: arguments and return values
- `r7-r8`: caller-saved temporaries
- `r9-r12`: callee-saved registers
- `r13`: stack pointer
- `r14`: link register
- `r15`: frame pointer when used

The ABI is not part of the core instruction semantics and may be specified separately.

### 2.3 Program counter

The program counter addresses bytes.

A normal SIA32-I instruction advances the PC by 2 bytes.

There are no architecturally visible branch delay slots.

---

## 3. Instruction length and future expansion

Every SIA32-I v0.1 instruction is one 16-bit halfword.

The most significant bit of the first halfword is the instruction-length discriminator:

```text
bit 15 = 0   current 16-bit SIA instruction
bit 15 = 1   reserved prefix for a future 32-bit SIA instruction
```

A v0.1 implementation shall treat an instruction whose first halfword has bit 15 set as an illegal instruction.

A future 32-bit encoding will consume the first halfword with bit 15 set plus the following 16-bit halfword. Future 32-bit instructions will remain aligned only to 16-bit boundaries unless a later specification states otherwise.

This reservation guarantees that future wider instructions can be added without reinterpreting any valid v0.1 instruction.

---

## 4. Why SIA is not a fully three-address ISA

After the length bit is reserved, a 16-bit SIA instruction has 15 remaining encoding bits.

Three unrestricted register operands require:

```text
4 + 4 + 4 = 12 bits
```

That leaves only three bits for distinguishing the operation.

Therefore the entire 16-bit namespace could encode at most eight unrestricted three-register operations if every encoding were consumed by that format.

SIA deliberately does not spend its encoding budget that way.

Instead:

- a few especially valuable operations use full three-register encodings;
- most ALU operations use two-address destructive forms;
- immediates use dedicated compact formats;
- control flow uses register-plus-relative-displacement formats;
- future 32-bit instructions are reserved for operations whose operand structure genuinely needs more bits.

This is the principal architectural difference from the earlier Lighting ISA v2 three-register model.

---

## 5. Execution and conditional model

SIA has no arithmetic condition flags.

Comparisons write a normal integer register.

A true comparison result is:

```text
0xFFFFFFFF
```

A false comparison result is:

```text
0x00000000
```

This makes compare results directly useful as masks and as conditions for conditional move or branch instructions.

Example:

```asm
MOV     r7, r1
CMPLT   r7, r2          ; r7 = (r1 < r2) ? 0xFFFFFFFF : 0
MOV     r3, r2          ; false value
CMOV    r3, r1, r7      ; if r7 != 0, r3 = r1
```

Conceptually:

```c
r3 = ((int32_t)r1 < (int32_t)r2) ? r1 : r2;
```

`CMOV` does not need a condition-code register.

---

## 6. Byte order and alignment

The initial SIA machine is little-endian for both instructions and data.

Required natural alignment:

- byte access: any byte address
- halfword access: address divisible by 2
- word access: address divisible by 4

A misaligned halfword or word memory access traps in the base architecture.

A later extension may define hardware-supported unaligned access without changing the base semantics.

---

# 7. Draft binary encoding

Opcode allocation in this section is provisional, but the formats demonstrate that the required base architecture fits in the 16-bit namespace while preserving bit 15 for future 32-bit instructions.

## 7.1 Primary map

```text
15 14 13 12 11                         0
+--+--------+----------------------------+
|0 | primary|          payload           |
+--+--------+----------------------------+
```

The initial primary assignments are:

| Bits `14:12` | Format / instruction |
|---|---|
| `000` | `ADD rd, ra, rb` |
| `001` | `CMOV rd, rs, rc` |
| `010` | `LDX.W rd, [rb + ri]` |
| `011` | `LDA.W rd, [rb + (ri << 2)]` |
| `100` | two-register ALU group |
| `101` | 7-bit immediate group |
| `110` | scalar memory group |
| `111` | control-flow / system group |

The first four entries are intentionally expensive full three-register formats. Each consumes 4096 possible 16-bit encodings. They are reserved for operations where the third register removes particularly common instruction sequences.

---

## 7.2 Full three-register format

```text
15   14:12   11:8   7:4   3:0
+--+--------+------+------+------+
|0 | opcode |  rd  |  ra  |  rb  |
+--+--------+------+------+------+
```

### `ADD rd, ra, rb`

```c
rd = ra + rb;
```

`ADD` is the principal non-destructive arithmetic/address-generation instruction.

It also provides common pseudos such as:

```asm
MOV rd, rs       ; ADD rd, r0, rs
```

and acts as the basic unscaled `LEA` operation:

```asm
ADD rd, base, index
```

### `CMOV rd, rs, rc`

```c
if (rc != 0)
    rd = rs;
```

If `rc` is zero, `rd` retains its old value.

This destructive-destination conditional move is chosen because a full four-register select:

```text
SEL rd, true_value, false_value, condition
```

cannot be encoded in 16 bits with 16 registers and the reserved length bit.

A full select is therefore:

```asm
MOV     rd, false_value
CMOV    rd, true_value, condition
```

### `LDX.W rd, [rb + ri]`

```c
rd = load_u32(rb + ri);
```

The index is an unscaled byte offset.

This form is intended for already-scaled offsets, structure tables, interpreter tables, and addresses produced by previous arithmetic.

### `LDA.W rd, [rb + (ri << 2)]`

```c
rd = load_u32(rb + (ri << 2));
```

This is the direct 32-bit array-load instruction.

It performs the common operation:

```c
rd = ((uint32_t *)rb)[ri];
```

without a separate shift or address-generation instruction.

The base ISA does not provide an indexed store. An indexed store is synthesized using `ADD` plus `SW`. A future 32-bit encoding may add fully general indexed/scaled loads and stores.

---

## 7.3 Destination-zero escape encodings

For the four full three-register primary operations, `rd = r0` would normally discard the result and is therefore not useful.

SIA reserves those patterns as compact escape encodings.

The initial assignments are:

| Primary operation with `rd = r0` | Reinterpreted instruction |
|---|---|
| `ADD` slot | `CLZ rd, rs` |
| `CMOV` slot | `CTZ rd, rs` |
| `LDX.W` slot | `CPOP rd, rs` |
| `LDA.W` slot | `NOT rd, rs` |

In these escape encodings, the two remaining 4-bit register fields are reinterpreted as destination and source.

This recovers useful encoding space without removing any meaningful operation, because writing the normal result to `r0` would be discarded.

### `CLZ rd, rs`

Count leading zero bits.

```c
rd = clz32(rs);
```

### `CTZ rd, rs`

Count trailing zero bits.

```c
rd = ctz32(rs);
```

### `CPOP rd, rs`

Population count.

```c
rd = popcount32(rs);
```

### `NOT rd, rs`

```c
rd = ~rs;
```

---

# 8. Integer ALU group

Encoding:

```text
15 14:12 11:8 7:4 3:0
+--+-----+----+---+----+
|0 | 100 | rd |rs | fn |
+--+-----+----+---+----+
```

Unless otherwise stated, the two-address operation reads the old value of `rd` as its first source and then replaces `rd`.

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

## 8.1 Checked arithmetic

`ADDC` and `SUBC` use `C` to mean **checked**, not carry.

`ADDC rd, rs` performs signed 32-bit addition and traps on signed overflow.

`SUBC rd, rs` performs signed 32-bit subtraction and traps on signed overflow.

Ordinary `ADD`, `ADDI`, and `SUB` wrap modulo 2^32.

There is no architectural carry flag.

Multi-precision arithmetic, if later standardized, will use explicit register results rather than a hidden flags register.

## 8.2 Shifts

For register shifts, the effective shift count is:

```c
rs & 31
```

SIA retains conventional shift instructions in the 16-bit base.

The earlier Lighting design attempted to use general bitfield operations as the base shift substrate. With only 16 instruction bits, two unrestricted 5-bit bitfield immediates plus register fields do not fit cleanly. Full `BFEXT`/`BFINS` operations are therefore candidates for the future 32-bit encoding space rather than the 16-bit base.

## 8.3 Derived comparisons

Only three primitive comparisons are required in the base encoding:

- equal
- signed less-than
- unsigned less-than

Other comparisons are assembler pseudos or compiler sequences.

For example:

```text
CMPNE   = CMPEQ followed by NOT
CMPGE   = CMPLT followed by NOT
CMPGEU  = CMPLTU followed by NOT
```

`CMPLE` and `CMPLEU` are formed by reversing operands for the corresponding greater-than test and inverting the result.

---

# 9. Immediate group

Encoding:

```text
15 14:12 11 10:7 6:0
+--+-----+--+----+-------+
|0 | 101 |op| rd | imm7  |
+--+-----+--+----+-------+
```

`imm7` is a signed two's-complement immediate in the range `-64..+63`.

| `op` | Instruction |
|---|---|
| `0` | `LI rd, imm7` |
| `1` | `ADDI rd, imm7` |

### `LI rd, imm7`

```c
rd = sign_extend_7(imm7);
```

### `ADDI rd, imm7`

```c
rd = rd + sign_extend_7(imm7);
```

This form covers common stack adjustment, loop stepping, small constants, and pointer movement.

Larger constants are built using literal loads or multi-instruction sequences. A future 32-bit instruction format is expected to provide larger immediate fields.

---

# 10. Scalar memory group

Encoding:

```text
15 14:12 11:8 7:4 3:0
+--+-----+----+----+------+
|0 | 110 | rv | rb | mode |
+--+-----+----+----+------+
```

`rv` is the value register and `rb` is the base-address register.

The initial mode map is:

| `mode` | Instruction | Effective address / update |
|---|---|---|
| `0` | `LB rv, [rb]` | byte, sign extend |
| `1` | `LBU rv, [rb]` | byte, zero extend |
| `2` | `LH rv, [rb]` | halfword, sign extend |
| `3` | `LHU rv, [rb]` | halfword, zero extend |
| `4` | `LW rv, [rb]` | word |
| `5` | `SB rv, [rb]` | byte store |
| `6` | `SH rv, [rb]` | halfword store |
| `7` | `SW rv, [rb]` | word store |
| `8` | `LW rv, [rb + 4]` | word displacement |
| `9` | `SW rv, [rb + 4]` | word displacement |
| `A` | `LW rv, [rb + 8]` | word displacement |
| `B` | `SW rv, [rb + 8]` | word displacement |
| `C` | `LW rv, [rb]+` | load, then `rb += 4` |
| `D` | `SW rv, [rb]+` | store, then `rb += 4` |
| `E` | `LW rv, [rb]-` | load, then `rb -= 4` |
| `F` | `SW rv, [rb]-` | store, then `rb -= 4` |

The update forms are intentionally explicit architectural operations, not assembler macros.

They are useful for:

- pointer walks
- loops over word arrays
- streaming parsers
- stack-like structures
- copy/fill loops

The base draft uses post-update word operations because they provide the highest value per encoding bit. Pre-update forms and byte/halfword update forms remain open candidates for later encoding revisions or the future 32-bit extension.

## 10.1 Larger displacements

The 16-bit base does not attempt to put a general displacement field on every load and store.

For a larger field offset, software can use:

```asm
MOV     rt, rb
ADDI    rt, offset
LW      rd, [rt]
```

or an equivalent sequence.

This is an intentional tradeoff: indexed/scaled loads and update modes receive encoding space before large scalar displacements.

---

# 11. Control flow

SIA control flow is PC relative where practical and has no delay slots.

## 11.1 Branch on nonzero / true

Encoding:

```text
15 14:11 10:7 6:0
+--+-----+----+-------+
|0 |1110 | rs | disp7 |
+--+-----+----+-------+
```

For `rs != r0`:

```asm
BNZ rs, target
```

Semantics:

```c
if (rs != 0)
    pc = next_pc + sign_extend_7(disp7) * 2;
```

Because compare instructions produce zero or all-ones masks, `BNZ` is also the natural branch-on-true instruction.

`BT` is an assembler alias for `BNZ`.

The branch range is approximately ±128 bytes from the following instruction.

A separate branch-on-false encoding is not required in the initial base. Compilers may invert the comparison, invert its mask, or arrange the likely path as fall-through.

## 11.2 Direct branch and call

Encoding:

```text
15 14:11 10 9:0
+--+-----+--+----------+
|0 |1111 | L| disp10   |
+--+-----+--+----------+
```

### `B target`

`L = 0`

```c
pc = next_pc + sign_extend_10(disp10) * 2;
```

### `BL target`

`L = 1`

```c
r14 = next_pc;
pc  = next_pc + sign_extend_10(disp10) * 2;
```

`BL` therefore uses the architectural link register and does not spend four encoding bits selecting an arbitrary link destination.

The direct branch/call range is approximately ±1 KiB.

## 11.3 Register branch escape

The encoding that would otherwise be `BNZ r0, ...` can never branch and is therefore reserved as a control escape.

Its seven low bits are interpreted as:

```text
6:4   subop
3:0   register / small immediate
```

Initial suboperations:

| `subop` | Instruction |
|---|---|
| `000` | `JR rs` |
| `001` | `JALR rs` |
| `010` | `TRAP imm4` |
| `011` | `BREAK` when low field is zero |
| `100` | `NOP` when low field is zero |
| `101-111` | reserved |

### `JR rs`

```c
pc = rs;
```

### `JALR rs`

```c
r14 = next_pc;
pc = rs;
```

### `RET`

Assembler alias:

```asm
RET     ; JR r14
```

### `TRAP imm4`

Raises a synchronous software trap with a 4-bit architected trap number.

### `BREAK`

Raises the architected breakpoint/debug exception.

---

# 12. Base instruction summary

## Arithmetic and logic

```text
ADD     rd, ra, rb
SUB     rd, rs
ADDI    rd, imm7
ADDC    rd, rs
SUBC    rd, rs
AND     rd, rs
OR      rd, rs
XOR     rd, rs
NOT     rd, rs
SHL     rd, rs
SHR     rd, rs
SAR     rd, rs
MIN     rd, rs
MINU    rd, rs
MAX     rd, rs
MAXU    rd, rs
```

## Compare and conditional move

```text
CMPEQ   rd, rs
CMPLT   rd, rs
CMPLTU  rd, rs
CMOV    rd, rs, rc
```

Compare results are always full 32-bit masks.

## Bit helpers

```text
CLZ     rd, rs
CTZ     rd, rs
CPOP    rd, rs
```

## Constants

```text
LI      rd, imm7
```

## Memory

```text
LB      rd, [rb]
LBU     rd, [rb]
LH      rd, [rb]
LHU     rd, [rb]
LW      rd, [rb]
SB      rs, [rb]
SH      rs, [rb]
SW      rs, [rb]

LW      rd, [rb + 4]
LW      rd, [rb + 8]
SW      rs, [rb + 4]
SW      rs, [rb + 8]

LW      rd, [rb]+
SW      rs, [rb]+
LW      rd, [rb]-
SW      rs, [rb]-

LDX.W   rd, [rb + ri]
LDA.W   rd, [rb + (ri << 2)]
```

## Control flow

```text
BNZ     rs, target
B       target
BL      target
JR      rs
JALR    rs
RET
TRAP    imm4
BREAK
NOP
```

---

# 13. Required assembler pseudos

The assembler should provide common operations even when the base encoding intentionally avoids allocating a separate opcode.

### Move

```asm
MOV rd, rs
```

expands to:

```asm
ADD rd, r0, rs
```

### Full conditional select

```asm
SEL rd, rt, rf, rc
```

expands to:

```asm
MOV  rd, rf
CMOV rd, rt, rc
```

### Compare not equal

```asm
CMPNE rd, rs
```

expands to:

```asm
CMPEQ rd, rs
NOT   rd, rd
```

### Compare greater or equal

```asm
CMPGE rd, rs
```

expands to:

```asm
CMPLT rd, rs
NOT   rd, rd
```

Unsigned `CMPGEU` is analogous.

### Larger load/store displacement

The assembler may accept:

```asm
LW rd, [rb + imm]
```

for offsets not directly encodable and expand it using a temporary register when explicitly permitted by the assembler interface. Compiler back ends should normally perform the expansion themselves so register allocation remains visible.

---

# 14. Features intentionally not in the 16-bit base

The following Lighting/MIA ideas remain architecturally desirable but do not receive dedicated v0.1 16-bit encodings yet.

## 14.1 Full immediate bitfield operations

The earlier forms:

```text
BFEXTU rd, rs, pos, len
BFEXTS rd, rs, pos, len
BFINS  rd, ra, rb, pos, len
```

are too operand-heavy for a clean unrestricted 16-bit encoding with 16 registers.

They are strong candidates for the future 32-bit instruction space.

## 14.2 Full four-register select

```text
SEL rd, rt, rf, rc
```

would consume all 16 bits merely naming four registers, before any opcode or length discriminator is encoded.

The base therefore uses destructive `CMOV` plus the `SEL` pseudo.

## 14.3 Fully general scaled loads and stores

The base directly includes the two high-value word-load forms:

```text
LDX.W  rd, [base + index]
LDA.W  rd, [base + index*4]
```

It does not attempt to encode every combination of:

- byte / halfword / word
- load / store
- scale x1 / x2 / x4 / x8
- arbitrary displacement
- pre/post update

That full addressing matrix belongs in the future 32-bit encoding space if workload measurements justify it.

## 14.4 Additional boolean helpers

`ANDN`, `ORN`, and `XNOR` are useful but can be synthesized from `NOT` plus the basic logic operations. They are candidates for later extensions if profiling shows enough density or performance benefit.

## 14.5 Single-bit operations

`BSET`, `BCLR`, and `BINV` remain candidates for a later encoding pass. The base should not allocate them until real compiler output is measured against the remaining opcode space.

---

# 15. Programming examples

## 15.1 Array load

C:

```c
x = array[i];
```

SIA:

```asm
LDA.W   r3, [r1 + (r2 << 2)]
```

One 16-bit instruction performs the scale, address add, and word load.

## 15.2 Pointer walk

C:

```c
x = *p++;
```

SIA:

```asm
LW      r1, [r2]+
```

## 15.3 Reverse pointer walk

C:

```c
x = *p;
p -= 4;
```

SIA:

```asm
LW      r1, [r2]-
```

## 15.4 Conditional minimum using compare mask and CMOV

```asm
MOV     r7, r1
CMPLT   r7, r2
MOV     r3, r2
CMOV    r3, r1, r7
```

For ordinary signed minimum, the dedicated `MIN` instruction is shorter:

```asm
MOV     r3, r1
MIN     r3, r2
```

## 15.5 Counted loop

```asm
loop:
        ; body
        ADDI    r4, -1
        BNZ     r4, loop
```

A dedicated decrement-and-branch instruction is not yet allocated in v0.1. It remains a candidate for code-density measurement.

---

# 16. Extension model

SIA is intended to grow through explicitly named extensions and implementation profiles.

The current document defines only the integer base `SIA32-I`.

Possible future architectural work includes:

- 32-bit extended instruction encodings using the reserved length bit
- integer multiply/divide extension
- atomic-memory extension
- bit-manipulation extension
- privileged architecture
- MMU and protection architecture
- floating-point extension
- vector/SIMD extension

None of those are required to implement the integer base described here.

An implementation shall not repurpose an encoding marked reserved by this specification unless a later SIA specification assigns it.

---

# 17. Open decisions before v1.0

The following items should be resolved using small compiler and code-density experiments rather than intuition alone.

1. **Auto-update direction** — keep the current post-increment/post-decrement word forms, or replace decrement with pre-decrement for stack use.
2. **Indexed-load budget** — retain both unscaled `LDX.W` and x4 `LDA.W`, or use one of those large opcode regions for another three-register operation.
3. **Three-register arithmetic** — decide whether any operation beyond `ADD` deserves an unrestricted three-register encoding.
4. **Branch-on-false** — determine whether a second short conditional branch materially improves generated code enough to spend encoding space.
5. **DBNZ** — measure whether a decrement-and-branch instruction is worth a dedicated encoding.
6. **Bit operations** — measure `BSET`, `BCLR`, `BINV`, `ANDN`, `ORN`, and `XNOR` frequency.
7. **Scalar displacement** — determine whether `[base+4]` and `[base+8]` are the best direct word offsets or whether a different compact displacement scheme gives better density.
8. **Large constants** — decide whether literal pools are sufficient until the 32-bit extension, or whether more 16-bit immediate-building support is required.
9. **Misaligned access** — retain mandatory traps in the base or define architecturally supported unaligned loads/stores.
10. **Exact opcode values** — freeze only after assembler, emulator, and compiler experiments validate the format allocation.

---

# 18. Current design position

The current SIA32-I draft makes the following core bets:

- **16 registers are enough** for a dense 16-bit RISC ISA.
- **16-bit instructions are the native ISA**, not a secondary compressed mode.
- **one instruction-length bit is worth reserving now** for clean future 32-bit growth.
- **flags are unnecessary**; comparisons should produce normal integer masks.
- **conditional move is worth a full three-register encoding**.
- **direct indexed and scaled array loads are worth encoding** because they remove common address-generation sequences.
- **auto-update memory operations are worth keeping** for dense pointer-walking code.
- **full bitfield and general addressing forms should wait for 32-bit encodings** rather than damaging the regularity of the 16-bit base.

This is the baseline to test against generated code before the v1.0 encoding is frozen.
