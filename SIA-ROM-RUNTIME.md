# SIA ROM Runtime — Stability, Global Libraries, and Shared Allocator Code

## Status

- Purpose: define the system-ROM software model used by Lighting/Cosmic
- Scope: ROM ABI stability, global mappings, shared runtime code, allocator reuse
- Related specifications: `SIA32-MMU.md`, `SIA32-P.md`, `SIA32-MEM.md`

The Lighting system is expected to place substantial common runtime functionality in immutable system ROM and map that ROM identically into every process.

The design goal is:

> **One stable ROM interface, one shared implementation of common runtime algorithms, many independent process/kernel state instances.**

ROM should reduce RAM consumption, improve TLB/cache reuse, and make common system facilities available without duplicating code into every executable.

---

# 1. Global ROM mappings

System ROM should normally be mapped with the SIA MMU `G` bit so one translation can be reused across address-space switches.

Typical mappings:

```text
system library code       R-X U G
system constants          R-- U G
IPC/system stubs          R-X U G
kernel-only ROM code      R-X   G
```

For `G=1` mappings, the virtual address, physical target, page size, and effective permissions must be identical in every address space that contains the mapping.

Mutable state must not be placed in shared ROM.

```text
ROM
    algorithms
    immutable lookup tables
    ABI vectors
    constants
    common code

RAM
    heap state
    arena descriptors
    slab metadata
    freelists
    locks
    per-process globals
    TLS
```

---

# 2. ROM ABI stability

Applications should not depend directly on arbitrary internal implementation addresses inside a particular ROM revision.

Instead, the platform should expose a small, stable **ROM interface/vector table** at a fixed globally mapped virtual address.

Conceptually:

```text
ROM ABI header
    magic
    ABI major version
    ABI minor version
    table size
    feature bits

ROM ABI vectors
    memory/runtime entry points
    allocator entry points
    IPC stubs
    string/memory helpers
    optional math/runtime services
```

A program targets the stable ABI vector or named interface rather than assuming that `memcpy`, `arena_alloc`, or another routine always lives at one implementation address.

This separates:

```text
stable ROM ABI
        from
replaceable ROM implementation
```

A later Lighting hardware revision may reorganize the ROM internally while retaining the same ABI table.

---

# 3. Compatibility policy

The ROM ABI should follow these rules:

1. Existing ABI vector numbers/meanings are never silently redefined.
2. New vectors are appended or exposed through versioned extension tables.
3. The ABI header identifies the implemented version and table size.
4. Programs may test feature bits before using optional facilities.
5. A major incompatible redesign uses a new ABI major version rather than changing old semantics.
6. ROM implementation code may freely move as long as the stable ABI remains valid.

Where practical, several compatible ABI generations may coexist in a newer ROM.

---

# 4. Why ROM libraries are useful with the SIA MMU

The SIA MMU is deliberately optimized for global shared mappings.

A universal ROM library region can be:

```text
G = 1
U = 1
R = 1
W = 0
X = 1
```

Then a translation remains valid across:

```text
process A / ASID 17
        |
        | IPC/context switch
        v
process B / ASID 442
```

without needing to re-tag or reload the ROM TLB entry.

The 1 MiB SIA superpage is especially suitable for stable ROM regions because a large body of shared code can occupy a single global TLB entry.

---

# 5. Shared allocator code

Cosmic should not define separate, unrelated allocator algorithms for kernel and user space if one generic implementation can serve both.

The preferred model is:

```text
                     global ROM
                        |
              generic allocator code
                        |
             +----------+----------+
             |                     |
        kernel instance        user instance
             |                     |
       kernel RAM state       process RAM state
```

The code is shared.

The allocator state and authority are not.

This mirrors the useful architectural idea of an arena allocator plus slab/object caches: the same allocation machinery can manage different resource domains by changing the backing provider and state object.

---

# 6. Allocation layers

The recommended allocation stack is:

```text
physical memory/resources
        |
        v
frame/resource allocator
        |
        v
arena allocator
        |
        v
slab/object caches or general heap
```

## 6.1 Frame/resource allocator

The lowest memory allocator manages the physical 2 KiB frames defined by `SIA32-MMU`.

Required operations include conceptually:

```text
allocate frame
free frame
allocate aligned frames
allocate contiguous frames
```

The kernel/memory service owns the authority to allocate physical frames.

User code must never gain physical-memory authority merely because it executes the same ROM allocator routines.

## 6.2 Arena allocator

The generic arena abstraction manages ranges of fungible resources.

An arena may represent:

```text
virtual-address ranges
physical-memory ranges
ASID numbers
device identifiers
DMA windows/channels
other integer/resource namespaces
```

Conceptually:

```text
Arena {
    range/state
    quantum
    backing_source
    alloc()
    free()
}
```

An arena may import more resource from another provider when it cannot satisfy an allocation locally.

## 6.3 Slab/object caches

Typed object caches are built above arenas/frame backing.

Kernel examples:

```text
Thread
AddressSpace
Endpoint
CapabilityNode
Timer
IRQBinding
PageTable
QDXQueue
```

User/runtime examples may include fixed-size Forge runtime objects or general heap size classes.

