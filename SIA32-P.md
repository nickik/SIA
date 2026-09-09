# SIA32-P — Privileged Architecture

## Status

- Extension: **SIA32-P**
- Target base: `SIA32-I`
- Purpose: protected operating systems, traps, interrupts, system control, and MMU control
- Required by Lighting/Cosmic: **YES**
- Initial target: single-processor Lighting system
- MMU semantics: [`SIA32-MMU.md`](SIA32-MMU.md)
- Memory ordering: [`SIA32-MEM.md`](SIA32-MEM.md)
- Platform contract: [`SIA-PLATFORM.md`](SIA-PLATFORM.md)

`SIA32-P` defines the minimum privileged CPU state needed by Cosmic.

The design deliberately keeps platform facilities out of the CPU. In particular, interrupt pending bits, interrupt masks, source priorities, timer state, software-interrupt state, and device-interrupt identification belong to the **SIA Platform interrupt controller**, not to privileged CPU registers.

The core privilege model is:

```text
User mode
    |
    | trap / exception / platform interrupt
    v
Supervisor mode
    |
    | SRET or SRETCTX
    v
User mode
```

There are exactly two architectural privilege levels:

```text
U   User
S   Supervisor
```

There is no separate machine, hypervisor, IRQ, abort, undefined-instruction, or firmware privilege mode in the baseline.

---

# 1. Design goals

`SIA32-P` is designed for:

- a small capability microkernel;
- strict User/Supervisor isolation;
- precise exceptions;
- very fast system calls and IPC;
- asynchronous platform interrupts;
- timer-driven preemption through the platform interrupt controller;
- user-level system services and drivers;
- protected virtual memory through `SIA32-MMU`;
- protected DMA through platform facilities such as PLIO;
- deterministic implementation in the Rust full-system VM;
- straightforward FPGA/custom implementation;
- future SMP use with `SIA32-A` plus platform SMP facilities.

A major performance objective is that an IPC handoff between two address spaces whose translations are already cached requires **no TLB flush**.

The privileged CPU state should remain small, explicit, and free of hidden banked integer registers.

## 1.1 Non-goals

The baseline does not define:

- processes or thread objects;
- capabilities or IPC endpoint objects;
- scheduler policy;
- filesystem or POSIX abstractions;
- device classes or DMA descriptor formats;
- interrupt-controller source tables;
- timer registers;
- hardware virtualization;
- banked general-purpose registers;
- software-filled TLBs as the mandatory paging interface.

---

# 2. Privilege levels

## 2.1 User mode (`U`)

User mode:

- executes ordinary unprivileged SIA instructions;
- accesses only mappings permitted by the current `VMCTX` and PTE permissions;
- enters Supervisor mode with `TRAP`;
- may execute explicitly unprivileged system operations such as `FENCE` and `SYNC.I`;
- may not read or write privileged system registers;
- may not modify translation state;
- may not execute privileged return, TLB-management, or wait operations.

Attempting a privileged operation in User mode raises `PRIVILEGE`.

## 2.2 Supervisor mode (`S`)

Supervisor mode is the highest baseline privilege level.

It may:

- read/write permitted privileged system registers;
- install an MMU context;
- change page tables in ordinary memory;
- synchronize cached translations;
- map physical memory and MMIO through page tables;
- globally enable/disable asynchronous interrupt acceptance through `STATUS.IE`;
- access the platform interrupt controller through MMIO;
- return to User or Supervisor execution.

Supervisor mode does not automatically bypass page `R/W/X` permissions while translation is enabled.

---

# 3. Reset state

On architectural reset:

```text
mode            = S
STATUS.IE       = 0
STATUS.VM       = 0
PC              = platform RESET_VECTOR
```

Translation is initially disabled, so firmware begins with physical addressing.

The SIA Platform Specification defines:

- `RESET_VECTOR`;
- `TRAP_VECTOR`;
- ROM and RAM layout;
- MMIO layout;
- interrupt controller;
- monotonic timer;
- device discovery and platform identification.

---

# 4. Privileged system state

The complete baseline privileged register set is only **six 32-bit registers**:

```text
ID    Register     Access      Purpose
--    --------     ----------  ---------------------------------------------
0x0   STATUS       S read/write execution, previous-state and VM control
0x1   EPC          S read/write saved exception/interrupt return PC
0x2   CAUSE        S read-only  trap/interrupt cause written by hardware
0x3   BADADDR      S read-only  faulting address where applicable
0x4   SCRATCH      S read/write software trap scratch; SSWAP permitted
0x5   VMCTX        S read/write page-table root + 12-bit ASID
0x6-F reserved
```

All six are ordinary 32-bit architectural registers. The 4-bit `0x0..0xF` values are only identifiers used by `SREAD`, `SWRITE`, and `SSWAP` encodings.

There is deliberately no architectural:

```text
TVEC
VMROOT
ASID
IENABLE
IPENDING
```

`TVEC` is replaced by the platform-defined fixed `TRAP_VECTOR`.

`VMROOT` and `ASID` are fields of `VMCTX` and cannot be independently installed.

Interrupt pending/mask/source state belongs to the platform interrupt controller.

---

# 5. System-register instructions

SIA32-P defines:

```asm
SREAD  rd, sr          ; rd = system_register[sr]
SWRITE sr, rs          ; system_register[sr] = rs
SSWAP  rd, sr          ; exchange GPR and swap-safe system register

SRET                   ; return using current VMCTX
SRETCTX rs             ; install VMCTX from rs and return

TLBFENCE               ; synchronize/invalidate all required local translations
TLBFENCE.VA rs          ; synchronize translation for virtual address
TLBFENCE.ASID rs        ; synchronize non-global translations for ASID

WFI                    ; wait-for-platform-interrupt hint
```

`FENCE` and `SYNC.I` use SYSTEM encoding space but are unprivileged and specified by `SIA32-MEM`.

## 5.1 Register access

```text
STATUS      read/write
EPC         read/write
CAUSE       read-only
BADADDR     read-only
SCRATCH     read/write
VMCTX       read/write
```

Writing a read-only or reserved system-register ID in Supervisor mode raises `ILLEGAL_INSTRUCTION`.

## 5.2 `SSWAP`

The baseline permits `SSWAP` only with `SCRATCH`.

The intended trap-stack transition is:

```asm
SSWAP sp, SCRATCH
```

When User mode was running:

```text
before:
    sp      = user stack pointer
    SCRATCH = CPU/kernel stack pointer

after:
    sp      = CPU/kernel stack pointer
    SCRATCH = saved user stack pointer
```

This provides the useful part of banked exception-stack state with one explicit word rather than hidden banked GPRs.

---

# 6. `STATUS`

`STATUS` is 32 bits:

```text
bit 0      IE       global asynchronous-interrupt enable
bit 1      PIE      saved previous IE
bit 2      PP       previous privilege (0=U, 1=S)
bit 3      VM       virtual-memory translation enable
bits 4..31          reserved; read as zero until assigned
```

The current privilege mode is architectural processor state, not a directly writable field.

## 6.1 `IE`

`IE` is the **only CPU-resident interrupt mask**.

When:

```text
STATUS.IE = 1
AND
platform interrupt condition = asserted
```

the CPU may take the asynchronous platform interrupt at an architecturally valid interrupt boundary.

When `IE=0`, the CPU does not take maskable asynchronous interrupts. Pending sources remain recorded by the platform interrupt controller; they are not lost merely because CPU interrupt acceptance is disabled.

Synchronous exceptions are unaffected by `IE`.

## 6.2 `PIE` and `PP`

Trap entry automatically records the previous interrupt-enable state and privilege level in `PIE` and `PP`.

There is one architectural hardware save level. A kernel that deliberately allows nested traps must save `EPC`, `CAUSE`, `STATUS`, and other required state before re-enabling `IE`.

## 6.3 `VM`

When `VM=0`, ordinary instruction and data addresses are physical addresses.

When `VM=1`, instruction and data accesses use `VMCTX` under `SIA32-MMU`.

Changing `VMCTX` does **not** flush the TLB.

---

# 7. `VMCTX`

`VMCTX` is the only architectural address-space context register.

