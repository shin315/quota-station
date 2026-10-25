// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What switching Codex shows (M8): the popover's Codex row and its banner,
// the confirmation sheet's five facts and the sheets around it, the
// feedback after a switch or a restart, and the Codex tab. Pure, like
// `agySwitch.ts`: the same model gives the same words. Copy follows the
// approved prototype (report section 5): the custom provider goes by the
// name its table in config.toml gives it, never a name built in here.

import { cellOf, emailNames, type CellView } from './accounts';
import { age, clock, secondsSince } from './format';
import { t } from './i18n';
import type { CodexKeyOutcome, CodexKeyPreview, CodexKeyRefusal, CodexOutcome, CodexRefusal, CodexRestart, CodexTarget, CodexView, Model } from './model';
import type { FactView } from './switching';
import type { Action, BannerView, ChipView, FeedbackView, NoteView } from './view';

/** The id Codex files the ChatGPT login under. */
export const CHATGPT_ID = 'openai';

/** Short, for chips and arrows: `ChatGPT`, or the provider's name. */
export function shortName(v: CodexView, id: string): string {
  if (id === CHATGPT_ID) return 'ChatGPT';
  if (v.custom && id === v.custom.id) return v.custom.name;
  return id || t('không rõ', 'unknown');
}

/** In a sentence: `tài khoản ChatGPT`, or the provider's name. */
export function longName(v: CodexView, id: string): string {
  return id === CHATGPT_ID ? t('tài khoản ChatGPT', 'the ChatGPT account') : shortName(v, id);
}

/** The provider id in use now. */
export function currentId(v: CodexView): string {
  if (v.current === 'custom' && v.custom) return v.custom.id;
  if (v.current === 'other' && v.other) return v.other;
  return CHATGPT_ID;
}

export function targetId(v: CodexView, target: CodexTarget): string {
  return target === 'custom' ? (v.custom?.id ?? 'custom') : CHATGPT_ID;
}

/** Where the row's arrow points: away from the provider in use. */
export function nextTarget(v: CodexView): CodexTarget {
  return v.current === 'custom' ? 'chatGpt' : v.custom ? 'custom' : 'chatGpt';
}

/** Codex has a ChatGPT login the app can see, or one kept in the Keychain. */
export function hasLogin(v: CodexView): boolean {
  return v.login === 'chatGpt' || v.login === 'keychain';
}

/** Something other than the app may write the file next. */
export function ccSwitchManages(v: CodexView): boolean {
  return v.ccSwitch.running && (v.ccSwitch.data || v.ccSwitch.traces.length > 0);
}

const down = (v: CodexView) => !!v.custom && v.reach.state === 'down';

/** The custom provider, when config.toml does not name it. */
const customWord = () => t('nhà cung cấp riêng', 'the custom provider');

/** `3 phiên codex` / `3 codex sessions`, bold, or without a count when it is not known. */
function codexSessions(n: number | null | undefined): { vi: string; en: string } {
  if (n == null) return { vi: 'các phiên codex', en: 'codex sessions' };
  return { vi: '**' + n + ' phiên codex**', en: '**' + n + (n === 1 ? ' codex session**' : ' codex sessions**') };
}

/** Why the switch to the custom provider is refused, as the note under the row says it. */
export function refusalText(v: CodexView, r: CodexRefusal): string {
  const name = v.custom?.name ?? customWord();
  switch (r.kind) {
    case 'noCustom':
      return t(
        'config.toml chưa có bảng nhà cung cấp riêng ([model_providers.<tên>]) để chuyển sang.',
        'config.toml has no custom provider table ([model_providers.<name>]) to switch to.',
      );
    case 'noKey':
      return t(
        name + ' chưa có khoá API riêng nên chưa chuyển được. Dán khoá để dùng ' + name + '.',
        name + ' has no API key of its own, so Codex cannot switch to it yet. Paste a key to use ' + name + '.',
      );
    case 'placeholder':
      return t(
        name + ' chỉ có khoá giữ chỗ của CC Switch nên chưa chuyển được. Dán khoá thật để dùng ' + name + '.',
        name + ' only has CC Switch’s placeholder key, so Codex cannot switch to it yet. Paste a real key to use ' + name + '.',
      );
    case 'profile':
      return t(
        'Profile "' + r.profile + '" đang dùng tự chọn đường đi (' + r.key + '), nên đổi dòng chọn nhà cung cấp không có tác dụng. App không chuyển.',
        'Profile “' + r.profile + '” uses automatic routing (' + r.key + '), so changing the provider line has no effect. The app does not switch.',
      );
    case 'shape':
      return t(
        'Bảng ' + name + ' viết dạng gọn trên một dòng; app chỉ sửa bảng dạng [model_providers.' + (v.custom?.id ?? '…') + '].',
        'The ' + name + ' table is written inline on one line; the app only edits tables written as [model_providers.' + (v.custom?.id ?? '…') + '].',
      );
  }
}

// ===== The popover row =====

export interface CodexRowView {
  /** `Codex App đang mở`, in the section header. */
  head: string;
  chips: ChipView[];
  sub: string;
  /** `→ ChatGPT` */
  go: string;
  /** The row cannot open the sheet: the switch it offers is refused. */
  off: boolean;
  label: string;
  note: NoteView | null;
  /** "Dán khoá…" under a note that says the provider has no key of its own. */
  fix: { label: string; act: Action } | null;
}

/** Codex goes through the custom provider without the ChatGPT login: plugins unticked. */
function loginOff(v: CodexView): boolean {
  return v.current === 'custom' && v.custom?.requiresOpenaiAuth === false;
}

