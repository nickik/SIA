# SIA Platform Specification

## Status

- Scope: machine-level contract surrounding the SIA ISA
- CPU architecture: [`SIA.md`](SIA.md), [`SIA32-P.md`](SIA32-P.md), [`SIA32-MMU.md`](SIA32-MMU.md), [`SIA32-MEM.md`](SIA32-MEM.md)
- First concrete profile: **Lighting-1**
- Status: **partial normative Lighting-1 platform specification**

The SIA ISA deliberately does not define a complete computer.

The **SIA Platform Specification** defines reset, trap entry, the physical address space, interrupts, timers, ROM, MMIO, boot, device discovery, PLIO integration, and machine-control behavior.

The CPU remains deliberately small:

```text
SIA32-P CPU state
    STATUS
    EPC
    CAUSE
    BADADDR
    SCRATCH
    VMCTX
```

There is no CPU `TVEC`, `IENABLE`, or `IPENDING` register. Trap-vector placement and interrupt source state belong to the platform.

---

# 1. Design principles

The platform specification shall be:

- small enough to implement faithfully in the Rust full-system VM;
- concrete enough that Cosmic never depends on emulator-only behavior;
- stable across hardware revisions;
- discoverable where variability helps;
- fixed where variability would only complicate boot and trap code;
- designed around PLIO/QDX as the normal I/O architecture;
- suitable for a capability microkernel with user-level drivers;
- extensible to SMP without complicating first-generation Lighting.

The platform defines mechanism, not Cosmic scheduling or driver policy.

A major design rule for Lighting is:

> **Most I/O interrupts are PLIO Notifications. The Lighting interrupt controller identifies major machine subsystems, not individual QDX devices.**

---

# 2. Platform profiles

Initial profiles:

```text
SIA-Platform-Base
    common bootable protected-machine contract

Lighting-1
    first single-CPU workstation profile

Neutron-1
    later coherent SMP profile
```

A profile fixes concrete addresses, source counts, mandatory devices, limits, and boot behavior.

---

# 3. Lighting-1 reset and fixed vectors

Lighting-1 freezes:

```text
RESET_VECTOR = 0xFFFF0000
TRAP_VECTOR  = 0xFFFFF000
```

Neither address is held in a writable CPU register.

## 3.1 Reset vector

On reset:

```text
mode       = S
STATUS.IE  = 0
STATUS.VM  = 0
PC         = 0xFFFF0000
```

Because translation is disabled, `RESET_VECTOR` is a physical address into system ROM.

Lighting-1 reserves the top 64 KiB of physical address space as the initial system-ROM window:

```text
0xFFFF0000  RESET_VECTOR / ROM base
    ...
0xFFFFF000  physical-mode TRAP_VECTOR
    ...
0xFFFFFFFF  end of ROM window
```

The exact internal ROM contents and ABI are specified separately.

## 3.2 Trap vector

Every synchronous exception, `TRAP`, and asynchronous platform interrupt enters at:

```text
TRAP_VECTOR = 0xFFFFF000
```

When:

```text
STATUS.VM = 0
```

`0xFFFFF000` is interpreted physically and therefore enters a ROM trap/diagnostic stub.

When:

```text
STATUS.VM = 1
```

`0xFFFFF000` is interpreted virtually under the current `VMCTX`.

Cosmic shall map the trap-entry page identically in every active address space, normally:

```text
VA        = 0xFFFFF000
U         = 0
R         = 1
W         = 0
X         = 1
G         = 1
```

Thus protected-mode traps enter Cosmic directly through a global translation rather than branching through ROM on every trap or IPC.

The physical ROM target and protected-mode Cosmic target may therefore be different physical memory while sharing the same numerical architectural address.

## 3.3 Vector-page reservation

Lighting/Cosmic should reserve the high virtual region around the trap vector for stable system entry use.

At minimum:

```text
0xFFFFF000..0xFFFFF7FF   primary 2 KiB Cosmic trap-entry page
0xFFFFF800..0xFFFFFFFF   reserved for future fixed system-entry use
```

No user mapping may conflict with this global system region.

## Remaining TODO

- [x] Freeze `Lighting-1` `RESET_VECTOR`.
- [x] Freeze `Lighting-1` `TRAP_VECTOR`.
- [x] Define physical-mode trap backing as system ROM.
- [x] Define protected-mode global Cosmic trap mapping.
- [x] Freeze initial 64 KiB ROM window.
- [ ] Define cold-reset device state.
- [ ] Define warm-reset semantics.

