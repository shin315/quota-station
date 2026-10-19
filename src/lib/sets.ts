// The Bộ cấu hình tab's words and the plain rules behind them (M15): a set's
// one-line summary, "Lấy từ cấu hình hiện tại", how far the applied set has
// drifted, the preview by agent, and the lines after a write and an undo.
// What a set changes is decided by the Rust side (`agents::sets`); this
// only says it.

import { AGENT_NAME, AGENTS, failureText } from './mcp';
import { skillFailureText, skillLockText } from './skills';
import { clock } from './format';
import type {
  Agent,
  CodexTarget,
  CodexView,
  ConfigSet,
  McpTable,
  SetDrifted,
  SetOutcome,
  SetPreview,
  SetSkipped,
  SetSent,
  SetUndone,
  SetWant,
  SetsView,
  SkillTable,
} from './model';
import type { FeedbackView } from './view';

/** The Codex tab's name for each side of the switch. */
const SIDE: Record<CodexTarget, string> = { chatGpt: 'chatgpt', custom: 'custom' };

export function emptySet(): ConfigSet {
  return { id: '', name: '', mcp: {}, skills: {}, codex: null };
}

function count(entries: Record<string, SetWant>, want: SetWant): number {
  return Object.values(entries).filter((w) => w === want).length;
}

/** "Tài khoản ChatGPT", or the custom provider's name. */
export function sideName(target: CodexTarget, codex: CodexView): string {
  return target === 'chatGpt' ? 'tài khoản ChatGPT' : (codex.custom?.name ?? 'nhà cung cấp riêng');
}

export function codexWords(set: ConfigSet, codex: CodexView): string {
  if (!set.codex) return 'Codex giữ nguyên';
  return set.codex === 'chatGpt' ? 'Codex về tài khoản ChatGPT' : 'Codex sang ' + sideName('custom', codex);
}

