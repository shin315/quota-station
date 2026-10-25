// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Skills, Plugin and File hướng dẫn tabs' words (M11), pure so Vitest
// covers them without a window: the filters, the line under each name, the
// chips, why a box is locked, the batch's lead line, and the lines after a
// write, an install or a check. The boxes themselves use `mcp.ts`'s helpers.
// Copy follows the approved phase 2 prototype.

import { clock } from './format';
import { t } from './i18n';
import { AGENT_NAME, AGENTS, changesWord, pendingList, type Pending, type PendingItem } from './mcp';
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

/** The filters, in the order the row of buttons shows them. */
export const FILTERS: SkillFilter[] = ['all', 'github', 'hand', 'update'];

export function filterWord(f: SkillFilter): string {
  switch (f) {
    case 'all':
      return t('Tất cả', 'All');
    case 'github':
      return t('Từ GitHub', 'From GitHub');
    case 'hand':
      return t('Cài tay', 'By hand');
    case 'update':
      return t('Có bản mới', 'Update available');
  }
}

/** "3 skills", "1 command": English's plural. */
const many = (n: number, word: string): string => n + ' ' + word + (n === 1 ? '' : 's');

/** Under the name: where it comes from. */
export function sourceLine(row: SkillRow): string {
  if (row.source === 'github') return 'GitHub · ' + row.repo;
  if (row.source === 'other') return t('nguồn khác · ', 'other source · ') + row.repo;
  return row.home
    ? t('cài tay · thư mục của ' + AGENT_NAME[row.home], 'installed by hand · ' + AGENT_NAME[row.home] + '’s folder')
    : t('cài tay', 'installed by hand');
}

/** Chips only when something is the matter, or new. */
export function skillChips(row: SkillRow): ChipView[] {
  const out: ChipView[] = [];
  if (row.update) out.push({ t: t('có bản mới', 'update available'), k: 'info', title: row.update.plain + t('. App không tự cập nhật.', '. The app does not update on its own.') });
  if (row.edited)
    out.push({
      t: t('bạn đã sửa', 'edited by you'),
      k: 'warn',
      title: t('Thư mục khác bản đã cài. Cập nhật sẽ thay phần bạn sửa.', 'The folder differs from the installed version. Updating replaces your edits.'),
    });
  if (Object.values(row.cells).some((c) => c.lock === 'taken'))
    out.push({
      t: t('trùng tên', 'name clash'),
      k: 'warn',
      title: t('Một agent có thư mục riêng cùng tên, nên app không đổi ô của agent đó.', 'An agent has its own folder with the same name, so the app leaves that agent’s box alone.'),
    });
  return out;
}

export function skillLockText(lock: SkillLock): string {
  switch (lock) {
    case 'readOnly':
      return t('cột này chỉ xem', 'this column is view only');
    case 'hand':
      return t('nằm ở thư mục riêng của agent, gom về một chỗ để bật tắt', 'in the agent’s own folder; gather it into one place to turn it on or off');
    case 'taken':
      return t('agent có thư mục riêng cùng tên, app để yên', 'the agent has its own folder with this name, left alone');
    case 'broken':
      return t('liên kết hỏng', 'broken link');
  }
}

/** The box's hover text. */
export function skillCellTitle(row: SkillRow, agent: Agent): string {
  const c = row.cells[agent];
  if (c.lock) return row.name + ': ' + skillLockText(c.lock);
  if (agent === 'codex') return t('Codex đọc thẳng thư mục chung. Tắt là một mục trong config.toml.', 'Codex reads the shared folder directly. Off is an entry in config.toml.');
  return '';
}

export function skillRefusalText(reason: SkillRefusal): string {
  if (reason === 'badName') return t('tên không hợp lệ', 'invalid name');
  if (reason === 'unknown') return t('không còn trong thư mục chung', 'no longer in the shared folder');
  if (reason === 'nothing') return t('đã như vậy rồi', 'already that way');
  return skillLockText(reason);
}

