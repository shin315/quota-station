// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the switching parts show (M3): the Claude switch sheet (F04), the
// pause sheet and the switcher's "why" line (F07), and the feedback after
// each action. Pure, like `view.ts`: the same model and clock give the same
// words. Copy follows the approved prototype (`sheetClaude`, `sheetPause`,
// `fbLine`, `whyCard`, wording review W01–W11, W43).

import { clock, pad, percent } from './format';
import { t } from './i18n';
import type { Account, Decision, Hold, LogLine, Model, Outcome, Sessions, Why } from './model';
import { BAD_USED, claudeAliases, claudeCells, tightest } from './accounts';
import { claudeNames, type Action, type BannerView, type FeedbackView } from './view';

/** The switcher's LaunchAgent runs every 180 seconds. */
const switcherEvery = () => t('trong vòng 3 phút', 'within 3 minutes');

/** `Team B`, or `tài khoản 5` for a slot the snapshot does not list. */
export type NameOf = (slot: number) => string;

export function namesOf(model: Model): NameOf {
  const names = claudeNames(model.snapshot.claude.accounts, null, model.advanced.on, claudeAliases(model));
  return (n) => names.get(n)?.name ?? t('tài khoản ' + n, 'account ' + n);
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
  if (day(d) === day(tomorrow)) return t('ngày mai ', 'tomorrow ') + clock(iso);
  if (day(d) === day(yesterday)) return t('hôm qua ', 'yesterday ') + clock(iso);
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
  return p.until
    ? t('Giữ ' + name + ' đến ' + dayClock(p.until, now), 'Holding ' + name + ' until ' + dayClock(p.until, now))
    : t('Giữ ' + name + ' đến khi bạn bật lại', 'Holding ' + name + ' until you turn it back on');
}