---

# 4. Physical address space

The platform publishes a physical memory map containing at minimum:

```text
RAM
ROM
system-control MMIO
interrupt controller
monotonic timer
PLIO host/controller apertures
optional boot console/framebuffer
reserved regions
```

Lighting-1 already reserves the following high-address regions:

```text
0xFFF00000   Lighting interrupt controller
0xFFF01000   monotonic timer              (interface not yet frozen)
0xFFF02000   system control               (interface not yet frozen)
0xFFF03000   boot diagnostics/console     (decision not yet frozen)

0xFFFF0000   64 KiB system ROM
0xFFFFF000   RESET-independent physical trap entry inside ROM
```

Each system MMIO function receives at least a 4 KiB aperture even when its first implementation uses only a few registers. This keeps address decoding and future compatible expansion simple.

## TODO

- [ ] Freeze complete `Lighting-1` physical address map.
- [ ] Define maximum directly addressable RAM for the first profile.
- [x] Reserve initial system-MMIO window.
- [x] Reserve interrupt-controller aperture.
- [x] Reserve timer aperture.
- [ ] Freeze PLIO host aperture(s).
- [ ] Define holes/reserved ranges.
- [ ] Define behavior of unmapped physical accesses.
- [ ] Decide whether physical aliases are permitted.

---

# 5. Physical memory attributes

Initial physical memory classes:

```text
NORMAL
    coherent RAM suitable for page tables and DMA

ROM
    normal readable/executable immutable memory

MMIO
    strongly ordered, non-speculative device memory

RESERVED
    no architectural target
```

`SIA32-MEM` defines ordering semantics; the platform assigns classes to ranges.

## TODO

- [ ] Freeze memory-region attribute model.
- [ ] Define whether ROM is cacheable as normal memory.
- [ ] Define executable-MMIO prohibition.
- [ ] Define page-table eligibility: NORMAL coherent RAM only.
- [ ] Define DMA-visible memory classes.

---

# 6. Platform identification and discovery

A minimal read-only **Platform Information Block** should describe variable platform properties without requiring a modern device-tree-style environment.

Candidate fields:

```text
signature
platform-specification version
platform profile ID
machine/revision ID
RAM descriptors
ROM descriptors
CPU count
clock/timer frequency
PLIO host location
interrupt-controller location
feature bits
additional-data pointer
checksum/version
```

Fixed Lighting-1 addresses do not need to be redundantly discovered unless the information block is also intended as convenient descriptive metadata.

## TODO

- [ ] Decide fixed information-block address or firmware pointer mechanism.
- [ ] Define header/versioning.
- [ ] Define profile-ID namespace.
- [ ] Define machine/revision IDs.
- [ ] Define RAM discovery.
- [ ] Define feature discovery.
- [ ] Decide fixed-profile versus discovered devices.

---

# 7. Lighting-1 interrupt architecture

SIA32-P exposes only:

```text
STATUS.IE
CAUSE.INTERRUPT
one platform-interrupt condition
```

The CPU contains no source bitmap, per-source mask, priority register, source identifier, timer-pending bit, or software-interrupt pending bit.

All source management belongs to the Lighting interrupt controller and, for ordinary I/O, PLIO.

## 7.1 PLIO-centric hierarchy

Most Lighting I/O does not receive a separate central interrupt-controller source.

The intended hierarchy is:

```text
QDX disk
QDX GNet
QDX graphics
keyboard/audio/etc.
        |
        v
PLIO Notification mechanism
        |
        v
PLIO host controller
        |
        | one aggregate condition per PLIO segment
        v
Lighting interrupt controller
        |
        v
one SIA platform-interrupt condition
        |
        v
SIA CPU -> TRAP_VECTOR
```

The Lighting interrupt controller answers:

> **Which major machine subsystem requires attention?**

PLIO then answers:

> **Which device/queue/Notification inside that segment requires attention?**

The central controller shall not duplicate detailed QDX/PLIO Notification state.

## 7.2 Source namespace

Lighting-1 defines a 4-bit / 16-value source namespace.

Source zero means no interrupt and is never enabled.

Initial allocation:

