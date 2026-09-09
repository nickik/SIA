# SIA32-P Encoding Proposal

## Status

- Purpose: proposed binary encoding for `SIA32-P` and closely related system/memory operations
- Semantic specification: [`SIA32-P.md`](SIA32-P.md)
- Memory model: [`SIA32-MEM.md`](SIA32-MEM.md)
- Status: **proposal for the v1 encoding freeze**

This document answers one narrow question: **how do we encode all operations needed by the privileged architecture without disturbing the dense SIA32-I user-level instruction set?**

The proposed answer is to use the already-reserved primary opcode `0xF` as a compact **SYSTEM/EXT** primary.

The existing ordinary user-level map remains unchanged.

---

# 1. Existing opcode pressure

The current provisional SIA32-I primary map is:

```text
0x0   ADD three-register
0x1   CMOV three-register
0x2   scaled indexed word load
0x3   scaled indexed word store
0x4   arithmetic / compare
0x5   logic / shift
0x6   small immediates
0x7   scalar memory
0x8   pair / quad memory
0x9   PC-relative literal load
0xA   conditional / counted branches
0xB   direct branch / call
0xC   misc / indirect / bit / system
0xD   reserved base growth; interpreter currently experiments with ADC
0xE   reserved base growth; interpreter currently experiments with SBB
0xF   reserved future EXT
```

The current `0xC` system subspace already carries the user-visible:

```text
TRAP imm8
BREAK
NOP
```

`TRAP imm8` intentionally consumes the compact immediate system-call namespace, so adding privilege register operands there would create unnecessary conflicts or force awkward special cases.

The privileged architecture should therefore **not** consume more of primary `0xC`.

---

# 2. Recommended top-level decision

For the v1 encoding freeze, use:

```text
0xD   ADC rd,rs,rc
0xE   SBB rd,rs,rc
0xF   SYSTEM / standard-extension primary
```

This resolves the current carry/borrow pressure cleanly and leaves all existing `0x0..0xC` user instructions intact.

The important property is:

> No currently defined SIA32-I user-level instruction needs to move in order to add `SIA32-P`.

A `SIA32-I` implementation that does not implement the relevant system extensions continues to treat unsupported `0xFxxx` instructions as illegal instructions.

---

# 3. Compact 16-bit SYSTEM format

The recommended 16-bit format is:

```text
15          12 11           8 7            4 3            0
+-------------+---------------+--------------+--------------+
|    0xF      |     SYSOP     |      R       |      X       |
+-------------+---------------+--------------+--------------+
    4 bits          4 bits          4 bits         4 bits
```

Interpretation depends on `SYSOP`.

This provides:

- 15 useful 16-bit system-operation groups before the long-extension escape;
- one GPR field when required;
- one system-register/suboperation field when required;
- no impact on ordinary user encodings;
- fixed 16-bit encodings for every operation on the critical trap/context-switch path.

---

# 4. Proposed SYSTEM opcode allocation

```text
SYSOP    Meaning
-----    -----------------------------------------------------
0x0      SREAD   R, X
0x1      SWRITE  X, R
0x2      SSWAP   R, X
0x3      SRET / return-control group
0x4      TLBFENCE group
0x5      WFI
0x6      SYNC.I
0x7      FENCE
0x8      reserved standard system/cache operation
0x9      reserved standard system/cache operation
0xA      reserved standard system/cache operation
0xB      reserved standard system/cache operation
0xC      reserved standard system/cache operation
0xD      reserved standard system/cache operation
0xE      reserved standard system/cache operation
0xF      LONG-EXT escape
```

Thus only eight of the sixteen `SYSOP` groups are consumed initially.

Half of the compact system space remains reserved.

---

# 5. System-register operations

## 5.1 `SREAD`

Format:

```text
F 0 rd sr
```

Semantics:

```text
rd = system_register[sr]
```

Example:

```asm
SREAD r4, EPC
```

## 5.2 `SWRITE`

Format:

```text
F 1 rs sr
```

Semantics:

```text
system_register[sr] = rs
```

Example:

```asm
SWRITE EPC, r4
```

## 5.3 `SSWAP`

Format:

```text
F 2 rg sr
```

Semantics:

```text
t = rg
rg = system_register[sr]
system_register[sr] = t
```

For the baseline `SIA32-P` profile, `SSWAP` is valid only for `SCRATCH`.

This deliberately avoids defining strange exchange semantics for system registers with side effects such as `STATUS`, `VMROOT`, or interrupt state.

The important trap-entry operation is therefore:

```asm
SSWAP sp, SCRATCH
```

Future extensions may designate additional swap-safe scratch registers if needed.

---

# 6. Proposed system-register numbers

Four bits provide sixteen compact system-register identifiers.

