// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The MCP tab's logic (M9), pure so Vitest covers it without a window: the
// boxes ticked and not written yet, the column menus, the batch they become,
// and the words around them. Copy follows the approved phase 2 prototype.
// The boxes' helpers take any row of the matrix, so the Skills tab (M11)
// uses them too.

import { clock } from './format';
import { mac, secretStore, t } from './i18n';
import type { Agent, McpCell, McpChange, McpDef, McpFailure, McpLock, McpOutcome, McpPreview, McpRefusal, McpRow, McpTable } from './model';
import type { ChipView, FeedbackView } from './view';

export const AGENTS: Agent[] = ['claude', 'codex', 'agy'];
export const AGENT_NAME: Record<Agent, string> = { claude: 'Claude Code', codex: 'Codex', agy: 'Antigravity' };
/** What the windows show, and send back, in a secret's place. */
export const MASK = '••••••';

/** Boxes ticked and not written yet: `name|agent` → what the box shows now. */
export type Pending = Record<string, boolean>;

export const pkey = (name: string, agent: Agent): string => name + '|' + agent;

/** A row of an Agent tab's matrix: an MCP server or a skill. */
export interface BoxRow {
  name: string;
  cells: Record<Agent, { state: 'on' | 'off' | 'absent'; lock: string | null }>;
}

/** The box as the file has it. */
export const original = (cell: Pick<McpCell, 'state'>): boolean => cell.state === 'on';

export function shown(pending: Pending, row: BoxRow, agent: Agent): boolean {
  const p = pending[pkey(row.name, agent)];
  return p === undefined ? original(row.cells[agent]) : p;
}

export const tickable = (row: BoxRow, agent: Agent): boolean => row.cells[agent].lock == null;

/** `pending` with the box set to `on`; set back to what the file has, it is no change at all. */
export function setBox(pending: Pending, row: BoxRow, agent: Agent, on: boolean): Pending {
  if (!tickable(row, agent)) return pending;
  const next = { ...pending };
  const key = pkey(row.name, agent);
  if (on === original(row.cells[agent])) delete next[key];
  else next[key] = on;
  return next;
}

export const toggle = (pending: Pending, row: BoxRow, agent: Agent): Pending => setBox(pending, row, agent, !shown(pending, row, agent));

/** "Bật tất cả" / "Tắt tất cả" of a column: a box that cannot be ticked is left as it is. */
export function setColumn(pending: Pending, rows: BoxRow[], agent: Agent, on: boolean): Pending {
  return rows.reduce((p, row) => setBox(p, row, agent, on), pending);
}

/** "Bỏ N thay đổi chưa ghi" of a column. */
export function clearColumn(pending: Pending, agent: Agent): Pending {
  return Object.fromEntries(Object.entries(pending).filter(([k]) => !k.endsWith('|' + agent)));
}

/** "Bật cả ba": every box of the row that can be ticked. */
export const rowAll = (pending: Pending, row: BoxRow): Pending => AGENTS.reduce((p, a) => setBox(p, row, a, true), pending);

/** Whether "Bật cả ba" has anything to do. */
export const rowCanAll = (pending: Pending, row: BoxRow): boolean => AGENTS.some((a) => tickable(row, a) && !shown(pending, row, a));

export interface PendingItem {
  name: string;
  agent: Agent;
  on: boolean;
  /** Not in the agent's list yet: ticking it adds it. */
  adds: boolean;
}

/** The changes not written, in the table's order, then the columns'. */
export function pendingList(pending: Pending, rows: BoxRow[]): PendingItem[] {
  const out: PendingItem[] = [];
  for (const row of rows)
    for (const agent of AGENTS) {
      const on = pending[pkey(row.name, agent)];
      if (on !== undefined) out.push({ name: row.name, agent, on, adds: on && row.cells[agent].state === 'absent' });
    }
  return out;
}

export const pendingText = (p: PendingItem): string => {
  const a = AGENT_NAME[p.agent];
  if (!p.on) return t(p.name + ': tắt cho ' + a, p.name + ': turn off for ' + a);
  return p.adds ? t(p.name + ': thêm cho ' + a, p.name + ': add to ' + a) : t(p.name + ': bật cho ' + a, p.name + ': turn on for ' + a);
};

