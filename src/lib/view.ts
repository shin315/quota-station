// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Turns the model into what the popover shows; the main window builds on
// the same view (`window.ts`). Pure: the same model and
// clock give the same view, so Vitest covers every state without a window.
// Copy follows the approved prototype (Hướng 1 · Kính macOS, bước 0.7).

import { agyCells, agyGroups, agyNames, cellOf, claudeCells, claudeNames, tightest, WARN_USED, type AccountName, type CellView, type UsedTone } from './accounts';
import { age, clock, secondsSince } from './format';
import type { Account, AgyLogin, ErrorView, LineKind, MenubarWindow, Model, Section } from './model';
import { codexBanner, currentId, shortName } from './codex';
import { switcherBanner } from './switching';

export type Tone = 'ok' | 'warn' | 'bad' | 'muted' | 'info';
export type Action =
  | 'refresh'
  | 'settings'
  | 'window'
  /** The popover's theme button: light ↔ dark. */
  | 'theme'
  | 'keychain'
  | 'dismiss'
  /** Removes the pause flag. */
  | 'resume'
  /** One more hour on a timed pause. */
  | 'addHour'
  | 'closeFeedback'
  | 'guide:auto'
  | 'guide:switcher'
  /** The Claude accounts' actions on banners (`ClaudeAccounts`). */
  | 'claude:rotation:on'
  | 'claude:import'
  /** Opens the switch sheet for a Claude slot. */
  | `switch:${number}`
  /** Opens the switch sheet for an Antigravity account (M4c). */
  | `agy:switch:${number}`
  /** "Thôi" on the overwrite banner: stops watching the last Antigravity switch. */
  | 'agy:keep'
  /** System Settings › Notifications (M6). */
  | 'notify:settings'
  /** Codex (M8): the row, a switch sheet, the restart sheet, the change made outside, guides and the tab. */
  | CodexAction
  /** The rotation editor's banners and feedback (M5). */
  | RulesAction
  /** The MCP tab (M9): open it, undo the last write, its backups, read again. */
  | McpAction
  /** The Sao lưu tab (M10): read again, its folder in Finder, undo the last restore, restore. */
  | BackupAction
  /** The Skills, Plugin and File hướng dẫn tabs (M11). */
  | SkillAction
  /** The variables that override a login (M14): Cài đặt's card and the Claude tab's banner. */
  | EnvAction
  /** The configuration sets (M15): a new one, undo the last apply or delete, apply one. */
  | SetsAction;

/** "Kiểm lại", "Xem danh sách", the card in Cài đặt, and "Mở file", "Bỏ qua", its "Hoàn tác" on one place's id. */
/** "Tạo bộ", "Hoàn tác bộ", "Hoàn tác" of a delete, "Áp…" on one set (M15). */
export type SetsAction = 'sets:new' | 'sets:undo' | 'sets:undelete' | `sets:apply:${string}`;

export type EnvAction = 'env:check' | 'env:list' | 'env:settings' | `env:open:${string}` | `env:ignore:${string}` | `env:back:${string}`;

export type SkillAction =
  | 'skills:undo'
  | 'skills:reread'
  | 'skills:check'
  | 'skills:npx'
  | `skills:update:${string}`
  | 'plugins:reread'
  | 'instr:reread'
  | `instr:key:${string}`;

export type BackupAction = 'backup:reread' | 'backup:open' | 'backup:undo' | 'backup:restore' | 'copy:compare';

export type McpAction = 'mcp:tab' | 'mcp:undo' | 'mcp:backups' | 'mcp:reread' | `mcp:key:${string}` | `mcp:howcmd:${string}` | `mcp:later:${string}`;

export type CodexAction =
  | 'codex:open'
  | 'codex:switch:chatGpt'
  | 'codex:switch:custom'
  | 'codex:daemon'
  | 'codex:external'
  | 'codex:keep'
  | 'codex:check'
  | 'codex:tab'
  | 'codex:guide:login'
  | 'codex:key'
  | 'codex:undoKey'
  | 'codex:plugins:apply'
  | 'codex:backup';

