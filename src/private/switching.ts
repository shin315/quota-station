// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The rotation's parts of the switching words (M3): the pause sheet, the
// hold on the Claude switch sheet, the switcher's "why" line (F07) and the
// feedback after a pause. Pure, like `lib/switching.ts`, whose shared parts
// these build on.

import { clock, pad, percent } from '@/lib/format';
import { mac, onWindows, t } from '@/lib/i18n';
import type { Account, Decision, Hold, LogLine, Model, Why } from '@/lib/model';
import { close, dayClock, namesOf, type ChoiceView, type ClaudeSheetView, type FactView, type NameOf } from '@/lib/switching';
import type { Action, BannerView, FeedbackView } from '@/lib/view';

/** The switcher's LaunchAgent runs every 180 seconds. */
export const switcherEvery = () => t('trong vòng 3 phút', 'within 3 minutes');

function addMinutes(d: Date, minutes: number): Date {
  return new Date(d.getTime() + minutes * 60000);
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

function inHours(range: string | null, now: Date): boolean | null {
  const m = /^(\d{1,2})-(\d{1,2})$/.exec(range ?? '');
  if (!m) return null;
  const h = now.getHours();
  return h >= Number(m[1]) && h <= Number(m[2]);
}

/** The rotation's facts on the Claude switch sheet; `personalOut` when the personal account is outside its hours. */
export function autoFacts(model: Model, target: Account, name: string, now: Date): { facts: FactView[]; personalOut: boolean } {
  const c = model.snapshot.claude;
  const facts: FactView[] = [];
  const personalOut = target.role === 'personal' && inHours(c.personalHours, now) === false;
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
  } else if ((target.membership === 'out' || target.membership === 'ended')) {
    facts.push({
      k: 'w',
      text: t(
        name + ' không còn trong vòng xoay. Nếu để tự chuyển, switcher sẽ đổi sang tài khoản trong vòng.',
        name + ' is no longer in the rotation. If you let it auto-switch, the switcher moves to an account in rotation.',
      ),
    });
  }
  return { facts, personalOut };
}