export function changesOf(pending: Pending, rows: McpRow[]): McpChange[] {
  return pendingList(pending, rows).map((p) => ({ name: p.name, agent: p.agent, to: { kind: p.on ? 'on' : 'off' } }));
}

export interface ColumnMenu {
  /** Boxes of the column that can be ticked, and how many are ticked now. */
  total: number;
  ticked: number;
  pending: number;
}

export function columnMenu(pending: Pending, rows: BoxRow[], agent: Agent): ColumnMenu {
  const can = rows.filter((r) => tickable(r, agent));
  return {
    total: can.length,
    ticked: can.filter((r) => shown(pending, r, agent)).length,
    pending: Object.keys(pending).filter((k) => k.endsWith('|' + agent)).length,
  };
}

/** The column can be written at all. */
export function columnOpen(table: McpTable, agent: Agent): boolean {
  return !!table.columns.find((c) => c.agent === agent)?.writable;
}

/** Under the name: runs on this Mac, and what; or over the network, and where. */
export function whereLine(row: { kind: McpRow['kind']; program: string | null; host: string | null }): string {
  if (row.kind === 'stdio') return mac(t('trên máy · ', 'on this Mac · ')) + (row.program ?? t('lệnh', 'command'));
  if (row.kind === 'http' || row.kind === 'sse') return t('qua mạng · ', 'over the network · ') + (row.host ?? t('địa chỉ', 'address')) + (row.kind === 'sse' ? ' · SSE' : '');
  return t('kiểu app chưa đọc được', 'a kind the app cannot read yet');
}

/** Chips only when something is the matter. */
export function rowChips(row: McpRow): ChipView[] {
  const out: ChipView[] = [];
  if (row.secrets.some((s) => !s.have))
    out.push({
      t: t('thiếu khoá', 'key missing'),
      k: 'bad',
      title: t('Không thấy khoá trong file của agent nào hay trong ' + secretStore() + '. Dán khoá để bật.', 'No key in any agent’s file or in the ' + secretStore() + '. Paste the key to turn it on.'),
    });
  const m = row.missingCommand;
  if (m?.path) out.push({ t: t('lệnh chưa cài', 'command not installed'), k: 'bad', title: mac(t(m.command + ' không có trên máy này.', m.command + ' is not on this Mac.')) });
  else if (m)
    out.push({
      t: t('không thấy lệnh', 'command not found'),
      k: 'warn',
      title: t(
        'Không thấy ' + m.command + ' ở các thư mục thường gặp. Agent vẫn có thể tìm thấy trong PATH của nó.',
        m.command + ' is not in the usual folders. The agent may still find it on its PATH.',
      ),
    });
  if (row.differs)
    out.push({ t: t('khác giữa các agent', 'differs between agents'), k: 'warn', title: t('Cùng tên mà cấu hình khác nhau. Bấm Sửa để so.', 'Same name, different settings. Press Edit to compare.') });
  return out;
}

export function lockText(lock: McpLock): string {
  switch (lock) {
    case 'readOnly':
      return t('cột này chỉ xem', 'this column is view only');
    case 'shape':
      return t('mục viết kiểu app không sửa', 'written in a form the app does not edit');
    case 'missingKey':
      return t('thiếu khoá', 'key missing');
    case 'noCommand':
      return t('lệnh chưa cài', 'command not installed');
    case 'sse':
      return t('Codex không chạy MCP kiểu SSE', 'Codex does not run SSE MCP servers');
  }
}

/** The box's hover text. */
export function cellTitle(row: McpRow, agent: Agent): string {
  const c = row.cells[agent];
  if (c.lock) return row.name + ': ' + lockText(c.lock);
  if (c.parked)
    return t('Đang tắt: app giữ cấu hình để bật lại cho ' + AGENT_NAME[agent] + '.', 'Off: the app keeps its settings to turn it back on for ' + AGENT_NAME[agent] + '.');
  if (c.state === 'absent') return t('Chưa có ở ' + AGENT_NAME[agent] + '. Tick để thêm.', 'Not in ' + AGENT_NAME[agent] + ' yet. Tick to add it.');
  return '';
}

