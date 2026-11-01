// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Agent › Cập nhật (Cập nhật Agent) in words: each tool's version, whether
// it is the newest for the way it was installed, and what "Cập nhật" does or
// why the app leaves it to the owner; a line at the top says it for all.
// The update is the tool's own command (`claude update`, `brew upgrade
// --cask codex`, `agy update`); a tool whose sessions would break waits
// until they are closed.

import { clock, pad } from './format';
import { t } from './i18n';
import type { CliFailure, CliHandsOff, CliInstall, CliTool, CliToolsView, CliToolView } from './model';

export const CLI_NAME: Record<CliTool, string> = { claude: 'Claude Code', codex: 'Codex CLI', agy: 'Antigravity CLI' };

/** Each tool's official way in, for one not installed. */
export const CLI_INSTALL: Record<CliTool, string> = {
  claude: 'curl -fsSL https://claude.ai/install.sh | bash',
  codex: 'brew install --cask codex',
  agy: 'curl -fsSL https://antigravity.google/cli/install.sh | bash',
};

export function installText(install: CliInstall): string {
  switch (install) {
    case 'homebrew':
      return 'Homebrew';
    case 'npm':
      return 'npm';
    default:
      return t('trình cài chính thức', 'official installer');
  }
}

/** Its sessions, counted: `3 phiên Claude Code`. */
function sessions(v: CliToolView): string {
  return t(v.running + ' phiên ' + CLI_NAME[v.tool], v.running + ' ' + CLI_NAME[v.tool] + (v.running === 1 ? ' session' : ' sessions'));
}

export function handsOffText(v: CliToolView, kind: CliHandsOff): string {
  switch (kind) {
    case 'npm':
      return t(
        'Cài qua npm, cách cũ: cập nhật ghi đè file mà phiên đang chạy dùng. Chạy `claude install` trong Terminal để chuyển sang trình cài chính thức; từ đó app cập nhật được.',
        'Installed with npm, the old way: an update writes over the files a running session uses. Run `claude install` in Terminal to move to the official installer; the app can update it from then on.',
      );
    case 'bundled':
      return t(
        'Bản này nằm trong ' + (v.bundledBy ?? 'một ứng dụng khác') + ' và do ứng dụng đó tự cập nhật. App không đụng vào.',
        'This copy is inside ' + (v.bundledBy ?? 'another app') + ', which updates it itself. The app leaves it alone.',
      );
    case 'unwritable':
      return t(
        'Thư mục cài cần quyền quản trị, mà app không bao giờ xin. Chạy lệnh bên dưới trong Terminal.',
        'Its folder needs an administrator, which the app never asks for. Run the command below in Terminal.',
      );
    case 'unknown':
      return t(
        'Không nhận ra cách cài nên app không đoán lệnh. Cập nhật theo cách bạn đã cài.',
        'The app cannot tell how it was installed, so it does not guess a command. Update it the way you installed it.',
      );
  }
}

export function failureText(v: CliToolView, failure: CliFailure, detail: string): string {
  switch (failure) {
    case 'offline':
      return t('Không tải được bản mới: không tới được máy chủ. Thử lại khi có mạng.', 'Could not download the new version: the server was not reachable. Try again when online.');
    case 'permission':
      return t('Không ghi được vào ' + detail + '. Cần quyền quản trị: chạy `sudo ' + (v.command ?? '') + '` trong Terminal.', 'Could not write to ' + detail + '. It needs an administrator: run `sudo ' + (v.command ?? '') + '` in Terminal.');
    case 'busy':
      return v.install === 'homebrew'
        ? t('Homebrew đang chạy một lệnh khác. Đợi nó xong rồi thử lại.', 'Homebrew is running another command. Wait for it to finish, then try again.')
        : t('Công cụ đang tự cập nhật ở nền. Đợi một phút rồi thử lại.', 'The tool is updating itself in the background. Wait a minute, then try again.');
    case 'exit':
      return t('`' + v.command + '` dừng lỗi: ' + detail, '`' + v.command + '` failed: ' + detail);
    case 'timeout':
      return t('Quá 10 phút chưa xong nên đã dừng. Phiên bản giữ nguyên.', 'Not done after 10 minutes, so it was stopped. The version is unchanged.');
    case 'unchanged':
      return t('Lệnh báo xong nhưng phiên bản vẫn là ' + v.version + '. Thử lại sau ít phút: bản mới có thể chưa tới kho.', 'The command finished but the version is still ' + v.version + '. Try again in a few minutes: the new version may not be in the repository yet.');
    case 'started': {
      const n = Number(detail) || v.running || 1;
      const open = { ...v, running: n };
      return t('Có ' + sessions(open) + ' vừa mở nên chưa cập nhật. Đóng rồi bấm Kiểm tra lại.', sessions(open) + ' just opened, so nothing was updated. Close it, then press Check again.');
    }
    case 'sessions':
      return t(
        'Không đọc được các tiến trình đang chạy nên chưa cập nhật, để khỏi làm lỗi phiên đang mở. Thử lại sau ít phút.',
        'The running processes could not be read, so nothing was updated, to keep any open session from failing. Try again in a few minutes.',
      );
  }
}

