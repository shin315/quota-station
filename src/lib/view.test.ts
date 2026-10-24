import { describe, expect, it } from 'vitest';
import { hideNames } from '../fixtures/fakeApi';
import { codexFixture, failing, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { popoverView, type CardView, type NowView } from './view';

const view = (name: string, over = {}, options = {}) => popoverView(fixtureModel(name, over), FIXTURE_NOW, options);
const cells = (c: CardView) => c.cells.map((x) => [x.label, x.text, x.reset]);
const rows = (c: CardView) => c.rows.map((x) => [x.label, x.text, x.reset]);
const now = (n: NowView) => n.rows.map((r) => [r.svc, r.name, r.text, r.unit, r.reset, r.note]);

describe('popover view', () => {
  it('shows a card per account in slot order, the one in use in its place, and Đang dùng for every service', () => {
    const v = view('fresh');
    expect(v.fresh).toEqual({ tone: 'ok', text: 'cập nhật 40 giây trước' });
    expect(v.banner).toBeNull();
    expect(v.claude.cards.map((c) => [c.name, c.sub, c.active])).toEqual([
      ['lanpham.dev', 'Cá nhân · 08–23', false],
      ['lan.acme1', 'Acme Studio', true],
      ['lan.acme2', 'Acme Studio', false],
      ['lan.acme3', 'Acme Studio', false],
    ]);
    // The menu bar's number for each service: the account in use and its five hours, by default.
    expect(now(v.now)).toEqual([
      ['Claude', 'lan.acme1', '74%', '5 giờ', '↻ 3h 8m', ''],
      ['Gemini', 'lan.ultra2', '2%', '5 giờ', '↻ 1h 57m', ''],
      // Codex goes through its own provider: no number, the ChatGPT account's on hover.
      ['Codex', 'relay', '—', '', '', 'ChatGPT đã dùng 23% 5 giờ'],
    ]);
    expect(v.now.rows[0]).toMatchObject({ sub: 'Acme Studio', tone: 'warn', title: 'Claude: lan.acme1 đã dùng 74% 5 giờ', extra: null });
    expect(v.now.rows[0].label).toBe(
      'Claude, lan.acme1. 5 giờ đã dùng 74%, đặt lại sau 3 giờ 08 phút. Tuần đã dùng 39%, đặt lại sau 4 ngày 19 giờ 39 phút. Fable đã dùng 87%, đặt lại sau 1 ngày 17 giờ 59 phút',
    );
    expect(v.now.rows[2].sub).toBe('Codex đi qua relay');
    // No room in accounts anywhere.
    expect(JSON.stringify(v)).not.toMatch(/tài khoản Claude|còn dùng được/);
  });

  it('the popover cards drop windows an account does not have and words that say nothing', () => {
    const v = view('fresh');
    const [personal, acme1, acme2] = v.claude.cards;
    expect(rows(acme1)).toEqual(cells(acme1));
    // An idle window has no `chưa dùng`; a Team seat has no week row at all.
    expect(rows(personal)[0]).toEqual(['5 giờ', '0%', '']);
    expect(rows(acme2).map((r) => r[0])).toEqual(['5 giờ', 'Fable']);
    // Antigravity: Gemini and Claude/GPT, each the tighter of its two windows.
    const [ultra1, ultra2] = v.agy.cards;
    expect(rows(ultra2)).toEqual([
      ['Gemini', '6%', '↻ 3d 23h'],
      ['Claude/GPT', '0%', '↻ 5d 23h'],
    ]);
    expect(rows(ultra1)).toEqual([
      ['Gemini', '37%', '↻ 4d 23h'],
      ['Claude/GPT', '18%', '↻ 5d 3h'],
    ]);
    // The spoken label keeps every window.
    expect(ultra2.label).toContain('Gemini 5 giờ đã dùng 2%');
    expect(ultra2.label).toContain('Claude/GPT tuần đã dùng 0%');
  });

  it('Antigravity’s Claude/GPT from 70 % used shows in Đang dùng, so folding never hides it', () => {
    const m = fixtureModel('fresh');
    const ultra2 = m.snapshot.agy.accounts.find((a) => a.active)!;
    ultra2.windows = ultra2.windows.map((w) => (w.kind === 'claudeGptWeek' ? { ...w, percent: 14 } : w));
    const row = popoverView(m, FIXTURE_NOW).now.rows[1];
    expect(row).toMatchObject({ name: 'lan.ultra2', text: '2%', extra: { text: '86%', tone: 'warn' } });
    expect(row.title).toBe('Gemini: lan.ultra2 đã dùng 2% 5 giờ · Claude/GPT 86%');
    expect(view('fresh').now.rows[1].extra).toBeNull();
  });

  it('Codex on its ChatGPT login shows that account’s number, and no Codex folder no row', () => {
    const chatgpt = view('fresh', { codex: codexFixture({ current: 'chatgpt' }) });
    expect(now(chatgpt.now)[2]).toEqual(['Codex', 'ChatGPT', '23%', '5 giờ', '↻ 2h 9m', '']);
    expect(view('fresh', { codex: codexFixture({ present: false }) }).now.rows.map((r) => r.id)).toEqual(['c', 'g']);
    // Not read yet, or quota-axi failing for Codex: the name and no number.
    const unread = fixtureModel('fresh', { codex: codexFixture({ current: 'chatgpt' }) });
    unread.snapshot.codex = { ...unread.snapshot.codex, status: 'unread', accounts: [] };
    expect(now(popoverView(unread, FIXTURE_NOW).now)[2]).toEqual(['Codex', 'ChatGPT', '—', '', '', 'chưa có số']);
    const err = view('fresh', { codex: codexFixture({ current: 'chatgpt' }), errors: { claude: null, agy: null, codex: { kind: 'timeout', detail: '' } } });
    expect(err.now.rows[2]).toMatchObject({ text: '—', note: 'không đọc được số' });
    // No ChatGPT login on a Mac whose Codex uses it: no row.
    const nologin = view('no-antigravity', { codex: codexFixture({ current: 'chatgpt' }) });
    expect(nologin.now.rows.map((r) => r.id)).toEqual(['c']);
  });

  it('Đang dùng shows the window picked for the menu bar, five hours where a service lacks it, else the tightest', () => {
    const pick = (menubarWindow: string) => now(view('fresh', { menubarWindow, codex: codexFixture({ current: 'chatgpt' }) }).now).map((r) => r.slice(2, 4));
    expect(pick('week')).toEqual([
      ['39%', 'tuần'],
      ['6%', 'tuần'],
      ['41%', 'tuần'],
    ]);
    // Only Claude has Fable tuần: Gemini and Codex show their five hours.
    expect(pick('fableWeek')).toEqual([
      ['87%', 'Fable'],
      ['2%', '5 giờ'],
      ['23%', '5 giờ'],
    ]);
    // The tightest, as before there was a choice.
    expect(pick('tightest')).toEqual([
      ['87%', 'Fable'],
      ['6%', 'tuần'],
      ['41%', 'tuần'],
    ]);
    const tight = view('fresh', { menubarWindow: 'tightest' }).now.rows[0];
    expect(tight.title).toBe('Claude: lan.acme1 đã dùng 87% Fable');
    // A Team seat has no week: its five hours; with only Fable, the tightest.
    const seat = fixtureModel('fresh', { menubarWindow: 'week' });
    const acme1 = seat.snapshot.claude.accounts.find((a) => a.active)!;
    acme1.windows = acme1.windows.filter((w) => w.kind !== 'week');
    expect(now(popoverView(seat, FIXTURE_NOW).now)[0].slice(2, 4)).toEqual(['74%', '5 giờ']);
    acme1.windows = acme1.windows.filter((w) => w.kind === 'fableWeek');
    expect(now(popoverView(seat, FIXTURE_NOW).now)[0].slice(2, 4)).toEqual(['87%', 'Fable']);
  });

  it('a window waiting after its reset leaves Đang dùng without a number', () => {
    const m = fixtureModel('fresh');
    const acme1 = m.snapshot.claude.accounts.find((a) => a.active)!;
    acme1.windows = acme1.windows.map((w) => (w.kind === 'session' ? { ...w, pending: true, percent: null } : w));
    expect(popoverView(m, FIXTURE_NOW).now.rows[0]).toMatchObject({ text: '—', note: 'không có số', used: null });
  });

  it('every window is the share used, with its reset, and an idle five hours reads chưa dùng', () => {
    const [personal, acme1, acme2] = view('fresh').claude.cards;
    expect(cells(acme1)).toEqual([
      ['5 giờ', '74%', '↻ 3h 8m'],
      ['Tuần', '39%', '↻ 4d 19h'],
      ['Fable', '87%', '↻ 1d 17h'],
    ]);
    expect(acme1.cells.map((c) => c.tone)).toEqual(['warn', 'ok', 'warn']);
    expect(acme1.chips).toEqual([{ t: 'rời vòng 17:45', k: 'warn' }]);
    expect(acme1.label).toBe(
      'lan.acme1, Acme Studio, đang dùng. 5 giờ đã dùng 74%, đặt lại sau 3 giờ 08 phút. Tuần đã dùng 39%, đặt lại sau 4 ngày 19 giờ 39 phút. Fable đã dùng 87%, đặt lại sau 1 ngày 17 giờ 59 phút. rời vòng 17:45',
    );
    expect(cells(personal)[0]).toEqual(['5 giờ', '0%', 'chưa dùng']);
    // A Team seat claude-swap gives no week.
    expect(acme2.cells[1]).toMatchObject({ label: 'Tuần', text: '—', none: true, title: 'claude-swap không báo hạn mức tuần cho tài khoản này' });
  });

  it('the tightest window wins, and a tie names the longer window', () => {
    const v = view('fresh');
    expect(v.claude.cards.find((c) => c.active)!.tight).toMatchObject({ label: 'Fable', used: 87, tone: 'warn' });
    const m = fixtureModel('fresh');
    const a = m.snapshot.claude.accounts.find((x) => x.number === 3)!;
    a.windows = a.windows.map((w) => ({ ...w, percent: 60 }));
    const tied = popoverView(m, FIXTURE_NOW).claude.cards.find((c) => c.slot === 3)!;
    expect(tied.tight).toMatchObject({ label: 'Fable', used: 40 });
  });

  it('Antigravity cards have the four windows from the app’s own reading', () => {
    const v = view('fresh');
    const [ultra1, ultra2] = v.agy.cards;
    expect(ultra2).toMatchObject({ name: 'lan.ultra2', active: true });
    expect(cells(ultra2)).toEqual([
      ['Gemini 5 giờ', '2%', '↻ 1h 57m'],
      ['Gemini tuần', '6%', '↻ 3d 23h'],
      ['Claude/GPT 5 giờ', '0%', '↻ 4h 59m'],
      ['Claude/GPT tuần', '0%', '↻ 5d 23h'],
    ]);
    expect(ultra1.name).toBe('lanpham.ultra');
    expect(ultra1.dim).toBe(true);
    expect(ultra1.chips).toEqual([{ t: 'số 25 phút trước', k: 'stale' }]);

  });

  it('accounts only Cockpit Tools reads show its one number per model group', () => {
    const [ultra1, ultra2] = view('cockpit').agy.cards;
    expect(cells(ultra1)).toEqual([
      ['Gemini', '37%', '↻ 4d 23h'],
      ['Claude/GPT', '18%', '↻ 5d 3h'],
    ]);
    // The account in use has its four windows from the live row.
    expect(ultra2.cells).toHaveLength(4);
  });

  it('hidden names fall back to the places, and the folded line follows', () => {
    const v = popoverView(hideNames(fixtureModel('fresh')), FIXTURE_NOW);
    expect(v.claude.cards.map((c) => [c.name, c.sub, c.full])).toEqual([
      ['Cá nhân', '08–23', ''],
      ['Team A', '', ''],
      ['Team B', '', ''],
      ['Team C', '', ''],
    ]);
    expect(v.agy.cards.map((c) => c.name)).toEqual(['Ultra 1', 'Ultra 2']);
    expect(JSON.stringify(v)).not.toContain('@');
    expect(JSON.stringify(v)).not.toContain('Acme');
  });

  it('a Mac without an account list sees only the signed-in Antigravity account', () => {
    const v = view('no-cockpit');
    expect(v.agy.cards.map((c) => c.name)).toEqual(['Tài khoản đang dùng']);
    expect(v.agy.cards[0].cells.map((c) => c.label)).toEqual(['Gemini 5 giờ', 'Gemini tuần', 'Claude/GPT 5 giờ', 'Claude/GPT tuần']);
    expect(v.agy.note?.text).toBe('Chỉ thấy tài khoản đang dùng: chưa có danh sách tài khoản');
  });

  it('asks for Keychain access once and says where the numbers come from', () => {
    const v = view('keychain');
    expect(v.banner).toEqual({
      tone: 'warn',
      title: 'Chưa cấp quyền Keychain cho quota-axi',
      body: 'Dòng Claude của Team A lấy tạm từ Quota Station (số chưa được cập nhật). Cấp quyền một lần để đọc trực tiếp.',
      actions: [
        { label: 'Cấp quyền một lần', act: 'keychain' },
        { label: 'Để sau', act: 'dismiss' },
      ],
    });
    const teamA = v.claude.cards.find((r) => r.name === 'Team A')!;
    expect(teamA.chips.map((c) => c.t)).toContain('số từ Quota Station');
    expect(view('keychain', {}, { keychainLater: true }).banner).toBeNull();
  });

  it('stale numbers keep their age and say the switcher may be stopped', () => {
    const v = view('stale');
    expect(v.fresh).toEqual({ tone: 'warn', text: 'cập nhật 41 phút trước' });
    expect(v.banner).toEqual({
      tone: 'warn',
      title: 'Hạn mức Claude chưa cập nhật trong 41 phút',
      body: 'Quota Station chưa làm mới số từ 13:39. Switcher có thể đang không chạy.',
      actions: [{ label: 'Làm mới', act: 'refresh' }],
    });
    expect(v.claude.cards.every((r) => r.dim)).toBe(true);
    expect(v.claude.cards[1].chips.map((c) => c.t)).toContain('số 41 phút trước');
  });

  it('never shows a number for a slot waiting after its reset', () => {
    const v = view('reset-pending');
    expect(v.banner).toMatchObject({
      tone: 'info',
      title: 'Team C vừa qua mốc đặt lại',
      body: 'Đang chờ Quota Station đọc số mới. App không hiện số khi chưa có số thật.',
    });
    const teamC = v.claude.cards.find((c) => c.slot === 4)!;
    expect(teamC.chips).toContainEqual({ t: 'chờ số sau mốc đặt lại', k: 'info' });
    expect(teamC.cells.filter((c) => !c.none).map((c) => [c.text, c.reset])).toEqual([
      ['—', 'chờ số'],
      ['—', 'chờ số'],
    ]);
  });

  it('says when an empty rotation comes back and who refills it', () => {
    const v = view('exhausted');
    expect(v.banner).toEqual({
      tone: 'bad',
      title: 'Hạn mức Claude trong vòng đã cạn',
      body: 'Có lại lúc 16:40 khi Cá nhân và Team B được đặt lại.',
      actions: [],
    });
    expect(v.now.rows[0]).toMatchObject({ text: '96%', tone: 'bad' });
  });

  it('without a rotation file names accounts by number', () => {
    const v = view('no-rotation');
    expect(v.claude.cards.map((r) => r.name).sort()).toEqual(['Tài khoản 1', 'Tài khoản 2', 'Tài khoản 3', 'Tài khoản 4']);
  });

  it('an unreadable claude-swap and account list say why without guessing numbers', () => {
    const v = view('unavailable');
    expect(v.claude.cards).toEqual([]);
    expect(v.claude.note?.text).toBe('Không đọc được danh sách tài khoản của claude-swap (usage_schema_mismatch).');
    expect(v.agy.note?.text).toBe('Không đọc được danh sách tài khoản: chỉ thấy tài khoản đang dùng');
  });

  it('a Mac without Antigravity has no Antigravity cards', () => {
    const v = view('no-antigravity');
    expect(v.agy.cards).toEqual([]);
    expect(v.now.rows.map((r) => r.svc)).toEqual(['Claude', 'Codex']);
    expect(v.agy.note?.text).toBe('Máy này chưa có Antigravity');
  });

  it('an Antigravity rotation still shows the account in use, not its room', () => {
    const v = view('agy-pool');
    expect(v.now.rows[1]).toMatchObject({ svc: 'Gemini' });
    expect(JSON.stringify(v)).not.toContain('tài khoản Antigravity');
  });

  it('quota-axi missing keeps the last numbers, marked old', () => {
    const v = view('fresh', { errors: failing('notFound') });
    expect(v.fresh).toEqual({ tone: 'bad', text: 'không đọc được số' });
    expect(v.banner).toEqual({
      tone: 'bad',
      title: 'Không chạy được quota-axi',
      body: 'Không tìm thấy lệnh quota-axi trên máy này. Đang hiện số cuối lúc 14:20.',
      actions: [
        { label: 'Chọn đường dẫn…', act: 'settings' },
        { label: 'Thử lại', act: 'refresh' },
      ],
    });
    // Đang dùng never shows an old number as a new one.
    expect(v.now.rows[0]).toMatchObject({ name: 'lan.acme1', text: '—', note: 'không đọc được số' });
    expect(v.claude.cards.every((r) => r.dim)).toBe(true);
  });

  it('other quota-axi failures name the reason', () => {
    const v = view('fresh', { errors: failing('timeout', 'quota-axi did not answer within 45s') });
    expect(v.banner?.body).toBe('quota-axi không chạy được: quá thời gian chờ. Đang hiện số cuối lúc 14:20.');
  });

  it('a provider that stopped answering keeps the time of its own last numbers', () => {
    const agyOld = fixtureModel('fresh', { errors: { claude: null, agy: { kind: 'timeout', detail: '' }, codex: null } });
    agyOld.snapshot.agy.generatedAt = '2026-01-16T06:50:00.000Z';
    const a = popoverView(agyOld, FIXTURE_NOW);
    expect(a.agy.note).toEqual({ tone: 'warn', text: 'Không đọc được số Antigravity mới. Đang hiện số lúc 13:50.' });
    expect(a.agy.cards.every((c) => c.dim)).toBe(true);

    const claudeOld = fixtureModel('fresh', { errors: { claude: { kind: 'timeout', detail: '' }, agy: null, codex: null } });
    claudeOld.snapshot.claude.generatedAt = '2026-01-16T06:50:00.000Z';
    const c = popoverView(claudeOld, FIXTURE_NOW);
    expect(c.banner?.body).toBe('quota-axi không chạy được: quá thời gian chờ. Đang hiện số cuối lúc 13:50.');
    expect(c.now.rows[0].note).toBe('không đọc được số');
    expect(a.now.rows[1].note).toBe('không đọc được số');
  });

  it('before the first read says it is reading', () => {
    const m = fixtureModel('fresh', { reading: true });
    m.snapshot.claude = { ...m.snapshot.claude, status: 'unread', accounts: [], newestReadingAt: null };
    m.snapshot.agy = { ...m.snapshot.agy, status: 'unread', accounts: [], newestReadingAt: null };
    expect(popoverView(m, FIXTURE_NOW).fresh).toEqual({ tone: 'muted', text: 'đang đọc số…' });
  });
});
