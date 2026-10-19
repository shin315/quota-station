// The Sao lưu tab's fake Rust side (M10, M12), for the tests and the browser
// preview: packs of three made-up Macs, one not downloaded from iCloud yet,
// a restore plan shaped like `agents::restore::plan`'s (servers, skills,
// plugins, instruction files), and a run that ends as `agents::restore::run`
// would. No file, no Keychain, no iCloud Drive; every name is made up and
// every secret is the mask.

import type {
  BackupView,
  McpDef,
  McpDefView,
  PackInfo,
  PackSummary,
  RestoreDone,
  RestoreItem,
  RestoreMore,
  RestorePartResult,
  RestorePlan,
  RestorePluginItem,
  RestoreSkillItem,
} from '../lib/model';

const MASK = '••••••';
/** 2026-01-16 07:20 UTC, the fixtures' "now". */
const NOW = 1768548000;

const view = (def: McpDef, slots: string[] = []): McpDefView => ({ def, slots, extra: [] });

export function packsFixture(): PackInfo[] {
  const p = (id: string, at: number, mac: string, mine: boolean, mcp: number, keys: number, state: PackInfo['state'] = 'ok'): PackInfo => ({
    id,
    state,
    at,
    mac: state === 'cloud' ? null : mac,
    mine,
    mcp,
    keys,
    newer: false,
  });
  return [
    p('agents-20260116-141000-studio-mini', NOW - 600, 'Mac mini', true, 6, 2),
    p('agents-20260116-091200-studio-mini', NOW - 18_480, 'Mac mini', true, 6, 2),
    p('agents-20260115-220500-travel-air', NOW - 33_900, 'Travel Air', false, 4, 1, 'cloud'),
    p('agents-20260115-143000-desk-mini', NOW - 60_000, 'Desk mini', false, 5, 2),
    p('agents-20260114-214800-studio-mini', NOW - 120_000, 'Mac mini', true, 5, 2),
  ];
}

export function backupFixture(over: Partial<BackupView> = {}): BackupView {
  return {
    icloud: true,
    packs: packsFixture().map((p) => (p.state === 'cloud' ? { ...p, state: 'ok', mac: 'Travel Air' } : p)),
    keys: [
      { mcp: 'docs-search', slot: 'header:Authorization', label: 'Authorization', bitwarden: 'Docs Search API' },
      { mcp: 'design-kit', slot: 'env:DESIGN_KIT_TOKEN', label: 'DESIGN_KIT_TOKEN', bitwarden: 'Design Kit token' },
    ],
    readAt: '2026-01-16T07:19:30Z',
    busy: null,
    auto: { afterChange: true, daily: true },
    restore: null,
    autoError: null,
    ...over,
  };
}

export function summaryFixture(id: string): PackSummary {
  const p = packsFixture().find((x) => x.id === id) ?? packsFixture()[0];
  return {
    id: p.id,
    mac: p.mac ?? 'Travel Air',
    at: p.at ?? NOW,
    servers: [
      { name: 'browser', agents: ['claude', 'agy'], on: ['claude'] },
      { name: 'design-kit', agents: ['claude', 'agy'], on: ['claude', 'agy'] },
      { name: 'docs-search', agents: ['claude', 'codex', 'agy'], on: ['claude', 'codex', 'agy'] },
      { name: 'files', agents: ['claude', 'codex'], on: ['claude', 'codex'] },
      { name: 'notes', agents: ['codex'], on: ['codex'] },
      { name: 'tracker', agents: ['codex', 'agy'], on: ['codex', 'agy'] },
    ].slice(0, p.mcp) as PackSummary['servers'],
    keys: [
      { mcp: 'docs-search', slot: 'header:Authorization', label: 'Authorization', bitwarden: 'Docs Search API' },
      { mcp: 'design-kit', slot: 'env:DESIGN_KIT_TOKEN', label: 'DESIGN_KIT_TOKEN', bitwarden: 'Design Kit token' },
    ].slice(0, p.keys),
  };
}

const item = (over: Partial<RestoreItem> & Pick<RestoreItem, 'name' | 'agents'>): RestoreItem => ({
  kind: 'add',
  checked: true,
  off: [],
  missingCommand: null,
  moved: false,
  keys: [],
  login: false,
  compare: [],
  ...over,
});