/** Null on a Mac without a Codex folder. */
export function codexRow(model: Model): CodexRowView | null {
  const v = model.codex;
  if (!v.present) return null;
  const cur = currentId(v);
  const chips: ChipView[] = [{ t: shortName(v, cur), k: v.current === 'custom' ? 'info' : '' }];
  if (down(v))
    chips.push(
      v.current === 'custom'
        ? { t: t('không trả lời', 'not answering'), k: 'bad' }
        : { t: t(v.custom!.name + ' không trả lời', v.custom!.name + ' not answering'), k: 'warn' },
    );
  if (ccSwitchManages(v)) chips.push({ t: t('CC Switch cũng quản lý', 'CC Switch also manages it'), k: 'warn' });
  if (v.external) chips.push({ t: t('đổi ngoài app', 'changed outside the app'), k: 'warn' });
  if (!hasLogin(v)) chips.push({ t: t('chưa đăng nhập', 'not signed in'), k: 'warn' });

  let sub =
    v.current === 'custom'
      ? down(v)
        ? t('Không trả lời từ ' + clock(v.reach.since), 'Not answering since ' + clock(v.reach.since))
        : v.reach.state === 'up'
          ? t('Trả lời ' + v.reach.ms + ' ms', 'Answers in ' + v.reach.ms + ' ms')
          : v.custom!.name
      : v.current === 'other'
        ? t('Nhà cung cấp ' + cur, 'Provider ' + cur)
        : t('Tài khoản ChatGPT', 'ChatGPT account');
  if (hasLogin(v))
    sub += loginOff(v) ? t(' · không dùng đăng nhập ChatGPT', ' · without the ChatGPT sign-in') : t(' · giữ đăng nhập ChatGPT', ' · keeps the ChatGPT sign-in');

  const target = nextTarget(v);
  const refusal = target === 'custom' ? v.refusal : v.backRefusal;
  const off = !!refusal && !v.external;
  const to = shortName(v, targetId(v, target));
  let note: NoteView | null = null;
  if (v.config === 'invalid' || v.config === 'unreadable')
    note = {
      tone: 'bad',
      text: t(
        'Không đọc được config.toml của Codex (' + (v.reason ?? 'không rõ lý do') + '). App không sửa file này.',
        'Could not read Codex’s config.toml (' + (v.reason ?? 'reason unknown') + '). The app does not edit this file.',
      ),
    };
  else if (v.config === 'missing')
    note = { tone: '', text: t('Codex chưa có config.toml: chưa có nhà cung cấp riêng để chuyển sang.', 'Codex has no config.toml yet, so there is no custom provider to switch to.') };
  else if (refusal && refusal.kind !== 'noCustom') note = { tone: 'bad', text: refusalText(v, refusal) };
  else if (refusal) note = { tone: '', text: refusalText(v, refusal) };
  else if (ccSwitchManages(v))
    note = {
      tone: '',
      text: t(
        'CC Switch cũng đang quản lý Codex. Lần chuyển sau của CC Switch sẽ ghi đè lựa chọn ở đây.',
        'CC Switch also manages Codex. Its next switch will overwrite the choice made here.',
      ),
    };
  const curName = longName(v, cur);
  const toName = longName(v, targetId(v, target));
  return {
    head: v.procs ? (v.procs.app ? t('Codex App đang mở', 'Codex App is open') : t('Codex App không mở', 'Codex App is not open')) : '',
    chips,
    sub,
    go: '→ ' + to,
    off,
    label: t(
      'Codex đang dùng ' + curName + '. ' + (off ? 'Chưa chuyển được sang ' + toName : 'Bấm để chuyển sang ' + toName),
      'Codex uses ' + curName + '. ' + (off ? 'Cannot switch to ' + toName + ' yet' : 'Press to switch to ' + toName),
    ),
    note,
    fix: refusal && (refusal.kind === 'noKey' || refusal.kind === 'placeholder') && v.takesKey ? { label: t('Dán khoá…', 'Paste key…'), act: 'codex:key' } : null,
  };
}

// ===== The ChatGPT accounts (Tổng quan, the Codex tab) =====

/** A ChatGPT account in a table, drawn as the Claude and Antigravity accounts are. */
export interface ChatGptRowView {
  key: string;
  /** The email's name while names show and the report has one; else `ChatGPT`. */
  name: string;
  sub: string;
  /** The whole email, for hover; '' without one. */
  full: string;
  chips: ChipView[];
  /** 5 giờ · Tuần. */
  cells: CellView[];
  /** Codex goes through this account now. */
  active: boolean;
  /** Old numbers: drawn muted. */
  dim: boolean;
  /** Why "Chuyển" cannot run now; '' when it can. */
  off: string;
  label: string;
}

/**
 * The ChatGPT accounts Codex can go through, each with its five hours and
 * its week. Codex keeps one sign-in (`auth.json`), so today the list holds
 * that one account; the layout is a list so more can join it.
 */
export function chatGptRows(model: Model, now: Date): ChatGptRowView[] {
  const v = model.codex;
  if (!v.present) return [];
  const s = model.snapshot.codex;
  // Codex is here even before its first reading: the account's row still shows, without numbers.
  const accounts = s.accounts.length ? s.accounts : [null];
  const names = emailNames(accounts.map((a) => a?.email));
  return accounts.map((a, i) => {
    const pick = (kind: 'session' | 'week') => a?.windows.find((l) => l.kind === kind) ?? a?.lines.find((l) => l.kind === kind);
    const cells = (['session', 'week'] as const).map((k) => cellOf(pick(k), k, now, '—'));
    const chips: ChipView[] = [];
    if (!hasLogin(v)) chips.push({ t: t('chưa đăng nhập', 'not signed in'), k: 'warn' });
    const since = secondsSince(a?.fetchedAt ?? null, now);
    if (a?.reading === 'resetPending') chips.push({ t: t('chờ số sau mốc đặt lại', 'waiting for the number after its reset'), k: 'info' });
    else if (a?.reading === 'noData') chips.push({ t: t('chưa có số', 'no number yet'), k: '' });
    else if (a?.reading === 'stale' && since != null) chips.push({ t: t('số ' + age(since) + ' trước', 'numbers ' + age(since) + ' old'), k: 'stale' });
    const active = v.current === 'chatgpt' && (a?.active ?? true);
    const off = active ? '' : v.backRefusal ? refusalText(v, v.backRefusal) : v.busy ? t('Codex đang chuyển', 'Codex is switching') : '';
    const email = a?.email ?? '';
    const name = (email && names.get(email)) || 'ChatGPT';
    const sub = v.login === 'keychain' ? t('đăng nhập trong Keychain', 'sign-in in the Keychain') : hasLogin(v) ? t('đăng nhập còn trong máy', 'sign-in kept on this Mac') : '';
    return {
      key: 'x' + (a?.number ?? 0) + ':' + i,
      name,
      sub,
      full: email,
      chips,
      cells,
      active,
      dim: a?.reading === 'stale',
      off,
      label:
        name +
        (sub ? ', ' + sub : '') +
        (active ? t(', đang dùng', ', in use') : '') +
        '. ' +
        cells.map((c) => c.words).join('. ') +
        (chips.length ? '. ' + chips.map((c) => c.t).join(', ') : '') +
        (off ? '. ' + off : ''),
    };
  });
}

