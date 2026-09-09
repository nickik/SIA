# SIA32-P — Privileged Architecture

## Status

- Extension: **SIA32-P**
- Target base: `SIA32-I`
- Purpose: protected operating systems, virtual memory, traps, interrupts, and system control
- Required by Lighting/Cosmic: **YES**
- Required for a user-only or deeply embedded SIA implementation: **NO**
- Initial target: single-processor Lighting system

`SIA32-P` defines the minimal privileged architecture needed to run a protected capability-oriented operating system such as Cosmic.

The design is intentionally small. It provides the mechanisms the kernel needs while avoiding architectural policy about processes, threads, capabilities, IPC, files, drivers, or scheduling.

The core model is:

```text
User mode
    |
    | trap / exception / interrupt
    v
Supervisor mode
    |
    | SRET
    v
User mode
```

There are exactly two architectural privilege levels:

```text
U   User
S   Supervisor
```

There is no separate machine mode, hypervisor mode, interrupt mode, abort mode, or undefined-instruction mode in the baseline architecture.

All synchronous exceptions and asynchronous interrupts enter the same supervisor trap mechanism and record their reason in privileged state.

---

# 1. Design goals

`SIA32-P` is designed to support:

- a small capability microkernel;
- strict isolation between kernel and user software;
- separate virtual address spaces;
- precise page faults;
- protected read/write/execute mappings;
- fast system calls;
- asynchronous interrupts;
- timer-driven preemption;
- user-level device servers;
- protected DMA through platform facilities such as PLIO;
- deterministic implementation in the SIA Rust full-system VM;
- straightforward later FPGA implementation;
- future SMP use with `SIA32-A`.

The architecture should remain small enough that the complete privileged state can be understood on one page.

## 1.1 Non-goals

The baseline does **not** define:

- process objects;
- capability objects;
- IPC objects;
- scheduler policy;
- filesystem semantics;
- POSIX abstractions;
- device classes;
- DMA descriptor formats;
- a hypervisor;
- nested virtualization;
- security domains above supervisor mode;
- ARM-style banked register sets;
- MIPS-style software-filled TLB entries as the mandatory paging interface.

Those belong to Cosmic, PLIO/QDX, later extensions, or platform specifications.

---

# 2. Design influences

The architecture takes useful ideas from several established RISC designs without copying any one of them.

## 2.1 Early ARM

Early ARM demonstrates that a small RISC core can provide protected execution, exception entry, privileged state, and interrupt masking with modest hardware.

SIA deliberately does **not** copy ARM's multiple privileged exception modes and banked general registers. Separate IRQ, FIQ, abort, undefined, and supervisor modes are unnecessary for the Cosmic model and complicate context handling.

SIA keeps one supervisor mode and one trap path.

## 2.2 MIPS

MIPS strongly influences the SIA trap model:

- compact privileged system state;
- explicit exception cause;
- saved exception program counter;
- fault-address reporting;
- simple exception return;
- software-visible interrupt pending/mask state.

MIPS's software-managed TLB is elegant, especially for early implementations, but it makes TLB refill a kernel ABI concern. The baseline SIA32-P instead defines a simple in-memory page-table format and architectural page-table walk. An implementation may cache translations however it chooses.

## 2.3 RISC-V

RISC-V contributes several useful simplifications:

- a clean user/supervisor distinction;
- explicit privileged system registers;
- a page-table-root register;
- address-space identifiers;
- explicit translation-cache synchronization;
- a scratch system register for trap entry;
- no requirement for banked integer registers.

SIA does not adopt RISC-V's separate machine privilege level for the baseline Lighting computer. The supervisor kernel directly owns the machine.

---

# 3. Privilege levels

## 3.1 User mode (`U`)

User mode executes ordinary applications and user-space system services.

User mode:

- may execute all unprivileged `SIA32-I` instructions;
- may execute supported unprivileged extensions;
- may access only virtual pages whose mappings permit user access;
- may invoke the kernel with `TRAP`;
- may not modify MMU state;
- may not modify interrupt state;
- may not access privileged system registers;
- may not execute privileged instructions;
- may not directly access supervisor-only mappings.

