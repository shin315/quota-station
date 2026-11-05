// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The variables that override a login (M14, I), in words: Cài đặt's card,
// the Claude tab's banner, "Gỡ"'s preview and the lines after "Mở file",
// "Bỏ qua", "Gỡ", "Hoàn tác" and "Kiểm lại". The app knows names and places
// only, never a value: a preview's line comes with the value masked.

import { clock } from './format';
import { t } from './i18n';
import { AGENT_NAME } from './mcp';
import type { EnvFound, EnvPlace, EnvPreview, EnvRefusal, EnvView, Model } from './model';
import type { BannerView, ChipView, FeedbackView } from './view';

/** The places that are no file: launchd, and Windows' user and system variables (the registry). */
type NoFile = 'launchctl' | 'userEnv' | 'machineEnv';

const FILES: Record<Exclude<EnvPlace, NoFile>, string> = {
  zshenv: '~/.zshenv',
  zprofile: '~/.zprofile',
  zshrc: '~/.zshrc',
  bashProfile: '~/.bash_profile',
  profile: '~/.profile',
  psProfile: '$PROFILE',
  pwshProfile: '$PROFILE (PowerShell 7)',
  bashrc: '~/.bashrc',
  claudeSettings: '~/.claude/settings.json',
  claudeLocalSettings: '~/.claude/settings.local.json',
};

/** A shell file, which "Mở file" opens at its line. */
export function isFile(p: EnvPlace): p is Exclude<EnvPlace, NoFile> {
  return p !== 'launchctl' && p !== 'userEnv' && p !== 'machineEnv';
}

/** "3 variables", "1 place": a count with its English noun. */
const plural = (n: number, one: string, many: string): string => n + ' ' + (n === 1 ? one : many);

/** `~/.zshrc`, `launchctl`, or Windows' "biến người dùng". */
export function placeLabel(p: EnvPlace): string {
  if (p === 'userEnv') return t('biến người dùng', 'user variables');
  if (p === 'machineEnv') return t('biến hệ thống', 'system variables');
  return p === 'launchctl' ? 'launchctl' : FILES[p];
}

/** A settings file of Claude Code: a key of its `env`, not a line. */
export function isJson(p: EnvPlace): boolean {
  return p === 'claudeSettings' || p === 'claudeLocalSettings';
}

/** "chỉ khi gọi hàm ccd", for a line inside a PowerShell function; null otherwise. */
export function onlyWhen(f: Pick<EnvFound, 'onlyIn'>): string | null {
  if (f.onlyIn == null) return null;
  return f.onlyIn ? t('chỉ khi gọi hàm ' + f.onlyIn, 'only when ' + f.onlyIn + ' is called') : t('chỉ khi gọi hàm chứa nó', 'only when its function is called');
}

/** Under the name: `~/.zshrc · dòng 42`, or where launchd's variables go. */
export function whereLine(f: EnvFound): string {
  if (f.place === 'launchctl') return t('launchctl · app mở từ Dock', 'launchctl · apps opened from the Dock');
  if (!isFile(f.place)) return placeLabel(f.place) + t(' · app mở từ Start', ' · apps opened from Start');
  const only = onlyWhen(f);
  return placeLabel(f.place) + (f.line != null ? t(' · dòng ', ' · line ') + f.line : '') + (only ? ' · ' + only : '');
}

/** In a sentence: `~/.zshrc dòng 42`, `$PROFILE dòng 5 (trong hàm ccd)`. */
function at(f: EnvFound): string {
  const inside = f.onlyIn != null ? t(' (trong hàm ' + (f.onlyIn || 'không tên') + ')', ' (in function ' + (f.onlyIn || 'without a name') + ')') : '';
  return placeLabel(f.place) + (f.line != null ? t(' dòng ', ' line ') + f.line + inside : '');
}

export function agentChip(f: EnvFound): ChipView {
  return { t: AGENT_NAME[f.agent], k: f.weight };
}

/** "Mở ~/.zshrc ở dòng 42": only for a file. */
export function openLabel(f: EnvFound): string | null {
  if (!isFile(f.place)) return null;
  return t('Mở ', 'Open ') + placeLabel(f.place) + (f.line != null ? t(' ở dòng ', ' at line ') + f.line : '');
}