export type RulesAction =
  | 'rules:create'
  | 'rules:retry'
  | 'rules:reload'
  | 'rules:overwrite'
  | 'rules:dismiss'
  | `rules:adopt:${string}`
  | `rules:compare:${string}`;

export interface ChipView {
  t: string;
  k: '' | 'on' | 'ok' | 'warn' | 'bad' | 'stale' | 'info';
  /** Hover: the whole story behind a short chip. */
  title?: string;
}

export interface CardView {
  key: string;
  kind: 'c' | 'g';
  /** claude-swap slot or Antigravity account number; 0 for the one account a row reads alone. */
  slot: number;
  name: string;
  /** Organization, `Cá nhân · 08–23`, ''. */
  sub: string;
  /** The whole email, for hover; '' while names are hidden. */
  full: string;
  active: boolean;
  /** Old numbers: drawn muted. */
  dim: boolean;
  /** Rotation and freshness chips, never `đang dùng` (the card and the table draw that themselves). */
  chips: ChipView[];
  /** 5 giờ · Tuần · Fable, or Antigravity's four windows. */
  cells: CellView[];
  /**
   * The popover's shorter card: no row for a window the account does not
   * have (a Team seat has no week), Antigravity's Gemini and Claude/GPT as
   * one row each (the tighter window), and no `chưa dùng`.
   */
  rows: CellView[];
  /** The window used the most: the number the menu bar and Đang dùng show. */
  tight: CellView | null;
  /** For screen readers. */
  label: string;
  /** Hover: the email and where the numbers come from. */
  title: string;
  /** An account the owner can switch to now. */
  switchable: boolean;
  /** Why this one cannot be switched to; '' when it can, or when it is the active one. */
  off: string;
}

/** One service in Đang dùng: the account in use, its tightest window and when that resets. */
export interface NowRowView {
  id: 'c' | 'g' | 'x';
  svc: 'Claude' | 'Gemini' | 'Codex';
  /** Whole, the layout cuts it: `lan.acme1`, `Team A`, `ChatGPT`, a provider's name. */
  name: string;
  /** Under the name on Tổng quan: the organization, `Antigravity`, `Codex đi qua relay`. */
  sub: string;
  used: number | null;
  /** `38%`, or `—` without a number. */
  text: string;
  tone: UsedTone;
  /** The window the number is: `5 giờ`, `tuần`, `Fable`. */
  unit: string;
  /** `↻ 0:09`; '' when the window has not been used. */
  reset: string;
  /** Without a number, why: `không có số`, `ChatGPT đã dùng 41% tuần`. */
  note: string;
  /** Antigravity's Claude/GPT models from 70 % used (`86%`), so folding the group never hides it. */
  extra: { text: string; tone: UsedTone } | null;
  /** Hover: the window named. */
  title: string;
  /** For screen readers: every window and when it resets. */
  label: string;
}