export function refusalText(reason: McpRefusal): string {
  switch (reason) {
    case 'readOnly':
      return t('file của agent này chỉ xem', 'this agent’s file is view only');
    case 'shape':
      return t('mục viết kiểu app không sửa', 'written in a form the app does not edit');
    case 'owned':
      return t('MCP có sẵn của agent, app để yên', 'the agent’s built-in MCP, left alone');
    case 'missingKey':
      return t('thiếu khoá', 'key missing');
    case 'noCommand':
      return t('lệnh chưa cài', 'command not installed');
    case 'sse':
      return t('Codex không chạy MCP kiểu SSE', 'Codex does not run SSE MCP servers');
    case 'badName':
      return t('tên chỉ được có chữ, số, dấu - _ .', 'names may only have letters, digits, - _ .');
    case 'unknown':
      return t('không có cấu hình để chép', 'no settings to copy');
    case 'nothing':
      return t('đã như vậy rồi', 'already that way');
  }
}

/** What restarting each agent takes, after a write. */
export function restartText(agent: Agent): string {
  if (agent === 'claude') return t('Phiên Claude Code đang chạy thấy thay đổi ở phiên mở sau.', 'Running Claude Code sessions: the change shows in the next session you open.');
  if (agent === 'codex') return t('Codex App và phiên codex đang mở dùng danh sách cũ đến khi mở lại.', 'Codex App and open codex sessions use the old list until reopened.');
  return t('Antigravity đọc lại khi mở lại.', 'Antigravity reads it again when reopened.');
}

export function fileName(agent: Agent): string {
  return agent === 'claude' ? t('danh sách MCP của Claude Code', 'Claude Code’s MCP list') : agent === 'codex' ? 'config.toml' : 'mcp_config.json';
}

function names(list: string[]): string {
  return list.length <= 1 ? list.join('') : list.slice(0, -1).join(', ') + t(' và ', ' and ') + list[list.length - 1];
}

/** "N thay đổi", with English's plural. */
export function changesWord(n: number): string {
  return t(n + ' thay đổi', n + (n === 1 ? ' change' : ' changes'));
}

/** " Đã sao lưu … lúc …." after a write, or nothing. */
function backedUp(kept: string, at: string): string {
  return kept ? t('Đã sao lưu ' + kept + ' lúc ' + at + '.', 'Backed up ' + kept + ' at ' + at + '.') : '';
}

/** The line after a write: what was written, its backups, and the undo. */
export function writtenFeedback(out: McpOutcome, undo: boolean): FeedbackView {
  const n = out.done.reduce((sum, d) => sum + d.changes, 0);
  const at = clock(new Date(out.at * 1000).toISOString());
  const kept = backedUp(names(out.done.filter((d) => d.backup).map((d) => fileName(d.agent))), at);
  const failed = out.failed ? failureText(out.failed.error, out.failed.agent) : null;
  const actions = [...(undo && out.undo ? [{ label: t('Hoàn tác', 'Undo'), act: 'mcp:undo' as const }] : []), { label: t('Bản lưu', 'Backups'), act: 'mcp:backups' as const }];
  if (failed && !n)
    return { tone: 'bad', title: failed.title, body: failed.body, actions: failed.reread ? [{ label: t('Đọc lại', 'Read again'), act: 'mcp:reread' }] : undefined };
  if (failed)
    return {
      tone: 'warn',
      title: t('Đã ghi ' + changesWord(n) + ', phần còn lại chưa ghi.', 'Wrote ' + changesWord(n) + '; the rest were not written.'),
      body: failed.title + ' ' + failed.body + (kept ? ' ' + kept : ''),
      actions,
    };
  return {
    tone: 'info',
    title: t('Đã ghi ' + changesWord(n) + '.', 'Wrote ' + changesWord(n) + '.'),
    body: (kept ? kept + ' ' : '') + t('Mở lại agent để dùng.', 'Reopen the agent to use it.'),
    actions,
  };
}

export function undoneFeedback(out: McpOutcome): FeedbackView {
  if (out.failed && !out.done.length) {
    const f = failureText(out.failed.error, out.failed.agent);
    return { tone: 'bad', title: t('Chưa hoàn tác: ', 'Not undone: ') + f.cause, body: f.body };
  }
  return {
    tone: 'info',
    title: t('Đã hoàn tác lần ghi vừa rồi.', 'Undid the last write.'),
    body: t('Các file trở về như trước lần ghi, có bản lưu của lúc trước khi hoàn tác.', 'The files are back as they were before the write, with a backup from just before the undo.'),
  };
}