An attempted privileged operation in User mode raises `PRIVILEGE`.

## 3.2 Supervisor mode (`S`)

Supervisor mode is the highest architectural privilege level.

Supervisor mode may:

- access privileged system registers;
- configure virtual memory;
- configure trap entry;
- mask/unmask CPU interrupt classes;
- return to User mode;
- map physical memory and devices;
- execute privileged memory-management operations.

The operating system is responsible for deciding which authority to delegate to user processes.

There is no privilege level above Supervisor in the baseline.

---

# 4. Reset state

On architectural reset:

```text
mode            = S
interrupts      = disabled
virtual memory  = disabled
PC              = platform reset vector
```

The processor begins in physical-address mode so firmware can execute before page tables exist.

Other privileged registers enter defined reset values given below.

The platform specification defines:

- reset-vector physical address;
- ROM location;
- RAM physical layout;
- MMIO layout;
- interrupt-controller device;
- timer device.

SIA32-P defines the CPU behavior, not those platform addresses.

---

# 5. Privileged system registers

The baseline privileged state consists of the following registers.

| Register | Access | Purpose |
|---|---|---|
| `STATUS` | S RW | privilege, interrupt and MMU control |
| `TVEC` | S RW | trap-entry address |
| `EPC` | S RW | saved exception/interrupt PC |
| `CAUSE` | S RW | trap cause |
| `BADADDR` | S RW | faulting virtual/physical address where applicable |
| `SCRATCH` | S RW | kernel-defined trap scratch value |
| `VMROOT` | S RW | physical page number of root page table |
| `ASID` | S RW | current address-space identifier |
| `IENABLE` | S RW | enabled interrupt classes |
| `IPENDING` | S RO/W1C* | pending interrupt classes |

`*` Platform-defined software-pending bits may be writable; hardware interrupt pending bits are normally read-only. Exact clearing of external interrupts belongs to the interrupt-controller specification.

A future extension may add performance counters, debug registers, SMP identifiers, virtualization state, or additional interrupt classes without changing these baseline semantics.

---

# 6. System-register instructions

SIA32-P defines privileged operations conceptually equivalent to:

```asm
SREAD  rd, sr          ; rd = system_register[sr]
SWRITE sr, rs          ; system_register[sr] = rs
SSWAP  rd, sr          ; atomically exchange rd and system_register[sr]
SRET                   ; return from supervisor trap
TLBFENCE                ; synchronize all translation state
TLBFENCE.VA rs          ; synchronize translation for virtual address
TLBFENCE.ASID rs        ; synchronize translations for ASID
WFI                    ; wait for interrupt hint
```

Exact binary encodings remain part of the SIA encoding-freeze work.

All except any explicitly documented user-readable register operation are privileged.

## 6.1 Why `SSWAP` exists

`SSWAP` gives a trap handler a safe first instruction without architectural banked registers.

The kernel may keep a kernel stack pointer in `SCRATCH` while User mode is running:

```asm
; immediately after trap from user
SSWAP sp, SCRATCH
```

After the instruction:

```text
sp       = kernel stack pointer
SCRATCH  = saved user stack pointer
```

The handler may then save the remaining user registers to the kernel stack.

Before returning:

```asm
SSWAP sp, SCRATCH
SRET
```

This follows the principle of a software-managed trap frame while avoiding unsafe use of a user-controlled stack.

---

# 7. `STATUS` register

The initial `STATUS` register contains the following architecturally defined fields.

```text
bit 0      IE       global interrupt enable
bit 1      PIE      saved previous IE
bit 2      PP       previous privilege (0=U, 1=S)
bit 3      VM       virtual-memory translation enable
bits 4..31          reserved, read as zero until assigned
```

The current privilege mode is architectural processor state rather than a normal writable field in `STATUS`.

## 7.1 `IE`

When `IE=1`, enabled asynchronous interrupt classes may trap to Supervisor mode.

When `IE=0`, maskable asynchronous interrupts do not trap, but their pending state may remain recorded.

Synchronous exceptions are not disabled by `IE`.

## 7.2 `PIE` and `PP`

`PIE` and `PP` are written automatically on trap entry and consumed by `SRET`.

