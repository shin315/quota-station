// What switching Codex shows (M8): the popover's Codex row and its banner,
// the confirmation sheet's five facts and the sheets around it, the
// feedback after a switch or a restart, and the Codex tab. Pure, like
// `agySwitch.ts`: the same model gives the same words. Copy follows the
// approved prototype (report section 5): the custom provider goes by the
// name its table in config.toml gives it, never a name built in here.

import { clock } from './format';
import type { CodexKeyOutcome, CodexKeyPreview, CodexKeyRefusal, CodexOutcome, CodexRefusal, CodexRestart, CodexTarget, CodexView, Model } from './model';
import type { FactView } from './switching';
import type { Action, BannerView, ChipView, FeedbackView, NoteView } from './view';

/** The id Codex files the ChatGPT login under. */
export const CHATGPT_ID = 'openai';

/** Short, for chips and arrows: `ChatGPT`, or the provider's name. */
export function shortName(v: CodexView, id: string): string {
  if (id === CHATGPT_ID) return 'ChatGPT';
  if (v.custom && id === v.custom.id) return v.custom.name;
  return id || 'không rõ';
}

/** In a sentence: `tài khoản ChatGPT`, or the provider's name. */
export function longName(v: CodexView, id: string): string {
  return id === CHATGPT_ID ? 'tài khoản ChatGPT' : shortName(v, id);
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

/** Why the switch to the custom provider is refused, as the note under the row says it. */
export function refusalText(v: CodexView, r: CodexRefusal): string {
  const name = v.custom?.name ?? 'nhà cung cấp riêng';
  switch (r.kind) {
    case 'noCustom':
      return 'config.toml chưa có bảng nhà cung cấp riêng ([model_providers.<tên>]) để chuyển sang.';
    case 'noKey':
      return name + ' chưa có khoá API riêng nên chưa chuyển được. Dán khoá để dùng ' + name + '.';
    case 'placeholder':
      return name + ' chỉ có khoá giữ chỗ của CC Switch nên chưa chuyển được. Dán khoá thật để dùng ' + name + '.';
    case 'profile':
      return 'Profile "' + r.profile + '" đang dùng tự chọn đường đi (' + r.key + '), nên đổi dòng chọn nhà cung cấp không có tác dụng. App không chuyển.';
    case 'shape':
      return 'Bảng ' + name + ' viết dạng gọn trên một dòng; app chỉ sửa bảng dạng [model_providers.' + (v.custom?.id ?? '…') + '].';
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
  if (down(v)) chips.push(v.current === 'custom' ? { t: 'không trả lời', k: 'bad' } : { t: v.custom!.name + ' không trả lời', k: 'warn' });
  if (ccSwitchManages(v)) chips.push({ t: 'CC Switch cũng quản lý', k: 'warn' });
  if (v.external) chips.push({ t: 'đổi ngoài app', k: 'warn' });
  if (!hasLogin(v)) chips.push({ t: 'chưa đăng nhập', k: 'warn' });

  let sub =
    v.current === 'custom'
      ? down(v)
        ? 'Không trả lời từ ' + clock(v.reach.since)
        : v.reach.state === 'up'
          ? 'Trả lời ' + v.reach.ms + ' ms'
          : v.custom!.name
      : v.current === 'other'
        ? 'Nhà cung cấp ' + cur
        : 'Tài khoản ChatGPT';
  if (hasLogin(v)) sub += loginOff(v) ? ' · không dùng đăng nhập ChatGPT' : ' · giữ đăng nhập ChatGPT';

  const target = nextTarget(v);
  const refusal = target === 'custom' ? v.refusal : v.backRefusal;
  const off = !!refusal && !v.external;
  const to = shortName(v, targetId(v, target));
  let note: NoteView | null = null;
  if (v.config === 'invalid' || v.config === 'unreadable') note = { tone: 'bad', text: 'Không đọc được config.toml của Codex (' + (v.reason ?? 'không rõ lý do') + '). App không sửa file này.' };
  else if (v.config === 'missing') note = { tone: '', text: 'Codex chưa có config.toml: chưa có nhà cung cấp riêng để chuyển sang.' };
  else if (refusal && refusal.kind !== 'noCustom') note = { tone: 'bad', text: refusalText(v, refusal) };
  else if (refusal) note = { tone: '', text: refusalText(v, refusal) };
  else if (ccSwitchManages(v)) note = { tone: '', text: 'CC Switch cũng đang quản lý Codex. Lần chuyển sau của CC Switch sẽ ghi đè lựa chọn ở đây.' };
  return {
    head: v.procs ? (v.procs.app ? 'Codex App đang mở' : 'Codex App không mở') : '',
    chips,
    sub,
    go: '→ ' + to,
    off,
    label: 'Codex đang dùng ' + longName(v, cur) + '. ' + (off ? 'Chưa chuyển được sang ' + longName(v, targetId(v, target)) : 'Bấm để chuyển sang ' + longName(v, targetId(v, target))),
    note,
    fix: refusal && (refusal.kind === 'noKey' || refusal.kind === 'placeholder') && v.takesKey ? { label: 'Dán khoá…', act: 'codex:key' } : null,
  };
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
      title: name + ' không trả lời từ ' + clock(v.reach.since),
      body: 'Codex đang đi qua ' + name + ' nên có thể lỗi. Chuyển về tài khoản ChatGPT trong lúc chờ?',
      actions: [
        { label: 'Chuyển về ChatGPT', act: 'codex:switch:chatGpt' },
        { label: 'Thử lại', act: 'codex:check' },
      ],
    };
  }
  if (v.login === 'missing' || v.login === 'apiKey') {
    return {
      tone: 'warn',
      title: 'Codex chưa có đăng nhập ChatGPT',
      body: 'Plugin và điều khiển từ xa của Codex App cần đăng nhập. Mở Codex App để đăng nhập; app này không đăng nhập hộ.',
      actions: [{ label: 'Hướng dẫn', act: 'codex:guide:login' }],
    };
  }
  if (v.external) return externalBanner(v);
  return null;
}

