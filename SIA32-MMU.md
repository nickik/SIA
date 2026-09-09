# SIA32-MMU — Fast-Context Virtual Memory Architecture

## Status

- Target: `SIA32-I + SIA32-P`
- Purpose: normative MMU semantics for protected SIA systems
- Primary design goal: **very low-cost address-space switching for IPC and scheduling**
- Design influence: later DEC Alpha ASN/global-translation behavior
- First required platform: Lighting / Cosmic
- Status: **normative MMU direction for SIA32 v1**

This document is the authoritative SIA32 MMU specification. It supersedes the older 4 KiB / two-level / 8-bit-ASID MMU details that remain in early drafts of `SIA32-P.md`.

The central rule is:

> **Changing address spaces must not normally require flushing translation state.**

Cosmic is expected to be a capability microkernel in which IPC may transfer execution directly between protection domains. Address-space switching is therefore a cheap context operation, not a destructive TLB event.

---

# 1. Architectural summary

```text
Address space             32-bit

Normal page               2 KiB
Superpage                 1 MiB

ASID                       12 bits / 4096 values

TLB entry:
    VPN
    ASID
    G
    page size
    physical page
    R/W/X/U

Page tables               3-level

VA:
    L2       6 bits
    L1       6 bits
    L0       9 bits
    offset  11 bits

L1 leaf                    1 MiB superpage
L0 leaf                    2 KiB page

VMCTX:
    root >> 12             20 bits
    ASID                   12 bits

root alignment             4 KiB

SWRITE VMCTX               never flushes TLB
```

Recommended mapping policy:

```text
G=1
    Cosmic kernel
    universal ROM libraries
    universal system stubs

ASID-tagged
    application code
    heap
    stack
    per-process data
    private shared-memory mappings
```

---

# 2. Design goals

The SIA32 MMU shall provide:

- 32-bit virtual addressing;
- fine-grained 2 KiB normal pages suitable for memory-constrained systems;
- 1 MiB superpages for large stable mappings;
- hardware page-table walking;
- read/write/execute/user permissions;
- ASID-tagged cached translations;
- global translations shared across address spaces;
- one-register address-space context switching;
- no TLB flush on ordinary IPC/process switches;
- selective invalidation only when mappings change;
- inexpensive ASID recycling;
- identical architectural behavior in the Rust VM and later FPGA/custom implementations.

The MMU does not define processes, threads, capabilities, IPC objects, page-replacement policy, swapping, or the user/kernel virtual-address split. Those are Cosmic/ABI policy.

---

# 3. Unified page/frame model

SIA does **not** define separate physical instruction pages and data pages.

A physical frame is ordinary memory. Its role is determined by the permissions of the virtual mapping:

```text
R W X U G
```

Examples:

```text
application code         R-X U
process data             RW- U
stack                    RW- U
ROM library              R-X U G
ROM constants            R-- U G
Cosmic kernel text       R-X   G
Cosmic kernel data       RW-   G
```

An implementation may have separate instruction and data TLBs/caches. That is microarchitecture and does not create different page types.

---

# 4. Virtual-address format

A SIA32 virtual address is divided as follows:

```text
31          26 25          20 19                 11 10          0
+-------------+--------------+----------------------+-------------+
| L2  6 bits  | L1  6 bits   | L0      9 bits      | offset 11b  |
+-------------+--------------+----------------------+-------------+
```

Therefore:

```text
L2 index     = VA[31:26]      64 entries
L1 index     = VA[25:20]      64 entries
L0 index     = VA[19:11]     512 entries
offset       = VA[10:0]     2048 bytes
```

A normal page is 2 KiB.

One complete L0 table contains 512 32-bit entries:

```text
512 * 4 bytes = 2048 bytes
```

Thus one L0 page table occupies exactly one normal 2 KiB physical frame and maps exactly:

```text
512 * 2 KiB = 1 MiB
```

This is intentional.

---

# 5. Page-table levels

## 5.1 L2 root

The L2 root contains 64 architecturally used entries.

Only 256 bytes are required for those entries, but the root object is allocated from normal memory and its base must be **4 KiB aligned** so its address can be represented compactly in `VMCTX`.

The unused space in the containing allocation is not architecturally interpreted.

An L2 entry is a non-leaf pointer in the baseline. L2 leaf mappings are reserved for possible future very-large-page support.