The baseline currently needs only ten:

```text
ID    Register     Access
--    --------     ------------------------------------
0x0   STATUS       S read/write
0x1   TVEC         S read/write
0x2   EPC          S read/write
0x3   CAUSE        S read-only; written by trap hardware
0x4   BADADDR      S read-only; written by trap hardware
0x5   SCRATCH      S read/write; SSWAP permitted
0x6   VMROOT       S read/write
0x7   ASID         S read/write
0x8   IENABLE      S read/write
0x9   IPENDING     S read; only architecturally writable bits may be changed
0xA   reserved
0xB   reserved
0xC   reserved
0xD   reserved
0xE   reserved
0xF   reserved
```

This leaves six identifiers for modest future privileged growth.

Performance counters, hardware debug, virtualization, and SMP control should **not** automatically consume this small baseline namespace. They can receive separate extension encodings if they are eventually standardized.

This keeps the fundamental privileged state deliberately small.

---

# 7. Return and wait operations

## 7.1 `SRET`

Format:

```text
F 3 0 0
```

All other `F3rx` encodings are reserved initially.

`SRET` is privileged and has the semantics defined by `SIA32-P.md`.

## 7.2 `WFI`

Format:

```text
F 5 0 0
```

All other `F5rx` encodings are reserved initially.

`WFI` remains a privileged wait/performance hint and may legally behave as a `NOP`.

---

# 8. Translation-fence encoding

Use one compact group rather than three full SYSOP values:

```text
F 4 rs mode
```

Proposed modes:

```text
mode    Operation             Register field
----    ------------------    -----------------------------
0x0     TLBFENCE              rs must be r0
0x1     TLBFENCE.VA           rs = virtual address
0x2     TLBFENCE.ASID         rs = ASID value
0x3     reserved for VA+ASID  future
0x4-F   reserved
```

Examples:

```asm
TLBFENCE
TLBFENCE.VA   r4
TLBFENCE.ASID r5
```

This leaves room for a combined virtual-address + ASID operation later without consuming another top-level system opcode.

---

# 9. `SYNC.I`

Format:

```text
F 6 0 0
```

`SYNC.I` is **unprivileged** even though it uses the SYSTEM primary.

Privilege is determined by the specific operation, not merely by primary opcode `0xF`.

This is important because:

- JIT compilers;
- dynamic-language runtimes;
- loaders;
- debuggers;
- runtime-generated trampolines

may need to synchronize generated code without a kernel transition on a single CPU.

The semantics are defined in `SIA32-MEM.md`.

---

# 10. `FENCE`

Format:

```text
F 7 0 0
```

`FENCE` is unprivileged.

It is the one full data-memory barrier defined by the strong SIA memory model.

A simple in-order CPU may implement it as a no-op while preserving the same binary for later buffered/cache-coherent implementations.

The semantics are defined in `SIA32-MEM.md`.

---

# 11. Long-extension escape

The compact system allocation should preserve a future long-instruction escape.

Recommended prefix:

```text
first halfword

15          12 11           8 7                         0
+-------------+---------------+---------------------------+
|    0xF      |     0xF       |      extension class      |
+-------------+---------------+---------------------------+

second halfword

15                                                       0
+---------------------------------------------------------+
|                extension-specific payload               |
+---------------------------------------------------------+
```

In shorthand:

```text
FFcc xxxx
```

where:

- `cc` is an 8-bit long-extension class;
- `xxxx` is a 16-bit continuation payload.

A CPU that does not implement the named long extension raises `ILLEGAL_INSTRUCTION` at the prefix instruction.

This means allocating compact `0xF0..0xFE` system operations does **not** eliminate the architecture's future longer-instruction escape.

It simply makes the escape more selective.

This is preferable to spending every `0xFxxx` encoding as a generic length prefix when the privileged architecture needs only a handful of compact operations.

---

# 12. Why not encode privilege in primary `0xC`

The existing primary `0xC` is already valuable user-level space:

- `JALR`;
- bit operations;
- optional multiply/divide prototypes;
- `TRAP imm8`;
- `BREAK`;
- `NOP`.

In particular, the current compact system form uses the eight payload bits for `TRAP imm8`.

Trying to insert `SREAD rd,sr`, `SWRITE sr,rs`, and `SSWAP` there would require one of:

- reducing the trap-immediate namespace;
- overloading special GPR combinations;
- stealing existing misc operations;
- introducing inconsistent instruction formats.

None is necessary while primary `0xF` remains unused.

---

# 13. Why not spend `0xD` and `0xE` on privilege

`ADC` and `SBB` are unusual SIA operations because each needs three unrestricted GPR fields:

```text
rd
rs
carry/borrow register
```

