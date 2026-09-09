# SIA Platform Specification

## Status

- Scope: machine-level contract surrounding the SIA ISA
- CPU architecture: [`SIA.md`](SIA.md), [`SIA32-P.md`](SIA32-P.md), [`SIA32-MMU.md`](SIA32-MMU.md), [`SIA32-MEM.md`](SIA32-MEM.md)
- First concrete profile: **Lighting**
- Status: **initial structure / TODO**

The SIA ISA deliberately does not define a complete computer.

The **SIA Platform Specification** defines reset, trap entry address, physical memory, interrupt controller, timer, ROM, MMIO, boot, device discovery, and machine-control behavior.

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

There is no CPU `TVEC`, `IENABLE`, or `IPENDING` register.

Those responsibilities are handled by fixed platform conventions and MMIO devices.

---

# 1. Design principles

The platform specification should be:

- small enough to implement faithfully in the Rust full-system VM;
- concrete enough that Cosmic never depends on emulator-only behavior;
- stable across hardware revisions;
- discoverable where variability helps;
- fixed where variability would only complicate boot/trap code;
- compatible with PLIO/QDX;
- suitable for a capability microkernel with user-level drivers;
- extensible to SMP without complicating first-generation Lighting.

The platform defines mechanism, not Cosmic scheduling or driver policy.

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

# 3. Reset and fixed vectors

Every platform profile must define two fixed addresses:

```text
RESET_VECTOR
TRAP_VECTOR
```

Neither is held in a writable CPU register.

## 3.1 `RESET_VECTOR`

On reset:

```text
mode       = S
STATUS.IE  = 0
STATUS.VM  = 0
PC         = RESET_VECTOR
```

`RESET_VECTOR` is therefore a physical address.

## 3.2 `TRAP_VECTOR`

Every synchronous exception, `TRAP`, and asynchronous platform interrupt enters at the single fixed `TRAP_VECTOR` defined by the platform profile.

When:

```text
STATUS.VM = 0
```

`TRAP_VECTOR` is used as a physical address.

When:

```text
STATUS.VM = 1
```

it is used as a virtual address under the current `VMCTX`.

Therefore a protected OS must map the trap entry identically in every active address space, normally:

```text
U=0
R=1
X=1
G=1
```

This is a deliberate architectural/platform contract.

A profile should choose `TRAP_VECTOR` so firmware can provide a valid target in physical mode and Cosmic can map the same numerical address globally after translation is enabled.

Typical arrangement:

```text
physical mode
    TRAP_VECTOR -> ROM trap/boot stub

protected Cosmic mode
    TRAP_VECTOR -> globally mapped Cosmic trap entry
```

The exact `Lighting-1` addresses are still open.

## TODO

- [ ] Freeze `Lighting-1` `RESET_VECTOR`.
- [ ] Freeze `Lighting-1` `TRAP_VECTOR`.
- [ ] Define trap-vector physical-mode backing.
- [ ] Define required Cosmic global mapping at `TRAP_VECTOR`.
- [ ] Freeze boot ROM physical base and size.
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
PLIO host/controller aperture
optional boot console/framebuffer
reserved regions
```

The map distinguishes coherent normal memory from MMIO.

## TODO

- [ ] Choose the `Lighting-1` physical address map.
- [ ] Define maximum physical memory for the first profile.
- [ ] Reserve ROM region.
- [ ] Reserve system-MMIO region.
- [ ] Reserve interrupt-controller region.
- [ ] Reserve timer region.
- [ ] Reserve PLIO aperture.
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

## TODO

- [ ] Decide fixed information-block address or firmware pointer mechanism.
- [ ] Define header/versioning.
- [ ] Define profile-ID namespace.
- [ ] Define machine/revision IDs.
- [ ] Define RAM discovery.
- [ ] Define feature discovery.
- [ ] Decide fixed-profile versus discovered devices.

---

# 7. Platform interrupt architecture

SIA32-P exposes only:

```text
STATUS.IE
CAUSE.INTERRUPT
single platform interrupt condition
```

The CPU does **not** contain:

```text
interrupt pending bitmap
per-source enable bitmap
interrupt priority state
interrupt source register
software-interrupt pending bit
 timer pending bit
```

All of that belongs to the platform interrupt controller.

## 7.1 Source flow

For Lighting:

```text
monotonic timer deadline
software-generated interrupt
PLIO Notification
direct platform device
        |
        v
