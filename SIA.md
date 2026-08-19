# Scalable Instruction Architecture (SIA)

## Integer Architecture — Draft v0.4

SIA is a compact, scalable 32-bit RISC instruction set architecture with a **fixed 16-bit instruction width** in the base architecture.

The first architecture defined here is the integer-only `SIA32-I` base. It has 16 architectural integer registers, no arithmetic condition-code register, no branch delay slots, and no floating-point or vector state.

The central design objective is a **32-bit machine with a dense 16-bit instruction stream**. Common operations are encoded directly. Less common rich operations may require two or more 16-bit instructions rather than introducing a second mandatory instruction length.

Numeric opcode assignments remain provisional until assembler, emulator, compiler, and code-density experiments are available.

---

# 1. Design goals

`SIA32-I` targets the following properties:

- 32-bit integer and address model.
- Exactly 16 architectural integer registers.
- Four-bit register identifiers.
- Every base instruction is exactly 16 bits.
- No instruction-set mode switch.
- No arithmetic flags register.
- Compare instructions produce full-register Boolean masks.
- Conditional move is architectural.
- Common pointer and stack walks use update addressing.
- Register-indexed scaled word load/store is architectural.
- Pair and four-register block load/store are architectural.
- PC-relative literal loading is first-class.
- Explicit carry/borrow arithmetic is architectural without hidden flags.
- Common counted loops have a compact form.
- No branch delay slots.
- Precise exceptions.
- Future extensions may add functionality but may not redefine existing encodings.

SIA deliberately accepts that some uncommon operations take two 16-bit instructions. A two-instruction sequence still occupies four bytes while preserving simple fixed-width fetch, decode, restart, and branch-target rules.

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

Every instruction is 2 bytes and every valid instruction address is 2-byte aligned.

Normal sequential execution advances the PC by 2 bytes.

There are no architecturally visible branch delay slots.

Because all v0.4 base instructions are the same size:

- no instruction may straddle a virtual-page boundary unless the 2-byte instruction itself straddles one, which cannot happen with page sizes aligned to at least 2 bytes;
- branch targets are always 2-byte aligned instruction boundaries;
- instruction restart needs no variable-length reconstruction.

---

# 3. Encoding philosophy

The base encoding uses the full 16-bit word.

Three unrestricted registers consume 12 bits:

```text
4 + 4 + 4 = 12 bits
```

leaving only four bits for selecting the operation. Three-register encodings are therefore treated as scarce and are allocated only to high-value common operations.

Two-register destructive forms are preferred for less common ALU operations. For example:

```asm
MOV r7, r3
AND r7, r4
```

occupies the same four instruction bytes as a hypothetical 32-bit three-register `AND r7,r3,r4`, at the cost of one extra dynamic instruction.

Compiler policy should prefer:

1. a direct 16-bit operation when available;
2. a destructive two-address form when register allocation permits;
3. a two-instruction sequence when preserving both sources is required;
4. literal pools, veneers, and address-generation sequences for uncommon large-range cases.

---

# 4. Reserved future extension mechanism

SIA v0.4 does **not** define any 32-bit instruction.

One primary opcode region is permanently reserved as `EXT` for future architectural growth. In v0.4 every instruction in this region raises the normal illegal-instruction exception.

A future architecture may define the reserved prefix as the first halfword of a longer 32-bit, 48-bit, or other extended instruction, but no such format is part of `SIA32-I` v0.4.

The intent is:

```text
SIA32-I v0.4 implementation:
    every valid instruction = 16 bits

future implementation, only if justified:
    reserved EXT prefix + continuation halfword(s)
```

The extension prefix is not stateful. If future longer instructions are defined, the prefix and continuation halfword(s) will constitute one architectural instruction.

---

# 5. Boolean and conditional model

SIA has no arithmetic condition-code register.

Scalar comparisons write a normal integer register.

True is represented as:

```text
0xFFFFFFFF
```

False is represented as:

```text
0x00000000
```

The base conditional operation is destructive `CMOV`:

```asm
CMOV rd, rs, rc
```

Semantics:

```c
if (rc != 0)
    rd = rs;
```

A full four-register select is intentionally not architectural.

```asm
SEL rd, rt, rf, rc
```

is synthesized as:

```asm
MOV  rd, rf
CMOV rd, rt, rc
```

Both sequences occupy four bytes if a hypothetical full select would otherwise require a 32-bit instruction.

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

# 7. Provisional primary opcode map

