// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the bell's panel (Thông báo) shows (F09). The cards carry the
// words and buttons the Rust side wrote for the macOS card, so both say the
// same; this file adds when, what was pressed and how it went. Pure, like
// `view.ts`. Copy follows the prototype's `tabAlerts` and `alertCard`.
// @private:begin
// The Firstmate tab too (F10), after the prototype's `tabFirstmate`.
// @private:end

import { clock } from './format';
import { appName, osName, systemSettings, t } from './i18n';
import type { AlertRecord, AlertTone, Model, NotifyPermission, SwitchKind } from './model';
import { dayLabel } from './history';
import type { BannerView, ChipView, FeedbackView } from './view';
// @private:begin
import { age, percent, secondsSince } from './format';
import type { AuthorModel } from '@/private/author';
import type { FirstmateView } from './model';
import { dayClock, namesOf } from './switching';
import type { Tone } from './view';
// @private:end

/**
 * The kinds, in the prototype's order and words. `agy`: this build
 * switches Antigravity, so the sign-in alert covers its accounts too.
 */
export function alertKindList(agy = true): { kind: SwitchKind; title: string; text: string }[] {
  let pool = {
    title: t('Hạn mức Claude sắp cạn / có lại', 'Claude quota running out / back'),
    text: t('Khi hạn mức đã dùng từ 90% trở lên và khi có lại.', 'When the quota is 90% used or more, and when it is back.'),
  };
  // @private:begin
  pool = {
    title: t('Hạn mức Claude trong vòng sắp cạn / có lại', 'Claude quota in rotation running out / back'),
    text: t(
      'Khi hạn mức trong vòng đã dùng từ 90% trở lên và khi có lại. Cùng luật với firstmate.',
      'When the quota in rotation is 90% used or more, and when it is back. Same rule as firstmate.',
    ),
  };
  // @private:end
  return [
    { kind: 'pool', ...pool },
    // @private:begin
    {
      kind: 'leaving',
      title: t('Tài khoản sắp rời vòng', 'An account is about to leave rotation'),
      text: t('30 phút trước giờ rời vòng trong file luật.', '30 minutes before its leave time in the rules file.'),
    },
    // @private:end
    {
      kind: 'login',
      title: t('Tài khoản cần đăng nhập lại', 'An account needs to sign in again'),
      text:
        t('3 ngày trước khi hết hạn, hoặc khi claude-swap báo lỗi đăng nhập.', '3 days before the sign-in expires, or when claude-swap reports a sign-in error.') +
        (agy ? t(' Cả tài khoản Antigravity cần đăng nhập lại hay xác minh.', ' Also Antigravity accounts that need to sign in again or verify.') : ''),
    },
    {
      kind: 'overwrite',
      title: t('Antigravity đổi về tài khoản cũ', 'Antigravity went back to the old account'),
      text: t('Trong 6 giờ sau khi chuyển. Ngừng đề nghị chuyển lại sau 2 lần.', 'Within 6 hours of a switch. Stops offering to switch back after 2 times.'),
    },
    // @private:begin
    {
      kind: 'firstmate',
      title: t('Firstmate đang chờ hạn mức Claude', 'Firstmate is waiting for Claude quota'),
      text: t('Khi firstmate báo hạn mức trong vòng đã cạn. Chỉ trên máy có firstmate.', 'When firstmate reports the quota in rotation is used up. Only on a Mac with firstmate.'),
    },
    // @private:end
  ];
}

/**
 * The kinds this Mac can turn on, in that order: the overwritten
 * Antigravity switch only in a build that switches Antigravity. Alert 1
 * says its thresholds.
 */
// @private:begin
/** The rotation's (2) and Firstmate's (5) only with Cài đặt › Nâng cao (Firstmate's behind its own switch too). */
// @private:end
export function alertKinds(model: Model): { kind: SwitchKind; title: string; text: string }[] {
  const agy = model.capabilities.agyAccounts;
  const p = model.alerts.poolAlert;
  // @private:begin
  const adv = model.advanced.on;
  const fm = adv && model.advanced.firstmate;
  // @private:end
  return alertKindList(agy)
    .filter((k) => k.kind !== 'overwrite' || agy)
    // @private:begin
    .filter((k) => (k.kind !== 'leaving' || adv) && (k.kind !== 'firstmate' || fm))
    // @private:end
    .map((k) => {
      if (k.kind !== 'pool') return k;
      let title = t('Hạn mức Claude sắp cạn / có lại', 'Claude quota running out / back');
      let where = ['', ''];
      let rule = ['', ''];
      // @private:begin
      if (adv) [title, where] = [k.title, [' trong vòng', ' in rotation']];
      if (fm) rule = [' Cùng luật với firstmate.', ' Same rule as firstmate.'];
      // @private:end
      return {
        ...k,
        title,
        text: t(
          'Khi tài khoản đang dùng còn ' + p.dryAtMost + '% trở xuống và không tài khoản nào' + where[0] + ' còn ' + p.roomAtLeast + '%, và khi có lại.' + rule[0],
          'When the account in use has ' + p.dryAtMost + '% or less left and no account' + where[1] + ' still has ' + p.roomAtLeast + '%, and when it is back.' + rule[1],
        ),
      };
    });
}

