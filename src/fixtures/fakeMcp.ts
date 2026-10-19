// The MCP tab's fake Rust side (M9), for the tests and the browser preview:
// a preview that looks like `agents::batch::preview`'s, a write that changes
// the fixture table, and a paste read in the page. No file, no Keychain, no
// `claude`; every secret is the mask.

import type { Agent, McpBlock, McpChange, McpDef, McpLine, McpOutcome, McpPaste, McpPreview, McpRefusal, McpRow, McpTable } from '../lib/model';

const MASK = '••••••';
const FILES: Record<Agent, string> = { claude: '~/.claude.json', codex: '~/.codex/config.toml', agy: '~/.gemini/config/mcp_config.json' };
const BACKUPS: Record<Agent, string> = {
  claude: '~/Library/Application Support/com.shin315.quota-station/backups/claude',
  codex: '~/Library/Application Support/com.shin315.quota-station/backups/codex',
  agy: '~/Library/Application Support/com.shin315.quota-station/backups/antigravity',
};
const SECRET = /key|token|secret|password|auth|credential|cookie|bearer|session/i;

const line = (op: McpLine['op'], text: string, secret = false): McpLine => ({ op, text, secret });

function defOf(row: McpRow | undefined, agent: Agent): McpDef | null {
  if (!row) return null;
  return (row.defs[agent] ?? Object.values(row.defs)[0])?.def ?? null;
}

function masked(def: McpDef): McpDef {
  if (def.kind === 'http') return { ...def, headers: def.headers.map((p) => (SECRET.test(p.name) ? { ...p, value: MASK } : p)) };
  return { ...def, env: def.env.map((p) => (SECRET.test(p.name) ? { ...p, value: MASK } : p)) };
}

function codexLines(name: string, def: McpDef, op: 'add' | 'del'): McpLine[] {
  const out = [line(op, '[mcp_servers.' + name + ']')];
  if (def.kind === 'http') {
    out.push(line(op, 'url = "' + def.url + '"'));
    if (def.headers.length) out.push(line(op, 'http_headers = { ' + def.headers.map((p) => p.name + ' = "' + p.value + '"').join(', ') + ' }', def.headers.some((p) => p.value === MASK)));
  } else {
    out.push(line(op, 'command = "' + def.command + '"'));
    if (def.args.length) out.push(line(op, 'args = [' + def.args.map((a) => '"' + a + '"').join(', ') + ']'));
    if (def.env.length) out.push(line(op, 'env = { ' + def.env.map((p) => p.name + ' = "' + p.value + '"').join(', ') + ' }', def.env.some((p) => p.value === MASK)));
  }
  return out;
}

function agyLines(name: string, def: McpDef, op: 'add' | 'del'): McpLine[] {
  const out = [line(op, '"' + name + '": {')];
  if (def.kind === 'http') {
    out.push(line(op, '  "serverUrl": "' + def.url + '"' + (def.headers.length ? ',' : '')));
    if (def.headers.length) out.push(line(op, '  "headers": { ' + def.headers.map((p) => '"' + p.name + '": "' + p.value + '"').join(', ') + ' }', def.headers.some((p) => p.value === MASK)));
  } else {
    out.push(line(op, '  "command": "' + def.command + '"' + (def.args.length || def.env.length ? ',' : '')));
    if (def.args.length) out.push(line(op, '  "args": [' + def.args.map((a) => '"' + a + '"').join(', ') + ']' + (def.env.length ? ',' : '')));
    if (def.env.length) out.push(line(op, '  "env": { ' + def.env.map((p) => '"' + p.name + '": "' + p.value + '"').join(', ') + ' }', def.env.some((p) => p.value === MASK)));
  }
  out.push(line(op, '},'));
  return out;
}

