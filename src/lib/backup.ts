// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Sao lưu tab's logic (M10, M12), pure so Vitest covers it without a
// window: how a pack and its time read, what each line of the restore
// preview says (servers, then skills, plugins and instruction files in
// groups), the keys the ticked servers need, the result in plain words and
// what is left for the owner to do. Copy follows the approved phase 2
// prototype (round 2: no folder paths, no "Keychain", no commands).

import { clock } from './format';
import { AGENT_NAME, AGENTS } from './mcp';
import type {
  Agent,
  BackupView,
  PackInfo,
  RestoreDone,
  RestoreFileItem,
  RestoreItem,
  RestoreItemResult,
  RestoreMore,
  RestorePartResult,
  RestorePlan,
  RestorePluginItem,
  RestoreSkillItem,
  RestoreUndone,
} from './model';
import type { FeedbackView } from './view';

const pad = (n: number) => String(n).padStart(2, '0');

/** "Hôm nay 14:30", "Hôm qua 22:05", "17/10 21:48". */
export function packWhen(at: number | null, now: Date): string {
  if (at == null) return 'Không rõ giờ';
  const d = new Date(at * 1000);
  const iso = d.toISOString();
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (d.toDateString() === now.toDateString()) return 'Hôm nay ' + clock(iso);
  if (d.toDateString() === yesterday.toDateString()) return 'Hôm qua ' + clock(iso);
  return pad(d.getDate()) + '/' + pad(d.getMonth() + 1) + ' ' + clock(iso);
}

/** The same, inside a sentence: "gói hôm nay 14:30". */
export const packWhenLower = (at: number | null, now: Date): string => {
  const t = packWhen(at, now);
  return t.charAt(0).toLowerCase() + t.slice(1);
};

/** What a pack holds, in one line. */
export const packLine = (p: PackInfo): string => p.mcp + ' MCP';

export const macOf = (p: PackInfo, here: string): string => p.mac ?? (p.mine ? here : 'Mac khác');

/** A pack that can be opened and restored. */
export const usable = (p: PackInfo): boolean => p.state === 'ok' && !p.newer;

export function packNote(p: PackInfo): string | null {
  if (p.state === 'cloud') return 'chưa tải về máy';
  if (p.state === 'broken') return 'gói hỏng, không đọc được';
  if (p.newer) return 'của bản app mới hơn';
  return null;
}

export const keysLine = (n: number): string => (n ? 'Không chứa khoá. Khi khôi phục, app hỏi lại ' + n + ' khoá.' : 'Không chứa khoá. Gói này không cần dán khoá.');

function agentsText(agents: Agent[]): string {
  const sorted = AGENTS.filter((a) => agents.includes(a));
  if (sorted.length === 3) return 'cả ba agent';
  return sorted.map((a) => AGENT_NAME[a]).join(', ');
}

export const itemKey = (i: { name: string; kind: string }): string => i.name + '|' + i.kind;

/** The line under a server in the restore preview. */
export function itemLine(i: RestoreItem): string {
  if (i.kind === 'replace') return 'Máy này đã có, khác gói. Bỏ chọn là giữ bản trên máy.';
  if (i.missingCommand) return 'Máy này chưa cài ' + i.missingCommand + ': thêm nhưng để tắt';
  const parts = ['Thêm cho ' + agentsText(i.agents)];
  if (i.off.length === i.agents.length) parts.push('để tắt như trong gói');
  else if (i.off.length) parts.push('tắt ở ' + agentsText(i.off) + ' như trong gói');
  if (i.keys.length) parts.push('cần dán khoá');
  if (i.moved) parts.push('app tự chỉnh cho hợp máy này');
  if (i.login) parts.push('sau đó đăng nhập lại trong agent nếu được hỏi');
  return parts.join(' · ');
}

/** What the restore leaves out on this Mac, said once. */
export function skippedLines(plan: RestorePlan): string[] {
  const out: string[] = [];
  for (const a of AGENTS) {
    if (plan.skipped.some((s) => s.agent === a && s.reason === 'absent')) out.push('Máy này chưa cài ' + AGENT_NAME[a] + ': bỏ qua phần của ' + AGENT_NAME[a] + '.');
    if (plan.skipped.some((s) => s.agent === a && s.reason === 'readOnly')) out.push('Chưa ghi được vào ' + AGENT_NAME[a] + ' lúc này: bỏ qua phần của ' + AGENT_NAME[a] + '. Xem tab MCP.');
  }
  const sse = plan.skipped.filter((s) => s.reason === 'sse' && s.name).map((s) => s.name as string);
  if (sse.length) out.push('Codex không chạy MCP kiểu SSE: ' + sse.join(', ') + ' chỉ khôi phục cho agent khác.');
  return out;
}

