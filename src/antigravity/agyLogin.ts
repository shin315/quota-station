// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// The words of the Antigravity login card (M4a): the accounts this app logs
// in, what is missing before it can, what each login or logout did, and how
// the app's own read of each account's quota stands (M4b). Accounts go by
// their email, as on the quota rows, or by number ("Ultra 1") when the
// owner hides names in Cài đặt.

import { agyNames } from '../lib/accounts';
import { clock } from '../lib/format';
import type { AgyAccounts, AgyLogin, AgyLoginDone, AgyReadError, ClientStatus } from '../lib/model';
import type { ChipView, FeedbackView } from '../lib/view';

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

const CHIPS: Record<AgyLogin['status'], ChipView> = {
  ok: { t: 'đã đăng nhập', k: 'ok' },
  needsLogin: { t: 'cần đăng nhập lại', k: 'bad' },
  needsVerification: { t: 'cần xác minh', k: 'warn' },
  forbidden: { t: 'Google chặn', k: 'bad' },
};

const NEEDS: Record<AgyLogin['status'], string> = {
  ok: '',
  needsLogin: 'Google không nhận đăng nhập cũ nữa. Đăng nhập lại để dùng tiếp tài khoản này.',
  needsVerification: 'Google yêu cầu xác minh tài khoản này trước khi dùng tiếp.',
  forbidden: 'Google từ chối đọc hạn mức của tài khoản này (403), nên app thôi đọc nó. Đăng nhập lại để thử lại.',
};

/** Why a read failed, after "đọc lỗi: ". */
const READ_ERRORS: Record<string, string> = {
  network: 'không kết nối được Google',
  unauthorized: 'Google không nhận đăng nhập (401)',
  rateLimited: 'Google báo gọi quá nhiều (429)',
  server: 'máy chủ Google đang lỗi',
  rejected: 'Google từ chối',
  verification: 'Google yêu cầu xác minh',
  parse: 'Google trả lời lạ',
  keychain: 'không đọc được Keychain',
  client: 'oauth-client.json chưa dùng được',
  refresh: 'không làm mới được đăng nhập',
  cache: 'không ghi được số vào thư mục của app',
  index: 'không đọc được danh sách tài khoản',
};

/** The kinds whose detail helps the owner tell what went wrong. */
const WITH_DETAIL = new Set(['rejected', 'parse', 'keychain', 'refresh', 'cache', 'index']);

export function agyReadLine(reading: boolean, error: AgyReadError | null): AgyRowView['read'] {
  if (reading) return { tone: 'info', text: 'đang đọc…' };
  if (!error) return null;
  const why = (READ_ERRORS[error.kind] ?? 'lỗi không rõ') + (WITH_DETAIL.has(error.kind) && error.detail ? ' (' + error.detail + ')' : '');
  const retry = clock(error.retryAt);
  const then = retry ? 'Giữ số cũ; app đọc lại lúc ' + retry + '.' : 'Giữ số cũ.';
  return { tone: error.retryAt ? 'warn' : 'bad', text: 'đọc lỗi: ' + why + '. ' + then };
}

export function agyRows(view: AgyAccounts): AgyRowView[] {
  const names = agyNames(view.accounts.map((a) => ({ number: a.number, email: a.email ?? undefined })));
  return view.accounts.map((a) => {
    const name = names.get(a.number)!;
    return {
      id: a.id,
      number: a.number,
      name: name.name,
      full: name.full,
      short: 'U' + a.number,
      sub: NEEDS[a.status],
      chip: CHIPS[a.status],
      relogin: a.status === 'needsLogin' || a.status === 'forbidden',
      verify: (a.status === 'needsVerification' || a.status === 'forbidden') && a.verify,
      read: agyReadLine(a.reading, a.readError),
    };
  });
}

