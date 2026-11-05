// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// The words of the Antigravity login card (M4a): the accounts this app logs
// in, what is missing before it can, what each login or logout did, and how
// the app's own read of each account's quota stands (M4b). Accounts go by
// their email, as on the quota rows, or by number ("Ultra 1") when the
// owner hides names in Cài đặt.

import { agyNames } from '../lib/accounts';
import { clock } from '../lib/format';
import { mac, onWindows, os, osName, secretStore, t } from '../lib/i18n';
import type { AgyAccounts, AgyLogin, AgyLoginDone, AgyReadError, ClientStatus } from '../lib/model';
import type { ChipView, FeedbackView } from '../lib/view';

/**
 * A line that names the desktop app: as written on macOS; on Windows the
 * app is Antigravity (`Antigravity.exe`), not Antigravity.app.
 */
export function desk(text: string): string {
  return onWindows() ? text.replace(/Antigravity\.app/g, 'Antigravity') : text;
}

export function agyLoginName(n: number): string {
  return 'Ultra ' + n;
}

export interface AgyRowView {
  id: string;
  /** Its place in the app's list, as the row numbers it. */
  number: number;
  name: string;
  /** The whole email, for hover; '' while names are hidden. */
  full: string;
  short: string;
  /** What the account needs, or ''. */
  sub: string;
  chip: ChipView;
  /** "Đăng nhập lại" on an account Google stopped accepting. */
  relogin: boolean;
  /** "Mở trang xác minh" when Google sent one. */
  verify: boolean;
  /** "đang đọc…" while the app reads its quota, "đọc lỗi: …" after a failed read. */
  read: { tone: 'info' | 'warn' | 'bad'; text: string } | null;
}

function statusChip(status: AgyLogin['status']): ChipView {
  switch (status) {
    case 'ok':
      return { t: t('đã đăng nhập', 'signed in'), k: 'ok' };
    case 'needsLogin':
      return { t: t('cần đăng nhập lại', 'sign in again'), k: 'bad' };
    case 'needsVerification':
      return { t: t('cần xác minh', 'needs verification'), k: 'warn' };
    case 'forbidden':
      return { t: t('Google chặn', 'blocked by Google'), k: 'bad' };
  }
}

function needs(status: AgyLogin['status']): string {
  switch (status) {
    case 'ok':
      return '';
    case 'needsLogin':
      return t('Google không nhận đăng nhập cũ nữa. Đăng nhập lại để dùng tiếp tài khoản này.', 'Google no longer accepts the old sign-in. Sign in again to keep using this account.');
    case 'needsVerification':
      return t('Google yêu cầu xác minh tài khoản này trước khi dùng tiếp.', 'Google wants this account verified before it can be used again.');
    case 'forbidden':
      return t('Google từ chối đọc hạn mức của tài khoản này (403), nên app thôi đọc nó. Đăng nhập lại để thử lại.', 'Google refused to give this account’s quota (403), so the app stopped reading it. Sign in again to retry.');
  }
}

/** Why a read failed, after "đọc lỗi: ". */
function readError(kind: string): string {
  const known: Record<string, string> = {
    network: t('không kết nối được Google', 'cannot reach Google'),
    unauthorized: t('Google không nhận đăng nhập (401)', 'Google did not accept the sign-in (401)'),
    rateLimited: t('Google báo gọi quá nhiều (429)', 'Google reports too many calls (429)'),
    server: t('máy chủ Google đang lỗi', 'Google’s servers are failing'),
    rejected: t('Google từ chối', 'Google refused'),
    verification: t('Google yêu cầu xác minh', 'Google wants verification'),
    parse: t('Google trả lời lạ', 'Google gave an unexpected answer'),
    keychain: t('không đọc được ' + secretStore(), 'cannot read the ' + secretStore()),
    client: t('OAuth client Antigravity chưa dùng được', 'the Antigravity OAuth client is not usable yet'),
    refresh: t('không làm mới được đăng nhập', 'could not refresh the sign-in'),
    cache: t('không ghi được số vào thư mục của app', 'could not write the numbers to the app’s folder'),
    index: t('không đọc được danh sách tài khoản', 'cannot read the account list'),
  };
  return known[kind] ?? t('lỗi không rõ', 'unknown error');
}