/** Whether a tool is the newest, as the last check found (or the one running now). */
export type CliCheck =
  | { kind: 'latest' }
  | { kind: 'newer' }
  | { kind: 'checking' }
  /** Its feed did not answer; `why` says how. */
  | { kind: 'failed'; why: string }
  | { kind: 'unchecked' }
  /** Nowhere to ask: a copy inside an app, or an install the app does not know. */
  | { kind: 'none' };

/** When the last check ended: `20:41`, with its day (`26/10 20:41`) when that was not today. */
export function checkedClock(at: number, now: Date): string {
  const d = new Date(at * 1000);
  const time = clock(d.toISOString());
  return d.toDateString() === now.toDateString() ? time : pad(d.getDate()) + '/' + pad(d.getMonth() + 1) + ' ' + time;
}

/** Why a feed did not answer, short. */
export function failedText(why: string): string {
  if (why === 'offline') return t('không tới được máy chủ', 'the server was not reachable');
  if (why === 'unreadable') return t('máy chủ không trả về số phiên bản', 'the server sent no version');
  return t('máy chủ trả lỗi ' + why, 'the server answered ' + why);
}

/** Whether `v` is the newest, from the view's last check and the one running now. */
export function cliCheck(v: CliToolView, view: Pick<CliToolsView, 'checking' | 'checkedAt'>): CliCheck {
  if (view.checking) return { kind: 'checking' };
  if (v.newer && v.latest) return { kind: 'newer' };
  if (v.failed) return { kind: 'failed', why: v.failed };
  if (v.latest) return { kind: 'latest' };
  if (!view.checkedAt) return { kind: 'unchecked' };
  return { kind: 'none' };
}

/** A tool whose sessions would break waits; one that keeps them running does not. */
export function waitsForSessions(v: CliToolView): boolean {
  return v.running > 0 && !v.safeWhileRunning;
}

/**
 * What a tool's row shows: its name and one line, the version and where it
 * stands; the longer words wait behind "Chi tiết".
 */
export interface CliRow {
  /** `0.163.0 → 0.164.0 · Có bản mới`, in the colour of what it says. */
  line: { tone: 'ok' | 'info' | 'warn' | 'bad' | 'off' | ''; text: string };
  /** Whether it is the newest; null when not installed or nowhere to ask. */
  check: Exclude<CliCheck['kind'], 'none'> | null;
  /** Behind "Chi tiết": how it was installed, when it was checked, and why. */
  more: string[];
  /** The command to run in Terminal (an install, or an update the app leaves to the owner), behind "Chi tiết". */
  command: string | null;
  action: { kind: 'update' } | { kind: 'updating' } | { kind: 'recheck' } | { kind: 'none' };
}

