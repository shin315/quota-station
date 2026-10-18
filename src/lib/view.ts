// Turns the model into what the popover shows; the main window builds on
// the same view (`window.ts`). Pure: the same model and
// clock give the same view, so Vitest covers every state without a window.
// Copy follows the approved prototype (Hướng 1 · Kính macOS, bước 0.7).

import { age, clock, hours, minutesUntil, percent, resetShort, resetWords, room, secondsSince } from './format';
import type { Account, ErrorView, Line, LineKind, Model, Pool, Section } from './model';
import { pausedSummary, switcherBanner } from './switching';

export type Tone = 'ok' | 'warn' | 'bad' | 'muted' | 'info';
export type Action =
  | 'refresh'
  | 'settings'
  | 'window'
  | 'keychain'
  | 'dismiss'
  /** Removes the pause flag. */
  | 'resume'
  /** One more hour on a timed pause. */
  | 'addHour'
  | 'closeFeedback'
  | 'guide:auto'
  | 'guide:switcher'
  /** Opens the switch sheet for a Claude slot. */
  | `switch:${number}`
  /** The rotation editor's banners and feedback (M5): `rules:retry`, `rules:adopt:<alias>`, … */
  | `rules:${string}`;

export interface ChipView {
  t: string;
  k: '' | 'on' | 'ok' | 'warn' | 'bad' | 'stale' | 'info';
}

export interface LineView {
  label: string;
  /** Meter fill and percent text; null draws an empty meter and `—`. */
  percent: number | null;
  text: string;
  reset: string;
  tone: 'ok' | 'warn' | 'bad' | 'muted';
  words: string;
  /** The reset in full words for the window tables: `Đặt lại sau 3 giờ 09 phút`. */
  long: string;
  /** Hover text, when the number needs a word of explanation. */
  title: string;
}

export interface RowView {
  key: string;
  name: string;
  short: string;
  kind: 'c' | 'g';
  active: boolean;
  dim: boolean;
  /** Every chip the popover shows: `tags` and then `freshness`. */
  chips: ChipView[];
  /** Role, rotation and source chips, without the freshness chip. */
  tags: ChipView[];
  /** How fresh the account's reading is, `40 giây trước`: its own age, never the app's last call. */
  freshness: ChipView | null;
  lines: LineView[];
  label: string;
  title: string;
  /** claude-swap slot or Cockpit account number. */
  slot: number;
  /** A Claude account the owner can switch to now. */
  switchable: boolean;
  /** Why this row cannot be switched to; '' when it can, or when it is the active one. */
  off: string;
}

export interface PoolView {
  label: string;
  kind: 'c' | 'g';
  value: string;
  unit: string | null;
  percent: number;
  sub: string;
  tone: 'ok' | 'bad' | 'muted';
  title: string;
  /** Rotation: each counted account's room, `Team A 27%`. */
  parts: string[];
}

export interface BannerView {
  tone: 'warn' | 'bad' | 'info';
  title: string;
  body: string;
  actions: { label: string; act: Action }[];
}

export interface NoteView {
  tone: '' | 'warn' | 'bad';
  text: string;
}

export interface SectionView {
  side: string;
  rows: RowView[];
  note: NoteView | null;
}

export interface PopoverView {
  machine: string;
  fresh: { tone: Tone; text: string };
  summary: { t: string; bad?: boolean }[];
  banner: BannerView | null;
  pools: { claude: PoolView; agy: PoolView };
  claude: SectionView;
  agy: SectionView;
}

export interface ViewOptions {
  /** "Để sau" was pressed on the Keychain banner. */
  keychainLater?: boolean;
}

const STALE_SECONDS = 900;

const LINE_LABEL: Record<LineKind, string> = {
  session: '5 giờ',
  week: 'Tuần',
  fableWeek: 'Fable tuần',
  gemini: 'Gemini',
  claudeGpt: 'Claude/GPT',
};

/** The unit after an active account's percent: `27% 5 giờ`, `94% tuần`. */
function unitOf(kind: LineKind | null): string {
  return kind === 'week' || kind === 'fableWeek' ? 'tuần' : kind === 'session' ? '5 giờ' : '';
}

function meterTone(p: number | null): LineView['tone'] {
  return p == null ? 'muted' : p <= 10 ? 'bad' : p <= 30 ? 'warn' : 'ok';
}