/** The kinds whose detail helps the owner tell what went wrong. */
const WITH_DETAIL = new Set(['rejected', 'parse', 'keychain', 'refresh', 'cache', 'index']);

export function agyReadLine(reading: boolean, error: AgyReadError | null): AgyRowView['read'] {
  if (reading) return { tone: 'info', text: t('đang đọc…', 'reading…') };
  if (!error) return null;
  const why = readError(error.kind) + (WITH_DETAIL.has(error.kind) && error.detail ? ' (' + error.detail + ')' : '');
  const retry = clock(error.retryAt);
  const then = retry ? t('Giữ số cũ; app đọc lại lúc ' + retry + '.', 'Keeping the old numbers; the app reads again at ' + retry + '.') : t('Giữ số cũ.', 'Keeping the old numbers.');
  return { tone: error.retryAt ? 'warn' : 'bad', text: t('lỗi đọc dữ liệu: ', 'read failed: ') + why + '. ' + then };
}

export function agyRows(view: AgyAccounts): AgyRowView[] {
  const names = agyNames(
    view.accounts.map((a) => ({ number: a.number, email: a.email ?? undefined })),
    true,
  );
  return view.accounts.map((a) => {
    const name = names.get(a.number)!;
    return {
      id: a.id,
      number: a.number,
      name: name.name,
      full: name.full,
      short: 'U' + a.number,
      sub: needs(a.status),
      chip: statusChip(a.status),
      relogin: a.status === 'needsLogin' || a.status === 'forbidden',
      verify: (a.status === 'needsVerification' || a.status === 'forbidden') && a.verify,
      read: agyReadLine(a.reading, a.readError),
    };
  });
}

const CLIENT_GUIDE = 'https://github.com/shin315/quota-station/blob/main/docs/how-it-works.md#the-oauth-client-file';

/** Where an OAuth client file goes and where the guide gives its format: the end of a notice. */
function clientFile(path: string): { vi: string; en: string } {
  return {
    vi: path + os(', quyền 600', '') + '; định dạng' + os('', ' và lệnh PowerShell tạo file') + ' có trong mục “The OAuth client file” của hướng dẫn: ' + CLIENT_GUIDE,
    en: path + os(', mode 600', '') + '; the guide’s “The OAuth client file” section gives the format' + os('', ' and the PowerShell command that makes it') + ': ' + CLIENT_GUIDE,
  };
}

