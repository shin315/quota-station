// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Turns the model into what the main window shows (M2, read only): the
// Tổng quan, Claude and Antigravity tabs. Built on the popover's view, so the
// window and the popover show the same snapshot with the same names, numbers
// and banners. Pure, like `view.ts`.

import { overwriteBanner } from '@agy/agySwitch';
import { envBanner } from './env';
import { mac, t } from './i18n';
import type { Model, Section } from './model';
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
  /** The Claude tab's red banner: a variable overrides Claude Code's login (M14). */
  envBanner: BannerView | null;
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
    envBanner: envBanner(model),
  };
}