// ===== Names =====

const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';

/** Claude accounts: Cá nhân, Team A, Team B… by slot order; `Tài khoản N` without a rotation file. */
export function claudeNames(accounts: Account[]): Map<number, { name: string; short: string }> {
  const out = new Map<number, { name: string; short: string }>();
  const team = accounts.filter((a) => a.role === 'team').map((a) => a.number).sort((x, y) => x - y);
  for (const a of accounts) {
    if (a.number === 0) out.set(0, { name: 'Tài khoản đang dùng', short: 'C' });
    else if (a.role === 'personal') out.set(a.number, { name: 'Cá nhân', short: 'CN' });
    else if (a.role === 'team') {
      const l = LETTERS[team.indexOf(a.number)] ?? String(a.number);
      out.set(a.number, { name: 'Team ' + l, short: l });
    } else out.set(a.number, { name: 'Tài khoản ' + a.number, short: String(a.number) });
  }
  return out;
}

export function agyName(a: Account): { name: string; short: string } {
  return a.number === 0 ? { name: 'Tài khoản đang dùng', short: 'G' } : { name: 'Ultra ' + a.number, short: 'U' + a.number };
}

// ===== Lines and rows =====

function lineView(line: Line, now: Date): LineView {
  const label = LINE_LABEL[line.kind];
  const mins = minutesUntil(line.resetsAt, now);
  let reset = '';
  let words = '';
  if (line.pending || (line.percent != null && mins != null && mins <= 0)) {
    reset = 'chờ số';
    words = 'đang chờ số mới';
  } else if (mins != null) {
    reset = '↻ ' + resetShort(mins);
    words = resetWords(mins);
  } else if (line.percent === 100 && !line.atLeast) {
    reset = 'chưa chạy';
    words = 'chưa bắt đầu tính giờ';
  }
  const p = line.pending ? null : line.percent;
  const atLeast = p != null && line.atLeast;
  return {
    label,
    percent: p,
    text: p == null ? '—' : (atLeast ? '≥' : '') + percent(p),
    reset,
    tone: meterTone(p),
    long: words ? words.charAt(0).toUpperCase() + words.slice(1) : '',
    words:
      'Hạn mức ' +
      label +
      ' ' +
      (p == null ? 'chưa có số' : (atLeast ? 'còn ít nhất ' : 'còn ') + percent(p)) +
      (words ? ', ' + words : ''),
    title: atLeast ? label + ' còn ít nhất ' + percent(p) + ': hạn mức của tài khoản đang giới hạn trước.' : '',
  };
}

interface RowContext {
  now: Date;
  /** Every number is the last one before quota-axi stopped answering. */
  old: boolean;
  keychainRequired: boolean;
  personalHours: string | null;
  /** Claude rows: what switching needs. Null for Antigravity, which switches in a later milestone. */
  switching: { listed: boolean; cswap: boolean; busy: number | null } | null;
}

function offReason(a: Account, ctx: RowContext): string {
  const sw = ctx.switching;
  if (a.active) return '';
  if (!sw) return 'Chuyển Antigravity có ở bản sau';
  if (!sw.listed || a.number === 0) return '';
  if (sw.busy != null) return 'Đang chuyển tài khoản';
  if (!sw.cswap) return 'Chưa chuyển được: không tìm thấy cswap';
  if (a.disabled) return 'Tài khoản đã tắt trong claude-swap';
  return '';
}