/** Đang dùng (replaces the rotation's room): the account in use on each service. */
export interface NowView {
  rows: NowRowView[];
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

export interface GroupView {
  id: 'c' | 'g';
  title: string;
  /** Active account first. */
  cards: CardView[];
  note: NoteView | null;
}

export interface PopoverView {
  machine: string;
  /** The popover writes it only when the tone is not `ok`: old numbers, none, an error. */
  fresh: { tone: Tone; text: string };
  banner: BannerView | null;
  now: NowView;
  claude: GroupView;
  agy: GroupView;
}

export interface ViewOptions {
  /** "Để sau" was pressed on the Keychain banner. */
  keychainLater?: boolean;
}

const STALE_SECONDS = 900;

export { claudeNames, agyNames, type AccountName, type CellView, type UsedTone } from './accounts';

/** Positional name of an Antigravity account, with its email when one is given. */
export function agyName(a: Pick<Account, 'number' | 'email'>): AccountName {
  return agyNames([a]).get(a.number)!;
}

// ===== Cards =====

interface CardContext {
  now: Date;
  /** Every number is the last one before quota-axi stopped answering. */
  old: boolean;
  keychainRequired: boolean;
  personalHours: string | null;
  /** Claude cards: what switching needs. */
  switching: { listed: boolean; ready: boolean; busy: number | null } | null;
  /** Antigravity cards: what switching needs (M4c). */
  agy: AgyRowSwitch | null;
}

interface AgyRowSwitch {
  /** quota-axi numbers the app's own list. */
  listed: boolean;
  busy: number | null;
  /** The accounts logged in here, by number. */
  logins: Map<number, AgyLogin>;
}

function agyOffReason(a: Account, g: AgyRowSwitch): string {
  if (a.active || a.number === 0) return '';
  if (!g.listed) return 'Chưa chuyển được: quota-axi chưa liệt kê tài khoản của Quota Station';
  const login = g.logins.get(a.number);
  if (!login) return 'Chưa đăng nhập tài khoản này trong Quota Station';
  if (g.busy != null) return 'Đang chuyển tài khoản';
  if (login.status === 'needsLogin') return 'Cần đăng nhập lại trong tab Antigravity';
  return '';
}

function offReason(a: Account, ctx: CardContext): string {
  if (ctx.agy) return agyOffReason(a, ctx.agy);
  const sw = ctx.switching;
  if (a.active) return '';
  if (!sw || !sw.listed || a.number === 0) return '';
  if (sw.busy != null) return 'Đang chuyển tài khoản';
  if (!sw.ready) return 'Chưa chuyển được: Quota Station chưa giữ tài khoản Claude';
  return '';
}

/** Who keeps the Claude accounts' list: Quota Station once it does, claude-swap before. */
function listName(ctx: CardContext): string {
  return ctx.switching?.ready === false ? 'claude-swap' : 'Quota Station';
}

function cardView(a: Account, kind: 'c' | 'g', names: AccountName, ctx: CardContext): CardView {
  const chips: ChipView[] = [];
  if (kind === 'c') {
    if (a.membership === 'out') chips.push({ t: 'ngoài vòng', k: '' });
    if (a.membership === 'ended') chips.push({ t: 'đã rời vòng', k: '' });
    if (a.leavesAt) chips.push({ t: 'rời vòng ' + clock(a.leavesAt), k: 'warn' });
    if (a.disabled) chips.push({ t: 'đã tắt', k: '' });
  }
  // Freshness shows only when the reading is not fresh: its own age, never the app's last call.
  if (!ctx.old) {
    const since = secondsSince(a.fetchedAt, ctx.now);
    if (a.reading === 'resetPending') chips.push({ t: 'chờ số sau mốc đặt lại', k: 'info' });
    else if (a.reading === 'noData') chips.push({ t: 'chưa có số', k: '' });
    else if (a.reading === 'stale' && since != null) chips.push({ t: 'số ' + age(since) + ' trước', k: 'stale' });
    if (kind === 'c' && a.active && a.source === 'claudeSwap' && ctx.keychainRequired) chips.push({ t: 'số từ ' + listName(ctx), k: 'info' });
  }
  const cells = kind === 'c' ? claudeCells(a, ctx.now) : agyCells(a, ctx.now);
  const where =
    a.number === 0
      ? ''
      : kind === 'c'
        ? listName(ctx) + ' · tài khoản ' + a.number
        : (ctx.agy?.listed ? 'Quota Station' : 'Cockpit Tools') + ' · tài khoản ' + a.number;
  const off = offReason(a, ctx);
  const listed = kind === 'c' ? !!ctx.switching?.listed : !!ctx.agy?.listed;
  const switchable = listed && a.number > 0 && !a.active && !off;
  return {
    key: kind + a.number,
    kind,
    slot: a.number,
    name: names.name,
    sub: names.sub,
    full: names.full,
    active: a.active,
    dim: ctx.old || a.reading === 'stale',
    chips,
    cells,
    rows: shortRows(kind, cells),
    tight: tightest(cells),
    label:
      names.name +
      (names.sub ? ', ' + names.sub : '') +
      (a.active ? ', đang dùng' : '') +
      '. ' +
      cells.map((c) => c.words).join('. ') +
      (chips.length ? '. ' + chips.map((c) => c.t).join(', ') : '') +
      (switchable ? '. Bấm để chuyển' : off ? '. ' + off : ''),
    title: [names.full, where, off].filter(Boolean).join(' · '),
    switchable,
    off,
  };
}

/**
 * In slot order, the order the main window's tables list them: the account
 * in use keeps its place and only its frame marks it.
 */
function bySlot(cards: CardView[]): CardView[] {
  return [...cards].sort((x, y) => x.slot - y.slot);
}

// ===== The popover's shorter cards =====

function quiet(c: CellView): CellView {
  return c.reset === 'chưa dùng' ? { ...c, reset: '' } : c;
}

function shortRows(kind: 'c' | 'g', cells: CellView[]): CellView[] {
  if (kind === 'c') return cells.filter((c) => !c.none).map(quiet);
  if (!cells.some((c) => c.label.startsWith('Gemini '))) return cells.map(quiet);
  return agyGroups(cells).map((g) => {
    const cell = g.cell ?? cellOf(undefined, g.label === 'Gemini' ? 'gemini' : 'claudeGpt', new Date(0), '');
    return quiet({ ...cell, label: g.label, none: false });
  });
}

// ===== Đang dùng =====

const WINDOW_KINDS: Record<Exclude<MenubarWindow, 'tightest'>, LineKind[]> = {
  fiveHours: ['session', 'geminiSession', 'claudeGptSession'],
  week: ['week', 'geminiWeek', 'claudeGptWeek'],
  fableWeek: ['fableWeek'],
};

/**
 * The window picked for the menu bar, the menu bar's rule (`icon::chosen`):
 * five hours when the account has no window of that kind, the tightest
 * without either. None while a window it rests on waits for a number after
 * its reset, never a number that may be the wrong one.
 */
function nowNumber(cells: CellView[], window: MenubarWindow): CellView | null {
  const real = cells.filter((c) => !c.none);
  let from = real;
  for (const w of [window, 'fiveHours'] as const) {
    if (w === 'tightest') break;
    const found = real.filter((c) => WINDOW_KINDS[w].includes(c.kind));
    if (found.length) {
      from = found;
      break;
    }
  }
  if (from.some((c) => c.used == null || c.reset === 'chờ số')) return null;
  return tightest(from);
}

function nowRow(
  base: Pick<NowRowView, 'id' | 'svc' | 'name' | 'sub'>,
  cells: CellView[],
  window: MenubarWindow,
  why: string | null,
  extra: NowRowView['extra'] = null,
): NowRowView {
  const t = why == null ? nowNumber(cells, window) : null;
  const note = why ?? (t ? '' : 'không có số');
  const words = cells.filter((c) => !c.none).map((c) => c.words).join('. ');
  return {
    ...base,
    used: t?.used ?? null,
    text: t ? t.text : '—',
    tone: t?.tone ?? 'muted',
    unit: t?.unit ?? '',
    reset: t && t.reset !== 'chưa dùng' ? t.reset : '',
    note,
    extra,
    title: base.svc + ': ' + (t ? base.name + ' đã dùng ' + t.text + ' ' + t.unit : base.name + ', ' + note) + (extra ? ' · Claude/GPT ' + extra.text : ''),
    label: base.svc + ', ' + base.name + '. ' + (why ?? (words || note)),
  };
}

function codexNow(model: Model, now: Date): NowRowView | null {
  const v = model.codex;
  if (!v.present) return null;
  const s = model.snapshot.codex;
  const a = s.accounts.find((x) => x.active);
  const cells = a ? (['session', 'week'] as const).map((k) => cellOf(a.windows.find((l) => l.kind === k), k, now, '')) : [];
  const err = !!model.errors.codex;
  if (v.current !== 'chatgpt') {
    const name = shortName(v, currentId(v));
    const t = !err ? nowNumber(cells, model.menubarWindow) : null;
    return nowRow({ id: 'x', svc: 'Codex', name, sub: 'Codex đi qua ' + name }, [], model.menubarWindow, t ? 'ChatGPT đã dùng ' + t.text + ' ' + t.unit : name + ' không báo hạn mức');
  }
  if (s.status === 'notSetUp') return null;
  const why = err ? 'không đọc được số' : s.status === 'unread' ? 'chưa có số' : s.status === 'unavailable' ? 'không đọc được số' : null;
  return nowRow({ id: 'x', svc: 'Codex', name: 'ChatGPT', sub: '' }, cells, model.menubarWindow, why);
}

function nowView(model: Model, claudeCards: CardView[], agyCards: CardView[], now: Date): NowView {
  const s = model.snapshot;
  const rows: NowRowView[] = [];
  const failing = (e: ErrorView | null, section: Section) => (e ? 'không đọc được số' : section.status === 'unread' ? 'chưa có số' : null);
  if (s.claude.status !== 'notSetUp') {
    const a = claudeCards.find((c) => c.active);
    const acc = s.claude.accounts.find((x) => x.active);
    const why = failing(model.errors.claude, s.claude) ?? (acc && (acc.reading === 'resetPending' || acc.reading === 'noData') ? 'chờ số mới' : null);
    rows.push(nowRow({ id: 'c', svc: 'Claude', name: a?.name ?? '—', sub: a?.sub ?? '' }, a?.cells ?? [], model.menubarWindow, a ? why : (why ?? 'chưa biết tài khoản nào')));
  }
  if (s.agy.status !== 'notSetUp') {
    const a = agyCards.find((c) => c.active);
    const acc = s.agy.accounts.find((x) => x.active);
    const why = failing(model.errors.agy, s.agy) ?? (acc && (acc.reading === 'resetPending' || acc.reading === 'noData') ? 'chờ số mới' : null);
    const gemini = a ? a.cells.filter((c) => c.label.startsWith('Gemini')) : [];
    const cg = a && why == null ? agyGroups(a.cells)[1].cell : null;
    const extra = cg && cg.used != null && cg.used >= WARN_USED ? { text: cg.text, tone: cg.tone } : null;
    rows.push(nowRow({ id: 'g', svc: 'Gemini', name: a?.name ?? '—', sub: 'Antigravity' }, gemini, model.menubarWindow, a ? why : (why ?? 'chưa biết tài khoản nào'), extra));
  }
  const codex = codexNow(model, now);
  if (codex) rows.push(codex);
  return { rows };
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

  const names = claudeNames(c.accounts, c.personalHours, model.advanced.on);
  const nameOf = (n: number) => names.get(n)?.name ?? 'Tài khoản ' + n;
  const keychainRequired = s.keychain === 'required';
  const ctx = (old: boolean, section: Section): CardContext => ({
    now,
    old,
    keychainRequired,
    personalHours: section.personalHours,
    switching: null,
    agy: null,
  });
  const claudeSwitching = { listed: c.status === 'ok', ready: model.claudeSwitch.ready, busy: model.claudeSwitch.busy };
  const claudeCards = c.accounts.map((a) => cardView(a, 'c', names.get(a.number)!, { ...ctx(!!cErr, c), switching: claudeSwitching }));
  const agySwitching: AgyRowSwitch = {
    listed: g.status === 'ok' && !!model.agySwitch?.listed,
    busy: model.agySwitch?.busy ?? null,
    logins: new Map((model.antigravity?.accounts ?? []).map((a) => [a.number, a])),
  };
  const gNames = agyNames(g.accounts, model.advanced.on);
  const agyCards = g.status === 'notSetUp' ? [] : g.accounts.map((a) => cardView(a, 'g', gNames.get(a.number)!, { ...ctx(!!gErr, g), agy: agySwitching }));
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
  const pool = c.pool;

  // One banner at a time, most urgent first.
  let banner: BannerView | null = null;
  const pendingAccount = c.accounts.find((a) => a.reading === 'resetPending');
  const switcher = switcherBanner(model, now);
  const codex = codexBanner(model);
  // An Antigravity switch written over comes as a macOS card with buttons (M6),
  // and stays on the main window's Antigravity tab; the popover has no banner for it.
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
      body: swapName(model) + ' chưa làm mới số từ ' + clock(c.newestReadingAt) + '. Switcher có thể đang không chạy.',
      actions: [{ label: 'Làm mới', act: 'refresh' }],
    };
  } else if (keychainRequired && !options.keychainLater) {
    banner = {
      tone: 'warn',
      title: 'Chưa cấp quyền Keychain cho quota-axi',
      body:
        'Dòng Claude' +
        (activeName ? ' của ' + activeName : '') +
        ' lấy tạm từ ' + swapName(model) + ' (số chưa được cập nhật). Cấp quyền một lần để đọc trực tiếp.',
      actions: [
        { label: 'Cấp quyền một lần', act: 'keychain' },
        { label: 'Để sau', act: 'dismiss' },
      ],
    };
  } else if (codex && codex.tone !== 'info') banner = codex;
  else if (pendingAccount) {
    banner = {
      tone: 'info',
      title: nameOf(pendingAccount.number) + ' vừa qua mốc đặt lại',
      body: 'Đang chờ ' + swapName(model) + ' đọc số mới. App không hiện số khi chưa có số thật.',
      actions: [{ label: 'Làm mới', act: 'refresh' }],
    };
  } else if (codex) banner = codex;
  else if (switcher) banner = switcher;

  // Group notes.
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
  else if (!model.claudeSwitch.ready && c.accounts.some((a) => a.number > 0))
    claudeNote = {
      tone: 'warn',
      text: 'Quota Station chưa giữ tài khoản Claude nào trên máy này: nhập từ claude-swap ở tab Claude để chuyển được tài khoản.',
    };

  let agyNote: NoteView | null = null;
  if (g.status === 'noSidecar') agyNote = { tone: '', text: 'Chỉ thấy tài khoản đang dùng: chưa có danh sách tài khoản' };
  else if (g.status === 'unavailable') agyNote = { tone: '', text: 'Không đọc được danh sách tài khoản: chỉ thấy tài khoản đang dùng' };
  else if (g.status === 'notSetUp') agyNote = { tone: '', text: 'Máy này chưa có Antigravity' };
  if (gErr && !cErr && g.status !== 'unread')
    agyNote = { tone: 'warn', text: 'Không đọc được số Antigravity mới' + (lastAgy ? '. Đang hiện số lúc ' + lastAgy + '.' : '.') };

  return {
    machine: model.machine,
    fresh,
    banner,
    now: nowView(model, claudeCards, agyCards, now),
    claude: {
      id: 'c',
      title: 'CLAUDE',
      cards: bySlot(claudeCards),
      note: claudeNote,
    },
    agy: {
      id: 'g',
      title: 'ANTIGRAVITY',
      cards: bySlot(agyCards),
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
/**
 * A line that only says something went well: a short toast that goes by
 * itself, with nothing to press. Any other line stays where it was asked
 * for until it is answered or times out.
 */
export function isQuiet(fb: FeedbackView): boolean {
  return fb.tone === '' && (fb.actions ?? []).every((a) => a.act === 'closeFeedback');
}

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

/** Who reads the Claude accounts' numbers: Quota Station once it keeps them, claude-swap before. */
function swapName(model: Model): string {
  return model.claudeSwitch.ready ? 'Quota Station' : 'claude-swap';
}