```text
31                    12 11                               0
+-----------------------+----------------------------------+
| root physical >> 12   |              ASID                |
|       20 bits         |             12 bits              |
+-----------------------+----------------------------------+
```

Therefore:

```text
root physical address = VMCTX[31:12] << 12
ASID                  = VMCTX[11:0]
```

The root is 4 KiB aligned even though normal SIA pages are 2 KiB.

`VMROOT` and `ASID` are not architectural aliases. Software extracts or constructs the fields in ordinary GPRs.

Writing `VMCTX` installs root + ASID as one translation-context change and **never invalidates cached translations merely because the context changed**.

Full MMU behavior is defined by `SIA32-MMU.md`.

---

# 8. Fixed trap vector

SIA32-P has no writable trap-vector register.

Every platform profile defines one fixed architectural address:

```text
TRAP_VECTOR
```

All synchronous exceptions, `TRAP` instructions, and asynchronous platform interrupts enter at this same address.

Requirements:

- `TRAP_VECTOR` is at least 2-byte aligned;
- when `STATUS.VM=0`, it is interpreted as a physical address;
- when `STATUS.VM=1`, it is interpreted as a virtual address in the current `VMCTX`;
- a protected OS must therefore map the trap entry identically in every active address space, normally as supervisor-only `G=1` executable memory;
- the platform profile must make the reset/firmware arrangement compatible with the fixed address in physical mode.

Typical Cosmic policy:

```text
TRAP_VECTOR
    -> small globally mapped Cosmic trap entry
    -> save/dispatch in software
```

A fixed vector avoids one privileged register and removes a control dependency from trap entry.

---

# 9. Trap entry

A trap may result from:

- a synchronous exception;
- `TRAP imm8`;
- an asynchronous platform interrupt.

Trap entry is precise and performs:

```text
EPC        = saved PC
CAUSE      = trap reason
BADADDR    = fault address where defined
STATUS.PIE = STATUS.IE
STATUS.PP  = previous privilege
STATUS.IE  = 0
mode       = S
PC         = TRAP_VECTOR
```

General-purpose registers and `VMCTX` are unchanged.

Because `VMCTX` remains installed, Cosmic should keep its trap/kernel mapping global (`G=1`) so trap entry never requires an address-space switch merely to execute kernel code.

## 9.1 Saved PC semantics

For a synchronous fault, `EPC` identifies the faulting instruction.

For `TRAP`, `EPC` identifies the `TRAP` instruction. A consumed system call normally advances `EPC` by 2 before return.

For an asynchronous platform interrupt, `EPC` identifies the next instruction that would otherwise execute.

## 9.2 Trap while already in Supervisor mode

Supervisor traps use the same mechanism.

Because there is only one hardware save level, software must save trap state before intentionally enabling nesting.

---

# 10. Trap return

## 10.1 `SRET`

`SRET` performs:

```text
mode       = STATUS.PP
STATUS.IE  = STATUS.PIE
STATUS.PIE = 1
STATUS.PP  = U
PC         = EPC
```

`VMCTX` is unchanged.

The subsequent instruction fetch is normal; `SRET` does not pre-walk or pre-validate the target mapping.

## 10.2 `SRETCTX rs`

`SRETCTX` is the fast address-space-switch-and-return operation for IPC and scheduling.

Conceptually:

```text
VMCTX      = rs
mode       = STATUS.PP
STATUS.IE  = STATUS.PIE
STATUS.PIE = 1
STATUS.PP  = U
PC         = EPC
```

Properties:

- privileged;
- installs the complete 32-bit `VMCTX` atomically;
- never flushes TLB entries merely because the context changes;
- preserves global translations;
- leaves old non-global translations cached under old ASIDs;
- serializes subsequent translation under the new context;
- is not a general data-memory `FENCE`;
- does not imply `SYNC.I`;
- does not pre-walk or pre-validate `EPC`.

Typical IPC tail:

```asm
SSWAP   sp, SCRATCH
SRETCTX r8              ; r8 = receiver VMCTX
```

---

# 11. `CAUSE`

`CAUSE` is 32 bits:

```text
bit 31      INTERRUPT
bits 30..16 reserved
bits 15..8  auxiliary cause data where defined
bits 7..0   cause code
```

Initial synchronous causes:

