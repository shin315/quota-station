// The Sao lưu tab's state (M10): the open dialog, "Sao lưu bây giờ", and
// the Khôi phục sheet's four steps: pick a pack, the preview the Rust side
// reads from the files now, the keys to paste from Bitwarden, and the
// restore with its result and its undo. A pasted key stays in the sheet
// only until the restore runs: it goes to the Keychain, never back.

import { useState } from 'react';
import type { Api } from '../lib/api';
import {
  backupNowError,
  initialTicks,
  itemKey,
  keySlot,
  keysFor,
  packedFeedback,
  picked,
  restoreError,
  undoError,
  undoneFeedback,
  usable,
  type Ticks,
} from '../lib/backup';
import type { BackupKey, Model, PackSummary, RestoreDone, RestorePlan } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';

export type RestoreStep = 'pick' | 'prev' | 'keys' | 'run' | 'done';

export interface RestoreDialog {
  kind: 'restore';
  step: RestoreStep;
  pack: string | null;
  plan: RestorePlan | null;
  ticks: Ticks;
  /** `mcp|slot` → what was pasted; emptied once the restore ran. */
  keys: Record<string, string>;
  done: RestoreDone | null;
  error: FeedbackView | null;
  /** The item whose "So sánh" is open. */
  compare: string | null;
}

export type BackupDialog =
  | RestoreDialog
  | { kind: 'view'; id: string; summary: PackSummary | null; error: FeedbackView | null }
  | { kind: 'undo'; changed: string | null; from: RestoreDialog | null }
  | { kind: 'key'; key: BackupKey }
  | { kind: 'delKey'; key: BackupKey };

