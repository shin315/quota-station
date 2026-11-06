// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the bell's panel (Thông báo) shows (F09). The cards carry the
// words and buttons the Rust side wrote for the macOS card, so both say the
// same; this file adds when, what was pressed and how it went. Pure, like
// `view.ts`. Copy follows the prototype's `tabAlerts` and `alertCard`.

import { clock } from './format';
import { appName, osName, systemSettings, t } from './i18n';
import type { AlertRecord, AlertTone, Model, NotifyPermission, SwitchKind } from './model';
import { dayLabel } from './history';
import type { BannerView, ChipView, FeedbackView } from './view';
import { advancedOn } from './advanced';
import ext from '@qs-ext';

/**
 * The kinds, in the prototype's order and words. `agy`: this build
 * switches Antigravity, so the sign-in alert covers its accounts too.
 * `rotation`: Nâng cao is on, so alert 1 counts the quota in rotation.
 */
export function alertKindList(agy = true, rotation = false): { kind: SwitchKind; title: string; text: string }[] {
  let pool = {
    title: t('Hạn mức Claude sắp cạn / có lại', 'Claude quota running out / back'),
    text: t('Khi hạn mức đã dùng từ 90% trở lên và khi có lại.', 'When the quota is 90% used or more, and when it is back.'),
  };
  if (rotation)
    pool = {
      title: t('Hạn mức Claude để tự chuyển sắp cạn / có lại', 'Claude quota to switch to running out / back'),
      text: t('Khi hạn mức để tự chuyển đã dùng từ 90% trở lên và khi có lại.', 'When the quota to switch to is 90% used or more, and when it is back.'),
    };
  if (rotation && ext.alerts) pool.text += ext.alerts.poolRule();
  return [
    { kind: 'pool', ...pool },
    {
      kind: 'leaving',
      title: t('Tài khoản sắp ngừng dùng', 'An account is about to stop being used'),
      text: t('30 phút trước giờ ngừng dùng trong file luật.', '30 minutes before its stop time in the rules file.'),
    },
    {
      kind: 'switched',
      title: t('Đã tự chuyển tài khoản Claude', 'Claude auto-switched'),
      text: t(
        'Mỗi lần Tự chuyển khi hết quota đổi sang tài khoản khác.',
        'Each time auto-switch when quota runs out moves to another account.',
      ),
    },
    {
      kind: 'login',
      title: t('Tài khoản cần đăng nhập lại', 'An account needs to sign in again'),
      text:
        t('3 ngày trước khi hết hạn, hoặc khi lần đọc hạn mức báo lỗi đăng nhập.', '3 days before the sign-in expires, or when a quota read reports a sign-in error.') +
        (agy ? t(' Cả tài khoản Antigravity cần đăng nhập lại hay xác minh.', ' Also Antigravity accounts that need to sign in again or verify.') : ''),
    },
    {
      kind: 'overwrite',
      title: t('Antigravity đổi về tài khoản cũ', 'Antigravity went back to the old account'),
      text: t('Trong 6 giờ sau khi chuyển. Ngừng đề nghị chuyển lại sau 2 lần.', 'Within 6 hours of a switch. Stops offering to switch back after 2 times.'),
    },
    ...(ext.alerts?.kinds() ?? []),
  ];
}

/**
 * The kinds this Mac can turn on, in that order: the overwritten
 * Antigravity switch only in a build that switches Antigravity. Alert 1
 * says its thresholds.
 */
/** The rotation's (2) only with Cài đặt › Nâng cao; an extension's where it says. */
export function alertKinds(model: Model): { kind: SwitchKind; title: string; text: string }[] {
  const agy = model.capabilities.agyAccounts;
  const p = model.alerts.poolAlert;
  const adv = advancedOn(model);
  const extKinds = new Set(ext.alerts?.kinds().map((k) => k.kind));
  const extShown = !!ext.alerts?.shown(model);
  return alertKindList(agy, adv)
    .filter((k) => k.kind !== 'overwrite' || agy)
    .filter((k) => k.kind !== 'leaving' || adv)
    .filter((k) => !extKinds.has(k.kind) || extShown)
    .map((k) => {
      if (k.kind !== 'pool') return k;
      let title = t('Hạn mức Claude sắp cạn / có lại', 'Claude quota running out / back');
      let where = ['', ''];
      if (adv) [title, where] = [k.title, [' được dùng', ' to switch to']];
      return {
        ...k,
        title,
        text:
          t(
            'Khi tài khoản đang dùng còn ' + p.dryAtMost + '% trở xuống và không tài khoản nào' + where[0] + ' còn ' + p.roomAtLeast + '%, và khi có lại.',
            'When the account in use has ' + p.dryAtMost + '% or less left and no account' + where[1] + ' still has ' + p.roomAtLeast + '%, and when it is back.',
          ) + (extShown ? ext.alerts!.poolRule() : ''),
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
