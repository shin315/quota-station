// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cài đặt › Chung › Cập nhật Agent in words: each tool's version, the
// newest for the way it was installed, and what "Cập nhật" does or why the
// app leaves it to the owner. The update is the tool's own command (`claude
// update`, `brew upgrade --cask codex`, `agy update`); a tool whose sessions
// would break waits until they are closed.

import { t } from './i18n';
import type { CliFailure, CliHandsOff, CliInstall, CliTool, CliToolView } from './model';

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
      return t('Có ' + sessions(open) + ' vừa mở nên chưa cập nhật. Đóng rồi bấm Kiểm lại.', sessions(open) + ' just opened, so nothing was updated. Close it, then press Check again.');
    }
    case 'sessions':
      return t(
        'Không đọc được các tiến trình đang chạy nên chưa cập nhật, để khỏi làm lỗi phiên đang mở. Thử lại sau ít phút.',
        'The running processes could not be read, so nothing was updated, to keep any open session from failing. Try again in a few minutes.',
      );
  }
}

/** What a tool's row shows. */
export interface CliRow {
  /** `0.163.0 → 0.164.0 · Homebrew`. */
  status: string;
  /** The line under it, when there is one to say. */
  note: { tone: 'ok' | 'bad' | 'warn' | ''; text: string } | null;
  action:
    | { kind: 'update' }
    | { kind: 'updating' }
    | { kind: 'recheck' }
    | { kind: 'latest' }
    | { kind: 'copy'; command: string }
    | { kind: 'install'; command: string }
    | { kind: 'none' };
}

/** A tool whose sessions would break waits; one that keeps them running does not. */
export function waitsForSessions(v: CliToolView): boolean {
  return v.running > 0 && !v.safeWhileRunning;
}

export function cliRow(v: CliToolView): CliRow {
  if (!v.version) return { status: t('Chưa cài trên Mac này.', 'Not installed on this Mac.'), note: null, action: { kind: 'install', command: CLI_INSTALL[v.tool] } };
  const how = v.install ? ' · ' + installText(v.install) : '';
  const status = v.newer && v.latest ? v.version + ' → ' + v.latest + how : v.version + (v.latest ? t(' · mới nhất', ' · newest') : '') + how;
  if (v.handsOff)
    return {
      status,
      note: { tone: 'warn', text: handsOffText(v, v.handsOff) },
      action: v.command && v.handsOff !== 'bundled' ? { kind: 'copy', command: v.command } : { kind: 'none' },
    };
  if (v.updating) return { status, note: { tone: '', text: t('Đang chạy `' + v.command + '`…', 'Running `' + v.command + '`…') }, action: { kind: 'updating' } };
  const kept = v.running > 0 ? t(' ' + sessions(v) + ' đang chạy giữ bản cũ tới khi mở lại.', ' ' + sessions(v) + ' running keep the old one until opened again.') : '';
  if (v.last?.ok && !v.newer)
    return { status, note: { tone: 'ok', text: t('Đã cập nhật ' + v.last.from + ' → ' + v.last.to + '.', 'Updated ' + v.last.from + ' → ' + v.last.to + '.') + (kept || t(' Phiên mở sau dùng bản mới.', ' The next session uses it.')) }, action: { kind: 'latest' } };
  const failed = v.last && !v.last.ok ? { tone: 'bad' as const, text: failureText(v, v.last.failure, v.last.detail) } : null;
  if (!v.newer) return { status, note: failed, action: v.latest ? { kind: 'latest' } : { kind: 'none' } };
  if (waitsForSessions(v))
    return {
      status,
      note: {
        tone: 'warn',
        text: t(
          sessions(v) + ' đang mở. Đóng chúng rồi bấm Kiểm lại; cập nhật lúc này có thể làm chúng lỗi.',
          sessions(v) + ' open. Close them, then press Check again; updating now could make them fail.',
        ),
      },
      action: { kind: 'recheck' },
    };
  return { status, note: failed ?? (kept ? { tone: '', text: kept.trim() } : null), action: { kind: 'update' } };
}
