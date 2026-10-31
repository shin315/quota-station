// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { describe, expect, it } from 'vitest';
import { FIXTURE_NOW } from '../fixtures/models';
import { agyNames, cellOf, claudeNames, emailNames, tightest, usedOf, usedTone } from './accounts';
import type { Account, Line } from './model';

const line = (over: Partial<Line> = {}): Line => ({ kind: 'session', percent: 62, resetsAt: null, pending: false, atLeast: false, ...over });

const account = (number: number, over: Partial<Account> = {}): Account => ({
  number,
  // @private:begin
  role: 'team',
  disabled: false,
  membership: 'in',
  leavesAt: null,
  // @private:end
  active: false,
  reading: 'fresh',
  fetchedAt: null,
  source: 'list',
  lines: [],
  windows: [],
  ...over,
});

describe('share used', () => {
  it('is 100 minus what is left, and nothing while a window waits for its number', () => {
    expect(usedOf(line({ percent: 62 }))).toBe(38);
    expect(usedOf(line({ percent: 100 }))).toBe(0);
    expect(usedOf(line({ percent: 0 }))).toBe(100);
    expect(usedOf(line({ percent: 10.4 }))).toBe(90);
    expect(usedOf(line({ percent: null }))).toBeNull();
    expect(usedOf(line({ percent: 40, pending: true }))).toBeNull();
  });

  it('warning colours rise with use: amber from 70 %, red from 90 %', () => {
    expect([0, 69, 70, 89, 90, 100].map(usedTone)).toEqual(['ok', 'ok', 'warn', 'warn', 'bad', 'bad']);
    expect(usedTone(null)).toBe('muted');
  });

  it('a window not started yet is 0 % chưa dùng; a bound reads as at most', () => {
    expect(cellOf(line({ percent: 100 }), 'session', FIXTURE_NOW)).toMatchObject({ text: '0%', reset: 'chưa dùng', tone: 'ok' });
    const bound = cellOf(line({ kind: 'fableWeek', percent: 96, atLeast: true }), 'fableWeek', FIXTURE_NOW);
    expect(bound).toMatchObject({ text: '≤4%', title: 'Fable đã dùng nhiều nhất 4%' });
    expect(cellOf(undefined, 'week', FIXTURE_NOW)).toMatchObject({ text: '—', none: true, used: null });
    const soon = new Date(FIXTURE_NOW.getTime() + (3 * 60 + 9) * 60000).toISOString();
    expect(cellOf(line({ percent: 10, resetsAt: soon }), 'session', FIXTURE_NOW)).toMatchObject({
      text: '90%',
      tone: 'bad',
      reset: '3h 9m',
      words: '5 giờ đã dùng 90%, đặt lại sau 3 giờ 09 phút',
    });
    const later = new Date(FIXTURE_NOW.getTime() + (4 * 1440 + 19 * 60 + 39) * 60000).toISOString();
    expect(cellOf(line({ kind: 'week', percent: 61, resetsAt: later }), 'week', FIXTURE_NOW).reset).toBe('4d 19h');
  });

  it('the tightest window is the one used most; a tie names the longer one', () => {
    const five = cellOf(line({ percent: 20 }), 'session', FIXTURE_NOW);
    const week = cellOf(line({ kind: 'week', percent: 20 }), 'week', FIXTURE_NOW);
    const fable = cellOf(line({ kind: 'fableWeek', percent: 50 }), 'fableWeek', FIXTURE_NOW);
    expect(tightest([five, fable])?.label).toBe('5 giờ');
    expect(tightest([five, week])?.label).toBe('Tuần');
    expect(tightest([cellOf(undefined, 'week', FIXTURE_NOW)])).toBeNull();
  });
});

describe('account names', () => {
  it('drop the domain only when every account shares it', () => {
    expect([...emailNames(['a.b@example.com', 'c@example.com']).values()]).toEqual(['a.b', 'c']);
    expect([...emailNames(['a.b@example.com', 'c@example.org']).values()]).toEqual(['a.b@example.com', 'c@example.org']);
    expect([...emailNames(['a@Example.com', 'b@example.COM']).values()]).toEqual(['a', 'b']);
  });

  it('Claude: the email, else by place; a nickname wins', () => {
    const names = claudeNames([account(1, { email: 'lan.acme1@example.com', org: 'Acme Studio' }), account(2), account(0)], new Map([[2, 'Work']]));
    expect(names.get(1)).toEqual({ name: 'lan.acme1', short: '1', sub: '', full: 'lan.acme1@example.com' });
    expect(names.get(2)).toEqual({ name: 'Work', short: '2', sub: '', full: '' });
    expect(names.get(0)).toMatchObject({ name: 'Tài khoản đang dùng', short: 'C' });
  });

  it('Antigravity: the email, else Antigravity N', () => {
    const names = agyNames([{ number: 1, email: 'lanpham.ultra@example.com' }, { number: 2, email: 'lan.ultra2@example.com' }, { number: 3 }]);
    expect([...names.values()].map((n) => n.name)).toEqual(['lanpham.ultra', 'lan.ultra2', 'Antigravity 3']);
    expect(agyNames([{ number: 0 }]).get(0)?.name).toBe('Tài khoản đang dùng');
  });

  // @private:begin
  it('Claude with roles: the email with the organization of a Team seat and the hours of the personal account', () => {
    const names = claudeNames(
      [
        account(1, { role: 'personal', email: 'lanpham.dev@example.com', org: "lanpham.dev@example.com's Organization" }),
        account(2, { email: 'lan.acme1@example.com', org: 'Acme Studio' }),
        account(3, { role: 'unknown', email: 'lan.solo@example.com' }),
      ],
      new Map(),
      { personalHours: '8-23' },
    );
    expect(names.get(1)).toEqual({ name: 'lanpham.dev', short: 'CN', sub: 'Cá nhân · 08–23', full: 'lanpham.dev@example.com' });
    expect(names.get(2)).toEqual({ name: 'lan.acme1', short: 'A', sub: 'Acme Studio', full: 'lan.acme1@example.com' });
    expect(names.get(3)).toMatchObject({ name: 'lan.solo', sub: '' });
  });

  it('Claude with names hidden: by place, no organization, and the personal hours alone', () => {
    const names = claudeNames([account(1, { role: 'personal' }), account(4), account(2), account(7, { role: 'unknown' })], new Map(), { personalHours: '8-23' });
    expect([...names.values()].map((n) => [n.name, n.sub, n.full])).toEqual([
      ['Cá nhân', '08–23', ''],
      ['Team B', '', ''],
      ['Team A', '', ''],
      ['Tài khoản 7', '', ''],
    ]);
  });

  it('Antigravity with roles: the email, else Ultra N', () => {
    const names = agyNames([{ number: 1, email: 'lanpham.ultra@example.com' }, { number: 2, email: 'lan.ultra2@example.com' }, { number: 3 }], true);
    expect([...names.values()].map((n) => n.name)).toEqual(['lanpham.ultra', 'lan.ultra2', 'Ultra 3']);
  });
  // @private:end
});
