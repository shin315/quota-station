// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Bộ cấu hình tab's words and the plain rules behind them (M15): a set's
// one-line summary, "Lấy từ cấu hình hiện tại", how far the applied set has
// drifted, the preview by agent, and the lines after a write and an undo.
// What a set changes is decided by the Rust side (`agents::sets`); this
// only says it.

import { AGENT_NAME, AGENTS, changesWord, failureText } from './mcp';
import { skillFailureText, skillLockText } from './skills';
import { clock } from './format';
import { t } from './i18n';
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
  return target === 'chatGpt' ? t('tài khoản ChatGPT', 'the ChatGPT account') : (codex.custom?.name ?? t('nhà cung cấp riêng', 'the custom provider'));
}

export function codexWords(set: ConfigSet, codex: CodexView): string {
  if (!set.codex) return t('Codex giữ nguyên', 'Codex unchanged');
  return set.codex === 'chatGpt'
    ? t('Codex về tài khoản ChatGPT', 'Codex to the ChatGPT account')
    : t('Codex sang ' + sideName('custom', codex), 'Codex to ' + sideName('custom', codex));
}

/** "Bật 3 MCP, tắt 1 · bật 2 skill · Codex giữ nguyên". */
export function setSummary(set: ConfigSet, codex: CodexView): string {
  const part = (entries: Record<string, SetWant>, what: string) => {
    const on = count(entries, 'on');
    const off = count(entries, 'off');
    // English: "2 skills on, 1 off".
    const en = (n: number) => n + ' ' + what + (what === 'skill' && n !== 1 ? 's' : '');
    if (!on && !off) return '';
    if (!off) return t('bật ' + on + ' ' + what, en(on) + ' on');
    if (!on) return t('tắt ' + off + ' ' + what, en(off) + ' off');
    return t('bật ' + on + ' ' + what + ', tắt ' + off, en(on) + ' on, ' + off + ' off');
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
  const out = drift.map((d) => {
    const where = d.agents.map((a) => AGENT_NAME[a]).join(', ');
    return d.want === 'on' ? t(d.name + ' đang tắt ở ' + where, d.name + ' is off in ' + where) : t(d.name + ' đang bật ở ' + where, d.name + ' is on in ' + where);
  });
  if (set && codexStep(set, codex)) out.push(t('Codex không ở ' + sideName(set.codex!, codex), 'Codex is not on ' + sideName(set.codex!, codex)));
  return out;
}

export function driftChip(n: number): { t: string; k: 'ok' | 'warn' } {
  return n ? { t: t('đã lệch ' + n + ' mục', n === 1 ? '1 entry differs' : n + ' entries differ'), k: 'warn' } : { t: t('đúng như bộ', 'matches the set'), k: 'ok' };
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
    mcp: p.mcp.changes.filter((c) => c.agent === agent).map((c) => (c.to.kind === 'on' ? t('bật ', 'turn on ') : t('tắt ', 'turn off ')) + c.name),
    skills: p.skills.changes.filter((c) => c.agent === agent).map((c) => (c.on ? t('bật ', 'turn on ') : t('tắt ', 'turn off ')) + c.name),
  })).filter((r) => r.mcp.length || r.skills.length);
}

export function changeCount(p: SetPreview): number {
  return p.mcp.changes.length + p.skills.changes.length;
}

export function sentOf(p: SetPreview): SetSent {
  return { mcp: p.mcp.changes, mcpBases: p.mcp.bases, skills: p.skills.changes, skillBases: p.skills.bases };
}

function reason(s: SetSkipped): string {
  if (s.reason === 'notHere') return t('máy này không có', 'not on this Mac');
  if (s.reason === 'owned') return t('có sẵn của agent, app để yên', 'built into the agent, left alone');
  if (s.kind === 'skill' && ['readOnly', 'hand', 'taken', 'broken'].includes(s.reason)) return skillLockText(s.reason as 'readOnly');
  switch (s.reason) {
    case 'readOnly':
      return t('file của agent này chỉ xem', 'this agent’s file is view only');
    case 'missingKey':
      return t('thiếu khoá', 'key missing');
    case 'noCommand':
      return t('lệnh chưa cài', 'command not installed');
    case 'sse':
      return t('Codex không chạy MCP kiểu SSE', 'Codex does not run SSE MCP servers');
    default:
      return t('mục viết kiểu app không sửa', 'written in a form the app does not edit');
  }
}

