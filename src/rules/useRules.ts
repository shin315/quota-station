// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The rotation editor's state (M5): what the owner changed and not saved
// yet, the action running, the banner a failed save leaves, the preview,
// and the dialogs (compare two copies, unsaved changes). The draft lives in
// the main window, so leaving the tab or closing the window can ask first.

import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api } from '../lib/api';
import type { Model, Rules, RulesError, Tab } from '../lib/model';
import {
  adoptFeedback,
  changedBanner,
  cloneRules,
  keepFeedback,
  retryFeedback,
  ruleChanges,
  ruleNames,
  rulesErrorBanner,
  rulesProblem,
  sameRules,
  saveFeedback,
} from '../lib/rules';
import { decisionText, namesOf } from '../lib/switching';
import type { Action, BannerView, FeedbackView } from '../lib/view';

/** The owner's edits; null while the editor shows the file as it is. */
interface Draft {
  rules: Rules;
  /** The file version the edits started from. */
  base: string | null;
  /** Made from the switcher's defaults for a Mac with no file. */
  fresh: boolean;
}

export type Working = 'save' | 'adopt' | 'keep' | 'retry';

export type Preview = { kind: 'loading' } | { kind: 'ok'; text: string; raw: string } | { kind: 'error'; text: string };

/** Ask before leaving the tab (`tab`), closing the window (`close`) or quitting the app (`quit`). */
export type Leave = { kind: 'tab'; tab: Tab } | { kind: 'close' } | { kind: 'quit' };

/** Waits this long after the last change before asking the switcher for a preview. */
const PREVIEW_MS = 700;

function isRulesError(e: unknown): e is RulesError {
  return typeof e === 'object' && e !== null && 'kind' in e;
}

const PREVIEW_ERRORS: Record<string, string> = {
  notFound: 'Không có switcher trên máy này.',
  timeout: 'Switcher không trả lời kịp.',
  noDecision: 'Switcher không in quyết định (claude-swap không trả số).',
};

