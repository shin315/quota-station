// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the switching parts show (M3): the Claude switch sheet (F04), the
// pause sheet and the switcher's "why" line (F07), and the feedback after
// each action. Pure, like `view.ts`: the same model and clock give the same
// words. Copy follows the approved prototype (`sheetClaude`, `sheetPause`,
// `fbLine`, `whyCard`, wording review W01–W11, W43).

import { clock, pad, percent } from './format';
import type { Account, Decision, Hold, LogLine, Model, Outcome, Sessions, Why } from './model';
import { BAD_USED, claudeCells, tightest } from './accounts';
import { claudeNames, type Action, type BannerView, type FeedbackView } from './view';

/** The switcher's LaunchAgent runs every 180 seconds. */
const SWITCHER_EVERY = 'trong vòng 3 phút';

/** `Team B`, or `tài khoản 5` for a slot the snapshot does not list. */
export type NameOf = (slot: number) => string;

export function namesOf(model: Model): NameOf {
  const names = claudeNames(model.snapshot.claude.accounts, null, model.advanced.on);
  return (n) => names.get(n)?.name ?? 'tài khoản ' + n;
}

function addMinutes(d: Date, minutes: number): Date {
  return new Date(d.getTime() + minutes * 60000);
}

/** `17:29` today, `ngày mai 03:20` tomorrow, `22/01 03:20` later or earlier. */
export function dayClock(iso: string, now: Date): string {
  const d = new Date(iso);
  const day = (x: Date) => x.toDateString();
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (day(d) === day(now)) return clock(iso);
  if (day(d) === day(tomorrow)) return 'ngày mai ' + clock(iso);
  if (day(d) === day(yesterday)) return 'hôm qua ' + clock(iso);
  return pad(d.getDate()) + '/' + pad(d.getMonth() + 1) + ' ' + clock(iso);
}

// ===== Is the switcher paused? =====

export interface PauseState {
  /** RFC 3339; null when there is no end. */
  until: string | null;
  reason: string | null;
}

/**
 * The pause in force now: the flag holds and its end, if any, is still ahead
 * on the window's clock. A switcher that showed it ignores the flag is not
 * paused, whatever the flag says.
 */
export function pauseOf(model: Model, now: Date): PauseState | null {
  const p = model.switcher.pause;
  if (!p || !p.holds || model.switcher.guard === false) return null;
  if (p.until && new Date(p.until).getTime() <= now.getTime()) return null;
  return { until: p.until, reason: p.reason };
}

function holdWords(name: string, p: PauseState, now: Date): string {
  return 'Giữ ' + name + (p.until ? ' đến ' + dayClock(p.until, now) : ' đến khi bạn bật lại');
}

/** `Đang tạm dừng tới 18:30: chuyển tay sang tài khoản 3 lúc 17:30` */
export function pausedLine(p: PauseState, now: Date): string {
  const head = p.until ? 'Đang tạm dừng tới ' + dayClock(p.until, now) : 'Đang tạm dừng đến khi bạn bật lại';
  return head + (p.reason ? ': ' + p.reason : '');
}

// ===== Hold and pause choices =====

export type HoldChoice = '30' | '60' | 'reset' | 'none';
export type PauseChoice = '60' | '1800' | 'inf';

export interface ChoiceView<V extends string> {
  v: V;
  t: string;
  s: string;
  /** When the hold or pause ends; null for none or no end. */
  until: Date | null;
}

/**
 * When the target's five-hour allowance resets: its own reset when the
 * window runs, else five hours from now, when a window started by use would
 * end. The prototype's "Đến lúc đặt lại".
 */
function nextReset(target: Account, now: Date): Date {
  const session = target.lines.find((l) => l.kind === 'session' && l.resetsAt && new Date(l.resetsAt) > now);
  return session?.resetsAt ? new Date(session.resetsAt) : addMinutes(now, 300);
}