export function externalBanner(v: CodexView): BannerView | null {
  const e = v.external;
  if (!e) return null;
  return {
    tone: 'info',
    title: 'Cấu hình Codex vừa đổi ngoài app',
    body:
      (e.at ? 'Lúc ' + clock(e.at) + ' file' : 'File') +
      ' config.toml của Codex được sửa. Codex giờ ' +
      (e.now === CHATGPT_ID ? 'dùng tài khoản ChatGPT' : 'đi qua ' + shortName(v, e.now)) +
      '. App không tự sửa lại.',
    actions: [
      { label: 'Xem thay đổi', act: 'codex:external' },
      { label: 'Đã hiểu', act: 'codex:keep' },
    ],
  };
}

/** The line the summary and the Overview card use. */
export function codexLine(v: CodexView): { text: string; tone: '' | 'bad' } {
  if (down(v) && v.current === 'custom') return { text: v.custom!.name + ' không trả lời', tone: 'bad' };
  if (v.current === 'custom') return { text: 'Codex qua ' + v.custom!.name, tone: '' };
  if (v.current === 'other') return { text: 'Codex dùng ' + currentId(v), tone: '' };
  return { text: 'Codex dùng tài khoản ChatGPT', tone: '' };
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
  const name = v.custom?.name ?? 'nhà cung cấp riêng';
  const facts: FactView[] = [];
  const cc = ccSwitchManages(v);
  const refusal = target === 'custom' ? v.refusal : v.backRefusal;
  if (refusal) facts.push({ k: 'x', text: refusalText(v, refusal) });
  if (cc)
    facts.push({
      k: 'w',
      text: '**CC Switch vẫn đang quản lý Codex** trên máy này. Lần chuyển sau của CC Switch sẽ ghi đè lựa chọn nhà cung cấp này. Chỉ chuyển Codex ở một nơi.',
    });
  const isDown = down(v);
  if (isDown && target === 'chatGpt')
    facts.push({ k: '', text: name + ' không trả lời từ ' + clock(v.reach.since) + ' (3 lần liền, mỗi lần chờ 5 giây).' });
  if (isDown && target === 'custom') facts.push({ k: 'x', text: '**' + name + ' đang không trả lời.** Codex sẽ lỗi ngay sau khi chuyển.' });

  // 1. Keep the ChatGPT login.
  if (!hasLogin(v))
    facts.push({
      k: 'w',
      text:
        target === 'chatGpt'
          ? '**Chưa có đăng nhập ChatGPT.** Sau khi chuyển, Codex sẽ mở trang đăng nhập.'
          : '**Chưa có đăng nhập ChatGPT.** Plugin và điều khiển từ xa của Codex App cần đăng nhập này.',
    });
  else if (target === 'custom' && v.custom?.key === 'headers')
    facts.push({
      k: 'w',
      text: 'Khoá của ' + name + ' gửi qua header, nên app đặt `requires_openai_auth = false` để đăng nhập ChatGPT không bị gửi tới ' + name + '. Plugin và điều khiển từ xa có thể không chạy khi dùng ' + name + '.',
    });
  else if (target === 'custom' && !v.plugins)
    facts.push({
      k: 'w',
      text: '**Không dùng đăng nhập ChatGPT** khi chạy ' + name + ' (ô "Dùng plugin" đang bỏ tick): plugin cần đăng nhập (apps, connector) sẽ tắt. `auth.json` vẫn giữ nguyên.',
    });
  else facts.push({ k: 'ok', text: 'Giữ đăng nhập ChatGPT để Codex App tiếp tục dùng plugin và điều khiển từ xa.' });
  // 2. One file, one line, a backup first.
  facts.push({
    k: '',
    text:
      'Chỉ sửa `config.toml` của Codex (dòng chọn nhà cung cấp' +
      (target === 'custom' ? ' và `requires_openai_auth` trong bảng ' + name : '') +
      '). Sao lưu bản cũ trước khi ghi. Không đụng `auth.json` và danh sách MCP.',
  });
  // 3. Codex must restart.
  const sessions = v.procs?.sessions;
  facts.push({
    k: 'w',
    text:
      'Codex phải khởi động lại: thoát hẳn Codex App (⌘Q) rồi mở lại; ' +
      (sessions == null ? 'các phiên codex' : '**' + sessions + ' phiên codex**') +
      ' trong Terminal cần mở lại.',
  });
  // 4. Finish what runs.
  facts.push({ k: 'w', text: 'Hoàn tất việc đang chạy trước khi chuyển. Yêu cầu tiếp theo của phiên cũ có thể dùng nhà cung cấp mới và gặp lỗi.' });
  // 5. Resumable sessions are filed by provider.
  facts.push({
    k: '',
    text: 'Các phiên có thể tiếp tục được tách theo nhà cung cấp: `codex resume` chỉ hiện phiên làm trên ' + toName + '. Codex App vẫn thấy hết.',
  });

  const risky = cc || (isDown && target === 'custom');
  return {
    title: target === 'custom' ? 'Chuyển Codex sang ' + name + '?' : 'Chuyển Codex về tài khoản ChatGPT?',
    lead:
      target === 'custom'
        ? 'Codex sẽ gửi yêu cầu qua ' + name + '.'
        : 'Codex dùng lại tài khoản ChatGPT.' + (v.custom ? ' Cấu hình ' + name + ' vẫn nằm trong file để phiên cũ còn tiếp tục được.' : ''),
    facts,
    ccSwitch: cc,
    confirm: risky ? 'Vẫn chuyển' : target === 'custom' ? 'Chuyển sang ' + name : 'Chuyển về ChatGPT',
    blocked: !!refusal || v.busy != null,
  };
}

