// The Sao lưu tab's logic (M10), pure so Vitest covers it without a window:
// how a pack and its time read, what each line of the restore preview says,
// the keys the ticked servers need, the result in plain words and what is
// left for the owner to do. Copy follows the approved phase 2 prototype
// (round 2: no folder paths, no "Keychain", no commands).

import { clock } from './format';
import { AGENT_NAME, AGENTS } from './mcp';
import type { Agent, BackupView, PackInfo, RestoreDone, RestoreItem, RestoreItemResult, RestorePlan, RestoreUndone } from './model';
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

export function initialTicks(plan: RestorePlan): Ticks {
  return Object.fromEntries(plan.items.map((i) => [itemKey(i), i.checked]));
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

/** "Còn lại bạn làm". */
export function todoLines(done: RestoreDone): string[] {
  const out: string[] = [];
  for (const r of done.items) {
    if (r.why === 'noKey' || r.skipped === 'noKey') out.push('Dán khoá cho ' + r.name + ' ở tab MCP, rồi bật ' + r.name + '.');
    else if (r.why === 'noCommand' || r.skipped === 'noCommand') out.push('Cài ' + (r.command ?? 'lệnh của ' + r.name) + ' rồi bật ' + r.name + ' ở tab MCP.');
    if (r.login && r.done.length) out.push('Nếu agent hỏi, đăng nhập lại ' + r.name + ' trong ' + agentsText(r.done) + '.');
  }
  if (done.failed) out.push('Một phần chưa ghi được. Mở tab MCP, bấm Đọc lại rồi khôi phục lại phần còn thiếu.');
  if (done.written.length || done.items.some((r) => r.done.length)) out.push('Mở lại agent để dùng MCP vừa khôi phục.');
  return out;
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

/** What "Hoàn tác lần khôi phục" will do, in words. */
export function undoLines(): [string, string][] {
  return [
    ['', 'Bỏ các MCP vừa thêm và trả các mục vừa thay về như trước khi khôi phục.'],
    ['', 'Bỏ các MCP app đang giữ tắt vì thiếu khoá hoặc lệnh.'],
    ['', 'Xoá các khoá vừa dán.'],
    ['ok', 'File bạn sửa sau lần khôi phục thì hỏi trước, không ghi đè.'],
  ];
}

export function undoneFeedback(u: RestoreUndone): FeedbackView {
  const n = u.outcome ? u.outcome.done.reduce((t, d) => t + d.changes, 0) : 0;
  const parts = [n ? 'Trả ' + n + ' mục về như trước' : '', u.kept ? 'bỏ ' + u.kept + ' MCP đang giữ tắt' : '', u.keys ? 'xoá ' + u.keys + ' khoá vừa dán' : ''].filter(Boolean);
  const said = parts.join(', ');
  return { tone: 'info', title: 'Đã hoàn tác lần khôi phục.', body: (said ? said.charAt(0).toUpperCase() + said.slice(1) + '. ' : '') + 'Mở lại agent để dùng.' };
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
  return s.startsWith('changed:') ? agentList(s.slice(8)) : null;
};