/** Why "Thêm tài khoản" cannot run yet, or null when it can. */
export function clientNotice(view: AgyAccounts): { tone: 'warn' | 'bad'; title: string; body: string } | null {
  const file = clientFile(view.clientPath);
  // Not found: the manual way, as a fallback.
  const orFile = t(' Hoặc đặt OAuth client vào file ' + file.vi, ' Or put an OAuth client in ' + file.en);
  // A file is there and wins: fix it, or remove it to use the installed Antigravity's.
  const theFile = t(
    'Sửa file, hoặc xoá nó để app tìm OAuth client trong Antigravity đã cài. File ở ' + file.vi,
    'Fix the file, or remove it to let the app look for the OAuth client in the installed Antigravity. The file is at ' + file.en,
  );
  const notices: Record<Exclude<ClientStatus, 'ok'>, { tone: 'warn' | 'bad'; title: string; body: string }> = {
    missing: {
      tone: 'warn',
      title: mac(t('Chưa thấy Antigravity trên Mac này', 'Antigravity is not on this Mac')),
      body: t('App đăng nhập Google bằng OAuth client của Antigravity đã cài trên máy. Cài Antigravity (app hoặc CLI agy) rồi mở lại tab này.', 'The app signs in to Google with the OAuth client of the Antigravity installed here. Install Antigravity (the app or the agy CLI), then open this tab again.') + orFile,
    },
    unrecognized: {
      tone: 'warn',
      title: t('Chưa nhận ra bản Antigravity này', 'This Antigravity build is not recognized'),
      body: t('Antigravity đã cài không có OAuth client mà app biết. Cập nhật Antigravity và Quota Station lên bản mới nhất rồi mở lại tab này.', 'The installed Antigravity has no OAuth client the app knows. Update Antigravity and Quota Station to their latest versions, then open this tab again.') + orFile,
    },
    tooOpen: {
      tone: 'bad',
      title: t('oauth-client.json đang để người khác đọc được', 'oauth-client.json can be read by others'),
      body: t('App chỉ dùng file khi chỉ bạn đọc được nó. Chạy chmod 600 cho file rồi mở lại tab này. ', 'The app only uses the file when only you can read it. Run chmod 600 on it, then open this tab again. ') + theFile,
    },
    invalid: {
      tone: 'bad',
      title: t('oauth-client.json chưa đúng', 'oauth-client.json is not right'),
      body: t('Cần client_id (đuôi .apps.googleusercontent.com) và client_secret thật, không phải giá trị mẫu. ', 'It needs a real client_id (ending in .apps.googleusercontent.com) and client_secret, not the sample values. ') + theFile,
    },
    unreadable: {
      tone: 'bad',
      title: t('Không đọc được oauth-client.json', 'Cannot read oauth-client.json'),
      body: t('Kiểm tra xem đó có phải là một file thường mà tài khoản ' + osName() + ' của bạn đọc được không. ', 'Check that it is a regular file your ' + osName() + ' account can read. ') + theFile,
    },
  };
  return view.client === 'ok' ? null : notices[view.client];
}

export function agyLoginFeedback(done: AgyLoginDone): FeedbackView {
  const name = agyLoginName(done.number);
  return done.added
    ? {
        tone: '',
        title: t('Đã thêm ', 'Added ') + name,
        body: desk(mac(t('Đăng nhập giữ trong ' + secretStore() + ' của app trên Mac này. Antigravity.app và agy không đổi.', 'The sign-in is kept in the app’s ' + secretStore() + ' on this Mac. Antigravity.app and agy are unchanged.'))),
      }
    : { tone: '', title: t('Đã đăng nhập lại ' + name, name + ' is signed in again'), body: t('App dùng đăng nhập mới từ giờ.', 'The app uses the new sign-in from now on.') };
}

/** The kind a rejected call carries: Tauri rejects with the error value, the fake with an Error. */
export function errorKind(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === 'object' && 'kind' in e && typeof e.kind === 'string') return e.kind;
  return 'unknown';
}

function errorDetail(e: unknown): string {
  return e && typeof e === 'object' && 'detail' in e && typeof e.detail === 'string' ? e.detail : '';
}

