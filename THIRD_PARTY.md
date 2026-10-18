# Third-party notices

Quota Station talks to the tools below. It runs them as separate programs
and reads their output. None of their source code is copied into this
repository.

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

- Project: https://github.com/jlcodes99/cockpit-tools (CC BY-NC-SA 4.0)
- How it is used: the app shows the Antigravity accounts that quota-axi's
  `cockpit` sidecar reports. Cockpit's menu bar card was a design reference
  for the popover. No Cockpit code is ported in this version, so there is no
  `NOTICE` file yet. When a module is ported, it will get a `NOTICE` that
  credits Cockpit Tools, links its license and says what was changed.

## Libraries

The Rust crates and npm packages this app builds with, such as Tauri, serde,
time, React and Vite, keep their own licenses. Those licenses are listed in
`Cargo.lock`, `package-lock.json` and each package's own files.
