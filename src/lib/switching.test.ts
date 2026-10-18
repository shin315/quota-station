import { describe, expect, it } from 'vitest';
import { hideNames } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel, heldSwitcher, switcherFixture } from '../fixtures/models';
import type { Decision } from './model';
import { bannerUnder, refreshedFeedback } from './view';
import {
  autoView,
  claudeSheet,
  dayClock,
  decisionText,
  holdOptions,
  logText,
  namesOf,
  pauseOf,
  pauseOptions,
  pausedFeedback,
  switchFeedback,
  switcherBanner,
} from './switching';

const model = fixtureModel('fresh');
const nameOf = namesOf(model);
const account = (n: number) => model.snapshot.claude.accounts.find((a) => a.number === n)!;
const at = (iso: string) => new Date(iso);

describe('bannerUnder', () => {
  const paused = switcherBanner(fixtureModel('fresh', { switcher: heldSwitcher() }), FIXTURE_NOW);
  it('the paused banner steps aside only for a line that already says the account is held', () => {
    const held = fixtureModel('fresh', { switcher: heldSwitcher() });
    expect(bannerUnder(paused, null)).toBe(paused);
    expect(bannerUnder(paused, pausedFeedback(held, FIXTURE_NOW))).toBeNull();
    const switched = switchFeedback({ kind: 'switched', slot: 3, hold: { until: '2026-01-16T08:20:00Z', failed: false } }, held, FIXTURE_NOW);
    expect(bannerUnder(paused, switched)).toBeNull();
    // A refresh or an unconfirmed switch leaves resume and "Thêm 1 giờ" in place.
    expect(bannerUnder(paused, refreshedFeedback(FIXTURE_NOW))).toBe(paused);
    expect(bannerUnder(paused, switchFeedback({ kind: 'unconfirmed', slot: 3, hold: null }, held, FIXTURE_NOW))).toBe(paused);
  });
});

describe('dayClock', () => {
  it('names today, tomorrow and yesterday, and the date further off', () => {
    expect(dayClock('2026-01-16T10:29:00Z', FIXTURE_NOW)).toBe('17:29');
    expect(dayClock('2026-01-16T20:20:00Z', FIXTURE_NOW)).toBe('ngày mai 03:20');
    expect(dayClock('2026-01-15T03:00:00Z', FIXTURE_NOW)).toBe('hôm qua 10:00');
    expect(dayClock('2026-01-22T03:00:00Z', FIXTURE_NOW)).toBe('22/01 10:00');
  });
});

describe('hold and pause choices', () => {
  it('holds for 30 minutes, an hour, to the five-hour reset, or not at all', () => {
    const opts = holdOptions(account(4), FIXTURE_NOW);
    expect(opts.map((o) => [o.v, o.t, o.s])).toEqual([
      ['30', '30 phút', 'đến 14:50'],
      ['60', '1 giờ', 'đến 15:20'],
      ['reset', 'Đến lúc đặt lại', 'đến 17:09'],
      ['none', 'Để tự chuyển', 'không giữ'],
    ]);
    expect(opts[1].until!.toISOString()).toBe('2026-01-16T08:20:40.000Z');
    expect(opts[3].until).toBeNull();
  });

  it('an account whose five-hour window has not started holds for five hours', () => {
    expect(holdOptions(account(3), FIXTURE_NOW)[2].until!.toISOString()).toBe('2026-01-16T12:20:40.000Z');
  });

  it('a reset already past does not count', () => {
    const later = at('2026-01-16T10:30:00Z');
    expect(holdOptions(account(2), later)[2].s).toBe('đến 22:30');
  });

  it('"Đến 18:00" is offered only before 18:00, and ends at 18:00 local time', () => {
    const before = pauseOptions(FIXTURE_NOW);
    expect(before.map((o) => o.v)).toEqual(['60', '1800', 'inf']);
    expect(before[1].until!.toISOString()).toBe('2026-01-16T11:00:00.000Z');
    expect(pauseOptions(at('2026-01-16T11:00:00Z')).map((o) => o.v)).toEqual(['60', 'inf']);
  });
});

