# Third-party notices

Quota Station talks to quota-axi and claude-swap below: it runs them as
separate programs and reads their output, and none of their source code is
copied into this repository. Parts of its Antigravity login and quota
reading are adapted from Cockpit Tools; [`NOTICE`](NOTICE) lists those
files and what was changed.

## quota-axi

- Project: https://github.com/kunchenguid/quota-axi (the app also uses a
  personal fork that adds claude-swap, rotation and Cockpit sidecars)
- How it is used: the app runs the `quota-axi` command and parses its
  `--json` report.
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
- How it is used: the app reads claude-swap's numbers only through
  quota-axi's `claudeSwap` and `claudePool` sidecars. To switch the active
  Claude account it runs one command, `cswap switch N --json`, when the owner
  confirms. It does not open claude-swap's files.
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
  token record and the account index) and the quota reading (the Cloud Code
  calls, the user agent and its version lookup, the host choice, and the
  quota cache layout with its TTL) are adapted from Cockpit Tools 1.3.70,
  commit 244b957. See [`NOTICE`](NOTICE) for the files and the changes; each
  adapted file also says so in its header. This is why Quota Station is
  shared under the same license. The app also shows the Antigravity
  accounts that quota-axi's `cockpit` sidecar reports, and Cockpit's menu bar
  card was a design reference for the popover.

## Libraries

The Rust crates and npm packages this app builds with, such as Tauri, serde,
time, reqwest, url, uuid, md5, sha2, React and Vite, keep their own licenses. Those licenses are listed in
`Cargo.lock`, `package-lock.json` and each package's own files.
