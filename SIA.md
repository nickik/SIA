# Scalable Instruction Architecture (SIA)

## Integer Architecture — Draft v0.3

SIA is a compact, scalable 32-bit RISC instruction set architecture built around **mandatory mixed 16-bit and 32-bit instructions**.

The first architecture defined here is the integer-only `SIA32-I` base. It has 16 architectural integer registers, no condition-code register, no branch delay slots, and no floating-point or vector state.

SIA has two equally architectural instruction lengths:

- **short** — 16-bit instructions for common operations and maximum code density;
- **wide** — 32-bit instructions for richer operand forms, larger immediates, generalized address generation, and operations that cannot be encoded cleanly in 16 bits.

Both lengths are mandatory. There is no compressed mode, wide mode, compatibility mode, or ISA-mode switch. A conforming `SIA32-I` processor executes arbitrary interleaved 16-bit and 32-bit instructions.

The design objective is an **extended 16-bit RISC**: common operations should usually fit in 16 bits, while the mandatory 32-bit form prevents code-density goals from impoverishing the instruction set.

Numeric opcode assignments remain provisional until assembler, compiler, emulator, and code-density experiments are available.

---

# 1. Design goals

`SIA32-I` targets the following properties:

- 32-bit integer and address model.
- Exactly 16 architectural integer registers.
- Four-bit register identifiers everywhere.
- Native 16-bit instructions for common operations.
- Native 32-bit instructions as a mandatory part of the same base ISA.
- The first halfword determines instruction length without mode state.
- No condition-code or arithmetic flags register.
- Compare instructions produce full-register Boolean masks.
- Conditional move and full conditional select are architectural operations.
- Common pointer and stack walks have update addressing.
- Register-indexed and scaled-array memory operations are architectural.
- Bitfield extract and insert are architectural.
- Explicit carry/borrow arithmetic is architectural without hidden flags.
- Multi-register load/store is architectural.
- PC-relative address and literal formation are first-class operations.
- No branch delay slots.
- Precise exceptions.
- Future extensions may add functionality but may not redefine existing encodings.

SIA is deliberately not a 32-bit ISA with a separate compressed operating mode. The 16-bit and 32-bit forms are parts of one instruction set.

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

There is no instruction-set mode bit in the PC or status register.

## 3.1 Crossing cache-line and virtual-page boundaries

A wide instruction may cross a cache-line, physical-page, or virtual-page boundary.

Conceptually:

```text
H0 = instruction_fetch16(PC)

if H0[15] == 0:
    instruction = H0
else:
    H1 = instruction_fetch16(PC + 2)
    instruction = H0:H1
```

Every byte belonging to an instruction must reside in memory for which instruction execution is permitted.

If translation, protection, or instruction fetch of any portion fails:

- the instruction performs no architectural operation;
- the saved exception PC is the address of `H0`;
- the fault-address register identifies the virtual address whose fetch failed;
- restart occurs from `H0` after the fault is serviced.

Software and linkers may choose to avoid page-crossing wide instructions for performance, but this is not an architectural requirement.

A control transfer is valid only when its target is the first halfword of an instruction. Software and toolchains are responsible for maintaining this property for indirect control flow; implementations are not required to reconstruct instruction boundaries by scanning backward.

---

# 4. Why SIA uses both 16 and 32 bits

After reserving the length bit, a short instruction has 15 remaining bits.

Three unrestricted registers require:

```text
4 + 4 + 4 = 12 bits
```

That leaves only three bits for identifying the operation.

Four unrestricted registers require all 16 bits before an opcode is encoded at all.

SIA therefore uses 16-bit encodings only where they provide strong density value. Richer forms use the mandatory wide encoding rather than contorting the architecture around tiny fields, implicit registers, or hidden condition state.

The intended compiler policy is:

1. Prefer a short instruction whenever it expresses the desired operation directly.
2. Use a wide instruction when it avoids extra moves, extra address-generation instructions, or destructive two-address constraints.
3. Optimize for both bytes and dynamic instruction count.

---

# 5. Boolean and conditional model

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

This makes compare results useful as Boolean conditions and as masks.

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

The base architecture does not include partial-word unaligned load/store instructions.

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

The exact short allocation is provisional. In particular, the two large indexed-load regions must be justified by code-density measurements now that generalized wide memory addressing is mandatory.

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

This is the compact fast path for a 32-bit array load.

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
| `1` | `ADDO rd, rs` | checked signed add |
| `2` | `SUBO rd, rs` | checked signed subtract |
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

