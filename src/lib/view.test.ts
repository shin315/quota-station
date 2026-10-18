import { describe, expect, it } from 'vitest';
import { hideNames } from '../fixtures/fakeApi';
import { failing, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { popoverView, type CardView } from './view';

const view = (name: string, over = {}, options = {}) => popoverView(fixtureModel(name, over), FIXTURE_NOW, options);
const cells = (c: CardView) => c.cells.map((x) => [x.label, x.text, x.reset]);

describe('popover view', () => {
  it('shows a card per account, the one in use first, and the room left in the rotation', () => {
    const v = view('fresh');
    expect(v.fresh).toEqual({ tone: 'ok', text: 'cập nhật 40 giây trước' });
    expect(v.banner).toBeNull();
    expect(v.claude.cards.map((c) => [c.name, c.sub, c.active])).toEqual([
      ['lan.acme1', 'Acme Studio', true],
      ['lanpham.dev', 'Cá nhân · 08–23', false],
      ['lan.acme2', 'Acme Studio', false],
      ['lan.acme3', 'Acme Studio', false],
    ]);
    expect(v.claude.room).toMatchObject({ value: '2,3', unit: 'tài khoản Claude', sub: 'còn dùng được trước ngưỡng 90%', mark: 90, tone: 'ok' });
    expect(v.claude.room!.segs.map((s) => [s.name, s.used, s.tone])).toEqual([
      ['lan.acme1', 87, 'warn'],
      ['lan.acme2', 50, 'ok'],
      ['lan.acme3', 2, 'ok'],
      ['lanpham.dev', 4, 'ok'],
    ]);
    expect(v.claude.room!.title).toBe('lan.acme1 đã dùng 87% Fable · lan.acme2 đã dùng 50% Fable · lan.acme3 đã dùng 2% Fable · lanpham.dev đã dùng 4% Fable');
  });

  it('every window is the share used, with its reset, and an idle five hours reads chưa dùng', () => {
    const [acme1, personal, acme2] = view('fresh').claude.cards;
    expect(cells(acme1)).toEqual([
      ['5 giờ', '74%', '↻ 3:08'],
      ['Tuần', '39%', '↻ 4n 19g'],
      ['Fable', '87%', '↻ 1n 17g'],
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
    expect(v.claude.folded).toEqual({ name: 'lan.acme1', parts: [{ label: 'Fable', text: '87%', tone: 'warn' }] });
    const m = fixtureModel('fresh');
    const a = m.snapshot.claude.accounts.find((x) => x.number === 3)!;
    a.windows = a.windows.map((w) => ({ ...w, percent: 60 }));
    const tied = popoverView(m, FIXTURE_NOW).claude.cards.find((c) => c.slot === 3)!;
    expect(tied.tight).toMatchObject({ label: 'Fable', used: 40 });
  });

  it('Antigravity cards have the four windows from the app’s own reading', () => {
    const v = view('fresh');
    const [ultra2, ultra1] = v.agy.cards;
    expect(ultra2).toMatchObject({ name: 'lan.ultra2', active: true });
    expect(cells(ultra2)).toEqual([
      ['Gemini 5 giờ', '2%', '↻ 1:57'],
      ['Gemini tuần', '6%', '↻ 3n 23g'],
      ['Claude/GPT 5 giờ', '0%', '↻ 4:59'],
      ['Claude/GPT tuần', '0%', '↻ 5n 23g'],
    ]);
    expect(ultra1.name).toBe('lanpham.ultra');
    expect(ultra1.dim).toBe(true);
    expect(ultra1.chips).toEqual([{ t: 'số 25 phút trước', k: 'stale' }]);
    expect(v.agy.folded).toEqual({
      name: 'lan.ultra2',
      parts: [
        { label: 'Gemini', text: '6%', tone: 'ok' },
        { label: 'C/GPT', text: '0%', tone: 'ok' },
      ],
    });
  });

  it('accounts only Cockpit Tools reads show its one number per model group', () => {
    const [ultra2, ultra1] = view('cockpit').agy.cards;
    expect(cells(ultra1)).toEqual([
      ['Gemini', '37%', '↻ 4n 23g'],
      ['Claude/GPT', '18%', '↻ 5n 3g'],
    ]);
    // The account in use has its four windows from the live row.
    expect(ultra2.cells).toHaveLength(4);
  });

  it('hidden names fall back to the places, and the folded line follows', () => {
    const v = popoverView(hideNames(fixtureModel('fresh')), FIXTURE_NOW);
    expect(v.claude.cards.map((c) => [c.name, c.sub, c.full])).toEqual([
      ['Team A', '', ''],
      ['Cá nhân', '08–23', ''],
      ['Team B', '', ''],
      ['Team C', '', ''],
    ]);
    expect(v.agy.cards.map((c) => c.name)).toEqual(['Ultra 2', 'Ultra 1']);
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
      body: 'Dòng Claude của Team A lấy tạm từ claude-swap (số chưa được cập nhật). Cấp quyền một lần để đọc trực tiếp.',
      actions: [
        { label: 'Cấp quyền một lần', act: 'keychain' },
        { label: 'Để sau', act: 'dismiss' },
      ],
    });
    const teamA = v.claude.cards.find((r) => r.name === 'Team A')!;
    expect(teamA.chips.map((c) => c.t)).toContain('số từ claude-swap');
    expect(v.claude.room).toBeNull();
    expect(view('keychain', {}, { keychainLater: true }).banner).toBeNull();
  });

  it('stale numbers keep their age and say the switcher may be stopped', () => {
    const v = view('stale');
    expect(v.fresh).toEqual({ tone: 'warn', text: 'cập nhật 41 phút trước' });
    expect(v.banner).toEqual({
      tone: 'warn',
      title: 'Hạn mức Claude chưa cập nhật trong 41 phút',
      body: 'claude-swap chưa làm mới số từ 13:39. Switcher có thể đang không chạy.',
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
      body: 'Đang chờ claude-swap đọc số mới. App không hiện số khi chưa có số thật.',
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
    expect(v.claude.room).toMatchObject({ value: '0,0', sub: 'cạn · có lại lúc 16:40', tone: 'bad' });
  });

  it('without a rotation file names accounts by number', () => {
    const v = view('no-rotation');
    expect(v.claude.cards.map((r) => r.name).sort()).toEqual(['Tài khoản 1', 'Tài khoản 2', 'Tài khoản 3', 'Tài khoản 4']);
    expect(v.claude.room).toBeNull();
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
    expect(v.agy.folded).toBeNull();
    expect(v.agy.note?.text).toBe('Máy này chưa có Antigravity');
  });

  it('an agy rotation counts room in accounts', () => {
    const v = view('agy-pool');
    expect(v.agy.room).toMatchObject({ value: '1,2', unit: 'tài khoản Antigravity', mark: null, tone: 'ok' });
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
    expect(v.claude.room).toMatchObject({ sub: 'số lúc 14:20', tone: 'muted' });
    expect(v.claude.cards.every((r) => r.dim)).toBe(true);
  });

  it('other quota-axi failures name the reason', () => {
    const v = view('fresh', { errors: failing('timeout', 'quota-axi did not answer within 45s') });
    expect(v.banner?.body).toBe('quota-axi không chạy được: quá thời gian chờ. Đang hiện số cuối lúc 14:20.');
  });

  it('a provider that stopped answering keeps the time of its own last numbers', () => {
    const agyOld = fixtureModel('fresh', { errors: { claude: null, agy: { kind: 'timeout', detail: '' } } });
    agyOld.snapshot.agy.generatedAt = '2026-01-16T06:50:00.000Z';
    const a = popoverView(agyOld, FIXTURE_NOW);
    expect(a.agy.note).toEqual({ tone: 'warn', text: 'Không đọc được số Antigravity mới. Đang hiện số lúc 13:50.' });
    expect(a.agy.cards.every((c) => c.dim)).toBe(true);

    const claudeOld = fixtureModel('fresh', { errors: { claude: { kind: 'timeout', detail: '' }, agy: null } });
    claudeOld.snapshot.claude.generatedAt = '2026-01-16T06:50:00.000Z';
    const c = popoverView(claudeOld, FIXTURE_NOW);
    expect(c.banner?.body).toBe('quota-axi không chạy được: quá thời gian chờ. Đang hiện số cuối lúc 13:50.');
    expect(c.claude.room).toMatchObject({ sub: 'số lúc 13:50', tone: 'muted' });
  });

  it('before the first read says it is reading', () => {
    const m = fixtureModel('fresh', { reading: true });
    m.snapshot.claude = { ...m.snapshot.claude, status: 'unread', accounts: [], newestReadingAt: null };
    m.snapshot.agy = { ...m.snapshot.agy, status: 'unread', accounts: [], newestReadingAt: null };
    expect(popoverView(m, FIXTURE_NOW).fresh).toEqual({ tone: 'muted', text: 'đang đọc số…' });
  });
});