/** Why the ChatGPT numbers are missing, under the accounts; null while they read. */
export function chatGptNote(model: Model): NoteView | null {
  const s = model.snapshot.codex;
  if (s.status === 'unavailable') return { tone: 'warn', text: t('Không đọc được hạn mức ChatGPT', 'Cannot read the ChatGPT quota') + (s.reason ? ' (' + s.reason + ')' : '') + '.' };
  if (s.status === 'single' && !s.accounts.length && s.reason) return { tone: 'warn', text: t('Không đọc được hạn mức ChatGPT', 'Cannot read the ChatGPT quota') + ' (' + s.reason + ').' };
  return null;
}

/**
 * The popover's banner for Codex, when one is due: the custom provider in
 * use stopped answering (warn), no ChatGPT login (warn), or the file changed
 * outside the app (info).
 */
export function codexBanner(model: Model): BannerView | null {
  const v = model.codex;
  if (!v.present) return null;
  if (down(v) && v.current === 'custom') {
    const name = v.custom!.name;
    return {
      tone: 'warn',
      title: t(name + ' không trả lời từ ' + clock(v.reach.since), name + ' has not answered since ' + clock(v.reach.since)),
      body: t(
        'Codex đang đi qua ' + name + ' nên có thể lỗi. Chuyển về tài khoản ChatGPT trong lúc chờ?',
        'Codex goes through ' + name + ', so it may fail. Switch back to the ChatGPT account while you wait?',
      ),
      actions: [
        { label: t('Chuyển về ChatGPT', 'Switch back to ChatGPT'), act: 'codex:switch:chatGpt' },
        { label: t('Thử lại', 'Try again'), act: 'codex:check' },
      ],
    };
  }
  if (v.login === 'missing' || v.login === 'apiKey') {
    return {
      tone: 'warn',
      title: t('Codex chưa có đăng nhập ChatGPT', 'Codex has no ChatGPT sign-in'),
      body: t(
        'Plugin và điều khiển từ xa của Codex App cần đăng nhập. Mở Codex App để đăng nhập; app này không đăng nhập hộ.',
        'Codex App’s plugins and remote control need a sign-in. Open Codex App to sign in; this app does not sign in for you.',
      ),
      actions: [{ label: t('Hướng dẫn', 'Guide'), act: 'codex:guide:login' }],
    };
  }
  if (v.external) return externalBanner(v);
  return null;
}

export function externalBanner(v: CodexView): BannerView | null {
  const e = v.external;
  if (!e) return null;
  const via = shortName(v, e.now);
  return {
    tone: 'info',
    title: t('Cấu hình Codex vừa đổi ngoài app', 'Codex settings changed outside the app'),
    body: t(
      (e.at ? 'Lúc ' + clock(e.at) + ' file' : 'File') +
        ' config.toml của Codex được sửa. Codex giờ ' +
        (e.now === CHATGPT_ID ? 'dùng tài khoản ChatGPT' : 'đi qua ' + via) +
        '. App không tự sửa lại.',
      (e.at ? 'At ' + clock(e.at) + ', ' : '') +
        'Codex’s config.toml was edited. Codex now ' +
        (e.now === CHATGPT_ID ? 'uses the ChatGPT account' : 'goes through ' + via) +
        '. The app does not change it back.',
    ),
    actions: [
      { label: t('Xem thay đổi', 'See changes'), act: 'codex:external' },
      { label: t('Đã hiểu', 'Got it'), act: 'codex:keep' },
    ],
  };
}

/** The line the summary and the Overview card use. */
export function codexLine(v: CodexView): { text: string; tone: '' | 'bad' } {
  if (down(v) && v.current === 'custom') return { text: t(v.custom!.name + ' không trả lời', v.custom!.name + ' not answering'), tone: 'bad' };
  if (v.current === 'custom') return { text: t('Codex qua ' + v.custom!.name, 'Codex through ' + v.custom!.name), tone: '' };
  if (v.current === 'other') return { text: t('Codex dùng ' + currentId(v), 'Codex uses ' + currentId(v)), tone: '' };
  return { text: t('Codex dùng tài khoản ChatGPT', 'Codex uses the ChatGPT account'), tone: '' };
}

// ===== The confirmation sheet =====

export interface CodexSheetView {
  title: string;
  lead: string;
  facts: FactView[];
  /** CC Switch also manages Codex: the sheet links to the guide. */
  ccSwitch: boolean;
  confirm: string;
  /** Not switchable now (the note says why). */
  blocked: boolean;
}