/** G02: four ways to hold the account switched to, each with its own end. */
export function holdOptions(target: Account, now: Date): ChoiceView<HoldChoice>[] {
  const at = (d: Date) => 'đến ' + dayClock(d.toISOString(), now);
  const h30 = addMinutes(now, 30);
  const h60 = addMinutes(now, 60);
  const reset = nextReset(target, now);
  return [
    { v: '30', t: '30 phút', s: at(h30), until: h30 },
    { v: '60', t: '1 giờ', s: at(h60), until: h60 },
    { v: 'reset', t: 'Đến lúc đặt lại', s: at(reset), until: reset },
    { v: 'none', t: 'Để tự chuyển', s: 'không giữ', until: null },
  ];
}

/** G03: how long to pause; "Đến 18:00" only before 18:00. */
export function pauseOptions(now: Date): ChoiceView<PauseChoice>[] {
  const h60 = addMinutes(now, 60);
  const six = new Date(now);
  six.setHours(18, 0, 0, 0);
  const out: ChoiceView<PauseChoice>[] = [{ v: '60', t: '1 giờ', s: 'đến ' + dayClock(h60.toISOString(), now), until: h60 }];
  if (now < six) out.push({ v: '1800', t: 'Đến 18:00', s: 'hôm nay', until: six });
  out.push({ v: 'inf', t: 'Đến khi tôi bật lại', s: 'không có giờ hết hạn', until: null });
  return out;
}

// ===== The Claude switch sheet (F04) =====

export interface FactView {
  /** `.facts li` class: '' bullet, `w` warning, `x` blocker, `ok` a promise kept. */
  k: '' | 'w' | 'x' | 'ok';
  /** Text with **bold** runs. */
  text: string;
}

export interface ClaudeSheetView {
  title: string;
  facts: FactView[];
  options: ChoiceView<HoldChoice>[];
  hint: { tone: '' | 'warn'; text: string };
  confirm: string;
  /** Nothing can switch now (no accounts kept yet, or an older switcher loaded): the confirm button is off. */
  blocked: boolean;
}

function inHours(range: string | null, now: Date): boolean | null {
  const m = /^(\d{1,2})-(\d{1,2})$/.exec(range ?? '');
  if (!m) return null;
  const h = now.getHours();
  return h >= Number(m[1]) && h <= Number(m[2]);
}

function sessionsFact(sessions: Sessions | null | 'loading'): FactView {
  if (sessions === 'loading') return { k: '', text: 'Đang đếm các phiên Claude đang chạy trên máy này…' };
  if (!sessions)
    return { k: 'w', text: 'Các phiên Claude đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo. Việc này dùng thêm hạn mức.' };
  if (sessions.live === 0 && sessions.unreadable === 0)
    return { k: '', text: 'Không có phiên Claude nào đang chạy trên máy này, nên không phiên nào phải tạo lại bộ nhớ đệm.' };
  if (sessions.live === 0)
    return {
      k: 'w',
      text:
        'Không đọc được ' + sessions.unreadable + ' bản ghi phiên Claude trên máy này, nên có thể có phiên đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo.',
    };
  const n = (sessions.unreadable ? 'Ít nhất ' : '') + sessions.live + ' phiên Claude';
  return { k: 'w', text: '**' + n + '** đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo. Việc này dùng thêm hạn mức.' };
}

