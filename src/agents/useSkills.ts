// The Skills tab's state (M11): the boxes ticked and not written yet, the
// filter, the open dialog, and the paths every change takes. Boxes go
// through the batch preview the Rust side reads from the folders now, then
// the write of what it showed, then the line with Hoàn tác, as the MCP tab's
// do. Installing, updating and removing for good go through `npx skills`
// on the Rust side; nothing here runs a skill.

import { useEffect, useRef, useState } from 'react';
import type { Api } from '../lib/api';
import { AGENT_NAME, AGENTS, clearColumn, original, setColumn, toggle, type Pending } from '../lib/mcp';
import type { Agent, Model, RepoList, SkillChange, SkillPreview, SkillRow } from '../lib/model';
import { npxText, skillChanges, skillRefusalText, skillWrittenFeedback, type SkillFilter } from '../lib/skills';
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

export type SkillDialog =
  | SkillBatch
  | SkillGh
  | { kind: 'import'; picked: string[] }
  | { kind: 'update'; name: string }
  | { kind: 'del'; name: string; mode: 'off' | 'hard' }
  | { kind: 'owned' }
  | { kind: 'broken'; agent: Agent; name: string }
  | { kind: 'npx' };

const failedPreview: FeedbackView = { tone: 'bad', title: 'Chưa xem trước được', body: 'App không đọc được thư mục skill lúc này. Bấm Đọc lại rồi thử lại.' };