function claudeJson(def: McpDef): string {
  const entry =
    def.kind === 'http'
      ? { type: 'http', url: def.url, ...(def.headers.length ? { headers: Object.fromEntries(def.headers.map((p) => [p.name, p.value])) } : {}) }
      : { type: 'stdio', command: def.command, args: def.args, ...(def.env.length ? { env: Object.fromEntries(def.env.map((p) => [p.name, p.value])) } : {}) };
  return JSON.stringify(entry);
}

function first(def: McpDef, agent: Agent): string {
  if (agent === 'codex') return def.kind === 'http' ? 'url = "' + def.url + '"' : 'command = "' + def.command + '"';
  return def.kind === 'http' ? '  "serverUrl": "' + def.url + '",' : '  "command": "' + def.command + '",';
}

function block(agent: Agent, row: McpRow | undefined, c: McpChange): McpBlock | McpRefusal {
  const cell = row?.cells[agent];
  const def = c.to.kind === 'def' ? masked(c.to.def) : defOf(row, agent);
  const name = c.name;
  const base = { name, commands: [] as string[], lines: [] as McpLine[], secret: !!row?.secrets.length, dropped: [] as string[], from: null as Agent | null, kept: false };
  if (cell?.lock && c.to.kind !== 'off' && c.to.kind !== 'absent') return cell.lock === 'readOnly' ? 'readOnly' : cell.lock;
  if (!def) return 'unknown';
  const head = agent === 'codex' ? '[mcp_servers.' + name + ']' : '"' + name + '": {';
  const adds = !cell || cell.state === 'absent';
  switch (c.to.kind) {
    case 'on':
      if (cell?.state === 'on') return 'nothing';
      if (agent === 'claude') return { ...base, action: 'add', commands: ['claude mcp add-json ' + name + " '" + claudeJson(def) + "' --scope user"] };
      if (adds) return { ...base, action: 'add', lines: agent === 'codex' ? codexLines(name, def, 'add') : agyLines(name, def, 'add') };
      return { ...base, action: 'on', lines: [line('ctx', head), line('ctx', first(def, agent)), line('del', agent === 'codex' ? 'enabled = false' : '  "disabled": true')] };
    case 'off':
      if (!cell || cell.state !== 'on') return 'nothing';
      if (agent === 'claude') return { ...base, action: 'off', commands: ['claude mcp remove ' + name + ' --scope user'], kept: true };
      return { ...base, action: 'off', lines: [line('ctx', head), line('ctx', first(def, agent)), line('add', agent === 'codex' ? 'enabled = false' : '  "disabled": true')] };
    case 'absent':
      if (adds) return 'nothing';
      if (agent === 'claude') return { ...base, action: 'remove', commands: cell?.parked ? [] : ['claude mcp remove ' + name + ' --scope user'], kept: c.to.store };
      return { ...base, action: 'remove', kept: c.to.store, lines: agent === 'codex' ? codexLines(name, def, 'del') : agyLines(name, def, 'del') };
    case 'def': {
      const from = c.to.from;
      if (agent === 'claude') return { ...base, action: adds ? 'add' : 'replace', from, commands: [...(adds ? [] : ['claude mcp remove ' + name + ' --scope user']), 'claude mcp add-json ' + name + " '" + claudeJson(def) + "' --scope user"] };
      if (adds) return { ...base, action: 'add', from, lines: agent === 'codex' ? codexLines(name, def, 'add') : agyLines(name, def, 'add') };
      const old = defOf(row, agent)!;
      const lines = agent === 'codex' ? codexLines(name, old, 'del') : agyLines(name, old, 'del');
      const next = agent === 'codex' ? codexLines(name, def, 'add') : agyLines(name, def, 'add');
      // Lines both have stay as context.
      const out: McpLine[] = [];
      lines.forEach((l, i) => {
        if (next[i] && next[i].text === l.text) out.push({ ...l, op: 'ctx' });
        else {
          out.push(l);
          if (next[i]) out.push(next[i]);
        }
      });
      next.slice(lines.length).forEach((l) => out.push(l));
      return { ...base, action: 'replace', from, lines: out };
    }
  }
}

