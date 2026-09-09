# SIA32-MMU — Fast-Context Virtual Memory Architecture

## Status

- Target: `SIA32-I + SIA32-P`
- Purpose: normative MMU semantics for protected SIA systems
- Primary design goal: **very low-cost address-space switching for IPC and scheduling**
- Design influence: later DEC Alpha ASN/global-translation behavior
- First required platform: Lighting / Cosmic
- Status: **normative MMU direction for SIA32 v1**

This document is the authoritative SIA32 MMU specification.

The central rule is:

> **Changing address spaces must not normally require flushing translation state.**

Cosmic is expected to be a capability microkernel in which IPC frequently transfers execution directly between protection domains. Address-space switching is therefore a cheap context operation, not a destructive TLB event.

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
    fixed TRAP_VECTOR entry
    universal ROM libraries
    universal ROM constants
    universal system/IPC stubs

ASID-tagged
    application code
    heap
    stack
    writable process data
    TLS
    non-global shared-memory mappings
```

---

# 2. Unified page/frame model

SIA does **not** define different physical instruction and data page types.

A physical frame is ordinary memory. Virtual mapping permissions define its role:

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
TRAP_VECTOR entry        R-X   G
```

Separate I-TLB/D-TLB or I-cache/D-cache structures are permitted microarchitecture and do not create different architectural page types.

---

# 3. Virtual-address format

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

One complete L0 table is:

```text
512 entries * 4 bytes = 2048 bytes
```

and maps:

```text
512 * 2 KiB = 1 MiB
```

This exact fit is intentional.

---

# 4. Page-table levels

## 4.1 L2 root

The L2 root contains 64 architecturally used entries.

Its base must be **4 KiB aligned** so the root address plus 12-bit ASID fit in one 32-bit `VMCTX` register.

L2 leaves are reserved in the baseline.

## 4.2 L1 table

An L1 entry may be:

- invalid;
- a non-leaf pointer to an L0 table;
- a **1 MiB superpage leaf**.

## 4.3 L0 table

An L0 table contains 512 entries and occupies one 2 KiB frame.

A valid L0 leaf maps one 2 KiB page.

---

# 5. Page-table entry format

Each PTE is one naturally aligned 32-bit little-endian word.

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

No hardware Accessed or Dirty bits are required in the baseline.

## 5.1 Invalid

```text
V = 0
```

The entry is invalid.

## 5.2 Non-leaf

```text
V = 1
R = 0
W = 0
X = 0
```

The PPN identifies the physical base of the next-level table.

`U` and `G` are ignored for non-leaf entries.

## 5.3 Leaf

```text
V = 1
AND
(R | W | X) != 0
```

At L0 the leaf maps 2 KiB.

At L1 the leaf maps 1 MiB.

L2 leaves are reserved.

## 5.4 Superpage alignment

A 1 MiB L1 leaf must identify a 1 MiB-aligned physical base.

Because PPNs are in 2 KiB units, the low 9 PPN bits of an L1 superpage leaf must be zero.

A malformed superpage PTE raises the corresponding page fault.

---

# 6. Page-table walk

For virtual address `va`:

```text
l2  = va[31:26]
l1  = va[25:20]
l0  = va[19:11]
off = va[10:0]
```

The active root is obtained only from `VMCTX`:

```text
root = VMCTX[31:12] << 12
```

There are **no architectural `VMROOT` or `ASID` alias registers**.

## 6.1 L2

```text
pte2 = physical_read32(root + l2 * 4)
```

A valid non-leaf supplies the L1 table base.

## 6.2 L1

```text
pte1 = physical_read32(l1_base + l1 * 4)
```

If `pte1` is a valid leaf:

```text
pa = (pte1.PPN << 11) | va[19:0]
```

If it is a valid non-leaf, it supplies the L0 table base.

## 6.3 L0

```text
pte0 = physical_read32(l0_base + l0 * 4)
```

A valid leaf produces:

```text
pa = (pte0.PPN << 11) | off
```

Page-table walks always use physical addresses and page tables must reside in normal coherent RAM, never MMIO.

---

# 7. Permissions

User access requires:

```text
U = 1
```

plus the appropriate permission:

```text
instruction fetch    X
load                 R
store                W
```

