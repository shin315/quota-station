// The variables that override a login (M14, I), in words: Cài đặt's card,
// the Claude tab's banner and the lines after "Mở file", "Bỏ qua" and
// "Kiểm lại". The app knows names and places only, never a value.

import { clock } from './format';
import { AGENT_NAME } from './mcp';
import type { EnvFound, EnvPlace, EnvView, Model } from './model';
import type { BannerView, ChipView, FeedbackView } from './view';

const FILES: Record<Exclude<EnvPlace, 'launchctl'>, string> = {
  zshenv: '~/.zshenv',
  zprofile: '~/.zprofile',
  zshrc: '~/.zshrc',
  bashProfile: '~/.bash_profile',
  profile: '~/.profile',
};

/** `~/.zshrc`, or `launchctl`. */
export function placeLabel(p: EnvPlace): string {
  return p === 'launchctl' ? 'launchctl' : FILES[p];
}

/** Under the name: `~/.zshrc · dòng 42`, or where launchd's variables go. */
export function whereLine(f: EnvFound): string {
  if (f.place === 'launchctl') return 'launchctl · app mở từ Dock';
  return placeLabel(f.place) + (f.line != null ? ' · dòng ' + f.line : '');
}

/** In a sentence: `~/.zshrc dòng 42`. */
function at(f: EnvFound): string {
  return placeLabel(f.place) + (f.line != null ? ' dòng ' + f.line : '');
}

export function agentChip(f: EnvFound): ChipView {
  return { t: AGENT_NAME[f.agent], k: f.weight };
}

/** "Mở ~/.zshrc ở dòng 42": only for a file. */
export function openLabel(f: EnvFound): string | null {
  if (f.place === 'launchctl') return null;
  return 'Mở ' + placeLabel(f.place) + (f.line != null ? ' ở dòng ' + f.line : '');
}

/** Under the rows: when, how much was looked at, that values are not read, and what "Bỏ qua" hides. */
export function checkedLine(env: EnvView): string {
  const when = env.checking ? 'Đang kiểm' : env.checkedAt ? 'Đã kiểm' : 'Chưa kiểm';
  const time = !env.checking && env.checkedAt ? ' lúc ' + clock(env.checkedAt) : '';
  const hidden = env.ignored ? ' Đã bỏ qua ' + env.ignored + ' chỗ.' : '';
  return when + ' ' + env.vars.length + ' biến ở ' + env.places.length + ' chỗ' + time + '. Chỉ biết biến có đặt hay không, không đọc giá trị.' + hidden;
}

/** Places that could not be looked at. */
export function unreadNote(env: EnvView): string | null {
  return env.unread.length ? 'Không đọc được ' + env.unread.map(placeLabel).join(', ') + '.' : null;
}

/** The Claude tab's red banner while a variable overrides Claude Code's login. */
export function envBanner(model: Model): BannerView | null {
  const claude = model.env.found.filter((f) => f.agent === 'claude');
  if (!claude.length) return null;
  if (claude.length === 1) {
    const f = claude[0];
    const open = openLabel(f);
    return {
      tone: 'bad',
      title: f.name + ' đang đè đăng nhập Claude',
      body: 'Đặt ở ' + at(f) + '. ' + f.effect,
      actions: [
        ...(open ? [{ label: open, act: `env:open:${f.id}` as const }] : []),
        { label: 'Bỏ qua', act: `env:ignore:${f.id}` },
        { label: 'Kiểm lại', act: 'env:check' },
      ],
    };
  }
  return {
    tone: 'bad',
    title: claude.length + ' biến đang đè đăng nhập Claude',
    body: claude.map((f) => f.name + ' ở ' + at(f)).join(', ') + '. Claude Code có thể không dùng tài khoản claude-swap đang chọn.',
    actions: [
      { label: 'Xem trong Cài đặt', act: 'env:settings' },
      { label: 'Kiểm lại', act: 'env:check' },
    ],
  };
}

/** After "Mở file": the editor went to the line, or the default one opened the file. */
export function openedFeedback(f: EnvFound, editor: string | null): FeedbackView {
  if (editor) return { tone: '', title: 'Đã mở ' + at(f) + ' trong ' + editor, body: 'App không sửa file shell.' };
  return {
    tone: '',
    title: 'Đã mở ' + placeLabel(f.place),
    body: (f.line != null ? f.name + ' ở dòng ' + f.line + '. ' : '') + 'App không sửa file shell.',
  };
}

export function openFailedFeedback(f: EnvFound | undefined): FeedbackView {
  return {
    tone: 'warn',
    title: 'Chưa mở được ' + (f ? placeLabel(f.place) : 'file'),
    body: 'Bấm Kiểm lại rồi thử lần nữa.',
    actions: [
      { label: 'Kiểm lại', act: 'env:check' },
      { label: 'Đóng', act: 'closeFeedback' },
    ],
  };
}

/** After "Bỏ qua", with its "Hoàn tác". */
export function ignoredFeedback(f: EnvFound): FeedbackView {
  return {
    tone: 'info',
    title: 'Đã bỏ qua ' + f.name + ' ở ' + placeLabel(f.place),
    body: 'Báo lại nếu biến được đặt ở chỗ khác.',
    actions: [{ label: 'Hoàn tác', act: `env:back:${f.id}` }],
  };
}

/** After "Kiểm lại". */
export function checkedFeedback(env: EnvView): FeedbackView {
  const n = env.found.length;
  const time = env.checkedAt ? ' lúc ' + clock(env.checkedAt) : '';
  return n
    ? { tone: 'warn', title: 'Đã kiểm' + time + ': ' + n + ' chỗ đặt biến đè đăng nhập', body: 'App không sửa file shell và không tự bỏ biến.' }
    : { tone: '', title: 'Đã kiểm' + time + ': không có biến nào đè đăng nhập', body: '' };
}

export function envFailedFeedback(): FeedbackView {
  return {
    tone: 'warn',
    title: 'Chưa làm được',
    body: 'Không có gì thay đổi.',
    actions: [{ label: 'Đóng', act: 'closeFeedback' }],
  };
}