export function fakePreview(table: McpTable, changes: McpChange[]): McpPreview {
  const groups: McpPreview['groups'] = [];
  const refused: McpPreview['refused'] = [];
  const accepted: McpChange[] = [];
  for (const agent of ['codex', 'agy', 'claude'] as Agent[]) {
    const blocks: McpBlock[] = [];
    for (const c of changes.filter((x) => x.agent === agent)) {
      const col = table.columns.find((x) => x.agent === agent);
      if (!col?.writable) {
        refused.push({ name: c.name, agent, reason: 'readOnly' });
        continue;
      }
      const b = block(agent, table.rows.find((r) => r.name === c.name), c);
      if (typeof b === 'string') refused.push({ name: c.name, agent, reason: b });
      else {
        blocks.push(b);
        accepted.push(c);
      }
    }
    if (blocks.length) groups.push({ agent, file: FILES[agent], backups: BACKUPS[agent], blocks });
  }
  const bases = Object.fromEntries(groups.map((g) => [g.agent, 'base-' + g.agent])) as McpPreview['bases'];
  return { groups, refused, changes: accepted, bases };
}

function inverse(table: McpTable, c: McpChange): McpChange {
  const row = table.rows.find((r) => r.name === c.name);
  const cell = row?.cells[c.agent];
  if (!cell || cell.state === 'absent') return { name: c.name, agent: c.agent, to: { kind: 'absent', store: false } };
  if (c.to.kind === 'def' || c.to.kind === 'absent') return { name: c.name, agent: c.agent, to: { kind: 'def', def: defOf(row, c.agent)!, from: null, keychain: [], off: cell.state === 'off' } };
  return { name: c.name, agent: c.agent, to: { kind: cell.state === 'on' ? 'on' : 'off' } };
}

/** The table after `changes`, as a write that went through leaves it. */
export function fakeApply(table: McpTable, changes: McpChange[], at: number): { table: McpTable; outcome: McpOutcome } {
  const undo = changes.map((c) => inverse(table, c));
  let rows = table.rows.map((r) => ({ ...r, cells: { ...r.cells }, defs: { ...r.defs } }));
  let stored = [...table.stored];
  for (const c of changes) {
    let row = rows.find((r) => r.name === c.name);
    const kept = stored.find((s) => s.name === c.name);
    if (!row && c.to.kind === 'def') {
      const d = c.to.def;
      row = {
        name: c.name,
        kind: d.kind === 'http' ? 'http' : 'stdio',
        program: d.kind === 'stdio' ? (d.command === 'npx' ? (d.args.find((a) => !a.startsWith('-')) ?? d.command) : d.command) : null,
        host: d.kind === 'http' ? new URL(d.url).host : null,
        secrets: c.to.keychain.map((id) => ({ id, label: id.split(':').pop() ?? id, place: id.startsWith('header') ? 'header' : 'env', have: true, bitwarden: null })),
        cells: { claude: { state: 'absent', lock: null, parked: false }, codex: { state: 'absent', lock: null, parked: false }, agy: { state: 'absent', lock: null, parked: false } },
        differs: false,
        missingCommand: null,
        defs: {},
        sameSecrets: {},
      };
      rows = [...rows, row].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
    }
    if (!row && kept && c.to.kind === 'on') {
      row = { name: kept.name, kind: kept.kind, program: kept.program, host: kept.host, secrets: [], cells: { claude: { state: 'absent', lock: null, parked: false }, codex: { state: 'absent', lock: null, parked: false }, agy: { state: 'absent', lock: null, parked: false } }, differs: false, missingCommand: null, defs: { [c.agent]: kept.def }, sameSecrets: {} };
      rows = [...rows, row].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
      stored = stored.filter((s) => s.name !== kept.name);
    }
    if (!row) continue;
    const any = Object.values(row.defs)[0];
    if (c.to.kind === 'on') row.cells[c.agent] = { state: 'on', lock: null, parked: false };
    if (c.to.kind === 'off') row.cells[c.agent] = { state: 'off', lock: null, parked: c.agent === 'claude' };
    if (c.to.kind === 'def') row.cells[c.agent] = { state: c.to.off ? 'off' : row.cells[c.agent].state === 'off' ? 'off' : 'on', lock: null, parked: false };
    if (c.to.kind === 'on' || c.to.kind === 'def') row.defs[c.agent] = c.to.kind === 'def' ? { def: masked(c.to.def), slots: row.secrets.map((s) => s.id), extra: [] } : (row.defs[c.agent] ?? any);
    if (c.to.kind === 'absent') {
      row.cells[c.agent] = { state: 'absent', lock: null, parked: false };
      delete row.defs[c.agent];
      const left = Object.values(row.cells).some((x) => x.state !== 'absent');
      if (!left) {
        if (c.to.store && any) stored = [...stored, { name: row.name, kind: row.kind, program: row.program, host: row.host, at, agents: [c.agent], def: any, missing: [] }];
        rows = rows.filter((r) => r !== row);
      }
    }
    if (row.defs.claude && row.defs.agy) row.differs = row.differs && c.to.kind !== 'def';
  }
  const agents = [...new Set(changes.map((c) => c.agent))];
  const stamp = new Date(at * 1000).toISOString().replace(/[-:]/g, '').slice(0, 15).replace('T', '-');
  const backupName: Record<Agent, string> = { claude: 'mcpServers.json.', codex: 'config.toml.', agy: 'mcp_config.json.' };
  return {
    table: { ...table, rows, stored },
    outcome: {
      done: agents.map((agent) => ({ agent, changes: changes.filter((c) => c.agent === agent).length, backup: backupName[agent] + stamp })),
      failed: null,
      undo: { changes: undo, bases: Object.fromEntries(agents.map((a) => [a, 'after-' + a])) },
      at,
    },
  };
}