They provide one architectural level of saved trap-return state.

A kernel that wishes to permit nested interrupts must save `EPC`, `CAUSE`, `STATUS`, and any other required state before re-enabling interrupts.

## 7.3 `VM`

When `VM=0`:

- instruction addresses are physical addresses;
- load/store addresses are physical addresses;
- page-table translation is bypassed.

When `VM=1`, normal instruction and data accesses are translated through the current `VMROOT`/`ASID` context.

Supervisor code is allowed to change `VM`.

Changing `VMROOT`, `ASID`, page-table memory, or `VM` may require a `TLBFENCE` operation as specified below.

---

# 8. Trap vector

`TVEC` contains the virtual address of the common supervisor trap entry point.

Requirements:

- `TVEC` must be 2-byte aligned;
- all synchronous exceptions enter at `TVEC`;
- all asynchronous interrupts enter at `TVEC`;
- the handler reads `CAUSE` to distinguish the reason.

The baseline deliberately uses a **single direct vector** rather than a table of hardware vectors.

This keeps CPU state and exception entry simple. Software may construct its own dispatch table immediately after entry.

A later optional vectored-interrupt extension may be added if measurements justify it.

---

# 9. Trap entry

A trap may be caused by:

- a synchronous exception;
- a `TRAP` instruction;
- an asynchronous interrupt.

Trap entry is precise.

The processor performs the following architectural transition:

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

Other general-purpose registers are unchanged.

There are no banked general registers.

## 9.1 Saved PC semantics

For a synchronous exception, `EPC` identifies the instruction that caused the exception.

For `TRAP`, `EPC` identifies the `TRAP` instruction itself. A kernel implementing a system call normally advances `EPC` by 2 before returning.

For an asynchronous interrupt, `EPC` identifies the next instruction that would have executed if the interrupt had not been taken.

This makes synchronous faults restartable while asynchronous interrupts resume naturally.

## 9.2 Trap while already in Supervisor mode

A trap in Supervisor mode uses the same mechanism.

Because the baseline has only one architectural `EPC`/`CAUSE` save level, supervisor software must save trap state before enabling nested interrupts.

An unexpected synchronous fault in an early supervisor trap prologue is considered a kernel-level failure and may lead to panic/reset according to platform policy.

This avoids hidden hardware nesting stacks.

---

# 10. Trap return

`SRET` is privileged.

Conceptually it performs:

```text
mode       = STATUS.PP
STATUS.IE  = STATUS.PIE
STATUS.PIE = 1
STATUS.PP  = U
PC         = EPC
```

`SRET` validates that the resulting PC obeys normal instruction alignment and translation rules.

Returning to User mode does not implicitly alter `VMROOT`, `ASID`, or mappings.

The kernel is expected to install the desired address-space state before `SRET`.

---

# 11. Exception causes

`CAUSE` is a 32-bit register.

```text
bit 31      INTERRUPT
bits 30..8  reserved
bits 7..0   cause code
```

When `INTERRUPT=0`, the cause is synchronous.

Initial synchronous cause codes:

| Code | Name | Meaning |
|---:|---|---|
| `0x00` | `ILLEGAL_INSTRUCTION` | undefined/reserved instruction |
| `0x01` | `PRIVILEGE` | privileged operation attempted without authority |
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

Additional synchronous causes may be assigned later.

For `TRAP`, the architectural `imm8` value must remain available to the handler. The simplest encoding is:

```text
CAUSE[15:8] = trap immediate
CAUSE[7:0]  = TRAP
```

when `CAUSE.INTERRUPT=0` and the cause is `TRAP`.

---

# 12. Interrupts

SIA32-P defines CPU-visible interrupt classes but does not define a complete platform interrupt controller.

Initial interrupt cause codes are:

| Code | Name | Purpose |
|---:|---|---|
| `0x01` | `SOFTWARE_INTERRUPT` | software/IPI-style notification |
| `0x02` | `TIMER_INTERRUPT` | scheduler/time interrupt |
| `0x03` | `EXTERNAL_INTERRUPT` | platform interrupt controller has pending work |

An interrupt is represented as:

```text
CAUSE.INTERRUPT = 1
CAUSE.CODE      = interrupt class
```