`O` means checked arithmetic overflow. `ADDO` and `SUBO` trap on signed overflow.

The `C` suffix is reserved for carry semantics and is not used to mean checked arithmetic.

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

Update forms:

```text
LW rv,[rb]+     load from rb, then rb += 4
SW rv,[rb]+     store to rb, then rb += 4
LW rv,-[rb]     rb -= 4, then load from rb
SW rv,-[rb]     rb -= 4, then store to rb
```

This provides natural forward pointer walking and stack operations:

```asm
SW r4, -[sp]     ; push
LW r4, [sp]+     ; pop
```

For an updating load, `rv == rb` is illegal.

---

# 11. Short control flow

There are no delay slots.

## 11.1 Branch on nonzero

```asm
BNZ rs, target
```

Conceptually:

```c
if (rs != 0)
    pc = next_pc + sign_extend_7(disp7) * 2;
```

## 11.2 Direct branch and call

Short direct branch/call uses a signed halfword displacement.

```asm
B  target
BL target
```

`BL` writes the address of the following instruction to `r14`.

## 11.3 Register control

Required compact control operations include:

```text
JR rs
JALR rs
RET
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

`A` and `B` are normally register fields but may become immediate bits for formats that do not need two registers.

Wide opcode allocation is grouped conceptually as follows:

| `xop` range | Use |
|---|---|
| `00-0F` | three-register arithmetic and logic |
| `10-1F` | compare, select, bitfield, carry |
| `20-2F` | immediate and constant formation |
| `30-3F` | generalized memory and address generation |
| `40-4F` | control flow |
| `50-5F` | extension space including optional multiply/divide |
| `60-6F` | bit manipulation and byte operations |
| `70-7F` | reserved base growth |

Exact opcode values remain provisional.

---

# 13. Wide arithmetic and logic

Wide three-register forms remove the destructive destination constraint of the short ALU group.

Required operations include:

```text
ADD     rd, ra, rb
SUB     rd, ra, rb
ADDO    rd, ra, rb
SUBO    rd, ra, rb

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

Boolean-not combinations:

```c
ANDN: rd = ra & ~rb;
ORN:  rd = ra | ~rb;
XNOR: rd = ~(ra ^ rb);
```

---

# 14. Explicit carry and borrow arithmetic

SIA does not have an architectural carry flag.

Multi-precision arithmetic uses an explicit general-purpose register containing a SIA Boolean mask:

```text
false / no carry / no borrow = 0x00000000
true  / carry    / borrow    = 0xFFFFFFFF
```

The base ISA includes wide four-register carry operations:

```text
ADC rd, rc, ra, rb
SBB rd, rc, ra, rb
```

`rc` is both carry/borrow input and carry/borrow output.

## 14.1 `ADC`

Conceptually:

```c
cin = (rc != 0) ? 1 : 0;
x = (uint64_t)ra + (uint64_t)rb + cin;
rd = (uint32_t)x;
rc = (x >> 32) ? 0xFFFFFFFFu : 0;
```

## 14.2 `SBB`

`SBB` uses explicit borrow semantics rather than an inverted carry convention:

```c
bin = (rc != 0) ? 1 : 0;
x = (uint64_t)rb + bin;
rd = ra - (uint32_t)x;
borrow = ((uint64_t)ra < x);
rc = borrow ? 0xFFFFFFFFu : 0;
```

Example 64-bit addition on SIA32:

```asm
LI      r7, 0
ADC     r8, r7, r1, r3
ADC     r9, r7, r2, r4
```

Result is `r9:r8`, with final carry in `r7`.

`ADDO`/`SUBO` remain separate checked signed-overflow operations.

---

# 15. Wide compare and select

Wide compares are non-destructive:

```text
CMPEQ   rd, ra, rb
CMPNE   rd, ra, rb
CMPLT   rd, ra, rb
CMPLTU  rd, ra, rb
CMPLE   rd, ra, rb
CMPLEU  rd, ra, rb
```

Every result is zero or `0xFFFFFFFF`.

The base ISA includes full four-register select:

```text
SEL rd, rt, rf, rc
```

Semantics:

```c
rd = (rc != 0) ? rt : rf;
```

The compact `CMOV` remains useful when one alternative already occupies the destination.

---

# 16. Bitfield and bit-manipulation operations

Full bitfield instructions are mandatory wide operations:

```text
BFEXTU rd, rs, pos, len
BFEXTS rd, rs, pos, len
BFINS  rd, ra, rb, pos, len
```

`pos` is `0..31`. `len` is `1..32`.

