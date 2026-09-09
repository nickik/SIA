# SIA Platform Specification

## Status

- Scope: machine-level contract surrounding the SIA ISA
- CPU architecture: [`SIA.md`](SIA.md), [`SIA32-P.md`](SIA32-P.md), [`SIA32-MMU.md`](SIA32-MMU.md), [`SIA32-MEM.md`](SIA32-MEM.md)
- First concrete profile: **Lighting-1**
- Status: **partial normative Lighting-1 platform specification**

The SIA ISA deliberately does not define a complete computer. The **SIA Platform Specification** defines reset, trap entry, physical memory, MMIO, interrupts, timers, ROM, PLIO integration, boot, discovery, and machine control.

The protected CPU itself remains deliberately small:

```text
STATUS
EPC
CAUSE
BADADDR
SCRATCH
VMCTX
```

There is no CPU `TVEC`, `IENABLE`, or `IPENDING`. Trap placement and interrupt-source state are platform responsibilities.

A central Lighting design rule is:

> **Most I/O is PLIO/QDX. The central interrupt controller identifies major machine subsystems, while PLIO identifies individual I/O Notifications.**

---

# 1. Platform profiles

```text
SIA-Platform-Base
    common protected-machine contract

Lighting-1
    first single-CPU workstation profile

Neutron-1
    later coherent SMP profile
```

A profile fixes concrete addresses, mandatory devices, limits, and boot behavior without changing the SIA ISA.

---

# 2. Lighting-1 fixed vectors

Lighting-1 freezes:

```text
RESET_VECTOR = 0xFFFF0000
TRAP_VECTOR  = 0xFFFFF000
```

Neither address is held in a writable CPU register.

## 2.1 Reset

On reset:

```text
mode       = S
STATUS.IE  = 0
STATUS.VM  = 0
PC         = 0xFFFF0000
```

Because translation is disabled, reset enters physical system ROM.

The initial ROM window is:

```text
0xFFFF0000..0xFFFFFFFF   64 KiB system ROM
```

`0xFFFF0000` is the reset entry.

## 2.2 Trap vector

Every synchronous exception, `TRAP`, and asynchronous platform interrupt enters:

```text
0xFFFFF000
```

With `STATUS.VM=0`, this is a physical ROM trap/diagnostic stub.

With `STATUS.VM=1`, it is interpreted through the current `VMCTX`. Cosmic maps the primary trap page identically in every address space:

```text
VA = 0xFFFFF000
U  = 0
R  = 1
W  = 0
X  = 1
G  = 1
```

This allows protected-mode traps and IPC to enter Cosmic directly through a global translation.

Reserve:

```text
0xFFFFF000..0xFFFFF7FF   primary 2 KiB Cosmic trap-entry page
0xFFFFF800..0xFFFFFFFF   reserved fixed system-entry space
```

---

# 3. Lighting-1 physical-address strategy

Lighting-1 uses a conventional split:

```text
low addresses             RAM

middle/high addresses     reserved expansion / PLIO

0xFFF00000..              fixed system MMIO
0xFFFF0000..0xFFFFFFFF    system ROM
```

The first fixed high-address assignments are:

```text
0xFFF00000   Lighting interrupt controller
0xFFF01000   monotonic timer
0xFFF02000   system control
0xFFF03000   boot diagnostics/console candidate

0xFFFF0000   64 KiB system ROM
0xFFFFF000   physical-mode trap entry inside ROM
```

Each fixed system device receives a 4 KiB MMIO aperture even if the first implementation uses only a few words.

## 3.1 One PLIO host in Lighting-1

Lighting-1 contains exactly:

```text
1 CPU
1 PLIO host
1 PLIO segment: PLIO0
```

A second PLIO host or segment is **not part of Lighting-1**. Later platform profiles may add additional PLIO hosts without changing the SIA ISA.

The exact physical aperture for `PLIO0` remains to be frozen as part of the complete Lighting-1 physical map.

## 3.2 Graphics

Lighting-1 has **no dedicated graphics or framebuffer physical-address region**.

Graphics is an optional PLIO/QDX graphics device:

```text
SIA CPU
   |
   v
PLIO0
   |
   v
optional QDX graphics card
   |
   +-- device registers
   +-- command queues
   +-- framebuffer / graphics memory as defined by the card
```

