// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Turns the model into what the main window shows (M2, read only): the
// Tổng quan, Claude and Antigravity tabs. Built on the popover's view, so the
// window and the popover show the same snapshot with the same names, numbers
// and banners. Pure, like `view.ts`.

import { overwriteBanner } from '@agy/agySwitch';
import { envBanner } from './env';
import { mac, os, t } from './i18n';
import type { Model, Section } from './model';
import { autoView, dayClock, pauseOf, type AutoView } from './switching';
import { popoverView, type BannerView, type CardView, type ChipView, type NoteView, type PopoverView, type ViewOptions } from './view';

export interface KeyValue {
  key: string;
  value: string;
  chip?: ChipView;
  /** Shown in a monospace face: a line as a program printed it. */
  mono?: boolean;
}

export interface WindowView {
  machine: string;
  fresh: PopoverView['fresh'];
  banner: BannerView | null;
  claude: {
    /** False on a Mac without Claude Code: the card is left out. */
    present: boolean;
    /** Claude Code is there with no account yet: the card offers to add one. */
    empty: boolean;
    /** `4 tài khoản Quota Station`; `claude-swap` until Quota Station keeps the accounts. */
    count: string;
    /** In slot order, as the tables list them. */
    cards: CardView[];
    note: NoteView | null;
  };
  agy: {
    /** False on a Mac without Antigravity: the tab shows an empty state. */
    present: boolean;
    cards: CardView[];
    /** Section note for the overview card. */
    note: NoteView | null;
    /**
     * The Antigravity tab's banner: a switch Antigravity.app wrote back
     * ("Chuyển Antigravity bị ghi đè"), else why only the signed-in account
     * shows.
     */
    banner: BannerView | null;
    /** The Antigravity tab's note: the Antigravity read gave no number. */
    tabNote: NoteView | null;
    /** The row lists the app's own accounts: the logins are the table's rows. */
    own: boolean;
  };
  /** Automatic switching and why Claude is on its account (F07). */
  auto: AutoView;
  /** The Claude tab's own banner: the rotation's LaunchAgent is not loaded. */
  claudeBanner: BannerView | null;
  /** The Claude tab's red banner: a variable overrides Claude Code's login (M14). */
  envBanner: BannerView | null;
  /** The Claude tab's Switcher card: one line, the rest folded. */
  switcher: { line: string; tone: 'ok' | 'warn' | 'bad' | 'muted'; details: KeyValue[] };
}

// ===== Antigravity =====

function agyBanner(g: Section): BannerView | null {
  if (g.status === 'single')
    return {
      tone: 'info',
      title: t('Chỉ thấy tài khoản đang dùng', 'Only the account in use shows'),
      body: mac(
        t(
          'Chưa có danh sách tài khoản Antigravity trên máy này. Đăng nhập tài khoản ở đây để thấy và chuyển mọi tài khoản.',
          'There is no Antigravity account list on this Mac yet. Sign in to the accounts here to see and switch them all.',
        ),
      ),
      actions: [],
    };
  if (g.status === 'unavailable')
    return {
      tone: 'warn',
      title: t('Không đọc được danh sách tài khoản', 'Cannot read the account list'),
      body: t('Chỉ thấy tài khoản đang dùng', 'Only the account in use shows') + (g.reason ? ' (' + g.reason + ')' : '') + '.',
      actions: [],
    };
  return null;
}

// ===== The switcher (F07) =====

