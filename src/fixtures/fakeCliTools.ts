// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Công cụ dòng lệnh on a made-up Mac drawn after the author's on 27/10/2026:
// Claude Code from its installer and up to date with three sessions open,
// Codex from Homebrew one version behind, the Antigravity CLI one behind.

import type { CliToolsView, CliToolView } from '../lib/model';

const CHECKED = Date.parse('2026-01-16T07:02:00Z') / 1000;

export function cliToolFixture(tool: CliToolView['tool'], over: Partial<CliToolView> = {}): CliToolView {
  const base: Record<CliToolView['tool'], CliToolView> = {
    claude: {
      tool: 'claude',
      version: '2.1.311',
      latest: '2.1.311',
      newer: false,
      failed: null,
      install: 'native',
      path: '~/.local/bin/claude → ~/.local/share/claude/versions/2.1.311',
      command: 'claude update',
      handsOff: null,
      running: 3,
      safeWhileRunning: true,
      updating: false,
      last: null,
    },
    codex: {
      tool: 'codex',
      version: '0.163.0',
      latest: '0.164.0',
      newer: true,
      failed: null,
      install: 'homebrew',
      path: '/opt/homebrew/bin/codex → /opt/homebrew/Caskroom/codex/0.163.0/bin/codex',
      command: 'brew upgrade --cask codex',
      handsOff: null,
      running: 0,
      safeWhileRunning: false,
      updating: false,
      last: null,
    },
    agy: {
      tool: 'agy',
      version: '1.4.7',
      latest: '1.4.8',
      newer: true,
      failed: null,
      install: 'script',
      path: '~/.local/bin/agy',
      command: 'agy update',
      handsOff: null,
      running: 0,
      safeWhileRunning: true,
      updating: false,
      last: null,
    },
  };
  return { ...base[tool], ...over };
}

export function cliToolsFixture(over: Partial<CliToolsView> = {}): CliToolsView {
  return {
    daily: true,
    checking: false,
    checkedAt: CHECKED,
    offline: false,
    tools: [cliToolFixture('claude'), cliToolFixture('codex'), cliToolFixture('agy')],
    ...over,
  };
}

/** The preview's `cli=` states, several joined with commas. */
export const CLI_STATES: Record<string, (v: CliToolsView) => CliToolsView> = {
  // Claude Code has a newer version with three sessions open (safe); Codex has two open (it waits).
  running: (v) => tools(v, { claude: { latest: '2.1.312', newer: true }, codex: { running: 2 } }),
  missing: (v) => tools(v, { agy: { version: null, latest: null, newer: false, install: null, path: null, command: null } }),
  npm: (v) =>
    tools(v, {
      claude: { install: 'npm', latest: '2.1.312', newer: true, handsOff: 'npm', path: '~/.nvm/versions/node/v24.18.0/bin/claude', command: 'claude install' },
    }),
  bundled: (v) =>
    tools(v, { codex: { install: null, handsOff: 'bundled', bundledBy: 'Codex.app', path: '/Applications/Codex.app/Contents/Resources/codex', command: null } }),
  unwritable: (v) =>
    tools(v, { codex: { install: 'npm', handsOff: 'unwritable', path: '/usr/local/bin/codex', command: 'sudo npm install -g @openai/codex@latest' } }),
  unknown: (v) => tools(v, { codex: { install: null, handsOff: 'unknown', path: '~/bin/codex', command: null } }),
  offline: (v) => ({ ...v, offline: true, tools: v.tools.map((t) => ({ ...t, latest: null, newer: false, failed: 'offline' })) }),
  // Every tool on its newest version.
  latest: (v) => tools(v, { codex: { version: '0.164.0', newer: false }, agy: { version: '1.4.8', newer: false } }),
  // Homebrew's feed answered 503; the others answered.
  failed: (v) => tools(v, { codex: { latest: null, newer: false, failed: '503' } }),
  checking: (v) => ({ ...v, checking: true }),
  never: (v) => ({ ...v, checkedAt: null, tools: v.tools.map((t) => ({ ...t, latest: null, newer: false })) }),
  updated: (v) => tools(v, { codex: { version: '0.164.0', newer: false, last: { ok: true, from: '0.163.0', to: '0.164.0', at: CHECKED + 60 } } }),
  updating: (v) => tools(v, { codex: { updating: true } }),
};

function tools(v: CliToolsView, over: Partial<Record<CliToolView['tool'], Partial<CliToolView>>>): CliToolsView {
  return { ...v, tools: v.tools.map((t) => ({ ...t, ...over[t.tool] })) };
}