Any framebuffer or graphics memory belongs to the graphics device and is exposed through PLIO/QDX mechanisms. The base platform does not reserve a permanent framebuffer window.

## 3.3 ROM/runtime expansion

Lighting-1 reserves only the initial 64 KiB ROM window as a fixed ROM region.

There is **no separately assigned runtime-ROM expansion window**.

If later hardware needs more ROM, universal runtime storage, or other fixed-function memory, it is allocated from generic reserved expansion space in a later compatible profile.

This avoids permanently fragmenting the physical address map around speculative future ROM needs.

## Physical-map TODO

- [ ] Freeze maximum directly addressable Lighting-1 RAM.
- [ ] Freeze exact RAM range.
- [ ] Freeze `PLIO0` host aperture.
- [ ] Define generic reserved expansion ranges.
- [ ] Define behavior of unmapped physical accesses.
- [ ] Decide whether physical aliases are permitted.
- [x] No dedicated graphics/framebuffer region.
- [x] No dedicated runtime-ROM expansion region.
- [x] Exactly one PLIO host/segment in Lighting-1.

---

# 4. Physical memory attributes

Physical regions belong to one of four classes:

```text
NORMAL
    coherent RAM suitable for page tables and PLIO DMA

ROM
    readable/executable immutable normal memory

MMIO
    strongly ordered, non-speculative device memory

RESERVED
    no architectural target
```

`SIA32-MEM` defines software-visible ordering. The platform defines which physical ranges have each class.

## TODO

- [ ] Freeze whether ROM is cacheable as normal read-only memory.
- [ ] Define executable-MMIO prohibition.
- [ ] Require page tables to reside in NORMAL coherent RAM.
- [ ] Freeze DMA-visible memory classes.

---

# 5. Lighting-1 interrupt architecture

The CPU exposes only:

```text
STATUS.IE
CAUSE.INTERRUPT
one platform-interrupt condition
```

There is no CPU interrupt pending bitmap, per-source mask, priority register, source register, timer-pending bit, or software-interrupt pending bit.

## 5.1 PLIO-centric hierarchy

Ordinary I/O follows:

```text
QDX disk / GNet / graphics / keyboard / audio / etc.
                    |
                    v
            PLIO Notification
                    |
                    v
                 PLIO0
                    |
          one aggregate condition
                    |
                    v
       Lighting interrupt controller
                    |
                    v
      one SIA platform interrupt
                    |
                    v
             TRAP_VECTOR
```

The central interrupt controller answers:

> Which major machine subsystem needs attention?

PLIO0 then answers:

> Which device/queue/Notification needs attention?

The central controller never duplicates detailed PLIO/QDX Notification identity.

## 5.2 Source namespace

Lighting-1 defines a 4-bit source namespace:

```text
ID    Source
--    ------------------------------------------------------
0     NONE          no eligible source
1     MACHINE       fatal/urgent platform condition
2     TIMER         monotonic scheduling deadline
3     SOFTWARE      software-generated interrupt
4     PLIO0         PLIO0 has eligible Notification(s)
5-15  reserved      compatible future platform growth
```

Thus the first machine has only four meaningful nonzero central interrupt sources, regardless of how many I/O devices are attached to PLIO0.

## 5.3 Fixed priority

Lower nonzero source ID has higher priority:

```text
MACHINE > TIMER > SOFTWARE > PLIO0
```

There is no programmable priority or threshold in Lighting-1.

Individual I/O prioritization belongs to PLIO/Cosmic.

## 5.4 Controller MMIO

```text
INTC_BASE = 0xFFF00000
```

Registers:

```text
offset   register    access   meaning
------   ----------  -------  ---------------------------------------------
0x00     PENDING     RO       pending-source bitmap
0x04     ENABLE      RW       enabled-source bitmap
0x08     CLAIM       RO*      claim highest-priority eligible source
0x0C     COMPLETE    WO*      complete active source
0x10     SOFTINT     WO       generate SOFTWARE source
```

All are 32-bit MMIO words. Only bits 0..15 are defined for the bitmaps.

At cold reset:

```text
ENABLE        = 0
active claim  = NONE
SOFTWARE      = not pending
```

## 5.5 Pending and enable

```text
eligible[N] = PENDING[N] && ENABLE[N]
```

The controller asserts the CPU interrupt condition when at least one eligible source exists and there is no active claim blocking delivery.