/** The five facts, in the design's order, with each state's own warning on top. */
export function codexSheet(model: Model, target: CodexTarget): CodexSheetView {
  const v = model.codex;
  const toId = targetId(v, target);
  const toName = longName(v, toId);
  const name = v.custom?.name ?? customWord();
  const facts: FactView[] = [];
  const cc = ccSwitchManages(v);
  const refusal = target === 'custom' ? v.refusal : v.backRefusal;
  if (refusal) facts.push({ k: 'x', text: refusalText(v, refusal) });
  if (cc)
    facts.push({
      k: 'w',
      text: t(
        '**CC Switch vẫn đang quản lý Codex** trên máy này. Lần chuyển sau của CC Switch sẽ ghi đè lựa chọn nhà cung cấp này. Chỉ chuyển Codex ở một nơi.',
        '**CC Switch still manages Codex** on this Mac. Its next switch will overwrite this provider choice. Switch Codex in one place only.',
      ),
    });
  const isDown = down(v);
  if (isDown && target === 'chatGpt')
    facts.push({
      k: '',
      text: t(
        name + ' không trả lời từ ' + clock(v.reach.since) + ' (3 lần liền, mỗi lần chờ 5 giây).',
        name + ' has not answered since ' + clock(v.reach.since) + ' (3 tries in a row, 5 seconds each).',
      ),
    });
  if (isDown && target === 'custom')
    facts.push({
      k: 'x',
      text: t('**' + name + ' đang không trả lời.** Codex sẽ lỗi ngay sau khi chuyển.', '**' + name + ' is not answering.** Codex will fail right after the switch.'),
    });

  // 1. Keep the ChatGPT login.
  if (!hasLogin(v))
    facts.push({
      k: 'w',
      text:
        target === 'chatGpt'
          ? t('**Chưa có đăng nhập ChatGPT.** Sau khi chuyển, Codex sẽ mở trang đăng nhập.', '**No ChatGPT sign-in yet.** After the switch, Codex opens its sign-in page.')
          : t(
              '**Chưa có đăng nhập ChatGPT.** Plugin và điều khiển từ xa của Codex App cần đăng nhập này.',
              '**No ChatGPT sign-in yet.** Codex App’s plugins and remote control need it.',
            ),
    });
  else if (target === 'custom' && v.custom?.key === 'headers')
    facts.push({
      k: 'w',
      text: t(
        'Khoá của ' + name + ' gửi qua header, nên app đặt `requires_openai_auth = false` để đăng nhập ChatGPT không bị gửi tới ' + name + '. Plugin và điều khiển từ xa có thể không chạy khi dùng ' + name + '.',
        name + '’s key is sent in a header, so the app sets `requires_openai_auth = false` to keep the ChatGPT sign-in from being sent to ' + name + '. Plugins and remote control may not work while on ' + name + '.',
      ),
    });
  else if (target === 'custom' && !v.plugins)
    facts.push({
      k: 'w',
      text: t(
        '**Không dùng đăng nhập ChatGPT** khi chạy ' + name + ' (ô "Dùng plugin" đang bỏ tick): plugin cần đăng nhập (apps, connector) sẽ tắt. `auth.json` vẫn giữ nguyên.',
        '**No ChatGPT sign-in** while on ' + name + ' (the “Use plugins” box is unticked): plugins that need it (apps, connectors) turn off. `auth.json` stays as it is.',
      ),
    });
  else
    facts.push({
      k: 'ok',
      text: t('Giữ đăng nhập ChatGPT để Codex App tiếp tục dùng plugin và điều khiển từ xa.', 'Keeps the ChatGPT sign-in so Codex App can still use plugins and remote control.'),
    });
  // 2. One file, one line, a backup first.
  facts.push({
    k: '',
    text: t(
      'Chỉ sửa `config.toml` của Codex (dòng chọn nhà cung cấp' +
        (target === 'custom' ? ' và `requires_openai_auth` trong bảng ' + name : '') +
        '). Sao lưu bản cũ trước khi ghi. Không đụng `auth.json` và danh sách MCP.',
      'Only edits Codex’s `config.toml` (the provider line' +
        (target === 'custom' ? ' and `requires_openai_auth` in the ' + name + ' table' : '') +
        '). Backs up the old file before writing. Does not touch `auth.json` or the MCP list.',
    ),
  });
  // 3. Codex must restart.
  const sessions = v.procs?.sessions;
  const s = codexSessions(sessions);
  facts.push({
    k: 'w',
    text: t(
      'Codex phải khởi động lại: thoát hẳn Codex App (⌘Q) rồi mở lại; ' + s.vi + ' trong Terminal cần mở lại.',
      'Codex has to restart: quit Codex App (⌘Q) and open it again; ' + s.en + ' in Terminal ' + (sessions === 1 ? 'needs' : 'need') + ' reopening.',
    ),
  });
  // 4. Finish what runs.
  facts.push({
    k: 'w',
    text: t(
      'Hoàn tất việc đang chạy trước khi chuyển. Yêu cầu tiếp theo của phiên cũ có thể dùng nhà cung cấp mới và gặp lỗi.',
      'Finish running work first. An older session’s next request may use the new provider and fail.',
    ),
  });
  // 5. Resumable sessions are filed by provider.
  facts.push({
    k: '',
    text: t(
      'Các phiên có thể tiếp tục được tách theo nhà cung cấp: `codex resume` chỉ hiện phiên làm trên ' + toName + '. Codex App vẫn thấy hết.',
      'Resumable sessions are kept per provider: `codex resume` only shows sessions made on ' + toName + '. Codex App still sees them all.',
    ),
  });

  const risky = cc || (isDown && target === 'custom');
  return {
    title: target === 'custom' ? t('Chuyển Codex sang ' + name + '?', 'Switch Codex to ' + name + '?') : t('Chuyển Codex về tài khoản ChatGPT?', 'Switch Codex back to the ChatGPT account?'),
    lead:
      target === 'custom'
        ? t('Codex sẽ gửi yêu cầu qua ' + name + '.', 'Codex will send its requests through ' + name + '.')
        : t(
            'Codex dùng lại tài khoản ChatGPT.' + (v.custom ? ' Cấu hình ' + name + ' vẫn nằm trong file để phiên cũ còn tiếp tục được.' : ''),
            'Codex uses the ChatGPT account again.' + (v.custom ? ' The ' + name + ' settings stay in the file so older sessions can continue.' : ''),
          ),
    facts,
    ccSwitch: cc,
    confirm: risky
      ? t('Vẫn chuyển', 'Switch anyway')
      : target === 'custom'
        ? t('Chuyển sang ' + name, 'Switch to ' + name)
        : t('Chuyển về ChatGPT', 'Switch back to ChatGPT'),
    blocked: !!refusal || v.busy != null,
  };
}

