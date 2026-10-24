// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, in words for the Claude tab: each
// account's row, the card before any account is kept, the rotation's
// LaunchAgent, and what the last action did.

import type { ClaudeAccountRow, ClaudeAccountsView, ClaudeActionDone } from './model';
import type { ChipView, FeedbackView } from './view';

export interface AccountRowView {
  slot: number;
  name: string;
  /** The email (while names show) and the account's kind. */
  sub: string;
  chips: ChipView[];
  enabled: boolean;
  active: boolean;
  /** Why it cannot be removed; null when it can. */
  keep: string | null;
}

export function accountName(row: Pick<ClaudeAccountRow, 'slot' | 'alias'>): string {
  return row.alias ? row.alias : 'Tài khoản ' + row.slot;
}

function statusChip(row: ClaudeAccountRow): ChipView | null {
  switch (row.status) {
    case 'reloginRequired':
      return { t: 'cần đăng nhập lại', k: 'bad' };
    case 'noData':
      return { t: 'chưa có số', k: '' };
    case 'stale':
      return { t: 'số cũ', k: 'stale' };
    default:
      return null;
  }
}

export function accountRows(v: ClaudeAccountsView): AccountRowView[] {
  return v.accounts.map((row) => {
    const chips: ChipView[] = [];
    if (row.active) chips.push({ t: 'đang dùng', k: 'ok' });
    if (!row.enabled) chips.push({ t: 'ngoài vòng xoay', k: '' });
    const status = statusChip(row);
    if (status) chips.push(status);
    const parts = [row.email ?? null, row.kind === 'setupToken' ? 'setup token' : null, 'số ' + row.slot].filter(Boolean);
    return {
      slot: row.slot,
      name: accountName(row),
      sub: parts.join(' · '),
      chips,
      enabled: row.enabled,
      active: row.active,
      keep: row.active ? 'Đang dùng: chuyển sang tài khoản khác trước khi xoá.' : null,
    };
  });
}

/** Before Quota Station keeps any account: what to do first. */
export function setupText(v: ClaudeAccountsView): { title: string; body: string } | null {
  if (v.ready) return null;
  return v.importable
    ? {
        title: 'Đưa các tài khoản Claude vào Quota Station',
        body:
          'Nhập từ claude-swap để mang các tài khoản sang, giữ nguyên số và không phải đăng nhập lại. claude-swap và dữ liệu của nó vẫn để nguyên.',
      }
    : {
        title: 'Chưa có tài khoản Claude nào',
        body: 'Đăng nhập một tài khoản mới, thêm tài khoản Claude Code đang đăng nhập, hoặc dán một setup token.',
      };
}

export interface RotationRowView {
  on: boolean;
  disabled: boolean;
  line: string;
  /** The older switcher still loaded: the warning, with its guide. */
  legacy: string | null;
}

export function rotationRow(v: ClaudeAccountsView): RotationRowView {
  const legacy = v.legacy
    ? v.legacy + ' vẫn được nạp. Quota Station không chuyển tài khoản khi nó còn chạy. Tắt nó trước khi bật tự chuyển ở đây.'
    : null;
  let line: string;
  if (!v.ready) line = 'Cần ít nhất một tài khoản.';
  else if (v.rotation === true) line = 'LaunchAgent chạy mỗi 3 phút, kể cả khi đã đóng app.';
  else if (v.rotation === false) line = 'Đang tắt: không có gì tự chuyển Claude.';
  else line = 'Không rõ LaunchAgent có đang nạp không.';
  return { on: v.rotation === true, disabled: !v.ready || (!!v.legacy && v.rotation !== true) || v.working != null, line, legacy };
}

/** The running action in words; null when none runs. */
export function workingText(v: ClaudeAccountsView): string | null {
  switch (v.working) {
    case null:
      return null;
    case 'import':
      return 'Đang nhập từ claude-swap…';
    case 'login':
      return 'Đang chờ đăng nhập trong trình duyệt… Trang đăng nhập của Claude mở trong trình duyệt mặc định. Chờ tối đa 10 phút.';
    case 'rotation':
      return 'Đang nạp LaunchAgent…';
    default:
      return 'Đang lưu…';
  }
}