Supervisor mode ignores `U` but still obeys `R/W/X` while translation is enabled.

The architecture permits independent combinations including write-only and execute-only mappings. Cosmic may enforce a narrower W^X policy.

---

# 8. `VMCTX`

`VMCTX` is the **only privileged address-space context register**.

```text
31                    12 11                               0
+-----------------------+----------------------------------+
| root physical >> 12   |              ASID                |
|       20 bits         |             12 bits              |
+-----------------------+----------------------------------+
```

Thus:

```text
root physical address = VMCTX[31:12] << 12
ASID                  = VMCTX[11:0]
```

The root is 4 KiB aligned even though normal pages are 2 KiB.

Software that needs the root or ASID separately reads `VMCTX` and masks/shifts it in an ordinary GPR.

## 8.1 Context installation

```asm
SWRITE VMCTX, rN
```

installs root + ASID together.

After it retires:

- subsequent translated instruction fetches use the new context;
- subsequent translated data accesses use the new context;
- no access observes a mixed old-root/new-ASID combination;
- stale prefetched instructions from the old virtual context may not execute as though they belonged to the new context.

## 8.2 No implicit invalidation

**`SWRITE VMCTX` never invalidates TLB entries merely because the active address space changed.**

Previously cached translations remain tagged under their ASIDs.

Global translations remain immediately reusable.

A `VMCTX` write is a local translation-context serialization point, not a full ordinary-memory barrier.

---

# 9. ASID-tagged translations

The ASID is 12 bits:

```text
0..4095
```

All values are usable; zero has no special meaning.

Every cached non-global translation behaves as though tagged by:

```text
virtual page
ASID
page size
```

A non-global hit requires both virtual-address and current-ASID match.

ASIDs are translation identities, not process IDs. In an SMP platform they may be CPU-local.

The 4096-entry namespace is intentionally large so ordinary IPC/context switching rarely forces ASID recycling.

---

# 10. Global mappings

`G=1` means the translation is ASID-independent.

A global cached translation remains usable across any `VMCTX` change.

Recommended uses:

```text
Cosmic kernel                 U=0 G=1
fixed TRAP_VECTOR mapping     U=0 G=1
universal ROM libraries       U=1 G=1
universal ROM constants       U=1 G=1
universal IPC/system stubs    U=1 G=1
```

`G` and `U` are independent; a global mapping may be user-accessible.

## 10.1 Global invariant

For a `G=1` virtual mapping, physical target, page size, and effective permissions must be identical in every address space where it exists.

Conflicting global mappings are invalid software behavior.

## 10.2 ROM

Immutable ROM is an ideal global mapping:

```text
system library code       R-X U G
system constants          R-- U G
```

Mutable library state remains ASID-private RAM.

---

# 11. Translation-cache architecture

An implementation may have:

- no TLB;
- one unified TLB;
- separate I-TLB and D-TLB;
- multi-level TLBs;
- page-walk caches.

Architecturally an entry behaves as if it contains:

```text
virtual page/tag
physical page/base
ASID
G
page size
R/W/X/U
```

For an implementation with `N` entries, adding ASIDs requires 12 ASID tag bits plus `G` per entry. This is intentionally modest incremental hardware relative to the TLB itself.

---

# 12. Context switching

## 12.1 Existing valid ASID

If the incoming address space already has a valid ASID:

```asm
SWRITE VMCTX, new_context
```

is sufficient.

No `TLBFENCE`, cache flush, or page-table rewrite is required.

## 12.2 Same address space

Threads sharing `VMCTX` require no MMU state change.

## 12.3 Fresh ASID

After substantial mapping changes, Cosmic may assign a fresh ASID rather than invalidate many stale entries.

Old entries remain harmless under the old ASID.

## 12.4 ASID reuse

Before reassigning an ASID to a different translation context on a CPU:

```asm
TLBFENCE.ASID rA
```

must invalidate old non-global translations carrying that ASID.

ASID reuse is maintenance, not part of ordinary switching.

---

# 13. Translation fences

Context changes and mapping changes are distinct.

## 13.1 One virtual address

```text
store new PTE
TLBFENCE.VA address
```

The fence invalidates local cached translations for that VA in the current ASID and any local global translation covering that VA.

