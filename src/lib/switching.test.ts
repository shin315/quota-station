// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { describe, expect, it } from 'vitest';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Model, Outcome, Sessions } from './model';
import { claudeSheet, dayClock, switchFeedback } from './switching';

const model = fixtureModel('fresh');

/** The sheet for `slot`, with the hold left as it comes. */
const sheet = (m: Model, slot: number, sessions: Sessions | null) =>
  claudeSheet(
    m,
    slot,
    // @private:begin
    '60',
    // @private:end
    sessions,
    FIXTURE_NOW,
  );

const feedback = (outcome: Outcome, m: Model = model) =>
  switchFeedback(
    outcome,
    m,
    // @private:begin
    FIXTURE_NOW,
    // @private:end
  );

describe('dayClock', () => {
  it('names today, tomorrow and yesterday, and the date further off', () => {
    expect(dayClock('2026-01-16T10:29:00Z', FIXTURE_NOW)).toBe('17:29');
    expect(dayClock('2026-01-16T20:20:00Z', FIXTURE_NOW)).toBe('ngày mai 03:20');
    expect(dayClock('2026-01-15T03:00:00Z', FIXTURE_NOW)).toBe('hôm qua 10:00');
    expect(dayClock('2026-01-22T03:00:00Z', FIXTURE_NOW)).toBe('22/01 10:00');
  });
});

describe('the switch sheet and its result', () => {
  it('warns when the target is almost used up, and blocks before Quota Station keeps the accounts', () => {
    const low = fixtureModel('fresh');
    const a = low.snapshot.claude.accounts.find((x) => x.number === 3)!;
    a.windows[0] = { ...a.windows[0], percent: 8 };
    const v = sheet({ ...low, claudeSwitch: { ready: false, busy: null } }, 3, { live: 1, unreadable: 0 })!;
    expect(v.title).toBe('Chuyển Claude sang lan.acme2?');
    expect(v.confirm).toBe('Chuyển sang lan.acme2');
    expect(v.facts[0].text).toBe('lan.acme2 đã dùng **92%** 5 giờ, **50%** Fable.');
    expect(v.facts[1].k).toBe('w');
    expect(v.facts.find((f) => f.text === 'lan.acme2 đã dùng 92% hạn mức 5 giờ.')?.k).toBe('w');
    expect(v.facts.at(-1)!.k).toBe('x');
    expect(v.blocked).toBe(true);
  });

  it('a slot the snapshot no longer lists has no sheet', () => {
    expect(sheet(model, 9, null)).toBeNull();
  });

  it('records that cannot be read with no live session say so instead of "Ít nhất 0"', () => {
    const v = sheet(model, 3, { live: 0, unreadable: 2 })!;
    expect(v.facts[1]).toEqual({
      k: 'w',
      text: 'Không đọc được 2 bản ghi phiên Claude trên máy này, nên có thể có phiên đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo.',
    });
  });

  it('errors name what went wrong', () => {
    const fb = (error: string, detail = '', active: number | null = null) => feedback({ kind: 'failed', slot: 3, error, detail, active });
    const body = (error: string, detail = '', active: number | null = null) => fb(error, detail, active).body;
    expect(body('busy')).toContain('khoá đăng nhập');
    expect(fb('busy').actions!.map((a) => a.label)).toEqual(['Thử lại', 'Đóng']);
    expect(body('unmanaged')).toContain('Thêm tài khoản đang đăng nhập');
    // The login did not read back: put back as it was, or the account in use when it could not be.
    expect(body('unconfirmed')).toBe('Ghi xong nhưng đọc lại không khớp, nên app đã đặt lại như cũ. Chưa đổi gì.');
    expect(body('unconfirmed', '', 2)).toBe('Ghi xong nhưng đọc lại không khớp, và không đặt lại được như cũ. Đang dùng lan.acme1.');
    expect(body('keychain', 'security exited 36')).toBe('Keychain không cho đọc hoặc ghi đăng nhập (security exited 36). Chưa đổi gì.');
  });
});