function rowView(a: Account, kind: 'c' | 'g', names: { name: string; short: string }, ctx: RowContext): RowView {
  const chips: ChipView[] = [];
  if (a.active) chips.push({ t: 'đang dùng', k: 'on' });
  if (kind === 'c') {
    if (a.role === 'personal' && a.membership === 'in') {
      const h = hours(ctx.personalHours);
      if (h) chips.push({ t: h, k: '' });
    }
    if (a.membership === 'out') chips.push({ t: 'ngoài vòng', k: '' });
    if (a.membership === 'ended') chips.push({ t: 'đã rời vòng', k: '' });
    if (a.leavesAt) chips.push({ t: 'rời vòng ' + clock(a.leavesAt), k: 'warn' });
    if (a.disabled) chips.push({ t: 'đã tắt', k: '' });
  }
  let freshness: ChipView | null = null;
  let source: ChipView | null = null;
  if (!ctx.old) {
    const since = secondsSince(a.fetchedAt, ctx.now);
    if (a.reading === 'resetPending') freshness = { t: 'chờ số sau mốc đặt lại', k: 'info' };
    else if (a.reading === 'noData') freshness = { t: 'chưa có số', k: '' };
    else if (since != null) freshness = { t: age(since) + ' trước', k: a.reading === 'stale' ? 'stale' : '' };
    if (kind === 'c' && a.active && a.source === 'claudeSwap' && ctx.keychainRequired) source = { t: 'số từ claude-swap', k: 'info' };
  }
  // The popover shows the freshness chip only when the reading is not fresh.
  const late = freshness && a.reading !== 'fresh' ? [a.reading === 'stale' ? { ...freshness, t: 'cập nhật ' + freshness.t } : freshness] : [];
  const tags = source ? [...chips, source] : chips;
  const all = [...chips, ...late, ...(source ? [source] : [])];
  const lines = a.lines.map((l) => lineView(l, ctx.now));
  const tail = all.filter((c) => c.k !== 'on').map((c) => c.t);
  const where = a.number === 0 ? '' : kind === 'c' ? 'claude-swap · tài khoản ' + a.number : 'Cockpit · tài khoản ' + a.number;
  const off = offReason(a, ctx);
  const switchable = !!ctx.switching?.listed && a.number > 0 && !a.active && !off;
  return {
    key: kind + a.number,
    name: names.name,
    short: names.short,
    kind,
    active: a.active,
    dim: ctx.old || a.reading === 'stale',
    chips: all,
    tags,
    freshness,
    lines,
    label:
      names.name +
      (where ? ' (' + where.replace(' · ', ', ') + ')' : '') +
      (a.active ? ', đang dùng' : '') +
      '. ' +
      lines.map((l) => l.words).join('. ') +
      (tail.length ? '. ' + tail.join(', ') : '') +
      (switchable ? '. Bấm để chuyển' : off ? '. ' + off : ''),
    title: [where, off].filter(Boolean).join(' · '),
    slot: a.number,
    switchable,
    off,
  };
}

// ===== Pools =====

function poolParts(pool: Pool, names: (n: number) => string): string[] {
  return pool.kind === 'rotation' ? pool.rooms.map((r) => names(r.number) + ' ' + percent(r.percent)) : [];
}

function poolTitle(pool: Pool, names: (n: number) => string, unit: string): string {
  if (pool.kind !== 'rotation' || !pool.rooms.length || pool.room == null) return '';
  return (
    'Cộng phần còn lại trước ngưỡng tự chuyển: ' +
    poolParts(pool, names).join(' + ') +
    ' ≈ ' +
    room(pool.room) +
    ' ' +
    unit +
    '.'
  );
}

function claudePool(s: Section, old: string | null, names: (n: number) => string, now: Date): PoolView {
  const p = s.pool;
  const label = p.kind === 'rotation' ? 'Claude trong vòng' : 'Claude';
  if (p.kind === 'none' || (p.kind === 'rotation' ? p.room == null : p.percent == null)) {
    return { label, kind: 'c', value: '—', unit: null, percent: 0, sub: 'chưa có số', tone: 'muted', title: '', parts: [] };
  }
  const rotation = p.kind === 'rotation';
  const view: PoolView = {
    label,
    kind: 'c',
    value: rotation ? room(p.room!) : percent(p.percent!),
    unit: rotation ? 'tài khoản' : unitOf(p.line) || null,
    percent: Math.max(p.percent ?? 0, 2),
    sub: rotation ? 'tương đương, cộng phần còn lại' : 'chỉ tài khoản đang dùng',
    tone: rotation ? 'ok' : 'muted',
    title: poolTitle(p, names, 'tài khoản'),
    parts: poolParts(p, names),
  };
  if (p.exhausted) {
    view.tone = 'bad';
    const at = minutesUntil(p.nextRefillAt, now) != null ? clock(p.nextRefillAt) : '';
    view.sub = at ? 'cạn · có lại lúc ' + at : 'cạn';
  }
  if (old) {
    view.sub = 'số lúc ' + old;
    view.tone = 'muted';
  }
  return view;
}