/** What the background-process sheet says, on its own or right after a switch. */
export function daemonFacts(): FactView[] {
  return [
    {
      k: 'w',
      text: t(
        'Mọi phiên codex trong Terminal dùng chung tiến trình nền này: **việc đang chạy sẽ bị ngắt**.',
        'Every codex session in Terminal shares this background process: **running work will stop**.',
      ),
    },
    {
      k: '',
      text: t(
        'Codex App và tiện ích trong trình soạn thảo không dùng tiến trình này: thoát hẳn rồi mở lại.',
        'Codex App and the editor extensions do not use this process: quit them fully and open them again.',
      ),
    },
    { k: '', text: t('App chạy lệnh của chính Codex: `codex app-server daemon restart`.', 'The app runs Codex’s own command: `codex app-server daemon restart`.') },
  ];
}

export interface ExternalSheetView {
  lead: string;
  rows: { key: string; value: string; mono?: boolean }[];
  /** "Về lại …": the provider the app wrote last, when it is not the one in use. */
  back: { label: string; target: CodexTarget } | null;
}

export function externalSheet(model: Model): ExternalSheetView | null {
  const v = model.codex;
  const e = v.external;
  if (!e) return null;
  const who = v.ccSwitch.running ? t('có thể do CC Switch', 'possibly by CC Switch') : t('có thể do CC Switch hoặc sửa tay', 'possibly by CC Switch or by hand');
  const since = e.lastAt ? clock(e.lastAt) : null;
  const rows = [
    { key: t('App ghi lần cuối', 'Last written by the app'), value: (e.lastAt ? clock(e.lastAt) + ' · ' : '') + longName(v, e.lastProvider) },
    {
      key: t('Bây giờ', 'Now'),
      value:
        e.now === CHATGPT_ID
          ? t('tài khoản ChatGPT (không có model_provider)', 'ChatGPT account (no model_provider)')
          : shortName(v, e.now) + ' (model_provider = "' + e.now + '")',
      mono: false,
    },
    { key: t('Đăng nhập ChatGPT', 'ChatGPT sign-in'), value: hasLogin(v) ? t('còn trong máy', 'still on this Mac') : t('chưa có', 'none') },
    {
      key: t('Danh sách MCP', 'MCP list'),
      value: t(
        e.mcpCount + ' máy chủ MCP.' + (e.mcpChanged ? ' Danh sách đã thay đổi' + (since ? ' từ ' + since : '') + '.' : ' Không đổi.'),
        e.mcpCount + (e.mcpCount === 1 ? ' MCP server.' : ' MCP servers.') + (e.mcpChanged ? ' The list changed' + (since ? ' since ' + since : '') + '.' : ' Unchanged.'),
      ),
    },
  ];
  const backTarget: CodexTarget = e.lastProvider === CHATGPT_ID ? 'chatGpt' : 'custom';
  const back =
    e.lastProvider !== e.now && (backTarget === 'chatGpt' || v.custom?.id === e.lastProvider)
      ? { label: t('Về lại ' + longName(v, e.lastProvider) + '…', 'Back to ' + longName(v, e.lastProvider) + '…'), target: backTarget }
      : null;
  return {
    lead: t(
      (e.at ? 'Lúc ' + clock(e.at) + ' ' : '') + 'config.toml được sửa, ' + who + '. App không tự sửa lại.',
      (e.at ? 'At ' + clock(e.at) + ', ' : '') + 'config.toml was edited, ' + who + '. The app does not change it back.',
    ),
    rows,
    back,
  };
}

// ===== Feedback =====

const done = () => ({ label: t('Đã xong', 'Done'), act: 'closeFeedback' as Action });
const close = () => ({ label: t('Đóng', 'Close'), act: 'closeFeedback' as Action });
const NOTHING_VI = ' Chưa đổi gì.';
const NOTHING_EN = ' Nothing changed.';

function restartLine(v: CodexView, name: string): string {
  const n = v.procs?.sessions;
  return t(
    'Thoát hẳn Codex App (⌘Q) rồi mở lại; ' + (n == null ? 'các phiên' : n + ' phiên') + ' codex trong Terminal cần mở lại để dùng ' + name + '.',
    'Quit Codex App (⌘Q) and open it again; ' +
      (n == null ? 'codex sessions' : n + (n === 1 ? ' codex session' : ' codex sessions')) +
      ' in Terminal ' + (n === 1 ? 'needs' : 'need') + ' reopening to use ' + name + '.',
  );
}