export function claudeSheet(
  model: Model,
  slot: number,
  choice: HoldChoice,
  sessions: Sessions | null | 'loading',
  now: Date,
): ClaudeSheetView | null {
  const c = model.snapshot.claude;
  const target = c.accounts.find((a) => a.number === slot);
  if (!target) return null;
  const nameOf = namesOf(model);
  const name = nameOf(slot);
  const facts: FactView[] = [];
  // Every window, as the share used.
  const cells = claudeCells(target, now).filter((c) => !c.none);
  const parts = cells.map((c) => (c.used == null ? 'chưa có số' : '**' + c.text + '**') + ' ' + c.unit);
  if (parts.length) facts.push({ k: '', text: name + ' đã dùng ' + parts.join(', ') + '.' });
  facts.push(sessionsFact(sessions));

  // The rotation and its hold are Nâng cao's: without it the sheet only switches.
  const auto = model.advanced.on;
  const personalOut = auto && target.role === 'personal' && inHours(c.personalHours, now) === false;
  if (personalOut) {
    const m = /^(\d{1,2})-(\d{1,2})$/.exec(c.personalHours ?? '')!;
    facts.push({
      k: 'w',
      text:
        'Bây giờ là ' + clock(now.toISOString()) + ', ngoài giờ ' + pad(Number(m[1])) + '–' + pad(Number(m[2])) +
        ' của tài khoản Cá nhân. Nếu để tự chuyển, switcher sẽ đổi sang tài khoản khác ' + SWITCHER_EVERY + '.',
    });
  } else if (auto && (target.membership === 'out' || target.membership === 'ended')) {
    facts.push({ k: 'w', text: name + ' không còn trong vòng xoay. Nếu để tự chuyển, switcher sẽ đổi sang tài khoản trong vòng.' });
  }
  const tight = tightest(cells);
  if (tight && tight.used! >= BAD_USED) facts.push({ k: 'w', text: name + ' đã dùng ' + tight.used + '% hạn mức ' + tight.unit + '.' });
  const legacy = model.switcher.legacy;
  const blocked = !model.claudeSwitch.ready || !!legacy;
  if (!model.claudeSwitch.ready)
    facts.push({ k: 'x', text: 'Quota Station chưa giữ tài khoản Claude nào trên máy này. Nhập từ claude-swap hoặc thêm tài khoản ở tab Claude.' });
  else if (legacy)
    facts.push({ k: 'x', text: 'Bộ tự chuyển cũ `' + legacy + '` còn chạy. Tắt nó trước (hướng dẫn ở tab Claude), để hai bên không chuyển chồng nhau.' });

  const title = 'Chuyển Claude sang ' + name + '?';
  const confirm = 'Chuyển sang ' + name;
  if (!auto) return { title, facts, options: [], hint: { tone: '', text: '' }, confirm, blocked };
  const options = holdOptions(target, now);
  const cur = options.find((o) => o.v === choice) ?? options[1];
  const paused = pauseOf(model, now);
  let hint: ClaudeSheetView['hint'];
  if (cur.until && paused && outlasts(paused, cur.until)) {
    // The hold is written to the minute, and never shortens a longer pause.
    hint = { tone: '', text: pausedLine(paused, now) + '. Lần tạm dừng này dài hơn nên được giữ nguyên.' };
  } else if (cur.until) {
    hint = { tone: '', text: 'Tạm dừng tự chuyển đến ' + dayClock(cur.until.toISOString(), now) + ' để giữ tài khoản này.' };
    if (model.switcher.guard === false)
      hint = { tone: 'warn', text: 'Switcher trên máy này chưa đọc cờ tạm dừng, nên vẫn có thể đổi tài khoản.' };
  } else if (paused) {
    hint = { tone: '', text: pausedLine(paused, now) + '. Tự chuyển vẫn tạm dừng.' };
  } else if (personalOut) {
    hint = { tone: 'warn', text: 'Ngoài giờ dùng tài khoản Cá nhân. Chọn thời lượng giữ để tránh tự chuyển sang tài khoản khác.' };
  } else {
    hint = { tone: '', text: 'Tự chuyển vẫn bật. Switcher có thể đổi sang tài khoản khác ở lần kiểm sau (' + SWITCHER_EVERY + ').' };
  }
  return { title, facts, options, hint, confirm, blocked };
}

/** The pause runs past a hold ending at `until`, which is written to the minute. */
function outlasts(p: PauseState, until: Date): boolean {
  const minute = new Date(until);
  minute.setSeconds(0, 0);
  return !p.until || new Date(p.until) > minute;
}

// ===== The pause sheet (F07) =====

export interface PauseSheetView {
  title: string;
  lead: string;
  options: ChoiceView<PauseChoice>[];
  hint: string;
}