```text
ID    Source
--    ------------------------------------------------
0     NONE / no eligible source
1     MACHINE       fatal/urgent platform condition
2     TIMER         monotonic scheduling deadline
3     SOFTWARE      software-generated platform interrupt
4     PLIO0         PLIO segment 0 has eligible Notification(s)
5     PLIO1         PLIO segment 1 has eligible Notification(s)
6-15  reserved for compatible Lighting platform growth
```

Lighting-1 therefore has only five architecturally meaningful interrupt sources despite potentially containing many QDX devices.

Additional PLIO segments may consume reserved source IDs in later compatible profiles.

## 7.3 Priority

Lighting-1 has **fixed priority**.

Lower nonzero source ID has higher priority:

```text
MACHINE > TIMER > SOFTWARE > PLIO0 > PLIO1 > reserved future sources
```

There is no programmable priority or threshold register in Lighting-1.

This requires only a small fixed priority encoder.

Individual I/O-device prioritization remains a PLIO/Cosmic concern rather than a central interrupt-controller function.

## 7.4 Controller base and MMIO interface

Lighting-1 freezes:

```text
INTC_BASE = 0xFFF00000
```

The controller occupies a 4 KiB MMIO aperture.

Core registers:

```text
offset   register    access   meaning
------   ----------  -------  ----------------------------------------------
0x00     PENDING     RO       pending-source bitmap
0x04     ENABLE      RW       enabled-source bitmap
0x08     CLAIM       RO*      claim highest-priority eligible source
0x0C     COMPLETE    WO*      complete previously claimed source
0x10     SOFTINT     WO       generate software source
```

All registers are accessed as 32-bit MMIO words. Only bits 0..15 are defined for `PENDING` and `ENABLE`; upper bits read zero and shall be ignored on writes.

`CLAIM`, `COMPLETE`, and `SOFTINT` use the low source-ID bits; other bits are reserved.

## 7.5 `PENDING`

`PENDING[N] = 1` means source `N` currently has pending work.

`PENDING` reports source state independently of `ENABLE`.

Examples:

```text
bit 1   MACHINE
bit 2   TIMER
bit 3   SOFTWARE
bit 4   PLIO0
bit 5   PLIO1
```

Source zero always reads zero.

For a level-sensitive source, the pending bit reflects the asserted source condition subject to active-claim suppression.

For a latched/edge source, the pending bit remains set until the source-specific acknowledgement rules consume it.

## 7.6 `ENABLE`

`ENABLE[N]` controls whether source `N` is eligible to assert the CPU platform-interrupt condition.

```text
ENABLE[N] = 1    source enabled
ENABLE[N] = 0    source masked
```

Source zero is permanently disabled.

At reset:

```text
ENABLE = 0
```

Masking a source does not discard its pending state.

## 7.7 Eligible source

Ignoring an active claim, a source is eligible when:

```text
PENDING[N] = 1
AND
ENABLE[N]  = 1
```

The controller asserts the SIA platform-interrupt condition whenever at least one eligible source exists and there is no active claim blocking delivery in Lighting-1.

The CPU takes the interrupt only when its final global gate also permits it:

```text
STATUS.IE = 1
```

On interrupt entry the CPU records only:

```text
CAUSE.INTERRUPT = 1
CAUSE.CODE      = PLATFORM_INTERRUPT
```

The source ID is not copied into `CAUSE`.

## 7.8 `CLAIM`

Reading `CLAIM` is an operation.

It returns:

```text
0       no eligible source
1..15   claimed source ID
```

For a nonzero result, the controller:

1. chooses the highest-priority eligible source;
2. marks that source active;
3. prevents the same source from being claimed again until completion;
4. records a single active claim for the CPU.

Lighting-1 deliberately permits only **one active claimed source** at a time. It does not support nested interrupt-controller claims.

This is independent of synchronous exceptions that may still occur while Supervisor code runs.

## 7.9 `COMPLETE`

Writing a valid active source ID to `COMPLETE` releases the active claim.

After completion the controller immediately reevaluates all sources.

If another source is eligible, or if the completed level-sensitive source is still asserted, the platform-interrupt condition becomes eligible again.

Writing zero or a source ID that is not the active claim has no useful completion effect and should be treated as a software error; exact diagnostic behavior may be defined later.

## 7.10 `SOFTINT`

Writing a nonzero value to `SOFTINT` sets source `3` (`SOFTWARE`) pending.