`STATUS.IE` is the final CPU-side global gate.

Masking a source does not discard its pending state.

## 5.6 Claim

Reading `CLAIM` returns:

```text
0       no eligible source
1..15   claimed source ID
```

For a nonzero result, the controller:

1. selects the highest-priority eligible source;
2. marks it active;
3. prevents it from being claimed again until completion;
4. records one active claim.

Lighting-1 supports only one active central interrupt claim at a time.

## 5.7 Complete

Writing the active source ID to `COMPLETE` releases the claim and reevaluates the source set.

If a level source is still asserted, it becomes eligible again immediately.

## 5.8 Software interrupt

Writing a nonzero value to `SOFTINT` sets source `3` pending.

Later SMP/IPI facilities are specified separately and do not require new CPU privileged registers.

## 5.9 PLIO0 summary source

`PLIO0` is level-sensitive:

```text
PLIO0 has >=1 eligible Notification
        -> INTC.PENDING[4] = 1
```

Normal flow:

```text
INTC.CLAIM
    -> PLIO0

PLIO0 Notification claim/query
    -> exact slot/device/Notification

handle / dispatch Notification
complete or acknowledge PLIO Notification

repeat while useful

INTC.COMPLETE = PLIO0
```

If eligible Notifications remain, `PLIO0` immediately becomes pending again.

Cosmic may drain multiple PLIO Notifications per central claim; work limits are scheduler policy.

## Interrupt TODO

- [x] 4-bit / 16-value source namespace.
- [x] Fixed Lighting-1 source IDs.
- [x] Exactly one PLIO aggregate source.
- [x] Fixed priority.
- [x] `PENDING`, `ENABLE`, `CLAIM`, `COMPLETE`, `SOFTINT` interface.
- [x] Single active central claim.
- [x] Level-sensitive PLIO summary semantics.
- [ ] Define exact MACHINE-source semantics.
- [ ] Synchronize detailed PLIO Notification claim/completion with the PLIO/QDX specification.
- [ ] Implement in Rust VM.

---

# 6. Monotonic timer

Lighting-1 reserves:

```text
TIMER_BASE = 0xFFF01000
INTC source 2 = TIMER
```

Initial direction:

```text
64-bit monotonically increasing counter
64-bit one-shot deadline/compare
```

There is no CPU timer register and no CPU timer-pending bit.

Periodic scheduling is synthesized in software.

## TODO

- [ ] Choose timer frequency or discovery mechanism.
- [ ] Define reset/start value.
- [ ] Define stable 64-bit reads on a 32-bit CPU.
- [ ] Define deadline programming.
- [ ] Define already-expired deadline behavior.
- [ ] Define acknowledgement/rearm semantics.
- [ ] Define wraparound behavior.
- [ ] Freeze timer MMIO register layout.

---

# 7. PLIO platform integration

PLIO/QDX remain separately specified peripheral architectures. Lighting-1 freezes only their machine-level integration.

Lighting-1 requires:

```text
one PLIO host
one PLIO segment: PLIO0
INTC source 4 for PLIO0 Notification summary
coherent protected DMA to NORMAL RAM
```

The platform must still define:

- PLIO0 physical MMIO aperture;
- PLIO0 reset state;
- slot/device enumeration order;
- Notification claim/completion interaction with the central `PLIO0` claim;
- protected-DMA integration;
- mandatory QDX boot-block profile.

Optional graphics, networking, storage, input, audio, and other I/O attach through PLIO/QDX rather than gaining permanent base-platform address ranges.

---

# 8. Platform information and discovery

A minimal read-only **Platform Information Block** should describe variable platform properties without requiring a large firmware/device-tree system.

Candidate fields:

```text
signature
platform-spec version
profile ID
machine/revision ID
RAM descriptors
ROM descriptor
CPU count
clock/timer frequency
PLIO0 host location
interrupt-controller location
feature bits
additional-data pointer
checksum/version
```

Because Lighting-1 fixes many addresses, this block is primarily discovery/version metadata rather than a substitute for the profile specification.

## TODO

- [ ] Choose fixed block address or firmware-passed pointer.
- [ ] Define header and versioning.
- [ ] Define profile ID namespace.
- [ ] Define RAM-region discovery.
- [ ] Define optional feature bits.

---

# 9. ROM and runtime

Initial physical ROM:

```text
0xFFFF0000..0xFFFFFFFF   64 KiB
```

