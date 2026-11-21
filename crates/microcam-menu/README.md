# microcam-menu

The on-screen menu of the YIZHAN 4K HDMI/USB microscope camera (SigmaStar SSC8836 / Mercury6, GC8613 sensor) under
OpenMicroCam: what the remote's keys open and change. It replaces the vendor's menu app `kjd_lvgl`. It is a client of
microcamd's control socket and touches nothing else: microcamd reads the remote, owns every camera setting and draws
the scene the menu sends it.

- `core` — the menu without I/O: a key, an event or a reply of microcamd, or the clock goes in; the state moves at once
  and the requests come out. Camera values are shown as microcamd confirms them; the screens, the lines, the language
  and the crosshair are the menu's own (the last two kept in the record microcamd stores for it).
- `menu` — the tables: the sections, their items, which control each shows.
- `text` — the languages: each is a file in `text/` with every word and whole sentence of the menu as the fields of one
  `Words`, its own name and its decimal mark; a language that lacks one does not build. A new language: its file, and
  its name in `Lang`.
- `view` — the state as the whole scene tree; `presenter` — the tree onto the outputs, a commit once the last is shown;
  `main` — the socket and the loop.

```sh
cargo test -p microcam-menu                          # the menu's behaviour key by key against a minimal microcamd
cargo run --release -p microcam-menu -- 192.168.1.100:7700   # from a computer, over the camera's USB network
```

On the camera `camera/app-start.sh` unpacks `/customer/microcam/microcam-menu.xz` into RAM and starts it after
microcamd, on the control socket, again whenever it ends: the pause doubles up to 30 s while it keeps ending at once.
It ends itself when a turn of its loop hangs (`alarm`) and when microcamd speaks another protocol. Its log, one line a
start and what went wrong, is `/tmp/microcam-menu.log`; `-v` adds a line with the times of each step shown.
`tools/cam/cam deploy` builds and uploads it with microcamd.

## Written as our own

The vendor's app was read for facts (`docs/firmware/components/kjd_lvgl.md` §5–§6); nothing of its writing is here.
The comparison, function by function, for likeness of the writing:

| ours | the vendor's | verdict |
|---|---|---|
| `Screen`, the level stack `Vec<Spot>`, `Level` | `screen_flag` numbers (0, 1, 0x14, 0x32 …) and the cursor `text_flag` in globals | own: typed screens and a stack, not a flag per screen |
| `menu` tables: sections, items, `Kind` | 17 main-menu rows as LVGL buttons in fixed slots of `guider_ui`, submenus as panels shown and hidden | own: rows are computed from items and the controls' list |
| keys by name from microcamd, `repeats` | `EV_KEY` and `EV_MSC` scancodes read from `/dev/input/event0`, a 150 ms repeat of its own | own: the kernel's repeats, chosen per screen |
| values through `set` / `adjust`, shown when confirmed | ids sent over TCP 8888, values written into `ws.bin` by offset | own: no file, no link of the vendor's |
| freeze → compare → camera on one key | the same three states on the freeze key | a fact of the device's behaviour, kept |
| the exposure shift −3.0…+3.0 by 0.3 | the same range and step | a fact (microcamd's control's range) |
| a clip at its end back to the strip | the end of a clip back to the file list | a fact of behaviour, kept |
| the format question, the reset question | YES / NO questions | a fact; our words and order |

Verdict: own; the facts kept are the device's behaviour and ranges, not code.