export type Ticks = Record<string, boolean>;

/** The ticks of the groups after the servers: all skills from GitHub, all copied by hand, the plugins. */
export const SKILLS_GITHUB = 'sk:github';
export const SKILLS_HAND = 'sk:hand';
export const PLUGINS = 'pl:all';
/** A Codex plugin off in the pack, ticked on its own. */
export const pluginKey = (p: { agent: Agent; id: string }): string => 'pl:' + p.agent + ':' + p.id;
export const fileKey = (path: string): string => 'file:' + path;

/** The pack's skills by group: what can be restored now, and what cannot. */
export function skillGroups(plan: RestorePlan): { github: RestoreSkillItem[]; hand: RestoreSkillItem[]; blocked: RestoreSkillItem[] } {
  const items = plan.skills?.items ?? [];
  return {
    github: items.filter((i) => i.kind === 'github' && !i.blocked),
    hand: items.filter((i) => i.kind === 'hand' && !i.blocked),
    blocked: items.filter((i) => i.blocked),
  };
}

/** The plugins: those installed together, the Codex ones off in the pack, those with no source here. */
export function pluginGroups(plan: RestorePlan): { main: RestorePluginItem[]; off: RestorePluginItem[]; blocked: RestorePluginItem[] } {
  const items = plan.plugins?.items ?? [];
  return {
    main: items.filter((i) => !i.note),
    off: items.filter((i) => i.note === 'codexOff'),
    blocked: items.filter((i) => i.note === 'noSource'),
  };
}

export function initialTicks(plan: RestorePlan): Ticks {
  const t: Ticks = Object.fromEntries(plan.items.map((i) => [itemKey(i), i.checked]));
  const sk = skillGroups(plan);
  if (sk.github.length) t[SKILLS_GITHUB] = true;
  if (sk.hand.length) t[SKILLS_HAND] = true;
  const pl = pluginGroups(plan);
  if (pl.main.length) t[PLUGINS] = true;
  for (const p of pl.off) t[pluginKey(p)] = false;
  for (const f of plan.files?.items ?? []) t[fileKey(f.path)] = f.checked;
  return t;
}

/** What the ticks ask of the skills, plugins and instruction files. */
export function moreFor(plan: RestorePlan, ticks: Ticks): RestoreMore {
  const sk = skillGroups(plan);
  const pl = pluginGroups(plan);
  return {
    skills: [...(ticks[SKILLS_GITHUB] ? sk.github : []), ...(ticks[SKILLS_HAND] ? sk.hand : [])].map((i) => i.name),
    plugins: [...(ticks[PLUGINS] ? pl.main : []), ...pl.off.filter((p) => ticks[pluginKey(p)])].map((p) => ({ agent: p.agent, id: p.id })),
    files: (plan.files?.items ?? []).filter((f) => ticks[fileKey(f.path)]).map((f) => f.path),
    fileBases: plan.files?.bases ?? {},
  };
}

/** Something is ticked: a server, a group or a file. */
export function anyTicked(plan: RestorePlan, ticks: Ticks): boolean {
  const m = moreFor(plan, ticks);
  return picked(plan, ticks).length > 0 || m.skills.length > 0 || m.plugins.length > 0 || m.files.length > 0;
}

/** A few names, then how many more. */
export function names(list: string[], max = 4): string {
  if (list.length <= max) return list.join(', ');
  return list.slice(0, max - 1).join(', ') + ' và ' + (list.length - max + 1) + ' khác';
}

const fileName = (path: string): string => path.split('/').pop() ?? path;

/** "CLAUDE.md của Claude Code", "quy tắc style.md của Claude Code", "team.md". */
export function fileLabel(path: string, agent: Agent | null): string {
  const name = fileName(path);
  if (!agent) return name;
  const rule = agent === 'claude' && path.includes('/rules/');
  return (rule ? 'quy tắc ' : '') + name + ' của ' + AGENT_NAME[agent];
}