describe('the pause', () => {
  it('counts while its end is ahead and the switcher reads the flag', () => {
    const held = fixtureModel('fresh', { switcher: heldSwitcher() });
    expect(pauseOf(held, FIXTURE_NOW)).toEqual({ until: '2026-01-16T08:20:00.000Z', reason: 'chuyển tay sang tài khoản 2 lúc 14:12' });
    expect(pauseOf(held, at('2026-01-16T08:20:00Z'))).toBeNull();
    expect(pauseOf(fixtureModel('fresh', { switcher: heldSwitcher({ guard: false }) }), FIXTURE_NOW)).toBeNull();
    expect(pauseOf(model, FIXTURE_NOW)).toBeNull();
  });

  it('a pause with no end lasts until resumed', () => {
    const held = fixtureModel('fresh', { switcher: heldSwitcher({ pause: { until: null, reason: null, holds: true } }) });
    expect(pauseOf(held, at('2027-01-01T00:00:00Z'))).toEqual({ until: null, reason: null });
    expect(autoView(held, FIXTURE_NOW).line).toBe('Đang tạm dừng đến khi bạn bật lại');
  });

  it('the banners: a switcher that ignores the flag first, then claude-swap’s own switcher, then the pause', () => {
    expect(switcherBanner(model, FIXTURE_NOW)).toBeNull();
    expect(switcherBanner(fixtureModel('fresh', { switcher: heldSwitcher({ guard: false }) }), FIXTURE_NOW)?.tone).toBe('bad');
    expect(
      switcherBanner(fixtureModel('fresh', { switcher: heldSwitcher({ autoService: true }) }), FIXTURE_NOW)?.title,
    ).toBe('Hai bộ tự chuyển cùng chạy');
    const paused = switcherBanner(fixtureModel('fresh', { switcher: heldSwitcher() }), FIXTURE_NOW)!;
    expect(paused.title).toBe('Đang giữ lan.acme1 đến 15:20');
    expect(paused.actions.map((a) => a.act)).toEqual(['resume', 'addHour']);
  });

  it('the feedback after pausing says until when', () => {
    const held = fixtureModel('fresh', { switcher: heldSwitcher() });
    expect(pausedFeedback(held, FIXTURE_NOW).body).toBe('Giữ lan.acme1 đến 15:20. Hết thời gian này, tự chuyển sẽ bật lại.');
  });
});

