// The MCP tab's pending list and column menus (M9), on the fixture table.

import { describe, expect, it } from 'vitest';
import { mcpTableFixture } from '../fixtures/models';
import {
  changesOf,
  clearColumn,
  columnMenu,
  formDef,
  formError,
  formOf,
  isSecretPair,
  lineHasSecret,
  MASK,
  pendingList,
  pendingText,
  previewLead,
  rowAll,
  rowCanAll,
  setColumn,
  shown,
  splitCommand,
  toggle,
  whereLine,
  type Pending,
} from './mcp';

const table = mcpTableFixture();
const row = (name: string) => table.rows.find((r) => r.name === name)!;

describe('the boxes not written yet', () => {
  it('a tick is pending until it is ticked back, then it is no change at all', () => {
    let p: Pending = {};
    p = toggle(p, row('browser'), 'agy');
    expect(shown(p, row('browser'), 'agy')).toBe(true);
    expect(pendingList(p, table.rows).map(pendingText)).toEqual(['browser: bật cho Antigravity']);
    p = toggle(p, row('browser'), 'agy');
    expect(p).toEqual({});
    expect(pendingList(p, table.rows)).toEqual([]);
  });

  it('an empty box adds the server; a ticked one turns it off', () => {
    let p: Pending = {};
    p = toggle(p, row('docs-search'), 'agy');
    p = toggle(p, row('files'), 'claude');
    expect(pendingList(p, table.rows).map(pendingText)).toEqual(['docs-search: thêm cho Antigravity', 'files: tắt cho Claude Code']);
    expect(changesOf(p, table.rows)).toEqual([
      { name: 'docs-search', agent: 'agy', to: { kind: 'on' } },
      { name: 'files', agent: 'claude', to: { kind: 'off' } },
    ]);
  });

  it('a locked box does not tick, and the column\'s Bật tất cả leaves it out', () => {
    expect(toggle({}, row('ticket-desk'), 'codex')).toEqual({});
    const p = setColumn({}, table.rows, 'agy', true);
    expect(Object.keys(p).sort()).toEqual(['browser|agy', 'docs-search|agy', 'tracker|agy']);
    expect(p['ticket-desk|agy']).toBeUndefined();
    const menu = columnMenu(p, table.rows, 'agy');
    expect(menu).toEqual({ total: 6, ticked: 6, pending: 3 });
  });

  it('Tắt tất cả, then Bỏ N thay đổi chưa ghi of one column leaves the others', () => {
    let p = setColumn({}, table.rows, 'codex', false);
    p = toggle(p, row('browser'), 'agy');
    expect(columnMenu(p, table.rows, 'codex')).toMatchObject({ ticked: 0, pending: 3 });
    p = clearColumn(p, 'codex');
    expect(p).toEqual({ 'browser|agy': true });
  });

  it('Bật cả ba ticks what can be ticked in the row, and is not offered once all are', () => {
    expect(rowCanAll({}, row('files'))).toBe(false);
    const p = rowAll({}, row('notes'));
    expect(p).toEqual({ 'notes|claude': true, 'notes|codex': true });
    expect(rowCanAll(p, row('notes'))).toBe(false);
  });
});

describe('the words around the table', () => {
  it('says where a server runs, never more', () => {
    expect(whereLine(row('files'))).toBe('trên máy · files-mcp');
    expect(whereLine(row('docs-search'))).toBe('qua mạng · docs.example.com');
  });

  it('puts one server\'s like changes in one phrase', () => {
    const lead = previewLead(
      {
        groups: [],
        refused: [],
        bases: {},
        changes: [
          { name: 'notes', agent: 'codex', to: { kind: 'on' } },
          { name: 'notes', agent: 'claude', to: { kind: 'on' } },
          { name: 'browser', agent: 'claude', to: { kind: 'off' } },
          { name: 'browser', agent: 'codex', to: { kind: 'off' } },
          { name: 'files', agent: 'agy', to: { kind: 'off' } },
        ],
      },
      table.rows,
    );
    expect(lead).toBe('notes: bật cho Codex · notes: thêm cho Claude Code · browser: tắt cho Claude Code, Codex · files: tắt cho Antigravity');
  });
});

describe('the add and edit form', () => {
  it('knows a secret by its name or its shape, not a reference to one', () => {
    expect(isSecretPair('GITHUB_TOKEN', 'x')).toBe(true);
    expect(isSecretPair('LOG_LEVEL', 'sk-abcdefghijklmnop')).toBe(true);
    expect(isSecretPair('GITHUB_TOKEN', '${GITHUB_TOKEN}')).toBe(false);
    expect(isSecretPair('KEY_FILE', '/tmp/k')).toBe(false);
    expect(lineHasSecret('npx tool --api-key abcdef')).toBe(true);
    expect(lineHasSecret('npx tool --token=abc')).toBe(true);
    expect(lineHasSecret('npx -y files-mcp ~/Projects')).toBe(false);
  });

  it('splits a command as a shell would', () => {
    expect(splitCommand('npx -y "my tool" \'a b\' c')).toEqual(['npx', '-y', 'my tool', 'a b', 'c']);
  });

  it('sends a secret as the mask, and keeps what the form does not show', () => {
    const view = row('design-kit').defs.claude!;
    const f = formOf('design-kit', { ...view.def, cwd: '/srv/kit' } as typeof view.def, view.slots, { claude: true, codex: false, agy: true });
    expect(f.pairs[0]).toMatchObject({ name: 'DESIGN_KIT_TOKEN', value: '', secret: true, kept: true });
    expect(formError(f, ['design-kit'])).toBeNull();
    const def = formDef(f);
    expect(def).toEqual({ kind: 'stdio', command: 'design-kit-mcp', args: ['--theme', 'dark'], env: [{ name: 'DESIGN_KIT_TOKEN', value: MASK }], cwd: '/srv/kit' });
  });

  it('refuses a name the agents do not take, a taken name, and a secret left empty', () => {
    const base = { kind: 'stdio' as const, line: 'x-mcp', pairs: [], bitwarden: '', agents: { claude: true, codex: false, agy: false }, edit: false, cwd: null, sse: false };
    expect(formError({ ...base, name: 'two words' }, [])).toMatch(/Tên chỉ có/);
    expect(formError({ ...base, name: 'files' }, ['files'])).toBe('Đã có MCP tên này.');
    expect(formError({ ...base, name: 'x', pairs: [{ name: 'API_KEY', value: '', secret: true, kept: false }] }, [])).toMatch(/giá trị khoá/);
    expect(formError({ ...base, name: 'x', agents: { claude: false, codex: false, agy: false } }, [])).toBe('Chọn ít nhất một agent.');
  });
});
