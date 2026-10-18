import { describe, expect, it } from 'vitest';
import { failing, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { popoverView } from './view';

const view = (name: string, over = {}, options = {}) => popoverView(fixtureModel(name, over), FIXTURE_NOW, options);

describe('popover view', () => {
  it('shows the whole rotation and every account when fresh', () => {
    const v = view('fresh');
    expect(v.fresh).toEqual({ tone: 'ok', text: 'cập nhật 40 giây trước' });
    expect(v.summary.map((s) => s.t)).toEqual(['Claude còn tương đương 2,3 tài khoản', 'Antigravity còn 94% Gemini']);
    expect(v.banner).toBeNull();
    expect(v.pools.claude).toMatchObject({
      label: 'Claude trong vòng',
      value: '2,3',
      unit: 'tài khoản',
      sub: 'tương đương, cộng phần còn lại',
      tone: 'ok',
    });
    expect(v.pools.claude.title).toBe(
      'Cộng phần còn lại trước ngưỡng tự chuyển: Team A 0% + Team B 100% + Team C 98% + Cá nhân 29% ≈ 2,3 tài khoản.',
    );
    expect(v.claude.side).toBe('4 tài khoản · đang dùng Team A');
    expect(v.claude.rows.map((r) => r.name)).toEqual(['Cá nhân', 'Team A', 'Team B', 'Team C']);
  });

  it('draws two allowance lines per Claude account with their resets', () => {
    const [personal, teamA, teamB] = view('fresh').claude.rows;
    expect(personal.chips.map((c) => c.t)).toEqual(['08–23']);
    expect(personal.lines.map((l) => [l.label, l.text, l.reset])).toEqual([
      ['Tuần', '100%', '↻ 6n 7g'],
      ['Fable tuần', '96%', '↻ 6n 7g'],
    ]);
    expect(teamA.active).toBe(true);
    expect(teamA.chips).toEqual([
      { t: 'đang dùng', k: 'on' },
      { t: 'rời vòng 17:45', k: 'warn' },
    ]);
    expect(teamA.lines.map((l) => [l.label, l.text, l.reset, l.tone])).toEqual([
      ['5 giờ', '26%', '↻ 3:08', 'warn'],
      ['Fable tuần', '13%', '↻ 1n 17g', 'warn'],
    ]);
    expect(teamA.label).toBe(
      'Team A (claude-swap, tài khoản 2), đang dùng. Hạn mức 5 giờ còn 26%, đặt lại sau 3 giờ 08 phút. Hạn mức Fable tuần còn 13%, đặt lại sau 1 ngày 17 giờ 59 phút. rời vòng 17:45',
    );
    expect(teamB.lines[0]).toMatchObject({ label: '5 giờ', text: '100%', reset: 'chưa chạy' });
  });

  it('marks a Fable number that is only a lower bound', () => {
    const teamC = view('fresh').claude.rows[3];
    expect(teamC.lines[1]).toMatchObject({ label: 'Fable tuần', text: '≥98%', reset: '' });
    expect(teamC.lines[1].words).toBe('Hạn mức Fable tuần còn ít nhất 98%');
    expect(teamC.lines[1].title).toContain('còn ít nhất 98%');
  });

  it('lists Cockpit accounts and marks an old reading', () => {
    const v = view('fresh');
    expect(v.agy.side).toBe('đang dùng Ultra 2');
    const [u1, u2] = v.agy.rows;
    expect(u1.name).toBe('Ultra 1');
    expect(u1.dim).toBe(true);
    expect(u1.chips).toEqual([{ t: 'cập nhật 25 phút trước', k: 'stale' }]);
    expect(u2.lines.map((l) => [l.label, l.text, l.reset])).toEqual([
      ['Gemini', '94%', '↻ 3n 23g'],
      ['Claude/GPT', '—', 'chờ số'],
    ]);
  });

  it('a Mac without Cockpit sees only the signed-in Antigravity account', () => {
    const v = view('no-cockpit');
    expect(v.agy.rows.map((r) => [r.name, r.short])).toEqual([['Tài khoản đang dùng', 'G']]);
    expect(v.agy.rows[0].lines.map((l) => l.label)).toEqual(['5 giờ', 'Tuần']);
    expect(v.agy.note?.text).toBe('Máy này chưa có Cockpit: chỉ thấy tài khoản đang dùng, chưa chuyển được');
    expect(v.pools.agy).toMatchObject({ value: '94%', unit: 'tuần', sub: 'chỉ tài khoản đang dùng', tone: 'muted' });
    expect(v.summary[1].t).toBe('Antigravity còn 94% tuần');
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
    const teamA = v.claude.rows.find((r) => r.name === 'Team A')!;
    expect(teamA.chips.map((c) => c.t)).toContain('số từ claude-swap');
    expect(v.pools.claude).toMatchObject({ label: 'Claude', value: '27%', unit: '5 giờ', sub: 'chỉ tài khoản đang dùng' });
    expect(v.summary[0].t).toBe('Claude còn 27% 5 giờ');
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
    expect(v.summary[0].t).toBe('Hạn mức Claude chưa cập nhật trong 41 phút');
    expect(v.claude.rows.every((r) => r.dim)).toBe(true);
    expect(v.claude.rows[0].chips.map((c) => c.t)).toContain('cập nhật 41 phút trước');
  });

  it('never shows 100% for a slot waiting after its reset', () => {
    const v = view('reset-pending');
    expect(v.banner).toMatchObject({
      tone: 'info',
      title: 'Team C vừa qua mốc đặt lại',
      body: 'Đang chờ claude-swap đọc số mới. App không hiện 100% khi chưa có số thật.',
    });
    const teamC = v.claude.rows[3];
    expect(teamC.chips).toContainEqual({ t: 'chờ số sau mốc đặt lại', k: 'info' });
    expect(teamC.lines.map((l) => [l.text, l.reset])).toEqual([
      ['—', 'chờ số'],
      ['—', 'chờ số'],
    ]);
  });

  it('says when an empty rotation comes back and who refills it', () => {
    const v = view('exhausted');
    expect(v.summary[0]).toEqual({ t: 'Claude cạn đến 16:40', bad: true });
    expect(v.banner).toEqual({
      tone: 'bad',
      title: 'Hạn mức Claude trong vòng đã cạn',
      body: 'Có lại lúc 16:40 khi Cá nhân và Team B được đặt lại.',
      actions: [],
    });
    expect(v.pools.claude).toMatchObject({ value: '0,0', sub: 'cạn · có lại lúc 16:40', tone: 'bad' });
  });

  it('without a rotation file names accounts by number and shows the active slot', () => {
    const v = view('no-rotation');
    expect(v.claude.rows.map((r) => r.name)).toEqual(['Tài khoản 1', 'Tài khoản 2', 'Tài khoản 3', 'Tài khoản 4']);
    expect(v.pools.claude).toMatchObject({ value: '26%', sub: 'chỉ tài khoản đang dùng' });
  });

  it('an unreadable claude-swap and Cockpit say why without guessing numbers', () => {
    const v = view('unavailable');
    expect(v.claude.rows).toEqual([]);
    expect(v.claude.note?.text).toBe('Không đọc được danh sách tài khoản của claude-swap (usage_schema_mismatch).');
    expect(v.pools.claude).toMatchObject({ value: '—', sub: 'chưa có số' });
    expect(v.summary[0].t).toBe('Chưa có số Claude');
    expect(v.agy.note?.text).toBe('Không đọc được Cockpit: chỉ thấy tài khoản đang dùng, chưa chuyển được');
  });

  it('a Mac without Antigravity has no Antigravity rows', () => {
    const v = view('no-antigravity');
    expect(v.agy.rows).toEqual([]);
    expect(v.pools.agy).toMatchObject({ value: '—', sub: 'không có trên máy này' });
    expect(v.agy.note?.text).toBe('Máy này chưa có Antigravity');
    expect(v.summary).toHaveLength(1);
  });

  it('an agy rotation counts room in accounts', () => {
    const v = view('agy-pool');
    expect(v.pools.agy).toMatchObject({ value: '1,2', unit: 'tài khoản', sub: 'tương đương, theo Gemini', tone: 'ok' });
    expect(v.summary[1].t).toBe('Antigravity còn tương đương 1,2 tài khoản');
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
    expect(v.summary).toEqual([{ t: 'Không đọc được số mới' }]);
    expect(v.pools.claude).toMatchObject({ sub: 'số lúc 14:20', tone: 'muted' });
    expect(v.claude.rows.every((r) => r.dim)).toBe(true);
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
    expect(a.pools.agy).toMatchObject({ sub: 'số lúc 13:50', tone: 'muted' });

    const claudeOld = fixtureModel('fresh', { errors: { claude: { kind: 'timeout', detail: '' }, agy: null } });
    claudeOld.snapshot.claude.generatedAt = '2026-01-16T06:50:00.000Z';
    const c = popoverView(claudeOld, FIXTURE_NOW);
    expect(c.banner?.body).toBe('quota-axi không chạy được: quá thời gian chờ. Đang hiện số cuối lúc 13:50.');
    expect(c.pools.claude).toMatchObject({ sub: 'số lúc 13:50', tone: 'muted' });
  });

  it('before the first read says it is reading', () => {
    const m = fixtureModel('fresh', { reading: true });
    m.snapshot.claude = { ...m.snapshot.claude, status: 'unread', accounts: [], newestReadingAt: null };
    m.snapshot.agy = { ...m.snapshot.agy, status: 'unread', accounts: [], newestReadingAt: null };
    expect(popoverView(m, FIXTURE_NOW).fresh).toEqual({ tone: 'muted', text: 'đang đọc số…' });
  });
});
