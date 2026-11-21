# Tools — working with the YIZHAN 4K camera from a Linux laptop

The laptop-side tools of OpenMicroCam for the YIZHAN 4K HDMI/USB microscope camera (SigmaStar SSC8836 / Mercury6, GC8613
sensor): building, reaching the camera over its USB network and serial console, deploying `microcamd`, and the bench
checks. They need Linux, Python 3 and the camera on USB; what the bench needs besides is in [BENCH.md](BENCH.md).

## Settings

Every tool reads one set of environment variables; `tools/hostconf.py` shows the values in force. Each comes from the
environment, else from a config file, else its default. The config file is a shell file of `NAME=value` lines, sourced
by `sh` in its own directory: the file `$MICROCAM_CONFIG` names, else `microcam.env` at the repository's top
(git-ignored). None needs setting for a stock camera.

| variable | default | what it is |
|---|---|---|
| `MICROCAM_CAMERA` | `192.168.1.100` | the camera's address on its USB network (RNDIS) |
| `MICROCAM_HOST` | `192.168.1.2` | the laptop's address there |
| `MICROCAM_USER`, `MICROCAM_PASSWORD` | `root`, `ws123321` | the camera's shell login (telnet, serial console); the default is the stock firmware's, set them only for a camera whose login was changed |
| `MICROCAM_VIDEO` | `/dev/v4l/by-id/usb-Linux_Foundation_Webcam_gadget-video-index0` | the camera's UVC node on the laptop |
| `MICROCAM_LOGS` | `$XDG_STATE_HOME/openmicrocam` | transcripts, the testers' details, the serial console's log, the panel rig's files |
| `MICROCAM_STAGE` | `stage/` here | built modules and tools, and the way back `cam deploy` keeps |
| `MICROCAM_SYSROOT` | `work/dump` here | the camera's dump, unpacked (`rootfs/lib`, `miservice/lib`, `miservice/modules`): `install.py dump` writes it |
| `MICROCAM_LINK_LIBS`, `KTREE` | `work/build/linklibs`, `work/build/linux-4.9.227` here | the build's inputs ([INSTALL.md](INSTALL.md) «Build OpenMicroCam»): `tools/mk-linklibs.sh` and `tools/install/prepare-ktree.sh` make them |
| `MICROCAM_SERIAL`, `MICROCAM_SERIAL_BAUD` | `/dev/ttyUSB0`, `115200` | the USB-serial adapter on the camera's console (the fallback way in) |
| `MICROCAM_PROGRESS` | plain | `at`: `cam deploy` prints `@stall` / `@show` progress lines for a status line that reads them |
| `MICROCAM_HOOK_*` | none | the hooks below |

The defaults lie in `work/`, git-ignored, where [INSTALL.md](INSTALL.md)'s commands put everything they make.

## Hooks

A hook is a shell command in a variable, run by `sh -c` with the settings in its environment; unset, nothing runs.

| variable | run | contract |
|---|---|---|
| `MICROCAM_HOOK_STREAM_RELEASE` | before a tool opens the camera's video node itself (`cam check`, the testers, the panel rig) or reboots the camera (`cam deploy`, `boottry.sh`) | exit 0 when it freed a stream it holds (a viewer of yours): the tool then runs the restore hook when it is done |
| `MICROCAM_HOOK_STREAM_RESTORE` | after, only when the release hook exited 0 | give the stream back; exit 0 when it is back |
| `MICROCAM_HOOK_STATE` | by `cam state` and `cam watch`, bounded to 3 s | prints lines: `allow PID…` = processes allowed to hold the video node (others are reported as stray holders); a line starting with `⚠` = an anomaly; any other line = an extra info line |
| `MICROCAM_HOOK_CHANGE` | before `cam deploy` and `cam check`, bounded to 10 s | exit 0 lets the command run; any other exit refuses it, and the hook's output is the reason `cam` prints |

## The tools

