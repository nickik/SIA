# SIA32-P — Privileged Architecture

## Status

- Extension: **SIA32-P**
- Target base: frozen `SIA32-I` VM baseline
- Purpose: protected operating systems, traps, interrupts, system control, and MMU control
- Required by Lighting/Cosmic: **YES**
- Initial target: single-processor Lighting system
- MMU semantics: [`SIA32-MMU.md`](SIA32-MMU.md)
- Memory ordering: [`SIA32-MEM.md`](SIA32-MEM.md)
- Platform contract: [`SIA-PLATFORM.md`](SIA-PLATFORM.md)
- Binary encoding: [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md)

`SIA32-P` defines the minimum privileged CPU mechanism required by Cosmic. It deliberately excludes kernel policy, platform interrupt-source state, device abstractions, and scheduler objects.

The privilege model is exactly:

```text
User mode (U)
    |
    | synchronous trap / exception
    | asynchronous platform interrupt
    v
Supervisor mode (S)
    |
    | SRET / SRETCTX
    v
previous U or S context
```

There are exactly two architectural privilege levels:

```text
U   User
S   Supervisor
```

There is no machine, hypervisor, IRQ, abort, undefined-instruction, or firmware privilege mode in the baseline.

---

# 1. Design goals

SIA32-P is designed for:

- strict User/Supervisor isolation;
- precise synchronous exceptions;
- very fast trap entry and IPC handoff;
- one simple CPU-level asynchronous interrupt gate;
- protected virtual memory through `SIA32-MMU`;
- user-level drivers and system services;
- deterministic implementation in the Rust VM;
- straightforward implementation from small silicon through larger custom CPUs;
- future SMP without adding baseline privileged-register clutter.

A key performance objective is that switching between two address spaces whose translations are already cached requires **no TLB flush**.

## 1.1 Non-goals

The baseline does not define:

```text
process/thread objects
capability or IPC endpoint objects
scheduler policy
filesystem/POSIX abstractions
device classes or DMA descriptors
interrupt-controller source tables
CPU-resident interrupt pending/mask bitmaps
timer registers
hardware virtualization
banked general-purpose registers
software-filled TLBs as the mandatory paging interface
```

---

# 2. Privilege levels

## 2.1 User mode

User mode may:

- execute ordinary SIA32-I instructions;
- access mappings permitted by the current `VMCTX` and page permissions;
- enter Supervisor mode using architectural `TRAP`;
- execute explicitly unprivileged SYSTEM operations such as `FENCE` and `SYNC.I`.

User mode may not:

- read/write privileged system registers;
- change translation context;
- execute privileged return/TLB/wait operations.

Executing a structurally valid privileged instruction in User mode raises `PRIVILEGE`.

## 2.2 Supervisor mode

Supervisor mode may:

- read/write permitted privileged registers;
- install `VMCTX`;
- change page tables in ordinary memory;
- synchronize translation state;
- globally gate maskable asynchronous interrupt acceptance through `STATUS.IE`;
- access platform controllers through ordinary MMIO mappings;
- return to User or Supervisor execution.

While translation is enabled, Supervisor ignores the PTE `U` bit but still obeys `R/W/X` permissions.

---

# 3. Reset state

Architectural reset is deterministic:

```text
mode        = S
STATUS      = 0
EPC         = 0
CAUSE       = 0
BADADDR     = 0
SCRATCH     = 0
VMCTX       = 0
PC          = platform RESET_VECTOR
```

Therefore initially:

```text
STATUS.IE = 0
STATUS.VM = 0
```

Firmware begins in Supervisor mode with physical addressing.

`RESET_VECTOR`, `TRAP_VECTOR`, ROM/RAM/MMIO layout, the interrupt controller, timer, and device-discovery mechanisms are platform definitions rather than SIA32-P registers.

---

# 4. Privileged architectural state

The complete baseline privileged register set is exactly six 32-bit registers:

```text
ID    Register     Access      Purpose
--    --------     ----------  ---------------------------------------------
0x0   STATUS       S read/write execution, previous-state and VM control
0x1   EPC          S read/write saved exception/interrupt return PC
0x2   CAUSE        S read-only  trap/interrupt cause written by hardware
0x3   BADADDR      S read-only  faulting address written by hardware
0x4   SCRATCH      S read/write software trap scratch; SSWAP target
0x5   VMCTX        S read/write page-table root + 12-bit ASID
0x6-F reserved
```