[`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md) defines stable shared ROM-library/runtime strategy.

Lighting-1 deliberately does not reserve additional physical runtime-ROM space. Later growth uses reserved expansion space.

## TODO

- [ ] Define ROM header/version identity.
- [ ] Define integrity/checksum mechanism.
- [ ] Define boot/runtime organization inside the initial ROM window.
- [ ] Define physical-mode trap stub contract.
- [ ] Define future expansion compatibility through reserved space.

---

# 10. Boot and Cosmic handoff

Initial boot direction:

```text
RESET_VECTOR
    -> system ROM
    -> discover/initialize RAM
    -> initialize interrupt controller and timer
    -> initialize PLIO0
    -> discover QDX boot block device
    -> load Cosmic
    -> establish global kernel/trap mappings
    -> transfer control to Cosmic
```

Candidate Cosmic entry state:

```text
mode              Supervisor
STATUS.IE         0
STATUS.VM         explicitly documented
r1                Platform Information Block pointer
r2                boot-device identity
r3                boot flags
remaining GPRs    unspecified
```

## TODO

- [ ] Decide VM-on versus VM-off handoff.
- [ ] Freeze boot argument registers.
- [ ] Define boot-device identity.
- [ ] Define boot image format dependency.
- [ ] Prefer no permanent privileged firmware dependency after Cosmic owns the machine.

---

# 11. DMA and coherency

For Lighting-1:

```text
NORMAL RAM
    CPU coherent
    PLIO DMA coherent
```

Drivers shall not require explicit data-cache clean/invalidate operations for baseline PLIO DMA.

PLIO protected DMA defines device authority; the CPU MMU does not translate device DMA addresses.

## TODO

- [ ] Define ordering between CPU descriptor publication and device observation.
- [ ] Define ordering between DMA completion and PLIO Notification visibility.
- [ ] Freeze the protected-DMA platform contract.

---

# 12. System control and diagnostics

Reserved:

```text
0xFFF02000   system control
0xFFF03000   optional boot diagnostics/console
```

The diagnostic console, if retained, is for firmware/bring-up and need not become a permanent high-performance I/O path. Normal user-visible I/O should use PLIO/QDX.

## TODO

- [ ] Define warm reset.
- [ ] Define cold reset request.
- [ ] Define power-off request if hardware supports it.
- [ ] Decide watchdog support.
- [ ] Decide whether the boot console is mandatory or optional.

---

# 13. Later SMP profile

Neutron or another SMP profile may add:

```text
multiple CPUs
multiple PLIO hosts
interrupt routing
IPIs
per-CPU timers
secondary-CPU startup
coherent shared memory
TLB shootdown support
```

None of this changes the six-register SIA32-P privileged-state model.

---

# 14. Immediate Lighting-1 platform TODO

With vectors and the central interrupt controller frozen, continue in this order:

```text
1. freeze RAM range and PLIO0 physical aperture
2. define monotonic timer
3. synchronize PLIO Notification claim/completion
4. define Platform Information Block
5. define ROM header/runtime organization
6. decide boot diagnostics console
7. define firmware -> Cosmic handoff
8. define power/reset controls
9. implement Lighting-1 in the Rust full-system VM
```

---

# 15. Lighting-1 completion criteria

Lighting-1 is platform-complete when:

- [x] `RESET_VECTOR` is frozen;
- [x] `TRAP_VECTOR` and global trap mapping are frozen;
- [x] initial system ROM window is frozen;
- [x] central interrupt-controller interface is frozen;
- [x] PLIO-centric interrupt hierarchy is frozen;
- [x] exactly one PLIO host/segment is part of Lighting-1;
- [x] graphics is defined as optional PLIO/QDX hardware rather than base-platform framebuffer space;
- [x] no dedicated runtime-ROM expansion region exists;
- [ ] RAM range and maximum RAM are frozen;
- [ ] PLIO0 MMIO aperture is frozen;
- [ ] memory attributes are frozen;
- [ ] monotonic timer is frozen;
- [ ] detailed PLIO Notification claim/completion is synchronized;
- [ ] platform information/discovery is frozen;
- [ ] Cosmic boot handoff is frozen;
- [ ] DMA/coherency rules are frozen;
- [ ] power/reset controls are sufficient;
- [ ] Rust VM can implement the machine without inventing unspecified behavior.
