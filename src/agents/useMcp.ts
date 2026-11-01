// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The MCP tab's state (M9): the boxes ticked and not written yet, the open
// dialog, and the one path every write takes: changes, then the batch
// preview the Rust side reads from the files now, then the write of what
// the preview showed, then the line with Hoàn tác. The window never holds
// a secret past the form it was typed in: it goes to the Keychain first.

import { useEffect, useRef, useState } from 'react';
import type { Api } from '../lib/api';
import { os, secretStore, t } from '../lib/i18n';
import {
  AGENTS,
  AGENT_NAME,
  changesOf,
  changesWord,
  clearColumn,
  emptyForm,
  formDef,
  formError,
  formOf,
  original,
  pairSlot,
  refusalText,
  rowAll,
  setColumn,
  toggle,
  typedSlots,
  undoneFeedback,
  writtenFeedback,
  type McpForm,
  type Pending,
} from '../lib/mcp';
import type { Agent, McpChange, McpPaste, McpPreview, McpRow, Model } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';

export interface FormDialog {
  kind: 'form';
  form: McpForm;
  mode: 'form' | 'paste';
  /** The pasted text, until its secrets are in the Keychain. */
  text: string;
  /** What the paste holds, masked. */
  found: McpPaste | null;
  /** The paste once its secrets went to the Keychain. */
  saved: McpPaste | null;
  /** The agent whose definition the edit form shows: its file fills the secrets kept. */
  from: Agent | null;
  /** Slots this dialog already put in the Keychain. */
  typed: string[];
  error: string | null;
}

export interface BatchDialog {
  kind: 'batch';
  preview: McpPreview;
  title: string;
  /** "Điền › Xem trước" over the preview of an add. */
  steps: boolean;
  /** Where "Quay lại" goes; none closes. */
  back: McpDialog | null;
  /** From the boxes: a write takes its changes off the pending bar. */
  ticks: boolean;
  /** "Xoá hẳn" with its keys: once written, out of the store and the Keychain. */
  forget?: { name: string; slots: string[] };
}

export type McpDialog =
  | FormDialog
  | BatchDialog
  | { kind: 'compare'; name: string; pick: string }
  | { kind: 'del'; name: string; mode: 'kho' | 'hard'; keys: boolean }
  | { kind: 'kho'; name: string; agents: Record<Agent, boolean> }
  | { kind: 'forget'; name: string; keys: boolean }
  | { kind: 'key'; name: string }
  | { kind: 'howCmd'; name: string }
  | { kind: 'owned' }
  | { kind: 'backups' };

const failedPreview = (): FeedbackView => ({
  tone: 'bad',
  title: t('Chưa xem trước được', 'Could not preview'),
  body: t('App không đọc được file của agent lúc này. Bấm Đọc lại rồi thử lại.', 'The app cannot read the agents’ files right now. Press Read again, then try again.'),
});

function refusalsLine(p: McpPreview): string {
  return p.refused.map((r) => t(r.name + ' cho ' + AGENT_NAME[r.agent] + ': ', r.name + ' for ' + AGENT_NAME[r.agent] + ': ') + refusalText(r.reason)).join(' · ');
}

/** The agents a server is in, in a way the edit form can write: Claude Code's kept one waits until it is on again. */
export function editable(row: McpRow): Agent[] {
  return AGENTS.filter((a) => row.defs[a] && row.cells[a].state !== 'absent' && !row.cells[a].parked && !row.cells[a].lock);
}

