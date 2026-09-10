# Lighting-1 Monotonic Timer

## Status

This document freezes the **Lighting-1 timer contract** used by the SIA platform and the LightingSimulation reference machine.

The timer is a platform device, not CPU architectural state.

```text
TIMER_BASE = 0xFFF01000
INTC source = 2 TIMER
```

There is no CPU timer register and no CPU timer-pending bit.

---

# 1. Design

Lighting-1 provides one simple monotonic one-shot timer:

```text
64-bit free-running counter
1 MHz frequency
1 tick = 1 microsecond
64-bit programmable deadline
one ENABLE bit
level-sensitive TIMER interrupt output
```

Periodic scheduling is synthesized in software by programming the next deadline after each timer event.

The timer never uses host wall-clock time as architectural state in the reference VM. Hardware implementations may derive the 1 MHz tick from any suitable stable clock source.

---

# 2. MMIO registers

All registers are aligned 32-bit MMIO words.

```text
offset   register       access   meaning
------   -------------  -------  ----------------------------------------------
0x00     COUNTER_LO     RO       low 32 bits of a coherent counter snapshot
0x04     COUNTER_HI     RO       high 32 bits of the most recent snapshot
0x08     DEADLINE_LO    RW       low 32 bits of programmed deadline
0x0C     DEADLINE_HI    RW       high 32 bits of programmed deadline
0x10     CONTROL        RW       bit 0 ENABLE; all other bits reserved
0x14     STATUS         RO       bit 0 EXPIRED; all other bits zero
0x18..   reserved       --       accesses fault
```

All 8-bit, 16-bit, unaligned, or otherwise unsupported MMIO accesses fault according to the Lighting system-MMIO rules.

---

# 3. Counter

The counter starts at zero on cold or machine reset and increments once per microsecond while the machine is running.

The counter is unsigned 64-bit and wraps modulo `2^64`.

At 1 MHz, wrap occurs only after approximately 584,542 years.

The counter is not writable.

## 3.1 Coherent 64-bit reads

Reading `COUNTER_LO` atomically snapshots the complete 64-bit counter and returns the low 32 bits of that snapshot.

The immediately following read of `COUNTER_HI` returns the high 32 bits of the same snapshot, regardless of intervening counter advancement.

Canonical software sequence:

```text
lo = read COUNTER_LO
hi = read COUNTER_HI
counter = (hi << 32) | lo
```

A read of `COUNTER_HI` before any `COUNTER_LO` read after reset returns the high 32 bits of the reset snapshot, which is zero.

Every new `COUNTER_LO` read replaces the saved snapshot.

---

# 4. Deadline programming

`DEADLINE_LO` and `DEADLINE_HI` hold the 64-bit one-shot deadline value.

Writing either deadline word does **not** by itself change `CONTROL.ENABLE`.

The recommended programming sequence is:

```text
write CONTROL = 0
write DEADLINE_LO
write DEADLINE_HI
write CONTROL = 1
```

Writing `CONTROL.ENABLE=1` arms or rearms the timer using the deadline value currently stored in the two deadline registers.

Writing `CONTROL.ENABLE=0` disarms the timer and immediately deasserts the timer's interrupt output.

Writing a new deadline while already enabled is permitted. The new combined deadline becomes effective after each individual 32-bit write. Software that requires atomic 64-bit replacement must use the disable/program/enable sequence above.

---

# 5. Expiration and interrupt semantics

The timer output is level-sensitive.

```text
EXPIRED = ENABLE && deadline_has_been_reached
```

When `EXPIRED=1`:

```text
STATUS.EXPIRED = 1
Lighting INTC source 2 TIMER = asserted
```

The timer remains expired and keeps source 2 asserted until software either:

1. writes `CONTROL.ENABLE=0`, or
2. programs a deadline that has not yet been reached.

`INTC.COMPLETE=TIMER` does **not** acknowledge or clear the timer device. If the timer remains expired, the controller immediately sees TIMER pending again after completion.

This keeps interrupt-controller claim/completion separate from timer-device acknowledgement.

## 5.1 Deadline already reached

If software arms the timer with a deadline that is already reached, the TIMER level asserts immediately.

This includes a deadline equal to the current counter.

---

# 6. Wraparound comparison

Deadline comparison uses modulo-`2^64` time ordering.

A programmed deadline must be less than `2^63` ticks into the future relative to the time at which it is evaluated.

Conceptually:

```text
reached(now, deadline) = ((now - deadline) mod 2^64) < 2^63
```

This gives unambiguous behavior across the 64-bit counter wrap while supporting deadlines up to approximately 292,271 years in the future at 1 MHz.

Normal operating-system deadlines are many orders of magnitude smaller.

---

# 7. Reset

On machine reset:

```text
COUNTER       = 0
snapshot      = 0
DEADLINE      = 0
CONTROL       = 0
STATUS        = 0
TIMER output  = deasserted
```

The counter begins advancing again when deterministic machine time advances.

---

# 8. Reference VM time

LightingSimulation uses **deterministic virtual machine time**.

For the initial correctness-first VM:

```text
1 retired CPU instruction = 1 timer tick = 1 microsecond
```

This is a simulation scheduling rule, not a promise that physical Lighting hardware executes one instruction per microsecond.

When the CPU executes `WFI`, the machine event loop may advance virtual time directly to the next armed timer deadline rather than busy-spinning through idle ticks.

This preserves deterministic architectural timer behavior while keeping simulation efficient.

Later timing models may assign more realistic instruction/device costs without changing the timer MMIO contract or its 1 MHz architectural frequency.

---

# 9. Frozen baseline

The Lighting-1 timer baseline freezes:

```text
1 MHz frequency
64-bit modulo counter
LO then HI coherent snapshot rule
64-bit deadline split across two words
CONTROL.ENABLE arm/disarm model
STATUS.EXPIRED
level-sensitive INTC TIMER source
past/equal deadline expires immediately
INTC COMPLETE does not acknowledge timer
reset counter/deadline/control state
```
