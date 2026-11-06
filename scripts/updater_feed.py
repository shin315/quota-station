#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Shin315
# SPDX-License-Identifier: MIT

"""The update key's public half and the updater's feed, for the Release workflow.

    python3 scripts/updater_feed.py check-key src-tauri/updater.pub
    python3 scripts/updater_feed.py manifest --version 1.2.0 \\
        --base-url https://github.com/shin315/quota-station/releases/download/v1.2.0 \\
        --pubkey src-tauri/updater.pub \\
        --entry darwin-aarch64,darwin-x86_64=Quota.Station_1.2.0_universal.app.tar.gz \\
        --entry windows-x86_64=Quota.Station_1.2.0_x64-setup.exe \\
        --out latest.json

`check-key` fails unless the file holds a minisign public key as `tauri signer
generate` writes it. `manifest` writes `latest.json`, the
file the app's updater reads from the latest release: for each entry, the
platforms it serves, the asset's download URL and the content of its `.sig`
(next to it). Each signature must be for this version (the app requires it,
`requireSignedVersion`) and, with `--pubkey`, made by that key: Tauri only warns
when the keys differ, and the app would refuse every such update.

Python's standard library only.
"""

import argparse
import base64
import binascii
import datetime
import json
import os
import sys


def fail(message):
    print(f"::error::{message}")
    sys.exit(1)


def b64(text):
    try:
        return base64.b64decode(text.strip(), validate=True)
    except (binascii.Error, ValueError):
        return None


def key_id(text):
    """The 8-byte id of a public key file's key, or None for anything else."""
    outer = b64(text)
    if outer is None:
        return None
    try:
        lines = outer.decode("utf-8").splitlines()
    except UnicodeDecodeError:
        return None
    if len(lines) < 2 or not lines[0].startswith("untrusted comment:"):
        return None
    key = b64(lines[1])
    if key is None or len(key) != 42 or key[:2] != b"Ed":
        return None
    return key[2:10]


def signature(text):
    """The key id and the trusted comment of a `.sig` file's content."""
    outer = b64(text)
    if outer is None:
        return None
    try:
        lines = outer.decode("utf-8").splitlines()
    except UnicodeDecodeError:
        return None
    if len(lines) < 4 or not lines[0].startswith("untrusted comment:"):
        return None
    sig = b64(lines[1])
    if sig is None or len(sig) != 74 or sig[:2] not in (b"Ed", b"ED"):
        return None
    comment = lines[2].removeprefix("trusted comment:").strip()
    return sig[2:10], comment


def signed_version(comment):
    for field in comment.split("\t"):
        if field.startswith("version:"):
            return field.removeprefix("version:")
    return None


def read(path):
    try:
        with open(path, encoding="utf-8") as f:
            return f.read()
    except OSError as e:
        fail(f"cannot read {path}: {e.strerror}")


def check_key(args):
    if not os.path.isfile(args.file):
        fail(f"{args.file} is missing: commit the update key's public half (docs/releasing.md)")
    if key_id(read(args.file)) is None:
        fail(f"{args.file} holds no update key's public half: commit the key's .pub file there (docs/releasing.md)")
    print(f"{args.file}: an update key's public half")


def manifest(args):
    want = key_id(read(args.pubkey)) if args.pubkey else None
    if args.pubkey and want is None:
        fail(f"{args.pubkey} holds no update key's public half")
    platforms = {}
    for entry in args.entry:
        keys, sep, path = entry.partition("=")
        if not sep or not keys or not path:
            fail(f"--entry {entry!r} is not PLATFORM[,PLATFORM...]=FILE")
        if not os.path.isfile(path):
            fail(f"no {path}")
        sig_text = read(path + ".sig").strip()
        parsed = signature(sig_text)
        if parsed is None:
            fail(f"{path}.sig is not a minisign signature")
        got, comment = parsed
        name = os.path.basename(path)
        if want is not None and got != want:
            fail(f"{name} was signed with another key than {args.pubkey}'s: the app would refuse it")
        version = signed_version(comment)
        if version is None or version.lstrip("v") != args.version:
            fail(f"{name}'s signature is for version {version or 'none'}, not {args.version}")
        for key in keys.split(","):
            if key in platforms:
                fail(f"platform {key} is given twice")
            platforms[key] = {"signature": sig_text, "url": f"{args.base_url.rstrip('/')}/{name}"}
        print(f"{name}: signed for {args.version}" + (", by the update key" if want else "") + f", serves {keys}")
    feed = {
        "version": args.version,
        "notes": args.notes,
        "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(feed, f, indent=2)
        f.write("\n")
    print(f"Wrote {args.out}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("check-key", help="fail unless FILE holds an update key's public half")
    p.add_argument("file")
    p.set_defaults(run=check_key)
    p = sub.add_parser("manifest", help="write latest.json")
    p.add_argument("--version", required=True)
    p.add_argument("--base-url", required=True)
    p.add_argument("--pubkey", help="the public key every signature must be made with")
    p.add_argument("--notes", default="")
    p.add_argument("--entry", action="append", required=True, help="PLATFORM[,PLATFORM...]=FILE, with FILE.sig next to it")
    p.add_argument("--out", required=True)
    p.set_defaults(run=manifest)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