/** What the background-process sheet says, on its own or right after a switch. */
export function daemonFacts(): FactView[] {
  return [
    { k: 'w', text: 'Mọi phiên codex trong Terminal dùng chung tiến trình nền này: **việc đang chạy sẽ bị ngắt**.' },
    { k: '', text: 'Codex App và tiện ích trong trình soạn thảo không dùng tiến trình này: thoát hẳn rồi mở lại.' },
    { k: '', text: 'App chạy lệnh của chính Codex: `codex app-server daemon restart`.' },
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
  const who = v.ccSwitch.running ? 'có thể do CC Switch' : 'có thể do CC Switch hoặc sửa tay';
  const rows = [
    { key: 'App ghi lần cuối', value: (e.lastAt ? clock(e.lastAt) + ' · ' : '') + longName(v, e.lastProvider) },
    {
      key: 'Bây giờ',
      value: e.now === CHATGPT_ID ? 'tài khoản ChatGPT (không có model_provider)' : shortName(v, e.now) + ' (model_provider = "' + e.now + '")',
      mono: false,
    },
    { key: 'Đăng nhập ChatGPT', value: hasLogin(v) ? 'còn trong máy' : 'chưa có' },
    {
      key: 'Danh sách MCP',
      value: e.mcpCount + ' máy chủ MCP.' + (e.mcpChanged ? ' Danh sách đã thay đổi' + (e.lastAt ? ' từ ' + clock(e.lastAt) : '') + '.' : ' Không đổi.'),
    },
  ];
  const backTarget: CodexTarget = e.lastProvider === CHATGPT_ID ? 'chatGpt' : 'custom';
  const back =
    e.lastProvider !== e.now && (backTarget === 'chatGpt' || v.custom?.id === e.lastProvider)
      ? { label: 'Về lại ' + longName(v, e.lastProvider) + '…', target: backTarget }
      : null;
  return { lead: (e.at ? 'Lúc ' + clock(e.at) + ' ' : '') + 'config.toml được sửa, ' + who + '. App không tự sửa lại.', rows, back };
}

// ===== Feedback =====

const CLOSE = { label: 'Đã xong', act: 'closeFeedback' as Action };

function restartLine(v: CodexView, name: string): string {
  const n = v.procs?.sessions;
  return 'Thoát hẳn Codex App (⌘Q) rồi mở lại; ' + (n == null ? 'các phiên' : n + ' phiên') + ' codex trong Terminal cần mở lại để dùng ' + name + '.';
}

/** After a switch. `restartNext`: the owner ticked the restart box, and the restart sheet comes next. */
export function codexFeedback(outcome: CodexOutcome, model: Model): FeedbackView {
  const v = model.codex;
  const to = longName(v, targetId(v, outcome.target));
  switch (outcome.kind) {
    case 'switched':
      if (outcome.from === outcome.to)
        return {
          tone: '',
          title: (v.plugins ? 'Đã bật' : 'Đã tắt') + ' plugin khi chạy ' + to,
          body: restartLine(v, to),
          actions: [{ label: 'Khởi động lại tiến trình nền Codex…', act: 'codex:daemon' }, CLOSE],
        };
      return {
        tone: '',
        title: 'Đã chuyển Codex sang ' + to,
        body:
          (hasLogin(v) ? (outcome.target === 'custom' && !v.plugins ? 'Không dùng đăng nhập ChatGPT (plugin tắt). ' : 'Vẫn giữ đăng nhập ChatGPT. ') : '') +
          restartLine(v, to) +
          ' Bản cũ của config.toml nằm trong thư mục sao lưu.',
        actions: [{ label: 'Khởi động lại tiến trình nền Codex…', act: 'codex:daemon' }, CLOSE],
      };
    case 'already':
      return { tone: 'info', title: 'Codex đã dùng ' + to, body: 'config.toml đã chọn ' + to + ', nên app không ghi gì.' };
    case 'refused':
      return { tone: 'bad', title: 'Chưa chuyển Codex sang ' + to, body: refusalText(v, outcome.refusal) + ' Chưa đổi gì.', actions: [{ label: 'Đóng', act: 'closeFeedback' }] };
    case 'invalid':
      return {
        tone: 'bad',
        title: 'Không sửa được config.toml',
        body: 'App không đọc được file này như Codex đọc (' + outcome.detail + '). Chưa đổi gì.',
        actions: [{ label: 'Đóng', act: 'closeFeedback' }],
      };
    case 'failed':
      return {
        tone: 'bad',
        title: 'Không ghi được config.toml',
        body:
          outcome.error.kind === 'changed'
            ? 'File bị sửa trong lúc ghi (mã băm khác). Chưa đổi gì. Bản cũ vẫn nằm trong thư mục sao lưu.'
            : outcome.error.kind === 'verify'
              ? 'Đọc lại sau khi ghi không khớp. Bản cũ nằm trong thư mục sao lưu.'
              : 'Lỗi khi ghi (' + outcome.error.detail + '). Chưa đổi gì.',
        actions: [
          { label: 'Thử lại', act: ('codex:switch:' + outcome.target) as Action },
          { label: 'Xem thư mục sao lưu', act: 'codex:backup' },
        ],
      };
  }
}

/** The switch call itself was refused (`busy`) or broke. */
export function codexErrorFeedback(target: CodexTarget, error: unknown, model: Model): FeedbackView {
  const to = longName(model.codex, targetId(model.codex, target));
  return {
    tone: 'bad',
    title: 'Chưa chuyển Codex sang ' + to,
    body: String(error) === 'busy' ? 'Một lần chuyển hoặc khởi động lại khác đang chạy. Chờ xong rồi thử lại.' : 'Không gọi được lệnh chuyển. Chưa đổi gì.',
    actions: [{ label: 'Đóng', act: 'closeFeedback' }],
  };
}

/** After a restart; `afterSwitch` when it came straight after a switch. */
export function codexRestartFeedback(r: CodexRestart | 'busy' | 'error', model: Model, afterSwitch: boolean): FeedbackView {
  const v = model.codex;
  const name = longName(v, currentId(v));
  if (r === 'busy' || r === 'error' || r.kind === 'failed' || r.kind === 'noCodex') {
    const why =
      r === 'busy'
        ? 'Một lần chuyển khác đang chạy.'
        : r === 'error'
          ? 'Không gọi được lệnh.'
          : r.kind === 'noCodex'
            ? 'Không tìm thấy lệnh codex trên máy này.'
            : 'codex app-server daemon restart báo lỗi (' + r.detail + ').';
    return {
      tone: 'warn',
      title: (afterSwitch ? 'Đã chuyển Codex sang ' + name + ', ' : '') + 'chưa khởi động lại được tiến trình nền',
      body: why + ' Mở lại codex trong Terminal; Codex App vẫn phải thoát hẳn (⌘Q) rồi mở lại.',
      actions: [{ label: 'Đóng', act: 'closeFeedback' }],
    };
  }
  if (r.kind === 'notRunning')
    return {
      tone: '',
      title: (afterSwitch ? 'Đã chuyển Codex sang ' + name + '. ' : '') + 'Tiến trình nền Codex không chạy',
      body: 'Không có gì để khởi động lại. Lần mở codex tới sẽ dùng ' + name + '.',
    };
  return afterSwitch
    ? {
        tone: '',
        title: 'Đã chuyển Codex sang ' + name + ' và khởi động lại tiến trình nền',
        body: 'Mở lại codex trong Terminal để dùng ' + name + '. Codex App vẫn phải thoát hẳn (⌘Q) rồi mở lại.',
        actions: [{ label: 'Đã hiểu', act: 'closeFeedback' }],
      }
    : { tone: '', title: 'Tiến trình nền Codex đã khởi động lại', body: 'Mở lại codex trong Terminal để dùng ' + name + '.' };
}


// ===== "Dùng plugin khi chạy Custom API" =====

/** The checkbox's words: the label and its one-line hint. */
export const PLUGINS_LABEL = 'Dùng plugin khi chạy Custom API (giữ đăng nhập ChatGPT)';
export const PLUGINS_HINT = 'Bỏ tick: Codex không dùng đăng nhập ChatGPT khi chạy Custom API, plugin cần đăng nhập sẽ tắt.';

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
  const name = v.custom?.name ?? 'nhà cung cấp riêng';
  const want = wantsWith(v, on) ?? false;
  const sessions = v.procs?.sessions;
  return {
    title: 'Áp dụng ngay cho ' + name + '?',
    facts: [
      on
        ? { k: 'ok', text: 'Codex dùng lại đăng nhập ChatGPT khi chạy ' + name + ': plugin chạy như với tài khoản ChatGPT.' }
        : { k: 'w', text: '**Không dùng đăng nhập ChatGPT** khi chạy ' + name + ': plugin cần đăng nhập (apps, connector) sẽ tắt.' },
      { k: '', text: 'Chỉ đổi `requires_openai_auth = ' + want + '` trong bảng ' + name + '. Sao lưu trước khi ghi. Không đụng `auth.json`.' },
      {
        k: 'w',
        text: 'Thoát hẳn Codex App (⌘Q) rồi mở lại; ' + (sessions == null ? 'các phiên codex' : '**' + sessions + ' phiên codex**') + ' trong Terminal cần mở lại.',
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
  const name = v.custom?.name ?? 'nhà cung cấp riêng';
  switch (r.kind) {
    case 'noCustom':
      return 'config.toml chưa có bảng nhà cung cấp riêng để ghi khoá.';
    case 'envKey':
      return name + ' đọc khoá từ biến ' + r.var + ': đặt biến này, app không ghi gì.';
    case 'headers':
      return name + ' gửi khoá qua header: app không thêm khoá thứ hai.';
    case 'shape':
      return 'Bảng ' + name + ' viết gọn trên một dòng; app chỉ sửa bảng dạng [model_providers.' + (v.custom?.id ?? '…') + '].';
    case 'empty':
      return 'Chưa có khoá.';
    case 'notAKey':
      return 'Khoá không được có dấu cách hay xuống dòng.';
    case 'tooLong':
      return 'Khoá quá dài.';
    case 'placeholder':
      return 'PROXY_MANAGED là khoá giữ chỗ của CC Switch, không phải khoá thật.';
    case 'reference':
      return 'Đây là tên biến, không phải khoá.';
    case 'badUrl':
      return 'Địa chỉ phải bắt đầu bằng https:// (hoặc http://) và không chứa mật khẩu hay khoá.';
    case 'noUndo':
      return 'Không còn gì để hoàn tác.';
  }
}

/** "Hoàn tác" with what it can and cannot put back on the button itself. */
export function undoLabel(u: NonNullable<CodexView['keyUndo']>): string {
  const back = u.token === 'placeholder' ? 'trả lại PROXY_MANAGED' : u.token === 'empty' ? 'trả lại khoá rỗng' : 'bỏ khoá vừa dán';
  return 'Hoàn tác: ' + back + (u.baseUrl ? ' và địa chỉ cũ' : '');
}

/** The preview's facts: the backup, the login, the undo and its limit. */
export function keyFacts(v: CodexView, p: CodexKeyPreview): FactView[] {
  const facts: FactView[] = [
    { k: '', text: 'Chỉ sửa bảng ' + (v.custom?.name ?? '…') + ' trong `config.toml`. Sao lưu trước khi ghi, bản lưu không chứa khoá. Không đụng `auth.json`.' },
  ];
  if (p.replaces) facts.push({ k: 'w', text: 'Khoá cũ bị thay. App không giữ khoá cũ nên **không hoàn tác được**.' });
  else if (p.undo)
    facts.push({
      k: '',
      text: 'Có nút Hoàn tác: ' + (p.undo === 'placeholder' ? 'trả lại khoá giữ chỗ PROXY_MANAGED' : p.undo === 'empty' ? 'trả lại khoá rỗng' : 'bỏ khoá vừa dán') + '.',
    });
  if (v.current === 'custom') facts.push({ k: 'w', text: 'Codex đang dùng ' + (v.custom?.name ?? '…') + ': thoát hẳn Codex App (⌘Q) rồi mở lại để dùng khoá mới.' });
  return facts;
}

/** After "Dán khoá" or its undo; `undo`: what the preview said the undo puts back. Never names the key. */
export function keyFeedback(out: CodexKeyOutcome, model: Model, undo: NonNullable<CodexView['keyUndo']> | null = null): FeedbackView {
  const v = model.codex;
  const name = v.custom?.name ?? 'nhà cung cấp riêng';
  switch (out.kind) {
    case 'written':
      return {
        tone: '',
        title: 'Đã ghi khoá cho ' + name,
        body: (v.current === 'custom' ? restartLine(v, name) : 'Giờ chuyển Codex sang ' + name + ' được.') + ' Bản cũ của config.toml nằm trong thư mục sao lưu.',
        actions: [...(out.undo && undo ? [{ label: undoLabel(undo), act: 'codex:undoKey' as Action }] : []), CLOSE],
      };
    case 'undone':
      return { tone: '', title: 'Đã hoàn tác khoá của ' + name, body: 'Bảng ' + name + ' trở lại như trước khi dán khoá.' };
    case 'already':
      return { tone: 'info', title: name + ' đã có đúng khoá này', body: 'App không ghi gì.' };
    case 'refused':
      return { tone: 'bad', title: 'Chưa ghi khoá', body: keyRefusalText(v, out.refusal) + ' Chưa đổi gì.', actions: [{ label: 'Đóng', act: 'closeFeedback' }] };
    case 'invalid':
      return { tone: 'bad', title: 'Không sửa được config.toml', body: 'App không đọc được file này như Codex đọc (' + out.detail + '). Chưa đổi gì.', actions: [{ label: 'Đóng', act: 'closeFeedback' }] };
    case 'failed':
      return {
        tone: 'bad',
        title: 'Chưa ghi khoá',
        body:
          out.error.kind === 'changed'
            ? 'config.toml đã đổi sau khi xem trước. Chưa đổi gì: mở lại để xem bản mới.'
            : out.error.kind === 'verify'
              ? 'Đọc lại sau khi ghi không khớp. Bản cũ nằm trong thư mục sao lưu.'
              : 'Lỗi khi ghi (' + out.error.detail + '). Chưa đổi gì.',
        actions: [{ label: 'Xem thư mục sao lưu', act: 'codex:backup' }, { label: 'Đóng', act: 'closeFeedback' }],
      };
  }
}

/** The recent list's line for one entry, short. */
export function recentText(v: CodexView, r: CodexView['recent'][number]): string {
  const name = shortName(v, r.to);
  if (r.result === 'key') return name + ': dán khoá API';
  if (r.result === 'keyUndo') return name + ': hoàn tác khoá';
  if (r.result === 'plugins') return name + ': ' + (r.requires === false ? 'tắt plugin' : 'bật plugin');
  const arrow = shortName(v, r.from) + ' → ' + name;
  if (r.result === 'observed') return arrow + ' · đổi ngoài app';
  return arrow + (r.restarted ? ' · đã khởi động lại tiến trình nền' : '');
}