The software source is latched until claimed/completed according to the controller's software-source rules.

On single-CPU Lighting this is useful for testing, deferred kernel work, and explicit self-notification.

Later SMP profiles shall define richer routed IPIs separately without adding CPU privileged registers.

## 7.11 Level and latched source behavior

Lighting-1 normalizes both source styles behind the same claim/complete interface.

### Level source

For level-sensitive sources such as a PLIO aggregate:

```text
underlying condition asserted
    -> PENDING = 1

CLAIM
    -> source active / temporarily suppressed

COMPLETE
    -> if underlying condition still asserted,
       source immediately becomes pending/eligible again
```

The device/subsystem must therefore clear or drain the underlying cause before completing the central source if it does not want immediate retriggering.

### Latched source

For edge/event sources, platform hardware latches the event into pending state until the defined source acknowledgement path consumes it.

The source-specific specification defines exactly when its pending latch clears.

## 7.12 PLIO aggregate sources

`PLIO0` and `PLIO1` are level-sensitive summary sources.

Conceptually:

```text
PLIOx has at least one eligible pending Notification
        -> INTC PENDING[PLIOx] = 1
```

The Lighting interrupt controller does not know which QDX device generated the Notification.

Normal Cosmic flow:

```text
INTC CLAIM
    -> PLIOx

PLIO Notification claim/query
    -> specific slot/device/Notification

handle or dispatch Notification
complete/acknowledge PLIO Notification

repeat while useful

INTC COMPLETE PLIOx
```

Cosmic may drain multiple PLIO Notifications during one central interrupt claim. A software work limit may be used to prevent sustained I/O from monopolizing CPU time; that limit is scheduling policy, not platform architecture.

If eligible Notifications remain after `INTC COMPLETE`, the PLIO summary source immediately becomes eligible again.

## 7.13 Normal interrupt path

```text
platform/PLIO/timer event
        |
        v
INTC source pending
        |
        v
source enabled?
        |
        v
single CPU interrupt condition
        |
        v
STATUS.IE == 1
        |
        v
CPU trap:
    EPC        = interrupted PC
    CAUSE      = PLATFORM_INTERRUPT
    STATUS.PIE = STATUS.IE
    STATUS.IE  = 0
    mode       = S
    PC         = 0xFFFFF000
        |
        v
Cosmic:
    SSWAP sp, SCRATCH
    save required state
    source = INTC.CLAIM
    dispatch major source
    service/acknowledge underlying source
    INTC.COMPLETE = source
    restore state
    SSWAP sp, SCRATCH
    SRET / SRETCTX
```

The CPU architecture therefore performs no per-device interrupt decoding.

## 7.14 Reset state

On cold reset:

```text
ENABLE        = 0
active claim  = NONE
SOFTWARE      = not pending
```

Pending level sources reflect their underlying hardware state after those devices leave reset.

Additional reset details remain source-specific.

## Interrupt TODO

- [x] Define source-ID width: 4 bits / 16 values.
- [x] Define Lighting-1 central source allocation.
- [x] Define PLIO aggregate model.
- [x] Define `PENDING`.
- [x] Define `ENABLE`.
- [x] Define `CLAIM`.
- [x] Define `COMPLETE`.
- [x] Define fixed priority.
- [x] Define no nested controller claims in Lighting-1.
- [x] Define spurious/no-source claim value = 0.
- [x] Define level/latched source model.
- [x] Define timer source ID.
- [x] Define software-interrupt source ID and generation.
- [x] Freeze controller MMIO base and core register layout.
- [ ] Define exact MACHINE-source semantics.
- [ ] Synchronize PLIO Notification register semantics with the PLIO/QDX specification.
- [ ] Implement identical model in Rust VM.

---

# 8. Monotonic timer

A protected preemptive OS requires a stable monotonic time source and deadline mechanism.

Initial direction:

```text
64-bit monotonically increasing counter
64-bit deadline/compare
one-shot deadline source into interrupt controller
```

Lighting-1 fixes the timer's central interrupt-controller source as:

```text
INTC source 2 = TIMER
```

There is no CPU timer register and no CPU timer-pending bit.

Periodic behavior is normally synthesized in software.

## TODO