/** After a switch. `restartNext`: the owner ticked the restart box, and the restart sheet comes next. */
export function codexFeedback(outcome: CodexOutcome, model: Model): FeedbackView {
  const v = model.codex;
  const to = longName(v, targetId(v, outcome.target));
  const restartDaemon = () => ({ label: t('Khởi động lại tiến trình nền Codex…', 'Restart the Codex background process…'), act: 'codex:daemon' as Action });
  switch (outcome.kind) {
    case 'switched':
      if (outcome.from === outcome.to)
        return {
          tone: '',
          title: v.plugins ? t('Đã bật plugin khi chạy ' + to, 'Turned on plugins while on ' + to) : t('Đã tắt plugin khi chạy ' + to, 'Turned off plugins while on ' + to),
          body: restartLine(v, to),
          actions: [restartDaemon(), done()],
        };
      return {
        tone: '',
        title: t('Đã chuyển Codex sang ' + to, 'Switched Codex to ' + to),
        body:
          (hasLogin(v)
            ? outcome.target === 'custom' && !v.plugins
              ? t('Không dùng đăng nhập ChatGPT (plugin tắt). ', 'Not using the ChatGPT sign-in (plugins off). ')
              : t('Vẫn giữ đăng nhập ChatGPT. ', 'The ChatGPT sign-in is kept. ')
            : '') +
          restartLine(v, to) +
          t(' Bản cũ của config.toml nằm trong thư mục sao lưu.', ' The old config.toml is in the backup folder.'),
        actions: [restartDaemon(), done()],
      };
    case 'already':
      return {
        tone: 'info',
        title: t('Codex đã dùng ' + to, 'Codex already uses ' + to),
        body: t('config.toml đã chọn ' + to + ', nên app không ghi gì.', 'config.toml already picks ' + to + ', so the app wrote nothing.'),
      };
    case 'refused':
      return {
        tone: 'bad',
        title: t('Chưa chuyển Codex sang ' + to, 'Could not switch Codex to ' + to),
        body: refusalText(v, outcome.refusal) + t(NOTHING_VI, NOTHING_EN),
        actions: [close()],
      };
    case 'invalid':
      return {
        tone: 'bad',
        title: t('Không sửa được config.toml', 'Could not edit config.toml'),
        body: t(
          'App không đọc được file này như Codex đọc (' + outcome.detail + '). Chưa đổi gì.',
          'The app cannot read this file the way Codex does (' + outcome.detail + '). Nothing changed.',
        ),
        actions: [close()],
      };
    case 'failed':
      return {
        tone: 'bad',
        title: t('Không ghi được config.toml', 'Could not write config.toml'),
        body:
          outcome.error.kind === 'changed'
            ? t(
                'File bị sửa trong lúc ghi (mã băm khác). Chưa đổi gì. Bản cũ vẫn nằm trong thư mục sao lưu.',
                'The file changed while it was being written (a different hash). Nothing changed. The old copy is still in the backup folder.',
              )
            : outcome.error.kind === 'verify'
              ? t('Đọc lại sau khi ghi không khớp. Bản cũ nằm trong thư mục sao lưu.', 'Reading it back after writing did not match. The old copy is in the backup folder.')
              : t('Lỗi khi ghi (' + outcome.error.detail + '). Chưa đổi gì.', 'Writing failed (' + outcome.error.detail + '). Nothing changed.'),
        actions: [
          { label: t('Thử lại', 'Try again'), act: ('codex:switch:' + outcome.target) as Action },
          { label: t('Xem thư mục sao lưu', 'Show backup folder'), act: 'codex:backup' },
        ],
      };
  }
}

/** The switch call itself was refused (`busy`) or broke. */
export function codexErrorFeedback(target: CodexTarget, error: unknown, model: Model): FeedbackView {
  const to = longName(model.codex, targetId(model.codex, target));
  return {
    tone: 'bad',
    title: t('Chưa chuyển Codex sang ' + to, 'Could not switch Codex to ' + to),
    body:
      String(error) === 'busy'
        ? t('Một lần chuyển hoặc khởi động lại khác đang chạy. Chờ xong rồi thử lại.', 'Another switch or restart is running. Wait for it to finish, then try again.')
        : t('Không gọi được lệnh chuyển. Chưa đổi gì.', 'Could not run the switch. Nothing changed.'),
    actions: [close()],
  };
}

/** After a restart; `afterSwitch` when it came straight after a switch. */
export function codexRestartFeedback(r: CodexRestart | 'busy' | 'error', model: Model, afterSwitch: boolean): FeedbackView {
  const v = model.codex;
  const name = longName(v, currentId(v));
  if (r === 'busy' || r === 'error' || r.kind === 'failed' || r.kind === 'noCodex') {
    const why =
      r === 'busy'
        ? t('Một lần chuyển khác đang chạy.', 'Another switch is running.')
        : r === 'error'
          ? t('Không gọi được lệnh.', 'Could not run the command.')
          : r.kind === 'noCodex'
            ? t('Không tìm thấy lệnh codex trên máy này.', 'The codex command was not found on this Mac.')
            : t('codex app-server daemon restart báo lỗi (' + r.detail + ').', 'codex app-server daemon restart failed (' + r.detail + ').');
    return {
      tone: 'warn',
      title: t(
        (afterSwitch ? 'Đã chuyển Codex sang ' + name + ', ' : '') + 'chưa khởi động lại được tiến trình nền',
        afterSwitch ? 'Switched Codex to ' + name + ', but could not restart the background process' : 'Could not restart the background process',
      ),
      body: why + t(' Mở lại codex trong Terminal; Codex App vẫn phải thoát hẳn (⌘Q) rồi mở lại.', ' Reopen codex in Terminal; Codex App still has to quit (⌘Q) and open again.'),
      actions: [close()],
    };
  }
  if (r.kind === 'notRunning')
    return {
      tone: '',
      title: t(
        (afterSwitch ? 'Đã chuyển Codex sang ' + name + '. ' : '') + 'Tiến trình nền Codex không chạy',
        (afterSwitch ? 'Switched Codex to ' + name + '. ' : '') + 'The Codex background process is not running',
      ),
      body: t('Không có gì để khởi động lại. Lần mở codex tới sẽ dùng ' + name + '.', 'Nothing to restart. The next codex you open uses ' + name + '.'),
    };
  return afterSwitch
    ? {
        tone: '',
        title: t('Đã chuyển Codex sang ' + name + ' và khởi động lại tiến trình nền', 'Switched Codex to ' + name + ' and restarted the background process'),
        body: t(
          'Mở lại codex trong Terminal để dùng ' + name + '. Codex App vẫn phải thoát hẳn (⌘Q) rồi mở lại.',
          'Reopen codex in Terminal to use ' + name + '. Codex App still has to quit (⌘Q) and open again.',
        ),
        actions: [{ label: t('Đã hiểu', 'Got it'), act: 'closeFeedback' }],
      }
    : {
        tone: '',
        title: t('Tiến trình nền Codex đã khởi động lại', 'The Codex background process restarted'),
        body: t('Mở lại codex trong Terminal để dùng ' + name + '.', 'Reopen codex in Terminal to use ' + name + '.'),
      };
}