```text
0x00  ILLEGAL_INSTRUCTION
0x01  PRIVILEGE
0x02  BREAKPOINT
0x03  TRAP
0x04  INSTRUCTION_ALIGNMENT
0x05  INSTRUCTION_PAGE_FAULT
0x06  LOAD_ALIGNMENT
0x07  LOAD_PAGE_FAULT
0x08  STORE_ALIGNMENT
0x09  STORE_PAGE_FAULT
0x0A  INSTRUCTION_ACCESS_FAULT
0x0B  LOAD_ACCESS_FAULT
0x0C  STORE_ACCESS_FAULT
0x0D  ARITHMETIC
```

For `TRAP imm8`:

```text
CAUSE.INTERRUPT = 0
CAUSE[15:8]     = trap immediate
CAUSE[7:0]      = TRAP
```

## 11.1 Asynchronous interrupt cause

The baseline defines only one CPU-level asynchronous cause:

```text
CAUSE.INTERRUPT = 1
CAUSE[7:0]      = 0x01   PLATFORM_INTERRUPT
```

The CPU does **not** encode timer/device/software source identity in `CAUSE`.

Cosmic obtains the actual source by reading the platform interrupt controller's `CLAIM` register.

This avoids duplicating source, pending, mask, and priority state in the CPU.

---

# 12. `BADADDR`

`BADADDR` is written for address-related synchronous faults.

Typical uses:

```text
instruction page/access fault
load page/access fault
store page/access fault
alignment fault where useful
```

For translated CPU accesses it normally records the original architectural virtual address.

For non-address traps or interrupts, its value is not architecturally meaningful and software must ignore it.

---

# 13. Platform interrupt model

The CPU has one conceptual asynchronous interrupt condition from the platform.

All detailed interrupt machinery is outside SIA32-P:

```text
timer deadline
software interrupt / later IPI
PLIO Notification
direct platform device
        |
        v
platform interrupt controller
    pending
    per-source mask
    priority/routing
    claim
    complete
        |
        v
single CPU platform-interrupt condition
        |
        v
SIA32-P trap at TRAP_VECTOR
```

The controller may continue accumulating pending sources while `STATUS.IE=0`.

A normal handler flow is:

```text
CPU takes PLATFORM_INTERRUPT
    -> IE automatically becomes 0
    -> Cosmic saves required trap state
    -> read controller CLAIM
    -> service/dispatch source
    -> write controller COMPLETE
    -> SRET
```

If more eligible sources remain pending, the controller keeps/reasserts the platform interrupt condition and another interrupt may be taken after `IE` is restored.

Nested interrupt policy is software/platform policy, not additional CPU state.

---

# 14. MMU relationship

All virtual-memory semantics are normative in [`SIA32-MMU.md`](SIA32-MMU.md).

Required first Lighting profile:

```text
Address space             32-bit
Normal page               2 KiB
Superpage                 1 MiB
ASID                      12 bits / 4096 values
Page tables               3 levels
VA                         L2=6, L1=6, L0=9, offset=11
L1 leaf                   1 MiB superpage
L0 leaf                   2 KiB page
VMCTX                     root>>12 (20 bits) + ASID (12 bits)
root alignment            4 KiB
SWRITE VMCTX              never flushes TLB
```

Recommended global mappings:

```text
G=1
    Cosmic kernel
    TRAP_VECTOR entry
    universal ROM libraries
    universal ROM constants
    universal system/IPC stubs
```

Recommended ASID-tagged mappings:

```text
G=0
    application code
    heap
    stack
    writable globals
    TLS
    per-process data
    non-global shared-memory mappings
```

SIA32-P does not define separate instruction and data page types. Permissions define use.

---

# 15. Page and physical faults

Translation/protection faults are precise:

```text
EPC     = faulting instruction
BADADDR = faulting virtual address
CAUSE   = corresponding page-fault cause
```

The faulting architectural operation has not completed.

Physical/bus failures after successful translation raise the corresponding `*_ACCESS_FAULT` rather than a page fault.

`LDP/STP/LD4/ST4` use the separate all-or-nothing semantics in `SIA32-MULTI-TRANSFER.md`.

---