export const SKILL_TITLE = { github: (n: number) => n + ' skill từ GitHub', hand: (n: number) => n + ' skill cài tay' };
export const skillLine = (kind: 'github' | 'hand', items: RestoreSkillItem[]): string =>
  (kind === 'github' ? 'Cài lại từ GitHub, bản mới nhất' : 'Chép từ gói') + ' · ' + names(items.map((i) => i.name));

/** Who installs the plugins: "Cài bằng chính Claude Code và Codex". */
export function pluginLine(items: RestorePluginItem[]): string {
  const agents = AGENTS.filter((a) => items.some((i) => i.agent === a)).map((a) => AGENT_NAME[a]);
  return 'Cài bằng chính ' + agents.join(' và ') + ' · ' + names(items.map((i) => i.id.split('@')[0]));
}

export function fileLine(f: RestoreFileItem): string {
  return f.kind === 'add' ? 'Máy này chưa có' : 'Khác ' + f.changed + ' dòng. Bỏ chọn là giữ bản trên máy.';
}

const LEFT_OUT: Record<string, string> = {
  key: 'có dòng giống khoá nên không có trong gói',
  link: 'có liên kết tới chỗ khác nên không chép vào gói',
  large: 'lớn quá nên không chép vào gói',
  unreadable: 'không đọc được lúc sao lưu',
  old: 'gói này cũ, chưa chép skill cài tay',
};

/** What the skills, plugins and files of the preview leave out, said once each. */
export function partNotes(plan: RestorePlan): string[] {
  const out: string[] = [];
  const sk = skillGroups(plan);
  const noNpx = sk.blocked.filter((i) => i.blocked === 'noNpx').map((i) => i.name);
  if (noNpx.length) out.push('Máy này chưa có Node: chưa cài lại được ' + names(noNpx) + '. Cài Node rồi khôi phục lại.');
  const cloud = sk.blocked.filter((i) => i.blocked === 'cloud').map((i) => i.name);
  if (cloud.length) out.push('iCloud Drive chưa tải về ' + names(cloud) + '. Mở thư mục gói trong Finder để tải.');
  for (const [why, text] of Object.entries(LEFT_OUT)) {
    const list = (plan.skills?.leftOut ?? []).filter((l) => l.why === why).map((l) => l.name);
    if (list.length) out.push('Skill ' + names(list) + ' ' + text + '.');
  }
  for (const s of plan.plugins?.skipped ?? []) {
    out.push(s.reason === 'noCli' ? 'Máy này chưa cài ' + AGENT_NAME[s.agent] + ': bỏ qua plugin của ' + AGENT_NAME[s.agent] + '.' : 'Chưa đọc được plugin của ' + AGENT_NAME[s.agent] + ' lúc này: bỏ qua.');
  }
  const noSource = pluginGroups(plan).blocked.map((p) => p.id);
  if (noSource.length) out.push('Plugin ' + names(noSource) + ' lấy từ thư mục trên Mac kia: cài tay nếu cần.');
  for (const k of plan.files?.kept ?? []) {
    const label = fileLabel(k.path, k.agent);
    if (k.why === 'key') out.push(label + ' trên máy có dòng giống khoá nên app giữ nguyên.');
    else if (k.why === 'absent' && k.agent) out.push('Máy này chưa cài ' + AGENT_NAME[k.agent] + ': bỏ qua ' + label + '.');
    else out.push('Bỏ qua ' + label + '.');
  }
  return out;
}

export const picked = (plan: RestorePlan, ticks: Ticks): RestoreItem[] => plan.items.filter((i) => ticks[itemKey(i)]);

export const keySlot = (k: { mcp: string; slot: string }): string => k.mcp + '|' + k.slot;

/** The keys the ticked servers need, in the plan's order. */
export function keysFor(plan: RestorePlan, ticks: Ticks): RestorePlan['keys'] {
  const want = new Set(picked(plan, ticks).flatMap((i) => i.keys.map((s) => i.name + '|' + s)));
  return plan.keys.filter((k) => want.has(keySlot(k)));
}

