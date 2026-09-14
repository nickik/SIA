# Lighting-1 Diagnostic Console

## Status

This document freezes the minimal architectural diagnostic-output facility for the Lighting-1 platform.

It exists for ROM bring-up, early Cosmic diagnostics, manufacturing/field diagnostics, and deterministic simulation. It is intentionally much smaller than a general terminal or serial device.

## Address

```text
DIAG_BASE = 0xFFF03000
aperture  = 4 KiB
```

The facility is MMIO and follows the Lighting strongly ordered 32-bit MMIO rules.

## Registers

```text
offset   register   access   meaning
------   --------   ------   ---------------------------------------------
0x00     TX_CHAR    WO       emit low 8 bits as one diagnostic byte
0x04     STATUS     RO       bit 0 TX_READY; always 1 in Lighting-1
0x08..   reserved            access faults until assigned
```

`TX_CHAR` ignores bits 31..8. Writing one 32-bit word emits exactly one byte from bits 7..0.

`STATUS.TX_READY=1` means software may always write another byte. Lighting-1 defines no transmit FIFO depth, baud rate, receive path, interrupt, or blocking state for this diagnostic facility.

## Intended use

Typical firmware helpers are:

```text
putc(byte)
puts(zero_terminated_string)
puthex32(value)
panic(message)
```

Example conceptual flow:

```text
ROM reset entry
    -> puts("LIGHTING-1 ROM\n")
    -> memory diagnostics
    -> puts("RAM OK\n")
    -> PLIO discovery
```

This port is an architectural machine facility. A hardware Lighting implementation may route it to a maintenance UART, service processor, front-panel/debug connector, or equivalent implementation-specific sink while preserving the software-visible contract.

## Separation from emulator debugging

Simulator facilities such as instruction traces, breakpoints, watchpoints, MMU traces, register dumps, and recent-event history are **not architectural state** and are not visible through this MMIO block.

ROM and Cosmic must not depend on emulator semihosting for ordinary diagnostic output. Semihosting may remain available as an optional test convenience.

## Reset

The diagnostic console has no guest-visible mutable state in the base profile, so reset leaves `STATUS.TX_READY=1` and requires no software initialization.

Already emitted diagnostic bytes are external observations and are not architecturally "erased" by reset.
