// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Turns the model into what the main window shows (M2, read only): the
// Tổng quan, Claude and Antigravity tabs. Built on the popover's view, so the
// window and the popover show the same snapshot with the same names, numbers
// and banners. Pure, like `view.ts`.

import { overwriteBanner } from '@agy/agySwitch';
import { envBanner } from './env';
import { clock, minutesUntil } from './format';
import { t } from './i18n';
import type { Account, LineKind, Model, Section } from './model';
import { autoView, dayClock, pauseOf, type AutoView } from './switching';
import { agyNames, claudeNames, popoverView, type BannerView, type CardView, type ChipView, type NoteView, type NowView, type PopoverView, type ViewOptions } from './view';

export interface UpcomingView {
  key: string;
  at: string;
  what: string;
  tone: '' | 'ok' | 'warn';
}

export interface KeyValue {
  key: string;
  value: string;
  chip?: ChipView;
  /** Shown in a monospace face: a line as a program printed it. */
  mono?: boolean;
}

export interface WindowView {
  machine: string;
  fresh: PopoverView['fresh'];
  banner: BannerView | null;
  /** Đang dùng, Tổng quan's first card: the account in use on each service. */
  now: NowView;
  claude: {
    /** False on a Mac without Claude Code: the card is left out. */
    present: boolean;
    /** Claude Code is there with no account yet: the card offers to add one. */
    empty: boolean;
    /** `4 tài khoản Quota Station`; `claude-swap` until Quota Station keeps the accounts. */
    count: string;
    /** In slot order, as the tables list them. */
    cards: CardView[];
    note: NoteView | null;
  };
  agy: {
    /** False on a Mac without Antigravity: the tab shows an empty state. */
    present: boolean;
    cards: CardView[];
    /** Section note for the overview card. */
    note: NoteView | null;
    /**
     * The Antigravity tab's banner: a switch Antigravity.app wrote back
     * ("Chuyển Antigravity bị ghi đè"), else why only the signed-in account
     * shows.
     */
    banner: BannerView | null;
    /** The Antigravity tab's note: the Antigravity read gave no number. */
    tabNote: NoteView | null;
    /** The row lists the app's own accounts: the logins are the table's rows. */
    own: boolean;
  };
  upcoming: UpcomingView[];
  /** Automatic switching and why Claude is on its account (F07). */
  auto: AutoView;
  /** The Claude tab's own banner: the rotation's LaunchAgent is not loaded. */
  claudeBanner: BannerView | null;
  /** The Claude tab's red banner: a variable overrides Claude Code's login (M14). */
  envBanner: BannerView | null;
  /** The Claude tab's Switcher card: one line, the rest folded. */
  switcher: { line: string; tone: 'ok' | 'warn' | 'bad' | 'muted'; details: KeyValue[] };
}

/** How far ahead Sắp tới looks. */
const UPCOMING_HOURS = 24;
const UPCOMING_MAX = 5;

// ===== Sắp tới (I15): only times already in the readings, never a forecast =====

const upcomingLabel = (): Record<LineKind, string> => ({
  session: t('5 giờ', '5-hour'),
  week: t('tuần', 'week'),
  fableWeek: t('Fable tuần', 'Fable week'),
  gemini: 'Gemini',
  claudeGpt: 'Claude/GPT',
  geminiSession: t('Gemini 5 giờ', 'Gemini 5-hour'),
  geminiWeek: t('Gemini tuần', 'Gemini week'),
  claudeGptSession: t('Claude/GPT 5 giờ', 'Claude/GPT 5-hour'),
  claudeGptWeek: t('Claude/GPT tuần', 'Claude/GPT week'),
});

const WEEKLY = new Set<LineKind>(['week', 'fableWeek', 'geminiWeek', 'claudeGptWeek']);

/** `17:29` today, `ngày mai 03:20` tomorrow. */
function whenWords(iso: string, now: Date): string {
  const d = new Date(iso);
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  return d.toDateString() === tomorrow.toDateString() ? t('ngày mai ', 'tomorrow ') + clock(iso) : clock(iso);
}

