// The Bộ cấu hình tab's state (M15): the open dialog (the editor, or the
// apply preview), and the one path every apply takes: the preview the Rust
// side reads from the files now, then the write of what it showed, then
// step 2 when the set wants Codex on the other side, which opens the Codex
// switch of version 1. Nothing is applied before "Áp bộ".

import { useState } from 'react';
import type { Api } from '../lib/api';
import type { CodexTarget, ConfigSet, Model, SetPreview } from '../lib/model';
import { appliedFeedback, codexStep, emptySet, fromCurrent, sentOf, setErrorText, undoneFeedback } from '../lib/sets';
import type { Action, FeedbackView } from '../lib/view';

export type SetDialog =
  | { kind: 'edit'; draft: ConfigSet; error: string | null }
  | { kind: 'apply'; set: ConfigSet; preview: SetPreview | null; sessions: number | null; error: string | null };

export function useSets(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void, onAction: (a: Action) => void) {
  const [dialog, setDialog] = useState<SetDialog | null>(null);
  const [working, setWorking] = useState(false);
  const v = model?.sets ?? null;

  /** "Tạo bộ": a new set; from the empty tab, from the configuration as it is. */
  const openNew = (current: boolean) => {
    const draft = emptySet();
    if (current && model) Object.assign(draft, fromCurrent(model.mcp.table, model.skills.table));
    setDialog({ kind: 'edit', draft, error: null });
  };

  const save = async () => {
    if (dialog?.kind !== 'edit') return;
    const d = dialog;
    if (!d.draft.name.trim()) return setDialog({ ...d, error: setErrorText('name') });
    setWorking(true);
    try {
      const saved = await api.setsSave(d.draft);
      setDialog(null);
      showFeedback({ tone: 'info', title: 'Đã lưu bộ ' + saved.name + '.', body: 'Chưa áp. Bấm Áp… khi cần.' });
    } catch (e) {
      setDialog({ ...d, error: setErrorText(e) });
    } finally {
      setWorking(false);
    }
  };

  const remove = async (set: ConfigSet) => {
    try {
      await api.setsDelete(set.id);
      showFeedback({ tone: '', title: 'Đã xoá bộ ' + set.name + '.', body: 'Không đổi gì trong file của agent.', actions: [{ label: 'Hoàn tác', act: 'sets:undelete' }] });
    } catch (e) {
      showFeedback({ tone: 'bad', title: 'Chưa xoá được bộ ' + set.name, body: setErrorText(e) });
    }
  };

  const undelete = async () => {
    try {
      await api.setsUndelete();
      showFeedback(null);
    } catch (e) {
      showFeedback({ tone: 'warn', title: 'Không đưa bộ lại được', body: String(e) === 'nothing' ? 'Không còn bộ nào vừa xoá.' : setErrorText(e) });
    }
  };

  /** "Áp…": the preview, read from the files now, and who keeps the old configuration. */
  const openApply = async (set: ConfigSet) => {
    showFeedback(null);
    setDialog({ kind: 'apply', set, preview: null, sessions: null, error: null });
    setWorking(true);
    try {
      const [preview, sessions] = await Promise.all([api.setsPreview(set.id), api.claudeSessions().catch(() => null)]);
      setDialog({ kind: 'apply', set, preview, sessions: sessions?.live ?? null, error: null });
    } catch (e) {
      setDialog({ kind: 'apply', set, preview: null, sessions: null, error: setErrorText(e) });
    } finally {
      setWorking(false);
    }
  };

  const openCodex = (target: CodexTarget) => onAction(target === 'chatGpt' ? 'codex:switch:chatGpt' : 'codex:switch:custom');

  const apply = async () => {
    if (dialog?.kind !== 'apply' || !dialog.preview || !model) return;
    const d = dialog;
    const p = d.preview!;
    setWorking(true);
    try {
      const out = await api.setsApply(d.set.id, sentOf(p));
      const step = out.ok ? codexStep(d.set, model.codex) : null;
      setDialog(null);
      // Step 2: the Codex switch of version 1, over the tab; the line stays under it.
      if (step) openCodex(step);
      showFeedback(appliedFeedback(d.set.name, out, step, model.codex));
    } catch (e) {
      setDialog({ ...d, error: setErrorText(e) });
    } finally {
      setWorking(false);
    }
  };

  const undo = async () => {
    if (!model) return;
    const name = v?.applied?.name ?? '';
    showFeedback(null);
    try {
      showFeedback(undoneFeedback(name, await api.setsUndo(), model.codex));
    } catch (e) {
      showFeedback(
        String(e) === 'nothing'
          ? { tone: 'warn', title: 'Không còn gì để hoàn tác', body: 'Bộ vừa áp đã được hoàn tác.' }
          : { tone: 'warn', title: 'Chưa hoàn tác', body: setErrorText(e) },
      );
    }
  };

  /** The tab's actions on feedback lines and cards. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'sets:undo') void undo();
    else if (a === 'sets:undelete') void undelete();
    else if (a === 'sets:new') openNew(!v?.sets.length);
    else if (a.startsWith('sets:apply:')) {
      const set = v?.sets.find((s) => s.id === a.slice('sets:apply:'.length));
      if (set) void openApply(set);
    } else return false;
    return true;
  };

  const close = () => {
    if (!working) setDialog(null);
  };

  return {
    view: v,
    dialog,
    working: working || !!v?.busy,
    act,
    close,
    openNew,
    openEdit: (set: ConfigSet) => setDialog({ kind: 'edit', draft: structuredClone(set), error: null }),
    setDraft: (draft: ConfigSet) => dialog?.kind === 'edit' && setDialog({ ...dialog, draft, error: null }),
    /** "Lấy từ cấu hình hiện tại": the name stays, the entries are taken now; Codex's side stays as chosen. */
    takeCurrent: () => {
      if (dialog?.kind !== 'edit' || !model) return;
      setDialog({ ...dialog, draft: { ...dialog.draft, ...fromCurrent(model.mcp.table, model.skills.table) }, error: null });
    },
    save: () => void save(),
    remove: (set: ConfigSet) => void remove(set),
    openApply: (set: ConfigSet) => void openApply(set),
    apply: () => void apply(),
    openCodex,
    undo: () => void undo(),
  };
}

export type Sets = ReturnType<typeof useSets>;