The same generic slab/cache implementation may be used for both contexts while maintaining separate state.

---

# 7. One implementation, many allocator instances

The canonical rule is:

> **Share allocator code, never allocator ownership/state by accident.**

For example:

```text
ROM arena_alloc()
ROM arena_free()
ROM slab_alloc()
ROM slab_free()
```

may be called by both Cosmic and applications.

But each caller supplies or references its own state:

```text
Cosmic:
    kernel arena state
    kernel slab lists
    kernel backing provider

Process A:
    process-A heap state
    process-A slab lists
    user backing provider

Process B:
    process-B heap state
    process-B slab lists
    user backing provider
```

The shared code does not imply a shared heap.

---

# 8. Backing-provider boundary

The allocator core should not contain hidden privilege.

When more backing memory is required, it invokes a context-specific provider interface.

Conceptually:

```text
allocator needs more memory
        |
        v
backing provider
```

Kernel provider:

```text
obtain/free physical frames directly from kernel-owned memory authority
```

User provider:

```text
request additional mapped memory from Cosmic or a user-space memory service
```

Therefore user code can execute exactly the same ROM allocation algorithm without being able to allocate arbitrary physical frames.

This is important for Cosmic's capability model.

---

# 9. Suggested user allocation path

A user heap may behave as follows:

```text
application calls allocation routine
        |
        v
ROM allocator searches local process arena/cache
        |
        +--> resource available -> return immediately
        |
        +--> resource exhausted
                  |
                  v
             request/map more memory through Cosmic service
                  |
                  v
             add new span to process arena
                  |
                  v
             continue allocation
```

The common case therefore remains entirely user-level and executes shared ROM code without a kernel transition.

Only arena expansion requires IPC/system involvement.

---

# 10. Suggested kernel allocation path

Cosmic uses the same generic mechanisms but different backing authority:

```text
kernel object allocation
        |
        v
ROM/shared allocator algorithm
        |
        v
kernel object cache
        |
        +--> available object -> return
        |
        +--> refill -> kernel frame/resource allocator
```

The kernel should preallocate/cache critical IPC-path objects where possible so allocator activity does not appear on the normal fast IPC path.

---

# 11. What belongs in ROM

Strong candidates for shared system ROM include:

```text
Forge low-level runtime
memory/string primitives
arena allocator algorithms
slab/cache algorithms
small general heap algorithms
IPC client stubs
system-call/trap stubs
integer helper routines
common immutable tables
basic math routines
loader/runtime helpers
```

Possible later additions include stable graphics, text, or protocol helper code where the ABI can be kept sufficiently stable.

ROM should favor mature, low-level, broadly reusable facilities rather than rapidly changing high-level policy.

---

# 12. What should remain outside ROM

Do not put policy-heavy mutable system components into ROM merely because ROM is globally mapped.

Examples better kept replaceable in RAM/disk software include:

```text
filesystem policy
network-service policy
GUI server policy
scheduler policy above minimal kernel mechanisms
complex device drivers
rapidly evolving high-level libraries
```

ROM is best used as a stable primitive/runtime substrate.

---

# 13. Security and authority model

Globally executable ROM routines are ordinary user-callable code when mapped `U=1`.

They must not gain privileged authority merely by being in ROM.

A ROM routine that needs privileged service must use the same explicit mechanism as any other user code:

```text
TRAP
IPC endpoint
capability/service handle
```

Kernel-only ROM routines may instead be mapped `U=0`.

The distinction is controlled by MMU mappings, not by the physical fact that bytes reside in ROM.

---

# 14. Position-independent code

Because universal ROM is mapped at the same virtual address in every address space, ROM code does not require position independence merely for process sharing.

Internal fixed references within one ROM ABI generation are acceptable.

However, external applications should still enter through stable ABI vectors rather than depending on internal symbol addresses. This allows implementation layout to change between ROM revisions.

---

# 15. ROM upgrades and disk overrides

A future system may need newer implementations than those physically present in an older ROM.

The stable ABI should therefore permit a software layer to redirect selected high-level interfaces to newer RAM/disk implementations where practical.

One possible design is:

```text
fixed ROM ABI entry
        |
        v
stable vector/dispatch record
        |
        +--> built-in ROM implementation
        |
        +--> optional system-installed replacement
```

The exact override mechanism is a Lighting/Cosmic ABI decision and is not required for the first implementation.

---

# 16. Required first Lighting position

For the first Lighting system:

1. Reserve a globally mapped system-ROM virtual region.
2. Map universal user ROM code `R-X U G`.
3. Map immutable universal constants `R-- U G`.
4. Keep writable runtime/allocator state in ASID-private RAM.
5. Define a small versioned ROM ABI table at a stable virtual address.
6. Implement generic arena and slab/cache algorithms so kernel and user space can execute the same code.
7. Give kernel and user allocators distinct backing providers and state.
8. Do not put allocation on the normal Cosmic IPC fast path.
9. Prefer 1 MiB global superpages for sufficiently large/aligned stable ROM regions.

This provides a compact shared system runtime without weakening process isolation or tying applications permanently to one internal ROM layout.
