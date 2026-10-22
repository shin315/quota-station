// The Skills, Plugin and File hướng dẫn tabs' words (M11), pure so Vitest
// covers them without a window: the filters, the line under each name, the
// chips, why a box is locked, the batch's lead line, and the lines after a
// write, an install or a check. The boxes themselves use `mcp.ts`'s helpers.
// Copy follows the approved phase 2 prototype.

import { clock } from './format';
import { AGENT_NAME, AGENTS, pendingList, type Pending, type PendingItem } from './mcp';
import type { Agent, NpxError, Plugin, SkillChange, SkillFailure, SkillLock, SkillOutcome, SkillRefusal, SkillRow, SkillTable, SkillUpdatedMany, SkillsView } from './model';
import type { BannerView, ChipView, FeedbackView } from './view';

export type SkillFilter = 'all' | 'github' | 'hand' | 'update';

export function filterRows(rows: SkillRow[], f: SkillFilter): SkillRow[] {
  if (f === 'github') return rows.filter((r) => r.source === 'github');
  if (f === 'hand') return rows.filter((r) => r.source === 'hand');
  if (f === 'update') return rows.filter((r) => r.update);
  return rows;
}

export function filterCounts(rows: SkillRow[]): Record<SkillFilter, number> {
  return { all: rows.length, github: filterRows(rows, 'github').length, hand: filterRows(rows, 'hand').length, update: filterRows(rows, 'update').length };
}

export const FILTER_WORD: Record<SkillFilter, string> = { all: 'Tất cả', github: 'Từ GitHub', hand: 'Cài tay', update: 'Có bản mới' };

/** Under the name: where it comes from. */
export function sourceLine(row: SkillRow): string {
  if (row.source === 'github') return 'GitHub · ' + row.repo;
  if (row.source === 'other') return 'nguồn khác · ' + row.repo;
  return row.home ? 'cài tay · thư mục của ' + AGENT_NAME[row.home] : 'cài tay';
}

/** Chips only when something is the matter, or new. */
export function skillChips(row: SkillRow): ChipView[] {
  const out: ChipView[] = [];
  if (row.update) out.push({ t: 'có bản mới', k: 'info', title: row.update.plain + '. App không tự cập nhật.' });
  if (row.edited) out.push({ t: 'bạn đã sửa', k: 'warn', title: 'Thư mục khác bản đã cài. Cập nhật sẽ thay phần bạn sửa.' });
  if (Object.values(row.cells).some((c) => c.lock === 'taken'))
    out.push({ t: 'trùng tên', k: 'warn', title: 'Một agent có thư mục riêng cùng tên, nên app không đổi ô của agent đó.' });
  return out;
}

export function skillLockText(lock: SkillLock): string {
  switch (lock) {
    case 'readOnly':
      return 'cột này chỉ xem';
    case 'hand':
      return 'nằm ở thư mục riêng của agent, gom về một chỗ để bật tắt';
    case 'taken':
      return 'agent có thư mục riêng cùng tên, app để yên';
    case 'broken':
      return 'liên kết hỏng';
  }
}

/** The box's hover text. */
export function skillCellTitle(row: SkillRow, agent: Agent): string {
  const c = row.cells[agent];
  if (c.lock) return row.name + ': ' + skillLockText(c.lock);
  if (agent === 'codex') return 'Codex đọc thẳng thư mục chung. Tắt là một mục trong config.toml.';
  return '';
}

export function skillRefusalText(reason: SkillRefusal): string {
  if (reason === 'badName') return 'tên không hợp lệ';
  if (reason === 'unknown') return 'không còn trong thư mục chung';
  if (reason === 'nothing') return 'đã như vậy rồi';
  return skillLockText(reason);
}

export const pendingSkillText = (p: PendingItem): string => p.name + ': ' + (p.on ? 'bật cho ' : 'tắt cho ') + AGENT_NAME[p.agent];

export function skillChanges(pending: Pending, rows: SkillRow[]): SkillChange[] {
  return pendingList(pending, rows).map((p) => ({ name: p.name, agent: p.agent, on: p.on }));
}

/** What each column holds, in the batch preview's words. */
export function groupFile(agent: Agent): string {
  return agent === 'codex' ? 'config.toml' : 'liên kết của ' + AGENT_NAME[agent];
}

function names(list: string[]): string {
  return list.length <= 1 ? list.join('') : list.slice(0, -1).join(', ') + ' và ' + list[list.length - 1];
}

