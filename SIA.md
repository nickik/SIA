# Scalable Instruction Architecture (SIA)

## Integer Architecture — Draft v0.5

SIA is a compact, scalable 32-bit RISC instruction set architecture with a **fixed 16-bit instruction width** in the base architecture.

The first architecture defined here is the integer-only `SIA32-I` base. It has 16 architectural integer registers, no arithmetic condition-code register, no branch delay slots, and no floating-point or vector state.

The central design objective is a **32-bit machine with a dense 16-bit instruction stream**. Common operations are encoded directly. Less-common rich operations may require two or more 16-bit instructions rather than introducing a second mandatory instruction length.

Draft v0.5 adds architectural stack-relative spill/reload operations and compact stack adjustment. The dedicated primary assignments for `ADC`, `SBB`, and the reserved `EXT` prefix remain architectural commitments. Other numeric assignments remain subject to assembler, emulator, compiler, and code-density measurement before v1.0.

The software calling convention and data-layout rules are specified separately in [`ABI.md`](ABI.md).

---

# 1. Design goals

`SIA32-I` targets:

- 32-bit integer and address model.
- Exactly 16 architectural integer registers.
- Four-bit register identifiers.
- Every base instruction exactly 16 bits.
- No instruction-set mode switch.
- No arithmetic flags register.
- Compare instructions produce full-register Boolean masks.
- Conditional move is architectural.
- Common pointer and stack walks use update addressing.
- Register-indexed scaled word load/store is architectural.
- Pair and four-register block load/store are architectural.
- Compact compiler spill/reload through stack-pointer-relative word operations.
- PC-relative literal loading is first-class.
- Explicit carry/borrow arithmetic without hidden flags.
- Common counted loops have a compact form.
- No branch delay slots.
- Precise exceptions.
- Future extensions may add functionality but may not redefine existing encodings.

SIA deliberately accepts that some uncommon operations take two 16-bit instructions. A two-instruction sequence still occupies four bytes while preserving simple fixed-width fetch, decode, restart, and branch-target rules.

---

# 2. Architectural state

## 2.1 Integer width

`XLEN = 32` in `SIA32-I`.

Integer arithmetic is modulo 2^32 unless an instruction explicitly specifies checked arithmetic.

Addresses are 32 bits and memory is byte addressed.

## 2.2 Integer registers

SIA defines 16 integer registers:

| Register | Architectural role |
|---|---|
| `r0` | constant zero |
| `r1-r12` | general-purpose integer registers |
| `r13` / `sp` | stack pointer; hard-wired base for `LWSP`, `SWSP`, and `ADJSP` |
| `r14` | general register; link register by the standard ABI |
| `r15` | general register; optional frame pointer by the standard ABI |

Reads from `r0` return zero. Ordinary writes to `r0` are discarded except where an encoding explicitly reuses an otherwise useless destination-zero pattern as an escape.

`r13` is architecturally designated as the stack pointer because stack-relative instructions reference it implicitly. It remains readable and writable by ordinary integer instructions.

The standard software ABI assigns calling-convention roles to the remaining registers; see `ABI.md`.

## 2.3 Program counter

The PC addresses bytes.

Every instruction is 2 bytes and every valid instruction address is 2-byte aligned. Normal sequential execution advances the PC by 2 bytes.

There are no architecturally visible branch delay slots.

Because every v0.5 base instruction is exactly 16 bits:

- normal instruction fetch cannot require a continuation halfword;
- branch targets are always 2-byte aligned instruction boundaries;
- instruction restart needs no variable-length reconstruction;
- with normally aligned even-sized pages, a 2-byte instruction cannot straddle a page boundary.

---

# 3. Encoding philosophy

The base encoding uses the full 16-bit word.

Three unrestricted registers consume 12 bits:

```text
4 + 4 + 4 = 12 bits
```

leaving only four bits for selecting the operation. Unrestricted three-register encodings are therefore scarce and are allocated only to high-value operations.

Two-register destructive forms are preferred for less-common ALU operations. For example:

```asm
MOV r7, r3
AND r7, r4
```

occupies the same four instruction bytes as a hypothetical 32-bit three-register `AND r7,r3,r4`, at the cost of one additional dynamic instruction.

The architecture may also specialize common compiler patterns when an implicit architectural register buys substantially more useful immediate range. `LWSP` and `SWSP` are the principal example: omitting the explicit base register allows all 16 value registers plus a 5-bit word-scaled offset in 16 bits.