function agyPool(s: Section, old: string | null): PoolView {
  const p = s.pool;
  const base = { label: 'Antigravity', kind: 'g' as const, title: '', parts: [] as string[] };
  if (s.status === 'notSetUp') {
    return { ...base, value: '—', unit: null, percent: 0, sub: 'không có trên máy này', tone: 'muted' };
  }
  if (p.kind === 'none' || (p.kind === 'rotation' ? p.room == null : p.percent == null)) {
    return { ...base, value: '—', unit: null, percent: 0, sub: 'chưa có số', tone: 'muted' };
  }
  const rotation = p.kind === 'rotation';
  const view: PoolView = {
    ...base,
    value: rotation ? room(p.room!) : percent(p.percent!),
    unit: rotation ? 'tài khoản' : unitOf(p.line) || null,
    percent: Math.max(p.percent ?? 0, 2),
    sub: rotation ? 'tương đương, theo ' + (p.scope === 'claude_gpt' ? 'Claude/GPT' : 'Gemini') : 'chỉ tài khoản đang dùng',
    tone: rotation ? 'ok' : 'muted',
    title: poolTitle(p, (n) => 'Ultra ' + n, 'tài khoản'),
    parts: poolParts(p, (n) => 'Ultra ' + n),
  };
  if (p.exhausted) view.tone = 'bad';
  if (old) {
    view.sub = 'số lúc ' + old;
    view.tone = 'muted';
  }
  return view;
}

// ===== Banner and summary =====

function errorReason(e: ErrorView): string {
  switch (e.kind) {
    case 'timeout':
      return 'quá thời gian chờ';
    case 'parse':
      return 'kết quả không đọc được';
    case 'spawn':
      return 'không khởi động được';
    default:
      return 'thoát với lỗi';
  }
}

function errorBanner(e: ErrorView, lastAt: string | null): BannerView {
  const last = lastAt ? ' Đang hiện số cuối lúc ' + lastAt + '.' : '';
  const missing = e.kind === 'notFound' || e.kind === 'notConfigured';
  return {
    tone: 'bad',
    title: 'Không chạy được quota-axi',
    body: (missing ? 'Không tìm thấy lệnh quota-axi trên máy này.' : 'quota-axi không chạy được: ' + errorReason(e) + '.') + last,
    actions: [
      { label: 'Chọn đường dẫn…', act: 'settings' },
      { label: 'Thử lại', act: 'refresh' },
    ],
  };
}

