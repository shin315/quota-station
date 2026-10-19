import { describe, expect, it } from 'vitest';
import { failing, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { popoverView } from './view';
import { windowView } from './window';

const view = (name: string, over = {}) => windowView(fixtureModel(name, over), FIXTURE_NOW);

describe('main window view', () => {
  it('shows the same snapshot as the popover, both in slot order', () => {
    const model = fixtureModel('fresh');
    const w = windowView(model, FIXTURE_NOW);
    const p = popoverView(model, FIXTURE_NOW);
    expect(w.machine).toBe('Mac mini');
    expect(w.fresh).toEqual(p.fresh);
    expect(w.banner).toEqual(p.banner);
    // Đang dùng on Tổng quan is the popover's, with the same numbers.
    expect(w.now).toEqual(p.now);
    expect(w.claude.cards.map((c) => c.slot)).toEqual([1, 2, 3, 4]);
    // The popover lists the accounts in the same order, the one in use in its place.
    expect(w.claude.cards).toEqual(p.claude.cards);
    expect(w.agy.cards.map((c) => c.name)).toEqual(['lanpham.ultra', 'lan.ultra2']);
    expect(w.agy.cards).toEqual(p.agy.cards);
  });

  it('counts the Claude accounts by what the Mac has', () => {
    expect(view('fresh').claude.count).toBe('4 tài khoản claude-swap');
    const unavailable = view('unavailable').claude;
    expect(unavailable).toMatchObject({ count: '', cards: [] });
    expect(unavailable.note?.text).toBe('Không đọc được danh sách tài khoản của claude-swap (usage_schema_mismatch).');
  });

  it('writes resets in the tables as the popover does, minutes only within a day', () => {
    const [personal, acme1, acme2] = view('fresh').claude.cards;
    expect(personal.cells.map((c) => c.reset)).toEqual(['chưa dùng', '↻ 6d 7h', '↻ 6d 7h']);
    expect(acme1.cells[0].reset).toBe('↻ 3h 8m');
    expect(acme2.cells[1]).toMatchObject({ none: true, reset: 'không có' });
    expect(view('reset-pending').claude.cards[3].cells[0].reset).toBe('chờ số');
  });

  it('lists what comes next from the readings only', () => {
    expect(view('fresh').upcoming.map((u) => [u.at, u.what, u.tone])).toEqual([
      ['16:18', 'lan.ultra2 có lại hạn mức Gemini 5 giờ', 'ok'],
      ['17:00', 'lanpham.ultra có lại hạn mức Claude/GPT 5 giờ', 'ok'],
      ['17:09', 'lan.acme3 có lại hạn mức 5 giờ', 'ok'],
      ['17:29', 'lan.acme1 có lại hạn mức 5 giờ', 'ok'],
      ['17:45', 'lan.acme1 rời vòng', 'warn'],
    ]);
    // Windows nothing was used of are left out, and a list is cut at five.
    const exhausted = view('exhausted').upcoming;
    expect(exhausted).toHaveLength(5);
    expect(exhausted).toContainEqual(expect.objectContaining({ at: '16:40', what: 'Cá nhân có lại hạn mức 5 giờ' }));
    // The one account a provider row reads goes by the provider's name.
    expect(view('no-cockpit').upcoming[0]).toMatchObject({ at: '16:18', what: 'Antigravity có lại hạn mức Gemini 5 giờ' });
    // Numbers from before quota-axi stopped answering give no times.
    expect(view('fresh', { errors: failing('notFound') }).upcoming).toEqual([]);
  });

  it('shows the Cockpit Tools card only where the accounts come from Cockpit Tools', () => {
    const own = view('fresh').agy;
    expect(own).toMatchObject({ present: true, banner: null, own: true, cockpit: null });
    const m = fixtureModel('cockpit');
    m.snapshot.agy.source = 'cockpit';
    const cockpit = windowView(m, FIXTURE_NOW).agy;
    expect(cockpit.own).toBe(false);
    expect(cockpit.cockpit).toEqual([
      { key: 'Cockpit Tools', value: 'đang chạy', chip: { t: 'đang chạy', k: 'ok' } },
      { key: 'Tài khoản trong Cockpit Tools', value: '2' },
    ]);
    // quota-axi from before it said whose files it read: no card either.
    expect(view('cockpit').agy.cockpit).toBeNull();
  });

  it('a Mac without an account list sees why only one Antigravity account shows', () => {
    const a = view('no-cockpit').agy;
    expect(a.banner).toEqual({
      tone: 'info',
      title: 'Chỉ thấy tài khoản đang dùng',
      body: 'quota-axi chưa có danh sách tài khoản Antigravity trên máy này. Đăng nhập tài khoản ở đây để thấy và chuyển mọi tài khoản.',
      actions: [],
    });
    expect(a.cockpit).toBeNull();
    expect(view('unavailable').agy.banner).toMatchObject({ tone: 'warn', title: 'Không đọc được danh sách tài khoản' });
  });

  it('the Antigravity tab says when quota-axi fails', () => {
    const both = view('fresh', { errors: failing('notFound') });
    expect(both.agy.banner).toEqual(both.banner);
    expect(both.agy.banner).toMatchObject({ tone: 'bad', title: 'Không chạy được quota-axi' });
    expect(view('no-antigravity', { errors: failing('notFound') }).agy.banner).toMatchObject({ title: 'Không chạy được quota-axi' });

    const agyOnly = view('no-cockpit', { errors: { claude: null, agy: { kind: 'timeout', detail: '' } } }).agy;
    expect(agyOnly.banner).toMatchObject({ title: 'Chỉ thấy tài khoản đang dùng' });
    expect(agyOnly.tabNote).toEqual({ tone: 'warn', text: 'Không đọc được số Antigravity mới. Đang hiện số lúc 14:20.' });
    expect(view('fresh').agy.tabNote).toBeNull();
  });

  it("an undone Antigravity switch takes the tab's banner, after a quota-axi failure", () => {
    const overwritten = { busy: null, listed: true, overwritten: { to: 1, from: 2, at: '2026-01-16T07:10:00.000Z', overwrites: 1 } };
    for (const name of ['no-cockpit', 'unavailable']) {
      expect(view(name, { agySwitch: overwritten }).agy.banner).toMatchObject({ tone: 'warn', title: 'Chuyển Antigravity bị ghi đè' });
    }
    const failed = view('no-cockpit', { agySwitch: overwritten, errors: failing('notFound') });
    expect(failed.agy.banner).toMatchObject({ title: 'Không chạy được quota-axi' });
  });

  it('the switcher card is one line, the rest folded', () => {
    const s = view('fresh').switcher;
    expect(s.line).toMatch(/^Chạy mỗi 3 phút · kiểm lúc \d\d:\d\d$/);
    expect(s.tone).toBe('ok');
    expect(s.details.map((d) => d.key)).toContain('LaunchAgent cswap-prefer-team');
    expect(s.details.find((d) => d.key.startsWith('Quyết định gần nhất'))).toMatchObject({ mono: true });
  });

  it('a Mac without Antigravity has no Antigravity card', () => {
    expect(view('no-antigravity').agy.present).toBe(false);
  });
});