/** Under the rows: when, how much was looked at, that values are not read, and what "Bỏ qua" hides. */
export function checkedLine(env: EnvView): string {
  const time = !env.checking && env.checkedAt ? clock(env.checkedAt) : '';
  const vars = plural(env.vars.length, 'variable', 'variables');
  const places = plural(env.places.length, 'place', 'places');
  const hidden = env.ignored ? t(' Đã bỏ qua ' + env.ignored + ' chỗ.', ' Ignored ' + plural(env.ignored, 'place', 'places') + '.') : '';
  const when = env.checking ? 'Đang kiểm tra' : env.checkedAt ? 'Đã kiểm tra' : 'Chưa kiểm tra';
  const what = env.checking
    ? 'Checking ' + vars + ' in ' + places + '.'
    : env.checkedAt
      ? 'Checked ' + vars + ' in ' + places + ' at ' + time + '.'
      : vars + ' in ' + places + ', not checked yet.';
  return (
    t(
      when + ' ' + env.vars.length + ' biến ở ' + env.places.length + ' chỗ' + (time ? ' lúc ' + time : '') + '. Chỉ biết biến có đặt hay không, không đọc giá trị.',
      what + ' The app only knows whether each is set, never its value.',
    ) + hidden
  );
}

/** Places that could not be looked at. */
export function unreadNote(env: EnvView): string | null {
  const list = env.unread.map(placeLabel).join(', ');
  return env.unread.length ? t('Không đọc được ' + list + '.', 'Could not read ' + list + '.') : null;
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
      title: t(f.name + ' đang đè đăng nhập Claude', f.name + ' is overriding the Claude sign-in'),
      body: t('Đặt ở ' + at(f) + '. ', 'Set in ' + at(f) + '. ') + f.effect,
      actions: [
        ...(open ? [{ label: open, act: `env:open:${f.id}` as const }] : []),
        { label: t('Bỏ qua', 'Ignore'), act: `env:ignore:${f.id}` },
        { label: t('Kiểm tra lại', 'Check again'), act: 'env:check' },
      ],
    };
  }
  return {
    tone: 'bad',
    title: t(claude.length + ' biến đang đè đăng nhập Claude', claude.length + ' variables are overriding the Claude sign-in'),
    body: t(
      claude.map((f) => f.name + ' ở ' + at(f)).join(', ') + '. Claude Code có thể không dùng tài khoản Claude đang chọn.',
      claude.map((f) => f.name + ' in ' + at(f)).join(', ') + '. Claude Code may not use the chosen Claude account.',
    ),
    actions: [
      { label: t('Xem trong Cài đặt', 'See in Settings'), act: 'env:settings' },
      { label: t('Kiểm tra lại', 'Check again'), act: 'env:check' },
    ],
  };
}

/** After "Mở file": the editor went to the line, or the default one opened the file. */
export function openedFeedback(f: EnvFound, editor: string | null): FeedbackView {
  const noEdit = t('App chỉ sửa file khi bạn bấm Gỡ.', 'The app edits a file only when you press Remove.');
  if (editor) return { tone: '', title: t('Đã mở ' + at(f) + ' trong ' + editor, 'Opened ' + at(f) + ' in ' + editor), body: noEdit };
  return {
    tone: '',
    title: t('Đã mở ', 'Opened ') + placeLabel(f.place),
    body: (f.line != null ? t(f.name + ' ở dòng ' + f.line + '. ', f.name + ' is on line ' + f.line + '. ') : '') + noEdit,
  };
}

export function openFailedFeedback(f: EnvFound | undefined): FeedbackView {
  return {
    tone: 'warn',
    title: f ? t('Chưa mở được ', 'Could not open ') + placeLabel(f.place) : t('Chưa mở được file', 'Could not open the file'),
    body: t('Bấm Kiểm tra lại rồi thử lần nữa.', 'Press Check again, then try once more.'),
    actions: [
      { label: t('Kiểm tra lại', 'Check again'), act: 'env:check' },
      { label: t('Đóng', 'Close'), act: 'closeFeedback' },
    ],
  };
}

/** After "Bỏ qua", with its "Hoàn tác". */
export function ignoredFeedback(f: EnvFound): FeedbackView {
  return {
    tone: 'info',
    title: t('Đã bỏ qua ' + f.name + ' ở ' + placeLabel(f.place), 'Ignoring ' + f.name + ' in ' + placeLabel(f.place)),
    body: t('Báo lại nếu biến được đặt ở chỗ khác.', 'You will hear again if it is set somewhere else.'),
    actions: [{ label: t('Hoàn tác', 'Undo'), act: `env:back:${f.id}` }],
  };
}

/** After "Kiểm tra lại". */
export function checkedFeedback(env: EnvView): FeedbackView {
  const n = env.found.length;
  const time = env.checkedAt ? clock(env.checkedAt) : '';
  const vi = 'Đã kiểm tra' + (time ? ' lúc ' + time : '');
  const en = 'Checked' + (time ? ' at ' + time : '');
  return n
    ? {
        tone: 'warn',
        title: t(vi + ': ' + n + ' chỗ đặt biến đè đăng nhập', en + ': ' + plural(n, 'place sets', 'places set') + ' a variable that overrides sign-in'),
        body: t('App không tự sửa file hay bỏ biến. Bấm Gỡ ở từng dòng để xoá đúng dòng đó.', 'The app does not edit files or unset variables by itself. Press Remove on a row to take out just that line.'),
      }
    : { tone: '', title: t(vi + ': không có biến nào đè đăng nhập', en + ': no variable overrides a sign-in'), body: '' };
}

