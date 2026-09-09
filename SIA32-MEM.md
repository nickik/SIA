# SIA32-MEM — Memory Ordering, MMIO, DMA, and Instruction Synchronization

## Status

- Architecture supplement: **SIA32-MEM**
- Target base: `SIA32-I`
- Purpose: define software-visible memory ordering, device ordering, DMA coherence, cache behavior, and instruction-stream synchronization
- Required by Lighting/Cosmic: **YES**
- Initial model: **strong, TSO-like ordering**

SIA deliberately chooses a strong memory model so systems code and application runtimes do not need to reason about a large matrix of acquire/release annotations for ordinary operations.

The normal-memory model is intentionally close to x86 Total Store Order (TSO), but is stated here as a SIA contract rather than by reference to another ISA.

The model is strong but not fully sequentially consistent. The one ordinary reordering permitted is the classic TSO case: a later load may become visible before an older store to a **different** address has become globally visible.

A single full `FENCE` operation closes that relaxation when required.

Instruction fetch, translation caches, MMIO, and DMA have separate explicit rules because ordinary data-memory ordering alone is not sufficient to define them.

---

# 1. Design goals

The model is designed to provide:

- simple reasoning for ordinary shared-memory code;
- program-order loads;
- program-order stores;
- no load-to-load reordering;
- no load-to-store reordering;
- no store-to-store reordering;
- coherent same-address behavior;
- one well-defined store-to-load relaxation;
- one full data-memory fence;
- strongly ordered MMIO;
- coherent DMA in the baseline Lighting profile;
- no baseline data-cache clean/invalidate API;
- explicit instruction-stream synchronization through `SYNC.I`;
- explicit translation synchronization through `TLBFENCE` from `SIA32-P`;
- implementation freedom for caches, write buffers, speculation, and pipelines when invisible to software.

The goal is not to expose microarchitectural caches or buffers. The goal is to define what software may observe.

---

# 2. Memory categories

SIA distinguishes three relevant kinds of access.

## 2.1 Normal memory

Normal memory is ordinary coherent RAM used for:

- code;
- stacks;
- heaps;
- page tables;
- shared memory;
- DMA buffers.

Normal memory participates in the SIA strong memory model defined below.

## 2.2 Device memory / MMIO

Device memory represents architecturally visible device registers and queues accessed through the physical address space.

Device accesses have stronger ordering and side-effect rules than normal memory.

The platform physical memory map identifies device regions. Mapping a device physical region through the MMU does not turn it into normal cacheable RAM.

## 2.3 Instruction fetch

Instruction fetch reads bytes from executable normal memory, but instruction visibility is not implied solely by ordinary data-cache coherence.

Software that writes or modifies code uses `SYNC.I` before relying on subsequent local instruction fetches to observe those writes.

---

# 3. Normal-memory model: SIA-TSO

For ordinary loads and stores to normal memory, SIA uses a Total Store Order style model.

For each CPU, architectural program order is preserved for:

```text
Load  -> Load
Load  -> Store
Store -> Store
```

A later load may be observed before an earlier store to a **different** address has become globally visible:

```text
Store X -> Load Y       X != Y
```

This is the only ordinary program-order relaxation in the baseline model.

Loads are never reordered with older stores to the same address.

A processor may implement the permitted Store -> Load relaxation with a write buffer, but a write buffer is not architectural state.

---

# 4. Ordering table

For normal memory on one CPU:

| Earlier operation | Later operation | Program order guaranteed? |
|---|---|---|
| Load | Load | **Yes** |
| Load | Store | **Yes** |
| Store | Store | **Yes** |
| Store X | Load X | **Yes** |
| Store X | Load Y, X != Y | **Not necessarily** |

`FENCE` makes all earlier normal-memory operations globally ordered before all later normal-memory operations.

---

# 5. Global store order

Normal-memory stores are multi-copy atomic in the baseline SIA model.

Conceptually, all completed normal-memory stores participate in a single global store order with these requirements:

1. stores issued by one CPU appear in that CPU's program order;
2. all CPUs agree on the order of stores to normal memory;
3. once a store is globally visible, another CPU may not later observe an older value of that location except through a separately defined noncoherent mechanism, which the baseline does not provide.

An implementation need not literally serialize every store through one physical queue. Cache coherence may provide equivalent behavior.

---

# 6. Loads and local store forwarding

A CPU may satisfy a load from its own not-yet-globally-visible earlier store to the same address.

Therefore:

```text
store X = 1
load  X
```

must return `1` in the absence of another architecturally intervening write that changes the result.

The Store -> Load relaxation applies only when the older store and younger load address different locations.

---

# 7. Speculation