Compiler policy should prefer:

1. a direct 16-bit operation when available;
2. a destructive two-address form when register allocation permits;
3. stack-relative spill/reload forms for stack slots in range;
4. a two-instruction sequence when preserving both sources is required;
5. literal pools, veneers, and explicit address-generation sequences for uncommon large-range cases.

---

# 4. Reserved future extension mechanism

SIA v0.5 defines no mandatory 32-bit instruction.

Primary opcode `1111` is permanently reserved as `EXT`. In SIA32-I v0.5, executing an `EXT` encoding raises the normal illegal-instruction exception.

A future architecture may define the prefix as the first halfword of a longer 32-bit, 48-bit, or other extended instruction, but no such format is part of SIA32-I v0.5.

The prefix is not stateful. If longer instructions are ever defined, the prefix and continuation halfword(s) together constitute one architectural instruction.

---

# 5. Boolean and conditional model

SIA has no arithmetic condition-code register.

Scalar comparisons write a normal GPR.

```text
false = 0x00000000
true  = 0xFFFFFFFF
```

The base conditional data operation is:

```asm
CMOV rd, rs, rc
```

with semantics:

```c
if (rc != 0)
    rd = rs;
```

A full four-register select is intentionally not architectural:

```asm
MOV  rd, rf
CMOV rd, rt, rc
```

implements `SEL rd,rt,rf,rc` in two 16-bit instructions.

---

# 6. Byte order and alignment

The initial SIA machine is little-endian for instructions and data.

Natural data alignment is required:

- byte access: any byte address;
- halfword access: address divisible by 2;
- word access: address divisible by 4.

A misaligned halfword or word access traps.

The base architecture does not include partial-word unaligned load/store helpers.

---

# 7. Primary opcode map

The current primary map is:

```text
15:12     use
-----     -------------------------------------------------
0000      ADD three-register
0001      CMOV three-register
0010      scaled indexed word load
0011      scaled indexed word store
0100      arithmetic / compare group
0101      logic / shift group
0110      small immediate group
0111      scalar memory / stack-relative group
1000      pair / four-register memory group
1001      PC-relative literal load
1010      conditional / counted branch group
1011      direct branch / call group
1100      misc / system / standard extension group
1101      ADC three-register
1110      SBB three-register
1111      reserved future EXT prefix
```

All ordinary primary values `0000` through `1110` are allocated. This does **not** mean the 16-bit encoding is exhausted: grouped primaries retain subfunction space. `1111` remains permanently reserved for future architectural extension.

The dedicated `ADC` and `SBB` primaries are intentional. Explicit carry/borrow is sufficiently important for multi-precision arithmetic, software floating point, emulation, and cryptographic/bignum workloads to justify two unrestricted three-register primary slots.

---

# 8. High-value three-register operations

## 8.1 `ADD rd, ra, rb`

```c
rd = ra + rb;
```

`MOV rd,rs` is encoded as:

```asm
ADD rd, r0, rs
```

## 8.2 `CMOV rd, rs, rc`

```c
if (rc != 0)
    rd = rs;
```

## 8.3 Scaled indexed word load

```asm
LDA.W rd, [rb + ri*4]
```

```c
rd = load_u32(rb + (ri << 2));
```

This is the compact fast path for 32-bit arrays and pointer tables.

## 8.4 Scaled indexed word store

```asm
STA.W rs, [rb + ri*4]
```

```c
store_u32(rb + (ri << 2), rs);
```

The base does not require an unrestricted byte-indexed three-register load/store. Unscaled indexed addresses may be synthesized with `ADD` followed by a scalar memory operation.

## 8.5 Carry and borrow

`ADC rd,rs,rc` and `SBB rd,rs,rc` are unrestricted three-register instructions using primary opcodes `1101` and `1110`.

---

# 9. Arithmetic and compare group

Two-register arithmetic operations are destructive:

```text
operation rd, rs
```

where old `rd` is the first source.

Required operations:

```text
SUB
ADDO
SUBO
CMPEQ
CMPLT
CMPLTU
MIN
MINU
MAX
MAXU
```

`ADDO` and `SUBO` trap on signed overflow.

Comparisons produce canonical SIA Boolean masks (`0` or `0xFFFFFFFF`).

Additional comparison relations should normally be synthesized by operand reversal and/or Boolean inversion rather than consuming redundant subopcodes.

---

# 10. Logic and shifts

SIA uses destructive two-register Boolean operations:

```text
AND
OR
XOR
```

