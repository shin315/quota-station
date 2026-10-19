// Turns the model into what the main window shows (M2, read only): the
// Tổng quan, Claude and Antigravity tabs. Built on the popover's view, so the
// window and the popover show the same snapshot with the same names, numbers
// and banners. Pure, like `view.ts`.

import { overwriteBanner } from './agySwitch';
import { clock, minutesUntil } from './format';
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
    /** `4 tài khoản claude-swap`. */
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
     * The Antigravity tab's banner: quota-axi not running, else a switch
     * Antigravity.app wrote back ("Chuyển Antigravity bị ghi đè"), else why
     * quota-axi sees only the signed-in account.
     */
    banner: BannerView | null;
    /** The Antigravity tab's note: the Antigravity read failed while the Claude one did not. */
    tabNote: NoteView | null;
    /** quota-axi read the app's own list: the logins are the table's rows. */
    own: boolean;
    /** Cockpit Tools, only when it is where the accounts come from. */
    cockpit: KeyValue[] | null;
  };
  upcoming: UpcomingView[];
  /** Automatic switching and why Claude is on its account (F07). */
  auto: AutoView;
  /** The Claude tab's own banner: the switcher's LaunchAgent is not loaded. */
  claudeBanner: BannerView | null;
  /** The Claude tab's Switcher card: one line, the rest folded. */
  switcher: { line: string; tone: 'ok' | 'warn' | 'bad' | 'muted'; details: KeyValue[] };
}

/** How far ahead Sắp tới looks. */
const UPCOMING_HOURS = 24;
const UPCOMING_MAX = 5;

// ===== Sắp tới (I15): only times already in the readings, never a forecast =====

const UPCOMING_LABEL: Record<LineKind, string> = {
  session: '5 giờ',
  week: 'tuần',
  fableWeek: 'Fable tuần',
  gemini: 'Gemini',
  claudeGpt: 'Claude/GPT',
  geminiSession: 'Gemini 5 giờ',
  geminiWeek: 'Gemini tuần',
  claudeGptSession: 'Claude/GPT 5 giờ',
  claudeGptWeek: 'Claude/GPT tuần',
};

const WEEKLY = new Set<LineKind>(['week', 'fableWeek', 'geminiWeek', 'claudeGptWeek']);

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
      for (const l of a.windows.length ? a.windows : a.lines) {
        // A window nothing was used of coming back is no news.
        if (!within(l.resetsAt) || (l.percent === 100 && !l.pending)) continue;
        const label = UPCOMING_LABEL[l.kind];
        out.push(
          WEEKLY.has(l.kind)
            ? { at: l.resetsAt, what: 'Hạn mức ' + label + ' của ' + name(a) + ' đặt lại', tone: '' }
            : { at: l.resetsAt, what: name(a) + ' có lại hạn mức ' + label, tone: 'ok' },
        );
      }
    }
  };
  // The one account a provider row reads goes by the provider's name here,
  // where Claude and Antigravity share one list.
  const names = claudeNames(s.claude.accounts);
  const gNames = agyNames(s.agy.accounts);
  add(s.claude, (a) => (a.number === 0 ? 'Claude' : (names.get(a.number)?.name ?? 'Tài khoản ' + a.number)), !!model.errors.claude);
  if (s.agy.status !== 'notSetUp') add(s.agy, (a) => (a.number === 0 ? 'Antigravity' : gNames.get(a.number)!.name), !!model.errors.agy);
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
      title: 'Chỉ thấy tài khoản đang dùng',
      body: 'quota-axi chưa có danh sách tài khoản Antigravity trên máy này. Đăng nhập tài khoản ở đây để thấy và chuyển mọi tài khoản.',
      actions: [],
    };
  if (g.status === 'unavailable')
    return {
      tone: 'warn',
      title: 'Không đọc được danh sách tài khoản',
      body: 'Chỉ thấy tài khoản đang dùng' + (g.reason ? ' (' + g.reason + ')' : '') + '.',
      actions: [],
    };
  return null;
}

/** Cockpit Tools: only on a Mac where the accounts come from it. */
function cockpitRows(g: Section, failing: boolean): KeyValue[] | null {
  if (g.source !== 'cockpit') return null;
  const chip: ChipView = failing ? { t: 'không rõ', k: '' } : g.status === 'ok' ? { t: 'đang chạy', k: 'ok' } : { t: 'không đọc được', k: 'bad' };
  const rows: KeyValue[] = [{ key: 'Cockpit Tools', value: chip.t, chip }];
  if (!failing && g.status === 'ok') rows.push({ key: 'Tài khoản trong Cockpit Tools', value: String(g.accounts.length) });
  return rows;
}