## 12.1 Interrupt enable

`IENABLE` contains one bit per architectural interrupt class.

An interrupt may trap when:

```text
STATUS.IE == 1
AND
IENABLE[class] == 1
AND
IPENDING[class] == 1
```

Synchronous exceptions ignore these masks.

## 12.2 External interrupt controller

The CPU need not expose one interrupt bit per device.

The Lighting platform may provide one `EXTERNAL_INTERRUPT` input from the platform interrupt/notification controller. After entry, Cosmic queries that controller to identify and acknowledge the source.

This matches PLIO's model in which device notifications are aggregated and software claims the specific pending source.

## 12.3 Timer

The architectural CPU only requires a timer interrupt class.

The actual monotonic counter and compare/timer registers may be platform MMIO devices rather than privileged CPU registers.

This keeps calendar/time facilities outside the core ISA.

---

# 13. Virtual address model

SIA32-P uses a 32-bit virtual address space.

The baseline page size is:

```text
4096 bytes (4 KiB)
```

Virtual-address decomposition:

```text
31                    22 21                    12 11             0
+-----------------------+------------------------+----------------+
|       L1 index        |       L0 index         | page offset    |
|       10 bits         |       10 bits          |   12 bits      |
+-----------------------+------------------------+----------------+
```

Each page-table page contains 1024 32-bit entries and therefore occupies exactly one 4 KiB page.

The baseline supports only 4 KiB leaf mappings.

Large/superpages are deliberately deferred.

---

# 14. `VMROOT` and `ASID`

## 14.1 `VMROOT`

`VMROOT` contains the 20-bit physical page number of the current L1 root page table.

Conceptually:

```text
root_physical_address = VMROOT << 12
```

The remaining high bits of the 32-bit system register are reserved if physical addresses are limited to 32 bits.

Page tables are always accessed by **physical address** during the page-table walk.

## 14.2 `ASID`

`ASID` identifies the current address space for cached translations.

Baseline width:

```text
8 bits
```

Therefore at least 256 distinguishable ASID values are available before reuse.

ASID value zero is valid and has no special meaning.

An implementation may contain no TLB at all, but must behave as if translations associated with distinct ASIDs do not alias.

When an ASID is reused for a different address space, software must perform the required translation fence.

---

# 15. Page-table entry format

A page-table entry is one naturally aligned 32-bit little-endian word.

```text
31                                      12 11                 0
+-----------------------------------------+--------------------+
|             physical PPN                |       flags        |
|               20 bits                   |      12 bits       |
+-----------------------------------------+--------------------+
```

Initial flag allocation:

```text
bit 0   V      valid
bit 1   R      readable
bit 2   W      writable
bit 3   X      executable
bit 4   U      accessible from User mode
bit 5   G      global mapping; ASID-independent TLB entry
bits 6..11    software-reserved
```

The six software-reserved bits are ignored by hardware and preserved in memory.

They may be used by Cosmic for mapping metadata.

## 15.1 Invalid entry

If `V=0`, the entry is invalid and translation raises the corresponding page fault.

## 15.2 Non-leaf entry

An entry is a pointer to the next page-table level when:

```text
V = 1
R = 0
W = 0
X = 0
```

Its PPN identifies the physical page containing the next-level page table.

`U` and `G` are ignored for a non-leaf entry in the baseline.

## 15.3 Leaf entry

An entry is a leaf mapping when:

```text
V = 1
AND
(R | W | X) != 0
```

The PPN identifies the mapped 4 KiB physical frame.

Unlike RISC-V Sv32, SIA does not require `R=1` when `W=1`; read-only, write-only, execute-only, read/write, read/execute, and read/write/execute pages are architecturally representable.

Operating systems may choose a narrower policy.

---

# 16. Page-table walk

When `STATUS.VM=1`, an instruction fetch or data access performs the following conceptual translation.

Given virtual address `va`:

```text
l1 = va[31:22]
l0 = va[21:12]
off = va[11:0]
```

## 16.1 Level 1

```text
root = VMROOT << 12
pte1_address = root + l1 * 4
pte1 = physical_read32(pte1_address)
```