function upcoming(model: Model, now: Date): UpcomingView[] {
  const s = model.snapshot;
  const out: { at: string; what: string; tone: UpcomingView['tone'] }[] = [];
  const within = (iso: string | null): iso is string => {
    const m = minutesUntil(iso, now);
    return m != null && m > 0 && m <= UPCOMING_HOURS * 60;
  };
  const add = (section: Section, name: (a: Account) => string) => {
    for (const a of section.accounts) {
      if (a.leavesAt && within(a.leavesAt)) out.push({ at: a.leavesAt, what: t(name(a) + ' rời vòng', name(a) + ' leaves rotation'), tone: 'warn' });
      for (const l of a.windows.length ? a.windows : a.lines) {
        // A window nothing was used of coming back is no news.
        if (!within(l.resetsAt) || (l.percent === 100 && !l.pending)) continue;
        const label = upcomingLabel()[l.kind];
        out.push(
          WEEKLY.has(l.kind)
            ? { at: l.resetsAt, what: t('Hạn mức ' + label + ' của ' + name(a) + ' đặt lại', name(a) + '’s ' + label + ' quota resets'), tone: '' }
            : { at: l.resetsAt, what: t(name(a) + ' có lại hạn mức ' + label, name(a) + ' gets its ' + label + ' quota back'), tone: 'ok' },
        );
      }
    }
  };
  // The one account a provider row reads goes by the provider's name here,
  // where Claude and Antigravity share one list.
  const names = claudeNames(s.claude.accounts, null, model.advanced.on);
  const gNames = agyNames(s.agy.accounts, model.advanced.on);
  add(s.claude, (a) => (a.number === 0 ? 'Claude' : (names.get(a.number)?.name ?? t('Tài khoản ' + a.number, 'Account ' + a.number))));
  if (s.agy.status !== 'notSetUp') add(s.agy, (a) => (a.number === 0 ? 'Antigravity' : gNames.get(a.number)!.name));
  return out
    .sort((x, y) => new Date(x.at).getTime() - new Date(y.at).getTime())
    .slice(0, UPCOMING_MAX)
    .map((u, i) => ({ key: u.at + i, at: whenWords(u.at, now), what: u.what, tone: u.tone }));
}

// ===== Antigravity =====

function agyBanner(g: Section): BannerView | null {
  if (g.status === 'single')
    return {
      tone: 'info',
      title: t('Chỉ thấy tài khoản đang dùng', 'Only the account in use shows'),
      body: t(
        'Chưa có danh sách tài khoản Antigravity trên máy này. Đăng nhập tài khoản ở đây để thấy và chuyển mọi tài khoản.',
        'There is no Antigravity account list on this Mac yet. Sign in to the accounts here to see and switch them all.',
      ),
      actions: [],
    };
  if (g.status === 'unavailable')
    return {
      tone: 'warn',
      title: t('Không đọc được danh sách tài khoản', 'Cannot read the account list'),
      body: t('Chỉ thấy tài khoản đang dùng', 'Only the account in use shows') + (g.reason ? ' (' + g.reason + ')' : '') + '.',
      actions: [],
    };
  return null;
}

// ===== The switcher (F07) =====