Current privilege `U/S` is separate architectural processor state, not a field directly writable by software.

There is deliberately no architectural:

```text
TVEC
VMROOT
ASID
IENABLE
IPENDING
```

The trap vector is fixed by the platform profile. Root and ASID are fields of `VMCTX`. Source-specific interrupt state belongs to the platform interrupt controller.

---

# 5. STATUS

```text
bit 0      IE       global asynchronous-interrupt enable
bit 1      PIE      saved previous IE
bit 2      PP       previous privilege: 0=U, 1=S
bit 3      VM       virtual-memory translation enable
bits 4..31          reserved
```

Reserved STATUS bits read as zero. In this baseline, writes to reserved bits are ignored; software should write them as zero for forward compatibility.

Writing STATUS does **not** directly change current privilege.

## 5.1 IE

`IE` is the only CPU-resident mask for maskable asynchronous interrupts.

An asynchronous platform interrupt is eligible when:

```text
STATUS.IE = 1
AND
platform interrupt condition = asserted
```

When `IE=0`, the CPU does not take the interrupt. The CPU does not consume or clear the platform condition; pending/source state remains a platform-controller responsibility.

Synchronous exceptions are unaffected by `IE`.

## 5.2 PIE and PP

Trap entry records old `IE` into `PIE` and old privilege into `PP`.

There is one hardware trap-save level. If Supervisor software intentionally enables nested traps, it must first save `EPC`, `CAUSE`, `BADADDR` when relevant, `STATUS`, and other needed state in memory.

## 5.3 VM

```text
VM=0   instruction/data addresses are physical
VM=1   instruction/data accesses use SIA32-MMU and VMCTX
```

`SWRITE STATUS,rs` executes entirely under the old STATUS. The new value becomes architectural at retirement. Therefore a changed `VM` bit affects the **next** instruction fetch and subsequent accesses, not the instruction performing the write.

A change to `IE` similarly becomes effective at retirement; an already asserted platform interrupt may be taken at the next asynchronous interrupt boundary.

Changing `VM` does not by itself flush translation state.

---

# 6. VMCTX

`VMCTX` is the only architectural address-space context register:

```text
31                    12 11                               0
+-----------------------+----------------------------------+
| root physical >> 12   |              ASID                |
|       20 bits         |             12 bits              |
+-----------------------+----------------------------------+
```

```text
root physical address = VMCTX[31:12] << 12
ASID                  = VMCTX[11:0]
```

The root is 4 KiB aligned.

`SWRITE VMCTX,rs` executes using the old translation context. The new root+ASID becomes architectural together at retirement. The next translated fetch/data access uses the new context.

A VMCTX write:

- never exposes a mixed old-root/new-ASID state;
- never invalidates cached translations merely because the context changed;
- serializes subsequent translation so stale prefetched instructions from the old context may not retire after the switch;
- is not a general data-memory `FENCE`;
- does not imply `SYNC.I`.

Full translation semantics are normative in `SIA32-MMU.md`.

---

# 7. System instructions

SIA32-P defines:

```text
SREAD  rd,sr
SWRITE sr,rs
SSWAP  rg,sr

SRET
SRETCTX rs

TLBFENCE
TLBFENCE.VA rs
TLBFENCE.ASID rs

WFI
```

`FENCE` and `SYNC.I` share SYSTEM encoding space but are unprivileged and are specified by `SIA32-MEM`.

## 7.1 System-register access

```text
STATUS      read/write
EPC         read/write
CAUSE       read-only
BADADDR     read-only
SCRATCH     read/write
VMCTX       read/write
```

Supervisor writes to `CAUSE`, `BADADDR`, or a reserved system-register selector raise `ILLEGAL_INSTRUCTION`.

A read of a reserved system-register selector in Supervisor mode raises `ILLEGAL_INSTRUCTION`.

For a structurally valid privileged instruction, privilege checking occurs before register-access permission checking. Thus the same SREAD/SWRITE form executed in User mode raises `PRIVILEGE`; a structurally reserved SYSTEM encoding raises `ILLEGAL_INSTRUCTION` in either mode because it is not a defined privileged instruction.

## 7.2 SSWAP

Baseline SIA32-P permits `SSWAP` only with `SCRATCH`:

```text
t = rg
rg = SCRATCH
SCRATCH = t
```

The intended entry sequence is:

```asm
SSWAP sp,SCRATCH
```