The final encoding may use source-negation modifiers so the same datapath can express useful forms such as:

```asm
AND.pn rd, rs     ; rd = rd & ~rs
AND.np rd, rs     ; rd = ~rd & rs
AND.nn rd, rs
OR.pn  rd, rs
OR.np  rd, rs
OR.nn  rd, rs
XOR.pn rd, rs     ; XNOR
```

Exact modifier bits remain provisional.

Required shifts:

```text
SHL
SHR
SAR
```

Register shift counts use the low five bits of the shift-count register.

Immediate shift forms for counts `0..31` are strongly preferred if they fit without displacing higher-value operations:

```text
SHLI
SHRI
SARI
```

---

# 11. Explicit carry and borrow arithmetic

SIA has no architectural carry flag.

Multi-precision arithmetic uses an explicit GPR containing a Boolean carry/borrow mask:

```text
false / no carry / no borrow = 0x00000000
true  / carry    / borrow    = 0xFFFFFFFF
```

Encodings:

```text
15:12  11:8  7:4  3:0
-----  ----  ---  ---
1101    rd    rs    rc      ADC
1110    rd    rs    rc      SBB
```

Forms:

```asm
ADC rd, rs, rc
SBB rd, rs, rc
```

`rc` is carry/borrow input and output.

Conceptually for `ADC`:

```c
cin = (rc != 0) ? 1 : 0;
x = (uint64_t)rd + (uint64_t)rs + cin;
rd = (uint32_t)x;
rc = (x >> 32) ? 0xFFFFFFFFu : 0;
```

For `SBB`:

```c
bin = (rc != 0) ? 1 : 0;
x = (uint64_t)rs + bin;
borrow = ((uint64_t)rd < x);
rd = rd - (uint32_t)x;
rc = borrow ? 0xFFFFFFFFu : 0;
```

Example 64-bit addition:

```asm
LI   r7, 0
ADC  r1, r3, r7
ADC  r2, r4, r7
```

Result is `r2:r1`, with final carry in `r7`.

---

# 12. Small immediates

The base provides compact small-immediate operations:

```text
LI
ADDI
```

The current candidate provides a signed 7-bit immediate:

```text
LI   rd, -64..63
ADDI rd, -64..63
```

Larger constants are formed through PC-relative literal loads.

Large logical immediates are not mandatory base instructions. A compiler may load a literal into a scratch register and use the ordinary Boolean operation.

---

# 13. Scalar memory operations

The scalar-memory primary is `0111`.

## 13.1 General scalar forms

The normal subformat is:

```text
15:12  11:8  7:4  3:0
0111    rv    rb   mode
```

Required general modes:

```text
0  LB
1  LBU
2  LH
3  LHU
4  LW
5  SB
6  SH
7  SW
C  LW  rv,[rb]+
D  SW  rv,[rb]+
E  LW  rv,-[rb]
F  SW  rv,-[rb]
```

Update semantics:

```text
LW rv,[rb]+     load from rb, then rb += 4
SW rv,[rb]+     store to rb, then rb += 4
LW rv,-[rb]     rb -= 4, then load from rb
SW rv,-[rb]     rb -= 4, then store to rb
```

An updating load with `rv == rb` is illegal.

The update forms provide natural scalar push/pop:

```asm
SW r4, -[sp]
LW r4, [sp]+
```

## 13.2 Stack-relative spill/reload subformat

General modes `8..B` are replaced by a dedicated stack-relative subformat. This consumes no additional primary opcode.

```text
15:12  11:8   7   6:4   3:2   1:0
-----  ----  ---  ----  ----  ----
0111    rv    S   imm   10    imm
```

The five immediate bits are reconstructed as:

```text
uimm5 = { bits[6:4], bits[1:0] }
```

For ordinary stack-relative word access:

```text
S = 0, rv != r0   LWSP rv, uimm5*4
S = 1             SWSP rv, uimm5*4
```

Semantics:

```c
LWSP rd, off: rd = load_u32(r13 + off);
SWSP rs, off: store_u32(r13 + off, rs);
```

where:

```text
off = uimm5 * 4
range = 0, 4, 8, ... 124 bytes
```

All 16 registers may be named by `SWSP`. `SWSP r0,off` stores a zero word and is therefore useful.

`LWSP r0,off` would otherwise discard the loaded word. Those 32 encodings are reclaimed as `ADJSP`.

## 13.3 `ADJSP`

`ADJSP` uses the `LWSP` subformat with `S=0` and `rv=r0`.

