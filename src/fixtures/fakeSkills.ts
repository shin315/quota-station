// The Skills, Plugin and File hướng dẫn tabs' fake Rust side (M11), for the
// tests and the browser preview: a table shaped like `agents::skills::view`'s,
// a batch preview and write like `skills::preview` and `skills::write`, a
// GitHub repository as `npx skills add --list` lists it, the plugins as each
// agent's `plugin list` gives them, and instruction files with one key
// masked. No folder, no `npx`, no network; every name is made up.

import type {
  Agent,
  InstrFile,
  InstrShown,
  InstrView,
  McpLine,
  PluginsView,
  RepoList,
  SkillCell,
  SkillChange,
  SkillOutcome,
  SkillPreview,
  SkillRow,
  SkillTable,
  SkillsView,
} from '../lib/model';

/** 2026-01-16 07:20 UTC, the fixtures' "now". */
const NOW = 1768548000;
const AGENTS: Agent[] = ['claude', 'codex', 'agy'];
const DIR: Record<Agent, string> = { claude: '~/.claude/skills/', codex: '~/.codex/config.toml', agy: '~/.gemini/config/skills/' };

const on: SkillCell = { state: 'on', lock: null };
const off: SkillCell = { state: 'off', lock: null };

function row(name: string, over: Partial<SkillRow>): SkillRow {
  return {
    name,
    source: 'github',
    repo: 'acme/agent-skills',
    home: null,
    cells: { claude: on, codex: on, agy: on },
    edited: false,
    update: null,
    size: 18_400,
    importable: false,
    ...over,
  };
}

export function skillTableFixture(over: Partial<SkillTable> = {}): SkillTable {
  return {
    columns: AGENTS.map((agent) => ({ agent, present: true, file: DIR[agent], state: 'ok', reason: null, writable: true })),
    rows: [
      row('deploy-check', {
        source: 'hand',
        repo: null,
        home: 'claude',
        cells: { claude: { state: 'on', lock: 'hand' }, codex: { state: 'absent', lock: 'hand' }, agy: { state: 'off', lock: 'hand' } },
        size: 6_200,
        importable: true,
      }),
      row('find-notes', { source: 'hand', repo: null, cells: { claude: off, codex: on, agy: off }, size: 4_100 }),
      row('pdf-tools', {}),
      row('release-notes', { cells: { claude: on, codex: off, agy: on } }),
      row('sql-helper', {
        repo: 'dataworks/skills',
        cells: { claude: on, codex: off, agy: off },
        edited: true,
        update: {
          fromAt: '2025-12-02T09:00:00Z',
          seenAt: NOW - 11_000,
          plain: 'Sửa hướng dẫn',
          link: 'https://github.com/dataworks/skills/commits/HEAD/sql-helper',
        },
      }),
      row('trace-reader', {
        source: 'hand',
        repo: null,
        home: 'agy',
        cells: { claude: { state: 'off', lock: 'hand' }, codex: { state: 'absent', lock: 'hand' }, agy: { state: 'on', lock: 'hand' } },
        size: 9_800,
        importable: true,
      }),
      row('ui-polish', {
        repo: 'uikit-lab/ui-skills',
        cells: { claude: on, codex: on, agy: off },
        update: {
          fromAt: '2025-12-12T08:00:00Z',
          seenAt: NOW - 11_000,
          plain: 'Sửa hướng dẫn, thêm 1 file',
          link: 'https://github.com/uikit-lab/ui-skills/commits/HEAD/skills/ui-polish',
        },
      }),
    ],
    owned: [
      { what: 'Codex', agent: 'codex', names: ['imagegen', 'skill-creator'] },
      { what: 'issue-hub@acme-plugins', agent: 'claude', names: ['issue-notes'] },
      { what: 'docs-pack@acme-plugins', agent: 'claude', names: ['api-ref', 'docs-writer'] },
    ],
    broken: [{ agent: 'agy', name: 'old-linter', target: '~/old-skills/old-linter' }],
    lockError: null,
    ...over,
  };
}

export function skillsFixture(over: Partial<SkillsView> = {}): SkillsView {
  return {
    table: skillTableFixture(),
    readAt: '2026-01-16T07:19:30Z',
    busy: null,
    progress: null,
    npx: true,
    last: null,
    updates: { checkedAt: '2026-01-16T02:12:00Z', error: null, failed: [], checking: false, settings: { daily: true, notify: false } },
    ...over,
  };
}

