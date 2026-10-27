// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Sao lưu tab's logic (M10, M12), pure so Vitest covers it without a
// window: how a pack and its time read, what each line of the restore
// preview says (servers, then skills, plugins and instruction files in
// groups), the keys the ticked servers need, the result in plain words and
// what is left for the owner to do. Copy follows the approved phase 2
// prototype (round 2: no folder paths, no "Keychain", no commands).

import { clock } from './format';
import { cloudDrive, fileManager, mac, os, t } from './i18n';
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

/** "3 keys", "1 key": a count with its English noun. */
export const plural = (n: number, one: string, many: string): string => n + ' ' + (n === 1 ? one : many);

/** The day a pack was made, its group in the list: "Hôm nay", "Hôm qua", "17/10". */
export function packDay(at: number | null, now: Date): string {
  if (at == null) return t('Không rõ giờ', 'Unknown time');
  const d = new Date(at * 1000);
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (d.toDateString() === now.toDateString()) return t('Hôm nay', 'Today');
  if (d.toDateString() === yesterday.toDateString()) return t('Hôm qua', 'Yesterday');
  return pad(d.getDate()) + '/' + pad(d.getMonth() + 1);
}

/** The time of day a pack was made, "14:30"; nothing when it is not known. */
export const packClock = (at: number | null): string => (at == null ? '' : clock(new Date(at * 1000).toISOString()));

/** "Hôm nay 14:30", "Hôm qua 22:05", "17/10 21:48". */
export function packWhen(at: number | null, now: Date): string {
  if (at == null) return packDay(at, now);
  return packDay(at, now) + ' ' + packClock(at);
}

/** The same, inside a sentence: "gói hôm nay 14:30". */
export const packWhenLower = (at: number | null, now: Date): string => {
  const w = packWhen(at, now);
  return w.charAt(0).toLowerCase() + w.slice(1);
};

/** What a pack holds, in one line. */
export const packLine = (p: PackInfo): string => p.mcp + ' MCP';

export const macOf = (p: PackInfo, here: string): string => p.mac ?? (p.mine ? here : mac(t('Mac khác', 'Another Mac')));

/** A pack that can be opened and restored. */
export const usable = (p: PackInfo): boolean => p.state === 'ok' && !p.newer;

export function packNote(p: PackInfo): string | null {
  if (p.state === 'cloud') return t('chưa tải về máy', 'not downloaded yet');
  if (p.state === 'broken') return t('gói hỏng, không đọc được', 'damaged pack, cannot be read');
  if (p.newer) return t('của bản app mới hơn', 'from a newer app version');
  return null;
}

export const keysLine = (n: number): string =>
  n
    ? t('Không chứa khoá. Khi khôi phục, app hỏi lại ' + n + ' khoá.', 'No keys inside. On restore, the app asks for ' + plural(n, 'key', 'keys') + ' again.')
    : t('Không chứa khoá. Gói này không cần dán khoá.', 'No keys inside. This pack needs no keys pasted.');

function agentsText(agents: Agent[]): string {
  const sorted = AGENTS.filter((a) => agents.includes(a));
  if (sorted.length === 3) return t('cả ba agent', 'all three agents');
  return sorted.map((a) => AGENT_NAME[a]).join(', ');
}

export const itemKey = (i: { name: string; kind: string }): string => i.name + '|' + i.kind;

/** The line under a server in the restore preview. */
export function itemLine(i: RestoreItem): string {
  if (i.kind === 'replace') return mac(t('Máy này đã có, khác gói. Bỏ chọn là giữ bản trên máy.', 'This Mac has it, different from the pack. Untick to keep this Mac’s version.'));
  if (i.missingCommand) return mac(t('Máy này chưa cài ' + i.missingCommand + ': thêm nhưng để tắt', i.missingCommand + ' is not installed on this Mac: added but left off'));
  const parts = [t('Thêm cho ' + agentsText(i.agents), 'Added for ' + agentsText(i.agents))];
  if (i.off.length === i.agents.length) parts.push(t('để tắt như trong gói', 'left off as in the pack'));
  else if (i.off.length) parts.push(t('tắt ở ' + agentsText(i.off) + ' như trong gói', 'off in ' + agentsText(i.off) + ' as in the pack'));
  if (i.keys.length) parts.push(t('cần dán khoá', 'needs a key'));
  if (i.moved) parts.push(mac(t('app tự chỉnh cho hợp máy này', 'the app adjusts it for this Mac')));
  if (i.login) parts.push(t('sau đó đăng nhập lại trong agent nếu được hỏi', 'then sign in again in the agent if asked'));
  return parts.join(' · ');
}

