// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Vietnamese and English (community edition, 24/10/2026). Every string the
// windows write is `t('tiếng Việt', 'English')`, as the approved prototype
// writes them: the two stay side by side, so a change to one is seen next
// to the other. The language is set once per render of a window's root
// (`useLang` in App.tsx) before anything below it builds its text, so the
// pure view functions read it without passing it down. Tabs the prototype
// does not draw are still Vietnamese only.

import type { Language, Platform } from './model';

export type Lang = 'vi' | 'en';

let current: Lang = 'vi';
let system: Platform = 'macos';

/** The text in the language on screen. */
export function t(vi: string, en: string): string {
  return current === 'en' ? en : vi;
}

/** The language on screen. */
export function lang(): Lang {
  return current;
}

export function setLang(next: Lang) {
  current = next;
}

// The system the app runs on (Windows plan W5, 27/10/2026), set beside the
// language by the root (`usePlatform` in App.tsx) from what Rust sends
// (`Capabilities.platform`). The text was written for macOS and stays as it
// is there, character for character: a line that names a part of macOS
// (Keychain, Finder, ⌘Q, "this Mac") goes through one of the helpers below,
// which gives it back unchanged on macOS and gives Windows its own words.

/** The system on screen: macOS until the root says otherwise. */
export function platform(): Platform {
  return system;
}

export function setPlatform(next: Platform) {
  system = next;
}

/** True on Windows. */
export function onWindows(): boolean {
  return system === 'windows';
}

/** macOS's text (or value) as written, or Windows' own. */
export function os<T>(mac: T, win: T): T {
  return system === 'windows' ? win : mac;
}

/**
 * A line that names the Mac ("this Mac", "Mac khác"): as written on macOS;
 * on Windows the Mac is a PC in English and a "máy" in Vietnamese. Wrap the
 * whole `t(…)`, so the words change in the language on screen. "MacBook" and
 * "macOS" are other words and stay.
 */
export function mac(text: string): string {
  if (system !== 'windows') return text;
  if (current === 'en') return text.replace(/\bMac(s?)\b/g, 'PC$1');
  return text.replace(/(^|[.!?]\s+)Mac\b/g, '$1Máy').replace(/\bMac\b/g, 'máy');
}

/** The system's name in a sentence: "Theo macOS", "thông báo Windows". */
export function osName(): string {
  return os('macOS', 'Windows');
}

/** The keys that quit an app: ⌘Q, or Alt+F4. */
export function quitKey(): string {
  return os('⌘Q', 'Alt+F4');
}

/** The system's file manager. */
export function fileManager(): string {
  return os('Finder', 'File Explorer');
}

/** The system's settings app, as its own menus name it. */
export function systemSettings(): string {
  return os(t('Cài đặt hệ thống', 'System Settings'), t('Cài đặt Windows', 'Windows Settings'));
}

/** Where a command line is typed. */
export function terminal(): string {
  return os('Terminal', 'Windows Terminal');
}

/**
 * Where the app keeps its own secrets (the Claude accounts' sign-ins, the
 * MCP keys, the backup key): the Keychain, or on Windows a file Windows
 * encrypts for the signed-in user (DPAPI).
 */
export function secretStore(): string {
  return os('Keychain', t('kho mã hoá', 'encrypted store'));
}

/** Where Claude Code and Codex keep a sign-in of their own: the Keychain, or Credential Manager. */
export function toolStore(): string {
  return os('Keychain', 'Credential Manager');
}

/** The cloud folder the backup packs go to by default. */
export function cloudDrive(): string {
  return os('iCloud Drive', 'OneDrive');
}

/** An app by its name: `Antigravity.app` on macOS, `Antigravity` on Windows. */
export function appName(name: string): string {
  return os(name + '.app', name);
}

/**
 * The owner's choice, or the system's: the first of its preferred
 * languages the app speaks. A system set to neither gets English.
 */
export function resolveLang(setting: Language | undefined, preferred: readonly string[]): Lang {
  if (setting === 'vi' || setting === 'en') return setting;
  for (const tag of preferred) {
    const base = tag.toLowerCase().split(/[-_]/)[0];
    if (base === 'vi' || base === 'en') return base;
  }
  return 'en';
}

/** The system's preferred languages, as the WebView hears them. */
export function systemLanguages(): readonly string[] {
  if (typeof navigator === 'undefined') return [];
  return navigator.languages?.length ? navigator.languages : navigator.language ? [navigator.language] : [];
}