function switcherCard(model: Model, auto: AutoView, now: Date): WindowView['switcher'] {
  const sw = model.switcher;
  const chip = (text: string, k: ChipView['k']): ChipView => ({ t: text, k });
  const service = !sw.installed
    ? chip(t('chưa có tài khoản', 'no account'), 'bad')
    : sw.service === true
      ? chip(t('chạy mỗi 3 phút', 'runs every 3 minutes'), 'ok')
      : sw.service === false
        ? chip(t('tắt', 'off'), 'bad')
        : chip(t('không rõ', 'unknown'), '');
  const legacy = sw.legacy ? chip(t(sw.legacy + ' đang nạp', sw.legacy + ' is loaded'), 'warn') : chip(t('tắt', 'off'), 'ok');
  const p = pauseOf(model, now);
  const details: KeyValue[] = [
    { key: os(t('LaunchAgent tự chuyển', 'Auto-switch LaunchAgent'), t('Hẹn giờ tự chuyển', 'Auto-switch timer')), value: service.t, chip: service },
    { key: t('Bộ tự chuyển cũ', 'Old auto-switcher'), value: legacy.t, chip: legacy },
    {
      key: t('Tạm dừng tự chuyển', 'Auto-switch paused'),
      value: p ? (p.until ? t('đến ', 'until ') + dayClock(p.until, now) : t('đến khi bật lại', 'until turned back on')) : t('không', 'no'),
    },
  ];
  if (sw.guard != null) {
    const guard = sw.guard ? chip(t('có', 'yes'), 'ok') : chip(t('chưa', 'not yet'), 'bad');
    details.push({ key: t('Switcher đọc cờ tạm dừng', 'Switcher reads the pause flag'), value: guard.t, chip: guard });
  }
  if (auto.raw) details.push({ key: t('Quyết định gần nhất', 'Latest decision') + (auto.at ? ' ' + auto.at : ''), value: auto.raw, mono: true });
  if (sw.lastLog) details.push({ key: t('Nhật ký', 'Log') + (sw.lastLog.at ? ' ' + dayClock(sw.lastLog.at, now) : ''), value: sw.lastLog.raw, mono: true });
  let line: string;
  let tone: WindowView['switcher']['tone'];
  if (!sw.installed) [line, tone] = [t('Quota Station chưa giữ tài khoản Claude nào', 'Quota Station keeps no Claude account yet'), 'bad'];
  else if (sw.legacy)
    [line, tone] = [t('Bộ tự chuyển cũ ' + sw.legacy + ' còn chạy: Quota Station chưa tự chuyển', 'The old auto-switcher ' + sw.legacy + ' still runs: Quota Station does not switch yet'), 'bad'];
  else if (sw.service === false) [line, tone] = [t('Tự chuyển đang tắt', 'Auto-switch is off'), 'bad'];
  else if (sw.service === true) [line, tone] = [auto.at ? t('Kiểm lúc ' + auto.at, 'Checked at ' + auto.at) : t('Đang chạy', 'Running'), 'ok'];
  else [line, tone] = [t('Không rõ switcher có chạy không', 'Not sure the switcher runs'), 'muted'];
  return { line, tone, details };
}

function claudeBanner(model: Model): BannerView | null {
  const sw = model.switcher;
  if (!sw.installed || sw.legacy || sw.service !== false) return null;
  return {
    tone: 'bad',
    title: t('Tự chuyển Claude đang tắt', 'Claude auto-switch is off'),
    body: os(
      t(
        'LaunchAgent tự chuyển của Quota Station chưa được nạp, nên không có gì tự chuyển Claude. Số vẫn cập nhật khi app đọc.',
        'Quota Station’s auto-switch LaunchAgent is not loaded, so nothing switches Claude. The numbers still update when the app reads them.',
      ),
      t(
        'Hẹn giờ tự chuyển của Quota Station đang không chạy, nên không có gì tự chuyển Claude. Số vẫn cập nhật khi app đọc.',
        'Quota Station’s auto-switch timer is not running, so nothing switches Claude. The numbers still update when the app reads them.',
      ),
    ),
    actions: [
      { label: t('Bật tự chuyển', 'Turn on auto-switch'), act: 'claude:rotation:on' },
      { label: t('Làm mới', 'Refresh'), act: 'refresh' },
    ],
  };
}

// ===== The window =====

/** `4 tài khoản Quota Station`, `4 Quota Station accounts`; empty with none. */
function accountCount(n: number, keeper: string): string {
  if (!n) return '';
  const vi = n + ' tài khoản' + (keeper ? ' ' + keeper : '');
  const en = n + (keeper ? ' ' + keeper : '') + (n === 1 ? ' account' : ' accounts');
  return t(vi, en);
}

export function windowView(model: Model, now: Date, options: ViewOptions = {}): WindowView {
  const pv = popoverView(model, now, options);
  const c = model.snapshot.claude;
  const g = model.snapshot.agy;
  const swap = c.accounts.some((a) => a.number > 0);
  const present = g.status !== 'notSetUp';
  const auto = autoView(model, now);
  return {
    machine: pv.machine,
    fresh: pv.fresh,
    banner: pv.banner,
    claude: {
      present: pv.claude.shown,
      empty: pv.claude.empty,
      count: accountCount(pv.claude.cards.length, swap ? (model.claudeSwitch.ready ? 'Quota Station' : 'claude-swap') : ''),
      cards: pv.claude.cards,
      note: pv.claude.note,
    },
    agy: {
      present,
      cards: pv.agy.cards,
      note: pv.agy.note,
      banner: overwriteBanner(model) ?? agyBanner(g),
      tabNote: g.status === 'single' && !g.accounts.length && g.reason ? pv.agy.note : null,
      own: g.status === 'ok' && g.source === 'quota-station',
    },
    auto,
    claudeBanner: claudeBanner(model),
    envBanner: envBanner(model),
    switcher: switcherCard(model, auto, now),
  };
}