Requirements:

- inaccessible physical page-table memory raises the appropriate access fault;
- invalid `pte1` raises the appropriate page fault;
- a leaf at L1 is reserved in the baseline and raises a page fault;
- a valid non-leaf supplies the L0 table PPN.

## 16.2 Level 0

```text
l0_table = pte1.PPN << 12
pte0_address = l0_table + l0 * 4
pte0 = physical_read32(pte0_address)
```

A valid leaf supplies the physical frame.

```text
pa = (pte0.PPN << 12) | off
```

Permission checks occur before the architectural memory operation is performed.

The walk is conceptual. Implementations may use TLBs, caches, microcode, dedicated walkers, or other mechanisms as long as visible behavior is equivalent.

---

# 17. Page permissions

For User mode:

```text
U must be 1
```

and the access type must be permitted:

```text
instruction fetch requires X
load              requires R
store             requires W
```

For Supervisor mode:

- `U` is ignored;
- access type still requires the corresponding `R`, `W`, or `X` permission when translation is enabled.

This means the kernel is not automatically allowed to write a read-only mapped page merely because it is privileged.

Supervisor software can change the page-table entry or temporarily access the physical mapping by a deliberate mechanism if required.

This rule catches kernel permission mistakes and keeps mapping semantics uniform.

## 17.1 No implicit executable data

A readable page is not automatically executable.

A writable page is not automatically executable.

Execute permission is explicit.

This permits Cosmic to enforce W^X-style policies without additional hardware support.

---

# 18. Page faults

Translation/protection faults are precise.

The relevant synchronous cause is written to `CAUSE` and:

```text
BADADDR = faulting virtual address
EPC     = faulting instruction
```

The architectural memory operation has not completed.

For a store fault, no part of the store may become architecturally visible.

For a load fault, the destination register is not modified.

For an instruction page fault, the instruction is not executed.

Multi-register memory instructions defined by SIA must obey their separately specified precise/restart semantics. They are already prohibited for MMIO in the base architecture.

---

# 19. Physical access faults

Page translation can succeed while the resulting physical access fails, for example because:

- no physical memory/device exists at the address;
- a platform bus reports an error;
- an MMIO target rejects the operation.

These raise `*_ACCESS_FAULT` rather than `*_PAGE_FAULT`.

`BADADDR` contains the address relevant to diagnosing the failed architectural access. For translated CPU accesses this should normally remain the original virtual address, while implementation/debug state may separately expose the physical bus address.

This distinction lets Cosmic separate mapping errors from machine/device errors.

---

# 20. Translation caches and `TLBFENCE`

The architecture permits implementations to cache page-table translations.

Page-table memory is ordinary memory. Writing a PTE does not by itself guarantee that a processor stops using an older cached translation.

The kernel therefore uses explicit translation synchronization.

## 20.1 `TLBFENCE`

```asm
TLBFENCE
```

After completion, no subsequent memory access by the current CPU may use a translation derived from stale page-table state for the current architecture context.

A simple implementation may flush its entire TLB.

## 20.2 `TLBFENCE.VA`

```asm
TLBFENCE.VA rs
```

Synchronizes cached translations corresponding to the virtual page containing `rs` for the current ASID, plus any implementation-required related walk-cache state.

## 20.3 `TLBFENCE.ASID`

```asm
TLBFENCE.ASID rs
```

Synchronizes cached non-global translations for the low 8-bit ASID supplied by `rs`.

The baseline does not require a combined VA+ASID operation. It may be added later if measurement shows value.

## 20.4 Ordering

A translation fence also orders prior stores to page-table memory before subsequent translations affected by the fence.

This makes the instruction a memory-management fence rather than merely a command to invalidate one named TLB structure.

---

# 21. Context switching

A Cosmic address-space switch conceptually performs:

```text
1. save old user CPU state
2. install new VMROOT
3. install new ASID
4. execute required TLBFENCE if the ASID is being reused or mappings require it
5. install new EPC and trap-return state
6. SRET
```

No hardware process identifier or capability table is required.

The kernel owns the association between Cosmic tasks and `(VMROOT, ASID)`.

Global mappings (`G=1`) may be used for kernel mappings that are identical in every address space.