/** What the restore leaves out on this Mac, said once. */
export function skippedLines(plan: RestorePlan): string[] {
  const out: string[] = [];
  for (const a of AGENTS) {
    const name = AGENT_NAME[a];
    if (plan.skipped.some((s) => s.agent === a && s.reason === 'absent'))
      out.push(mac(t('Máy này chưa cài ' + name + ': bỏ qua phần của ' + name + '.', name + ' is not installed on this Mac: skipping its part.')));
    if (plan.skipped.some((s) => s.agent === a && s.reason === 'readOnly'))
      out.push(t('Chưa ghi được vào ' + name + ' lúc này: bỏ qua phần của ' + name + '. Xem tab MCP.', 'Cannot write to ' + name + ' right now: skipping its part. See the MCP tab.'));
  }
  const sse = plan.skipped.filter((s) => s.reason === 'sse' && s.name).map((s) => s.name as string);
  if (sse.length)
    out.push(t('Codex không chạy MCP kiểu SSE: ' + sse.join(', ') + ' chỉ khôi phục cho agent khác.', 'Codex does not run SSE MCP servers: ' + sse.join(', ') + ' is restored for the other agents only.'));
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
  const out: Ticks = Object.fromEntries(plan.items.map((i) => [itemKey(i), i.checked]));
  const sk = skillGroups(plan);
  if (sk.github.length) out[SKILLS_GITHUB] = true;
  if (sk.hand.length) out[SKILLS_HAND] = true;
  const pl = pluginGroups(plan);
  if (pl.main.length) out[PLUGINS] = true;
  for (const p of pl.off) out[pluginKey(p)] = false;
  for (const f of plan.files?.items ?? []) out[fileKey(f.path)] = f.checked;
  return out;
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
  const head = list.slice(0, max - 1).join(', ');
  const more = list.length - max + 1;
  return t(head + ' và ' + more + ' khác', head + ' and ' + more + ' more');
}

const fileName = (path: string): string => path.split('/').pop() ?? path;

/** "CLAUDE.md của Claude Code", "quy tắc style.md của Claude Code", "team.md". */
export function fileLabel(path: string, agent: Agent | null): string {
  const name = fileName(path);
  if (!agent) return name;
  const rule = agent === 'claude' && path.includes('/rules/');
  return t((rule ? 'quy tắc ' : '') + name + ' của ' + AGENT_NAME[agent], (rule ? 'rule ' : '') + name + ' for ' + AGENT_NAME[agent]);
}

export const SKILL_TITLE = {
  github: (n: number) => t(n + ' skill từ GitHub', plural(n, 'skill', 'skills') + ' from GitHub'),
  hand: (n: number) => t(n + ' skill cài tay', plural(n, 'skill', 'skills') + ' installed by hand'),
};
export const skillLine = (kind: 'github' | 'hand', items: RestoreSkillItem[]): string =>
  (kind === 'github' ? t('Cài lại từ GitHub, bản mới nhất', 'Reinstalled from GitHub, latest version') : t('Chép từ gói', 'Copied from the pack')) +
  ' · ' +
  names(items.map((i) => i.name));

/** Who installs the plugins: "Cài bằng chính Claude Code và Codex". */
export function pluginLine(items: RestorePluginItem[]): string {
  const agents = AGENTS.filter((a) => items.some((i) => i.agent === a)).map((a) => AGENT_NAME[a]);
  const list = names(items.map((i) => i.id.split('@')[0]));
  return t('Cài bằng chính ' + agents.join(' và ') + ' · ' + list, 'Installed by ' + agents.join(' and ') + ' itself · ' + list);
}

export function fileLine(f: RestoreFileItem): string {
  return f.kind === 'add'
    ? mac(t('Máy này chưa có', 'Not on this Mac yet'))
    : mac(t('Khác ' + f.changed + ' dòng. Bỏ chọn là giữ bản trên máy.', plural(f.changed, 'line differs', 'lines differ') + '. Untick to keep this Mac’s version.'));
}

