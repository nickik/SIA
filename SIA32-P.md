# SIA32-P — Privileged Architecture

## Status

- Extension: **SIA32-P**
- Target base: `SIA32-I`
- Purpose: protected operating systems, traps, interrupts, system control, and MMU control
- Required by Lighting/Cosmic: **YES**
- Initial target: single-processor Lighting system
- MMU semantics: [`SIA32-MMU.md`](SIA32-MMU.md)
- Memory ordering: [`SIA32-MEM.md`](SIA32-MEM.md)

`SIA32-P` defines the minimal privileged CPU mechanisms required for a protected capability-oriented operating system such as Cosmic.

The privileged architecture deliberately defines **mechanism, not operating-system policy**. Processes, threads, capabilities, IPC objects, schedulers, drivers, filesystems, and services remain software abstractions.

The core privilege model is:

```text
User mode
    |
    | trap / exception / interrupt
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

There is no separate machine, hypervisor, IRQ, abort, undefined-instruction, or firmware privilege mode in the baseline architecture.

---

# 1. Design goals

`SIA32-P` is designed to support:

- a small capability microkernel;
- strict User/Supervisor isolation;
- precise exceptions;
- fast system calls and IPC;
- asynchronous interrupts;
- timer-driven preemption;
- user-level system services and drivers;
- protected virtual memory through `SIA32-MMU`;
- protected DMA through platform facilities such as PLIO;
- deterministic implementation in the Rust full-system VM;
- straightforward later FPGA implementation;
- future SMP use with `SIA32-A`.

A major performance objective is that an IPC handoff between two address spaces whose translations are already cached requires **no TLB flush**.

## 1.1 Non-goals

The baseline does not define:

- processes or thread objects;
- capability objects;
- IPC endpoint formats;
- scheduler policy;
- filesystem or POSIX abstractions;
- device classes or DMA descriptor formats;
- hardware virtualization;
- banked general-purpose registers;
- software-filled TLBs as the mandatory paging interface.

---

# 2. Privilege levels

## 2.1 User mode (`U`)

User mode:

- may execute all implemented unprivileged SIA instructions;
- may access only mappings permitted by the current `VMCTX` and PTE permissions;
- may enter Supervisor mode using `TRAP`;
- may not modify privileged system state;
- may not modify translation state;
- may not modify interrupt-control state;
- may not execute privileged operations.

An attempted privileged operation in User mode raises `PRIVILEGE`.

## 2.2 Supervisor mode (`S`)

Supervisor mode is the highest baseline privilege level.

It may:

- access privileged system registers;
- install an MMU context;
- change page tables in ordinary memory;
- synchronize cached translations;
- configure trap entry;
- mask and unmask CPU interrupt classes;
- map physical memory and MMIO through page tables;
- return to User or Supervisor execution.

Supervisor mode does not automatically bypass page `R/W/X` permissions when translation is enabled. This keeps mapping semantics uniform and catches kernel permission mistakes.

---

# 3. Reset state

On architectural reset:

```text
mode            = S
STATUS.IE       = 0
STATUS.VM       = 0
PC              = platform reset vector
```

Translation is initially disabled, so firmware begins with physical addressing.

The SIA Platform Specification defines:

- the reset-vector physical address;
- ROM and RAM layout;
- MMIO layout;
- timer;
- interrupt controller;
- device discovery and platform identification.

---

# 4. Privileged system registers

The baseline privileged state is deliberately small:

| ID | Register | Access | Purpose |
|---:|---|---|---|
| `0x0` | `STATUS` | S RW | interrupt, previous-mode, and VM-enable state |
| `0x1` | `TVEC` | S RW | common supervisor trap-entry address |
| `0x2` | `EPC` | S RW | saved exception/interrupt return PC |
| `0x3` | `CAUSE` | S RO | trap cause; written by hardware |
| `0x4` | `BADADDR` | S RO | faulting address where applicable |
| `0x5` | `SCRATCH` | S RW | software-defined trap scratch value; `SSWAP` permitted |
| `0x6` | `VMCTX` | S RW | complete MMU context: root + 12-bit ASID |
| `0x7` | reserved | — | reserved for future baseline growth |
| `0x8` | `IENABLE` | S RW | enabled architectural interrupt classes |
| `0x9` | `IPENDING` | S RO* | pending architectural interrupt classes |
| `0xA..0xF` | reserved | — | future baseline growth |

`*` A later platform/SMP extension may define specific software-pending bits as writable. External hardware pending state is not cleared by arbitrary CPU-register writes.

## 4.1 `VMCTX` is the only architectural address-space context register

The previous draft exposed separate `VMROOT` and `ASID` registers. They are removed from the baseline privileged architecture.

The active context is instead represented only by:

```text
VMCTX[31:12]    root physical address >> 12
VMCTX[11:0]     ASID
```

Thus:

```text
root physical address = VMCTX[31:12] << 12
ASID                  = VMCTX[11:0]
```

The root is therefore 4 KiB aligned even though normal SIA pages are 2 KiB.

Software that needs either field separately reads `VMCTX` and masks/shifts it in ordinary registers.

This avoids transient states in which a new root is paired with an old ASID or vice versa.

Full page-table geometry, PTE format, 2 KiB pages, 1 MiB superpages, ASID behavior, global mappings, and TLB semantics are normative in `SIA32-MMU.md`.

---

# 5. System-register instructions

SIA32-P defines:

```asm
SREAD  rd, sr          ; rd = system_register[sr]
SWRITE sr, rs          ; system_register[sr] = rs
SSWAP  rd, sr          ; exchange GPR and swap-safe system register