// ===== "Dùng plugin khi chạy Custom API" =====

/** The checkbox's words: the label and its one-line hint. */
export function pluginsLabel(): string {
  return t('Dùng plugin khi chạy Custom API (giữ đăng nhập ChatGPT)', 'Use plugins while on a Custom API (keeps the ChatGPT sign-in)');
}
export function pluginsHint(): string {
  return t(
    'Bỏ tick: Codex không dùng đăng nhập ChatGPT khi chạy Custom API, plugin cần đăng nhập sẽ tắt.',
    'Unticked: Codex does not use the ChatGPT sign-in on a Custom API, so plugins that need it turn off.',
  );
}

/** The `requires_openai_auth` a switch to the custom provider would write with the box `on`. */
export function wantsWith(v: CodexView, on: boolean): boolean | null {
  if (!v.custom || v.wantsRequires == null) return null;
  return (v.custom.key === 'bearer' || v.custom.key === 'envKey') && v.login !== 'missing' && v.login !== 'apiKey' && v.login !== 'unreadable' && v.login !== 'ephemeral' && on;
}

/** The box changed while Codex is on the custom provider: whether to offer applying it now. */
export function pluginsNeedApply(v: CodexView, on: boolean): boolean {
  const want = wantsWith(v, on);
  return v.current === 'custom' && want != null && v.custom?.requiresOpenaiAuth !== want && !v.refusal;
}

export interface PluginsSheetView {
  title: string;
  facts: FactView[];
}

/** "Áp dụng ngay?": the box changed while Codex is on the custom provider. */
export function pluginsSheet(model: Model, on: boolean): PluginsSheetView {
  const v = model.codex;
  const name = v.custom?.name ?? customWord();
  const want = wantsWith(v, on) ?? false;
  const sessions = v.procs?.sessions;
  const s = codexSessions(sessions);
  return {
    title: t('Áp dụng ngay cho ' + name + '?', 'Apply to ' + name + ' now?'),
    facts: [
      on
        ? {
            k: 'ok',
            text: t(
              'Codex dùng lại đăng nhập ChatGPT khi chạy ' + name + ': plugin chạy như với tài khoản ChatGPT.',
              'Codex uses the ChatGPT sign-in again while on ' + name + ': plugins work as they do on the ChatGPT account.',
            ),
          }
        : {
            k: 'w',
            text: t(
              '**Không dùng đăng nhập ChatGPT** khi chạy ' + name + ': plugin cần đăng nhập (apps, connector) sẽ tắt.',
              '**No ChatGPT sign-in** while on ' + name + ': plugins that need it (apps, connectors) turn off.',
            ),
          },
      {
        k: '',
        text: t(
          'Chỉ đổi `requires_openai_auth = ' + want + '` trong bảng ' + name + '. Sao lưu trước khi ghi. Không đụng `auth.json`.',
          'Only changes `requires_openai_auth = ' + want + '` in the ' + name + ' table. Backs up before writing. Does not touch `auth.json`.',
        ),
      },
      {
        k: 'w',
        text: t(
          'Thoát hẳn Codex App (⌘Q) rồi mở lại; ' + s.vi + ' trong Terminal cần mở lại.',
          'Quit Codex App (⌘Q) and open it again; ' + s.en + ' in Terminal ' + (sessions === 1 ? 'needs' : 'need') + ' reopening.',
        ),
      },
    ],
  };
}

// ===== "Dán khoá" =====

/** What the key sheet is for: a key to paste, a key to replace, or a variable to set. */
export function keyMode(v: CodexView): 'paste' | 'replace' | 'env' | null {
  const c = v.custom;
  if (!c) return null;
  if (c.key === 'envKey') return 'env';
  if (!v.takesKey) return null;
  return c.key === 'bearer' ? 'replace' : 'paste';
}

export function keyRefusalText(v: CodexView, r: CodexKeyRefusal): string {
  const name = v.custom?.name ?? customWord();
  switch (r.kind) {
    case 'noCustom':
      return t('config.toml chưa có bảng nhà cung cấp riêng để ghi khoá.', 'config.toml has no custom provider table to write the key to.');
    case 'envKey':
      return t(name + ' đọc khoá từ biến ' + r.var + ': đặt biến này, app không ghi gì.', name + ' reads its key from the variable ' + r.var + ': set that variable; the app writes nothing.');
    case 'headers':
      return t(name + ' gửi khoá qua header: app không thêm khoá thứ hai.', name + ' sends its key in a header: the app does not add a second key.');
    case 'shape':
      return t(
        'Bảng ' + name + ' viết gọn trên một dòng; app chỉ sửa bảng dạng [model_providers.' + (v.custom?.id ?? '…') + '].',
        'The ' + name + ' table is written inline on one line; the app only edits tables written as [model_providers.' + (v.custom?.id ?? '…') + '].',
      );
    case 'empty':
      return t('Chưa có khoá.', 'No key yet.');
    case 'notAKey':
      return t('Khoá không được có dấu cách hay xuống dòng.', 'A key cannot contain spaces or line breaks.');
    case 'tooLong':
      return t('Khoá quá dài.', 'The key is too long.');
    case 'placeholder':
      return t('PROXY_MANAGED là khoá giữ chỗ của CC Switch, không phải khoá thật.', 'PROXY_MANAGED is CC Switch’s placeholder key, not a real key.');
    case 'reference':
      return t('Đây là tên biến, không phải khoá.', 'This is a variable name, not a key.');
    case 'badUrl':
      return t(
        'Địa chỉ phải bắt đầu bằng https:// (hoặc http://) và không chứa mật khẩu hay khoá.',
        'The address must start with https:// (or http://) and must not contain a password or key.',
      );
    case 'noUndo':
      return t('Không còn gì để hoàn tác.', 'Nothing left to undo.');
  }
}