export function pauseSheet(model: Model, choice: PauseChoice, now: Date): PauseSheetView {
  const active = model.snapshot.claude.accounts.find((a) => a.active);
  const who = active && active.number > 0 ? namesOf(model)(active.number) : 'tài khoản đang dùng';
  const options = pauseOptions(now);
  const cur = options.find((o) => o.v === choice) ?? options[0];
  return {
    title: 'Tạm dừng tự chuyển Claude?',
    lead: 'Giữ ' + who + '. Switcher sẽ không đổi tài khoản Claude trong thời gian này. Hạn mức vẫn cập nhật.',
    options,
    hint: cur.until
      ? 'Hết thời gian này, tự chuyển sẽ bật lại. Firstmate vẫn thấy hạn mức trong vòng.'
      : 'Tự chuyển chỉ bật lại khi bạn bật công tắc. Firstmate vẫn thấy hạn mức trong vòng.',
  };
}

// ===== "Vì sao đang ở tài khoản này" (F07) =====

function used(n: number): string {
  return percent(n);
}

export function whyText(why: Why, name: string, nameOf: NameOf): string {
  switch (why.kind) {
    case 'expiring':
      return 'ưu tiên dùng ' + name + ' trước khi tài khoản rời vòng lúc ' + why.at + '.';
    case 'early':
      return 'hạn mức 5 giờ của ' + name + ' đặt lại lúc ' + why.at + ' khi mới dùng ' + (100 - why.left) + '%, sớm hơn ' + nameOf(why.before) + '.';
    case 'room':
      return name + ' còn hạn mức (đã dùng ' + used(why.used) + (why.ends ? ', hạn mức 5 giờ đặt lại lúc ' + why.ends : '') + ').';
    case 'personal':
      return 'không tài khoản Team nào còn hạn mức (' + name + ' đã dùng ' + used(why.used) + ').';
    case 'park':
      return 'không tài khoản nào còn hạn mức, ' + name + ' được đặt lại sớm nhất (' + why.at + ').';
    default:
      return 'xem dòng gốc bên dưới.';
  }
}

/** The switcher's decision in words (W09, W10, W43). */
export function decisionText(d: Decision, raw: string, active: string | null, nameOf: NameOf): string {
  const a = active ?? 'tài khoản đang dùng';
  switch (d.kind) {
    case 'stay':
      return 'Vẫn dùng ' + a + ': chưa tới ngưỡng tự chuyển.';
    case 'unknown':
      return 'Vẫn dùng ' + nameOf(d.slot) + ': chưa có số mới để quyết định.';
    case 'waiting':
      return 'Vẫn dùng ' + a + ': không tài khoản nào còn hạn mức, chờ ' + nameOf(d.slot) + ' được đặt lại lúc ' + d.at + '.';
    case 'wait':
      return 'Không tài khoản nào còn hạn mức. Switcher giữ tài khoản đang dùng.';
    case 'switch':
      return 'Lần kiểm tới sẽ chuyển sang ' + nameOf(d.slot) + ': ' + whyText(d.why, nameOf(d.slot), nameOf);
    case 'paused':
      return 'Switcher đang tạm dừng' + (d.reason ? ': ' + d.reason : '') + '.';
    case 'blocked':
      return 'Chưa tự chuyển: bộ tự chuyển cũ ' + d.label + ' còn chạy.';
    default:
      return 'Switcher báo: ' + raw;
  }
}

/** The switcher's last log line in words. */
export function logText(line: LogLine, nameOf: NameOf, now: Date): string {
  const at = line.at ? dayClock(line.at, now) + ' · ' : '';
  switch (line.event.kind) {
    case 'switch':
      return at + 'Switcher chuyển sang ' + nameOf(line.event.slot) + '.';
    case 'switchFailed':
      return at + 'Switcher không chuyển được sang ' + nameOf(line.event.slot) + '.';
    case 'primed':
      return at + 'Switcher bắt đầu tính hạn mức 5 giờ của ' + nameOf(line.event.slot) + '.';
    case 'pauseEnded':
      return at + 'Hết tạm dừng, tự chuyển bật lại.';
    default:
      return line.raw;
  }
}