export function useBackup(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void, opts: { rekey(name: string, slot: string): boolean; onTab(tab: 'mcp'): void }) {
  const [dialog, setDialog] = useState<BackupDialog | null>(null);
  const [working, setWorking] = useState(false);
  const view = model?.backup ?? null;
  const packs = view?.packs ?? [];

  const reread = () => {
    showFeedback(null);
    void api.backupRead().catch(() => {});
  };

  const backupNow = async () => {
    setWorking(true);
    try {
      await api.backupNow();
      showFeedback(packedFeedback(new Date()));
    } catch (e) {
      showFeedback(backupNowError(e));
    } finally {
      setWorking(false);
    }
  };

  const openView = async (id: string) => {
    setDialog({ kind: 'view', id, summary: null, error: null });
    try {
      const summary = await api.packSummary(id);
      setDialog((d) => (d?.kind === 'view' && d.id === id ? { ...d, summary } : d));
    } catch (e) {
      setDialog((d) => (d?.kind === 'view' && d.id === id ? { ...d, error: restoreError(e) } : d));
    }
  };

  const openRestore = (id?: string) => {
    const pack = id ?? packs.find((p) => p.mine && usable(p))?.id ?? packs.find(usable)?.id ?? null;
    setDialog({ kind: 'restore', step: 'pick', pack, plan: null, ticks: {}, keys: {}, done: null, error: null, compare: null });
  };

  const update = (f: (d: RestoreDialog) => RestoreDialog) => setDialog((d) => (d?.kind === 'restore' ? f(d) : d));

  /** Step 2: the preview, read now. */
  const toPreview = async (d: RestoreDialog) => {
    if (!d.pack) return;
    setWorking(true);
    try {
      const plan = await api.restorePlan(d.pack);
      const ticks = d.plan?.pack === plan.pack ? { ...initialTicks(plan), ...pick(d.ticks, plan) } : initialTicks(plan);
      setDialog({ ...d, step: 'prev', plan, ticks, error: null });
    } catch (e) {
      setDialog({ ...d, error: restoreError(e) });
    } finally {
      setWorking(false);
    }
  };

  /** The ticks still meaning a line of `plan`. */
  const pick = (ticks: Ticks, plan: RestorePlan): Ticks => Object.fromEntries(Object.entries(ticks).filter(([k]) => plan.items.some((i) => itemKey(i) === k)));

  const run = async (d: RestoreDialog) => {
    if (!d.pack || !d.plan) return;
    const plan = d.plan;
    const picks = picked(plan, d.ticks).map((i) => ({ name: i.name, kind: i.kind }));
    const keys = keysFor(plan, d.ticks)
      .map((k) => ({ mcp: k.mcp, slot: k.slot, value: (d.keys[keySlot(k)] ?? '').trim() }))
      .filter((k) => k.value);
    // The keys leave the sheet now: the Keychain has them, or nothing does.
    setDialog({ ...d, step: 'run', keys: {}, error: null });
    setWorking(true);
    try {
      const done = await api.restoreRun(d.pack, picks, keys, plan.bases);
      setDialog({ ...d, step: 'done', keys: {}, done, error: null });
    } catch (e) {
      setDialog({ ...d, step: 'prev', keys: {}, error: restoreError(e) });
    } finally {
      setWorking(false);
    }
  };

  /** After the preview: the keys to paste, or straight to the restore. */
  const next = (d: RestoreDialog) => {
    if (d.step === 'pick') return void toPreview(d);
    if (d.step === 'prev') return d.plan && keysFor(d.plan, d.ticks).length ? setDialog({ ...d, step: 'keys', error: null }) : void run(d);
    if (d.step === 'keys') return void run(d);
  };

  const undo = async (force: boolean, from: RestoreDialog | null) => {
    setWorking(true);
    try {
      const u = await api.restoreUndo(force);
      setDialog(null);
      showFeedback(undoneFeedback(u));
    } catch (e) {
      const s = String(e);
      if (s.startsWith('changed:')) setDialog({ kind: 'undo', changed: s.slice('changed:'.length), from });
      else {
        setDialog(null);
        showFeedback(undoError(e));
      }
    } finally {
      setWorking(false);
    }
  };

  const replaceKey = async (key: BackupKey, value: string) => {
    setWorking(true);
    try {
      await api.mcpSaveSecret(key.mcp, key.slot, value.trim(), null);
      setDialog(null);
      void api.backupRead().catch(() => {});
      // The new key into the agents that use it, through the MCP preview.
      if (!opts.rekey(key.mcp, key.slot))
        showFeedback({ tone: 'info', title: 'Đã thay khoá ' + key.mcp + '.', body: 'Agent nào bật ' + key.mcp + ' lần sau sẽ dùng khoá mới.' });
    } catch {
      showFeedback({ tone: 'bad', title: 'Chưa thay được khoá', body: 'Keychain không nhận khoá lúc này. Thử lại.' });
    } finally {
      setWorking(false);
    }
  };

  const deleteKey = async (key: BackupKey) => {
    setWorking(true);
    try {
      await api.backupDeleteKey(key.mcp, key.slot);
      showFeedback({ tone: '', title: 'Đã xoá khoá ' + key.mcp + ' khỏi máy này.', body: 'Agent đang dùng ' + key.mcp + ' vẫn giữ bản của nó.' });
    } catch {
      showFeedback({ tone: 'bad', title: 'Chưa xoá được khoá', body: 'Keychain không nhận lệnh lúc này. Thử lại.' });
    } finally {
      setWorking(false);
      setDialog(null);
    }
  };

  const openFolder = () =>
    void api.backupOpenFolder().catch(() => showFeedback({ tone: 'warn', title: 'Không mở được thư mục', body: 'iCloud Drive chưa bật trên máy này.' }));

  /** The tab's actions on banners and feedback lines. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'backup:reread') reread();
    else if (a === 'backup:open') openFolder();
    else if (a === 'backup:undo') setDialog({ kind: 'undo', changed: null, from: null });
    else if (a === 'backup:restore') openRestore();
    else return false;
    return true;
  };

  const close = () => {
    if (working) return;
    if (dialog?.kind === 'restore' && dialog.step === 'run') return;
    setDialog(null);
  };

  return {
    view,
    dialog,
    working: working || view?.busy != null,
    setDialog,
    update,
    close,
    act,
    reread,
    backupNow: () => void backupNow(),
    setAuto: (afterChange: boolean, daily: boolean) => void api.backupSetAuto(afterChange, daily).catch(() => {}),
    openView: (id: string) => void openView(id),
    openRestore,
    next,
    back: (d: RestoreDialog) => setDialog({ ...d, step: d.step === 'keys' ? 'prev' : 'pick', error: null }),
    toPreview: (d: RestoreDialog) => void toPreview(d),
    undo: (force: boolean, from: RestoreDialog | null) => void undo(force, from),
    replaceKey: (key: BackupKey, value: string) => void replaceKey(key, value),
    deleteKey: (key: BackupKey) => void deleteKey(key),
    openFolder,
    toMcp: () => {
      setDialog(null);
      opts.onTab('mcp');
    },
  };
}

export type Backup = ReturnType<typeof useBackup>;