It also orders prior relevant PTE stores before subsequent affected translations.

## 13.2 One ASID

```text
store changed PTEs
TLBFENCE.ASID asid
```

invalidates local non-global translations for the named ASID.

## 13.3 All local translation state

```text
TLBFENCE
```

invalidates all required local translations, including global entries and associated walk-cache state.

This should be rare.

## 13.4 Global mapping changes

A changed global mapping must be invalidated on every CPU that might cache it.

For a single VA, each CPU may use `TLBFENCE.VA`.

---

# 14. Fast IPC

The intended cross-address-space path is:

```text
sender User
   |
   | TRAP
   v
fixed globally mapped TRAP_VECTOR
   |
   v
Cosmic kernel using G=1 translations
   |
   | endpoint/capability checks
   | short message in registers
   | restore receiver state
   v
SRETCTX receiver_vmctx
   |
   v
receiver User
```

`SRETCTX` combines installing the receiver `VMCTX` with trap return and does not flush the TLB.

A warm receiver may therefore immediately reuse its previous ASID-tagged translations.

---

# 15. Superpages

A 1 MiB L1 superpage is intended where TLB reach matters more than fine-grained mapping control.

Strong candidates:

- large stable Cosmic kernel regions;
- immutable system ROM;
- universal runtime/library regions;
- large read-only tables;
- large framebuffer or shared-memory regions when permissions permit.

A global 1 MiB ROM/kernel mapping may allow one TLB entry to cover code used by every process.

---

# 16. Fault semantics

Translation/protection faults are precise.

```text
EPC     = faulting instruction
BADADDR = faulting virtual address
CAUSE   = relevant page-fault cause
```

The architectural operation has not completed.

A malformed page-table entry, invalid level, misaligned superpage, missing mapping, or permission violation raises the corresponding page fault.

Physical/bus failure after successful translation raises the corresponding access fault instead.

Multi-register operations use the all-or-nothing rules in `SIA32-MULTI-TRANSFER.md`.

---

# 17. Memory-model interaction

Page tables are normal coherent physical memory.

Typical mapping update:

```text
store PTE
TLBFENCE*
use new mapping
```

`TLBFENCE*` supplies the relevant ordering between the prior PTE store and subsequent affected translation; a separate `FENCE` is not required merely for local page-table publication.

`FENCE`, `TLBFENCE*`, `VMCTX`, and `SYNC.I` have distinct purposes:

```text
VMCTX write      choose translation context
TLBFENCE*        synchronize changed mappings
FENCE            ordinary data ordering
SYNC.I           data-write -> instruction-fetch synchronization
```

---

# 18. Executable-code publication

Typical W^X sequence:

```text
map RW/NX
write code
change PTE to RX
TLBFENCE.VA page
SYNC.I
execute
```

A mere `VMCTX` switch does not require `SYNC.I` if executable memory has not changed.

---

# 19. SMP behavior

ASIDs may be processor-local.

A mapping change must be synchronized on every CPU that may hold the affected translation.

Typical remote invalidation:

```text
CPU 0:
    store PTE
    local TLBFENCE.VA
    request platform IPI to relevant CPU

remote CPU:
    TLBFENCE.VA
    acknowledge through platform/kernel protocol
```

The IPI mechanism belongs to the SIA Platform Specification, not the MMU register set.

Global mapping changes require invalidation on all relevant CPUs.

---

# 20. Conformance requirements

The Rust VM and hardware implementations must test at minimum:

- 2 KiB L0 mapping;
- 1 MiB L1 superpage;
- malformed superpage alignment;
- all `R/W/X/U` permission cases;
- global user and supervisor mappings;
- same VA under different ASIDs;
- `SWRITE VMCTX` retaining old TLB entries;
- `SRETCTX` retaining old TLB entries;
- fresh-ASID switching;
- ASID reuse after `TLBFENCE.ASID`;
- global-entry reuse across `VMCTX` changes;
- global-entry invalidation with `TLBFENCE.VA`;
- exact cross-page multi-transfer fault behavior;
- page-table memory ordering;
- `SYNC.I` after executable publication.

The key performance conformance case is:

```text
A VMCTX resident translations
B VMCTX resident translations
A -> B -> A
```

with no mandatory translation flush on either switch.