export interface AutoView {
  paused: PauseState | null;
  title: string;
  /** The why line: the pause, else the switcher's decision with its time. */
  line: string;
  /**
   * The popover's why line: the same, only when there is something to act
   * on (a pause or hold, waiting for quota, a switch coming, the switcher
   * missing or failing); null while Claude simply stays on its account.
   */
  popLine: string | null;
  /** The decision alone, for the overview's "Vì sao" card. */
  why: string;
  /** The switcher's last log line in words. */
  log: string | null;
  /** The decision as the switcher printed it, and when it ran (`HH:MM`). */
  raw: string | null;
  at: string | null;
  logRaw: string | null;
  toggle: { on: boolean; disabled: boolean; label: string };
}

export function autoView(model: Model, now: Date): AutoView {
  const sw = model.switcher;
  const nameOf = namesOf(model);
  const activeAcc = model.snapshot.claude.accounts.find((a) => a.active && a.number > 0);
  const active = activeAcc ? nameOf(activeAcc.number) : null;
  const paused = pauseOf(model, now);
  const at = sw.dryRun ? clock(sw.dryRun.at) : null;
  let why: string;
  if (!sw.installed) why = 'Quota Station chưa giữ tài khoản Claude nào trên máy này: chưa có tự chuyển Claude.';
  else if (sw.legacy) why = 'Bộ tự chuyển cũ ' + sw.legacy + ' còn chạy, nên Quota Station chưa tự chuyển.';
  else if (sw.service === false) why = 'Tự chuyển của Quota Station đang tắt: bật nó ở tab Claude.';
  else if (sw.dryRun) why = decisionText(sw.dryRun.decision, sw.dryRun.raw, active, nameOf);
  else if (sw.dryRunError) why = 'Chưa đọc được quyết định của switcher (' + dryRunReason(sw.dryRunError) + ').';
  else why = 'Đang hỏi switcher…';
  const off = !sw.installed || !!sw.legacy || sw.service === false;
  const line = paused ? pausedLine(paused, now) : at && sw.dryRun && !off ? at + ' · ' + why : why;
  const usual = !paused && !off && (sw.dryRun ? sw.dryRun.decision.kind === 'stay' : !sw.dryRunError);
  return {
    paused,
    title: paused ? 'Tự chuyển đang tạm dừng' : 'Tự chuyển Claude',
    line,
    popLine: usual ? null : line,
    why: paused ? pausedLine(paused, now) + '.' : why,
    log: sw.lastLog ? logText(sw.lastLog, nameOf, now) : null,
    raw: sw.dryRun?.raw ?? null,
    at,
    logRaw: sw.lastLog?.raw ?? null,
    toggle: {
      on: !paused && !off,
      disabled: off && !paused,
      label: paused ? 'Tự chuyển Claude, đang tạm dừng' : 'Tự chuyển Claude',
    },
  };
}

function dryRunReason(kind: string): string {
  switch (kind) {
    case 'timeout':
      return 'quá thời gian chờ';
    case 'noAccounts':
      return 'chưa có tài khoản';
    default:
      return 'switcher chạy lỗi';
  }
}

// ===== Banners =====

/** The banner for the switcher, most urgent first; null when all is well. */
export function switcherBanner(model: Model, now: Date): BannerView | null {
  const sw = model.switcher;
  const flag = sw.pause;
  if (flag && flag.holds && sw.guard === false)
    return {
      tone: 'bad',
      title: 'Switcher chưa đọc cờ tạm dừng',
      body: 'Bộ tự chuyển trên máy này không dừng theo cờ tạm dừng, nên vẫn tự chuyển. Cho phép tự chuyển lại, hoặc tắt bộ tự chuyển đó.',
      actions: [{ label: 'Cho phép tự chuyển lại', act: 'resume' }],
    };
  if (sw.legacy && sw.installed)
    return {
      tone: 'warn',
      title: 'Bộ tự chuyển cũ còn chạy',
      body:
        sw.legacy +
        ' vẫn được nạp. Quota Station không chuyển tài khoản khi nó còn chạy, để hai bên không chuyển chồng nhau. Tắt nó theo hướng dẫn chuyển.',
      actions: [{ label: 'Hướng dẫn tắt', act: 'guide:auto' }],
    };
  return pausedBanner(model, now);
}

