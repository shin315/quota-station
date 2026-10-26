# Theme packs

Cài đặt › Hiển thị › Chủ đề picks the theme pack every window is drawn in:
the popover, the main window and its dialogs, and the menus. The packs come
from the styles the owner chose in October 2026. A pack redefines only how
things are drawn: grounds and surfaces, the glass (tint, opacity, blur,
saturation, rim), the accent, the text, the quota colours, the type and its
figures, the corners, the shadows, and a haze on the main window. It never
changes sizes, spacing, copy, features or which data is shown, and icons keep
the text's colour.

## How it works

- `src/styles/tokens.css` holds every value. The look the app wore before
  the packs is the top of the file and is the pack `default`; each other
  pack is one block, `[data-pack="…"]` (light, and the values both modes
  share), `[data-pack="…"][data-theme="dark"]`, and the dark block again for
  `data-theme="auto"` while macOS is dark.
- `data-pack` sits on `<html>` and on each window's root beside
  `data-theme` (`useThemePack` in `src/App.tsx`). The choice is saved as
  `themePack` in `settings.json` (`ThemePack` in
  `crates/quota-station-core/src/settings.rs`); every window reads it from
  the model, so a change redraws them all at once. A value a later version
  adds reads as `default`.
- The picker (`src/settings/ThemePicker.tsx`) shows each pack as the main
  window in small (`MiniWindow`): its desktop and two shapes behind its
  glass, its sidebar and the row in use, a title in its type, a group with
  two quota bars and its accent. Each carries its own `data-pack` and the
  look on screen, so it is drawn in that pack's tokens; while the windows
  follow macOS it shows the light look and the dark one split across the
  diagonal. Every pack sets the same names in both modes, so a picture
  never shows another pack's value.