That naturally consumes an entire primary opcode each in a 16-bit three-register format.

The current interpreter already uses `0xD` and `0xE` experimentally for exactly this reason.

Privileged operations are structurally different and fit efficiently inside the `0xF` system format.

Recommended division therefore remains:

```text
0xD   ADC
0xE   SBB
0xF   SYSTEM / EXT
```

This is cleaner than making privilege compete with three-register arithmetic.

---

# 14. Privilege check behavior

The following operations are privileged:

```text
SREAD
SWRITE
SSWAP
SRET
TLBFENCE*
WFI
```

Attempting one in User mode raises:

```text
PRIVILEGE
```

The following are unprivileged:

```text
SYNC.I
FENCE
```

An invalid system-register number, reserved SYSOP, invalid submode, illegal write to a read-only system register, or invalid reserved-field combination executed in Supervisor mode raises:

```text
ILLEGAL_INSTRUCTION
```

This distinction keeps "you are not privileged" separate from "this instruction encoding is not defined".

---

# 15. Reserved-field rule

For v1, reserved operand fields in defined no-operand system instructions must be zero.

For example:

```text
F300    SRET
F500    WFI
F600    SYNC.I
F700    FENCE
```

Other encodings in those groups remain reserved rather than being silently ignored.

This preserves room for later compatible variants.

---

# 16. Complete baseline privileged instruction inventory

The proposed architecture does **not** need additional privileged opcodes for the first Cosmic system.

Required kernel mechanisms are covered by:

```text
kernel entry            existing TRAP imm8
trap/interrupt entry    hardware trap mechanism
system state read       SREAD
system state write      SWRITE
safe stack exchange     SSWAP SCRATCH
trap return             SRET
translation sync        TLBFENCE*
idle                    WFI
instruction sync        SYNC.I   (unprivileged)
memory barrier          FENCE    (unprivileged)
```

Not required:

```text
separate IRQ return instruction
separate syscall instruction
interrupt-enable/disable opcodes
physical load/store opcodes
cache clean/invalidate opcodes
banked-register instructions
page-table-map/unmap instructions
capability instructions
process/thread instructions
IPI instruction
special device I/O instructions
```

Interrupt masks are ordinary privileged register state.

Page tables are ordinary memory structures.

Devices are MMIO.

Capabilities remain a Cosmic software abstraction.

This is the intended minimality boundary.

---

# 17. Recommended small cleanup to privileged state

Before v1, the system-register access permissions should be tightened to:

```text
STATUS      RW
TVEC        RW
EPC         RW
CAUSE       RO to software
BADADDR     RO to software
SCRATCH     RW
VMROOT      RW
ASID        RW
IENABLE     RW
IPENDING    RO except explicitly defined software-pending bits
```

`EPC` must remain writable because the kernel needs to advance or replace the return PC.

There is no clear baseline need for software to write `CAUSE` or `BADADDR`.

Keeping them read-only reduces accidental hidden semantics.

---

# 18. Decoder impact

A protected SIA CPU needs only one new top-level test:

```text
if primary != 0xF:
    existing SIA32-I decode
else:
    decode SYSTEM/EXT
```

The common privileged operations remain single 16-bit instructions.

Only the reserved `SYSOP=0xF` path requires a later implementation to consider a continuation halfword.

A first Lighting CPU that implements no long extensions can simply raise `ILLEGAL_INSTRUCTION` for `0xFFxx`.

Thus the privilege architecture does **not** force variable-length decode into the first CPU.

---

# 19. Compatibility with the current interpreter

The current interpreter already treats primary `0xF` as illegal/reserved.

Therefore adding `SIA32-P` decoding there is straightforward:

```text
SIA32-I-only mode:
    Fxxx -> illegal instruction

SIA32-I + P/MEM mode:
    F0xx..F7xx -> decode supported system operation
    unsupported Fxxx -> illegal instruction
```

The existing `TRAP`, `BREAK`, and `NOP` encoding need not change.

The current experimental `ADC=0xD` and `SBB=0xE` assignments remain compatible with this proposal.

---

# 20. Proposed v1 system encoding summary

```text
primary D
    ADC rd,rs,rc

primary E
    SBB rd,rs,rc

primary F
    F0rs    SREAD
    F1rs    SWRITE
    F2rs    SSWAP              baseline: SCRATCH only
    F300    SRET
    F4r0    TLBFENCE           r must be 0
    F4r1    TLBFENCE.VA r
    F4r2    TLBFENCE.ASID r
    F500    WFI
    F600    SYNC.I             unprivileged
    F700    FENCE              unprivileged
    F8xx-E  reserved compact standard system space
    FFcc    reserved long-extension prefix/class
```

This is the recommended encoding direction for the final SIA v1 opcode-freeze work.