export function pausedBanner(model: Model, now: Date): BannerView | null {
  const p = pauseOf(model, now);
  if (!p) return null;
  const active = model.snapshot.claude.accounts.find((a) => a.active && a.number > 0);
  const who = active ? namesOf(model)(active.number) : 'tài khoản đang dùng';
  return {
    tone: 'info',
    title: 'Đang giữ ' + who + (p.until ? ' đến ' + dayClock(p.until, now) : ' đến khi bạn bật lại'),
    body: p.until
      ? 'Tự chuyển Claude đang tạm dừng. Hết thời gian này, tự chuyển sẽ bật lại.'
      : 'Tự chuyển Claude đang tạm dừng cho tới khi bạn bật lại.',
    actions: [{ label: 'Cho phép tự chuyển lại', act: 'resume' }, ...(p.until ? [{ label: 'Thêm 1 giờ', act: 'addHour' as Action }] : [])],
  };
}

/** The summary's first item while paused: `Giữ Team B đến 18:30`. */
export function pausedSummary(model: Model, now: Date): string | null {
  const p = pauseOf(model, now);
  if (!p) return null;
  const active = model.snapshot.claude.accounts.find((a) => a.active && a.number > 0);
  return holdWords(active ? namesOf(model)(active.number) : 'tài khoản đang dùng', p, now);
}

// ===== Feedback after an action =====

function holdTail(hold: Hold | null, now: Date): string {
  if (!hold) return '';
  return hold.failed
    ? ' Chưa ghi được cờ tạm dừng, nên switcher vẫn có thể đổi tài khoản ở lần kiểm sau.'
    : ' Tự chuyển Claude tạm dừng tới ' + dayClock(hold.until, now) + '.';
}

const CLOSE = { label: 'Đóng', act: 'closeFeedback' as Action };

export function switchFeedback(outcome: Outcome, model: Model, now: Date): FeedbackView {
  const nameOf = namesOf(model);
  const t = nameOf(outcome.slot);
  const retry = { label: 'Thử lại', act: ('switch:' + outcome.slot) as Action };
  switch (outcome.kind) {
    case 'switched': {
      const h = outcome.hold;
      if (h && !h.failed)
        return {
          tone: '',
          title: 'Đã chuyển sang ' + t + '. Giữ tài khoản này đến ' + dayClock(h.until, now) + '.',
          body: 'Đọc lại sau 5 giây: ' + t + ' đang dùng ✓. Tự chuyển Claude tạm dừng tới lúc đó.',
          actions: [{ label: 'Cho phép tự chuyển lại', act: 'resume' }, CLOSE],
        };
      const paused = !h ? pauseOf(model, now) : null;
      return {
        tone: h?.failed ? 'warn' : '',
        title: 'Đã chuyển sang ' + t + '.',
        body:
          'Đọc lại sau 5 giây: ' + t + ' đang dùng ✓.' +
          (h
            ? holdTail(h, now)
            : paused
              ? ' ' + pausedLine(paused, now) + '.'
              : ' Tự chuyển vẫn bật: switcher kiểm lại ' + SWITCHER_EVERY + ' và có thể đổi tài khoản.'),
        actions: [CLOSE],
      };
    }
    case 'raced': {
      const a = nameOf(outcome.active);
      const h = outcome.hold;
      return {
        tone: 'warn',
        title: outcome.bySwitcherAt
          ? 'Switcher vừa chuyển sang ' + a + ' lúc ' + clock(outcome.bySwitcherAt)
          : 'Một lần chuyển khác vừa chuyển sang ' + a,
        body: 'Hai lần ghi gần nhau, lần sau thắng. Đang dùng ' + a + '.' + holdTail(h, now),
        actions: [
          { label: 'Chuyển lại sang ' + t, act: ('switch:' + outcome.slot) as Action },
          ...(h && !h.failed ? [{ label: 'Cho phép tự chuyển lại', act: 'resume' as Action }] : []),
          { label: 'Để vậy', act: 'closeFeedback' },
        ],
      };
    }
    case 'unconfirmed':
      return {
        tone: 'warn',
        title: 'Đã gửi lệnh chuyển sang ' + t,
        body: 'Đã ghi đăng nhập mới, nhưng Claude Code giờ đang ở một tài khoản Quota Station không giữ.' + holdTail(outcome.hold, now),
        actions: [{ label: 'Làm mới', act: 'refresh' }, CLOSE],
      };
    case 'failed': {
      const body = failedBody(outcome, nameOf);
      const stays = outcome.error === 'legacy' || outcome.error === 'unknownSlot' || outcome.error === 'unmanaged';
      return {
        tone: 'bad',
        title: 'Chưa chuyển được sang ' + t,
        body,
        actions: stays ? [CLOSE] : [retry, CLOSE],
      };
    }
  }
}

