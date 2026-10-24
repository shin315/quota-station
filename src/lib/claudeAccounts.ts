// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, in words for the Claude tab: each
// account's row, the card before any account is kept, the rotation's
// LaunchAgent, and what the last action did.

import { t } from './i18n';
import type { ClaudeAccountRow, ClaudeAccountsView, ClaudeActionDone } from './model';
import type { ChipView, FeedbackView } from './view';

export interface AccountRowView {
  slot: number;
  name: string;
  /** The name the owner gave it; '' when it goes by its number. */
  alias: string;
  /** The email (while names show) and the account's kind. */
  sub: string;
  chips: ChipView[];
  enabled: boolean;
  active: boolean;
  /** Why it cannot be removed; null when it can. */
  keep: string | null;
}

export function accountName(row: Pick<ClaudeAccountRow, 'slot' | 'alias'>): string {
  return row.alias ? row.alias : t('Tài khoản ', 'Account ') + row.slot;
}

function statusChip(row: ClaudeAccountRow): ChipView | null {
  switch (row.status) {
    case 'reloginRequired':
      return { t: t('cần đăng nhập lại', 'sign in again'), k: 'bad' };
    case 'noData':
      return { t: t('chưa có số', 'no numbers yet'), k: '' };
    case 'stale':
      return { t: t('số cũ', 'old numbers'), k: 'stale' };
    default:
      return null;
  }
}

export function accountRows(v: ClaudeAccountsView): AccountRowView[] {
  return v.accounts.map((row) => {
    const chips: ChipView[] = [];
    if (row.active) chips.push({ t: t('đang dùng', 'in use'), k: 'ok' });
    if (!row.enabled) chips.push({ t: t('ngoài vòng xoay', 'out of rotation'), k: '' });
    const status = statusChip(row);
    if (status) chips.push(status);
    const parts = [row.email ?? null, row.kind === 'setupToken' ? 'setup token' : null, t('số ', 'No. ') + row.slot].filter(Boolean);
    return {
      slot: row.slot,
      name: accountName(row),
      alias: row.alias ?? '',
      sub: parts.join(' · '),
      chips,
      enabled: row.enabled,
      active: row.active,
      keep: row.active ? t('Đang dùng: chuyển sang tài khoản khác trước khi xoá.', 'In use: switch to another account before removing it.') : null,
    };
  });
}