export function envFailedFeedback(): FeedbackView {
  return {
    tone: 'warn',
    title: t('Chưa làm được', 'Could not do that'),
    body: t('Không có gì thay đổi.', 'Nothing changed.'),
    actions: [{ label: t('Đóng', 'Close'), act: 'closeFeedback' }],
  };
}

/** Why "Gỡ" will not take a place out, in a sentence. */
export function refusalText(r: EnvRefusal | string): string {
  switch (r) {
    case 'multiline':
      return t('Giá trị kéo sang dòng sau (dấu nháy hay ngoặc chưa đóng), nên app không biết xoá tới đâu.', 'The value goes on past this line (an open quote or bracket), so the app cannot tell where it ends.');
    case 'continued':
      return t('Dòng này nối với dòng trước hoặc dòng sau (\\, dấu ` hay |), nên xoá riêng nó sẽ làm hỏng lệnh.', 'This line is joined to the one before or after it (\\, ` or |), so taking it out alone would break the command.');
    case 'hereString':
      return t('Giá trị là here-string (@\'…\'@) trải nhiều dòng.', 'The value is a here-string (@\'…\'@) over several lines.');
    case 'several':
      return t('Dòng này còn làm việc khác ngoài đặt biến (biến khác, lệnh khác hay dấu đóng khối).', 'This line does more than set the variable (another variable, a command or the end of a block).');
    case 'json':
      return t('File không phải JSON hợp lệ.', 'The file is not valid JSON.');
    case 'unreadable':
      return t('Không đọc được file.', 'The file could not be read.');
    case 'noFile':
      return t('Biến này không nằm trong file.', 'This variable is not set in a file.');
    case 'changed':
      return t('File đã đổi sau khi xem trước. Không có gì bị xoá.', 'The file changed after the preview. Nothing was removed.');
    default:
      return t('Biến không còn ở chỗ này. Bấm Kiểm tra lại.', 'The variable is no longer there. Press Check again.');
  }
}

/** After "Gỡ", with its "Hoàn tác". */
export function removedFeedback(p: EnvPreview): FeedbackView {
  return {
    tone: 'info',
    title: t('Đã gỡ ' + p.name + ' khỏi ' + placeLabel(p.place), 'Removed ' + p.name + ' from ' + placeLabel(p.place)),
    body: t('Phần đã xoá được giữ trong thư mục riêng của app để hoàn tác.', 'What was removed is kept in the app’s own folder for Undo.'),
    actions: [{ label: t('Hoàn tác', 'Undo'), act: `env:restore:${p.id}` }],
  };
}

/** "Gỡ" did not happen. */
export function removeFailedFeedback(p: EnvPreview | undefined, why: string): FeedbackView {
  return {
    tone: 'warn',
    title: p ? t('Chưa gỡ được ' + p.name, 'Could not remove ' + p.name) : t('Chưa gỡ được', 'Could not remove it'),
    body: why === 'io' ? t('Không ghi được file. Không có gì thay đổi.', 'The file could not be written. Nothing changed.') : refusalText(why),
    actions: [
      { label: t('Kiểm tra lại', 'Check again'), act: 'env:check' },
      { label: t('Đóng', 'Close'), act: 'closeFeedback' },
    ],
  };
}

/** After "Hoàn tác" of a "Gỡ". */
export function restoredFeedback(p: EnvPreview): FeedbackView {
  return { tone: '', title: t('Đã đặt lại ' + p.name + ' ở ' + placeLabel(p.place), 'Put ' + p.name + ' back in ' + placeLabel(p.place)), body: '' };
}

export function restoreFailedFeedback(why: string): FeedbackView {
  return {
    tone: 'warn',
    title: t('Chưa hoàn tác được', 'Could not undo'),
    body:
      why === 'changed'
        ? t('File đã đổi sau khi gỡ, nên app không ghi đè. Mở file để thêm lại dòng đó nếu cần.', 'The file changed after the removal, so the app does not overwrite it. Open the file to add the line back if you need it.')
        : why === 'missing'
          ? t('Không còn bản để hoàn tác.', 'There is nothing left to undo.')
          : t('Không ghi được file. Không có gì thay đổi.', 'The file could not be written. Nothing changed.'),
    actions: [{ label: t('Đóng', 'Close'), act: 'closeFeedback' }],
  };
}
