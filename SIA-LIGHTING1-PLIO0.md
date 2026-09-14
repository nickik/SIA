# Lighting-1 PLIO0 Host-Controller Profile

## Status

- Platform: **Lighting-1**
- PLIO logical protocol: **PLIO v0.6**
- Scope: CPU-visible host mapping, protected DMA authority, Notification delivery, and reset behavior
- Status: **normative Lighting-1 platform profile**

This document does not define QDX card protocols. A QDX-B, QDX-BA, graphics, GNet, or other card implements its own PLIO worker-space registers and device behavior.

Lighting-1 defines one PLIO host/segment:

```text
PLIO0_BASE = 0xFFE00000
PLIO0_END  = 0xFFEFFFFF
size       = 1 MiB
INTC source = 4
```

## 1. Logical PLIO resources

Lighting-1 supports:

```text
8 geographic slots
32 MiB worker space per slot
16 protected DMA channels per slot
4 Notification channels per slot
DMA bursts of 1, 4, 8, or 16 32-bit words
```

The card-side PLIO address model is independent of the CPU-side Lighting mapping.

## 2. Lighting IOchannel mapping

Lighting does not permanently map eight 32 MiB card spaces into CPU physical memory. Its 1 MiB host aperture is divided into:

```text
+0x00000..+0x7FFFF   host controller / tables / reserved
+0x80000..+0x8FFFF   IOchannel 0
+0x90000..+0x9FFFF   IOchannel 1
+0xA0000..+0xAFFFF   IOchannel 2
+0xB0000..+0xBFFFF   IOchannel 3
+0xC0000..+0xCFFFF   IOchannel 4
+0xD0000..+0xDFFFF   IOchannel 5
+0xE0000..+0xEFFFF   IOchannel 6
+0xF0000..+0xFFFFF   IOchannel 7
```

Each 64 KiB IOchannel maps one 64 KiB page of one PLIO slot.

Mapping registers:

```text
+0x0100 + channel*4   IOCHANNEL_MAP[channel]
```

Encoding:

```text
31       ENABLE
18:16    SLOT (0..7)
8:0      PAGE (0..511)
```

The PLIO worker address presented to the card is:

```text
(PAGE << 16) | offset_within_IOchannel
```

Worker MMIO supports naturally aligned 8-, 16-, and 32-bit accesses as defined by the card.

## 3. Host-controller registers

```text
+0x0000  ID              RO   "PLIO"
+0x0004  VERSION         RO   0x00000600
+0x0008  STATUS          RO
+0x000C  CONTROL         RW
+0x0010  SLOT_PRESENT    RO
+0x0014  NOTIFY_PENDING  RO
+0x0018  NOTIFY_ENABLE   RW
+0x001C  NOTIFY_MASK     RW
+0x0020  NOTIFY_CLAIM    RO, claiming
+0x0024  NOTIFY_DATA     RO
+0x0028  ERROR_STATUS    RW, nonzero write clears
+0x002C  ERROR_INFO      RO
```

`SLOT_PRESENT[7:0]` is the geographic card-presence bitmap.

`STATUS`:

```text
bit0 ENABLED
bit1 ERROR
bit2 DEVICE_PRESENT
bit3 eligible Notification pending
```

`CONTROL`:

```text
bit0 RESET
bit1 ENABLE
```

## 4. Protected DMA authority

PLIO devices never receive unrestricted host physical addresses as DMA authority.

Each slot owns sixteen host-programmed DMA capability channels. The table begins at `+0x1000` and contains 128 entries in slot-major, channel-minor order.

Each 16-byte entry is:

```text
+0x0 BASE        RW while unbound
+0x4 LENGTH      RW while unbound
+0x8 CONTROL     command/status
+0xC GENERATION  RO
```

`CONTROL`:

```text
bit0 VALID / BIND command
bit1 DEVICE_READ
bit2 DEVICE_WRITE
bit3 REVOKE command
```

