#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Shin315
# SPDX-License-Identifier: MIT

"""Writes the community UI fixtures from the full ones.

The popover's fixtures in src/fixtures/snapshots/ are what the full build
sends. Without the `author-tools` feature a snapshot has no roles, no
rotation and no room pool: Claude's pool is the active account's own
numbers. This writes those snapshots to community/overlay/src/fixtures/
snapshots/, which the export puts in place of the full ones and which
`cargo test --test ui_fixtures` checks in a build without the feature
(`UPDATE_UI_FIXTURES=1` there rewrites them the same way).
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
FULL = ROOT / "src/fixtures/snapshots"
OUT = ROOT / "community/overlay/src/fixtures/snapshots"

ACCOUNT_KEYS = ("role", "disabled", "membership", "leavesAt")
POOL_KEYS = ("room", "members", "rooms", "refills")


def active_pool(section):
    """The pool from the active account's first line, as Rust's `active_pool`."""
    pool = {"kind": "none", "percent": None, "line": None, "scope": None,
            "nextRefillAt": None, "exhausted": False, "reason": None}
    active = next((a for a in section["accounts"] if a["active"]), None)
    line = active["lines"][0] if active and active["lines"] else None
    if line and line["percent"] is not None:
        pool.update(kind="active", percent=line["percent"], line=line["kind"],
                    exhausted=line["percent"] <= 0, nextRefillAt=line["resetsAt"])
    return pool


def community(snapshot):
    for name in ("claude", "agy", "codex"):
        section = snapshot.get(name)
        if not isinstance(section, dict):
            continue
        section.pop("personalHours", None)
        for account in section.get("accounts", []):
            for key in ACCOUNT_KEYS:
                account.pop(key, None)
        for key in POOL_KEYS:
            section["pool"].pop(key, None)
    claude = snapshot["claude"]
    claude["pool"] = active_pool(claude)
    return snapshot


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for path in sorted(FULL.glob("*.json")):
        snapshot = community(json.loads(path.read_text()))
        text = json.dumps(snapshot, indent=2, ensure_ascii=False) + "\n"
        (OUT / path.name).write_text(text)
        print(OUT / path.name)


if __name__ == "__main__":
    main()