/** Why a switch did not land, in words. */
function failedBody(outcome: Extract<Outcome, { kind: 'failed' }>, nameOf: NameOf): string {
  const detail = outcome.detail ? ' (' + outcome.detail + ')' : '';
  switch (outcome.error) {
    case 'legacy':
      return 'Bộ tự chuyển cũ ' + outcome.detail + ' còn chạy. Tắt nó trước theo hướng dẫn ở tab Claude. Chưa đổi gì.';
    case 'busy':
      return 'Claude Code đang giữ khoá đăng nhập (đang làm mới hoặc ghi đăng nhập). Thử lại sau vài giây. Chưa đổi gì.';
    case 'noCredential':
      return 'Quota Station không có đăng nhập dùng được của tài khoản này. Đăng nhập lại nó ở tab Claude. Chưa đổi gì.';
    case 'unmanaged':
      return 'Claude Code đang đăng nhập một tài khoản Quota Station chưa giữ. Thêm tài khoản đang đăng nhập ở tab Claude trước, để không mất đăng nhập đó. Chưa đổi gì.';
    case 'unconfirmed':
      return outcome.active != null
        ? 'Ghi xong nhưng đọc lại không khớp, và không đặt lại được như cũ. Đang dùng ' + nameOf(outcome.active) + '.'
        : 'Ghi xong nhưng đọc lại không khớp, nên app đã đặt lại như cũ. Chưa đổi gì.';
    case 'unknownSlot':
      return 'Quota Station không giữ tài khoản này. Chưa đổi gì.';
    case 'keychain':
      return 'Keychain không cho đọc hoặc ghi đăng nhập' + detail + '. Chưa đổi gì.';
    default:
      return 'Chuyển không được' + detail + '. Chưa đổi gì.';
  }
}

/** The switch call itself was refused (`busy`, `unknownSlot`) or broke. */
export function switchErrorFeedback(slot: number, error: unknown, model: Model): FeedbackView {
  const t = namesOf(model)(slot);
  const why = String(error);
  return {
    tone: 'bad',
    title: 'Chưa chuyển được sang ' + t,
    body:
      why === 'busy'
        ? 'Một lần chuyển khác đang chạy. Chờ nó xong rồi thử lại.'
        : why === 'unknownSlot'
          ? 'Quota Station không còn giữ tài khoản này. Bấm Làm mới rồi thử lại.'
          : 'Không gọi được lệnh chuyển. Chưa đổi gì.',
    actions: [CLOSE],
  };
}

