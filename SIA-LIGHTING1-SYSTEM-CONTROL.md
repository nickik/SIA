# Lighting-1 System Control

This document freezes the minimal Lighting-1 machine-control MMIO block used by firmware, operating systems, conformance tests, and the reference virtual machine.

It intentionally contains only operations needed for machine bring-up and deterministic shutdown/reset. Watchdogs, RTCs, power-management states, boot-bank selection, persistent firmware state, and similar facilities are outside this first profile.

## 1. Address

```text
SYSTEM_CONTROL_BASE = 0xFFF02000
aperture size       = 4 KiB
```

All defined registers are aligned 32-bit MMIO words. Unsupported widths, misaligned accesses, reserved offsets, and invalid access directions raise the normal physical access fault through the Lighting MMIO rules.

## 2. Registers

```text
offset  register         access  meaning
------  ---------------  ------  ------------------------------------------
0x00    MACHINE_ID       RO      Lighting platform/profile identifier
0x04    RESET_CONTROL    WO      request architectural warm reset
0x08    HALT_CONTROL     WO      request machine halt / power-off
0x0C    STATUS           RO      current machine-control state
```

### 2.1 MACHINE_ID

Lighting-1 returns:

```text
MACHINE_ID = 0x4C495431
```

which corresponds to the ASCII identifier `LIT1` when written in register-value order.

This identifies the Lighting-1 platform profile; it is not a unique machine serial number.

### 2.2 RESET_CONTROL

Writing exactly:

```text
RESET_CONTROL = 1
```

requests an architectural warm reset.

Other values are invalid.

The store that requests reset retires. Before another guest instruction executes, the machine performs the warm-reset transition described below and begins instruction fetch at `RESET_VECTOR`.

### 2.3 HALT_CONTROL

Writing exactly:

```text
HALT_CONTROL = 1
```

requests machine halt / power-off.

Other values are invalid.

The store retires, after which no further guest instruction executes until an external reset/restart action. A virtual implementation reports this as a clean machine-halted outcome, not as a CPU exception.

### 2.4 STATUS

```text
bit 0  RUNNING
bit 1  RESET_REQUESTED
bit 2  HALTED
bits 3..31 reserved, read zero
```

`RUNNING` is set while the machine is not halted. `RESET_REQUESTED` may be transient because a conforming implementation can service the request immediately after the requesting store retires. `HALTED` is set once halt has taken effect.

Software must not require observing `RESET_REQUESTED=1` before reset occurs.

## 3. Warm reset

Warm reset establishes the same architectural CPU/platform reset state used for Lighting-1 startup while preserving installed RAM contents.

Required effects:

```text
CPU privilege       Supervisor
PC                  RESET_VECTOR = 0xFFFF0000
STATUS              0 (IE=0, VM=0)
EPC                 0
CAUSE               0
BADADDR             0
SCRATCH             0
VMCTX               0
CPU transient wait  cleared
TLB                 cleared
INTC                reset
monotonic timer     reset
system control      reset/running
PLIO0               reset when implemented
```

Required preservation:

```text
installed RAM       preserved
system ROM          unchanged
```

Host-side simulator diagnostics such as captured console history, trace buffers, breakpoints, and log files are not architectural guest state and may remain available across warm reset. Guest software cannot rely on or observe those host-only facilities except through the defined diagnostic-console MMIO interface.

## 4. Cold construction versus warm reset

A physical cold start and construction of a virtual Lighting machine may initialize backing RAM according to implementation/platform policy. The software-visible `RESET_CONTROL` operation is specifically the warm-reset operation above and therefore preserves RAM.

## 5. Intended firmware/OS use

Typical deterministic test completion:

```text
print "[TEST] foo PASS\n" to diagnostic TX_CHAR
write 1 to HALT_CONTROL
```

Typical software-requested restart:

```text
flush relevant software state
write 1 to RESET_CONTROL
<next guest fetch occurs at RESET_VECTOR>
```

No semihosting trap is required for either operation.