The following map is provisional and is intended to demonstrate that the required base fits comfortably in a fixed 16-bit encoding.

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
0111      scalar memory group
1000      pair / four-register memory group
1001      PC-relative literal load
1010      conditional / counted branch group
1011      direct branch / call group
1100      misc / system / standard extension group
1101      reserved base growth
1110      reserved base growth
1111      reserved future EXT prefix
```

This leaves three complete primary regions unavailable to ordinary v0.4 allocation:

- `1101` — reserved 16-bit base growth;
- `1110` — reserved 16-bit base growth;
- `1111` — future extension prefix.

Thus 18.75% of the primary encoding space remains completely reserved before counting unused subfunctions within allocated groups.

---

# 8. High-value three-register operations

Three-register primary slots are scarce because three unrestricted registers consume 12 of the 16 bits.

## 8.1 `ADD rd, ra, rb`

```c
rd = ra + rb;
```

`MOV rd, rs` is encoded as:

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

Semantics:

```c
rd = load_u32(rb + (ri << 2));
```

This is the compact fast path for C-style 32-bit arrays and pointer tables.

## 8.4 Scaled indexed word store

```asm
STA.W rs, [rb + ri*4]
```

Semantics:

```c
store_u32(rb + (ri << 2), rs);
```

The base does not require an unrestricted byte-indexed three-register load/store. Unscaled byte offsets can be synthesized with `ADD` followed by a scalar load/store when necessary.

---

# 9. Destination-zero escapes

Writes to `r0` are normally discarded. Selected otherwise-useless destination-zero encodings may be reclaimed for useful unary operations.

Candidate uses include:

```text
CLZ
CTZ
CPOP
REV8
```

Exact assignments are provisional until the final primary map is frozen.

---

# 10. Arithmetic and compare group

Two-register arithmetic operations are destructive:

```text
operation rd, rs
```

where the old value of `rd` is the first source.

Required operations include:

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

Comparison instructions produce SIA Boolean masks (`0` or `0xFFFFFFFF`).

Unsigned and signed comparisons are distinct operations.

Additional comparison relations should normally be synthesized by operand reversal and/or Boolean inversion rather than consuming redundant opcode space.

---

# 11. Logic and shift group

SIA uses destructive two-register logic operations.

The basic Boolean families are:

```text
AND
OR
XOR
```

The encoding may include source-negation modifiers so the same datapath can directly express useful variants such as:

```asm
AND.pn rd, rs     ; rd = rd & ~rs
AND.np rd, rs     ; rd = ~rd & rs
AND.nn rd, rs     ; rd = ~rd & ~rs
OR.pn  rd, rs
OR.np  rd, rs
OR.nn  rd, rs
XOR.pn rd, rs     ; equivalent to XNOR
```

Exact modifier syntax and bit assignments remain provisional, but the architecture should prefer source-negation modifiers over separate `ANDN`, `ORN`, `NOR`, `NAND`, and `XNOR` opcodes where this reduces encoding pressure without increasing the logic critical path.

Required shifts are:

```text
SHL
SHR
SAR
```

Register shift counts use the low five bits of the shift-count register.

Immediate shift forms for all counts `0..31` are strongly preferred if they can be encoded without displacing higher-value operations:

```text
SHLI
SHRI
SARI
```

Rotates are desirable but remain provisional pending encoding experiments.

---

# 12. Explicit carry and borrow arithmetic

SIA has no architectural carry flag.

Multi-precision arithmetic uses an explicit general-purpose register containing a Boolean carry/borrow mask:

```text
false / no carry / no borrow = 0x00000000
true  / carry    / borrow    = 0xFFFFFFFF
```

The compact forms are destructive:

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

Example 64-bit addition when the first operand may be overwritten:

```asm
LI   r7, 0
ADC  r1, r3, r7
ADC  r2, r4, r7
```

Result is `r2:r1`, with final carry in `r7`.

---

# 13. Small immediates

The base must provide compact small-immediate operations.

At minimum:

```text
LI
ADDI
```

A candidate format provides a signed 7-bit immediate:

```text
LI   rd, -64..63
ADDI rd, -64..63
```

Larger constants are formed through PC-relative literal loads rather than requiring a second instruction width.

Large logical immediates are not mandatory base instructions. A compiler may use:

```asm
LDPC.W rT, constant
AND    rd, rT
```

or the corresponding logic operation.

---

# 14. Scalar memory operations

Scalar memory operations use a value register, base register, and compact addressing-mode field.

Required operations include:

```text
LB
LBU
LH
LHU
LW
SB
SH
SW
```

Required word update forms include:

```asm
LW rv, [rb]+
SW rv, [rb]+
LW rv, -[rb]
SW rv, -[rb]
```

Semantics:

```text
LW rv,[rb]+     load from rb, then rb += 4
SW rv,[rb]+     store to rb, then rb += 4
LW rv,-[rb]     rb -= 4, then load from rb
SW rv,-[rb]     rb -= 4, then store to rb
```

This provides natural forward pointer walking and scalar stack operations:

```asm
SW r4, -[sp]     ; push
LW r4, [sp]+     ; pop
```

For an updating load, destination equal to base is illegal.

Small fixed word offsets such as `[rb+4]` and `[rb+8]` remain provisional. Pair/quad transfers reduce their importance, so unused scalar-memory modes should be preserved unless profiling demonstrates strong value.

---

# 15. Pair and four-register load/store

SIA replaces the previous arbitrary 16-register-mask `LDM/STM` design with bounded multiple-register transfers.

The required operations are:

```text
LDP   load a pair of consecutive 32-bit registers
STP   store a pair of consecutive 32-bit registers
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