/** Why "Thêm tài khoản" cannot run yet, or null when it can. */
export function clientNotice(view: AgyAccounts): { tone: 'warn' | 'bad'; title: string; body: string } | null {
  const where = 'File đặt ở ' + view.clientPath + ', quyền 600. README ghi định dạng.';
  const notices: Record<Exclude<ClientStatus, 'ok'>, { tone: 'warn' | 'bad'; title: string; body: string }> = {
    missing: {
      tone: 'warn',
      title: 'Chưa có oauth-client.json',
      body:
        'App đăng nhập Google bằng OAuth client của Antigravity, như Cockpit. Mã client không nằm trong mã nguồn, nên cần đặt nó vào file oauth-client.json trên máy này. ' +
        where,
    },
    tooOpen: {
      tone: 'bad',
      title: 'oauth-client.json đang để người khác đọc được',
      body: 'App chỉ dùng file khi chỉ bạn đọc được nó. Chạy chmod 600 cho file rồi mở lại tab này.',
    },
    invalid: {
      tone: 'bad',
      title: 'oauth-client.json chưa đúng',
      body: 'Cần client_id (đuôi .apps.googleusercontent.com) và client_secret thật, không phải giá trị mẫu. ' + where,
    },
    unreadable: {
      tone: 'bad',
      title: 'Không đọc được oauth-client.json',
      body: 'Kiểm tra xem đó có phải là một file thường mà tài khoản macOS của bạn đọc được không. ' + where,
    },
  };
  return view.client === 'ok' ? null : notices[view.client];
}

export function agyLoginFeedback(done: AgyLoginDone): FeedbackView {
  const name = agyLoginName(done.number);
  return done.added
    ? { tone: '', title: 'Đã thêm ' + name, body: 'Đăng nhập giữ trong Keychain của app trên Mac này. Antigravity.app và agy không đổi.' }
    : { tone: '', title: 'Đã đăng nhập lại ' + name, body: 'App dùng đăng nhập mới từ giờ.' };
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
      return { tone: 'info', title: 'Đã huỷ đăng nhập', body: 'Chưa thêm tài khoản nào.' };
    case 'timeout':
      return { tone: 'warn', title: 'Hết 10 phút chờ đăng nhập', body: 'Chưa thêm tài khoản. Bấm Thêm tài khoản để thử lại.' };
    case 'noCode':
      return {
        tone: 'warn',
        title: 'Google không gửi mã đăng nhập',
        body: 'Có thể bạn đã bấm Huỷ trên trang của Google. Chưa thêm tài khoản; thử lại khi sẵn sàng.',
      };
    case 'stateMismatch':
      return {
        tone: 'bad',
        title: 'Đăng nhập không khớp',
        body: 'Trang Google gửi về không thuộc lần đăng nhập này, nên app bỏ qua. Bắt đầu lại từ Thêm tài khoản.',
      };
    case 'noRefreshToken':
      return {
        tone: 'bad',
        title: 'Google không trả refresh token',
        body: 'Tài khoản này đã cấp quyền trước đó. Mở myaccount.google.com/permissions, thu hồi quyền của "Antigravity Tools", rồi đăng nhập lại.',
      };
    case 'invalidGrant':
      return { tone: 'bad', title: 'Google từ chối mã đăng nhập', body: 'Mã đã hết hạn hoặc đã được dùng. Bắt đầu lại từ Thêm tài khoản.' };
    case 'network':
      return { tone: 'warn', title: 'Không kết nối được Google', body: withDetail('Chưa thêm tài khoản. Kiểm tra mạng rồi thử lại.') };
    case 'exchange':
      return { tone: 'bad', title: 'Google từ chối đăng nhập', body: withDetail('Chưa thêm tài khoản.') };
    case 'userInfo':
      return { tone: 'bad', title: 'Không lấy được email của tài khoản', body: withDetail('Chưa thêm tài khoản.') };
    case 'keychain':
      return {
        tone: 'bad',
        title: 'Không lưu được vào Keychain',
        body: withDetail('Chưa thêm tài khoản và danh sách giữ nguyên. Mở khoá Keychain rồi thử lại.'),
      };
    case 'index':
      return { tone: 'bad', title: 'Không ghi được danh sách tài khoản', body: withDetail('Đăng nhập chưa được giữ.') };
    case 'client':
      return { tone: 'warn', title: 'Chưa dùng được oauth-client.json', body: 'Xem hướng dẫn trong thẻ Đăng nhập trong Quota Station.' };
    case 'browser':
      return { tone: 'bad', title: 'Không mở được trình duyệt', body: 'Chưa thêm tài khoản. Thử lại.' };
    case 'bind':
      return { tone: 'bad', title: 'Không mở được cổng cục bộ cho đăng nhập', body: withDetail('Chưa thêm tài khoản.') };
    case 'busy':
      return { tone: 'info', title: 'Đang có một lần đăng nhập', body: 'Xong lần đó, hoặc bấm Huỷ, rồi thử lại.' };
    case 'switching':
      return { tone: 'info', title: 'Đang chuyển Antigravity', body: 'Chờ lần chuyển xong rồi đăng nhập.' };
    case 'loggingOut':
      return { tone: 'info', title: 'Đang đăng xuất một tài khoản', body: 'Chờ lần đăng xuất xong rồi đăng nhập.' };
    default:
      return { tone: 'bad', title: 'Chưa đăng nhập được', body: withDetail('Chưa thêm tài khoản.') };
  }
}

