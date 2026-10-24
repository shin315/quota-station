// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Vitest: a Mac whose language is Vietnamese, so "auto" reads as it does
// on the author's Macs. A test of English sets the language itself.

import { afterEach } from 'vitest';
import { setLang } from './lib/i18n';

Object.defineProperty(window.navigator, 'languages', { value: ['vi-VN', 'en-US'], configurable: true });
Object.defineProperty(window.navigator, 'language', { value: 'vi-VN', configurable: true });

afterEach(() => setLang('vi'));
