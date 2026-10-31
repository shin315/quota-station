# The community edition

This repository holds two editions of Quota Station:

- **The full build**, for the owner's own Macs: everything, built by
  `scripts/install.sh` with the `author-tools` Cargo feature on.
- **The community edition**, published in the public repository
  (`https://github.com/shin315/quota-station`): the same app without the
  private part. It is cut from this repository by `community/export.py`,
  one way only: work, fixes included, is done here and reaches the public
  repository through the export. A change asked for by the community is
  made here too, then exported.

The private part is Firstmate, the automatic Claude rotation with its rules,
its pause and its switcher, the other Macs and sending the rules to them over
SSH, `Copy to another Mac`, the update from source, `Settings › Advanced`, the
account roles (Personal, Team A, Ultra N) and the guide for leaving CC Switch.

## Marking the private part

- **Rust.** Everything private is behind `#[cfg(feature = "author-tools")]`.
  A module only the feature builds starts with
  `#![cfg(feature = "author-tools")]` (and its `mod` line is under the
  feature); the export drops the file. Inside a shared file the export
  removes each item under `#[cfg(feature = "author-tools")]`, with the doc
  comments, comments and attributes right above it, and keeps the item under
  `#[cfg(not(feature = "author-tools"))]` without its attribute. A build
  without the feature compiles none of the private code.
- **The interface.** Screens and logic only the private part uses live in
  `src/private/`, which the export drops. Shared files mark the lines that
  reach into it.
- **Any text file** (TypeScript, Markdown, YAML, shell, JSON fixtures, the
  docs): the lines from one holding `@private:begin` to the next holding
  `@private:end` are dropped, both included. Blocks do not nest. In
  TypeScript the marker is a `// @private:begin` comment line (or
  `{/* @private:begin */}` among JSX children); in Markdown an
  `<!-- @private:begin -->` line; in YAML and shell a `# @private:begin`
  line. To give the community a different value, write the community's first
  and override it in a marked block:

  ```ts
  let title = t('Hạn mức Claude sắp cạn', 'Claude quota running out');
  // @private:begin
  title = t('Hạn mức Claude trong vòng sắp cạn', 'Claude quota in rotation running out');
  // @private:end
  ```

  The community tree type-checks with `noUnusedLocals`, so an import or a
  helper only private code uses goes in a marked block too.
- **JSON keys** only the private part reads: `community/json-keys.txt`.
- **Whole paths**: `community/private-paths.txt` (the export itself, the
  owner's own pages, the README's screenshots, `src/private/`, the private
  Rust modules).
- **Files the community gets instead**: `community/overlay/`, copied over the
  tree last. Its `src/fixtures/snapshots/` are the popover's fixtures as a
  build without the feature sends them; `community/make-fixtures.py` writes
  them from the full ones (and `cargo test --test ui_fixtures` without the
  feature checks them).

The tests follow the same marks: a test of the private part is in
`src/private/`, in a test file only the feature builds, or in a marked block;
a shared test whose expectation differs gives the community's and overrides
it in a marked block.

## Running the export

```sh
python3 community/export.py --out /tmp/qs-community --worktree   # the files as they are now
python3 community/export.py --out /tmp/qs-community              # the tree of main
python3 community/export.py --out /tmp/qs-community --worktree --build
```

`--build` then runs, in the community tree, `npm ci`, `npm test`, `npm run
build`, `cargo fmt --all` (the cut is laid out by rustfmt, not by hand),
clippy with `-D warnings` and `cargo test`. `--public-url` sets the
repository the community tree's links name. The folder given to `--out`
must be empty, an earlier export, or hold a build's `target/` alone (CI
restores it there from its cache); `target/` is kept for the build.

The leak check reads every path and every text line of the community tree
against `community/leak-patterns.txt` and fails on any match, listing each
one. A new private word goes there when the private part grows.

CI runs `--worktree --build` on every change (the `Community tree (macOS)`
job), so a change that leaks, or that leaves the community tree unable to
build or pass its tests, fails here first.

## Pushing

`--push` clones the public repository, replaces its files with the community
tree, and pushes one commit to its `main` when anything changed. It is
turned off (`PUSH_ENABLED = False` in `export.py`) until the public
repository exists and the credential it pushes with is in place; nothing runs
it after a merge yet.

## Not in the community tree yet

The README's screenshots are taken from the full build's preview, whose
sidebar and switches show the private part, so the community tree has none:
`docs/screenshots/` is a private path and the READMEs' images are marked.
New ones taken from the community tree's own preview (`npm run dev` in an
export, `build=release`) can go in `community/overlay/docs/screenshots/`,
with the image lines unmarked.