Conceptually:

```c
BFEXTU: rd = (rs >> pos) & mask(len);
BFEXTS: rd = sign_extend((rs >> pos) & mask(len), len);
BFINS:  rd = (ra & ~(mask(len) << pos)) |
             ((rb & mask(len)) << pos);
```

A field extending past bit 31 is illegal.

Single-bit operations are also architectural:

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

Required helpers include:

```text
CLZ
CTZ
CPOP
NOT
SEXT.B
SEXT.H
ZEXT.B
ZEXT.H
REV8
```

---

# 17. Wide immediate arithmetic and constants

Required wide immediate operations include:

```text
ADDI rd, ra, imm16
ANDI rd, ra, imm16
ORI  rd, ra, imm16
XORI rd, ra, imm16
```

Arithmetic immediates are sign extended. Logical immediates are zero extended.

Constant formation includes:

```text
MOVI rd, simm20
LUI  rd, uimm20
```

Conceptually:

```c
MOVI: rd = sign_extend_20(simm20);
LUI:  rd = uimm20 << 12;
```

---

# 18. Address generation

## 18.1 General `LEA`

The wide base includes:

```text
LEA rd, [rb + ri*scale + disp]
```

where:

```text
scale = 1, 2, 4, or 8
```

Conceptually:

```c
rd = rb + ri * scale + sign_extend(disp);
```

`ri = r0` means no index.

### No architectural `SH1ADD`, `SH2ADD`, or `SH3ADD`

SIA does **not** allocate separate opcodes for shift-and-add instructions because both major uses are already covered:

```asm
LWX rd, [base + index*4 + disp]   ; scaled memory access
LEA rd, [base + index*4 + disp]   ; scaled address generation
```

Assemblers may accept convenient aliases:

```text
SH1ADD rd, ri, rb  == LEA rd, [rb + ri*2]
SH2ADD rd, ri, rb  == LEA rd, [rb + ri*4]
SH3ADD rd, ri, rb  == LEA rd, [rb + ri*8]
```

These aliases consume no architectural opcode space.

## 18.2 PC-relative address

```text
ADR rd, target
```

`ADR` forms a PC-relative address using a wide signed displacement.

## 18.3 PC-relative literal load

```text
LDLIT.W rd, target
```

This provides a single-instruction path for constants and addresses held in nearby literal pools.

---

# 19. Generalized wide memory addressing

Wide indexed memory instructions support:

- byte, halfword, and word accesses;
- signed and unsigned byte/halfword loads;
- loads and stores;
- register index scales x1, x2, x4, x8;
- signed byte displacement;
- optional base update.

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

`ri = r0` means no index.

Update modes include:

```text
none
post-increment base by access size
post-decrement base by access size
pre-increment base by access size
pre-decrement base by access size
```

For an updating load, destination equal to base is illegal.

The displacement is always a byte displacement; the scale applies only to the register index.

---

# 20. Multi-register load/store

SIA exploits its fixed set of 16 architectural registers by using a 16-bit register mask in a wide instruction.

The base ISA includes:

```text
LDM [rb],  {reglist}
LDM [rb]+, {reglist}

STM [rb],  {reglist}
STM -[rb], {reglist}
```

The initial base does not require every possible ARM-style increment/decrement-before/after combination. The selected forms cover the principal use cases:

- no-update block transfer;
- forward block load with post-increment;
- stack save with pre-decrement.

A future encoding may add symmetric `STM [rb]+` or `LDM -[rb]` if profiling demonstrates value.

## 20.1 Register-list ordering

The register list is a 16-bit mask, one bit per architectural register.

Selected registers are transferred in increasing register-number order to increasing memory addresses:

```text
lowest-numbered selected register  -> lowest address
...
highest-numbered selected register -> highest address
```

The register list must not be empty.

## 20.2 Writeback

For `LDM [rb]+,{list}`:

```c
EA = rb;
transfer selected registers;
rb = rb + 4 * popcount(list);
```

For `STM -[rb],{list}`:

```c
rb = rb - 4 * popcount(list);
EA = rb;
transfer selected registers;
```

When writeback is enabled, the base register may not appear in the register list.

Examples:

```asm
STM -[sp], {r9-r12,r14,r15}
...
LDM [sp]+, {r9-r12,r14,r15}
RET
```

## 20.3 Exceptions and memory type

A register-list transfer spans at most 64 bytes.

For ordinary memory, implementations must provide precise architectural exceptions. Software must not observe a partially completed architectural register update when a synchronous translation or protection fault is reported.

