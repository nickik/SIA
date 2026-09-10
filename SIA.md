# Scalable Instruction Architecture (SIA)

## SIA32-I Integer Architecture

### Status

**SIA32-I VM baseline: FROZEN — 2026-09-10.**

This document defines the integer ISA baseline used by the Rust executable reference in `nickik/LightingSimulation`.

The freeze means SIA32-P, the MMU, the ABI, Forge, and later hardware work may depend on these encodings and semantics. It does **not** declare an immutable final SIA v1.0: later compiler and code-density evidence may justify a separately reviewed successor baseline.

Any change to a frozen item below requires an explicit baseline revision and synchronized specification, assembler, interpreter, and conformance-test changes.

---

# 1. Architectural model

```text
XLEN               32 bits
address width       32 bits
byte order          little-endian
instruction width   16 bits, fixed
registers           r0..r15
```

Register conventions:

```text
r0       constant zero
r1-r12   general-purpose
r13      sp by ABI
r14      lr by ABI
r15      general-purpose; optional frame pointer by ABI
```

Reads of `r0` return zero. Ordinary writes to `r0` are discarded, except that selected destination-zero encodings are intentionally assigned to unary instructions.

There is no arithmetic flags register, no branch delay slot, no instruction-set mode bit, and no mandatory floating-point or vector state.

Integer arithmetic is modulo 2^32 unless an instruction explicitly defines a checked operation.

Comparisons return a full-register Boolean mask:

```text
false = 0x00000000
true  = 0xFFFFFFFF
```

---

# 2. Alignment and precise faults

Every instruction is exactly 2 bytes and every valid instruction address is 2-byte aligned.

Natural data alignment is required:

```text
byte       any address
halfword   address divisible by 2
word       address divisible by 4
```

A failing instruction is precise: architectural effects belonging to that instruction are not partially committed.

SIA32-I defines the integer behavior; SIA32-P defines the architectural trap state used to report faults.

---

# 3. Frozen primary opcode map

```text
15:12  use
-----  ---------------------------------------------------------
0x0    ADD / CLZ destination-zero escape
0x1    CMOV / CTZ destination-zero escape
0x2    scaled indexed LDA.W / CPOP destination-zero escape
0x3    scaled indexed STA.W
0x4    arithmetic / compare
0x5    logic / shift
0x6    LI / ADDI
0x7    scalar memory
0x8    pair / four-register memory
0x9    LDPC.W
0xA    BNZ / DBNZ
0xB    B / BL
0xC    indirect control / bit operations / extension subspace / TRAP
0xD    ADC
0xE    SBB
0xF    extension/system primary; illegal in bare SIA32-I
```

Bare SIA32-I assigns no valid `0xFxxx` instruction. Standard extensions may define compatible meanings there. SIA32-P uses part of this primary while preserving an `0xFFxx` long-extension escape.

---

# 4. Three-register common path

```text
ADD rd,ra,rb
    rd = ra + rb modulo 2^32

MOV rd,rs
    alias of ADD rd,r0,rs

CMOV rd,rs,rc
    if rc != 0: rd = rs

LDA.W rd,[rb+ri*4]
    rd = load32(rb + ri*4)

STA.W rs,[rb+ri*4]
    store32(rb + ri*4, rs)
```

The scaled indexed forms use wrapping 32-bit address arithmetic before normal alignment/protection checks.

---

# 5. Destination-zero unary escapes

The following assignments are frozen:

```text
primary 0x0 with encoded rd=0   CLZ  rd,rs
primary 0x1 with encoded rd=0   CTZ  rd,rs
primary 0x2 with encoded rd=0   CPOP rd,rs
```

Semantics:

```text
CLZ    count leading zero bits
CTZ    count trailing zero bits
CPOP   population count
```

Assembler source that would accidentally spell the ordinary three-register operation with encoded destination `r0` is rejected when it aliases one of these unary instructions.

`REV8` is not a destination-zero escape; it has its own `0xC` subfunction.

---

# 6. Arithmetic and compare group

Format:

```text
0100 rd rs fn
```

All operations are destructive two-register forms; old `rd` is the first source.

```text
fn   instruction
---  -----------
0x0  SUB
0x1  ADDO
0x2  SUBO
0x3  CMPEQ
0x4  CMPLT
0x5  CMPLTU
0x6  MIN
0x7  MINU
0x8  MAX
0x9  MAXU
A-F  reserved
```

`ADDO` and `SUBO` perform signed 32-bit checked arithmetic. On overflow they fault and do not modify `rd`.

Signed and unsigned comparisons/min/max are distinct. Additional comparison relations are synthesized by operand order and ordinary Boolean manipulation.

---

# 7. Logic and shift group

Format:

```text
0101 rd rs-or-imm4 fn
```

```text
fn       instruction
-------  ----------------
0x0      AND rd,rs
0x1      OR rd,rs
0x2      XOR rd,rs
0x3      SHL rd,rs
0x4      SHR rd,rs
0x5      SAR rd,rs
0x6/7    SHLI rd,0..31
0x8/9    SHRI rd,0..31
0xA/B    SARI rd,0..31
0xC-F    reserved
```

Register shift counts use the low five bits of the shift-count register.

Immediate shift encodings cover every count `0..31`.

Earlier source-negation Boolean variants are **not part of the frozen SIA32-I VM baseline**.

---

# 8. Explicit carry and borrow

SIA has no hidden carry flag.

```text
ADC rd,rs,rc    primary 0xD
SBB rd,rs,rc    primary 0xE
```

`rc` contains the carry/borrow input and receives the output as a Boolean mask:

```text
0x00000000   false
0xFFFFFFFF   true
```

`ADC`:

```text
cin = (old_rc != 0) ? 1 : 0
sum = old_rd + old_rs + cin
rd  = low32(sum)
rc  = carry ? 0xFFFFFFFF : 0
```

`SBB`:

```text
bin    = (old_rc != 0) ? 1 : 0
rhs    = old_rs + bin
borrow = old_rd < rhs
rd     = low32(old_rd - rhs)
rc     = borrow ? 0xFFFFFFFF : 0
```

Because `rd` and `rc` are both architectural outputs, `rd == rc` is illegal.

All legal overlapping source/output cases use the pre-instruction values.

---

# 9. Small immediates

```text
LI   rd,imm7
ADDI rd,imm7
```

The immediate is signed 7-bit:

```text
-64 .. +63
```

`ADDI` is modulo 2^32.

Larger constants use literal pools or multi-instruction sequences.

---

# 10. Scalar memory

Format:

```text
0111 rv rb mode
```

```text
mode  instruction
----  ----------------
0x0   LB
0x1   LBU
0x2   LH
0x3   LHU
0x4   LW
0x5   SB
0x6   SH
0x7   SW
0x8-B reserved
0xC   LW rv,[rb]+
0xD   SW rv,[rb]+
0xE   LW rv,-[rb]
0xF   SW rv,-[rb]
```

Update semantics:

```text
LW rv,[rb]+     access old rb; on success rb += 4
SW rv,[rb]+     access old rb; on success rb += 4
LW rv,-[rb]     candidate = rb-4; access candidate; on success rb=candidate
SW rv,-[rb]     candidate = rb-4; access candidate; on success rb=candidate
```

Updating loads with `rv == rb` are illegal.

All alignment, translation, permission, and access checks must succeed before architecturally visible writeback. A fault therefore does not leave a changed destination/base or partial store from the faulting instruction.

---

# 11. Pair and four-register transfers

Format:

```text
1000 first rb mode
```

```text
mode  instruction
----  ----------------------
0x0   LDP  first,[rb]
0x1   LDP  first,[rb]+
0x2   STP  first,[rb]
0x3   STP  first,[rb]+
0x4   STP  first,-[rb]
0x5   LD4  first,[rb]
0x6   LD4  first,[rb]+
0x7   ST4  first,[rb]
0x8   ST4  first,[rb]+
0x9   ST4  first,-[rb]
0xA-F reserved
```

`LDP/STP` transfer exactly 8 bytes and two consecutive registers.

`LD4/ST4` transfer exactly 16 bytes and four consecutive registers.

Increasing register number corresponds to increasing memory address.

Register groups may not wrap past `r15`.

An updating load may not use a base register contained in its destination group.

Pre-decrement subtracts the complete transfer size before the first access; post-increment adds the complete size after success.

Recoverable faults are all-or-nothing as specified by `SIA32-MULTI-TRANSFER.md`.

These operations are not valid for MMIO mappings.

---

# 12. PC-relative literal load

```text
LDPC.W rd,disp8
```

The frozen address rule is:

```text
base    = align_down(instruction_PC + 4, 4)
address = base + sign_extend(disp8) * 4
rd      = load32(address)
```

`disp8` is signed `-128..127` words.

This yields approximately ±512 bytes of literal reach.

---

# 13. Conditional and counted branches

```text
BNZ  rs,disp7
DBNZ rs,disp7
```

Displacements are signed halfword counts relative to the address of the following instruction:

```text
-64 .. +63 halfwords
```

`BNZ` branches when `rs != 0`.

`DBNZ` performs:

```text
rs = rs - 1 modulo 2^32
if rs != 0: branch
```

There is **no dedicated BZ** in the frozen baseline.

---

# 14. Direct branch and call

```text
B  disp11
BL disp11
```