/** "Chưa đổi gì: …", the title of a write that changed nothing. */
const nothingChanged = (cause: string): string => t('Chưa đổi gì: ', 'Nothing changed: ') + cause;

/** Why a write failed; `cause` is the title without "Chưa đổi gì: ", for the undo's line. */
export function failureText(f: McpFailure, agent: Agent): { title: string; body: string; reread: boolean; cause: string } {
  const file = fileName(agent);
  const plain = (title: string, body: string, reread: boolean) => ({ title, body, reread, cause: title });
  switch (f.kind) {
    case 'changed': {
      const cause = t(file + ' vừa bị chương trình khác ghi.', 'another program just wrote ' + file + '.');
      return {
        title: nothingChanged(cause),
        cause,
        body: (agent === 'codex' ? t('Có thể Codex App vừa cập nhật file. ', 'Codex App may have just updated the file. ') : '') + t('Đọc lại rồi thử lại.', 'Read again, then try again.'),
        reread: true,
      };
    }
    case 'refused': {
      const cause = t('danh sách đã khác lúc xem trước.', 'the list changed since the preview.');
      return {
        title: nothingChanged(cause),
        cause,
        body:
          f.refused.map((r) => t(r.name + ' cho ' + AGENT_NAME[r.agent] + ': ', r.name + ' for ' + AGENT_NAME[r.agent] + ': ') + refusalText(r.reason)).join(' · ') +
          t('. Đọc lại rồi xem trước lần nữa.', '. Read again, then preview again.'),
        reread: true,
      };
    }
    case 'edit':
      return plain(
        t('Chưa ghi ' + file + '.', file + ' was not written.'),
        t('App không sửa được đúng mục đó, nên không ghi gì vào file.', 'The app could not edit that exact entry, so nothing was written to the file.'),
        true,
      );
    case 'write':
      return plain(
        t('Chưa ghi ' + file + '.', file + ' was not written.'),
        f.error.kind === 'io'
          ? t('Lỗi khi ghi: ' + f.error.detail + '. File đang dùng không đổi.', 'Write error: ' + f.error.detail + '. The file in use is unchanged.')
          : t('File đọc lại không đúng bản vừa ghi.', 'The file read back does not match what was written.'),
        true,
      );
    case 'cli':
      return plain(t('Lệnh claude chưa chạy xong.', 'The claude command did not finish.'), f.detail, true);
    case 'keychain':
      return plain(
        t('Chưa lưu được khoá vào ' + secretStore() + '.', 'Could not save the key to the ' + secretStore() + '.'),
        t(f.detail + '. Chưa đổi gì ở ' + AGENT_NAME[agent] + '.', f.detail + '. Nothing changed in ' + AGENT_NAME[agent] + '.'),
        false,
      );
    case 'verify':
      return plain(
        t('Danh sách của ' + AGENT_NAME[agent] + ' chưa đúng như xem trước.', AGENT_NAME[agent] + '’s list does not match the preview.'),
        t('Đọc lại để xem bây giờ thế nào.', 'Read again to see how it is now.'),
        true,
      );
  }
}

type LeadVerb = 'add' | 'on' | 'off' | 'remove' | 'edit';

function leadHead(name: string, verb: LeadVerb, agents: string): string {
  switch (verb) {
    case 'add':
      return t(name + ': thêm cho ' + agents, name + ': add to ' + agents);
    case 'on':
      return t(name + ': bật cho ' + agents, name + ': turn on for ' + agents);
    case 'off':
      return t(name + ': tắt cho ' + agents, name + ': turn off for ' + agents);
    case 'remove':
      return t(name + ': bỏ khỏi ' + agents, name + ': remove from ' + agents);
    case 'edit':
      return t(name + ': sửa ở ' + agents, name + ': edit in ' + agents);
  }
}

