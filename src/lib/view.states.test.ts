// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The popover's states the community edition's design added (D07, D08):
// a first read, no Claude account yet, no agent at all, an account to sign in
// again, the account in use running dry, and the English words.

import { describe, expect, it } from 'vitest';
import { claudeAccountsFixture, codexFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { setLang } from './i18n';
import type { Model } from './model';
import { popoverView } from './view';

const view = (m: Model) => popoverView(m, FIXTURE_NOW, {});

/** Every Claude window of every account at `left` % left. */
function allLeft(m: Model, left: number): Model {
  for (const a of m.snapshot.claude.accounts) for (const l of [...a.lines, ...a.windows]) l.percent = left;
  return m;
}

describe('popover states', () => {
  it('the first read shows the names with no numbers yet', () => {
    const m = fixtureModel('fresh', { reading: true });
    m.snapshot.claude.newestReadingAt = null;
    m.snapshot.agy.newestReadingAt = null;
    const v = view(m);
    expect(v.loading).toBe(true);
    expect(v.fresh).toEqual({ tone: 'muted', text: 'đang đọc số…' });
    expect(v.claude.cards.map((c) => c.name)).toEqual(['lanpham.dev', 'lan.acme1', 'lan.acme2', 'lan.acme3']);
    expect(view(fixtureModel('fresh')).loading).toBe(false);
  });

  it('Claude Code with no account yet offers to add one; a Mac without it shows no group', () => {
    const m = fixtureModel('fresh');
    m.snapshot.claude.accounts = [];
    const v = view(m);
    expect(v.claude.shown).toBe(true);
    expect(v.claude.empty).toBe(true);
    expect(v.claude.cards).toEqual([]);
    // Keychain closed is not "no account": the banner says why, and the group waits out of sight.
    expect(view(fixtureModel('keychain')).claude.empty).toBe(false);
    expect(view(fixtureModel('keychain')).claude.shown).toBe(false);
    // Claude Code not set up: no group, and none of the empty card.
    const off = fixtureModel('fresh');
    off.snapshot.claude.status = 'notSetUp';
    expect(view(off).claude.shown).toBe(false);
    expect(view(off).claude.empty).toBe(false);
  });

  it('a Mac with no agent at all has the one empty card', () => {
    const m = fixtureModel('fresh', { codex: codexFixture({ present: false }) });
    m.snapshot.claude.status = 'notSetUp';
    m.snapshot.agy.status = 'notSetUp';
    expect(view(m).none).toBe(true);
    expect(view(fixtureModel('fresh')).none).toBe(false);
  });

  it('an account to sign in again has its chip, and a banner that opens the sign-in', () => {
    const accounts = claudeAccountsFixture();
    accounts.accounts[2] = { ...accounts.accounts[2], status: 'reloginRequired' };
    const v = view(fixtureModel('fresh', { claudeAccounts: accounts }));
    const card = v.claude.cards.find((c) => c.slot === 3)!;
    expect(card.relogin).toBe(true);
    expect(card.chips.map((c) => c.t)).toContain('cần đăng nhập lại');
    expect(v.claude.cards.filter((c) => c.relogin).map((c) => c.slot)).toEqual([3]);
    expect(v.banner).toMatchObject({ tone: 'info', actions: [{ act: 'claude:relogin:3' }] });
  });

  it('the account in use running dry with no room elsewhere says so, with the earliest reset', () => {
    const v = view(allLeft(fixtureModel('fresh'), 5));
    expect(v.banner?.tone).toBe('bad');
    expect(v.banner?.title).toBe('Hạn mức Claude sắp cạn');
    expect(v.banner?.body).toMatch(/^\*\*lan\.acme1\*\* còn 5% hạn mức .* và không tài khoản nào còn từ 15% trở lên\. Sớm nhất có lại lúc \d\d:\d\d \(.+\)\.$/);
    // One account with room is enough to keep quiet.
    const room = allLeft(fixtureModel('fresh'), 5);
    for (const l of [...room.snapshot.claude.accounts[2].lines, ...room.snapshot.claude.accounts[2].windows]) l.percent = 60;
    expect(view(room).banner).toBeNull();
    // Without the pool alert's limits there is no rule to read.
    expect(view(allLeft(fixtureModel('fresh', { alerts: undefined }), 5)).banner?.title).not.toBe('Hạn mức Claude sắp cạn');
  });

  it('speaks English when the language is English', () => {
    setLang('en');
    const v = view(fixtureModel('stale'));
    expect(v.fresh).toEqual({ tone: 'warn', text: 'updated 41 min ago' });
    expect(v.banner).toMatchObject({
      title: 'Claude quota data is 41 min old',
      body: 'Last updated successfully at 13:39.',
      actions: [{ label: 'Refresh', act: 'refresh' }],
    });
    expect(v.claude.cards[0].rows.map((r) => r.label)).toEqual(['Week', 'Fable']);
    expect(view(fixtureModel('fresh')).claude.cards[0].rows[0].reset).toBe('unused');
  });
});