---

# 22. Kernel address-space convention

SIA32-P does not reserve a fixed virtual address range for the kernel.

That is an ABI/platform decision.

For example, Cosmic may choose a split such as:

```text
lower virtual region       user address space
upper virtual region       globally mapped Cosmic kernel
```

but the exact boundary is not architectural.

A capability microkernel may therefore choose the layout that best fits its implementation.

---

# 23. System calls

The existing SIA `TRAP imm8` instruction is the architectural kernel-entry mechanism.

In User mode:

```asm
TRAP imm8
```

enters Supervisor mode through the normal trap path.

The architecture does not prescribe:

- syscall numbers;
- argument registers;
- capability arguments;
- IPC semantics;
- return-value conventions.

Those belong to the Cosmic ABI.

Because all SIA base instructions are 16 bits, the kernel normally advances:

```text
EPC += 2
```

before returning from a successfully consumed system call.

A fault/restart mechanism may deliberately leave `EPC` unchanged.

---

# 24. Breakpoints and debugging

`BREAK` raises `BREAKPOINT` through the normal trap path.

The baseline does not define hardware breakpoints, watchpoints, single-step state, or a separate debug privilege mode.

Those may be added as `SIA32-D` or platform debug facilities later.

The Rust full-system VM may provide stronger non-architectural debugging features without exposing them to guest software.

---

# 25. Wait for interrupt

`WFI` is a privileged power/performance hint.

Architecturally:

- it may stop instruction issue until an interrupt becomes pending;
- an implementation is permitted to treat it as a `NOP`;
- it must not cause an enabled interrupt to be lost;
- it does not itself enable interrupts.

The SIA Rust VM may use `WFI` as an opportunity to advance virtual time directly to the next scheduled device event.

---

# 26. MMIO and devices

SIA32-P does not introduce special I/O instructions.

Devices are accessed through the normal physical memory address space.

Cosmic controls device authority by controlling virtual mappings and, for DMA-capable PLIO devices, by programming the platform's protected DMA capability mechanism.

User software cannot access an MMIO device unless Supervisor software explicitly maps that physical device region into its address space or delegates access through a driver/service.

## 26.1 Page-table walks and MMIO

Page-table memory must reside in normal physical memory suitable for page-table reads.

A page-table walk must never use a device/MMIO mapping as a page table.

The platform memory map may enforce this directly.

---

# 27. DMA and IOMMU relationship

CPU virtual memory protection does not by itself protect RAM from a bus-mastering device.

SIA32-P therefore explicitly distinguishes:

```text
CPU address translation/protection     SIA32-P MMU

device DMA authority/protection        platform I/O architecture
```

For Lighting, PLIO provides the protected device DMA boundary. Devices receive bounded DMA handles rather than unrestricted physical addresses.

No general-purpose IOMMU is required in the baseline Lighting machine if PLIO's DMA capability mechanism supplies the required isolation.

This is important for a capability operating system: user-level drivers may be delegated device authority without giving the device arbitrary access to kernel memory.

---

# 28. Interaction with `SIA32-A`

`SIA32-P` and `SIA32-A` are independent extensions.

A single-processor protected machine may implement:

```text
SIA32-I + SIA32-P
```

A coherent shared-memory multiprocessor intended to run Cosmic should implement:

```text
SIA32-I + SIA32-P + SIA32-A
```

On a trap, interrupt, or context switch, an implementation may invalidate an `LR.W` reservation as already allowed by `SIA32-A`.

For SMP, additional platform mechanisms are required:

- CPU identification;
- interprocessor interrupts;
- coherent memory;
- boot/stop control for secondary CPUs;
- remote TLB-shootdown protocol implemented in software using IPIs.

These do not require changing the basic two-level privilege model.

---

# 29. Requirements for a seL4-style microkernel

A small capability microkernel fundamentally needs mechanisms rather than policy.

SIA32-P provides the CPU-side mechanisms required for that model:

## Protection

- User/Supervisor isolation.
- User-access bit in every leaf PTE.
- separate read/write/execute permissions.
- precise privilege faults.

## Address spaces

