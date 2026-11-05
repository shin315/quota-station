# Third-party notices

Quota Station contains code ported or adapted from the projects below, which
[`NOTICE`](NOTICE) credits file by file. The agents' and the services' logos
come from svgl, and the interface's icons from Phosphor.

## Quota Station's own licenses

Copyright © 2026 Shin315.

Quota Station is licensed under the [MIT License](LICENSE), except:

- **The Antigravity accounts** (signing several Antigravity accounts in, reading
  their quota and switching between them), which contain code adapted under
  CC BY-NC-SA 4.0 (below), are licensed under the [Creative Commons
  Attribution-NonCommercial-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-nc-sa/4.0/)
  ([`LICENSES/CC-BY-NC-SA-4.0.txt`](LICENSES/CC-BY-NC-SA-4.0.txt)): personal and
  non-commercial use. They are `crates/quota-station-core/src/antigravity/`,
  `src-tauri/src/{antigravity,agy_quota,agy_switch}.rs`, `src/antigravity/`, and
  their tests and fixtures, as [`NOTICE`](NOTICE) lists them. They are compiled
  only with the Cargo feature `antigravity-accounts` and bundled into the
  interface only with `QS_ANTIGRAVITY_ACCOUNTS=1`; `scripts/install.sh` turns
  both on (`QS_ANTIGRAVITY_ACCOUNTS=0` leaves them out), and `cargo tauri build`
  and `npm run app` leave them out unless given `--features
  antigravity-accounts` (with `QS_ANTIGRAVITY_ACCOUNTS=1` for a release build's
  interface). An app built without them is MIT only: it reads the Antigravity
  account signed in and does not sign in or switch accounts.
- **The Inter font** (`src/assets/fonts/InterVariable-vi.woff2`) is under the
  [SIL Open Font License 1.1](LICENSES/OFL-1.1.txt) (below).

Every file names its license in an `SPDX-License-Identifier` line or in
[`REUSE.toml`](REUSE.toml), and `reuse lint` checks it in CI. Ported and
adapted code is credited in [`NOTICE`](NOTICE) and in the sections below.

## quota-axi

- Project: https://github.com/kunchenguid/quota-axi
- How it is used: ported code. The Codex, Antigravity and Claude Code
  quota reads, the reading model, the cache and stale rules, and which
  windows bound which scope are ported from quota-axi 0.1.66 into
  `crates/quota-station-core/src/quota/` (`mod.rs`, `cache.rs`,
  `codex.rs`, `agy.rs`, `claude.rs`, `semantics.rs`), read-only: no login is
  refreshed or written. See [`NOTICE`](NOTICE) for the files and the
  changes; each ported file also says so in its header. The app no longer
  runs quota-axi.
- License: MIT

```
MIT License

Copyright (c) 2026 Kun Chen

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## claude-swap

- Project: https://github.com/realiti4/claude-swap
- How it is used: adapted code (`crates/quota-station-core/src/claude/`),
  listed in [`NOTICE`](NOTICE).
- License: MIT

```
MIT License

Copyright (c) 2026 Onur Cetinkol

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Cockpit Tools

- Project: https://github.com/jlcodes99/cockpit-tools
- License: CC BY-NC-SA 4.0 (https://creativecommons.org/licenses/by-nc-sa/4.0/),
  © jlcodes
- How it is used: adapted code from version 1.3.70, commit 244b957
  (`crates/quota-station-core/src/antigravity/`), listed in
  [`NOTICE`](NOTICE). This is why the Antigravity accounts are shared under
  the same license, apart from the rest of Quota Station, which is MIT. They
  are built only with the `antigravity-accounts` feature; an app built
  without it holds none of this code.

## CC Switch

- Project: https://github.com/farion1231/cc-switch
- How it is used: ported code from version 4.0.15, commit 79e754ab
  (`crates/quota-station-core/src/codex/`), listed in [`NOTICE`](NOTICE).
- License: MIT

```
MIT License

Copyright (c) 2025 Jason Young

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## shadcn/ui

- Project: https://github.com/shadcn-ui/ui
- How it is used: adapted code. The components in `src/components/ui/`
  follow shadcn/ui's components (their structure, slots and Radix wiring)
  and are restyled to the app's macOS controls; `components.json` records
  their setup. See [`NOTICE`](NOTICE).
- License: MIT

```
MIT License

Copyright (c) 2023 shadcn

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## svgl

- Project: https://github.com/pheralb/svgl (https://svgl.app)
- How it is used: vendored logos. The Claude, Codex and Antigravity logos,
  and the logos of the services an MCP server, a plugin or a connector is
  for (GitHub, Notion, Figma, Slack, Linear, Supabase, Vercel, Cloudflare,
  Gmail, Google Drive, Google Calendar, Stripe, Atlassian, Asana, Canva,
  Playwright and PostgreSQL), were added through svgl's shadcn registry
  (`components.json`) into `src/components/ui/svgs/`. They are part of the
  app, so nothing is fetched while it runs. The only change: the two Codex
  files lose their `<title>`, since the name is written beside the logo.
- Trademarks: each logo is a trademark of its owner. The app uses a logo
  only to name that service next to its name; it does not mean the owner
  made, endorses or supports Quota Station, and no logo is used as the
  app's own icon.
- License (svgl's code): MIT

```
MIT License

Copyright (c) 2022 Pablo Hdez

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Inter

- Project: https://github.com/rsms/inter (https://rsms.me/inter)
- How it is used: a font a theme can choose (the interface itself draws in
  macOS's system font). Inter 4.001's variable font
  (`opsz` 14-32, `wght` 100-900) is bundled in
  `src/assets/fonts/InterVariable-vi.woff2`, cut with `pyftsubset` to
  Latin, Latin Extended-A/B, Vietnamese, the combining marks and the arrows
  and signs the app writes, keeping every OpenType feature. It is part of
  the app, so no font is fetched while it runs; the page names it
  "Inter QS". Nothing else in it is changed.
- License: SIL Open Font License 1.1, `LICENSES/OFL-1.1.txt`,
  Copyright 2020 The Inter Project Authors.

## Phosphor Icons

- Project: https://github.com/phosphor-icons/react (https://phosphoricons.com)
- How it is used: the interface's icons, in the window, its dialogs and
  the popover: their Regular weight, in the colour of the text around them,
  and their Bold for a control's plus, dots, caret or arrow. The
  `@phosphor-icons/react` package (2.1) is a dependency in `package.json`;
  `src/components/icons.tsx` imports only the glyphs it draws, which the
  build bundles into the app unchanged, so nothing is fetched while it runs.
- License: MIT

```
MIT License

Copyright (c) 2020 Phosphor Icons

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Libraries

The Rust crates and npm packages this app builds with, such as Tauri, serde,
time, reqwest, url, uuid, md5, sha2, toml_edit, objc2, React, Vite, Tailwind
CSS, Radix, class-variance-authority, clsx and tailwind-merge, keep their own licenses. Those licenses are listed in
`Cargo.lock`, `package-lock.json` and each package's own files.