export const pendingSkillText = (p: PendingItem): string => {
  const a = AGENT_NAME[p.agent];
  return p.on ? t(p.name + ': bật cho ' + a, p.name + ': turn on for ' + a) : t(p.name + ': tắt cho ' + a, p.name + ': turn off for ' + a);
};

export function skillChanges(pending: Pending, rows: SkillRow[]): SkillChange[] {
  return pendingList(pending, rows).map((p) => ({ name: p.name, agent: p.agent, on: p.on }));
}

/** What each column holds, in the batch preview's words. */
export function groupFile(agent: Agent): string {
  return agent === 'codex' ? 'config.toml' : t('liên kết của ' + AGENT_NAME[agent], AGENT_NAME[agent] + '’s links');
}

function names(list: string[]): string {
  return list.length <= 1 ? list.join('') : list.slice(0, -1).join(', ') + t(' và ', ' and ') + list[list.length - 1];
}

export function skillFailureText(f: SkillFailure, agent: Agent): { title: string; body: string } {
  const file = groupFile(agent);
  switch (f.kind) {
    case 'changed':
      return {
        title: t('Chưa đổi gì: ' + file + ' vừa bị chương trình khác đổi.', 'Nothing changed: another program just changed ' + file + '.'),
        body: t('Đọc lại rồi thử lại.', 'Read again, then try again.'),
      };
    case 'refused':
      return {
        title: t('Chưa đổi gì: danh sách đã khác lúc xem trước.', 'Nothing changed: the list changed since the preview.'),
        body:
          f.refused.map((r) => t(r.name + ' cho ' + AGENT_NAME[r.agent] + ': ', r.name + ' for ' + AGENT_NAME[r.agent] + ': ') + skillRefusalText(r.reason)).join(' · ') +
          t('. Đọc lại rồi xem trước lần nữa.', '. Read again, then preview again.'),
      };
    case 'edit':
      return { title: t('Chưa ghi ' + file + '.', file + ' was not written.'), body: t('App không sửa được đúng mục đó, nên không ghi gì.', 'The app could not edit that exact entry, so nothing was written.') };
    case 'write':
      return {
        title: t('Chưa ghi ' + file + '.', file + ' was not written.'),
        body: t('Lỗi khi ghi: ' + f.detail + '. Phần đang dùng không đổi.', 'Write error: ' + f.detail + '. What is in use is unchanged.'),
      };
  }
}

/** The line after a batch: what was written, its backups, and the undo. */
export function skillWrittenFeedback(out: SkillOutcome): FeedbackView {
  const n = out.done.reduce((sum, d) => sum + d.changes, 0);
  const at = clock(new Date(out.at * 1000).toISOString());
  const files = names(out.done.filter((d) => d.backup).map((d) => groupFile(d.agent)));
  const kept = files ? t('Đã sao lưu ' + files + ' lúc ' + at + '.', 'Backed up ' + files + ' at ' + at + '.') : '';
  const failed = out.failed ? skillFailureText(out.failed.error, out.failed.agent) : null;
  const actions = out.undo ? [{ label: t('Hoàn tác', 'Undo'), act: 'skills:undo' as const }] : [];
  if (failed && !n) return { tone: 'bad', title: failed.title, body: failed.body, actions: [{ label: t('Đọc lại', 'Read again'), act: 'skills:reread' }] };
  if (failed)
    return {
      tone: 'warn',
      title: t('Đã ghi ' + changesWord(n) + ', phần còn lại chưa ghi.', 'Wrote ' + changesWord(n) + '; the rest were not written.'),
      body: failed.title + ' ' + failed.body + (kept ? ' ' + kept : ''),
      actions,
    };
  return { tone: 'info', title: t('Đã ghi ' + changesWord(n) + '.', 'Wrote ' + changesWord(n) + '.'), body: (kept ? kept + ' ' : '') + t('Mở lại agent để dùng.', 'Reopen the agent to use it.'), actions };
}