- 32-bit virtual addresses.
- 4 KiB frames.
- explicit page-table objects in ordinary physical memory.
- explicit root page table.
- ASIDs.
- map/unmap through kernel-controlled PTE updates.
- explicit translation synchronization.

## Kernel entry/exit

- precise traps.
- common trap vector.
- saved exception PC.
- cause register.
- fault address.
- trap scratch register.
- safe kernel-stack switch using `SSWAP`.
- `SRET`.

## Scheduling

- timer interrupt class.
- global interrupt enable.
- per-class masks/pending state.
- `WFI` for idle.

## User-level drivers

- kernel-controlled device mappings.
- external interrupts routed through kernel-visible interrupt objects/services.
- protected DMA supplied by PLIO rather than unrestricted physical DMA.

## SMP when required

- add `SIA32-A` atomics and fences;
- platform IPIs;
- coherent memory;
- software TLB shootdown.

The hardware does **not** need to understand capabilities. Cosmic builds capabilities from these lower-level protection mechanisms.

---

# 30. Why only two privilege levels

Two privilege levels are sufficient for the baseline system because the trust structure is:

```text
hardware
  |
  v
Cosmic microkernel              Supervisor
  |
  +--> drivers/services         User
  +--> filesystem               User
  +--> network stack            User
  +--> display server           User
  +--> applications             User
```

The operating system deliberately moves most traditional kernel services out of Supervisor mode.

Adding separate architectural modes for drivers, interrupts, aborts, firmware, or hypervisors would not improve this trust model.

If future SIA systems need hardware virtualization, it should be a separately specified extension rather than permanent baseline complexity.

---

# 31. Why no banked registers

Banked registers make exception entry fast but create hidden per-mode integer state and complicate context switching, debugging, formal reasoning, and software conventions.

SIA instead has:

```text
one integer register file
+
one SCRATCH system register
+
software trap frames
```

`SSWAP` provides the one operation needed to safely transition from a user stack to a kernel stack.

This is sufficient for a small kernel while keeping architectural state explicit.

---

# 32. Why hardware-walked page tables

A software-managed TLB can reduce MMU hardware, and remains a reasonable future profile for extremely small implementations.

The baseline Lighting profile instead standardizes the page-table walk because it provides:

- one portable Cosmic paging model;
- one deterministic Rust VM model;
- one page-fault contract;
- simpler kernel code;
- straightforward FPGA differential testing;
- freedom for implementations to choose TLB size/shape invisibly.

The hardware walker is intentionally small: at most two dependent 32-bit PTE reads for a TLB miss.

Nothing in the architecture requires a cache or large TLB.

---

# 33. Required first Lighting profile

The first Lighting CPU intended to boot Cosmic must implement:

```text
SIA32-I
SIA32-P
```

plus whichever base integer multiply/divide profile is selected for Lighting.

The mandatory `SIA32-P` subset is the entire baseline defined here:

```text
U/S privilege
STATUS
TVEC
EPC
CAUSE
BADADDR
SCRATCH
VMROOT
ASID
IENABLE
IPENDING
SREAD / SWRITE / SSWAP
SRET
TLBFENCE
TLBFENCE.VA
TLBFENCE.ASID
WFI
4 KiB pages
two-level page tables
R/W/X/U/G permissions
precise page/access faults
software/timer/external interrupt classes
```

The initial Lighting workstation is single-CPU and therefore does not require `SIA32-A` for correctness.

A later Lighting/Neutron SMP profile requires it.

---

# 34. Full-system VM implementation requirements

The Rust SIA VM must implement SIA32-P as architectural guest behavior rather than host shortcuts.

Required VM components include:

- current privilege mode;
- all privileged registers;
- trap entry and return;
- page-table walker;
- TLB or equivalent optional cache;
- translation fences;
- page-fault generation;
- physical access faults;
- interrupt pending/enable logic;
- timer/external interrupt injection;
- physical memory bus;
- ROM/RAM/MMIO distinction.

The VM should boot through the architectural reset vector and firmware.

Normal Cosmic execution must not use emulator semihosting for services that the real machine provides through devices.

Semihosting may remain as an explicit development/debug option.

---

# 35. FPGA implementation requirements