Lighting interrupt controller
        |
        | pending
        | masking
        | priority
        | routing
        | claim / complete
        v
single CPU platform-interrupt condition
        |
        v
SIA CPU
        |
        v
TRAP_VECTOR
```

When the controller has at least one eligible source, it asserts the platform interrupt condition.

If `STATUS.IE=1`, the CPU may take the interrupt and records:

```text
CAUSE.INTERRUPT = 1
CAUSE.CODE      = PLATFORM_INTERRUPT
```

The source ID is **not** copied into `CAUSE`.

Cosmic identifies the source through the controller.

## 7.2 Claim/complete model

The controller should provide a very small MMIO claim/complete interface.

Conceptual registers:

```text
PENDING        read pending-source bitmap/status
ENABLE         source enable/mask state
CLAIM          read highest eligible pending source ID
COMPLETE       write completed source ID
THRESHOLD      optional priority threshold
SOFTINT        software interrupt generation
```

Exact register organization is not yet frozen.

Normal handler sequence:

```text
1. CPU traps to fixed TRAP_VECTOR.
2. STATUS.IE has been cleared by trap entry.
3. Cosmic saves required trap state.
4. Read CLAIM.
5. Dispatch claimed source.
6. Acknowledge/clear the underlying source as required.
7. Write COMPLETE.
8. SRET.
```

If another eligible source remains, the controller keeps/reasserts the CPU interrupt condition.

## 7.3 Pending while CPU interrupts are disabled

When `STATUS.IE=0`, controller source state is unchanged.

The controller may continue to record new pending events. They are delivered once CPU interrupt acceptance is re-enabled and the sources are eligible.

This makes CPU `IPENDING` unnecessary.

## 7.4 Per-source masking

All source enable/mask state resides in the controller.

This makes CPU `IENABLE` unnecessary.

The CPU only supplies the final global gate:

```text
STATUS.IE
```

## 7.5 Timer and software interrupts

The scheduling timer is an interrupt-controller source.

Software interrupts are also controller sources.

For a later SMP machine, IPIs are generated through controller/MMIO facilities and delivered as routed controller sources; they do not require new baseline CPU pending registers.

## 7.6 PLIO Notifications

PLIO device notifications enter the controller either:

- as individually assigned interrupt sources; or
- through one or more PLIO aggregate sources whose detailed notification is then claimed from the PLIO host.

The exact `Lighting-1` mapping remains to be defined after reviewing the PLIO Notification model.

## TODO

- [ ] Define interrupt-source ID width.
- [ ] Define maximum sources for `Lighting-1`.
- [ ] Define pending representation.
- [ ] Define enable/mask registers.
- [ ] Define `CLAIM` semantics.
- [ ] Define `COMPLETE` semantics.
- [ ] Decide programmable priorities versus fixed priority.
- [ ] Define tie breaking.
- [ ] Define optional threshold/nesting behavior.
- [ ] Define spurious claim value.
- [ ] Define edge/level source rules.
- [ ] Define timer source ID.
- [ ] Define software-interrupt source ID and generation.
- [ ] Define PLIO Notification mapping.
- [ ] Define reset state.
- [ ] Freeze MMIO layout.
- [ ] Implement identical Rust VM model.

---

# 8. Monotonic timer

A protected preemptive OS requires a stable monotonic time source and deadline mechanism.

Initial direction:

```text
64-bit monotonically increasing counter
64-bit deadline/compare
one-shot deadline source into interrupt controller
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
- [ ] Freeze MMIO layout.
- [ ] Decide per-CPU timer form for later SMP.
- [ ] Keep wall-clock/RTC separate.

---

# 9. Boot firmware contract

Initial Lighting boot direction:

```text
reset at RESET_VECTOR
  -> system ROM
  -> platform discovery
  -> initialize RAM / interrupt controller / timer / PLIO
  -> discover QDX boot block device
  -> load Cosmic
  -> establish global trap/kernel mappings
  -> transfer control to Cosmic
```

Possible Cosmic entry state:

```text
mode              Supervisor
STATUS.IE         0
STATUS.VM         explicitly documented
r1                 Platform Information Block pointer
r2                 boot-device identity
r3                 boot flags
remaining GPRs     unspecified
```

