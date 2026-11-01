// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Skills tab's state (M11): the boxes ticked and not written yet, the
// filter, the open dialog, and the paths every change takes. Boxes go
// through the batch preview the Rust side reads from the folders now, then
// the write of what it showed, then the line with Hoàn tác, as the MCP tab's
// do. Installing, updating and removing for good go through `npx skills`
// on the Rust side; nothing here runs a skill.

import { useEffect, useRef, useState } from 'react';
import type { Api } from '../lib/api';
import { t } from '../lib/i18n';
import { AGENT_NAME, AGENTS, changesWord, clearColumn, original, setColumn, toggle, type Pending } from '../lib/mcp';
import type { Agent, Model, RepoList, SkillChange, SkillPreview, SkillRow } from '../lib/model';
import { bulkPicked, bulkUpdatedFeedback, npxText, skillChanges, skillRefusalText, skillWrittenFeedback, updateGroups, type SkillFilter } from '../lib/skills';
import type { Action, FeedbackView } from '../lib/view';

export interface SkillBatch {
  kind: 'batch';
  preview: SkillPreview;
  title: string;
  /** From the boxes: a write takes its changes off the pending bar. */
  ticks: boolean;
}

/** "Cài từ GitHub": the repository, the skills to pick, the preview, the run. */
export interface SkillGh {
  kind: 'gh';
  step: 'find' | 'pick' | 'preview';
  repo: string;
  error: string | null;
  list: RepoList | null;
  picked: string[];
  agents: Record<Agent, boolean>;
  /** "Đọc SKILL.md": the skill being read, and its text once it came. */
  reading: { name: string; text: string | null; error: string | null } | null;
}

/** "Cập nhật tất cả" or "Cập nhật cả bộ": the skills with a newer version, those ticked to update. */
export interface SkillUpdateMany {
  kind: 'updateMany';
  /** The repository of "Cập nhật cả bộ"; none for "Cập nhật tất cả". */
  repo: string | null;
  names: string[];
  picked: string[];
}

export type SkillDialog =
  | SkillBatch
  | SkillGh
  | { kind: 'import'; picked: string[] }
  | { kind: 'update'; name: string }
  | SkillUpdateMany
  | { kind: 'del'; name: string; mode: 'off' | 'hard' }
  | { kind: 'owned' }
  | { kind: 'broken'; agent: Agent; name: string }
  | { kind: 'npx' };

const failedPreview = (): FeedbackView => ({
  tone: 'bad',
  title: t('Chưa xem trước được', 'Could not preview'),
  body: t('App không đọc được thư mục skill lúc này. Bấm Đọc lại rồi thử lại.', 'The app cannot read the skill folders right now. Press Read again, then try again.'),
});

const busyFeedback = (): FeedbackView => ({
  tone: 'warn',
  title: t('Chưa ghi', 'Not written'),
  body: t('Một lần ghi khác hoặc lần chuyển Codex đang chạy. Đợi xong rồi thử lại.', 'Another write or Codex switch is running. Wait for it to finish, then try again.'),
});

