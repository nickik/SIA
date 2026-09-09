# SIA Architecture Completion TODO

This is the canonical checklist for taking SIA from a compact integer ISA plus initial privilege/atomic extensions to a complete system architecture suitable for Cosmic, the Rust full-system VM, and later FPGA implementation.

Work through this list in order unless a later item becomes a direct blocker for implementation or measurement.

## Status convention

- `[ ]` not started
- `[~]` in progress
- `[x]` complete

---

# 1. Finalize the instruction encoding

SIA32-I still contains provisional opcode assignments. Before the architecture can become a stable implementation target, the base and privileged instruction encodings need to be frozen.

- [ ] Freeze the complete SIA32-I primary opcode map.
- [ ] Resolve the current `ADC` / `SBB` opcode pressure.
- [ ] Freeze destination-zero escape encodings.
- [ ] Freeze the `EXT` mechanism.
- [ ] Allocate encodings for SIA32-P privileged operations:
  - [ ] `SREAD`
  - [ ] `SWRITE`
  - [ ] `SSWAP`
  - [ ] `SRET`
  - [ ] `TLBFENCE`
  - [ ] `TLBFENCE.VA`
  - [ ] `TLBFENCE.ASID`
  - [ ] `WFI`
- [ ] Add `SYNC.I` or equivalent instruction-fetch synchronization operation.
- [ ] Decide whether `SYNC.I` belongs in SIA32-I or a mandatory system/cache extension.
- [ ] Freeze illegal/reserved encoding behavior.
- [ ] Update interpreter/disassembler/assembler tables to use the frozen encoding.
- [ ] Mark v1.0 instruction encodings as stable and non-redefinable.

---

# 2. Define precise multi-register exception semantics

`LDP`, `STP`, `LD4`, and `ST4` can touch multiple words and therefore need exact restart/fault behavior.

Example problem:

```text
word 0 succeeds
word 1 succeeds
word 2 faults
```

A microkernel requires predictable restart semantics.

- [ ] Define whether multi-register transfers are architecturally atomic with respect to faults.
- [ ] Prefer all-or-nothing architectural completion where practical.
- [ ] Define whether address translation for the entire transfer is validated before any architectural register or memory modification.
- [ ] Define load destination update behavior on a fault.
- [ ] Define store visibility on a fault.
- [ ] Define base-register writeback behavior on a fault.
- [ ] Define exception PC for a failed multi-register instruction.
- [ ] Define precise behavior if a transfer crosses a page boundary.
- [ ] Define behavior for MMIO mappings explicitly; current policy prohibits multi-register transfers to MMIO.
- [ ] Add conformance tests for all page-boundary and permission combinations.

---

# 3. Freeze the SIA memory model

SIA should intentionally use a fairly strong, simple memory model, close in programming model to x86 for normal cacheable RAM, while retaining explicit mechanisms where data ordering alone is insufficient.

The specification must cover:

```text
normal RAM
MMIO
loads vs loads
loads vs stores
stores vs stores
atomics
DMA visibility
page-table updates
instruction fetch
```

- [ ] Define ordering of ordinary loads relative to older loads.
- [ ] Define ordering of ordinary loads relative to older stores.
- [ ] Define ordering of ordinary stores relative to older loads.
- [ ] Define ordering of ordinary stores relative to older stores.
- [ ] Define when speculative execution may become architecturally visible.
- [ ] Define single-copy visibility expectations for ordinary memory.
- [ ] Define interaction with `SIA32-A` atomics.
- [ ] Define exact semantics of `FENCE`.
- [ ] Decide whether the strong baseline model allows `FENCE` to be rare in ordinary code.
- [ ] Define MMIO ordering separately from normal RAM.
- [ ] Define whether MMIO accesses are strongly ordered by default.
- [ ] Define DMA visibility requirements.
- [ ] Define CPU-to-device publish sequence.
- [ ] Define device-to-CPU completion/readback sequence.
- [ ] Define interaction between page-table writes and `TLBFENCE`.
- [ ] Define interaction between code writes and `SYNC.I`.
- [ ] Define whether instruction fetch participates in ordinary memory coherence or only through explicit synchronization.
- [ ] Add litmus tests to the Rust interpreter/VM.

