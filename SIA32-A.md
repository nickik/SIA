# SIA32-A — Optional Atomic and Multiprocessing Extension

## Status

- Extension: **OPTIONAL**
- Target base: `SIA32-I`
- Memory model: [`SIA32-MEM.md`](SIA32-MEM.md)
- Primary purpose: shared-memory multiprocessors and low-level synchronization
- Required by first Neutron SMP implementation: **YES**

`SIA32-A` adds a deliberately small set of atomic memory operations to the SIA base architecture. It follows the principle that atomics are a separable architectural capability rather than mandatory complexity in every processor.

A simple single-CPU SIA implementation may omit `SIA32-A` completely. A system advertising coherent shared-memory multiprocessing must implement the required `SIA32-A` profile.

The strong ordinary-memory ordering and the general `FENCE` instruction are defined by `SIA32-MEM`; this extension adds the atomic operations required for shared-memory synchronization.

## 1. Design goals

The extension exists to support kernel locks, user-space mutexes and semaphores, lock-free queues and counters, process/thread scheduling on multiple CPUs, reference counting, capability-table synchronization, and future multi-core SIA implementations.

The architectural model is:

```text
SIA-TSO ordinary loads/stores
        +
full FENCE from SIA32-MEM
        +
LR/SC reservation pair
        +
small atomic read-modify-write family
```

The first architecture deliberately avoids a large acquire/release encoding matrix. The baseline memory model is already strong, and the atomic operations below provide strong ordering semantics.

## 2. Scope

First-generation `SIA32-A` operates on naturally aligned 32-bit words. Byte and halfword atomics are not required. Atomic accesses to device/MMIO mappings are not permitted unless a later device profile explicitly defines them.

## 3. Load-reserved / store-conditional

### `LR.W rd, [rb]`

Atomically loads one aligned 32-bit word and establishes an implementation-defined reservation covering that location.

`LR.W` follows the ordinary SIA load-ordering rules. Under SIA-TSO, younger loads and stores do not pass the `LR.W`, providing the ordering normally required on the acquire side of a lock.

### `SC.W rd, rs, [rb]`

Attempts to store `rs` if the calling CPU still holds a valid reservation.

Result:

```text
rd = 0x00000000    store succeeded
rd = 0xFFFFFFFF    store failed
```

This uses normal SIA Boolean-mask convention. A failed `SC.W` performs no store. Software must tolerate spurious failure and retry.

Reservations may be invalidated by another CPU writing the coherence unit, interrupts/context switches, cache replacement, or other documented platform events.

A **successful** `SC.W` is a full ordering point: all older memory operations are ordered before it and all younger memory operations are ordered after it. This closes the Store -> Load relaxation that ordinary SIA-TSO permits.

A failed `SC.W` does not perform a globally visible write and is not by itself a substitute for `FENCE` in an algorithm that requires one regardless of success.

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

Every AMO is also a **full memory-ordering point**, equivalent to a full `FENCE` before and after the atomic memory operation.

`AMOSWAP.W` supports simple locks; `AMOADD.W` supports counters/reference counts; logical AMOs support bitset/event synchronization. Min/max atomics are deferred unless profiling justifies them.

## 5. Memory ordering

The ordinary memory model is defined by `SIA32-MEM`.

The relevant rules for `SIA32-A` are:

```text
ordinary memory       SIA-TSO
FENCE                 full barrier
LR.W                  ordinary strongly ordered load
successful SC.W       atomic conditional store + full ordering point
AMOxxx.W              indivisible RMW + full ordering point
```

This is deliberately stronger and simpler than an architecture in which each atomic carries independent acquire/release bits.

Lock-based software should therefore be straightforward to write and audit.

## 6. Coherence boundary

`SIA32-A` defines processor-visible atomic semantics, not a specific coherence protocol.

A conforming SMP platform must guarantee that all participating processors agree on serialization of atomic operations to a location and satisfy the global store/coherence rules of `SIA32-MEM`.

Possible implementations include write-through caches with broadcast invalidation, snooping coherent caches, centralized atomic memory controllers, and later directory schemes.

Neutron generation 1 is expected to favor a simple write-through/invalidation design.

## 7. Traps and context switches

`LR.W` does not create software-visible state beyond the success/failure of the following `SC.W`. A trap, interrupt, context switch, or processor migration may invalidate the reservation.

Software must never depend on an `SC.W` succeeding after such an event.

## 8. Encoding policy

The base SIA architecture remains fixed-width 16-bit and does not spend common primary opcode space on multiprocessing instructions.

The compact SYSTEM primary proposal in [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md) reserves `F700` for the general `FENCE` instruction.

The atomics themselves require multiple unrestricted register fields and should use the standard long-extension mechanism or another reserved extension encoding chosen during the final opcode freeze.

Exact atomic binary encodings remain provisional until assembler/emulator/compiler experiments select the final extension layout.

```text
small SIA CPU
    SIA32-I + SIA32-MEM behavior

multiprocessor SIA CPU
    SIA32-I + SIA32-MEM + SIA32-A
```

No existing base user instruction is redefined.

## 9. Required Neutron profile

The first four-CPU Neutron system requires:

```text
FENCE              from SIA32-MEM
LR.W
SC.W
AMOSWAP.W
AMOADD.W
AMOAND.W
AMOOR.W
AMOXOR.W
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