An implementation may speculatively execute or prefetch normal-memory operations when doing so cannot change architectural behavior.

Speculation must not:

- make a fault appear for an instruction that architecturally never executes;
- make an MMIO side effect occur speculatively;
- expose values forbidden by the SIA memory model;
- violate precise exception semantics;
- cause stale instructions to execute after a completed `SYNC.I`.

Speculation is therefore entirely microarchitectural.

---

# 8. `FENCE`

SIA defines one full unprivileged data-memory fence:

```asm
FENCE
```

After `FENCE` completes:

- every normal-memory load before the fence is ordered before every normal-memory load/store after the fence;
- every normal-memory store before the fence is globally visible before any normal-memory load/store after the fence is allowed to become architecturally visible;
- all earlier MMIO operations by the CPU are complete according to their device semantics before later memory/MMIO operations are issued;
- all earlier CPU stores that publish data to coherent DMA devices are visible before later device doorbells/commands.

A simple in-order implementation with no write buffer may implement `FENCE` as a legal no-op.

The instruction remains architectural so binaries work unchanged on later implementations with write buffers, caches, speculation, or SMP.

Ordinary code should rarely require `FENCE` because the baseline model already preserves Load -> Load, Load -> Store, and Store -> Store ordering.

`FENCE` is primarily required for:

- algorithms that must close the permitted Store -> Load relaxation;
- some lock-free synchronization sequences;
- generic synchronization code that requires a full barrier;
- future platform mechanisms whose ordering is not already made stronger by the MMIO rules below.

---

# 9. Relationship to `SIA32-A`

`SIA32-A` supplies atomic read/modify/write operations and LR/SC for coherent multiprocessors.

The baseline memory-ordering rule for atomics is intentionally strong:

- every AMO is an indivisible memory operation;
- every AMO is a full ordering point equivalent to a `FENCE` before and after the atomic access;
- a successful `SC.W` is a full ordering point;
- an `LR.W` follows ordinary load ordering, which already provides acquire-like ordering for younger loads/stores under SIA-TSO;
- a failed `SC.W` performs no store and does not by itself create a globally visible write.

This makes ordinary locks straightforward and avoids requiring acquire/release opcode variants in the first architecture.

`FENCE` belongs to the general SIA memory model, not conceptually to multiprocessing alone. `SIA32-A` requires and uses the baseline `FENCE` definition.

---

# 10. MMIO ordering

Device/MMIO accesses are **strongly ordered**.

For one CPU:

1. MMIO loads and stores occur in program order with respect to each other.
2. An MMIO access is not issued before older normal-memory accesses whose results or visibility are required to precede the device operation.
3. A later normal-memory or MMIO access is not made architecturally visible before an older MMIO access has occurred according to the device's access semantics.
4. MMIO accesses are never speculative.
5. MMIO accesses are never silently merged, widened, narrowed, repeated, or split into multiple device operations unless the platform device specification explicitly defines that behavior.

The access width encoded by the scalar load/store is the access width presented to the device.

This is intentionally stronger than normal RAM ordering because device programming should be simple and predictable.

## 10.1 Multi-register instructions and MMIO

`LDP`, `STP`, `LD4`, and `ST4` are prohibited for device/MMIO memory.

Only scalar accesses may produce device side effects.

---

# 11. CPU -> device publication

The Lighting profile requires the following useful property:

```text
CPU writes command/data descriptors to normal coherent memory
CPU writes a device doorbell / queue register
```

The device must not observe the doorbell before the CPU's earlier descriptor/data stores are visible to the device.

Because MMIO is strongly ordered against prior normal-memory stores, an additional `FENCE` is not required for the ordinary Lighting QDX submission path.

Software may still use `FENCE` in generic synchronization code without changing correctness.

---

# 12. DMA coherence — baseline Lighting profile

SIA itself does not define a particular DMA engine, but the baseline **Lighting** platform requires coherent DMA for normal memory.

For memory granted to a DMA-capable device:

- CPU stores become visible to the device without explicit data-cache clean operations;
- device DMA writes become visible to CPU loads without explicit data-cache invalidate operations;
- CPU caches, if present, must participate in whatever coherence mechanism is required to provide this behavior;
- a device completion/notification may not become visible to software before the DMA writes associated with that completion are globally visible.

This is a deliberate programmer-simplicity requirement.

A simple first implementation may satisfy it through:

- no data cache;
- write-through cache;
- centralized memory arbitration;
- DMA snooping/invalidation;
- another transparent coherent mechanism.

The mechanism is not architectural.

## 12.1 No baseline data-cache maintenance instructions

Because baseline Lighting DMA is coherent, SIA32 does **not** require instructions such as:

```text
DCACHE.CLEAN
DCACHE.INVALIDATE
DCACHE.FLUSH
```

for correct ordinary software.

A future intentionally noncoherent embedded SIA platform must define a separate platform/ISA profile with explicit cache ownership and maintenance operations. Such a platform is not the baseline Cosmic/Lighting target.

---

# 13. Device -> CPU completion

For a DMA completion, interrupt, or PLIO Notification that semantically reports completion of device writes:

```text
DMA writes
    happen-before
completion becomes observable to the CPU
    happens-before
CPU loads performed in response to the completion
```

Therefore a Cosmic driver may:

```text
wait for completion
read DMA buffer
```

without a data-cache invalidation operation.

The QDX/device specification remains responsible for defining which completion corresponds to which DMA work.

---

# 14. Cache architecture

SIA does not mandate:

- existence of caches;
- unified vs split caches;
- cache line size;
- associativity;
- write-back vs write-through implementation;
- replacement policy;
- TLB size;
- instruction prefetch depth.

These are implementation details as long as the architectural memory, DMA, MMIO, and synchronization rules are met.

## 14.1 Normal RAM

Normal RAM is coherent according to the rules above.

## 14.2 MMIO

Device/MMIO memory is non-cacheable from the software-visible point of view. Implementations must not satisfy an MMIO access from stale ordinary cache state.

## 14.3 Page tables

Page tables reside in normal coherent physical memory.

Page-table writes follow the normal memory model, while translation-cache visibility is controlled by the `TLBFENCE*` operations defined by `SIA32-P`.

---

# 15. `SYNC.I`

SIA defines an unprivileged instruction-stream synchronization operation:

```asm
SYNC.I
```

`SYNC.I` is distinct from `FENCE`.

`FENCE` orders data-memory operations.

`SYNC.I` synchronizes prior data stores with subsequent **local instruction fetches**.

After `SYNC.I` completes on a CPU:

1. all prior stores by that CPU are ordered before completion of the synchronization;
2. subsequent instruction fetches by that CPU observe those stores;
3. stale instruction-cache, prefetch, decode, or other instruction-side state may not cause an older instruction image to execute;
4. the CPU may implement this by invalidating an I-cache, flushing a pipeline, synchronizing coherent cache state, or doing nothing beyond serialization on a cacheless implementation.

`SYNC.I` affects the executing CPU. It does not by itself force another CPU to discard stale instruction-side state.

This operation is unprivileged because JIT compilers, dynamic-language runtimes, loaders, debuggers, and runtime-generated trampolines may need it.

---

# 16. Self-modifying and dynamically generated code

For code written into an already executable mapping on a single CPU:

```text
write instruction bytes
SYNC.I
execute new code
```

For a W^X-style runtime:

```text
map page writable, non-executable
write code
change PTE to executable, non-writable
TLBFENCE.VA
SYNC.I
execute
```

`TLBFENCE.VA` synchronizes translation/permission state.

`SYNC.I` synchronizes instruction-fetch state.

Neither operation substitutes for the other.

---

# 17. Cross-CPU code publication

On an SMP system, local `SYNC.I` is not sufficient to synchronize instruction state on other CPUs.

A generic publication sequence is:

```text
writer CPU:
    write code
    FENCE
    request remote synchronization / send IPI

remote CPU before executing the new code:
    SYNC.I
```

Cosmic should normally hide this protocol behind a kernel code-synchronization operation rather than require applications to manage remote CPUs directly.

A future implementation with fully coherent instruction caches may make remote synchronization cheap, but software-visible behavior remains the same.

---

# 18. Translation ordering

Writing a page-table entry changes ordinary memory but may leave cached translations stale.

The required sequence is defined by `SIA32-P`:

```text
write PTE(s)
TLBFENCE / TLBFENCE.VA / TLBFENCE.ASID
use mapping
```

A translation fence orders the relevant earlier PTE stores before subsequent affected translations.

A separate `FENCE` is therefore not required merely to publish PTE changes to the local translation machinery.

If the change also makes newly written code executable, `SYNC.I` is additionally required before local execution.

---

# 19. Multi-register transfers

`LDP`, `STP`, `LD4`, and `ST4` obey the normal-memory model but have stronger **fault completion** rules defined by `SIA32-I`:

- the complete transfer is validated before architectural completion;
- a recoverable fault causes no partial register, memory, or base-register update;
- successful component memory accesses are ordered in increasing register/address order;
- the instruction is **not** an interprocessor atomic transaction merely because it is all-or-nothing with respect to architectural faults.

Another CPU may observe individual stores of a successful `STP`/`ST4` according to normal memory ordering unless an atomic instruction is used.

---

# 20. Interrupts and memory ordering

