# C04 brand icons

C04 is the approved Quota Station identity: a blue application tile with a
circular liquid gauge, and a monochrome menu bar gauge with a rim bead.
The paused template places two upright bars inside the gauge.

The approved production masters live in
`/Users/shin-mac-mini/ui-ux-staging/quota-station/brand-icons/masters/`.
Their generation provenance and approval receipt are in
`/Users/shin-mac-mini/Workspace/1-Projects/firstmate/data/qs-brand-icons/report.md`.
These paths are external design sources, not build dependencies.

| Master | SHA-256 |
| --- | --- |
| `C04-app-icon-master-2048.png` | `6ca2aed2ce62110f30a457b40a34a75564aa03e5ec242a1418f305f353751f96` |
| `C04-menubar-normal-master.png` | `27079804750a582bba23cce131fb8dbe3a1aa0d86cf51fd07a78b6cba7867466` |
| `C04-menubar-paused-master.png` | `06d2baef2eafebf9183c005e0537b628f72e56eabfeb00a0bc2d87a1743a513e` |

The Tauri CLI generates the application icon set from the unmodified 2048px
RGBA app master. Run `npm run tauri -- icon /absolute/path/to/master.png
--output /temporary/output` and copy `32x32.png`, `128x128.png`,
`128x128@2x.png`, `icon.png`, and `icon.icns` into `src-tauri/icons/`.
These are every icon referenced by `tauri.conf.json`; the ICNS includes
16–1024px representations and preserves the master's transparent inset.

`assets/menubar/qs-tray-{base,paused,air,nodata}.svg` are technical traces of
the C04 template masters, with a reference liquid level of 45%. The base and
air traces share the same identity. The no-data trace has an empty reservoir.
The existing `qs-tray-empty.svg` warning triangle still represents an
exhausted Claude rotation.

`quota_station_core::icon::render_glyph` rasterizes the traced geometry with
4×4 coverage samples per pixel. Normal and paused frames are both 18pt square;
the tray supplies a 36×36px Retina image and explicitly enables macOS template
tinting on creation and each update. RGB stays black; antialiasing is in alpha.
The liquid height follows the selected service and quota window, matching the
title. Unknown quota leaves the reservoir empty. Pause bars have a transparent
halo to remain legible even at 100%; exhausted and unavailable readings retain
the existing pause precedence. The tooltip and VoiceOver account descriptions
are preserved.

The open-app button in the popover and other in-app icons are separate assets.
