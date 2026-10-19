// The Bộ cấu hình tab's fake Rust side (M15), for the tests and the browser
// preview: three made-up sets named after made-up projects, what a set
// changes in the fixture MCP and Skills tables as `agents::sets::changes`
// decides it, and its drift. No file, no `claude`, no `npx`.

import type { Agent, ConfigSet, McpChange, McpTable, SetDrifted, SetSkipped, SetsView, SkillChange, SkillTable } from '../lib/model';

const AGENTS: Agent[] = ['claude', 'codex', 'agy'];

export const WEB: ConfigSet = {
  id: 'web',
  name: 'Web app',
  mcp: { 'docs-search': 'on', browser: 'on', 'design-kit': 'on', tracker: 'off' },
  skills: { 'pdf-tools': 'on', 'deploy-check': 'on' },
  codex: null,
};
export const DOCS: ConfigSet = {
  id: 'docs',
  name: 'Viết tài liệu',
  mcp: { 'docs-search': 'on', browser: 'off', 'design-kit': 'off' },
  skills: { 'pdf-tools': 'on', 'release-notes': 'on' },
  codex: 'chatGpt',
};
export const MIN: ConfigSet = {
  id: 'min',
  name: 'Tối giản',
  mcp: { 'docs-search': 'on', browser: 'off', 'design-kit': 'off', files: 'off', tracker: 'off' },
  skills: {},
  codex: null,
};

/** Web app applied this morning; browser has been turned off in Antigravity since. */
export function setsFixture(over: Partial<SetsView> = {}): SetsView {
  return {
    sets: [WEB, DOCS, MIN].map((s) => structuredClone(s)),
    applied: {
      id: 'web',
      name: 'Web app',
      at: '2026-01-16T02:20:00Z',
      drift: [{ kind: 'mcp', name: 'browser', want: 'on', agents: ['agy'] }],
      canUndo: true,
      codexBefore: null,
    },
    busy: null,
    removed: null,
    ...over,
  };
}

/** What `set` changes in the tables, as `agents::sets::changes` decides it. */
export function fakeSetChanges(set: ConfigSet, mcp: McpTable | null, skills: SkillTable | null): { mcp: McpChange[]; skills: SkillChange[]; skipped: SetSkipped[] } {
  const out = { mcp: [] as McpChange[], skills: [] as SkillChange[], skipped: [] as SetSkipped[] };
  for (const [name, want] of Object.entries(set.mcp)) {
    const row = mcp?.rows.find((r) => r.name === name);
    if (!row) {
      out.skipped.push({ kind: 'mcp', name, agent: null, reason: mcp?.owned.some((o) => o.name === name) ? 'owned' : 'notHere' });
      continue;
    }
    for (const agent of AGENTS) {
      const c = row.cells[agent];
      if (c.state === 'absent' || (c.state === 'on') === (want === 'on')) continue;
      if (c.lock) out.skipped.push({ kind: 'mcp', name, agent, reason: c.lock });
      else out.mcp.push({ name, agent, to: { kind: want } });
    }
  }
  for (const [name, want] of Object.entries(set.skills)) {
    const row = skills?.rows.find((r) => r.name === name);
    if (!row) {
      out.skipped.push({ kind: 'skill', name, agent: null, reason: 'notHere' });
      continue;
    }
    for (const agent of AGENTS) {
      const c = row.cells[agent];
      if (c.state === 'absent' || (c.state === 'on') === (want === 'on')) continue;
      if (c.lock) out.skipped.push({ kind: 'skill', name, agent, reason: c.lock });
      else out.skills.push({ name, agent, on: want === 'on' });
    }
  }
  return out;
}

export function fakeSetDrift(set: ConfigSet, mcp: McpTable | null, skills: SkillTable | null): SetDrifted[] {
  const ch = fakeSetChanges(set, mcp, skills);
  const out: SetDrifted[] = [];
  const add = (kind: 'mcp' | 'skill', name: string, on: boolean, agent: Agent) => {
    const d = out.find((x) => x.kind === kind && x.name === name);
    if (d) d.agents.push(agent);
    else out.push({ kind, name, want: on ? 'on' : 'off', agents: [agent] });
  };
  ch.mcp.forEach((c) => add('mcp', c.name, c.to.kind === 'on', c.agent));
  ch.skills.forEach((c) => add('skill', c.name, c.on, c.agent));
  return out;
}
