// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { agyLoginErrorFeedback } from '../antigravity/agyLogin';
import { ClaudeAccounts } from '../claude/ClaudeAccounts';
import { CodexTab } from '../codex/CodexTab';
import { claudeAccountsFixture, FIXTURE_NOW, fixtureModel, historyFixture, peerFixture, rotationFixture, rulesFixture } from '../fixtures/models';
import { alertKinds, permissionChip } from './alerts';
import type { Api } from './api';
import { accountRows, workingText } from './claudeAccounts';
import { dayLabel, historyDays } from './history';
import { setLang } from './i18n';
import { compareRows, peerLine, ruleNames, rulesProblem, saveFeedback, sidebarLine } from './rules';
import { num, readLine, tokensText } from './stats';

// The tabs' words in English, built when called: the language set here is the one they come out in.
describe('the tabs in English', () => {
  beforeEach(() => setLang('en'));
  afterEach(cleanup);

  it('Rotation rules: each Mac, the problems and the compare table', () => {
    const model = fixtureModel('fresh');
    expect(peerLine(peerFixture('Mac C', 'mac-c')).text).toBe('received 14:02');
    expect(peerLine(peerFixture('Mac C', 'mac-c', { state: null })).short).toBe('Mac C: never sent');
    expect(sidebarLine(model)).toBe('rules sent to 2 other Macs at 14:02');
    expect(saveFeedback(model).title).toBe('Saved the rules for all three Macs at 14:02');
    expect(rulesProblem(rulesFixture({ team: [] }))).toBe('Pick at least one Team account.');
    const rows = compareRows(rulesFixture(), rulesFixture({ full: 88 }), ruleNames(model), FIXTURE_NOW);
    expect(rows.map((r) => r.label)).toContain('Auto-switch threshold');
    const lonely = fixtureModel('fresh', { rotation: rotationFixture({ peers: [] }) });
    expect(saveFeedback(lonely).title).toBe('Saved the rules for this Mac at 14:02');
  });

  it('History: days, tags and reasons', () => {
    expect(dayLabel(FIXTURE_NOW.toISOString(), FIXTURE_NOW)).toBe('Today');
    const days = historyDays(historyFixture(), 'all', fixtureModel('fresh'), FIXTURE_NOW);
    expect(days[0].label).toBe('Today');
    const pills = days.flatMap((d) => d.rows.map((r) => r.pill.t));
    expect(pills).toContain('Switch');
    expect(pills).not.toContain('Chuyển');
    expect(days.flatMap((d) => d.rows.map((r) => r.why)).some((w) => w.startsWith('You switched'))).toBe(true);
  });

  it('Stats: English numbers and units', () => {
    expect(num(1234.56)).toBe('1,234.6');
    expect(tokensText(45_500_000)).toBe('45.5 million tokens');
    expect(tokensText(820)).toBe('820 tokens');
    expect(readLine({ ...fixtureModel('fresh').stats, reading: true })).toBe('reading logs…');
  });

  it('Alerts: the kinds and the permission chip', () => {
    const kinds = alertKinds(fixtureModel('fresh'));
    expect(kinds.map((k) => k.title)).toContain('An account needs to sign in again');
    expect(kinds.find((k) => k.kind === 'pool')!.text).toMatch(/^When the account in use has \d+% or less left/);
    expect(permissionChip('denied').t).toBe('off');
  });

  it('Claude accounts: rows, the running action and the card', () => {
    const view = claudeAccountsFixture();
    const rows = accountRows(view);
    expect(rows.find((r) => r.active)!.chips[0].t).toBe('in use');
    expect(workingText({ ...view, working: 'import' })).toBe('Importing from claude-swap…');
    render(<ClaudeAccounts view={view} api={{} as Api} onAction={() => {}} />);
    expect(screen.getByRole('region', { name: 'Accounts Quota Station keeps' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Add a setup token' })).toBeTruthy();
  });

  it('Antigravity sign-in feedback', () => {
    expect(agyLoginErrorFeedback('timeout')!.body).toBe('No account added. Press Add account to try again.');
  });

  it('the Codex tab', () => {
    render(
      <CodexTab model={fixtureModel('fresh')} fresh={{ tone: 'ok', text: 'now' }} feedback={null} now={FIXTURE_NOW} onAction={() => {}} onPlugins={() => {}} />,
    );
    expect(screen.getByRole('button', { name: 'Check again' })).toBeTruthy();
    expect(screen.getByRole('list', { name: 'Codex providers' })).toBeTruthy();
    expect(screen.getByText('Codex on this Mac')).toBeTruthy();
  });
});
