# SIA Platform Specification

## Status

- Scope: machine-level contract surrounding the SIA ISA
- CPU architecture: [`SIA.md`](SIA.md), [`SIA32-P.md`](SIA32-P.md), [`SIA32-MMU.md`](SIA32-MMU.md), [`SIA32-MEM.md`](SIA32-MEM.md)
- First concrete profile: **Lighting**
- Status: **initial structure / TODO**

The SIA instruction architecture deliberately does not define a complete computer.

The **SIA Platform Specification** defines the machine environment that operating systems, firmware, boot loaders, and full-system virtual machines may rely upon regardless of the internal implementation of the CPU.

Conceptually:

```text
SIA ISA
    registers
    instructions
    privilege
    MMU
    memory ordering

        +

SIA Platform
    reset
    physical address space
    RAM / ROM / MMIO
    timer
    interrupts
    platform identification
    boot contract
    device discovery
    power/reset control
    DMA/coherency environment

        =

bootable SIA computer
```

The same architectural CPU may therefore appear in different platform profiles:

```text
Lighting workstation
Neutron SMP
small embedded SIA system
future SIA server
```

A platform profile selects concrete addresses, mandatory devices, limits, and boot behavior without changing the ISA.

---

# 1. Design principles

The platform specification should be:

- small enough to implement faithfully in the Rust full-system VM;
- concrete enough that Cosmic never depends on emulator-only behavior;
- stable across hardware revisions;
- discoverable where variability is useful;
- fixed where variability would merely complicate boot software;
- compatible with PLIO/QDX as the normal peripheral architecture;
- suitable for a capability microkernel with user-level drivers;
- independent of any one firmware implementation;
- extensible to SMP without burdening the first single-CPU Lighting system.

The platform specification should define mechanisms, not require a particular Cosmic policy.

---

# 2. Platform profiles

The generic specification defines common rules and discovery structures.

Concrete profiles define mandatory implementations.

Initial profiles should be:

```text
SIA-Platform-Base
    minimum bootable protected SIA machine contract

Lighting-1
    first single-CPU workstation profile

Neutron-1                 later
    coherent SMP server profile
```

`Lighting-1` is the first profile that must be executable in the Rust VM.

---

# 3. Reset and initial CPU state

The platform must define:

- physical reset-vector address;
- reset-vector alignment;
- reset privilege state;
- initial translation state;
- initial interrupt state;
- initial stack policy, if any;
- whether RAM contents are undefined or cleared;
- warm reset versus cold reset;
- reset behavior of MMIO devices;
- reset behavior of PLIO and QDX devices;
- whether boot begins in ROM directly or through a small fixed reset stub.

The CPU-level reset state remains defined by `SIA32-P`; this document supplies the platform addresses and device behavior.

## TODO

- [ ] Freeze `Lighting-1` reset vector.
- [ ] Freeze boot ROM physical base and size.
- [ ] Define cold-reset device state.
- [ ] Define warm-reset semantics.
- [ ] Define firmware handoff after reset.

---

# 4. Physical address space

The platform must publish a physical memory map containing at minimum:

```text
RAM
ROM
system-control MMIO
interrupt controller
monotonic timer
PLIO host/controller aperture
optional framebuffer / boot console aperture
reserved regions
```

The map must distinguish normal coherent memory from MMIO.

The map should leave deliberate expansion windows rather than assigning every address in the first profile.

## TODO

- [ ] Choose the `Lighting-1` physical address map.
- [ ] Define maximum directly addressable physical memory for the first profile.
- [ ] Reserve ROM region.
- [ ] Reserve system MMIO region.
- [ ] Reserve PLIO aperture.
- [ ] Define holes/reserved ranges.
- [ ] Define behavior of unmapped physical accesses.
- [ ] Define whether aliases are permitted.

---

# 5. Physical memory attributes

Every physical region belongs to a platform memory class.

Initial classes should be:

```text
NORMAL
    coherent RAM / ROM-like normal memory semantics

ROM
    normal readable/executable memory, not writable by ordinary stores

MMIO
    strongly ordered, non-speculative device memory

RESERVED
    no architectural target
```

`SIA32-MEM` defines the software-visible ordering rules.

The platform defines which physical ranges have which class.

## TODO

- [ ] Freeze the memory-region attribute model.
- [ ] Define whether ROM is cacheable as normal memory.
- [ ] Define whether executable MMIO is always prohibited.
- [ ] Define page-table eligibility: only NORMAL coherent RAM.
- [ ] Define DMA-visible memory classes.

---

# 6. Platform identification and discovery

Software needs a stable way to identify the platform and discover variable properties.

A minimal read-only **Platform Information Block** should be considered.

Potential fields:

```text
signature
platform architecture version
platform profile ID
machine/revision ID
RAM region descriptors
ROM region descriptors
CPU count
clock/timer frequency
PLIO host location
interrupt-controller location
feature bits
pointer to additional discovery data
CRC/versioning
```