If Cosmic is entered with `STATUS.VM=1`, the firmware/kernel setup must already ensure that `TRAP_VECTOR` is valid under the installed `VMCTX`.

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
Notification -> interrupt-controller connection
protected-DMA relationship to RAM
reset/enumeration behavior
boot-device requirements
```

## TODO

- [ ] Freeze first PLIO host MMIO base.
- [ ] Define mandatory segment count.
- [ ] Define host reset state.
- [ ] Define enumeration order.
- [ ] Define Notification -> interrupt-controller mapping.
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

## TODO

- [ ] Decide whether `Lighting-1` requires a boot console.
- [ ] Define MMIO layout if present.
- [ ] Define its interrupt-controller source if interrupt-driven.
- [ ] Keep emulator semihosting explicitly non-architectural.

---

# 12. ROM and universal runtime

The physical platform exposes system ROM.

[`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md) defines stable global ROM libraries and shared allocator/runtime code.

The platform defines:

- physical ROM base/size;
- ROM revision identity;
- integrity/version metadata;
- relationship between boot ROM and runtime ROM;
- physical backing relevant to `RESET_VECTOR` and physical-mode `TRAP_VECTOR`.

Virtual global placement of universal ROM libraries remains a Cosmic/ABI decision.

## TODO

- [ ] Freeze ROM physical layout.
- [ ] Define ROM header/version identity.
- [ ] Define integrity/checksum mechanism.
- [ ] Decide boot-ROM versus runtime-ROM split.
- [ ] Define physical-mode trap stub arrangement.
- [ ] Define expansion/update compatibility.

---

# 13. DMA and coherency

For `Lighting-1`:

```text
normal RAM
    coherent for CPU access
    coherent for PLIO DMA
```

Drivers should not require explicit data-cache clean/invalidate operations.

PLIO protected DMA defines device authority; the CPU MMU does not translate device DMA addresses.

## TODO

- [ ] Freeze coherent-DMA statement.
- [ ] Define ordering between DMA completion and Notification/interrupt visibility.
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

- [ ] Define system-control MMIO block.
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

## TODO

- [ ] Define as a separate `Neutron-1`/SIA SMP platform profile later.

---

# 16. Rust full-system VM conformance

The Rust full-system VM must model:

```text
RESET_VECTOR
TRAP_VECTOR
ROM
RAM
physical bus
system MMIO
Platform Information Block
interrupt controller
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
source enabled + STATUS.IE=1 -> CPU traps to TRAP_VECTOR
CAUSE = PLATFORM_INTERRUPT
CLAIM returns real source
COMPLETE retires source
additional pending source retriggers correctly
```

## TODO

- [ ] Add named `Lighting-1` machine configuration.
- [ ] Boot from `RESET_VECTOR`.
- [ ] Test synchronous trap to `TRAP_VECTOR` in physical mode.
- [ ] Test trap to global `TRAP_VECTOR` under VM.
- [ ] Generate timer source through interrupt controller.
- [ ] Generate/claim/complete PLIO/device source.
- [ ] Enumerate PLIO/QDX.
- [ ] DMA from QDX block device.
- [ ] Load Cosmic through architectural boot path.
- [ ] Remove semihosting dependencies from normal full-system operation.

---

# 17. Immediate platform-definition order

```text
1. Lighting-1 physical memory map
2. RESET_VECTOR + TRAP_VECTOR + ROM layout
3. Platform Information Block
4. interrupt controller
5. monotonic timer
6. minimal boot console decision
7. PLIO host integration
8. firmware/Cosmic handoff
9. power/reset control
10. Rust VM implementation
```

---

# 18. Definition of `Lighting-1` platform complete

The first platform profile is complete when:

- [ ] reset state and `RESET_VECTOR` are frozen;
- [ ] fixed `TRAP_VECTOR` and its physical/global mapping contract are frozen;
- [ ] physical RAM/ROM/MMIO map is frozen;
- [ ] memory attributes are frozen;
- [ ] platform identification/discovery is frozen;
- [ ] interrupt-controller source/mask/claim/complete contract is frozen;
- [ ] monotonic timer is frozen;
- [ ] boot console is frozen or deliberately omitted;
- [ ] PLIO integration is frozen;
- [ ] Cosmic entry ABI is frozen;
- [ ] DMA/coherency rules are frozen;
- [ ] power/reset controls are sufficient;
- [ ] Rust VM can implement the machine without inventing unspecified behavior;
- [ ] Cosmic can boot entirely through documented platform mechanisms.