/** Before Quota Station keeps any account: what to do first. */
export function setupText(v: ClaudeAccountsView): { title: string; body: string } | null {
  if (v.ready) return null;
  return v.importable
    ? {
        title: t('Đưa các tài khoản Claude vào Quota Station', 'Bring your Claude accounts into Quota Station'),
        body: t(
          'Nhập từ claude-swap để mang các tài khoản sang, giữ nguyên số và không phải đăng nhập lại. claude-swap và dữ liệu của nó vẫn để nguyên.',
          'Import from claude-swap to bring the accounts over, with the same numbers and no new sign-in. claude-swap and its data stay as they are.',
        ),
      }
    : {
        title: t('Chưa có tài khoản Claude nào', 'No Claude accounts yet'),
        body: t(
          'Đăng nhập một tài khoản mới, thêm tài khoản Claude Code đang đăng nhập, hoặc dán một setup token.',
          'Sign in to a new account, add the account Claude Code is signed in to, or paste a setup token.',
        ),
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
    ? v.legacy +
      t(
        ' vẫn được nạp. Quota Station không chuyển tài khoản khi nó còn chạy. Tắt nó trước khi bật tự chuyển ở đây.',
        ' is still loaded. Quota Station does not switch accounts while it runs. Turn it off before turning on auto-switch here.',
      )
    : null;
  let line: string;
  if (!v.ready) line = t('Cần ít nhất một tài khoản.', 'Needs at least one account.');
  else if (v.rotation === true) line = t('LaunchAgent chạy mỗi 3 phút, kể cả khi đã đóng app.', 'The LaunchAgent runs every 3 minutes, even when the app is closed.');
  else if (v.rotation === false) line = t('Đang tắt: không có gì tự chuyển Claude.', 'Off: nothing switches Claude automatically.');
  else line = t('Không rõ LaunchAgent có đang nạp không.', 'Not known whether the LaunchAgent is loaded.');
  return { on: v.rotation === true, disabled: !v.ready || (!!v.legacy && v.rotation !== true) || v.working != null, line, legacy };
}

/** The running action in words; null when none runs. */
export function workingText(v: ClaudeAccountsView): string | null {
  switch (v.working) {
    case null:
      return null;
    case 'import':
      return t('Đang nhập từ claude-swap…', 'Importing from claude-swap…');
    case 'login':
      return t(
        'Đang chờ đăng nhập trong trình duyệt… Trang đăng nhập của Claude mở trong trình duyệt mặc định. Chờ tối đa 10 phút.',
        'Waiting for sign-in in the browser… Claude’s sign-in page opens in the default browser. Waits up to 10 minutes.',
      );
    case 'rotation':
      return t('Đang nạp LaunchAgent…', 'Loading the LaunchAgent…');
    default:
      return t('Đang lưu…', 'Saving…');
  }
}

function errorText(done: ClaudeActionDone): string {
  const detail = done.detail ? ': ' + done.detail : '';
  switch (done.error) {
    case 'busy':
      return t('Một thao tác khác đang giữ tài khoản. Thử lại sau vài giây.', 'Another action is holding the accounts. Try again in a few seconds.');
    case 'active':
      return t('Không xoá được tài khoản đang dùng. Chuyển sang tài khoản khác trước.', 'Cannot remove the account in use. Switch to another account first.');
    case 'noLogin':
      return t('Không thấy đăng nhập nào để thêm.', 'No sign-in found to add.');
    case 'badToken':
      return t('Token trống hoặc không nằm trên một dòng.', 'The token is empty or not on one line.');
    case 'badAlias':
      return t('Tên quá dài: tối đa 24 ký tự.', 'The name is too long: 24 characters at most.');
    case 'login':
      return t('Đăng nhập không xong', 'Sign-in did not finish') + detail + '.';
    case 'cancelled':
      return t('Đã huỷ đăng nhập. Không thêm gì.', 'Sign-in cancelled. Nothing was added.');
    case 'keychain':
      return t('Keychain không cho ghi', 'The Keychain refused the write') + detail + '.';
    case 'legacy':
      return t('Bộ tự chuyển cũ ' + (done.detail ?? '') + ' còn chạy: tắt nó trước.', 'The old auto-switcher ' + (done.detail ?? '') + ' is still running: turn it off first.');
    case 'launchctl':
      return t('launchctl báo lỗi', 'launchctl reported an error') + detail + '.';
    case 'notFound':
      return t('Không thấy dữ liệu claude-swap trên máy này.', 'No claude-swap data on this Mac.');
    case 'corrupt':
      return t('Không đọc được sequence.json của claude-swap. Không nhập gì.', 'Cannot read claude-swap’s sequence.json. Nothing was imported.');
    case 'unknownSlot':
      return t('Quota Station không giữ tài khoản này.', 'Quota Station does not keep this account.');
    default:
      return t('Không làm được (', 'Could not do it (') + (done.error ?? t('không rõ', 'unknown')) + ')' + detail + '.';
  }
}

/** `2 tài khoản mới`; English takes `one` or `many` by the count. */
function count(n: number, what: string, one: string, many: string): string | null {
  return n ? n + ' ' + t(what, n === 1 ? one : many) : null;
}

function importText(done: ClaudeActionDone): string {
  const i = done.imported;
  if (!i) return t('Đã nhập từ claude-swap.', 'Imported from claude-swap.');
  const parts = [
    count(i.added.length, 'tài khoản mới', 'new account', 'new accounts'),
    count(i.updated.length, 'đăng nhập mới hơn', 'newer sign-in', 'newer sign-ins'),
    count(i.unchanged.length, 'giữ nguyên', 'unchanged', 'unchanged'),
  ].filter(Boolean);
  let text = t('Đã nhập từ claude-swap: ', 'Imported from claude-swap: ') + (parts.length ? parts.join(', ') : t('không có gì mới', 'nothing new')) + '.';
  if (i.skipped.length)
    text += t(' Bỏ qua tài khoản ', i.skipped.length === 1 ? ' Skipped account ' : ' Skipped accounts ') + i.skipped.map(([s]) => s).join(', ') + '.';
  if (i.rotation) text += t(' Đã chép luật xoay vòng.', ' Copied the rotation rules.');
  return text + t(' claude-swap vẫn để nguyên.', ' claude-swap stays as it is.');
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
      title = done.renewed ? t('Đã cập nhật đăng nhập của ' + who + '.', 'Updated ' + who + '’s sign-in.') : t('Đã thêm ' + who + '.', 'Added ' + who + '.');
      break;
    case 'token':
      title = t('Đã thêm setup token thành ' + who + '.', 'Added the setup token as ' + who + '.');
      break;
    case 'remove':
      title = t(
        'Đã xoá tài khoản ' + done.slot + ' khỏi Quota Station. Đăng nhập của Claude Code không đổi.',
        'Removed account ' + done.slot + ' from Quota Station. Claude Code’s sign-in is unchanged.',
      );
      break;
    case 'alias':
      title = t('Đã đổi tên thành ' + who + '.', 'Renamed to ' + who + '.');
      break;
    case 'enabled': {
      const row = v.accounts.find((a) => a.slot === done.slot);
      title =
        row?.enabled === false
          ? who + t(' ra khỏi vòng xoay; vẫn chuyển tay được.', ' left the rotation; you can still switch to it by hand.')
          : who + t(' vào lại vòng xoay.', ' is back in rotation.');
      break;
    }
    case 'rotation':
      title = v.rotation
        ? t('Đã bật tự chuyển bằng Quota Station.', 'Turned on auto-switch with Quota Station.')
        : t('Đã tắt tự chuyển bằng Quota Station.', 'Turned off auto-switch with Quota Station.');
      break;
  }
  return { tone: '', title, body: '', actions: [] };
}
