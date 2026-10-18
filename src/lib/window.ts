// Turns the model into what the main window shows (M2, read only): the
// Tổng quan, Claude and Antigravity tabs. Built on the popover's view, so the
// window and the popover show the same snapshot with the same names, numbers
// and banners. Pure, like `view.ts`.

import { age, clock, minutesUntil, secondsSince } from './format';
import type { Account, LineKind, Model, Section } from './model';
import { agyName, claudeNames, popoverView, type BannerView, type ChipView, type NoteView, type PoolView, type PopoverView, type RowView, type ViewOptions } from './view';

export interface BigPoolView {
  title: string;
  kind: 'c' | 'g';
  value: string;
  unit: string | null;
  percent: number;
  /** Meter class: the provider colour, or `bad` / `muted`. */
  fill: string;
  line: string;
  /** Hover text: how the rotation number adds up. */
  hover: string;
}

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
}

export interface WindowView {
  machine: string;
  fresh: PopoverView['fresh'];
  banner: BannerView | null;
  pools: { claude: BigPoolView; agy: BigPoolView };
  claude: {
    /** Card title on Tổng quan: `Claude Code · 4 tài khoản`. */
    title: string;
    /** Card title on the Claude tab. */
    tabTitle: string;
    rows: RowView[];
    note: NoteView | null;
  };
  agy: {
    /** False on a Mac without Antigravity: the tab shows an empty state. */
    present: boolean;
    title: string;
    rows: RowView[];
    /** Section note for the overview card. */
    note: NoteView | null;
    /** The Antigravity tab's banner: quota-axi not running, else why it sees only the signed-in account. */
    banner: BannerView | null;
    /** The Antigravity tab's note: the Antigravity read failed while the Claude one did not. */
    tabNote: NoteView | null;
    /** How old the numbers of the accounts not in use are. */
    others: string;
    cockpit: KeyValue[];
  };
  upcoming: UpcomingView[];
}

/** How far ahead Sắp tới looks. */
const UPCOMING_HOURS = 24;
const UPCOMING_MAX = 5;

function cap(s: string): string {
  return s ? s.charAt(0).toUpperCase() + s.slice(1) : s;
}

function bigPool(title: string, p: PoolView): BigPoolView {
  const withParts = p.tone === 'ok' && p.parts.length > 0;
  return {
    title,
    kind: p.kind,
    value: p.value,
    unit: p.unit,
    percent: p.percent,
    fill: p.tone === 'bad' ? 'bad' : p.tone === 'muted' ? 'muted' : p.kind,
    line: cap(p.sub) + (withParts ? ' · ' + p.parts.join(' + ') : ''),
    hover: p.title,
  };
}

// ===== Sắp tới (I15): only times already in the readings, never a forecast =====

const UPCOMING_LABEL: Record<LineKind, string> = {
  session: '5 giờ',
  week: 'tuần',
  fableWeek: 'Fable tuần',
  gemini: 'Gemini',
  claudeGpt: 'Claude/GPT',
};

/** `17:29` today, `ngày mai 03:20` tomorrow. */
function whenWords(iso: string, now: Date): string {
  const d = new Date(iso);
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  return d.toDateString() === tomorrow.toDateString() ? 'ngày mai ' + clock(iso) : clock(iso);
}

function upcoming(model: Model, now: Date): UpcomingView[] {
  const s = model.snapshot;
  const out: { at: string; what: string; tone: UpcomingView['tone'] }[] = [];
  const within = (iso: string | null): iso is string => {
    const m = minutesUntil(iso, now);
    return m != null && m > 0 && m <= UPCOMING_HOURS * 60;
  };
  const add = (section: Section, name: (a: Account) => string, failing: boolean) => {
    if (failing) return;
    for (const a of section.accounts) {
      if (a.leavesAt && within(a.leavesAt)) out.push({ at: a.leavesAt, what: name(a) + ' rời vòng', tone: 'warn' });
      for (const l of a.lines) {
        // A full allowance coming back to full is no news.
        if (!within(l.resetsAt) || (l.percent === 100 && !l.pending)) continue;
        const label = UPCOMING_LABEL[l.kind];
        out.push(
          l.kind === 'week' || l.kind === 'fableWeek'
            ? { at: l.resetsAt, what: 'Hạn mức ' + label + ' của ' + name(a) + ' đặt lại', tone: '' }
            : { at: l.resetsAt, what: name(a) + ' có lại hạn mức ' + label, tone: 'ok' },
        );
      }
    }
  };
  // The one account a provider row reads goes by the provider's name here,
  // where Claude and Antigravity share one list.
  const names = claudeNames(s.claude.accounts);
  add(s.claude, (a) => (a.number === 0 ? 'Claude' : (names.get(a.number)?.name ?? 'Tài khoản ' + a.number)), !!model.errors.claude);
  if (s.agy.status !== 'notSetUp') add(s.agy, (a) => (a.number === 0 ? 'Antigravity' : agyName(a).name), !!model.errors.agy);
  return out
    .sort((x, y) => new Date(x.at).getTime() - new Date(y.at).getTime())
    .slice(0, UPCOMING_MAX)
    .map((u, i) => ({ key: u.at + i, at: whenWords(u.at, now), what: u.what, tone: u.tone }));
}

