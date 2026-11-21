# mhal-scl-jpe-cmdq — the SCL scaler, the JPE MJPEG encoder and the CMDQ command queues of mhal.ko
<!-- ev:mhal-scl-jpe-cmdq -->

Three blocks of `mhal.ko` that work together on the video path of
this camera: **CMDQ**, eight hardware command-queue engines through which the other blocks and the MI modules queue
register writes, waits and polls; **SCL**, the VPE scaler that takes the ISP's realtime output (or a DRAM frame, the VIF,
a pattern generator or the rotate DMA) and writes up to six scaled output ports to DRAM or hands them to an encoder
through a handshake; and **JPE**, the MJPEG encoder at bank 1620 that `mi_venc` drives for the UVC MJPEG stream. This
document covers each block's hardware registers, driver layers, exports and their callers' contracts, interrupts, debug
surface and the parts present but never driven. Module-level facts (block map and sizes, module parameters, character
devices, DT properties mhal reads, ISRCB registry): mhal-core.md §2.1, §5, §4, §7, §3.4. RIU addressing and the bank → block index of the SoC,
interrupt numbers, clock tree, MIU: kernel-platform.md §3.1, §4.1, §14, §5.1, §12. OTP words and the throughput flag derived from them: ipl.md
§7.3. The callers: `mi_scl` — mi-capture.md §6; `mi_venc` — mi_venc.md §7; mi_sys's CMDQ objects and task completion —
mi_sys-1-engine.md §7.

Tags: **[R]** read from the binary, its disassembly or decompile, or from a cited file; **[S]** stated by a source;
**[I]** inference. A section heading's tag covers its text unless a statement carries its own.

## 1. Conventions and working files
<!-- ev:mhal-scl-jpe-cmdq.1 -->

- **Register notation** [R]: `bank:reg` = 16-bit RIU register `reg` (index = byte offset / 2) of bank `bank` (hex);
  `bank:reg[h:l]` a bit field; `1214+p` = bank 0x1214 plus port number p. CPU VA = `0xFD000000 + bank·0x200 + reg·4`,
  PA = `0x1F000000 + bank·0x200 + reg·4` (kernel-platform.md §3.1). Where an address is passed as one word (CMDQ commands,
  `HalUtility*`, JPE CMDQ records) it is the **RIU byte address** `bank<<8 | byteoff`, so the CPU address is
  `0xFD000000 + addr·2`. In §2 (CMDQ engine registers) and §4 (JPE) a bare `regNN` is a register of the block's own bank.
- **Addresses of functions** [R]: given as `.text` offsets of `mhal.ko` (objdump) or as addresses of the image rebased to 0x40000000
  (= `.text` offset + 0x40000028). Helpers without a symbol are named here by what they do [I].
- **Reference sources** [R]: the I2 SDK tree jockyw2001/sdk `src/mhal/i2/` (`vpe`, `jpe`, `cmdq_service`) and
  the CMDQ public header of OpenIPC u-boot OpenIPC `u-boot-sigmastar` `drivers/mstar/cmdq_service/drv/pub/mhal_cmdq.h`
  (2019–20). How close each block is to them: §2.1, §3.1, §4.1.
- **Extent**: struct layouts are given as far as the code reads them; register values are what the code writes — no live
  register dump of these blocks has been taken.
- **Use on this camera**: the blocks' callers on this camera are `mi_sys` (CMDQ), `mi_scl` (SCL, one instance on pipe 0),
  `mi_venc` (JPE, MJPEG), and mhal's ISP / DISP / LDC / GE blocks (CMDQ, as clients of the interface table). What this
  document marks **unused** is code that exists in the binary and is specified, but that nothing on this camera reaches.

## 2. CMDQ — hardware command queues

### 2.1 Engines and relation to the reference sources [R]
- **Eight engines** (`MDrvCmdqGetHWNum` = 8), banks from `gHalRegCmdCtlBase` = **1120, 1123, 1124, 1125, 1126, 1127, 1128,
  1129** (engines 0–7). DT nodes `sstar,cmdq<id>` give each engine's clock and interrupt (§2.5; DT properties read:
  mhal-core.md §7); `cmdq0..3` share GIC SPI 0x31, `cmdq4..7` share SPI 0x77.
- The I2 source jockyw2001/sdk `src/mhal/i2/cmdq_service` (2018, 5 engines) is the same driver family: same
  function names (`MDrvCmdq*`, `HAL_CMDQ_*`, `_MHAL_CMDQ_*` buffer pool), **same 64-bit command encoding and command types**,
  **same register layout inside an engine bank** (`hal_cmdq_reg.h` `REG_CMDQCtrl` matches every offset the M6 code
  touches). Every I2 `HAL_CMDQ_*` accessor exists in M6 with the same register fields (I2 `hal_cmdq.c` + `hal_cmdq_reg.h`
  specify them). M6-only HAL functions: `Buff_Read_Pointer`, `GetHWDummyRegAddr`, `GetTimeout_Mask`, `GetTrigEventStr`,
  `Get_Cmdq_RiuAddr`, `Get_Dummy2_Register_RiuAddr`, `Get/Set_Engine_Number`, `Get_Phy_TrigEvnt`, `Get_Reg_Bridge`,
  `Reset_All`, `SetWeight` (all `HAL_CMDQ_`-prefixed).