/** "Bật 3 MCP, tắt 1 · bật 2 skill · Codex giữ nguyên". */
export function setSummary(set: ConfigSet, codex: CodexView): string {
  const part = (entries: Record<string, SetWant>, what: string) => {
    const on = count(entries, 'on');
    const off = count(entries, 'off');
    if (!on && !off) return '';
    if (!off) return 'bật ' + on + ' ' + what;
    if (!on) return 'tắt ' + off + ' ' + what;
    return 'bật ' + on + ' ' + what + ', tắt ' + off;
  };
  const parts = [part(set.mcp, 'MCP'), part(set.skills, 'skill'), codexWords(set, codex)].filter(Boolean);
  const text = parts.join(' · ');
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** Whether every agent that has the entry has it on, off, or some of each. */
function agreed(states: ('on' | 'off' | 'absent')[]): SetWant | null {
  const here = states.filter((s) => s !== 'absent');
  if (!here.length) return null;
  if (here.every((s) => s === 'on')) return 'on';
  if (here.every((s) => s === 'off')) return 'off';
  return null;
}

/**
 * "Lấy từ cấu hình hiện tại": an entry on for every agent that has it is
 * Bật, off for all of them Tắt, and one on for some agents and off for
 * others Không đổi. Applied straight away, such a set changes nothing.
 */
export function fromCurrent(mcp: McpTable | null, skills: SkillTable | null): Pick<ConfigSet, 'mcp' | 'skills'> {
  const out: Pick<ConfigSet, 'mcp' | 'skills'> = { mcp: {}, skills: {} };
  for (const r of mcp?.rows ?? []) {
    const w = agreed(AGENTS.map((a) => r.cells[a].state));
    if (w) out.mcp[r.name] = w;
  }
  for (const r of skills?.rows ?? []) {
    const w = agreed(AGENTS.map((a) => r.cells[a].state));
    if (w) out.skills[r.name] = w;
  }
  return out;
}

/** The side the set wants Codex on, when Codex is not on it: step 2 opens the Codex switch for it. */
export function codexStep(set: { codex: CodexTarget | null }, codex: CodexView): CodexTarget | null {
  if (!set.codex || !codex.present) return null;
  return codex.current === SIDE[set.codex] ? null : set.codex;
}

/** The applied set's drift, Codex's side included: one line per entry. */
export function driftLines(drift: SetDrifted[], set: ConfigSet | null, codex: CodexView): string[] {
  const out = drift.map(
    (d) => d.name + ' đang ' + (d.want === 'on' ? 'tắt' : 'bật') + ' ở ' + d.agents.map((a) => AGENT_NAME[a]).join(', '),
  );
  if (set && codexStep(set, codex)) out.push('Codex không ở ' + sideName(set.codex!, codex));
  return out;
}

export function driftChip(n: number): { t: string; k: 'ok' | 'warn' } {
  return n ? { t: 'đã lệch ' + n + ' mục', k: 'warn' } : { t: 'đúng như bộ', k: 'ok' };
}

/** The set applied, and its drift; null when none is, or the tables are not read yet. */
export function appliedState(v: SetsView, codex: CodexView): { set: ConfigSet | null; lines: string[] | null } | null {
  const a = v.applied;
  if (!a) return null;
  const set = v.sets.find((s) => s.id === a.id) ?? null;
  return { set, lines: a.drift ? driftLines(a.drift, set, codex) : null };
}

/** What the preview changes, by agent: MCP and skills, in words. */
export function byAgent(p: SetPreview): { agent: Agent; mcp: string[]; skills: string[] }[] {
  return AGENTS.map((agent) => ({
    agent,
    mcp: p.mcp.changes.filter((c) => c.agent === agent).map((c) => (c.to.kind === 'on' ? 'bật ' : 'tắt ') + c.name),
    skills: p.skills.changes.filter((c) => c.agent === agent).map((c) => (c.on ? 'bật ' : 'tắt ') + c.name),
  })).filter((r) => r.mcp.length || r.skills.length);
}

export function changeCount(p: SetPreview): number {
  return p.mcp.changes.length + p.skills.changes.length;
}

export function sentOf(p: SetPreview): SetSent {
  return { mcp: p.mcp.changes, mcpBases: p.mcp.bases, skills: p.skills.changes, skillBases: p.skills.bases };
}

function reason(s: SetSkipped): string {
  if (s.reason === 'notHere') return 'máy này không có';
  if (s.reason === 'owned') return 'có sẵn của agent, app để yên';
  if (s.kind === 'skill' && ['readOnly', 'hand', 'taken', 'broken'].includes(s.reason)) return skillLockText(s.reason as 'readOnly');
  switch (s.reason) {
    case 'readOnly':
      return 'file của agent này chỉ xem';
    case 'missingKey':
      return 'thiếu khoá';
    case 'noCommand':
      return 'lệnh chưa cài';
    case 'sse':
      return 'Codex không chạy MCP kiểu SSE';
    default:
      return 'mục viết kiểu app không sửa';
  }
}

/** "Bỏ qua tracker: máy này không có." */
export function skippedLine(s: SetSkipped): string {
  return 'Bỏ qua ' + s.name + (s.agent ? ' cho ' + AGENT_NAME[s.agent] : '') + ': ' + reason(s) + '.';
}

/** The preview's refusals besides "already so": the files changed since the table was read. */
export function refusedLines(p: SetPreview): string[] {
  return [
    ...p.mcp.refused.filter((r) => r.reason !== 'nothing').map((r) => 'Bỏ qua ' + r.name + ' cho ' + AGENT_NAME[r.agent] + '.'),
    ...p.skills.refused.filter((r) => r.reason !== 'nothing').map((r) => 'Bỏ qua ' + r.name + ' cho ' + AGENT_NAME[r.agent] + '.'),
  ];
}

/** Who keeps the old configuration: the sessions running now. */
export function sessionsLine(claude: number | null, codex: CodexView): string {
  const parts: string[] = [];
  if (claude) parts.push(claude + ' phiên Claude Code');
  const n = codex.procs?.sessions ?? 0;
  if (n) parts.push(n + ' phiên Codex');
  if (codex.procs?.app) parts.push('Codex App');
  if (!parts.length) return 'Phiên đang chạy giữ cấu hình cũ đến khi mở lại.';
  const who = parts.length === 1 ? parts[0] : parts.slice(0, -1).join(', ') + ' và ' + parts[parts.length - 1];
  return who + ' đang chạy giữ cấu hình cũ đến khi mở lại.';
}

function written(o: SetOutcome): number {
  return [...(o.mcp?.done ?? []), ...(o.skills?.done ?? [])].reduce((t, d) => t + d.changes, 0);
}

/** The line after "Áp bộ"; with `step`, Codex's switch opens next. */
export function appliedFeedback(name: string, o: SetOutcome, step: CodexTarget | null, codex: CodexView): FeedbackView {
  const n = written(o);
  const undo = { label: 'Hoàn tác bộ', act: 'sets:undo' as const };
  const failed = o.mcp?.failed
    ? failureText(o.mcp.failed.error, o.mcp.failed.agent)
    : o.skills?.failed
      ? skillFailureText(o.skills.failed.error, o.skills.failed.agent)
      : null;
  if (failed && !n) return { tone: 'bad', title: failed.title, body: failed.body };
  if (failed)
    return { tone: 'warn', title: 'Đã áp một phần bộ ' + name + ': ' + n + ' thay đổi.', body: failed.title + ' ' + failed.body, actions: [undo] };
  const at = clock(new Date(o.at * 1000).toISOString());
  const cx = step ? ' Bước 2: chuyển Codex về ' + sideName(step, codex) + ' ở tấm chuyển Codex.' : '';
  return {
    tone: 'info',
    title: n ? 'Đã áp bộ ' + name + ': ' + n + ' thay đổi.' : 'Bộ ' + name + ' đang dùng.',
    body: (n ? 'Đã sao lưu file của agent lúc ' + at + '. Phiên mở sau sẽ dùng cấu hình mới.' : 'Máy đã đúng như bộ, không có gì để ghi.') + cx,
    actions: [
      ...(n ? [undo] : []),
      ...(step ? [{ label: 'Chuyển Codex', act: step === 'chatGpt' ? ('codex:switch:chatGpt' as const) : ('codex:switch:custom' as const) }] : []),
    ],
  };
}

/** The line after "Hoàn tác bộ". */
export function undoneFeedback(name: string, u: SetUndone, codex: CodexView): FeedbackView {
  const failed = u.mcp?.failed
    ? failureText(u.mcp.failed.error, u.mcp.failed.agent)
    : u.skills?.failed
      ? skillFailureText(u.skills.failed.error, u.skills.failed.agent)
      : null;
  if (failed) return { tone: 'bad', title: 'Chưa hoàn tác hết bộ ' + name, body: failed.title + ' ' + failed.body, actions: [{ label: 'Thử lại', act: 'sets:undo' }] };
  const left = u.left.length ? ' Giữ ' + u.left.join(', ') + ': đã đổi sau khi áp.' : '';
  const back = u.codexBefore === 'chatgpt' ? ('chatGpt' as const) : u.codexBefore === 'custom' ? ('custom' as const) : null;
  const switchBack = back && codex.current !== u.codexBefore;
  return {
    tone: 'info',
    title: 'Đã hoàn tác bộ ' + name + '.',
    body:
      'MCP và skills trở về như trước khi áp.' +
      left +
      (switchBack ? ' Codex vẫn ở ' + (codex.current === 'chatgpt' ? 'tài khoản ChatGPT' : sideName('custom', codex)) + '.' : ''),
    actions: switchBack ? [{ label: 'Chuyển Codex về ' + sideName(back, codex), act: back === 'chatGpt' ? 'codex:switch:chatGpt' : 'codex:switch:custom' }] : undefined,
  };
}

/** Why "Lưu bộ" or "Áp bộ" did not run, from the Rust side's code. */
export function setErrorText(e: unknown): string {
  const s = String(e);
  if (s === 'name') return 'Đặt tên cho bộ (tối đa 60 ký tự).';
  if (s === 'taken') return 'Đã có bộ cùng tên.';
  if (s === 'full') return 'Đã có 50 bộ. Xoá bớt rồi tạo bộ mới.';
  if (s.startsWith('badEntry:')) return 'Tên ' + s.slice('badEntry:'.length) + ' không hợp lệ.';
  if (s === 'busy') return 'Một lần ghi khác hoặc lần chuyển Codex đang chạy. Đợi xong rồi thử lại.';
  if (s === 'missing') return 'Bộ này không còn.';
  if (s.startsWith('changed:')) {
    const a = s.slice('changed:'.length) as Agent;
    return 'Chưa đổi gì: file của ' + (AGENT_NAME[a] ?? 'một agent') + ' vừa đổi sau lúc xem trước. Bấm Áp… lại để xem trước lần nữa.';
  }
  if (s === 'notTheSet') return 'Chưa đổi gì: máy đã khác lúc xem trước. Bấm Áp… lại để xem trước lần nữa.';
  return 'App không ghi được lúc này. Các file không đổi.';
}