export function skillFailureText(f: SkillFailure, agent: Agent): { title: string; body: string } {
  const file = groupFile(agent);
  switch (f.kind) {
    case 'changed':
      return { title: 'Chưa đổi gì: ' + file + ' vừa bị chương trình khác đổi.', body: 'Đọc lại rồi thử lại.' };
    case 'refused':
      return {
        title: 'Chưa đổi gì: danh sách đã khác lúc xem trước.',
        body: f.refused.map((r) => r.name + ' cho ' + AGENT_NAME[r.agent] + ': ' + skillRefusalText(r.reason)).join(' · ') + '. Đọc lại rồi xem trước lần nữa.',
      };
    case 'edit':
      return { title: 'Chưa ghi ' + file + '.', body: 'App không sửa được đúng mục đó, nên không ghi gì.' };
    case 'write':
      return { title: 'Chưa ghi ' + file + '.', body: 'Lỗi khi ghi: ' + f.detail + '. Phần đang dùng không đổi.' };
  }
}

/** The line after a batch: what was written, its backups, and the undo. */
export function skillWrittenFeedback(out: SkillOutcome): FeedbackView {
  const n = out.done.reduce((t, d) => t + d.changes, 0);
  const at = clock(new Date(out.at * 1000).toISOString());
  const kept = names(out.done.filter((d) => d.backup).map((d) => groupFile(d.agent)));
  const failed = out.failed ? skillFailureText(out.failed.error, out.failed.agent) : null;
  const actions = out.undo ? [{ label: 'Hoàn tác', act: 'skills:undo' as const }] : [];
  if (failed && !n) return { tone: 'bad', title: failed.title, body: failed.body, actions: [{ label: 'Đọc lại', act: 'skills:reread' }] };
  if (failed)
    return { tone: 'warn', title: 'Đã ghi ' + n + ' thay đổi, phần còn lại chưa ghi.', body: failed.title + ' ' + failed.body + (kept ? ' Đã sao lưu ' + kept + ' lúc ' + at + '.' : ''), actions };
  return { tone: 'info', title: 'Đã ghi ' + n + ' thay đổi.', body: (kept ? 'Đã sao lưu ' + kept + ' lúc ' + at + '. ' : '') + 'Mở lại agent để dùng.', actions };
}

/** What `npx skills` said, in words: a code from the Rust side, or its error. */
export function npxText(e: NpxError | string): { title: string; body: string } {
  const code = typeof e === 'string' ? e : e.kind === 'failed' ? 'failed:' + e.detail : e.kind;
  if (code === 'noNpx') return { title: 'Không tìm thấy npx', body: 'Cài Node.js rồi bấm Đọc lại. Bật, tắt và gom skill vẫn dùng được.' };
  if (code === 'badRepo') return { title: 'Không phải repo GitHub', body: 'Gõ dạng chủ/tên-repo, hoặc dán link GitHub của repo.' };
  if (code === 'noSkills') return { title: 'Repo này không có skill nào', body: 'Không thấy thư mục nào có SKILL.md trong repo.' };
  if (code === 'timeout') return { title: 'npx skills chạy quá lâu', body: 'Kiểm mạng rồi thử lại.' };
  if (code === 'busy') return { title: 'Đang có một lần ghi khác', body: 'Đợi xong rồi thử lại.' };
  if (code.startsWith('failed:')) return { title: 'npx skills báo lỗi', body: code.slice('failed:'.length) || 'Không rõ lý do.' };
  return { title: 'Chưa làm được', body: code };
}

/** The skills with a newer version, by the repository they came from: a set from one repository usually updates together. A skill whose source is not known is only in "Cập nhật tất cả". */
export function updateGroups(rows: SkillRow[]): { repo: string; names: string[] }[] {
  const out = new Map<string, string[]>();
  for (const r of rows) if (r.update && r.repo) out.set(r.repo, [...(out.get(r.repo) ?? []), r.name]);
  return [...out].map(([repo, names]) => ({ repo, names }));
}

/** The skills of `names` ticked when the preview opens: all but those edited on this Mac, whose edits an update would replace. */
export function bulkPicked(rows: SkillRow[], names: string[]): string[] {
  return names.filter((n) => rows.some((r) => r.name === n && r.update && !r.edited));
}

