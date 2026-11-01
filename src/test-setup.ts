// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Vitest: a Mac whose language is Vietnamese, so "auto" reads as it does
// on the author's Macs. A test of English sets the language itself, and a
// test of Windows (`*.win.test.tsx`) the system.

import { afterEach } from 'vitest';
import { setLang, setPlatform } from './lib/i18n';

Object.defineProperty(window.navigator, 'languages', { value: ['vi-VN', 'en-US'], configurable: true });
Object.defineProperty(window.navigator, 'language', { value: 'vi-VN', configurable: true });

afterEach(() => {
  setLang('vi');
  setPlatform('macos');
  delete document.documentElement.dataset.os;
});