A binding is valid only when its physical range lies wholly inside installed NORMAL RAM.

The device-visible 32-bit DMA address is:

```text
31:28 channel
27:24 generation
23:0  byte offset
```

Before beat zero the host validates:

1. legal 1/4/8/16-word burst and alignment;
2. channel is bound;
3. generation matches;
4. requested direction is permitted;
5. offset plus burst length lies inside the capability;
6. translated host range is installed NORMAL RAM.

A failed validation performs no DMA transfer.

Revocation invalidates the mapping. Rebinding increments the 4-bit generation. Lighting-1 does not silently wrap generation 15 to 0; software must reset PLIO before reusing that channel after exhaustion.

PLIO DMA uses host **physical** NORMAL memory and bypasses CPU virtual translation.

## 5. DMA coherence and memory system

Lighting-1 NORMAL RAM is coherent between CPU and PLIO DMA as required by `SIA32-MEM`.

The implementation may arbitrate CPU, MMU-walker, refresh, and PLIO memory use internally. These timing/arbitration details are not visible through PLIO registers.

A DMA write becomes visible before the device reports the corresponding completion Notification.

## 6. Bus-manager arbitration

A card requiring DMA or a Notification becomes a PLIO bus manager.

Lighting-1 uses rotating round-robin service between requesting geographic slots. One grant contains one bounded PLIO transaction. A DMA transaction is bounded to one legal PLIO burst.

The exact electrical timing of grants, turnaround, parity, or bus phases is outside the architectural contract.

The PLIO controller continues to make forward progress while the CPU is in `WFI`; otherwise an I/O completion could not wake the CPU.

## 7. Notifications

Each slot owns four Notification channels, giving 32 controller-side pending sources:

```text
bit = slot*4 + channel
```

Per-source Notification class registers begin at:

```text
+0x0400 + bit*4
```

Class values are `0..3`; larger classes have higher claim priority.

Eligible Notification state is:

```text
NOTIFY_PENDING & NOTIFY_ENABLE & ~NOTIFY_MASK
```

If any eligible source exists, PLIO0 asserts Lighting INTC source 4 as a level-sensitive input.

`NOTIFY_CLAIM` returns:

```text
7:0    slot
15:8   Notification channel
23:16  class
```

and removes that selected PLIO pending source. If nothing is eligible it returns `0xFFFFFFFF`.

`NOTIFY_DATA` returns the payload associated with the most recent successful PLIO claim.

Claim order is highest class first, then lowest slot/channel for ties.

Software normally services the nested interrupt hierarchy as:

```text
INTC CLAIM -> PLIO0
PLIO NOTIFY_CLAIM -> slot/channel/class
PLIO NOTIFY_DATA
service card
INTC COMPLETE -> PLIO0
```

If another eligible PLIO source remains, the level input remains/reasserts according to normal INTC semantics.

## 8. Reset

Lighting warm reset preserves installed RAM and physical card attachment, but resets PLIO host state.

PLIO reset clears:

- IOchannel mappings;
- protected DMA bindings and generations;
- Notification pending/payload state;
- Notification masks/classes to platform defaults;
- controller error state;
- arbitration position.

Every attached card also receives its card-reset condition.

## 9. Errors

Invalid worker mappings fault the CPU MMIO access.

Invalid DMA requests complete to the requesting device with a PLIO fault and latch controller error context. They do not grant unrelated memory authority.

The host exposes the current error code through `ERROR_STATUS` and slot/channel context through `ERROR_INFO`.

## 10. Relationship to QDX

PLIO0 provides:

```text
slot discovery
worker addressing
bus-manager arbitration
protected DMA
Notifications
aggregate interrupt delivery
```

QDX provides:

```text
device identity
block/graphics/network semantics
command and completion queues
card registers
media/device behavior
```

Therefore adding a new QDX architecture does not require modifying the Lighting PLIO host controller unless that card requires a future PLIO protocol extension.