The first implementation may place this block at a fixed physical/ROM address.

The design should be much smaller than a modern firmware/device-tree environment while still preventing hard-coded configuration from spreading through Cosmic.

## TODO

- [ ] Decide fixed platform-information address or firmware pointer mechanism.
- [ ] Define Platform Information Block header.
- [ ] Define versioning rules.
- [ ] Define profile ID namespace.
- [ ] Define machine/revision IDs.
- [ ] Define RAM-region discovery.
- [ ] Define optional feature discovery.
- [ ] Decide which devices are fixed by profile versus discovered.

---

# 7. Interrupt architecture

The CPU sees the architectural classes from `SIA32-P`:

```text
SOFTWARE_INTERRUPT
TIMER_INTERRUPT
EXTERNAL_INTERRUPT
```

The platform must define how real interrupt sources become those classes.

For Lighting, ordinary device notifications should flow conceptually as:

```text
QDX / PLIO device
        |
        v
PLIO Notification
        |
        v
Lighting interrupt controller
        |
        v
SIA EXTERNAL_INTERRUPT
```

The interrupt controller should be deliberately small and optimized for claim/complete behavior.

## TODO

- [ ] Define interrupt-source namespace.
- [ ] Define maximum sources for `Lighting-1`.
- [ ] Define pending bits.
- [ ] Define per-source enable/mask.
- [ ] Define claim/identify operation.
- [ ] Define complete/acknowledge operation.
- [ ] Define priority semantics or explicitly specify no programmable priority.
- [ ] Define tie-breaking between simultaneous sources.
- [ ] Define spurious-interrupt behavior.
- [ ] Define level versus edge semantics visible to software.
- [ ] Define PLIO Notification mapping.
- [ ] Define software-interrupt generation.
- [ ] Define CPU-side interaction with `IENABLE` / `IPENDING`.
- [ ] Define reset state.
- [ ] Freeze MMIO layout.
- [ ] Implement identical model in Rust VM.

---

# 8. Monotonic timer

A protected preemptive OS requires a stable monotonic time source and scheduling deadline mechanism.

The initial direction is:

```text
64-bit monotonically increasing counter
64-bit deadline/compare register
one-shot timer interrupt
```

Periodic scheduling should normally be synthesized in software rather than requiring a separate periodic mode.

## TODO

- [ ] Define counter frequency or discovery mechanism.
- [ ] Define counter start/reset value.
- [ ] Define atomic read semantics on a 32-bit CPU.
- [ ] Define compare/deadline programming.
- [ ] Define behavior for deadlines already in the past.
- [ ] Define timer-pending semantics.
- [ ] Define acknowledgement/rearm behavior.
- [ ] Define wraparound behavior.
- [ ] Define MMIO register layout.
- [ ] Define reset state.
- [ ] Decide whether timer is per-CPU only in SMP profiles.
- [ ] Keep wall-clock/RTC semantics separate.

---

# 9. Boot firmware contract

The platform defines how software gets from reset to an operating-system image without making a specific firmware implementation architectural.

The first Lighting direction is:

```text
reset
  -> system ROM
  -> platform discovery
  -> initialize RAM / console / PLIO
  -> discover boot QDX block device
  -> load Cosmic
  -> transfer control to Cosmic
```

The boot contract should define what Cosmic may assume at entry.

Possible handoff state:

```text
mode              Supervisor
VM                disabled or explicitly documented
interrupts         disabled
r1                 pointer to Platform Information Block
r2                 boot-device identifier/handle
r3                 optional boot flags
remaining GPRs     unspecified
```

This is not yet frozen.

## TODO

- [ ] Define firmware entry/reset contract.
- [ ] Define Cosmic kernel entry contract.
- [ ] Decide VM-on versus VM-off kernel handoff.
- [ ] Define boot argument registers.
- [ ] Define boot-device identity.
- [ ] Define firmware error/recovery behavior.
- [ ] Define boot image format dependency.
- [ ] Define ROM/runtime ABI relationship.
- [ ] Define firmware services, if any, available after OS entry.
- [ ] Prefer no permanent privileged firmware runtime dependency once Cosmic owns the machine.

---

# 10. PLIO platform integration

PLIO/QDX remain separately specified peripheral architectures.

The SIA platform profile must define only their machine integration:

```text
PLIO host/controller physical address
number of initial segments
interrupt/Notification connection
DMA relationship to physical memory
reset/enumeration behavior
boot-device discovery requirements
```

## TODO

- [ ] Freeze first PLIO host MMIO base.
- [ ] Define number of mandatory PLIO segments for `Lighting-1`.
- [ ] Define host-controller reset state.
- [ ] Define enumeration order.
- [ ] Define Notification -> interrupt-controller mapping.
- [ ] Define protected-DMA integration.
- [ ] Define DMA coherency as required by `SIA32-MEM`.
- [ ] Define mandatory QDX boot-block profile.
- [ ] Define hot-plug behavior later if required.

