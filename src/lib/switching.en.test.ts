// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it } from 'vitest';
import { FIXTURE_NOW, fixtureModel, heldSwitcher } from '../fixtures/models';
import { codexRow, codexSheet } from './codex';
import { setLang } from './i18n';
import { claudeSheet, holdOptions, pausedBanner, switchFeedback } from './switching';

// The same words in English, built when called: the language set here is the one they come out in.
describe('switching in English', () => {
  beforeEach(() => setLang('en'));

  it('the hold choices', () => {
    const model = fixtureModel('fresh');
    const account = model.snapshot.claude.accounts.find((a) => a.number === 4)!;
    expect(holdOptions(account, FIXTURE_NOW).map((o) => [o.t, o.s])).toEqual([
      ['30 minutes', 'until 14:50'],
      ['1 hour', 'until 15:20'],
      ['Until reset', 'until 17:09'],
      ['Let it auto-switch', 'no hold'],
    ]);
  });

  it('the Claude switch sheet', () => {
    const v = claudeSheet(fixtureModel('fresh'), 3, '60', { live: 3, unreadable: 0 }, FIXTURE_NOW)!;
    expect(v.title).toBe('Switch Claude to lan.acme2?');
    expect(v.confirm).toBe('Switch to lan.acme2');
    expect(v.facts[1].text).toBe('**3 Claude sessions** are running. Each rebuilds its cache on its next message, which uses extra quota.');
    expect(v.hint.text).toBe('Pauses auto-switch until 15:20 to keep this account.');
  });

  it('the paused banner and a switch that holds', () => {
    const held = fixtureModel('fresh', { switcher: heldSwitcher() });
    const banner = pausedBanner(held, FIXTURE_NOW)!;
    expect(banner.title).toBe('Holding lan.acme1 until 15:20');
    expect(banner.actions!.map((a) => a.label)).toEqual(['Allow auto-switch again', 'One more hour']);
    const fb = switchFeedback({ kind: 'switched', slot: 3, hold: { until: '2026-01-16T08:20:00Z', failed: false } }, held, FIXTURE_NOW);
    expect(fb.title).toBe('Switched to lan.acme2. Holding it until 15:20.');
    expect(fb.body).toBe('Read again after 5 seconds: lan.acme2 is in use ✓. Claude auto-switch is paused until then.');
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