/** One line of the result, with its tone: `''` done, `w` added off, `x` not written. */
export function resultLine(r: RestoreItemResult): [string, string, string] {
  const n = r.done.length + r.kept.length;
  if (r.kind === 'replace') {
    if (r.done.length) return ['', 'MCP ' + r.name + ' · thay bằng bản trong gói', ''];
    if (r.skipped) return ['w', 'MCP ' + r.name + ' · giữ bản trên máy', r.skipped === 'noKey' ? 'chưa có khoá' : 'máy chưa cài lệnh của nó'];
    return ['x', 'MCP ' + r.name, 'chưa ghi được'];
  }
  if (r.failed.length && !n) return ['x', 'MCP ' + r.name, 'chưa ghi được'];
  if (r.kept.length && !r.done.length)
    return ['w', 'MCP ' + r.name + ' · thêm nhưng tắt', r.why === 'noCommand' ? 'máy chưa cài ' + (r.command ?? 'lệnh của nó') : r.why === 'noKey' ? 'chưa có khoá' : 'tắt như trong gói'];
  const sub = [
    r.kept.length ? 'tắt ở ' + agentsText(r.kept) + (r.why === 'noKey' ? ', chưa có khoá' : r.why === 'noCommand' ? ', chưa có lệnh' : '') : '',
    r.failed.length ? 'chưa ghi được ở ' + agentsText(r.failed) : '',
    r.login ? 'có thể cần đăng nhập lại trong agent' : '',
  ].filter(Boolean);
  return [r.failed.length ? 'w' : '', 'MCP ' + r.name + ' · ' + n + ' agent', sub.join(' · ')];
}

const WHY: Record<string, string> = {
  noNpx: 'máy chưa có Node',
  npx: 'chưa cài được từ GitHub',
  cloud: 'iCloud Drive chưa tải về',
  damaged: 'bản trong gói bị hỏng',
  taken: 'máy đã có skill cùng tên',
  links: 'chưa bật được cho agent',
  noCli: 'máy chưa cài agent này',
  login: 'cần đăng nhập',
  command: 'lệnh cài của agent chưa chạy được',
  changed: 'file vừa đổi trên máy',
  io: 'chưa ghi được',
};

/** A part's name in the result: "Skill pdf-tools", "Plugin formatter", "CLAUDE.md của Claude Code". */
export function partName(r: RestorePartResult): string {
  if (r.part === 'skill') return 'Skill ' + r.id;
  if (r.part === 'plugin') return 'Plugin ' + r.id.split('@')[0];
  return fileLabel(r.id, r.agent);
}

/** One line of the result for the skills, plugins and files: those done together, each failure on its own. */
export interface PartLine {
  tone: '' | 'x';
  title: string;
  sub: string;
  /** The failed part "Thử lại" runs again. */
  retry: RestorePartResult | null;
}

export function partLines(parts: RestorePartResult[]): PartLine[] {
  const out: PartLine[] = [];
  const ok = (part: string, kind?: string) => parts.filter((r) => r.ok && r.part === part && (!kind || r.kind === kind));
  const gh = ok('skill', 'github');
  if (gh.length) out.push({ tone: '', title: SKILL_TITLE.github(gh.length), sub: names(gh.map((r) => r.id)), retry: null });
  const hand = ok('skill', 'hand');
  if (hand.length) out.push({ tone: '', title: SKILL_TITLE.hand(hand.length), sub: names(hand.map((r) => r.id)), retry: null });
  const pl = ok('plugin');
  if (pl.length) out.push({ tone: '', title: pl.length + ' plugin', sub: names(pl.map((r) => r.id.split('@')[0])), retry: null });
  for (const f of ok('file')) out.push({ tone: '', title: partName(f) + (f.kind === 'replace' ? ' · thay bằng bản trong gói' : ''), sub: '', retry: null });
  for (const r of parts.filter((x) => !x.ok)) out.push({ tone: 'x', title: partName(r), sub: WHY[r.why ?? 'io'] ?? WHY.io, retry: r.retry ? r : null });
  return out;
}

