import { describe, expect, it } from 'vitest';
import { failing, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { popoverView } from './view';
import { windowView } from './window';

const view = (name: string, over = {}) => windowView(fixtureModel(name, over), FIXTURE_NOW);

describe('main window view', () => {
  it('shows the same snapshot as the popover', () => {
    const model = fixtureModel('fresh');
    const w = windowView(model, FIXTURE_NOW);
    const p = popoverView(model, FIXTURE_NOW);
    expect(w.machine).toBe('Mac mini');
    expect(w.fresh).toEqual(p.fresh);
    expect(w.banner).toEqual(p.banner);
    expect(w.claude.rows).toEqual(p.claude.rows);
    expect(w.agy.rows).toEqual(p.agy.rows);
    expect(w.pools.claude.value).toBe(p.pools.claude.value);
  });

  it('spells out how the rotation number adds up', () => {
    const v = view('fresh');
    expect(v.pools.claude).toMatchObject({
      title: 'Hạn mức Claude trong vòng',
      value: '2,3',
      unit: 'tài khoản',
      fill: 'c',
      line: 'Tương đương, cộng phần còn lại · Team A 0% + Team B 100% + Team C 98% + Cá nhân 29%',
    });
    expect(v.pools.agy).toMatchObject({ title: 'Antigravity', value: '94%', fill: 'muted', line: 'Chỉ tài khoản đang dùng' });
    expect(view('agy-pool').pools.agy).toMatchObject({ value: '1,2', fill: 'g', line: 'Tương đương, theo Gemini · Ultra 2 52% + Ultra 1 70%' });
  });

  it('an empty rotation says when it comes back, without the parts', () => {
    expect(view('exhausted').pools.claude).toMatchObject({ value: '0,0', fill: 'bad', line: 'Cạn · có lại lúc 16:40' });
  });

  it('names the Claude cards by what the Mac has', () => {
    expect(view('fresh').claude).toMatchObject({ title: 'Claude Code · 4 tài khoản', tabTitle: '4 tài khoản claude-swap trên máy này' });
    const unavailable = view('unavailable').claude;
    expect(unavailable).toMatchObject({ title: 'Claude Code', tabTitle: 'Tài khoản Claude Code trên máy này', rows: [] });
    expect(unavailable.note?.text).toBe('Không đọc được danh sách tài khoản của claude-swap (usage_schema_mismatch).');
  });

  it('gives every account a freshness chip from its own reading', () => {
    const rows = view('fresh').claude.rows;
    expect(rows.map((r) => r.freshness)).toEqual([
      { t: '2 phút trước', k: '' },
      { t: '40 giây trước', k: '' },
      { t: '4 phút trước', k: '' },
      { t: '5 phút trước', k: '' },
    ]);
    // The freshness column replaces the popover's freshness chip; the other chips stay.
    expect(rows[1].tags.map((c) => c.t)).toEqual(['đang dùng', 'rời vòng 17:45']);

    const stale = view('stale').claude.rows[2];
    expect(stale.freshness).toEqual({ t: '41 phút trước', k: 'stale' });
    expect(stale.tags).toEqual([]);
    expect(stale.chips).toEqual([{ t: 'cập nhật 41 phút trước', k: 'stale' }]);

    const pending = view('reset-pending').claude.rows[3];
    expect(pending.freshness).toEqual({ t: 'chờ số sau mốc đặt lại', k: 'info' });
    expect(pending.tags).toEqual([]);
  });

  it('writes resets in full words for the tables', () => {
    const [personal, teamA, teamB, teamC] = view('fresh').claude.rows;
    expect(personal.lines.map((l) => l.long)).toEqual(['Đặt lại sau 6 ngày 7 giờ 59 phút', 'Đặt lại sau 6 ngày 7 giờ 59 phút']);
    expect(teamA.lines[0].long).toBe('Đặt lại sau 3 giờ 08 phút');
    expect(teamB.lines[0].long).toBe('Chưa bắt đầu tính giờ');
    expect(teamC.lines[1].long).toBe('');
    expect(view('reset-pending').claude.rows[3].lines[0].long).toBe('Đang chờ số mới');
  });

  it('lists what comes next from the readings only', () => {
    expect(view('fresh').upcoming.map((u) => [u.at, u.what, u.tone])).toEqual([
      ['17:09', 'Team C có lại hạn mức 5 giờ', 'ok'],
      ['17:29', 'Team A có lại hạn mức 5 giờ', 'ok'],
      ['17:45', 'Team A rời vòng', 'warn'],
      ['ngày mai 03:20', 'Hạn mức Fable tuần của Team B đặt lại', ''],
    ]);
    // Full allowances are left out, and a list is cut at five.
    const exhausted = view('exhausted').upcoming;
    expect(exhausted).toHaveLength(5);
    expect(exhausted[0]).toMatchObject({ at: '16:40', what: 'Cá nhân có lại hạn mức 5 giờ' });
    // The one account a provider row reads goes by the provider's name.
    expect(view('no-cockpit').upcoming[0]).toMatchObject({ at: '16:18', what: 'Antigravity có lại hạn mức 5 giờ' });
    // Numbers from before quota-axi stopped answering give no times.
    expect(view('fresh', { errors: failing('notFound') }).upcoming).toEqual([]);
  });

  it('the Antigravity tab says how old the other accounts are and that Cockpit runs', () => {
    const a = view('fresh').agy;
    expect(a).toMatchObject({ present: true, title: 'Antigravity · 2 tài khoản', banner: null });
    expect(a.others).toBe(
      'Số của tài khoản không dùng do Cockpit làm mới không đều: Ultra 1 đọc 25 phút trước. App không cảnh báo khi Antigravity xuống thấp.',
    );
    expect(a.cockpit).toEqual([
      { key: 'Cockpit Tools', value: 'đang chạy', chip: { t: 'đang chạy', k: 'ok' } },
      { key: 'Tài khoản trong Cockpit', value: '2' },
    ]);
  });

  it('a Mac without Cockpit sees why only one Antigravity account shows', () => {
    const a = view('no-cockpit').agy;
    expect(a.banner).toEqual({
      tone: 'info',
      title: 'Chưa thấy Cockpit trên máy này',
      body: 'Số Antigravity lấy từ agy, chỉ có tài khoản đang dùng. Cần Cockpit để thấy mọi tài khoản và chuyển.',
      actions: [],
    });
    expect(a.others).toBe('');
    expect(a.cockpit).toEqual([{ key: 'Cockpit Tools', value: 'chưa thấy', chip: { t: 'chưa thấy', k: 'bad' } }]);
    expect(view('unavailable').agy.banner).toMatchObject({ tone: 'warn', title: 'Không đọc được Cockpit' });
  });

  it('the Antigravity tab says when quota-axi fails and stops calling Cockpit running', () => {
    const both = view('fresh', { errors: failing('notFound') });
    expect(both.agy.banner).toEqual(both.banner);
    expect(both.agy.banner).toMatchObject({ tone: 'bad', title: 'Không chạy được quota-axi' });
    expect(both.agy.others).toBe('');
    expect(both.agy.cockpit).toEqual([{ key: 'Cockpit Tools', value: 'không rõ', chip: { t: 'không rõ', k: '' } }]);
    expect(view('no-antigravity', { errors: failing('notFound') }).agy.banner).toMatchObject({ title: 'Không chạy được quota-axi' });

    const agyOnly = view('no-cockpit', { errors: { claude: null, agy: { kind: 'timeout', detail: '' } } }).agy;
    expect(agyOnly.banner).toMatchObject({ title: 'Chưa thấy Cockpit trên máy này' });
    expect(agyOnly.tabNote).toEqual({ tone: 'warn', text: 'Không đọc được số Antigravity mới. Đang hiện số lúc 14:20.' });
    expect(view('fresh').agy.tabNote).toBeNull();
    expect(view('no-cockpit').agy.tabNote).toBeNull();
  });

  it('an undone Antigravity switch takes the tab\'s banner, after a quota-axi failure', () => {
    const overwritten = { busy: null, listed: true, overwritten: { to: 1, from: 2, at: '2026-01-16T07:10:00.000Z', overwrites: 1 } };
    for (const name of ['no-cockpit', 'unavailable']) {
      expect(view(name, { agySwitch: overwritten }).agy.banner).toMatchObject({ tone: 'warn', title: 'Chuyển Antigravity bị ghi đè' });
    }
    const failed = view('no-cockpit', { agySwitch: overwritten, errors: failing('notFound') });
    expect(failed.agy.banner).toMatchObject({ title: 'Không chạy được quota-axi' });
  });

  it('a Mac without Antigravity has no Antigravity card', () => {
    const v = view('no-antigravity');
    expect(v.agy.present).toBe(false);
    expect(v.pools.agy).toMatchObject({ value: '—', line: 'Không có trên máy này' });
  });
});