function errorText(done: ClaudeActionDone): string {
  const detail = done.detail ? ': ' + done.detail : '';
  switch (done.error) {
    case 'busy':
      return 'Một thao tác khác đang giữ tài khoản. Thử lại sau vài giây.';
    case 'active':
      return 'Không xoá được tài khoản đang dùng. Chuyển sang tài khoản khác trước.';
    case 'noLogin':
      return 'Không thấy đăng nhập nào để thêm.';
    case 'badToken':
      return 'Token trống hoặc không nằm trên một dòng.';
    case 'badAlias':
      return 'Tên quá dài: tối đa 24 ký tự.';
    case 'login':
      return 'Đăng nhập không xong' + detail + '.';
    case 'cancelled':
      return 'Đã huỷ đăng nhập. Không thêm gì.';
    case 'keychain':
      return 'Keychain không cho ghi' + detail + '.';
    case 'legacy':
      return 'Bộ tự chuyển cũ ' + (done.detail ?? '') + ' còn chạy: tắt nó trước.';
    case 'launchctl':
      return 'launchctl báo lỗi' + detail + '.';
    case 'notFound':
      return 'Không thấy dữ liệu claude-swap trên máy này.';
    case 'corrupt':
      return 'Không đọc được sequence.json của claude-swap. Không nhập gì.';
    case 'unknownSlot':
      return 'Quota Station không giữ tài khoản này.';
    default:
      return 'Không làm được (' + (done.error ?? 'không rõ') + ')' + detail + '.';
  }
}

function count(n: number, what: string): string | null {
  return n ? n + ' ' + what : null;
}

function importText(done: ClaudeActionDone): string {
  const i = done.imported;
  if (!i) return 'Đã nhập từ claude-swap.';
  const parts = [count(i.added.length, 'tài khoản mới'), count(i.updated.length, 'đăng nhập mới hơn'), count(i.unchanged.length, 'giữ nguyên')].filter(
    Boolean,
  );
  let text = 'Đã nhập từ claude-swap: ' + (parts.length ? parts.join(', ') : 'không có gì mới') + '.';
  if (i.skipped.length) text += ' Bỏ qua tài khoản ' + i.skipped.map(([s]) => s).join(', ') + '.';
  if (i.rotation) text += ' Đã chép luật xoay vòng.';
  return text + ' claude-swap vẫn để nguyên.';
}

/** What the last action did, as a line under the card; null when there is none. */
export function doneFeedback(v: ClaudeAccountsView): FeedbackView | null {
  const done = v.last;
  if (!done) return null;
  if (!done.ok) return { tone: 'bad', title: errorText(done), body: '', actions: [] };
  const who = done.slot != null ? accountName(v.accounts.find((a) => a.slot === done.slot) ?? { slot: done.slot, alias: null }) : '';
  let title: string;
  switch (done.action) {
    case 'import':
      title = importText(done);
      break;
    case 'login':
    case 'current':
      title = done.renewed ? 'Đã cập nhật đăng nhập của ' + who + '.' : 'Đã thêm ' + who + '.';
      break;
    case 'token':
      title = 'Đã thêm setup token thành ' + who + '.';
      break;
    case 'remove':
      title = 'Đã xoá tài khoản ' + done.slot + ' khỏi Quota Station. Đăng nhập của Claude Code không đổi.';
      break;
    case 'alias':
      title = 'Đã đổi tên thành ' + who + '.';
      break;
    case 'enabled': {
      const row = v.accounts.find((a) => a.slot === done.slot);
      title = row?.enabled === false ? who + ' ra khỏi vòng xoay; vẫn chuyển tay được.' : who + ' vào lại vòng xoay.';
      break;
    }
    case 'rotation':
      title = v.rotation ? 'Đã bật tự chuyển bằng Quota Station.' : 'Đã tắt tự chuyển bằng Quota Station.';
      break;
  }
  return { tone: '', title, body: '', actions: [] };
}
