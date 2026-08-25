# SIA32-A — Optional Atomic and Multiprocessing Extension

## Status

- Extension: **OPTIONAL**
- Target base: `SIA32-I`
- Primary purpose: shared-memory multiprocessors and low-level synchronization
- Required by first Neutron SMP implementation: **YES**

`SIA32-A` adds a deliberately small set of atomic memory operations and ordering primitives to the SIA base architecture. It follows the later RISC-V `A` extension principle: atomics are a separable architectural capability rather than mandatory complexity in every processor.

A simple single-CPU SIA implementation may omit `SIA32-A` completely. A system advertising coherent shared-memory multiprocessing must implement the required `SIA32-A` profile.

## 1. Design goals

The extension exists to support kernel locks, user-space mutexes and semaphores, lock-free queues and counters, process/thread scheduling on multiple CPUs, reference counting, capability-table synchronization, and future multi-core SIA implementations.

The architectural model is:

```text
ordinary SIA loads/stores
        +
LR/SC reservation pair
        +
small atomic read-modify-write family
        +
explicit ordering fence
```

## 2. Scope

First-generation `SIA32-A` operates on naturally aligned 32-bit words. Byte and halfword atomics are not required. Atomic accesses to device/MMIO mappings are not permitted unless a later device profile explicitly defines them.

## 3. Load-reserved / store-conditional

### `LR.W rd, [rb]`

Atomically loads one aligned 32-bit word and establishes an implementation-defined reservation covering that location.

### `SC.W rd, rs, [rb]`

Attempts to store `rs` if the calling CPU still holds a valid reservation.

Result:

```text
rd = 0x00000000    store succeeded
rd = 0xFFFFFFFF    store failed
```

This uses normal SIA Boolean-mask convention. A failed `SC.W` performs no store. Software must tolerate spurious failure and retry.

Reservations may be invalidated by another CPU writing the coherence unit, interrupts/context switches, cache replacement, or other documented platform events.

## 4. Atomic read-modify-write operations

The first profile requires:

```text
AMOSWAP.W
AMOADD.W
AMOAND.W
AMOOR.W
AMOXOR.W
```

General form:

```asm
AMOxxx.W rd, rs, [rb]
```

Semantics:

```text
old = memory[rb]
new = operation(old, rs)
memory[rb] = new
rd = old
```

The read and write are indivisible with respect to all processors in the coherent memory domain.

`AMOSWAP.W` supports simple locks; `AMOADD.W` supports counters/reference counts; logical AMOs support bitset/event synchronization. Min/max atomics are deferred unless profiling justifies them.

## 5. Memory ordering

### `FENCE`

`FENCE` provides a full ordering point for the calling CPU's ordinary memory and atomic accesses.

The first Neutron SMP profile should expose a deliberately strong and easy-to-program memory model. Hardware may internally buffer or overlap accesses, but lock-based software should remain straightforward.

The initial extension does not require a large acquire/release encoding matrix. Later implementations may add lighter-weight ordering modifiers if profiling demonstrates a material benefit.

## 6. Coherence boundary

`SIA32-A` defines processor-visible atomic semantics, not a specific coherence protocol.

A conforming SMP platform must guarantee that all participating processors agree on serialization of atomic operations to a location.

Possible implementations include write-through caches with broadcast invalidation, snooping coherent caches, centralized atomic memory controllers, and later directory schemes.

Neutron generation 1 is expected to favor a simple write-through/invalidation design.

## 7. Traps and context switches

`LR.W` does not create software-visible state beyond the success/failure of the following `SC.W`. A trap, interrupt, context switch, or processor migration may invalidate the reservation.

## 8. Encoding policy

The base SIA architecture remains fixed-width 16-bit and does not spend common primary opcode space on multiprocessing instructions.

`SIA32-A` uses the reserved SIA extension mechanism. Exact binary encoding remains provisional until assembler/emulator/compiler experiments select the final `EXT` format.

```text
small SIA CPU
    SIA32-I only

multiprocessor SIA CPU
    SIA32-I + SIA32-A
```

No base instruction is redefined.

## 9. Required Neutron profile

The first four-CPU Neutron system requires:

```text
LR.W
SC.W
AMOSWAP.W
AMOADD.W
AMOAND.W
AMOOR.W
AMOXOR.W
FENCE
```

The Neutron platform additionally provides coherent shared memory, interprocessor notification, CPU identification/control and operating-system mechanisms outside the ISA proper.

## 10. Long-term role

The same synchronization model should be reusable across:

```text
ECL Neutron SMP
        |
future board-level SIA multiprocessors
        |
CMOS SIA servers
        |
multi-chip modules
        |
future multi-core SIA processors
```

The extension is therefore a reusable systems-architecture investment rather than a Neutron-specific instruction set.