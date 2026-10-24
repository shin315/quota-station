// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Vietnamese and English (community edition, 24/10/2026). Every string the
// windows write is `t('tiếng Việt', 'English')`, as the approved prototype
// writes them: the two stay side by side, so a change to one is seen next
// to the other. The language is set once per render of a window's root
// (`useLang` in App.tsx) before anything below it builds its text, so the
// pure view functions read it without passing it down. Tabs the prototype
// does not draw are still Vietnamese only.

import type { Language } from './model';

export type Lang = 'vi' | 'en';

let current: Lang = 'vi';

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

/**
 * The owner's choice, or macOS's: the first of the Mac's preferred
 * languages the app speaks. A Mac set to neither gets English.
 */
export function resolveLang(setting: Language | undefined, preferred: readonly string[]): Lang {
  if (setting === 'vi' || setting === 'en') return setting;
  for (const tag of preferred) {
    const base = tag.toLowerCase().split(/[-_]/)[0];
    if (base === 'vi' || base === 'en') return base;
  }
  return 'en';
}

/** macOS's preferred languages, as the WebView hears them. */
export function systemLanguages(): readonly string[] {
  if (typeof navigator === 'undefined') return [];
  return navigator.languages?.length ? navigator.languages : navigator.language ? [navigator.language] : [];
}