allowing one explicit per-CPU/kernel stack pointer without hidden banked GPRs.

---

# 8. Fixed TRAP_VECTOR

SIA32-P has no writable trap-vector register.

Every platform profile defines a fixed architectural address:

```text
TRAP_VECTOR
```

All synchronous exceptions, architectural `TRAP`, `BREAK`, and asynchronous platform interrupts enter at this same address.

Requirements:

- `TRAP_VECTOR` is 2-byte aligned;
- with `VM=0`, it is interpreted as a physical address;
- with `VM=1`, it is interpreted as a virtual address under the current `VMCTX`;
- protected software must map it identically in every active address space, normally supervisor-only, executable, and `G=1`;
- the platform profile must provide compatible physical-mode firmware backing at the same numerical address.

Trap entry does not disable VM or replace VMCTX.

If the CPU cannot fetch the first instruction at `TRAP_VECTOR` after completing trap entry, this is a **trap-entry failure**, not an ordinary recursively reported guest exception. The architectural CPU state from the original trap remains valid for diagnosis. The platform profile determines whether hardware then halts, resets, or enters implementation-defined fatal handling.

---

# 9. Precise execution and exception priority

Synchronous exceptions are detected as part of the current instruction before that instruction commits architectural effects.

Conceptually:

```text
fetch/decode current instruction
    -> evaluate/check synchronous faults
    -> commit instruction effects
    -> retire
    -> consider asynchronous platform interrupt
```

Therefore:

- a synchronous exception from the current instruction takes priority over an asynchronous platform interrupt asserted during that instruction;
- asynchronous interrupts are taken only between retired instructions;
- the interrupt EPC is the next instruction that would have executed.

Within a memory instruction, address alignment is checked before translation/access for that architectural constituent. Page/access-fault priority after alignment is defined by `SIA32-MMU` and the platform physical-access model.

Multi-register operations use the all-or-nothing and first-failing-constituent rules in `SIA32-MULTI-TRANSFER.md`.

---

# 10. Trap entry

A trap may result from:

```text
architectural TRAP imm8
BREAK
illegal instruction
privilege violation
instruction alignment
load/store alignment
page/protection fault
physical access fault
checked arithmetic fault
asynchronous platform interrupt
```

For a synchronous exception at `fault_pc`, trap entry atomically performs:

```text
EPC        = fault_pc
CAUSE      = encoded cause
BADADDR    = fault address if this cause defines one; otherwise unchanged
STATUS.PIE = old STATUS.IE
STATUS.PP  = old mode
STATUS.IE  = 0
mode       = S
PC         = TRAP_VECTOR
```

General-purpose registers and VMCTX are unchanged unless the completed instruction before an asynchronous interrupt had changed them normally.

## 10.1 TRAP

For `TRAP imm8`:

```text
EPC            = address of TRAP instruction
CAUSE.INTERRUPT= 0
CAUSE[15:8]    = imm8
CAUSE[7:0]     = TRAP
BADADDR        unchanged
```

A consumed syscall normally advances saved EPC by 2 in software before `SRET`/`SRETCTX`.

## 10.2 BREAK

`BREAK` raises `BREAKPOINT` as a synchronous exception. It is not an implicit host-debug escape in the architectural model.

## 10.3 Supervisor traps

A trap taken while already in Supervisor mode sets `PP=S`. The same single hardware save level is used; old trap state is overwritten.

---

# 11. CAUSE

CAUSE is 32 bits:

```text
bit 31      INTERRUPT
bits 30..16 reserved; written zero
bits 15..8  auxiliary cause data where defined
bits 7..0   cause code
```

Synchronous cause codes:

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

Unless a cause explicitly defines auxiliary information, `CAUSE[15:8]=0`.

The only baseline asynchronous CPU-level cause is:

```text
CAUSE.INTERRUPT = 1
CAUSE[15:8]     = 0
CAUSE[7:0]      = 0x01 PLATFORM_INTERRUPT
```

The actual timer/device/software source is discovered through the platform interrupt controller, not CAUSE.

---

# 12. BADADDR

BADADDR is hardware-written and deterministic for address-related faults:

```text
JALR odd target                    target address
SRET/SRETCTX odd EPC               attempted return EPC
instruction-fetch alignment        attempted PC
instruction page/access fault      attempted instruction virtual address
load alignment/page/access fault   attempted load virtual address
store alignment/page/access fault  attempted store virtual address
multi-transfer address fault       first failing constituent VA in logical order
```

