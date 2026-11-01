// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Sao lưu tab's state (M10, M12): the open dialog, "Sao lưu bây giờ",
// and the Khôi phục sheet's four steps: pick a pack, the preview the Rust
// side reads now (servers, skills, plugins, instruction files), the keys to
// paste from Bitwarden, and the restore with its result, "Thử lại" on a
// part that failed, and the undo. A pasted key stays in the sheet only
// until the restore runs: it goes to the Keychain, never back.

import { useState } from 'react';
import type { Api } from '../lib/api';
import {
  backupNowError,
  initialTicks,
  keySlot,
  keysFor,
  moreFor,
  packedFeedback,
  picked,
  restoreError,
  undoError,
  undoneFeedback,
  usable,
  type Ticks,
} from '../lib/backup';
import { cloudDrive, mac, os, t } from '../lib/i18n';
import type { BackupKey, Model, PackSummary, RestoreDone, RestorePartResult, RestorePlan } from '../lib/model';
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
  /** The part "Thử lại" runs now, as `part|agent|id`. */
  retrying?: string | null;
}

export const partKey = (r: RestorePartResult): string => r.part + '|' + (r.agent ?? '') + '|' + r.id;

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
  const pick = (ticks: Ticks, plan: RestorePlan): Ticks => {
    const known = initialTicks(plan);
    return Object.fromEntries(Object.entries(ticks).filter(([k]) => k in known));
  };

  const run = async (d: RestoreDialog) => {
    if (!d.pack || !d.plan) return;
    const plan = d.plan;
    const picks = picked(plan, d.ticks).map((i) => ({ name: i.name, kind: i.kind }));
    const more = moreFor(plan, d.ticks);
    const keys = keysFor(plan, d.ticks)
      .map((k) => ({ mcp: k.mcp, slot: k.slot, value: (d.keys[keySlot(k)] ?? '').trim() }))
      .filter((k) => k.value);
    // The keys leave the sheet now: the Keychain has them, or nothing does.
    setDialog({ ...d, step: 'run', keys: {}, error: null });
    setWorking(true);
    try {
      const done = await api.restoreRun(d.pack, picks, more, keys, plan.bases);
      setDialog({ ...d, step: 'done', keys: {}, done, error: null });
    } catch (e) {
      setDialog({ ...d, step: 'prev', keys: {}, error: restoreError(e) });
    } finally {
      setWorking(false);
    }
  };

  /** "Thử lại" on one part of the result: its line becomes what this run says. */
  const retry = async (d: RestoreDialog, r: RestorePartResult) => {
    if (!d.pack || !d.done) return;
    const key = partKey(r);
    setDialog({ ...d, retrying: key });
    setWorking(true);
    try {
      const again = await api.restoreRetry(d.pack, r.part, r.id, r.agent);
      setDialog((x) =>
        x?.kind === 'restore' && x.done
          ? { ...x, retrying: null, done: { ...x.done, canUndo: x.done.canUndo || again.ok, parts: (x.done.parts ?? []).map((p) => (partKey(p) === key ? again : p)) } }
          : x,
      );
    } catch (e) {
      setDialog((x) => (x?.kind === 'restore' ? { ...x, retrying: null } : x));
      showFeedback(restoreError(e));
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
        showFeedback({
          tone: 'info',
          title: t('Đã thay khoá ' + key.mcp + '.', 'Replaced the key for ' + key.mcp + '.'),
          body: t('Agent nào bật ' + key.mcp + ' lần sau sẽ dùng khoá mới.', 'Agents that turn on ' + key.mcp + ' will use the new key from now on.'),
        });
    } catch {
      showFeedback({
        tone: 'bad',
        title: t('Chưa thay được khoá', 'Could not replace the key'),
        body: os(
          t('Keychain không nhận khoá lúc này. Thử lại.', 'Keychain is not accepting keys right now. Try again.'),
          t('Kho mã hoá không nhận khoá lúc này. Thử lại.', 'The encrypted store is not accepting keys right now. Try again.'),
        ),
      });
    } finally {
      setWorking(false);
    }
  };

  const deleteKey = async (key: BackupKey) => {
    setWorking(true);
    try {
      await api.backupDeleteKey(key.mcp, key.slot);
      showFeedback({
        tone: '',
        title: mac(t('Đã xoá khoá ' + key.mcp + ' khỏi máy này.', 'Deleted the key for ' + key.mcp + ' from this Mac.')),
        body: t('Agent đang dùng ' + key.mcp + ' vẫn giữ bản của nó.', 'Agents using ' + key.mcp + ' keep their own copy.'),
      });
    } catch {
      showFeedback({
        tone: 'bad',
        title: t('Chưa xoá được khoá', 'Could not delete the key'),
        body: os(
          t('Keychain không nhận lệnh lúc này. Thử lại.', 'Keychain is not responding right now. Try again.'),
          t('Kho mã hoá không nhận lệnh lúc này. Thử lại.', 'The encrypted store is not responding right now. Try again.'),
        ),
      });
    } finally {
      setWorking(false);
      setDialog(null);
    }
  };

  const openFolder = () =>
    void api.backupOpenFolder().catch(() => showFeedback({ tone: 'warn', title: t('Không mở được thư mục', 'Could not open the folder'), body: mac(t(cloudDrive() + ' chưa bật trên máy này.', cloudDrive() + ' is off on this Mac.')) }));

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
    retry: (d: RestoreDialog, r: RestorePartResult) => void retry(d, r),
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
