// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it } from 'vitest';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { codexRow, codexSheet } from './codex';
import { setLang } from './i18n';
import { claudeSheet } from './switching';

// The same words in English, built when called: the language set here is the one they come out in.
describe('switching in English', () => {
  beforeEach(() => setLang('en'));

  it('the Claude switch sheet', () => {
    const v = claudeSheet(
      fixtureModel('fresh'),
      3,
      // @private:begin
      '60',
      // @private:end
      { live: 3, unreadable: 0 },
      FIXTURE_NOW,
    )!;
    expect(v.title).toBe('Switch Claude to lan.acme2?');
    expect(v.confirm).toBe('Switch to lan.acme2');
    expect(v.facts[1].text).toBe('**3 Claude sessions** are running. Each rebuilds its cache on its next message, which uses extra quota.');
  });

  it('the Codex row and its sheet', () => {
    const model = fixtureModel('fresh');
    const row = codexRow(model)!;
    expect(row.head).toBe('Codex App is open');
    expect(row.sub).toBe('Answers in 180 ms · keeps the ChatGPT sign-in');
    expect(row.label).toBe('Codex uses relay. Press to switch to the ChatGPT account');
    const sheet = codexSheet(model, 'chatGpt');
    expect(sheet.title).toBe('Switch Codex back to the ChatGPT account?');
    expect(sheet.confirm).toBe('Switch back to ChatGPT');
    expect(sheet.facts.find((f) => f.k === 'ok')!.text).toBe('Keeps the ChatGPT sign-in so Codex App can still use plugins and remote control.');
  });
});