/** What a login that kept nothing says; null when there is nothing to say. */
export function agyLoginErrorFeedback(e: unknown): FeedbackView | null {
  const kind = errorKind(e);
  const detail = errorDetail(e);
  const withDetail = (body: string) => (detail ? body + ' (' + detail + ')' : body);
  switch (kind) {
    case 'cancelled':
      return { tone: 'info', title: t('Đã huỷ đăng nhập', 'Sign-in cancelled'), body: t('Chưa thêm tài khoản nào.', 'No account was added.') };
    case 'timeout':
      return { tone: 'warn', title: t('Hết 10 phút chờ đăng nhập', 'The 10-minute wait for sign-in ran out'), body: t('Chưa thêm tài khoản. Bấm Thêm tài khoản để thử lại.', 'No account added. Press Add account to try again.') };
    case 'noCode':
      return {
        tone: 'warn',
        title: t('Google không gửi mã đăng nhập', 'Google sent no sign-in code'),
        body: t('Có thể bạn đã bấm Huỷ trên trang của Google. Chưa thêm tài khoản; thử lại khi sẵn sàng.', 'You may have pressed Cancel on Google’s page. No account added; try again when you are ready.'),
      };
    case 'stateMismatch':
      return {
        tone: 'bad',
        title: t('Đăng nhập không khớp', 'The sign-in did not match'),
        body: t('Trang Google gửi về không thuộc lần đăng nhập này, nên app bỏ qua. Bắt đầu lại từ Thêm tài khoản.', 'The page Google sent back is not from this sign-in, so the app ignored it. Start again from Add account.'),
      };
    case 'noRefreshToken':
      return {
        tone: 'bad',
        title: t('Google không trả refresh token', 'Google returned no refresh token'),
        body: t('Tài khoản này đã cấp quyền trước đó. Mở myaccount.google.com/permissions, thu hồi quyền của "Antigravity Tools", rồi đăng nhập lại.', 'This account granted access before. Open myaccount.google.com/permissions, remove access for "Antigravity Tools", then sign in again.'),
      };
    case 'invalidGrant':
      return { tone: 'bad', title: t('Google từ chối mã đăng nhập', 'Google refused the sign-in code'), body: t('Mã đã hết hạn hoặc đã được dùng. Bắt đầu lại từ Thêm tài khoản.', 'The code expired or was already used. Start again from Add account.') };
    case 'network':
      return { tone: 'warn', title: t('Không kết nối được Google', 'Cannot reach Google'), body: withDetail(t('Chưa thêm tài khoản. Kiểm tra mạng rồi thử lại.', 'No account added. Check the network, then try again.')) };
    case 'exchange':
      return { tone: 'bad', title: t('Google từ chối đăng nhập', 'Google refused the sign-in'), body: withDetail(t('Chưa thêm tài khoản.', 'No account added.')) };
    case 'userInfo':
      return { tone: 'bad', title: t('Không lấy được email của tài khoản', 'Could not get the account’s email'), body: withDetail(t('Chưa thêm tài khoản.', 'No account added.')) };
    case 'keychain':
      return {
        tone: 'bad',
        title: t('Không lưu được vào ' + secretStore(), 'Could not save to the ' + secretStore()),
        body: withDetail(
          os(
            t('Chưa thêm tài khoản và danh sách giữ nguyên. Mở khoá Keychain rồi thử lại.', 'No account added and the list is unchanged. Unlock the Keychain, then try again.'),
            t('Chưa thêm tài khoản và danh sách giữ nguyên. Thử lại.', 'No account added and the list is unchanged. Try again.'),
          ),
        ),
      };
    case 'index':
      return { tone: 'bad', title: t('Không ghi được danh sách tài khoản', 'Could not write the account list'), body: withDetail(t('Đăng nhập chưa được giữ.', 'The sign-in was not kept.')) };
    case 'client':
      return { tone: 'warn', title: t('OAuth client Antigravity chưa dùng được', 'The Antigravity OAuth client is not usable yet'), body: t('Xem thẻ Đăng nhập trong Quota Station.', 'See the Sign-in card in Quota Station.') };
    case 'browser':
      return { tone: 'bad', title: t('Không mở được trình duyệt', 'Could not open the browser'), body: t('Chưa thêm tài khoản. Thử lại.', 'No account added. Try again.') };
    case 'bind':
      return { tone: 'bad', title: t('Không mở được cổng cục bộ cho đăng nhập', 'Could not open a local port for the sign-in'), body: withDetail(t('Chưa thêm tài khoản.', 'No account added.')) };
    case 'busy':
      return { tone: 'info', title: t('Đang có một lần đăng nhập', 'A sign-in is already running'), body: t('Xong lần đó, hoặc bấm Huỷ, rồi thử lại.', 'Finish that one, or press Cancel, then try again.') };
    case 'switching':
      return { tone: 'info', title: t('Đang chuyển Antigravity', 'Switching Antigravity'), body: t('Chờ lần chuyển xong rồi đăng nhập.', 'Wait for the switch to finish, then sign in.') };
    case 'loggingOut':
      return { tone: 'info', title: t('Đang đăng xuất một tài khoản', 'Signing an account out'), body: t('Chờ lần đăng xuất xong rồi đăng nhập.', 'Wait for the sign-out to finish, then sign in.') };
    default:
      return { tone: 'bad', title: t('Chưa đăng nhập được', 'Sign-in did not finish'), body: withDetail(t('Chưa thêm tài khoản.', 'No account added.')) };
  }
}