# 16. MMIO and DMA

SIA32-P defines no special I/O instructions.

Devices and platform controllers are MMIO.

User software can access an MMIO region only if Supervisor software deliberately maps it.

CPU MMU protection and device DMA authority are separate:

```text
CPU translation/protection    SIA32-P + SIA32-MMU
PLIO DMA authority             platform / PLIO protected DMA
```

For Lighting, PLIO provides bounded protected DMA without requiring a general-purpose IOMMU.

Page-table walks may use only suitable normal coherent RAM, never MMIO.

---

# 17. `WFI`

`WFI` is a privileged power/performance hint.

Architecturally:

- it may stop instruction issue until the platform interrupt condition becomes asserted;
- it may be implemented as a `NOP`;
- it does not enable interrupts;
- it must not cause a pending interrupt-controller source to be lost.

The Rust VM may use `WFI` to advance virtual time to the next platform event.

---

# 18. SMP relationship

The first Lighting profile is single CPU.

A later SMP platform adds:

- per-CPU or routed platform interrupt conditions;
- software-interrupt/IPI sources in the interrupt controller;
- CPU identification/startup facilities;
- coherent memory;
- remote translation-shootdown protocol;
- `SIA32-A` atomics.

These do not add baseline privileged registers.

In particular, an IPI is a platform interrupt-controller source, not an `IPENDING` bit in the CPU.

---

# 19. First Lighting privileged profile

A Cosmic-capable first Lighting CPU implements:

```text
U / S privilege

six privileged 32-bit registers:
    STATUS
    EPC
    CAUSE
    BADADDR
    SCRATCH
    VMCTX

fixed platform TRAP_VECTOR
single platform interrupt condition

SREAD / SWRITE / SSWAP
SRET / SRETCTX
TLBFENCE / TLBFENCE.VA / TLBFENCE.ASID
WFI

SIA32-MMU
SIA32-MEM
```

There are no banked GPRs, CPU interrupt-controller registers, programmable trap-vector register, or separate MMU-root/ASID registers.

---

# 20. Fast Cosmic IPC shape

The intended common cross-address-space IPC path is:

```text
Sender User
    r1-r6 = short message
    TRAP IPC_CALL
        |
        v
fixed globally mapped TRAP_VECTOR
        |
        v
Cosmic Supervisor
    SSWAP sp,SCRATCH
    validate endpoint
    save only required sender persistent state
    load receiver persistent state
    leave/prepare r1-r6 as message registers
    SSWAP sp,SCRATCH
    SRETCTX receiver_vmctx
        |
        v
Receiver User
```

No TLB flush, trap-vector register load, interrupt-mask-register save, or page-table-root/ASID pair update is required on the normal path.

---

# 21. Conformance requirements

The Rust VM and later hardware must test at minimum:

## Privilege

- User privileged instruction faults.
- Supervisor privileged instruction succeeds.
- six-register namespace and reserved IDs behave correctly.
- writes to `CAUSE`/`BADADDR` fault as illegal instructions.

## Trap entry

- every synchronous cause records correct `EPC`;
- `TRAP` records `imm8`;
- `BADADDR` is correct where defined;
- previous mode/IE are recorded;
- `IE` clears on entry;
- PC becomes platform `TRAP_VECTOR`;
- `VMCTX` remains unchanged.

## Platform interrupt

- controller pending source with `IE=0` is retained but not taken;
- setting `IE=1` permits delivery;
- `CAUSE` records `PLATFORM_INTERRUPT` only;
- actual source is obtained from controller claim state;
- complete/retrigger behavior is platform-correct.

## Return

- `SRET` restores previous state using current `VMCTX`;
- `SRETCTX` installs new `VMCTX` without TLB flush;
- neither return form forces a target page-table walk before normal fetch.

## MMU

- 2 KiB pages;
- 1 MiB superpages;
- 12-bit ASIDs;
- global mappings;
- ASID reuse fences;
- exact mapping/fault behavior from `SIA32-MMU`.

---

# 22. Encoding status

Privileged semantics are defined here. Binary encodings are tracked in [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md).

The compact system-register namespace now needs only six identifiers, leaving `0x6..0xF` reserved for future growth.