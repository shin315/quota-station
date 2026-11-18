// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The fake Rust side's Antigravity logins (`fakeApi.ts`): which account a login lands on.
// Plain code: the preview bundle must not load Vitest.

import type { AgyLogin, AgyLoginDone } from '../lib/model';

/**
 * Where a fake login lands: on the account `picks` (an id) when the browser was signed in to it,
 * else on an account that needs a login (which it answers), else on a new account added at the
 * end. The list comes back with that account signed in; the caller renumbers it.
 */
export function agyLoginLands(list: AgyLogin[], picks?: string): { next: AgyLogin[]; done: AgyLoginDone } {
  const again = list.find((a) => (picks ? a.id === picks : a.status !== 'ok'));
  if (again) {
    const next = list.map((a) => (a.id === again.id ? { ...a, status: 'ok' as const, verify: false } : a));
    return { next, done: { id: again.id, number: again.number, added: false } };
  }
  const id = picks ?? 'antigravity_fake' + (list.length + 1);
  const added: AgyLogin = { id, number: 0, email: null, name: null, status: 'ok', verify: false, reading: false, readError: null, rotationPaused: false };
  return { next: [...list, added], done: { id, number: list.length + 1, added: true } };
}