- The public API (prototypes, enums, usage examples) of all functions, M6-only ones included, is in the newer u-boot
  header `mhal_cmdq.h` (§1) [R]; the behaviour below is from the binary. Differences of this build:
  - **Ring-buffer mode only** for pool-backed services (`_MDrvCmdqCheckBufferAvail` prints "We only support
    RING_BUFFER_MODE ONLY!" for any other).
  - The interface table is **0x98 bytes** with 3 leading words and ISR-callback slots the u-boot header lacks (§2.3).
  - Trigger-event numbering is M6-specific (§2.8), not the u-boot enum.
  - New: buffer pool with best-fit split (`_MHAL_CMDQ_GetBestCmdqBuffer`), MemScript (software-only) queues with a hash
    table (`SS_ht_*`) for partial update (§2.9), direct mode and host/slave queues (§2.10), early-run hand-over of a
    U-Boot-started queue (§2.11), weights, `/proc/cmdq` statistics (§2.12).
- **Use on this camera**: the pool, id allocation, `GetSysCmdqService` and the hardware-queue slots of §2.3. MemScript,
  early run and byte mode have **no caller** anywhere (no `.ko` imports them either; callers inside mhal from the
  relocations of the disassembly); the direct-mode service and host/slave are used only by the ISP index mode
  (`CameraIspCmdqBuffer_IndexMode_*` of mhal's ISP block), not by the single ISP channel of this camera. §2.9–§2.12 are
  **unused** on this camera.

### 2.2 Exports mi_sys calls (`mhal_cmdq.c` layer, all under `gInitBufMutex`) [R]
| export | args | what it does |
|---|---|---|
| `MHAL_CMDQ_InitCmdqMmapInfo` | `info*` = u-boot `MHAL_CMDQ_Mmap_Info_t` (+4 MIU addr, +8 kernel VA, +0xc size) | once (flag): pool = one free area `{miu, va, size}` in a list `gCmdBufferHead`, kmem cache "cmdq…" (0x28-byte area records); `DrvCmdqStatInit`. mi_sys passes `CMDMEM` (`cmdQBufSize` KiB, 3 MiB with demo.sh) |
| `MHAL_CMDQ_DeinitCmdqMmapInfo` | — | if no service holds an area: free the pool records; else −1 "pool released but cmdq service still in used" |
| `MHAL_CMDQ_GetCmdqHWNumber` | — | 8 |
| `MHAL_CMDQ_GetFreeId` | — | lowest free id 0..7 (bitmask under a mutex), −1 if none |
| `MHAL_CMDQ_GetSpecificId` | id | the same for one id (tail call `MdrvCmdqGetFreeId(id)`); −1 if taken |
| `MHAL_CMDQ_ReleaseId` | id | clear the id bit |
| `MHAL_CMDQ_GetSysCmdqService` | id, `{u32 size; u16 align}`, bForceRIU | see below; returns the table of engine `id` or NULL |
| `MHAL_CMDQ_ReleaseSysCmdqService` | id | return the buffer area to the pool (merge), release the private data, clear the table's ctx slot |

**`GetSysCmdqService(id, desc, forceRIU)`**: the pool must be initialised (else NULL); id < 8. First call ever: fill the
8 tables (`gCmdqOps[8]`, 0x98 each, §2.3), `CmdqProcInit` (`/proc/cmdq/*`, §2.12). If the engine has no service yet:
`MDrvCmdqInit(id, 0)` (§2.5) → private data (0x1a8 bytes, `gDrvCmdPriv[id]`); `priv+0xc = forceRIU`; `size` rounded up to
`align` (0 → 16); best-fit area from the pool (16 bytes at its end zeroed); `MDrvCmdqSetBufInfo` programs the engine
(§2.5); table slot +0x4c = priv. A second Get for the same id returns NULL ("double allocate"). mi_sys always passes
`forceRIU = 0` (mi_sys-1-engine.md §7.1). In forceRIU mode every write goes straight to `0xFD000000 + addr·2` and
kick / poll / wait are no-ops.

### 2.3 The interface table (`MHAL_CMDQ_CmdqInterface_t`, 0x98 bytes) [R; callers from the MI decompiles]
Every slot function takes the table pointer first and works on `table+0x4c` (priv); slots +0x10..+0x48 check
`table+0x8c == 0` (0 = hardware queue, 1 = MemScript) and return −1 otherwise. The MemScript variant of the table: §2.9.
| off | function (args after the table) | used by |
|---|---|---|
| 0x00–0x08 | not set (zero) | — |
| 0x0c | CheckBufAvailable(n cmds) → free slots or 0 (waits ≤ 151 × 10 ms for room, then dumps) | mi_isp, mhal SCL / JPE |
| 0x10 | WriteDummyRegCmdq(u16) — append a write of the value to the engine's dummy register `reg78` | **mi_sys** (task tag) |
| 0x14 | ReadDummyRegCmdq(u16 *out) — read `reg78` directly (CPU) | **mi_isp, mi_scl, mi_disp** (task done = tag reached) |
| 0x18 | WriteRegCmdqMask(addr, u16 val, u16 mask) | mhal blocks |
| 0x1c | WriteRegCmdqMaskMulti(`{u32 addr; u16 val; u16 mask}[]`, n) → written count | mhal blocks |
| 0x20 | not set for hardware queues (MemScript multi-write) | — |
| 0x24 | WriteRegCmdq(addr, u16 val) | mhal blocks |
| 0x28 | WriteRegCmdqMulti(`{u32 addr; u32 val}[]`, n) | mhal blocks |
| 0x2c | CmdqPollRegBits(addr, val, mask, bPollEq) | mi_venc (`0x132BFC`, mask 0xF), mhal JPE |
| 0x30 | CmdqAddWaitEventCmd(event) | mhal blocks |
| 0x34 | CmdqAbortBuffer() — write pointer back to the last kick | |
| 0x38 | CmdqResetEngine() — reset SW pointers, soft-reset pulse | mi_venc |
| 0x3c | ReadStatusCmdq(u32 *out) — `reg45` decoded (§2.6) | |
| 0x40 | KickOffCmdq() → commands since last kick, −1 on error | **mi_sys, mi_disp, mi_venc** |
| 0x44 | ClearTriggerEvent() — 4 writes of 0xFFFF to `reg54` | |
| 0x48 | IsCmdqEmptyIdle(bool *out) — `reg45` bit 11 | mi_disp |
| 0x4c | ctx = private data | |
| 0x50 | KickOffCmdqByTag(tag) (prints "not suport", then kicks with a cache flush) | |
| 0x54 | CmdDelay(ns) (§2.6) | |
| 0x58 | GetCurrentCmdqNumber() | |
| 0x5c | CmdqPollRegBits_ByTime(addr, val, mask, eq, ns) | mhal SCL |
| 0x60 | CmdqGetAppEventDummyRegAddr() → `bank<<8 | 0xEE` (`reg77`) | |
| 0x64 | SetWeight(w) (bus weight, engines 0–3 only, §2.12) | |
| 0x68–0x88 | not set for hardware queues (MemScript ops, §2.9) | — |
| 0x8c | type (0 = hardware) | |
| 0x90 | CmdqRegisterIsrCallBack(fn) — `priv+0x19c = fn`, called with the engine id on an error IRQ | **mi_sys** (`MI_SYS_IMPL_CMDQ_ResetIsrNotify`) |
| 0x94 | CmdqDeleteIsrCallBack() | |

`addr` everywhere is the RIU byte address `bank<<8 | byteoff` (§1), the same form mhal blocks use for direct access.

### 2.4 Engine registers [R; names from the I2 `REG_CMDQCtrl`]
Registers of one engine (base = the engine's bank, §2.1):
| reg | use in this build |
|---|---|
| 00 | bits 0–1 "engine number" (written only on engines 0 and 4: `|= 2`) |
| 01 | bit 0 enable |
| 02 | mode, bits 1–2: 0 direct, 1 increment, 2 ring (`HAL_CMDQ_Set_Mode`; pool-backed services: bits cleared, then `|4` = ring) |
| 03 | bit 1 start / move command pointer (kick); bit 8 latch the read pointer before reading 0a/0b |
| 04/05, 06/07 | buffer start / end pointer, MIU address >> 4 (low / high 16 bits) |
| 08/09 | software write pointer, MIU address >> 4 (the kick target) |
| 0a/0b | hardware read pointer (>> 4) |
| 0c/0d | SRAM read / write pointer (debug) |
| 10 | bit 1 MIU1 select (set when the buffer's MIU address has bit 31) |
| 11 | request length / threshold / burst: `|= 0x190D` at enable (`|= 0x90D` on engines 0 and 4 at global init) |
| 20 | bit 7 mask setting |
| 23 | bit 2 trigger-bus sample enable (I2 `CMDQ_EN_TRIG_SAMPLE`, §2.12) |
| 28/29 | timeout: 28 = 0, 29 = 0xFF07 (base 0xFF, amount 7 [I]) |
| 31 | bit 0 soft reset (active low: clear then set) |
| 3d | low byte = type of the command that timed out (0xB0 / 0x30 / 0x20 / 0x10) |
| 44 | interrupt status (read by the ISR and by kick) |
| 45 | raw status; bit 11 idle |
| 47 | interrupt mask (1 = masked); bit 8 poll-neq timeout, 9 poll-eq timeout, 10 wait timeout, 12 write timeout (`irqevt`, §2.12) |
| 48 | interrupt clear (write 1s) |
| 4a/4b | poll timer / ratio: `MDrvCMDQSetTimerRatio(id, 0xFF, 0xF)` ORs both values into **reg 4a** (`HAL_CMDQ_Set_Ratio` is a copy of `Set_Timer` — vendor bug [I]) |
| 50–53 | engine 0's bank only: MIU weights of engines 0–3 (§2.12) |
| 54 | trigger-bus clear (write event bits) |
| 77 | "HW dummy" (app event register) |
| 78 | dummy register = task tag |

**Bank 1001 / 1201, register 17, bit 0**: `HAL_CMDQ_SetBank` sets it in both banks for every engine. The I2
`HAL_CMDQ_SetBank` writes `0x10012e` with the comment "write 0x100117 bit 0 as 1 to enable riu mask write function" [S: I2
source comment]; banks 1001 and 1201 are read as the RIU bridges of the two register segments (0x1F20xxxx and
0x1F24xxxx), so masked writes from the CMDQ (and the mask-write path of `HalUtilityW2BYTEMSK`) work in both [I]. No M6
register reference names the bit; the write itself is [R].

### 2.5 Engine set-up [R]
- **Global init** (first `MDrvCmdqInit`): private data reset (RIU base 0xFD000000 for all); for each engine 0..7:
  `HAL_CMDQ_SetBank` (also sets bit 0 of `1001:17` and `1201:17`, §2.4), clear IRQ `reg48 |= 0x1700`, mask
  `reg47 = 0xFFFF`; then `Set_Engine_Number(8)` (engines 0 and 4: `reg00 |= 2`, `reg11 |= 0x90D`).
- **Per engine** (`MDrvCmdqInit(id, 0)`): mode = 4 (ring); clock = DT `sstar,cmdq<id>` clock 0 rate in MHz (default 432);
  IRQ = DT irq 0 of the same node; if U-Boot left the engine running ("early run", bit in `gDrvCmdPriv+0xdd4`, §2.11), wait
  ≤ 10 s (10 000 × 1 ms) for `reg78 == 0xA55A`; `reg01 |= 1`, `reg11 |= 0x190D`, `reg20 |= 0x80`, `reg29 = 0xFF07`,
  `reg28 = 0`; soft-reset pulse `reg31` bit 0 → 0 → 1; `reg02 = (reg02 & ~6) | 4`; `reg4a |= 0xFF`; `reg48 = 0xFFFF`;
  `reg48 |= 0x1700`; **`reg47 = 0xFEFF` (only bit 8, the error / timeout interrupt, enabled)**;
  `CamOsIrqRequest(irq, _MDrvCmdqStatusIsr, "cmdq<id>", priv)`. `MDrvCmdqInit(id, 1)` sets up direct mode instead (§2.10).
- **Buffer** (`MDrvCmdqSetBufInfo`): MIU select (`reg10` bit 1); SW start = area MIU address, end = start + size − 16,
  write = start; `reg04/05 = start>>4`, `reg06/07 = end>>4`, `reg08/09 = start>>4`, soft-reset pulse `reg31`. Capacity =
  size / 8 commands.

### 2.6 Commands, kick, completion, errors [R]
- **Command** = 8 bytes in the ring, stored as two LE words `lo, hi`: `lo = value<<16 | maskfield`, `hi = type<<24 |
  (addr>>1)` (`addr>>1` = `bank<<7 | reg`). Types: **0x10 write**, **0x20 wait trigger-bus event** (`lo = ~eventbits &
  0xFFFF`, value 0), **0x30 poll until (reg & mask) == value**, **0xB0 poll until not equal**, **0 = null** (`hi 0, lo
  0xFFFF`, padding).
- **Mask field**: `maskfield = ~mask` (1 = keep bit); for a full write (mask 0xFFFF) `MDrvCmdqWriteCommandMask` and
  `MDrvMemCmdqWriteCommandMask` store **0xFFFF**, not 0 [R]. Poll commands use `~mask` always. The I2 source encodes the
  field as `~mask` always (a full write → 0) [R]. With "1 = keep this bit" (I2's meaning, which the partial masks need),
  0xFFFF would keep every bit, so the M6 engine treats 0xFFFF as "mask off, plain write" [I]; the null command uses the same
  value with type 0. Not confirmed on hardware; one queued write with field 0 against one with 0xFFFF would show it.
- **Wait event** = one 0x20 command + 4 write commands `reg54 = eventbits` (clear the trigger bus).
- **Kick** (`MDrvCmdqKickOfftCommand`): under the engine mutex; if the write pointer is not 16-byte aligned, append a null
  command; if `reg44 & 0x1700` (read twice) → print, dump, return −1; else remember the previous fire pointer, new fire =
  write>>4, `CamOsMiuPipeFlush()` (no cache flush — the `CMDMEM` mapping must be uncached [I]), `reg08/09 = fire`,
  `reg03 |= 2`; return the number of commands queued since the last kick.
- **Free space**: read pointer from `reg0a/0b`·16 (after `reg03 |= 0x100`) against the SW write pointer in the ring; a
  write never waits for the engine except through this check.
- **Completion** has no interrupt: the client appends a tag write to `reg78` (slot 0x10) before the kick and later
  compares `reg78` (slot 0x14) — mi_sys, ISP, SCL and DISP do exactly this (mi_sys-1-engine.md §7.2). Idle = `reg45`
  bit 11.
- **Errors**: only bit 8 is unmasked. `_MDrvCmdqStatusIsr`: status = `reg44`; if bit 8, refine by `reg3d` low byte (0xB0
  → 0x100 poll-neq timeout, 0x30 → 0x200 poll-eq timeout, 0x20 → 0x400 wait timeout, 0x10 → 0x1000 write timeout); clear
  `reg48 |= status`; if `status & 0x1700`: call the registered callback (mi_sys `ResetIsrNotify`, which schedules the pass
  reset — mi_sys-1-engine.md §7), dump the failing command and pointers. Recovery = slot 0x38 (SW pointers to start, soft
  reset). `_MDrvCmdqStatusSingleIsr` reads the final IRQ status, then runs the common ISR; `_MDrvCmdqHwDummyIsr` (the
  dummy-register IRQ) clears `reg77`.
- **Delay** (slot 0x54) is a deliberate timeout: mask IRQs (`reg47 = 0xFFFF`), program `reg28/29` from the ns value and the
  clock, a poll-eq on `reg01 == 0xFFFF` that can never match, restore `reg28 = 0`, `reg29 = 0xFF07`, clear `reg48`,
  restore the mask — all as queued commands. The timeout and poll-time values come from `_MdrvCmdCalcu{Timeout,PollTime}Vlaue`
  (used by `CmdDelay` and `PollRegBits_ByTime`).

### 2.7 Kick variants and driver extras [R]
- Kick by RIU / by tag (`KickOfftCommandByRiu`, `…ByTag` — the latter prints "not suport" and kicks).
- Forbidden tags (`_MdrvCmdAdd/CheckForbiddenTag`): up to 10 tag values the dummy register must not take, shown in
  `engdump` (§2.12).
- Crash analysis (`MDrvCmdqAnalysisCrashCommand` / `PrintfCrashCommand`): decode the failing command, bank / offset and
  event name.

### 2.8 Trigger-bus events (`gstCmd0TrigEvt`, 16 lines, same for all engines) [R]
`{u16 event id, u16 bit, char name[16]}`; `HAL_CMDQ_Get_Phy_TrigEvnt(id, event)` maps the API event to the bit:
| bit | event | name | bit | event | name |
|---|---|---|---|---|---|
| 0x0001 | 25 | ISP_TRIG | 0x0100 | 34 | CODEC_INT |
| 0x0002 | 30 | ISP_VSYNC | 0x0200 | 7 | DUMMY09 |
| 0x0004 | 31 | DISP0_INT | 0x0400 | 35 | WDMA0_DONE |
| 0x0008 | 32 | DISP1_INT | 0x0800 | 36 | WDMA1_DONE |
| 0x0010 | 22 | DMA_DONE | 0x1000 | 37 | WDMA2_DONE |
| 0x0020 | 21 | JPE_TRIG | 0x2000 | 38 | WDMA3_DONE |
| 0x0040 | 29 | IVE_TRIG | 0x4000 | 26 | LDC_TRIG |
| 0x0080 | 33 | BDMA_TRIG | 0x8000 | 1 | DUMMY15 |

- Event **99** → bits 0|1 (ISP_TRIG | ISP_VSYNC) [R].
- Event **0x11** is remapped to **0x26** (= 38, WDMA3_DONE, bit 0x2000) [R]. In the common event enum of the u-boot
  `mhal_cmdq.h`, 17 = `E_MHAL_CMDQEVE_DIP_TRIG` [R]; M6 has no DIP, and the remap reads as a compatibility alias for code
  that waits on "DIP_TRIG" for the scaler write-back [I]; no source states the reason.
- SCL waits on events 35–38 (§3.10), JPE on 21 (§4.10).

### 2.9 MemScript queues and partial update (hash table) — unused [R]
- `MHAL_CMDQ_CreateMemoryScriptCmdq(desc)` (`desc+0x20` u16 size; buffer from `CamOsDirectMemAlloc`): one of 32 contexts
  (`gCmdqMemScriptCtx`, 0xc0 each: in-use byte, `+4` hash table (0x24), `+0x28` the returned 0x98-byte interface table).
  Table slots (§2.3 numbering): `0x20` = `MHAL_MemCMDQ_WriteRegCmdqMaskMulti`, `0x4c` = private data, `0x68`
  KickOffMemScriptCmdq, `0x6c` GetCurPos, `0x70` MemCmdqAbortBuf, `0x74` SeekToNextWritePos, `0x78` GetMemCmdqUsedSize,
  `0x7c` MemCmdqAddWaitEventCmd, `0x80` MemCmdqPollRegBits, `0x84` WriteRegWithSeq, `0x88` WriteRegWithSeqMask, `0x8c` = 1
  (type MEMSCRIPT); the hardware slots `0x10–0x48` refuse (type ≠ 0). Private data (`gDrvCmdMemScriptPriv[i]`, 0x1a8, same
  layout as a hardware engine's): mode 4, MIU address `+0x4c`, virt `+0x50`, size `+0x54`, start / end / write
  `+0x58/+0x5c/+0x60` (end = start + size − 16), capacity `+0x78` = commands, hash table `+0x1a0`, master flag `+0x1a5`.
- Commands use the hardware 8-byte format (§2.6) and are written into the script buffer, never to an engine.
- **Sequence keys**: `WriteRegWithSeq(addr, seq, value)` (seq ≠ 0) builds a key `{addr[23:0] | seq << 24, seq >> 8}`; the
  first time the command is appended and the hash table stores key → byte offset; later calls with the same key overwrite
  the command at that offset (in-place update of a recorded script). `WriteRegWithSeqMask` the same with a mask; waits
  (`MemCmdqAddWaitEventCmd(seq, event)`, key `event | seq << 16 | 0x2000`) and polls likewise; the wait also appends the
  four trigger-clear writes (§2.6).
- `BeginPartialUpdate(addr, seq, flag)`: positions the write pointer at the recorded offset of that key (or the end when
  unknown) and sets `+0x1a6 = seq`, `+0x1a4 = flag`; `EndPartialUpdate` clears them. `SeekToNextWritePos(pos)` (8-aligned,
  inside the written part; `0xffffffff` = end); `GetCurPos` = write − start; `GetMemCmdqUsedSize` = commands × 8;
  `MemCmdqAbortBuf` = empty the script and its hash table (refused on a master that still has duplicates);
  `GetMemCmdqBufAddr` = physical start.
- `CreateDupMemCmdq(src, desc)`: a second script buffer sharing the master's hash table (reference count `table+0xc`);
  `DupMemScriptInfo(dst, src)` copies the commands; `DestoryMemoryScriptCmdq` frees (master only after its duplicates).
- **Execution**: `KickOffMemScriptCmdq(hw, script)` copies the script's buffer pointers into the hardware engine's private
  data, programs the engine's start / end registers to the script buffer (`MDrvCmdqMemScriptSetBufInfo`) and kicks it
  (`MDrvMemCmdqKickOfftCommand`: null-pad to 16 bytes, refuse on a pending error `0x1700`, write pointer `08/09`, start) —
  the engine executes the script in place, no copy.
- Hash table (`SS_ht_*`): a generic chained hash table — buckets array, capacity ≥ 8 doubling when full, key 8 bytes, value
  4 bytes, default hash / compare, insert-or-replace, lookup / contains / erase / clear / copy / swap / move / reserve /
  rehash / destroy.

### 2.10 Direct mode, host / slave (ISP index mode only) [R]
- `MHAL_CMDQ_GetSysCmdqServiceBufferDirectMode(id)`: same 0x98 table as `GetSysCmdqService` (and the same lazy init of
  the 8 tables, pool re-init if requested, `/proc`), but the engine is set up with `MDrvCmdqInit(id, 1)` = **direct mode**
  (`reg02[2:1]` = 0) and gets no pool buffer: the client supplies the command buffers. Release:
  `ReleaseSysCmdqServiceBufferDirectMode`.
- `MHAL_CMDQ_HostControlSlave(host, slaveVirt, slavePhys, nCmds, slaveId)`: one or two null commands are put at the end
  of the slave buffer (n commands padded to an even count) and the cache is flushed; then the host engine's queue gets
  writes that program the slave engine — slave `04/05` = start >> 4, `06/07` = end >> 4, soft-reset pulse slave `31[0]`
  0 → 1, then slave `03 = 2` (start). `HostAddWaitSlaveIdle(host, slaveId)`: the host polls slave `45[11]` (idle) == 1.

### 2.11 Early run (U-Boot / pre-driver queues) — unused [R]
- `MHAL_CMDQ_DirectModeEarlyRun(id, phys, size)` → `MDrvCmdqDirectModeEarlyRun`: size 16-aligned; banks / engine number
  set once; clock from DT (`sstar,cmdq<id>`, default 432 MHz); enable, debug pause / continue (`HAL_CMDQ_Reset2`), direct
  mode; the last command slot of the buffer is overwritten with a write of **0xA55A to the engine's own `reg78`** (bytes
  `00 00 5A A5`, address, type 0x10); buffer start / end programmed, early-run bit `gDrvCmdPriv+0xdd4` set, cache flushed,
  start. `MDrvCmdqInit` later waits for `reg78 == 0xA55A` on engines with that bit (§2.5) — so a queue started before the
  driver ("EarlyInit done") finishes before the driver takes the engine.
- `DirectModeEarlyRunStop(id)`: trigger-bus clear × 3, pause / continue, bit cleared. `GetEarlyRunStatus()`: 0x9090
  (`E_CMDQ_EARLYRUN_DONE`) or 0xBC5D (`…_BUSY`) from `HAL_CMDQ_Read_Idle_Done`.
- `DirectModeByteModeCmd(id, sel, out[8])`: builds one byte-lane write command for a register reached through the
  engine's RIU bridge (`HAL_CMDQ_Get_Reg_Bridge`: bank of the engine | 0x40, mask 0xff or 0xc7): bytes `C3 FF lo hi addr…
  10`; `sel` = `E_CMDQ_BYTE_SEL_LBYTE` (2) only (`HBYTE` refused).

### 2.12 Weights, trigger-bus sample, `/proc/cmdq` — unused [R]
- **Weight** (slot 0x64 → `HAL_CMDQ_SetWeight(engine, w)`; no direct caller in mhal): engines 0–3 only, through engine 0's
  bank (1120): `1120:50[0]` = 1 (weight enable, set once), `1120:51[3:0]` engine 0, `[11:8]` engine 1, `1120:52[3:0]`
  engine 2, `[11:8]` engine 3 — engines 2/3 are OR-ed in without clearing (a lower weight cannot be set back) [R]. Meaning:
  MIU request arbitration weight among the CMDQ engines [I].
- `HAL_CMDQ_Enable_TriggerBusSample(id)`: `reg23[2]` = 1 (I2 `CMDQ_EN_TRIG_SAMPLE`, "enable trigger sample"). No caller in
  any binary of the dump; what the sample does is not known.
- `/proc/cmdq/` (`CmdqProcInit`, created with the first service): `dbglvl` (r: level; w: `%li`, bits 1/2/4 = debug levels,
  7 all), `dumpall` (w: engine id, or 100 = debug dump of a buffer overrun; r: the selected engine's whole buffer),
  `dumpcmd` (w: engine id; r: the commands of its last kick, "no fire yet" before one), `engdump` (r: context dump of all
  engines — mode, ids, MIU / size / addresses, fire pointer and counts, forbidden tags; HW version string "V1.4"),
  `finalcmd` (r: last command per engine), `irqevt` (r: interrupt mask per engine; w: one hex mask applied to every open
  engine, bit 0 = poll-neq timeout (`reg47[8]`), 1 = poll-eq (`[9]`), 2 = wait (`[10]`), 3 = write (`[12]`)), `stat`
  (r: per engine buffer s/e/w/r, SRAM w/r, depth, command-type counts, kick history; w: engine id → reset statistics).
  All debug.

## 3. SCL — the VPE scaler

### 3.1 Relation to the reference sources [R]
<!-- ev:mhal-scl-jpe-cmdq.3.1 -->
The I2 source jockyw2001/sdk `src/mhal/i2/vpe` (`mhal_vpe.c`, `drv_scl_*`, `hal_scl_*`) is **not** this
driver's design: its API is `MHalVpeSclCropConfig/InputConfig/OutputMDWinConfig/ImiPortConfig/SetSwTriggerIrq` over one
HVSP/DMA/VIP pipeline. The M6 API (`InstanceAttach/Detach`, `PreCropConfig`, `PortMirrorFlipConfig`,
`Send/ReceiveSwMessage`, `GetActiveBuf`, `Reset`, `GetQpHistogram`) and the layers under it (`DrvSclIf*`, `DrvSclCtx*`,
`HalSclQuery` with Get/Set "query" functions, `HalSclHw*`) have no public source: no public tree we hold contains these
names (`.c/.h` searched for `MHalVpeSclInstanceAttach`, `DrvSclIfAttachSclInstance`, `HalSclHwSetDmaBufConfig`); only other
M6 `mhal.ko` builds have them. What carries over from I2: the register-write utility (`HalUtilityW2BYTE*`, "inquire
buffer", CMDQ fire — `hal_utility.c`), the OS shim names (`DrvSclOs*`), the debug / proc names and the filter-SRAM load
procedure (§3.14). Everything below is from the binary. Register helpers 0x400e93fc … 0x400e9b98 are named here by what they write.

### 3.2 Layers, queries, register access, contexts [R]
- `MHalVpeScl*` (exports): argument checks + debug print, then one `DrvSclIf*` call. Return 1 = OK, 0 = fail (bool).
- `DrvSclIf*`: per-instance state; compares a new config with the stored one and only then issues a **query**:
  `scl_if_query(inst, q)` (0xf3840) → `HalSclQuery(halctx, q)`: 9 query types, each a pair Get (compute / validate, `+0xd8` table)
  and Set (program, `+0x48` table) installed by `HalSclInit` in `.bss` after `gbDbgMessageLv`:
  | type | name | Get → Set → hardware function | payload size |
  |---|---|---|---|
  | 0 | ROTATION | `_HalSclGetImageCfg` → `_HalSclSetRotationCfg` → `HalSclHwSetDmaRot` | 0x10 |
  | 1 | MIRRORFLIP | `_HalSclGetImageCfg` → `_HalSclSetMirrorFlipCfg` → `HalSclHwSetDmaMirror` + `HalSclHwSetDmaFlip` | 0x10 |
  | 2 | DMACONFIG | `_HalSclGetDmaCfg` → `_HalSclSetDmaCfg` → `HalSclHwSetDmaConfig` | 0x18 |
  | 3 | SIZECONFIG | `_HalSclGetSizeCfg` → `_HalSclSetSizeCfg` → `HalSclHwSetSizeConfig` | 0x5c |
  | 4 | INPUTCONFIG | `_HalSclGetInputCfg` → `_HalSclSetInputCfg` → `HalSclHwSetInputConfig` | 0x28 |
  | 5 | PROCESS | `_HalSclGetProcessCfg` → `_HalSclSetProcessCfg` → `HalSclHwSetDmaBufConfig` | 0xf8 |
  | 6 | FLIPREG | `_HalSclGetFlipRegCfg` → `_HalSclFlipRegCfg` → `HalSclHwRegFlip` | 0x10 |
  | 7 | QP_HIST | `_HalSclGetQpHistogramCfg` → `_HalSclQpHistogramCfg` → `HalSclHwGetQpHistogram` | 8 |
  | 8 | RESET | `_HalSclGetSwResetCfg` → `_HalSclSwResetCfg` → `HalSclHwSwReset` | 0xc |

  Further query types are used by the IRQ layer (§3.12).
- **Register access** (`HalUtility*`): a write either goes straight to RIU (`0xFD000000 + addr·2`, read-modify-write with
  mask) or, when the "access mode" is 1 (during config) / a CMDQ is attached, is appended as `{u32 addr, u16 val, u16
  mask}` to an **inquire buffer** (≤ 0x32 entries for the direct-CMDQ list). `HalUtilityFilpRegFire` /
  `W2BYTEMSKDirectCmdqWrite` / `W2BYTEMSKTblWrite` push a buffer through the CMDQ table: slot 0x0c `CheckBufAvailable(n+10)`
  (retry while busy), then slot 0x1c `WriteRegCmdqMaskMulti(buf, n)`; without a CMDQ they write RIU directly. Waits use
  slot 0x30 `AddWaitEventCmd`, or slot 0x5c `PollRegBits_ByTime`, or a CPU poll (≤ 200 × delay).
- **Instance** = `DrvSclCtxAllocate`: up to 0x32 contexts per class, each with three heap blocks (0xc00 driver, 0x780 "MHAL
  ctx" with the fields below, 0x938 HW ctx). Fields used everywhere: `+0x774` **pipe type** (internal input mode, §3.3),
  `+0x770` frame-input flag, `+0x77c` output-port mask, `+0x768` frame counter, `+0x666/+0x668` max / input W/H, `+0x738`
  compression request (§3.7), `+0x6d4`, per-port size / format arrays, per-port enable byte `+0x108`.

### 3.3 Pipes, ports, Attach, routing [R]
`MHalVpeSclInstanceAttach` → `DrvSclIfAttachSclInstance(cfg)` (cfg = `{u16 maxW, u16 maxH, input, dev, port mask}`): pipe
type (`ctx+0x774`) = input 0 → 0, 1 → 1 (or 3 when dev = 1), 2 → 2, 3 → 4, 4 → 5, else 0; input 5 → 0 (dev 0) / 2 (dev 1)
/ 6 (else) with `ctx+0x6d4 = 3`. Accepted combinations: pipes 0, 1, 2, 4 with dev 0, pipe 3 with dev 1 (others print
"Not Support This Device Inputmode" but are not refused). Port mask (`ctx+0x77c`) = cfg bits 0–8 (output ports 0–5 +
more), `| 0x1000` for inputs 1 and 4, `= 0x3000` for input 3. Frame-input flag `ctx+0x770` = input 1, 3, 4.
| pipe | name (from strings and use) | input | used on this camera |
|---|---|---|---|
| 0 | ISP realtime | ISP → scaler | **yes** (mi_scl dev 0, input 0) |
| 1 | RDMA0 (frame input, bank 1212) | DRAM → scaler | no |
| 2 | VIF realtime / pattern generator (timing generator in `1210:30–3f`) | VIF or PTGEN | no |
| 3 | RDMA1 (bank 1213) | DRAM | no |
| 4 | rotate DMA (bank 1226, port 6 in / port 7 out) | DRAM → rotate → DRAM | no |
| 5, 6 | "special dev" variants of input 5 | — | no |

Pipe types 5 and 6 have no branch of their own in the HW layer: the per-pipe register selects either fall back to the
pipe-0 registers or skip the write.

**Ports**: MI ↔ internal index (`scl_port_mi_to_internal` (0x400e3a02) / `scl_port_internal_to_mi` (0x400e39da)): MI ports 0–5 = outputs 0–5, MI 9 = internal 7 (rotate
out, "ROTOUT"), MI 10 = internal 6 ("INPUTPORT", RDMA / rotate in), MI 6–8 invalid. MI port 11 is the input crop of the
pipe in `PreCropConfig` (§3.5).

**Routing** `1210:06` (`HalSclCreateChannel` → `HalSclHwChanOpen`, 2 bits per output port p at `[2p+1:2p]`): pipe 0 → 0,
pipe 2 → 1, pipe 1 → 2, pipes 3/4 → 3. ChanOpen then sets, per enabled port p, from the port's enable byte v (`+0x108`):
`1214+p:02[0]` = !v (`scl_set_port_disable` (0x400e99e0)), `1220+p:01[3]`, `[1]`, `[2]` = v, `1210:07[p]` = !v (`scl_set_port_mask` (0x400e99b0)),
`1220+p:01[15]` = v (`scl_set_port_enable_hi` (0x400e9a08)), and the pipe's type bit = v (`scl_set_pipe_type_bit` (0x400e9960): pipe 0 → `1210:02[p]`, 2 →
`1210:02[8+p]`, 1 → `1210:03[p]`, 3 → `1210:03[8+p]`); for pipes 1/3 with the input port enabled: the type bit and
`1212/1213:01[15]` (`scl_set_input_type_bit` (0x400e9a34)); for pipe 4 with port 6: `1210:06[15]` = 1 (rotate path) and `1219:02[0]` = 0, with
port 7: `1226:00[8]` = 1.

[S] (SigmaStar Tiramisu MI_SCL document `scl_zh` §1.2, §2.11): six hardware scalers; device 0 takes only the ISP
port 0 realtime link; maximum output width 8192 (YUV422) or 4608 for HWSCL0 / HWSCL1 and 3840 for HWSCL2–5; outputs bind
frame-based, realtime to VENC JPEG, or ring / HW_AUTOSYNC to VENC H.26x; upscaling with VIF → ISP → SCL all realtime risks an
ISP FIFO full. [S] (SigmaStar FAE, 2022-07, Tiramisu): the scaling ratio must not exceed 16×.

### 3.4 Banks [R from the code; roles I]
| bank | role (from what the code does with it) |
|---|---|
| 1210 | SCL top: soft reset `01`, pipe type bits `02/03`, clock / line-buffer and routing fields `05–1c`, routing `06`, `1d` per-port LOW-mode enable (§3.7), CMDQ-trigger IRQ groups `20–2f` (4 × {mask, force, status, clear}, §3.10), RDMA triggers `10`/`13`, input timing generator `30–3f` (§3.16), interrupt masks `40–42`/`50–52`/`60–62`/`70–72` (4 IRQ "tops"), flags `46–48` (+0x10 per top), clear `49–4b` |
| 1211 | encoder handshakes: `01` per-port RGB path bits, `02–0c` thresholds, `10–18` VENC / JPE / JPD handshake control and ring addresses (§3.8) |
| 1212, 1213 | read DMA 0 / 1 (frame input, pipe types 1 / 3): control `01`, format `02/03`, addresses `08..18`, request thresholds `21–24`, ring addresses `28–2f`, strides `30–32`, `37`, JPD handshake `38/39`, `50–53`, thresholds `55–5d` |
| 1214–1219 | per-output-port pre-scaler crop (port p = 1214+p): `02[0]`, `10` enable, `11/12` input W/H, `13/14` X/Y, `15/16` crop W/H, `60` (chip revision 2); `1214:61` pipe-1 CMDQ-idle flag |
| 121a–121f | per-port HVSP scaler (port p = 121a+p): ratios / phases `01–0a`, filter / mode `0b–0c`, SRAM port `41–45`, `51`, QP histogram `52/53`, chroma swap `55[3]`, sizes `60–65`, output size `20–23`; per-pipe status words `58–5b` in 121a (pipe 0), 121b / 121e (trigger and active-port records), 121c (init flag `5b`, blanking flags), 121d (pipe 2), 121f (pipes 1, 3/4 frame counters) (§3.10) |
| 1220–1225 | per-port write DMA (port p = 1220+p): `01` bit 0 frame enable + control bits, `02/03` format, `04/05` output W/H, `08–0f` Y addresses (4 ring slots × 32-bit), `10–17` C, `18–1f` V, `21–24` request thresholds, `30/31` Y/C stride, `32` bit 0, `37` handshake / frame index, `38–3f` handshake addresses, `40` lock owner word (1221 JPE, 1222 JPD, 1223 VENC), `42` MatchV; `1224:40` pipe 3/4 CMDQ-idle flag; `1225:40` (RIU byte address 0x122580) rotate / "others" busy and IRQ bit |
| 1226 | rotate DMA (pipe type 4): `00` control, `01` format and angle, `04–07` strides, `08–0d` input, `10–15` output addresses, `1e` IRQ |
| 1003:05 | hardware semaphore of the engine locks `HalSclHw{Set,Get}{Jpe,Jpd,Venc}Lock` (§3.8) |
| 1038:64 | `fclk1` clock select (`HalSclHwSetFclk1Clock`) |
| 1a29:7b | VPU bank register used as a software mailbox by `Send/ReceiveSwMessage` (§3.15) |

Most SCL bit fields have no known name: their positions and the code's use of them are known, and no M6 register reference
is available.
Port widths limited by the code: scaler line-buffer mode `121a+p:0b` bit 11 set when output W > 4096 (ports 0, 1), > 2048
(port 5), > 1920 (ports 2–4) [R constants].

### 3.5 Exports `mi_scl` calls [R]
| export | args (offsets of the config struct) | what happens |
|---|---|---|
| `MHalVpeSclInit(ops*, 0)` | ops = mi_scl's {alloc, free, map, unmap, flush} (mi-capture.md §6) | once (flag `gbDbgLevel+4 == 2`): keep the ops (`DrvSclOsKeepAllocFunction`; used by `DrvSclOsDirectMemAlloc` for SRAM / inquire buffers), IRQ bookkeeping init (all 4 tops masked `0xFFFFFFFF/0xFFFF`, CMDQ-idle flags set), context table init, clocks (`DrvSclModuleClkInit`), `HalSclInit` (query tables, `HalSclHwInit` §3.13, LPF filter, HVSP SRAM tables for ports 0/1) |
| `MHalVpeSclDeInit()` | — | `HalSclDeInit`, free contexts, IRQ exit, clocks off |
| `MHalVpeCreateSclInstance(&inst)` | — | `DrvSclCtxAllocate(0)` |
| `MHalVpeDestroySclInstance(inst)` | — | free context |
| `MHalVpeSclInstanceAttach(inst, cfg)` | `+0 u16 maxW, +2 u16 maxH` (clamped to 8192), `+4 input, +8 dev, +0xc out-port mask` | pipe type and port mask (§3.3); all port sizes reset to maxW/H, format "none" (4); `HalSclCreateChannel` → `HalSclHwChanOpen` (routing `1210:06`, §3.3); also `1226:00` |
| `MHalVpeSclInstanceDetach(inst)` | — | if last user of the HW: `HalSclCloseHw` (forced CMDQ IRQ §3.11, `HalSclHwClose`: `121a:60/62/64`, releases both output locks per port §3.8), Reset query; clean context |
| `MHalVpeSclPreCropConfig(inst, cfg)` | `+0 port, +4 u8 en, +6 x, +8 y, +0xa w, +0xc h` (u16) | port 11 = input crop of the pipe, ports 0–5 = per-port crop; stored; if changed: SIZECONFIG query for all 8 ports |
| `MHalVpeSclPortSizeConfig(inst, cfg)` | `+0 port, +4 mode, +8 arg, +0xc w, +0xe h` | stored per port; if changed → SIZECONFIG for all ports (§3.6) |
| `MHalVpeSclDmaPortConfig(inst, cfg)` | `+0 port, +4 MI pixel format, +8 compress, +0xc write mode, +0x10 u16 MatchV` | format mapped and stored → DMACONFIG query (`HalSclHwSetDmaConfig`, §3.7) |
| `MHalVpeSclPortMirrorFlipConfig(inst, cfg)` | `+0 port, +4 u8 mirror, +5 u8 flip` | MIRRORFLIP query (`1220+p:02`, §3.7) |
| `MHalVpeSclRotationConfig(inst, cfg)` | `+0 rot` | stored for the rotate port → ROTATION query (§3.11); pipe type 4 only |
| `MHalVpeSclProcess(inst, cmdq, buf)` | `buf` = per-port frame descriptors, 0x68 bytes each, indexed by MI port (§3.9) | §3.10 |
| `MHalVpeSclSetWaitDone(inst, cmdq, type)` | | §3.10 |
| `MHalVpeSclGetActiveBuf(buf)` | | per port `+0x60` = "port produced a frame" from the IRQ bookkeeping (`DrvSclIrqGetPortActive`) |
| `MHalVpeSclReset(inst)` | | RESET query → `_HalSclSetSwReset` (§3.11) — direct RIU |
| `MHalVpeSclSendSwMessage(inst, msg)` / `MHalVpeSclReceiveSwMessage(inst, msg)` | `{+0 ?, +4 type, +8 u16 count, +0xc *value, +0x10 *mask}` | mailbox `1a29:7b` (§3.15) |
| `MHalVpeSclGetIrqNum(&irq, cfg)` | `+4 mode, +8 pipe, +0xc sub` | Linux IRQ of "top" t from DT `sigmastar,vpe` (`DrvSclOsGetSclIrqID`): pipe 0 → top 0, pipe 2 → top 1, pipe 1 → top 2 (sub 0) / 3 (sub 1), pipe 3 → top 3 |
| `MHalVpeSclEnableIrq(cfg)` | `+4 mode bits (& 0x33), +8 pipe, +0xc sub, +0x10/+0x14 6-bit masks, +0x18 u8 enable` | set the ISR open mode, then unmask (`1210:40–42` of the top, clear `49–4b`) the 6 interrupt groups of the pipe (`HalSclIrqQueryInfo`, §3.12), or mask them |
| `MHalVpeSclCheckIrq(cfg)` | `+8 pipe, +0xc sub` | `DrvSclIrqIsrHandler`: read flags (`1210:46–48` of the top), mask with enable, per group update counters / CMDQ-idle / active-port state, **clear (`1210:49–4b`) inside**; return bits: 1 = VSYNC (group 0), 2 = SC DONE (group 1), 8 = VENCST (group 2), 0x10/0x20/0x40 = MATCH V0/V1/V2 (groups 3–5, only when enabled by the mode); 4 is never produced (§3.12) |
| `MHalVpeSclClearIrq()` | | returns 1, does nothing (clearing is in CheckIrq) |
| `MHalVpeSclGetQpHistogram(inst, port, &out)` | | QP_HIST query (§3.15) |

The DT IRQ numbers of the four tops (`sigmastar,vpe` `interrupts[0..3]`) and the char device: mhal-core.md §7, §4.2.

### 3.6 Size configuration (`HalSclHwSetSizeConfig`, port p) [R]
<!-- ev:mhal-scl-jpe-cmdq.3.6 -->
- Crop `1214+p:10` bit 0 enable, `11` in W, `12` in H, `13` X, `14` Y, `15` crop W, `16` crop H.
- Scaler `121a+p:60/61` (bit 15 enable, 13-bit value), `62/63` in W/H, `64/65`.
- **H ratio** (1.20 fixed point, 0x100000 = 1:1): `07` = ratio[15:0], `08` bits 0–7 = ratio[23:16], `08` bit 8 = H scaling
  on, `01`/`02` initial phase = ratio/2 (0 at 1:1); **V ratio**: `09`, `0a` bits 0–7, `0a` bit 8 on, `03`/`04` phase.
- Filter mode `0b` (bits 0, 1–3, 6–7 H; 8, 9–10, 14–15 V; bit 11 line-buffer mode for wide output, §3.4), `0c` bits 6–11
  (fixed-filter select via `_HalSclHvspGetEnFixedFilter`); output size `20/21`, `22/23`; bypass `_HalSclHvspSetBypassEn`
  (`121a:08`) when both ratios are 0 and W fits.
- Write DMA size `1220+p:04` W, `05` H. Ports 6/7 = RDMA / rotate sizes (`1212/1213:04/05`, `1226`).

### 3.7 DMA format and write mode (`HalSclHwSetDmaConfig`) [R]
`MHalVpeSclDmaPortConfig(inst, {port, MI format, compress, write mode, u16 MatchV})`:
- **MI → HAL format** f: YUYV 0→0, YVYU 0xf→1, UYVY 0xe→2, VYUY 0x10→3, **NV12 0xb→4** (tile 0xd→4), NV21 0xc→5, 0x36→6,
  SP422 0xa→7, 422P 0x11→8, 420P 0x12→9, BGRA 3→0xa, ABGR 2→0xc, ARGB 1→0xd, RGB565 4→0xe.
- **Write mode** (`+0xc`, strings in `_HalSclGetDmaCfg`): 0 NORMAL (frame), 1 DUAL, 2 RING (VENC handshake), 3 IMI, 4 EMI,
  5 REMI (JPE handshake), 6 LOW (low-latency, line-count match).
- **Compression** (`+8`, `ctx+0x738`): every non-zero value is refused ("Compress [%s] Not Support", error 3) — there is no
  compression in this build; nothing is written for it.
- `1220+p:37[10:8]` = 4 for write modes 1–5 (handshake on), 0 for modes 0 and 6.
- Mode 6 (LOW): `1210:1d[p]` (direct, mask 0xff) and `1220+p:42` = MatchV (direct).
- Then `HalSclHwSetJpeLock(port, mode ∈ 3–5)` and `HalSclHwSetVencLock(port, mode ∈ 1–2)` (§3.8).
- **Output ports 0–5** (`1220+p:02/03`, port p < 6): `02[2:0]` = 4 | (f ∈ {1,3,5}) | (f ∈ {2,3}) << 1 (component order),
  `02[3]` = 420 (f 4, 5, 6, 9), `02[4]` f 9 (420P), `02[5]` f 7, `02[6]` f 8 (422P), `02[7]` packed 422 (f < 4),
  `02[8]`/`02[9]` RGB (f 10–13), `02[14]` RGB; `03[9:8]` 2 for f 10/11, 1 for 12/13, `03[11:10]` 3/1/2/0 for f
  10/11/12/13, `03[13:12]` 1/3/0/2, `03[15]` RGB, `03[7:0]` = 0. RGB also sets `1211:01[p]` and clears `1211:01[8+p]`
  (`scl_set_rgn_path` (0x400e9858); per-port RGB / CSC path select [I]; the bits have no name). Request thresholds 0x30 (YUV) / 0x7e (RGB)
  into `21[15:8]`, `23`, `24[7:0]`, `21[7:0]` (`scl_set_req_threshold` 0x400e9478 / 0x400e94ec / 0x400e959c; skipped when the debug flags `gbFunc+0xb/0xc` are
  set); port 5 also `1219:03[0]` = (f == 6).
- **Input port 6**: pipes 1/3 — RDMA `1212/1213:02`: `[11:10]` = 1 for packed 422 or RGB, `[12]` f ∈ {2,3}, `[9:8]` = 3
  unless RGB, `[4]` and `[2]` = 420 (f 4–6, 9), `[7]` f < 4, `[5]` f 7, `[6]` f 8, `[3]` f 9; `03`: `[9:8]`, `[11:10]`,
  `[13:12]` RGB order as for outputs, `[15]` RGB; `1212/1213:37[10:8]` = 0; `scl_set_input_rgb` (0x400e97ca): `50[0]`, `53[0]`, `53[4]` =
  RGB, `52[0]` = 0; `121a+q:55[3]` (each output port q in the mask) = f ∈ {1, 3, 5} (chroma swap); JPD lock
  (`mode ∈ 3–5`). Pipe 4 — `1226:01[8, 3:0]` = format code (YUYV 0 → 4, 0xe → 1, 2 → 0x104, 7 → 5, 8 → 6, 4 → 8, 9 → 9,
  10 → 0).
- **Mirror / flip** (`HalSclHwSetDmaMirror` / `Flip`): `1220+p:02` bits 13 / 10 (mirror) and 12 (flip).

### 3.8 Encoder handshakes and engine locks [R]
- **Engine locks** (`HalSclHw{Set,Get}{Jpe,Jpd,Venc}Lock`): a hardware semaphore `1003:05[1:0]` (write 1, read back 1 =
  owned, ≤ 65 537 tries, release by writing 0) guards three owner words: `1221:40` (JPE), `1222:40` (JPD), `1223:40` (VENC)
  = `0xBE10 | port` (JPD: `| pipe type`) while bound, 0 when released by the same owner. Get: 1 = held by this port,
  2 = free, 0 = held by another port ("already be used"). `HalSclHwClose` releases both output locks per port.
- **VENC handshake** (write modes 1/2; `HalSclHwSetDmaBufConfig` for the port named in the VENC view, through the direct
  CMDQ list): `1211:10[2:0]` = port (7 = none), `1211:10[3]` = enable (only with `gbFunc+2`), `1220+p:37[15:12]` = frame
  index, `1211:11/12` = Y ring start >> 4, `1211:13/14` = Y end >> 4 − 1, `1211:15/16` = C start, `1211:17/18` = C end − 1;
  turning it off writes zeros once (`gbFunc+0x578` remembers "on").
- **JPE handshake** (modes 3–5): `1211:10[10:8]` = port, `[11]` = enable (with `gbFunc+1`), `[12]` = IMI (1) / EMI (0);
  `1220+p:37[15:12]` = frame index; IMI / EMI: `1220+p:3c/3d` = Y size − 1 and `3e/3f` = C size − 1 in 16-byte units (ring
  height × stride, halved for 420 chroma; no height → whole stride·2); REMI (ring EMI): `1220+p:38/39` Y start, `3c/3d` Y
  end − 1, `3a/3b` C start, `3e/3f` C end − 1. `1220+p:37[0]` = JPE handshake flag (`scl_set_jpe_handshake` (0x400e9a7c)).
- **JPD handshake** (input port 6 on pipes 1/3, JPD decoding straight into the scaler): `1211:10[14]` = RDMA1 (pipe 3),
  `[15]` = handshake on, `1212/1213:38/39` = rows·stride / 8 − 1 (rows from the descriptor, min 16); YUYV only.
- The JPE side of the handshake: JPE input modes 2/4 (ring) and 3 (IMI) with the frame index in `1620:00[7:4]` (§4.8) — SCL
  and JPE agree on the slot through the frame-index fields.
- **RDMA ring** (pipes 1/3, input from a ring): `1212/1213:02[15]` = ring (when the descriptor's ring height ≠ 0 and the
  format is not 8/9), `1212/1213:28/29` Y start >> 4, `2a/2b` Y end, `2c/2d` C start, `2e/2f` C end (C height halved for
  420).
- Initial ring thresholds `1211:02–0c`: §3.13. The VENC / JPE handshakes serve the HW_RING bind (prog_uvc's `-H` ring
  mode); the stream path of this camera does not run in that mode and its ports write NORMAL frames [I].

### 3.9 The per-port frame descriptor of `MHalVpeSclProcess` (0x68 bytes) [R]
The array is indexed by MI port (0–5, 9, 10 — at least 11 entries):
| off | size | field | use |
|---|---|---|---|
| 0x00 | 3 × u64 | plane phys addresses Y, C, V | WDMA / RDMA / rotate addresses |
| 0x18 | 3 × u32 | strides Y, C, V | |
| 0x24 | 4 | — | not read |
| 0x28 | 3 × u64 | handshake buffer start addresses (ring / IMI base) | VENC / JPE / JPD handshake (§3.8) |
| 0x40 | 3 × u64 | handshake buffer end addresses | |
| 0x58 | u16 | ring height in lines | handshake |
| 0x5a | u16 | frame index (slot) | WDMA slot = index & 3; written to `1220+p:37[15:12]` |
| 0x5c | u16 | start line | handshake |
| 0x60 | u8 | port enabled for this frame | `1220+p:01[0]` arm, trigger |

`DrvSclIfProcess` copies every descriptor into the port context twice — a JPE-handshake view with flags bJpeHk = write
mode 3–5, bEmi = 4–5, bRing = 5, and a VENC view with bRing = mode 2, bDual = mode 1 — counts statistics (blanking /
active, per-interrupt fps, `gbHistints`), then runs the PROCESS query on all 8 ports (`scl_query_process_all` (0x400e386a), stops at the first
port whose Get fails with 1/2) and the FLIPREG query.

`_HalSclGetProcessCfg` per port: addresses moved to the last line for vertical flip (Y + stride·(H−1); C/V per format:
formats 4–6 half height, 7 full, 8/9 two planes), to the last pixel for mirror (`+W−1`, packed 422 ×2, RGB ×4, others
16-aligned); strides halved / quartered for packed / RGB (pixel units); "venc bind multi-port" / "jpe bind multi-port" when
more than one port uses the same handshake (error 1); "Jpd HK only support YUYV" (formats ≥ 4 on the input handshake,
error 1); stride alignment check (`stride % align`: 0x80 format 6, 0x20 formats 8/9, 8 packed, 4 RGB, 8 format 0xe, 0x10
else → "must be … align", error 2); pre-crop larger than the source → error 2; port disabled → 5.

### 3.10 Per-frame sequence [R]
`mi_scl` per frame (mi-capture.md §6.5): Pre-crop / Dma / Size / MirrorFlip config per port (writes only when something
changed — they land in the inquire buffer), then `SendSwMessage` (optional), then:
1. **`MHalVpeSclProcess(inst, cmdq, buf)`** (`DrvSclIfProcess`): store `cmdq` in the context; copy the port descriptors
   (§3.9); if the pipe's CMDQ-idle flag is set (table below), clear it and count a "blanking" / "active" event (`121a:59` /
   `121d:59` decide which); frame counter `+0x768 += 1`; PROCESS query on all 8 ports → `HalSclHwSetDmaBufConfig` per port:
   Y/C/V addresses `1220+p:08/10/18` (+4·slot, value = MIU address >> 4, 32-bit as two registers), strides `30/31`, `32`
   bit 0 (enable), `37` bit 0, and the handshake registers of §3.8; then the FLIPREG query → **`HalSclHwRegFlip`**:
   - CMDQ-idle flag bit 0 = 0, start frame counter = frame counter, blanking bit (`121a:59[1]` for pipe 0) = 0 (direct CMDQ
     list);
   - push the inquire buffer (all queued config writes) through CMDQ slot 0x1c; load changed HVSP SRAM filter tables
     (§3.14);
   - CMDQ-trigger IRQ group of the pipe: mask bit = 0 (unmask), clear bit = 1 (pipe 0: `1210:20` bit 8 = 0, `1210:23` bit 8
     = 1);
   - for each port with a buffer: `1220+p:01` bit 0 = 1 (arm DMA), trigger bit of the port (pipe 0: `121b:59` |= 1<<p);
   - extra trigger of pipes 1/3/4 (table); push the list.
2. **`MHalVpeSclSetWaitDone(inst, cmdq, type)`** → `HalSclHwWaitDone`: **CMDQ wait on trigger-bus event 35 (WDMA0_DONE)
   for pipe 0** (36 pipe 2, 37 pipe 1, 38 pipes 3/4); without a CMDQ a CPU poll of the group's status bit (pipe 0:
   `1210:22` bit 8; `26` / `2a` / `2e` for the others); then CMDQ-idle flag bit 0 = 1, done frame counter = frame counter,
   group clear bit = 1, histogram bookkeeping (active-port record = active ports, trigger register cleared), blanking bit =
   1; push. A delay command (`HalUtilityAddDelayCmd`) is computed but that function is a stub.
3. `mi_sys` appends the tag write (CMDQ slot 0x10) and kicks; SCL's interrupt (`CheckIrq`) serves only statistics,
   active-port state and waking mi_sys — completion of the task is the CMDQ tag (§2.6).

Registers per pipe (CMDQ list unless noted):
| step | pipe 0 | pipe 2 | pipe 1 | pipes 3/4 |
|---|---|---|---|---|
| CMDQ-idle flag cleared (RegFlip) / set (WaitDone) | `121a:58[0]` | `121d:58[0]` | `1214:61[0]` | `1224:40[0]` |
| frame counter at start | `121a:5a` (+ `121a:59[1]` = 0) | `121d:5a` (+ `121d:59[1]` = 0) | `121f:58` | `121f:5a` |
| frame counter at done | `121a:5b` | `121d:5b` | `121f:59` | `121f:5b` |
| trigger bits (port p) | `121b:59[p]` | `121e:59[p]` | `121b:5b[p]` | `121e:5b[p]` (rotate: `[7]`) |
| active-port record at done | `121b:58[7:0]`, clear `121b:59` | `121e:58`, `121e:59` | `121b:5a`, `121b:5b` | `121e:5a`, `121e:5b` (rotate 0x80) |
| CMDQ-trigger IRQ group {mask, force, status, clear} | `1210:20–23` bit 8 | `1210:24–27` bit 9 | `1210:28–2b` bit 10 | `1210:2c–2f` bit 11 (rotate bit 15) |
| CMDQ wait event | 35 WDMA0_DONE | 36 | 37 | 38 |
| extra trigger | — | — | `1210:10[0]` (RDMA0 start) | pipe 3: `1210:13[0]`; pipe 4 with port 7: CPU poll `1225:40[0] == 0` (≤ 2 s), `1225:40[0]` = 1, `1210:01[15:14]` pulse, `1226:00[0]` = 1 |
| at done | `121a:59[1]` = 1 | `121d:59[1]` = 1 | — | pipe 4: `1210:06[15]` = 0, `1226:1e[9:8]` = 3 |

### 3.11 Rotation, software reset, forced CMDQ interrupt [R]
- **Rotation** (`MHalVpeSclRotationConfig {rot}` → `_HalSclSetRotationCfg`, pipe 4 only): 0 off, 1 = 90°, 2 = 180°,
  3 = 270° → `1226:01[5:4]` (`HalSclHwSetDmaRot`). Rotate DMA addresses (`HalSclHwSetDmaBufConfig`, full byte addresses,
  not >> 4): input (port 6) strides `1226:04/05`, Y/C/V `1226:08–0d`; output (port 7) strides `1226:06/07`, Y/C/V
  `1226:10–15`.
- **Software reset** (`_HalSclSetSwReset`, RESET query): `1210:01` = 2 (pipe 0), 4 (pipe 2), 8 (pipe 1), 0x10 then 0x4000
  then 0x8000 (pipes 3/4), 1 (all), then `1210:01` = 0 — direct RIU.
- **`HalSclForceCmdqIrq(mode)`** (used by `HalSclCloseHw`): mode 2 = if the group's status has no unmasked bit, mask all,
  reset the pipe, restore the mask, force 0; mode 1 = force 0xffff; mode 0 = force 0 and a clear pulse.

### 3.12 Interrupts [R]
Each pipe has six interrupt groups (`HalSclIrqQueryInfo` cases 0–5), named by `HalSclIrqExecHandle`; their bits sit in
the low / high word of the top's flag register (`1210:46–48` + 0x10 per top; masks `1210:40–42` …, clear `49–4b`):
| group | name | bit | statistic code |
|---|---|---|---|
| 0 | **VSYNC** (frame start) | low: pipe 0 → 0x1, pipe 2 → 0x2, pipe 1 → 0x4, pipe 3 → 0x8, pipe 4 → none | 2 (start time, active ports) |
| 1 | **SC DONE** (scaler frame done) | low: pipe 0 → 0x200, pipe 2 → 0x400, pipe 1 → 0x800, pipe 3 → 0x1000; pipe 4 high 0x8000 | 8 (done count, active time) |
| 2 | **VENCST** (VENC start) | high 0x40 | 0x10 |
| 3–5 | **MATCH V0 / V1 / V2** (line-count match, the LOW write mode's MatchV) | high 0x100 / 0x200 / 0x400, only when IRQ mode bits 8/9/10 are set | 0x1000000 / 0x2000000 / 0x4000000 |

Query types 10/11 read the trigger / active-port registers (§3.10), 12–14 a port's current Y/C/V address, 15/16 the done /
start frame counters. `HalSclIrqExecHandle` side effects: VSYNC → blanking flag off (`121a:59[1]` / `121d:59[1]` = 0, or
`121c:59[0]` / `121c:5a[0]` for pipes 1/3), SC DONE → on (`DrvSclIrqGetIsBlankingRegion`). `MHalVpeSclCheckIrq` reports
the groups as return bits (§3.5). The IRQ statistics are shown by the sysfs `check_*_ints` files (§3.16).

### 3.13 Init (`HalSclHwInit`) [R]
<!-- ev:mhal-scl-jpe-cmdq.3.13 -->
Runs at the first open only while `121c:5b[0]` is clear (not already set up by an early init) and sets it at the end.
Writes, in order (direct RIU unless noted):
1. Chip revision (`CamOsChipRevision()`) == 2 → `1214..1219:60 = 1`.
2. `1210:08 |= 0x80` (reads this write as `1210:08 = 0x80`), `1210:12 = 0xff`, `1210:1a[15:8] = 0x10`,
   `1210:1a[7:0] = 0x10`, `1210:1b[7:0] = 8`, `1210:1c[0] = 0`, `1210:1c[3:1] = 0`, `1210:1c[15:8] = 0x0c`, `1210:30[1] = 0`,
   `1214..1219:10 |= 0x80`, `121a..121f:51 |= 8`.
3. CMDQ-able: `1210:07[13:8] = 0x3f`, `1210:05[2] = 1`, `[0] = 1`, `1210:0d[2:1] = 1`, `1210:0a[2:1] = 0`,
   `1210:10[2:1] = 2`, `1210:11 = 0x700`, `1210:13[2:1] = 2`, `1210:14 = 0x700`, `1211:02[15:8] = 0`, `1211:03 = 0x8080`,
   `1211:01[15] = 1`, `1211:04–0c` (14 bits) and `1212/1213:55–5d` (13 bits) from the threshold table at `.data`
   `gbFunc+0x554…0x576`.
4. Per output port: request thresholds `1220+p:21–24` = 0x30 / 0x30 / 0x18, `1220+p:01` bits 1–3 = 1 (bit 3 and the two
   others through `scl_set_out_bit3` (0x400e9426) / `scl_set_out_bits12` (0x400e944e)); input port 0x20 / 0x10; `1210:06[15] = 0`; per port all four type bits
   (`1210:02/03`) and `1210:07[p]` reset.
5. `1214+p:02[0]` = 0 → filter SRAM load of the 12 tables (`scl_load_filter_tables` (0x400e9fdc), `121a+p:41`, §3.14) → `1214+p:02[0]` = 1.
6. `121c:5b[0] = 1`, `1225:40[0] = 0`.
The chip revision of this camera's SoC has not been read; other M6 boards report "soc-revision 2" (22 and the
sibling camera in; kernel-platform.md §3.2), so the revision-2 step most likely runs here [I]. `1214:60` = 1
after boot would show it.

### 3.14 HVSP filter SRAM [R]
- **Tables**: four coefficient tables of 64 phases × 5 bytes in `.data` — table id 0x01 = `MST_SRAM1_Main[0..0x13f]`,
  0x11 = `MST_SRAM1_Main[0x140..]`, 0x00 = `MST_SRAM2_Main[0..]`, 0x10 = `MST_SRAM2_Main[0x140..]`; `MST_SRAM_LPF` (`.data`
  0x1D7F4, 20 B) = 4 × 5-byte low-pass entries (levels 0–3).
  `gbSram[12]` (`.data`) = the table id loaded per port, H0–H5 then V0–V5: all 0x01 at init.
- **Load** per table (`scl_load_filter_table` (0x400e9ec8), port register base `121a+p:41`): `41[0] = 1`; for each of the 64 phases: `42 = phase`
  (+0x40 for vertical tables), `43 = b0 | b1<<8`, `44 = b2 | b3<<8`, `45[7:0] = b4`, `41 |= 0x301` (write pulse); finally
  `41[0] = 0`. Same procedure as the I2 source (`HalSclHvspSRAMDumpHSP/VSP`).
- **Content**: the four M6 tables are **not** any of the eight named I2 filters (`InvSinc4Tc4p4…`). The 40-bit entry
  layout (5 bytes → taps) is not decoded: the obvious 4 × 10-bit split does not give constant tap sums. A scaler sweep with
  a test pattern on the hardware would expose it; reproducing the scaler's behaviour needs only the bytes as they are.
- **Per frame** (`HalSclHwRegFlip`): the table id is chosen by the scaling ratio (`scl_pick_filter_table` (0x400e9880), ratio in 1.20 fixed point):
  exactly 1.0 or 0 → 4 (keep) or 0x11 (the LPF table) depending on the call's second argument; 0.5–0.8 → 1; < 0.5 or
  0.8–1.0 → 4; 1.0–1.18 → 4; 1.18–1.67 → 0; > 1.67 → 0x10; skipped while `gbForceSram` holds the port. A table that differs
  from the loaded one is loaded through `HalUtilityPrepareSRAMTbl` + `HalUtilityW2BYTEMSKTblWrite`.
- **Overrides**: `gbForceSram[port]` + `gpvHvspFilterTbl[port]` = a user table from the sysfs file `check_hvsp`
  (`echo <hvspId> <up/down> <file_v> <file_h>`). `HalSclLpfFilterInit`: if `gbForceSram+0x1a` (u16 LPF level 1–3) is set,
  table 0x11 is filled with the LPF entry of that level (all phases; source `MST_SRAM_LPF + 5·level + i%5`, so it reads at
  most up to 0x1D807).

### 3.15 QP histogram, software messages — unused [R]
- `MHalVpeSclGetQpHistogram(inst, port, &out)` → QP_HIST query: `out = 121a+p:52/53` (32-bit; port 0: `121a:52/53`) for ports
  0–5, 0 for the input port; refused for MI ports 6–8 and 10. Nothing in the driver writes a histogram configuration — the value is
  whatever the scaler accumulates [I]. Called by `mi_scl` only from its debug path.
- `MHalVpeSclSendSwMessage(inst, {+0 ?, +4 type, +8 u16 count, +0xc *value, +0x10 *mask})`: only type 0 with count 1
  (`count == (type == 0)`, else "Message Cnt not currect" but returns 1): `1a29:7b = value` under mask (mask 0 → 0xffff).
  `MHalVpeSclReceiveSwMessage(inst, msg)`: count set to 1 for type 0, `*value = 1a29:7b`. A 16-bit mailbox in a VPU bank
  register shared with whatever reads it on the other side [I: the VDEC / VPU firmware or another core].

### 3.16 Debug surface, pattern generator, CSC presets — unused [R]
<!-- ev:mhal-scl-jpe-cmdq.3.16 -->
- **Char device and sysfs**: char device "mscl" (DT `sigmastar,vpe`, `_DrvSclVpeModuleInit`; registration: mhal-core.md §3.2, §4.2);
  sysfs: `check_clk` (fclk1 rate select via `HalSclHwSetFclk1Clock` → `1038:64`), `check_csc` (below), `check_hvsp`
  (§3.14), `check_ptgen`, `check_func`, `check_dbgmg` (message levels: ctx, halif, info, input, irq, isrban, module, prio,
  util, vpeisp, vpescl, reset), `check_proc`, `check_inst`, `check_{vif,m2m,dip,rot,}_ints` (interrupt statistics:
  vsync / done counts, fps, active and blanking time, process diff time histograms).
- **Pattern generator / input timing** (input types 2/6, `HalSclHwSetInputConfig`, `HalSclHwSetPatGenConfig`,
  `scl_set_input_timing` 0x400e9b98), from the pipe size W×H: `1210:37` = Vtotal = 1.3·H (0xffff first when the frame is short), `1210:31` = 1,
  `1210:32` = 5 (V sync), `1210:33/35` = V active start, `1210:34/36` = V active end (centred), `1210:3f` = Htotal = 1.3·W,
  `1210:39` = 4, `1210:3a` = 0x2f (H sync), `1210:3b/3d` = H active start, `1210:3c/3e` = end, `1210:1a` = W and H in
  8/16/32/64-pixel units by range, `1210:30[2]` = (type == 6); enables `1210:18[0,1,15,4]`, `1210:18[2]` = mode bit 1,
  `1210:30[0]` / `[1]` = mode bit 0, `1210:1c[0]`; mode = 2 for types 2/4 | 1 for types > 2. `gbPtgen` set for types 2/6;
  ptgen static / dynamic modes from `DrvSclDebugSetPatgenEx`.
- **`check_csc`** [R: disassembly; no reference source contains this code]:
  - `check_csc_store` (.text 0xe8eac) splits the written text on spaces with `strsep` (`DrvSclOsSplitString` cuts the last
    character of the last token — the `echo` newline); fewer than 2 tokens → prints `"echo [wdma/rdma] [601/709/user]
    [user file]"`; with 3 tokens it opens token 3 as a file (`DrvSclFsOpenFile`), reads all of it into a buffer of size + 1
    and NUL-terminates it; then calls **`DrvSclDebugSetDmaCsc(token[1], token[0], buf)`** — the first two tokens are passed
    swapped (`ldrd r9, r8` → `r0 = r8`). `check_csc_show` (0xe8d40) is `return 0` — reading the file gives nothing.
  - `DrvSclDebugSetDmaCsc(dma, mode, buf)` (0xe5874): `dma` `"wdma"` → 1, `"rdma"` → 2, else 0; `mode` `"601"` → 1, `"709"`
    → 2, `"user"` → 3 if `buf != NULL` else 0, else 0; tail-calls `HalSclCscChange(dma, mode, buf)`.
  - Because of the swap, the documented form `echo wdma 601 > check_csc` gives `dma = "601"` → 0 → **no-op**; only the
    reversed form `echo 601 wdma` (or `echo user rdma <file>`) reaches the presets [I from the two readings].
  - **`HalSclCscChange(dma, mode, buf)`** (0xed288): `dma == 0` or `mode == 0` → return; otherwise 9 × u16 are copied into
    the current matrix of that direction:
    | dma | mode 1 (601) source | mode 2 (709) source | mode ≥ 3 (user) | target (current matrix) |
    |---|---|---|---|---|
    | 1 `wdma` | `.rodata+0x1326E` | `.rodata+0x13280` | `buf` bytes 0–17, u16 little-endian | `.data+0x1D808` (18 B) |
    | 2 (any ≠ 1) `rdma` | `.rodata+0x13292` | `.rodata+0x132A4` | same | `.data+0x1D81A` (18 B) |

    The user file is taken as raw binary — the first 18 bytes (9 × u16 LE), no text parsing, no length check (a shorter
    file reads past its buffer) [R: byte-pair loop; consequence I]. The four presets are anonymous `.rodata` blocks between
    the `__func__` strings `"_HalSclHwRegFlipSetDoneDummy"` and `"HalSclHwSwReset"`, referenced only from
    `HalSclCscChange`. The two targets are unnamed `.data` arrays directly after `MST_SRAM_LPF`; their load-time contents
    equal the mode-1 preset of the same direction.
  - **Preset values** [R values; I decoding], raw u16 in `.rodata` order (row-major, 3 × 3):
    | preset | words |
    |---|---|
    | wdma 601 (`0x1326E`) | `0B37 0800 0000` · `39AA 0800 3D40` · `0000 0800 0E2D` |
    | wdma 709 (`0x13280`) | `0C98 0800 0000` · `3C40 0800 3E7F` · `0000 0800 0ED9` |
    | rdma 601 (`0x13292`) | `0200 1E53 1FAD` · `0132 0259 0075` · `1F53 1EAD 0200` |
    | rdma 709 (`0x132A4`) | `0200 1E2F 1FD1` · `00DA 02DC 004A` · `1F8A 1E76 0200` |

    Decoding [I — the coefficients match the standard full-range conversions, which fixes formats and order]:
    **wdma = YUV → RGB** (the write-DMA output side): 14-bit two's complement, 1.0 = 0x800; rows R, G, B; columns Cr (V),
    Y, Cb (U). 601: R = Y + 1.402 Cr; G = Y − 0.792 Cr − 0.344 Cb; B = Y + 1.772 Cb. 709: R = Y + 1.574 Cr; G = Y − 0.469 Cr
    − 0.188 Cb; B = Y + 1.856 Cb (BT.709 full range exactly). The 601 G←Cr term `0x39AA` = −1622/2048 = −0.792 differs from
    BT.601's −0.714 (`0x3A4A`); every other 601 term matches [R value; I comparison]. **rdma = RGB → YUV** (the read-DMA
    input side): 13-bit two's complement, 1.0 = 0x400; rows Cr (V), Y, Cb (U); columns R, G, B. 601: V = 0.500 R − 0.419 G
    − 0.081 B; Y = 0.299 R + 0.587 G + 0.114 B; U = −0.169 R − 0.331 G + 0.500 B. 709: V = 0.500 R − 0.454 G − 0.046 B;
    Y = 0.213 R + 0.715 G + 0.072 B; U = −0.115 R − 0.385 G + 0.500 B (full range, no offsets in the matrix).
  - **Nothing applies the matrices** [R + I]: the two target arrays are referenced by no other relocation in any section of
    `mhal.ko` (only the two literals of `HalSclCscChange`); the module uses no MOVW/MOVT relocations (all data addresses
    come from ABS32 literals); the only code that indexes the neighbouring `MST_SRAM_LPF` (`HalSclLpfFilterInit`, §3.14)
    stops one byte before the first array [R]. No SCL function writes a CSC register (the only SCL symbol containing "Csc"
    besides `HalSclCscChange` is `DrvSclDebugSetDmaCsc`) [R]. So `check_csc` changes
    RAM copies that no code reads and has no effect on the hardware in this build [I].
  - Whether the SCL DMA paths apply any colour conversion at all, and from where (fixed in hardware or programmed
    elsewhere), is not known: it is not taken from these arrays [I], and no SCL function writes a CSC register.

## 4. JPE — the MJPEG encoder

Bank **1620** (DT `sstar,cedric-jpe`, reg 0x1F2C4000; IRQ GIC SPI 0x3d; clock `CKG_jpe` + up to 5 more DT clocks). A bare
register number in this section is a register of bank 1620 (`regNN` or `NN`). The ios code writes `base16[i] = v`, so
reg = i/2.

### 4.1 Relation to the reference source [R]
I2 jockyw2001/sdk `src/mhal/i2/jpe` is the same driver family and largely the same code: `mhal_jpe.c`
(`MHAL_JPE_*`), `drv_jpe_ctx.c` (`JpeCtx*`, `JpeCtxEncFireAndReturn`, `JpeCtxEncPostProc`), `drv_jpe_dev.c` (`JpeDev*`),
`hal_jpe_ops.c` (`_SetConf` / `_GetConf` / `_EncConf` / `_EncDone`, Q-table scaling `_JpeSetQTable`), `hal_jpe_ios.c`
(`_SetBank`, `_EncFire`, `_IsrFunc`, `_AddOutBuff`), `jpegenc_marker.c` (the JFIF header written by the CPU into the output buffer), and
the register layout `hal_jpe_ios.h` (`JpeReg_t`), which matches what the M6 code touches (§4.8). Differences in the M6
binary:
- **Several jobs in flight**: each fire writes a job number into **reg 7f** (low byte, bit 8 set); `_IsrFunc` matches the
  finished number against the ios "encoding" and "done" job lists; `g_perctx_fire_num` (module parameter, 2) and
  `g_parallel_chn_inq` (1) bound it; each context has a two-slot job queue; `_CheckStatus` reports per instance (§4.13).
- **Interrupt control moved**: the IRQ mask written at the end of a fire is **reg 22** (I2 used reg 1c); regs 23 and 26 are
  new IRQ controls; status is still reg 1e, clear reg 1d (§4.8, §4.9).
- **CMDQ mode**: with a CMDQ the register writes are recorded and replayed into the queue (§4.10).
- **Input modes**: frame, ring, IMI (SRAM) and EMI input (§4.7); I2 had only frame / row.
- **Register-dump DMA** (regs 52/53) from which the frame size is taken (§4.8).
- New: rate control (`jpe_enc_rc_*`, QF per frame from a target size, §4.12), restart interval, header removal, user data
  in a COM segment (§4.11), dumps and `/proc` statistics (§4.3). `MHAL_JPE_IsrProc` is a stub ("not support") — the
  interrupt work happens in `GetEngStatus`.

### 4.2 Objects [R]
Layers: `MHAL_JPE_*` (exports) → `JpeCtx*` (context, 0x3d0 B) → ops (`JpeOpsAcquire`, 0x500 B: `_JpeJob`, `_SetConf`,
`_GetConf`, `_EncBuff`, `_EncDone`, `_OpsFree`, `_EncConf`) and `JpeDev*` (device, 0x100 B, `g_jpe_dev`) → ios
(`JpeIosAcquire`, 0x1b4 B: `_Release`, `_SetBank`, `_EncFire`, `_IsrFunc`, `_AddOutBuff`, `_EncCancel`, `_EncDone`,
`_CheckStatus` at slots 0/1/2/3/5/6/7/8). Same split as I2 (`drv_jpe_ctx.c`, `hal_jpe_ops.c`, `hal_jpe_ios.c`).
- **Device** (`MHAL_JPE_CreateDevice`, one global, refcount `dev+0xc4`): ios, mutexes `+0x2c` (user table) and `+0x54`
  (engine), IRQ number `+0x24`, MMA callbacks `+0x38..0x48`, users table `dev+0x80` = 16 context slots (`JpeDevRegister`),
  OTP flag `+0xf8` (§4.3), clock rate `dev[7]`; `jpe_dev_init` (clocks), proc, monotonic stamps.
- **Context** (`JpeCtxAcquire`, 0x3d0): `+0` release fn, `+4` device, `+8` ops, `+0x10` slot id (0–15), `+0x14/+0x18` last
  MB position seen by `CheckInstanceStatus`, `+0x1c` mutex, `+0x44/48/4c` result of the last `_EncDone` (out address,
  bytes, header bytes), `+0x50..+0x22c` statistics (bytes / frames / time over 1/3/5/10/60 s windows, computed in
  `EncodeFrameDone`), `+0x280` jobs in flight, `+0x284..+0x29c` counters (fire try / done, done, cancel, barrier),
  `+0x2a0/+0x328` input / output dump requests (0x88 B each, from `dump_dbg`), `+0x3b0` list of output windows kept for the
  dump.
- **Ops** (0x500): `+0x20 + i·0x98` the two EncQ slots; `+0x150` input cfg (0x58, type 1, §4.6); `+0x1a8` output cfg (type
  2: virt, phys u64, size; then `+0x1b4..0x1bc` = the same minus the header); `+0x1c0` CMDQ cfg (type 3: u8 enabled,
  handle; `+0x1c8..` = record buffer `+0x40c`, total and multi counts `+0x410/+0x414`); `+0x1d4` internal buffer (type 5:
  virt, phys, size; three 0x100-byte areas at `+0x1e0/+0x1f0/+0x200` {virt, phys, size, busy}); `+0x20c` picture (type 6:
  format, W, H, crop W/H/X/Y); `+0x228` RC (type 7, 0x9c: mode, user Q tables `+0x22c` / `+0x26c`, init QF `+0x2ac`, max QF
  `+0x2ad`, min QF `+0x2ae`, bitrate `+0x2b0`, fps num / den `+0x2b4/+0x2b8`, initial delay ms `+0x2bc`, buffer level
  `+0x2c0`); `+0x2c4/+0x2c8` frame cfg (type 8); `+0x2cc/+0x2d0` restart (type 9: MCU count, IRQ flag); `+0x2d4`
  header-omit (type 10); `+0x2d8/+0x2dc` user-data buffer (type 12, 0x804 B); `+0x2e0` flush callback (type 11); `+0x2e4`
  header dirty; `+0x2e8` frame count; `+0x2ec` frame QF; `+0x2f0` Q scale; `+0x2f4` / `+0x334` scaled Q tables (64 B each);
  `+0x374` RC dirty; `+0x378` last frame bits; `+0x380` RC state (§4.12); `+0x3f0` header buffer (0x400); `+0x3f4` header
  size; `+0x3fc` marker list; `+0x404` current job; `+0x40c` CMDQ record buffer (0x2000 = 1024 entries).
  Defaults (`JpeOpsAcquire`): RC mode 0 (fixed QF), QF 70 (`0x46`), max QF 90, min QF 1, bitrate 20 000 000, fps 0/1, delay
  1000 ms, buffer level 8, base tables `luminanceQTable` / `chrominanceQTable` (rodata, the standard IJG Annex K tables
  [I: same symbol names as I2]).
- **Job** = `ops+0x404` (`_JpeJob`): `+0` pointer to one of the **two EncQ slots**, `+0x14/+0x18` Q tables, `+0x1c` ?,
  `+0x20` record-list pointer, `+0x24…` register shadow in the I2 `JpeReg_t` order with M6 additions (reg 00 at
  `job+0x24`, …, `11` `+0x40`, `12`–`15` `+0x42..0x48`, `18` `+0x4c`, `19/1a` `+0x4e/0x50`, `1d` `+0x54`, `22/23`
  `+0x5e/0x60`, `26` `+0x66`, `2c` `+0x6c`, `37/38` `+0x7c/0x7e`, `3b` `+0x84`, `52/53` `+0x9e/0xa0`, `5e/5f` `+0xb6/0xb8`,
  `61/62/63` `+0xbc/0xbe/0xc0`, `6a` `+0xce`, `7c/7d` `+0xf2/0xf4`, `7f` `+0xf8`).
- **EncQ slot** (0x98): `+8` busy, `+9` CMDQ flag, `+0xa` drop flag, `+0xc` ctx id, `+0x10` input cfg copy, `+0x68` output
  cfg copy, `+0x80` dump area, `+0x84` area 2, `+0x88` bytes accumulated, `+0x8c` header size, `+0x90` job number, `+0x94`
  flush callback. `_EncConf` fails with "EncQueue not enough" when both slots are busy — the limit behind
  `g_perctx_fire_num` = 2.
- **ios**: `+0xc0` job number counter (1–255), `+0xc5` bit-mux select of the status register (§4.13), "encoding" and "done"
  job lists.

### 4.3 Platform driver, clocks, OTP cap, sysfs, proc [R]
- **Probe** `jpe_probe(pdev)` (reached through `codec_probe`, mhal-core.md §3.2; DT `sstar,cedric-jpe`, must be `available`):
  0x50-byte driver struct; `MHAL_JPE_CreateDevice(0, …)`; char device region "jpe" (16 minors) with `jpe_ops` whose
  `open` / `release` / `ioctl` only print "not support please enable MACRO MHAL_VENC_UT_ENABLE" and return −14 (**unused**:
  the debug char device exists but does nothing); class device "jpe" under `msys_get_sysfs_class()`; bank = platform
  resource 0 (`res.start − 0x22000000` = RIU VA) → ios `_SetBank`; IRQ = DT irq 0 → `dev+0x24`; sysfs (`jpe_sys_init`).
  `jpe_remove` undoes it. `JpeDevGetResourceMem` (DT reg → RIU VA and size) exists but the probe does not call it.
- **OTP throughput cap**: the probe derives the JPE / VENC throughput flag from the OTP2 (eFuse) words `101f.26`,
  `101f.2c`, `101f.2e`, `101f.2f` (derivation: ipl.md §7.3) and stores it at `dev+0xf8`. With the flag = 1, `venc_check_res_rate` (0x400b3ca8) refuses (error 8) any
  SetParam resolution / crop / stream-on and EncodeOneFrame with `cropW · cropH · fps > 0x9f60000` (167 116 800 pixel/s ≈
  3840×2160 at 20 fps). The VIF has its own input cap from bit 2 of `101f.26` (`HalVif_OTP_VENC_Fps_Capability`,
  mhal-vif.md §7.2). The flag's value on this chip is not known (the OTP words have not been read).
- **Clocks**: `JpeDevClkInit`: DT clock "CKG_jpe" + up to 5 more DT clocks (indices 1–5). `JpeDevClkRate(req)` walks the
  parents of clock 0 (≤ 6), skipping those `vcodec_clock_special_source_disable` rejects, and picks the lowest parent rate
  ≥ `req` (else the highest); `jpe_clock` (global) = chosen rate. `JpeDevClkOn(on)`: enables clocks 1–5, sets clock 0 to
  `dev[7]` and enables it; off: every clock back to parent 0, disable, unprepare. `JpeDevPowerOn` = clock on at the first
  fire, off in `UnlockAfterEncDone` when the ios reports no job (`_CheckStatus` byte 0 = 0). `JpeDevClkDeinit`,
  `jpe_dev_deinit`: clocks off and released.
- **sysfs** on the class device: `jpe_clock` (show: parent rates, " <--" at the active one; store: decimal Hz → same parent
  pick → `CamClkSetRate`), `jpe_rc` (show: per context
  `[ctx]_[bitrate]_[initial_delay]_[init_qfactor]_[min_qfactor]_[max_qfactor]_[buf_level]` for CBR contexts; store:
  `"%d_%u_%d_%d_%d_%d_%d"` into the context's RC fields, QF fields only if 1–99, sets the RC-dirty flag `ops+0x374`); a link
  "jpe" from the platform device.
- **`/proc/jpe/`** (`JpeProcInit` per device, the first one creates the directory): `dev_info` (per device: bank,
  `mem_cnt`, then per context W, H, frame count, RC mode CQP / CBR, fps, QF or bitrate / QF range, CMDQ pointer "CmdQ: %p"),
  `ctx_info` (write `"%li %li"` = dev id, ctx id; read: one or all contexts — input mode, resolution and 420/422, fps, frame
  count, fired / done counts, QF, last output bits, Fire / Barrier / Done / Cancel / ISR counters, real kbps and fps over
  1/3/5/10/60 s, timing of fire / ISR / done), `dump_dbg` (write `jpein|jpeout <dev> <chn> <count> <path>`: dump the next
  `count` input YUV frames to `<path>/dev%02d_chn%02d_%d.yuv` or output JPEGs to `<path>/es_dev%02d_chn%02d_%02d.jpg` via
  `VcodecDumpBuffer`, done in `EncodeFrameDone`; read: usage text). All debug, **unused** on the stream path.
- **Dev config** (`SetDevConfig`): `0x80030013` (CLK) prints "not support", returns 0; `0x80030014` = 5 MMA callbacks (all
  must be non-NULL; stored `dev+0x38..0x48`, never called by the JPE code — no reader); others → 3. `GetDevConfig`: §4.4.

### 4.4 Exports `mi_venc` calls [R]
All go through mi_venc's `MHAL_VENC_*` dispatch (mi_venc.md §1.5); return 0 = OK, MHAL error codes otherwise.
| export | args | what it does |
|---|---|---|
| `CreateDevice(id, &dev)` | | one global device (`g_jpe_dev`, refcounted): `JpeIosAcquire("jpe")`, mutexes, `jpe_dev_init` (clocks: `CKG_jpe` rate from `JpeDevClkRate`, `vcodec_clock_special_source_disable`), `/proc` |
| `DestroyDevice(dev)` | | last ref: free ios, clocks, proc |
| `GetDevConfig(dev, idx, p)` | versioned structs `{u32 size, u32 ver, …}` | `0x80030001` IRQ number (from DT), `0x80030002` CMDQ buffer length 0x1800, `0x80030003` capability table (100 B), `0x8003000e` resolution limits {align 16×16, max W 8192, max pixels 0x4000000}, `0x80030013` "not support", 0, `0x8003001a` {1, 1, 0xa40, 0x1000, fire_num, parallel}, `0x80030024` bool 1 |
| `SetDevConfig(dev, idx, p)` | | §4.3; not imported by mi_venc, used inside mhal |
| `CreateInstance(dev, &inst)` / `DestroyInstance(inst)` | | `JpeCtxAcquire`: context + ops (`JpeOpsAcquire`, marker init), registered in the device's 16-slot user table; destroy = `_JpeCtxRelease` (refuses while jobs are in flight; ops free, unregister, free) |
| `SetParam(inst, idx, p)` / `GetParam` | | §4.5 |
| `QueryBufSize(inst, p)` | | `MHAL_JPE_QueryBufSize` → `JpeCtxGetCfg(4)` = conf type 4 (§4.6): `p+0x18` = the internal-buffer request (three 0x100-byte areas, total rounded to 4 KiB = 0x1000); all other sizes 0 |
| `EncodeOneFrame(inst, io)` | `io` = mi_venc's 0xd0-byte input descriptor (mi_venc.md §7.4) | §4.7; then `JpeCtxEncFireAndReturn` → device lock, power on (clock), ops `_EncBuff` (JFIF header written by the CPU into the output buffer, `CamOsMemFlush`, Q tables scaled from QF, RC picture init) → `_EncConf` → ios **`_EncFire`** (§4.9); with a CMDQ: replay into the queue (§4.10) |
| `BarrierOneFrame(inst)` | | CMDQ only (§4.10) |
| `GetEngStatus(dev, st)` | | must not run in interrupt context; ios `_IsrFunc` and the state → status mapping (§4.13) |
| `EncodeFrameDone(inst, res)` | | `JpeCtxEncPostProc` → ops `_EncDone` (§4.13), device unlock / power off; `res+0x18` bytes produced, `res+0x20` header bytes; timing statistics, optional dumps |
| `EncodeAddOutBuff(dev, b)` | | ios `_AddOutBuff` (§4.13): new output window for the running job |
| `EncodeCancel(inst)` | | ios `_EncCancel` (§4.13) |
| `CheckInstanceStatus(inst)` | | ios `_CheckStatus` (§4.13) |
| `IsrProc(dev)` | | prints "not support", returns 0 |

### 4.5 `MHAL_JPE_SetParam` / `MHAL_JPE_GetParam` — every id [R]
All payloads start `{u32 size, u32 version}`; a mismatch prints and returns 3. RC, restart, header-remove and user-data
sets while a job is in flight (`ctx+0x280 ≠ 0`) only warn and return 0. `SetParam` dispatches the 0x8003xxxx ids through a
table indexed by id − 0x80030004.
| id | payload (size, ver) | Set | Get |
|---|---|---|---|
| 0x80020001 resolution | 0x14, 1: `+8` W (multiple of 8), `+0xc` H (even), `+0x10` format 0 NV12 / 1 NV21 / 2 YUYV / 3 YVYU | conf type 6: if W/H changed or no crop yet, crop = full picture; crop W 16–8192, H ≥ 16, W·H ≤ 64 Mi; format → internal code 3 / 4 / 0 / 1 (`ctx+0x20c`), else error 3 (stored anyway); restart interval cleared if it no longer fits; OTP throughput check (§4.3) → 8 | W, H, format back-mapped |
| 0x80020002 crop | 0x1c, 1: `+0xc` x, `+0x10` y, `+0x14` w, `+0x18` h | x % 256 == 0, y % 16 == 0, w % 8 == 0, h even, inside the picture, w 16–8192, h ≥ 16, w·h ≤ 64 Mi; restart re-check; OTP check | x, y, w, h |
| 0x80020003 RC | 0xb0, 2: `+8` type 0xb (fixed QF) / 0xc (CBR); `+0xc` QF (0xb) or bitrate (0xc); `+0x10` 64 B luma table, `+0x50` 64 B chroma table; `+0x90/+0x94` fps num / den; `+0x98` u8 max QF, `+0x9c` u8 min QF (CBR) | conf type 7: mode 0/1; QF > 90 warns ("set user qtable with qf=0"); CBR max / min QF outside 1–90 → 90 / 1; CBR clears the init QF; marks header and RC dirty | same layout |
| 0x80020004 restart interval | 0xc, 2: `+8` u16 MCU rows, `+0xa` u8 IRQ flag | conf type 9: interval = rows × MB width; ≥ MBs per picture → 0 (off, warning) | rows = interval / MB width, flag |
| 0x80020005 header remove | 0xc, 1: `+8` u8 | conf type 10 (§4.11) | flag |
| 0x80020006 user data | 0x10, 1: `+8` ptr, `+0xc` len 1–1024 | conf type 12 (§4.11) | — (unsupported → 8 path) |
| 0x80030004 stream on | 0xd8, 1: `+8` internal buffer virt, `+0x10` phys, `+0x18` size | OTP check; conf type 5 (three 0x100 areas, phys 16- and 256-aligned, size ≥ 0x300) or "stream on without AlBuf" when phys = 0; `JpeCtxStreamOn` (stub) | — |
| 0x80030005 stream off | — | conf type 3 cleared (CMDQ off), 4 ms sleep, `JpeCtxStreamOn(0)` | — |
| 0x80030006 partial notify | — | prints the log string, returns 0 | "not support", 0 |
| 0x8003000c frame cfg (I2 enum name `E_MHAL_VENC_FRAME_CFG`) | 0x14, 1: `+8` s8 QF step, `+0xc` size threshold (16-aligned) | conf type 8, only when idle; the QF step is subtracted from the next frame's QF once (§4.12); the threshold is stored and never read by the JPE code | both |
| 0x8003000d flush callback | 0xc, 1: `+8` fn | conf type 11: called instead of `CamOsMemFlush` for header, Q and dump buffers | — |
| other 0x80030001–0x80030025 | — | accepted, no effect, 0 | same |
| anything else | — | "command 0x%x is not support", 8 | 8 |

`mi_venc` on this camera's path: Set 0x80020001 / 0x80020003 / 0x80020004 (resolution and RC at channel creation),
0x80030004 / 0x80030005, Get 0x80020003 (mi_venc.md §6.6, §6.7); 0x8003000c from its oversize / re-encode code (mi_venc's
oversize path).

### 4.6 Conf types (`_SetConf` / `_GetConf`) and the input address arms [R]
Types: 1 input, 2 output, 3 CMDQ, 4 (get only) internal-buffer request (areas 0x100 each, total rounded to 4 KiB), 5
internal buffer, 6 picture, 7 RC, 8 frame cfg, 9 restart, 10 header omit, 11 flush callback, 12 user data (storage: §4.2).
Type 1 payload (from `EncodeOneFrame`): mode bits (`+0x150`: 1 frame, 2 ring, 4 ring-2, 8 IMI, 0x10 EMI), MIU flag (input
above 2 GiB, `+0x154`), pitch (`+0x158`, raised to the crop width), Y phys `+0x160`, Y size `+0x164`, C phys `+0x16c`, C
size `+0x170`, frame index (io `+0x1e` clipped 0–15, `+0x174`), ring start line `+0x178`, ring height `+0x17a`, ring start
addresses Y `+0x19c` / C `+0x1a0` (§4.8 regs 12–15), IMI flag `+0x1a4`. Type 1 then computes the hardware addresses; its
switch uses two tables (packed YUV422 / NV12-21) indexed by mode bits − 1: 0 (mode 1) → frame arm, 1 and 3 (modes 2, 4) →
ring arm, 7 (mode 8) → IMI arm, 2/4/5/6 → return 0 (invalid combinations):
| mode | packed 422 (format 0/1) | NV12 / NV21 (3/4) |
|---|---|---|
| 1 frame | Y = base + (cropY·pitch + cropX)·2 (pitch in pixels); size = H·pitch·2 | Y = base + cropX + cropY·pitch; C = Cbase + cropX + cropY·pitch/2; sizes accordingly |
| 2, 4 ring | Y = ring base; start row = startline/8; last row = height/8 − 1; start address Y = base + start row·pitch·8 (C likewise) | rows of 16 lines: start row = startline/16, last row = height/16 − 1, Y start = base + start row·pitch·16, C start = Cbase + start row·pitch·8 |
| 8 IMI | IMI flag set: Y/C = base (no crop); flag clear: all addresses 0, C = 0x14800 (420) [R; IMI-internal offset, I] | same |
| 0x10 EMI | Y = base (420: also C = Cbase) | same |
| 3, 5, 6, 7 | invalid, nothing computed | |

Because the input addresses go to the hardware in 256-byte units (§4.8), a crop X must be a multiple of 256 bytes / 2
bytes per pixel — the reason SetParam 0x80020002 demands x % 256 == 0.

### 4.7 `EncodeOneFrame`: the input descriptor and the input modes [R]
`io` (`InOutBuf`) is `mi_venc`'s mhal input descriptor (0xd0 bytes, version 6; full layout and writers: mi_venc.md
§7.4). JPE reads: `+0x10` CMDQ handle (the mi_sys CMDQ interface, or 0), `+0x18` input mode, `+0x1c` ring height and `+0x24` ring start line
(u16, 16-aligned, else a warning), `+0x1e` frame index (clipped 0–15), `+0x28` Y phys (u64; non-zero in frame mode), `+0x38`
Y size, `+0x3c` pitch (16-aligned, else a warning), `+0x40` C phys, `+0x50` C size, `+0x58` third-plane phys (MIU-range check
only), `+0x68` output phys (u64), `+0x70` output virt, `+0x78` output size, `+0x94` flush callback (`mi_sys_VFlushInvCache`),
`+0xc8` IMI mode flag (`eImiMode`).
- **Input mode** `io+0x18` → internal bits: 0 → 1 (frame, needs a Y address), 1 → 2 (ring), 2 → 4, 3 → 8 (IMI), 4 → 0x10
  (EMI). Modes 1–4 need a CMDQ ("IMI/EMI mode need with CMDQ!") and no crop (crop must equal the picture). Then a
  supported-mask test `(bits & 0x1d) == 0` → "input mode %x not support", error 3: **mode 1 (ring, bits 2) is always
  rejected in this build**; mode 4 (EMI) passes here but `_EncConf` refuses it ("not support EMI mod, plz use ring mod").
- Mode 3 (IMI) additionally needs `io+0xc8 == 1` and `_CheckDynamicSramConfig(cropW, fmt, Ysram, Csram)`: 420 needs
  `align8(W)·32 ≤ Y size` and `align8(W)·16 ≤ C size`, 422 needs `align8(W)·32 ≤ Y size`; else 0xc.
- Other checks: output phys and size 16-aligned (else the 0x11 path); Y / C / V / output either all below or all above
  2 GiB and consistent with the internal buffer's MIU (else "mismatch miu0/miu1 Ref"); the OTP throughput check (§4.3).
- Then `JpeCtxSetCfg` with conf types 1 (input), 2 (output), 3 (`io+0x10` CMDQ → `{1, cmdq}`), 0xb = 11 (`io+0x94`
  callback, if set), `JpeCtxEncFireAndReturn`, replay (§4.10), timing, dump bookkeeping.

### 4.8 Register map (bank 1620) [R = written as listed; meaning from the I2 `hal_jpe_ios.h` names where marked "I2", else I]
Values come from `_EncConf` (the job's register shadow, §4.2) unless stated. Write order: §4.9.
| reg | written value / use | meaning |
|---|---|---|
| 00 | I2 control fields; bit 0 frame-start strobe; bit 1 soft reset (I2 `nRegJpeGSoftRstz`); encode mode and format; bit 3 = 1 for packed 422 (formats 0/1), 0 for 420; **bits 4–7 = frame index** (modes 2/4/8: input-cfg `+0x174`); Barrier and IMI cancel write `[7:4] = 0xF` | I2 control; I2 `nRegJpeGFrameIdx` is 2 bits, M6 uses 4 — ring / IMI slot index of the SCL handshake (§3.8); 0xF = idle / stop [I] |
| 01 / 02 | crop W aligned 16 / crop H aligned 8 (422) or 16 (420) | picture W / H (I2; I2 rounds the height to 8) |
| 03 | bit 0 = 1 (new); bit 6 TBC mode (0 during the Q-table load, then 1) | I2 TBC mode; bit 0 unknown |
| 06/07, 0a/0b | Y / C address in **256-byte units** (`[15:0] = addr[23:8]`, `07/0b[7:0] = addr[31:24]`; same in I2 `_HalJpeSetInBuf`) | I2 input Y / C address |
| 08/09, 0c/0d | 0 | I2 second buffer, unused |
| **11** | bits 0–1: 1 for ring modes 2/4, else 0 (an earlier value 2 is overwritten) | input row / ring mode select [I] |
| **12/13** | Y address of the start row inside the ring (input-cfg `+0x19c`), same 256-B split as 06/07 (which then hold the ring base); 0 outside ring modes | ring start position Y [I] |
| **14/15** | the same for C (`+0x1a0`) | ring start position C [I] |
| 16 | `|= 0x37f` | I2: clock gating (`nRegJpeGClkGateEn` bits 0–4 = 0x1f, SRAM gate bit 5) + bits 6, 8, 9 (new) |
| 18 | `(x & 0x7803) | 0x80fc`: bit 2 = 1 (frame-buffer mode), bits 3–8 = 0x1f (auto-reset wait count), bits 9–10 = 0 (FSVS mode), bit 15 = 1 (VIU soft-reset release); in IMI mode (8) bit 15 is pulsed 0 → 1 around the soft reset | I2 names (`JpeBuffermode` …) |
| **19** | low 11 bits = last ring row − start row; 1 outside ring modes | ring rows to go [I] |
| **1a** | low 11 bits = last ring row (`rows − 1`); 1 outside ring modes | ring height in rows [I] |
| 1d | 0x7ff at fire start; IRQ clear (low 11 bits) | I2 IRQ clear |
| 1e | interrupt status (§4.13) | I2 IRQ status |
| 20 | Barrier: 1 | I2 `nRegJpeGIrqForce` bit 0 = force frame-done |
| 21 | bits 0–2 = 0 | I2 SRAM shut-down enable |
| **22** | at the end of the fire: low 11 bits = 0x7fe (`jpe_set_irq_mask(0xf00007ae)` (0x400d3ae0): all events but frame-done, with the TXIP/VS and NET/ROW muxed bits); never changed later | CPU-side IRQ mask — read by analogy with I2 reg 1c (1 = masked) it leaves only frame-done able to interrupt [I] |
| **23** | `|= 0x7ff` at fire start; Barrier writes 0x7ff | IRQ control, IP / trigger side [I] |
| **26** | low 11 bits = 0 at fire start and in Barrier; CMDQ-mode cancel writes 1 | IRQ control, IP / trigger side [I] |
| 2c | last-quant index from the Q-table load (bit 7 kept) | I2 |
| 35, 36 | read: `36` low 9 bits and `35` = the engine's current MB position (§4.13) | I2 `nRegJpeSEcdbMbx/Mby` |
| 37/38 | restart: `37[0]` enable, MCU count `(n & 0x7fff) << 1` in 37 and `n[26:16]` in 38, `38[11]` = restart IRQ enable (payload flag) | I2 (`RstEn`, `RstMcuCnt`, `RstIrqEn`) |
| 3b–3e | output window (`jpe_set_out_start` (0x400d3c5c) / `jpe_set_out_end` (0x400d3b90)): `3b = (x & 0xf840) | 0x400` (output buffer mode, bit 10 set), start then end through `3c/3d` (address in 4-byte units, `3d[15:14]` = 0 start / 1 end), `3e[0]` strobe; the set-address strobe reads as `3b[0]` in one of our readings and as `3b` bit 10 in another (not settled) | I2 output buffer |
| 42, 44/45 | byte index (`42 & 3`) and write pointer of the output (in 4-byte units) | I2; M6 takes the same values from the register-dump area (52/53) |
| 48–4b | Q-table port (TBC): write = data `4a`, address `49[15:8]`, `49[0]` write, `48[12]` enable; poll `49[2]` done, clear `49[1]`; read back `4b` | I2 `_HalJpeQTableEntryRead/Write`; under CMDQ the done-poll becomes a CMDQ poll (`{0x49, …, mask 3}`) and there is no read-back |
| **52/53** | register-dump buffer: `52 = addr >> 4` (low 16), `53 = 0x8000 | addr[30:20]` (bit 15 = enable); address = the job's 0x100-byte dump area | at frame end the engine writes its result registers to DRAM; `_EncDone` reads the size from there (`dump[3]·4 − dump[2]·4 + (dump byte 7 & 3) + 1`, i.e. write pointer − start + byte index, the I2 reg 44/45/42 formula) |
| 5e | Y and C pitch, 8 bits each = `pitch >> 4` with the pitch counted in pixels (packed formats: bytes / 2, raised to W) | I2 pitch |
| 5f | `(x & 0xe030) | 0x104`; bit 14 IMI enable (mode 8); bit 15 C swap (NV21, YVYU) | I2 YUV loader (`RlastThd` 4, priority thd 4) |
| 61 | `(x & 0x3800) | 0x8001`: IMI bound 1, handshake select 2 (EVD); modes 2/4/8 clear bits 14–15 (handshake with the scaler) | I2 loader row / tile / handshake |
| 62 | pitch MSBs | I2 |
| **63** | `[15:12]` = aligned H − crop H, `[11:8]` = aligned W − crop W | padding to the 16-pixel grid [I] |
| **6a** | `(x & 0xc0c0) | 0x3f00`; bit 5 = 1 in ring modes | unknown fields; 0x3f in bits 8–13 |
| **7c** | bits 5–6 = MIU flag of the input (`+0x154 & 3`) | input MIU select [I] |
| **7d** | `job+0xf4` (not set by `_EncConf`: 0) | unknown |
| 7f | `[7:0]` job number (1–255, 0 skipped), bit 8 = "sf_coding_flag"; `[15:8]` ≠ 0 while a job runs | job id (M6) |

Bold registers have no I2 name. Not known: which of 22 / 23 / 26 is the CPU mask, the IP mask and the IP force (a read of
`1620:22/23/26` during an MJPEG stream and after a frame would show it); the meaning of reg 03 bit 0, reg 6a and reg 7d; no
M6 register reference is available. `jpe_dump_bank` (0x400d4850) dumps the bank (128 registers) for debug.

### 4.9 `_EncFire` register sequence [R: order; condensed listing]
<!-- ev:mhal-scl-jpe-cmdq.4.9 -->
All writes go straight to bank 1620, or into the CMDQ record list (§4.10). `_EncFire` first appends the EncQ slot to the
ios "encoding" list and assigns the job number (`ios+0xc0`, 1–255).
1. `reg26` low 11 bits = 0, `reg1d = 0x7ff` (clear all IRQs), `reg23 |= 0x7ff`.
2. `reg00 |= 1` (frame-start strobe, I2 order), `reg18` (mode 8 only: bit 15 pulse), soft reset: `reg00 &= ~2` written
   several times, then `reg00` restored (`reg00` is written 7 times in this part).
3. Q tables: `reg03` TBC mode = 0, 64 luma + 64 chroma entries through the TBC port (each written and, in direct mode, read
   back until it matches), `reg2c` last quant index, `reg03` TBC mode = 1.
4. `reg52`, `reg53` (register-dump buffer), `reg3b` (output buffer mode), output buffer start / end (`reg3c/3d`), `reg1d`
   clear again.
5. Picture: `reg01` width, `reg02` height, `reg5e` Y/C pitch, `reg62/63`, `reg7c/7d`, `reg00` (encode mode, format, frame
   index), `reg5f` (YUV loader: C swap for NV21 / YVYU), `reg18` buffer mode, `reg06/07` Y address, `reg0a/0b` C address,
   `reg37/38` restart interval, `reg61` (loader row / tile / handshake — set for the ring / IMI modes), `reg12–15`,
   `reg19/1a`, `reg11`, `reg5f`, `reg6a`. `reg16` and `reg21` (§4.8) are written too; their place in the order is in the
   condensed listing.
6. **`reg7f` = job number | 0x100**, **`reg22` = 0x7fe**, **`reg00` frame-start bit** → the hardware runs.

### 4.10 CMDQ record and replay, Barrier [R]
- **Record**: with a CMDQ (`job.recordlist+4 ≠ 0`) every ios write is appended to the context's record buffer (`ops+0x40c`,
  8-byte entries) instead of hitting the bank: `{u32 riu_addr, u32 value | mask<<16}` with `riu_addr = (VA & 0xffffff) >> 1`
  (= `bank<<8 | byteoff`).
- **Replay** (`MHAL_JPE_EncodeOneFrame` after the fire): before it up to 100 × `CheckBufAvailable(total)` (slot 0x0c);
  failure → error 0xd. Entries before the first "multi" one (the fire-start part: 26, 1d, 23, 00 × 7, 18, 03, 64 + 64 TBC
  writes with their poll entries, 2c, 03) are replayed one by one: mask = 0 → slot 0x24 `WriteRegCmdq`; mask ≠ 0 → slot
  0x2c `PollRegBits(addr, value, mask, 1)` (only the TBC done-polls). The rest (counted in `+0x10`, the picture part from 52
  on) goes as one slot-0x28 `WriteRegCmdqMulti` call. In CMDQ mode the Q-table entries are not read back.
- **`MHAL_JPE_BarrierOneFrame`** (CMDQ only, else returns 0): `CheckBufAvailable(15)` (≤ 100 tries, else 0xd),
  `AddWaitEventCmd(21 = JPE_TRIG)`, `WriteRegCmdqMask(1620:7f, 0, 0x100)`, `WriteRegCmdq(1620:20, 1)`,
  `WriteRegCmdqMask(1620:00, 0xf0, 0xf0)` (`reg00[7:4] = 0xF`), `WriteRegCmdq(1620:26, 0)`, `WriteRegCmdq(1620:23, 0x7ff)`;
  counter `ctx+0x29c`.
- **Whether `mi_venc` hands JPE a CMDQ** [R code, I conclusion]: `_MI_VENC_RejectCmdq(…, devres)` (disassembly `mi_venc.ko`
  `b180`): device index ≤ 1 (H.26x) → accept; index > 1 (JPE devices, for which the proc printer uses MHAL type 2) → reject
  only if `devres+0x1b0 == 0`. `venc_set_dev_ops(index, type)` (0x1b27c) stores `astMHalDevConfig[type].ops` (non-NULL for all three
  types) at `devres+0x1a8 + type·4`, so for a JPE device `+0x1b0` is the JPE ops pointer → **accepted**; mi_venc then asks
  `GetDevConfig(0x80030002)` (0x1800) × `devres+0x598` for the CMDQ buffer, and `EnqueueInputTask` passes the mi_sys CMDQ
  as `io+0x10` (mi_venc.md §7.3). So MJPEG frames on this camera most likely run in CMDQ mode (record + replay + Barrier)
  [I]; it has not been observed. While MJPEG streams it is visible in `/proc/jpe/dev_info` ("CmdQ: %p") and in the
  `CmdqHandle` column of mi_venc's proc device table (`/proc/mi_modules/mi_venc/…`).

### 4.11 JPEG header, header removal, user data [R]
`drv_jpegenc_encoder_HwJpegMarkerInit` rebuilds the header when `ops+0x2e4` (dirty) is set, else reuses it:
`SOI` (FF D8) · APP0 JFIF (18 B template in `.rodata`) · user data if any · DQT luma (table 0) · DQT chroma (table 1)
(64 bytes each, zig-zag order from a 64-byte index table, 0 → 1) · SOF0 (19 B: H, W, components with sampling 0x21 for 422
or 0x22 for 420) · DHT × 4 (33/183/33/183 B templates = the standard Annex K DC/AC tables [I: by size]) · DRI (6 B) when a
restart interval is set · SOS (14 B) · **APP15 (FF EF) filler** so that output start + header ends 16-byte aligned: with
`u = (out + header) & 15` the filler payload is `12 − u` bytes, or `28 − u` when `16 − u` ≤ 4 (fixed QF) / ≤ 7 (CBR), so it
is ≥ 1 byte (fixed QF) / ≥ 4 bytes (CBR); in CBR mode the first 4 filler bytes hold the frame's QF value. The marker list is
built in the 0x400-byte header buffer (`jpe_build_header` (0x400d59f8): entries ≤ 0x400 total, "Jpeg header len out of range"); it must fit
the output buffer ("no enough outbuf for jpeg header"), is copied to the output start and flushed; output virt / phys / size
advance past it (`_EncBuff`). Marker-list helpers: `drv_jpegenc_marker_{init,release,create_app_n,size,dump}`,
`_memcpy_nonalign` (the I2 `jpegenc_marker.c` family).
- **User data** (conf type 12): one 0x804-byte buffer holding a **COM** segment `FF FE <len16> <data>`, each 0xFF data
  byte followed by 0x00; the segment is in the header of **one** frame only (ops `_EncDone` clears the length and marks the
  header dirty).
- **Header removal** (conf type 10, flag set): when the header is not dirty, no header is written at all (header size 0).

### 4.12 Q tables and rate control [R]
- `_EncBuff` per frame: fixed-QF mode → frame QF = `ops+0x2ac`; CBR → `jpe_enc_rc_pic_init` gives qp, frame QF = 100 − qp
  (header dirty if it changed). A pending frame-cfg QF step (0x8003000c) is subtracted once. QF clipped 1–90; scale =
  5000/QF (QF < 50) or 200 − 2·QF; each table entry = `(base·scale + 50) / 100` clipped 1–255 (IJG); QF 0 = user tables as
  given.
- **CBR model** (`ops+0x380`, own float type: 16-bit mantissa + 16-bit exponent, `FINIT/FADD/FSUB/FMUL/FDIV/FLOG2/FPOW2/
  FINT/FMIN/FMAX/FMINMAX/FGE/FGT/FABS/FSQRT`, normaliser `jpe_fp_normalise` (0x400d75a8)): `qp = a·log2(bpp) + b`, initially
  `a = −820/256 ≈ −3.20`, `b = 12225/256 ≈ 47.75` (`jpe_enc_rc_init`).
  - `jpe_enc_rc_seq_init`: pixels = W·H, bitrate ≤ 800 Mbit/s, bits per picture = bitrate·fps_den/fps_num, min / max qp =
    100 − max / min QF, buffer = delay_ms·bitrate/1000, initial level = buffer/100·(6·level + 2), shift 0/1/2/3 for level
    < 6 / 12 / 14 / ≥ 14.
  - `jpe_enc_rc_pic_init`: target bits from the buffer (`jpe_rc_buffer_weight` (0x400d6f20): weight = buffer-fullness ratio clamped to
    [0.10, 12], limited by headroom), qp from the model, clipped to the qp range and to ±6 of the previous qp; the very first
    frame uses `100 − ((avg_bpp − 0x254d·2⁻¹⁶)/(0x2ff·2⁻¹⁶))` when the init QF is outside 1–99.
  - `jpe_enc_rc_pic_end(real bits, qp)`: model update `a += 0.25/(1 + log2²(bpp))·(qp − model)·log2(bpp)` clamped to
    [−4, −2], `b = 0.75·b + 0.25·(qp − a·log2(bpp))` (constants 0x4000·2⁻¹⁶, 0x6000·2⁻¹⁵), buffer level update
    (`update_rc_buf`, global `g_bit_offset`).
  - Helpers `iMin/iMax/iAbs/iClip3/uiMin/uiMax/uClip3/i64Clip3/u64Clip3/fSqrt/fLog/fExp/fPow64` and `fw_util_*` (abs, min,
    max, clip, round / ceil divide, sqrt, msb, ceil-log2, align, square; a small ring queue `fw_util_queue_*`) are generic
    and shared with the RC.
- On this camera's path the MJPEG channel runs fixed QF (Qfactor 80 → mi_venc sets RC type 0xb); CBR is **unused**.

### 4.13 Job queue, interrupt, status [R]
- **Fire**: `_EncConf` takes a free EncQ slot, copies input / output cfg, sets the dump area and the job registers;
  `JpeCtxEncFireAndReturn` → `_EncBuff` (header, Q) → `_EncConf` → `JpeDevLockAndFire` (engine mutex, clock on, ios
  `_EncFire`). With `g_parallel_chn_inq` = 1 the engine mutex is released right after the fire when the context has a CMDQ
  and the dump area is valid (`venc_dump_area_valid` (0x400b39f8)), so another context can queue its job; otherwise it is held until
  `UnlockAfterEncDone`.
- **`_IsrFunc`** (called from `GetEngStatus`, which refuses interrupt context with error 9; `JpeDevGetEngStatus` = ios ISR
  call, then copies state, current output window (address, end), produced size and ctx id from the ios): status = `reg1e`
  with the I2 bit-mux (bit 4 → TXIP timeout or VS, bit 6 → net trigger or VIU row done, chosen by `ios+0xc5`); checked in
  this order, each bit cleared in `reg1d`:
  - `0x100` IMI overflow → state 5; every job with the current or previous number marked drop;
  - `0x1` frame done → clear; `reg7f` coding flag off (direct mode) / finished number = 7f − 1 while the flag is set (CMDQ
    mode); find the job in the encoding (then done) list, take its addresses and size → state 2 ("miss" → no owner);
  - `0x2` / `0x8` out-buffer 0 / 1 full → state 4 with the current window;
  - `0x200` restart-interval chunk ("rst isr") → state 6 with addresses;
  - anything else → state 10 ("ErrIrq"); nothing pending → state 2 "isr out for empty Q" / "isr out for cmdq".
- **`MHAL_JPE_GetEngStatus`** maps the state to the `MHAL_VENC_EncStatus` value mi_venc gets: 2 → **0** (frame done,
  ISR-to-done time recorded), 4 → **1** (+ output addresses: the buffer needs `EncodeAddOutBuff`), 1 → **2**, 6 → **3**
  (+ addresses: a restart chunk is ready), 5 → **4** (error / overflow), anything else (10 included) → **5**; `st[7]` =
  owning context. mi_venc treats 1 and 3 alike (both carry addresses) [R: the `(st & ~2) != 1` test in
  `_MI_VENC_HandleIsr`]. The enum's M6 names are not known: there is no M6 header, and I2 has only 0 FRM_DONE,
  1 BUF_RDY_UNDER_RUN, 2 BUF_RDY, 3 OTHERS.
- **ios `_EncDone(ios, ctx)`**: moves finished jobs (number ≤ done number, 8-bit window of 32) from encoding to done, takes
  this context's job: size from the dump area (§4.8 regs 52/53) or the ISR value, flush via the callback; returns 7 if the
  job was marked drop, 10 if none found. `JpeCtxEncPostProc` → ops `_EncDone` (RC picture end, frame count, clears one-shot
  user data) → `EncodeFrameDone` result: `res+0x18` bytes (header included), `res+0x20` header bytes; drop →
  `res+8 = 0x800000` flag, size 0. `UnlockAfterEncDone`: engine unlock, clock off when no job remains (§4.3).
- **`_CheckStatus(ios, ctx)` / `MHAL_JPE_CheckInstanceStatus`**: 1 busy (job in the encoding list) → 1, 2 done → 2, 7 drop
  → 4, 3 → 3, else 0; `JpeCtxCheckInstanceStatus` reports 3 (timeout) when the engine's MB position (`1620:35`, `36` low 9
  bits) has not moved since the last call (`ctx+0x14/+0x18`) while the job is still running. `JpeDevCheckIosStatus` and
  `JpeDevEncodeCancel` are tail calls into ios slots 8 / 6.
- **`_EncCancel`**: engine idle → nothing; direct mode: IMI input (`5f[14]`) → `reg00[7:4] = 0xF`, coding flag off, job
  marked drop; CMDQ mode: `reg26 = 1` (recorded or direct), job marked drop.
- **`_AddOutBuff`**: only while a job runs (`7f[15:8] ≠ 0`): store the new window in the job and program it (§4.8 3b–3e).
  `JpeDevAddOutBuff` refuses unless the state is 4 or 6.
- Other ios / device functions: `_Release` (ios free, clears the `7f[8:0]` shadow), `_SetBank` (bank VA set, `7f` low
  byte cleared), `_JpeTimespecDiffNs` (timespec difference).

## 5. Throughput limits observed on this camera [R: measurements on the running camera, unless marked]
<!-- ev:mhal-scl-jpe-cmdq.5 -->
- **Scaler clock**: at 432 MHz the SCL cannot write a 3840-wide output within the sensor's 9.48 µs line (ISP P0 FIFO FULL →
  ~12 % green frames: one well-formed JPEG holding a broken picture, ~1.34× the neighbours' size). At 480 MHz the output is
  clean. Rate ~1 pixel per clock [I].
- **Two outputs share the scaler**: the cost is output pixels per sensor line time summed over every enabled port. HDMI 1080p
  + USB 4K at a 9.48 µs line → a few overflows; at 9.85 µs (sensor mode 30) clean. HDMI 4K + any fast USB mode → heavy
  overflow on both outputs (1080-row modes upscaled to 4K: ~1 Gpixel/s); only sensor mode 22 (10.8 µs line, 4K40) is
  clean. Frame-rate control on the SCL → DISP bind (HDMI port at 29 fps in procfs) does not reduce it [I: the SCL computes
  every input line for every enabled port].
- **The ISP's line into the scaler**: while an output port runs 3840 wide, the shortest line the ISP can hand SCL dev 0 is
  ~9.0–9.2 µs at 480 MHz (USB 4K alone: 9.12 µs clean, 8.93 µs `P0_FIFO_FULL`) and ~10.3 µs at 432 MHz (HDMI 2160p and
  USB 4K: 10.39 µs clean, 10.21 µs not). 480 MHz is fclk1's highest rate, so 3840-wide modes through the scaler stop at
  ~47–48 fps, while the ISP alone reaches ~52.9 (isp-1-hal.md §5.5).
- **MIU arbiter** (`/sys/devices/system/miu/miu_arb0`; kernel-platform.md §12): SC0_FRM_W priority, JPE_R flow control,
  group-2/3 high-priority masks, USB20 priority / burst no-limit — no measurable effect on these limits. The group-0 HP
  mask at 0x1F202490 is 0xFFFF after boot; writing 0 brings back ~45 green frames per 10 s (a reboot restores it).
- **Destroying an SCL device hangs** in `_HalSclWaitInputSrcIdle` when a port is still enabled or its source stopped
  mid-frame («NO frame done», fence not finished); stopping the consumers before the SCL avoids it (prog_uvc.md §2.4).