const FB_TONE: Record<AlertTone, FeedbackView['tone']> = { ok: '', warn: 'warn', bad: 'bad', info: 'info' };

export interface AlertCardView {
  id: string;
  /** The `.ai` square: its color and its mark. */
  icon: { k: '' | 'bad' | 'ok' | 'info'; mark: string };
  title: string;
  body: string;
  /** Empty once a button was pressed. */
  actions: { id: string; label: string }[];
  /** `Bạn đã chọn “Ổn”.` */
  chose: string | null;
  /** The day it came, as Lịch sử names days: `Hôm nay`, `Hôm qua`, `Thứ Hai 12/01`. */
  day: string;
  /** How it went; `working` while a switch back runs. */
  result: FeedbackView | { working: string } | null;
  meta: string;
}

function iso(unix: number): string {
  return new Date(unix * 1000).toISOString();
}

function working(action: string): string {
  return action.startsWith('agy:quitFirst:')
    ? t('Đang thoát ' + appName('Antigravity') + ' và chuyển tài khoản…', 'Quitting ' + appName('Antigravity') + ' and switching accounts…')
    : t('Đang chuyển tài khoản Antigravity…', 'Switching the Antigravity account…');
}

export function alertCard(r: AlertRecord, now: Date): AlertCardView {
  const a = r.alert;
  const answer = r.answer;
  const icon = a.tone === 'warn' ? '' : a.tone;
  // The day is the group's head; the line keeps the time.
  const meta = [clock(iso(a.at))];
  if (a.test) meta.push(t('thông báo thử', 'test notification'));
  meta.push(r.notified ? t('đã gửi thông báo ' + osName(), 'sent as a ' + osName() + ' notification') : t('chỉ có trong app', 'in the app only'));
  return {
    id: r.id,
    icon: { k: icon, mark: a.tone === 'ok' ? '✓' : '!' },
    title: a.title,
    body: a.body,
    actions: answer ? [] : a.actions,
    chose: answer ? t('Bạn đã chọn “' + answer.label + '”.', 'You chose “' + answer.label + '”.') : null,
    day: dayLabel(iso(a.at), now),
    result: !answer
      ? null
      : answer.result
        ? { tone: FB_TONE[answer.result.tone], title: answer.result.title, body: answer.result.body }
        : { working: working(answer.action) },
    meta: meta.join(' · '),
  };
}

/** Above the cards when macOS will not show them. */
export function permissionBanner(p: NotifyPermission): BannerView | null {
  if (p === 'denied')
    return {
      tone: 'warn',
      title: t('Thông báo của Quota Station đang tắt', 'Quota Station notifications are off'),
      body: t(osName() + ' không hiện thông báo nào. Các thông báo vẫn nằm ở đây, kèm các nút bấm tương ứng.', osName() + ' shows no notifications. Notifications still show up here, with the same buttons.'),
      actions: [{ label: t('Mở ', 'Open ') + systemSettings(), act: 'notify:settings' }],
    };
  if (p === 'unbundled')
    return {
      tone: 'info',
      title: t('Bản chạy thử không gửi thông báo ' + osName(), 'A dev build sends no ' + osName() + ' notifications'),
      body: t(
        'Chỉ bản ứng dụng đã đóng gói mới gửi được thông báo. Các thông báo vẫn nằm ở đây, kèm các nút bấm tương ứng.',
        'Only the packaged app can send notifications. Notifications still show up here, with the same buttons.',
      ),
      actions: [],
    };
  return null;
}

/** The Thông báo row of Cài đặt. */
export function permissionChip(p: NotifyPermission): ChipView {
  if (p === 'allowed') return { t: t('đang bật', 'on'), k: 'ok' };
  if (p === 'denied') return { t: t('đang tắt', 'off'), k: 'warn' };
  if (p === 'unbundled') return { t: t('bản chạy thử', 'dev build'), k: '' };
  return { t: t('chưa rõ', 'unknown'), k: '' };
}

// @private:begin
// ===== Firstmate =====

export interface FirstmateTabView {
  tone: Tone;
  title: string;
  body: string;
  /** "Firstmate đang thấy": empty when there is no line to show. */
  seen: FirstmateRow[];
  /** "Đã dùng từng tài khoản": each account Firstmate's line names, with its share used. */
  used: FirstmateRow[];
  /** "Kết nối": what Firstmate's quota-axi can read. */
  links: FirstmateRow[];
  note: string | null;
  /** The file is not set or not found: the button to Cài đặt. */
  setUp: boolean;
}