SRET                   ; return from supervisor trap
SRETCTX rs             ; install VMCTX from rs and return

TLBFENCE               ; synchronize/invalidate required local translation state
TLBFENCE.VA rs          ; synchronize translation for virtual address
TLBFENCE.ASID rs        ; synchronize non-global translations for ASID

WFI                    ; wait-for-interrupt hint
```

`FENCE` and `SYNC.I` use the system encoding space but are unprivileged operations specified by `SIA32-MEM`.

## 5.1 `SSWAP`

The baseline permits `SSWAP` only with `SCRATCH`.

The intended fast trap-stack transition is:

```asm
SSWAP sp, SCRATCH
```

If User mode was running:

```text
before:
    sp      = user stack pointer
    SCRATCH = CPU/kernel stack pointer

after:
    sp      = CPU/kernel stack pointer
    SCRATCH = saved user stack pointer
```

No banked GPR set is required.

---

# 6. `STATUS`

The baseline `STATUS` register contains:

```text
bit 0      IE       global interrupt enable
bit 1      PIE      saved previous IE
bit 2      PP       previous privilege (0=U, 1=S)
bit 3      VM       virtual-memory translation enable
bits 4..31          reserved; read as zero until assigned
```

The current privilege mode is architectural processor state, not a directly writable `STATUS` field.

## 6.1 `IE`

When `IE=1`, enabled asynchronous interrupt classes may trap.

When `IE=0`, maskable asynchronous interrupts do not trap, although pending state may remain recorded.

Synchronous exceptions are never disabled by `IE`.

## 6.2 `PIE` and `PP`

Trap entry automatically saves the previous interrupt-enable state and privilege level in `PIE` and `PP`.

There is one architectural level of trap-return state. A kernel that enables nested interrupts must first save `EPC`, `CAUSE`, `STATUS`, and any other required state in software.

## 6.3 `VM`

When `VM=0`, normal instruction and data addresses are physical addresses and translation is bypassed.

When `VM=1`, instruction fetches and data accesses use `VMCTX` under the rules in `SIA32-MMU.md`.

Changing `VMCTX` does **not** flush the TLB.

Changing page-table memory may require a `TLBFENCE*` operation as specified by `SIA32-MMU`.

---

# 7. Trap vector

`TVEC` contains the address of the common Supervisor trap-entry point.

Requirements:

- `TVEC` is 2-byte aligned;
- synchronous exceptions enter at `TVEC`;
- asynchronous interrupts enter at `TVEC`;
- software reads `CAUSE` to dispatch.

The baseline deliberately uses one direct vector. A hardware vector table is not required.

---

# 8. Trap entry

A trap may result from:

- a synchronous exception;
- `TRAP`;
- an asynchronous interrupt.

Trap entry is precise and performs:

```text
EPC        = saved PC
CAUSE      = trap reason
BADADDR    = fault address where defined
STATUS.PIE = STATUS.IE
STATUS.PP  = previous privilege
STATUS.IE  = 0
mode       = S
PC         = TVEC
```

General-purpose registers are unchanged.

The active `VMCTX` is **not changed on trap entry**. Cosmic kernel mappings are expected to be globally mapped (`G=1`) where appropriate, so the kernel can execute immediately in every user address space.

## 8.1 Saved PC semantics

For a synchronous fault, `EPC` identifies the faulting instruction.

For `TRAP`, `EPC` identifies the `TRAP` instruction. A consumed system call normally advances `EPC` by 2 before return.

For an asynchronous interrupt, `EPC` identifies the next instruction that would otherwise execute.

## 8.2 Trap while already in Supervisor mode

Supervisor traps use the same mechanism.

Because there is only one hardware save level, software must save trap state before intentionally enabling nesting.

---

# 9. Trap return

## 9.1 `SRET`

`SRET` performs:

```text
mode       = STATUS.PP
STATUS.IE  = STATUS.PIE
STATUS.PIE = 1
STATUS.PP  = U
PC         = EPC
```

`VMCTX` is unchanged.

The redirected instruction fetch then occurs normally under the resulting privilege state and current MMU context. `SRET` does not pre-walk the target mapping merely to validate it. If the next fetch faults, the normal precise instruction fault is taken.

## 9.2 `SRETCTX rs`

`SRETCTX` is the fast address-space-switch-and-return operation used by IPC and scheduling.

Conceptually it performs one architectural control transition:

```text
VMCTX      = rs
mode       = STATUS.PP
STATUS.IE  = STATUS.PIE
STATUS.PIE = 1
STATUS.PP  = U
PC         = EPC
```

Properties:

- `SRETCTX` is privileged.
- `rs` supplies the complete 32-bit `VMCTX` value.
- the new root and ASID become active together;
- installing `VMCTX` **never flushes TLB entries**;
- global (`G=1`) translations remain usable;
- old non-global translations remain cached under their old ASIDs;
- the operation is locally serializing with respect to instruction and data translation;
- it is not a general data-memory `FENCE`;
- it does not imply `SYNC.I`;
- it does not pre-walk `EPC` or otherwise force a page-table walk on the return path.

After `SRETCTX` retires, the next instruction fetch uses the newly installed `VMCTX`. If that fetch cannot translate or execute `EPC`, the resulting instruction page/access fault is taken normally and precisely.

Typical Cosmic IPC tail:

```asm
; receiver EPC/STATUS already installed
; receiver user SP is in SCRATCH
SSWAP   sp, SCRATCH
SRETCTX r8              ; r8 = receiver VMCTX
```

This is intentionally optimized for direct handoff between protection domains.

---

# 10. Exception causes

`CAUSE` is 32 bits:

```text
bit 31      INTERRUPT
bits 30..8  reserved, except TRAP immediate field below
bits 7..0   cause code
```

Initial synchronous causes:

| Code | Name | Meaning |
|---:|---|---|
| `0x00` | `ILLEGAL_INSTRUCTION` | undefined or reserved instruction |
| `0x01` | `PRIVILEGE` | privileged operation attempted in User mode |
| `0x02` | `BREAKPOINT` | `BREAK` |
| `0x03` | `TRAP` | software `TRAP imm8` |
| `0x04` | `INSTRUCTION_ALIGNMENT` | invalid instruction alignment |
| `0x05` | `INSTRUCTION_PAGE_FAULT` | instruction translation/protection fault |
| `0x06` | `LOAD_ALIGNMENT` | misaligned data load |
| `0x07` | `LOAD_PAGE_FAULT` | load translation/protection fault |
| `0x08` | `STORE_ALIGNMENT` | misaligned data store |
| `0x09` | `STORE_PAGE_FAULT` | store translation/protection fault |
| `0x0A` | `INSTRUCTION_ACCESS_FAULT` | physical instruction access failed |
| `0x0B` | `LOAD_ACCESS_FAULT` | physical load access failed |
| `0x0C` | `STORE_ACCESS_FAULT` | physical store access failed |
| `0x0D` | `ARITHMETIC` | trapping arithmetic operation |

For `TRAP`:

```text
CAUSE[15:8] = trap immediate
CAUSE[7:0]  = TRAP
```

`BADADDR` contains the faulting address for address-related exceptions where defined.

---

# 11. Interrupts

SIA32-P defines three CPU-visible interrupt classes:

| Code | Name | Purpose |
|---:|---|---|
| `0x01` | `SOFTWARE_INTERRUPT` | software/IPI-style notification |
| `0x02` | `TIMER_INTERRUPT` | scheduler/deadline interrupt |
| `0x03` | `EXTERNAL_INTERRUPT` | platform interrupt controller has pending work |

An asynchronous interrupt is represented as:

```text
CAUSE.INTERRUPT = 1
CAUSE.CODE      = interrupt class
```

An interrupt may trap when:

```text
STATUS.IE == 1
AND
IENABLE[class] == 1
AND
IPENDING[class] == 1
```

Synchronous exceptions ignore these masks.

The SIA Platform Specification defines the timer, external interrupt-controller interface, software-interrupt generation, claim/complete rules, and mapping from PLIO Notification to `EXTERNAL_INTERRUPT`.

---

# 12. MMU relationship

All virtual-memory semantics are normative in [`SIA32-MMU.md`](SIA32-MMU.md).

The required first Lighting MMU profile is:

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

Typical global mappings are:

```text
G=1
    Cosmic kernel
    universal ROM libraries
    universal ROM constants
    universal system/IPC stubs
