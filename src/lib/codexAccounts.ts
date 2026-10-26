// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Codex accounts (the ChatGPT sign-ins the app keeps and the custom
// providers in config.toml) as the Codex tab, Tổng quan and the popover show
// them, the sheet that switches to one, and the lines after an add, a rename
// or a removal. Pure: the same model gives the same words.

import { cellOf, emailNames, type CellView } from './accounts';
import { ccSwitchManages } from './codex';
import { age, clock, secondsSince } from './format';
import { t } from './i18n';
import type { CodexAccount, CodexAccountDone, CodexLoginFail, CodexProviderCheck, CodexView, Model } from './model';
import type { FactView } from './switching';
import type { ChipView, FeedbackView } from './view';

/** One account in a table or the popover. */
export interface CodexAccountRowView {
  key: string;
  id: string;
  kind: CodexAccount['kind'];
  name: string;
  sub: string;
  /** The whole email or address, for hover. */
  full: string;
  chips: ChipView[];
  /** 5 giờ · Tuần of a ChatGPT account; null for a custom provider, which reports no quota. */
  cells: CellView[] | null;
  active: boolean;
  /** Its sign-in is the one in the Codex folder while Codex goes through a custom provider. */
  kept: boolean;
  /** Old numbers: drawn muted. */
  dim: boolean;
  /** ChatGPT refused its sign-in: "Đăng nhập lại…" stands where "Chuyển" would. */
  relogin: boolean;
  /** Why "Chuyển" cannot run now; '' when it can. */
  off: string;
  label: string;
}

/** `chatgpt:2` → 2. */
export function chatGptNumber(id: string): number | null {
  return id.startsWith('chatgpt:') ? Number(id.slice('chatgpt:'.length)) : null;
}

export function accountOf(v: CodexView, id: string): CodexAccount | undefined {
  return v.accounts.find((a) => a.id === id);
}

/** The account's name as the table shows it. */
export function accountName(model: Model, a: CodexAccount): string {
  if (a.nickname || a.kind === 'custom') return a.name;
  const names = emailNames(model.codex.accounts.map((x) => x.email ?? undefined));
  return (a.email && names.get(a.email)) || a.name;
}