/** The lead line of the batch preview: each change in words, one server's like changes together. */
export function previewLead(p: McpPreview, rows: McpRow[]): string {
  const said: { name: string; verb: LeadVerb; agents: Agent[] }[] = [];
  for (const c of p.changes) {
    const row = rows.find((r) => r.name === c.name);
    const adds = row ? row.cells[c.agent].state === 'absent' : true;
    const verb: LeadVerb = c.to.kind === 'on' ? (adds ? 'add' : 'on') : c.to.kind === 'off' ? 'off' : c.to.kind === 'absent' ? 'remove' : adds ? 'add' : 'edit';
    const same = said.find((x) => x.name === c.name && x.verb === verb);
    if (same) same.agents.push(c.agent);
    else said.push({ name: c.name, verb, agents: [c.agent] });
  }
  return said.map((x) => leadHead(x.name, x.verb, AGENTS.filter((a) => x.agents.includes(a)).map((a) => AGENT_NAME[a]).join(', '))).join(' · ');
}

// ===== The add and edit forms =====

const SECRET_WORDS = ['key', 'token', 'secret', 'password', 'passwd', 'auth', 'credential', 'cookie', 'bearer', 'session', 'signature'];
const PLACE_ENDINGS = ['_path', '_paths', '_dir', '_file', '_home', '_url', '_name'];

/** A name whose value is a secret, as the Rust side decides (`def::secret_name`). */
export function isSecretName(name: string): boolean {
  const n = name.replace(/^-+/, '').toLowerCase().replace(/-/g, '_');
  return SECRET_WORDS.some((w) => n.includes(w)) && !PLACE_ENDINGS.some((e) => n.endsWith(e));
}

const TOKEN_STARTS = ['sk-', 'sk_', 'ghp_', 'gho_', 'ghu_', 'ghs_', 'github_pat_', 'glpat-', 'xoxb-', 'xoxp-', 'xapp-', 'AKIA', 'AIza', 'ya29.'];

/** A value shaped like a token whatever its name (`def::token_shaped`). */
export function isTokenShaped(value: string): boolean {
  const v = value.trim().replace(/^Bearer /, '');
  return TOKEN_STARTS.some((s) => v.startsWith(s) && v.length >= s.length + 8) || (v.startsWith('eyJ') && v.length >= 24 && v.split('.').length === 3);
}

/** `${TOKEN}`, `Bearer ${TOKEN}`: the agent fills it in, so it is no secret. */
export const onlyReferences = (value: string): boolean => /^[A-Za-z ]{0,10}(\$\{[^}]+\}|\$[A-Za-z0-9_]+)$/.test(value.trim());

/** The value under `name` goes to the Keychain (`def::secret_value`); an empty one by its name. */
export function isSecretPair(name: string, value: string): boolean {
  if (!value.trim()) return isSecretName(name);
  return !onlyReferences(value) && (isSecretName(name) || isTokenShaped(value));
}

/** A command line split as a shell would, quotes kept together. */
export function splitCommand(line: string): string[] {
  const out: string[] = [];
  let cur = '';
  let quote: '"' | "'" | null = null;
  let any = false;
  for (const ch of line) {
    if (quote) {
      if (ch === quote) quote = null;
      else cur += ch;
    } else if (ch === '"' || ch === "'") {
      quote = ch;
      any = true;
    } else if (/\s/.test(ch)) {
      if (cur || any) out.push(cur);
      cur = '';
      any = false;
    } else cur += ch;
  }
  if (cur || any) out.push(cur);
  return out;
}

/** The command line holds a secret, as `def::secret_args` finds one. */
export function lineHasSecret(line: string): boolean {
  const args = splitCommand(line).slice(1);
  return args.some((a, i) => {
    const eq = a.indexOf('=');
    if (a.startsWith('-')) return eq > 0 && isSecretPair(a.slice(0, eq), a.slice(eq + 1));
    const prev = args[i - 1];
    return isTokenShaped(a) || (!!prev && prev.startsWith('-') && !prev.includes('=') && isSecretName(prev) && !onlyReferences(a));
  });
}

