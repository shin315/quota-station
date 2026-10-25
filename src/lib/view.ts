// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Turns the model into what the popover shows; the main window builds on
// the same view (`window.ts`). Pure: the same model and
// clock give the same view, so Vitest covers every state without a window.
// Copy follows the approved prototype (Hướng 1 · Kính macOS, bước 0.7).

import { agyCells, agyGroups, agyNames, cellOf, claudeAliases, claudeCells, claudeNames, isGemini, tightest, WARN_USED, type AccountName, type CellView, type UsedTone } from './accounts';
import { age, clock, secondsSince } from './format';
import { t } from './i18n';
import type { Account, AgyLogin, LineKind, MenubarWindow, Model, Section } from './model';
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
  /** Signs a Claude account in again (its slot): the Claude tab's dialog. */
  | `claude:relogin:${number}`
  | `claude:unrest:${number}`
  /** Adds a Claude account: the Claude tab's dialog. */
  | 'claude:add'
  /** A newer release: its dialog, putting it off, checking now, undoing a skip. */
  | 'release:open'
  | 'release:later'
  | 'release:check'
  | 'release:unskip'
  /** The GitHub bug form, in the browser. */
  | 'release:issue'
  /** The diagnostic info's dialog. */
  | 'diag:open'
  /** Opens the switch sheet for a Claude slot. */
  | `switch:${number}`
  /** Opens the switch sheet for an Antigravity account (M4c). */
  | `agy:switch:${number}`
  /** "Thôi" on the overwrite banner: stops watching the last Antigravity switch. */
  | 'agy:keep'
  /** System Settings › Notifications (M6). */
  | 'notify:settings'
  /** The popover's bell: the main window with the bell's panel open. */
  | 'notifications'
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
  k: '' | 'on' | 'ok' | 'warn' | 'bad' | 'stale' | 'info' | 'tag';
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
  /** Its login in Quota Station expired: pressing it signs in again instead of switching. */
  relogin: boolean;
  /** Held out of the rotation or turned off: drawn with a dashed edge, its text unchanged. */
  resting: boolean;
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
  /** `3h 9m`; empty when the window has not been used. */
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
  /** What a note that is not an error is about, for its icon: old numbers, a permission, anything else. */
  icon?: 'stale' | 'perm' | 'note';
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
  /** Only the account signed in to the service, with no list to switch in (a read-only tag). */
  readOnly: boolean;
  /** The Mac has the service: the popover shows only the services the Mac has. */
  shown: boolean;
  /** Claude Code is here and the app keeps no account yet. */
  empty: boolean;
  /** Active account first. */
  cards: CardView[];
  note: NoteView | null;
}

export interface PopoverView {
  machine: string;
  /** The first read is running: names may show, numbers are grey bars. */
  loading: boolean;
  /** None of Claude Code, Codex and Antigravity is on this Mac. */
  none: boolean;
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
  personalHours: string | null;
  /** Claude cards: what switching needs. */
  switching: { listed: boolean; ready: boolean; busy: number | null } | null;
  /** Claude accounts whose login in Quota Station expired. */
  relogin?: Set<number>;
  /** Antigravity cards: what switching needs (M4c). */
  agy: AgyRowSwitch | null;
}

interface AgyRowSwitch {
  /** The row lists the app's own accounts. */
  listed: boolean;
  busy: number | null;
  /** The accounts logged in here, by number. */
  logins: Map<number, AgyLogin>;
}

function agyOffReason(a: Account, g: AgyRowSwitch): string {
  if (a.active || a.number === 0) return '';
  if (!g.listed) return t('Chưa chuyển được: chưa đọc được danh sách tài khoản của Quota Station', 'Cannot switch yet: Quota Station’s account list is not read yet');
  const login = g.logins.get(a.number);
  if (!login) return t('Chưa đăng nhập tài khoản này trong Quota Station', 'This account is not signed in to Quota Station');
  if (g.busy != null) return t('Đang chuyển tài khoản', 'Switching accounts');
  if (login.status === 'needsLogin') return t('Cần đăng nhập lại trong tab Antigravity', 'Sign in again in the Antigravity tab');
  return '';
}

