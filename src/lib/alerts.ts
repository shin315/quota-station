// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the Cảnh báo and Firstmate tabs show (F09, F10). The cards carry the
// words and buttons the Rust side wrote for the macOS card, so both say the
// same; this file adds when, what was pressed and how it went. Pure, like
// `view.ts`. Copy follows the prototype's `tabAlerts`, `alertCard` and
// `tabFirstmate`.

import { age, clock, percent, secondsSince } from './format';
import type { AuthorModel } from './author';
import type { AlertRecord, AlertTone, FirstmateView, Model, NotifyPermission, SwitchKind } from './model';
import { dayClock, namesOf } from './switching';
import type { BannerView, ChipView, FeedbackView, Tone } from './view';

/** The five kinds, in the prototype's order and words. */
export const ALERT_KINDS: { kind: SwitchKind; title: string; text: string }[] = [
  {
    kind: 'pool',
    title: 'Hạn mức Claude trong vòng sắp cạn / có lại',
    text: 'Khi hạn mức trong vòng đã dùng từ 90% trở lên và khi có lại. Cùng luật với firstmate.',
  },
  { kind: 'leaving', title: 'Tài khoản sắp rời vòng', text: '30 phút trước giờ rời vòng trong file luật.' },
  {
    kind: 'login',
    title: 'Tài khoản cần đăng nhập lại',
    text: '3 ngày trước khi hết hạn, hoặc khi claude-swap báo lỗi đăng nhập. Cả tài khoản Antigravity cần đăng nhập lại hay xác minh.',
  },
  {
    kind: 'overwrite',
    title: 'Antigravity đổi về tài khoản cũ',
    text: 'Trong 6 giờ sau khi chuyển. Thôi đề nghị chuyển lại sau 2 lần.',
  },
  {
    kind: 'firstmate',
    title: 'Firstmate đang chờ hạn mức Claude',
    text: 'Khi firstmate báo hạn mức trong vòng đã cạn. Chỉ trên máy có firstmate.',
  },
];

/**
 * The kinds this Mac can turn on, in that order: the rotation's (2) and
 * Firstmate's (5) only with Cài đặt › Nâng cao (Firstmate's behind its own
 * switch too), the overwritten Antigravity switch (4) only in a build that
 * switches Antigravity. Alert 1 says its thresholds.
 */