/** The command and its arguments on one line, quoted where a space needs it. */
export function joinCommand(command: string, args: string[]): string {
  const q = (s: string) => (s === '' || /[\s"']/.test(s) ? (s.includes('"') ? "'" + s + "'" : '"' + s + '"') : s);
  return [command, ...args].map(q).join(' ');
}

export interface FormPair {
  name: string;
  /** What was typed; for a secret already kept, empty means "keep it". */
  value: string;
  secret: boolean;
  /** A secret already in a file or the Keychain: shown as the mask. */
  kept: boolean;
}

export interface McpForm {
  name: string;
  kind: 'stdio' | 'http';
  /** The command and its arguments, or the address. */
  line: string;
  pairs: FormPair[];
  /** The Bitwarden item the secrets are in. */
  bitwarden: string;
  agents: Record<Agent, boolean>;
  /** Editing a server already there: its name stays. */
  edit: boolean;
  /** Kept as the agent has them; the form does not show them. */
  cwd: string | null;
  sse: boolean;
}

export function emptyForm(columns: McpTable['columns']): McpForm {
  const agents = Object.fromEntries(AGENTS.map((a) => [a, !!columns.find((c) => c.agent === a)?.writable])) as Record<Agent, boolean>;
  return { name: '', kind: 'stdio', line: '', pairs: [], bitwarden: '', agents, edit: false, cwd: null, sse: false };
}

/** The form for editing `def`, a server's definition as an agent has it. */
export function formOf(name: string, def: McpDef, slots: string[], agents: Record<Agent, boolean>): McpForm {
  const slot = (place: string, n: string) => slots.includes(place + ':' + n);
  if (def.kind === 'stdio')
    return {
      name,
      kind: 'stdio',
      line: joinCommand(def.command, def.args),
      pairs: def.env.map((p) => ({ name: p.name, value: slot('env', p.name) ? '' : p.value, secret: slot('env', p.name), kept: slot('env', p.name) })),
      bitwarden: '',
      agents,
      edit: true,
      cwd: def.cwd,
      sse: false,
    };
  return {
    name,
    kind: 'http',
    line: def.url,
    pairs: def.headers.map((p) => ({ name: p.name, value: slot('header', p.name) ? '' : p.value, secret: slot('header', p.name), kept: slot('header', p.name) })),
    bitwarden: '',
    agents,
    edit: true,
    cwd: null,
    sse: def.sse,
  };
}

export function formError(f: McpForm, taken: string[]): string | null {
  if (!/^[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}$/.test(f.name))
    return t('Tên chỉ có chữ, số, dấu - _ . và không bắt đầu bằng - hay .', 'Names may only have letters, digits, - _ . and cannot start with - or .');
  if (!f.edit && taken.includes(f.name)) return t('Đã có MCP tên này.', 'There is already an MCP with this name.');
  if (f.kind === 'stdio' && !splitCommand(f.line).length) return t('Điền lệnh để chạy.', 'Enter the command to run.');
  if (f.kind === 'http' && !/^https?:\/\/\S+$/.test(f.line.trim())) return t('Địa chỉ bắt đầu bằng http:// hoặc https://.', 'The address starts with http:// or https://.');
  if (f.pairs.some((p) => !p.name.trim())) return t('Mỗi dòng cần có tên.', 'Each line needs a name.');
  if (f.pairs.some((p) => p.secret && !p.kept && !p.value)) return t('Điền giá trị khoá, hoặc bỏ dòng đó.', 'Enter the key’s value, or remove that line.');
  if (!AGENTS.some((a) => f.agents[a])) return t('Chọn ít nhất một agent.', 'Pick at least one agent.');
  return null;
}

/** A pair's slot id, as its Keychain item is named. */
export const pairSlot = (f: McpForm, p: FormPair): string => (f.kind === 'stdio' ? 'env:' : 'header:') + p.name.trim();

/** The definition the form sends: a secret is the mask, its value already in the Keychain. */
export function formDef(f: McpForm): McpDef {
  const pairs = f.pairs.map((p) => ({ name: p.name.trim(), value: p.secret ? MASK : p.value }));
  if (f.kind === 'stdio') {
    const [command, ...args] = splitCommand(f.line);
    return { kind: 'stdio', command, args, env: pairs, cwd: f.cwd };
  }
  return { kind: 'http', url: f.line.trim(), headers: pairs, sse: f.sse };
}

/** The slots whose value the form typed: taken from the Keychain first. */
export const typedSlots = (f: McpForm): string[] => f.pairs.filter((p) => p.secret && p.value).map((p) => pairSlot(f, p));