export function useSkills(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const [pending, setPending] = useState<Pending>({});
  const [dialog, setDialog] = useState<SkillDialog | null>(null);
  const [filter, setFilter] = useState<SkillFilter>('all');
  const [working, setWorking] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);
  const flashTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const table = model?.skills.table ?? null;
  const rows = table?.rows ?? [];

  // A box the folders now have as ticked, or that can no longer be ticked, leaves the bar.
  useEffect(() => {
    if (!table) return;
    setPending((p) => {
      const next: Pending = {};
      for (const [k, on] of Object.entries(p)) {
        const name = k.slice(0, k.lastIndexOf('|'));
        const agent = k.slice(k.lastIndexOf('|') + 1) as Agent;
        const row = table.rows.find((r) => r.name === name);
        if (row && !row.cells[agent].lock && original(row.cells[agent]) !== on) next[k] = on;
      }
      return Object.keys(next).length === Object.keys(p).length ? p : next;
    });
  }, [table]);
  useEffect(() => () => clearTimeout(flashTimer.current), []);

  const lit = (name: string) => {
    clearTimeout(flashTimer.current);
    setFlash(name);
    flashTimer.current = setTimeout(() => setFlash(null), 1600);
  };

  const run = async <T>(call: () => Promise<T>): Promise<T | null> => {
    setWorking(true);
    try {
      return await call();
    } catch (e) {
      const said = npxText(String(e));
      showFeedback(String(e) === 'busy' ? busyFeedback() : { tone: 'bad', title: said.title, body: said.body });
      return null;
    } finally {
      setWorking(false);
    }
  };

  const preview = async (changes: SkillChange[], opts: { title?: string; ticks?: boolean }) => {
    setWorking(true);
    try {
      const p = await api.skillsPreview(changes);
      if (!p.changes.length) {
        setDialog(null);
        const why = p.refused.length
          ? p.refused.map((r) => t(r.name + ' cho ' + AGENT_NAME[r.agent] + ': ', r.name + ' for ' + AGENT_NAME[r.agent] + ': ') + skillRefusalText(r.reason)).join(' · ') + '.'
          : t('Đã như vậy rồi.', 'Already that way.');
        showFeedback({ tone: 'warn', title: t('Không có gì để ghi', 'Nothing to write'), body: why });
        return;
      }
      setDialog({ kind: 'batch', preview: p, title: opts.title ?? t('Ghi ' + changesWord(p.changes.length) + '?', 'Write ' + changesWord(p.changes.length) + '?'), ticks: !!opts.ticks });
    } catch {
      showFeedback(failedPreview());
    } finally {
      setWorking(false);
    }
  };

  const write = async () => {
    if (dialog?.kind !== 'batch') return;
    const d = dialog;
    setWorking(true);
    try {
      const out = await api.skillsWrite(d.preview.changes, d.preview.bases);
      const done = new Set(out.done.map((x) => x.agent));
      if (d.ticks)
        setPending((p) => {
          const next = { ...p };
          for (const c of d.preview.changes) if (done.has(c.agent)) delete next[c.name + '|' + c.agent];
          return next;
        });
      showFeedback(skillWrittenFeedback(out));
      if (out.done.length && d.preview.changes[0]) lit(d.preview.changes[0].name);
    } catch (e) {
      showFeedback(
        String(e) === 'busy'
          ? busyFeedback()
          : { tone: 'bad', title: t('Chưa ghi', 'Not written'), body: t('App không chạy được lần ghi này. Thư mục skill không đổi.', 'The app could not run this write. The skill folders are unchanged.') },
      );
    } finally {
      setWorking(false);
      setDialog(null);
    }
  };

  const undo = async () => {
    showFeedback(null);
    try {
      const r = await api.skillsUndo();
      showFeedback(
        r === 'undone'
          ? {
              tone: 'info',
              title: t('Đã hoàn tác lần vừa rồi.', 'Undid the last change.'),
              body: t('Skill và các ô tick trở về như trước. App lưu bản trước khi hoàn tác.', 'The skills and the ticks are back as they were. The app kept a backup from before the undo.'),
            }
          : {
              tone: 'warn',
              title: t('Chưa hoàn tác hết', 'Not fully undone'),
              body: t('Một phần chưa trả về được. Đọc lại để xem bây giờ thế nào.', 'Part of it could not be put back. Read again to see how it is now.'),
              actions: [{ label: t('Đọc lại', 'Read again'), act: 'skills:reread' }],
            },
      );
    } catch (e) {
      showFeedback(
        String(e) === 'nothing'
          ? {
              tone: 'warn',
              title: t('Không còn gì để hoàn tác', 'Nothing left to undo'),
              body: t('Lần vừa rồi đã được hoàn tác, hoặc app vừa mở lại.', 'The last change was already undone, or the app was just reopened.'),
            }
          : busyFeedback(),
      );
    }
  };

  const reread = () => {
    showFeedback(null);
    void api.skillsRead().catch(() => {});
  };

  const check = () => void api.skillsCheck().catch(() => {});

  // ===== Cài từ GitHub =====

  const allAgents = (): Record<Agent, boolean> =>
    Object.fromEntries(AGENTS.map((a) => [a, !!table?.columns.find((c) => c.agent === a)?.writable])) as Record<Agent, boolean>;

  const openGh = () => setDialog({ kind: 'gh', step: 'find', repo: '', error: null, list: null, picked: [], agents: allAgents(), reading: null });

  const findRepo = async (d: SkillGh) => {
    setWorking(true);
    try {
      const list = await api.skillsListRepo(d.repo);
      const first = list.skills.find((s) => !s.installed);
      setDialog({ ...d, step: 'pick', error: null, list, repo: list.repo, picked: first ? [first.name] : [] });
    } catch (e) {
      const said = npxText(String(e));
      setDialog({ ...d, error: said.title + '. ' + said.body });
    } finally {
      setWorking(false);
    }
  };

  const readMd = async (d: SkillGh, name: string) => {
    setDialog({ ...d, reading: { name, text: null, error: null } });
    try {
      const text = await api.skillsReadMd(d.repo, name);
      setDialog((cur) => (cur?.kind === 'gh' && cur.reading?.name === name ? { ...cur, reading: { name, text, error: null } } : cur));
    } catch (e) {
      const said = npxText(String(e));
      setDialog((cur) => (cur?.kind === 'gh' && cur.reading?.name === name ? { ...cur, reading: { name, text: null, error: said.title + '. ' + said.body } } : cur));
    }
  };

  const install = async (d: SkillGh) => {
    const out = await run(() => api.skillsInstall(d.repo, d.picked, d.agents));
    setDialog(null);
    if (!out) return;
    if (out.error && !out.names.length) {
      const said = npxText(out.error);
      return showFeedback({ tone: 'bad', title: t('Chưa cài: ', 'Not installed: ') + said.title, body: said.body });
    }
    const on = AGENTS.filter((a) => d.agents[a]).map((a) => AGENT_NAME[a]);
    const what = out.names.join(', ');
    if (out.outcome.failed) {
      const missing = d.picked.filter((n) => !out.names.includes(n));
      return showFeedback({
        tone: 'warn',
        title: t('Đã cài ' + what + ', nhưng chưa bật đủ agent.', 'Installed ' + what + ', but not on for every agent yet.'),
        body:
          (missing.length ? t('Chưa cài ' + missing.join(', ') + '. ', 'Not installed: ' + missing.join(', ') + '. ') : '') +
          t('Tick lại ô còn thiếu trong bảng.', 'Tick the missing boxes in the table again.'),
        actions: [{ label: t('Hoàn tác', 'Undo'), act: 'skills:undo' }],
      });
    }
    showFeedback({
      tone: 'info',
      title: t('Đã cài ' + what + '.', 'Installed ' + what + '.'),
      body: on.length
        ? t(
            'Từ ' + d.repo + '. Bật cho ' + on.join(', ') + '. Không chạy script nào của skill.',
            'From ' + d.repo + '. On for ' + on.join(', ') + '. No script of the skill was run.',
          )
        : t('Từ ' + d.repo + '. Chưa bật ở agent nào. Không chạy script nào của skill.', 'From ' + d.repo + '. Not on for any agent yet. No script of the skill was run.'),
      actions: [{ label: t('Hoàn tác', 'Undo'), act: 'skills:undo' }],
    });
    if (out.names[0]) lit(out.names[0]);
  };

  // ===== The other dialogs =====

  const gather = async (names: string[]) => {
    const g = await run(() => api.skillsImport(names));
    setDialog(null);
    if (!g) return;
    if (g.failed && !g.moved.length)
      return showFeedback({ tone: 'bad', title: t('Chưa gom được ' + g.failed[0], 'Could not gather ' + g.failed[0]), body: g.failed[1] + t('. Chưa chuyển gì.', '. Nothing was moved.') });
    const n = g.moved.length;
    showFeedback({
      tone: g.failed ? 'warn' : 'info',
      title: t('Đã gom ' + n + ' skill về thư mục chung.', 'Gathered ' + n + (n === 1 ? ' skill' : ' skills') + ' into the shared folder.'),
      body:
        (g.failed ? t('Chưa gom được ' + g.failed[0] + ': ' + g.failed[1] + '. ', 'Could not gather ' + g.failed[0] + ': ' + g.failed[1] + '. ') : '') +
        t('Agent nào đang bật vẫn bật. Bản cũ đã lưu lại.', 'Agents that had it on still do. The old copies are kept.'),
      actions: [{ label: t('Hoàn tác', 'Undo'), act: 'skills:undo' }],
    });
  };

  const update = async (name: string) => {
    const u = await run(() => api.skillsUpdate(name));
    setDialog(null);
    if (!u) return;
    if (u.error) {
      const said = npxText(u.error);
      return showFeedback({
        tone: 'bad',
        title: t('Chưa cập nhật ' + name + ': ', name + ' not updated: ') + said.title,
        body: said.body + (u.backup ? t(' Bản đang có đã lưu lại.', ' The current version is kept.') : ''),
      });
    }
    showFeedback({
      tone: 'info',
      title: t('Đã cập nhật ' + name + '.', 'Updated ' + name + '.'),
      body: t('Agent nào đang bật vẫn bật. Bản cũ đã lưu lại trong bản lưu của app.', 'Agents that had it on still do. The old version is kept in the app’s backups.'),
    });
    lit(name);
  };

  /** The preview of several updates: every skill with a newer version, or those of one repository. */
  const openUpdateMany = (repo: string | null) => {
    const names = repo ? (updateGroups(rows).find((g) => g.repo === repo)?.names ?? []) : rows.filter((r) => r.update).map((r) => r.name);
    setDialog({ kind: 'updateMany', repo, names, picked: bulkPicked(rows, names) });
  };

  const updateMany = async (names: string[]) => {
    const out = await run(() => api.skillsUpdateMany(names));
    setDialog(null);
    if (!out) return;
    showFeedback(bulkUpdatedFeedback(out));
    const first = out.results.find((r) => !r.error);
    if (first) lit(first.name);
  };

  const skip = async (name: string) => {
    await run(() => api.skillsSkip(name));
    setDialog(null);
    showFeedback({ tone: '', title: t('Đã bỏ qua bản mới của ' + name, 'Skipped the update of ' + name), body: t('App báo lại khi có bản sau nữa.', 'The app tells you again when a later one comes.') });
  };

  const remove = async (row: SkillRow, mode: 'off' | 'hard') => {
    if (mode === 'off') {
      const changes = AGENTS.filter((a) => row.cells[a].state === 'on' && !row.cells[a].lock).map((agent) => ({ name: row.name, agent, on: false }));
      return preview(changes, { title: t('Tắt ' + row.name + ' ở mọi agent?', 'Turn ' + row.name + ' off for every agent?') });
    }
    const r = await run(() => api.skillsRemove(row.name));
    setDialog(null);
    if (!r) return;
    if (r.error && !r.names.length) {
      const said = npxText(r.error);
      return showFeedback({ tone: 'bad', title: t('Chưa gỡ ' + row.name + ': ', row.name + ' not removed: ') + said.title, body: said.body });
    }
    showFeedback({
      tone: 'info',
      title: t('Đã gỡ hẳn ' + row.name + '.', 'Removed ' + row.name + ' for good.'),
      body: t('Thư mục đã lưu lại trong 10 bản lưu gần nhất của app.', 'The folder is kept in the app’s 10 most recent backups.'),
    });
  };

  const dropLink = async (agent: Agent, name: string) => {
    const r = await run(() => api.skillsDropLink(agent, name).then(() => true));
    setDialog(null);
    if (r)
      showFeedback({
        tone: '',
        title: t('Đã bỏ liên kết ' + name, 'Removed the link ' + name),
        body: t('Chỉ bỏ liên kết ở ' + AGENT_NAME[agent] + ', không xoá thư mục nào.', 'Only the link in ' + AGENT_NAME[agent] + ' was removed; no folder was deleted.'),
      });
  };

  /** The Skills tab's actions on banners and feedback lines. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'skills:undo') void undo();
    else if (a === 'skills:reread') reread();
    else if (a === 'skills:check') check();
    else if (a === 'skills:npx') setDialog({ kind: 'npx' });
    else if (a.startsWith('skills:update:')) setDialog({ kind: 'update', name: a.slice('skills:update:'.length) });
    else return false;
    return true;
  };

  const close = () => {
    if (!working) setDialog(null);
  };

  return {
    table,
    pending,
    dialog,
    filter,
    working: working || (model?.skills.busy != null && model.skills.busy !== 'reading'),
    flash,
    setDialog,
    setFilter,
    close,
    act,
    reread,
    check,
    toggle: (row: SkillRow, agent: Agent) => setPending((p) => toggle(p, row, agent)),
    setColumn: (agent: Agent, on: boolean, list: SkillRow[]) => setPending((p) => setColumn(p, list, agent, on)),
    clearColumn: (agent: Agent) => setPending((p) => clearColumn(p, agent)),
    clearPending: () => setPending({}),
    previewPending: () => void preview(skillChanges(pending, rows), { ticks: true }),
    write: () => void write(),
    openGh,
    findRepo: (d: SkillGh) => void findRepo(d),
    readMd: (d: SkillGh, name: string) => void readMd(d, name),
    install: (d: SkillGh) => void install(d),
    gather: (names: string[]) => void gather(names),
    update: (name: string) => void update(name),
    openUpdateMany,
    updateMany: (names: string[]) => void updateMany(names),
    skip: (name: string) => void skip(name),
    remove: (row: SkillRow, mode: 'off' | 'hard') => void remove(row, mode),
    dropLink: (agent: Agent, name: string) => void dropLink(agent, name),
    openLink: (url: string) => void api.skillsOpenLink(url).catch(() => {}),
  };
}

export type Skills = ReturnType<typeof useSkills>;