function offReason(a: Account, ctx: CardContext): string {
  if (ctx.agy) return agyOffReason(a, ctx.agy);
  const sw = ctx.switching;
  if (a.active) return '';
  if (!sw || !sw.listed || a.number === 0) return '';
  if (sw.busy != null) return t('Đang chuyển tài khoản', 'Switching accounts');
  if (!sw.ready) return t('Chưa chuyển được: Quota Station chưa giữ tài khoản Claude', 'Cannot switch yet: Quota Station keeps no Claude account');
  return '';
}

/** Who keeps the Claude accounts' list: Quota Station once it does, claude-swap before. */
function listName(ctx: CardContext): string {
  return ctx.switching?.ready === false ? 'claude-swap' : 'Quota Station';
}

function cardView(a: Account, kind: 'c' | 'g', names: AccountName, ctx: CardContext): CardView {
  const chips: ChipView[] = [];
  if (kind === 'c') {
    if (a.membership === 'out') chips.push({ t: t('ngoài vòng', 'out of rotation'), k: '' });
    if (a.membership === 'ended') chips.push({ t: t('đã rời vòng', 'left rotation'), k: '' });
    if (a.leavesAt) chips.push({ t: t('rời vòng ', 'leaves rotation ') + clock(a.leavesAt), k: 'warn' });
    if (a.disabled) chips.push({ t: t('nghỉ tự chuyển', 'out of auto-switch'), k: '' });
  }
  // Freshness shows only when the reading is not fresh: its own age, never the app's last call.
  const since = secondsSince(a.fetchedAt, ctx.now);
  if (a.reading === 'resetPending') chips.push({ t: t('chờ số sau mốc đặt lại', 'waiting for the number after its reset'), k: 'info' });
  else if (a.reading === 'noData') chips.push({ t: t('chưa có số', 'no number yet'), k: '' });
  else if (a.reading === 'stale' && since != null) chips.push({ t: t('số ' + age(since) + ' trước', 'numbers ' + age(since) + ' old'), k: 'stale' });
  const cells = kind === 'c' ? claudeCells(a, ctx.now) : agyCells(a, ctx.now);
  const where =
    a.number === 0
      ? ''
      : kind === 'c'
        ? listName(ctx) + t(' · tài khoản ', ' · account ') + a.number
        : t('Quota Station · tài khoản ', 'Quota Station · account ') + a.number;
  const off = offReason(a, ctx);
  const listed = kind === 'c' ? !!ctx.switching?.listed : !!ctx.agy?.listed;
  const relogin = kind === 'c' && !!ctx.relogin?.has(a.number);
  if (relogin) chips.push({ t: t('cần đăng nhập lại', 'sign in again'), k: '' });
  const switchable = listed && a.number > 0 && !a.active && !off;
  return {
    key: kind + a.number,
    kind,
    slot: a.number,
    name: names.name,
    sub: names.sub,
    full: names.full,
    active: a.active,
    dim: a.reading === 'stale',
    chips,
    cells,
    rows: shortRows(kind, cells),
    tight: tightest(cells),
    label:
      names.name +
      (names.sub ? ', ' + names.sub : '') +
      (a.active ? t(', đang dùng', ', in use') : '') +
      '. ' +
      cells.map((c) => c.words).join('. ') +
      (chips.length ? '. ' + chips.map((c) => c.t).join(', ') : '') +
      (switchable ? t('. Bấm để chuyển', '. Press to switch') : off ? '. ' + off : ''),
    title: [names.full, where, off].filter(Boolean).join(' · '),
    switchable,
    relogin,
    resting: a.disabled || a.membership === 'out' || a.membership === 'ended',
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

function shortRows(kind: 'c' | 'g', cells: CellView[]): CellView[] {
  if (kind === 'c') return cells.filter((c) => !c.none);
  if (!cells.some((c) => c.kind === 'geminiSession' || c.kind === 'geminiWeek')) return cells;
  return agyGroups(cells).map((g, i) => {
    const cell = g.cell ?? cellOf(undefined, i === 0 ? 'gemini' : 'claudeGpt', new Date(0), '');
    return { ...cell, label: g.label, none: false };
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
  if (from.some((c) => c.used == null || c.waiting)) return null;
  return tightest(from);
}

function nowRow(
  base: Pick<NowRowView, 'id' | 'svc' | 'name' | 'sub'>,
  cells: CellView[],
  window: MenubarWindow,
  why: string | null,
  extra: NowRowView['extra'] = null,
): NowRowView {
  const num = why == null ? nowNumber(cells, window) : null;
  const note = why ?? (num ? '' : t('không có số', 'no number'));
  const words = cells.filter((c) => !c.none).map((c) => c.words).join('. ');
  return {
    ...base,
    used: num?.used ?? null,
    text: num ? num.text : '—',
    tone: num?.tone ?? 'muted',
    unit: num?.unit ?? '',
    reset: num && !num.unused ? num.reset : '',
    note,
    extra,
    title:
      base.svc +
      ': ' +
      (num ? base.name + t(' đã dùng ', ' used ') + num.text + ' ' + num.unit : base.name + ', ' + note) +
      (extra ? ' · Claude/GPT ' + extra.text : ''),
    label: base.svc + ', ' + base.name + '. ' + (why ?? (words || note)),
  };
}

function codexNow(model: Model, now: Date): NowRowView | null {
  const v = model.codex;
  if (!v.present) return null;
  const s = model.snapshot.codex;
  const a = s.accounts.find((x) => x.active);
  const cells = a ? (['session', 'week'] as const).map((k) => cellOf(a.windows.find((l) => l.kind === k), k, now, '')) : [];
  if (v.current !== 'chatgpt') {
    const name = shortName(v, currentId(v));
    const num = nowNumber(cells, model.menubarWindow);
    return nowRow(
      { id: 'x', svc: 'Codex', name, sub: t('Codex đi qua ', 'Codex goes through ') + name },
      [],
      model.menubarWindow,
      num ? t('ChatGPT đã dùng ', 'ChatGPT used ') + num.text + ' ' + num.unit : t(name + ' không báo hạn mức', name + ' reports no quota'),
    );
  }
  if (s.status === 'notSetUp') return null;
  const why = s.status === 'unread' ? t('chưa có số', 'no number yet') : s.status === 'unavailable' ? t('không đọc được số', 'cannot read numbers') : null;
  return nowRow({ id: 'x', svc: 'Codex', name: 'ChatGPT', sub: '' }, cells, model.menubarWindow, why);
}

function nowView(model: Model, claudeCards: CardView[], agyCards: CardView[], now: Date): NowView {
  const s = model.snapshot;
  const rows: NowRowView[] = [];
  const failing = (section: Section) =>
    section.status === 'unread'
      ? t('chưa có số', 'no number yet')
      : section.status === 'single' && !section.accounts.length && section.reason
        ? t('không đọc được số', 'cannot read numbers')
        : null;
  if (s.claude.status !== 'notSetUp') {
    const a = claudeCards.find((c) => c.active);
    const acc = s.claude.accounts.find((x) => x.active);
    const why = failing(s.claude) ?? (acc && (acc.reading === 'resetPending' || acc.reading === 'noData') ? t('chờ số mới', 'waiting for new numbers') : null);
    rows.push(
      nowRow({ id: 'c', svc: 'Claude', name: a?.name ?? '—', sub: a?.sub ?? '' }, a?.cells ?? [], model.menubarWindow, a ? why : (why ?? t('chưa biết tài khoản nào', 'account not known yet'))),
    );
  }
  if (s.agy.status !== 'notSetUp') {
    const a = agyCards.find((c) => c.active);
    const acc = s.agy.accounts.find((x) => x.active);
    const why = failing(s.agy) ?? (acc && (acc.reading === 'resetPending' || acc.reading === 'noData') ? t('chờ số mới', 'waiting for new numbers') : null);
    const gemini = a ? a.cells.filter(isGemini) : [];
    const cg = a && why == null ? agyGroups(a.cells)[1].cell : null;
    const extra = cg && cg.used != null && cg.used >= WARN_USED ? { text: cg.text, tone: cg.tone } : null;
    rows.push(
      nowRow({ id: 'g', svc: 'Gemini', name: a?.name ?? '—', sub: 'Antigravity' }, gemini, model.menubarWindow, a ? why : (why ?? t('chưa biết tài khoản nào', 'account not known yet')), extra),
    );
  }
  const codex = codexNow(model, now);
  if (codex) rows.push(codex);
  return { rows };
}

// ===== Banner and summary =====

export function popoverView(model: Model, now: Date, options: ViewOptions = {}): PopoverView {
  const s = model.snapshot;
  const c = s.claude;
  const g = s.agy;
  const names = claudeNames(c.accounts, c.personalHours, model.advanced.on, claudeAliases(model));
  const nameOf = (n: number) => names.get(n)?.name ?? t('Tài khoản ', 'Account ') + n;
  const keychainRequired = s.keychain === 'required';
  const ctx = (section: Section): CardContext => ({
    now,
    personalHours: section.personalHours,
    switching: null,
    agy: null,
  });
  const claudeSwitching = { listed: c.status === 'ok', ready: model.claudeSwitch.ready, busy: model.claudeSwitch.busy };
  const relogin = new Set(model.claudeAccounts.accounts.filter((r) => r.status === 'reloginRequired').map((r) => r.slot));
  const claudeCards = c.accounts.map((a) => cardView(a, 'c', names.get(a.number)!, { ...ctx(c), switching: claudeSwitching, relogin }));
  const agySwitching: AgyRowSwitch = {
    listed: g.status === 'ok' && !!model.agySwitch?.listed,
    busy: model.agySwitch?.busy ?? null,
    logins: new Map((model.antigravity?.accounts ?? []).map((a) => [a.number, a])),
  };
  const gNames = agyNames(g.accounts, model.advanced.on);
  const agyCards = g.status === 'notSetUp' ? [] : g.accounts.map((a) => cardView(a, 'g', gNames.get(a.number)!, { ...ctx(g), agy: agySwitching }));

  // Header freshness: the age of the newest reading, never the time of the app's call.
  let fresh: PopoverView['fresh'];
  const newest = c.newestReadingAt ?? g.newestReadingAt;
  const since = secondsSince(newest, now);
  if (since == null && c.status === 'single' && c.reason) fresh = { tone: 'bad', text: t('không đọc được số', 'cannot read numbers') };
  else if (since == null) fresh = { tone: 'muted', text: model.reading ? t('đang đọc số…', 'reading…') : t('chưa có số', 'no numbers yet') };
  else fresh = { tone: since > STALE_SECONDS ? 'warn' : 'ok', text: t('cập nhật ' + age(since) + ' trước', 'updated ' + age(since) + ' ago') };

  const loading = since == null && model.reading;
  const claudeAge = secondsSince(c.newestReadingAt, now);
  const staleWords = claudeAge != null ? age(claudeAge) : '';
  const pool = c.pool;

  // One banner at a time, most urgent first.
  let banner: BannerView | null = null;
  let dry: BannerView | null;
  const pendingAccount = c.accounts.find((a) => a.reading === 'resetPending');
  const switcher = switcherBanner(model, now);
  const codex = codexBanner(model);
  // An Antigravity switch written over comes as a macOS card with buttons (M6),
  // and stays on the main window's Antigravity tab; the popover has no banner for it.
  if (switcher && switcher.tone !== 'info') banner = switcher;
  else if (pool.exhausted) {
    const who = pool.refills.map(nameOf);
    const at = clock(pool.nextRefillAt);
    banner = {
      tone: 'bad',
      title: t('Hạn mức Claude trong vòng đã cạn', 'Claude quota in rotation is used up'),
      body: at
        ? t('Có lại lúc ' + at + (who.length ? ' khi ' + who.join(' và ') + ' được đặt lại' : '') + '.', 'Back at ' + at + (who.length ? ' when ' + who.join(' and ') + ' resets' : '') + '.')
        : '',
      actions: [],
    };
  } else if ((dry = dryBanner(model, claudeCards, nameOf))) {
    banner = dry;
  } else if (c.stale && staleWords) {
    banner = {
      tone: 'warn',
      icon: 'stale',
      title: t('Số liệu Claude đã cũ ' + staleWords, 'Claude quota data is ' + staleWords + ' old'),
      body: t('Cập nhật thành công lần cuối lúc ' + clock(c.newestReadingAt) + '.', 'Last updated successfully at ' + clock(c.newestReadingAt) + '.'),
      actions: [{ label: t('Làm mới', 'Refresh'), act: 'refresh' }],
    };
  } else if (keychainRequired && !options.keychainLater) {
    banner = {
      tone: 'warn',
      icon: 'perm',
      title: t('Chưa cấp quyền Keychain cho Quota Station', 'Quota Station has no Keychain access yet'),
      body: t(
        'Quota Station cần đọc đăng nhập Claude trong Keychain để lấy hạn mức của từng tài khoản. Cấp quyền một lần.',
        'Quota Station needs to read the Claude sign-in from the Keychain to get each account’s quota. Grant it once.',
      ),
      actions: [
        { label: t('Cấp quyền một lần', 'Grant once'), act: 'keychain' },
        { label: t('Để sau', 'Later'), act: 'dismiss' },
      ],
    };
  } else if (claudeCards.some((x) => x.relogin)) {
    const who = claudeCards.find((x) => x.relogin)!;
    banner = {
      tone: 'info',
      title: t(who.name + ' cần đăng nhập lại', who.name + ' needs to sign in again'),
      body: t(
        'Đăng nhập của ' + who.name + ' trong Quota Station đã hết hạn, nên app không đọc được hạn mức và không chuyển sang được.',
        who.name + '’s sign-in in Quota Station has expired, so the app cannot read its quota or switch to it.',
      ),
      actions: [{ label: t('Đăng nhập lại…', 'Sign in again…'), act: `claude:relogin:${who.slot}` }],
    };
  } else if (codex && codex.tone !== 'info') banner = codex;
  else if (pendingAccount) {
    banner = {
      tone: 'info',
      title: t(nameOf(pendingAccount.number) + ' vừa qua mốc đặt lại', nameOf(pendingAccount.number) + ' just passed its reset'),
      body: t(
        'Đang chờ ' + swapName(model) + ' đọc số mới. App không hiện số khi chưa có số thật.',
        'Waiting for ' + swapName(model) + ' to read the new numbers. The app shows no number until it has a real one.',
      ),
      actions: [{ label: t('Làm mới', 'Refresh'), act: 'refresh' }],
    };
  } else if (codex) banner = codex;
  else if (switcher) banner = switcher;

  // Group notes.
  let claudeNote: NoteView | null = null;
  if (c.status === 'single' && !c.accounts.length && c.reason && !keychainRequired)
    claudeNote = { tone: 'warn', text: t('Không đọc được số của Claude Code (' + c.reason + ').', 'Cannot read Claude Code’s numbers (' + c.reason + ').') };
  else if (c.status === 'notSetUp') claudeNote = { tone: '', text: t('Máy này chưa có Claude Code', 'No Claude Code on this Mac') };
  else if (!model.claudeSwitch.ready && c.accounts.some((a) => a.number > 0))
    claudeNote = {
      tone: 'warn',
      text: t(
        'Quota Station chưa giữ tài khoản Claude nào trên máy này: nhập từ claude-swap ở tab Claude để chuyển được tài khoản.',
        'Quota Station keeps no Claude account on this Mac yet: import from claude-swap in the Claude tab to switch accounts.',
      ),
    };

  let agyNote: NoteView | null = null;
  if (g.status === 'single' && !g.accounts.length && g.reason)
    agyNote = { tone: 'warn', text: t('Không đọc được số Antigravity (' + g.reason + ').', 'Cannot read Antigravity’s numbers (' + g.reason + ').') };
  else if (g.status === 'single') agyNote = { tone: '', text: t('Tài khoản đang đăng nhập trong Antigravity.', 'The account signed in to Antigravity.') };
  else if (g.status === 'unavailable')
    agyNote = { tone: '', text: t('Không đọc được danh sách tài khoản: chỉ thấy tài khoản đang dùng', 'Cannot read the account list: only the account in use shows') };
  else if (g.status === 'notSetUp') agyNote = { tone: '', text: t('Máy này chưa có Antigravity', 'No Antigravity on this Mac') };

  const claudeEmpty = c.status !== 'notSetUp' && !claudeCards.length && model.claudeAccounts.claude && !keychainRequired && !c.reason;
  return {
    machine: model.machine,
    loading,
    none: c.status === 'notSetUp' && g.status === 'notSetUp' && !model.codex.present,
    fresh,
    banner,
    now: nowView(model, claudeCards, agyCards, now),
    claude: {
      id: 'c',
      title: 'Claude',
      readOnly: false,
      // A group with nothing to show (Keychain closed: the banner says why) is left out.
      shown: c.status !== 'notSetUp' && (claudeCards.length > 0 || claudeEmpty || claudeNote != null),
      empty: claudeEmpty,
      cards: bySlot(claudeCards),
      note: claudeNote,
    },
    agy: {
      id: 'g',
      title: 'Antigravity',
      readOnly: g.status === 'single' && g.accounts.length > 0,
      shown: g.status !== 'notSetUp',
      empty: false,
      cards: bySlot(agyCards),
      note: agyNote,
    },
  };
}

/**
 * Claude nearly gone (alert 1's rule, as Cài đặt › Thông báo sets it): the
 * account in use has `dryAtMost` % left or less, and no other account has
 * `roomAtLeast` %. Without a rotation it looks only at the accounts here.
 */
function dryBanner(model: Model, cards: CardView[], nameOf: (n: number) => string): BannerView | null {
  const limits = model.alerts?.poolAlert;
  const active = cards.find((x) => x.active);
  if (!limits || !active?.tight || active.tight.used == null) return null;
  const left = (x: CardView) => (x.tight?.used == null ? 100 : 100 - x.tight.used);
  const activeLeft = left(active);
  const others = cards.filter((x) => !x.active && !x.resting && x.slot > 0);
  if (activeLeft > limits.dryAtMost || others.some((x) => left(x) >= limits.roomAtLeast)) return null;
  // The earliest an account short of room comes back.
  const accounts = model.snapshot.claude.accounts;
  let next: { at: string; slot: number } | null = null;
  for (const x of [active, ...others]) {
    const a = accounts.find((y) => y.number === x.slot);
    const line = a?.windows.find((l) => l.kind === x.tight?.kind) ?? a?.lines.find((l) => l.kind === x.tight?.kind);
    if (line?.resetsAt && (!next || line.resetsAt < next.at)) next = { at: line.resetsAt, slot: x.slot };
  }
  const unit = active.tight.unit;
  const room = limits.roomAtLeast;
  const when = next ? t(' Sớm nhất có lại lúc ' + clock(next.at) + ' (' + nameOf(next.slot) + ').', ' The earliest comes back at ' + clock(next.at) + ' (' + nameOf(next.slot) + ').') : '';
  return {
    tone: 'bad',
    title: t('Hạn mức Claude sắp cạn', 'Claude quota is nearly gone'),
    body:
      t(
        '**' + active.name + '** còn ' + activeLeft + '% hạn mức ' + unit + ' và không tài khoản nào còn từ ' + room + '% trở lên.',
        '**' + active.name + '** has ' + activeLeft + '% of its ' + unit + ' quota left and no account has ' + room + '% or more.',
      ) + when,
    actions: [],
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
  return { tone: '', title: t('Đã làm mới', 'Refreshed'), body: t('Đọc lại số lúc ', 'Read the numbers again at ') + clock(now.toISOString()) + '.' };
}

export function settingsFailedFeedback(): FeedbackView {
  return {
    tone: 'bad',
    title: t('Chưa lưu được cài đặt', 'Settings not saved'),
    body: t('Không ghi được file cài đặt của app trên máy này. Thử lại.', 'Could not write the app’s settings file on this Mac. Try again.'),
  };
}

export function keychainFeedback(model: Model | null): FeedbackView {
  return model?.snapshot.keychain === 'granted'
    ? { tone: '', title: t('Đã cấp quyền Keychain', 'Keychain access granted'), body: t('Dòng Claude giờ có số mới của Claude Code.', 'The Claude row now has Claude Code’s new numbers.') }
    : {
        tone: 'bad',
        title: t('Chưa cấp được quyền Keychain', 'Keychain access not granted'),
        body: t(
          'Quota Station vẫn chưa đọc được Keychain. Thử lại, rồi bấm "Luôn cho phép" khi macOS hỏi.',
          'Quota Station still cannot read the Keychain. Try again, then press “Always Allow” when macOS asks.',
        ),
      };
}

/** Who reads the Claude accounts' numbers: Quota Station once it keeps them, claude-swap before. */
function swapName(model: Model): string {
  return model.claudeSwitch.ready ? 'Quota Station' : 'claude-swap';
}