When VM=0, these architectural addresses are physical addresses because VA=PA.

For `ILLEGAL_INSTRUCTION`, `PRIVILEGE`, `BREAKPOINT`, `TRAP`, `ARITHMETIC`, and `PLATFORM_INTERRUPT`, BADADDR is unchanged from its previous value.

Software must therefore consult CAUSE before interpreting BADADDR.

---

# 13. JALR relationship

The frozen SIA32-I baseline defines odd `JALR` targets as instruction-alignment faults rather than masking bit 0.

Under SIA32-P:

```text
EPC     = address of faulting JALR
CAUSE   = INSTRUCTION_ALIGNMENT
BADADDR = odd target
```

The link destination is not written and the branch does not occur.

---

# 14. SRET

`SRET` is privileged.

Before changing return state, the CPU checks `EPC` alignment.

If `EPC[0]=1`:

```text
raise INSTRUCTION_ALIGNMENT at the SRET instruction
BADADDR = old EPC
SRET has no architectural return-state effect
```

Otherwise `SRET` atomically performs:

```text
mode       = old STATUS.PP
STATUS.IE  = old STATUS.PIE
STATUS.PIE = 1
STATUS.PP  = U
PC         = old EPC
VMCTX      unchanged
```

The return target mapping is **not** pre-walked or prevalidated. If the subsequent instruction fetch fails under the resulting translation state, that fetch raises a normal instruction page/access fault.

---

# 15. SRETCTX

`SRETCTX rs` is the fast address-space-switch-and-return operation.

The CPU captures the old GPR value of `rs` and checks old `EPC` alignment before changing any state.

If `EPC[0]=1`, it raises `INSTRUCTION_ALIGNMENT` exactly as `SRET` does and leaves VMCTX/return state unchanged.

Otherwise it atomically performs:

```text
VMCTX      = captured old value of rs
mode       = old STATUS.PP
STATUS.IE  = old STATUS.PIE
STATUS.PIE = 1
STATUS.PP  = U
PC         = old EPC
```

Properties:

- root+ASID install together;
- ordinary address-space switching never flushes the TLB;
- global translations remain reusable;
- old non-global translations remain cached under old ASIDs;
- subsequent translation uses the new VMCTX;
- it is not a data `FENCE`;
- it does not imply `SYNC.I`;
- it does not pre-walk/prevalidate the return mapping.

Typical Cosmic IPC tail:

```asm
SSWAP   sp,SCRATCH
SRETCTX r8
```

---

# 16. Platform interrupt model

The CPU receives one conceptual asynchronous platform interrupt condition:

```text
timer deadline
software interrupt / future IPI
PLIO Notification
direct platform device
        |
        v
platform interrupt controller
    pending
    enable/mask
    priority/routing
    claim
    complete
        |
        v
single CPU platform-interrupt condition
```

The CPU contains none of the source-specific state above.

If the condition remains asserted after a handler completes one source, another interrupt may be taken after `IE` is restored.

Nested interrupt policy is software/platform policy.

---

# 17. WFI

`WFI` is privileged and is only a power/performance hint.

Architecturally:

- it may be implemented as NOP;
- it never changes `IE`;
- if an implementation actually waits, it must resume when the platform interrupt condition is asserted even if `IE=0`;
- an asserted condition is taken as an interrupt only if `IE=1` at the normal interrupt boundary;
- WFI may not cause platform pending state to be lost.

This prevents `WFI` with interrupts disabled from creating an architectural deadlock requirement.

---

# 18. MMU relationship

The first Lighting protected profile uses:

```text
32-bit VA
2 KiB normal page
1 MiB superpage
12-bit ASID
3-level 6/6/9/11 walk
VMCTX = root>>12 + ASID
4 KiB root alignment
```

Recommended `G=1` mappings include Cosmic kernel code/data where suitable, the fixed TRAP_VECTOR entry, universal ROM libraries/constants, and universal system/IPC stubs.

`SWRITE VMCTX` and `SRETCTX` do not flush translations merely because the active context changed.

Translation details and TLBFENCE behavior are normative in `SIA32-MMU.md`.

---

# 19. Page and physical-access faults

Translation/protection faults are precise:

```text
EPC     = faulting instruction
BADADDR = faulting architectural address
CAUSE   = relevant page-fault cause
```

Physical/bus failure after successful translation raises the corresponding `*_ACCESS_FAULT`.