- Names a pack may set beyond the old ones: `--meter-ok` (a quota bar that
  is not short; green in `default`, the pack's grey elsewhere), `--panel`
  and `--panel-line` (a group's panel in the main window and its edge),
  `--side-glass` (the sidebar's tint), `--win-mist` (a haze over the main
  window's glass), `--font-features`, `--ink3-glass` and
  `--accent-ink-glass` (the stronger text on glass, which `glass.css` used
  to hold), and `--rim-more`.
- Reduce Transparency (macOS's flag, `data-material="solid"`): every glass
  surface draws its pack's `--glass-solid`; the haze stays, as it is opaque
  over that ground.
- Increase Contrast (macOS's flag read by `windows::increase_contrast`,
  `data-contrast="more"`, and `prefers-contrast: more`): the rims of the
  glass, the panels' edges and the lines between rows are drawn in
  `--rim-more` in every pack.
- `src/styles/tokens.test.ts` checks each pack in light and dark: every text
  colour (`--ink`, `--ink2`, `--ink3`, `--accent-ink`, `--info-ink`,
  `--ok-ink`, `--warn-ink`, `--bad-ink`) reaches 4.5:1 on every surface it
  is drawn on (the glass over a white or grey ground in light and over a
  black or grey ground in dark, the panel on it, a button's fill on the
  panel, the row in use, the sidebar, the solid ground, the dialogs),
  `--on-accent` on `--accent` and white on `--bad-fill` reach 4.5:1, and the
  three quota bars stay apart from each other (OKLab ΔE ≥ 0.05) and from
  their track (≥ 0.12) for normal sight, protanopia and deuteranopia. The
  new packs pass with nothing left over. `default`, kept as it was, has a
  few smaller texts under 4.5:1 where the worst ground shows through its
  glass; the test lists them so none is added.

## Measuring

The sources are concept rasters: their features, logos and layout were
ignored and only the look kept. Colours are the median of a flat box; text
colours the median of the 15 % of the box's pixels farthest from its ground
(the stroke's core, not its antialiased edge). Boxes are `x0,y0,x1,y1` in the
image's own pixels (1536 × 1024 for the Q images, 1448 × 1086 for redraw A).
Where a measured colour fell short of 4.5:1 it was moved along its own hue
until it passed; those are marked *adjusted*. The four Q styles were drawn
only in dark: their light modes are derived (same hue, glass and corners on
a light ground) and marked *derived*.

Sources (read only, in Firstmate's data folder):

- Q03, Q06, Q11, Q13: `qs-ce-look/assets/Q{03,06,11,13}.png`
- Redraw A: `qs-ce-look/redraw/assets/R{O,P,C,S}-A-{light,dark}.png`
- Design-system sheets D01–D10: `qs-ce-look/ds/assets/D01.png` … `D10.png`
- The community edition's measurements on the same images:
  `qs-ce-ui-build/design/DESIGN.md`

## Mặc định

The look as it was, unchanged: system type, Apple's blue, green bars,
frosted glass at 34 px blur. No block of its own.

## Kính Nhịp (redraw A, D01–D10)

Porcelain and charcoal; frosted glass thick enough not to see through (the
redraw's rule, "kính dày gần đặc"); a mist on the main window; Inter (the
font the app already ships, `Inter QS`); soft round controls (D01: controls
8, cards 12, windows 16); grey bars, amber when short.

| Token | Light | Dark | Measured on |
| --- | --- | --- | --- |
| `--glass-solid`, `--winbg` | `#f5f8fc` | `#0f1a24` | RO-A window ground `1120,700,1420,990`: `#f5f8fc` / `#0f1a24` |
| `--glass` | `rgba(245,248,252,.86)` | `rgba(15,26,36,.86)` | the solid ground at 0.86: thick glass, judged against the redraw |
| `--side` | `#eff5fb` | `#13212e` | RO-A sidebar `30,960,270,975`: `#eff5fb` / `#13202d` |
| `--panel` | `rgba(255,255,255,.62)` | `rgba(22,35,47,.66)` | RO-A group `330,135,1400,140`: `#f8fafd` / `#121f2b`; card `#16232f` |
| `--sel` | `rgba(11,107,254,.10)` | `rgba(26,106,240,.32)` | RO-A sidebar chosen `200,150,265,180`: `#d5e5fd` / `#1b4286`; light *adjusted* lighter |
| `--ink` | `#0b1226` | `#f5f8fd` | RO-A title `310,35,490,70` ink: `#01081d` / `#fcfcfd` (CE DESIGN.md §6) |
| `--ink2` | `#536079` | `#a2b3c8` | RO-A email `370,450,508,467` ink: `#838ca4` / `#95a3b6`; *adjusted* to AA |
| `--ink3` | `#56637c` | `#96a8bb` | RO-A sub line `310,80,610,98` ink: `#8e99b1` / `#8194aa`; *adjusted* to AA |
| `--accent` | `#0a62ea` | `#1a64e6` | RO-A button `880,930,1075,960`: `#196bea` (light); dark from RC-A's button (CE §6) |
| `--accent-ink` | `#0e56e6` | `#6cb3f7` | CE DESIGN.md §6, chip text `528,630,605,650`: `#2360d7` |
| `--meter-ok` | `#78869d` | `#9aa4bb` | RO-A bar `685,445,740,453`: `#79869e` / `#9aa5bc` |
| `--track` | `rgba(20,40,80,.12)` | `rgba(255,255,255,.12)` | RO-A track `760,445,808,453`: `#dbe1e9` / `#232f3b` |
| `--warn` / `--warn-ink` | `#fea004` / `#994400` | `#f5a32e` / `#ffb23f` | RO-A bar `535,445,630,453`: `#fda104` / `#f6a32f`; light ink *adjusted* (measured `#fd8707`, 2.3:1) |
| `--bad` / `--bad-ink` | `#fd4b50` / `#c01722` | `#fd4b50` / `#ff7f7c` | D01 ramp, CE §6; light ink *adjusted* |
| `--ok` / `--ok-ink` | `#24c94c` / `#13652b` | `#34d354` / `#4fe07a` | CE §6 |
| `--win-mist` | blue haze top left, lilac bottom right | the same, deeper | RO-A window top `#f7fafd` against bottom `#f5f8fc`, the "misty" ground the brief names |
| `--bl-pop`, `--sat` | 40px, 1.6 | 40px, 1.6 | judged: frosted, nothing behind legible |
| radii | pop/win/sheet 16, card/tile 12, row 10, ctl 8 | | D01 §6 "bo 8/12/16" |

## Dải sáng tĩnh (Q03)

A compact list on dense navy glass, a blue band for what is chosen, grey
bars and amber when short, tighter corners. Light *derived*: the same dense
glass in a cool pale tone.

| Token | Dark | Light (*derived*) | Measured on Q03 (dark) |
| --- | --- | --- | --- |
| `--glass-solid` | `#111d2c` | `#eef2f7` | window ground `400,318,740,342`: `#111d2c`, sd 0.9 |
| `--glass` | `rgba(17,29,44,.93)` | `rgba(239,243,248,.93)` | the aurora does not show through any window: dense |
| `--side` | `#182432` | `#e5ebf2` | sidebar `60,600,230,800`: `#182432` |
| `--panel` | `rgba(255,255,255,.035)` | `rgba(255,255,255,.55)` | account box `520,418,840,448`: `#182430` on `#111d2c` |
| `--fill2` | `rgba(255,255,255,.09)` | `rgba(17,29,44,.09)` | Add account button `1300,620,1490,650`: `#222f40` |
| `--sel` | `rgba(65,112,206,.34)` | `rgba(46,79,141,.15)` | sidebar chosen `160,405,235,432`: `#2e4f8d`; *adjusted* lighter so grey text on the row in use passes |
| `--ink` | `#f4f7f9` | `#0f1a2a` | name `347,424,378,438` ink: `#f4f7f9` |
| `--ink2` | `#b8c1cd` | `#4a5668` | label `350,453,426,468` ink: `#b0b9c6`; *adjusted* |
| `--ink3` | `#a3afbf` | `#4f5b6c` | email `393,424,500,438` ink: `#7e8b9b`; *adjusted* (3.9:1) |
| `--accent` / `--accent-ink` | `#2f62c9` / `#9dbdf2` | `#2f5fc4` / `#2a56b5` | chip `1410,497,1446,508`: `#20395e`, its text `#92aacd` |
| `--meter-ok` | `#808ca3` | `#7d889c` | bar `465,483,545,491`: `#808ca3` |
| `--track` | `rgba(255,255,255,.10)` | `rgba(17,29,44,.11)` | track `560,483,655,491`: `#2a3441` |
| `--warn` | `#f7a73d` | `#ec930f` | bar `465,456,600,464`: `#f7a73d` |
| `--ok` | `#48cb5e` | `#2fa84a` | dot `327,428,335,436`: `#48cb5e` |
| `--win-mist` | a faint light band at the top | the same | the still light band the style is named for |
| radii | pop/win/sheet 12, card 10, tile 9, row 8, ctl 6 | | the boxes' corners, ~8-10 px |

## Một nhịp nhìn (Q06)

Flat charcoal, no glass and no picture behind: the tint is opaque, so a
window reads as one surface. Bright numbers, a yellow bar when short, a blue
for what is chosen, figures all the same width (`"tnum"`). Light *derived*:
the same flatness on off-white.

| Token | Dark | Light (*derived*) | Measured on Q06 (dark) |
| --- | --- | --- | --- |
| `--glass`, `--glass-solid` | `#1a1e23` | `#f6f6f7` | window ground `600,380,940,420`: `#1a1e23` |
| `--side` | `#191d21` | `#efeff1` | settings sidebar `990,600,1130,800`: `#191d21` |
| `--panel`, `--card` | `#1f2328` | `#ffffff` | service tile `380,700,630,780`: `#202429` |
| `--glass-line` | `rgba(255,255,255,.08)` | `rgba(0,0,0,.12)` | popover separator `860,212,1230,215`: `#2d3034` |
| `--sel` | `rgba(38,110,220,.34)` | `rgba(31,99,201,.12)` | settings chosen `1095,380,1122,405`: `#1f62c9` (a softer band for rows) |
| `--accent` | `#1f63c9` | `#1f63c9` | the same, white text 5.5:1 |
| `--accent-ink` | `#6aa8f5` | `#1d5bbf` | link `825,335,920,348` ink: `#3981bd`; *adjusted* (3.5:1) |
| `--ink` | `#f4f5f6` | `#17191c` | big number `70,635,135,668` ink: `#fbfcfc` |
| `--ink2` | `#a9adb5` | `#52565d` | muted `246,662,323,677` ink: `#878a94`; *adjusted* |
| `--ink3` | `#a0a4ac` | `#5d6168` | subtitle `55,433,515,452` ink: `#6e7279`; *adjusted* (3.3:1) |
| `--warn` / `--warn-ink` | `#f8c331` / `#f8c331` | `#e6ac00` / `#755600` | bar `680,680,795,687`: `#f8c331` |
| `--meter-ok` | `#9599a0` | `#8a8e95` | bar `385,682,440,688`: `#9599a0` |
| `--track` | `rgba(255,255,255,.11)` | `rgba(0,0,0,.1)` | track `460,682,520,688`: `#373d44` |
| `--toggle-on` | `#2686f7` | `#1f63c9` | radio `72,885,86,898`: `#2686f7` |
| `--font-features` | `"tnum" 1` | | the big numbers line up |
| radii | pop/win/sheet 12, card/tile 10, row 8, ctl 6 | | |

## Bàn điều khiển (Q11)

A console: even-width type everywhere (`ui-monospace`, SF Mono), near-opaque
slate, hairline frames round each group, a blue band for the row in use,
bluish bar tracks, small corners. Even-width type is wider, so a few long
names end in "…" sooner than in the other packs. Light *derived*: the same
console on paper.

| Token | Dark | Light (*derived*) | Measured on Q11 (dark) |
| --- | --- | --- | --- |
| `--glass-solid` | `#13171c` | `#f6f7f8` | window ground `300,320,640,360`: `#13171c` |
| `--glass` | `rgba(19,23,28,.96)` | `rgba(246,247,248,.96)` | nothing shows through |
| `--side` | `#171c22` | `#eef0f3` | settings sidebar `990,560,1120,800`: `#171c22` |
| `--panel-line` | `rgba(255,255,255,.11)` | `rgba(20,30,45,.16)` | card frame `1150,440,1152,490`: `#21262c` |
| `--sel` | `rgba(30,112,243,.3)` | `rgba(27,98,208,.12)` | row in use `560,460,600,490`: `#142a4d`; settings chosen `1100,330,1118,355`: `#183f7d` |
| `--accent` | `#1b6fe0` | `#1b62d0` | button `860,876,1000,905`: `#1f7bf9`; *adjusted* for white text (3.9:1) |
| `--accent-ink` | `#7fb0f0` | `#1a5bc2` | link `683,337,800,351` ink: `#7b98b3`; *adjusted* |
| `--ink` | `#f5f6f7` | `#15191e` | title `45,333,195,355` ink: `#f5f6f7` |
| `--ink2` | `#abb6c1` | `#4b5562` | column head `100,433,155,447` ink: `#abb6c1` |
| `--ink3` | `#8f9db0` | `#59636f` | email `150,469,265,483` ink: `#8f9db5` |
| `--meter-ok` | `#a0aec1` | `#6f7d90` | bar `312,470,352,480`: `#a0aec1` |
| `--track` | `rgba(96,128,170,.3)` | `rgba(30,60,110,.14)` | track `360,470,423,480`: `#344763` |
| `--warn` | `#eeaf2e` | `#dc9b10` | bar `625,510,715,520`: `#eeaf2e` |
| `--font` | `ui-monospace, "SF Mono", Menlo` | | every line in Q11 |
| radii | pop/win/sheet 8, card/tile 6, row 5, ctl 4 | | |

## Chiều lặng (Q13)

Clear glass to look through, tinted blue-violet, a light rim; groups one
step more opaque than the window; a soft periwinkle for what is chosen and
for buttons, with dark text on them; a warm orange when short. Light
*derived*: a lavender dusk with the same clarity.

| Token | Dark | Light (*derived*) | Measured on Q13 (dark) |
| --- | --- | --- | --- |
| `--glass` | `rgba(26,37,71,.80)` | `rgba(244,243,252,.78)` | CE DESIGN.md §5: regression on the window's left edge, a = 0.80, tint `#1a2547` |
| `--bl-pop`, `--sat` | 16px, 1.2 | 16px, 1.2 | CE DESIGN.md §5 (blur of the empty sidebar, saturation slope) |
| `--glass-line` | `rgba(255,255,255,.28)` | `rgba(255,255,255,.7)` | CE §5: edge `#6c6c81`, white at 0.31-0.34 |
| `--glass-solid` | `#1b2433` | `#f1f0fa` | popover ground `1050,320,1450,326`: `#182133` |
| `--panel` | `rgba(50,63,80,.6)` | `rgba(255,255,255,.5)` | CE §5: cards +10-14 over the window's glass |
| `--card` | `#232a3a` | `#f8f7fd` | dialog `1080,550,1150,600`: `#1d2737` |
| `--sel` | `rgba(140,150,255,.2)` | `rgba(91,95,214,.14)` | row in use `300,505,330,512`: `#2c3547` with a violet radio |
| `--accent` / `--on-accent` | `#7e8bfa` / `#15133b` | `#5357d4` / `#ffffff` | dialog button `1290,640,1450,665`: `#7e8bfa`, its text `#15133b` |
| `--toggle-on` | `#7185f8` | `#5f6ff0` | switch `1405,830,1440,845`: `#7185f8` |
| `--ink` | `#f8fafb` | `#1b1a33` | title `270,165,380,188` ink: `#f8fafb` |
| `--ink2` | `#bcc1d2` | `#56556f` | label `292,310,330,325` ink: `#a5aabc`; *adjusted* (panels are lighter) |
| `--ink3` | `#b0b5c7` | `#5b5a76` | subtitle `270,197,450,212` ink: `#8d95a7`; *adjusted* |
| `--warn` / `--warn-ink` | `#efa058` / `#f0bf6f` | `#ec8a33` / `#8f4a00` | bar `352,335,430,346`: `#efa058`; number `1163,147,1197,162`: `#f0bf6f` |
| `--meter-ok` | `#a0a3b0` | `#8a8ca3` | bar `352,395,385,405`: `#a0a3b0` |
| `--track` | `rgba(255,255,255,.16)` | `rgba(60,50,130,.13)` | track `400,395,455,405`: `#474f61` |
| `--win-mist` | violet haze top right, blue bottom left | lavender | the dusk of Q13's wallpaper, kept faint inside the window |
| radii | pop 16, win/sheet 14, card/tile 12, row 10, ctl 8 | | |
