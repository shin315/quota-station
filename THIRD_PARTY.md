# Third-party notices

Quota Station reads the quota with code ported from quota-axi, and runs no
quota-axi program. The way it keeps the Claude accounts is adapted from
claude-swap. Parts of its
Antigravity login, quota reading and account switch are adapted from
Cockpit Tools, parts of
its Codex switch are ported from CC Switch, and its interface components
are adapted from shadcn/ui; [`NOTICE`](NOTICE) lists those files and what
was changed. The agents' and the services' logos come from svgl, and the
interface's icons from Phosphor and Lucide.

## quota-axi

- Project: https://github.com/kunchenguid/quota-axi
- How it is used: ported code. The Codex, Antigravity and Claude Code
  quota reads, the reading model, the cache and stale rules, and which
  windows bound which scope are ported from quota-axi 0.1.66 into
  `crates/quota-station-core/src/quota/` (`mod.rs`, `cache.rs`,
  `codex.rs`, `agy.rs`, `claude.rs`, `semantics.rs`), read-only: no login is
  refreshed or written. See [`NOTICE`](NOTICE) for the files and the
  changes; each ported file also says so in its header. The app no longer
  runs quota-axi; the rotation file keeps quota-axi's format, and the
  copies the app writes for Firstmate (`docs/claude-accounts.md`) are for
  the quota-axi Firstmate runs.
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
- How it is used: Quota Station keeps the Claude accounts itself, and the
  logic that touches Claude Code's login is adapted from claude-swap's
  source: the `security` commands, Claude Code's Keychain items and lock
  folders, the switch, the usage and token calls, the session folders,
  adding a login or a setup token, and reading claude-swap's own folder for
  `Nhập từ claude-swap`. [`NOTICE`](NOTICE) lists the files and what was
  changed. The app runs no claude-swap command and never writes claude-swap's
  files or Keychain items; the import only reads them, when the owner asks.
  The README's cutover checklist leaves claude-swap installed.
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
- How it is used: adapted code. The Antigravity login (the OAuth URL, the
  loopback callback, the code exchange, the token refresh, userinfo, the
  token record and the account index), the quota reading (the Cloud Code
  calls, the user agent and its version lookup, the host choice, and the
  quota cache layout with its TTL) and the Antigravity account switch (the
  shared Keychain login it writes, the Antigravity.app version rule and the
  order of a switch) are adapted from Cockpit Tools 1.3.70, commit 244b957.
  See [`NOTICE`](NOTICE) for the files and the changes; each
  adapted file also says so in its header. This is why the Antigravity
  accounts (those files, the parts that join them to the app and their
  screens) are shared under the same license, apart from the rest of Quota
  Station, which is MIT. They are built only with the `antigravity-accounts`
  feature; an app built without it holds none of this code. Cockpit's menu
  bar card was a design reference for the popover.

## CC Switch

- Project: https://github.com/farion1231/cc-switch
- How it is used: ported code. The rules the Codex switch follows (the
  reserved provider ids, how a provider table's own key is told apart, the
  `requires_openai_auth` rule, the refusals, the `auth.json` login fields
  and how Codex's shared background process is found and restarted) are
  ported from CC Switch 4.0.15, commit 79e754ab. See [`NOTICE`](NOTICE) for
  the files and the changes. The app never runs CC Switch or reads its data:
  it only checks whether CC Switch.app and its folder exist, whether it is
  running (from the process table), and what it left in `config.toml`.
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
- How it is used: the interface's icons, in their Duotone weight (and Bold
  for a control's plus or dots), each tinted its own colour. The
  `@phosphor-icons/react` package (2.1) is a dependency in `package.json`;
  `src/components/icons.tsx` imports only the glyphs it draws, which the
  build bundles into the app unchanged, so nothing is fetched while it runs.
  The sidebar and Tổng quan, Thống kê and Claude draw with them.
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

## Lucide

- Project: https://github.com/lucide-icons/lucide (https://lucide.dev)
- How it is used: the icons of the pages not yet moved to Phosphor. 45
  icons of Lucide 0.460.0 (lucide-static), the set the approved prototype
  draws with, are part of the app in `src/components/icons.tsx`, unchanged;
  nothing is fetched while it runs.
- License: ISC, `LICENSES/ISC.txt`. Copyright (c) for portions of Lucide
  are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other
  copyright (c) for Lucide are held by Lucide Contributors 2022.

## Libraries

The Rust crates and npm packages this app builds with, such as Tauri, serde,
time, reqwest, url, uuid, md5, sha2, toml_edit, objc2, React, Vite, Tailwind
CSS, Radix, class-variance-authority, clsx and tailwind-merge, keep their own licenses. Those licenses are listed in
`Cargo.lock`, `package-lock.json` and each package's own files.