## 5.2 L1 table

An L1 table contains 64 architecturally used entries.

An L1 entry may be:

- invalid;
- a non-leaf pointer to an L0 table;
- a **1 MiB superpage leaf**.

## 5.3 L0 table

An L0 table contains 512 entries and occupies exactly one 2 KiB frame.

A valid L0 leaf maps one 2 KiB page.

---

# 6. Page-table entry format

Every PTE is one naturally aligned 32-bit little-endian word.

```text
31                                   11 10                  0
+--------------------------------------+---------------------+
|        physical page number          |        flags        |
|              21 bits                 |       11 bits       |
+--------------------------------------+---------------------+
```

The physical page number is expressed in 2 KiB units.

Baseline flags:

```text
bit 0   V      valid
bit 1   R      readable
bit 2   W      writable
bit 3   X      executable
bit 4   U      accessible from User mode
bit 5   G      global / ASID-independent
bits 6..10    software-reserved
```

SIA32 does not require hardware Accessed or Dirty bits in the baseline. Cosmic may use software-reserved bits for mapping metadata.

## 6.1 Invalid PTE

```text
V = 0
```

The entry is invalid.

## 6.2 Non-leaf PTE

```text
V = 1
R = 0
W = 0
X = 0
```

The PPN identifies the physical base of the next-level table.

`U` and `G` are ignored for non-leaf entries in the baseline.

## 6.3 Leaf PTE

```text
V = 1
AND
(R | W | X) != 0
```

At L0, the leaf maps a 2 KiB frame.

At L1, the leaf maps a 1 MiB superpage.

At L2, leaf PTEs are reserved in the baseline.

## 6.4 Superpage alignment

A 1 MiB L1 leaf must identify a 1 MiB-aligned physical base.

Because PPNs are expressed in 2 KiB units, the low 9 PPN bits of an L1 superpage leaf must be zero.

A malformed/misaligned superpage PTE raises the corresponding page fault.

---

# 7. Page-table walk

For virtual address `va`:

```text
l2  = va[31:26]
l1  = va[25:20]
l0  = va[19:11]
off = va[10:0]
```

The root physical address is obtained from the active context:

```text
root = VMCTX.root << 12
```

## 7.1 L2

```text
pte2 = physical_read32(root + l2 * 4)
```

A valid non-leaf supplies the L1-table physical base.

An invalid or malformed entry faults.

## 7.2 L1

```text
pte1 = physical_read32(l1_base + l1 * 4)
```

If `pte1` is a valid leaf, it maps a 1 MiB superpage:

```text
pa = (pte1.PPN << 11) | va[19:0]
```

If `pte1` is a valid non-leaf, it supplies the L0-table base.

## 7.3 L0

```text
pte0 = physical_read32(l0_base + l0 * 4)
```

A valid leaf maps one 2 KiB frame:

```text
pa = (pte0.PPN << 11) | off
```

Page-table walks always use physical addresses. Page-table memory must be normal coherent RAM, not MMIO.

The walk is architectural; TLBs, walk caches, microcode, or dedicated walkers are implementation choices.

---

# 8. Permissions

For User-mode access:

```text
U must be 1
```

and the access type requires:

```text
instruction fetch    X
load                 R
store                W
```

Supervisor mode ignores `U` but still obeys `R/W/X` while translation is enabled.

This deliberately supports mappings such as:

```text
R--
-W-
--X
RW-
R-X
RWX
```

Cosmic is expected normally to enforce W^X policy even though the architecture can represent RWX.

---

# 9. ASID-tagged translations

The SIA32 ASID is **12 bits**:

```text
ASID = 0..4095
```

All values are usable. ASID zero has no special architectural meaning.

Every cached non-global translation behaves as if tagged by:

```text
virtual page
ASID
page size
```

A non-global TLB hit requires the virtual address and current ASID to match.

ASIDs identify translation contexts, not process objects. In SMP systems ASID assignment may be CPU-local.

The 4096-value namespace is intentionally large enough that ordinary IPC/context switching should almost never force immediate ASID recycling.

---

# 10. Global mappings

`G=1` means the mapping is independent of the current ASID.

A cached global translation may be reused across any `VMCTX` change.

Recommended uses include:

```text
Cosmic kernel                 U=0 G=1
universal ROM libraries       U=1 G=1
universal ROM constants       U=1 G=1
universal IPC/system stubs    U=1 G=1
```

A global user mapping is fully valid: `G` does not imply supervisor-only access. `U` controls user access independently.

## 10.1 Global mapping invariant

For any virtual address mapped `G=1`, the physical target, page size, and effective permissions must be identical in every address space in which the mapping exists.

Software must never create conflicting global mappings for the same VA.

This permits one TLB translation to remain valid across process switches.

## 10.2 ROM use

Immutable ROM is an especially strong global-mapping use case.

For example:

```text
system library code       R-X U G
system constants          R-- U G
```

Mutable library state remains ASID-private RAM.

---

# 11. VMCTX — combined translation context

Fast address-space switching uses the privileged `VMCTX` register.

```text
31                    12 11                               0
+-----------------------+----------------------------------+
| root physical >> 12   |             ASID                 |
|       20 bits         |            12 bits               |
+-----------------------+----------------------------------+
```

The root is therefore required to be 4 KiB aligned even though ordinary pages are 2 KiB.

`VMROOT` and `ASID` may remain architectural aliases for management/debug software, but the fast context path should use `VMCTX`.

## 11.1 Context installation

```asm
SWRITE VMCTX, rN
```

atomically installs both root and ASID.

After it retires:

- subsequent translated fetches use the new context;
- subsequent translated data accesses use the new context;
- no access may observe a mixed old-root/new-ASID state;
- stale prefetched instructions from the previous virtual context may not execute as though they belonged to the new context.

## 11.2 No implicit TLB invalidation

**`SWRITE VMCTX` never flushes or invalidates TLB entries merely because the address-space context changes.**

Previously cached translations remain resident under their ASIDs.

Global translations remain immediately usable.

This rule is fundamental to SIA microkernel IPC performance.

A `VMCTX` write is a local translation-context serialization point, not a full ordinary-memory barrier.

---

# 12. Translation-cache architecture

An implementation may use:

- no TLB;
- one unified TLB;
- separate instruction and data TLBs;
- multi-level TLBs;
- walk caches.

Architecturally, a TLB entry contains or behaves as if it contains:

```text
virtual page/tag
physical page/base
ASID
G
page size (2 KiB or 1 MiB)
R/W/X/U permissions
```

The architecture does not require instruction and data pages to be separate merely because an implementation uses separate I-TLB and D-TLB structures.

---

# 13. Context switching and IPC

## 13.1 Existing valid ASID

If the incoming address space already owns a valid ASID:

```asm
SWRITE VMCTX, receiver_context
```

is sufficient.

No `TLBFENCE`, cache flush, or page-table rewrite is required.

## 13.2 Threads in one address space

Threads sharing root and ASID require no MMU state change at all.

## 13.3 Fresh ASID

Cosmic may assign a fresh ASID instead of invalidating many stale translations after substantial mapping changes.

Old entries remain harmless because their old ASID no longer matches.

## 13.4 ASID reuse

Before assigning an ASID to a different translation context on a given CPU, software must invalidate old non-global entries for that ASID:

```asm
TLBFENCE.ASID rA
```

Only ASID reuse requires this maintenance step; ordinary process switching does not.

---

# 14. Mapping changes and translation fences

Context changes and mapping changes are different operations.

## 14.1 One page in current ASID

```text
store new PTE
TLBFENCE.VA address
```

`TLBFENCE.VA` invalidates locally cached translations for that VA in the current ASID and any locally cached global translation covering that VA.

It orders prior relevant PTE stores before subsequent affected translations.

## 14.2 One ASID

```text
store changed PTEs
TLBFENCE.ASID asid
```

invalidates local non-global translations carrying that ASID.

## 14.3 Entire local translation state

```text
TLBFENCE
```

invalidates all local cached translations, including global entries, and relevant walk-cache state.

This should be rare.

## 14.4 Global mapping changes

A global mapping change must be invalidated on every CPU that may cache the mapping.

On each relevant CPU, `TLBFENCE.VA` is sufficient for a single changed global VA.

---

# 15. Fast IPC mapping policy

The intended Cosmic layout is conceptually:

```text
G=1
    Cosmic kernel mappings
    universal ROM libraries
    universal system/IPC stubs

ASID-tagged
    application executable mappings
    heap
    stacks
    writable process globals
    TLS
    process-private shared-memory mappings
```

The architecture does not freeze exact virtual addresses for those regions.

A typical direct IPC handoff is therefore:

```text
sender user code
        |
        | TRAP
        v
Cosmic using global kernel translations
        |
        | validate endpoint
        | transfer short message in registers
        | install receiver VMCTX
        v
Cosmic still using the same global kernel translations
        |
        | SRET
        v
receiver user code
```

No TLB flush occurs merely because sender and receiver have different address spaces.

---

# 16. Superpage use

The 1 MiB L1 superpage is intended for mappings where TLB reach matters more than fine-grained protection.

Strong candidates include:

- large Cosmic kernel regions;
- immutable system ROM;
- universal runtime/library regions;
- large read-only tables;
- later large shared-memory mappings where identical permissions are appropriate.

A single global superpage TLB entry can therefore cover 1 MiB of system code shared by every process.

Ordinary application heaps/stacks should normally use 2 KiB pages unless their size and stability justify superpages.

---

# 17. Fault semantics

Translation and permission faults are precise.

On a fault:

```text
EPC     = faulting instruction
BADADDR = faulting virtual address
CAUSE   = corresponding instruction/load/store page fault
```

The architectural memory operation has not completed.

For `LDP/STP/LD4/ST4`, the separately specified all-or-nothing fault semantics apply.

Malformed page-table entries, including misaligned superpage leaves, raise the corresponding page fault.

---

# 18. Interaction with instruction synchronization

Changing `VMCTX` does not by itself make newly written instruction bytes visible to instruction fetch.

For generated or modified code:

```text
write code into writable page
change permissions/mapping as required
TLBFENCE.VA
SYNC.I
execute
```

A normal process switch does not require `SYNC.I` when executable contents have not changed.

---

# 19. Interaction with the strong memory model

The mechanisms remain deliberately distinct:

```text
SWRITE VMCTX      choose address-space context; no TLB flush
TLBFENCE*         synchronize changed translations/page tables
FENCE             full ordinary-data ordering point
SYNC.I            prior code stores -> later local instruction fetches
```

Page-table memory is ordinary coherent RAM.

`TLBFENCE*` orders prior relevant PTE stores before subsequent affected translations.

---

# 20. DMA relationship

The CPU MMU protects CPU virtual memory accesses.

Device DMA authority is separate and belongs to the platform I/O architecture. For Lighting, PLIO protected DMA handles provide that boundary.

The Lighting memory profile is expected to keep normal RAM and PLIO DMA coherent so ordinary drivers do not require explicit data-cache clean/invalidate operations.

---

# 21. SMP behavior

ASIDs may be processor-local translation identities.

A mapping modification must be synchronized on every CPU that may retain the affected translation.

Typical shootdown:

```text
CPU 0:
    store PTE
    local TLBFENCE.VA
    send IPI

remote CPU:
    TLBFENCE.VA
    acknowledge
```

Global mapping changes require corresponding invalidation on every relevant CPU.

SIA does not require hardware broadcast TLB invalidation.

---

# 22. Why this design favors microkernel IPC

The expensive operation in a traditional address-space switch is often not writing a page-table-root register; it is discarding useful translations and rebuilding working sets afterward.

SIA avoids that cost through:

```text
12-bit ASIDs
+ global mappings
+ retained translations across VMCTX changes
+ large global superpages
+ one-register root+ASID installation
```

For two processes whose hot translations remain resident, the MMU work of a direct IPC switch is conceptually only:

```text
load receiver VMCTX
SWRITE VMCTX
```

The architecture requires no full TLB flush, cache flush, page-table rewrite, or page walk solely because the protection domain changed.

---

# 23. Required first Lighting profile

The first protected Lighting/Cosmic implementation shall support:

```text
32-bit VA
2 KiB pages
1 MiB L1 superpages
three-level hardware page-table walk
12-bit ASIDs
G global mappings
R/W/X/U permissions
VMCTX root+ASID context register
SWRITE VMCTX without TLB invalidation
TLBFENCE
TLBFENCE.VA
TLBFENCE.ASID
precise translation/protection faults
```

This is the MMU contract to implement first in the Rust full-system VM and later reproduce in FPGA hardware.