/** A line of a Firstmate card: its name and its value, a chip, or a value in a tone. */
export interface FirstmateRow {
  key: string;
  value: string;
  chip?: ChipView;
  tone?: Tone;
}

/** Why Firstmate's file could not be read, by `FirstmateView.status`. */
export function unreadText(status: string): string {
  if (status === 'notPlain')
    return t('Đường dẫn không phải một file thường, hoặc file lớn hơn file của firstmate.', 'The path is not a regular file, or the file is larger than firstmate’s file.');
  if (status === 'notPoolFile') return t('File phải tên là quota-pool.last.', 'The file must be named quota-pool.last.');
  return t('Không mở được file.', 'Cannot open the file.');
}

/** One line per account Firstmate's line names: used up shows red. */
function slotRows(f: FirstmateView, nameOf: (n: number) => string): FirstmateRow[] {
  return f.line!.slots.map((s) => {
    const used = s.percent == null ? null : 100 - s.percent;
    return {
      key: nameOf(s.slot),
      value: s.reset ? t('vừa đặt lại', 'just reset') : used == null ? '?' : percent(used),
      tone: !s.reset && used != null && used >= 100 ? 'bad' : undefined,
    };
  });
}

const ON_OFF = (v: boolean | null, yes: ChipView, no: ChipView): { value: string; chip?: ChipView } =>
  v == null ? { value: '—' } : { value: '', chip: v ? yes : no };

export function firstmateTab(model: AuthorModel, now: Date): FirstmateTabView {
  const f = model.firstmate;
  const nameOf = namesOf(model);
  const fallback = t('Luật "cạn" vẫn tự tính từ số của app để báo.', 'The "used up" notification still works from the app’s own numbers.');
  if (f.status === 'notSet')
    return {
      tone: 'muted',
      title: t('Chưa nối với firstmate', 'Not connected to firstmate'),
      body: t('Đặt đường dẫn file quota-pool.last của firstmate trong Cài đặt. ', 'Set the path to firstmate’s quota-pool.last file in Settings. ') + fallback,
      seen: [],
      used: [],
      links: [],
      note: null,
      setUp: true,
    };
  if (f.status === 'missing')
    return {
      tone: 'muted',
      title: t('Không thấy firstmate trên máy này', 'No firstmate on this Mac'),
      body: t('Không có file ' + (f.file ?? '') + '. ', 'No file at ' + (f.file ?? '') + '. ') + fallback,
      seen: [],
      used: [],
      links: [],
      note: null,
      setUp: true,
    };
  if (f.status !== 'ok' || !f.line)
    return {
      tone: 'bad',
      title: t('Không đọc được file của firstmate', 'Cannot read firstmate’s file'),
      body: unreadText(f.status) + ' ' + fallback,
      seen: [],
      used: [],
      links: [],
      note: null,
      setUp: true,
    };
  const line = f.line;
  const ago = secondsSince(f.writtenAt, now);
  const written = f.writtenAt ? dayClock(f.writtenAt, now) + (ago != null ? ' · ' + t(age(ago) + ' trước', age(ago) + ' ago') : '') : '—';
  let head: Pick<FirstmateTabView, 'tone' | 'title' | 'body'>;
  if (!f.fresh)
    head = {
      tone: 'warn',
      title: t('Thông tin firstmate chưa cập nhật trong ', 'Firstmate’s info has not updated in ') + (ago != null ? age(ago) : t('một lúc', 'a while')),
      body: t(
        'File của firstmate chưa được ghi lại từ ' + (f.writtenAt ? clock(f.writtenAt) : '—') + '. Firstmate có thể không chạy.',
        'Firstmate’s file has not been written since ' + (f.writtenAt ? clock(f.writtenAt) : '—') + '. Firstmate may not be running.',
      ),
    };
  else if (line.dry)
    head = {
      tone: 'bad',
      title: t('Firstmate đang chờ hạn mức Claude', 'Firstmate is waiting for Claude quota'),
      body:
        t('Hạn mức trong vòng đã cạn: firstmate giữ việc Claude mới đến khi có lại.', 'The quota in rotation is used up: firstmate holds new Claude work until it is back.') +
        (line.nextReset ? t(' Có lại lúc ', ' Back at ') + dayClock(line.nextReset, now) + '.' : ''),
    };
  else if (line.dry === false)
    head = { tone: 'ok', title: t('Firstmate thấy đủ hạn mức Claude', 'Firstmate sees enough Claude quota'), body: t('Không chờ hạn mức.', 'Not waiting for quota.') };
  else
    head = {
      tone: 'muted',
      title: t('Chưa lấy được dữ liệu hạn mức Claude trên máy này', 'Claude quota data on this Mac is not available yet'),
      body: t('File không ghi hạn mức đã cạn hay chưa.', 'The file does not say whether the quota is used up.'),
    };

  const no = t('không', 'no');
  const seen: FirstmateRow[] = [
    { key: t('Chờ hạn mức Claude', 'Waiting for Claude quota'), ...ON_OFF(line.dry, { t: t('đang chờ', 'waiting'), k: 'bad' }, { t: no, k: 'ok' }) },
    { key: t('Tài khoản đang dùng', 'Account in use'), value: line.active != null ? nameOf(line.active) : '—' },
    { key: t('Có lại lúc', 'Back at'), value: line.nextReset ? dayClock(line.nextReset, now) : '—' },
    { key: t('Cập nhật', 'Updated'), value: written },
  ];
  const links: FirstmateRow[] = [
    { key: t('quota-axi của firstmate có +cswap', 'firstmate’s quota-axi has +cswap'), ...ON_OFF(line.fork, { t: t('có', 'yes'), k: 'ok' }, { t: no, k: 'bad' }) },
    { key: t('Đọc được claude-swap', 'Reads claude-swap'), ...ON_OFF(line.claudeSwap, { t: t('được', 'yes'), k: 'ok' }, { t: no, k: 'bad' }) },
  ];
  const own = f.ownDry;
  const differs = f.fresh && own != null && line.dry != null && own !== line.dry;
  const read = model.snapshot.claude.generatedAt ? clock(model.snapshot.claude.generatedAt) : null;
  return {
    ...head,
    seen,
    used: slotRows(f, nameOf),
    links,
    note: differs
      ? t(
          'App tính từ số của nó ' + (read ? 'lúc ' + read : 'gần nhất') + ' thì hạn mức trong vòng ' + (own ? 'đã cạn' : 'chưa cạn') +
            '. Hai bên đọc ở hai lúc khác nhau; lần kiểm tra sau của firstmate sẽ khớp lại.',
          'By the app’s own numbers ' + (read ? 'from ' + read : 'from its latest read') + ', the quota in rotation ' + (own ? 'is used up' : 'is not used up') +
            '. The two read at different times; firstmate’s next check will match again.',
        )
      : t(
          'Firstmate ghi file này sau mỗi lần kiểm tra' + (read ? '; ứng dụng cập nhật dữ liệu lúc ' + read : '') +
            '. Hai giờ cập nhật có thể lệch nhau; firstmate đọc cùng các tài khoản qua bản sao của app.',
          'Firstmate writes this file after every check' + (read ? '; the app read its numbers at ' + read : '') +
            '. The two update times can differ; firstmate reads the same accounts through the app’s copy.',
        ),
    setUp: false,
  };
}
// @private:end