/** The lines one change writes, as `skills::preview` shows them. */
function lines(c: SkillChange): McpLine[] {
  if (c.agent === 'codex')
    return c.on
      ? [
          { op: 'del', text: '[[skills.config]]', secret: false },
          { op: 'del', text: 'name = "' + c.name + '"', secret: false },
          { op: 'del', text: 'enabled = false', secret: false },
        ]
      : [
          { op: 'add', text: '[[skills.config]]', secret: false },
          { op: 'add', text: 'name = "' + c.name + '"', secret: false },
          { op: 'add', text: 'enabled = false', secret: false },
        ];
  return [{ op: c.on ? 'add' : 'del', text: DIR[c.agent] + c.name + ' → ~/.agents/skills/' + c.name, secret: false }];
}

export function fakeSkillPreview(table: SkillTable, changes: SkillChange[]): SkillPreview {
  const refused: SkillPreview['refused'] = [];
  const ok: SkillChange[] = [];
  for (const c of changes) {
    const cell = table.rows.find((r) => r.name === c.name)?.cells[c.agent];
    if (!cell) refused.push({ name: c.name, agent: c.agent, reason: 'unknown' });
    else if (cell.lock) refused.push({ name: c.name, agent: c.agent, reason: cell.lock });
    else if ((cell.state === 'on') === c.on) refused.push({ name: c.name, agent: c.agent, reason: 'nothing' });
    else ok.push(c);
  }
  const groups = AGENTS.filter((a) => ok.some((c) => c.agent === a)).map((agent) => {
    const mine = ok.filter((c) => c.agent === agent);
    return { agent, file: DIR[agent], count: mine.length, lines: mine.flatMap(lines) };
  });
  return { groups, refused, changes: ok, bases: Object.fromEntries(groups.map((g) => [g.agent, 'base-' + g.agent])) };
}

export function fakeSkillApply(table: SkillTable, changes: SkillChange[], at: number): { table: SkillTable; outcome: SkillOutcome } {
  const rows = table.rows.map((r) => {
    const mine = changes.filter((c) => c.name === r.name);
    if (!mine.length) return r;
    const cells = { ...r.cells };
    for (const c of mine) cells[c.agent] = { state: c.on ? 'on' : 'off', lock: null };
    return { ...r, cells };
  });
  const agents = AGENTS.filter((a) => changes.some((c) => c.agent === a));
  return {
    table: { ...table, rows },
    outcome: {
      done: agents.map((agent) => ({
        agent,
        changes: changes.filter((c) => c.agent === agent).length,
        backup: agent === 'codex' ? 'config.toml.20260116-142000' : agent + '-links.json.20260116-142000',
      })),
      failed: null,
      undo: { changes: changes.map((c) => ({ ...c, on: !c.on })), bases: {} },
      at,
    },
  };
}

export function repoFixture(repo: string): RepoList {
  return {
    repo,
    skills: [
      { name: 'changelog-writer', description: 'Writes a changelog from the commit history.', installed: false },
      { name: 'pdf-tools', description: 'Reads and fills PDF files.', installed: true },
      { name: 'release-notes', description: 'Drafts release notes for a tag.', installed: true },
      { name: 'test-planner', description: 'Plans tests before a change.', installed: false },
    ],
  };
}

export const SKILL_MD = '---\nname: changelog-writer\ndescription: Writes a changelog from the commit history.\n---\n\n# Changelog writer\n\nRead the commits since the last tag, group them by kind, and write CHANGELOG.md.\n';

export function pluginsFixture(over: Partial<PluginsView> = {}): PluginsView {
  return {
    sides: [
      {
        agent: 'claude',
        state: 'ok',
        detail: null,
        plugins: [
          { id: 'issue-hub@acme-plugins', enabled: true, version: '1.2.0', brings: { mcp: 1, skills: ['issue-notes'], commands: 3, hooks: false, agents: 0 } },
          { id: 'formatter@tools-market', enabled: true, version: '0.4.1', brings: { mcp: 0, skills: [], commands: 1, hooks: true, agents: 0 } },
          { id: 'docs-pack@acme-plugins', enabled: true, version: '2.0.0', brings: { mcp: 0, skills: ['api-ref', 'docs-writer'], commands: 2, hooks: false, agents: 0 } },
          { id: 'review-kit@tools-market', enabled: false, version: '1.0.3', brings: { mcp: 0, skills: [], commands: 1, hooks: false, agents: 2 } },
        ],
        marketplaces: [
          { name: 'acme-plugins', kind: 'github', source: 'acme/claude-plugins' },
          { name: 'tools-market', kind: 'github', source: 'tools-co/market' },
        ],
      },
      {
        agent: 'codex',
        state: 'ok',
        detail: null,
        plugins: [
          { id: 'sheets@codex-market', enabled: true, version: null, brings: { mcp: 1, skills: [], commands: 0, hooks: false, agents: 0 } },
          { id: 'browser@openai-bundled', enabled: true, version: null, brings: { mcp: 1, skills: [], commands: 0, hooks: false, agents: 0 } },
        ],
        marketplaces: [
          { name: 'codex-market', kind: 'github', source: 'tools-co/codex-market' },
          { name: 'openai-bundled', kind: 'builtin', source: null },
        ],
      },
    ],
    readAt: '2026-01-16T07:19:30Z',
    busy: false,
    reinstall: {
      claude: [
        'claude plugin marketplace add acme/claude-plugins',
        'claude plugin marketplace add tools-co/market',
        'claude plugin install issue-hub@acme-plugins',
        'claude plugin install formatter@tools-market',
        'claude plugin install docs-pack@acme-plugins',
        'claude plugin install review-kit@tools-market',
        'claude plugin disable review-kit@tools-market',
      ],
      codex: ['codex plugin marketplace add tools-co/codex-market', 'codex plugin add sheets@codex-market'],
    },
    ...over,
  };
}