| tool | what it does |
|---|---|
| `tools/hostconf.py [--sh]` | the settings in force; `--sh` prints `export` lines for a shell script |
| `tools/install-toolchain.sh` | the build toolchain: Rust with the armv7 target, zig, cargo-zigbuild, and on Debian/Ubuntu the arm-linux-gnueabihf cross gcc and binutils, libclang, patchelf |
| `tools/install/` | dump the camera (over its USB network, a microSD card or the serial console), unpack the dump and check it against the tested camera's parts, prepare the kernel tree, install, back to stock; `--card` writes the card's files into `work/card/dump`, `install` or `stock` for you to copy onto the card: [INSTALL.md](INSTALL.md) |
| `tools/mk-linklibs.sh` | link-time copies of the dump's MI libraries with a SONAME, into `MICROCAM_LINK_LIBS` |
| `tools/elfcheck.sh BINARY` | what the camera will see of a binary: interpreter, NEEDED, GLIBC ≤ 2.30, architecture |
| `kmods/build-ko.sh DIR` | a kernel module against the prepared 4.9.227 tree (`KTREE`) |
| `kmods/g_webcam/build.sh` | the UVC gadget `g_webcam.ko`, its descriptors taken from the dump's stock `g_webcam.ko` |
| `camera/tools/build.sh TOOL.c [-lLIB…]` | a camera-side C tool against the dump's glibc and MI libraries (`-lLIB`: one more of them the tool needs) |
| `camera/tools/isptool.c` | ISP IQ records (ColorToGray, NR3D) of the running pipeline: dump, set, save, load |
| `camera/tools/dpctool.c` | the ISP's defect-pixel correction records: dump, save, load, poke a field |
| `camera/tools/dispdump.c` | read-only raw dump of the live DISP, HDMI and SCL-port attributes |
| `camera/tools/portdepth.c` | read or set an MI output port's depth (user fifo, buffer queue) |
| `tools/cam/cam` | the camera in one entry: `state` (what it does, anomalies first), `wait up\|microcamd\|stream\|idle`, `sh` (its shell: output and exit code only), `log` (microcamd's log, filtered), `deploy` (build, keep the way back, upload, reboot, wait; `--ko` a module too, a sensor driver into `/customer/fast`), `check` (the bench checks, [BENCH.md](BENCH.md)), `watch` (a line when something turns worse) |
| `tools/cam/camlib.py` | the library under `cam` and the testers: shell, control protocol, link, hooks, report lines |
| `tools/cam/cam-net.sh` | the laptop's address on the camera's USB network after every camera reboot (through `sudo ip`) |
| `tools/cam/boottry.sh DRIVER.ko` | a one-boot trial of a sensor driver file in `/customer/fast`; reports what came up |
| `tools/cam/ctester.sh FILE.c` | build a C tester (it includes `mi.h`) against the camera's libraries and run it on the camera |
| `tools/cam/serve-slow.py` | a throttled HTTP server of `MICROCAM_STAGE` for uploads over the stock USB driver |
| `tools/cam/uart.py` | the serial console: `bridge` owns the port and logs it, `send` a line (and wait for a pattern), `login`, `uboot` gets the U-Boot prompt |
| `tools/cam/uboot-rescue.sh CMD` | catch the U-Boot prompt, boot once into a RAM shell, mount `/customer`, run a fix |
| `tools/cam/nor-dump-raw.sh` | dump the 16 MB NOR flash from U-Boot to raw SD-card sectors, read back and compared |
| `tools/cam/usb-preview.sh`, `view.sh` | low-latency viewers of the USB stream (ffplay; mpv or GStreamer fullscreen) |
| `tools/cam/isocap.py` | a full usbmon capture of the isochronous stream with a UVC header check |
| `tools/cam/screen.sh` | a test image as the GNOME wallpaper, for the camera to look at, and back |
| `tools/cam/test/*.py` | the bench checks one by one ([BENCH.md](BENCH.md)); each takes `--json` |
| `tools/cam/panel/` | the panel rig, tests that show the camera a picture on the laptop panel: its view calibration (`view.py`), the latency clock (`clock.py`, `latency.py`), dropped frames read from a recording (`framegaps.py`); and canvas hold times (`holds.py`): its README |
| `tools/scheme/scheme.py SRC.d2 [--lang L] [-o OUT]` | a D2 scheme of `docs/artifacts/` as an interactive HTML page, in the language asked ([`tools/scheme/README.md`](../tools/scheme/README.md)) |
| `tools/ui/demo-menu*.json` | the demo menu scene the testers and the crates' tests use |