```

Typical ASID-tagged mappings are:

```text
G=0
    application code
    heap
    stack
    writable globals
    TLS
    per-process data
    private/shared-memory mappings whose VA/PA identity is not globally invariant
```

SIA32-P does not define separate instruction and data page types. `R/W/X/U/G` permissions define mapping use.

---

# 13. Translation synchronization

SIA32-P provides:

```asm
TLBFENCE
TLBFENCE.VA rs
TLBFENCE.ASID rs
```

Their exact translation-cache semantics are defined by `SIA32-MMU`.

Key distinction:

```text
SWRITE VMCTX       switch address-space context; never flush
TLBFENCE*          synchronize mappings that actually changed
FENCE              ordinary data-memory ordering
SYNC.I             data-write -> local instruction-fetch synchronization
```

Ordinary IPC/context switching must not require `TLBFENCE` merely because the active address space changes.

---

# 14. System calls

`TRAP imm8` is the architectural User-to-Supervisor entry mechanism.

The ISA does not prescribe syscall numbers, capability formats, IPC object semantics, or argument registers. Those belong to the Cosmic/SIA ABI.

Because base instructions are 16 bits, a successfully consumed syscall normally advances:

```text
EPC += 2
```

before return.

A restartable operation may deliberately leave `EPC` unchanged.

---

# 15. MMIO and DMA

SIA32-P does not define special port-I/O instructions.

Devices occupy the physical address space and are accessed through normal scalar loads/stores to MMIO mappings.

MMIO ordering and cacheability are defined by `SIA32-MEM` and the platform profile.

CPU virtual-memory protection and device DMA authority are distinct:

```text
CPU translation/protection     SIA32-MMU
Device DMA protection          platform I/O architecture
```

For Lighting, PLIO provides protected DMA capability channels, so no general-purpose baseline IOMMU is required.

Page tables must reside in normal coherent physical memory and must never be fetched through MMIO mappings.

---

# 16. `WFI`

`WFI` is a privileged power/performance hint.

Architecturally:

- it may stop instruction issue until an interrupt becomes pending;
- it may legally be implemented as a `NOP`;
- it must not lose an interrupt;
- it does not itself enable interrupts.

The Rust VM may use it as an opportunity to advance virtual time directly to the next scheduled event.

---

# 17. Interaction with `SIA32-A`

A single-CPU protected Lighting system uses:

```text
SIA32-I + SIA32-P + SIA32-MMU + SIA32-MEM
```

A coherent shared-memory multiprocessor additionally uses `SIA32-A` and a platform SMP specification.

A trap, interrupt, or context switch may invalidate an `LR.W` reservation as permitted by `SIA32-A`.

SMP platform mechanisms such as CPU identification, secondary-CPU startup, IPIs, and remote TLB shootdown are intentionally outside this baseline privilege specification.

---

# 18. Why only two privilege levels

The intended trust structure is:

```text
hardware
   |
   v