The same five payload bits are interpreted as a signed two's-complement immediate:

```asm
ADJSP simm5*16
```

Semantics:

```c
r13 = r13 + sign_extend(simm5) * 16;
```

Range:

```text
-256, -240, ... -16, 0, +16, ... +240 bytes
```

Examples:

```asm
ADJSP -64      ; allocate 64 bytes
ADJSP  64      ; release 64 bytes
```

`ADJSP` performs no memory access and cannot raise an alignment fault merely because of the arithmetic update. The standard ABI requires appropriate stack alignment at call boundaries.

The asymmetry of signed 5-bit range means `-256` is representable but `+256` is not. Compilers that require a symmetric single-instruction allocate/deallocate pair should limit such frames to 240 bytes or use more than one adjustment instruction.

## 13.4 Rationale

The stack-relative forms are architectural because compiler spills and reloads are both frequent and especially painful on a 16-register machine. Without them, a spill at `sp+24` can require temporary address formation and thereby increase register pressure precisely when the allocator has already run out of registers.

The special subformat provides:

- every GPR as a spill/reload value register;
- 32 directly addressable word slots;
- no extra primary opcode;
- no third register-file read port;
- no hidden stack state beyond the architecturally designated `r13`.

Compilers should preferentially place hot spill slots and small fixed locals in the first 128 bytes above the stable frame `sp`.

---

# 14. Pair and four-register load/store

SIA uses bounded multiple-register transfers rather than arbitrary register masks.

Required operations:

```text
LDP   load two consecutive 32-bit registers
STP   store two consecutive 32-bit registers
LD4   load four consecutive 32-bit registers
ST4   store four consecutive 32-bit registers
```

The encoded register names the first register in the group.

Examples:

```asm
LDP r4:r5, [r8]
LDP r4:r5, [r8]+
STP r4:r5, [r8]
STP r4:r5, [r8]+
STP r4:r5, -[sp]

LD4 r4:r7, [r8]
LD4 r4:r7, [r8]+
ST4 r4:r7, [r8]
ST4 r4:r7, [r8]+
ST4 r4:r7, -[sp]
```

Pair transfer moves 8 bytes. Four-register transfer moves 16 bytes.

Memory order is increasing register number to increasing address. For example:

```text
ST4 r4:r7,[r8]
[r8+0]  = r4
[r8+4]  = r5
[r8+8]  = r6
[r8+12] = r7
```

Post-increment adds the complete transfer size after all accesses. Pre-decrement subtracts the complete transfer size before the first access.

Register groups may not wrap beyond `r15`. In updating multi-load forms, the base register may not overlap the destination group.

These instructions are for ordinary memory, not MMIO mappings. Externally visible device side effects use scalar accesses.

Precise exception behavior is architectural: a faulting multiple transfer must be restartable at the original instruction without exposing a partially completed register/base update as architecturally complete.

---

# 15. PC-relative literal load

Required:

```asm
LDPC.W rd, target
```

The instruction loads a 32-bit word from a signed PC-relative displacement.

The current candidate encoding uses an 8-bit word-scaled displacement, approximately ±512 bytes. The exact architectural PC base remains to be frozen after compiler/literal-pool experiments.

Example:

```asm
LDPC.W r4, .LC17
...
.LC17:
    .word 0x12345678
```

Large constants, addresses, and far targets are expected to use nearby literal pools managed by the assembler/linker.

---

# 16. Address generation

SIA does not require a generalized `base + scaled-index + displacement` LEA instruction.

The dominant 32-bit array operations already have direct forms:

```asm
LDA.W rd, [base + index*4]
STA.W rs, [base + index*4]
```

Other addresses are composed from `ADD`, `ADDI`, literal loads, and ordinary memory operations.

A three-register `SH2ADD`-style address operation remains a candidate only if compiler measurements show sufficient benefit.

---

# 17. Conditional and counted branches

Required compact register-test branches:

```asm
BNZ  rs, target
DBNZ rs, target
```

`BNZ` branches when `rs != 0`.

`DBNZ` performs:

```c
rs = rs - 1;
if (rs != 0)
    pc = target;
```

A compact `BZ` is desirable if it fits naturally.

Direct compare-and-branch operations such as `BLT ra,rb,target` are intentionally not mandatory; comparisons produce Boolean GPRs consumed by `BNZ`/`BZ`.

---

# 18. Direct branch and call

Required:

```asm
B  target
BL target
```

`BL` writes the address of the following instruction to `r14`.