function host(url: string | null): string {
  if (!url) return t('không có địa chỉ', 'no address');
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

function reachWords(a: CodexAccount): string {
  const r = a.reach;
  if (!r || r.state === 'unknown') return a.custom?.baseUrl ? t('đang kiểm…', 'checking…') : '';
  if (r.state === 'up') return t('trả lời ' + r.ms + ' ms', 'answers in ' + r.ms + ' ms');
  return t('không trả lời từ ', 'not answering since ') + clock(r.since);
}

function customChips(a: CodexAccount): ChipView[] {
  const c = a.custom;
  if (!c) return [];
  const chips: ChipView[] = [];
  if (c.key === 'placeholder') chips.push({ t: t('chỉ khoá giữ chỗ', 'placeholder key only'), k: 'bad' });
  if (c.key === 'none') chips.push({ t: t('chưa có khoá', 'no key yet'), k: 'bad' });
  if (c.proxy) chips.push({ t: t('qua proxy CC Switch', 'through the CC Switch proxy'), k: 'warn' });
  if (a.reach?.state === 'down') chips.push({ t: t('không trả lời', 'not answering'), k: 'bad' });
  return chips;
}

/** Why "Chuyển" to `a` cannot run now. */
function offText(v: CodexView, a: CodexAccount): string {
  if (a.active) return '';
  if (v.busy) return t('Codex đang chuyển', 'Codex is switching');
  if (a.kind === 'chatgpt' && a.login === 'relogin') return t('Cần đăng nhập lại trước', 'Needs a new sign-in first');
  if (a.refusal) return a.refusal.kind === 'noKey' ? t('Chưa có khoá API', 'No API key yet') : a.refusal.kind === 'placeholder' ? t('Chỉ có khoá giữ chỗ', 'Only a placeholder key') : t('Chưa chuyển được', 'Cannot switch yet');
  return '';
}

/** Every account, ChatGPT sign-ins first, each with its windows or its provider line. */
export function accountRows(model: Model, now: Date): CodexAccountRowView[] {
  const v = model.codex;
  if (!v.present) return [];
  const quota = model.snapshot.codex.accounts;
  return v.accounts.map((a) => {
    const name = accountName(model, a);
    const chips: ChipView[] = [];
    let cells: CellView[] | null = null;
    let sub: string;
    let full: string;
    let dim = false;
    const kept = a.kind === 'chatgpt' && a.signedIn && !a.active;
    if (a.kind === 'chatgpt') {
      const q = quota.find((x) => x.number === a.number);
      const pick = (kind: 'session' | 'week') => q?.windows.find((l) => l.kind === kind) ?? q?.lines.find((l) => l.kind === kind);
      cells = (['session', 'week'] as const).map((k) => cellOf(pick(k), k, now, '—'));
      const since = secondsSince(q?.fetchedAt ?? null, now);
      if (a.login === 'relogin') chips.push({ t: t('cần đăng nhập lại', 'sign in again'), k: 'bad', title: t('ChatGPT từ chối đăng nhập này. Đăng nhập lại để đọc hạn mức và chuyển sang.', 'ChatGPT refused this sign-in. Sign in again to read its quota and switch to it.') });
      else if (a.login === 'idle')
        chips.push({
          t: since != null ? t('số ' + age(since) + ' trước', 'numbers ' + age(since) + ' old') : t('số cũ', 'old numbers'),
          k: 'stale',
          title: t(
            'Đăng nhập của tài khoản này đã quá hạn đọc khi không dùng. Codex làm mới nó khi bạn chuyển sang; số hiện là lần đọc cuối.',
            'This account’s sign-in ran out while it was not in use. Codex renews it when you switch to it; the numbers are its last reading.',
          ),
        });
      else if (q?.reading === 'resetPending') chips.push({ t: t('chờ số sau mốc đặt lại', 'waiting for the number after its reset'), k: 'info' });
      else if (q?.reading === 'stale' && since != null) chips.push({ t: t('số ' + age(since) + ' trước', 'numbers ' + age(since) + ' old'), k: 'stale' });
      dim = a.login !== 'ok' || q?.reading === 'stale';
      const plan = a.plan ? 'ChatGPT ' + a.plan : 'ChatGPT';
      sub = kept ? plan + t(' · plugin dùng đăng nhập này', ' · plugins use this sign-in') : plan;
      full = a.email ?? '';
    } else {
      chips.push(...customChips(a));
      sub = ['Custom API', host(a.custom?.baseUrl ?? null), reachWords(a)].filter(Boolean).join(' · ');
      full = a.custom?.baseUrl ?? '';
    }
    const off = offText(v, a);
    return {
      key: a.id,
      id: a.id,
      kind: a.kind,
      name,
      sub,
      full,
      chips,
      cells,
      active: a.active,
      kept,
      dim,
      relogin: a.kind === 'chatgpt' && a.login === 'relogin',
      off,
      label:
        name +
        ', ' +
        sub +
        (a.active ? t(', đang dùng', ', in use') : '') +
        '. ' +
        (cells ? cells.map((c) => c.words).join('. ') : t('Không báo hạn mức', 'Reports no quota')) +
        (chips.length ? '. ' + chips.map((c) => c.t).join(', ') : '') +
        (off ? '. ' + off : ''),
    };
  });
}

/** The account Codex goes through now. */
export function activeAccount(v: CodexView): CodexAccount | undefined {
  return v.accounts.find((a) => a.active);
}

/** The ChatGPT sign-in in the Codex folder. */
export function signedInAccount(v: CodexView): CodexAccount | undefined {
  return v.accounts.find((a) => a.kind === 'chatgpt' && a.signedIn);
}

/** `3 phiên codex`, bold, or without a count when it is not known. */
function sessions(n: number | null | undefined): { vi: string; en: string } {
  if (n == null) return { vi: 'các phiên codex', en: 'codex sessions' };
  return { vi: '**' + n + ' phiên codex**', en: '**' + n + (n === 1 ? ' codex session**' : ' codex sessions**') };
}

export interface AccountSheetView {
  title: string;
  lead: string;
  facts: FactView[];
  confirm: string;
  blocked: boolean;
  /** CC Switch also switches Codex: the guide to stop using it goes on the sheet. */
  ccSwitch: boolean;
}

/**
 * The sheet before Codex moves to another ChatGPT account: what moves
 * (two `auth.json` files, by name only), what the running sessions do, and
 * the restart, which this kind of switch needs.
 */
export function accountSheet(model: Model, id: string): AccountSheetView | null {
  const v = model.codex;
  const to = accountOf(v, id);
  if (!to || to.kind !== 'chatgpt') return null;
  const toName = accountName(model, to);
  const cur = activeAccount(v);
  const curName = cur ? accountName(model, cur) : t('tài khoản hiện tại', 'the current account');
  const inFolder = signedInAccount(v);
  const inName = inFolder ? accountName(model, inFolder) : null;
  const facts: FactView[] = [];
  const cc = ccSwitchManages(v);
  if (cc)
    facts.push({
      k: 'w',
      text: t(
        '**CC Switch vẫn đang quản lý Codex** trên máy này. Lần chuyển sau của CC Switch sẽ ghi đè tài khoản chọn ở đây. Chỉ chuyển Codex ở một nơi.',
        '**CC Switch still manages Codex** on this Mac. Its next switch will overwrite the account chosen here. Switch Codex in one place only.',
      ),
    });
  if (to.login === 'relogin')
    facts.push({ k: 'x', text: t('ChatGPT đã từ chối đăng nhập của ' + toName + '. Đăng nhập lại trước khi chuyển.', 'ChatGPT refused ' + toName + '’s sign-in. Sign in again before switching.') });
  const here = inFolder?.id === to.id;
  facts.push({
    k: 'ok',
    text: here
      ? t('Đăng nhập của ' + toName + ' đã nằm trong `' + v.home + '`: không dời file nào.', toName + '’s sign-in is already in `' + v.home + '`: no file moves.')
      : inName
        ? t(
            'App dời hai file `auth.json`: của ' + inName + ' về thư mục riêng, của ' + toName + ' vào `' + v.home + '`. Không đọc hay chép token.',
            'The app moves two `auth.json` files: ' + inName + '’s back to its own folder, ' + toName + '’s into `' + v.home + '`. No token is read or copied.',
          )
        : t('App dời file `auth.json` của ' + toName + ' vào `' + v.home + '`. Không đọc hay chép token.', 'The app moves ' + toName + '’s `auth.json` into `' + v.home + '`. No token is read or copied.'),
  });
  const viaCustom = v.current === 'custom' && cur != null;
  if (viaCustom)
    facts.push({
      k: '',
      text: t(
        'Bỏ dòng `model_provider` trong `config.toml` (sao lưu trước); bảng ' + curName + ' vẫn giữ.',
        'The `model_provider` line goes from `config.toml` (backed up first); the ' + curName + ' table stays.',
      ),
    });
  const s = sessions(v.procs?.sessions);
  const reopen = t(
    ' Thoát hẳn Codex App (⌘Q) rồi mở lại; ' + s.vi + ' trong Terminal mở lại bằng `codex resume`.',
    ' Quit Codex App (⌘Q) and open it again; reopen ' + s.en + ' in Terminal with `codex resume`.',
  );
  facts.push({
    k: 'w',
    text: viaCustom
      ? t('Phiên đang chạy vẫn đi qua ' + curName + ' tới khi mở lại.', 'Running sessions keep going through ' + curName + ' until reopened.') + reopen
      : t(
          'Phiên đang chạy vẫn dùng ' + (inName ?? curName) + ' tới khi mở lại, và đòi đăng nhập lại ở lần làm mới đăng nhập kế tiếp.',
          'Running sessions keep ' + (inName ?? curName) + ' until reopened, and ask to sign in again at their next sign-in renewal.',
        ) + reopen,
  });
  const blocked = to.login === 'relogin' || v.busy != null;
  return {
    title: t('Chuyển Codex sang ' + toName + '?', 'Switch Codex to ' + toName + '?'),
    lead: t(
      'Codex dùng tài khoản ChatGPT ' + toName + (to.plan ? ' (' + to.plan + ')' : '') + ' thay cho ' + curName + '.',
      'Codex uses the ChatGPT account ' + toName + (to.plan ? ' (' + to.plan + ')' : '') + ' instead of ' + curName + '.',
    ),
    facts,
    confirm: cc ? t('Vẫn chuyển', 'Switch anyway') : t('Chuyển sang ' + toName, 'Switch to ' + toName),
    blocked,
    ccSwitch: cc,
  };
}

/** Why a sign-in ended with nothing added. */
export function loginFailText(reason: CodexLoginFail, detail: string | null): string {
  switch (reason) {
    case 'timeout':
      return t('Hết 10 phút mà trình duyệt chưa đăng nhập xong.', 'The browser did not finish signing in within 10 minutes.');
    case 'port':
      return t(
        'Cổng đăng nhập của Codex (1455) đang bận: có thể một lần đăng nhập Codex khác đang mở. Đóng nó rồi thử lại, hoặc dùng mã thiết bị.',
        'Codex’s sign-in port (1455) is busy: another Codex sign-in may be open. Close it and try again, or use a device code.',
      );
    case 'noCodex':
      return t('Không thấy lệnh codex trên máy này. Cài Codex CLI rồi thử lại.', 'The codex command was not found on this Mac. Install the Codex CLI and try again.');
    case 'denied':
      return t('Trang đăng nhập báo không cho phép (tài khoản không được dùng Codex, hoặc bạn đã bấm Huỷ).', 'The sign-in page said no (the account cannot use Codex, or you pressed Cancel).');
    case 'cancelled':
      return t('Đã dừng đăng nhập.', 'The sign-in was stopped.');
    default:
      return t('Codex báo lỗi khi đăng nhập', 'Codex reported an error while signing in') + (detail ? ': ' + detail : '.');
  }
}

/** The line after an account action that ends in the tab. */
export function accountFeedback(done: CodexAccountDone, model: Model): FeedbackView | null {
  const v = model.codex;
  const nameOf = (id: string) => {
    const a = accountOf(v, id);
    return a ? accountName(model, a) : id;
  };
  switch (done.kind) {
    case 'renamed':
      return { tone: 'info', title: t('Đã đổi tên gọi thành ' + nameOf(done.id) + '.', 'Renamed to ' + nameOf(done.id) + '.'), body: '' };
    case 'removed':
      return {
        tone: 'info',
        title: t('Đã xoá ' + done.name + ' khỏi máy này.', 'Removed ' + done.name + ' from this Mac.'),
        body: done.loggedOut ? t('Đăng nhập của nó đã được thu hồi.', 'Its sign-in was revoked.') : '',
      };
    case 'renewed':
      return { tone: 'info', title: t('Đã đăng nhập lại ' + nameOf(done.id) + '.', 'Signed ' + nameOf(done.id) + ' in again.'), body: '' };
    case 'refused':
      return { tone: 'bad', title: refusedText(done.reason), body: '' };
    default:
      return null;
  }
}

export function refusedText(reason: Extract<CodexAccountDone, { kind: 'refused' }>['reason']): string {
  switch (reason) {
    case 'busy':
      return t('Codex đang chuyển hoặc đang đăng nhập; thử lại sau ít giây.', 'Codex is switching or signing in; try again in a moment.');
    case 'inUse':
      return t('Tài khoản đang dùng: chuyển sang tài khoản khác trước.', 'The account is in use: switch to another one first.');
    case 'signedIn':
      return t('Đăng nhập này đang nằm trong Codex (plugin dùng nó): chuyển sang tài khoản ChatGPT khác trước.', 'This sign-in sits in Codex (kept for plugins): switch to another ChatGPT account first.');
    case 'changed':
      return t('config.toml vừa đổi ngoài app; mở lại để xem bản mới.', 'config.toml just changed outside the app; open it again to see the new one.');
    case 'badUrl':
      return t('Địa chỉ phải bắt đầu bằng http:// hoặc https://.', 'The address must start with http:// or https://.');
    case 'name':
      return t('Tên này đã có, hoặc trùng tên Codex giữ riêng (openai, ollama, lmstudio).', 'This name is taken, or is one Codex reserves (openai, ollama, lmstudio).');
    case 'empty':
      return t('Cần tên, địa chỉ và khoá.', 'A name, an address and a key are needed.');
    case 'key':
      return t('Khoá này không dùng được (trống, có khoảng trắng, hoặc quá dài).', 'This key cannot be used (empty, with spaces, or too long).');
  }
}

/** The result of "Kiểm tra" on a new provider, in one line and a tone. */
export function checkText(c: CodexProviderCheck): { tone: '' | 'ok' | 'bad' | 'warn'; text: string } {
  if (c.reach === 'badUrl') return { tone: 'bad', text: t('Địa chỉ không hợp lệ.', 'The address is not valid.') };
  if (c.reach === 'down')
    return { tone: 'bad', text: t('Không trả lời sau 3 lần thử (5 giây mỗi lần). Kiểm lại địa chỉ, hoặc nhà cung cấp đang ngừng.', 'No answer after 3 tries (5 seconds each). Check the address, or the provider is down.') };
  if (c.key === 'rejected')
    return { tone: 'bad', text: t('Địa chỉ trả lời ' + c.ms + ' ms nhưng khoá bị từ chối (' + c.status + '). Kiểm lại khoá.', 'The address answers in ' + c.ms + ' ms but the key was refused (' + c.status + '). Check the key.') };
  if (c.key === 'accepted') return { tone: 'ok', text: t('Trả lời sau ' + c.ms + ' ms · khoá dùng được.', 'Answers in ' + c.ms + ' ms · the key was accepted.') };
  return { tone: 'warn', text: t('Trả lời ' + c.ms + ' ms · chưa biết khoá có đúng không (nhà cung cấp không báo).', 'Answers in ' + c.ms + ' ms · cannot tell whether the key is right (the provider does not say).') };
}
