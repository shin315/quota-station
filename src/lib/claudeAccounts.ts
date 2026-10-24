// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, in words for the Claude tab: an
// account's name, sign-in and quota for its dialog, the card before any
// account is kept, the rotation's LaunchAgent, and what an action did.

import { t } from './i18n';
import { clock, percent } from './format';
import type { ClaudeAccountRow, ClaudeAccountsView, ClaudeActionDone, ClaudeWindowOut } from './model';
import type { Action, ChipView, FeedbackView } from './view';

/** The display name: the nickname, or the account's number. */
export function accountName(row: Pick<ClaudeAccountRow, 'slot' | 'alias'>): string {
  return row.alias ? row.alias : t('Tài khoản ', 'Account ') + row.slot;
}

/** What a nickname stands in for: the part of the email before @, or the account's number. */
export function localName(row: Pick<ClaudeAccountRow, 'slot' | 'email'>): string {
  return row.email ? row.email.split('@')[0] : t('Tài khoản ', 'Account ') + row.slot;
}

/** The ⋯ menu and the detail dialog hold an account out of the rotation only while it can run. */
export function rotationMatters(v: ClaudeAccountsView, advanced: boolean): boolean {
  return v.rotation === true || advanced;
}

/** Whether the sign-in Quota Station keeps still works, as the detail dialog's chip. */
export function signInChip(row: ClaudeAccountRow): ChipView {
  switch (row.status) {
    case 'reloginRequired':
      return { t: t('hết hạn', 'expired'), k: 'bad' };
    case 'stale':
      return { t: t('số cũ', 'old numbers'), k: 'stale' };
    case 'noData':
      return { t: t('chưa có số', 'no numbers yet'), k: '' };
    default: {
      const at = clock(row.fetchedAt);
      return { t: at ? t('ổn · đọc lúc ' + at, 'fine · read at ' + at) : t('ổn', 'fine'), k: 'ok' };
    }
  }
}

/** `73% 5 giờ · 12% tuần`: the windows it has numbers for; '' when none. */
export function quotaLine(row: ClaudeAccountRow): string {
  const w = row.windows;
  return [
    [w.fiveHour, t('5 giờ', '5-hour')],
    [w.sevenDay, t('tuần', 'week')],
    [w.fable, t('Fable tuần', 'Fable week')],
  ]
    .filter((x): x is [ClaudeWindowOut, string] => x[0] != null)
    .map(([x, name]) => percent(x.usedPercent) + ' ' + name)
    .join(' · ');
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

/** What went wrong, in a line. */
export function errorText(done: ClaudeActionDone): string {
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

const close = (): { label: string; act: Action } => ({ label: t('Đóng', 'Close'), act: 'closeFeedback' });

/**
 * What an action from the tab did, once it is done, as the line under the
 * tab's head. `who`: the account's name as the table shows it, taken before
 * the action (a removed account has none after). The adds report in their
 * own dialog; null for those.
 */
export function resultFeedback(v: ClaudeAccountsView, done: ClaudeActionDone, who: string): FeedbackView | null {
  if (!done.ok) return { tone: 'bad', title: errorText(done), body: '', actions: [close()] };
  const row = v.accounts.find((a) => a.slot === done.slot);
  switch (done.action) {
    case 'import':
      return { tone: '', title: importText(done), body: '', actions: [] };
    case 'remove':
      return {
        tone: '',
        title: t('Đã xoá ' + who + ' khỏi máy này.', 'Removed ' + who + ' from this Mac.'),
        body: t('Lịch sử vẫn giữ các lần chuyển cũ. Đăng nhập của Claude Code không đổi.', 'History keeps its past switches. Claude Code’s sign-in is unchanged.'),
        actions: [close()],
      };
    case 'alias':
      return row?.alias
        ? {
            tone: '',
            title: t('Đã lưu tên gọi ' + row.alias + '.', 'Saved the nickname ' + row.alias + '.'),
            body: t('Popover, cửa sổ này và Lịch sử dùng tên này.', 'The popover, this window and History use this name.'),
            actions: [close()],
          }
        : { tone: '', title: t('Đã bỏ tên gọi của ' + who + '.', 'Removed ' + who + '’s nickname.'), body: '', actions: [] };
    case 'enabled':
      return row?.enabled === false
        ? {
            tone: '',
            title: t('Đã cho ' + who + ' nghỉ khỏi tự chuyển.', who + ' is now out of auto-switch.'),
            body: t('Tự chuyển trên máy này bỏ qua tài khoản này cho tới khi bạn cho vào lại.', 'Auto-switch on this Mac skips this account until you put it back.'),
            actions: [{ label: t('Hoàn tác', 'Undo'), act: `claude:unrest:${done.slot ?? 0}` }, close()],
          }
        : { tone: '', title: t('Đã cho ' + who + ' vào lại tự chuyển.', who + ' is back in auto-switch.'), body: '', actions: [] };
    case 'rotation':
      return {
        tone: '',
        title: v.rotation
          ? t('Đã bật tự chuyển bằng Quota Station.', 'Turned on auto-switch with Quota Station.')
          : t('Đã tắt tự chuyển bằng Quota Station.', 'Turned off auto-switch with Quota Station.'),
        body: '',
        actions: [],
      };
    default:
      return null;
  }
}

/** After an add's dialog closes: the account is in, its numbers come with the next read. */
export function addedFeedback(who: string, relogin: boolean): FeedbackView {
  return {
    tone: '',
    title: relogin ? t(who + ' đã đăng nhập lại.', who + ' is signed in again.') : t('Đã thêm ' + who + '.', 'Added ' + who + '.'),
    body: t('Hạn mức có ở lần đọc tới.', 'Its quota shows up on the next read.'),
    actions: [close()],
  };
}
