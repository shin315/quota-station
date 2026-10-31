// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude tab's accounts: the open dialog and what each action does once
// the app answers. An add goes through its dialog (the three ways in, the
// wait for the browser, then added, already here, or not signed in); the ⋯
// menu opens the details, signing in again and removing; the rest end on
// the line under the tab's head. Each call returns
// the model with what it did (`claudeAccounts.last`), so the answer is read
// from the call rather than from whichever model comes next.

import { useCallback, useState } from 'react';
import type { Api } from '../lib/api';
import { addedFeedback, errorText, resultFeedback } from '../lib/claudeAccounts';
import type { ClaudeActionDone, Model } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';

/** The ways in the add dialog offers. */
export type AddHow = 'browser' | 'current' | 'token' | 'import';

export type ClaudeDialog =
  | { kind: 'add'; how: AddHow; error: string | null }
  /** Waiting for the browser; `slot`: the account signing in again. */
  | { kind: 'wait'; slot: number | null; started: number }
  | { kind: 'token'; error: string | null }
  | { kind: 'done'; slot: number }
  | { kind: 'dup'; slot: number }
  | { kind: 'fail'; text: string }
  | { kind: 'detail'; slot: number }
  // @private:begin
  | { kind: 'rest'; slot: number }
  // @private:end
  | { kind: 'remove'; slot: number }
  | { kind: 'relogin'; slot: number };

/** A call refused because another action holds the accounts. */
const BUSY: ClaudeActionDone = { action: 'alias', ok: false, error: 'busy', detail: null, slot: null, renewed: false, imported: null };

export function useClaudeAccounts(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void, nameOf: (slot: number) => string) {
  const [dialog, setDialog] = useState<ClaudeDialog | null>(null);
  /** A call from a dialog is on its way: its buttons wait. */
  const [pending, setPending] = useState(false);
  const close = useCallback(() => setDialog(null), []);
  /** Moves on only while the dialog that asked is still open. */
  const from = (kind: ClaudeDialog['kind'], next: ClaudeDialog | null) => setDialog((d) => (d?.kind === kind ? next : d));

  /** One call; null when the app refused it (another action runs). */
  const call = async (p: Promise<Model>): Promise<{ model: Model; done: ClaudeActionDone } | null> => {
    try {
      const m = await p;
      return m.claudeAccounts.last ? { model: m, done: m.claudeAccounts.last } : null;
    } catch {
      return null;
    }
  };

  /** An action that ends on the line under the tab's head. */
  const plain = async (p: Promise<Model>, who: string) => {
    const r = await call(p);
    if (!r) return showFeedback({ tone: 'bad', title: errorText(BUSY), body: '', actions: [] });
    const fb = resultFeedback(r.model.claudeAccounts, r.done, who);
    if (fb) showFeedback(fb);
  };

  /** Sign in with the browser: a new account, or `slot` signing in again. */
  const login = async (slot: number | null) => {
    setDialog({ kind: 'wait', slot, started: Date.now() });
    const r = await call(api.claudeAddLogin());
    if (!r) return from('wait', { kind: 'fail', text: errorText(BUSY) });
    const d = r.done;
    if (!d.ok) return from('wait', d.error === 'cancelled' ? null : { kind: 'fail', text: errorText(d) });
    if (slot != null && d.renewed && d.slot === slot) {
      from('wait', null);
      return showFeedback(addedFeedback(nameOf(slot), true));
    }
    from('wait', d.slot == null ? null : { kind: d.renewed ? 'dup' : 'done', slot: d.slot });
  };

  /** The add dialog's Tiếp tục. */
  const add = async (how: AddHow) => {
    if (how === 'browser') return void login(null);
    if (how === 'token') return setDialog({ kind: 'token', error: null });
    setPending(true);
    const r = await call(how === 'import' ? api.claudeImport() : api.claudeAddCurrent());
    setPending(false);
    if (!r) return from('add', { kind: 'add', how, error: errorText(BUSY) });
    const d = r.done;
    if (!d.ok) return from('add', { kind: 'add', how, error: errorText(d) });
    if (how === 'import') {
      from('add', null);
      const fb = resultFeedback(r.model.claudeAccounts, d, '');
      if (fb) showFeedback(fb);
      return;
    }
    from('add', d.slot == null ? null : { kind: d.renewed ? 'dup' : 'done', slot: d.slot });
  };

  const addToken = async (token: string, email: string | null) => {
    setPending(true);
    const r = await call(api.claudeAddToken(token, email));
    setPending(false);
    if (!r) return from('token', { kind: 'token', error: errorText(BUSY) });
    const d = r.done;
    if (!d.ok) return from('token', { kind: 'token', error: errorText(d) });
    from('token', d.slot == null ? null : { kind: d.renewed ? 'dup' : 'done', slot: d.slot });
  };

  /** The added dialog's Xong: its nickname, if one was typed, then the line. */
  const finish = async (slot: number, alias: string) => {
    setDialog(null);
    if (alias.trim()) await call(api.claudeSetAlias(slot, alias.trim()));
    showFeedback(addedFeedback(alias.trim() || nameOf(slot), false));
  };

  const setAlias = (slot: number, alias: string) => {
    setDialog(null);
    void plain(api.claudeSetAlias(slot, alias.trim()), nameOf(slot));
  };
  // @private:begin
  const setEnabled = (slot: number, on: boolean) => {
    setDialog(null);
    void plain(api.claudeSetEnabled(slot, on), nameOf(slot));
  };
  const setRotation = (on: boolean) => void plain(api.claudeSetRotation(on), '');
  // @private:end
  const remove = (slot: number) => {
    setDialog(null);
    void plain(api.claudeRemove(slot), nameOf(slot));
  };
  const cancelLogin = () => void api.claudeCancelLogin().catch(() => {});

  /** The actions on banners and lines. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'claude:add') setDialog({ kind: 'add', how: model?.claudeAccounts.claude === false ? 'current' : 'browser', error: null });
    else if (a === 'claude:import') void plain(api.claudeImport(), '');
    else if (a.startsWith('claude:relogin:')) setDialog({ kind: 'relogin', slot: Number(a.slice('claude:relogin:'.length)) });
    // @private:begin
    else if (a === 'claude:rotation:on') setRotation(true);
    else if (a.startsWith('claude:unrest:')) {
      showFeedback(null);
      setEnabled(Number(a.slice('claude:unrest:'.length)), true);
    }
    // @private:end
    else return false;
    return true;
  };

  return {
    dialog,
    open: setDialog,
    close,
    pending,
    act,
    add,
    login,
    cancelLogin,
    addToken,
    finish,
    setAlias,
    remove,
    // @private:begin
    setEnabled,
    setRotation,
    // @private:end
  };
}

export type ClaudeAccountsState = ReturnType<typeof useClaudeAccounts>;
