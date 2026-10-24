// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Chép sang Mac khác tab's state (M13): the Mac and the way chosen, "So
// sánh" and what it found, the items ticked, "Gửi cả khoá qua SSH" (off to
// begin with, and off again for each new comparison), and the dialogs: one
// item side by side, then the copy's preview, its run, its result and the
// undo on the Mac it was written to. Nothing is written before "Chép".

import { useState } from 'react';
import type { Api } from '../lib/api';
import { copyError, initialTicks, macState, picked, selectionOf, undoneFeedback, type CopyTicks } from '../lib/copy';
import { t } from '../lib/i18n';
import type { CopyComparison, Model, RestoreDone } from '../lib/model';
import type { FeedbackView } from '../lib/view';

export type CopyPhase = 'idle' | 'reading' | 'diff' | 'error';

export type CopyDialog =
  | { kind: 'item'; key: string }
  | { kind: 'go'; step: 'prev' | 'run' | 'done' | 'undo' | 'undoing'; done: RestoreDone | null; error: FeedbackView | null; changed: string | null };

export function useCopy(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const peers = model?.rotation?.peers ?? [];
  const [alias, setAlias] = useState<string | null>(null);
  const [push, setPush] = useState(true);
  const [phase, setPhase] = useState<CopyPhase>('idle');
  const [cmp, setCmp] = useState<CopyComparison | null>(null);
  const [error, setError] = useState<FeedbackView | null>(null);
  /** How the Mac answered the last comparison that failed: `unreach`, `old`, `noApp`. */
  const [errorKind, setErrorKind] = useState<string | null>(null);
  const [ticks, setTicks] = useState<CopyTicks>({});
  const [keys, setKeys] = useState(false);
  const [showSame, setShowSame] = useState(false);
  const [dialog, setDialog] = useState<CopyDialog | null>(null);
  const [answers, setAnswers] = useState<Record<string, 'ok' | 'unreach' | 'old' | 'noApp'>>({});
  const [working, setWorking] = useState(false);

  const mac = alias && peers.some((p) => p.alias === alias) ? alias : (peers[0]?.alias ?? null);
  const peer = peers.find((p) => p.alias === mac) ?? null;
  /** The other Mac's name: its own once read, else the one in settings. */
  const other = (cmp?.alias === mac ? cmp?.mac : null) || peer?.name || mac || t('Mac kia', 'the other Mac');
  const here = model?.machine ?? t('Máy này', 'This Mac');

  const choose = (a: string) => {
    if (a === mac) return;
    setAlias(a);
    setPhase('idle');
    setCmp(null);
    setError(null);
    setTicks({});
    setKeys(false);
  };

  const way = (p: boolean) => {
    setPush(p);
    if (cmp) setTicks(initialTicks(cmp, p));
  };

  const compare = async () => {
    if (!mac) return;
    const for_ = mac;
    setPhase('reading');
    setError(null);
    showFeedback(null);
    try {
      const c = await api.copyCompare(for_);
      setCmp(c);
      setTicks(initialTicks(c, push));
      setKeys(false);
      setShowSame(false);
      setPhase('diff');
      setAnswers((x) => ({ ...x, [for_]: 'ok' }));
    } catch (e) {
      setCmp(null);
      setError(copyError(e, peer?.name || for_));
      setPhase('error');
      const st = macState(e);
      setErrorKind(st ?? null);
      if (st) setAnswers((x) => ({ ...x, [for_]: st }));
    }
  };

  const items = cmp ? picked(cmp, ticks, push) : [];

  const run = async () => {
    if (!mac || !cmp || !items.length) return;
    setDialog({ kind: 'go', step: 'run', done: null, error: null, changed: null });
    setWorking(true);
    try {
      const done = await api.copyRun(mac, push, selectionOf(items), keys);
      setDialog({ kind: 'go', step: 'done', done, error: null, changed: null });
    } catch (e) {
      setDialog({ kind: 'go', step: 'prev', done: null, error: copyError(e, other), changed: null });
    } finally {
      setWorking(false);
    }
  };

  const undo = async (force: boolean) => {
    if (!mac || dialog?.kind !== 'go' || !dialog.done) return;
    const d = dialog;
    const done = d.done;
    if (!done) return;
    setDialog({ ...d, step: 'undoing', error: null });
    setWorking(true);
    try {
      const u = await api.copyUndo(mac, push, done.at, force);
      setDialog(null);
      setPhase('idle');
      showFeedback(undoneFeedback(u, push ? other : here));
    } catch (e) {
      const s = String(e);
      if (s.startsWith('changed:')) setDialog({ ...d, step: 'undo', changed: s.slice('changed:'.length), error: null });
      else setDialog({ ...d, step: 'done', error: copyError(e, other) });
    } finally {
      setWorking(false);
    }
  };

  /** "Xong": the result closes, and the list needs a new comparison. */
  const finish = () => {
    if (dialog?.kind === 'go' && dialog.done) {
      const dest = push ? other : here;
      showFeedback({
        tone: 'info',
        title: push ? t('Đã chép sang ' + other + '.', 'Copied to ' + other + '.') : t('Đã chép về máy này.', 'Copied to this Mac.'),
        body: t(
          'Quota Station trên ' + dest + ' đã sao lưu trước khi ghi. Bấm So sánh lại để xem hai máy bây giờ.',
          'Quota Station on ' + dest + ' backed up before writing. Press Compare again to see both Macs as they are now.',
        ),
      });
      setPhase('idle');
      setCmp(null);
    }
    setDialog(null);
  };

  const close = () => {
    if (working) return;
    if (dialog?.kind === 'go' && (dialog.step === 'run' || dialog.step === 'undoing')) return;
    if (dialog?.kind === 'go' && dialog.done) return finish();
    setDialog(null);
  };

  return {
    peers,
    mac,
    peer,
    other,
    here,
    push,
    phase,
    cmp,
    error,
    errorKind,
    ticks,
    keys,
    showSame,
    dialog,
    answers,
    items,
    working,
    choose,
    way,
    compare: () => void compare(),
    tick: (key: string, on: boolean) => setTicks((x) => ({ ...x, [key]: on })),
    setKeys,
    setShowSame,
    openItem: (key: string) => setDialog({ kind: 'item', key }),
    openGo: () => setDialog({ kind: 'go', step: 'prev', done: null, error: null, changed: null }),
    run: () => void run(),
    askUndo: () => dialog?.kind === 'go' && setDialog({ ...dialog, step: 'undo', changed: null, error: null }),
    backToDone: () => dialog?.kind === 'go' && setDialog({ ...dialog, step: 'done', changed: null }),
    undo: (force: boolean) => void undo(force),
    finish,
    close,
  };
}

export type Copy = ReturnType<typeof useCopy>;