---

# 4. Define cache-management and instruction synchronization semantics

The architecture does not need to mandate a cache topology, but software-visible cache behavior must be specified.

- [ ] Define `SYNC.I` semantics:
  - [ ] prior stores become visible to subsequent local instruction fetches;
  - [ ] stale prefetch/decode/I-cache state cannot cause old instructions to execute after synchronization;
  - [ ] operation remains valid on cacheless systems as a legal no-op or pipeline synchronization.
- [ ] Define SMP expectations for remote instruction synchronization.
- [ ] Require Cosmic to coordinate cross-CPU code publication when needed.
- [ ] Define whether explicit data-cache clean/invalidate operations are required by baseline SIA.
- [ ] Prefer coherent normal RAM if practical so ordinary software does not manage caches.
- [ ] Define cacheability attributes for MMIO.
- [ ] Define cacheability attributes for normal RAM.
- [ ] Define DMA/cache interaction.
- [ ] Decide whether PLIO DMA is architecturally coherent with CPU caches.
- [ ] If DMA is not coherent, define exact clean/invalidate operations and ownership transitions.
- [ ] Define page-table cacheability requirements.
- [ ] Define self-modifying/JIT code sequence, e.g.:

```text
write code
change mapping if required
TLBFENCE.VA
SYNC.I
execute
```

- [ ] Document recommended W^X policy for Cosmic.

---

# 5. Define the Lighting platform interrupt-controller interface

SIA32-P defines the CPU-level interrupt classes:

```text
software
timer
external
```

The actual Lighting interrupt controller belongs in the platform specification rather than the generic SIA ISA.

- [ ] Define CPU-visible external interrupt line/condition.
- [ ] Define PLIO Notification to CPU interrupt mapping.
- [ ] Define pending state.
- [ ] Define per-source masking.
- [ ] Define claim/identify operation.
- [ ] Define acknowledge/complete operation.
- [ ] Define priority model.
- [ ] Define nesting/preemption policy.
- [ ] Define spurious interrupt behavior.
- [ ] Define interrupt-controller MMIO register layout.
- [ ] Define interaction with `IENABLE` / `IPENDING` in SIA32-P.
- [ ] Define software interrupt generation.
- [ ] Add a Rust VM interrupt-controller model.

---

# 6. Define architectural timer expectations

A seL4-style kernel needs a reliable scheduling timer and monotonic time source.

The timer should probably remain a platform MMIO device rather than become additional CPU privileged state.

- [ ] Define a 64-bit monotonic counter.
- [ ] Define counter frequency or discovery mechanism.
- [ ] Define read semantics on a 32-bit CPU.
- [ ] Define compare/deadline register.
- [ ] Define one-shot timer behavior.
- [ ] Define timer interrupt generation.
- [ ] Define acknowledgement/rearm behavior.
- [ ] Define behavior when programmed deadline is already in the past.
- [ ] Define wraparound expectations.
- [ ] Define reset value/behavior.
- [ ] Decide whether periodic mode exists or is synthesized in software.
- [ ] Define real-time clock separately from monotonic scheduling time.
- [ ] Implement timer in Rust VM before Cosmic scheduler work.

---

# 7. Freeze the SIA ABI

Forge and Cosmic need a normative SIA ABI rather than a suggested convention.

- [ ] Freeze argument registers.
- [ ] Freeze return-value registers.
- [ ] Freeze caller-saved registers.
- [ ] Freeze callee-saved registers.
- [ ] Freeze stack pointer role.
- [ ] Freeze link register role.
- [ ] Freeze frame pointer convention.
- [ ] Define stack growth direction.
- [ ] Define stack alignment.
- [ ] Define scalar argument extension rules.
- [ ] Define 64-bit integer argument/return conventions.
- [ ] Define struct/aggregate argument passing.
- [ ] Define small aggregate return convention.
- [ ] Define large aggregate return convention.
- [ ] Define varargs convention if Forge needs it.
- [ ] Define TLS model/register convention if required.
- [ ] Define syscall ABI.
- [ ] Define trap argument/result convention.
- [ ] Define kernel entry scratch-register usage.
- [ ] Define unwind/debug frame conventions if desired.
- [ ] Publish a normative `SIA32-ABI.md`.

