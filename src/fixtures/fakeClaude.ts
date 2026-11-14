// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The fake Rust side's Claude accounts (`fakeApi.ts`): what an account action
// lands as.

import type { Account, ClaudeAccountRow, ClaudeAccountsView, ClaudeActionDone, Model } from '../lib/model';

/** The quota rows follow the kept accounts, as the app's list does: a new one has no numbers yet. */
export function keptSnapshot(snap: Model['snapshot'], kept: ClaudeAccountRow[]): Model['snapshot'] {
  const had = new Map(snap.claude.accounts.map((a) => [a.number, a]));
  const accounts = kept.map(
    (r): Account =>
      had.has(r.slot)
        ? {
            ...had.get(r.slot)!,
            disabled: !r.enabled,
          }
        : {
            number: r.slot,
            role: 'unknown',
            disabled: !r.enabled,
            membership: 'unknown',
            leavesAt: null,
            active: r.active,
            reading: 'noData',
            fetchedAt: null,
            source: 'list',
            lines: [],
            windows: [],
            ...(r.email ? { email: r.email } : {}),
          },
  );
  return { ...snap, claude: { ...snap.claude, accounts } };
}

/** A sign-in renews the kept account `slot`; when that is the one in use, whether Claude Code's own login got it (`live`, by default it did). */
export function renewedDone(v: ClaudeAccountsView, slot: number, live = true): Partial<ClaudeActionDone> {
  return { slot, renewed: true, ...(v.accounts.some((a) => a.slot === slot && a.active) ? { live } : {}) };
}