describe('the switcher in words', () => {
  const cases: [Decision, string][] = [
    [{ kind: 'stay' }, 'Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.'],
    [{ kind: 'unknown', slot: 3 }, 'Vẫn dùng lan.acme2: chưa có số mới để quyết định.'],
    [{ kind: 'waiting', slot: 4, at: '17:09' }, 'Vẫn dùng lan.acme1: không tài khoản nào còn hạn mức, chờ lan.acme3 được đặt lại lúc 17:09.'],
    [{ kind: 'wait' }, 'Không tài khoản nào còn hạn mức. Switcher giữ tài khoản đang dùng.'],
    [
      { kind: 'switch', slot: 2, why: { kind: 'expiring', at: '17:45' } },
      'Lần kiểm tới sẽ chuyển sang lan.acme1: ưu tiên dùng lan.acme1 trước khi tài khoản rời vòng lúc 17:45.',
    ],
    [
      { kind: 'switch', slot: 4, why: { kind: 'early', at: '17:09', left: 30, before: 3 } },
      'Lần kiểm tới sẽ chuyển sang lan.acme3: hạn mức 5 giờ của lan.acme3 đặt lại lúc 17:09 khi mới dùng 70%, sớm hơn lan.acme2.',
    ],
    [
      { kind: 'switch', slot: 3, why: { kind: 'room', used: 0, ends: null } },
      'Lần kiểm tới sẽ chuyển sang lan.acme2: lan.acme2 còn hạn mức (đã dùng 0%).',
    ],
    [
      { kind: 'switch', slot: 1, why: { kind: 'personal', used: 12 } },
      'Lần kiểm tới sẽ chuyển sang lanpham.dev: không tài khoản Team nào còn hạn mức (lanpham.dev đã dùng 12%).',
    ],
    [
      { kind: 'switch', slot: 4, why: { kind: 'park', at: '17:09' } },
      'Lần kiểm tới sẽ chuyển sang lan.acme3: không tài khoản nào còn hạn mức, lan.acme3 được đặt lại sớm nhất (17:09).',
    ],
    [{ kind: 'paused', until: '2026-01-16T15:20', reason: 'giữ tay' }, 'Switcher đang tạm dừng: giữ tay.'],
    [{ kind: 'other' }, 'Switcher báo: something new'],
  ];
  it.each(cases)('%j', (decision, words) => {
    expect(decisionText(decision, 'something new', 'lan.acme1', nameOf)).toBe(words);
  });

  it('goes by Team A, Team B… while names are hidden', () => {
    const hidden = namesOf(hideNames(model));
    expect(decisionText({ kind: 'switch', slot: 3, why: { kind: 'room', used: 0, ends: null } }, '', 'Team A', hidden)).toBe(
      'Lần kiểm tới sẽ chuyển sang Team B: Team B còn hạn mức (đã dùng 0%).',
    );
    expect(hidden(1)).toBe('Cá nhân');
  });

  it('says what the last log line did, with its time', () => {
    expect(logText({ at: '2026-01-16T02:12:05Z', event: { kind: 'switch', slot: 2 }, raw: '' }, nameOf, FIXTURE_NOW)).toBe(
      '09:12 · Switcher chuyển sang lan.acme1.',
    );
    expect(logText({ at: null, event: { kind: 'pauseEnded' }, raw: '' }, nameOf, FIXTURE_NOW)).toBe('Hết tạm dừng, tự chuyển bật lại.');
    expect(logText({ at: null, event: { kind: 'other' }, raw: 'raw line' }, nameOf, FIXTURE_NOW)).toBe('raw line');
  });

  it('a switcher that is missing or not loaded turns the toggle off', () => {
    const missing = autoView(fixtureModel('fresh', { switcher: switcherFixture({ installed: false }) }), FIXTURE_NOW);
    expect(missing.toggle).toEqual({ on: false, disabled: true, label: 'Tự chuyển Claude' });
    const stopped = autoView(fixtureModel('fresh', { switcher: switcherFixture({ service: false }) }), FIXTURE_NOW);
    expect(stopped.why).toBe('Switcher không chạy: LaunchAgent com.cswap.prefer-team chưa được nạp.');
    expect(stopped.toggle.disabled).toBe(true);
  });

  it('a dry run that failed says why', () => {
    const v = autoView(fixtureModel('fresh', { switcher: switcherFixture({ dryRun: null, dryRunError: 'noDecision' }) }), FIXTURE_NOW);
    expect(v.why).toBe('Chưa đọc được quyết định của switcher (switcher không in quyết định, có thể cswap list lỗi).');
  });
});