const busyFeedback: FeedbackView = { tone: 'warn', title: 'Chưa ghi', body: 'Một lần ghi khác hoặc lần chuyển Codex đang chạy. Đợi xong rồi thử lại.' };

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
      const t = npxText(String(e));
      showFeedback(String(e) === 'busy' ? busyFeedback : { tone: 'bad', title: t.title, body: t.body });
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
        const why = p.refused.length ? p.refused.map((r) => r.name + ' cho ' + AGENT_NAME[r.agent] + ': ' + skillRefusalText(r.reason)).join(' · ') + '.' : 'Đã như vậy rồi.';
        showFeedback({ tone: 'warn', title: 'Không có gì để ghi', body: why });
        return;
      }
      setDialog({ kind: 'batch', preview: p, title: opts.title ?? 'Ghi ' + p.changes.length + ' thay đổi?', ticks: !!opts.ticks });
    } catch {
      showFeedback(failedPreview);
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
      showFeedback(String(e) === 'busy' ? busyFeedback : { tone: 'bad', title: 'Chưa ghi', body: 'App không chạy được lần ghi này. Thư mục skill không đổi.' });
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
          ? { tone: 'info', title: 'Đã hoàn tác lần vừa rồi.', body: 'Skill và các ô tick trở về như trước. App lưu bản trước khi hoàn tác.' }
          : { tone: 'warn', title: 'Chưa hoàn tác hết', body: 'Một phần chưa trả về được. Đọc lại để xem bây giờ thế nào.', actions: [{ label: 'Đọc lại', act: 'skills:reread' }] },
      );
    } catch (e) {
      showFeedback(
        String(e) === 'nothing'
          ? { tone: 'warn', title: 'Không còn gì để hoàn tác', body: 'Lần vừa rồi đã được hoàn tác, hoặc app vừa mở lại.' }
          : busyFeedback,
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
      const t = npxText(String(e));
      setDialog({ ...d, error: t.title + '. ' + t.body });
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
      const t = npxText(String(e));
      setDialog((cur) => (cur?.kind === 'gh' && cur.reading?.name === name ? { ...cur, reading: { name, text: null, error: t.title + '. ' + t.body } } : cur));
    }
  };

  const install = async (d: SkillGh) => {
    const out = await run(() => api.skillsInstall(d.repo, d.picked, d.agents));
    setDialog(null);
    if (!out) return;
    if (out.error && !out.names.length) {
      const t = npxText(out.error);
      return showFeedback({ tone: 'bad', title: 'Chưa cài: ' + t.title, body: t.body });
    }
    const on = AGENTS.filter((a) => d.agents[a]).map((a) => AGENT_NAME[a]);
    const what = out.names.join(', ');
    if (out.outcome.failed) {
      const missing = d.picked.filter((n) => !out.names.includes(n));
      return showFeedback({
        tone: 'warn',
        title: 'Đã cài ' + what + ', nhưng chưa bật đủ agent.',
        body: (missing.length ? 'Chưa cài ' + missing.join(', ') + '. ' : '') + 'Tick lại ô còn thiếu trong bảng.',
        actions: [{ label: 'Hoàn tác', act: 'skills:undo' }],
      });
    }
    showFeedback({
      tone: 'info',
      title: 'Đã cài ' + what + '.',
      body: 'Từ ' + d.repo + (on.length ? '. Bật cho ' + on.join(', ') : '. Chưa bật ở agent nào') + '. Không chạy script nào của skill.',
      actions: [{ label: 'Hoàn tác', act: 'skills:undo' }],
    });
    if (out.names[0]) lit(out.names[0]);
  };

  // ===== The other dialogs =====

  const gather = async (names: string[]) => {
    const g = await run(() => api.skillsImport(names));
    setDialog(null);
    if (!g) return;
    if (g.failed && !g.moved.length) return showFeedback({ tone: 'bad', title: 'Chưa gom được ' + g.failed[0], body: g.failed[1] + '. Chưa chuyển gì.' });
    showFeedback({
      tone: g.failed ? 'warn' : 'info',
      title: 'Đã gom ' + g.moved.length + ' skill về thư mục chung.',
      body: (g.failed ? 'Chưa gom được ' + g.failed[0] + ': ' + g.failed[1] + '. ' : '') + 'Agent nào đang bật vẫn bật. Bản cũ đã lưu lại.',
      actions: [{ label: 'Hoàn tác', act: 'skills:undo' }],
    });
  };

  const update = async (name: string) => {
    const u = await run(() => api.skillsUpdate(name));
    setDialog(null);
    if (!u) return;
    if (u.error) {
      const t = npxText(u.error);
      return showFeedback({ tone: 'bad', title: 'Chưa cập nhật ' + name + ': ' + t.title, body: t.body + (u.backup ? ' Bản đang có đã lưu lại.' : '') });
    }
    showFeedback({ tone: 'info', title: 'Đã cập nhật ' + name + '.', body: 'Agent nào đang bật vẫn bật. Bản cũ đã lưu lại trong bản lưu của app.' });
    lit(name);
  };

  const skip = async (name: string) => {
    await run(() => api.skillsSkip(name));
    setDialog(null);
    showFeedback({ tone: '', title: 'Đã bỏ qua bản mới của ' + name, body: 'App báo lại khi có bản sau nữa.' });
  };

  const remove = async (row: SkillRow, mode: 'off' | 'hard') => {
    if (mode === 'off') {
      const changes = AGENTS.filter((a) => row.cells[a].state === 'on' && !row.cells[a].lock).map((agent) => ({ name: row.name, agent, on: false }));
      return preview(changes, { title: 'Tắt ' + row.name + ' ở mọi agent?' });
    }
    const r = await run(() => api.skillsRemove(row.name));
    setDialog(null);
    if (!r) return;
    if (r.error && !r.names.length) {
      const t = npxText(r.error);
      return showFeedback({ tone: 'bad', title: 'Chưa gỡ ' + row.name + ': ' + t.title, body: t.body });
    }
    showFeedback({ tone: 'info', title: 'Đã gỡ hẳn ' + row.name + '.', body: 'Thư mục đã lưu lại trong 10 bản lưu gần nhất của app.' });
  };

  const dropLink = async (agent: Agent, name: string) => {
    const r = await run(() => api.skillsDropLink(agent, name).then(() => true));
    setDialog(null);
    if (r) showFeedback({ tone: '', title: 'Đã bỏ liên kết ' + name, body: 'Chỉ bỏ liên kết ở ' + AGENT_NAME[agent] + ', không xoá thư mục nào.' });
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
    skip: (name: string) => void skip(name),
    remove: (row: SkillRow, mode: 'off' | 'hard') => void remove(row, mode),
    dropLink: (agent: Agent, name: string) => void dropLink(agent, name),
    openLink: (url: string) => void api.skillsOpenLink(url).catch(() => {}),
  };
}

export type Skills = ReturnType<typeof useSkills>;
