// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Chép sang Mac khác (M13) for the browser preview and the tests: what "So
// sánh" finds between this Mac and a made-up other one, and what a copy and
// its undo report. Every name, path and Mac is made up.

import type { CopyComparison, CopyItem, CopySelection, RestoreDone, RestoreItemResult, RestorePartResult, RestoreUndone } from '../lib/model';

const item = (over: Partial<CopyItem> & Pick<CopyItem, 'group' | 'id' | 'state'>): CopyItem => ({
  agent: null,
  what: [],
  push: null,
  pull: null,
  note: null,
  hand: false,
  keys: [],
  mineOn: [],
  theirsOn: [],
  compare: [],
  mineAt: null,
  theirsAt: null,
  lines: [],
  changed: 0,
  ...over,
});

export const SAME = [
  'docs-search',
  'browser',
  'pdf-tools',
  'test-writer',
  'release-notes',
  'sql-helper',
  'issue-hub@acme-plugins',
  'docs-pack@acme-plugins',
  '~/.codex/AGENTS.md',
  '~/.gemini/GEMINI.md',
  '~/.claude/rules/style.md',
];

/** This Mac against `mac`: a server only here and one only there, one with other arguments, a newer skill, a skill installed by hand, a plugin and a file. */
export function comparisonFixture(alias: string, mac: string, over: Partial<CopyComparison['diff']> = {}): CopyComparison {
  const items: CopyItem[] = [
    item({
      group: 'mcp',
      id: 'design-kit',
      state: 'mine',
      keys: ['DESIGN_KIT_TOKEN'],
      mineOn: ['claude', 'agy'],
      compare: [
        { agent: 'claude', mine: { kind: 'stdio', command: 'design-kit-mcp', args: [], env: [{ name: 'DESIGN_KIT_TOKEN', value: '••••••' }], cwd: null }, theirs: null },
        { agent: 'agy', mine: { kind: 'stdio', command: 'design-kit-mcp', args: [], env: [{ name: 'DESIGN_KIT_TOKEN', value: '••••••' }], cwd: null }, theirs: null },
      ],
    }),
    item({
      group: 'mcp',
      id: 'files',
      state: 'diff',
      what: ['args'],
      mineOn: ['claude', 'codex'],
      theirsOn: ['claude', 'codex'],
      compare: [
        {
          agent: 'claude',
          mine: { kind: 'stdio', command: '~/.local/bin/files-mcp', args: ['--root', '~/Workspace'], env: [], cwd: null },
          theirs: { kind: 'stdio', command: '/opt/homebrew/bin/files-mcp', args: ['--root', '~/Code'], env: [], cwd: null },
        },
        {
          agent: 'codex',
          mine: { kind: 'stdio', command: '~/.local/bin/files-mcp', args: ['--root', '~/Workspace'], env: [], cwd: null },
          theirs: { kind: 'stdio', command: '/opt/homebrew/bin/files-mcp', args: ['--root', '~/Code'], env: [], cwd: null },
        },
      ],
    }),
    item({
      group: 'mcp',
      id: 'tracker',
      state: 'theirs',
      theirsOn: ['codex'],
      compare: [{ agent: 'codex', mine: null, theirs: { kind: 'http', url: 'https://tracker.example.com/mcp', headers: [], sse: false } }],
    }),
    item({ group: 'skill', id: 'ui-polish', state: 'ver', mineAt: '2026-10-17T08:00:00Z', theirsAt: '2026-09-12T08:00:00Z', pull: 'older' }),
    item({ group: 'skill', id: 'deploy-check', state: 'mine', hand: true, mineOn: ['claude'] }),
    item({ group: 'plugin', id: 'formatter@tools-market', agent: 'claude', state: 'mine' }),
    item({
      group: 'file',
      id: '~/.claude/CLAUDE.md',
      agent: 'claude',
      state: 'text',
      changed: 2,
      lines: [
        { op: 'ctx', text: '# Chung', secret: false },
        { op: 'del', text: '- Dùng pnpm cho dự án mới.', secret: false },
        { op: 'add', text: '- Dùng npm.', secret: false },
        { op: 'ctx', text: '- Chạy test trước khi commit.', secret: false },
        { op: 'del', text: '- Hỏi trước khi xoá file.', secret: false },
      ],
    }),
  ];
  return { alias, mac, at: 1_768_548_900, diff: { items, same: SAME, ...over } };
}

/** What a copy of `sel` reports: servers short of a key added off, the rest written. */
export function copyDoneFixture(c: CopyComparison, sel: CopySelection, keys: boolean, at: number): RestoreDone {
  const items: RestoreItemResult[] = sel.mcp.map((name) => {
    const i = c.diff.items.find((x) => x.group === 'mcp' && x.id === name);
    const on = i?.state === 'theirs' ? i.theirsOn : (i?.mineOn ?? []);
    const noKey = !!i?.keys.length && !keys;
    return {
      name,
      kind: i?.state === 'diff' ? 'replace' : 'add',
      done: noKey ? [] : on,
      kept: noKey ? on : [],
      why: noKey ? 'noKey' : null,
      failed: [],
      skipped: null,
      login: false,
      command: null,
    };
  });
  const part = (which: RestorePartResult['part'], id: string, kind: string, agent: RestorePartResult['agent'] = null): RestorePartResult => ({
    part: which,
    id,
    agent,
    kind,
    ok: true,
    why: null,
    retry: false,
  });
  const parts = [
    ...sel.skills.map((s) => part('skill', s, c.diff.items.find((x) => x.id === s)?.hand ? 'hand' : 'github')),
    ...sel.updates.map((s) => part('skill', s, 'update')),
    ...sel.plugins.map((p) => part('plugin', p.id, '', p.agent)),
    ...sel.files.map((f) => part('file', f, 'replace', 'claude')),
  ];
  return { at, items, failed: null, written: [{ agent: 'codex', changes: items.length, backup: null }], keys: keys ? 1 : 0, parts, canUndo: true };
}

export const copyUndoneFixture = (): RestoreUndone => ({ outcome: null, kept: 1, keys: 0, skills: 2, plugins: 1, files: 1, left: 0 });