Cosmic microkernel                  Supervisor
   |
   +-- drivers/services             User
   +-- filesystem                   User
   +-- network stack                User
   +-- display server               User
   +-- applications                 User
```

Extra architectural modes for device drivers, interrupts, firmware, or system services do not improve this trust model.

If hardware virtualization is later required, it should be a separate extension.

---

# 19. Required first Lighting profile

The first CPU capable of booting Cosmic must implement:

```text
U/S privilege
STATUS
TVEC
EPC
CAUSE
BADADDR
SCRATCH
VMCTX
IENABLE
IPENDING

SREAD
SWRITE
SSWAP
SRET
SRETCTX
TLBFENCE
TLBFENCE.VA
TLBFENCE.ASID
WFI

precise exceptions
software/timer/external interrupt classes
SIA32-MMU semantics
SIA32-MEM semantics
```

`VMROOT` and `ASID` are **not** separate baseline architectural system registers.

---

# 20. Full-system VM implementation requirements

The Rust SIA VM must implement:

- current privilege mode;
- all baseline privileged registers;
- trap entry;
- `SRET` and `SRETCTX`;
- the `SIA32-MMU` page-table walker and translation-cache semantics;
- page/access faults;
- translation fences;
- interrupt pending/enable behavior;
- physical bus with ROM/RAM/MMIO distinction;
- timer/external-interrupt injection from the platform model.

Normal Cosmic execution must not depend on host semihosting for machine services.

---

# 21. FPGA/reference-model requirements

The Rust VM is the architectural reference for later RTL/FPGA implementations.

Differential tests must compare at least:

- trap PC and cause;
- privilege transitions;
- `SRET`/`SRETCTX` behavior;
- `VMCTX` installation;
- page-table walks;
- ASID/global translation matching;
- page-size handling;
- permission failures;
- translation fences;
- interrupt acceptance;
- physical access faults.

Microarchitecture remains free to choose TLB organization, page-walk implementation, cache topology, and internal pipelines.

---

# 22. Conformance tests

Before `SIA32-P` is frozen, test at least:

## Privilege

- User ordinary instruction succeeds.
- User privileged instruction raises `PRIVILEGE`.
- Supervisor privileged instruction succeeds.
- User cannot access supervisor-only mapping.

## Trap entry/return

- every synchronous cause records correct `EPC`;
- interrupt `EPC` resumes the next instruction;
- `TRAP` immediate is preserved;
- `BADADDR` is correct;
- `IE/PIE/PP` transitions are correct;
- `SRET` restores the expected state;
- `SRETCTX` installs the complete new context without flushing translations;
- a fault on the first post-`SRETCTX` fetch traps normally under the new context.

## MMU/context

- 2 KiB leaf translation;
- 1 MiB superpage translation;
- 12-bit ASID separation;
- `G=1` mapping survives arbitrary `VMCTX` changes;
- `SWRITE VMCTX` never destroys unrelated TLB entries;
- ASID recycling requires explicit invalidation;
- selective `TLBFENCE` operations remove stale translations as specified.

## Trap stack transition

- `SSWAP sp,SCRATCH` safely exchanges user and kernel stacks;
- User mode cannot modify `SCRATCH`;
- nested Supervisor-trap behavior is deterministic.

---

# 23. Remaining freeze work

The privileged **semantics** are now substantially defined.

Remaining v1 work is primarily:

- freeze binary encodings including `SRETCTX` and `VMCTX` register ID;
- freeze the complete SIA32-I opcode map;
- implement conformance tests in the Rust VM;
- freeze the SIA ABI;
- freeze the SIA Platform Specification required for boot and interrupts.