/** What `npx skills` said, in words: a code from the Rust side, or its error. */
export function npxText(e: NpxError | string): { title: string; body: string } {
  const code = typeof e === 'string' ? e : e.kind === 'failed' ? 'failed:' + e.detail : e.kind;
  if (code === 'noNpx')
    return {
      title: t('Không tìm thấy npx', 'npx not found'),
      body: t('Cài Node.js rồi bấm Đọc lại. Bật, tắt và gom skill vẫn dùng được.', 'Install Node.js, then press Read again. Turning skills on and off and gathering them still work.'),
    };
  if (code === 'badRepo')
    return { title: t('Không phải repo GitHub', 'Not a GitHub repo'), body: t('Gõ dạng chủ/tên-repo, hoặc dán link GitHub của repo.', 'Type owner/repo-name, or paste the repo’s GitHub link.') };
  if (code === 'noSkills')
    return { title: t('Repo này không có skill nào', 'This repo has no skills'), body: t('Không thấy thư mục nào có SKILL.md trong repo.', 'No folder in the repo has a SKILL.md.') };
  if (code === 'timeout') return { title: t('npx skills chạy quá lâu', 'npx skills took too long'), body: t('Kiểm mạng rồi thử lại.', 'Check the network, then try again.') };
  if (code === 'busy') return { title: t('Đang có một lần ghi khác', 'Another write is running'), body: t('Đợi xong rồi thử lại.', 'Wait for it to finish, then try again.') };
  if (code.startsWith('failed:')) return { title: t('npx skills báo lỗi', 'npx skills reported an error'), body: code.slice('failed:'.length) || t('Không rõ lý do.', 'Unknown reason.') };
  return { title: t('Chưa làm được', 'Could not do it'), body: code };
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
  const why = failed.map((r) => r.name + ' (' + (r.error!.kind === 'failed' ? r.error!.detail || t('không rõ lý do', 'unknown reason') : npxText(r.error!).title.toLowerCase()) + ')').join(', ');
  const body = [
    failed.length ? t('Chưa cập nhật ' + why + '.', 'Not updated: ' + why + '.') : '',
    done.length ? t('Agent nào đang bật vẫn bật.', 'Agents that had it on still do.') : '',
    out.results.some((r) => r.backup) ? t('Bản cũ đã lưu ở ' + out.backups + '.', 'The old versions are saved in ' + out.backups + '.') : '',
  ]
    .filter(Boolean)
    .join(' ');
  if (!done.length) return { tone: 'bad', title: t('Chưa cập nhật skill nào.', 'No skills were updated.'), body };
  if (!failed.length) return { tone: 'info', title: t('Đã cập nhật ' + done.length + ' skill.', 'Updated ' + many(done.length, 'skill') + '.'), body };
  return {
    tone: 'warn',
    title: t(
      'Đã cập nhật ' + done.length + ' skill, ' + failed.length + ' skill chưa cập nhật.',
      'Updated ' + many(done.length, 'skill') + '; ' + failed.length + (failed.length === 1 ? ' was' : ' were') + ' not updated.',
    ),
    body,
  };
}

/** "kiểm bản mới lúc 09:12", in the title row. */
export function checkedLine(v: SkillsView): string {
  const u = v.updates;
  if (u.checking) return t('đang kiểm bản mới…', 'checking for updates…');
  if (!u.settings.daily && !u.checkedAt) return t('không tự kiểm bản mới', 'no automatic update check');
  return u.checkedAt ? t('kiểm bản mới lúc ' + clock(u.checkedAt), 'checked for updates at ' + clock(u.checkedAt)) : t('chưa kiểm bản mới', 'not checked for updates yet');
}

/**
 * The banners above the table: npx missing, a check that failed, the lock
 * file, a column. The newer versions have their own card (SkillsTab).
 */