/** "Còn lại bạn làm". */
export function todoLines(done: RestoreDone): string[] {
  const out: string[] = [];
  for (const r of done.items) {
    if (r.why === 'noKey' || r.skipped === 'noKey') out.push('Dán khoá cho ' + r.name + ' ở tab MCP, rồi bật ' + r.name + '.');
    else if (r.why === 'noCommand' || r.skipped === 'noCommand') out.push('Cài ' + (r.command ?? 'lệnh của ' + r.name) + ' rồi bật ' + r.name + ' ở tab MCP.');
    if (r.login && r.done.length) out.push('Nếu agent hỏi, đăng nhập lại ' + r.name + ' trong ' + agentsText(r.done) + '.');
  }
  if (done.failed) out.push('Một phần chưa ghi được. Mở tab MCP, bấm Đọc lại rồi khôi phục lại phần còn thiếu.');
  const parts = done.parts ?? [];
  for (const r of parts.filter((x) => !x.ok)) {
    if (r.why === 'login') out.push('Đăng nhập trong ' + (r.agent ? AGENT_NAME[r.agent] : 'agent') + ' rồi bấm Thử lại ở ' + partName(r).toLowerCase() + '.');
    else if (r.why === 'noNpx') out.push('Cài Node rồi khôi phục lại ' + partName(r).toLowerCase() + '.');
    else if (r.why === 'cloud') out.push('Mở thư mục gói trong Finder để iCloud Drive tải về, rồi bấm Thử lại.');
    else if (r.why === 'changed') out.push(partName(r) + ' vừa đổi trên máy: mở Khôi phục lần nữa để xem trước lại.');
  }
  const used = [
    done.written.length || done.items.some((r) => r.done.length) ? 'MCP' : '',
    parts.some((r) => r.ok && r.part === 'skill') ? 'skill' : '',
    parts.some((r) => r.ok && r.part === 'plugin') ? 'plugin' : '',
  ].filter(Boolean);
  if (used.length) out.push('Mở lại agent để dùng ' + used.join(', ') + ' vừa khôi phục.');
  return [...new Set(out)];
}

const agentList = (code: string): string =>
  code
    .split(',')
    .filter(Boolean)
    .map((a) => (a === 'claude' ? 'Claude Code' : a === 'codex' ? 'Codex' : a === 'agy' ? 'Antigravity' : a))
    .join(', ');

/** Why a step of the restore did not run, in words. */
export function restoreError(e: unknown): FeedbackView {
  const s = String(e);
  if (s === 'busy') return { tone: 'warn', title: 'Chưa khôi phục', body: 'Một lần ghi khác hoặc lần chuyển Codex đang chạy. Đợi xong rồi thử lại.' };
  if (s.startsWith('changed:')) return { tone: 'warn', title: 'Chưa khôi phục: file của ' + agentList(s.slice(8)) + ' vừa đổi', body: 'Chưa ghi gì. Quay lại bước xem trước để đọc lại.' };
  if (s === 'keychain') return { tone: 'bad', title: 'Chưa lưu được khoá', body: 'Keychain không nhận khoá lúc này. Chưa ghi gì.' };
  if (s === 'cloud') return { tone: 'warn', title: 'Gói chưa tải về máy', body: 'iCloud Drive mới có tên file. Mở thư mục trong Finder để tải, hoặc chờ.' };
  if (s === 'newer') return { tone: 'warn', title: 'Gói của bản app mới hơn', body: 'Cập nhật Quota Station trên máy này rồi thử lại.' };
  if (s === 'missing') return { tone: 'warn', title: 'Không còn gói này', body: 'Gói vừa bị xoá hoặc đổi tên. Bấm Kiểm lại.' };
  if (s.startsWith('invalid')) return { tone: 'bad', title: 'Gói hỏng, không đọc được', body: 'Chọn một gói khác.' };
  return { tone: 'bad', title: 'Chưa khôi phục', body: 'App không chạy được bước này. Các file không đổi.' };
}

export function backupNowError(e: unknown): FeedbackView {
  const s = String(e);
  if (s === 'busy') return { tone: 'warn', title: 'Đang sao lưu', body: 'Một lần sao lưu đang chạy. Đợi xong rồi thử lại.' };
  if (s === 'noICloud') return { tone: 'bad', title: 'Chưa sao lưu: iCloud Drive đang tắt', body: 'Bật iCloud Drive trong Cài đặt hệ thống › Apple ID › iCloud rồi thử lại.' };
  if (s === 'secret') return { tone: 'bad', title: 'Chưa sao lưu', body: 'Gói có chỗ trông giống khoá nên app không ghi. Không có gì rời khỏi máy.' };
  return { tone: 'bad', title: 'Chưa sao lưu', body: 'Không ghi được vào iCloud Drive' + (s.startsWith('io:') ? ': ' + s.slice(3) : '') + '.' };
}

export function packedFeedback(at: Date): FeedbackView {
  return { tone: 'info', title: 'Đã sao lưu lúc ' + clock(at.toISOString()) + '.', body: 'Không có khoá trong gói.' };
}

