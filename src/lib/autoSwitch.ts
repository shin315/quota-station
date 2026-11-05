// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// "Tự chuyển khi hết quota", the main feature's words: its card in the
// Claude tab and its line at the popover's foot, shown with Cài đặt › Nâng
// cao off. The switch is the rotation's job (LaunchAgent on a Mac, the
// app's timer on Windows); without a rotation rules file it moves Claude
// when the account in use reaches the threshold (Settings' `autoSwitchAt`)
// in its fullest window. Nâng cao shows the rotation's own card instead.
// Pure, like `rotation.ts`, whose pause it shares.

import { advancedOn } from './advanced';
import { t } from './i18n';
import type { Model } from './model';
import { pauseOf, pausedLine, switcherOf, type PauseState } from './rotation';
import { dayClock, namesOf } from './switching';

/** A switch the rotation logged this long ago or less shows as the latest news. */
const RECENT_MS = 12 * 3600 * 1000;

export interface SimpleAutoView {
  /** The card and the foot line show: Nâng cao off, and two accounts or more (or it is on). */
  shown: boolean;
  title: string;
  /** The rotation runs here (its LaunchAgent or the app's timer). */
  on: boolean;
  /** The switch cannot be pressed now. */
  disabled: boolean;
  /** The status: watching, the last switch, paused until when, or why it cannot run. */
  line: string;
  /** The status line is a problem to act on. */
  warn: boolean;
  paused: PauseState | null;
  /** The pause button shows: on and not paused. */
  canPause: boolean;
  /** The older switcher still loaded. */
  legacy: string | null;
}

/** `Tự chuyển khi hết quota`. */
export function autoTitle(): string {
  return t('Tự chuyển khi hết quota', 'Auto-switch when quota runs out');
}

/** What the threshold does, in one line: the off state and Cài đặt say it. */
export function thresholdWords(at: number): string {
  return t(
    'Khi tài khoản đang dùng tới ' + at + '% (5 giờ hoặc tuần), chuyển sang tài khoản còn nhiều quota nhất.',
    'When the account in use reaches ' + at + '% (5-hour or week), switch to the account with the most quota left.',
  );
}

export function simpleAutoView(model: Model, now: Date): SimpleAutoView {
  const v = model.claudeAccounts;
  const sw = switcherOf(model);
  const nameOf = namesOf(model);
  const at = v.autoSwitchAt;
  const on = v.rotation === true;
  const enough = v.accounts.length >= 2;
  const paused = on ? pauseOf(model, now) : null;
  const legacy = v.legacy;
  let line: string;
  let warn = false;
  if (!v.ready || !enough) line = t('Cần ít nhất hai tài khoản Claude.', 'Needs at least two Claude accounts.');
  else if (legacy) {
    warn = true;
    line = t('Bộ tự chuyển cũ ' + legacy + ' còn chạy: tắt nó trước.', 'The old auto-switcher ' + legacy + ' is still running: turn it off first.');
  } else if (!on) line = t('Đang tắt. ', 'Off. ') + thresholdWords(at);
  else if (paused) line = pausedLine(paused, now) + '.';
  else if (v.autoPolicy === 'stopped') {
    warn = true;
    line = t(
      'File luật xoay vòng có lỗi nên chưa chuyển gì. Sửa nó trong Luật xoay vòng (Nâng cao).',
      'The rotation rules file has an error, so nothing switches. Fix it in Rotation rules (Advanced).',
    );
  } else {
    const last = sw.lastLog;
    const recent =
      last && last.event.kind === 'switch' && last.at && now.getTime() - new Date(last.at).getTime() <= RECENT_MS && now.getTime() >= new Date(last.at).getTime();
    const watching =
      v.autoPolicy === 'rules'
        ? t('Đang theo dõi theo luật xoay vòng (Nâng cao)', 'Watching, by the rotation rules (Advanced)')
        : t('Đang theo dõi · chuyển ở ' + at + '%', 'Watching · switches at ' + at + '%');
    if (sw.dryRun?.decision.kind === 'wait') {
      warn = true;
      line = t(
        'Không tài khoản nào còn dưới ' + at + '%: giữ tài khoản đang dùng.',
        'No account is under ' + at + '%: keeping the account in use.',
      );
    } else if (recent && last.event.kind === 'switch')
      line =
        t('Đã chuyển sang ' + nameOf(last.event.slot) + ' lúc ' + dayClock(last.at!, now), 'Switched to ' + nameOf(last.event.slot) + ' at ' + dayClock(last.at!, now)) +
        ' · ' +
        watching.charAt(0).toLowerCase() +
        watching.slice(1);
    else line = watching + '.';
  }
  return {
    shown: !advancedOn(model) && (enough || on || !!legacy),
    title: autoTitle(),
    on,
    disabled: !v.ready || v.working != null || (!on && (!enough || !!legacy)),
    line,
    warn,
    paused,
    canPause: on && !paused && !legacy && enough,
    legacy,
  };
}