/** "Bỏ qua tracker: máy này không có." */
export function skippedLine(s: SetSkipped): string {
  if (s.agent) return t('Bỏ qua ' + s.name + ' cho ' + AGENT_NAME[s.agent] + ': ' + reason(s) + '.', 'Skipping ' + s.name + ' for ' + AGENT_NAME[s.agent] + ': ' + reason(s) + '.');
  return t('Bỏ qua ' + s.name + ': ' + reason(s) + '.', 'Skipping ' + s.name + ': ' + reason(s) + '.');
}

/** The preview's refusals besides "already so": the files changed since the table was read. */
export function refusedLines(p: SetPreview): string[] {
  const line = (name: string, agent: Agent) => t('Bỏ qua ' + name + ' cho ' + AGENT_NAME[agent] + '.', 'Skipping ' + name + ' for ' + AGENT_NAME[agent] + '.');
  return [
    ...p.mcp.refused.filter((r) => r.reason !== 'nothing').map((r) => line(r.name, r.agent)),
    ...p.skills.refused.filter((r) => r.reason !== 'nothing').map((r) => line(r.name, r.agent)),
  ];
}

/** Who keeps the old configuration: the sessions running now. */
export function sessionsLine(claude: number | null, codex: CodexView): string {
  const parts: string[] = [];
  if (claude) parts.push(t(claude + ' phiên Claude Code', claude + (claude === 1 ? ' Claude Code session' : ' Claude Code sessions')));
  const n = codex.procs?.sessions ?? 0;
  if (n) parts.push(t(n + ' phiên Codex', n + (n === 1 ? ' Codex session' : ' Codex sessions')));
  if (codex.procs?.app) parts.push('Codex App');
  if (!parts.length) return t('Phiên đang chạy giữ cấu hình cũ đến khi mở lại.', 'Running sessions keep the old setup until they are reopened.');
  const who = parts.length === 1 ? parts[0] : parts.slice(0, -1).join(', ') + t(' và ', ' and ') + parts[parts.length - 1];
  return t(who + ' đang chạy giữ cấu hình cũ đến khi mở lại.', 'Keeping the old setup until reopened: ' + who + ', running now.');
}

function written(o: SetOutcome): number {
  return [...(o.mcp?.done ?? []), ...(o.skills?.done ?? [])].reduce((t, d) => t + d.changes, 0);
}