/** `Đang tạm dừng tới 18:30: chuyển tay sang tài khoản 3 lúc 17:30` */
export function pausedLine(p: PauseState, now: Date): string {
  const head = p.until
    ? t('Đang tạm dừng tới ' + dayClock(p.until, now), 'Paused until ' + dayClock(p.until, now))
    : t('Đang tạm dừng đến khi bạn bật lại', 'Paused until you turn it back on');
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

/** `đến 15:20` */
function untilWords(d: Date, now: Date): string {
  const at = dayClock(d.toISOString(), now);
  return t('đến ' + at, 'until ' + at);
}

/** G02: four ways to hold the account switched to, each with its own end. */
export function holdOptions(target: Account, now: Date): ChoiceView<HoldChoice>[] {
  const h30 = addMinutes(now, 30);
  const h60 = addMinutes(now, 60);
  const reset = nextReset(target, now);
  return [
    { v: '30', t: t('30 phút', '30 minutes'), s: untilWords(h30, now), until: h30 },
    { v: '60', t: t('1 giờ', '1 hour'), s: untilWords(h60, now), until: h60 },
    { v: 'reset', t: t('Đến lúc đặt lại', 'Until reset'), s: untilWords(reset, now), until: reset },
    { v: 'none', t: t('Để tự chuyển', 'Let it auto-switch'), s: t('không giữ', 'no hold'), until: null },
  ];
}

/** G03: how long to pause; "Đến 18:00" only before 18:00. */
export function pauseOptions(now: Date): ChoiceView<PauseChoice>[] {
  const h60 = addMinutes(now, 60);
  const six = new Date(now);
  six.setHours(18, 0, 0, 0);
  const out: ChoiceView<PauseChoice>[] = [{ v: '60', t: t('1 giờ', '1 hour'), s: untilWords(h60, now), until: h60 }];
  if (now < six) out.push({ v: '1800', t: t('Đến 18:00', 'Until 18:00'), s: t('hôm nay', 'today'), until: six });
  out.push({ v: 'inf', t: t('Đến khi tôi bật lại', 'Until I turn it back on'), s: t('không có giờ hết hạn', 'no end time'), until: null });
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
  /** The account's name, for the title while the switch runs. */
  name: string;
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
  if (sessions === 'loading') return { k: '', text: t('Đang đếm các phiên Claude đang chạy trên máy này…', 'Counting the Claude sessions running on this Mac…') };
  if (!sessions)
    return {
      k: 'w',
      text: t(
        'Các phiên Claude đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo. Việc này dùng thêm hạn mức.',
        'Running Claude sessions rebuild their cache on their next message, which uses extra quota.',
      ),
    };
  if (sessions.live === 0 && sessions.unreadable === 0)
    return {
      k: '',
      text: t(
        'Không có phiên Claude nào đang chạy trên máy này, nên không phiên nào phải tạo lại bộ nhớ đệm.',
        'No Claude sessions are running on this Mac, so none has to rebuild its cache.',
      ),
    };
  if (sessions.live === 0)
    return {
      k: 'w',
      text: t(
        'Không đọc được ' + sessions.unreadable + ' bản ghi phiên Claude trên máy này, nên có thể có phiên đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo.',
        'Could not read ' + sessions.unreadable + ' Claude session ' + (sessions.unreadable === 1 ? 'record' : 'records') +
          ' on this Mac, so a running session may rebuild its cache on its next message.',
      ),
    };
  const one = sessions.live === 1;
  const n = t(
    (sessions.unreadable ? 'Ít nhất ' : '') + sessions.live + ' phiên Claude',
    (sessions.unreadable ? 'At least ' : '') + sessions.live + (one ? ' Claude session' : ' Claude sessions'),
  );
  return {
    k: 'w',
    text: t(
      '**' + n + '** đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo. Việc này dùng thêm hạn mức.',
      '**' + n + '** ' + (one ? 'is' : 'are') + ' running. Each rebuilds its cache on its next message, which uses extra quota.',
    ),
  };
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
  const parts = cells.map((c) => (c.used == null ? t('chưa có số', 'no numbers yet') : '**' + c.text + '**') + ' ' + c.unit);
  if (parts.length) facts.push({ k: '', text: t(name + ' đã dùng ' + parts.join(', ') + '.', name + ' has used ' + parts.join(', ') + '.') });
  facts.push(sessionsFact(sessions));

  // The rotation and its hold are Nâng cao's: without it the sheet only switches.
  const auto = model.advanced.on;
  const personalOut = auto && target.role === 'personal' && inHours(c.personalHours, now) === false;
  if (personalOut) {
    const m = /^(\d{1,2})-(\d{1,2})$/.exec(c.personalHours ?? '')!;
    const nowAt = clock(now.toISOString());
    const hours = pad(Number(m[1])) + '–' + pad(Number(m[2]));
    facts.push({
      k: 'w',
      text: t(
        'Bây giờ là ' + nowAt + ', ngoài giờ ' + hours + ' của tài khoản Cá nhân. Nếu để tự chuyển, switcher sẽ đổi sang tài khoản khác ' + switcherEvery() + '.',
        'It is ' + nowAt + ', outside the Personal account’s hours (' + hours + '). If you let it auto-switch, the switcher moves to another account ' + switcherEvery() + '.',
      ),
    });
  } else if (auto && (target.membership === 'out' || target.membership === 'ended')) {
    facts.push({
      k: 'w',
      text: t(
        name + ' không còn trong vòng xoay. Nếu để tự chuyển, switcher sẽ đổi sang tài khoản trong vòng.',
        name + ' is no longer in the rotation. If you let it auto-switch, the switcher moves to an account in rotation.',
      ),
    });
  }
  const tight = tightest(cells);
  if (tight && tight.used! >= BAD_USED)
    facts.push({ k: 'w', text: t(name + ' đã dùng ' + tight.used + '% hạn mức ' + tight.unit + '.', name + ' has used ' + tight.used + '% of its ' + tight.unit + ' quota.') });
  const legacy = model.switcher.legacy;
  const blocked = !model.claudeSwitch.ready || !!legacy;
  if (!model.claudeSwitch.ready)
    facts.push({
      k: 'x',
      text: t(
        'Quota Station chưa giữ tài khoản Claude nào trên máy này. Nhập từ claude-swap hoặc thêm tài khoản ở tab Claude.',
        'Quota Station keeps no Claude accounts on this Mac yet. Import from claude-swap or add an account in the Claude tab.',
      ),
    });
  else if (legacy)
    facts.push({
      k: 'x',
      text: t(
        'Bộ tự chuyển cũ `' + legacy + '` còn chạy. Tắt nó trước (hướng dẫn ở tab Claude), để hai bên không chuyển chồng nhau.',
        'The old auto-switcher `' + legacy + '` is still running. Turn it off first (the steps are in the Claude tab) so the two do not switch over each other.',
      ),
    });

  const title = t('Chuyển Claude sang ' + name + '?', 'Switch Claude to ' + name + '?');
  const confirm = t('Chuyển sang ' + name, 'Switch to ' + name);
  if (!auto) return { name, title, facts, options: [], hint: { tone: '', text: '' }, confirm, blocked };
  const options = holdOptions(target, now);
  const cur = options.find((o) => o.v === choice) ?? options[1];
  const paused = pauseOf(model, now);
  let hint: ClaudeSheetView['hint'];
  if (cur.until && paused && outlasts(paused, cur.until)) {
    // The hold is written to the minute, and never shortens a longer pause.
    hint = { tone: '', text: pausedLine(paused, now) + t('. Lần tạm dừng này dài hơn nên được giữ nguyên.', '. This pause is longer, so it stays as it is.') };
  } else if (cur.until) {
    const until = dayClock(cur.until.toISOString(), now);
    hint = { tone: '', text: t('Tạm dừng tự chuyển đến ' + until + ' để giữ tài khoản này.', 'Pauses auto-switch until ' + until + ' to keep this account.') };
    if (model.switcher.guard === false)
      hint = {
        tone: 'warn',
        text: t(
          'Switcher trên máy này chưa đọc cờ tạm dừng, nên vẫn có thể đổi tài khoản.',
          'The switcher on this Mac does not read the pause flag yet, so it may still change accounts.',
        ),
      };
  } else if (paused) {
    hint = { tone: '', text: pausedLine(paused, now) + t('. Tự chuyển vẫn tạm dừng.', '. Auto-switch stays paused.') };
  } else if (personalOut) {
    hint = {
      tone: 'warn',
      text: t(
        'Ngoài giờ dùng tài khoản Cá nhân. Chọn thời lượng giữ để tránh tự chuyển sang tài khoản khác.',
        'Outside the Personal account’s hours. Pick how long to hold it so auto-switch does not move to another account.',
      ),
    };
  } else {
    hint = {
      tone: '',
      text: t(
        'Tự chuyển vẫn bật. Switcher có thể đổi sang tài khoản khác ở lần kiểm sau (' + switcherEvery() + ').',
        'Auto-switch stays on. The switcher may move to another account at its next check (' + switcherEvery() + ').',
      ),
    };
  }
  return { name, title, facts, options, hint, confirm, blocked };
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
  const who = active && active.number > 0 ? namesOf(model)(active.number) : t('tài khoản đang dùng', 'the account in use');
  const options = pauseOptions(now);
  const cur = options.find((o) => o.v === choice) ?? options[0];
  return {
    title: t('Tạm dừng tự chuyển Claude?', 'Pause Claude auto-switch?'),
    lead: t(
      'Giữ ' + who + '. Switcher sẽ không đổi tài khoản Claude trong thời gian này. Hạn mức vẫn cập nhật.',
      'Keep ' + who + '. The switcher will not change the Claude account during this time. Quota keeps updating.',
    ),
    options,
    hint: cur.until
      ? t(
          'Hết thời gian này, tự chuyển sẽ bật lại. Firstmate vẫn thấy hạn mức trong vòng.',
          'When the time is up, auto-switch turns back on. Firstmate still sees the quota in rotation.',
        )
      : t(
          'Tự chuyển chỉ bật lại khi bạn bật công tắc. Firstmate vẫn thấy hạn mức trong vòng.',
          'Auto-switch only comes back when you turn the switch on. Firstmate still sees the quota in rotation.',
        ),
  };
}

// ===== "Vì sao đang ở tài khoản này" (F07) =====

function used(n: number): string {
  return percent(n);
}

export function whyText(why: Why, name: string, nameOf: NameOf): string {
  switch (why.kind) {
    case 'expiring':
      return t(
        'ưu tiên dùng ' + name + ' trước khi tài khoản rời vòng lúc ' + why.at + '.',
        name + ' goes first, before it leaves rotation at ' + why.at + '.',
      );
    case 'early':
      return t(
        'hạn mức 5 giờ của ' + name + ' đặt lại lúc ' + why.at + ' khi mới dùng ' + (100 - why.left) + '%, sớm hơn ' + nameOf(why.before) + '.',
        name + '’s 5-hour quota resets at ' + why.at + ' with only ' + (100 - why.left) + '% used, earlier than ' + nameOf(why.before) + '.',
      );
    case 'room':
      return t(
        name + ' còn hạn mức (đã dùng ' + used(why.used) + (why.ends ? ', hạn mức 5 giờ đặt lại lúc ' + why.ends : '') + ').',
        name + ' has quota left (' + used(why.used) + ' used' + (why.ends ? ', 5-hour quota resets at ' + why.ends : '') + ').',
      );
    case 'personal':
      return t(
        'không tài khoản Team nào còn hạn mức (' + name + ' đã dùng ' + used(why.used) + ').',
        'no Team account has quota left (' + name + ' has used ' + used(why.used) + ').',
      );
    case 'park':
      return t(
        'không tài khoản nào còn hạn mức, ' + name + ' được đặt lại sớm nhất (' + why.at + ').',
        'no account has quota left; ' + name + ' resets first (' + why.at + ').',
      );
    default:
      return t('xem dòng gốc bên dưới.', 'see the raw line below.');
  }
}

/** The switcher's decision in words (W09, W10, W43). */
export function decisionText(d: Decision, raw: string, active: string | null, nameOf: NameOf): string {
  const a = active ?? t('tài khoản đang dùng', 'the account in use');
  switch (d.kind) {
    case 'stay':
      return t('Vẫn dùng ' + a + ': chưa tới ngưỡng tự chuyển.', 'Staying on ' + a + ': the auto-switch threshold is not reached yet.');
    case 'unknown':
      return t('Vẫn dùng ' + nameOf(d.slot) + ': chưa có số mới để quyết định.', 'Staying on ' + nameOf(d.slot) + ': no new numbers to decide on yet.');
    case 'waiting':
      return t(
        'Vẫn dùng ' + a + ': không tài khoản nào còn hạn mức, chờ ' + nameOf(d.slot) + ' được đặt lại lúc ' + d.at + '.',
        'Staying on ' + a + ': no account has quota left; waiting for ' + nameOf(d.slot) + ' to reset at ' + d.at + '.',
      );
    case 'wait':
      return t('Không tài khoản nào còn hạn mức. Switcher giữ tài khoản đang dùng.', 'No account has quota left. The switcher keeps the account in use.');
    case 'switch':
      return t('Lần kiểm tới sẽ chuyển sang ', 'The next check switches to ') + nameOf(d.slot) + ': ' + whyText(d.why, nameOf(d.slot), nameOf);
    case 'paused':
      return t('Switcher đang tạm dừng', 'The switcher is paused') + (d.reason ? ': ' + d.reason : '') + '.';
    case 'blocked':
      return t('Chưa tự chuyển: bộ tự chuyển cũ ' + d.label + ' còn chạy.', 'Not auto-switching: the old auto-switcher ' + d.label + ' is still running.');
    default:
      return t('Switcher báo: ', 'The switcher says: ') + raw;
  }
}

/** The switcher's last log line in words. */
export function logText(line: LogLine, nameOf: NameOf, now: Date): string {
  const at = line.at ? dayClock(line.at, now) + ' · ' : '';
  switch (line.event.kind) {
    case 'switch':
      return at + t('Switcher chuyển sang ' + nameOf(line.event.slot) + '.', 'The switcher switched to ' + nameOf(line.event.slot) + '.');
    case 'switchFailed':
      return at + t('Switcher không chuyển được sang ' + nameOf(line.event.slot) + '.', 'The switcher could not switch to ' + nameOf(line.event.slot) + '.');
    case 'primed':
      return at + t('Switcher bắt đầu tính hạn mức 5 giờ của ' + nameOf(line.event.slot) + '.', 'The switcher started ' + nameOf(line.event.slot) + '’s 5-hour window.');
    case 'pauseEnded':
      return at + t('Hết tạm dừng, tự chuyển bật lại.', 'The pause ended; auto-switch is back on.');
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
  if (!sw.installed)
    why = t(
      'Quota Station chưa giữ tài khoản Claude nào trên máy này: chưa có tự chuyển Claude.',
      'Quota Station keeps no Claude accounts on this Mac yet, so there is no Claude auto-switch.',
    );
  else if (sw.legacy)
    why = t(
      'Bộ tự chuyển cũ ' + sw.legacy + ' còn chạy, nên Quota Station chưa tự chuyển.',
      'The old auto-switcher ' + sw.legacy + ' is still running, so Quota Station is not auto-switching.',
    );
  else if (sw.service === false) why = t('Tự chuyển của Quota Station đang tắt: bật nó ở tab Claude.', 'Quota Station’s auto-switch is off: turn it on in the Claude tab.');
  else if (sw.dryRun) why = decisionText(sw.dryRun.decision, sw.dryRun.raw, active, nameOf);
  else if (sw.dryRunError)
    why = t(
      'Chưa đọc được quyết định của switcher (' + dryRunReason(sw.dryRunError) + ').',
      'Could not read the switcher’s decision (' + dryRunReason(sw.dryRunError) + ').',
    );
  else why = t('Đang hỏi switcher…', 'Asking the switcher…');
  const off = !sw.installed || !!sw.legacy || sw.service === false;
  const line = paused ? pausedLine(paused, now) : at && sw.dryRun && !off ? at + ' · ' + why : why;
  const usual = !paused && !off && (sw.dryRun ? sw.dryRun.decision.kind === 'stay' : !sw.dryRunError);
  return {
    paused,
    title: paused ? t('Tự chuyển đang tạm dừng', 'Auto-switch is paused') : t('Tự chuyển Claude', 'Claude auto-switch'),
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
      label: paused ? t('Tự chuyển Claude, đang tạm dừng', 'Claude auto-switch, paused') : t('Tự chuyển Claude', 'Claude auto-switch'),
    },
  };
}

function dryRunReason(kind: string): string {
  switch (kind) {
    case 'timeout':
      return t('quá thời gian chờ', 'timed out');
    case 'noAccounts':
      return t('chưa có tài khoản', 'no accounts yet');
    default:
      return t('switcher chạy lỗi', 'the switcher failed');
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
      title: t('Switcher chưa đọc cờ tạm dừng', 'The switcher does not read the pause flag'),
      body: t(
        'Bộ tự chuyển trên máy này không dừng theo cờ tạm dừng, nên vẫn tự chuyển. Cho phép tự chuyển lại, hoặc tắt bộ tự chuyển đó.',
        'The auto-switcher on this Mac does not stop for the pause flag, so it keeps switching. Allow auto-switch again, or turn that auto-switcher off.',
      ),
      actions: [{ label: t('Cho phép tự chuyển lại', 'Allow auto-switch again'), act: 'resume' }],
    };
  if (sw.legacy && sw.installed)
    return {
      tone: 'warn',
      title: t('Bộ tự chuyển cũ còn chạy', 'The old auto-switcher is still running'),
      body: t(
        sw.legacy +
          ' vẫn được nạp. Quota Station không chuyển tài khoản khi nó còn chạy, để hai bên không chuyển chồng nhau. Tắt nó theo hướng dẫn chuyển.',
        sw.legacy +
          ' is still loaded. Quota Station does not switch accounts while it runs, so the two do not switch over each other. Turn it off by following the guide.',
      ),
      actions: [{ label: t('Hướng dẫn tắt', 'How to turn it off'), act: 'guide:auto' }],
    };
  return pausedBanner(model, now);
}

export function pausedBanner(model: Model, now: Date): BannerView | null {
  const p = pauseOf(model, now);
  if (!p) return null;
  const active = model.snapshot.claude.accounts.find((a) => a.active && a.number > 0);
  const who = active ? namesOf(model)(active.number) : t('tài khoản đang dùng', 'the account in use');
  return {
    tone: 'info',
    title: p.until
      ? t('Đang giữ ' + who + ' đến ' + dayClock(p.until, now), 'Holding ' + who + ' until ' + dayClock(p.until, now))
      : t('Đang giữ ' + who + ' đến khi bạn bật lại', 'Holding ' + who + ' until you turn it back on'),
    body: p.until
      ? t('Tự chuyển Claude đang tạm dừng. Hết thời gian này, tự chuyển sẽ bật lại.', 'Claude auto-switch is paused. It turns back on when this time is up.')
      : t('Tự chuyển Claude đang tạm dừng cho tới khi bạn bật lại.', 'Claude auto-switch is paused until you turn it back on.'),
    actions: [
      { label: t('Cho phép tự chuyển lại', 'Allow auto-switch again'), act: 'resume' },
      ...(p.until ? [{ label: t('Thêm 1 giờ', 'One more hour'), act: 'addHour' as Action }] : []),
    ],
  };
}

/** The summary's first item while paused: `Giữ Team B đến 18:30`. */
export function pausedSummary(model: Model, now: Date): string | null {
  const p = pauseOf(model, now);
  if (!p) return null;
  const active = model.snapshot.claude.accounts.find((a) => a.active && a.number > 0);
  return holdWords(active ? namesOf(model)(active.number) : t('tài khoản đang dùng', 'the account in use'), p, now);
}

// ===== Feedback after an action =====

function holdTail(hold: Hold | null, now: Date): string {
  if (!hold) return '';
  return hold.failed
    ? t(
        ' Chưa ghi được cờ tạm dừng, nên switcher vẫn có thể đổi tài khoản ở lần kiểm sau.',
        ' Could not write the pause flag, so the switcher may still change accounts at its next check.',
      )
    : t(' Tự chuyển Claude tạm dừng tới ' + dayClock(hold.until, now) + '.', ' Claude auto-switch is paused until ' + dayClock(hold.until, now) + '.');
}

const close = () => ({ label: t('Đóng', 'Close'), act: 'closeFeedback' as Action });

export function switchFeedback(outcome: Outcome, model: Model, now: Date): FeedbackView {
  const nameOf = namesOf(model);
  const n = nameOf(outcome.slot);
  const retry = { label: t('Thử lại', 'Try again'), act: ('switch:' + outcome.slot) as Action };
  switch (outcome.kind) {
    case 'switched': {
      const h = outcome.hold;
      if (h && !h.failed)
        return {
          tone: '',
          title: t(
            'Đã chuyển sang ' + n + '. Giữ tài khoản này đến ' + dayClock(h.until, now) + '.',
            'Switched to ' + n + '. Holding it until ' + dayClock(h.until, now) + '.',
          ),
          body: t(
            'Đọc lại sau 5 giây: ' + n + ' đang dùng ✓. Tự chuyển Claude tạm dừng tới lúc đó.',
            'Read again after 5 seconds: ' + n + ' is in use ✓. Claude auto-switch is paused until then.',
          ),
          actions: [{ label: t('Cho phép tự chuyển lại', 'Allow auto-switch again'), act: 'resume' }, close()],
        };
      const paused = !h ? pauseOf(model, now) : null;
      return {
        tone: h?.failed ? 'warn' : '',
        title: t('Đã chuyển sang ' + n + '.', 'Switched to ' + n + '.'),
        body:
          t('Đọc lại sau 5 giây: ' + n + ' đang dùng ✓.', 'Read again after 5 seconds: ' + n + ' is in use ✓.') +
          (h
            ? holdTail(h, now)
            : paused
              ? ' ' + pausedLine(paused, now) + '.'
              : t(
                  ' Tự chuyển vẫn bật: switcher kiểm lại ' + switcherEvery() + ' và có thể đổi tài khoản.',
                  ' Auto-switch stays on: the switcher checks again ' + switcherEvery() + ' and may change accounts.',
                )),
        actions: [close()],
      };
    }
    case 'raced': {
      const a = nameOf(outcome.active);
      const h = outcome.hold;
      return {
        tone: 'warn',
        title: outcome.bySwitcherAt
          ? t('Switcher vừa chuyển sang ' + a + ' lúc ' + clock(outcome.bySwitcherAt), 'The switcher just switched to ' + a + ' at ' + clock(outcome.bySwitcherAt))
          : t('Một lần chuyển khác vừa chuyển sang ' + a, 'Another switch just moved to ' + a),
        body: t('Hai lần ghi gần nhau, lần sau thắng. Đang dùng ' + a + '.', 'Two writes came close together and the later one won. ' + a + ' is in use.') + holdTail(h, now),
        actions: [
          { label: t('Chuyển lại sang ' + n, 'Switch back to ' + n), act: ('switch:' + outcome.slot) as Action },
          ...(h && !h.failed ? [{ label: t('Cho phép tự chuyển lại', 'Allow auto-switch again'), act: 'resume' as Action }] : []),
          { label: t('Để vậy', 'Leave it'), act: 'closeFeedback' },
        ],
      };
    }
    case 'unconfirmed':
      return {
        tone: 'warn',
        title: t('Đã gửi lệnh chuyển sang ' + n, 'Sent the switch to ' + n),
        body:
          t(
            'Đã ghi đăng nhập mới, nhưng Claude Code giờ đang ở một tài khoản Quota Station không giữ.',
            'The new sign-in is written, but Claude Code is now on an account Quota Station does not keep.',
          ) + holdTail(outcome.hold, now),
        actions: [{ label: t('Làm mới', 'Refresh'), act: 'refresh' }, close()],
      };
    case 'failed': {
      const body = failedBody(outcome, nameOf);
      const stays = outcome.error === 'legacy' || outcome.error === 'unknownSlot' || outcome.error === 'unmanaged';
      return {
        tone: 'bad',
        title: t('Chưa chuyển được sang ' + n, 'Could not switch to ' + n),
        body,
        actions: stays ? [close()] : [retry, close()],
      };
    }
  }
}

/** Why a switch did not land, in words. */
function failedBody(outcome: Extract<Outcome, { kind: 'failed' }>, nameOf: NameOf): string {
  const detail = outcome.detail ? ' (' + outcome.detail + ')' : '';
  switch (outcome.error) {
    case 'legacy':
      return t(
        'Bộ tự chuyển cũ ' + outcome.detail + ' còn chạy. Tắt nó trước theo hướng dẫn ở tab Claude. Chưa đổi gì.',
        'The old auto-switcher ' + outcome.detail + ' is still running. Turn it off first with the steps in the Claude tab. Nothing changed.',
      );
    case 'busy':
      return t(
        'Claude Code đang giữ khoá đăng nhập (đang làm mới hoặc ghi đăng nhập). Thử lại sau vài giây. Chưa đổi gì.',
        'Claude Code holds the sign-in lock (refreshing or writing its sign-in). Try again in a few seconds. Nothing changed.',
      );
    case 'noCredential':
      return t(
        'Quota Station không có đăng nhập dùng được của tài khoản này. Đăng nhập lại nó ở tab Claude. Chưa đổi gì.',
        'Quota Station has no usable sign-in for this account. Sign it in again in the Claude tab. Nothing changed.',
      );
    case 'unmanaged':
      return t(
        'Claude Code đang đăng nhập một tài khoản Quota Station chưa giữ. Thêm tài khoản đang đăng nhập ở tab Claude trước, để không mất đăng nhập đó. Chưa đổi gì.',
        'Claude Code is signed in to an account Quota Station does not keep yet. Add the signed-in account in the Claude tab first so that sign-in is not lost. Nothing changed.',
      );
    case 'unconfirmed':
      return outcome.active != null
        ? t(
            'Ghi xong nhưng đọc lại không khớp, và không đặt lại được như cũ. Đang dùng ' + nameOf(outcome.active) + '.',
            'The write finished but reading it back did not match, and it could not be put back. ' + nameOf(outcome.active) + ' is in use.',
          )
        : t(
            'Ghi xong nhưng đọc lại không khớp, nên app đã đặt lại như cũ. Chưa đổi gì.',
            'The write finished but reading it back did not match, so the app put it back. Nothing changed.',
          );
    case 'unknownSlot':
      return t('Quota Station không giữ tài khoản này. Chưa đổi gì.', 'Quota Station does not keep this account. Nothing changed.');
    case 'keychain':
      return t(
        'Keychain không cho đọc hoặc ghi đăng nhập' + detail + '. Chưa đổi gì.',
        'The Keychain did not allow reading or writing the sign-in' + detail + '. Nothing changed.',
      );
    default:
      return t('Chuyển không được' + detail + '. Chưa đổi gì.', 'The switch failed' + detail + '. Nothing changed.');
  }
}

/** The switch call itself was refused (`busy`, `unknownSlot`) or broke. */
export function switchErrorFeedback(slot: number, error: unknown, model: Model): FeedbackView {
  const n = namesOf(model)(slot);
  const why = String(error);
  return {
    tone: 'bad',
    title: t('Chưa chuyển được sang ' + n, 'Could not switch to ' + n),
    body:
      why === 'busy'
        ? t('Một lần chuyển khác đang chạy. Chờ nó xong rồi thử lại.', 'Another switch is running. Wait for it to finish, then try again.')
        : why === 'unknownSlot'
          ? t('Quota Station không còn giữ tài khoản này. Bấm Làm mới rồi thử lại.', 'Quota Station no longer keeps this account. Press Refresh, then try again.')
          : t('Không gọi được lệnh chuyển. Chưa đổi gì.', 'Could not run the switch. Nothing changed.'),
    actions: [close()],
  };
}

export function pausedFeedback(model: Model, now: Date): FeedbackView {
  const p = pauseOf(model, now);
  const active = model.snapshot.claude.accounts.find((a) => a.active && a.number > 0);
  const who = active ? namesOf(model)(active.number) : t('tài khoản đang dùng', 'the account in use');
  return {
    tone: 'info',
    title: t('Đã tạm dừng tự chuyển Claude', 'Claude auto-switch is paused'),
    body: p
      ? p.until
        ? t(
            'Giữ ' + who + ' đến ' + dayClock(p.until, now) + '. Hết thời gian này, tự chuyển sẽ bật lại.',
            'Holding ' + who + ' until ' + dayClock(p.until, now) + '. When the time is up, auto-switch turns back on.',
          )
        : t(
            'Giữ ' + who + ' đến khi bạn bật lại. Bật lại bằng công tắc Tự chuyển Claude.',
            'Holding ' + who + ' until you turn it back on. Use the Claude auto-switch toggle to turn it back on.',
          )
      : t('Tự chuyển Claude đang tạm dừng.', 'Claude auto-switch is paused.'),
    actions: [{ label: t('Bật lại tự chuyển', 'Turn auto-switch back on'), act: 'resume' }, close()],
  };
}

export function resumedFeedback(): FeedbackView {
  return {
    tone: '',
    title: t('Tự chuyển Claude đã bật lại', 'Claude auto-switch is back on'),
    body: t('Switcher kiểm lại ' + switcherEvery() + '.', 'The switcher checks again ' + switcherEvery() + '.'),
  };
}

export function extendedFeedback(model: Model, now: Date): FeedbackView {
  const s = pausedSummary(model, now);
  return {
    tone: 'info',
    title: s ?? t('Đã thêm 1 giờ', 'Added one hour'),
    body: t('Đã thêm 1 giờ. Tự chuyển Claude vẫn tạm dừng.', 'Added one hour. Claude auto-switch stays paused.'),
  };
}

export function pauseFailedFeedback(resume: boolean): FeedbackView {
  return resume
    ? {
        tone: 'bad',
        title: t('Chưa bật lại được tự chuyển', 'Could not turn auto-switch back on'),
        body: t('Không xoá được cờ tạm dừng của switcher trên máy này. Thử lại.', 'Could not remove the switcher’s pause flag on this Mac. Try again.'),
      }
    : {
        tone: 'bad',
        title: t('Chưa ghi được cờ tạm dừng', 'Could not write the pause flag'),
        body: t('Không ghi được cờ tạm dừng của switcher trên máy này. Tự chuyển vẫn bật.', 'Could not write the switcher’s pause flag on this Mac. Auto-switch stays on.'),
      };
}

// ===== Guides =====

export interface GuideView {
  title: string;
  lead: string;
  steps: string[];
}

/**
 * `auto`: stop the older switchers (step 1 of the cutover); `switcher`: the whole cutover from
 * claude-swap, as the README's checklist has it. The step for Firstmate comes only in a build
 * with the author's tools (`authorTools`).
 */
export function guide(id: 'auto' | 'switcher', authorTools = true): GuideView {
  return id === 'auto'
    ? {
        title: t('Tắt bộ tự chuyển cũ', 'Turn off the old auto-switcher'),
        lead: t(
          'Quota Station không chuyển tài khoản khi com.cswap.prefer-team hoặc com.cswap.auto còn được nạp, để hai bên không chuyển chồng nhau. Quota Station không tự chạy các lệnh này; claude-swap và dữ liệu của nó vẫn giữ nguyên.',
          'Quota Station does not switch accounts while com.cswap.prefer-team or com.cswap.auto is loaded, so the two do not switch over each other. Quota Station does not run these commands itself; claude-swap and its data stay as they are.',
        ),
        steps: [
          t('Mở Terminal trên máy này.', 'Open Terminal on this Mac.'),
          t(
            'Giữ một bản sao plist: `cp ~/Library/LaunchAgents/com.cswap.prefer-team.plist ~/Library/LaunchAgents/com.cswap.prefer-team.plist.bak`',
            'Keep a copy of the plist: `cp ~/Library/LaunchAgents/com.cswap.prefer-team.plist ~/Library/LaunchAgents/com.cswap.prefer-team.plist.bak`',
          ),
          t('Chạy `launchctl bootout gui/$(id -u)/com.cswap.prefer-team`', 'Run `launchctl bootout gui/$(id -u)/com.cswap.prefer-team`'),
          t(
            'Nếu com.cswap.auto cũng đang nạp: `launchctl bootout gui/$(id -u)/com.cswap.auto`',
            'If com.cswap.auto is loaded too: `launchctl bootout gui/$(id -u)/com.cswap.auto`',
          ),
          t(
            'Quay lại tab Claude và bấm Làm mới. Cảnh báo bộ tự chuyển cũ phải biến mất.',
            'Go back to the Claude tab and press Refresh. The old auto-switcher warning should go away.',
          ),
        ],
      }
    : {
        title: t('Chuyển từ claude-swap sang Quota Station', 'Move from claude-swap to Quota Station'),
        lead: t(
          'Làm lần lượt trên từng Mac. claude-swap và dữ liệu của nó được giữ nguyên, để quay lại được khi cần.',
          'Do this on one Mac at a time. claude-swap and its data stay as they are, so you can go back if needed.',
        ),
        steps: [
          t(
            'Tắt bộ tự chuyển cũ: giữ bản sao plist rồi `launchctl bootout gui/$(id -u)/com.cswap.prefer-team`.',
            'Turn off the old auto-switcher: keep a copy of the plist, then `launchctl bootout gui/$(id -u)/com.cswap.prefer-team`.',
          ),
          t(
            'Ở tab Claude, bấm **Nhập từ claude-swap**. Chạy lại lần nữa cũng không sao: chỉ đăng nhập mới hơn được lấy thêm.',
            'In the Claude tab, press **Import from claude-swap**. Running it again is fine: only newer sign-ins are added.',
          ),
          t(
            'Kiểm tra danh sách tài khoản, rồi bật **Tự chuyển bằng Quota Station**.',
            'Check the account list, then turn on **Auto-switch with Quota Station**.',
          ),
          ...(authorTools
            ? [
                t(
                  'Firstmate đọc số từ `quota-station claude status --json` thay cho `cswap list --json`, và chạy `quota-station claude run N --` thay cho `cswap run N --`.',
                  'Firstmate reads its numbers from `quota-station claude status --json` instead of `cswap list --json`, and runs `quota-station claude run N --` instead of `cswap run N --`.',
                ),
              ]
            : []),
          t('Chỉ gỡ claude-swap khi đã hỏi và mọi Mac đã chạy ổn.', 'Remove claude-swap only after asking, and once every Mac runs smoothly.'),
        ],
      };
}