function switcherCard(model: Model, auto: AutoView, now: Date): WindowView['switcher'] {
  const sw = model.switcher;
  const chip = (text: string, k: ChipView['k']): ChipView => ({ t: text, k });
  const service = !sw.installed
    ? chip(t('chưa có tài khoản', 'no account'), 'bad')
    : sw.service === true
      ? chip(t('chạy mỗi 3 phút', 'runs every 3 minutes'), 'ok')
      : sw.service === false
        ? chip(t('tắt', 'off'), 'bad')
        : chip(t('không rõ', 'unknown'), '');
  const legacy = sw.legacy ? chip(t(sw.legacy + ' đang nạp', sw.legacy + ' is loaded'), 'warn') : chip(t('tắt', 'off'), 'ok');
  const p = pauseOf(model, now);
  const details: KeyValue[] = [
    { key: t('LaunchAgent tự chuyển', 'Auto-switch LaunchAgent'), value: service.t, chip: service },
    { key: t('Bộ tự chuyển cũ', 'Old auto-switcher'), value: legacy.t, chip: legacy },
    {
      key: t('Tạm dừng tự chuyển', 'Auto-switch paused'),
      value: p ? (p.until ? t('đến ', 'until ') + dayClock(p.until, now) : t('đến khi bật lại', 'until turned back on')) : t('không', 'no'),
    },
  ];
  if (sw.guard != null) {
    const guard = sw.guard ? chip(t('có', 'yes'), 'ok') : chip(t('chưa', 'not yet'), 'bad');
    details.push({ key: t('Switcher đọc cờ tạm dừng', 'Switcher reads the pause flag'), value: guard.t, chip: guard });
  }
  if (auto.raw) details.push({ key: t('Quyết định gần nhất', 'Latest decision') + (auto.at ? ' ' + auto.at : ''), value: auto.raw, mono: true });
  if (sw.lastLog) details.push({ key: t('Nhật ký', 'Log') + (sw.lastLog.at ? ' ' + dayClock(sw.lastLog.at, now) : ''), value: sw.lastLog.raw, mono: true });
  let line: string;
  let tone: WindowView['switcher']['tone'];
  if (!sw.installed) [line, tone] = [t('Quota Station chưa giữ tài khoản Claude nào', 'Quota Station keeps no Claude account yet'), 'bad'];
  else if (sw.legacy)
    [line, tone] = [t('Bộ tự chuyển cũ ' + sw.legacy + ' còn chạy: Quota Station chưa tự chuyển', 'The old auto-switcher ' + sw.legacy + ' still runs: Quota Station does not switch yet'), 'bad'];
  else if (sw.service === false) [line, tone] = [t('Tự chuyển đang tắt', 'Auto-switch is off'), 'bad'];
  else if (sw.service === true) [line, tone] = [t('Chạy mỗi 3 phút', 'Runs every 3 minutes') + (auto.at ? t(' · kiểm lúc ', ' · checked at ') + auto.at : ''), 'ok'];
  else [line, tone] = [t('Không rõ switcher có chạy không', 'Not sure the switcher runs'), 'muted'];
  return { line, tone, details };
}

function claudeBanner(model: Model): BannerView | null {
  const sw = model.switcher;
  if (!sw.installed || sw.legacy || sw.service !== false) return null;
  return {
    tone: 'bad',
    title: t('Tự chuyển Claude đang tắt', 'Claude auto-switch is off'),
    body: t(
      'LaunchAgent tự chuyển của Quota Station chưa được nạp, nên không có gì tự chuyển Claude. Số vẫn cập nhật khi app đọc.',
      'Quota Station’s auto-switch LaunchAgent is not loaded, so nothing switches Claude. The numbers still update when the app reads them.',
    ),
    actions: [
      { label: t('Bật tự chuyển', 'Turn on auto-switch'), act: 'claude:rotation:on' },
      { label: t('Làm mới', 'Refresh'), act: 'refresh' },
    ],
  };
}

// ===== The window =====

/** `4 tài khoản Quota Station`, `4 Quota Station accounts`; empty with none. */
function accountCount(n: number, keeper: string): string {
  if (!n) return '';
  const vi = n + ' tài khoản' + (keeper ? ' ' + keeper : '');
  const en = n + (keeper ? ' ' + keeper : '') + (n === 1 ? ' account' : ' accounts');
  return t(vi, en);
}

export function windowView(model: Model, now: Date, options: ViewOptions = {}): WindowView {
  const pv = popoverView(model, now, options);
  const c = model.snapshot.claude;
  const g = model.snapshot.agy;
  const swap = c.accounts.some((a) => a.number > 0);
  const present = g.status !== 'notSetUp';
  const auto = autoView(model, now);
  return {
    machine: pv.machine,
    fresh: pv.fresh,
    banner: pv.banner,
    now: pv.now,
    claude: {
      present: pv.claude.shown,
      empty: pv.claude.empty,
      count: accountCount(pv.claude.cards.length, swap ? (model.claudeSwitch.ready ? 'Quota Station' : 'claude-swap') : ''),
      cards: pv.claude.cards,
      note: pv.claude.note,
    },
    agy: {
      present,
      cards: pv.agy.cards,
      note: pv.agy.note,
      banner: overwriteBanner(model) ?? agyBanner(g),
      tabNote: g.status === 'single' && !g.accounts.length && g.reason ? pv.agy.note : null,
      own: g.status === 'ok' && g.source === 'quota-station',
    },
    upcoming: upcoming(model, now),
    auto,
    claudeBanner: claudeBanner(model),
    envBanner: envBanner(model),
    switcher: switcherCard(model, auto, now),
  };
}
