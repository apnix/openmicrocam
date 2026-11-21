# isp-1-hal — the ISP hal/driver layer of mhal.ko
<!-- ev:isp-1-hal -->

The lowest layer of the ISP code in `mhal.ko`: the driver layer
`DrvIsp_*`, the `/dev/isp` platform driver with its clock code and the "ISP driver" work-queue thread, the register HAL
`ISP_*` / `HalIsp*` (one function per field group over a handle of bank pointers), and the interrupt HAL. It owns the ISP's
interrupt controller and the event map of its interrupt, the bring-up, power, reset and input configuration of the ISP,
banks 1302 and 1303 field by field, the RDMA / WDMA / FBC / external-WDMA engines, the 3DNR, rotation, motion and
noise-estimate engines, the IQ-enable, gain and bypass registers, and the map of the 54 ISP register banks. It also carries the
map of the whole ISP block of mhal (its eight sub-parts) and the public material related to it. Not here: ispmid (the
interface `mi_isp` calls, the per-frame flow, CMDQ use, debug-fence use, frame mode), the statistics engines, MLOAD (bank 130D)
and ISPSCL (banks 1309, 134C–134F, 1342) — isp-2-ispmid-mload-ispscl.md; the ISP API ids and the API bin —
isp-3-iq-api.md; the IQ engine, the per-IP `HalIsp_Apply*` field tables, `Regmap` and the IQ file — isp-4-iq-engine.md; the
in-kernel AE / AWB / AF — isp-5-3a.md; libcus3a and the user 3A — isp-6-userspace.md; mhal's block map, the ISRCB registry,
character devices, module parameters and DT properties — mhal-core.md; the clock tree (`CLK_isp` parents) and interrupt
numbers — kernel-platform.md §5.1, §14; VIF's writes to banks 1302 / 1303 / 1342 — mhal-vif.md §4.5; the CMDQ engines and trigger-bus
events — mhal-scl-jpe-cmdq.md §2.1, §2.8.

**Conventions.** Origin tags: [R] read from the binary, its disassembly / decompile / emulation, a data section, the DT, the
boot log or a cited file; [S] stated by a source; [I] inference. A tag on a heading covers the section unless a line says
otherwise. Addresses are `.text` / `.data` / `.rodata` / `.bss` offsets of the relocatable module (Ghidra address = `.text` +
0x40000028). Register notation `BANK.rr[bits]` = RIU bank, 16-bit register rr: PA `0x1F000000 + BANK·0x200 + rr·4`, CPU VA
`0xFD000000 + BANK·0x200 + rr·4` (the RIU addressing itself: kernel-platform.md §3.1). A sub-block such as `1308.20` starts at
register 0x20 of the bank. **Reading the code**: the HAL keeps one `u16 *` per bank, so the C index `b1302[i]` and the `r`
argument of `ISP_RegW/RegWM(bank, r, val, mask)` are in 16-bit units — RIU register = `i/2` [R]. `W.xx` = a register of an ISP
WDMA block (§7.3), `R.xx` of an RDMA block (§7.2), `X.xx` of a WDMA group's common block. "This camera" = gc8613 Bayer RAW10,
VIF pipe 0 → ISP pipe 0 realtime, no HDR, no rotation, output to SCL realtime [R].

## 1. The ISP block of mhal.ko

### 1.1 Sub-parts [R sizes and names; grouping I]
The ISP code of mhal.ko is 310 KB / 2381 functions: the ISP block 2199, ISPSCL 84, MLOAD 98 [R]. Its object files are linked
one after another, so the sub-parts are address ranges (address-ordered name-prefix runs), refined by name for statistics:

| sub-part | KB | funcs | exports | what |
|---|---|---|---|---|
| hal/driver | 39.8 | 559 | 136 | `DrvIsp_*` (driver layer), `HalIsp*` / `ISP_*` (register HAL), RDMA/WDMA, 3DNR/rotation/motion/NE engines, IRQ HAL, clocks, `/dev/isp` — this document |
| statistics | 18.7 | 190 | 41 | AE/AWB/histogram/AF/RGBIR statistics engines: windows, sources, buffers, HDR switching; ispmid's statistics services — isp-2-ispmid-mload-ispscl.md §10 |
| in-kernel 3A | 61.7 | 314 | 7 | `pAE_*`/`AE_*`/`DoAe`, `pAF_*`/`AF_*`/`DoAf`, `pAWB_*`/`DoAwb`, `hal_AF_InitParameters_*` — isp-5-3a.md |
| IQ engine | 78.6 (80 536 B) | 273 | 10 | `pIQ_*`/`IQ_*` core (ISO/CT interpolation, triggers, cfg tables), `IQ_API20_SET_*`, `hal_IQ_*`, `IQ_Patch_*` (44, of which 27 stubs), `IspRegSet*`/`WriteRegs`, `IspMLoadSet*` — isp-4-iq-engine.md |
| IQ API | 39.5 | 503 | 5 | `DrvAlgo_IF_*` (id dispatch to IQ/AE/AWB/AF/calibration), in-kernel copy of the user `MI_ISP_*` / `call_MI_ISP_*` API and its bin loader — isp-3-iq-api.md |
| ispmid | 56.2 | 360 | 35 | `MHalVpeIsp*`/`MHalIspApi*` (mi_isp's entry points), `MS_CAM_*`, `Camera*` frame flow, CMDQ buffer, work queues, `/dev/ispmid`, `/dev/isp_fe` — isp-2-ispmid-mload-ispscl.md |
| menuload | 6.8 | 98 | 19 | MLOAD block: `IspMLoad*`, `DrvIsp_MLoad*`, `Mload*`, `/dev/mload` — DMA of register tables into ISP SRAMs — isp-2-ispmid-mload-ispscl.md §12 |
| ispscl | 8.2 | 84 | 8 | ISPSCL block: `DrvIspScl_*`, `HalIspScl_*` — YUV→Bayer front and YUV-pipe settle, `/dev/ispscl` — isp-2-ispmid-mload-ispscl.md §13 |

The hal/driver sub-part lies in `.text` 0x42b64–0x501c4, interleaved with the other ISP sub-parts; its size is
40.7 KB [R] (39.8 KiB). Nothing of the ISP has
public source; the closest material is the user-side API headers (ids and structs match) and the Mercury5 / Infinity register
headers (§12) [R search].

### 1.2 How the ISP block runs on this camera [I unless tagged]
- **The in-kernel 3A is dead code here**: prog_uvc's libcus3a sends `MI_ISP_CUS3A_Enable{AE 1, AWB 1, AF 1}` (live log [R]),
  mhal sets `nFlagCus3A = 0x1A`, and `CameraIspAlgo_DoAe/DoAwb` skip `DoAe/DoAwb` when those bits are set [R code]. AE / AWB /
  AF run in user space (libcus3a + libispalgo inside prog_uvc). The kernel keeps doing statistics, wakes user space through
  `/dev/isp_fe`, serves statistics on request and applies the exposure / WB results (isp-6-userspace.md §1.1, isp-5-3a.md §1.5). The
  user build of the 3A differs from the kernel copy in using double-precision `log2` / `pow` for the APEX conversions instead
  of the kernel's integer table, and its AF setters / defaults program the AF engine (kernel: store only, defaults zero) [R].
- **The IQ engine stays in the kernel and runs every frame**: it interpolates the IQ parameters by ISO / colour temperature
  and writes about 40 ISP sub-blocks (`IspRegSet*`) through the per-frame command buffer and menuload tables
  (isp-4-iq-engine.md §10).
- **One frame** in realtime: the per-frame ISP programming runs from `FrameStartProc` (driven by this layer's frame-start
  event, §4.4) into the ISP's own CMDQ engine behind an ISP_VSYNC wait; `MHalVpeIspProcess` returns at once and
  `CameraContextSwitch` is frame-mode only [R, isp-2-ispmid-mload-ispscl.md §6].

### 1.3 The hal/driver sub-part: objects [R address runs + names; grouping I]
Four objects in `.text` order, with statistics functions (`ISP_AE_*`, `ISP_AWB_*`, `ISP_AF_*`, `ISP_HISTO_*`, …) interleaved
inside the register HAL:

| range | object [I name] | content |
|---|---|---|
| 0x42b64–0x45058 | `drv_isp.c` | `DrvIsp_*` driver layer, `DrvIspISR`, the "ISP driver" thread work queue `IspDriverThreadWq*` |
| 0x45064–0x45e30 | `drv_isp_module.c` + clock HAL | `/dev/isp` misc device and platform driver (`isp_*`), sysfs, `DrvIspModule*Clk*`, `HalClkgen*`, `HalIsp*Clock*`, `IspSys*` (RIU base) |
| 0x45e32–0x4e2a6 | `hal_isp.c` | register HAL `ISP_*` and `HalIsp*`: one function per field group; `HalIsp_Init`, `HalIsp_InitReg`, `HalIsp_Apply*`, register-access helpers |
| 0x4e33c–0x501c4 | `hal_isp_irq.c` | interrupt controller HAL (`HalIspEnable/Disable/Mask/Clear*`), `HalIspIsr`, `HalIsp_ShowInts`, IRQ time stamps |

Function counts per prefix, two counts [R both, not reconciled]: by name over the ISP block: `DrvIsp_*`
≈180, `ISP_*` ≈330, `HalIsp*` ≈150; inside the hal/driver ranges only: `DrvIsp_*` ≈138, `ISP_*` ≈230,
`HalIsp*` ≈95. 115 of the 559 functions are stubs and 99 are unreferenced (§11).

What the groups hold [R names]:
- **Driver layer** `DrvIsp_*`: `Open/Close/InitHw/PowerOn/PowerOff/ResetIp`, `InOutModeConfig` (input format, crop, line-count
  interrupts, HDR), `ApplyShadow` / `ApplyIspBypassMode` / `ApplyDGain`, the CMDQ hooks (`ChangeCmdqIF`,
  `Clr/Mask/Force/PollCmdqIrq`, `GetIspCmdqTrigBusId`), fences (`Set/GetRdmaFenceID`, `Set/GetDebugFenceID`), raw store / fetch
  pipes, RDMA / WDMA / FBC, 3DNR (`DNREngine*`, `3DNR_Cal*BufSize`), rotation (`ROTEngine*`), motion and NE engines, WDR buffers,
  capability checks, `DrvIspISR`. Mostly thin wrappers that pass the HAL handle down.
- **Register HAL** `ISP_*` and `HalIsp*`: `ISP_Sen*` (input format / precision / Bayer order), `ISP_RDMA*` / `ISP_WDMA*`,
  `ISP_DNR*`, `ISP_ROT*`, `ISP_Motion*`, line-count / IRQ reset (`ISP_IrqResetIp`, 2 KB), `HalIsp_Init` (bank set-up, 1.3 KB),
  `HalIsp_Apply*` (copy of an IQ sub-block's fields from the shadow, with its bypass / enable bit: OBC, DPC, DM, LSC, ALSC, HDR,
  3DNR, CCM, YEE, GNR, …), `HalIspIsr` (2.8 KB status decode), `HalIspEnable/Mask/ClearInt*`, register-access helpers
  (`ISP_RegW/RegWM/RegIndexTableWM/RegValueTableWM`, `HalIspWrite/ReadRegWithBankAddr`).
- **Kernel glue**: `IspDriverThreadWq*`, `isp_probe`, `isp_ioctl` / `isp_fwrite` (debug), sysfs, `DrvIspModule*Clk*`,
  `HalIspClkgen*` (§3).

Nothing of this layer is imported by another module [R: none of the 559 names is an undefined symbol of any other `.ko` of the
dump]; the 136 "exports" are only `__ksymtab` entries. Callers are ispmid, statistics, the IQ engine (through
`DrvIsp_ApplyShadow`) and MLOAD / ISPSCL [R].

## 2. Objects and register access

### 2.1 Global objects [R]
| object | where | layout |
|---|---|---|
| HAL handle `IspHal` | heap, 0x150 B = 84 word slots (`HalIsp_Init`); pointer in `g_pIspHalHandler` (.bss 0x17d28) and in the driver context +0 | `[0x00…0x51]` u16 pointers to the 82 bank (sub-)blocks (§9) = `RIU VA + bank·0x200 (+sub·4)` (§9); `+0x148` CMDQ interface (0 = direct CPU stores; set by `HalIsp_ChangeCmdqIF`, isp-2-ispmid-mload-ispscl.md §6.3); `+0x14c` 0 |
| driver context | .bss 0x17c08, 0x44 B (`DrvIsp_Open` clears it, returns its address) | `+0` HAL handle; `+0xc + 8·s` {callback, data} for callback slots s = 0…6 (`DrvIsp_RegisterCallback(ctx, s, fn, data)`); rest 0 |
| module globals | .bss 0x17cb8 | `+0x00` Linux IRQ number (bit 15 = "ISR disabled" toggle of the `isp_isr_toggle` command); `+0x04` platform device; `+0x08` driver context; `+0x0c` RIU VA (ioremap 0x1F000000, 4 MiB, made once by `IspSysGetRegBase`); `+0x10…+0x2c` clk handles [8]; `+0x30…+0x4c` parent rates [8]; `+0x50` clock enabled; `+0x54` current index; `+0x58` parent count; `+0x5c` current rate; `+0x60` preset rate (sysfs `isp_clk`); `+0x64` "clock on" (suspend / resume state); `+0x68/+0x6c` char-dev region / dev_t of `isp0` |
| platform driver | .data 0x2650 | probe `isp_probe`, remove `isp_remove`, suspend `isp_suspend`, resume `isp_resume`, name `isp`, of-match table at .data 0x267c (DT `compatible = "isp"`) |
| preset clock index | .data 0x26ac | −1 = none; `isp_probe` writes 1. It is the module parameter `drv_isp_module.g_s32PreSetClkIdx` (mhal-core.md §5), which therefore has no effect |
| file operations | .data 0x26b0 | read `isp_fread`, write `isp_fwrite`, poll `isp_poll`, unlocked_ioctl `isp_ioctl`, open `isp_open`, release `isp_release` (Linux 4.9 `file_operations` offsets) |
| `misc` device struct | kmalloc-64 object allocated in `isp_probe` | minor 0xFF (dynamic), name `isp`, fops at .data 0x26b0, parent = platform device |
| interrupt counters + time stamps | .bss 0x17d2c (`gtIspFrameIntervalInfo`), ≥ 0xb98 B | §4.5 |
| `isp_ints` counters `gIspInts` | 0x1C4 B, cleared at probe | §3.1 |
| interrupt mask table | .data 0x3edc, 7 × u32 | §4.2 |
| driver-thread queue `_g_tIspDriverThreadWqPool` | .bss 0x17c4c | §3.4 |

`IspSysRegV2R(va)` = `va − RIU VA` (bank byte offset), used by the CMDQ path to turn a bank pointer into a RIU address.
`HalIspRead/WriteRegWithBankAddr(addr)` access `RIU VA + ((addr << 1) & 0x1FFFFFE)` — i.e. `addr` = bank·0x100 + reg (the form
the debug consoles use).

### 2.2 Register access [R]
- `ISP_RegW(bank, r, val)`: with a CMDQ interface (`g_pIspHalHandler->cmdqIf` ≠ 0) → `cmdqIf->RegW(cmdqIf, RIU/2 + r, val)`;
  else `bank[r] = val`.
- `ISP_RegWM(bank, r, val, mask)` (`.text` 0x4e20e): `val` is shifted left to the lowest set bit of `mask` (`_RegBitShift`),
  then either `cmdqIf->RegWM(cmdqIf, RIU/2 + r, val<<s & 0xFFFF, mask)` or a read-modify-write of `bank[r]`.
- `ISP_RegIndexTableWM(bank, src, table, n)` / `ISP_RegValueTableWM`: a field table {r, mask, source offset} (or {r, mask,
  value}) applied with `ISP_RegWM` per entry (used by `HalIsp_Apply*`, isp-4-iq-engine.md §10.3, and by `ISP_IQ_Init_Cfg`, §10.3).
- **Reads never go through CMDQ**: every status / readback function loads `bank[r]` directly (byte or 16-bit).
- Many interrupt-controller functions and a few others write the bank directly with byte / 16-bit stores (not `ISP_RegWM`),
  so those writes are always immediate CPU writes even while a CMDQ interface is installed.

## 3. Kernel glue: platform driver, `/dev/isp`, sysfs, clock, driver thread

### 3.1 Bring-up [R]
<!-- ev:isp-1-hal.3.1 -->
`Isp_Driver_Init` (step 8 of mhal's `camdriver_init`, mhal-core.md §3.2): log `[Isp_Driver_Init]`, **`IspDriverEarlyInit`**
(creates the "ISP driver" thread, §3.4), `CamPlatformDriverRegister(.data 0x2650)`. DT node `isp { compatible = "isp"; banks =
<0x1302>; interrupts = <0 0x39 4>; clocks = <&CLK_isp>; clock-frequency-index = <5>; }` [R]. The
DT gives only bank 0x1302 and GIC SPI 0x39; every other bank comes from `HalIsp_Init` (§9).

`isp_probe(pdev)`:
1. kmalloc the misc device; read DT `clock-frequency-index` into a local — **the value is not used**: the log line
   `[s32CurClkIdx] = %d` prints the constant 1 and the preset index (.data 0x26ac) is set to **1** [R; boot log
   `[s32CurClkIdx] = 1`].
2. `DrvIspModuleClkInit()` (§3.3) → clock index 1.
3. `misc_register` (`isp`, dynamic minor, fops .data 0x26b0); `alloc_chrdev_region(…, "isp0")`; `CamDeviceCreate(msys class,
   …, "isp0")` + sysfs link from the platform device; `drvdata` = misc struct.
4. **`DrvIsp_Open()`**: clear the 0x44-byte driver context, `HalIsp_Init` (allocate and fill the HAL handle — 82 bank pointers;
   logs `IspSysGetRegBase = 0xFD000000`, `isp0_cfg base = 0xFD260400` [R boot log]), `DrvIsp_InitSwIfWdma` (§5.1). Context
   address → module globals +8.
5. `CamIrqOfParseAndMap(node, 0)` → module globals +0 (Linux 27 for GIC SPI 0x39: boot log `[ISP] Request IRQ: 27, 57`).
   **The IRQ is not requested here**: mi_isp requests it (isp-2-ispmid-mload-ispscl.md §5); the only `request_threaded_irq` in
   this layer is the `isp_isr` debug command (§3.2).
6. `isp_create_attr_file`: clear the 0x1C4-byte `gIspInts` counters, create sysfs `isp_ints`, `isp_info`, `isp_clk`,
   `isp_reset` on `isp0`.

The only hardware writes at probe are those of `HalIsp_InitSwIfWdma` (§5.1; direct CPU stores, no CMDQ interface exists yet);
its last step `ISP_IrqResetIp(0x10)` prints the boot-log fragment `[rt]0x10-` just before `[ISP] Request IRQ` [R]. The ISP
proper is initialised later by ispmid: `CameraInit` → `DrvIsp_Resume` = `DrvIsp_InitHw` + `HalIspEnableInt` (§5.1, §4.2).

`isp_remove`: `DrvIsp_Close` (`DeinitSwIfWdma`, `HalIsp_Deinit`), clock de-init, remove sysfs, `misc_deregister`, destroy device,
unregister chrdev, kfree. `isp_suspend` (platform PM): `DrvIsp_DeinitSwIfWdma` + `Isp_Clock_Suspend` (clock off once);
`isp_resume`: `Isp_Clock_Resume` (re-enable at the remembered index) + `DrvIsp_InitSwIfWdma`. `DrvIsp_Suspend` (called by
ispmid) only logs; `DrvIsp_Resume` re-runs the full hardware init. `Isp_Driver_Exit`: unregister, `IspDriverFinalRelease` (stop
the thread, free its ring).

### 3.2 `/dev/isp` (misc `isp`, char dev `isp0`) — debug only [R]
Nobody on the camera opens it [R: the string `/dev/isp` with a terminating NUL occurs in no file of the dump; the only
`/dev/isp…` strings are `/dev/isp_fe` in prog_uvc and libcus3a]. `open` stores (module globals +4)->`drvdata` in
`file->private_data`; `release` clears it; `read` and `poll` return 0.

`ioctl(cmd, arg)`: the type byte (bits 15:8) must be `'I'` (0x49), else −ENOTTY (−25). The case index is `cmd − 0x4901` on the
**whole** 32-bit `cmd`, so only numbers without `_IOC` direction / size bits reach a case (anything else → 0):

| cmd | action |
|---|---|
| 0x4901 | `copy_from_user` 0x288 bytes into a stack buffer, then nothing (return 0) |
| 0x4902 | `DrvIsp_PowerOff(ctx)` (§5.1) — the only effective command |
| 0x4903–0x4907 | return 0 |
| 0x4908 | copy 0x14 bytes {…, H @+8, W @+0xc, BufAddr @+0x10} and only log `ISP output wdma: W=%u , H=%u, BufAddr=0x%X` |
| 0x4909 | copy 0x18 bytes, nothing else |

A short copy zero-fills the tail and executes `udf #2` (kernel BUG) [R 0x45118: the `copy_from_user` size check of this
kernel].

`write(buf)`: `sscanf(buf, "%s %d %d %d %d")` → command + 4 integers:

| command | effect |
|---|---|
| `log_en a b` | `IspLogSetEn(a, b)` (the ISP log mask used by every `IspLogGetEn()` test) |
| `isp_isr` | `request_threaded_irq(irq, isp_ISR, NULL, 0, "isp interrupt")` + affinity hint to the online CPUs — a second, debug handler |
| `isp_isr_toggle` | toggle bit 15 of the stored IRQ number: while set, `isp_ISR` returns IRQ_NONE without touching hardware |
| `isp_irq_mask` | `DrvIsp_IspMaskIntAll` → `HalIspMaskIntAll` (§4.2) |

`isp_ISR`: if the driver context exists and bit 15 is clear, `DrvIspISR(ctx)` (§4.4) and IRQ_HANDLED.

### 3.3 Clock [R]
<!-- ev:isp-1-hal.3.3 -->
Hardware: `CLK_isp` = clkgen **1038.61[12:8]**: bit 8 gate (1 = off), bits 12:10 mux (DT `reg = <0x1f207184 4>`, mux-shift 10,
width 3, gate-shift 8; 8 parents — names in kernel-platform.md §5.1). The rate table of the direct path (`.rodata+0x784c`, MHz
then Hz):

| index | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| rate | 480 | 432 | 384 | 320 | 240 | 216 | 172.8 | 72 MHz |

Two paths:
- **DT path** (platform device present — always on the camera): `DrvIspModuleInitClk(pdev, 8)` takes `of_clk_get(node, 0)` and
  records the rate of each of the 8 parents; `DrvIspModuleSetClkByIndex(pdev, i)` / `SetClkRate(pdev, rate)` = set parent i,
  `clk_set_rate`, prepare + enable; `CloseClk` = parent 0, disable, unprepare.
- **Direct path** (no platform device): `HalIspSetClockRate(rate)` → `HalClkgenCtrl(0, idx, on)` writes the byte at
  1038.61[15:8] = `(byte & 0xE0) | (idx << 2) | (rate == 0)`; `HalClkgenGetFreq` reads it back (used by `ISP_ROTSetFramePeriod`,
  §8.2). `HalIspClkgenToId` maps a rate (MHz or Hz) to its index; `HalIspGetDefaultClock` = 432 MHz / index 1.

Selection at probe / resume (`DrvIspModuleClkInit`): the preset rate (sysfs) if set, else the preset index if ≠ −1, else the
default 432 MHz. After probe the preset index is 1 → parent 1 = **432 MHz**. Live reading: `isp_clk` = 432000000 [R: item "ISP clock is not the line-time limit"].

sysfs on `/sys/devices/virtual/mstar/isp0/`:

| file | read | write |
|---|---|---|
| `isp_clk` | current rate (preset rate, else index rate, else 432000000) | decimal Hz: 0 → clock off; else `SetClkRate(0, rate)` (direct path) and remember it as preset rate [I: with pdev = 0 this programs 1038.61 directly, bypassing the clk framework] |
| `isp_reset` | help text `1: ISP All reset / 2: ISP channel 1 reset / 4: ISP 3DNR reset / 8: ISP ROT reset` | decimal flags → `DrvIsp_ForceIpReset` (§5.3) |
| `isp_ints` | `HalIsp_ShowInts` (§4.5) | — |
| `isp_info` | `HalIsp_ShowInfo`; among its lines: input "RGB" vs "YUV422", BayerID, "HDR mode", "P<n> HS", "PIPE0 ACK TIE" (the fields of §6.1 marked `isp_info`) | — |

Clock functions not reached [R call graph]: `DrvIsp_GetClockIdxByRate`, `DrvIsp_GetClockRateByIdx`, `DrvIspModulePreSetClkIdx`,
`DrvIspModuleGetOfNode`, `IspClkGenInit` (`preset index := arg; ClkInit`), `IspClkGenDeinit` — no caller, no pointer (unused).

### 3.4 The "ISP driver" thread (`IspDriverThreadWq*`) [R]
One global queue (.bss 0x17c4c, `_g_tIspDriverThreadWqPool`): mutex @0, atomic count @0x28, write index @0x2c, read index
@0x30, semaphore @0x34, thread @0x54, ring pointer @0x58 (0x6C00 bytes allocated, 256 entries × 16 B used: {fn, arg, time
stamp, valid}), stop flag @0x5c, counters @0x60 (wake-ups), @0x64 (functions run), @0x68 (5 s time-outs).
- `Init` (from `IspDriverEarlyInit`): thread "IspDriverThreadWq", priority attribute 99, body `Proc`.
- `AddInISR(fn, arg, ts)` → `Add`: refuse when 256 entries are pending or fn is 0; store, advance, count, post the semaphore.
  Called only by `DrvIspISR`; a refusal logs `ISP_IRQ_WQ_FRAME_START/DONE add WQ error!` plus the four status counters.
- `Proc`: wait on the semaphore with a 5000 ms time-out; on a post drain the ring calling `fn(arg, ts)`; on time-out count and
  loop; any other wait error ends the thread; it also ends when the stop flag is set.
- `Flush` (ispmid `CameraIsp_WorkQueueFlush`): drop all pending entries, reset the indices, clear the ring. `Deinit` = the same
  as `IspDriverFinalRelease` (unreferenced copy). `Status{NumWorks,WorkCounter,FuncCounter,TimeoutCounter}` return the
  counters.

So every ISP interrupt callback (frame start / done, DMA done …) runs in this thread, in order, with the interrupt's time
stamp.

## 4. Interrupt controller (bank 1302.28–.43) and the ISR

The ISP has one interrupt, GIC SPI 0x39 (Linux 27), requested by mi_isp; its chain down to this layer is in
isp-2-ispmid-mload-ispscl.md §5. This section is the controller, its bit map and the event map.

### 4.1 Registers [R `HalIspEnable/Disable/Mask*/Clear*`, `HalIspGetIntStatusN`]
The register functions are `HalIspGetIntStatus*` (status), `HalIspMask*` (mask) and `HalIspClear*` / `HalIspMaskClearN`
(acknowledge). Seven groups g = 1…7, four registers each at **1302.(0x28 + 4(g−1)) + k**:

| k | role | evidence |
|---|---|---|
| +0 | mask (1 = masked) | `EnableInt` writes the table, `MaskInt` ORs bits in, `EnableIrq(…, on)` clears bits |
| +1 | force [I name] | `MaskClearN` clears the acknowledged bits here; never set by any code |
| +2 | clear (write 1) | `MaskClearN` ORs the acknowledged bits in; `EnableInt` writes `~mask` (clears every unmasked bit); `ClearIntAll` copies it to the mask and zeroes it |
| +3 | status | `GetIntStatusN = status & ~mask` |

Widths: g1, g2, g5 16 bits; g3 11 (byte + 3 bits); g4, g6 12; g7 7 (bit 7 of 1302.40 / .42 / .44 is preserved by every writer,
i.e. another field shares the byte [I]). The narrow groups are written byte by byte so the upper bits of those registers are
left alone. All interrupt-controller writes are direct CPU stores (never CMDQ).

### 4.2 Default enable table [R .data 0x3edc]
`HalIspEnableInt` (from `DrvIsp_Resume`, i.e. at ispmid's `CameraInit`) loads the masks; `HalIspDisableInt` and
`HalIspMaskIntAll` mask everything (`MaskIntAll` also re-clears with `~table`). `DisableInt`, `MaskInt1…7`, `ClearInt`,
`ClearIntAll` are unreferenced.

| group | mask word | unmasked bits |
|---|---|---|
| 1 | 0x2A3F | 6, 7, 8, 10, 12, 14, 15 |
| 2 | 0xA2FF | 8, 10, 11, 12, 14 |
| 3 | 0xFF06 (11 bits: 0x706) | 0, 3, 4, 5, 6, 7 |
| 4 | 0xF074 (12 bits: 0x074) | 0, 1, 3, 7, 8, 9, 10, 11 |
| 5 | 0xFB64 | 0, 1, 3, 4, 7, 10 |
| 6 | 0xFB6D (12 bits: 0xB6D) | 1, 4, 7, 10 |
| 7 | 0xFF87 (7 bits: 0x07) | 3, 4, 5, 6 |

`HalIspEnableIrq(g−1, bits, on)` is the only later change: mi_isp's `EnableIrq(ctx, 1, on)` → `HalIspEnableIrqMapTable` → g1
bit 10 (ISP_IDLE). That bit is already unmasked by the table, so the call changes nothing when `on` = 1.

### 4.3 Bit map [R: names from the `isp_ints` counter labels in `HalIsp_ShowInts`; actions from `HalIspIsr`]
Each handled bit increments the counter `gtIspFrameIntervalInfo + 0x8c8 + 0x40·(g−1) + 4·b`. "ev n" = bit n of the word
`HalIspIsr` returns; §4.4 says where each ev goes. "U" = unmasked by default.

| g.b | U | name | action in `HalIspIsr` |
|---|---|---|---|
| 1.0 | | `P0 VIF FS Rising` | time stamp (0,0) |
| 1.1–1.5 | | — | counted only |
| 1.6 | U | `P0_FIFO_FULL` | reset request 0x101 (frame mode 0x180) → `ISP_IrqResetIpFlag` (§5.3); ev 24 in frame mode; log `ISP P0 FIFO FULL(line, ext-WDMA line)` |
| 1.7 | U | `P1_FIFO_FULL` | reset 0x212 (0x280); ev 25 in frame mode |
| 1.8 | U | `P2_FIFO_FULL` | reset 0x400 (0x480); ev 26 in frame mode |
| 1.9 | | — | counted, time stamp (0,9) |
| 1.10 | U | `ISP_IDLE` | clear the "RDMA0 seen" flag; outside frame mode run a pending forced reset (`ISP_IrqResetIpFlag(g_nIspResetIpFlag)`, §5.3); time stamp (0,10); the bit mi_isp sees as code 1 (§4.6) |
| 1.11 | | — | not handled |
| 1.12 | U | `ISPIF_VSYNC` (frame start) | **ev 0**; time stamp (0,12), input line count saved, frame counter ++ |
| 1.13 | | — | counted |
| 1.14 | U | `P0_VS_ERR` | counted |
| 1.15 | U | `P1_VS_ERR` | counted |
| 2.0–2.7 | | — | counted only |
| 2.8 | U | `HDR_FRAME_DONE` | counted |
| 2.9, 2.15 | | — | not handled |
| 2.10 | U | `ROT_FRAME_DONE` | time stamp (1,10) |
| 2.11 | U | `ROT_ERR_TRIG` | reset request 1 → `ISP_IrqResetIpFlag`; log `ISP ROT ERROR(output line count)` |
| 2.12 | U | `NE_DONE` | counted |
| 2.13 | | — | counted |
| 2.14 | U | `SW_INT_OUTPUT_DONE` (frame end) | **ev 1**; time stamp (1,14); argument word := RDMA fence 0 \| (RDMA fence 1 & 0x1F) << 16 \| (output-done period in 100 µs) << 20; frame-mode per-channel counters (`CH<n>` lines of `isp_ints`); with the "RDMA0 seen" flag set and a frame end without IDLE, log `[ISP_MISC:…]` |
| 3.0 | U | `3DNR0_FRAME_DONE` | time stamp (2,0) |
| 3.1, 3.2 | | — | counted |
| 3.3 | U | `RDMA0_DONE` | sets the "RDMA0 seen" flag |
| 3.4 | U | `RDMA1_DONE` | ev 5 |
| 3.5 | U | `WDMA420_DONE` | counted |
| 3.6 | U | `WDMA420_LINE_HIT` | ev 7 |
| 3.7 | U | `WDMA420_DONE_HIT` | counted |
| 4.0 | U | `MLOAD_DONE` | counted |
| 4.1 | U | `AE_DONE` | time stamp (3,1), line count saved |
| 4.2 | | — | counted, line count saved |
| 4.3 | U | `AWB_DONE` | counted |
| 4.4 | | `AWB_ROW_DONE` | counted |
| 4.5 | | — | counted, line count saved |
| 4.7 | U | `AF_DONE` | counted |
| 4.8 | U | `AF_ROW_DONE` | counted |
| 4.9 | U | `AF_DMA_ERR` | log `ISP AF DMA ERROR` |
| 4.10 | U | `RGBIR_HISTO_DONE` | counted |
| 4.11 | U | `WDR_DONE` | counted |
| 5.0 / 5.3 | U / U | `WDMA0_DONE` / `WDMA1_DONE` | ev 2 / ev 3 |
| 5.6 / 5.9 | | `WDMA2_DONE` / `WDMA3_DONE` | ev 11 / ev 12 |
| 5.1, 5.4, 5.7, 5.10 | U | `WDMA0…3_LINE_HIT` | ev 8, 9, 10, 13 |
| 6.0, 6.3, 6.6, 6.9 | | `WDMA4…7_DONE` | ev 16, 17, 18, 19 |
| 6.1, 6.4, 6.7, 6.10 | U | [I: `WDMA4…7_LINE_HIT`, pattern of group 5; no label] | ev 20, 21, 22, 23 |
| 7.2 | | `P3_FIFO_FULL` (counter at 0xa50) | reset 0x800 (0x880); ev 27 in frame mode |
| 7.3–7.6 | U | `ISP1_START`, `ISP1_DONE`, `ISP1_IN_HIT`, `ISP1_OUT_HIT` | counted |

At the end every group is acknowledged with `HalIspMaskClearN(handled bits)`. Bits not listed are neither counted nor cleared.
`HalIspIsr` also samples, at every interrupt: the debug input line count (1302.27), the FIFO max-valid level of pipe 0
(1302.19), the ext-WDMA line count (130C.44), and debug fence ids 0, 1, 6 (the frame-mode flag, stored at
`gtIspFrameIntervalInfo`+0x700) and 7 (the open-channel mask, stored at +0xb10).

### 4.4 Dispatch [R `DrvIspISR`, `DrvIsp_CheckIrq`]
GIC SPI 0x39 → mi_isp → `MHalVpeIspCheckIrq` → `DrvIsp_CheckIrq(ctx)` = `HalIspCheckIrqMapTable` (returns the code mi_isp sees,
§4.6), then `DrvIspISR(ctx)`. `DrvIspISR` turns ev bits into work for the driver thread (§3.4, `IspDriverThreadWqAddInISR`
with the interrupt's time stamp) or into ISR-callback calls (`ISRCB_Proc(id)`, the mhal-wide registry of mhal-core.md §3.4;
ids from the I6E header `drv_isrcb.h`, extended in this build):

| ev | goes to | registered by (ispmid unless said) |
|---|---|---|
| 0 | thread: callback slot 1 | `FrameStartProc` |
| 1 | thread: slot 0 | `FrameDoneProc` |
| 2, 3 | thread: slots 5, 6 | raw-store / image-store DMA-done handlers (frame mode, debug) |
| 5 | thread: slot 3 | RDMA (raw fetch / HDR) done handler |
| 7 | thread: slot 4 | WDMA420 line-hit handler |
| 6 | thread: slot 2 | never produced by `HalIspIsr` [R] — slot 2 never fires |
| 8, 9, 10, 13 | `ISRCB_Proc(4, 5, 6, 7)` | I6E names `ISP_WDMA0…3_HIT_LINE_CNT`; mhal registers `SensorFrameDoneISR` on them |
| 11, 12 | `ISRCB_Proc(2, 3)` | `ISP_WDMA2/3_DONE` |
| 16–19 | `ISRCB_Proc(0x28…0x2B)` | VIF WDMA done of pipes 4–7 (the same ids as ISPSCL's interrupt group 1, isp-2-ispmid-mload-ispscl.md §13.6) |
| 20–23 | `ISRCB_Proc(0x34…0x37)` | `SensorFrameDoneISR` [I: line-hit of pipes 4–7] |
| 24–26 | `ISRCB_Proc(0x38…0x3A)` | FIFO-full P0–P2 in frame mode; VIF's `DrvVif_GroupEnable(g)` registers `DrvVif_DoVifFifoFullEventCb` on 0x38 + g (mhal-vif.md §9.1); 0x3b is registered but never raised |
| 4, 27 | nothing | — |

Thread work items get `(data, argument word)`; the argument word is non-zero only when ev 1 was set (§4.3 row 2.14).

### 4.5 Counters and time stamps [R]
<!-- ev:isp-1-hal.4.5 -->
`gtIspFrameIntervalInfo` (.bss 0x17d2c); in `16g+b` the group g counts from 0:

| offset | content |
|---|---|
| `+0x000 + 8·(16g+b)` | {period ns, previous ns} per (group, bit), computed by the static helper at `.text` 0x4e2d4 (difference of the nanosecond parts of two `CamOsGetTimeOfDay` stamps with 1 s wrap, kept only if < 0.5 s) |
| `+0x380 + 8·(16g+b)` | {s, ns} of the last occurrence (`ISP_IrqUpdateTimeStamp`) |
| `+0x700` | debug fence 6 sampled every interrupt [R; §4.6 also reads the word as a frame-interval guard] |
| `+0x708 + 4·(16g+b)` | line counts (the slots `ISP_IrqUpdateLineCnt` is named for; that function itself has no caller, §11 — `HalIspIsr` stores them [I]) |
| `+0x8c8 + 4·(16g+b)` | event counters |
| `+0xa88` | frame counter |
| `+0xa8c + 4·ch` / `+0xb18 + 4·ch` | per-channel frame-end count / period (frame mode) |
| `+0xb0c` | "reset done" (§5.3) [I: offset taken relative to this object] |
| `+0xb10` | debug fence 7 sampled every interrupt |
| `+0xb12` | "RDMA0 seen" |
| `+0xb14` | reset-request word |

`gIspIrqLineCntStats` counts line-hit values per event; `ISP_IrqClearCounter` (from `DrvIsp_PowerOn`) zeroes counters and
stamps. `isp_ints` prints `frame_count`, `frame interval`, the named counters of §4.3 with time stamps / periods for VSYNC,
AE_DONE, OUTPUT_DONE, ISP_IDLE ("ISP BUSY period"), 3DNR, ROT.

`ISP_IrqRotProcFramePeriod` (from `DrvIsp_IrqRotProcFramePeriod`, which is unreferenced): when both the 3DNR-done and ROT-done
periods are ≥ 1 µs and the ROT period is longer than the 3DNR period, or shorter by more than 1/8 of it, while the 3DNR period
(= the frame period [I]) is 15.001–40 ms, re-program the rotation frame period to 90 % of the 3DNR period — which never writes
anything (§8.2).

### 4.6 What mi_isp sees [R disassembly 0x4ed0c]
<!-- ev:isp-1-hal.4.6 -->
`HalIspCheckIrqMapTable(h, 1)`, the return of `DrvIsp_CheckIrq`, on g1 status (`1302.2B & ~1302.28`):

| condition | code |
|---|---|
| g1 bit 10 (ISP_IDLE) | **1** |
| g1 bit 12 (VSYNC, frame start) | **4** (wins over 1) |
| g1 bits 6 / 7 (P0 / P1 FIFO full) or g2 bit 11 (ROT error), while the word at `gtIspFrameIntervalInfo`+0x700 is 0 | **2** |
| debug fence 11 = 0x5A5A (set by the frame-mode context switch) | **2**, fence cleared |

The `+0x700` word is read in two ways [R both, not settled]: as "not in frame mode" (it holds the sampled debug fence 6, the
frame-mode flag —) and as a frame-interval guard. The "external YUV420 WDMA
active" test that guards codes 1 and 2 is `ISP_IsExtWdam420()`, which **always returns 1** in this build — so codes 1 and 2
occur in realtime too, and in realtime mi_isp gets **4 at every frame start and 1 at every ISP idle** (frame end) [R]. What
mi_isp does with each code: isp-2-ispmid-mload-ispscl.md §5 (4 → frame-start list, 1 → frame done /
`mi_sys_NotifyWakeupISREvent`).

## 5. Driver layer: initialisation, power, input configuration, resets

### 5.1 Hardware initialisation and power [R]
<!-- ev:isp-1-hal.5.1 -->
Three entry points write the ISP's global state; all write directly (no CMDQ interface exists at that time). Whether each write
changes the reset default, i.e. which writes are no-ops, is not known: no dump of the ISP registers taken before mhal loads
exists [R; CAMERA-CHECKS P10].

**`HalIsp_InitSwIfWdma`** (probe and platform resume, via `DrvIsp_InitSwIfWdma`):
1. input mask on for pipes 0 and 1 (`HalIspInputMask(0/1, 1)`: 1302.0E[0] / 1302.0F[0] = 1);
2. 1302.10[1] = 1, 1302.10[5] = 1 (raw-fetch "HF" bits of pipes 0 / 1 set);
3. 1302.09[8..11] 0 → 1 (four async-FIFO resets, active low); 1302.05[8..11] = 1;
4. all eight VIF→ISP pipe routes off: 1303.10+p `[2:0] = 4`, `[9:8] = 3` (p = 0…7; register value 0x0304) — the same value VIF
   writes for a disabled pipe (mhal-vif.md §5);
5. 1302.68 = 0x0300, 1302.69 = 0x0001, 1302.6A = 0x00FF, 1302.6B = 0x000F [meaning unknown, no other writer or reader];
6. every WDMA w = 0…15: dummy config `W.12 = 0x8010` (C index 0x24 of the block, §7.3); FBC off for all 16
   (`ISP_DmaFbcEnable(w, 0)`);
7. interface clocks of pipes 0–2 on (1302.05[0..2] = 1); SW-interface resets 0–2 pulsed (1302.09[0..2] 0 → 1);
8. `ISP_IrqResetIp(0x10)` (RDMA + WDMA reset, §5.3), which prints `[rt]0x10-`.

`HalIsp_DeinitSwIfWdma` (remove / platform suspend): `IrqResetIp(0x10)`, pulse the three SW-interface resets, interface clocks
off.

**`DrvIsp_InitHw`** (from `DrvIsp_Resume`, i.e. ispmid's `CameraInit` and `CameraIsp_Resume`), followed by `HalIspEnableInt`
(§4.2):
1. `ISP_LatchMode(0)`: 1302.0C[2] = 0; `ISP_Kernel/Raw/ISPDP_GatedClk` — empty stubs in this build;
2. for pipe 0…2: `SWReset(1)`, `SWReset(0)` (both act on the single bit 1302.08[0]: 0 then 1), `EnableISP(pipe, 0)`
   (1302.00[0] = 0, 1302.01[0] = 0, pipe 2 → 1302.16[4] = 1);
3. rotation reset pulse 1302.08[2]; double buffer: `EnableDblBuf(…, 0)` twice → 1302.0C[15] = 1, 1302.0C[1] = 0;
4. `ISP_SenFmtConv(7)`: 1302.14[3] = table[7] & 1 (table `.rodata+0x788c`, 8 bytes);
5. `ISP_IQEnable(id, 0)` for ids 0, 4, 5, 7, 8, 9, 10, 11, 13, 14, 17, 19, 20, 21, 22 — the enable bits of §10.1 cleared;
6. RDMA 0 / 1 off (130A.00[0], 130A.20[0] = 0); `ISP_RMUXSrc` stub; `ISP_WMUXSrc(…, 1)` → 1303.01[0] = 0 (1 only for source 7);
7. **`HalIsp_InitReg`**: 1302.16[0], [4], [1] = 1 (pipe-0 ACK tie on, pipe-2 async off, bit 1); statistics block 131B.03[3:0] =
   2, 131B.04[3:0] = 2, 131B.00[0] = 1, 131B.00[4] = 1; input masks on (pipes 0, 1); 1302.09[12] 0 → 1; 1302.05[12] = 1;
   1302.04[7] = 1; debug fence 10 1302.7A = 0x5A5A; FBD 0 and FBD 1 off (1308.78[8], [9] = 0, §7.3); FBC v-sync select 3 in all
   four WDMA groups (`X.7F[5:4] = 3` for X = 1308, 130B, 1348, 1349 — written as byte +0x07 of the 0x78 common block);
   `ISP_IQ_Init_Cfg` (§10.3: ALSC interpolation kernel, R2Y table, DM / YUV defaults); 132C.64[7:0] = 0x16, 132C.65[7:0] = 0x13;
   RDMA 0 request blank 0x3F, threshold 0x7F, burst 0x20 (§7.2); 1328.12[7:0] = 2, 1328.11[6:0] = 7, 1328.11[13:7] = 2 (3DNR DMA
   defaults, §8.1).

**`DrvIsp_PowerOn`** (ispmid `MhalCameraOpen`, after the input configuration): `DisableISPDPClk(0)`, `DisableRawDNClk(0)`
(stubs); per pipe 0…2 SW-reset pulse + `EnableISP(pipe, 1)` (1302.00[0] = 1, 1302.01[0] = 1; pipe 2 untouched when enabling);
`Isp2SclEnable(1)` (1302.03[0], [1] = 1); clocks on: 1302.04[0] Bayer top, [1] pixel top, [5] DPC, [6] WDR; clear the IRQ
counters (§4.5).

**`DrvIsp_PowerOff`** (close, `isp_ioctl 0x4902`, ispmid early init): reset pulses, ISP→SCL off, the four clocks off, pipes 0–2
off (pipe 2 → 1302.16[4] = 1), RDMA 0 and 1 reset (§7.2). `DrvIsp_PowerOffYuvPipe` = `EnableISP(2, 0)` only.

### 5.2 Input configuration — `DrvIsp_InOutModeConfig(ctx, cfg)` [R]
Called by ispmid at open and per context switch with the channel's input record (ch +0x1977c; offsets as words `[n]` / bytes
`+x`): `[0]` mode (frame modes 2, 3, 5), `+8` / `+10` u16 width / height, `[4]` `[5]` crop x, y, `[6]` crop height, `[7]` crop
width [I: from the argument order of `SetISPCrop(x, y, [7], [6])` and the line-count uses], `[0x18]` Bayer id, `[0x1a]` YC order,
`[0x9d]` HDR type, u8 `[0x9f]` ACK tie, u8 `+0x279` HDR on, `+0x27a` "line count on crop height", `+0x27b` YUV input.

Pipe = 0, except YUV input in realtime → `ISP_GetYuvPipeId()` (a stub returning 0 — so also 0). Steps for that pipe:
- `SenBayerFmt(pipe, id)` (pipe 0 only: 1302.14[9:8] = table[8 + id], `.rodata+0x7894`);
- `SenDataType(pipe, !yuv)` (pipe 0: 1302.14[0] = Bayer, 1302.14[2] = YUV);
- `InputImageResolution(pipe, w − 1, h − 1)` (pipe 0 → 1303.30 / 1303.31 [12:0], pipe 1 → 1303.32 / .33, pipe 2 → 1303.24 /
  .25);
- `SetISPCrop` — **an empty stub**: no crop register is written;
- output line-count interrupt (pipe 0: 1302.1D[12:0] = crop height, or crop width for HDR types 1 / 3 / 6 / 7 when `+0x27a` is
  set, clamped 0x1FFF; pipe 2: 1302.1E and 1302.1F = n − 1);
- line-count interrupt 0 `1312.5D[12:0] = [6]`;
- YUV: `SenYCOrder` (1302.14[1]);
- `SetBayerScaler` — empty stub;
- `ICPASync(pipe, ack tie)` (pipe 0 → 1302.16[0]);
- `EnableISP_HDR(hdr)` (§5.4).

With HDR on, the same for pipe 1 plus the HDR buffer / core line counts 131E.78 / 131E.79 = `[6]`
(`SetHDRBufLineCountY/CoreLineCountY`).

### 5.3 Resets [R `ISP_IrqResetIp`, `ISP_IrqResetIpFlag`, `DrvIsp_ForceIpReset`]
`ISP_IrqResetIp(h, flags)` — direct byte stores, in this order:

| flag | action |
|---|---|
| (always first) | debug fence 10 1302.7A := 0 |
| 0x001 "ISP all" | with 1302.0C[15] forced 1: 130C.20[15] := 0 (ext WDMA off); ISP reset 1302.08[0] and SW-IF 0 reset 1302.09[0] 1 → 0 → 1; 1303.51 := 0x0002 (force CMDQ-irq id 1); 1302.54 := 0x7FFF, 1302.55 := 0x8000 (CMDQ-irq group B masked except bit 15, force bit 15 = id 4) [I: wakes a CMDQ thread waiting on an ISP event] |
| 0x002 "channel 1" | pulse 1302.08[0] and 1302.09[1] |
| 0x004 "3DNR" | pulse 1302.08[1] |
| 0x008 "ROT" | pulse 1302.08[2] |
| 0x010 DMA | pulse RDMA 1302.08[3] and WDMA groups 1302.09[3], 1302.09[4] |
| 0x020 | the 130C.20[15] := 0 step alone |
| 0x080 + pipe mask in bits 11:8 | WDMA recovery. Mask 0: wait until 1302.48[9] = 0, pulse 1302.09[3]. Else for every route k (1303.10+k with `[2]` = 0 and `1 << [1:0]` in the mask): if YUV→Bayer unit k is on (1309.(0x10k)[0]) pulse 1342.45[k]; if 1342.46[k] is set pulse it and mark "group of k busy" (k < 4 → rw0, else rw1); chip revision 1 with any hit → all four; per marked group g = 0…3: (g = 0: wait 1302.48[9] = 0) pulse 1302.09[3 + g], fence 10 := 0x5A5A, log `~rw<g>~` |
| 0x100…0x800 | async FIFO k = 0…3: pulse 1302.09[8 + k], set 1302.18[k]; flag 0x400 also pulses 1302.09[2] (SW-IF 2) |
| (always last) | fence 10 := 0x5A5A, log `[rt]0x<flags>-` |

`ISP_IrqResetIpFlag(h, &word)`: if `word` ≠ 0, `IrqResetIp(word & ~0x8000)`; keep the word only if bit 15 was set (sticky
request), else clear it; set "reset done" (`+0xb0c` = 1). It is run from the ISR on FIFO full (requests 0x101 / 0x212 / 0x400 /
0x800, +0x80 in frame mode — i.e. ISP reset + WDMA recovery of the pipe), on ROT error (1), and on ISP_IDLE for a pending forced
reset. `DrvIsp_ForceIpReset(ctx, flags)` (sysfs `isp_reset`): store `flags & 0x7FFF` as the pending request, clear "done", spin
up to 10⁹ iterations for the ISR to perform it at the next ISP_IDLE; on time-out do it directly. ispmid's CMDQ recovery
(`CameraIspCmdqBuffer_CleanCmdq`) calls `DrvIsp_ResetIp(1)` (isp-2-ispmid-mload-ispscl.md §7.5).

Other reset helpers — all active low (written `!reset`): `ISP_SWReset` 1302.08[0] (pipe argument ignored),
`ISP_DNREngineSWReset` 1302.08[1], `ISP_RotationSWReset` / `ISP_ROTEngineSWReset` 1302.08[2], `ISP_RdmaSwReset` 1302.08[3],
`ISP_Dma420SwReset` 1302.08[4], `ISP_MloadSwReset` 1302.08[5], `ISP_RiuSwReset` 1302.08[8], `ISP_WdmaSwReset` 1302.09[3] + [4],
`ISP_IrqResetSwIf(n)` 1302.09[n]. `RdmaSwReset`, `Dma420SwReset`, `MloadSwReset`, `RiuSwReset`, `WdmaSwReset` have no caller
(unused). `ISP_RDMAReset(ch)`: RDMA ch `R.00[15]` = 1, 1302.08[3] = 0, `R.00[15]` = 0, 1302.08[3] = 1. `ISP_WdmaReset(w)`: WDMA
w `W.00[15]` = 1, group reset 1302.09[3] (w < 4) or [4] (w ≥ 4) 0 → 1, `W.00[15]` = 0.

### 5.4 HDR, double buffer, input mask [R]
- `ISP_EnableISP_HDR(on)`: on → 1302.04[2] = 1, 1302.06[4] = 0, 131E.00[0] = 1; off → (with 1302.0C[15] forced 1) 131E.00[0] =
  0, 1302.04[2] = 0, 1302.06[4] = 1. `HalIsp_ShowInfo` prints 1302.04[2] as "HDR mode".
- Register double buffering: 1302.0C[15] = 1 disables it (writes take effect at once) — `EnableDblBuf(en)` writes `!en` and
  clears 1302.0C[1]; `TriggerDblBuf` (unreferenced) writes 1302.0C[0] = 1. ispmid's `CameraIsp_EnableIspRegShadow(mode, 0)` →
  `DrvIspEnableHwRegShadow` keeps it disabled; the per-frame programming relies on the CMDQ timing instead
  (isp-2-ispmid-mload-ispscl.md §7).
- ISP input mask (stops the ISP accepting input from VIF while its registers are rewritten; ispmid masks during set-up and while
  the frame batch is built, isp-2-ispmid-mload-ispscl.md §6.1): pipe 0 write 1302.0E[0], read-back 1302.0E[8]; pipe 1
  1302.0F[0] / [8]; `HalIspInputMask` returns the previous state.
- `HalIspVif_SensorMask` / `GetSensorMask` (`DrvIsp_Vif_*`): the VIF channel sensor mask (mhal-vif.md §3.3,
  `vif_chN_sensor_mask`).

### 5.5 Capabilities and chip facts [R]
<!-- ev:isp-1-hal.5.5 -->
| function | result |
|---|---|
| `DrvIsp_CheckChipCapability(w)` | eFuse words 101F.2E, 101F.26, 101F.2F, 101F.2C (via `HalIspReadRegWithBankAddr`; the fuse fields: ipl.md §7.2, §7.3): when the fuse bits say "restricted", widths > 3072 (0xC00) are refused; else any width. The 3840-wide stream opens on this camera, so the part is unrestricted [I from the live behaviour] |
| `CheckHdrCapability` | 1 |
| `CheckRotationCapability(r)` | r < 8 |
| `CheckLineBufferCapability(w, &max)` | max = 4208 (0x1070), w ≤ 4208 |
| `ReadDramId` | 1012.01[3:2] (DRAM type; 1 → larger MIU request sizes, §7.4) |
| `ReadResolutionId` | 101F.70[5:4], logged as "ISP chip ID" |
| `GetVpeCmdqFenceId` | reads 1120.78 (SCL/VPE CMDQ fence) |
| `IMI_ReservedSize` / `IMI_GetWidthLimit` | 0x16800 bytes / 2560 pixels (HDR line buffer in IMI SRAM, §7.6) |
| `HWBufferAllocSize` | 0x14480 (statistics DMA area, isp-2-ispmid-mload-ispscl.md §10.1) |
| `GetCmdqBuffSizePerFrame` | 0x8000 |
| `IsHwPost3DNR`, `NE_IsSupported` | 1 |
| `GetYuvPipeId`, `Vif_GetHdrChOffset` | 0, 1 |

[S] (SigmaStar Tiramisu MI and «Sensor Porting» pages): no page gives a pixel rate or a resolution × fps
ceiling for VIF, ISP or SCL; the only throughput statement is qualitative — a line or pixel rate too fast for the ISP gives
«isp fifo full» even when VIF receives the frame. [S] (SigmaStar FAE on the Comake developer chat, SSD268G, 2022-04): all sensors attached
together should not exceed 4K30 timing. [S] (SigmaStar FAE, 2023-01, SSD268G): without 3DNR and without an external
DRAM frame buffer the ISP path delays a frame by about one line. [S] (SigmaStar FAE, 2022-02, SSD268G): the ISP
has no fixed-pattern-noise correction — against the FPN API id 0x101A of isp-3-iq-api.md.

[R] (measured on this camera: the GC8613's 4K mode, its line length changed live, a frame of 2314 lines): the ISP takes
about one pixel per ISP clock plus ~80 clocks a line. A 3840-wide line runs clean at 9.12 µs and overflows at 9.02 µs with
`isp_clk` at 432 MHz; with 480 MHz it runs clean at 8.20 µs (52.7 fps) and overflows at 8.11 µs. An overflow is
`P0_FIFO_FULL` on every frame; the ISP recovers by itself once the line is long enough again. These limits hold with port
0's realtime link into SCL dev 0 idle (no SCL output port enabled) and with port 0 disabled and port 1 writing NV12
3840×2160 into memory: at 480 MHz port 1 then gives 52–53 frames a second, and a JPEG channel bound to it (FRAME_BASE)
encodes 51–52 fps with none lost. With an SCL output port running 3840 wide the scaler limits first at 480 MHz
(mhal-scl-jpe-cmdq.md §5), and 480 MHz on the ISP is then a little worse than 432 (beside HDMI 2160p and USB 4K: 9.30 µs
clean, 9.21 µs overflowing; the reason is not known).

### 5.6 What runs on this camera [I: assembled from the code paths of §3–§5, §10 and isp-2-ispmid-mload-ispscl.md §1.1]
- **Load**: banks 1302–134F mapped (§9); the driver thread started (§3.4); `CLK_isp` parent 1 (432 MHz) through the clk
  framework (§3.3).
- **Probe state** (`InitSwIfWdma`, §5.1): input masks on, all VIF→ISP routes off (1303.10–.17 = 0x0304), async-FIFO and
  SW-interface resets released, interface clocks on, WDMA dummy config and FBC off, RDMA / WDMA reset.
- **Camera init** (`InitHw` + `InitReg` + `EnableInt`, §5.1, §4.2): ISP reset released, pipes off, double buffer off
  (1302.0C[15] = 1), format-conversion bit, IQ enables cleared, RDMAs off, ACK-tie / async bits, statistics-block defaults, debug
  fence 10 = 0x5A5A, FBC v-sync select, ALSC kernel + R2Y defaults (`IQ_Init_Cfg`), 3DNR DMA defaults; interrupt masks from the
  table.
- **Open** (Bayer realtime): `InOutModeConfig` — 1302.14 = Bayer + order, 1303.30 / .31 = W − 1 / H − 1, 1302.1D = output
  line-count line, 1312.5D, 1302.16[0] = ACK tie, HDR off (1302.04[2] = 0, 1302.06[4] = 1, 131E.00[0] = 0); fence 6 := 0, fence
  7 |= 1 << ch; `PowerOn` (clocks 1302.04[0, 1, 5, 6], pipes 0 / 1 on, ISP→SCL 1302.03 = 3); raw store / fetch of DMA 0 off; 3DNR
  engines per level (§8.1); the ISP input unmasked (1302.0E[0] = 0). `HalIsp_ApplyIspBypassMode` is not called on this path
  (§10.4).
- **Each interrupt** (§4): the 7 groups read; frame start (1.12) and frame end (2.14) queue the frame-start and frame-done
  work; the rest counted; FIFO full recovered with reset 0x101 (ISP + WDMA of the pipe, §5.3); exactly the handled bits
  acknowledged; 4 / 1 / 2 returned to mi_isp (§4.6).
- **Each frame start** (isp-2-ispmid-mload-ispscl.md §6.1): input masked, the register batch built (digital gain §10.2, IQ
  shadow → `Apply<IP>` field tables of isp-4-iq-engine.md §10.3, menuload), input unmasked, the batch handed to the ISP's CMDQ behind
  the ISP_VSYNC wait (CMDQ-irq registers §6).
- Everything in §7, §8.2–§8.3, the external WDMA and the raw paths serves only frame mode, HDR, rotation, capture or ISP output
  port 1; none of it runs here.

## 6. Banks 1302 and 1303 field by field
<!-- ev:isp-1-hal.6 -->

Writers from other blocks are included and named: VIF (mhal-vif.md §4.5, §5), ISPSCL / MLOAD and ispmid
(isp-2-ispmid-mload-ispscl.md §6.3, §12, §13). "This camera" = the state here where the code path makes it certain [I unless the path is
quoted]. Fields not listed are never accessed by any code found. Fields marked [meaning unknown] are written with fixed values
and read by nobody; no M6 register reference that would name them is on hand [R].

### 6.1 Bank 1302 — ISP top [R writers / readers; names from log strings and function names; roles I]
<!-- ev:isp-1-hal.6.1 -->
| reg | bits | field (writer / reader) | this camera |
|---|---|---|---|
| 00 | 0 | pipe 0 enable (`ISP_EnableISP(0)`) | 1 after `PowerOn` |
| 01 | 0 | pipe 1 enable (`EnableISP(1)`; HDR second pipe) | 1 after `PowerOn` [I: enabled even without HDR] |
| 02 | 0 | rotation direction bit (`ISP_ROTDirection`: 1 for directions 2, 3, 4, 7) | 0 |
| 03 | 0, 1 | ISP→SCL output enables (`Isp2SclEnable`: both; ISPSCL YUV pipe: [1]) | 1, 1 (= 3 at `PowerOn`) |
| 04 | 0 / 1 | Bayer-top / pixel-top clock enable | 1 / 1 |
| 04 | 2 | HDR mode (`EnableISP_HDR`; shown by `isp_info`) | 0 |
| 04 | 3 | 3DNR engine clock (`DNREngineClockEnable`) | per 3DNR level |
| 04 | 4 | rotation engine enable (`ROTEngineEnable`; also the NE engine, §8.3) | 0 [I: no rotation] |
| 04 | 5 / 6 | DPC clock / WDR clock (`DpcClkEnable`, `WdrClkEnable`; `DrvIsp_SetWdrClk` per frame) | 1 / per IQ |
| 04 | 7 | set 1 by `InitReg` [meaning unknown] | 1 |
| 05 | 0, 1, 2 | interface clock pipes 0, 1, 2 (`InterfaceClkEnable`; ISPSCL YUV pipe [2]) | 1, 1, 1 ([2] = 1 at probe) |
| 05 | 8–11, 12 | set 1 by `InitSwIfWdma` / `InitReg` [meaning unknown] | 1 |
| 06 | 4 | HDR path off (1 when HDR disabled) | 1 |
| 08 | 0 | ISP soft reset, active low (`SWReset`, `IrqResetIp` 0x1 / 0x2) | 1 |
| 08 | 1 / 2 | 3DNR / rotation reset, active low | 1 |
| 08 | 3 / 4 / 5 / 8 | RDMA / DMA420 / menuload / RIU reset, active low | 1 |
| 09 | 0, 1, 2 | SW-interface resets 0–2, active low (`IrqResetSwIf`; ISPSCL [2]) | 1 |
| 09 | 3–6 | WDMA group 0–3 resets, active low (`WdmaReset`, `WdmaSwReset`, `IrqResetIp` 0x10 / 0x80) | 1 |
| 09 | 8–11 | async-FIFO resets of VIF groups 0–3, active low (`InitSwIfWdma`, VIF `GroupReset`, `IrqResetIp` 0x100–0x800) | 1 |
| 09 | 12 | reset pulsed by `InitReg` [meaning unknown] | 1 |
| 0C | 0 | double-buffer trigger, write 1 (`TriggerDblBuf`, unused) | — |
| 0C | 1 | cleared by `EnableDblBuf` | 0 |
| 0C | 2 | latch mode (`LatchMode`) | 0 |
| 0C | 15 | double buffer **off** (1 = writes take effect at once; also forced 1 around 131E / 130C writes) | 1 |
| 0E / 0F | 0 | ISP input mask pipe 0 / 1 (1 = masked; ispmid masks during set-up and while the frame batch is built) | 0 while streaming |
| 0E / 0F | 8 | input-mask status (read back by `HalIspGetInputMask`) | — |
| 10 | 0, 4, 8, 12 | raw-store "HS" pipes 0–3 (`RAW_Store_Mode` 0–3; `isp_info` "P<n> HS") | 0 |
| 10 | 1, 5 | raw-fetch "HF" = frame mode pipes 0, 1 (`RAW_Fetch_Mode`; `HalIsp_IsFrameMode` reads [1]) | set 1 by `InitSwIfWdma`; on a realtime open without HDR pipe ispmid's `CameraIsp_HdrPipeInit` runs `RawStorePipeDisable(0)` + `RawFetchPipeDisable(0)` → [1] = 0, [5] stays 1 [R path; `HalIsp_IsFrameMode` has no caller] |
| 11 | 3:0 / 7:4 | RDMA 0 / 1 auto-trigger mode 0…15 (`RdmaAutoTriggerModeSelect`) | — |
| 12 | 0, 4, 8, 12 | raw-store HS pipes 4–7 (`RAW_Store_Mode` 4–7) | 0 |
| 13 | 7:0 | raw-store selects 8–15 (`RAW_Store_Mode` 8–15) | 0 |
| 14 | 0 | input is Bayer / RGB (1) (`SenDataType`; `isp_info` "RGB" vs "YUV422") | 1 |
| 14 | 1 | YC order (`SenYCOrder`, YUV only) | — |
| 14 | 2 | input is YUV (1) | 0 |
| 14 | 3 | format conversion (`SenFmtConv(i)` → table[i] & 1, table `.rodata+0x788c` = {0,1,2,3,0,1,2,3}; init uses i = 7) | 1 |
| 14 | 9:8 | Bayer order (`SenBayerFmt`: id 0→0, 1→1, 2→3, 3→2, table `.rodata+0x7894`; `isp_info` BayerID) | the sensor's order |
| 14 | 15:12 | RGB-IR format (`SenRGBIRFmt`, from the statistics configuration) | 0 [I] |
| 16 | 0 | pipe-0 **ACK tie** to SCL (`ICPASync(0)` / `DrvIsp_ConfigAckTie1`; `isp_info` "PIPE0 ACK TIE") | from mi_isp's `ConfigAckTie1` |
| 16 | 1 | set 1 by `InitReg` [meaning unknown] | 1 |
| 16 | 4 | pipe-2 (YUV pipe) async; 1 = off (`ICPASync(2)`, `EnableISP(2, 0)`, ISPSCL) | 1 (at init) |
| 17, 19, 1A, 1B | 15:0 | FIFO max-valid level of pipe 3, 0, 1, 2 (`GetFifoMaxValid`; pipe 0 sampled every ISR) | — |
| 18 | 0–3 | async-FIFO k set after its reset (VIF `GroupReset` — VIF's AFIFO reset, `IrqResetIp`) | — |
| 1D | 12:0 | pipe-0 output line-count interrupt line (`SetIspOutputLineCountInt(0)`) | crop height |
| 1E, 1F | 12:0 | pipe-2 output line-count lines (n − 1) (`SetIspOutputLineCountInt(2)`, ISPSCL) | — |
| 20, 21[7:0] | — | debug output word, 24 bits (`GetDbgOut`) | — |
| 22 | 3:0 / 11:8 | debug output select (sub / main): main 3 SCL ack (bit 6 → `SclAck`), 9 rotation base (with 1310.20[2:0] = 5), 0xB misc-done (sub 2), 0xC input pixel count, 0xE output pixel count, 0xF output line count | restored after each read |
| 24 | 12:0 | sensor (input) line count (`GetSenLineCnt`, unused) | — |
| 27 | 12:0 | debug input line count (`GetDbgInputLineCnt`, sampled every ISR) | — |
| 28–43 | | interrupt groups 1–7 (§4.1) | §4.2 |
| 48 | 9 | WDMA busy, polled before a WDMA-group reset (`IrqResetIp` 0x80) [I name] | — |
| 50–53 | 15:0 | CMDQ-irq group A mask / force / clear / status: ids 0x80 → bit 5, 0x100 → 15, 0x200 → 14 (`HalIsp_Mask/Force/Clr/PollCmdqIrq`) | — |
| 54–57 | 15:0 | CMDQ-irq group B: ids 4 → bit 15, 8 → bit 13 (`IrqResetIp` 0x1 writes 54 = 0x7FFF, 55 = 0x8000) | — |
| 68, 69, 6A, 6B | 15:0 | written 0x0300, 0x0001, 0x00FF, 0x000F by `InitSwIfWdma` [meaning unknown] | as written |
| 70–79, 7B | 15:0 | debug fence ids 0–9, 11 (`ISP_Get/SetDebugFenceID`; 2 = batch counter, 6 = frame-mode flag, 7 = open-channel mask, 11 = 0x5A5A frame-mode marker — isp-2-ispmid-mload-ispscl.md §7.6). ISPSCL's init writes 1302.76 = 0x5757 and 1302.77 = 0x7A7A (fences 6 and 7); ispmid's `MhalCameraOpen` overwrites them (fence 6 := 0 for realtime, fence 7 \|= 1 << ch) [R], so the ISPSCL values live only until the first camera open | fence 6 = 0 |
| 7A | 15:0 | fence 10 = reset marker: 0 while `IrqResetIp` runs, 0x5A5A after (also `InitReg`) | 0x5A5A |
| 7C–7F | 15:0 | CMDQ index mode (ispmid `CameraIspCmdqBuffer_IndexMode_*`: 7D = slot \| 0xA500 on enable, 7C on slave kick-off) | unused (single channel) |

### 6.2 Bank 1303 — ISP top 2 / VIF routing [R]
| reg | bits | field (writer) | this camera |
|---|---|---|---|
| 01 | 0 | WDMA source mux = 7 (`ISP_WMUXSrc`, from `InitHw` and `ExtWdmaActConfig`) | 0 |
| 08–0B | 12:8 | FIFO virtual-channel enable of VIF group 0–3 (VIF `GroupSetConfig`) | 1303.08[12] = 1 |
| 10–17 | 1:0 | VIF→ISP pipe p: source group (VIF `PipeSetConfig`; `ISP_ConfigIspPipeSrcMux` for the test paths, values 0–3, ≥ 4 → 4) | 1303.10 = 0x0000 |
| 10–17 | 2 | pipe p disconnected | 0 for pipe 0, 1 for the others |
| 10–17 | 9:8 | channel within the group (ch & 3; ≥ 3 → 3) | 0 |
| 24, 25 | 12:0 | pipe-2 input width − 1, height − 1 (`InputImageResolution(2)`, ISPSCL YUV pipe) | — |
| 30, 31 | 12:0 | pipe-0 input width − 1, height − 1 (`InputImageResolution(0)`) | 3839 / 2159 for 4K [I] |
| 32, 33 | 12:0 | pipe-1 input width − 1, height − 1 | HDR only |
| 40 | 0 | IMI (internal SRAM) line buffer for HDR on (`ISP_IMI_Enable`, with 1308.78[13]) | 0 |
| 41 | 15:0 | YUV→Bayer H-down-sample enables / YC swap (ISPSCL) | 0 |
| 42 | 7:0 | IspScl CSC setting (ISPSCL) | 0x10 |
| 50–53 | 15:0 | CMDQ-irq group C mask / force / clear / status: ids 1 → bit 1, 2 → bit 7, 0x40 → bit 15; `IrqResetIp` 0x1 writes 51 = 0x0002 | — |
| 54, 55 | 15:0 | group D: mask always 0xFFFF, force 0 (`Mask/ForceCmdqIrq`) | 0xFFFF / 0 |
| 60 | 1 | status polled for CMDQ-irq id 0x10 (`PollCmdqIrq` waits on 1303.60[1]) | — |

CMDQ-irq id 0x20 is polled on **130D.0A[1]** (menuload busy, wait for 0) — the CMDQ can wait for menuload to finish [R
`HalIsp_PollCmdqIrq`]; `PollCmdqIrq(0x40)` is degenerate (register 0, mask 0). The trigger-bus event of an id
(`ISP_GetIpCmdsqTrigBusId`): ids with bits 0x73 → 0x1E (ISP_VSYNC), 0x0C or 0x380 → 0x19 (ISP_TRIG). How ispmid uses the
ids: isp-2-ispmid-mload-ispscl.md §7.5; the events: mhal-scl-jpe-cmdq.md §2.8.

## 7. DMA engines: RDMA, WDMA, FBC / FBD, external (YUV) WDMA, IMI — banks 1308, 130A, 130B, 130C, 1348, 1349

None of these run on the camera's realtime path except the reset / "off" writes of §5.1 and `RawStore/FetchPipeDisable(0)`
(§6.1); they serve frame mode, HDR (two-frame), raw capture / fetch, image store and the ISP output port 1
(isp-2-ispmid-mload-ispscl.md §8) [R call graph: every configuring caller is `SensorFrameDoneISR`, `CameraRawStore/RawFetch*`,
`CameraIsp_HdrPipe*`, `CameraIsp_Capture*`, `CameraISP_ImageStore*`].

### 7.1 Channel map [R `HalIsp_Init`, static helpers at `.text` 0x45cf4 (WDMA) and 0x45d8c (RDMA)]
| engine | register block (0x20 registers) |
|---|---|
| RDMA 0, 1 | 130A.00, 130A.20 |
| WDMA 0–3 (group 0) | 1308.00, .20, .40, .60; group common 1308.78 |
| WDMA 4–7 (group 1) | 130B.00, .20, .40, .60; common 130B.78 |
| WDMA 8–11 (group 2) | 1348.00, .20, .40, .60; common 1348.78 |
| WDMA 12–15 (group 3) | 1349.00, .20, .40, .60; common 1349.78 |
| external WDMA (id 7, `ISP_ExtWdmaGetId`) | bank 130C + crop in 133E.50–.54 |

Driver "node" numbers (static helper at 0x44244, used by every `DrvIsp_Raw*` / `DrvIsp_DMA_*` wrapper): node 1–3 → DMA n and
pipe n; node 4–15 → DMA n, pipe 0; anything else → 0 / 0. ispmid uses node 4 for the ISP output (port 1) and 7 for the IR
output (port 2, isp-2-ispmid-mload-ispscl.md §8).

### 7.2 RDMA block `R` (130A.00 / 130A.20) [R]
| reg | bits | field (function) |
|---|---|---|
| 00 | 0 | enable (`RDMAEnable`, `RDMAisEnable`) |
| 00 | 1 | v-sync source: 1 = generated from a WDMA line count, 0 = external (`RdmaVsyncFromLineOrExt`) |
| 00 | 2 | ring-buffer mode (`DMARingBufferEnable`, `IsEnable`) |
| 00 | 4 | auto mode (`RDMAAuto`) |
| 00 | 6 | double buffer, written `!enable` (`DMA_DBF_Enable`, both RDMAs together) |
| 00 | 7 | high priority (`RDMASetHighPri`) |
| 00 | 11:8 | precision code: 0/4 → 0, 1/5 → 1, 2/6 → 2, 3/7 → 3, 10 (FBC 12-bit) → 6 (`RDMADataPrecision`) |
| 00 | 13:12 | MIU select 0–3 (`RDMAMiuSel`) |
| 00 | 15 | reset pulse (`RDMAReset`) |
| 01 | 12:0 | pitch in 16-byte units (`RDMASetPitch`: `(pitch & 0xFFFFF) >> 4`) |
| 02 | 12:0 | ring-buffer size in lines − 1 (`DMASetRingBufferSize`) |
| 03 | 15:0 | fence id (`Set/GetRdmaFenceID`; ispmid writes `channel \| 0x8000` in frame mode) |
| 04 | 12:0 | width in units − 1 (unit 1 pixel for precision 0–3, 8 for code 10) |
| 05 | 12:0 | height − 1 |
| 06 | 5:0 | start offset / unit (7th argument of `RDMASetPitch`) [I: horizontal start] |
| 08, 09[11:0] | | frame 0 address: `addr >> 4` (bits 19:4), `addr >> 20` (`RDMABaseAddr`, `BaseAddrEx(…, 0)`) |
| 0C | 7:0 / 15:8 | MIU request threshold / burst count (`RDMAReqThd`, `RDMAReqBustNum`; `InitReg`: 0x7F / 0x20) |
| 0D | 5:0 | request blank (`RDMAReqBlank`; `InitReg` 0x3F) |
| 0E | 7:0 | high-priority threshold (`RDMAReqHighPriThd`) |
| 10 | 0 | software trigger, write 1 (`RDMASwTrig`) |
| 11 | 0 | busy (`RDMAisBusy`) |
| 12, 13[11:0] | | frame 1 address (`BaseAddrEx(…, 1)`) |
| 19 | 0 / 4 / 9:8 | current frame index / frame-index (ping-pong) mode / start line-count threshold 0–3 |
| 1A | 12:0 | FBD frame width − 1 (`DmaFbdFrameWidth`) |

The auto-trigger mode of RDMA 0 / 1 lives in 1302.11 (§6.1); 130A.00 also carries the RDMA part of `ISP_IrqResetIp`.

### 7.3 WDMA block `W` and group commons `X` [R]
| reg | bits | field (function) |
|---|---|---|
| W.00 | 0 / 4 / 7 | enable (`WdmaEnable`: via `ISP_RegWM`, or a direct byte store when the 4th argument is set) / auto / high priority |
| W.00 | 11:8 | precision code 0–7, 10 → 6 (`WDMADataPrecision`) |
| W.00 | 13:12 / 15 | MIU select / reset pulse (`WdmaReset`) |
| W.01 | 12:0 | pitch in 16-byte units: precision 0 → ⌈w/16⌉ (8-bit), 1 → w·10/128, 2 → ⌈w/8⌉ (16-bit), 3 → w·12/128, 4 → ⌈w/16⌉, 5 → ⌈w/64⌉·5, 6 → ⌈w/8⌉, 7 → ⌈w/32⌉·3, 10 → ⌈w/16⌉; max 0xFFF |
| W.02 | 12:0 | line-count interrupt line (`WDMASetLineCnt`); W.0B[12:0] a second one (`SetLineCnt2`) |
| W.03 | 0 / 5:4 / 9:8 | current frame index / frame-index mode / start line-count threshold |
| W.04 | 15:0 | width in units − 1 (unit 1 pixel for precision 0–3, 4 pixels for 4–7, 8 for 10) |
| W.05 | 12:0 / 15 | height − 1 / set 1 |
| W.08, W.09[11:0] | | frame 0 address (`addr >> 4`, `addr >> 20`); W.09[15] = 1 when the address is 0 [I: "no buffer"] |
| W.0A | 15:0 | line number at which this WDMA generates a v-sync for an RDMA (`WdmaGenVsyncLineToRdma`) |
| W.0C | 7:0 / 13:8 | request threshold (≤ 0x7F) / high-priority threshold (≤ 0x1F) |
| W.0D | 7:0 | burst count |
| W.0E, W.0F | | frame 1 address (`WDMABaseAddrEx(…, 1)`), W.0F[15] null flag |
| W.10 | 1, 0 / 8 | software trigger (write [1] then [0]) / auto-align |
| W.12 | 15:0 | "dummy config", 0x8010 for all 16 at probe [meaning unknown] |
| W.16 | 0 / 8 / 15 | busy / line-buffer full / status clear (write 1) |
| W(g·4+2).18–.1B | 15:0 | width − 1 of the four WDMAs of group g, kept in the third block of the group (`WDMASetPitch`) |
| X.78 | 7:4 | FBC (frame-buffer compression) enable of members 0–3 (`DmaFbcEnable`) |
| 1308.78 | 9:8 | FBD (decompression) enable of RDMA 0 / 1 (`DmaFbdEnable`) [naming of bit 8 against FBD: not settled] |
| 1308.78 | 13 | IMI line buffer (with 1303.40[0], `ISP_IMI_Enable`) |
| X.79 | 1:0, 5:4, 9:8, 13:12 | sensor-data precision of members 0–3 (`SenDataPrecision`, value & 3) |
| X.79 | 2, 6, 10, 14 | "4-pixel pack at MSB" of members 0–3 (`4Pixel_Data_Pak_AtMsb_To_Wdma`; set for raw formats 4, 5, 7, 10 [R mask 0x4B0]) |
| 1308.7E | 15:0 | WDMA FIFO arrangement, eight 2-bit fields: mode 0 = 0,1,2,3,0,0,0,0; mode 1 (HDR pipe) = 0,2,1,3,0,1,0,3 (`WdmaFifoConfig`) |
| X.7F | 5:4 | FBC v-sync select (`DmaFbcVsyncSelection`; `InitReg`: 3 in all four groups) |

`ISP_DmaCompressEnable` is an empty stub. Raw-store format codes are re-mapped by the table `.rodata+0x773c` = {4, 5, 6, 7, 6, 6,
6, 6, 6, 6, 10} before use (`RawStorePitchAddrConfig`).

### 7.4 Driver sequences [R]
- **Raw store** (`DrvIsp_RawStorePipeConfig(ctx, node, {w, h, fmt, on, auto, thd, burst})`): 4-pixel pack, WDMA off (node 1:
  reset), pitch, `RAW_Store_Mode` (the 1302.10 / .12 / .13 bit of the DMA), precision, priority 0, thresholds (thd, 0xF), burst,
  auto, status clear, enable. `RawStorePipeDisable`: off, threshold 0x10, burst 0x20, store mode 0. `RawStorePitchAddrConfig`
  (HDR capture): FBC for format 10, pitch, precision, then a single address + ring off, or ping-pong addresses. `UpdateAddr` /
  `Trigger` set the address / software trigger.
- **Raw fetch** (`RawFetchPipeConfig(ctx, node, {pitch, w, h, fmt, …, bayer, yc, yuv, thd?, burst?, auto, vsync line, ext vsync,
  trigger mode})`): RDMA off, pitch, fetch mode (1302.10 HF bit), Bayer order / data type / YC order of the pipe, optional
  threshold and burst, auto, v-sync line and source, auto-trigger mode, interface clock, enable. `RawFetchPipeDisable`: off,
  fetch mode 0. `RawFetchPitchAddrConfig` (HDR): FBD for format 10, pitch, ping-pong mode with both addresses, ring size (h or
  h/8).
- **MIU request parameters by DRAM type** (`HalIsp_*_GetMiuReqParam`, 1012.01[3:2] = 1 vs other): RDMA threshold / burst 0x40
  (0x28 for 1280×720) vs 0; WDMA 0x1E / 0x40 vs 0x10 / 0x20; rotation 0x40 vs 0x20; motion RDMA 0x40.

### 7.5 External WDMA — ISP output to memory (bank 130C) [R `HalIsp_ExtWdma*`, `DrvIsp_ExtWdmaActConfig`]
`DrvIsp_ExtWdmaActConfig(ctx, w, h, x, y, mode, addrY, addrC, …)`: mode 7 (and `IsExtWdam420`, constant 1) → the YUV420 / 422
writer of 130C (`HalIsp_ExtWdmaActConfig`); other modes reuse WDMA 7 as a raw writer (address, line count h − 1, pitch, store
mode, WDMA mux source `{2→1, 3→2, 4/10→3, 5→4, 6→5, 7→6, 8→7}` into 1303.01[0] (only 7 sets it), priority 1, thresholds 0xB /
0xF, burst 0x20, auto, enable, trigger; mode 8 also enables the RGB-IR monitor). `ExtWdmaActDisable`: mode 7 → disable 130C,
else WDMA 7 off.

130C fields:

| reg | bits | field |
|---|---|---|
| 20 | 15 | enable (`HalIsp_IsExtWdamEnable` also reports enabled when the line counter .44 ≠ 0) |
| 20 | 1:0 | MIU = 0 |
| 20 [14], 22 [2:0], 50 [4] | | output format ("domain") bits from the format code {1 → 4, 2 → 8, 3 → 0, 4 → 2, 5 → 0x10, else 1} |
| 23, 3D, 40, 41[7:0] | | 130C.23 = 0x3434, 130C.3D = 0x3434, 130C.40 = 0x3C3C, 130C.41 = 0x7C (MIU request parameters [I]; meaning unknown) |
| 24 / 25[12:0] | | Y plane address in 8-byte units (`(a & 0x7FFFF) >> 3`, `a >> 19`) |
| 26 / 27 | | C plane address |
| 28 / 29 | | third plane = Y + Y size |
| 3C | 4:3 | 0 |
| 3E / 3F | | width / height |
| 43 | 7 | 1 |
| 44 | | output line count (read; also sampled by the ISR) |
| 4A | | 5 |
| 4B | | height |
| 4C / 4D | | pitches (full for formats < 5, else /2) |

Crop on the ISP side in 133E: .50[0] crop on, .50[2:1] option [meaning unknown], .51 x, .52 w − 1, .53 y, .54 h − 1 (13 bits
each). 133E.10[10] = "format flag ≠ 0" at configure, cleared at disable. Plane sizes by format (`GetDomainSize`): 1 → Y = C =
w·h; 2 → C = w·h/2; 3 → C = w·h/2, third 0; 4 → C = w·h/4; 5 → Y = 4·w·h; else Y = 2·w·h.

### 7.6 IMI line buffer (HDR) [R]
`ISP_IMI_Enable(on, addr)`: on and `addr < 0x100000` → 1308.78[13] = 1, 1303.40[0] = 1, then `DNRDataArrangeMode(1)`; off →
both 0. Size reserved by ispmid: 0x16800 bytes, width limit 2560 (§5.5). The IMI heap itself: mhal-core.md §3.5.

## 8. 3DNR, rotation, motion and noise-estimate engines, WDR helpers

### 8.1 3DNR (temporal NR) — banks 1328, 132B, 132C, 132D, with 1326 (2DNR) [R]
Bank roles: 1328 = 3DNR core control (+ IQ fields, isp-4-iq-engine.md §12.3), 132B = its DMA addresses / compression, 132C = its RDMA
(reference + history read) geometry, **132D = its WDMA (history write) geometry** (only `ISP_DNRSetWdmaConfig` touches 132D, as
HAL slot 0x19), 1326 = Bayer 2DNR (BNR) whose enable is switched together with 3DNR.

| reg | bits | field (function) |
|---|---|---|
| 1328.00 / .01 | 11:0 | input width / height (`DNRSetInputImageResolution`, clamp 0xFFF) |
| 1328.02 | 1:0 | Bayer order: id 1 → 1, 2 → 3, 3 → 2, else 0 (`DNRSetBayerFmt`, table `.rodata+0x7a1b`) |
| 1328.03 | 0 | 3DNR enable (`DNREnable3DNR`, `DNREngineEnable`) |
| 1328.04 | 0 | first-frame trigger, write 1 (`ISP_DNRTrigger1stFrame`, `.text` 0x4aca2); also the IQ shadow bit +0x2410 copied by `HalIsp_Apply3DNR` |
| 1328.05 | 1 / 0 | data arrangement: [1] = mode ≠ 0 (also set by `IMI_Enable`), [0] = mode = 0 (`DNRDataArrangeMode`) |
| 1328.11 | 6:0 / 13:7 | 7 / 2 (`InitReg`) [meaning unknown] |
| 1328.12 | 7:0 | 2 (`InitReg`) [meaning unknown] |
| 1328.1D / .1F | 5:0 / 7:6 | debug select (0x27 = ring index) / readback (`GetDbg3DNRRingIndex`, restores the select) |
| 1328.3C | 1:0 | pre-history down-sample select 1–3, else 0 (`DNRPreHistDsSelect`) |
| 1326.00 | 0 | 2DNR enable (`DNREnable2DNR`; on with 3DNR in `DNREngineEnable`, forced 1 by the first-frame trigger) |
| 1326.11 | 5:0 | 0 at the first-frame trigger |
| 132B.01 | 13:12 / 15:14 | RDMA / WDMA MIU select (`DNRRdmaMiuSel`, `DNRWdmaMiuSel`) |
| 132B.05 | 0, 1 | "PV" enable, both set (`3DNR_PV_Enable`) |
| 132B.06 / .07 | | history frame address (`addr >> 4`, `addr >> 20`) |
| 132B.08 / .09, .0A / .0B | | reference frame addresses 0 and 1 (`DNRSetReferenceFrameAddress`) |
| 132B.0D | 8:0 | compressed line size: mode 0 → ⌊(w+9)/10⌋, mode 2 → ⌈w/16⌉ (`DNRCompressBitDepth`) |
| 132B.30 | 0 | compression mode 2 (1) / 0 |
| 132C.51 | 12:0 | h/2 − 1 (RDMA) |
| 132C.52 / .56 | 12:0 / 10:0 | reference line size: compression 2 → ⌈w/8⌉, else ⌊(w+4)/5⌋ [R: `((w+4)·0xCCCCCCCD >> 32) >> 2`] |
| 132C.54 / .55 | 7:0 | RDMA parameter p8 − 2 / p7 |
| 132C.61 | 12:0 | h − 1 |
| 132C.62 / .66 | 12:0 / 10:0 | history line size: `(w' + 31) >> 5` with w' = w (mode 0), w/2 + 1 (1), ⌈w/4⌉ + 1 (2), ⌈w/8⌉ + 1 (3) |
| 132C.64 / .65 | 7:0 | 0x16 / 0x13 (`InitReg`) [meaning unknown] |
| 132D.52 / .56 | 12:0 / 14:0 | WDMA reference line size (as 132C.52) |
| 132D.53 / .54[15:8] | 7:0 | WDMA parameters p7 / p8 − 2 |
| 132D.62 / .66 | 12:0 / 14:0 | WDMA history line size |

Driver: `DrvIsp_DNREngineInit(ctx, ch, cfg)` calls, in order, input size, Bayer order, compression, arrangement, block width /
DRAM row mode / ring mode / block count (all four **empty stubs**), history and reference addresses, pre-history down-sample,
RDMA and WDMA geometry; `cfg` = {u16 w, u16 h, int bayer @+4, int compression @+8, int arrangement @+0xc, block width @+0x10, row
mode @+0x14, history addr @+0x18, reference addrs @+0x1c / +0x2c, u8 ring @+0x3c, pre-hist @+0x40, u8 RDMA p7 / p8 @+0x44 /
+0x45, WDMA @+0x46 / +0x47}. `DNREngineEnable(on)`; `DeInit` = enable 0; `DNRMIUSel` both selects; `DNREngineClockEnable`
1302.04[3]; `DNREngineParallelMode`, `DNR_SelectPosition` stubs.

**First-frame trigger** [R; purpose I]: `ISP_DNRTrigger1stFrame` writes 1328.04[0] := 1, 1326.00[0] := 1 (2DNR on) and
1326.11[5:0] := 0 (`ISP_RegWM` byte offsets 8, 0, 0x22 = registers 0x04, 0x00, 0x11). "First frame" = the 3DNR starts its
temporal history afresh instead of blending with the stored reference [I from the name]. `DrvIsp_DNREngineSoftwareResetFor1stFrame
(h, 0, mode)`: mode ≠ 0 → `ISP_DNRTrigger1stFrame`, mode 0 → `ISP_DNREngineSWReset` 1 then 0 (pulse 1302.08[1]). Its only caller
is an unnamed ispmid helper (`.text` 0x7fa94, `isp_3dnr_set_level(level)` (0x4007fabc)): `Camera3DNR_GetConfigByLevel(level, &en, …)` (en
defaults to 1), then the call above with mode = en; ispmid runs it in `MhalCameraOpen` (level 0 before `Camera3DNR_Init`, then
the configured level after it) and in `CameraContextSwitchIspPipeActive` (frame-mode pipe re-activation, when ch+0x19bbc = 1 and
ch+0x19b68 ≠ 0). The IQ engine pulses the same bit through its shadow whenever the temporal filter is switched on again
(isp-4-iq-engine.md §10.2). The HAL never writes 0 to 1328.04[0]; whether the hardware bit clears itself is not known.

Buffer sizes (ispmid allocates them): reference `HalIsp_3DNR_CalRefBufSize(w, h, mode, compressed)`: mode 1 → h·align16(w)
(uncompressed) or ⌊(w+9)/10⌋·h·16; else ⌈w/128⌉ (or ⌈w/80⌉ compressed) · ⌈⌈h/4⌉/32⌉ · 0x4000; history `CalHistBufSize(w, h,
mode)` = h · ⌈w'/32⌉ · 16 with w' as above.

### 8.2 Rotation engine — bank 1310 (+ 1302.02 / .04 / .08) [R]
| reg | bits | field |
|---|---|---|
| 1310.00 | 0 | rotation RDMA enable (`ROTRDMAEnable`) |
| 1310.03 | 0 / 6:4 | ring mode / ring count − 1, 0–3 (`ROTSetRDMARingMode(on, n)`) |
| 1310.04 | 1:0 | bit depth: 8-bit → 1, 10-bit → 0 (`ROTBitDepth`) |
| 1310.05 | 1:0 | direction field: 2 for directions 2, 5; 1 for 3, 6; 0 otherwise (`ROTDirection`, with 1302.02[0]) |
| 1310.08–.0F | | four RDMA buffer addresses (`addr >> 4`, `addr >> 20`; one or four — `ROTSetRDMABaseAddr(addrs, ring)`) |
| 1310.11 | 15:0 | RDMA horizontal blanking (`ROTRdmaHBlanking`, ≤ 0xFFF) |
| 1310.13 | 9:0 | line size: 8-bit ⌈w/8⌉, 10-bit ⌊(w+4)/5⌋ |
| 1310.1C / .1D | 6:0 / 15:0 | MIU request number 1–64 / request cycle |
| 1310.1F | 12:0 | output line count (read; logged on `ROT_ERR_TRIG`) |
| 1310.20 | 2:0 | debug select (5 = base address, `GetDbgRotBaseAddr`) |

Rotation codes (mi_isp's `RotationConfig`, isp-2-ispmid-mload-ispscl.md §3.2): the engine is on when `(code & ~4) != 0`
(`ISP_RotEngineIsEnable`); 1302.02[0] = 1 for codes 2, 3, 4, 7. `DrvIsp_ROTEngineInit(ctx, cfg)`: direction; if on: input size
(**stub**), bit depth, arrangement / block width / row mode (stubs), base addresses, MIU number / cycle; ring mode; engine
(1302.04[4]) and RDMA enable. `DeInit` reverses.

**The frame-period adaptation is dead**: `ISP_ROTSetFramePeriod(h, period, w, hgt)` would set the h-blank to `(period·f_isp/w −
hgt)/2` (4…0xFFF; f_isp from `HalClkgenGetFreq` in MHz), but the only caller `ISP_IrqRotProcFramePeriod` (§4.5) takes w, h from
`ISP_ROTGetInputImageResolution`, a stub returning 0, so it returns before writing [R].

### 8.3 Motion and noise-estimate (NE) engines — bank 1310.28–.3C [R]
| reg | bits | field |
|---|---|---|
| 1310.28 | 0 / 4 | motion enable (`MotionEnable`) / NE enable (`NE_Enable`) |
| 1310.30–.37 | | motion buffer addresses: two (read, write) or a ring of four (`MotionSetBaseAddr`) |
| 1310.39 / .3A | 6:0 / 15:0 | motion WDMA MIU request number / cycle |
| 1310.3B / .3C | 6:0 / 15:0 | motion RDMA request number / cycle |

`DrvIsp_MotionEngineInit(ctx, cfg)` programs addresses and MIU parameters only when the rotation engine is on for `cfg[1]`, then
`MotionEnable(cfg[0])`; buffer size `((w/2+8)/8 + 15)/16 · ((h+7)/8 + 31)/32 · 0x4000`. `DrvIsp_NE_EngineInit`: NE is supported;
with rotation off and `cfg[0]` set it uses the rotation engine as carrier — ring mode (0, 1), 1302.04[4] = 1, NE on; else NE off.

NE buffer: 0x240 bytes (12 records × 48 bytes). `ISP_DNRReadIQNoiseEst(h, ch, out, buf)` unpacks it (12 × {u16, u32, u16, u32,
u16, …} spread over six parallel arrays of `out`; the record layout per colour channel: isp-4-iq-engine.md §9.2) and appends
1321.7E / 1321.7F; ispmid runs it at every frame start (isp-2-ispmid-mload-ispscl.md §6.1). **1321.7E** (copied to ch+0x19e7c
each frame start) is not only a noise-estimate readout: it is the IQ engine's debug command word and ispmid's 0xdb freeze code
(isp-4-iq-engine.md §11.1); its value on the running camera has not been read (expected 0 [I], CAMERA-CHECKS K18).
`ISP_HDRReadDynamicAlignEst`: 16 values of 11 bits from 131E.1B–.2A (HDR alignment).

### 8.4 WDR helpers — bank 1322 [R]
1322.00[5] multi-frame WDR (`WDRMultipleFrameEnable`); 1322.0B / .0C [12:0] global width − 1 / height − 1
(`WDRSetWdrGlobalSize`); 1322.7E / .7F WDR menuload buffer address (`addr >> 4`, `addr >> 20`; `WDRSetWdrMloadBuffer`, set at open
with the "ISP_WDR" buffer); `WDRSetWdrLocalSize/EX` stubs. `ISP_SaveDebugValue(v)` → 1321.7F (unreferenced). WDR clock 1302.04[6]
(`DrvIsp_SetWdrClk`, per frame from ispmid).

## 9. Register bank map (banks 0x1302–0x134F)
<!-- ev:isp-1-hal.9 -->

Addressing as in the conventions: bank B at PA `0x1F000000 + B·0x200`, CPU VA `0xFD000000 + B·0x200`, 16-bit register r at
+4·r. The ISP uses **54 banks in 0x1302–0x134F**, all ioremapped from the 4 MiB RIU window by `HalIsp_Init` into the 0x150-byte
HAL handle (84 word slots, 82 of them bank pointers; some slots point at a 0x20-register sub-block inside a bank) [R]. ISPSCL maps 19 of these banks, VIF's WDMA helper 7. The DT
gives only `isp banks = <0x1302>` and IRQ GIC SPI 0x39 (ISPSCL: 0x71). Known slot numbers: 0x19 = 132D, 0x3c / 0x3d = 130E /
130F, 0x3e / 0x3f = 134A / 134B; 130D is in the handle at byte +0x100 [R].

**Key to the IQ code** [R]: in the IQ engine's register map (`Regmap`, 1072 entries of {u8 bank code, u8 register, u32 mask},
indexed by the IQ cfg enum) the bank code is **RIU bank − 0x1300**, and a fixed switch in `WriteRegs` gives each bank code its
own 0x200 slot of the shadow image (e.g. CCM: code 0x38 ↔ bank 1338 ↔ shadow +0x3E00, exactly the range `HalIsp_ApplyCCM`
copies; DECOMP: shadow +0x400 ↔ 1311; NR3D: +0x2400 ↔ 1328). The same table exists in Mercury5 source form (`iq_reg_map.h`,
§12) with other bank numbers. `Regmap` and the per-IP field tables: isp-4-iq-engine.md §12.1, §10.3.

Columns: "IQ fields" = `Regmap` entries per bank [R]; "written by" = `HalIsp_Apply<IP>` functions writing the bank [R] and `ISP_*`
functions that pick the bank from the handle [R]; IP names are SigmaStar's, their function in plain words
is [I] unless a section of this document specifies the bank.

| bank | IQ fields | written by (apply / HAL) | role |
|---|---|---|---|
| 1302, 1303 | — | most `ISP_Sen*`, crop, RDMA / WDMA HAL; VIF (1302.09 / .18 AFIFO reset, 1303.08 / .10 FIFO VC and pipe routing); ISPSCL | ISP top: enables, clocks, resets, input format, Bayer order, line-count interrupts, pipe mux, interrupt controller (1302.28–.43), CMDQ-irq registers, debug fences, CMDQ index registers 1302.7C–.7F — §6 |
| 1308 (+.20 / .40 / .60 / .78), 130B (+.20 / .40 / .60 / .78), 1348 (+sub), 1349 (+sub) | — | `ISP_Dma*`, `ISP_WdmaFifoConfig`, `ISP_DMA_DBF_Enable` | the 16 WDMAs, four groups × four, group commons at +0x78 — §7.3 |
| 130A (+.20) | — | `ISP_RDMA*`, `ISP_Dma*` | the two RDMAs — §7.2 |
| 130C | — | `HalIsp_ExtWdma*` | external YUV WDMA — §7.5 |
| 130D | — | MLOAD block; this layer only reads 130D.0A[1] (§6.2) | menuload engine — isp-2-ispmid-mload-ispscl.md §12.2 |
| 1309 (+.10 … +.70) | — | ISPSCL; this layer only reads 1309.(0x10k)[0] (unit on) in `ISP_IrqResetIp` 0x80 | the eight YUV→Bayer converters, one 16-register unit per VIF pipe — isp-2-ispmid-mload-ispscl.md §13.3 |
| 130E, 130F | — | none: mapped (also by ISPSCL), never read or written by hal/driver, statistics, ISPSCL or MLOAD | unused |
| 1310 | — | `ISP_ROT*`, `ISP_Motion*`, `ISP_NE_*` | rotation, motion and noise-estimate engines — §8.2, §8.3 |
| 1311 | 17 | `HalIsp_ApplyDECOMP` (17 fields 1311.00–.10 from shadow +0x400; field list in); bypass clears 1311.00[0] | **DECOMP** (HDR decompanding / knee curve) |
| 1312 | 10 | OBC, OBC_b, WBG; `ISP_IQEnable`, `ISP_AE_SetDgain/_Post`, line-count IRQs 0–2 | optical black clamp, WB gain, digital gain, IQ enables — §10 |
| 1313 | 10 | OBC_P1; `ISP_AE_SetDgain_P1` | second-path (P1) OBC / gain |
| 1316 | 56 | DPC, SDC | defect pixel correction, static defect table |
| 1317 | 23 | SpikeNR, ANTICT | spike noise reduction, anti-crosstalk |
| 1318 | 12 | RGBIR; `ISP_RGBIR_*` | RGB-IR separation + its statistics |
| 1319 | 11 | LSC | lens shading (radial) |
| 131A, 131C | — | `ISP_AF_*` (filters, windows, Y map, LDG) | AF statistics engine |
| 131B | — | `ISP_AE_*`, `ISP_AWB_*`, `ISP_HISTO_*`, `ISP_IQ_Init_Cfg`, `HalIsp_InitReg` | AE / AWB block statistics + histograms |
| 131D | 12 | ALSC; `ISP_IQ_Init_Cfg` (131D.45–.47 kernel, §10.3); `ISP_IQEnable` 0x16 (131D.40[0]); `ISP_EnableISP_HDR` listed for 131D by our function-to-bank scan while its decompile writes 131E.00[0] (§5.4) [R both, not reconciled] | adaptive (grid) lens shading |
| 131E | 67 | HDR, WBG; `ISP_HDRReadDynamicAlignEst`, HDR line counts (131E.78 / .79), `ISP_EnableISP_HDR` (131E.00[0]), WB copy 131E.66–.72 | HDR fusion |
| 1320 | 5 | GAMA16to12 | 16→12-bit gamma |
| 1321, 1322 | 12, 35 | WDR_LOC; `ISP_WDR*` (1322), `ISP_DNRReadIQNoiseEst` (1321.7E / .7F) | local WDR (tone mapping) — §8.4 |
| 1325 | 33 | DM | demosaic |
| 1326 | 10 | BNR; `ISP_DNREnable2DNR` | Bayer 2D noise reduction |
| 1327 | 31 | PFC, WBG | purple-fringe correction (+ WB copy, §10.2) |
| 1328, 1329, 132A, 132B, 132C | 35, —, 8, 22, — | 3DNR; `ISP_DNR*` (config, history / reference frame addresses, compression, MIU) | temporal (3D) noise reduction + its DMA — §8.1 |
| 132D | — | `ISP_DNRSetWdmaConfig` only | 3DNR history / reference **WDMA** geometry — §8.1 |
| 1330, 1331 | 22, — | GNR | another NR stage (GNR) |
| 1333, 1334, 1335 | 60, 27, 25 | YEE | luma edge enhancement (sharpness) |
| 1338 | 18 | CCM, HSV | colour matrix, hue / saturation |
| 1339 | 16 | GammaC2A, R2Y; `ISP_IQ_Init_Cfg` (R2Y defaults, §10.3) | RGB gamma, RGB→YUV |
| 133A | 31 | YEE, SIIR | sharpness (cont.), SIIR filter |
| 133D | 21 | XNR, UVM | chroma NR, UV matrix |
| 133E | 42 | ADJ_UV, MXNR, YUV_GAMA, YCUVM10, COLOR_TRANS; ext-WDMA crop 133E.50–.54 | YUV-domain adjustments, YUV gamma, colour transform |
| 1342 | — | ISPSCL, VIF; this layer pulses 1342.45[k] (unit reset) and 1342.46[k] (when set) in `IrqResetIp` 0x80; their meaning beyond "per-unit bit, pulsed", and whether anything else rewrites them under the vendor stack, is not known (no register dump under the vendor stack exists [S CAMERA-CHECKS K23]) | 1342.00–.17 = six VIF-WDMA interrupt groups; 1342.40 / .41 / .45 / .46 per-unit YUV→Bayer / IspScl bits — isp-2-ispmid-mload-ispscl.md §13.6, mhal-vif.md §4.5 |
| 134A, 134B | — | none: mapped (also by ISPSCL), never accessed | unused |
| 134C–134F (+.40) | — | ISPSCL | the eight IspScl scaler / CSC units — isp-2-ispmid-mload-ispscl.md §13.4 |

The IQ blocks without a working apply function have 4-byte stub apply functions in this build; their tables go through
menuload (§10.4).

## 10. IQ enables, gains, default tables, bypass and shadow helpers

### 10.1 IQ sub-block enables — `ISP_IQEnable(h, id, on)` [R switch at `.text` 0x47062]
| id | register(s) | IP [I name] |
|---|---|---|
| 0 | 1312.00[0] and 1313.00[0] | OBC (both paths) |
| 1, 3 | 1312.20[0] := 0, 1313.20[0] := on | path-1 select [I] |
| 2 | 1312.20[0] := on, 1313.20[0] := 0 | |
| 4 | 1319.00[0] | LSC |
| 6 | 131B.00[0] and [4] | statistics block enables |
| 7 | 1316.00[0] | DPC |
| 8 | 1317.20[0] | ANTICT / SpikeNR group |
| 0x11 | 1338.00[0] | CCM |
| 0x12 | 1339.10[0] | R2Y / gamma group |
| 0x16 | 131D.40[0] | ALSC |
| 5, 9–13, 0x10, 0x15 | nothing (return 0) | |
| others | return −1 | |

`DrvIsp_InitHw` calls it with ids 0, 4, 5, 7, 8, 9, 10, 11, 13, 14, 17, 19, 20, 21, 22 and `on = 0`; ids 14, 0x13, 0x14 hit the
−1 branch, so the effective clears are OBC, LSC, DPC, 1317.20, CCM, ALSC (17 = 0x11 CCM, 22 = 0x16 ALSC). The IQ engine later
turns the blocks on through the shadow (`HalIsp_Apply*`, isp-4-iq-engine.md §10.3).

### 10.2 White-balance and digital gains [R]
`ISP_IQ_WBG_Cfg(h, cfg, …)` — called by `DrvIsp_ApplyDGain` (per-frame digital-gain work) and by `HalISP_ApplyShadow(…, 1)` with
the shadow's WBG record (+0x4600). `cfg`: u16 @0 mode bit, u8 / u16 @4 set selector, ints @8 / @0xC / @0x10 / @0x14 / @0x18 /
@0x1C / @0x20 / @0x24 = {gain, offset} × 4 channels, u8 @0x28 "unity" (gains 1.0).
- Set select: 1312.30[0] = selector, 1312.40[0] = (selector == 0); the fields below go to set A when the selector is ≠ 0, else
  to set B.
- Set A fields (table `.data 0x3e94`): 1312.30[1] = mode bit; gains [12:0] (≤ 0x1FFF) / offsets [15:0] (≤ 0x7FF) of channels 0–3
  in 1312.31 / .32, .34 / .35, .3A / .3B, .37 / .38. Set B (`.data 0x3eb8`): 1312.40[1]; .41 / .42, .44 / .45, .4A / .4B, .47 /
  .48. Unity: gains 0x400 (= 1.0, u3.10 [I]), offsets 0.
- Copy for PFC (purple fringe, 1327): 1327.08–.0B = gains of channels 0, 1, 3, 2; 1327.0C–.0F = 2²²/gain (inverse, ≤ 0x1FFF);
  unity: 0x400 and 0x1000. In unity mode the real gains go to HDR instead: 131E.66–.69 and 131E.6F–.72 = gains 0, 1, 3, 2
  (≤ 0x3FFF).

Digital gain (`DrvIsp_ApplyDGain(ctx, wbg, on, gain, gainP1, gainPost)`): `ISP_AE_SetDgain` 1312.10[0] = on, 1312.11 =
gain[15:0], 1312.12[1:0] = gain[17:16]; `SetDgain_P1` 1313.10 / .11 / .12; `SetDgain_Post` 1312.60 / .61 / .62 (both always
enabled). Line-count interrupts 0 / 1 / 2 of the IQ path: 1312.5D / .5E / .5F [12:0] (`SetLineCountInt0/1/2`; only 0 is used,
§5.2). Who supplies the AWB gains per frame (`CameraSetAwbParam`, ch+0x146): isp-2-ispmid-mload-ispscl.md §1.1, §2.2.

### 10.3 `ISP_IQ_Init_Cfg` — fixed defaults loaded at init (`.text` 0x4d9e0, from `HalIsp_InitReg`) [R]
1. **131D interpolation kernel.** With the 131D base, for pass p = 0…3: 131D.47[8] := p & 1 (direct byte write); then for i =
   0…129: 131D.46 := high u16 of word i, 131D.45 := low u16, 131D.47[7:0] := i, 131D.47[9] := 1; after the odd passes
   131D.47[10] := 0. The literal pointers used on even and odd passes (`.text` 0x4dab4 and 0x4dabc) both relocate to
   `.rodata+0x7aa8`, so every pass loads the same 130 words. 131D.47[10] is never set to 1 here. The meaning of bits 8, 9, 10 is
   not named anywhere in mhal [I: 8 = which of two tables / banks, 9 = write strobe, 10 = SRAM access mode; the four passes fill
   the two banks of a double-buffered table twice].
   - **What the table is** [R values; meaning I]: each word = four bytes t0…t3 (t0 = bits 7:0); t0–t2 unsigned, t3 signed. For
     **every** entry 0…128, t0 + t1 + t2 + t3 = 256 exactly; entry 129 is 0. t0 falls monotonically 54 → 8, t1 151 → 121, t2
     rises 54 → 120, t3 goes −3 … 7. Entry 0 = (54, 151, 54, −3) (symmetric about t1); entry 127 = entry 128 = (8, 121, 120, 7)
     (symmetric about the t1 / t2 midpoint).
   - So it is a **4-tap polyphase interpolation kernel in 1/256 units, 129 phases**: taps at positions −1, 0, +1, +2 around the
     sample, phase k = k/256 of a cell from 0 to ½ (the symmetric half-cell phase is k = 128); the second half of a cell is
     presumably served by mirroring [I: only half the phases are stored and phase 128 is symmetric].
   - Shape: smoother than a cubic B-spline — at phase 0 the neighbours get 54/256 each (B-spline 42.7) and the node 151/256
     (B-spline 170.7); at the half phase 8/121/120/7 (B-spline 5.3/122.7/122.7/5.3). It does not reproduce the grid values at
     the nodes (centre weight 151/256 at phase 0) [R values].
   - Role [I: bank + kernel shape; no name or register field in mhal]: in bank 131D (ALSC: enable, offset, grid size / pitch
     131D.40–.44, scale.4a /.4b, cell-size LUTs.50… /.60…) the gain tables themselves reach the hardware by menuload (SRAM 2,
     isp-2-ispmid-mload-ispscl.md §12.3); this second, register-indexed SRAM holding a normalised kernel is the **coefficient table the
     ALSC block uses to interpolate its gain grid to pixel positions**. It is not a gain or curve table.
   - It is fixed: `ISP_IQ_Init_Cfg` is its only writer — no IQ-file parameter, API or `Regmap` entry touches 131D.45–.47 [R:
     `Regmap`'s 131D fields are.40–.44,.4a,.4b,.50…,.60…; the other ISP functions writing register offsets 0x45–0x47
     (`ISP_AWB_SetWin`, `ISP_AF_SetLdg`) do so on other banks (`[r4+0x28]`, `[r5+0x2c]`)].
   - Unused part: entry 129 (all zero) is written but lies past the half-cell phase 128 [R]; index values 130…255 of the 8-bit
     index are never written [R].
2. **R2Y defaults**: `ISP_RegValueTableWM(1339,.data 0x3e10, 22 entries of {r, mask, value})`: 1339.10[0] = 1, 1339.13[0], [4] =
   1, the other enables of.11–.14 = 0, the 3 × 3 matrix 1339.15–.1D = 0x200, 0x1E53, 0x1FAD / 0x132, 0x259, 0x075 / 0x1F53,
   0x1EAD, 0x200 (13-bit two's complement, ×1/1024: the BT.601 rows Cr, Y, Cb [I]), 1339.20[2:0] = 4.
3. 1325.00[0] = 1 (DM), 133E.68[1] = 1, 131B.18[0] = 0.

`ISP_IQ_Init_Cfg_CameraOpen` and `ISP_IQ_WBG_LSC_ALSC_SrcSel` are stubs.

### 10.4 Bypass and shadow helpers [R]
<!-- ev:isp-1-hal.10.4 -->
`HalIsp_ApplyIspBypassMode` (via `DrvIsp_ApplyIspBypassMode`; callers `MhalCameraOpen`, `FrameStartProc`,
`CameraContextSwitchIspPipeActive/Bypass`). `MhalCameraOpen` calls it only in its YUV-input branch (bytes-per-pixel code 4 / 5:
after `PowerOn`, before `Camera3DNR_Enable` / `InOutModeConfig` / `HdrPipeInit`); the Bayer branch — this camera's — does not [R: `MhalCameraOpen`]. It clears the enable of every IQ sub-block in one go — 1311.00[0], 1326.00[0],
1328.20[6:0] and [14:8], 1312.50[0], 1312.00[0], 1313.00[0], 1319.00[0], 131D.40[0], 1316.00[0], 1317.00[0], 1322.00[0],
1317.20[0], 1316.6A[0], 1325.00[4], 1338.00[0], 1338.20[0], 1339.00[0], 133D.00[0], 133A.72[0], 133A.00[0], 133A.01[0],
1327.00[0], 1330.12[8:0], 1330.13[8:0], 1330.00[1], [3], [6], [2], 133D.1B[7:0], 133D.1D[0], 133E.20[0], 133E.20[3:2],
133E.00[0], 133E.40[0], 133E.60[0], 133E.68[0], 1333.60[1].

`DrvIsp_ApplyShadow(ctx, shadow, cmdq, id)` = the 46-entry switch over the IQ sub-blocks (isp-4-iq-engine.md §10.3). `DrvIsp_ApplySimpleBayerShadow` = its reduced variant for simple-Bayer mode [R jump table decoded from the
disassembly; Ghidra cannot follow it]: id 1 → 3DNR, 0x12 → WDR_LOC, 0x15 → DM, 0x16 → CCM, 0x17 → HSV, 0x18 → GammaC2A, 0x1F →
YEE, 0x2B → BNR, 0x2C → GNR; every other id nothing. `HalISP_ApplyShadow(h, shadow, cmdq, set)`: set 0 → OBC, DPC, SpikeNR, DM,
GammaC2A, CCM, HSV, R2Y; set 1 → WBG + DECOMP, OBC, OBC_P1, LSC, ALSC, HDR, SDC, DPC, ANTICT, RGBIR, SpikeNR, DM, GammaC2A, CCM,
HSV, R2Y, YEE, ADJ_UV, MXNR, YUV_GAMA, YCUVM10, COLOR_TRANS — **no caller** (unused).

Apply functions that are 4-byte stubs in this build: `HalIsp_ApplyFPN`, `FPNP1`, `GAMA16to16ISP`, `GAMAC2C_P1`, `LSCP1`,
`ALSCP1`, `GE`, `FalseColor`, `HDR16to12`, `NM`, `PostDN`, `BSNR`, `3DNR_NE`, and `HalIsp_SetWDRTbl`, `GetWDRTbl`, `ApplyWDRTbl`,
`ReadWDRTbl` (the WDR-table set) — i.e. the IPs FPN, FPN_P1, GAMA16to16, GAMAC2C_P1, LSC_P1, ALSC_P1, GE, FalseColor,
HDR16to12, NM, PostDN, BSNR, 3DNR_NE and the WDR table have no apply code. The tables these IPs would carry go through menuload (isp-2-ispmid-mload-ispscl.md §12.3).

`HalIsp_LoadBank(h, bank, src)` writes 128 registers of a bank through `ISP_RegW` from `src`; `HalIsp_ReadBank` copies 0x200
bytes; `ISP_HAL_Dump_Reg(bank)` / `DrvIsp_Dump_Reg` print 128 registers (log mask bit 0). All three are unreferenced.

The statistics HAL functions inside the hal/driver range (`HalIsp_HwStatistics*`) feed `DrvIsp_BufferPtrAssign`; they are specified with the statistics engines in isp-2-ispmid-mload-ispscl.md §10.2.

`int_to_twos_complement(v, bits)` / `twos_complement_to_int(v, bits)` are helpers of the AF filter attributes (statistics,
`ISP_AF_Set/GetFilterAttr`).

## 11. Function inventory [R]
<!-- ev:isp-1-hal.11 -->

Every one of the 559 hal/driver functions was inventoried with its address, size, stub body if it is one, and its
referrers (code callers with their sub-part, function-pointer uses, data references; `__ksymtab` entries ignored — no
other module imports any of them, §1.3); the counts below are from that inventory.

**Stubs** (115 functions whose whole body is `return <constant or argument expression>;`, listed in with the body):
`DrvIsp_ApplyWdrEfRatio`, `DrvIsp_IsHwPost3DNR`, `DrvIsp_HWBufferAllocSize`, `DrvIsp_GetCmdqBuffSizePerFrame`, `DrvIsp_GetHandle`, `DrvIsp_GetClockRateByIdx`, `DrvIsp_GetSupportClockNum`, `DrvIsp_IMI_ReservedSize`, `DrvIsp_IMI_GetWidthLimit`, `DrvIsp_RotEngineIsEnable`, `DrvIsp_MotionCalBufSize`, `DrvIsp_NE_IsSupported`, `DrvIsp_NE_CalBufSize`, `DrvIsp_GetYuvPipeId`, `DrvIsp_Vif_GetHdrChOffset`, `DrvIsp_GetSensorFrameCount`, `DrvIsp_CheckHdrCapability`, `DrvIsp_CheckRotationCapability`, `DrvIsp_IsExtWdma420`, `DrvIsp_IsExtWdma2`, `DrvIsp_GetFrameDoneIrqsCnt`, `IspDriverThreadWqStatusWorkCounter`, `IspDriverThreadWqStatusFuncCounter`, `IspDriverThreadWqStatusTimeoutCounter`, `DrvIsp_GetIrqNum`, `isp_fread`, `isp_poll`, `IspSysRegV2R`, `DrvIspModuleGetOfNode`, `HalIspGetClockRateByIdx`, `HalIspGetSupportClockNum`, `ISP_DmaCompressEnable`, `ISP_ExtWdmaGetId`, `ISP_IsExtWdam420`, `ISP_IsExtWdam2`, `HalIsp_ExtWdmaGetLineCnt`, `ISP_SetISPCrop`, `ISP_GetISPCrop`, `ISP_SetBayerScaler`, `ISP_SetWDMACrop`, `ISP_GetWDMACrop`, `ISP_ISPMasterEnable`, `ISP_SenClkPol`, `ISP_SenPCLKDelay`, `ISP_GetYuvPipeId`, `ISP_RMUXSrc`, `ISP_RVFSrc`, `ISP_SensorBus`, `ISP_GetSensorBus`, `ISP_HsyncPipeDelayYCmode`, `ISP_HsyncPipeDelayYCmodeMask`, `ISP_AFIFOSWReset`, `ISP_3DSensorReset`, `ISP_3DSensorPowerOff`, `ISP_InputEnable`, `ISP_ISPDP_GatedClk`, `ISP_Raw_GatedClk`, `ISP_Kernel_GatedClk`, `ISP_DPRstEveryVsync`, `ISP_DisableRawDNClk`, `ISP_DisableISPDPClk`, `ISP_EnableRawDNWDMAClk`, `ISP_SetAWBDSPMode`, `ISP_isISPBusy`, `ISP_isFIFOFull`, `ISP_GetDbgInputLineCnt`, `HalIsp_IsFrameMode`, `ISP_VDOSInit`, `ISP_DNREngineParallelMode`, `ISP_DNRSetDMARingMode`, `ISP_DNRSetBlockWidth`, `ISP_DNRDramRowMode`, `ISP_DNRBlockHorizontalCnt`, `ISP_DNR_SelectPosition`, `ISP_IMI_ReservedSize`, `ISP_IMI_GetWidthLimit`, `ISP_RotEngineIsEnable`, `ISP_ROTSetInputImageResolution`, `ISP_ROTGetInputImageResolution`, `ISP_ROTGetOutputLineCnt`, `ISP_ROTDataArrangeMode`, `ISP_ROTSetBlockWidth`, `ISP_ROTDramRowMode`, `ISP_MotionCalRefBufSize`, `ISP_NE_IsSupported`, `ISP_NE_CalRefBufSize`, `ISP_WDRSetWdrLocalSize`, `ISP_WDRSetWdrLocalSizeEX`, `HalIspVif_GetHdrChOffset`, `HalIsp_GetCmdqBuffSizePerFrame`, `ISP_WaitIspInputLineCount`, `HalIsp_IsHwPost3DNR`, `HalIsp_CheckHdrCapability`, `HalIsp_CheckRotationCapability`, `HalIsp_ApplyBSNR`, `HalIsp_ApplyGE`, `HalIsp_ApplyFalseColor`, `HalIsp_ApplyFPN`, `HalIsp_ApplyFPNP1`, `HalIsp_ApplyGAMA16to16ISP`, `HalIsp_ApplyGAMAC2C_P1`, `HalIsp_ApplyLSCP1`, `HalIsp_ApplyALSCP1`, `HalIsp_ApplyHDR16to12`, `HalIsp_ApplyNM`, `HalIsp_ApplyPostDN`, `HalIsp_SetWDRTbl`, `HalIsp_GetWDRTbl`, `HalIsp_ApplyWDRTbl`, `HalIsp_ReadWDRTbl`, `HalIsp_Apply3DNR_NE`, `ISP_IQ_WBG_LSC_ALSC_SrcSel`, `ISP_IQ_Init_Cfg_CameraOpen`, `HalIsp_GetSensorFrameCount`, `ISP_GetIrqFrameDoneCount`.
Besides constants they include the always-true/false switches `ISP_IsExtWdam420` = 1 / `ISP_IsExtWdam2` = 0, `ISP_ExtWdmaGetId` = 7,
`ISP_GetYuvPipeId` = 0, and the empty setters `ISP_SetISPCrop`, `ISP_SetBayerScaler`, `ISP_SetWDMACrop`, `ISP_RMUXSrc`,
`ISP_ROTSetInputImageResolution` that the driver layer calls as if they did something (§5.2, §8.2).

**Unreferenced** (99 functions with no code caller, no pointer and no data reference — unused on any path; 33 of them are also
stubs): `DrvIsp_ApplyWdrEfRatio`, `DrvIsp_GetClockIdxByRate`, `DrvIsp_GetClockRateByIdx`, `DrvIsp_ReadWDRTbl`, `DrvIsp_DNRMIUSel`, `DrvIsp_ROTSetFramePeriod`, `DrvIsp_NE_IsSupported`, `DrvIsp_NE_CalBufSize`, `DrvIsp_WDRSetWdrGlobalSize`, `DrvIsp_WDRSetWdrLocalSize`, `DrvIsp_GetIspInputMask`, `DrvIsp_GetDbgRotBaseAddr`, `DrvIsp_GetDbgInputLineCnt`, `DrvIsp_Dump_Reg`, `DrvIsp_RawStorePipeTrigger`, `DrvIsp_DMA_FbcVsyncSelection`, `DrvIsp_DMA_FbcInit`, `DrvIsp_IrqRotProcFramePeriod`, `IspDriverThreadWqDeinit`, `DrvIspModuleGetOfNode`, `DrvIspModulePreSetClkIdx`, `IspClkGenInit`, `IspClkGenDeinit`, `ISP_RDMAisEnable`, `ISP_RDMASetHighPri`, `ISP_RDMAReqHighPriThd`, `ISP_RDMAisBusy`, `ISP_WDMAisEnable`, `ISP_WDMAisAuto`, `ISP_WDMAAutoAlign`, `ISP_WDMACheckLBFull`, `ISP_WDMAisBusy`, `ISP_DMARingBufferIsEnable`, `ISP_WDMASetLineCnt2`, `HalIsp_ExtWdmaGetBaseAddr`, `ISP_GetWDMACrop`, `ISP_ISPMasterEnable`, `ISP_SenClkPol`, `ISP_SenPCLKDelay`, `ISP_GetSenDataType`, `ISP_GetSenDataPrecision`, `ISP_RVFSrc`, `ISP_SensorBus`, `ISP_GetSensorBus`, `ISP_HsyncPipeDelayYCmode`, `ISP_HsyncPipeDelayYCmodeMask`, `ISP_BypassISP`, `ISP_AFIFOSWReset`, `ISP_WdmaSwReset`, `ISP_RdmaSwReset`, `ISP_Dma420SwReset`, `ISP_MloadSwReset`, `ISP_RiuSwReset`, `ISP_TriggerDblBuf`, `ISP_3DSensorReset`, `ISP_3DSensorPowerOff`, `ISP_InputEnable`, `ISP_DPRstEveryVsync`, `ISP_SetAWBDSPMode`, `ISP_GetSenLineCnt`, `ISP_isISPBusy`, `ISP_isFIFOFull`, `ISP_SetLineCountInt1`, `ISP_SetLineCountInt2`, `HalIsp_IsFrameMode`, `ISP_VDOSInit`, `ISP_RDMASetCurrentFrameIndex`, `ISP_RDMAStartLineCntThreshold`, `ISP_WDMASetCurrentFrameIndex`, `ISP_WDMAStartLineCntThreshold`, `ISP_DNREnable3DNR`, `ISP_DNREnable2DNR`, `ISP_DNR_SelectPosition`, `ISP_ROTEngineSWReset`, `HalIsp_GetMaskCmdqIrq`, `ISP_WaitIspInputLineCount`, `HalIsp_ApplyBSNR`, `HalIsp_ApplyFalseColor`, `HalIsp_ApplyHDR16to12`, `HalIsp_ApplyNM`, `HalIsp_ApplyPostDN`, `HalIsp_SetWDRTbl`, `HalIsp_GetWDRTbl`, `HalISP_ApplyShadow`, `HalIsp_LoadBank`, `HalIsp_ReadBank`, `HalIspDisableInt`, `HalIspMaskInt1`, `HalIspMaskInt2`, `HalIspMaskInt3`, `HalIspMaskInt4`, `HalIspMaskInt5`, `HalIspMaskInt6`, `HalIspMaskInt7`, `HalIspClearInt`, `HalIspClearIntAll`, `HalIsp_SetSensorFrameCount`, `ISP_IrqUpdateFrameIntervalInfo`, `ISP_IrqUpdateLineCnt`.

## 12. Related public material [R unless tagged]
<!-- ev:isp-1-hal.12 -->

No source of the ISP HAL, ispmid or the IQ engine is public: GitHub code search for `HalIsp_ApplyYEE`, `IQ_Patch_NR3D`,
`hal_IQ_Set_ALL`, `CameraIspAlgo_DoAe`, `CameraIspCmdqBuffer_FinishBufferWrite`, `HalIspIsr`, `IspMidWorkExecute`,
`MS_CAM_IspApiSet` → 0 hits (2026-10-18); `MhalCameraOpen` occurs only in symbol lists and OpenIPC waybeam docs.

| material | where | what it gives | fit |
|---|---|---|---|
| Mercury5 ISP middleware headers | GitHub `longyanjun2020/SDK_pulbic` `Mercury5/proj/sc/middleware/isp/inc` | `isp_3a_if.h` (`AeInit/DoAe/AwbInit/DoAwb/IqInit/DoIq/DoIqPerFrame` with `AeInput_t/AeOutput_t/IqInput_t/IspRegShd_t`), `camalgo/drv_cam_autoexposure.h` (`pAE_*` prototypes), `drv_cam_autowhitebalance.h`, `drv_cam_iq.h`, `iq_apiset.h`, `iq_regset.h`, `iq_shadow.h` (`IspRegSet*`), `iq_struct.h`, `libcamera_io.h`, `common/iq_reg_map.h` (`Reginfo Regmap[DBG_TAG_MAX]` = the design of this build's `Regmap`), `common/dbg_id.h` (1195 cfg tags), `dbg_iq/ae/awb/cali_param.h`, `default_iqfile.xml`, `ISP_GUI.xml` | older chip (Mercury5), same library family: same function and table names; struct layouts and the tag list differ (this `Regmap` has 1072 entries, M5 1195; bank numbering differs) |
| Mercury5 IQ files and tools (not in the archive) | same repo, `isp/inc/iqfile/*_iqfile.{bin,xml}`, `Iq_Bin_Gen.exe`, `IqFileToBin.exe`, `Xml_Parser.exe`; `common/iq_bin.h` (1.6 MB) | IQ-file format by example (xml ↔ bin) | the M5 iqfile is 326 KB, this camera's 39 KB — another version of the format [R sizes] |
| CUS3A user interface | gitee qiming-zhixian/ssd2xx-openwrt-open `…/include/isp_cus3a_if.h` | `CUS3A_RegInterfaceEX(ch, adaptor, type, algo)`, `ISP_AE/AWB/AF_INTERFACE`, `Cus3AOpenIspFrameSync/WaitIspFrameSync` (= `/dev/isp_fe`), `CUS3A_GetAlgoStatus`, AE sample down-scaling | the public way to plug an own AE / AWB / AF into prog_uvc-style code without the kernel; SSD21x (Takoyaki) version |
| CUS3A demo sources | GitHub `ZYCX8888/Democode-TAKOYAKI-BETA001-0312` `sdk/verify/mi_demo/{alderaan,hoth}/{cus3a,uvc_cus3a,sigma3a}/` | complete user-space custom-3A examples (isp_fe loop, statistics read, AE / AWB result write) | other chip, same CUS3A ABI [I: not diffed against this libcus3a] |
| User-side ISP API headers | gitee Xsyt/sdk-sigmastar `files/public/i6e/MI/include/isp/{ispahan,pudding,macaron}/` (`mi_isp_api_id.h`, `mi_isp_datatype.h`, `mi_isp_iq_datatype.h`), qiming ssd21x `isp/{tiramisu,…}/`, an I6E `mi_isp_iq_datatype.h` and a SIYI A8 `mi_star_m6_isp.h` (channel / attr structs for M6) | id enums and payload structs of the ISP API (counted 190–211 matching `MI_ISP_*` names) | the per-id sizes (isp-3-iq-api.md §2) are the check for which header set matches this build [I] |
| Infinity2 ispmid API headers | jockyw2001/sdk `src/mhal/i2/vpe/drv/isp/pub/libcamera.h`, `libcamera_if.h`, `hal_isp_cmdq_if.h` | `MhalCameraOpen/Close`, `CameraContextSwitch`, `CameraRawFetch/Store`, `CameraLoadIQFile`, `CameraChangeCmdqIF`, `IspCmdqIf_t {RegW, RegWM, RegPollWait}` — the same ispmid layer one generation back | names match; argument structs older |
| Infinity2 ISP register headers | `…/i2/vif/hal/infinity2/inc/arch/infinity2_reg_isp{0..12}.h`, `_isp_dmag*.h`, `_isp_wdma.h`, `_isp_dma_irq.h`, `_isp_miu2sram.h`, `_ispsc_vif.h`; u-boot `isp/infinity_reg_isp0.h` | register bit names (`reg_en_isp_obc`, `reg_isp_lsc_center_x`, `reg_fpn_en`, `reg_isp_rdma_en`, …) | the I2 ISP sits at banks 0x15xx with another layout — names help, offsets do not transfer |
| Infinity2 menuload with DWARF | jockyw2001/sdk `src/mhal/i2/mhal.o` (debug info) — `DrvIsp_MLoad*`, `ISP_MLoad*`, `ISP_WaitMloadDone` | the earlier MLOAD driver with types | close relative of this MLOAD block [I] |
| I6E ISR-callback header | OpenIPC/linux `sigmastar-infinity6e` `drivers/sstar/isrcb/drv/pub/drv_isrcb.h` | ISRCB id names (`ISP_WDMA0…3_HIT_LINE_CNT`, `ISP_WDMA2/3_DONE`, …) | ids extended in this build (§4.4) |
| Other M6 mhal.ko builds | the firmware of SIYI A8 mini, RunCam Split-H, Anjoy MC-H12S and other M6 products (10 distinct builds) | diffing, other IQ files | no debug info |

The IQ file the kernel loads at open [R]: mhal's built-in `iq_bin` (39 175 B in `.data`) = a 40-byte header (a UUID string
`50e7b4dc-636e-4095-b3f5-d6d48438548e` + 4 bytes) followed by a byte-exact copy of `/config/iqfile/imx307_iqfile.bin` (39 135 B).
The user 3A loads the same tuning: `/config/iqfile/iqfile0…3.bin` are symlinks to `imx307_iqfile.bin`, so AE / AWB start from the
IMX307 tuning before the API bin overrides them. The API-bin record layout is `{u32 key; u32 type = 5; u32 len; data}` with the
API id in the first u16 of `data` (the second word is 5 in all 87 records of `iq_file_gc8613.bin`). The formats: IQ file —
isp-4-iq-engine.md §13; API bin — isp-3-iq-api.md §6.1.