The current candidate uses an 11-bit signed halfword displacement, giving approximately ±2 KiB reach.

Far transfers use linker veneers and/or PC-relative literal loads.

---

# 19. Indirect control flow

Required:

```asm
JALR rd, rb
```

Aliases:

```text
JR    rb   == JALR r0,  rb
CALLR rb   == JALR r14, rb
RET        == JALR r0,  r14
```

No immediate displacement is mandatory in `JALR`.

---

# 20. Bit manipulation

Operations that are inexpensive in hardware and disproportionately verbose to synthesize are preferred.

Strong candidates/current compact operations include:

```text
CLZ
CTZ
CPOP
REV8
BSET
BCLR
BINV
BEXT
```

Destination-zero escapes may be used for unary operations such as `CLZ`, `CTZ`, and `CPOP` where this does not conflict with other architectural encodings.

Full immediate bitfield extract/insert is not mandatory because register + position + length consumes too much of a 16-bit instruction.

---

# 21. Optional integer multiply/divide

Hardware multiply/divide is not mandatory in `SIA32-I`.

## 21.1 `SIA-Zmul`

```text
MUL
MULH
MULHU
MULHSU
MULO
```

Compact destructive two-register forms are acceptable. There are no hidden HI/LO result registers.

## 21.2 `SIA-M`

`SIA-M` implies `SIA-Zmul` and additionally provides:

```text
DIV
DIVU
REM
REMU
```

Unsupported extension instructions raise the normal illegal-instruction exception and may be emulated by privileged software.

Divide-by-zero and signed-overflow behavior must be frozen before v1.0.

---

# 22. System operations

The base requires compact forms for at least:

```text
TRAP
BREAK
NOP
```

A later privileged architecture defines:

- user/supervisor state;
- MMU/TLB behavior;
- page permissions;
- fast kernel-call/return conventions;
- interrupt delivery;
- atomic operations and memory ordering;
- virtualization support.

The integer ISA does not encode kernel objects, capability objects, processes, or device abstractions into ordinary instructions.

Simulator semihosting conventions are not architectural I/O requirements.

---

# 23. Extension model

```text
SIA32-I        mandatory integer architecture
SIA-Zmul       optional integer multiply
SIA-M          optional multiply/divide; implies SIA-Zmul
```

Future separately specified extensions may include atomics, floating point, vectors/SIMD, virtualization, and specialized cryptography.

Optional extensions should preferentially use unused 16-bit subspaces where practical. Primary `EXT` is retained for cases where a larger instruction format is genuinely justified.

---

# 24. Architectural instruction summary

## 24.1 Mandatory common-path facilities

```text
ADD three-register
SUB and destructive arithmetic
ADDI
ADDO / SUBO
AND / OR / XOR
SHL / SHR / SAR
MIN / MINU / MAX / MAXU
CMPEQ / CMPLT / CMPLTU
CMOV
ADC / SBB three-register
LI
scalar byte/halfword/word load/store
post-increment / pre-decrement word load/store
LWSP / SWSP
ADJSP
scaled indexed word load/store
LDP / STP
LD4 / ST4
LDPC.W
BNZ / DBNZ
B / BL
JALR / JR / CALLR / RET
TRAP / BREAK / NOP
```

## 24.2 Intentionally synthesized facilities

```text
full four-register SEL
non-destructive three-register AND/OR/XOR/SUB/etc.
large immediate arithmetic/logical operations
general LEA with base + scaled index + displacement
unrestricted byte-indexed memory operations
direct compare-and-branch
long branch/call
JALR with displacement
full immediate bitfield extract/insert
arbitrary register-mask LDM/STM
```

## 24.3 Optional facilities

```text
SIA-Zmul:
    MUL / MULH / MULHU / MULHSU / MULO

SIA-M:
    all SIA-Zmul operations
    DIV / DIVU / REM / REMU
```

---

# 25. Compiler and ABI policy

The standard ABI is defined in `ABI.md`.

For code generation, compilers should:

1. Keep `sp` stable through the body of ordinary fixed-frame functions where practical.
2. Place hot word-sized spill slots within `sp+0..124` so `LWSP/SWSP` can access them directly.
3. Use `ADJSP` for 16-byte-multiple frame adjustments in range.
4. Use `ADDI sp,imm` or multiple adjustments for sizes not representable by one `ADJSP`.
5. Prefer `LDP/LD4/STP/ST4` for consecutive callee-save groups.
6. Prefer scaled indexed word memory operations for 32-bit arrays.
7. Use update forms for pointer walks and scalar push/pop.
8. Use literal pools for large constants and far targets.
9. Let the linker relax branches and insert veneers.
10. Measure spills, reloads, inserted moves, literal-pool bytes, and veneer counts before adding new ISA primitives.

