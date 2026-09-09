# SIA32-MMU — Fast-Context Virtual Memory Architecture

## Status

- Target: `SIA32-I + SIA32-P`
- Purpose: normative MMU semantics for protected SIA systems
- Primary design goal: **very low-cost address-space switching for IPC and scheduling**
- Design influence: later DEC Alpha ASN/ASM context behavior
- First required platform: Lighting / Cosmic

This document refines the virtual-memory portion of `SIA32-P` into a dedicated MMU contract.

The most important design rule is:

> **Changing address spaces must not normally require flushing translation state.**

Cosmic is expected to use a capability-microkernel design in which an IPC operation may transfer execution directly from a sender in one address space to a receiver in another. The MMU therefore treats address-space switching as a cheap architectural operation rather than as a destructive TLB event.

The model deliberately borrows the useful context semantics of later Alpha processors while retaining SIA's simpler hardware-walked two-level page tables.

---

# 1. Design goals

The SIA32 MMU shall provide:

- 32-bit virtual addressing;
- 4 KiB pages;
- two-level page tables;
- hardware page-table walking;
- read/write/execute/user permissions;
- tagged cached translations;
- global mappings shared across address spaces;
- one-instruction address-space context switching;
- no required TLB flush on ordinary process or IPC switches;
- selective invalidation when mappings actually change;
- inexpensive ASID recycling;
- identical semantics in the Rust full-system VM and FPGA implementations.

The MMU does **not** define:

- processes;
- threads;
- IPC objects;
- capabilities;
- scheduler policy;
- page replacement policy;
- swapping;
- kernel virtual-address layout.

Those remain Cosmic policy.

---

# 2. Alpha-style context principle

Later Alpha implementations attached an Address Space Number (ASN) to translation-buffer entries. Mappings marked as address-space independent could remain valid across process switches, while process-specific entries were distinguished by ASN.

SIA adopts the same fundamental principle:

```text
cached translation key
    = virtual page + ASID

unless PTE.G = 1
    = virtual page only
```

Consequently, switching from address space A to B does not invalidate A's translations. They may remain resident and become immediately useful when A runs again.

This is especially valuable for IPC-heavy microkernels.

---

# 3. Virtual-address format

SIA32 uses a 32-bit virtual address and 4 KiB pages.

```text
31                    22 21                    12 11             0
+-----------------------+------------------------+----------------+
|       L1 index        |       L0 index         | page offset    |
|       10 bits         |       10 bits          |   12 bits      |
+-----------------------+------------------------+----------------+
```

Each page-table page contains 1024 32-bit entries and occupies exactly 4 KiB.

The baseline defines only 4 KiB leaf mappings.

---

# 4. MMU context state

The active virtual-memory context consists logically of:

```text
VMROOT     root page-table physical page number
ASID       current address-space identifier
```

For fast context switching, SIA32-P additionally exposes the composite system register:

```text
VMCTX
```

## 4.1 `VMCTX` format

```text
31                    12 11          8 7                    0
+-----------------------+--------------+----------------------+
| VMROOT physical PPN   |   reserved   |        ASID          |
|       20 bits         |    4 bits    |        8 bits        |
+-----------------------+--------------+----------------------+
```

The reserved bits must be written as zero and read as zero.

Thus:

```text
VMROOT = VMCTX[31:12]
ASID   = VMCTX[7:0]
```

`VMROOT` and `ASID` remain readable/writable aliases for software that needs to manipulate them individually.

## 4.2 Atomic context installation

A write to `VMCTX` installs both `VMROOT` and `ASID` as one architectural context change.

Example:

```asm
SWRITE VMCTX, r4
```

After the instruction retires:

- all subsequent translated instruction fetches use the new context;
- all subsequent translated data accesses use the new context;
- no subsequent access may accidentally use the old `VMROOT` with the new `ASID`, or vice versa;
- stale prefetched instructions from the previous context must not execute under the new context.

The context write is locally serializing with respect to address translation.