export function useMcp(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const [pending, setPending] = useState<Pending>({});
  const [dialog, setDialog] = useState<McpDialog | null>(null);
  const [working, setWorking] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);
  const [later, setLater] = useState<string[]>([]);
  const flashTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const table = model?.mcp.table ?? null;
  const rows = table?.rows ?? [];

  // A box the files now have as ticked, or that can no longer be ticked, leaves the bar.
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

  /** The batch preview of `changes`; nothing to write says why, where it was asked. */
  const preview = async (changes: McpChange[], opts: { title?: string; back?: McpDialog | null; ticks?: boolean; steps?: boolean; forget?: BatchDialog['forget'] }) => {
    setWorking(true);
    try {
      const p = await api.mcpPreview(changes);
      if (!p.changes.length) {
        const why = p.refused.length ? refusalsLine(p) + '.' : t('Các file đã như vậy rồi.', 'The files are already that way.');
        if (opts.back?.kind === 'form') setDialog({ ...opts.back, error: t('Không có gì để ghi: ', 'Nothing to write: ') + why });
        else {
          setDialog(opts.back ?? null);
          showFeedback({ tone: 'warn', title: t('Không có gì để ghi', 'Nothing to write'), body: why });
        }
        return;
      }
      setDialog({
        kind: 'batch',
        preview: p,
        title: opts.title ?? t('Ghi ' + changesWord(p.changes.length) + '?', 'Write ' + changesWord(p.changes.length) + '?'),
        steps: !!opts.steps,
        back: opts.back ?? null,
        ticks: !!opts.ticks,
        forget: opts.forget,
      });
    } catch {
      if (opts.back?.kind === 'form') setDialog({ ...opts.back, error: failedPreview().body });
      else showFeedback(failedPreview());
    } finally {
      setWorking(false);
    }
  };

  const write = async () => {
    if (dialog?.kind !== 'batch') return;
    const d = dialog;
    setWorking(true);
    try {
      const out = await api.mcpWrite(d.preview.changes, d.preview.bases);
      const done = new Set(out.done.map((x) => x.agent));
      if (d.ticks)
        setPending((p) => {
          const next = { ...p };
          for (const c of d.preview.changes) if (done.has(c.agent)) delete next[c.name + '|' + c.agent];
          return next;
        });
      if (d.forget && !out.failed) await api.mcpForget(d.forget.name, d.forget.slots, true).catch(() => {});
      showFeedback(writtenFeedback(out, true));
      if (out.done.length && d.preview.changes[0]) lit(d.preview.changes[0].name);
    } catch (e) {
      showFeedback(
        String(e) === 'busy'
          ? {
              tone: 'warn',
              title: t('Chưa ghi', 'Not written'),
              body: t('Một lần ghi khác hoặc lần chuyển Codex đang chạy. Đợi xong rồi thử lại.', 'Another write or Codex switch is running. Wait for it to finish, then try again.'),
            }
          : { tone: 'bad', title: t('Chưa ghi', 'Not written'), body: t('App không chạy được lần ghi này. Các file không đổi.', 'The app could not run this write. The files are unchanged.') },
      );
    } finally {
      setWorking(false);
      setDialog(null);
    }
  };

  const undo = async () => {
    showFeedback(null);
    try {
      showFeedback(undoneFeedback(await api.mcpUndo()));
    } catch (e) {
      showFeedback(
        String(e) === 'nothing'
          ? {
              tone: 'warn',
              title: t('Không còn gì để hoàn tác', 'Nothing left to undo'),
              body: t('Lần ghi vừa rồi đã được hoàn tác, hoặc app vừa mở lại.', 'The last write was already undone, or the app was just reopened.'),
            }
          : { tone: 'warn', title: t('Chưa hoàn tác', 'Not undone'), body: t('Một lần ghi khác đang chạy. Đợi xong rồi thử lại.', 'Another write is running. Wait for it to finish, then try again.') },
      );
    }
  };

  const openForm = (row: McpRow | null) => {
    if (!table) return;
    if (!row) {
      setDialog({ kind: 'form', form: emptyForm(table.columns), mode: 'form', text: '', found: null, saved: null, from: null, typed: [], error: null });
      return;
    }
    const agents = editable(row);
    const from = agents[0] ?? AGENTS.find((a) => row.defs[a]) ?? null;
    const view = from ? row.defs[from] : undefined;
    if (!from || !view) return;
    const ticks = Object.fromEntries(AGENTS.map((a) => [a, agents.includes(a)])) as Record<Agent, boolean>;
    setDialog({ kind: 'form', form: formOf(row.name, view.def, view.slots, ticks), mode: 'form', text: '', found: null, saved: null, from, typed: [], error: null });
  };

  const submitForm = async (d: FormDialog) => {
    const f = d.form;
    const bitwarden = f.bitwarden.trim() || null;
    const agents = AGENTS.filter((a) => f.agents[a]);
    if (!agents.length) return setDialog({ ...d, error: t('Chọn ít nhất một agent.', 'Pick at least one agent.') });
    if (d.mode === 'paste') {
      let saved = d.saved;
      if (!saved) {
        setWorking(true);
        const res = await api.mcpPaste(d.text, true, bitwarden).catch(() => null);
        setWorking(false);
        if (!res || res.error || !res.servers.length) return setDialog({ ...d, found: res, error: res ? null : t('Không đọc được đoạn đã dán.', 'Could not read the pasted text.') });
        saved = res;
      }
      // The text, secrets and all, leaves the window: the Keychain has them now.
      const back: FormDialog = { ...d, text: '', found: saved, saved, error: null };
      const changes: McpChange[] = saved.servers.flatMap((s) =>
        agents.map((agent) => ({ name: s.name, agent, to: { kind: 'def' as const, def: s.def, from: null, keychain: s.slots.map((x) => x.id), off: false } })),
      );
      const title =
        saved.servers.length === 1
          ? t('Thêm ' + saved.servers[0].name + '?', 'Add ' + saved.servers[0].name + '?')
          : t('Thêm ' + saved.servers.length + ' MCP?', 'Add ' + saved.servers.length + ' MCP servers?');
      return preview(changes, { title, back, steps: true });
    }
    const taken = rows.map((r) => r.name);
    const error = formError(f, taken);
    if (error) return setDialog({ ...d, error });
    setWorking(true);
    try {
      for (const p of f.pairs.filter((x) => x.secret && x.value)) await api.mcpSaveSecret(f.name, pairSlot(f, p), p.value, bitwarden);
    } catch {
      setWorking(false);
      return setDialog({ ...d, error: t('Chưa lưu được khoá vào ' + secretStore() + '. Chưa ghi gì.', 'Could not save the key to the ' + secretStore() + '. Nothing was written.') });
    }
    setWorking(false);
    const typed = [...new Set([...d.typed, ...typedSlots(f)])];
    // Typed secrets leave the form: they are kept, in the Keychain.
    const form: McpForm = { ...f, pairs: f.pairs.map((p) => (p.secret && p.value ? { ...p, value: '', kept: true } : p)) };
    const def = formDef(form);
    const changes: McpChange[] = agents.map((agent) => ({ name: form.name, agent, to: { kind: 'def', def, from: d.from, keychain: typed, off: false } }));
    return preview(changes, {
      title: form.edit ? t('Lưu thay đổi của ' + form.name + '?', 'Save the changes to ' + form.name + '?') : t('Thêm ' + form.name + '?', 'Add ' + form.name + '?'),
      back: { ...d, form, typed, error: null },
      steps: !form.edit,
    });
  };

  /** "Bản của …" in the comparison: that agent's definition for every other agent that has it. */
  const useOne = (name: string, pick: Agent) => {
    const row = rows.find((r) => r.name === name);
    const view = row?.defs[pick];
    if (!row || !view) return;
    const others = editable(row).filter((a) => a !== pick);
    const changes: McpChange[] = others.map((agent) => ({ name, agent, to: { kind: 'def', def: view.def, from: pick, keychain: [], off: false } }));
    const back = dialog;
    const to = others.map((a) => AGENT_NAME[a]).join(', ');
    return preview(changes, { title: t('Dùng bản của ' + AGENT_NAME[pick] + ' cho ' + to + '?', 'Use ' + AGENT_NAME[pick] + '’s version for ' + to + '?'), back });
  };

  const remove = (d: { name: string; mode: 'kho' | 'hard'; keys: boolean }) => {
    const row = rows.find((r) => r.name === d.name);
    if (!row) return;
    const changes: McpChange[] = AGENTS.filter((a) => row.cells[a].state !== 'absent').map((agent) => ({ name: d.name, agent, to: { kind: 'absent', store: d.mode === 'kho' } }));
    const forget = d.mode === 'hard' && d.keys && row.secrets.length ? { name: d.name, slots: row.secrets.map((s) => s.id) } : undefined;
    return preview(changes, {
      title: d.mode === 'kho' ? t('Cất ' + d.name + ' vào kho?', 'Put ' + d.name + ' away in the store?') : t('Xoá hẳn ' + d.name + '?', 'Delete ' + d.name + ' for good?'),
      back: dialog,
      forget,
    });
  };

  const unstore = (name: string, agents: Record<Agent, boolean>) => {
    const changes: McpChange[] = AGENTS.filter((a) => agents[a]).map((agent) => ({ name, agent, to: { kind: 'on' } }));
    return preview(changes, { title: t('Bật lại ' + name + '?', 'Turn ' + name + ' back on?'), back: dialog });
  };

  const forget = async (name: string, keys: boolean) => {
    const kept = table?.stored.find((s) => s.name === name);
    setWorking(true);
    try {
      await api.mcpForget(name, kept?.def.slots ?? [], keys);
      showFeedback({
        tone: '',
        title: t('Đã xoá ' + name + ' khỏi kho', 'Deleted ' + name + ' from the store'),
        body: keys ? t('Khoá của nó trong ' + secretStore() + ' cũng đã xoá.', 'Its keys in the ' + secretStore() + ' were deleted too.') : t('Khoá của nó vẫn trong ' + secretStore() + '.', 'Its keys are still in the ' + secretStore() + '.'),
      });
    } catch {
      showFeedback({ tone: 'bad', title: t('Chưa xoá được ' + name, 'Could not delete ' + name), body: t('App không ghi được kho hoặc ' + secretStore() + '. Thử lại.', 'The app could not write the store or the ' + secretStore() + '. Try again.') });
    } finally {
      setWorking(false);
      setDialog(null);
    }
  };

  const saveKeys = async (name: string, values: Record<string, string>, bitwarden: string) => {
    setWorking(true);
    try {
      for (const [slot, value] of Object.entries(values)) if (value) await api.mcpSaveSecret(name, slot, value, bitwarden.trim() || null);
      showFeedback({
        tone: 'info',
        title: t('Đã lưu khoá cho ' + name, 'Saved the key for ' + name),
        body: t('Khoá ở ' + secretStore() + ' của Quota Station. Giờ tick ' + name + ' cho agent cần dùng.', 'The key is in Quota Station’s ' + secretStore() + '. Now tick ' + name + ' for the agents that need it.'),
      });
      setDialog(null);
    } catch {
      showFeedback({
        tone: 'bad',
        title: t('Chưa lưu được khoá', 'Could not save the key'),
        body: os(
          t('Keychain không nhận khoá lúc này. Thử lại.', 'The Keychain is not taking the key right now. Try again.'),
          t('Kho mã hoá không nhận khoá lúc này. Thử lại.', 'The encrypted store is not taking the key right now. Try again.'),
        ),
      });
    } finally {
      setWorking(false);
    }
  };

  const reread = () => {
    showFeedback(null);
    void api.mcpRead().catch(() => {});
  };

  /** A key replaced in the Sao lưu tab: into every agent whose entry holds it, through the preview. */
  const rekey = (name: string, slot: string): boolean => {
    const row = rows.find((r) => r.name === name);
    if (!row) return false;
    const agents = editable(row).filter((a) => row.defs[a]?.slots.includes(slot));
    const changes: McpChange[] = agents.flatMap((agent) => {
      const v = row.defs[agent];
      return v ? [{ name, agent, to: { kind: 'def' as const, def: v.def, from: agent, keychain: [slot], off: false } }] : [];
    });
    if (!changes.length) return false;
    const into = agents.map((a) => AGENT_NAME[a]).join(', ');
    void preview(changes, { title: t('Đưa khoá mới của ' + name + ' vào ' + into + '?', 'Put the new key for ' + name + ' into ' + into + '?') });
    return true;
  };

  /** The MCP tab's actions on banners and feedback lines. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'mcp:undo') void undo();
    else if (a === 'mcp:backups') setDialog({ kind: 'backups' });
    else if (a === 'mcp:reread') reread();
    else if (a.startsWith('mcp:key:')) setDialog({ kind: 'key', name: a.slice('mcp:key:'.length) });
    else if (a.startsWith('mcp:howcmd:')) setDialog({ kind: 'howCmd', name: a.slice('mcp:howcmd:'.length) });
    else if (a.startsWith('mcp:later:')) setLater((l) => [...l, a.slice('mcp:later:'.length)]);
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
    working: working || model?.mcp.busy === 'writing',
    flash,
    later,
    setDialog,
    close,
    act,
    reread,
    rekey,
    toggle: (row: McpRow, agent: Agent) => setPending((p) => toggle(p, row, agent)),
    setColumn: (agent: Agent, on: boolean) => setPending((p) => setColumn(p, rows, agent, on)),
    clearColumn: (agent: Agent) => setPending((p) => clearColumn(p, agent)),
    rowAll: (row: McpRow) => setPending((p) => rowAll(p, row)),
    clearPending: () => setPending({}),
    previewPending: () => void preview(changesOf(pending, rows), { ticks: true }),
    write: () => void write(),
    back: () => dialog?.kind === 'batch' && setDialog(dialog.back),
    add: () => openForm(null),
    edit: (row: McpRow) => (row.differs ? setDialog({ kind: 'compare', name: row.name, pick: 'keep' }) : openForm(row)),
    editForm: (row: McpRow) => openForm(row),
    submitForm: (d: FormDialog) => void submitForm(d),
    useOne: (name: string, pick: Agent) => void useOne(name, pick),
    remove: (d: { name: string; mode: 'kho' | 'hard'; keys: boolean }) => void remove(d),
    unstore: (name: string, agents: Record<Agent, boolean>) => void unstore(name, agents),
    forget: (name: string, keys: boolean) => void forget(name, keys),
    saveKeys: (name: string, values: Record<string, string>, bitwarden: string) => void saveKeys(name, values, bitwarden),
    paste: (text: string) => api.mcpPaste(text, false, null),
    openBackups: (agent: Agent) => void api.mcpOpenBackups(agent).catch(() => {}),
  };
}

export type Mcp = ReturnType<typeof useMcp>;