/** The hint under the pasted-URL field when the URL is refused. */
export function pasteHint(e: unknown): string {
  switch (errorKind(e)) {
    case 'stateMismatch':
      return 'URL này không thuộc lần đăng nhập đang chờ.';
    case 'noCode':
      return 'URL không có mã đăng nhập (code=…).';
    case 'noState':
      return 'URL thiếu state=…. Chép cả dòng trên thanh địa chỉ.';
    case 'noLogin':
      return 'Không còn lần đăng nhập nào đang chờ.';
    default:
      return 'Không phải URL callback (…/oauth-callback?state=…&code=…).';
  }
}

export function agyLogoutFeedback(name: string): FeedbackView {
  return { tone: '', title: 'Đã đăng xuất ' + name, body: 'Đã xoá đăng nhập của app. Antigravity.app và agy vẫn đăng nhập như cũ.' };
}

/** Why "Mở trang xác minh" opened nothing. */
export function agyVerifyErrorFeedback(name: string, e: unknown): FeedbackView {
  return errorKind(e) === 'browser'
    ? { tone: 'bad', title: 'Không mở được trình duyệt', body: 'Trang xác minh của ' + name + ' chưa mở. Thử lại.' }
    : {
        tone: 'warn',
        title: 'Không còn trang xác minh cho ' + name,
        body: 'Trạng thái của tài khoản đã đổi, hoặc trang Google gửi không phải trang của Google. App không mở trang nào.',
      };
}

export function agyLogoutErrorFeedback(name: string, e: unknown): FeedbackView {
  const kind = errorKind(e);
  return {
    tone: 'bad',
    title: 'Chưa đăng xuất được ' + name,
    body:
      kind === 'keychain'
        ? 'Keychain không xoá được mục của app; tài khoản vẫn trong danh sách. Mở khoá Keychain rồi thử lại.'
        : kind === 'notFound'
          ? 'Tài khoản không còn trong danh sách.'
          : kind === 'busy'
            ? 'Một lần chuyển Antigravity đang chạy. Chờ nó xong rồi thử lại.'
            : kind === 'loggingOut'
              ? 'Một lần đăng xuất khác đang chạy. Chờ nó xong rồi thử lại.'
              : kind === 'loggingIn'
                ? 'Một lần đăng nhập đang chờ. Xong lần đó, hoặc bấm Huỷ, rồi thử lại.'
                : 'Không ghi được danh sách tài khoản. Thử lại.',
  };
}