function pressRefused(code: string): string {
  const known: Record<string, string> = {
    answered: t('Thông báo này đã có câu trả lời, có thể từ thông báo ' + osName() + '.', 'This notification already has an answer, possibly from the ' + osName() + ' notification.'),
    unknown: t('App không còn giữ thông báo này.', 'The app no longer keeps this notification.'),
    noSuchAction: t('Thông báo này không có nút đó.', 'This notification has no such button.'),
    noPage: t(
      'Google chưa gửi trang xác minh cho tài khoản này. Đăng nhập lại trong tab Antigravity.',
      'Google has not sent a verification page for this account. Sign in again in the Antigravity tab.',
    ),
    notFound: t('Tài khoản này không còn đăng nhập trong app.', 'This account is no longer signed in to the app.'),
    browser: t('Không mở được trình duyệt.', 'Could not open the browser.'),
  };
  return known[code] ?? t('Lý do: ', 'Reason: ') + code + '.';
}

/** A button on a card that the Rust side refused, or that broke. */
export function alertFailedFeedback(error: unknown): FeedbackView {
  const code = String(error);
  return {
    tone: 'warn',
    title: t('Chưa làm được', 'Could not do that'),
    body: pressRefused(code),
    actions: [{ label: t('Đóng', 'Close'), act: 'closeFeedback' }],
  };
}

/** "Gửi thử" did not go out; it saves nothing, so nothing else changed. */
export function testFailedFeedback(error: unknown): FeedbackView {
  return {
    tone: 'warn',
    title: t('Chưa gửi được thông báo thử', 'Could not send the test notification'),
    body: t('Lý do: ' + String(error) + '. Không có gì thay đổi.', 'Reason: ' + String(error) + '. Nothing changed.'),
    actions: [{ label: t('Đóng', 'Close'), act: 'closeFeedback' }],
  };
}