/** Why a skill copied by hand is not in the pack, by reason. */
const leftOut = (): Record<string, string> => ({
  key: t('có dòng giống khoá nên không có trong gói', 'a line looks like a key, so it is not in the pack'),
  link: t('có liên kết tới chỗ khác nên không chép vào gói', 'links elsewhere, so it was not copied into the pack'),
  large: t('lớn quá nên không chép vào gói', 'too large, so it was not copied into the pack'),
  unreadable: t('không đọc được lúc sao lưu', 'could not be read during the backup'),
  old: t('gói này cũ, chưa chép skill cài tay', 'this pack is older and has no skills installed by hand'),
});

/** What the skills, plugins and files of the preview leave out, said once each. */
export function partNotes(plan: RestorePlan): string[] {
  const out: string[] = [];
  const sk = skillGroups(plan);
  const noNpx = sk.blocked.filter((i) => i.blocked === 'noNpx').map((i) => i.name);
  if (noNpx.length)
    out.push(mac(t('Máy này chưa có Node: chưa cài lại được ' + names(noNpx) + '. Cài Node rồi khôi phục lại.', 'Node is not installed on this Mac: cannot reinstall ' + names(noNpx) + '. Install Node, then restore again.')));
  const cloud = sk.blocked.filter((i) => i.blocked === 'cloud').map((i) => i.name);
  if (cloud.length)
    out.push(t(cloudDrive() + ' chưa tải về ' + names(cloud) + '. Mở thư mục gói trong ' + fileManager() + ' để tải.', cloudDrive() + ' has not downloaded ' + names(cloud) + ' yet. Open the pack folder in ' + fileManager() + ' to download.'));
  for (const [why, text] of Object.entries(leftOut())) {
    const list = (plan.skills?.leftOut ?? []).filter((l) => l.why === why).map((l) => l.name);
    if (list.length) out.push(t('Skill ' + names(list) + ' ' + text + '.', (list.length === 1 ? 'Skill ' : 'Skills ') + names(list) + ': ' + text + '.'));
  }
  for (const s of plan.plugins?.skipped ?? []) {
    const name = AGENT_NAME[s.agent];
    out.push(
      s.reason === 'noCli'
        ? mac(t('Máy này chưa cài ' + name + ': bỏ qua plugin của ' + name + '.', name + ' is not installed on this Mac: skipping its plugins.'))
        : t('Chưa đọc được plugin của ' + name + ' lúc này: bỏ qua.', 'Cannot read ' + name + '’s plugins right now: skipped.'),
    );
  }
  const noSource = pluginGroups(plan).blocked.map((p) => p.id);
  if (noSource.length)
    out.push(
      mac(
        t(
          'Plugin ' + names(noSource) + ' lấy từ thư mục trên Mac kia: cài tay nếu cần.',
          (noSource.length === 1 ? 'Plugin ' : 'Plugins ') + names(noSource) + ' came from a folder on the other Mac: install by hand if needed.',
        ),
      ),
    );
  for (const k of plan.files?.kept ?? []) {
    const label = fileLabel(k.path, k.agent);
    if (k.why === 'key') out.push(mac(t(label + ' trên máy có dòng giống khoá nên app giữ nguyên.', label + ' on this Mac has a line that looks like a key, so the app leaves it as is.')));
    else if (k.why === 'absent' && k.agent)
      out.push(mac(t('Máy này chưa cài ' + AGENT_NAME[k.agent] + ': bỏ qua ' + label + '.', AGENT_NAME[k.agent] + ' is not installed on this Mac: skipping ' + label + '.')));
    else out.push(t('Bỏ qua ' + label + '.', 'Skipping ' + label + '.'));
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
  const mcp = 'MCP ' + r.name;
  const notWritten = t('chưa ghi được', 'not written');
  const noKey = t('chưa có khoá', 'no key yet');
  if (r.kind === 'replace') {
    if (r.done.length) return ['', t(mcp + ' · thay bằng bản trong gói', mcp + ' · replaced with the pack’s version'), ''];
    if (r.skipped)
      return ['w', mac(t(mcp + ' · giữ bản trên máy', mcp + ' · kept this Mac’s version')), r.skipped === 'noKey' ? noKey : t('máy chưa cài lệnh của nó', 'its command is not installed')];
    return ['x', mcp, notWritten];
  }
  if (r.failed.length && !n) return ['x', mcp, notWritten];
  if (r.kept.length && !r.done.length)
    return [
      'w',
      t(mcp + ' · thêm nhưng tắt', mcp + ' · added but off'),
      r.why === 'noCommand'
        ? t('máy chưa cài ' + (r.command ?? 'lệnh của nó'), r.command ? r.command + ' is not installed' : 'its command is not installed')
        : r.why === 'noKey'
          ? noKey
          : t('tắt như trong gói', 'off as in the pack'),
    ];
  const sub = [
    r.kept.length
      ? t('tắt ở ', 'off in ') + agentsText(r.kept) + (r.why === 'noKey' ? t(', chưa có khoá', ', no key yet') : r.why === 'noCommand' ? t(', chưa có lệnh', ', no command yet') : '')
      : '',
    r.failed.length ? t('chưa ghi được ở ', 'not written in ') + agentsText(r.failed) : '',
    r.login ? t('có thể cần đăng nhập lại trong agent', 'may need to sign in again in the agent') : '',
  ].filter(Boolean);
  return [r.failed.length ? 'w' : '', mcp + ' · ' + t(n + ' agent', plural(n, 'agent', 'agents')), sub.join(' · ')];
}

/** Why a part of the restore failed, by reason. */
const whyText = (): Record<string, string> => ({
  noNpx: t('máy chưa có Node', 'Node is not installed'),
  npx: t('chưa cài được từ GitHub', 'could not install from GitHub'),
  cloud: t(cloudDrive() + ' chưa tải về', cloudDrive() + ' has not downloaded it'),
  damaged: t('bản trong gói bị hỏng', 'the pack’s copy is damaged'),
  taken: t('máy đã có skill cùng tên', 'a skill with this name is already here'),
  links: t('chưa bật được cho agent', 'could not turn it on for the agents'),
  noCli: t('máy chưa cài agent này', 'this agent is not installed'),
  login: t('cần đăng nhập', 'needs sign-in'),
  command: t('lệnh cài của agent chưa chạy được', 'the agent’s install command did not run'),
  changed: mac(t('file vừa đổi trên máy', 'the file just changed on this Mac')),
  io: t('chưa ghi được', 'not written'),
});

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

/** `from`: what a replaced file was replaced with, the pack's copy (a restore) or the sending Mac's (a copy). */
export function partLines(parts: RestorePartResult[], from: 'pack' | 'sender' = 'pack'): PartLine[] {
  const out: PartLine[] = [];
  const why = whyText();
  const replaced = from === 'sender' ? mac(t(' · thay bằng bản của máy gửi', ' · replaced with the sending Mac’s version')) : t(' · thay bằng bản trong gói', ' · replaced with the pack’s version');
  const ok = (part: string, kind?: string) => parts.filter((r) => r.ok && r.part === part && (!kind || r.kind === kind));
  const gh = ok('skill', 'github');
  if (gh.length) out.push({ tone: '', title: SKILL_TITLE.github(gh.length), sub: names(gh.map((r) => r.id)), retry: null });
  const hand = ok('skill', 'hand');
  if (hand.length) out.push({ tone: '', title: SKILL_TITLE.hand(hand.length), sub: names(hand.map((r) => r.id)), retry: null });
  const pl = ok('plugin');
  if (pl.length) out.push({ tone: '', title: t(pl.length + ' plugin', plural(pl.length, 'plugin', 'plugins')), sub: names(pl.map((r) => r.id.split('@')[0])), retry: null });
  for (const f of ok('file')) out.push({ tone: '', title: partName(f) + (f.kind === 'replace' ? replaced : ''), sub: '', retry: null });
  for (const r of parts.filter((x) => !x.ok)) out.push({ tone: 'x', title: partName(r), sub: why[r.why ?? 'io'] ?? why.io, retry: r.retry ? r : null });
  return out;
}

/** "Còn lại bạn làm". */
export function todoLines(done: RestoreDone): string[] {
  const out: string[] = [];
  for (const r of done.items) {
    if (r.why === 'noKey' || r.skipped === 'noKey') out.push(t('Dán khoá cho ' + r.name + ' ở tab MCP, rồi bật ' + r.name + '.', 'Paste the key for ' + r.name + ' in the MCP tab, then turn ' + r.name + ' on.'));
    else if (r.why === 'noCommand' || r.skipped === 'noCommand')
      out.push(
        t(
          'Cài ' + (r.command ?? 'lệnh của ' + r.name) + ' rồi bật ' + r.name + ' ở tab MCP.',
          'Install ' + (r.command ?? r.name + '’s command') + ', then turn ' + r.name + ' on in the MCP tab.',
        ),
      );
    if (r.login && r.done.length) out.push(t('Nếu agent hỏi, đăng nhập lại ' + r.name + ' trong ' + agentsText(r.done) + '.', 'If asked, sign in to ' + r.name + ' again in ' + agentsText(r.done) + '.'));
  }
  if (done.failed)
    out.push(t('Một phần chưa ghi được. Mở tab MCP, bấm Đọc lại rồi khôi phục lại phần còn thiếu.', 'Part of it was not written. Open the MCP tab, press Read again, then restore what is missing.'));
  const parts = done.parts ?? [];
  for (const r of parts.filter((x) => !x.ok)) {
    const name = partName(r);
    if (r.why === 'login')
      out.push(
        t(
          'Đăng nhập trong ' + (r.agent ? AGENT_NAME[r.agent] : 'agent') + ' rồi bấm Thử lại ở ' + name.toLowerCase() + '.',
          'Sign in to ' + (r.agent ? AGENT_NAME[r.agent] : 'the agent') + ', then press Try again on ' + name + '.',
        ),
      );
    else if (r.why === 'noNpx') out.push(t('Cài Node rồi khôi phục lại ' + name.toLowerCase() + '.', 'Install Node, then restore ' + name + ' again.'));
    else if (r.why === 'cloud') out.push(t('Mở thư mục gói trong ' + fileManager() + ' để ' + cloudDrive() + ' tải về, rồi bấm Thử lại.', 'Open the pack folder in ' + fileManager() + ' so ' + cloudDrive() + ' downloads it, then press Try again.'));
    else if (r.why === 'changed') out.push(mac(t(name + ' vừa đổi trên máy: mở Khôi phục lần nữa để xem trước lại.', name + ' just changed on this Mac: open Restore again to preview it anew.')));
  }
  const used = [
    done.written.length || done.items.some((r) => r.done.length) ? 'MCP' : '',
    parts.some((r) => r.ok && r.part === 'skill') ? t('skill', 'skills') : '',
    parts.some((r) => r.ok && r.part === 'plugin') ? t('plugin', 'plugins') : '',
  ].filter(Boolean);
  if (used.length) out.push(t('Mở lại agent để dùng ' + used.join(', ') + ' vừa khôi phục.', 'Reopen the agents to use the restored ' + used.join(', ') + '.'));
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
  if (s === 'busy')
    return {
      tone: 'warn',
      title: t('Chưa khôi phục', 'Not restored'),
      body: t('Một lần ghi khác hoặc lần chuyển Codex đang chạy. Đợi xong rồi thử lại.', 'Another write or a Codex switch is running. Wait for it to finish, then try again.'),
    };
  if (s.startsWith('changed:'))
    return {
      tone: 'warn',
      title: t('Chưa khôi phục: file của ' + agentList(s.slice(8)) + ' vừa đổi', 'Not restored: ' + agentList(s.slice(8)) + ' files just changed'),
      body: t('Chưa ghi gì. Quay lại bước xem trước để đọc lại.', 'Nothing was written. Go back to the preview to read again.'),
    };
  if (s === 'keychain')
    return {
      tone: 'bad',
      title: t('Chưa lưu được khoá', 'Could not save the key'),
      body: os(
        t('Keychain không nhận khoá lúc này. Chưa ghi gì.', 'Keychain is not accepting keys right now. Nothing was written.'),
        t('Kho mã hoá không nhận khoá lúc này. Chưa ghi gì.', 'The encrypted store is not accepting keys right now. Nothing was written.'),
      ),
    };
  if (s === 'cloud')
    return {
      tone: 'warn',
      title: t('Gói chưa tải về máy', 'Pack not downloaded yet'),
      body: t(cloudDrive() + ' mới có tên file. Mở thư mục trong ' + fileManager() + ' để tải, hoặc chờ.', cloudDrive() + ' only has the file name so far. Open the folder in ' + fileManager() + ' to download it, or wait.'),
    };
  if (s === 'newer')
    return { tone: 'warn', title: t('Gói của bản app mới hơn', 'Pack from a newer app version'), body: mac(t('Cập nhật Quota Station trên máy này rồi thử lại.', 'Update Quota Station on this Mac, then try again.')) };
  if (s === 'missing')
    return { tone: 'warn', title: t('Không còn gói này', 'This pack is gone'), body: t('Gói vừa bị xoá hoặc đổi tên. Bấm Kiểm lại.', 'The pack was just deleted or renamed. Press Check again.') };
  if (s.startsWith('invalid')) return { tone: 'bad', title: t('Gói hỏng, không đọc được', 'Damaged pack, cannot be read'), body: t('Chọn một gói khác.', 'Choose another pack.') };
  return { tone: 'bad', title: t('Chưa khôi phục', 'Not restored'), body: t('App không chạy được bước này. Các file không đổi.', 'The app could not run this step. No files changed.') };
}

export function backupNowError(e: unknown): FeedbackView {
  const s = String(e);
  if (s === 'busy') return { tone: 'warn', title: t('Đang sao lưu', 'Backing up'), body: t('Một lần sao lưu đang chạy. Đợi xong rồi thử lại.', 'A backup is running. Wait for it to finish, then try again.') };
  if (s === 'noICloud')
    return {
      tone: 'bad',
      title: t('Chưa sao lưu: ' + cloudDrive() + ' đang tắt', 'Not backed up: ' + cloudDrive() + ' is off'),
      body: os(
        t('Bật iCloud Drive trong Cài đặt hệ thống › Apple ID › iCloud rồi thử lại.', 'Turn on iCloud Drive in System Settings › Apple ID › iCloud, then try again.'),
        t('Đăng nhập OneDrive rồi thử lại.', 'Sign in to OneDrive, then try again.'),
      ),
    };
  if (s === 'secret')
    return {
      tone: 'bad',
      title: t('Chưa sao lưu', 'Not backed up'),
      body: mac(t('Gói có chỗ trông giống khoá nên app không ghi. Không có gì rời khỏi máy.', 'Part of the pack looks like a key, so the app did not write it. Nothing left this Mac.')),
    };
  const detail = s.startsWith('io:') ? ': ' + s.slice(3) : '';
  return { tone: 'bad', title: t('Chưa sao lưu', 'Not backed up'), body: t('Không ghi được vào ' + cloudDrive() + detail + '.', 'Could not write to ' + cloudDrive() + detail + '.') };
}

export function packedFeedback(at: Date): FeedbackView {
  const time = clock(at.toISOString());
  return { tone: 'info', title: t('Đã sao lưu lúc ' + time + '.', 'Backed up at ' + time + '.'), body: t('Không có khoá trong gói.', 'No keys in the pack.') };
}

/** A pack made on its own that did not go, said above the packs. */
export function autoErrorText(v: BackupView): string | null {
  if (!v.autoError) return null;
  if (v.autoError.kind === 'noICloud') return t('Tự sao lưu chưa chạy được: ' + cloudDrive() + ' đang tắt.', 'Automatic backup could not run: ' + cloudDrive() + ' is off.');
  if (v.autoError.kind === 'secret')
    return t('Tự sao lưu dừng lại: gói có chỗ trông giống khoá nên app không ghi.', 'Automatic backup stopped: part of the pack looks like a key, so the app did not write it.');
  return t('Tự sao lưu chưa ghi được vào ' + cloudDrive() + '.', 'Automatic backup could not write to ' + cloudDrive() + '.');
}

/** What "Hoàn tác lần khôi phục" will do, in words; `done` says what the restore did, when known. */
export function undoLines(done?: RestoreDone | null): [string, string][] {
  const parts = done?.parts?.filter((r) => r.ok) ?? null;
  const has = (part: string) => parts == null || parts.some((r) => r.part === part);
  const out: [string, string][] = [
    ['', t('Bỏ các MCP vừa thêm và trả các mục vừa thay về như trước khi khôi phục.', 'Removes the MCP servers just added and puts back what was replaced, as before the restore.')],
    ['', t('Bỏ các MCP app đang giữ tắt vì thiếu khoá hoặc lệnh.', 'Removes the MCP servers the app keeps off for a missing key or command.')],
    ['', t('Xoá các khoá vừa dán.', 'Deletes the keys just pasted.')],
  ];
  if (has('skill') || has('plugin')) out.push(['', t('Gỡ các skill và plugin vừa cài, bằng lệnh của agent.', 'Uninstalls the skills and plugins just installed, with the agent’s own commands.')]);
  if (has('file')) out.push(['', t('Trả file hướng dẫn vừa ghi về như trước.', 'Puts the instruction files just written back as they were.')]);
  out.push(['ok', t('File bạn sửa sau lần khôi phục thì hỏi trước, không ghi đè.', 'Files you edited after the restore are asked about first, never overwritten.')]);
  return out;
}

export function undoneFeedback(u: RestoreUndone): FeedbackView {
  const n = u.outcome ? u.outcome.done.reduce((sum, d) => sum + d.changes, 0) : 0;
  const parts = [
    n ? t('Trả ' + n + ' mục về như trước', 'put back ' + plural(n, 'item', 'items')) : '',
    u.kept ? t('bỏ ' + u.kept + ' MCP đang giữ tắt', 'removed ' + u.kept + ' MCP kept off') : '',
    u.keys ? t('xoá ' + u.keys + ' khoá vừa dán', 'deleted ' + plural(u.keys, 'key', 'keys') + ' just pasted') : '',
    u.skills ? t('gỡ ' + u.skills + ' skill', 'uninstalled ' + plural(u.skills, 'skill', 'skills')) : '',
    u.plugins ? t('gỡ ' + u.plugins + ' plugin', 'uninstalled ' + plural(u.plugins, 'plugin', 'plugins')) : '',
    u.files ? t('trả ' + u.files + ' file hướng dẫn về như trước', 'put back ' + plural(u.files, 'instruction file', 'instruction files')) : '',
  ].filter(Boolean);
  const said = parts.join(', ');
  const left = u.left ? t('Giữ nguyên ' + u.left + ' mục bạn đã sửa hoặc chưa gỡ được. ', 'Kept ' + plural(u.left, 'item', 'items') + ' you edited or that could not be removed. ') : '';
  return {
    tone: 'info',
    title: t('Đã hoàn tác lần khôi phục.', 'Restore undone.'),
    body: (said ? said.charAt(0).toUpperCase() + said.slice(1) + '. ' : '') + left + t('Mở lại agent để dùng.', 'Reopen the agents to use it.'),
  };
}

export function undoError(e: unknown): FeedbackView {
  const s = String(e);
  if (s === 'nothing') return { tone: 'warn', title: t('Không còn gì để hoàn tác', 'Nothing left to undo'), body: t('Lần khôi phục đã được hoàn tác rồi.', 'The restore was already undone.') };
  if (s === 'busy') return { tone: 'warn', title: t('Chưa hoàn tác', 'Not undone'), body: t('Một lần ghi khác đang chạy. Đợi xong rồi thử lại.', 'Another write is running. Wait for it to finish, then try again.') };
  return { tone: 'bad', title: t('Chưa hoàn tác', 'Not undone'), body: t('App không trả được file về như trước. Mở tab MCP để xem.', 'The app could not put the files back. Open the MCP tab to check.') };
}

/** "File của Codex đã sửa sau lần khôi phục": the agents in a `changed:` refusal. */
export const changedAgents = (e: unknown): string | null => {
  const s = String(e);
  return s.startsWith('changed:') ? agentList(s.slice(8).split('\n')[0]) || null : null;
};

/** Each thing the undo found changed since the restore (or, for `copy`, since the copy), in words. */
export function changedLines(e: unknown, since: 'restore' | 'copy' = 'restore'): string[] {
  const s = String(e);
  if (!s.startsWith('changed:')) return [];
  const [agents, ...rest] = s.slice(8).split('\n');
  const out: string[] = [];
  const who = agentList(agents);
  const vi = since === 'copy' ? ' đã sửa sau lần chép.' : ' đã sửa sau lần khôi phục.';
  const en = since === 'copy' ? ' after the copy.' : ' after the restore.';
  if (who) out.push(t('File của ' + who + vi, who + ' files were edited' + en));
  for (const r of rest) {
    if (r.startsWith('file:')) {
      const path = r.slice(5);
      const agent: Agent | null = path.startsWith('~/.claude/') ? 'claude' : path.startsWith('~/.codex/') ? 'codex' : path.startsWith('~/.gemini/') ? 'agy' : null;
      const label = fileLabel(path, agent);
      out.push(t(label + vi, label + ' was edited' + en));
    } else if (r.startsWith('skill:')) out.push(t('Skill ' + r.slice(6) + vi, 'Skill ' + r.slice(6) + ' was edited' + en));
  }
  return out;
}