export function planFixture(id: string): RestorePlan {
  const p = packsFixture().find((x) => x.id === id) ?? packsFixture()[0];
  const kitHere = { kind: 'stdio' as const, command: 'design-kit-mcp', args: ['--theme', 'light'], env: [{ name: 'DESIGN_KIT_TOKEN', value: MASK }], cwd: null };
  const kitPack = { ...kitHere, args: ['--theme', 'dark'] };
  return {
    pack: p.id,
    mac: p.mac ?? 'Travel Air',
    at: p.at ?? NOW,
    items: [
      item({ name: 'docs-search', agents: ['claude', 'codex', 'agy'], keys: ['header:Authorization'] }),
      item({ name: 'files', agents: ['claude', 'codex'], moved: true }),
      item({ name: 'tracker', agents: ['codex', 'agy'], login: true }),
      item({ name: 'notes', agents: ['codex'], missingCommand: 'notes-mcp' }),
      item({
        name: 'design-kit',
        kind: 'replace',
        checked: false,
        agents: ['claude'],
        compare: [{ agent: 'claude', here: view(kitHere, ['env:DESIGN_KIT_TOKEN']), pack: view(kitPack, ['env:DESIGN_KIT_TOKEN']) }],
      }),
    ],
    same: ['browser'],
    keys: [{ mcp: 'docs-search', slot: 'header:Authorization', label: 'Authorization', bitwarden: 'Docs Search API' }],
    skipped: [],
    bases: { claude: 'b-claude', codex: 'b-codex', agy: 'b-agy' },
    skills: {
      items: [
        ...['pdf-tools', 'ui-polish', 'release-notes', 'sql-helper', 'changelog-writer'].map((name) => skill(name, 'github')),
        ...['deploy-check', 'trace-reader', 'notes-helper'].map((name) => skill(name, 'hand')),
      ],
      same: [],
      leftOut: [],
    },
    plugins: {
      items: [
        plugin('claude', 'issue-hub@acme-plugins', 'acme-plugins'),
        plugin('claude', 'review-kit@acme-plugins', null, false),
        plugin('claude', 'formatter@tools-market', 'tools-market'),
        plugin('claude', 'design-sync@acme-plugins', null),
        plugin('codex', 'sheets@codex-market', 'codex-market'),
        plugin('codex', 'slides@codex-market', null),
        plugin('codex', 'notes-sync@codex-market', null),
      ],
      same: [],
      skipped: [],
    },
    files: {
      items: [
        { path: '~/.claude/CLAUDE.md', agent: 'claude', kind: 'add', checked: true, changed: 0, diff: [] },
        {
          path: '~/.codex/AGENTS.md',
          agent: 'codex',
          kind: 'replace',
          checked: false,
          changed: 3,
          diff: [
            { op: 'ctx', text: '# Codex', secret: false },
            { op: 'del', text: 'Run the tests first.', secret: false },
            { op: 'add', text: 'Run the tests and the linter first.', secret: false },
            { op: 'add', text: 'Keep commits small.', secret: false },
            { op: 'add', text: 'Write Vietnamese in the app.', secret: false },
          ],
        },
      ],
      same: ['~/.claude/rules/style.md', '~/.claude/rules/review.md', '~/.gemini/GEMINI.md'],
      kept: [],
      bases: { '~/.claude/CLAUDE.md': '', '~/.codex/AGENTS.md': 'b-agents' },
    },
  };
}

function skill(name: string, kind: 'github' | 'hand'): RestoreSkillItem {
  return { name, kind, repo: kind === 'github' ? 'acme/agent-skills' : null, on: ['claude', 'agy'], checked: true, blocked: null };
}

function plugin(agent: 'claude' | 'codex', id: string, market: string | null, enabled = true): RestorePluginItem {
  return { agent, id, enabled, market, checked: true, note: null };
}

/** How the skills, plugins and files ticked end; with `partial`, the formatter plugin needs a login. */
export function fakeParts(plan: RestorePlan, more: RestoreMore, partial: boolean): RestorePartResult[] {
  const ok = (part: RestorePartResult['part'], id: string, agent: RestorePartResult['agent'], kind: string): RestorePartResult => ({
    part,
    id,
    agent,
    kind,
    ok: true,
    why: null,
    retry: false,
  });
  return [
    ...(plan.skills?.items ?? []).filter((i) => more.skills.includes(i.name)).map((i) => ok('skill', i.name, null, i.kind)),
    ...(plan.plugins?.items ?? [])
      .filter((i) => more.plugins.some((p) => p.agent === i.agent && p.id === i.id))
      .map((i) => (partial && i.id.startsWith('formatter@') ? { ...ok('plugin', i.id, i.agent, ''), ok: false, why: 'login' as const, retry: true } : ok('plugin', i.id, i.agent, ''))),
    ...(plan.files?.items ?? []).filter((f) => more.files.includes(f.path)).map((f) => ok('file', f.path, f.agent, f.kind)),
  ];
}

/** What a restore of `picks` ends as, with `pasted` the slots a key was pasted for. */
export function fakeRestore(plan: RestorePlan, picks: { name: string; kind: string }[], pasted: string[], at: number): RestoreDone {
  const items = plan.items
    .filter((i) => picks.some((p) => p.name === i.name && p.kind === i.kind))
    .map((i) => {
      const noKey = i.keys.some((s) => !pasted.includes(i.name + '|' + s));
      const why = noKey ? ('noKey' as const) : i.missingCommand ? ('noCommand' as const) : null;
      if (i.kind === 'replace')
        return { name: i.name, kind: i.kind, done: why ? [] : i.agents, kept: [], why: null, failed: [], skipped: why, login: false, command: null };
      return {
        name: i.name,
        kind: i.kind,
        done: why ? [] : i.agents,
        kept: why ? i.agents : [],
        why,
        failed: [],
        skipped: null,
        login: i.login,
        command: i.missingCommand,
      };
    });
  const written = items.some((i) => i.done.length) ? [{ agent: 'codex' as const, changes: items.filter((i) => i.done.includes('codex')).length, backup: 'config.toml.20260116-142000' }] : [];
  return { at, items, failed: null, written, keys: pasted.length, canUndo: items.length > 0 };
}