/** The same plugins named after real services, so their logos show; `formatter` names none. */
export function pluginsBrandsFixture(): PluginsView {
  const p = pluginsFixture();
  const plugin = (id: string, mcp: number) => ({ id, enabled: true, version: '1.0.0', brings: { mcp, skills: [], commands: 1, hooks: false, agents: 0 } });
  return {
    ...p,
    sides: p.sides!.map((s) =>
      s.agent === 'claude'
        ? { ...s, plugins: [plugin('github@acme-plugins', 1), plugin('figma@acme-plugins', 1), s.plugins.find((x) => x.id.startsWith('formatter@'))!, plugin('stripe@tools-market', 1)] }
        : { ...s, plugins: [plugin('gmail@codex-market', 1), plugin('google-calendar@codex-market', 1), plugin('linear@codex-market', 1)] },
    ),
  };
}

function file(path: string, agent: Agent | null, over: Partial<InstrFile> = {}): InstrFile {
  return {
    path,
    agent,
    modifiedAt: '2026-01-15T09:40:00Z',
    size: 2_400,
    lines: 48,
    keyLines: [],
    error: null,
    include: true,
    extra: false,
    vsPack: 'same',
    ...over,
  };
}

export function instrFixture(over: Partial<InstrView> = {}): InstrView {
  return {
    view: {
      files: [
        file('~/.claude/CLAUDE.md', 'claude', { size: 3_100, lines: 62 }),
        file('~/.claude/rules/deploy.md', 'claude', { modifiedAt: '2026-01-16T06:10:00Z', size: 380, lines: 14, keyLines: [12], vsPack: 'new' }),
        file('~/.claude/rules/style.md', 'claude', { modifiedAt: '2026-01-16T07:02:00Z', size: 640, lines: 9, vsPack: 'changed' }),
        file('~/.codex/AGENTS.md', 'codex', { size: 1_900, lines: 40 }),
        file('~/.gemini/GEMINI.md', 'agy', { size: 900, lines: 21, include: false }),
      ],
      packAt: NOW - 600,
    },
    readAt: '2026-01-16T07:19:30Z',
    ...over,
  };
}

const DEPLOY = ['# Deploy', '', '1. Run the tests.', '2. Tag the release.', '3. Push to the store.', '', '## Server', '', '- Host: deploy.example.com', '- User: deploy', '', 'DEPLOY_TOKEN=••••••', '', 'Ask before deploying on a Friday.'];
const STYLE = ['# Style', '', '- Answer briefly.', '- Keep function and file names in English.', '- No comments when the code is clear.', '- Before editing many files, say the plan in three lines.', '', '## Tests', '- Run the tests before saying done.'];

export function instrShownFixture(path: string): InstrShown {
  const text = path.endsWith('deploy.md') ? DEPLOY : STYLE;
  return { path, lines: text.map((t, i) => ({ text: t, key: path.endsWith('deploy.md') && i === 11, notKey: false })) };
}

export function instrDiffFixture(): McpLine[] {
  return [
    { op: 'ctx', text: '- Answer briefly.', secret: false },
    { op: 'ctx', text: '- Keep function and file names in English.', secret: false },
    { op: 'del', text: '- No comments.', secret: false },
    { op: 'add', text: '- No comments when the code is clear.', secret: false },
    { op: 'add', text: '- Before editing many files, say the plan in three lines.', secret: false },
  ];
}