/** A row as of `view`: the check running now and when the last one ended (`now` dates an older one). */
export function cliRow(v: CliToolView, view: Pick<CliToolsView, 'checking' | 'checkedAt'>, now: Date): CliRow {
  const none = { kind: 'none' } as const;
  if (!v.version)
    return {
      line: { tone: 'off', text: t('Chưa cài trên Mac này', 'Not installed on this Mac') },
      check: null,
      more: [t('Cài bằng lệnh chính thức, trong Terminal:', 'Install it with its official command, in Terminal:')],
      command: CLI_INSTALL[v.tool],
      action: none,
    };
  const c = cliCheck(v, view);
  const version = v.newer && v.latest ? v.version + ' → ' + v.latest : v.version;
  const say = (tone: CliRow['line']['tone'], state: string): CliRow['line'] => ({ tone, text: state ? version + ' · ' + state : version });
  const more: string[] = [];
  if (v.install) more.push(t('Cài bằng ', 'Installed with ') + installText(v.install) + '.');
  if (view.checkedAt && c.kind !== 'checking') more.push(t('Kiểm tra lúc ', 'Checked at ') + checkedClock(view.checkedAt, now) + '.');
  const row = (line: CliRow['line'], action: CliRow['action'] = none, command: string | null = null): CliRow => ({
    line,
    check: c.kind === 'none' ? null : c.kind,
    more,
    command,
    action,
  });
  const state = (): CliRow['line'] => {
    switch (c.kind) {
      case 'latest':
        return say('ok', t('Đã là bản mới nhất', 'Already the newest'));
      case 'newer':
        return say('info', t('Có bản mới', 'New version available'));
      case 'checking':
        return say('', t('Đang kiểm tra…', 'Checking…'));
      case 'failed':
        return say('bad', t('Không kiểm tra được: ', 'Could not check: ') + failedText(c.why));
      case 'unchecked':
        return say('off', t('Chưa kiểm tra', 'Not checked yet'));
      case 'none':
        return say('', '');
    }
  };
  if (v.handsOff) {
    more.push(handsOffText(v, v.handsOff));
    const bundled = v.handsOff === 'bundled';
    const left = bundled ? t('Do ' + (v.bundledBy ?? 'ứng dụng khác') + ' tự cập nhật', 'Updated by ' + (v.bundledBy ?? 'another app')) : t('Cập nhật trong Terminal', 'Update in Terminal');
    const line = c.kind === 'newer' ? say('warn', left) : c.kind === 'none' ? say('', left) : state();
    return row(line, none, bundled ? null : v.command);
  }
  if (v.updating) {
    more.push(t('Đang chạy `' + v.command + '`.', 'Running `' + v.command + '`.'));
    return row(say('', t('Đang cập nhật…', 'Updating…')), { kind: 'updating' });
  }
  if (v.running > 0 && v.safeWhileRunning) more.push(t(sessions(v) + ' đang chạy giữ bản cũ tới khi mở lại.', sessions(v) + ' running keep the old one until opened again.'));
  if (v.last && !v.last.ok) more.push(failureText(v, v.last.failure, v.last.detail));
  if (v.last?.ok && !v.newer && c.kind !== 'checking') {
    if (!v.running) more.push(t('Phiên mở sau dùng bản mới.', 'The next session uses it.'));
    return row(say('ok', t('Đã cập nhật từ ' + v.last.from, 'Updated from ' + v.last.from)));
  }
  if (!v.newer || c.kind === 'checking') return row(state());
  if (waitsForSessions(v)) {
    more.push(
      t(
        sessions(v) + ' đang mở. Đóng chúng rồi bấm Kiểm tra lại; cập nhật lúc này có thể làm chúng lỗi.',
        sessions(v) + ' open. Close them, then press Check again; updating now could make them fail.',
      ),
    );
    return row(say('warn', t('Chờ đóng ' + sessions(v), 'Waiting for ' + sessions(v) + ' to close')), { kind: 'recheck' });
  }
  if (v.last && !v.last.ok) return row(say('bad', t('Cập nhật không xong', 'Update did not finish')), { kind: 'update' });
  return row(state(), { kind: 'update' });
}

/** The page's line at the top: every tool at once. */
export interface CliSummary {
  tone: 'ok' | 'info' | 'warn' | 'bad' | 'stale' | 'off';
  text: string;
  /** `kiểm tra lúc 20:41`, after the text. */
  when: string | null;
}

export function cliSummary(view: CliToolsView, now: Date): CliSummary {
  const at = view.checkedAt ? checkedClock(view.checkedAt, now) : null;
  const when = at ? t('kiểm tra lúc ' + at, 'checked at ' + at) : null;
  const installed = view.tools.filter((v) => v.version);
  if (view.checking) return { tone: 'stale', text: t('Đang kiểm tra bản mới…', 'Checking for new versions…'), when: null };
  if (!installed.length) return { tone: 'off', text: t('Chưa cài công cụ nào trên Mac này.', 'None of the tools is installed on this Mac.'), when };
  if (!view.checkedAt) return { tone: 'off', text: t('Chưa kiểm tra lần nào.', 'Not checked yet.'), when: null };
  const newer = installed.filter((v) => v.newer && v.latest);
  const failed = installed.filter((v) => !(v.newer && v.latest) && v.failed);
  const names = (vs: CliToolView[]) => vs.map((v) => CLI_NAME[v.tool]).join(', ');
  const cant = failed.length ? t('không kiểm tra được ' + names(failed), 'could not check ' + names(failed)) : '';
  if (newer.length) {
    const head =
      newer.length === 1
        ? t(CLI_NAME[newer[0].tool] + ' có bản mới ' + newer[0].latest, CLI_NAME[newer[0].tool] + ' ' + newer[0].latest + ' is out')
        : t(newer.length + ' công cụ có bản mới', newer.length + ' tools have a new version');
    return { tone: 'info', text: head + (cant ? ' · ' + cant : ''), when };
  }
  if (view.offline) return { tone: 'bad', text: t('Không kiểm tra được: không tới được máy chủ nào', 'Could not check: no server was reachable'), when };
  if (failed.length) {
    const cap = cant.charAt(0).toUpperCase() + cant.slice(1);
    return { tone: 'warn', text: cap, when };
  }
  // A tool with nowhere to ask (inside an app, an unknown install) is not counted.
  const asked = installed.filter((v) => v.latest);
  if (!asked.length) return { tone: 'off', text: t('Không có công cụ nào để kiểm tra', 'No tool to check'), when };
  return { tone: 'ok', text: installed.length === 1 ? t('Đã là bản mới nhất', 'Already the newest') : t('Tất cả đã là bản mới nhất', 'All are the newest'), when };
}