describe('the switch sheet and its result', () => {
  it('warns when the target is almost used up or out of the rotation, and blocks without cswap', () => {
    const low = fixtureModel('fresh');
    const a = low.snapshot.claude.accounts.find((x) => x.number === 3)!;
    a.windows[0] = { ...a.windows[0], percent: 8 };
    a.membership = 'out';
    const v = claudeSheet({ ...low, claudeSwitch: { cswap: false, busy: null } }, 3, '60', { live: 1, unreadable: 0 }, FIXTURE_NOW)!;
    expect(v.facts.map((f) => f.k)).toEqual(['', 'w', 'w', 'w', 'x']);
    expect(v.facts[2].text).toBe('lan.acme2 không còn trong vòng xoay. Nếu để tự chuyển, switcher sẽ đổi sang tài khoản trong vòng.');
    expect(v.facts[0].text).toBe('lan.acme2 đã dùng **92%** 5 giờ, **50%** Fable.');
    expect(v.facts[3].text).toBe('lan.acme2 đã dùng 92% hạn mức 5 giờ.');
    expect(v.blocked).toBe(true);
  });

  it('a hold on a switcher that ignores the flag is a warning', () => {
    const m = fixtureModel('fresh', { switcher: switcherFixture({ guard: false }) });
    expect(claudeSheet(m, 3, '60', null, FIXTURE_NOW)!.hint).toEqual({
      tone: 'warn',
      text: 'Switcher trên máy này chưa đọc cờ tạm dừng, nên vẫn có thể đổi tài khoản.',
    });
  });

  it('a slot the snapshot no longer lists has no sheet', () => {
    expect(claudeSheet(model, 9, '60', null, FIXTURE_NOW)).toBeNull();
  });

  it('a hold that could not be written is a warning', () => {
    const fb = switchFeedback({ kind: 'switched', slot: 3, hold: { until: '2026-01-16T08:20:00Z', failed: true } }, model, FIXTURE_NOW);
    expect(fb.tone).toBe('warn');
    expect(fb.body).toBe(
      'Đọc lại sau 5 giây: lan.acme2 đang dùng ✓. Chưa ghi được cờ tạm dừng, nên switcher vẫn có thể đổi tài khoản ở lần kiểm sau.',
    );
  });

  it('switching while already paused says the pause goes on', () => {
    const held = fixtureModel('fresh', { switcher: heldSwitcher() });
    expect(switchFeedback({ kind: 'switched', slot: 3, hold: null }, held, FIXTURE_NOW).body).toBe(
      'Đọc lại sau 5 giây: lan.acme2 đang dùng ✓. Đang tạm dừng tới 15:20: chuyển tay sang tài khoản 2 lúc 14:12.',
    );
  });

  it('errors name what went wrong', () => {
    const fb = (error: string, detail = '', active: number | null = null) =>
      switchFeedback({ kind: 'failed', slot: 3, error, detail, active }, model, FIXTURE_NOW);
    const body = (error: string, detail = '', active: number | null = null) => fb(error, detail, active).body;
    expect(body('notFound')).toBe('Không tìm thấy lệnh cswap trên máy này. Cài lại claude-swap rồi thử lại.');
    expect(body('spawn', 'could not start cswap: denied')).toBe('cswap không chạy được: could not start cswap: denied. Chưa đổi gì.');
    // cswap stopped without an answer: the account read again, or a refresh when it could not be.
    expect(body('timeout', 'cswap did not answer within 60s', 2)).toBe('cswap không trả lời trong 60 giây. Đọc lại: đang dùng lan.acme1.');
    expect(fb('timeout', '', 2).actions!.map((a) => a.label)).toEqual(['Thử lại', 'Đóng']);
    expect(body('timeout')).toBe('cswap không trả lời trong 60 giây. Bấm Làm mới để xem tài khoản đang dùng.');
    expect(fb('timeout').actions!.map((a) => a.label)).toEqual(['Làm mới', 'Đóng']);
    expect(body('exit', 'cswap exited 2: boom', 2)).toBe('cswap dừng mà không báo kết quả (cswap exited 2: boom). Đọc lại: đang dùng lan.acme1.');
  });

  it('records that cannot be read with no live session say so instead of "Ít nhất 0"', () => {
    const v = claudeSheet(model, 3, '60', { live: 0, unreadable: 2 }, FIXTURE_NOW)!;
    expect(v.facts[1]).toEqual({
      k: 'w',
      text: 'Không đọc được 2 bản ghi phiên Claude trên máy này, nên có thể có phiên đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo.',
    });
  });

  it('a hold shorter than the pause already on says the pause stays', () => {
    const open = fixtureModel('fresh', { switcher: heldSwitcher({ pause: { until: null, reason: 'tạm dừng', holds: true } }) });
    expect(claudeSheet(open, 3, '60', null, FIXTURE_NOW)!.hint.text).toBe(
      'Đang tạm dừng đến khi bạn bật lại: tạm dừng. Lần tạm dừng này dài hơn nên được giữ nguyên.',
    );
    // Paused until 15:20: a 30-minute hold (14:50) keeps it, "Đến lúc đặt lại" (17:09) replaces it.
    const held = fixtureModel('fresh', { switcher: heldSwitcher() });
    expect(claudeSheet(held, 4, '30', null, FIXTURE_NOW)!.hint.text).toMatch(/được giữ nguyên\.$/);
    expect(claudeSheet(held, 4, 'reset', null, FIXTURE_NOW)!.hint.text).toBe('Tạm dừng tự chuyển đến 17:09 để giữ tài khoản này.');
  });
});