The displacement is a signed halfword count relative to the following instruction:

```text
-1024 .. +1023 halfwords
```

`BL` writes the following instruction address to `r14` before transferring control.

Because relative targets are halfword scaled, valid encoded branch targets are naturally 2-byte aligned.

Far transfers use literal pools and linker veneers.

---

# 15. Indirect control flow

```text
JALR rd,rb
JR rb        == JALR r0,rb
CALLR rb     == JALR r14,rb
RET          == JALR r0,r14
```

`JALR` captures the target from the **old** value of `rb` before any link write, so `rd == rb` is well-defined.

Frozen semantics:

```text
target = old_rb

if target bit 0 != 0:
    instruction-alignment fault
    no link write
    no control transfer
else:
    rd = address of following instruction
    PC = target
```

SIA does **not** silently clear target bit 0. There is no instruction-set mode bit encoded in addresses; an odd function pointer is an error.

---

# 16. Bit operations

The frozen base assignments in primary `0xC` include:

```text
BSET rd,rs    set bit rd[rs[4:0]]
BCLR rd,rs    clear bit rd[rs[4:0]]
BINV rd,rs    invert bit rd[rs[4:0]]
BEXT rd,rs    rd = selected old-rd bit as 0 or 1
REV8 rd,rs    rd = byte_reverse(rs)
```

Bit indices use the low five bits.

Full immediate bitfield extract/insert is not mandatory in the base.

---

# 17. TRAP, BREAK, and NOP

Primary `0xC`, function `0xF`, uses bits `11:4` as an 8-bit operation value:

```text
0x00..0xFD   TRAP imm8
0xFE         BREAK
0xFF         NOP
```

These encodings are frozen.

Under SIA32-P:

- `TRAP` is an architectural synchronous trap to Supervisor mode;
- `BREAK` raises the breakpoint exception;
- `NOP` has no architectural effect beyond normal retirement.

An emulator may provide explicit debug/semihosting interception, but that is not SIA architecture.

---

# 18. Optional integer multiply/divide extensions

The mandatory SIA32-I VM baseline does **not** require multiply or divide.

The current executable reference retains experimental `SIA-Zmul` / `SIA-M` encodings in the `0xC` subspace:

```text
SIA-Zmul
    MUL
    MULH
    MULHU
    MULHSU
    MULO

SIA-M
    all SIA-Zmul operations
    DIV
    DIVU
    REM
    REMU
```

These optional extension semantics are **outside this baseline freeze** and may be separately frozen after compiler validation.

Unsupported extension instructions raise illegal instruction.

---

# 19. Relationship to SIA32-P and other specifications

SIA32-I contains no privileged architectural register state.

Protected implementations add:

```text
SIA32-P       U/S privilege, trap state, SYSTEM instructions
SIA32-MMU     virtual memory / ASIDs / translation fences
SIA32-MEM     memory ordering / FENCE / SYNC.I
SIA32-A       optional atomics and multiprocessing
SIA Platform  vectors, interrupt controller, timer, physical machine
```

Primary `0xF` is illegal in bare SIA32-I and becomes the SYSTEM/extension namespace when SIA32-P or another compatible extension is implemented.

---

# 20. Frozen SIA32-I VM baseline boundary

The 2026-09-10 VM freeze includes:

```text
32-bit architectural integer model
16 registers and r0 zero semantics
fixed 16-bit instruction width
primary opcode allocation
ADC/SBB primary assignments and rd!=rc rule
CLZ/CTZ/CPOP destination-zero escapes
arithmetic/compare function allocation
logic/shift function allocation and all immediate shifts
absence of source-negation forms
LI/ADDI signed 7-bit range
scalar memory mode allocation
pair/quad mode allocation and fault atomicity
LDPC.W PC base, scale, and range
BNZ/DBNZ and B/BL encoding rules
absence of BZ
JALR/JR/CALLR/RET including odd-target fault
BSET/BCLR/BINV/BEXT/REV8 assignments
TRAP/BREAK/NOP encodings
reserved base encodings
```

Not frozen by this milestone:

```text
final SIA-Zmul/SIA-M extension semantics
ABI details beyond architectural register identities
object/relocation format
SIA32-P binary freeze
future long-extension payload formats
future compiler-driven successor ISA baseline
```

---

# 21. Conformance requirement

The baseline was accepted after the Rust reference implementation established the canonical validation path:

```text
SIA assembly source
    -> canonical siaasm
    -> exact 16-bit machine encoding
    -> interpreter
    -> expected architectural result/fault
```

The suite includes exact golden encodings, direct semantic tests, realistic short programs, invalid encodings, precise failure cases, all-or-nothing memory faults, and deterministic randomized differential testing.

SIA32-P implementation may now begin against this baseline.