It does **not** imply a global data-memory `FENCE` and does **not** invalidate translation entries.

---

# 5. Address Space Identifiers

The baseline SIA32 ASID width is 8 bits.

```text
ASID = 0..255
```

All values are usable. ASID zero has no special hardware meaning.

A cached non-global translation is associated with the ASID that was active when the translation was created.

Conceptually:

```text
TLB entry:
    virtual_page
    physical_page
    permissions
    ASID
    global
```

A non-global entry matches only when:

```text
entry.virtual_page == requested.virtual_page
AND
entry.ASID == current.ASID
```

A global entry ignores ASID.

## 5.1 ASIDs are local translation identities

An ASID identifies translation state, not a process object.

Cosmic may assign ASIDs independently on different CPUs in an SMP implementation.

The same Cosmic address space need not use the same numeric ASID on every CPU.

This keeps ASID allocation and recycling local to each processor and simplifies SMP operation.

---

# 6. Global mappings

The PTE `G` bit means that the translation is independent of the current ASID.

A cached global translation matches regardless of `ASID`.

Typical use:

```text
user mappings       G=0
Cosmic kernel       G=1
common fixed pages  G=1 where appropriate
```

The kernel can therefore remain mapped while switching directly between user address spaces.

SIA does not mandate a specific user/kernel virtual-address split. The Lighting/Cosmic ABI should choose one common kernel region and use identical global mappings in every process address space.

## 6.1 Global mapping invariant

Software must not create conflicting `G=1` mappings for the same virtual page on the same CPU.

A virtual page marked global is expected to resolve identically regardless of current ASID.

Changing a global mapping requires appropriate translation invalidation on every CPU that may cache it.

---

# 7. Fast IPC context switch

A typical small-message IPC fast path can be:

```text
sender executes TRAP
        |
        v
trap enters globally mapped Cosmic kernel
        |
        v
kernel validates endpoint/capabilities
        |
        v
small message transferred in registers
        |
        v
load receiver VMCTX
        |
        v
restore receiver register state
        |
        v
SRET directly into receiver
```

MMU work in the normal path is only:

```asm
SWRITE VMCTX, receiver_context
```

No page-table modification is required.

No `TLBFENCE` is required.

No cache flush is required.

Previously cached sender translations remain cached under the sender ASID.

Previously cached receiver translations may immediately hit after the context switch.

This is the intended SIA/Cosmic IPC model.

---

# 8. Page-table entry format

Each PTE is one naturally aligned 32-bit little-endian word.

```text
31                                      12 11                 0
+-----------------------------------------+--------------------+
|             physical PPN                |       flags        |
|               20 bits                   |      12 bits       |
+-----------------------------------------+--------------------+
```

Baseline flags:

```text
bit 0   V      valid
bit 1   R      readable
bit 2   W      writable
bit 3   X      executable
bit 4   U      accessible from User mode
bit 5   G      global / ASID-independent
bits 6..11    software-reserved
```

SIA does not require hardware Accessed or Dirty bits in the baseline.

Cosmic may use software-reserved PTE bits for its own metadata.

---

# 9. Page-table walk

SIA retains the hardware-defined two-level walk already specified by SIA32-P.

Given virtual address `va`:

```text
l1  = va[31:22]
l0  = va[21:12]
off = va[11:0]
```

The root page address is:

```text
root = VMROOT << 12
```

Level 1:

```text
pte1 = physical_read32(root + l1 * 4)
```

A valid non-leaf PTE supplies the L0 page-table PPN.

Level 0:

```text
pte0 = physical_read32((pte1.PPN << 12) + l0 * 4)
```

A valid leaf produces:

```text
pa = (pte0.PPN << 12) | off
```

The page-table walker always reads page tables by physical address.

TLBs and walk caches are implementation details.

---

# 10. Translation-cache behavior

An implementation may provide:

- no TLB;
- one unified TLB;
- separate ITB and DTB structures;
- multi-level TLBs;
- page-walk caches.

All are architecturally invisible.