`LDM` and `STM` are not permitted for device/MMIO memory mappings. Device memory uses scalar accesses so that externally visible side effects remain explicit and restartable.

The privileged/memory architecture will define the exact mechanism by which ordinary and device memory are distinguished.

---

# 21. Control flow

The wide base includes direct compare-and-branch operations:

```text
BEQ   ra, rb, target
BNE   ra, rb, target
BLT   ra, rb, target
BGE   ra, rb, target
BLTU  ra, rb, target
BGEU  ra, rb, target
```

These exist for control-flow-only comparisons where materializing a Boolean mask would be wasteful.

A counted-loop instruction is also included as a candidate base operation:

```text
DBNZ rs, target
```

Semantics:

```c
rs = rs - 1;
if (rs != 0)
    pc = target;
```

Long direct control flow uses:

```text
B.W  target
BL.W target
```

Indirect control flow uses:

```text
JALR rd, rb, imm16
```

Conceptually:

```c
target = (rb + sign_extend_16(imm16)) & ~1u;
rd = next_pc;
pc = target;
```

Aliases:

```text
JR    rb   == JALR r0,  rb, 0
CALLR rb   == JALR r14, rb, 0
RET        == JALR r0,  r14, 0
```

---

# 22. Optional integer multiply/divide extensions

Hardware multiply and divide are **not mandatory in `SIA32-I`**.

The base ISA reserves standard extension encodings and names so software can target well-defined implementation profiles.

## 22.1 `SIA-Zmul`

The multiply-only extension contains:

```text
MUL
MULH
MULHU
MULHSU
MULO
```

`MUL` returns the low 32 bits.

`MULH`, `MULHU`, and `MULHSU` return the high 32 bits of signed×signed, unsigned×unsigned, and signed×unsigned products.

`MULO` traps on signed multiplication overflow.

## 22.2 `SIA-M`

The full multiply/divide extension includes all `SIA-Zmul` instructions plus:

```text
DIV
DIVU
REM
REMU
```

`SIA-M` implies `SIA-Zmul`.

Implementations may support neither extension, `SIA-Zmul` alone, or full `SIA-M`.

Unsupported extension instructions raise the normal illegal-instruction exception and may be emulated by privileged software where appropriate.

The exact divide-by-zero and signed-overflow result/trap policy remains to be frozen before v1.0.

There are no special HI/LO result registers.

---

# 23. Extension model

`SIA32-I` is the mandatory integer architecture.

Standard optional extensions use stable names and capability discovery.

Initial extension structure:

```text
SIA32-I        mandatory integer architecture
SIA-Zmul       optional integer multiply
SIA-M          optional integer multiply/divide; implies SIA-Zmul
```

Future separately specified extensions may include:

- atomics and memory consistency;
- floating point;
- vectors/SIMD;
- virtualization;
- specialized cryptography.

Media/DSP/vector families are intentionally outside this integer specification.

---

# 24. Architectural instruction summary

## 24.1 Mandatory short/common-path facilities

```text
ADD
SUB
ADDI
ADDO / SUBO
AND / OR / XOR / NOT
SHL / SHR / SAR
MIN / MINU / MAX / MAXU
CMPEQ / CMPLT / CMPLTU
CMOV
CLZ / CTZ / CPOP
LI
scalar byte/halfword/word load/store
short update load/store
short indexed/scaled word load candidates
short branches/call/return
```

## 24.2 Mandatory wide facilities

```text
non-destructive three-register ALU
ANDN / ORN / XNOR
ROL / ROR
ADC / SBB with explicit carry/borrow register
full three-register compares
full four-register SEL
BFEXTU / BFEXTS / BFINS
single-bit set/clear/invert/extract
wide immediate ALU and constant formation
LEA with base + scaled index + displacement
ADR
LDLIT.W
general scaled/indexed loads and stores
pre/post base update
LDM / STM register-mask transfers
direct compare-and-branch
long branch/call
JALR with displacement
SEXT / ZEXT helpers
REV8
```

## 24.3 Optional facilities

```text
SIA-Zmul:
    MUL / MULH / MULHU / MULHSU / MULO

SIA-M:
    all SIA-Zmul operations
    DIV / DIVU / REM / REMU
```

`SH1ADD`, `SH2ADD`, and `SH3ADD` are not architectural instructions; they are optional assembler aliases for scaled `LEA` forms.

---

# 25. Design influences retained

## 25.1 SuperH

SIA retains several useful density ideas:

