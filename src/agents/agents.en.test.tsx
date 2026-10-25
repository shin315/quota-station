// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The MCP, Skills, Plugin, File hướng dẫn and Bộ cấu hình tabs in English:
// the owner's language set to English, on the fake Rust side. Every server,
// skill, set and file is made up.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { WEB } from '../fixtures/fakeSets';
import { codexFixture, FIXTURE_NOW, fixtureModel, mcpTableFixture } from '../fixtures/models';
import type { Api } from '../lib/api';
import { setLang } from '../lib/i18n';
import { emptyForm, formError, pendingList, pendingText, setBox, undoneFeedback, writtenFeedback } from '../lib/mcp';
import type { McpOutcome, Model, Tab } from '../lib/model';
import { driftChip, sessionsLine, setErrorText, setSummary } from '../lib/sets';
import { npxText, sizeText } from '../lib/skills';

async function open(tab: Tab, model: Model) {
  const fake = fakeApi(model, tab);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

const english = (over: Partial<Model> = {}) => fixtureModel('fresh', { language: 'en', ...over });
const dialog = () => screen.getByRole('dialog');
const button = (name: string | RegExp) => screen.getByRole('button', { name });
const box = (name: string) => screen.getByRole('checkbox', { name: new RegExp('^' + name) });

afterEach(cleanup);

describe('the Agent tabs in English', () => {
  it('draws the MCP tab, its boxes and the pending bar in English', async () => {
    await open('mcp', english());
    await screen.findByRole('heading', { level: 2, name: 'MCP' });
    expect(button('Add MCP')).toBeTruthy();
    expect(screen.getAllByText('Name').length).toBeGreaterThan(0);
    await press(box('browser for Antigravity'));
    const bar = screen.getByText('1 change not written yet').closest('.pendbar') as HTMLElement;
    expect(within(bar).getByText('browser: turn on for Antigravity')).toBeTruthy();
    await press(within(bar).getByRole('button', { name: 'Preview and write…' }));
    expect(within(dialog()).getByRole('heading', { name: 'Write 1 change?' })).toBeTruthy();
    expect(within(dialog()).getByRole('button', { name: 'Write 1 change' })).toBeTruthy();
  });

  it('draws the Skills tab and a batch preview in English', async () => {
    await open('skills', english());
    await screen.findByRole('heading', { level: 2, name: 'Skills' });
    expect(button('Install from GitHub')).toBeTruthy();
    expect(screen.getByText('installed by hand · Claude Code’s folder')).toBeTruthy();
    await press(box('ui-polish for Antigravity'));
    await press(box('pdf-tools for Codex'));
    const bar = screen.getByText('2 changes not written yet').closest('.pendbar') as HTMLElement;
    expect(within(bar).getByText('pdf-tools: turn off for Codex · ui-polish: turn on for Antigravity')).toBeTruthy();
    await press(within(bar).getByRole('button', { name: 'Preview and write…' }));
    expect(within(dialog()).getByRole('heading', { name: 'Write 2 changes?' })).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Write 2 changes' }));
    expect(screen.getByText('Wrote 2 changes.')).toBeTruthy();
    expect(button('Undo')).toBeTruthy();
  });

  it('draws the Plugin tab in English', async () => {
    await open('plugins', english());
    await screen.findByRole('heading', { level: 2, name: 'Plugins' });
    expect(button('Reinstall commands')).toBeTruthy();
    expect(screen.getByText(/^Plugins are view only · restored through the agent’s own command\./)).toBeTruthy();
    expect(screen.getByRole('heading', { level: 4, name: 'Installed plugins' })).toBeTruthy();
    expect(screen.getByRole('list', { name: 'Claude Code plugins' })).toBeTruthy();
    await press(button('Details: Plugins'));
    expect(screen.getByText(/A backup pack only records name@marketplace/)).toBeTruthy();
  });

  it('draws the Instruction files tab and its key line in English', async () => {
    await open('instr', english());
    await screen.findByRole('heading', { level: 2, name: 'Instruction files' });
    expect(button('Add file…')).toBeTruthy();
    expect(screen.getByRole('group', { name: 'Instruction files' })).toBeTruthy();
    expect(screen.getByRole('list', { name: 'Claude Code files' })).toBeTruthy();
    expect(screen.getByText('deploy.md is not in the backup pack')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Put ~/.claude/rules/deploy.md in the pack' })).toBeTruthy();
    await press(button('View line 12'));
    expect(within(dialog()).getByRole('button', { name: 'Line 12 is not a key' })).toBeTruthy();
  });

  it('draws the Config sets tab and the apply preview in English', async () => {
    await open('sets', english());
    await screen.findByRole('heading', { level: 2, name: 'Config sets' });
    expect(screen.getByText('1 entry differs')).toBeTruthy();
    expect(screen.getByText(/since then browser is off in Antigravity/)).toBeTruthy();
    expect(screen.getByText('3 MCP on, 1 off · 2 skills on · Codex unchanged')).toBeTruthy();
    expect(within(screen.getByRole('list', { name: 'Sets' })).getAllByRole('listitem')).toHaveLength(3);
    expect(button('New set')).toBeTruthy();
    await press(button('Apply the set Tối giản'));
    const d = within(dialog());
    await d.findByText('Claude Code');
    expect(d.getByText('MCP: turn off browser, turn off design-kit, turn off files')).toBeTruthy();
    expect(d.getByRole('button', { name: 'Apply set' })).toBeTruthy();
    expect(d.getByRole('button', { name: 'Cancel' })).toBeTruthy();
  });
});

describe('the Agent tabs’ words in English', () => {
  beforeEach(() => setLang('en'));

  it('builds the MCP lines', () => {
    const table = mcpTableFixture();
    const row = table.rows.find((r) => r.name === 'docs-search')!;
    const p = setBox({}, row, 'agy', true);
    expect(pendingList(p, table.rows).map(pendingText)).toEqual(['docs-search: add to Antigravity']);
    expect(formError({ ...emptyForm(table.columns), name: '-bad' }, [])).toBe('Names may only have letters, digits, - _ . and cannot start with - or .');
    const out: McpOutcome = { at: 0, done: [{ agent: 'codex', changes: 1, backup: 'config.toml.bak' }], failed: null, undo: { changes: [], bases: {} } };
    const fb = writtenFeedback(out, true);
    expect(fb.title).toBe('Wrote 1 change.');
    expect(fb.body).toMatch(/^Backed up config\.toml at \d\d:\d\d\. Reopen the agent to use it\.$/);
    expect(fb.actions!.map((a) => a.label)).toEqual(['Undo', 'Backups']);
    const failed: McpOutcome = { at: 0, done: [], failed: { agent: 'codex', error: { kind: 'changed' } }, undo: null };
    expect(undoneFeedback(failed).title).toBe('Not undone: another program just wrote config.toml.');
  });

  it('builds the Skills and sets lines', () => {
    expect(npxText('noNpx').title).toBe('npx not found');
    expect(sizeText(1.5 * 1024 * 1024)).toBe('1.5 MB');
    expect(setSummary(WEB, codexFixture())).toBe('3 MCP on, 1 off · 2 skills on · Codex unchanged');
    expect(setSummary({ ...WEB, mcp: {}, skills: {}, codex: 'custom' }, codexFixture())).toBe('Codex to relay');
    expect(driftChip(2).t).toBe('2 entries differ');
    expect(sessionsLine(0, codexFixture({ procs: null }))).toBe('Running sessions keep the old setup until they are reopened.');
    expect(setErrorText('taken')).toBe('There is already a set with this name.');
  });
});