---

# 11. Console and early diagnostics

The full machine should be debuggable before the complete QDX software stack exists.

A minimal architectural boot-console device may be worthwhile even if normal operation uses PLIO/QDX devices.

Possible first profile:

```text
simple MMIO UART-like console
    transmit byte
    receive byte
    status
```

It should remain a platform/debug device rather than become part of the SIA ISA.

## TODO

- [ ] Decide whether `Lighting-1` requires a minimal boot UART.
- [ ] Define MMIO layout if yes.
- [ ] Define interrupt behavior.
- [ ] Define whether firmware may use display/keyboard instead.
- [ ] Define emulator semihosting as explicitly non-architectural debug-only functionality.

---

# 12. ROM and universal-runtime mapping

The physical platform exposes system ROM.

`SIA-ROM-RUNTIME.md` defines the software strategy for stable universal ROM libraries and shared allocator/runtime code.

The platform profile must define:

- physical ROM base;
- physical ROM size;
- update/revision identity;
- integrity/version metadata;
- relationship between boot ROM and universal runtime ROM;
- whether one ROM image contains both or they are separate physical regions.

Virtual placement of global ROM mappings is a Cosmic/ABI decision, not a physical-platform requirement.

## TODO

- [ ] Freeze ROM physical layout.
- [ ] Define ROM header/version identity.
- [ ] Define integrity/checksum mechanism.
- [ ] Decide boot-ROM versus runtime-ROM split.
- [ ] Define expansion/update compatibility rules.

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

- [ ] Freeze exact platform statement of coherent DMA.
- [ ] Define ordering between DMA completion and interrupt/Notification visibility.
- [ ] Define ordering between CPU descriptor stores and device observation.
- [ ] Define power-loss/durable-storage boundary only in QDX/storage specifications.

---

# 14. Power and reset control

A real platform needs a minimal machine-control interface.

Potential operations:

```text
warm reset
cold reset request
power-off request where hardware supports it
machine identification/status
watchdog later
```

## TODO

- [ ] Define system-control MMIO block.
- [ ] Define reset request semantics.
- [ ] Define power-off semantics.
- [ ] Decide watchdog support.
- [ ] Define machine-check/fatal-error reporting later if needed.

---

# 15. SMP platform extensions — later

The baseline Lighting profile is single-CPU.

A later SMP platform specification must add:

```text
CPU count / IDs
secondary CPU reset/start
startup address/mailbox
IPIs
per-CPU timer behavior
per-CPU interrupt routing
coherent memory contract
TLB-shootdown support
CPU halt/park/restart
```

These should not complicate `Lighting-1`.

## TODO

- [ ] Define as a separate `SIA-SMP-PLATFORM` or `Neutron-1` profile later.

---

# 16. Rust full-system VM conformance

The Rust SIA VM becomes the first executable implementation of the Platform Specification.

It must model:

```text
reset
ROM
RAM
physical bus
system MMIO
platform-information block
interrupt controller
timer
boot console
PLIO host
QDX boot block device
DMA
power/reset controls
```

No normal Cosmic behavior may depend on host services not represented by an architectural device or firmware interface.

## TODO

- [ ] Add a named `Lighting-1` machine configuration.
- [ ] Boot from architectural reset vector.
- [ ] Execute ROM firmware.
- [ ] Discover platform information.
- [ ] Generate timer interrupt.
- [ ] Generate/claim/complete external interrupt.
- [ ] Enumerate PLIO/QDX.
- [ ] DMA from QDX block device.
- [ ] Load Cosmic through the architectural boot path.
- [ ] Remove semihosting dependencies from normal full-system operation.

---

# 17. Immediate platform-definition order

The first platform work should proceed in this order:

```text
1. Lighting-1 physical memory map
2. reset vector + ROM layout
3. Platform Information Block
4. interrupt controller
5. monotonic timer
6. minimal boot console
7. PLIO host integration
8. boot firmware handoff
9. power/reset control
10. Rust VM implementation
```

The first six items are sufficient to begin serious full-system VM and early Cosmic bring-up before the complete storage and graphics stacks exist.

---

# 18. Definition of `Lighting-1` platform complete

The first platform profile is complete when:

- [ ] reset state and reset vector are frozen;
- [ ] physical RAM/ROM/MMIO map is frozen;
- [ ] memory attributes are frozen;
- [ ] platform identification/discovery is frozen;
- [ ] interrupt controller is frozen;
- [ ] monotonic timer is frozen;
- [ ] boot console is frozen or deliberately omitted;
- [ ] PLIO host integration is frozen;
- [ ] boot-device discovery is frozen;
- [ ] Cosmic entry/handoff ABI is frozen;
- [ ] DMA/coherency rules are frozen;
- [ ] power/reset controls are sufficient;
- [ ] the Rust VM can implement the machine without inventing unspecified platform behavior;
- [ ] Cosmic can boot entirely through documented platform mechanisms.