- native 16-bit common-path instructions;
- PC-relative literal loading;
- PC-relative address formation;
- post-increment and pre-decrement addressing;
- compact loop/control operations.

SIA does not adopt SuperH's global T condition bit or branch delay slots.

## 25.2 Alpha

Useful ideas include:

- zero register;
- conditional data movement without ordinary arithmetic flags;
- explicit memory ordering in a later memory-model specification;
- high-word integer results as ordinary registers rather than hidden accumulator state.

## 25.3 RISC-V

Useful ideas include:

- clean standard-extension naming and profiles;
- optional multiply/divide with a multiply-only subset;
- high-value bit manipulation such as `ANDN`, `ORN`, `XNOR`, CLZ/CTZ/CPOP, min/max, rotates, and single-bit operations;
- simple halfword-aligned mixed-width instruction fetch.

SIA does not need architectural `SH1ADD`/`SH2ADD`/`SH3ADD` because generalized scaled memory addressing and `LEA` already provide the same address-generation capability.

## 25.4 MIPS

Useful ideas include:

- bitfield extract/insert;
- sign/zero extension helpers;
- simple upper-immediate constant construction;
- conventional explicit integer operations.

SIA does not adopt HI/LO multiply state, delay slots, or compressed-ISA mode switching.

## 25.5 PA-RISC

Useful ideas include:

- strong bitfield operations;
- rich but explicit address generation;
- direct compare-and-branch where materializing a comparison result is unnecessary.

SIA does not currently adopt instruction nullification.

## 25.6 ARM register-list transfer

SIA adopts the useful core of ARM-style multiple-register transfer but simplifies it:

- exactly one architectural 16-bit register mask;
- canonical register ordering;
- only the most useful no-update/post-increment/pre-decrement forms initially;
- no base-register-in-list writeback ambiguity;
- no device-memory use.

The 16-register SIA model makes this instruction especially encoding-efficient.

---

# 26. Open decisions before v1.0

The largest remaining questions are:

1. Whether all four expensive short three-register primary regions are justified after real code-density measurements.
2. Whether both short `LDX.W` and `LDA.W` survive now that wide generalized memory addressing is mandatory.
3. Whether `DBNZ` receives a dedicated 16-bit encoding in addition to its wide form.
4. Whether `BNEZ` deserves a separate compact encoding.
5. Exact displacement widths for wide generalized memory and `LEA`.
6. Whether symmetric `STM [rb]+` and `LDM -[rb]` belong in the base multi-transfer set.
7. Exact precise-fault microarchitectural requirements for `LDM/STM`, especially across page boundaries.
8. Whether `ZAP`/`ZAPNOT`-style byte-mask instructions add enough value beyond bitfields and masks.
9. Whether `REV16` or full bit-reverse operations belong in the integer base.
10. Divide-by-zero and signed-division-overflow semantics for `SIA-M`.
11. Exact opcode numbers and relocation encodings.
12. Whether every short instruction should have a semantically identical canonical wide form, simplifying decoding/tooling at the cost of some wide opcode space.

The architecture should be frozen only after an assembler, emulator, compiler backend, and representative code-density benchmark suite exist.

---

# 27. Current design position

SIA32-I v0.3 makes the following architectural commitments:

- **16 architectural integer registers.**
- **16-bit and 32-bit instructions are both mandatory.**
- **No ISA mode switch.**
- **The first halfword determines instruction length.**
- **Wide instructions may cross virtual-page boundaries with precise restart semantics.**
- **No arithmetic flags register.**
- **Comparisons produce full-register masks.**
- **Short `CMOV` and wide full `SEL` are architectural.**
- **Checked arithmetic uses `ADDO`/`SUBO`.**
- **Carry and borrow use explicit GPR mask state through `ADC`/`SBB`.**
- **Bitfield extract/insert is mandatory.**
- **Scaled/indexed loads and stores are mandatory.**
- **Pre/post base update is architectural.**
- **General scaled address generation is architectural through `LEA`.**
- **`SH1ADD`/`SH2ADD`/`SH3ADD` are aliases, not architectural opcodes.**
- **Multi-register `LDM/STM` is architectural.**
- **PC-relative address and literal formation are architectural.**
- **Multiply/divide is optional through standard extensions.**
- **No hidden HI/LO multiply state.**
- **No delay slots.**
- **No media/SIMD family is defined in the integer base.**

SIA is therefore a **density-first mixed-width RISC**: the 16-bit encoding is the common path, while the mandatory 32-bit form provides the full expressive instruction set without forcing complex operations into cramped short encodings.