/** A paste read in the page: `mcpServers`, `servers` or a bare map; TOML is left to the real side. */
export function fakePaste(text: string): McpPaste {
  const empty = { servers: [], skipped: [], detail: null };
  if (!text.trim()) return { ...empty, error: 'empty' };
  let root: unknown;
  try {
    root = JSON.parse(text.trim().startsWith('"') ? '{' + text + '}' : text);
  } catch {
    return { ...empty, error: 'parse', detail: 'JSON' };
  }
  const o = root as Record<string, unknown>;
  if ('command' in o || 'url' in o) return { ...empty, error: 'noName' };
  const map = ((o.mcpServers ?? o.servers ?? o) as Record<string, Record<string, unknown>>) || {};
  const servers: McpPaste['servers'] = [];
  const skipped: string[] = [];
  for (const [name, v] of Object.entries(map).sort(([a], [b]) => a.localeCompare(b))) {
    if (!/^[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}$/.test(name) || typeof v !== 'object' || !v || !(v.command || v.url || v.serverUrl)) {
      skipped.push(name);
      continue;
    }
    const pairs = (x: unknown) => Object.entries((x as Record<string, string>) ?? {}).map(([k, value]) => ({ name: k, value: SECRET.test(k) ? MASK : String(value) }));
    const def: McpDef = v.command
      ? { kind: 'stdio', command: String(v.command), args: (v.args as string[]) ?? [], env: pairs(v.env), cwd: null }
      : { kind: 'http', url: String(v.url ?? v.serverUrl), headers: pairs(v.headers), sse: v.type === 'sse' };
    const slots = (def.kind === 'http' ? def.headers : def.env)
      .filter((p) => p.value === MASK)
      .map((p) => ({ id: (def.kind === 'http' ? 'header:' : 'env:') + p.name, label: p.name, place: def.kind === 'http' ? ('header' as const) : ('env' as const) }));
    servers.push({ name, def, slots });
  }
  return { servers, skipped, error: servers.length || skipped.length ? null : 'none', detail: null };
}