Pair transfer moves exactly 8 bytes.

Four-register transfer moves exactly 16 bytes.

The memory order is increasing register number to increasing address.

For example:

```asm
ST4 r4:r7, [r8]
```

stores:

```text
[r8 +  0] = r4
[r8 +  4] = r5
[r8 +  8] = r6
[r8 + 12] = r7
```

Post-increment adds 8 for pair operations and 16 for four-register operations.

Pre-decrement subtracts the complete transfer size before the first access.

When writeback is enabled, the base register may not overlap a destination register of a load.

Register groups that would wrap beyond `r15` are illegal.

These instructions are not permitted for device/MMIO mappings. Device memory uses scalar accesses so externally visible side effects remain explicit and restartable.

## 15.1 Function-call example

A typical callee can preserve six registers in four instruction bytes:

```asm
ST4 r9:r12,  -[sp]
STP r14:r15, -[sp]
```

Restore:

```asm
LDP r14:r15, [sp]+
LD4 r9:r12,  [sp]+
RET
```

This retains most of the code-density benefit of arbitrary register-list transfers while bounding every multiple-transfer instruction to exactly two or four memory operations.

---

# 16. PC-relative literal load

PC-relative data access is considered sufficiently common to deserve a dedicated base instruction.

Required:

```asm
LDPC.W rd, target
```

The instruction loads a 32-bit word from a signed PC-relative displacement.

A candidate encoding uses an 8-bit word-scaled displacement, providing approximately a ±512-byte literal-pool range.

Example:

```asm
LDPC.W r4, .LC17
...
.LC17:
    .word 0x12345678
```

The assembler and linker are expected to place literal pools close enough to their uses.

The exact displacement width and scale remain open and must be determined from compiled-code measurements.

A separate `ADR/ADDPC` instruction is not yet mandatory. Addresses may be loaded from literal pools when necessary.

---

# 17. Address generation

SIA deliberately does not require a generalized `base + scaled-index + displacement` `LEA` instruction in the base.

The two dominant array-memory operations are already directly supported:

```asm
LDA.W rd, [base + index*4]
STA.W rs, [base + index*4]
```

Other address calculations may be composed from simple instructions.

A compact three-register `SH2ADD`-style operation remains a candidate if compiler measurements show that explicit formation of `base + index*4` is sufficiently frequent outside loads/stores.

No `SH1ADD`, `SH2ADD`, or `SH3ADD` operation is frozen in v0.4.

---

# 18. Conditional and counted branches

The base should provide compact register-test branches.

Required:

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

A compact `BZ` is desirable if it fits naturally in the same encoding group.

Direct compare-and-branch operations such as `BLT ra,rb,target` are intentionally not mandatory. They can be synthesized with a compare that produces a Boolean mask followed by `BNZ`/`BZ`.

---

# 19. Direct branch and call

The base provides direct relative control flow:

```asm
B  target
BL target
```

`BL` writes the address of the following instruction to `r14`.

A candidate encoding uses an 11-bit signed halfword displacement, giving approximately ±2 KiB reach.

Far transfers use linker-generated veneers and/or PC-relative literal loads.

Example far call sequence:

```asm
LDPC.W r8, .target_pointer
JALR   r14, r8
```

---

# 20. Indirect control flow

Required compact register control includes:

```asm
JALR rd, rb
```

Aliases:

```text
JR    rb   == JALR r0,  rb
CALLR rb   == JALR r14, rb
RET        == JALR r0,  r14
```

There is no mandatory immediate displacement in `JALR`. If address adjustment is needed, software performs it explicitly before the transfer.

---

# 21. Bit manipulation

The base should retain operations that are inexpensive in hardware and difficult or verbose to synthesize.

