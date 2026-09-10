# SIA32-P Encoding

## Status

- Target base: frozen SIA32-I VM baseline
- Semantics: [`SIA32-P.md`](SIA32-P.md)
- MMU semantics: [`SIA32-MMU.md`](SIA32-MMU.md)
- Memory model: [`SIA32-MEM.md`](SIA32-MEM.md)
- Status: **implementation baseline; binary freeze follows executable P conformance**

SIA32-I freezes primary `0xD` as ADC and `0xE` as SBB. Bare SIA32-I treats primary `0xF` as illegal. SIA32-P defines the compatible SYSTEM namespace inside `0xF`.

---

# 1. Top-level allocation

```text
0x0..0xC   frozen SIA32-I groups
0xD        ADC rd,rs,rc
0xE        SBB rd,rs,rc
0xF        SYSTEM / standard-extension primary when SIA32-P is present
```

`TRAP`, `BREAK`, and `NOP` remain in the frozen `0xC` misc/system subspace.

---

# 2. Compact SYSTEM format

```text
15          12 11           8 7            4 3            0
+-------------+---------------+--------------+--------------+
|    0xF      |     SYSOP     |      R       |      X       |
+-------------+---------------+--------------+--------------+
    4 bits          4 bits          4 bits         4 bits
```

`R` and `X` are interpreted by SYSOP.

---

# 3. SYSOP allocation

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

---

# 4. System-register namespace

```text
ID    Register     Access
--    --------     -----------------------------------------
0x0   STATUS       S read/write
0x1   EPC          S read/write
0x2   CAUSE        S read-only; hardware trap state
0x3   BADADDR      S read-only; hardware fault address
0x4   SCRATCH      S read/write; SSWAP permitted
0x5   VMCTX        S read/write; root + 12-bit ASID
0x6-F reserved
```

There is no selector for TVEC, VMROOT, ASID, IENABLE, or IPENDING.

---

# 5. SREAD

```text
F 0 rd sr
```

```text
rd = system_register[sr]
```

A reserved selector is illegal in Supervisor mode.

---

# 6. SWRITE

```text
F 1 rs sr
```

```text
system_register[sr] = old rs
```

Legal destinations:

```text
STATUS
EPC
SCRATCH
VMCTX
```

Writes to CAUSE, BADADDR, or reserved selectors are illegal in Supervisor mode.

STATUS reserved bits are ignored and read zero as defined by SIA32-P.

A STATUS or VMCTX change becomes architectural at instruction retirement; subsequent fetch/translation observes the new state.

---

# 7. SSWAP

```text
F 2 rg sr
```

```text
t = old rg
rg = system_register[sr]
system_register[sr] = t
```

Baseline SIA32-P permits only:

```text
sr = SCRATCH (0x4)
```

All other SSWAP selectors are illegal in Supervisor mode.

---

# 8. Return-control group

## 8.1 SRET

```text
F 3 0 0
```

Before any return-state change, old EPC must be 2-byte aligned. Odd EPC raises INSTRUCTION_ALIGNMENT as specified by SIA32-P.

## 8.2 SRETCTX

```text
F 3 rs 1
```

`rs` supplies the complete new VMCTX value. Its GPR value is captured before any architectural state change.

Old EPC must be 2-byte aligned before VMCTX or return state changes.

All other `F3rx` encodings are reserved.

---

# 9. Translation-fence group

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

For `TLBFENCE`, nonzero `rs` is a reserved encoding rather than an ignored field.

`TLBFENCE*` synchronizes mapping changes/ASID reuse; it is not part of ordinary VMCTX switching.

---

# 10. WFI / SYNC.I / FENCE

Exactly defined no-operand encodings:

```text
F500    WFI        privileged
F600    SYNC.I     unprivileged
F700    FENCE      unprivileged
```

Unused operand fields must be zero. Other encodings in those SYSOP groups are reserved.

`WFI` does not enable interrupts and depends only on the platform interrupt condition, not CPU-resident pending registers.

---

# 11. Long-extension escape

Reserve SYSOP `0xF`:

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

No long extension is defined by baseline SIA32-P. Executing `FFxx` therefore raises ILLEGAL_INSTRUCTION until a separately specified extension assigns that class.

---

# 12. Privilege/decode priority

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

Unprivileged operations:

```text
SYNC.I
FENCE
```

Decode rules:

1. Determine whether the 16-bit word is a structurally defined instruction encoding.
2. A structurally reserved encoding raises ILLEGAL_INSTRUCTION in either mode.
3. For a structurally defined privileged instruction, User mode raises PRIVILEGE.
4. In Supervisor mode, selector/access restrictions are then checked; violations raise ILLEGAL_INSTRUCTION.

This gives deterministic behavior for cases such as a valid SREAD form using a reserved system-register selector.

---

# 13. Exact summary

```text
primary D
    Ddrs    ADC rd,rs,rc     ; three 4-bit register fields

primary E
    Edrs    SBB rd,rs,rc

primary F
    F0ds    SREAD  rd,sr
    F1ss    SWRITE sr,rs
    F2gs    SSWAP  rg,sr      ; baseline sr=SCRATCH only

    F300    SRET
    F3r1    SRETCTX r

    F400    TLBFENCE
    F4r1    TLBFENCE.VA r
    F4r2    TLBFENCE.ASID r

    F500    WFI
    F600    SYNC.I
    F700    FENCE

    F8xx-FExx reserved
    FFxx    long-extension escape, currently unassigned
```

The shorthand letters above are descriptive, not additional encoding fields; the authoritative bit positions are the compact SYSTEM format in section 2.

---

# 14. Binary-freeze gate

The SIA32-P encoding should be declared frozen only after the Rust VM executes assembly-driven conformance tests for:

```text
privilege faults
all six system-register selectors
reserved selectors
read-only register writes
SSWAP restriction
TRAP/BREAK trap entry
SRET/SRETCTX including odd EPC
STATUS/VMCTX serialization
reserved F encodings
FENCE/SYNC.I User execution
WFI privilege behavior
TLBFENCE forms once MMU exists
```

Until then, this document is the implementation baseline and should not be changed casually, but SIA32-P is not yet labeled an immutable binary freeze.