Any cached non-global translation must be tagged by ASID or behave equivalently.

Any cached global translation must remain usable across `VMCTX` changes.

An implementation must never require software to flush all translations merely because `VMROOT` or ASID changes to a different valid context.

---

# 11. Context switching rules

## 11.1 Switching to an existing valid ASID

If the incoming address space retains a valid assigned ASID:

```text
SWRITE VMCTX,new
```

is sufficient.

No `TLBFENCE` is required.

## 11.2 Switching between threads in one address space

If two threads use the same `VMROOT` and ASID, no MMU state change is required at all.

## 11.3 Fresh ASID assignment

When an address space receives an ASID that has not been used for conflicting cached translations since its last invalidation, it may be installed immediately without a TLB flush.

This permits the operating system to avoid invalidating old translations by allocating a fresh ASID.

## 11.4 ASID reuse

Before reusing an ASID for a different translation context on the same CPU, software must invalidate old non-global translations for that ASID:

```asm
TLBFENCE.ASID rA
```

After completion, the ASID may safely be assigned to another address space.

This makes ASID exhaustion a rare maintenance event rather than part of normal context switching.

---

# 12. Mapping changes

Changing address spaces and changing mappings are distinct operations.

Ordinary context switching does not require invalidation.

Changing a PTE may require invalidation.

## 12.1 Current ASID, one virtual page

After replacing or revoking a mapping for one virtual page in the current ASID:

```text
store new PTE
TLBFENCE.VA address
```

The fence orders the PTE store before subsequent affected translation and removes stale cached translation state.

## 12.2 Many mappings in one ASID

Software may use:

```text
store changed PTEs
TLBFENCE.ASID asid
```

## 12.3 Whole local translation state

`TLBFENCE` invalidates all locally cached translations required by the instruction definition.

This should be uncommon.

## 12.4 Fresh-ASID optimization

Instead of invalidating many stale entries, Cosmic may assign the changed address space a fresh ASID and install it with `VMCTX`.

Old translations remain harmless because they carry the previous ASID.

They are reclaimed lazily when that old ASID is eventually recycled.

This is an intentional Alpha-style optimization.

---

# 13. Kernel mappings and IPC

For the first Lighting/Cosmic profile, the recommended structure is:

```text
+------------------------------+ high VA
| Cosmic kernel / kernel data  | G=1
| kernel stacks / core objects | G=1 where appropriate
+------------------------------+
|                              |
| process-specific user space  | G=0, ASID tagged
|                              |
+------------------------------+ low VA
```

The exact boundary belongs to the SIA ABI / Lighting platform profile.

Trap entry therefore does not require changing page tables merely to execute the kernel.

During a fast IPC handoff the kernel remains executing from the same global translation while `VMCTX` changes underneath the user portion of the address space.

---

# 14. Supervisor access semantics

Supervisor accesses use the currently installed `VMCTX` when translation is enabled.

Supervisor mode ignores the PTE `U` bit but continues to obey `R`, `W`, and `X` permissions.

The baseline does not add special instructions to address memory through an arbitrary foreign ASID.

A kernel that needs another address space may:

- switch `VMCTX`;
- map the relevant physical frame into kernel space;
- use platform/physical-memory mechanisms defined outside the ordinary user MMU.

The fast IPC path does not require arbitrary foreign-address-space loads because short IPC payloads should normally travel in registers.

---

# 15. Fault semantics

Page faults remain precise.

On a translation or permission fault:

```text
EPC     = faulting instruction
BADADDR = faulting virtual address
CAUSE   = relevant page-fault cause
```

The architectural operation has not completed.

For multi-register `LDP/STP/LD4/ST4`, the separately defined all-or-nothing semantics apply.

---

# 16. Interaction with `SYNC.I`

Changing `VMCTX` does not by itself synchronize code written through data accesses.

When software creates or modifies executable instructions, it must use the SIA instruction-stream synchronization rules.

Typical W^X sequence:

```text
write code into RW page
change PTE to RX
TLBFENCE.VA page
SYNC.I
execute
```

A mere process context switch does not require `SYNC.I` if executable memory has not been modified.

---

# 17. Interaction with the strong memory model

`VMCTX` is a translation-context serialization point, not a general interprocessor memory barrier.

The SIA strong memory model continues to govern ordinary RAM ordering.

`TLBFENCE*` orders prior relevant PTE stores before subsequent affected translations.

`FENCE` remains the full ordinary-memory barrier.

`SYNC.I` remains the data-write to instruction-fetch synchronization operation.

These mechanisms are deliberately separate:

```text
VMCTX write      choose address-space context
TLBFENCE*        synchronize changed mappings
FENCE            synchronize ordinary data ordering
SYNC.I           synchronize generated/modified instructions
```

---

# 18. SMP behavior

ASIDs are processor-local translation tags.

A mapping modification must be synchronized on every CPU that may have cached the affected translation.

Typical SMP shootdown:

```text
CPU 0:
    store PTE
    local TLBFENCE.VA
    send IPI to relevant CPUs

remote CPU:
    TLBFENCE.VA
    acknowledge
```

Cosmic may instead use fresh per-CPU ASIDs where that avoids immediate invalidation.

Global mapping changes require invalidation on all relevant CPUs.

SIA32-P does not require hardware broadcast TLB invalidation.

---

# 19. Why retain hardware page-table walking

Alpha's ASN concept does not require copying Alpha's PALcode-managed translation-buffer refill model.

SIA keeps a standardized hardware walk because it gives:

- one page-table format for Cosmic;
- deterministic Rust VM behavior;
- simpler FPGA/software agreement;
- no kernel entry on an ordinary TLB miss;
- freedom to change TLB organization without changing the OS;
- low software overhead on small implementations.

The useful Alpha feature for SIA is **translation identity and context retention**, not software refill itself.

---

# 20. Expected fast-path cost

For two runnable threads in different address spaces whose translations are already resident, the MMU portion of an IPC context switch should reduce conceptually to:

```text
load receiver VMCTX value
SWRITE VMCTX
```

The architecture requires no:

```text
full TLB flush
page-table rewrite
cache flush
instruction-cache flush
page-table walk solely because the context changed
```

A TLB miss may of course occur later if the receiver's needed translation is not resident.

This is the main performance objective of the MMU design.

---

# 21. Required conformance tests

## ASID retention

- run address space A and populate translation;
- switch to B and populate translation for same VA with different PA;
- switch back to A;
- A must recover its own translation without mandatory flush.

## Global mapping

- install common `G=1` kernel mapping;
- switch through many ASIDs;
- mapping remains valid and identical.

## `VMCTX`

- combined root/ASID change is atomic architecturally;
- no instruction/data access observes mixed context fields;
- prefetched user instruction from old context cannot execute after context switch.

## ASID reuse

- reuse without required invalidation may expose stale translation and is software error;
- `TLBFENCE.ASID` makes reuse safe.

## Mapping modification

- stale translation may remain before required fence;
- `TLBFENCE.VA` makes the changed mapping visible;
- assigning a fresh ASID makes stale old-ASID entries harmless.

## IPC fast path

- trap from sender;
- kernel remains executable through global mapping;
- switch `VMCTX` to receiver;
- `SRET` executes receiver using receiver address space;
- no whole-TLB invalidation occurs.

---

# 22. First Lighting profile

The first Lighting implementation shall support:

```text
32-bit virtual addresses
4 KiB pages
2-level hardware-walked page tables
8-bit ASIDs
ASID-tagged non-global translations
G global mappings
VMCTX combined context register
VMROOT / ASID aliases
TLBFENCE
TLBFENCE.VA
TLBFENCE.ASID
R/W/X/U/G permissions
precise page faults
```

The Rust full-system VM should implement this behavior before Cosmic IPC optimization work begins.

The FPGA implementation must match the same architectural behavior, but may choose any TLB organization and replacement policy.
