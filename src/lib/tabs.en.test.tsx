// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { agyLoginErrorFeedback } from '../antigravity/agyLogin';
import { ClaudeDialogBody } from '../claude/ClaudeDialogs';
import type { ClaudeAccountsState } from '../claude/useClaudeAccounts';
import { CodexTab } from '../codex/CodexTab';
import { claudeAccountsFixture, FIXTURE_NOW, fixtureModel, historyFixture, peerFixture, rotationFixture, rulesFixture } from '../fixtures/models';
import { alertKinds, permissionChip } from './alerts';
import { quotaLine, resultFeedback, signInChip } from './claudeAccounts';
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

  it('Claude accounts: the details, the lines and the add dialog', () => {
    const view = claudeAccountsFixture();
    const row = view.accounts[1];
    expect(signInChip({ ...row, status: 'reloginRequired' }).t).toBe('expired');
    expect(quotaLine(row)).toBe('73% 5-hour');
    const rested = { ...view, accounts: view.accounts.map((a) => (a.slot === 3 ? { ...a, enabled: false } : a)) };
    const fb = resultFeedback(rested, { action: 'enabled', ok: true, error: null, detail: null, slot: 3, renewed: false, imported: null }, 'Team B');
    expect(fb!.title).toBe('Team B is now out of auto-switch.');
    expect(fb!.actions!.map((a) => a.label)).toEqual(['Undo', 'Close']);
    const claude = { dialog: { kind: 'add', how: 'browser', error: null }, pending: false } as unknown as ClaudeAccountsState;
    render(
      <Dialog open>
        <DialogContent>
          <ClaudeDialogBody claude={claude} view={view} advanced={false} nameOf={(n) => 'Account ' + n} onTab={() => {}} />
        </DialogContent>
      </Dialog>,
    );
    expect(screen.getByRole('heading', { name: 'Add a Claude account' })).toBeTruthy();
    expect(screen.getByRole('radio', { name: /Sign in with the browser/ })).toBeTruthy();
    expect(screen.getByRole('radio', { name: /Import from claude-swap|Import again from claude-swap/ })).toBeTruthy();
  });

  it('Antigravity sign-in feedback', () => {
    expect(agyLoginErrorFeedback('timeout')!.body).toBe('No account added. Press Add account to try again.');
  });

  it('the Codex tab', () => {
    render(
      <CodexTab model={fixtureModel('fresh')} fresh={{ tone: 'ok', text: 'now' }} feedback={null} now={FIXTURE_NOW} onAction={() => {}} onPlugins={() => {}} />,
    );
    expect(screen.getByRole('button', { name: 'Check again' })).toBeTruthy();
    expect(screen.getByRole('list', { name: 'ChatGPT accounts' })).toBeTruthy();
    expect(screen.getByRole('list', { name: 'Custom API' })).toBeTruthy();
    expect(screen.getAllByText('5-hour').length).toBeGreaterThan(0);
    expect(screen.getByRole('listitem', { name: /^ChatGPT, sign-in kept on this Mac\. 5-hour used 23%/ })).toBeTruthy();
    expect(screen.getByText('Codex on this Mac')).toBeTruthy();
  });
});