/** The hold's choices and hint on the Claude switch sheet. */
export function holdPart(
  model: Model,
  target: Account,
  choice: HoldChoice,
  personalOut: boolean,
  now: Date,
): Pick<ClaudeSheetView, 'options' | 'hint'> {
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
        text: mac(
          t(
            'Switcher trên máy này chưa đọc cờ tạm dừng, nên vẫn có thể đổi tài khoản.',
            'The switcher on this Mac does not read the pause flag yet, so it may still change accounts.',
          ),
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
        'Tự động chuyển vẫn bật. Bộ tự chuyển có thể đổi sang tài khoản khác ở lần kiểm tra sau (' + switcherEvery() + ').',
        'Auto-switch stays on. Auto-switch may move to another account on the next check (' + switcherEvery() + ').',
      ),
    };
  }
  return { options, hint };
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
      return t('Không tài khoản nào còn hạn mức. Tự động chuyển giữ tài khoản đang dùng.', 'No account has quota left. Auto-switch keeps the account in use.');
    case 'switch':
      return t('Lần kiểm tra tới sẽ chuyển sang ', 'The next check switches to ') + nameOf(d.slot) + ': ' + whyText(d.why, nameOf(d.slot), nameOf);
    case 'paused':
      return t('Tự động chuyển đang tạm dừng', 'Auto-switch is paused') + (d.reason ? ': ' + d.reason : '') + '.';
    case 'blocked':
      return t('Chưa tự chuyển: bộ tự chuyển cũ ' + d.label + ' còn chạy.', 'Not auto-switching: the old auto-switcher ' + d.label + ' is still running.');
    default:
      return t('Tự động chuyển báo: ', 'Auto-switch says: ') + raw;
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
    why = mac(
      t(
        'Quota Station chưa giữ tài khoản Claude nào trên máy này: chưa có tự chuyển Claude.',
        'Quota Station keeps no Claude accounts on this Mac yet, so there is no Claude auto-switch.',
      ),
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
      'Chưa đọc được quyết định tự động chuyển (' + dryRunReason(sw.dryRunError) + ').',
      'Could not read the auto-switch decision (' + dryRunReason(sw.dryRunError) + ').',
    );
  else why = t('Đang kiểm tra quyết định tự động chuyển…', 'Checking auto-switch decision…');
  const off = !sw.installed || !!sw.legacy || sw.service === false;
  const line = paused ? pausedLine(paused, now) : at && sw.dryRun && !off ? at + ' · ' + why : why;
  const usual = !paused && !off && (sw.dryRun ? sw.dryRun.decision.kind === 'stay' : !sw.dryRunError);
  return {
    paused,
    title: paused ? t('Tự động chuyển đang tạm dừng', 'Auto-switch is paused') : t('Tự động chuyển Claude', 'Claude auto-switch'),
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
      body: mac(
        t(
          'Bộ tự chuyển trên máy này không dừng theo cờ tạm dừng, nên vẫn tự chuyển. Cho phép tự chuyển lại, hoặc tắt bộ tự chuyển đó.',
          'The auto-switcher on this Mac does not stop for the pause flag, so it keeps switching. Allow auto-switch again, or turn that auto-switcher off.',
        ),
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

export function holdTail(hold: Hold | null, now: Date): string {
  if (!hold) return '';
  return hold.failed
    ? t(
        ' Chưa ghi được cờ tạm dừng, nên tự động chuyển vẫn có thể đổi tài khoản ở lần kiểm tra sau.',
        ' Could not write the pause flag, so auto-switch may still change accounts on the next check.',
      )
    : t(' Tự chuyển Claude tạm dừng tới ' + dayClock(hold.until, now) + '.', ' Claude auto-switch is paused until ' + dayClock(hold.until, now) + '.');
}

/** The feedback after a switch that also wrote a hold. */
export function heldFeedback(hold: Hold | null, n: string, now: Date): FeedbackView | null {
  if (!hold || hold.failed) return null;
  return {
    tone: '',
    title: t(
      'Đã chuyển sang ' + n + '. Giữ tài khoản này đến ' + dayClock(hold.until, now) + '.',
      'Switched to ' + n + '. Holding it until ' + dayClock(hold.until, now) + '.',
    ),
    body: t(
      'Đọc lại sau 5 giây: ' + n + ' đang dùng ✓. Tự chuyển Claude tạm dừng tới lúc đó.',
      'Read again after 5 seconds: ' + n + ' is in use ✓. Claude auto-switch is paused until then.',
    ),
    actions: [{ label: t('Cho phép tự chuyển lại', 'Allow auto-switch again'), act: 'resume' }, { label: t('Đóng', 'Close'), act: 'closeFeedback' }],
  };
}

/** What a switch without a hold says of the rotation: its tone and the words after the switch's own. */
export function switchedTail(hold: Hold | null, model: Model, now: Date): [FeedbackView['tone'], string] {
  const paused = !hold ? pauseOf(model, now) : null;
  return [
    hold?.failed ? 'warn' : '',
    hold
      ? holdTail(hold, now)
      : paused
        ? ' ' + pausedLine(paused, now) + '.'
        : t(
            ' Tự động chuyển vẫn bật: bộ tự chuyển kiểm tra lại ' + switcherEvery() + ' và có thể đổi tài khoản.',
            ' Auto-switch stays on: auto-switch checks again ' + switcherEvery() + ' and may change accounts.',
          ),
  ];
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
    body: t('Tự động chuyển kiểm tra lại ' + switcherEvery() + '.', 'Auto-switch checks again ' + switcherEvery() + '.'),
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
        body: mac(t('Không xoá được cờ tạm dừng của switcher trên máy này. Thử lại.', 'Could not remove the switcher’s pause flag on this Mac. Try again.')),
      }
    : {
        tone: 'bad',
        title: t('Chưa ghi được cờ tạm dừng', 'Could not write the pause flag'),
        body: mac(t('Không ghi được cờ tạm dừng của switcher trên máy này. Tự chuyển vẫn bật.', 'Could not write the switcher’s pause flag on this Mac. Auto-switch stays on.')),
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
        lead: mac(
          t(
            'Làm lần lượt trên từng Mac. claude-swap và dữ liệu của nó được giữ nguyên, để quay lại được khi cần.',
            'Do this on one Mac at a time. claude-swap and its data stay as they are, so you can go back if needed.',
          ),
        ),
        steps: [
          // claude-swap's auto-switcher is a LaunchAgent: Windows has none to turn off.
          ...(onWindows()
            ? []
            : [
                t(
                  'Tắt bộ tự chuyển cũ: giữ bản sao plist rồi `launchctl bootout gui/$(id -u)/com.cswap.prefer-team`.',
                  'Turn off the old auto-switcher: keep a copy of the plist, then `launchctl bootout gui/$(id -u)/com.cswap.prefer-team`.',
                ),
              ]),
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
          mac(t('Chỉ gỡ claude-swap khi đã hỏi và mọi Mac đã chạy ổn.', 'Remove claude-swap only after asking, and once every Mac runs smoothly.')),
        ],
      };
}