An interrupt does not erase or reorder completed architectural memory operations.

Trap entry itself is not intended as a general-purpose substitute for `FENCE`.

Device completion semantics and MMIO ordering ensure that a driver entered due to a completion interrupt can safely observe the coherent DMA writes that the completion reports.

---

# 21. Examples

## 21.1 Message publication

With one writer and one reader using a synchronization flag implemented through an appropriate atomic operation:

```text
writer:
    data = value
    atomic_store_or_exchange(flag, READY)

reader:
    wait using atomic operation/load protocol
    read data
```

The strong baseline ordering and full-barrier atomics make the usual lock-based case straightforward.

## 21.2 Store-buffering case

The following outcome is permitted by SIA-TSO:

```text
initially X = 0, Y = 0

CPU0:              CPU1:
X = 1              Y = 1
r1 = Y             r2 = X
```

Result:

```text
r1 = 0
r2 = 0
```

is permitted because each load may pass the CPU's earlier store to a different location.

Adding `FENCE` between each store and load forbids that outcome.

## 21.3 Ordered QDX submission

```text
write descriptor fields in coherent RAM
write QDX/PLIO doorbell through MMIO
```

No explicit cache clean or ordinary `FENCE` is required in the baseline Lighting profile.

## 21.4 JIT code generation

```text
write generated instructions
SYNC.I
call generated code
```

On SMP, Cosmic must additionally arrange remote synchronization if the generated code may execute on another CPU.

---

# 22. Why not sequential consistency for every ordinary access

Full sequential consistency would be even simpler to describe, but it would prohibit the conventional store buffer optimization in which a CPU continues executing later loads while older stores drain toward coherent memory.

SIA therefore permits exactly the most valuable relaxation while retaining a simple programming model:

```text
only Store -> later Load to a different address may reorder
```

This gives implementations useful freedom without exposing programmers to a broadly weak memory model.

---

# 23. Why explicit `SYNC.I` remains necessary

Strong data-memory ordering does not define instruction-cache or prefetch visibility.

Even an implementation with a strong TSO-like data model may have:

```text
D-side writes -> coherent memory
I-cache       -> stale instruction bytes
```

`SYNC.I` provides the explicit architectural point at which those streams become synchronized for the local CPU.

This keeps JITs and loaders portable from a cacheless first implementation to later split-cache CPUs.

---

# 24. Required Lighting profile

The first Lighting implementation must provide:

```text
SIA-TSO normal-memory ordering
FENCE
strongly ordered MMIO
coherent CPU/DMA normal memory
no required D-cache maintenance operations
SYNC.I
SIA32-P TLBFENCE operations
```

This applies equally to:

- the Rust full-system VM;
- the later FPGA implementation.

The Rust VM should deliberately model the permitted Store -> Load relaxation in optional memory-model tests even if its simplest execution engine normally behaves more strongly.

---

# 25. Conformance tests

The architecture test suite should include at least:

## Normal memory

- Load -> Load cannot reorder.
- Load -> Store cannot reorder.
- Store -> Store cannot reorder.
- same-address Store -> Load returns the local/new value.
- different-address Store -> Load litmus permits the TSO result.
- `FENCE` forbids the store-buffering result.

## Atomics

- AMOs are indivisible.
- AMOs act as full ordering points.
- successful `SC.W` acts as a full ordering point.

## MMIO

- accesses occur in program order.
- no speculative device read occurs.
- access width is preserved.
- prior RAM descriptor stores are visible before a later device doorbell.

## DMA

- CPU writes are visible to DMA without clean operations.
- DMA writes are visible after completion without invalidate operations.
- completion is not observable before associated DMA writes.

## Instruction synchronization

- stale instruction state is allowed before required synchronization.
- `SYNC.I` makes prior local code stores visible to subsequent local fetches.
- `SYNC.I` is legal on a cacheless implementation.
- remote CPUs require their own synchronization in SMP.

## Translation

- PTE writes followed by `TLBFENCE` affect subsequent translation.
- code-permission changes followed by `TLBFENCE` but not `SYNC.I` do not substitute for instruction synchronization.

---

# 26. Summary

The baseline SIA memory-programming model is intentionally compact:

```text
normal RAM       strong TSO-like ordering
atomics          indivisible + full barriers
FENCE            full data-memory barrier
MMIO             strongly ordered, non-speculative
DMA              coherent in Lighting profile
D-cache upkeep   no software maintenance required
TLBFENCE         translation synchronization
SYNC.I            data-store -> local instruction-fetch synchronization
```

This is strong enough to make Cosmic and Forge runtime programming straightforward while still permitting useful implementation techniques such as write buffers, caches, speculation, and split instruction/data paths.