The FPGA implementation must produce the same architectural behavior as the Rust VM.

In particular, the VM becomes the differential reference for:

- trap PC values;
- cause codes;
- page-table walks;
- permission failures;
- TLB invalidation behavior;
- interrupt acceptance;
- `SRET` transitions;
- physical access faults.

Microarchitectural details remain free:

```text
VM                         FPGA
------------------------------------------------------
Rust hash/map TLB          associative hardware TLB
Rust page walk             FSM/microcoded page walk
virtual event interrupt    hardware interrupt input
Vec/RAM backing            SDRAM/DDR controller
```

Only architectural observations must match.

---

# 36. Open encoding work

This document freezes privileged **semantics**, not final instruction encodings.

The SIA encoding work still needs to allocate encodings for:

```text
SREAD
SWRITE
SSWAP
SRET
TLBFENCE
TLBFENCE.VA
TLBFENCE.ASID
WFI
```

These should use reserved base-growth or `EXT` space rather than stealing heavily used SIA32-I primary opcodes without code-density measurements.

System-register identifiers also require stable numeric assignments.

---

# 37. Architectural conformance tests

Before SIA32-P is considered frozen, the Rust VM should pass tests for all of the following.

## Privilege

- User ordinary instruction succeeds.
- User privileged instruction faults.
- Supervisor privileged instruction succeeds.
- User cannot access supervisor-only page.
- Supervisor respects R/W/X permissions.

## Trap entry

- every synchronous cause records correct `EPC`;
- `TRAP` records immediate;
- `BADADDR` is correct for each address fault;
- previous mode is recorded correctly;
- interrupts become disabled on entry;
- `TVEC` receives control.

## Trap return

- return to User;
- return to Supervisor;
- previous interrupt state restored;
- correct `EPC` resume behavior.

## Paging

- valid mapping;
- invalid L1;
- invalid L0;
- user permission failure;
- read permission failure;
- write permission failure;
- execute permission failure;
- page-table physical access failure;
- physical target access failure;
- ASID separation;
- global mapping behavior.

## Translation synchronization

- stale translation may persist before required fence;
- `TLBFENCE` removes stale behavior;
- `TLBFENCE.VA` affects selected virtual page;
- `TLBFENCE.ASID` affects selected address space;
- page-table stores are ordered before post-fence translation.

## Interrupts

- pending disabled interrupt does not trap;
- enabled pending interrupt traps;
- global `IE` masks all maskable interrupts;
- synchronous fault still traps with `IE=0`;
- timer cause is correct;
- external cause is correct;
- interrupt `EPC` resumes next instruction.

## Trap stack transition

- `SSWAP sp,SCRATCH` safely exchanges user/kernel stacks;
- user cannot modify `SCRATCH`;
- nested supervisor trap rules are deterministic.

---

# 38. Design references

Useful external architectural references for this design include:

- ARM Architecture Reference Manual, especially the classic privilege/exception mode and CPSR/SPSR model.
- MIPS32 Privileged Resource Architecture, especially Status, Cause, EPC, BadVAddr and TLB/exception concepts.
- RISC-V Privileged Architecture, especially U/S privilege, `satp`, `stvec`, `sepc`, `scause`, `stval`, `sscratch`, `sstatus`, and `SFENCE.VMA`.
- seL4 architecture and porting documentation for the practical hardware mechanisms required by a capability microkernel.

The SIA architecture intentionally adopts mechanisms selectively rather than reproducing any of these designs.

---

# 39. Summary

The complete baseline privileged model is deliberately small:

```text
Privilege:
    User
    Supervisor

Trap state:
    STATUS
    TVEC
    EPC
    CAUSE
    BADADDR
    SCRATCH

Virtual memory:
    VMROOT
    ASID
    4 KiB pages
    two-level 10/10/12 page tables
    R W X U G permissions
    explicit TLBFENCE operations

Interrupts:
    global enable
    class enable/pending
    software
    timer
    external

Control:
    SREAD
    SWRITE
    SSWAP
    SRET
    WFI
```

This is enough machinery to build Cosmic as a seL4-style capability microkernel without turning the SIA CPU into an operating-system policy engine.