/** The line after "Cập nhật tất cả" or "Cập nhật cả bộ": how many updated, which failed and why, and where the copies are. */
export function bulkUpdatedFeedback(out: SkillUpdatedMany): FeedbackView {
  const done = out.results.filter((r) => !r.error);
  const failed = out.results.filter((r) => r.error);
  const why = failed.map((r) => r.name + ' (' + (r.error!.kind === 'failed' ? r.error!.detail || 'không rõ lý do' : npxText(r.error!).title.toLowerCase()) + ')').join(', ');
  const body = [
    failed.length ? 'Chưa cập nhật ' + why + '.' : '',
    done.length ? 'Agent nào đang bật vẫn bật.' : '',
    out.results.some((r) => r.backup) ? 'Bản cũ đã lưu ở ' + out.backups + '.' : '',
  ]
    .filter(Boolean)
    .join(' ');
  if (!done.length) return { tone: 'bad', title: 'Chưa cập nhật skill nào.', body };
  if (!failed.length) return { tone: 'info', title: 'Đã cập nhật ' + done.length + ' skill.', body };
  return { tone: 'warn', title: 'Đã cập nhật ' + done.length + ' skill, ' + failed.length + ' skill chưa cập nhật.', body };
}

/** "kiểm bản mới lúc 09:12", in the title row. */
export function checkedLine(v: SkillsView): string {
  const u = v.updates;
  if (u.checking) return 'đang kiểm bản mới…';
  if (!u.settings.daily && !u.checkedAt) return 'không tự kiểm bản mới';
  return u.checkedAt ? 'kiểm bản mới lúc ' + clock(u.checkedAt) : 'chưa kiểm bản mới';
}

/** The banners above the table: npx missing, newer versions, a check that failed, the lock file, a column. */
export function skillBanners(v: SkillsView, table: SkillTable): BannerView[] {
  const out: BannerView[] = [];
  if (!v.npx)
    out.push({
      tone: 'warn',
      title: 'Không tìm thấy npx',
      body: 'Chưa cài hay cập nhật skill từ GitHub được. Bật, tắt và gom skill vẫn dùng được.',
      actions: [{ label: 'Cách cài Node', act: 'skills:npx' }],
    });
  const upd = table.rows.filter((r) => r.update);
  if (upd.length && v.npx)
    out.push({
      tone: 'info',
      title: upd.length + ' skill có bản mới',
      body: upd.map((r) => r.name).join(', ') + '. App không tự cập nhật: bạn xem trước rồi mới cập nhật.',
      actions: upd.slice(0, 2).map((r) => ({ label: 'Xem ' + r.name, act: `skills:update:${r.name}` as const })),
    });
  const u = v.updates;
  if (u.error)
    out.push({
      tone: 'warn',
      title: 'Chưa kiểm được bản mới',
      body: (u.error === 'rateLimited' ? 'GitHub tạm chặn vì hỏi nhiều quá.' : u.error === 'offline' ? 'Không kết nối được GitHub.' : 'GitHub trả lời ' + u.error + '.') + ' App thử lại sau một giờ.',
      actions: [{ label: 'Kiểm lại', act: 'skills:check' }],
    });
  else if (u.failed.length)
    out.push({ tone: 'warn', title: 'Chưa kiểm được ' + u.failed.join(', '), body: 'GitHub không trả lời cho repo này. Các repo khác đã kiểm.', actions: [] });
  if (table.lockError)
    out.push({ tone: 'warn', title: 'Không đọc được file khoá của npx skills', body: table.lockError + '. Skill nào cũng hiện là cài tay đến khi đọc được.', actions: [{ label: 'Đọc lại', act: 'skills:reread' }] });
  for (const c of table.columns) {
    if (c.state === 'invalid' || c.state === 'unreadable')
      out.push({
        tone: 'bad',
        title: 'Không đọc được ' + c.file + ' của ' + AGENT_NAME[c.agent],
        body: (c.reason ?? 'Không rõ lý do') + '. Cột ' + AGENT_NAME[c.agent] + ' chỉ xem; app không ghi vào file lỗi.',
        actions: [{ label: 'Đọc lại', act: 'skills:reread' }],
      });
  }
  return out;
}

/** "18 KB". */
export function sizeText(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return Math.round(bytes / 1024) + ' KB';
  return (bytes / 1024 / 1024).toFixed(1).replace('.', ',') + ' MB';
}

/** The agents a skill is on in, in words. */
export function onAt(row: SkillRow): string {
  const on = AGENTS.filter((a) => row.cells[a].state === 'on').map((a) => AGENT_NAME[a]);
  return on.length ? on.join(', ') : 'chưa bật ở agent nào';
}

/** "Mang theo" of a plugin: what it brings, in words. */
export function brings(p: Plugin): string {
  const b = p.brings;
  const parts: string[] = [];
  if (b.mcp) parts.push(b.mcp + ' MCP');
  if (b.skills.length) parts.push(b.skills.length + ' skill');
  if (b.commands) parts.push(b.commands + ' lệnh');
  if (b.agents) parts.push(b.agents + ' agent');
  if (b.hooks) parts.push('hook');
  return parts.length ? parts.join(', ') : '—';
}