export function popoverView(model: Model, now: Date, options: ViewOptions = {}): PopoverView {
  const s = model.snapshot;
  const c = s.claude;
  const g = s.agy;
  const cErr = model.errors.claude;
  const gErr = model.errors.agy;
  const hasClaude = c.status !== 'unread';
  const lastClaude = cErr && hasClaude ? clock(c.generatedAt) || null : null;
  const lastAgy = gErr && g.status !== 'unread' ? clock(g.generatedAt) || null : null;

  const names = claudeNames(c.accounts);
  const nameOf = (n: number) => names.get(n)?.name ?? 'Tài khoản ' + n;
  const keychainRequired = s.keychain === 'required';
  const ctx = (old: boolean, section: Section): RowContext => ({
    now,
    old,
    keychainRequired,
    personalHours: section.personalHours,
    switching: null,
  });
  const claudeSwitching = { listed: c.status === 'ok', cswap: model.claudeSwitch.cswap, busy: model.claudeSwitch.busy };

  const claudeRows = c.accounts.map((a) =>
    rowView(a, 'c', names.get(a.number)!, { ...ctx(!!cErr, c), switching: claudeSwitching }),
  );
  const agyRows = g.status === 'notSetUp' ? [] : g.accounts.map((a) => rowView(a, 'g', agyName(a), ctx(!!gErr, g)));
  const active = c.accounts.find((a) => a.active);
  const activeName = active ? nameOf(active.number) : null;

  // Header freshness: the age of the newest reading, never the time of the app's call.
  let fresh: PopoverView['fresh'];
  const newest = c.newestReadingAt ?? g.newestReadingAt;
  const since = secondsSince(newest, now);
  if (cErr) fresh = { tone: 'bad', text: 'không đọc được số' };
  else if (since == null) fresh = { tone: 'muted', text: model.reading ? 'đang đọc số…' : 'chưa có số' };
  else fresh = { tone: since > STALE_SECONDS ? 'warn' : 'ok', text: 'cập nhật ' + age(since) + ' trước' };

  const claudeAge = secondsSince(c.newestReadingAt, now);
  const staleWords = claudeAge != null ? age(claudeAge) : '';

  // Summary (I11): only numbers already read, no forecast. A held account comes first.
  const summary: PopoverView['summary'] = [];
  const pool = c.pool;
  const held = pausedSummary(model, now);
  if (cErr) summary.push({ t: 'Không đọc được số mới' });
  else if (held) summary.push({ t: held });
  else if (c.stale && staleWords) summary.push({ t: 'Hạn mức Claude chưa cập nhật trong ' + staleWords });
  else if (pool.exhausted)
    summary.push({ t: pool.nextRefillAt ? 'Claude cạn đến ' + clock(pool.nextRefillAt) : 'Claude đã cạn', bad: true });
  else if (pool.kind === 'rotation' && pool.room != null) summary.push({ t: 'Claude còn tương đương ' + room(pool.room) + ' tài khoản' });
  else if (pool.kind === 'active' && pool.percent != null)
    summary.push({ t: ('Claude còn ' + percent(pool.percent) + ' ' + unitOf(pool.line)).trim() });
  else summary.push({ t: 'Chưa có số Claude' });
  if (!cErr && g.status !== 'notSetUp' && g.status !== 'unread') {
    const gActive = g.accounts.find((a) => a.active);
    const gemini = gActive?.lines.find((l) => l.kind === 'gemini');
    if (g.pool.kind === 'rotation' && g.pool.room != null) summary.push({ t: 'Antigravity còn tương đương ' + room(g.pool.room) + ' tài khoản' });
    else if (g.status === 'ok' && gemini?.percent != null) summary.push({ t: 'Antigravity còn ' + percent(gemini.percent) + ' Gemini' });
    else if (g.pool.kind === 'active' && g.pool.percent != null)
      summary.push({ t: ('Antigravity còn ' + percent(g.pool.percent) + ' ' + unitOf(g.pool.line)).trim() });
  }

  // One banner at a time, most urgent first.
  let banner: BannerView | null = null;
  const pendingAccount = c.accounts.find((a) => a.reading === 'resetPending');
  const switcher = switcherBanner(model, now);
  if (cErr) banner = errorBanner(cErr, lastClaude);
  else if (switcher && switcher.tone !== 'info') banner = switcher;
  else if (pool.exhausted) {
    const who = pool.refills.map(nameOf);
    const at = clock(pool.nextRefillAt);
    banner = {
      tone: 'bad',
      title: 'Hạn mức Claude trong vòng đã cạn',
      body: at ? 'Có lại lúc ' + at + (who.length ? ' khi ' + who.join(' và ') + ' được đặt lại' : '') + '.' : '',
      actions: [],
    };
  } else if (c.stale && staleWords) {
    banner = {
      tone: 'warn',
      title: 'Hạn mức Claude chưa cập nhật trong ' + staleWords,
      body: 'claude-swap chưa làm mới số từ ' + clock(c.newestReadingAt) + '. Switcher có thể đang không chạy.',
      actions: [{ label: 'Làm mới', act: 'refresh' }],
    };
  } else if (keychainRequired && !options.keychainLater) {
    banner = {
      tone: 'warn',
      title: 'Chưa cấp quyền Keychain cho quota-axi',
      body:
        'Dòng Claude' +
        (activeName ? ' của ' + activeName : '') +
        ' lấy tạm từ claude-swap (số chưa được cập nhật). Cấp quyền một lần để đọc trực tiếp.',
      actions: [
        { label: 'Cấp quyền một lần', act: 'keychain' },
        { label: 'Để sau', act: 'dismiss' },
      ],
    };
  } else if (pendingAccount) {
    banner = {
      tone: 'info',
      title: nameOf(pendingAccount.number) + ' vừa qua mốc đặt lại',
      body: 'Đang chờ claude-swap đọc số mới. App không hiện 100% khi chưa có số thật.',
      actions: [{ label: 'Làm mới', act: 'refresh' }],
    };
  } else if (switcher) banner = switcher;

  // Section notes.
  let claudeNote: NoteView | null = null;
  if (c.status === 'unavailable') claudeNote = { tone: 'warn', text: 'Không đọc được danh sách tài khoản của claude-swap (' + (c.reason ?? 'không rõ lý do') + ').' };
  else if (c.status === 'noSidecar')
    claudeNote = {
      tone: 'warn',
      text:
        model.quotaAxi.fork === false && model.quotaAxi.version
          ? 'Bản ' + model.quotaAxi.version + ' không có +cswap: thiếu số claude-swap và luật xoay vòng.'
          : 'quota-axi không báo danh sách tài khoản của claude-swap.',
    };
  else if (c.status === 'notSetUp') claudeNote = { tone: '', text: 'Máy này chưa có Claude Code' };
  else if (!model.claudeSwitch.cswap && c.accounts.some((a) => a.number > 0))
    claudeNote = { tone: 'warn', text: 'Không tìm thấy cswap trên máy này: chưa chuyển được tài khoản Claude.' };

  let agyNote: NoteView | null = null;
  if (g.status === 'noSidecar') agyNote = { tone: '', text: 'Máy này chưa có Cockpit: chỉ thấy tài khoản đang dùng, chưa chuyển được' };
  else if (g.status === 'unavailable') agyNote = { tone: '', text: 'Không đọc được Cockpit: chỉ thấy tài khoản đang dùng, chưa chuyển được' };
  else if (g.status === 'notSetUp') agyNote = { tone: '', text: 'Máy này chưa có Antigravity' };
  if (gErr && !cErr && g.status !== 'unread')
    agyNote = { tone: 'warn', text: 'Không đọc được số Antigravity mới' + (lastAgy ? '. Đang hiện số lúc ' + lastAgy + '.' : '.') };

  const gActive = g.accounts.find((a) => a.active);
  return {
    machine: model.machine,
    fresh,
    summary,
    banner,
    pools: { claude: claudePool(c, lastClaude, nameOf, now), agy: agyPool(g, lastAgy) },
    claude: {
      side: c.accounts.length ? c.accounts.length + ' tài khoản' + (activeName ? ' · đang dùng ' + activeName : '') : '',
      rows: claudeRows,
      note: claudeNote,
    },
    agy: {
      side: gActive && g.accounts.length > 1 ? 'đang dùng ' + agyName(gActive).name : '',
      rows: agyRows,
      note: agyNote,
    },
  };
}