`LDP/STP/LD4/ST4` retain their separate all-or-nothing rules.

---

# 20. MMIO and DMA

SIA32-P defines no special I/O instruction.

Devices and platform controllers are MMIO. User access exists only when Supervisor software deliberately maps the region.

CPU virtual-memory protection and device DMA authority are independent:

```text
CPU translation/protection    SIA32-P + SIA32-MMU
PLIO DMA authority             SIA Platform / PLIO protected DMA
```

Page-table walks use suitable normal coherent RAM, never MMIO.

---

# 21. SMP relationship

The first Lighting profile is single CPU.

A later SMP platform adds coherent memory, per-CPU/routed interrupt conditions, software IPI sources, CPU startup/identity, remote TLB shootdown protocol, and `SIA32-A` atomics.

None of these require new baseline privileged registers.

---

# 22. First Lighting privileged CPU contract

A Cosmic-capable first Lighting CPU implements:

```text
U/S privilege

STATUS
EPC
CAUSE
BADADDR
SCRATCH
VMCTX

fixed platform TRAP_VECTOR
one CPU platform-interrupt condition

SREAD/SWRITE/SSWAP
SRET/SRETCTX
TLBFENCE/TLBFENCE.VA/TLBFENCE.ASID
WFI
FENCE/SYNC.I

SIA32-MMU
SIA32-MEM
```

There are no banked GPRs, CPU interrupt-source registers, writable TVEC, or separate VMROOT/ASID registers.

---

# 23. Fast Cosmic IPC shape

```text
Sender U
    r1-r6 = short message
    TRAP IPC_CALL
        |
        v
fixed globally mapped TRAP_VECTOR
        |
        v
Cosmic S
    SSWAP sp,SCRATCH
    validate endpoint
    save required persistent sender state
    load receiver persistent state
    retain/prepare r1-r6 message registers
    SSWAP sp,SCRATCH
    SRETCTX receiver_vmctx
        |
        v
Receiver U
```

The common path requires no TLB flush, programmable trap-vector load, CPU interrupt-bitmap save, or separate root/ASID register update.

---

# 24. Conformance requirements

The Rust VM and later hardware must test at minimum:

## Reset/state

- deterministic six-register reset values;
- current mode resets to S;
- reserved STATUS bits read zero and ignore writes.

## Privilege/system registers

- User privileged instructions raise PRIVILEGE;
- Supervisor privileged instructions execute;
- reserved SYSTEM encodings raise ILLEGAL_INSTRUCTION;
- reserved register selectors raise ILLEGAL_INSTRUCTION in S;
- writes to CAUSE/BADADDR raise ILLEGAL_INSTRUCTION;
- SSWAP works only with SCRATCH.

## Trap entry

- each synchronous cause records exact EPC;
- TRAP records imm8;
- BREAK maps to BREAKPOINT;
- BADADDR rules above are exact;
- PP/PIE capture old state;
- IE clears;
- mode becomes S;
- PC becomes TRAP_VECTOR;
- VMCTX is unchanged;
- trap from S records PP=S;
- trap-entry fetch failure is treated as fatal entry failure, not recursive normal exception.

## Priority

- synchronous fault wins over simultaneously eligible asynchronous interrupt;
- asynchronous interrupt occurs only between instructions.

## Return

- SRET restores old PP/PIE state;
- SRETCTX also installs captured VMCTX;
- odd EPC faults before any return-state/VMCTX change;
- valid returns do not pre-walk the target mapping.

## Context serialization

- SWRITE STATUS VM transition affects the next fetch after retirement;
- SWRITE VMCTX affects the next translated fetch/access after retirement;
- neither operation permits stale old-context instructions to retire afterward.

## Platform interrupt

- asserted condition with IE=0 is not consumed;
- IE=1 permits delivery;
- CAUSE records only PLATFORM_INTERRUPT;
- source identification remains outside CPU state.

## MMU

- all conformance requirements from `SIA32-MMU.md`.

---

# 25. Implementation staging

The recommended Rust VM sequence is:

```text
1 structured guest exception type
2 U/S state + six registers
3 SYSTEM decode and register access
4 central trap-entry machinery
5 TRAP/BREAK/base integer exceptions
6 SRET/SRETCTX
7 one platform-interrupt input
8 MMU/VMCTX translation
9 TLBFENCE family
10 Lighting interrupt controller/timer/platform
```

This keeps privilege/trap correctness separable from the later page walker and machine peripherals.