- [ ] Define counter frequency or discovery mechanism.
- [ ] Define reset/start value.
- [ ] Define stable 64-bit reads on a 32-bit CPU.
- [ ] Define deadline programming.
- [ ] Define past-deadline behavior.
- [ ] Define timer-source pending behavior.
- [ ] Define acknowledgement/rearm semantics.
- [ ] Define wraparound behavior.
- [ ] Freeze MMIO layout at `0xFFF01000`.
- [ ] Decide per-CPU timer form for later SMP.
- [ ] Keep wall-clock/RTC separate.

---

# 9. Boot firmware contract

Initial Lighting boot direction:

```text
reset at 0xFFFF0000
  -> system ROM
  -> platform discovery
  -> initialize RAM / interrupt controller / timer / PLIO
  -> discover QDX boot block device
  -> load Cosmic
  -> establish global trap/kernel mappings
  -> map Cosmic trap entry at 0xFFFFF000
  -> transfer control to Cosmic
```

Possible Cosmic entry state:

```text
mode              Supervisor
STATUS.IE         0
STATUS.VM         explicitly documented
r1                Platform Information Block pointer
r2                boot-device identity
r3                boot flags
remaining GPRs    unspecified
```

If Cosmic is entered with `STATUS.VM=1`, firmware/kernel setup must already ensure that `TRAP_VECTOR` is valid under the installed `VMCTX`.

## TODO

- [ ] Define firmware reset contract.
- [ ] Define Cosmic kernel entry contract.
- [ ] Decide VM-on versus VM-off handoff.
- [ ] Define boot argument registers.
- [ ] Define boot-device identity.
- [ ] Define boot image format dependency.
- [ ] Define ROM/runtime ABI relationship.
- [ ] Prefer no permanent privileged firmware dependency after Cosmic owns the machine.

---

# 10. PLIO platform integration

The platform profile defines:

```text
PLIO host/controller physical address
number of initial segments
Notification -> PLIO aggregate source relationship
protected-DMA relationship to RAM
reset/enumeration behavior
boot-device requirements
```

Lighting-1 central interrupt assignments are already frozen:

```text
PLIO segment 0 -> INTC source 4
PLIO segment 1 -> INTC source 5
```

Detailed device notification identity remains inside the PLIO host and PLIO/QDX protocol.

## TODO

- [ ] Freeze first PLIO host MMIO base.
- [ ] Decide whether Lighting-1 mandates one or two populated PLIO segments.
- [ ] Define host reset state.
- [ ] Define enumeration order.
- [x] Define Notification -> central interrupt aggregation principle.
- [ ] Synchronize PLIO Notification claim/completion semantics with central claim lifetime.
- [ ] Define protected-DMA integration.
- [ ] Define coherent-DMA behavior required by `SIA32-MEM`.
- [ ] Define mandatory QDX boot-block profile.

---

# 11. Console and early diagnostics

A minimal boot-console device may be useful before the full QDX stack exists.

Potential first profile:

```text
simple MMIO console
    transmit byte
    receive byte
    status
```

It is a platform/debug device, not an ISA facility.

If the console becomes a normal interrupting peripheral rather than a boot-only debug path, preference is to attach it through PLIO/QDX rather than consume a permanent central interrupt source.

## TODO

- [ ] Decide whether `Lighting-1` requires a boot console.
- [ ] Define MMIO layout if present.
- [ ] Prefer polling for minimal ROM diagnostics unless interrupts are materially useful.
- [ ] Keep emulator semihosting explicitly non-architectural.

---

# 12. ROM and universal runtime

The physical platform exposes system ROM.