export function skillBanners(v: SkillsView, table: SkillTable): BannerView[] {
  const out: BannerView[] = [];
  if (!v.npx)
    out.push({
      tone: 'warn',
      title: t('Không tìm thấy npx', 'npx not found'),
      body: t(
        'Chưa cài hay cập nhật skill từ GitHub được. Bật, tắt và gom skill vẫn dùng được.',
        'Skills from GitHub cannot be installed or updated for now. Turning skills on and off and gathering them still work.',
      ),
      actions: [{ label: t('Cách cài Node', 'How to install Node'), act: 'skills:npx' }],
    });
  const u = v.updates;
  if (u.error)
    out.push({
      tone: 'warn',
      title: t('Chưa kiểm được bản mới', 'Could not check for updates'),
      body:
        (u.error === 'rateLimited'
          ? t('GitHub tạm chặn vì hỏi nhiều quá.', 'GitHub is briefly blocking the app for asking too often.')
          : u.error === 'offline'
            ? t('Không kết nối được GitHub.', 'Could not reach GitHub.')
            : t('GitHub trả lời ' + u.error + '.', 'GitHub answered ' + u.error + '.')) + t(' App thử lại sau một giờ.', ' The app tries again in an hour.'),
      actions: [{ label: t('Kiểm lại', 'Check again'), act: 'skills:check' }],
    });
  else if (u.failed.length)
    out.push({
      tone: 'warn',
      title: t('Chưa kiểm được ' + u.failed.join(', '), 'Could not check ' + u.failed.join(', ')),
      body: t('GitHub không trả lời cho repo này. Các repo khác đã kiểm.', 'GitHub did not answer for this repo. The other repos were checked.'),
      actions: [],
    });
  if (table.lockError)
    out.push({
      tone: 'warn',
      title: t('Không đọc được file khoá của npx skills', 'Could not read the npx skills lock file'),
      body: table.lockError + t('. Skill nào cũng hiện là cài tay đến khi đọc được.', '. Every skill shows as installed by hand until it can be read.'),
      actions: [{ label: t('Đọc lại', 'Read again'), act: 'skills:reread' }],
    });
  for (const c of table.columns) {
    if (c.state === 'invalid' || c.state === 'unreadable')
      out.push({
        tone: 'bad',
        title: t('Không đọc được ' + c.file + ' của ' + AGENT_NAME[c.agent], 'Could not read ' + AGENT_NAME[c.agent] + '’s ' + c.file),
        body:
          (c.reason ?? t('Không rõ lý do', 'Unknown reason')) +
          t('. Cột ' + AGENT_NAME[c.agent] + ' chỉ xem; app không ghi vào file lỗi.', '. The ' + AGENT_NAME[c.agent] + ' column is view only; the app does not write to a broken file.'),
        actions: [{ label: t('Đọc lại', 'Read again'), act: 'skills:reread' }],
      });
  }
  return out;
}

/** "18 KB". */
export function sizeText(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return Math.round(bytes / 1024) + ' KB';
  const mb = (bytes / 1024 / 1024).toFixed(1);
  return t(mb.replace('.', ','), mb) + ' MB';
}

/** The agents a skill is on in, in words. */
export function onAt(row: SkillRow): string {
  const on = AGENTS.filter((a) => row.cells[a].state === 'on').map((a) => AGENT_NAME[a]);
  return on.length ? on.join(', ') : t('chưa bật ở agent nào', 'not on for any agent');
}

/** "Mang theo" of a plugin: what it brings, in words. */
export function brings(p: Plugin): string {
  const b = p.brings;
  const parts: string[] = [];
  if (b.mcp) parts.push(b.mcp + ' MCP');
  if (b.skills.length) parts.push(t(b.skills.length + ' skill', many(b.skills.length, 'skill')));
  if (b.commands) parts.push(t(b.commands + ' lệnh', many(b.commands, 'command')));
  if (b.agents) parts.push(t(b.agents + ' agent', many(b.agents, 'agent')));
  if (b.hooks) parts.push(t('hook', 'hooks'));
  return parts.length ? parts.join(', ') : '—';
}