Strong candidates include:

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

Immediate single-bit operations are attractive because a register plus a 5-bit bit number fits comfortably in a 16-bit instruction.

Full immediate bitfield extract/insert operations from v0.3 are removed from the mandatory base.

The reason is encoding pressure: even a destructive bitfield operation needs roughly 4 register bits + 5 position bits + 5 length bits before opcode selection.

Uncommon fields should be synthesized with shifts, masks, and logical operations unless later profiling justifies a specialized extension.

---

# 22. Optional integer multiply/divide extensions

Hardware multiply and divide are not mandatory in `SIA32-I`.

The base reserves standard extension encodings and names.

## 22.1 `SIA-Zmul`

The multiply-only extension should include:

```text
MUL
MULH
MULHU
MULHSU
MULO
```

Compact forms may be destructive two-register operations unless profiling justifies allocating a scarce three-register encoding for `MUL`.

There are no hidden HI/LO result registers.

## 22.2 `SIA-M`

The full multiply/divide extension includes all `SIA-Zmul` operations plus:

```text
DIV
DIVU
REM
REMU
```

`SIA-M` implies `SIA-Zmul`.

Unsupported extension instructions raise the normal illegal-instruction exception and may be emulated by privileged software where appropriate.

The exact divide-by-zero and signed-overflow policy remains to be frozen before v1.0.

---

# 23. System operations

The base requires compact forms for at least:

```text
TRAP
BREAK
NOP
```

A later privileged architecture will define:

- user/supervisor state;
- MMU and TLB behavior;
- page permissions;
- fast kernel-call/return conventions;
- interrupt delivery;
- atomic operations and the memory model;
- virtualization support.

The integer ISA intentionally does not encode kernel objects, capability objects, process types, or device abstractions into ordinary instructions.

---

# 24. Extension model

`SIA32-I` is the mandatory integer architecture.

Initial optional extension structure:

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

Optional extensions should preferentially use reserved 16-bit subspaces where practical. The primary `EXT` region is retained for future cases where a larger instruction format is genuinely justified.

---

# 25. Architectural instruction summary

## 25.1 Mandatory common-path facilities

```text
ADD three-register
SUB and destructive arithmetic
ADDI
ADDO / SUBO
AND / OR / XOR with possible source-negation modifiers
SHL / SHR / SAR
MIN / MINU / MAX / MAXU
CMPEQ / CMPLT / CMPLTU
CMOV
explicit ADC / SBB
CLZ / CTZ / CPOP candidates
LI
scalar byte/halfword/word load/store
post-increment / pre-decrement word load/store
scaled indexed word load/store
LDP / STP
LD4 / ST4
LDPC.W
BNZ / DBNZ
B / BL
JALR / JR / CALLR / RET
TRAP / BREAK / NOP
```

## 25.2 Intentionally synthesized facilities

The following are not required as single base instructions:

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

These operations are normally expressed as two or more 16-bit instructions, literal-pool references, or linker veneers.

## 25.3 Optional facilities

```text
SIA-Zmul:
    MUL / MULH / MULHU / MULHSU / MULO

SIA-M:
    all SIA-Zmul operations
    DIV / DIVU / REM / REMU
```

---

# 26. Why pair/quad transfers replace register masks

The previous v0.3 architecture used a 16-bit register mask in a 32-bit `LDM/STM` instruction.

v0.4 instead uses bounded `LDP/STP/LD4/ST4` transfers.

Advantages:

- every base instruction remains 16 bits;
- two-register save/restore is denser than a 32-bit mask instruction;
- four-register transfer has the same four-byte code size as two pair operations and often matches the old mask form for common prologues;
- hardware knows each operation performs exactly two or four memory transfers;
- exception and restart behavior are simpler;
- ECL implementations can expose internal memory parallelism more easily;
- small implementations can serialize the fixed number of transfers;
- instruction fetch and decode remain fixed-width.

The cost is that arbitrary sparse register sets may need multiple instructions. This is accepted in exchange for simpler hardware and architecture.

---

# 27. Why v0.4 removes mandatory 32-bit instructions

The v0.3 mixed-width design required every implementation to support:

- instruction-length detection;
- second-halfword fetch;
- wide instruction packing;
- wide instruction restart rules;
- possible cache-line and page-boundary crossing;
- more complicated branch-target and predecode behavior.

As the 16-bit base became richer through update addressing, scaled array accesses, conditional move, multiple-register pair/quad transfers, PC-relative literal loads, and counted loops, fewer operations justified that complexity.

In many cases a rich 32-bit operation can be replaced by two 16-bit instructions with identical static code size.