/** The hint under the pasted-URL field when the URL is refused. */
export function pasteHint(e: unknown): string {
  switch (errorKind(e)) {
    case 'stateMismatch':
      return t('URL này không thuộc lần đăng nhập đang chờ.', 'This URL is not from the sign-in that is waiting.');
    case 'noCode':
      return t('URL không có mã đăng nhập (code=…).', 'The URL has no sign-in code (code=…).');
    case 'noState':
      return t('URL thiếu state=…. Chép cả dòng trên thanh địa chỉ.', 'The URL is missing state=…. Copy the whole line from the address bar.');
    case 'noLogin':
      return t('Không còn lần đăng nhập nào đang chờ.', 'No sign-in is waiting any more.');
    default:
      return t('Không phải URL callback (…/oauth-callback?state=…&code=…).', 'Not a callback URL (…/oauth-callback?state=…&code=…).');
  }
}

export function agyLogoutFeedback(name: string): FeedbackView {
  return { tone: '', title: t('Đã đăng xuất ', 'Signed out ') + name, body: desk(t('Đã xoá đăng nhập của app. Antigravity.app và agy vẫn đăng nhập như cũ.', 'Removed the app’s sign-in. Antigravity.app and agy stay signed in as before.')) };
}

/** Why "Mở trang xác minh" opened nothing. */
export function agyVerifyErrorFeedback(name: string, e: unknown): FeedbackView {
  return errorKind(e) === 'browser'
    ? { tone: 'bad', title: t('Không mở được trình duyệt', 'Could not open the browser'), body: t('Trang xác minh của ' + name + ' chưa mở. Thử lại.', name + '’s verification page did not open. Try again.') }
    : {
        tone: 'warn',
        title: t('Không còn trang xác minh cho ', 'No verification page left for ') + name,
        body: t('Trạng thái của tài khoản đã đổi, hoặc trang Google gửi không phải trang của Google. App không mở trang nào.', 'The account’s state changed, or the page sent is not a Google page. The app opened nothing.'),
      };
}

export function agyLogoutErrorFeedback(name: string, e: unknown): FeedbackView {
  const kind = errorKind(e);
  return {
    tone: 'bad',
    title: t('Chưa đăng xuất được ', 'Could not sign out ') + name,
    body:
      kind === 'keychain'
        ? os(
            t('Keychain không xoá được mục của app; tài khoản vẫn trong danh sách. Mở khoá Keychain rồi thử lại.', 'The Keychain could not remove the app’s item; the account stays in the list. Unlock the Keychain, then try again.'),
            t('Không xoá được đăng nhập của app trong kho mã hoá; tài khoản vẫn trong danh sách. Thử lại.', 'Could not remove the app’s sign-in from its encrypted store; the account stays in the list. Try again.'),
          )
        : kind === 'notFound'
          ? t('Tài khoản không còn trong danh sách.', 'The account is no longer in the list.')
          : kind === 'busy'
            ? t('Một lần chuyển Antigravity đang chạy. Chờ nó xong rồi thử lại.', 'An Antigravity switch is running. Wait for it to finish, then try again.')
            : kind === 'loggingOut'
              ? t('Một lần đăng xuất khác đang chạy. Chờ nó xong rồi thử lại.', 'Another sign-out is running. Wait for it to finish, then try again.')
              : kind === 'loggingIn'
                ? t('Một lần đăng nhập đang chờ. Xong lần đó, hoặc bấm Huỷ, rồi thử lại.', 'A sign-in is waiting. Finish it, or press Cancel, then try again.')
                : t('Không ghi được danh sách tài khoản. Thử lại.', 'Could not write the account list. Try again.'),
  };
}