export function useRules(api: Api, model: Model | null, clock: () => Date, showFeedback: (fb: FeedbackView | null) => void, tab: Tab | null) {
  const [draft, setDraft] = useState<Draft | null>(null);
  const [error, setError] = useState<{ banner: BannerView; restored: boolean } | null>(null);
  const [working, setWorking] = useState<Working | null>(null);
  const [compare, setCompare] = useState<string | null>(null);
  const [leave, setLeave] = useState<Leave | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);

  const r = model?.rotation;
  const saved = r?.status === 'ok' ? r.rules : null;
  // A broken file is edited from the switcher's defaults; a missing one waits for "Tạo từ mặc định".
  const current = draft?.rules ?? saved ?? (r?.status === 'invalid' ? r.defaults : null);
  const base = draft ? draft.base : (r?.version ?? null);
  const dirty = !!draft;
  const stale = !!draft && !!r && draft.base !== r.version;
  const nameOf = model ? ruleNames(model) : (n: number) => 'Tài khoản ' + n;
  const changes =
    model && current
      ? [
          ...(draft?.fresh ? ['Tạo file luật mới từ mặc định'] : r?.status === 'invalid' ? ['Ghi đè file luật không hợp lệ'] : []),
          ...ruleChanges(saved ?? r?.defaults ?? current, current, nameOf, clock()),
        ]
      : [];

  // The Rust side asks before closing the main window while this is true.
  useEffect(() => {
    void api.rulesDirty(dirty);
  }, [api, dirty]);

  const edit = (change: (rules: Rules) => void) => {
    if (!current || !r) return;
    const next = cloneRules(current);
    change(next);
    setError(null);
    const fresh = draft?.fresh ?? false;
    if (!fresh && saved && sameRules(next, saved)) setDraft(null);
    else setDraft({ rules: next, base: draft ? draft.base : r.version, fresh });
  };

  const run = async (what: Working, call: () => Promise<Model>, feedback: (m: Model) => FeedbackView, peer?: string) => {
    if (working) return false;
    const before = model?.rotation.stamp ?? null;
    setWorking(what);
    try {
      const m = await call();
      // Retrying leaves this Mac's file alone, so the edits stay.
      if (what !== 'retry') setDraft(null);
      setError(null);
      showFeedback(feedback(m));
      return true;
    } catch (e) {
      // `changed` shows as the "newer copy" banner once the model has the new file.
      if (isRulesError(e) && e.kind !== 'changed' && e.kind !== 'busy') setError({ banner: rulesErrorBanner(e, before, peer), restored: e.restored });
      else if (!isRulesError(e)) setError({ banner: rulesErrorBanner({ kind: 'write', detail: String(e), restored: false }, before), restored: false });
      return false;
    } finally {
      setWorking(null);
    }
  };

  const save = (overwrite = false) =>
    current ? run('save', () => api.saveRotation(current, overwrite ? (r?.version ?? null) : base), saveFeedback) : Promise.resolve(false);

  const peerName = (alias: string) => r?.peers.find((p) => p.alias === alias)?.name ?? alias;

  const adopt = (alias: string) => {
    setCompare(null);
    const peer = r?.peers.find((p) => p.alias === alias);
    const their = peer?.state && (peer.state.kind === 'newer' || peer.state.kind === 'edited') ? peer.state.their : null;
    return run('adopt', () => api.adoptRotation(alias, r?.version ?? null), () => adoptFeedback(peerName(alias), their, clock()), peerName(alias));
  };
  const keep = (alias: string) => {
    setCompare(null);
    return run('keep', () => api.keepRotation(r?.version ?? null), (m) => keepFeedback(m, peerName(alias)));
  };
  const retry = () => run('retry', () => api.retryRotation(), (m) => retryFeedback(m, clock()));

  /** The tab's banner and feedback buttons. Returns false for other actions. */
  const act = (a: Action): boolean => {
    if (!a.startsWith('rules:')) return false;
    const [, what, alias] = a.split(':');
    if (what === 'retry') void retry();
    else if (what === 'adopt' && alias) void adopt(alias);
    else if (what === 'compare' && alias) setCompare(alias);
    else if (what === 'create' && r) setDraft({ rules: cloneRules(r.defaults), base: r.version, fresh: true });
    else if (what === 'reload') setDraft(null);
    else if (what === 'overwrite') void save(true);
    else if (what === 'dismiss') setError(null);
    return true;
  };

  // The switcher's decision with the rules in the editor, a moment after the last change.
  const key = current ? JSON.stringify(current) : '';
  const latest = useRef(model);
  latest.current = model;
  useEffect(() => {
    if (tab !== 'rules' || !current || rulesProblem(current)) {
      setPreview(null);
      return;
    }
    let live = true;
    setPreview({ kind: 'loading' });
    const id = setTimeout(() => {
      api.previewRotation(current).then(
        (d) => {
          const m = latest.current;
          if (!live || !m) return;
          const active = m.snapshot.claude.accounts.find((a) => a.active && a.number > 0);
          const names = namesOf(m);
          setPreview({ kind: 'ok', text: decisionText(d.decision, d.raw, active ? names(active.number) : null, names), raw: d.raw });
        },
        (e) => {
          if (live) setPreview({ kind: 'error', text: PREVIEW_ERRORS[String(e)] ?? 'Chưa xem trước được: ' + String(e) });
        },
      );
    }, PREVIEW_MS);
    return () => {
      live = false;
      clearTimeout(id);
    };
    // `current` is read through `key`, so a new object with the same rules does not ask again.
  }, [api, key, tab]);

  /** Leaving the tab with unsaved changes asks; anything else goes straight on. */
  const guardTab = useCallback(
    (from: Tab | null, to: Tab, go: (t: Tab) => void) => {
      if (from === 'rules' && to !== 'rules' && dirty) setLeave({ kind: 'tab', tab: to });
      else go(to);
    },
    [dirty],
  );

  const dirtyNow = useRef(dirty);
  dirtyNow.current = dirty;
  useEffect(() => {
    let live = true;
    const stops: (() => void)[] = [];
    const keep = (unlisten: () => void) => {
      if (live) stops.push(unlisten);
      else unlisten();
    };
    void api
      .onCloseRequested(() => {
        if (dirtyNow.current) setLeave({ kind: 'close' });
        else void api.closeMain(false);
      })
      .then(keep);
    // The request is taken from the Rust side, so a window opened for it
    // still sees it once it listens, and an event seen twice acts once.
    const quit = () =>
      void api.takeQuit().then((asked) => {
        if (!live || !asked) return;
        if (dirtyNow.current) setLeave({ kind: 'quit' });
        else void api.closeMain(true);
      });
    void api
      .onQuitRequested(quit)
      .then(keep)
      .then(() => live && quit());
    return () => {
      live = false;
      stops.forEach((s) => s());
    };
  }, [api]);

  /** The unsaved-changes dialog's three buttons. */
  const answerLeave = async (answer: 'discard' | 'cancel' | 'save', go: (t: Tab) => void) => {
    const l = leave;
    if (!l) return;
    if (answer === 'cancel') return setLeave(null);
    if (answer === 'save') {
      setLeave(null);
      if (!(await save())) return;
    } else {
      setLeave(null);
      setDraft(null);
      setError(null);
    }
    if (l.kind === 'tab') go(l.tab);
    else {
      await api.rulesDirty(false);
      await api.closeMain(l.kind === 'quit');
    }
  };

  return {
    current,
    saved,
    dirty,
    stale,
    changes,
    nameOf,
    error,
    working: working ?? (r?.saving ? ('save' as const) : null),
    preview,
    compare,
    leave,
    banner: stale && model ? changedBanner(model) : null,
    edit,
    save: () => void save(),
    revert: () => {
      setDraft(null);
      setError(null);
    },
    act,
    adopt,
    keep,
    closeCompare: () => setCompare(null),
    guardTab,
    answerLeave,
  };
}

export type RulesState = ReturnType<typeof useRules>;