export function pausedFeedback(model: Model, now: Date): FeedbackView {
  const p = pauseOf(model, now);
  const active = model.snapshot.claude.accounts.find((a) => a.active && a.number > 0);
  const who = active ? namesOf(model)(active.number) : 'tài khoản đang dùng';
  return {
    tone: 'info',
    title: 'Đã tạm dừng tự chuyển Claude',
    body: p
      ? p.until
        ? 'Giữ ' + who + ' đến ' + dayClock(p.until, now) + '. Hết thời gian này, tự chuyển sẽ bật lại.'
        : 'Giữ ' + who + ' đến khi bạn bật lại. Bật lại bằng công tắc Tự chuyển Claude.'
      : 'Tự chuyển Claude đang tạm dừng.',
    actions: [{ label: 'Bật lại tự chuyển', act: 'resume' }, CLOSE],
  };
}

export function resumedFeedback(): FeedbackView {
  return { tone: '', title: 'Tự chuyển Claude đã bật lại', body: 'Switcher kiểm lại ' + SWITCHER_EVERY + '.' };
}

export function extendedFeedback(model: Model, now: Date): FeedbackView {
  const s = pausedSummary(model, now);
  return { tone: 'info', title: s ?? 'Đã thêm 1 giờ', body: 'Đã thêm 1 giờ. Tự chuyển Claude vẫn tạm dừng.' };
}

export function pauseFailedFeedback(resume: boolean): FeedbackView {
  return resume
    ? { tone: 'bad', title: 'Chưa bật lại được tự chuyển', body: 'Không xoá được cờ tạm dừng của switcher trên máy này. Thử lại.' }
    : { tone: 'bad', title: 'Chưa ghi được cờ tạm dừng', body: 'Không ghi được cờ tạm dừng của switcher trên máy này. Tự chuyển vẫn bật.' };
}

// ===== Guides =====

export interface GuideView {
  title: string;
  lead: string;
  steps: string[];
}

/**
 * `auto`: stop the older switchers (step 1 of the cutover); `switcher`: the whole cutover from
 * claude-swap, as the README's checklist has it.
 */
export function guide(id: 'auto' | 'switcher'): GuideView {
  return id === 'auto'
    ? {
        title: 'Tắt bộ tự chuyển cũ',
        lead:
          'Quota Station không chuyển tài khoản khi com.cswap.prefer-team hoặc com.cswap.auto còn được nạp, để hai bên không chuyển chồng nhau. Quota Station không tự chạy các lệnh này; claude-swap và dữ liệu của nó vẫn giữ nguyên.',
        steps: [
          'Mở Terminal trên máy này.',
          'Giữ một bản sao plist: `cp ~/Library/LaunchAgents/com.cswap.prefer-team.plist ~/Library/LaunchAgents/com.cswap.prefer-team.plist.bak`',
          'Chạy `launchctl bootout gui/$(id -u)/com.cswap.prefer-team`',
          'Nếu com.cswap.auto cũng đang nạp: `launchctl bootout gui/$(id -u)/com.cswap.auto`',
          'Quay lại tab Claude và bấm Làm mới. Cảnh báo bộ tự chuyển cũ phải biến mất.',
        ],
      }
    : {
        title: 'Chuyển từ claude-swap sang Quota Station',
        lead: 'Làm lần lượt trên từng Mac. claude-swap và dữ liệu của nó được giữ nguyên, để quay lại được khi cần.',
        steps: [
          'Tắt bộ tự chuyển cũ: giữ bản sao plist rồi `launchctl bootout gui/$(id -u)/com.cswap.prefer-team`.',
          'Ở tab Claude, bấm **Nhập từ claude-swap**. Chạy lại lần nữa cũng không sao: chỉ đăng nhập mới hơn được lấy thêm.',
          'Kiểm tra danh sách tài khoản, rồi bật **Tự chuyển bằng Quota Station**.',
          'Firstmate đọc số từ `quota-station claude status --json` thay cho `cswap list --json`, và chạy `quota-station claude run N --` thay cho `cswap run N --`.',
          'Chỉ gỡ claude-swap khi đã hỏi và mọi Mac đã chạy ổn.',
        ],
      };
}
