# SIA32-P Encoding Proposal

## Status

- Purpose: proposed binary encoding for `SIA32-P` and related system/memory operations
- Privilege semantics: [`SIA32-P.md`](SIA32-P.md)
- MMU semantics: [`SIA32-MMU.md`](SIA32-MMU.md)
- Memory model: [`SIA32-MEM.md`](SIA32-MEM.md)
- Status: **proposal for the v1 encoding freeze**

The objective is to encode the complete protected-system architecture without disturbing the dense ordinary SIA32-I user instruction set.

The recommended approach uses the already-reserved primary opcode `0xF` as a compact **SYSTEM/EXT** primary.

---

# 1. Top-level primary allocation

Recommended v1 direction:

```text
0x0..0xC   existing ordinary SIA32-I groups
0xD        ADC rd,rs,rc
0xE        SBB rd,rs,rc
0xF        SYSTEM / standard-extension primary
```

`TRAP`, `BREAK`, and `NOP` remain in the existing `0xC` misc/system subspace.

No ordinary user opcode needs to move.

---

# 2. Compact SYSTEM format

```text
15          12 11           8 7            4 3            0
+-------------+---------------+--------------+--------------+
|    0xF      |     SYSOP     |      R       |      X       |
+-------------+---------------+--------------+--------------+
    4 bits          4 bits          4 bits         4 bits
```

Interpretation of `R` and `X` depends on `SYSOP`.

---

# 3. SYSTEM opcode allocation

```text
SYSOP    Meaning
-----    ----------------------------------------------
0x0      SREAD
0x1      SWRITE
0x2      SSWAP
0x3      supervisor return-control group
0x4      TLBFENCE group
0x5      WFI
0x6      SYNC.I
0x7      FENCE
0x8-E    reserved
0xF      LONG-EXT escape
```

Only half of the compact SYSTEM groups are consumed initially.

---

# 4. System-register operations

## 4.1 `SREAD`

```text
F 0 rd sr
```

```text
rd = system_register[sr]
```

## 4.2 `SWRITE`

```text
F 1 rs sr
```

```text
system_register[sr] = rs
```

## 4.3 `SSWAP`

```text
F 2 rg sr
```

```text
t = rg
rg = system_register[sr]
system_register[sr] = t
```

Baseline `SIA32-P` permits `SSWAP` only for `SCRATCH`.

---

# 5. Simplified system-register namespace

The entire baseline privileged register set is six 32-bit registers:

```text
ID    Register     Access
--    --------     -----------------------------------------
0x0   STATUS       S read/write
0x1   EPC          S read/write
0x2   CAUSE        S read-only; hardware trap state
0x3   BADADDR      S read-only; hardware fault state
0x4   SCRATCH      S read/write; SSWAP permitted
0x5   VMCTX        S read/write; root + 12-bit ASID
0x6   reserved
0x7   reserved
0x8   reserved
0x9   reserved
0xA   reserved
0xB   reserved
0xC   reserved
0xD   reserved
0xE   reserved
0xF   reserved
```

There is no system-register ID for:

```text
TVEC
VMROOT
ASID
IENABLE
IPENDING
```

The trap vector is fixed by the platform profile.

Interrupt pending, masking, priority, claim, and complete state is MMIO state in the platform interrupt controller.

`VMROOT` and `ASID` are fields of `VMCTX` only.

---

# 6. `VMCTX`

```text
31                    12 11                               0
+-----------------------+----------------------------------+
| root physical >> 12   |              ASID                |
|       20 bits         |             12 bits              |
+-----------------------+----------------------------------+
```

Writing `VMCTX` installs both fields together and never flushes cached translations merely because the active context changes.

---

# 7. Return-control group

## 7.1 `SRET`

```text
F 3 0 0
```

Returns using the current `VMCTX`.

## 7.2 `SRETCTX rs`

```text
F 3 rs 1
```

Semantics:

```text
VMCTX = rs
restore previous privilege/interrupt state
PC = EPC
```

The operation:

- installs new root + ASID together;
- never flushes the TLB merely because the context changes;
- preserves global translations;
- serializes subsequent translation under the new context;
- does not perform a data `FENCE`;
- does not imply `SYNC.I`;
- does not pre-walk or pre-validate `EPC`.

All other `F3rx` encodings remain reserved.

---

# 8. Translation-fence group

```text
F 4 rs mode
```

```text
mode    Operation               Register field
----    --------------------    -----------------------------
0x0     TLBFENCE                rs must be r0
0x1     TLBFENCE.VA             rs = virtual address
0x2     TLBFENCE.ASID           rs[11:0] = ASID
0x3     reserved VA+ASID form   future
0x4-F   reserved
```

`TLBFENCE*` is used for mapping changes and ASID recycling, not ordinary `VMCTX` switches.

---

# 9. Other system operations

```text
F500    WFI        privileged wait/platform-interrupt hint
F600    SYNC.I     unprivileged instruction-fetch synchronization
F700    FENCE      unprivileged full data-memory barrier
```

`WFI` waits on the platform interrupt condition conceptually; it has no dependency on CPU-resident pending registers because none exist.

---

# 10. Long-extension escape

Reserve:

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

Shorthand:

```text
FFcc xxxx
```

A baseline CPU with no long extensions raises `ILLEGAL_INSTRUCTION` on this prefix.

---

# 11. Privilege behavior

Privileged operations:

```text
SREAD
SWRITE
SSWAP
SRET
SRETCTX
TLBFENCE*
WFI
```

Executing one in User mode raises `PRIVILEGE`.

Unprivileged SYSTEM-space operations:

```text
SYNC.I
FENCE
```

Reserved/invalid encodings executed in Supervisor mode raise `ILLEGAL_INSTRUCTION`.

Writes to `CAUSE`, `BADADDR`, or reserved system-register IDs are illegal instructions.

---

# 12. Reserved-field rule

Unused operand fields in defined no-operand instructions must be zero.

```text
F300    SRET
F500    WFI
F600    SYNC.I
F700    FENCE
```

Other encodings remain reserved for compatible growth.

---

# 13. Baseline privileged instruction inventory

The first Cosmic-capable CPU needs only:

```text
kernel entry             TRAP imm8
system-state read        SREAD
system-state write       SWRITE
safe stack exchange      SSWAP SCRATCH
return                    SRET
fast context return       SRETCTX
translation sync          TLBFENCE*
idle                      WFI
instruction sync          SYNC.I    unprivileged
data-memory barrier       FENCE     unprivileged
```

Not required:

```text
trap-vector register
CPU interrupt pending register
CPU interrupt mask register
separate IRQ-return instruction
separate syscall instruction
physical load/store opcodes
cache clean/invalidate opcodes
banked-register operations
page-map/unmap instructions
capability instructions
process/thread instructions
special device I/O instructions
```

---

# 14. Proposed v1 system encoding summary

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
    F3r1    SRETCTX r
    F4r0    TLBFENCE           r must be 0
    F4r1    TLBFENCE.VA r
    F4r2    TLBFENCE.ASID r
    F500    WFI
    F600    SYNC.I             unprivileged
    F700    FENCE              unprivileged
    F8xx-FExx reserved
    FFcc    long-extension prefix/class
```

This is the recommended direction for the final SIA v1 opcode freeze.