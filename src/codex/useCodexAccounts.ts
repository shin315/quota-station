// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Codex tab's accounts: the open dialog and what each action does once
// the app answers. "Thêm tài khoản" offers a ChatGPT sign-in (the wait for
// the browser, then added, already here, or not signed in) or a custom
// provider (its form, "Kiểm tra", the preview of the lines it adds); the ⋯
// menu opens the details and nickname, a new sign-in and the removal. A
// switch goes through the switch sheet, as the popover's does.

import { useCallback, useState } from 'react';
import type { Api } from '../lib/api';
import { accountFeedback, refusedText } from '../lib/codexAccounts';
import type { CodexAccountDone, CodexLoginFail, Model } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';

/** The ways in "Thêm tài khoản" offers. */
export type CodexAddHow = 'chatgpt' | 'custom';

export type CodexDialog =
  | { kind: 'add'; how: CodexAddHow; error: string | null }
  /** Waiting for the browser; `id`: the account signing in again. */
  | { kind: 'wait'; id: string | null; started: number }
  | { kind: 'done'; id: string }
  | { kind: 'dup'; id: string }
  | { kind: 'fail'; reason: CodexLoginFail; detail: string | null; id: string | null }
  | { kind: 'provider' }
  | { kind: 'providerDone'; id: string }
  | { kind: 'detail'; id: string }
  | { kind: 'remove'; id: string }
  | { kind: 'relogin'; id: string };

export function useCodexAccounts(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const [dialog, setDialog] = useState<CodexDialog | null>(null);
  /** A call from a dialog is on its way: its buttons wait. */
  const [pending, setPending] = useState(false);
  const close = useCallback(() => setDialog(null), []);
  /** Moves on only while the dialog that asked is still open. */
  const from = (kind: CodexDialog['kind'], next: CodexDialog | null) => setDialog((d) => (d?.kind === kind ? next : d));

  const call = async (p: Promise<CodexAccountDone>): Promise<CodexAccountDone> => {
    try {
      return await p;
    } catch {
      return { kind: 'refused', reason: 'busy' };
    }
  };

  /** Sign in with the browser: a new account, or `id` signing in again. */
  const login = async (id: string | null) => {
    setDialog({ kind: 'wait', id, started: Date.now() });
    const d = await call(api.codexLogin(id));
    if (d.kind === 'failed') return from('wait', d.reason === 'cancelled' ? null : { kind: 'fail', reason: d.reason, detail: d.detail, id });
    if (d.kind === 'refused') return from('wait', { kind: 'fail', reason: 'other', detail: refusedText(d.reason), id });
    if (d.kind === 'renewed') {
      from('wait', null);
      if (model) showFeedback(accountFeedback(d, model));
      return;
    }
    if (d.kind === 'added' || d.kind === 'dup') from('wait', { kind: d.kind === 'added' ? 'done' : 'dup', id: d.id });
  };

  /** "Thêm tài khoản"'s Tiếp tục. */
  const add = (how: CodexAddHow) => {
    if (how === 'chatgpt') return void login(null);
    setDialog({ kind: 'provider' });
  };

  /** An action that ends on the line under the tab's head. */
  const plain = async (p: Promise<CodexAccountDone>, latest: () => Model | null) => {
    setPending(true);
    const d = await call(p);
    setPending(false);
    const m = latest();
    if (m) showFeedback(accountFeedback(d, m));
  };

  const rename = (id: string, name: string, latest: () => Model | null) => {
    setDialog(null);
    void plain(api.codexRename(id, name), latest);
  };
  const remove = (id: string, logout: boolean, latest: () => Model | null) => {
    setDialog(null);
    void plain(api.codexRemove(id, logout, model?.codex.hash ?? null), latest);
  };
  const cancelLogin = () => void api.codexCancelLogin().catch(() => {});

  /** The actions on rows, banners and lines. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'codex:add') setDialog({ kind: 'add', how: 'chatgpt', error: null });
    else if (a.startsWith('codex:relogin:')) setDialog({ kind: 'relogin', id: a.slice('codex:relogin:'.length) });
    else return false;
    return true;
  };

  return { api, dialog, open: setDialog, close, pending, setPending, act, add, login, cancelLogin, rename, remove };
}

export type CodexAccountsState = ReturnType<typeof useCodexAccounts>;