/** A pack made on its own that did not go, said above the packs. */
export function autoErrorText(v: BackupView): string | null {
  if (!v.autoError) return null;
  if (v.autoError.kind === 'noICloud') return 'Tự sao lưu chưa chạy được: iCloud Drive đang tắt.';
  if (v.autoError.kind === 'secret') return 'Tự sao lưu dừng lại: gói có chỗ trông giống khoá nên app không ghi.';
  return 'Tự sao lưu chưa ghi được vào iCloud Drive.';
}

/** What "Hoàn tác lần khôi phục" will do, in words; `done` says what the restore did, when known. */
export function undoLines(done?: RestoreDone | null): [string, string][] {
  const parts = done?.parts?.filter((r) => r.ok) ?? null;
  const has = (part: string) => parts == null || parts.some((r) => r.part === part);
  const out: [string, string][] = [
    ['', 'Bỏ các MCP vừa thêm và trả các mục vừa thay về như trước khi khôi phục.'],
    ['', 'Bỏ các MCP app đang giữ tắt vì thiếu khoá hoặc lệnh.'],
    ['', 'Xoá các khoá vừa dán.'],
  ];
  if (has('skill') || has('plugin')) out.push(['', 'Gỡ các skill và plugin vừa cài, bằng lệnh của agent.']);
  if (has('file')) out.push(['', 'Trả file hướng dẫn vừa ghi về như trước.']);
  out.push(['ok', 'File bạn sửa sau lần khôi phục thì hỏi trước, không ghi đè.']);
  return out;
}

export function undoneFeedback(u: RestoreUndone): FeedbackView {
  const n = u.outcome ? u.outcome.done.reduce((t, d) => t + d.changes, 0) : 0;
  const parts = [
    n ? 'Trả ' + n + ' mục về như trước' : '',
    u.kept ? 'bỏ ' + u.kept + ' MCP đang giữ tắt' : '',
    u.keys ? 'xoá ' + u.keys + ' khoá vừa dán' : '',
    u.skills ? 'gỡ ' + u.skills + ' skill' : '',
    u.plugins ? 'gỡ ' + u.plugins + ' plugin' : '',
    u.files ? 'trả ' + u.files + ' file hướng dẫn về như trước' : '',
  ].filter(Boolean);
  const said = parts.join(', ');
  const left = u.left ? 'Giữ nguyên ' + u.left + ' mục bạn đã sửa hoặc chưa gỡ được. ' : '';
  return { tone: 'info', title: 'Đã hoàn tác lần khôi phục.', body: (said ? said.charAt(0).toUpperCase() + said.slice(1) + '. ' : '') + left + 'Mở lại agent để dùng.' };
}

export function undoError(e: unknown): FeedbackView {
  const s = String(e);
  if (s === 'nothing') return { tone: 'warn', title: 'Không còn gì để hoàn tác', body: 'Lần khôi phục đã được hoàn tác rồi.' };
  if (s === 'busy') return { tone: 'warn', title: 'Chưa hoàn tác', body: 'Một lần ghi khác đang chạy. Đợi xong rồi thử lại.' };
  return { tone: 'bad', title: 'Chưa hoàn tác', body: 'App không trả được file về như trước. Mở tab MCP để xem.' };
}

/** "File của Codex đã sửa sau lần khôi phục": the agents in a `changed:` refusal. */
export const changedAgents = (e: unknown): string | null => {
  const s = String(e);
  return s.startsWith('changed:') ? agentList(s.slice(8).split('\n')[0]) || null : null;
};

/** Each thing the undo found changed since the restore, in words. */
export function changedLines(e: unknown): string[] {
  const s = String(e);
  if (!s.startsWith('changed:')) return [];
  const [agents, ...rest] = s.slice(8).split('\n');
  const out: string[] = [];
  const who = agentList(agents);
  if (who) out.push('File của ' + who + ' đã sửa sau lần khôi phục.');
  for (const r of rest) {
    if (r.startsWith('file:')) {
      const path = r.slice(5);
      const agent: Agent | null = path.startsWith('~/.claude/') ? 'claude' : path.startsWith('~/.codex/') ? 'codex' : path.startsWith('~/.gemini/') ? 'agy' : null;
      out.push(fileLabel(path, agent) + ' đã sửa sau lần khôi phục.');
    } else if (r.startsWith('skill:')) out.push('Skill ' + r.slice(6) + ' đã sửa sau lần khôi phục.');
  }
  return out;
}