export function alertKinds(model: Model): { kind: SwitchKind; title: string; text: string }[] {
  const adv = model.advanced.on;
  const fm = adv && model.advanced.firstmate;
  const agy = model.capabilities.agyAccounts;
  const p = model.alerts.poolAlert;
  return ALERT_KINDS.filter((k) => (k.kind !== 'leaving' || adv) && (k.kind !== 'firstmate' || fm) && (k.kind !== 'overwrite' || agy)).map((k) => {
    if (k.kind === 'pool')
      return {
        ...k,
        title: adv ? k.title : 'Hạn mức Claude sắp cạn / có lại',
        text:
          'Khi tài khoản đang dùng còn ' + p.dryAtMost + '% trở xuống và không tài khoản' + (adv ? ' nào trong vòng' : ' nào') +
          ' còn ' + p.roomAtLeast + '%, và khi có lại.' + (fm ? ' Cùng luật với firstmate.' : ''),
      };
    if (k.kind === 'login' && !agy) return { ...k, text: k.text.replace(/ Cả tài khoản Antigravity.*$/, '') };
    return k;
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
  /** How it went; `working` while a switch back runs. */
  result: FeedbackView | { working: string } | null;
  meta: string;
}

function iso(unix: number): string {
  return new Date(unix * 1000).toISOString();
}

function working(action: string): string {
  return action.startsWith('agy:quitFirst:')
    ? 'Đang thoát Antigravity.app và chuyển tài khoản…'
    : 'Đang chuyển tài khoản Antigravity…';
}

export function alertCard(r: AlertRecord, now: Date): AlertCardView {
  const a = r.alert;
  const answer = r.answer;
  const icon = a.tone === 'warn' ? '' : a.tone;
  const meta = [dayClock(iso(a.at), now)];
  if (a.test) meta.push('thông báo thử');
  meta.push(r.notified ? 'đã gửi thông báo macOS' : 'chỉ có trong app');
  return {
    id: r.id,
    icon: { k: icon, mark: a.tone === 'ok' ? '✓' : '!' },
    title: a.title,
    body: a.body,
    actions: answer ? [] : a.actions,
    chose: answer ? 'Bạn đã chọn “' + answer.label + '”.' : null,
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
      title: 'Thông báo của Quota Station đang tắt',
      body: 'macOS không hiện thẻ nào. Các cảnh báo vẫn nằm ở đây, kèm nút như trên thẻ.',
      actions: [{ label: 'Mở Cài đặt hệ thống', act: 'notify:settings' }],
    };
  if (p === 'unbundled')
    return {
      tone: 'info',
      title: 'Bản chạy thử không gửi thông báo macOS',
      body: 'Chỉ app đã đóng gói mới có thẻ thông báo. Các cảnh báo vẫn nằm ở đây, kèm nút như trên thẻ.',
      actions: [],
    };
  return null;
}

/** The Thông báo row of Cài đặt. */
export const PERMISSION_CHIPS: Record<NotifyPermission, ChipView> = {
  allowed: { t: 'đang bật', k: 'ok' },
  denied: { t: 'đang tắt', k: 'warn' },
  unknown: { t: 'chưa rõ', k: '' },
  unbundled: { t: 'bản chạy thử', k: '' },
};

// ===== Firstmate =====

export interface FirstmateTabView {
  tone: Tone;
  title: string;
  body: string;
  /** "Firstmate đang thấy": empty when there is no line to show. */
  rows: { key: string; value: string; chip?: ChipView }[];
  note: string | null;
  /** The file is not set or not found: the button to Cài đặt. */
  setUp: boolean;
}

/** Why Firstmate's file could not be read, by `FirstmateView.status`. */
export const UNREAD: Record<string, string> = {
  notPlain: 'Đường dẫn không phải một file thường, hoặc file lớn hơn file của firstmate.',
  notPoolFile: 'File phải tên là quota-pool.last.',
  unreadable: 'Không mở được file.',
};

function slots(f: FirstmateView, nameOf: (n: number) => string): string {
  const line = f.line!;
  if (!line.slots.length) return '—';
  return line.slots
    .map((s) => nameOf(s.slot) + ' ' + (s.reset ? 'vừa đặt lại' : s.percent == null ? '?' : 'đã dùng ' + percent(100 - s.percent)))
    .join(' · ');
}

const ON_OFF = (v: boolean | null, yes: ChipView, no: ChipView): { value: string; chip?: ChipView } =>
  v == null ? { value: '—' } : { value: '', chip: v ? yes : no };

export function firstmateTab(model: AuthorModel, now: Date): FirstmateTabView {
  const f = model.firstmate;
  const nameOf = namesOf(model);
  const fallback = 'Luật "cạn" vẫn tự tính từ số của app để báo.';
  if (f.status === 'notSet')
    return {
      tone: 'muted',
      title: 'Chưa nối với firstmate',
      body: 'Đặt đường dẫn file quota-pool.last của firstmate trong Cài đặt. ' + fallback,
      rows: [],
      note: null,
      setUp: true,
    };
  if (f.status === 'missing')
    return {
      tone: 'muted',
      title: 'Không thấy firstmate trên máy này',
      body: 'Không có file ' + (f.file ?? '') + '. ' + fallback,
      rows: [],
      note: null,
      setUp: true,
    };
  if (f.status !== 'ok' || !f.line)
    return {
      tone: 'bad',
      title: 'Không đọc được file của firstmate',
      body: (UNREAD[f.status] ?? UNREAD.unreadable) + ' ' + fallback,
      rows: [],
      note: null,
      setUp: true,
    };
  const line = f.line;
  const ago = secondsSince(f.writtenAt, now);
  const written = f.writtenAt ? dayClock(f.writtenAt, now) + (ago != null ? ' · ' + age(ago) + ' trước' : '') : '—';
  let head: Pick<FirstmateTabView, 'tone' | 'title' | 'body'>;
  if (!f.fresh)
    head = {
      tone: 'warn',
      title: 'Thông tin firstmate chưa cập nhật trong ' + (ago != null ? age(ago) : 'một lúc'),
      body: 'File của firstmate chưa được ghi lại từ ' + (f.writtenAt ? clock(f.writtenAt) : '—') + '. Firstmate có thể không chạy.',
    };
  else if (line.dry)
    head = {
      tone: 'bad',
      title: 'Firstmate đang chờ hạn mức Claude',
      body:
        'Hạn mức trong vòng đã cạn: firstmate giữ việc Claude mới đến khi có lại.' +
        (line.nextReset ? ' Có lại lúc ' + dayClock(line.nextReset, now) + '.' : ''),
    };
  else if (line.dry === false) head = { tone: 'ok', title: 'Firstmate thấy đủ hạn mức Claude', body: 'Không chờ hạn mức.' };
  else head = { tone: 'muted', title: 'Firstmate chưa tính được hạn mức Claude', body: 'File không ghi hạn mức đã cạn hay chưa.' };

  const rows: FirstmateTabView['rows'] = [
    { key: 'Chờ hạn mức Claude', ...ON_OFF(line.dry, { t: 'đang chờ', k: 'bad' }, { t: 'không', k: 'ok' }) },
    { key: 'Tài khoản đang dùng', value: line.active != null ? nameOf(line.active) : '—' },
    { key: 'Đã dùng từng tài khoản', value: slots(f, nameOf) },
    { key: 'Có lại lúc', value: line.nextReset ? dayClock(line.nextReset, now) : '—' },
    { key: 'quota-axi của firstmate có +cswap', ...ON_OFF(line.fork, { t: 'có', k: 'ok' }, { t: 'không', k: 'bad' }) },
    { key: 'Đọc được claude-swap', ...ON_OFF(line.claudeSwap, { t: 'được', k: 'ok' }, { t: 'không', k: 'bad' }) },
    { key: 'Cập nhật', value: written },
  ];
  const own = f.ownDry;
  const differs = f.fresh && own != null && line.dry != null && own !== line.dry;
  const read = model.snapshot.claude.generatedAt ? clock(model.snapshot.claude.generatedAt) : null;
  return {
    ...head,
    rows,
    note: differs
      ? 'App tính từ số của nó ' + (read ? 'lúc ' + read : 'gần nhất') + ' thì hạn mức trong vòng ' + (own ? 'đã cạn' : 'chưa cạn') +
        '. Hai bên đọc ở hai lúc khác nhau; lần kiểm sau của firstmate sẽ khớp lại.'
      : 'Firstmate ghi file này sau mỗi lần kiểm' + (read ? '; app đọc số lúc ' + read : '') +
        '. Hai giờ cập nhật có thể lệch nhau; firstmate đọc cùng các tài khoản qua bản sao của app.',
    setUp: false,
  };
}

const PRESS_REFUSED: Record<string, string> = {
  answered: 'Cảnh báo này đã có câu trả lời, có thể từ thẻ macOS.',
  unknown: 'App không còn giữ cảnh báo này.',
  noSuchAction: 'Thẻ này không có nút đó.',
  noPage: 'Google chưa gửi trang xác minh cho tài khoản này. Đăng nhập lại trong tab Antigravity.',
  notFound: 'Tài khoản này không còn đăng nhập trong app.',
  browser: 'Không mở được trình duyệt.',
};

/** A button on a card that the Rust side refused, or that broke. */
export function alertFailedFeedback(error: unknown): FeedbackView {
  const code = String(error);
  return {
    tone: 'warn',
    title: 'Chưa làm được',
    body: PRESS_REFUSED[code] ?? 'Lý do: ' + code + '.',
    actions: [{ label: 'Đóng', act: 'closeFeedback' }],
  };
}

/** "Gửi thử" did not go out; it saves nothing, so nothing else changed. */
export function testFailedFeedback(error: unknown): FeedbackView {
  return {
    tone: 'warn',
    title: 'Chưa gửi được thông báo thử',
    body: 'Lý do: ' + String(error) + '. Không có gì thay đổi.',
    actions: [{ label: 'Đóng', act: 'closeFeedback' }],
  };
}