// ===== The switcher (F07) =====

function switcherCard(model: Model, auto: AutoView, now: Date): WindowView['switcher'] {
  const sw = model.switcher;
  const chip = (t: string, k: ChipView['k']): ChipView => ({ t, k });
  const service = !sw.installed
    ? chip('không có trên máy này', 'bad')
    : sw.service === true
      ? chip('chạy mỗi 3 phút', 'ok')
      : sw.service === false
        ? chip('chưa nạp', 'bad')
        : chip('không rõ', '');
  const autoChip = sw.autoService === true ? chip('đang nạp', 'warn') : sw.autoService === false ? chip('tắt', 'ok') : chip('không rõ', '');
  const p = pauseOf(model, now);
  const details: KeyValue[] = [
    { key: 'LaunchAgent cswap-prefer-team', value: service.t, chip: service },
    { key: 'com.cswap.auto', value: autoChip.t, chip: autoChip },
    { key: 'Tạm dừng tự chuyển', value: p ? (p.until ? 'đến ' + dayClock(p.until, now) : 'đến khi bật lại') : 'không' },
  ];
  if (sw.guard != null) {
    const guard = sw.guard ? chip('có', 'ok') : chip('chưa', 'bad');
    details.push({ key: 'Switcher đọc cờ tạm dừng', value: guard.t, chip: guard });
  }
  if (auto.raw) details.push({ key: 'Quyết định gần nhất' + (auto.at ? ' ' + auto.at : ''), value: auto.raw, mono: true });
  if (sw.lastLog) details.push({ key: 'Nhật ký' + (sw.lastLog.at ? ' ' + dayClock(sw.lastLog.at, now) : ''), value: sw.lastLog.raw, mono: true });
  let line: string;
  let tone: WindowView['switcher']['tone'];
  if (!sw.installed) [line, tone] = ['Không có cswap-prefer-team trên máy này', 'bad'];
  else if (sw.service === false) [line, tone] = ['LaunchAgent chưa được nạp: không tự chuyển', 'bad'];
  else if (sw.service === true) [line, tone] = ['Chạy mỗi 3 phút' + (auto.at ? ' · kiểm lúc ' + auto.at : ''), 'ok'];
  else [line, tone] = ['Không rõ switcher có chạy không', 'muted'];
  return { line, tone, details };
}

function claudeBanner(model: Model): BannerView | null {
  if (!model.switcher.installed || model.switcher.service !== false) return null;
  return {
    tone: 'bad',
    title: 'Switcher không chạy',
    body: 'LaunchAgent com.cswap.prefer-team chưa được nạp, nên switcher không tự chuyển Claude. Số vẫn cập nhật khi claude-swap làm mới.',
    actions: [
      { label: 'Hướng dẫn nạp lại', act: 'guide:switcher' },
      { label: 'Làm mới', act: 'refresh' },
    ],
  };
}

// ===== The window =====

export function windowView(model: Model, now: Date, options: ViewOptions = {}): WindowView {
  const pv = popoverView(model, now, options);
  const c = model.snapshot.claude;
  const g = model.snapshot.agy;
  const swap = c.accounts.some((a) => a.number > 0);
  const cErr = model.errors.claude;
  const gErr = model.errors.agy;
  const present = g.status !== 'notSetUp';
  const auto = autoView(model, now);
  return {
    machine: pv.machine,
    fresh: pv.fresh,
    banner: pv.banner,
    now: pv.now,
    claude: {
      count: pv.claude.cards.length ? pv.claude.cards.length + ' tài khoản' + (swap ? ' claude-swap' : '') : '',
      cards: pv.claude.cards,
      note: pv.claude.note,
    },
    agy: {
      present,
      cards: pv.agy.cards,
      note: pv.agy.note,
      banner: cErr ? pv.banner : (overwriteBanner(model) ?? agyBanner(g)),
      tabNote: gErr && !cErr ? pv.agy.note : null,
      own: g.status === 'ok' && g.source === 'quota-station',
      cockpit: cockpitRows(g, !!gErr),
    },
    upcoming: upcoming(model, now),
    auto,
    claudeBanner: claudeBanner(model),
    switcher: switcherCard(model, auto, now),
  };
}