The stack-relative subformat is deliberately designed around real compiler pressure: it reduces both dynamic instruction count and temporary-register demand during spills.

---

# 26. Design influences retained

## 26.1 SuperH

SIA retains native 16-bit common-path instructions, PC-relative literal loading, update addressing, and compact control flow. SuperH's short displacement memory forms reinforce the value of scaled offsets in a 16-register/16-bit ISA.

## 26.2 ARM / AArch64

SIA retains bounded multiple-register memory operations, but uses pair/four-register transfers rather than arbitrary masks. Conditional data operations are targeted rather than general scalar predication.

## 26.3 RISC-V compressed instructions

RISC-V compressed stack-relative loads/stores demonstrate that an implicit stack pointer is an efficient use of compressed encoding space. SIA follows this principle while retaining all 16 value-register names and a 5-bit word offset.

## 26.4 Alpha / Am29000-style condition model

SIA avoids a global arithmetic flags register and represents conditions as ordinary GPR values.

## 26.5 MRISC32 / M88k

SIA retains explicit attention to PC-relative addressing and may use Boolean source-negation modifiers where they are cheap.

---

# 27. Open decisions before v1.0

The largest remaining questions are:

1. Final numeric assignments for still-provisional grouped subfunctions.
2. Exact source-negation modifier encoding.
3. Whether immediate shifts receive their current candidate forms.
4. Whether compact `BZ` is mandatory alongside `BNZ`.
5. Exact `LDPC.W` PC-base rule and final displacement interpretation.
6. Whether `ADR/ADDPC` deserves a direct 16-bit form.
7. Whether `SH2ADD` merits a scarce three-register slot.
8. Whether general scalar `[base+4]` / `[base+8]` forms remain useful after `LWSP/SWSP` and pair/quad transfers.
9. Final pair/quad mode allocation and alias restrictions.
10. Exact compact branch displacement widths after linker experiments.
11. Final destination-zero assignments for `CLZ`, `CTZ`, `CPOP`, `REV8`, and related unary operations.
12. Whether full bitfields remain entirely outside the base.
13. Divide-by-zero and signed-division-overflow semantics for `SIA-M`.
14. Exact atomics and memory model for multiprocessing.
15. Future semantics, if any, of `EXT`.
16. Final ELF relocation set and object ABI after the Forge/SIA toolchain exists.

`LWSP`, `SWSP`, `ADJSP`, `ADC`, `SBB`, and the `EXT` primary are no longer open design questions in this draft.

---

# 28. Current design position

SIA32-I v0.5 commits to:

- **16 architectural integer registers.**
- **32-bit integer and address model.**
- **Every base instruction exactly 16 bits.**
- **`r13` architecturally designated as `sp`.**
- **No mandatory 32-bit instruction form.**
- **Primary `1111` reserved for future `EXT`.**
- **No arithmetic flags register.**
- **Comparisons produce full-register Boolean masks.**
- **`CMOV` architectural; four-register `SEL` synthesized.**
- **Destructive two-register ALU forms preferred for less-common arithmetic.**
- **`ADC` and `SBB` are dedicated three-register operations.**
- **Scaled indexed 32-bit load/store architectural.**
- **Post-increment and pre-decrement scalar word addressing architectural.**
- **`LWSP` and `SWSP` provide 0..124-byte word-scaled stack access to all 16 registers.**
- **`ADJSP` provides signed 16-byte-scaled stack adjustment from -256 through +240 bytes.**
- **`LDP/STP` and `LD4/ST4` replace arbitrary register-mask transfers.**
- **PC-relative literal loading architectural.**
- **`DBNZ` architectural.**
- **Large immediates and far control use literal pools and linker veneers.**
- **Full immediate bitfields are not mandatory.**
- **Multiply/divide remains optional.**
- **No hidden HI/LO multiply state.**
- **No delay slots.**
- **No media/SIMD family in the integer base.**

SIA is therefore a **fixed-16-bit, 32-bit-data RISC architecture** optimized around dense memory access, compiler-friendly spills, conditional data movement, fast function entry/exit, pointer walking, array indexing, and implementation simplicity across very small CPUs through high-performance systems.