/** The line after "Áp bộ"; with `step`, Codex's switch opens next. */
export function appliedFeedback(name: string, o: SetOutcome, step: CodexTarget | null, codex: CodexView): FeedbackView {
  const n = written(o);
  const undo = { label: t('Hoàn tác bộ', 'Undo the set'), act: 'sets:undo' as const };
  const failed = o.mcp?.failed
    ? failureText(o.mcp.failed.error, o.mcp.failed.agent)
    : o.skills?.failed
      ? skillFailureText(o.skills.failed.error, o.skills.failed.agent)
      : null;
  if (failed && !n) return { tone: 'bad', title: failed.title, body: failed.body };
  if (failed)
    return {
      tone: 'warn',
      title: t('Đã áp một phần bộ ' + name + ': ' + changesWord(n) + '.', 'Applied part of the set ' + name + ': ' + changesWord(n) + '.'),
      body: failed.title + ' ' + failed.body,
      actions: [undo],
    };
  const at = clock(new Date(o.at * 1000).toISOString());
  const cx = step
    ? t(' Bước 2: chuyển Codex về ' + sideName(step, codex) + ' ở tấm chuyển Codex.', ' Step 2: switch Codex to ' + sideName(step, codex) + ' in the Codex switch sheet.')
    : '';
  return {
    tone: 'info',
    title: n
      ? t('Đã áp bộ ' + name + ': ' + changesWord(n) + '.', 'Applied the set ' + name + ': ' + changesWord(n) + '.')
      : t('Bộ ' + name + ' đang dùng.', 'The set ' + name + ' is in use.'),
    body:
      (n
        ? t('Đã sao lưu file của agent lúc ' + at + '. Phiên mở sau sẽ dùng cấu hình mới.', 'Backed up the agents’ files at ' + at + '. Sessions opened from now on use the new setup.')
        : t('Máy đã đúng như bộ, không có gì để ghi.', 'This Mac already matches the set; nothing to write.')) + cx,
    actions: [
      ...(n ? [undo] : []),
      ...(step ? [{ label: t('Chuyển Codex', 'Switch Codex'), act: step === 'chatGpt' ? ('codex:switch:chatGpt' as const) : ('codex:switch:custom' as const) }] : []),
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
  if (failed)
    return {
      tone: 'bad',
      title: t('Chưa hoàn tác hết bộ ' + name, 'The set ' + name + ' was not fully undone'),
      body: failed.title + ' ' + failed.body,
      actions: [{ label: t('Thử lại', 'Try again'), act: 'sets:undo' }],
    };
  const left = u.left.length ? t(' Giữ ' + u.left.join(', ') + ': đã đổi sau khi áp.', ' Kept ' + u.left.join(', ') + ': changed after the set was applied.') : '';
  const back = u.codexBefore === 'chatgpt' ? ('chatGpt' as const) : u.codexBefore === 'custom' ? ('custom' as const) : null;
  const switchBack = back && codex.current !== u.codexBefore;
  const now = sideName(codex.current === 'chatgpt' ? 'chatGpt' : 'custom', codex);
  return {
    tone: 'info',
    title: t('Đã hoàn tác bộ ' + name + '.', 'Undid the set ' + name + '.'),
    body:
      t('MCP và skills trở về như trước khi áp.', 'MCP and skills are back as they were before the set was applied.') +
      left +
      (switchBack ? t(' Codex vẫn ở ' + now + '.', ' Codex is still on ' + now + '.') : ''),
    actions: switchBack
      ? [{ label: t('Chuyển Codex về ' + sideName(back, codex), 'Switch Codex to ' + sideName(back, codex)), act: back === 'chatGpt' ? 'codex:switch:chatGpt' : 'codex:switch:custom' }]
      : undefined,
  };
}

/** Why "Lưu bộ" or "Áp bộ" did not run, from the Rust side's code. */
export function setErrorText(e: unknown): string {
  const s = String(e);
  if (s === 'name') return t('Đặt tên cho bộ (tối đa 60 ký tự).', 'Give the set a name (up to 60 characters).');
  if (s === 'taken') return t('Đã có bộ cùng tên.', 'There is already a set with this name.');
  if (s === 'full') return t('Đã có 50 bộ. Xoá bớt rồi tạo bộ mới.', 'There are already 50 sets. Delete some, then create a new one.');
  if (s.startsWith('badEntry:')) {
    const entry = s.slice('badEntry:'.length);
    return t('Tên ' + entry + ' không hợp lệ.', 'The name ' + entry + ' is not valid.');
  }
  if (s === 'busy') return t('Một lần ghi khác hoặc lần chuyển Codex đang chạy. Đợi xong rồi thử lại.', 'Another write or Codex switch is running. Wait for it to finish, then try again.');
  if (s === 'missing') return t('Bộ này không còn.', 'This set no longer exists.');
  if (s.startsWith('changed:')) {
    const a = s.slice('changed:'.length) as Agent;
    return AGENT_NAME[a]
      ? t(
          'Chưa đổi gì: file của ' + AGENT_NAME[a] + ' vừa đổi sau lúc xem trước. Bấm Áp… lại để xem trước lần nữa.',
          'Nothing changed: ' + AGENT_NAME[a] + '’s file changed after the preview. Press Apply… again to preview again.',
        )
      : t(
          'Chưa đổi gì: file của một agent vừa đổi sau lúc xem trước. Bấm Áp… lại để xem trước lần nữa.',
          'Nothing changed: an agent’s file changed after the preview. Press Apply… again to preview again.',
        );
  }
  if (s === 'notTheSet')
    return t('Chưa đổi gì: máy đã khác lúc xem trước. Bấm Áp… lại để xem trước lần nữa.', 'Nothing changed: this Mac changed after the preview. Press Apply… again to preview again.');
  return t('App không ghi được lúc này. Các file không đổi.', 'The app cannot write right now. The files are unchanged.');
}