// ===== Feedback after an action =====

export interface FeedbackView {
  tone: '' | 'info' | 'warn' | 'bad';
  title: string;
  body: string;
  /** Buttons under the line; a line with buttons stays until one is pressed. */
  actions?: { label: string; act: Action }[];
}

/**
 * The banner to show under a feedback line. A line that already says the
 * account is held (it offers to resume, or reports the pause) makes the
 * paused banner step aside rather than repeat it and its buttons; any other
 * line leaves it in place.
 */
export function bannerUnder(banner: BannerView | null, feedback: FeedbackView | null | undefined): BannerView | null {
  if (!banner || !feedback) return banner;
  const paused = banner.tone === 'info' && banner.actions.some((a) => a.act === 'resume');
  const repeats = feedback.tone === 'info' || !!feedback.actions?.some((a) => a.act === 'resume');
  return paused && repeats ? null : banner;
}

export function refreshedFeedback(now: Date): FeedbackView {
  return { tone: '', title: 'Đã làm mới', body: 'Đọc lại số lúc ' + clock(now.toISOString()) + '.' };
}

export function settingsFailedFeedback(): FeedbackView {
  return { tone: 'bad', title: 'Chưa lưu được cài đặt', body: 'Không ghi được file cài đặt của app trên máy này. Thử lại.' };
}

export function keychainFeedback(model: Model | null): FeedbackView {
  return model?.snapshot.keychain === 'granted'
    ? { tone: '', title: 'Đã cấp quyền Keychain', body: 'Dòng Claude giờ đọc trực tiếp từ quota-axi.' }
    : {
        tone: 'bad',
        title: 'Chưa cấp được quyền Keychain',
        body: 'quota-axi vẫn chưa đọc được Keychain. Thử lại, rồi bấm "Luôn cho phép" khi macOS hỏi.',
      };
}