// ===== Antigravity =====

function agyBanner(g: Section): BannerView | null {
  if (g.status === 'noSidecar')
    return {
      tone: 'info',
      title: 'Chưa thấy Cockpit trên máy này',
      body: 'Số Antigravity lấy từ agy, chỉ có tài khoản đang dùng. Cần Cockpit để thấy mọi tài khoản và chuyển.',
      actions: [],
    };
  if (g.status === 'unavailable')
    return {
      tone: 'warn',
      title: 'Không đọc được Cockpit',
      body: 'Số Antigravity lấy từ agy, chỉ có tài khoản đang dùng' + (g.reason ? ' (' + g.reason + ')' : '') + '.',
      actions: [],
    };
  return null;
}

function agyOthers(g: Section, now: Date): string {
  const read = g.accounts
    .filter((a) => !a.active && a.number > 0)
    .map((a) => {
      const since = secondsSince(a.fetchedAt, now);
      return since == null ? null : agyName(a).name + ' đọc ' + age(since) + ' trước';
    })
    .filter((x): x is string => !!x);
  return (
    'Số của tài khoản không dùng do Cockpit làm mới không đều: ' +
    (read.length ? read.join(', ') + '.' : 'chưa có.') +
    ' App không cảnh báo khi Antigravity xuống thấp.'
  );
}

function cockpitRows(g: Section, failing: boolean): KeyValue[] {
  const chip: ChipView = failing
    ? { t: 'không rõ', k: '' }
    : g.status === 'ok'
      ? { t: 'đang chạy', k: 'ok' }
      : g.status === 'unavailable'
        ? { t: 'không đọc được', k: 'bad' }
        : g.status === 'unread'
          ? { t: 'chưa đọc', k: '' }
          : { t: 'chưa thấy', k: 'bad' };
  const rows: KeyValue[] = [{ key: 'Cockpit Tools', value: chip.t, chip }];
  if (!failing && g.status === 'ok') rows.push({ key: 'Tài khoản trong Cockpit', value: String(g.accounts.length) });
  return rows;
}

// ===== The window =====

export function windowView(model: Model, now: Date, options: ViewOptions = {}): WindowView {
  const pv = popoverView(model, now, options);
  const c = model.snapshot.claude;
  const g = model.snapshot.agy;
  const swap = c.accounts.some((a) => a.number > 0);
  const count = pv.claude.rows.length;
  const cErr = model.errors.claude;
  const gErr = model.errors.agy;
  const present = g.status !== 'notSetUp';
  return {
    machine: pv.machine,
    fresh: pv.fresh,
    banner: pv.banner,
    pools: {
      claude: bigPool('Hạn mức ' + pv.pools.claude.label, pv.pools.claude),
      agy: bigPool('Antigravity', pv.pools.agy),
    },
    claude: {
      title: count ? 'Claude Code · ' + count + ' tài khoản' : 'Claude Code',
      tabTitle: swap ? count + ' tài khoản claude-swap trên máy này' : 'Tài khoản Claude Code trên máy này',
      rows: pv.claude.rows,
      note: pv.claude.note,
    },
    agy: {
      present,
      title: pv.agy.rows.length ? 'Antigravity · ' + pv.agy.rows.length + ' tài khoản' : 'Antigravity',
      rows: pv.agy.rows,
      note: pv.agy.note,
      banner: cErr ? pv.banner : agyBanner(g),
      tabNote: gErr && !cErr ? pv.agy.note : null,
      others: g.status === 'ok' && !gErr ? agyOthers(g, now) : '',
      cockpit: cockpitRows(g, !!gErr),
    },
    upcoming: upcoming(model, now),
  };
}