/** "Hoàn tác" with what it can and cannot put back on the button itself. */
export function undoLabel(u: NonNullable<CodexView['keyUndo']>): string {
  const back =
    u.token === 'placeholder'
      ? t('trả lại PROXY_MANAGED', 'put back PROXY_MANAGED')
      : u.token === 'empty'
        ? t('trả lại khoá rỗng', 'put back the empty key')
        : t('bỏ khoá vừa dán', 'remove the pasted key');
  return t('Hoàn tác: ', 'Undo: ') + back + (u.baseUrl ? t(' và địa chỉ cũ', ' and the old address') : '');
}

/** The preview's facts: the backup, the login, the undo and its limit. */
export function keyFacts(v: CodexView, p: CodexKeyPreview): FactView[] {
  const name = v.custom?.name ?? '…';
  const facts: FactView[] = [
    {
      k: '',
      text: t(
        'Chỉ sửa bảng ' + name + ' trong `config.toml`. Sao lưu trước khi ghi, bản lưu không chứa khoá. Không đụng `auth.json`.',
        'Only edits the ' + name + ' table in `config.toml`. Backs up before writing; the backup holds no key. Does not touch `auth.json`.',
      ),
    },
  ];
  if (p.replaces)
    facts.push({ k: 'w', text: t('Khoá cũ bị thay. App không giữ khoá cũ nên **không hoàn tác được**.', 'The old key is replaced. The app does not keep it, so this **cannot be undone**.') });
  else if (p.undo)
    facts.push({
      k: '',
      text:
        t('Có nút Hoàn tác: ', 'There is an Undo button: it ') +
        (p.undo === 'placeholder'
          ? t('trả lại khoá giữ chỗ PROXY_MANAGED', 'puts back the placeholder key PROXY_MANAGED')
          : p.undo === 'empty'
            ? t('trả lại khoá rỗng', 'puts back the empty key')
            : t('bỏ khoá vừa dán', 'removes the pasted key')) +
        '.',
    });
  if (v.current === 'custom')
    facts.push({
      k: 'w',
      text: t(
        'Codex đang dùng ' + name + ': thoát hẳn Codex App (⌘Q) rồi mở lại để dùng khoá mới.',
        'Codex is using ' + name + ': quit Codex App (⌘Q) and open it again to use the new key.',
      ),
    });
  return facts;
}

/** After "Dán khoá" or its undo; `undo`: what the preview said the undo puts back. Never names the key. */
export function keyFeedback(out: CodexKeyOutcome, model: Model, undo: NonNullable<CodexView['keyUndo']> | null = null): FeedbackView {
  const v = model.codex;
  const name = v.custom?.name ?? customWord();
  switch (out.kind) {
    case 'written':
      return {
        tone: '',
        title: t('Đã ghi khoá cho ' + name, 'Wrote the key for ' + name),
        body:
          (v.current === 'custom' ? restartLine(v, name) : t('Giờ chuyển Codex sang ' + name + ' được.', 'Codex can now switch to ' + name + '.')) +
          t(' Bản cũ của config.toml nằm trong thư mục sao lưu.', ' The old config.toml is in the backup folder.'),
        actions: [...(out.undo && undo ? [{ label: undoLabel(undo), act: 'codex:undoKey' as Action }] : []), done()],
      };
    case 'undone':
      return {
        tone: '',
        title: t('Đã hoàn tác khoá của ' + name, 'Undid the key for ' + name),
        body: t('Bảng ' + name + ' trở lại như trước khi dán khoá.', 'The ' + name + ' table is back to how it was before the key was pasted.'),
      };
    case 'already':
      return { tone: 'info', title: t(name + ' đã có đúng khoá này', name + ' already has this key'), body: t('App không ghi gì.', 'The app wrote nothing.') };
    case 'refused':
      return { tone: 'bad', title: t('Chưa ghi khoá', 'Could not write the key'), body: keyRefusalText(v, out.refusal) + t(NOTHING_VI, NOTHING_EN), actions: [close()] };
    case 'invalid':
      return {
        tone: 'bad',
        title: t('Không sửa được config.toml', 'Could not edit config.toml'),
        body: t(
          'App không đọc được file này như Codex đọc (' + out.detail + '). Chưa đổi gì.',
          'The app cannot read this file the way Codex does (' + out.detail + '). Nothing changed.',
        ),
        actions: [close()],
      };
    case 'failed':
      return {
        tone: 'bad',
        title: t('Chưa ghi khoá', 'Could not write the key'),
        body:
          out.error.kind === 'changed'
            ? t(
                'config.toml đã đổi sau khi xem trước. Chưa đổi gì: mở lại để xem bản mới.',
                'config.toml changed after the preview. Nothing changed: open it again to see the new version.',
              )
            : out.error.kind === 'verify'
              ? t('Đọc lại sau khi ghi không khớp. Bản cũ nằm trong thư mục sao lưu.', 'Reading it back after writing did not match. The old copy is in the backup folder.')
              : t('Lỗi khi ghi (' + out.error.detail + '). Chưa đổi gì.', 'Writing failed (' + out.error.detail + '). Nothing changed.'),
        actions: [{ label: t('Xem thư mục sao lưu', 'Show backup folder'), act: 'codex:backup' }, close()],
      };
  }
}

/** The recent list's line for one entry, short. */
export function recentText(v: CodexView, r: CodexView['recent'][number]): string {
  const name = shortName(v, r.to);
  if (r.result === 'key') return name + t(': dán khoá API', ': pasted an API key');
  if (r.result === 'keyUndo') return name + t(': hoàn tác khoá', ': undid the key');
  if (r.result === 'plugins') return name + ': ' + (r.requires === false ? t('tắt plugin', 'plugins off') : t('bật plugin', 'plugins on'));
  const arrow = shortName(v, r.from) + ' → ' + name;
  if (r.result === 'observed') return arrow + t(' · đổi ngoài app', ' · changed outside the app');
  return arrow + (r.restarted ? t(' · đã khởi động lại tiến trình nền', ' · restarted the background process') : '');
}