---

# 8. Define object format and relocations

SIA uses short branches and PC-relative literal pools, so the assembler/linker ABI is particularly important.

- [ ] Choose or define object-file container format.
- [ ] Define section types.
- [ ] Define symbol representation.
- [ ] Define absolute 32-bit relocation.
- [ ] Define PC-relative branch relocation.
- [ ] Define PC-relative call relocation.
- [ ] Define `LDPC.W` literal relocation.
- [ ] Define far call/jump relocation strategy.
- [ ] Define linker veneers/trampolines.
- [ ] Define literal-pool placement rules.
- [ ] Define overflow handling for out-of-range branches.
- [ ] Define data-pointer relocations.
- [ ] Define TLS relocations if TLS is adopted.
- [ ] Define executable image format for Cosmic user programs.
- [ ] Define kernel/firmware linker requirements.
- [ ] Add relocation tests to Forge toolchain.

---

# 9. Add a debug architecture later

This is not required for first Cosmic boot because the Rust VM can provide richer debugging externally, but real FPGA systems eventually need hardware debug support.

Possible `SIA32-D` work:

- [ ] hardware instruction breakpoint.
- [ ] hardware data watchpoint.
- [ ] single-step support.
- [ ] debug halt.
- [ ] debug resume.
- [ ] debug cause/status.
- [ ] register inspection/modification mechanism.
- [ ] memory inspection/modification mechanism.
- [ ] external debug-port architecture.
- [ ] interaction with interrupts and privilege state.
- [ ] interaction with SMP.

---

# 10. Define SMP platform architecture when needed

`SIA32-A` supplies atomics and ordering, but a complete SMP platform also needs processor-management mechanisms.

These should be specified separately from `SIA32-P`, likely as a Neutron/SIA SMP platform document.

- [ ] CPU/hart identification.
- [ ] discover number of CPUs.
- [ ] secondary CPU reset/startup state.
- [ ] secondary CPU release/start address.
- [ ] interprocessor interrupt generation.
- [ ] interprocessor interrupt acknowledgement.
- [ ] CPU halt/park.
- [ ] CPU restart.
- [ ] TLB shootdown protocol expectations.
- [ ] cache-coherence contract.
- [ ] memory-ordering implications for SMP.
- [ ] `SYNC.I` cross-CPU publication requirements.
- [ ] per-CPU timer/interrupt expectations.
- [ ] CPU failure/offline semantics if required.

---

# Immediate specification order

Work through the architecture in this sequence:

```text
SIA32-I                mostly defined
   |
   v
SIA32-P                defined
   |
   v
1. encoding freeze
   |
   v
2. multi-register fault semantics
   |
   v
3. memory model
   |
   v
4. cache + SYNC.I semantics
   |
   v
5. Lighting interrupt controller
   |
   v
6. Lighting timer
   |
   v
7. SIA ABI
   |
   v
8. object format + relocations
   |
   v
Rust full-system VM
   |
   v
Cosmic development
```

The debug and SMP specifications can follow once the single-CPU Lighting/Cosmic platform is working.

---

# Definition of "SIA32 system architecture complete"

For the first protected single-CPU Lighting implementation, SIA is complete enough when all of the following are stable:

- [ ] SIA32-I instruction semantics and encodings are frozen.
- [ ] SIA32-P is frozen.
- [ ] multi-register exception/restart behavior is frozen.
- [ ] memory ordering is frozen.
- [ ] `FENCE`, `SYNC.I`, and `TLBFENCE` relationships are frozen.
- [ ] MMIO ordering/cacheability behavior is frozen.
- [ ] DMA visibility/coherency rules are frozen.
- [ ] SIA ABI is frozen.
- [ ] object/relocation formats are sufficient for Forge.
- [ ] Lighting timer and interrupt-controller contracts are frozen.
- [ ] the Rust VM can implement all architectural behavior without inventing unspecified rules.
- [ ] Cosmic can boot and run protected user processes entirely against documented architecture.