[`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md) defines stable global ROM libraries and shared allocator/runtime code.

Lighting-1 freezes the initial physical ROM window:

```text
0xFFFF0000..0xFFFFFFFF   64 KiB
```

The platform must additionally define:

- ROM revision identity;
- integrity/version metadata;
- relationship between boot ROM and universal runtime ROM;
- physical backing relevant to reset and physical-mode trap handling.

Virtual global placement of universal ROM libraries remains a Cosmic/ABI decision except for the fixed Cosmic trap entry at `0xFFFFF000`.

## TODO

- [x] Freeze initial ROM physical base and size.
- [ ] Define ROM header/version identity.
- [ ] Define integrity/checksum mechanism.
- [ ] Decide boot-ROM versus runtime-ROM split.
- [ ] Define physical-mode trap stub contents/contract.
- [ ] Define expansion/update compatibility.

---

# 13. DMA and coherency

For Lighting-1:

```text
normal RAM
    coherent for CPU access
    coherent for PLIO DMA
```

Drivers shall not require explicit data-cache clean/invalidate operations for baseline PLIO DMA.

PLIO protected DMA defines device authority; the CPU MMU does not translate device DMA addresses.

## TODO

- [ ] Freeze coherent-DMA statement in the platform profile.
- [ ] Define ordering between DMA completion and PLIO Notification visibility.
- [ ] Define ordering between CPU descriptor stores and device observation.

---

# 14. Power and reset control

Potential system-control operations:

```text
warm reset
cold reset request
power-off request
machine identification/status
watchdog later
```

## TODO

- [ ] Define system-control MMIO block at `0xFFF02000`.
- [ ] Define reset-request semantics.
- [ ] Define power-off semantics.
- [ ] Decide watchdog support.

---

# 15. SMP platform extensions — later

A later SMP profile adds:

```text
CPU IDs/count
secondary CPU reset/start
startup mailbox/address
interrupt-controller routing
software IPIs
per-CPU timers
coherent memory
TLB shootdown protocol
CPU halt/park/restart
```

Baseline CPU privileged state remains unchanged.

The Lighting-1 single-active-claim controller need not directly scale to Neutron; the common software-visible philosophy may be retained while SMP routing is specified separately.

## TODO

- [ ] Define as a separate `Neutron-1`/SIA SMP platform profile later.

---

# 16. Rust full-system VM conformance

The Rust full-system VM must model:

```text
RESET_VECTOR = 0xFFFF0000
TRAP_VECTOR  = 0xFFFFF000
ROM
RAM
physical bus
system MMIO
Platform Information Block
Lighting interrupt controller
monotonic timer
boot console if present
PLIO host
QDX boot block device
DMA
power/reset controls
```

Required interrupt tests include:

```text
source becomes pending
source masked -> no CPU interrupt
source enabled + STATUS.IE=0 -> remains pending, no trap
source enabled + STATUS.IE=1 -> CPU traps to 0xFFFFF000
CAUSE = PLATFORM_INTERRUPT
CLAIM returns highest-priority major source
PLIO source -> PLIO supplies detailed Notification identity
COMPLETE retires central claim
level source still asserted -> retriggers
additional pending source retriggers correctly
```

## TODO

- [ ] Add named `Lighting-1` machine configuration.
- [ ] Boot from `0xFFFF0000`.
- [ ] Test synchronous trap to `0xFFFFF000` in physical mode.
- [ ] Test trap to global `0xFFFFF000` under VM.
- [ ] Generate and claim timer source.
- [ ] Generate and claim PLIO aggregate source.
- [ ] Enumerate PLIO/QDX.
- [ ] DMA from QDX block device.
- [ ] Load Cosmic through architectural boot path.
- [ ] Remove semihosting dependencies from normal full-system operation.

---

# 17. Immediate platform-definition order

With vectors and the central interrupt controller now frozen, continue in this order:

```text
1. complete Lighting-1 physical memory map
2. monotonic timer
3. PLIO host integration / Notification claim contract
4. Platform Information Block
5. ROM header + boot/runtime split
6. minimal boot console decision
7. firmware -> Cosmic handoff
8. power/reset control
9. Rust VM implementation
```

---

# 18. Definition of `Lighting-1` platform complete

The first platform profile is complete when:

- [x] `RESET_VECTOR` is frozen;
- [x] fixed `TRAP_VECTOR` and physical/global mapping contract are frozen;
- [ ] complete physical RAM/ROM/MMIO map is frozen;
- [ ] memory attributes are frozen;
- [ ] platform identification/discovery is frozen;
- [x] central interrupt-controller source/mask/claim/complete contract is frozen;
- [x] PLIO-centric central interrupt aggregation is frozen;
- [ ] detailed PLIO Notification claim/completion contract is synchronized;
- [ ] monotonic timer is frozen;
- [ ] boot console is frozen or deliberately omitted;
- [ ] PLIO host integration is frozen;
- [ ] Cosmic entry ABI is frozen;
- [ ] DMA/coherency rules are frozen;
- [ ] power/reset controls are sufficient;
- [ ] Rust VM can implement the machine without inventing unspecified behavior;
- [ ] Cosmic can boot entirely through documented platform mechanisms.
