// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The interface's extension registry: what a build may add to the windows
// without the shared code knowing it. The app imports one registry,
// `@qs-ext`: `src/ext/none.ts` (nothing) unless the build names another
// folder in `QS_EXT_DIR` (vite.config.ts), whose `index.ts` exports its own.
// Every part is optional but `tabs`; a part that is absent adds nothing.
//
// An extension's ids join the shared unions by declaration merging:
//
//     declare module '@/ext/types' {
//       interface ExtTabs { mine: true }
//       interface ExtSwitchKinds { mine: true }
//     }

import type { ComponentType } from 'react';
import type { invoke } from '@tauri-apps/api/core';
import type { GlyphName } from '@/components/icons';
import type { FakeOptions } from '@/fixtures/fakeApi';
import type { Api } from '@/lib/api';
import type { Alert, AlertKind, Model, SwitchKind, Tab } from '@/lib/model';
import type { Action, PopoverView } from '@/lib/view';

/** The main window's tabs an extension adds (`Tab`): its ids as keys. */
export interface ExtTabs {}

/** The alert kinds with a switch an extension adds (`SwitchKind`): its ids as keys. */
export interface ExtSwitchKinds {}

/** What the app hands an extension's views. */
export interface ExtProps {
  model: Model;
  api: Api;
  /** How fresh the numbers are, as the page titles say it. */
  fresh: PopoverView['fresh'];
  now: Date;
  onAction(act: Action): void;
  /** A setting was not saved: the app's feedback says so. */
  failed(): void;
}

/** A tab of the main window. */
export interface ExtTab {
  id: keyof ExtTabs;
  /** Its name in the sidebar and on the page. */
  label(): string;
  /** Its sidebar glyph, one no other place has. */
  glyph: GlyphName;
  /** The sidebar item it comes right after. */
  after: Tab;
  /** Whether this Mac shows it; hidden, it gives way to Tổng quan. */
  shown(model: Model): boolean;
  Page: ComponentType<ExtProps>;
}

/** An alert kind with a switch in Cài đặt › Thông báo. */
export interface ExtAlertKind {
  kind: SwitchKind;
  title: string;
  text: string;
}

/** What the fake API (tests and the browser preview) gives an extension's fake calls. */
export interface FakeContext {
  current(): Model;
  push(next: Model): void;
  options: FakeOptions;
}

export interface Extension {
  tabs: ExtTab[];
  /** Cài đặt › Nâng cao. */
  advanced?: {
    /** Its name, last in the list of what Nâng cao holds ("Luật, tạm dừng và …"). */
    name(): string;
    /** Its rows, first under Nâng cao's switch while it is on. */
    Rows: ComponentType<ExtProps>;
  };
  /** Its alert kinds, after the app's in Cài đặt › Thông báo. */
  alerts?: {
    kinds(): ExtAlertKind[];
    /** Whether they show on this Mac. */
    shown(model: Model): boolean;
    /** Said after alert 1's rule where the kinds show (and in the full list while Nâng cao is on). */
    poolRule(): string;
  };
  /** Said after the pause sheet's hint. */
  pauseNote?(): string;
  /** Steps of the move from the other account switcher, before its last. */
  guideSteps?(): string[];
  /** In a build with the advanced part, the Claude tab's "Chạy nền": a line before its Chi tiết, and more inside. */
  rotationNote?: {
    lead(): string;
    Details: ComponentType;
  };
  /** Its calls to the Rust side, added to the app's `Api`. */
  api?(call: typeof invoke): object;
  /** The fake API's side. */
  fake?: {
    /** Its calls, added to the fake `Api`. */
    api(fake: FakeContext): object;
    /** What "Gửi thử" sends for one of its kinds; null for a kind not its own. */
    sample(kind: AlertKind): Pick<Alert, 'tone' | 'title' | 'body' | 'actions'> | null;
  };
  /** The fixtures' side (tests and the browser preview). */
  fixtures?: {
    /** Its parts of the author's Mac's model (`fixtureModel`). */
    add(model: Model): Model;
    /** The same Mac on a build without it (`communityModel`, `releaseModel`). */
    remove(model: Model): Model;
    /** Its alert switches as they ship (`alertsFixture`). */
    switches(): Partial<Record<SwitchKind, boolean>>;
  };
  /** The browser preview's side. */
  preview?: {
    /** Its URL parameters, applied to the model. */
    apply(params: URLSearchParams, over: Partial<Model>): void;
    /** What its fake calls read. */
    options(): Partial<FakeOptions>;
  };
}