v0.4 therefore makes fixed-width simplicity the default and leaves one primary opcode region reserved for a future extension mechanism if real workloads later justify longer instructions.

---

# 28. Design influences retained

## 28.1 SuperH

SIA retains:

- native 16-bit common-path instructions;
- PC-relative literal loading;
- post-increment and pre-decrement addressing;
- compact loop/control operations.

SIA does not adopt delay slots or a global condition bit.

## 28.2 Alpha

Useful ideas retained include:

- zero register;
- conditional data movement without arithmetic flags;
- ordinary-register integer results instead of hidden accumulator state.

## 28.3 ARM / AArch64

SIA retains the value of multiple-register memory operations but chooses bounded pair and four-register transfers rather than arbitrary register masks.

This combines old ARM's prologue/epilogue density goal with the bounded implementation complexity of AArch64-style pair transfers.

## 28.4 MRISC32 / M88k

SIA adopts two lessons:

- PC-relative operations deserve explicit code-density attention;
- source-negation modifiers can increase the usefulness of Boolean logic encodings at little hardware cost.

SIA does not currently replace shifts with full bitfield operations because the 16-bit encoding cannot afford unrestricted register, position, and length fields efficiently.

## 28.5 RISC-V

Useful ideas retained include:

- named optional extension profiles;
- optional multiply/divide with a multiply-only subset;
- high-value bit manipulation;
- no hidden multiply result state.

SIA differs fundamentally by making every base instruction 16 bits rather than using a 32-bit base with compressed forms.

---

# 29. Open decisions before v1.0

The largest remaining questions are:

1. Exact primary opcode assignments after code-density experiments.
2. Whether `SUB` deserves a scarce three-register form in addition to destructive `SUB`.
3. Whether `MUL` deserves a three-register encoding in `SIA-Zmul`.
4. Exact source-negation modifier encoding for Boolean logic.
5. Whether immediate shifts belong in the base and their exact encoding.
6. Whether `BZ` receives a compact form alongside `BNZ`.
7. Exact displacement width and scaling for `LDPC.W`.
8. Whether PC-relative address formation (`ADR`/`ADDPC`) deserves a direct 16-bit form.
9. Whether `SH2ADD` deserves a scarce three-register slot for explicit 32-bit address generation.
10. Whether scalar `[base+4]` or `[base+8]` forms are useful enough to keep once `LDP/LD4` exist.
11. Exact mode allocation for pair/quad transfers.
12. Whether pair/quad byte or halfword forms are ever justified; the default answer is no.
13. Exact compact branch displacement widths.
14. Which destination-zero escape encodings should provide `CLZ`, `CTZ`, `CPOP`, `REV8`, or other unary operations.
15. Whether full bitfield operations should remain entirely outside the base or receive a later standard extension.
16. Divide-by-zero and signed-division-overflow semantics for `SIA-M`.
17. Exact atomics required by the later multiprocessing architecture.
18. The exact future semantics, if any, of the reserved `EXT` primary region.

The architecture should be frozen only after an assembler, emulator, compiler backend, and representative code-density benchmark suite exist.

---

# 30. Current design position

SIA32-I v0.4 makes the following architectural commitments:

- **16 architectural integer registers.**
- **32-bit integer and address model.**
- **Every base instruction is exactly 16 bits.**
- **No mandatory 32-bit instruction form.**
- **One primary opcode region is reserved for future extension-length mechanisms.**
- **No arithmetic flags register.**
- **Comparisons produce full-register Boolean masks.**
- **`CMOV` is architectural; full four-register `SEL` is synthesized.**
- **Destructive two-register ALU forms are preferred to wide three-register forms.**
- **Explicit carry and borrow use a GPR Boolean mask through `ADC`/`SBB`.**
- **Scaled indexed 32-bit load and store are architectural.**
- **Post-increment and pre-decrement scalar word addressing are architectural.**
- **`LDP/STP` and `LD4/ST4` replace arbitrary register-mask `LDM/STM`.**
- **PC-relative literal loading is architectural.**
- **`DBNZ` is architectural.**
- **Large immediates and far control flow use literal pools and linker veneers.**
- **Full immediate bitfield extract/insert is not mandatory in the base.**
- **Multiply/divide remains optional through standard extensions.**
- **No hidden HI/LO multiply state.**
- **No delay slots.**
- **No media/SIMD family is defined in the integer base.**

SIA is therefore a **fixed-16-bit, 32-bit-data RISC architecture** optimized around dense memory access, conditional execution, fast function entry/exit, pointer walking, array indexing, and simple implementation across very small CPUs through high-performance ECL systems.
