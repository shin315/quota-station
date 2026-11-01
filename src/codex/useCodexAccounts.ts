// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Codex tab's accounts: the open dialog and what each action does once
// the app answers. "Thêm tài khoản" offers a ChatGPT sign-in (the wait for
// the browser, then added, already here, or not signed in), a custom
// provider (its form, "Kiểm tra", the preview of the lines it adds) or
// sign-in files (the app's file picker, what each file holds); the ⋯
// menu opens the details and nickname, a new sign-in and the removal. A
// switch goes through the switch sheet, as the popover's does.

import { useCallback, useState } from 'react';
import type { Api } from '../lib/api';
import { accountFeedback, refusedText } from '../lib/codexAccounts';
import { t } from '../lib/i18n';
import type { CodexAccountDone, CodexImportPreview, CodexLoginFail, Model } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';

/** The ways in "Thêm tài khoản" offers. */
export type CodexAddHow = 'chatgpt' | 'custom' | 'json';

export type CodexDialog =
  | { kind: 'add'; how: CodexAddHow; error: string | null }
  /** Waiting for the browser; `id`: the account signing in again; `device`: with a code. */
  | { kind: 'wait'; id: string | null; started: number; device: boolean }
  | { kind: 'done'; id: string }
  | { kind: 'dup'; id: string }
  | { kind: 'fail'; reason: CodexLoginFail; detail: string | null; id: string | null }
  | { kind: 'provider' }
  | { kind: 'providerDone'; id: string }
  /** The files picked or the JSON pasted, read: what importing each sign-in would do. */
  | { kind: 'import'; preview: CodexImportPreview }
  | { kind: 'imported'; ids: string[]; removed: boolean; pasted: boolean }
  | { kind: 'detail'; id: string }
  | { kind: 'remove'; id: string }
  | { kind: 'relogin'; id: string }
  /** "Codex trên máy này"'s Chi tiết…: the technical detail and the restart. */
  | { kind: 'mac' };

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

  /** Sign in with the browser: a new account, or `id` signing in again; `device`: with a code typed on ChatGPT's page. */
  const login = async (id: string | null, device = false) => {
    setDialog({ kind: 'wait', id, started: Date.now(), device });
    const d = await call(api.codexLogin(id, device));
    if (d.kind === 'failed') return from('wait', d.reason === 'cancelled' ? null : { kind: 'fail', reason: d.reason, detail: d.detail, id });
    if (d.kind === 'refused') return from('wait', { kind: 'fail', reason: 'other', detail: refusedText(d.reason), id });
    if (d.kind === 'renewed') {
      from('wait', null);
      if (model) showFeedback(accountFeedback(d, model));
      return;
    }
    if (d.kind === 'added' || d.kind === 'dup') from('wait', { kind: d.kind === 'added' ? 'done' : 'dup', id: d.id });
  };

  /** The app's file picker; nothing picked leaves "Thêm tài khoản" open. */
  const pick = async () => {
    setPending(true);
    const p = await api.codexImportPick().catch(() => undefined);
    setPending(false);
    if (p === undefined) return from('add', { kind: 'add', how: 'json', error: refusedText('busy') });
    if (p) setDialog({ kind: 'import', preview: p });
  };

  /** "Đọc JSON đã dán": the same preview, of the JSON pasted; a refusal stays on "Thêm tài khoản". */
  const readText = async (text: string) => {
    setPending(true);
    const p = await api.codexImportText(text).catch((e: unknown) => (e === 'large' ? ('large' as const) : undefined));
    setPending(false);
    if (p === 'large') return from('add', { kind: 'add', how: 'json', error: t('JSON dài quá 1 MB.', 'The JSON is over 1 MB.') });
    if (!p) return from('add', { kind: 'add', how: 'json', error: refusedText('busy') });
    from('add', { kind: 'import', preview: p });
  };

  /** Imports what the preview showed. */
  const importFiles = async (token: string, removeSource: boolean, latest: () => Model | null) => {
    setPending(true);
    const d = await call(api.codexImport(token, removeSource));
    setPending(false);
    if (d.kind === 'imported') return from('import', { kind: 'imported', ids: d.ids, removed: d.removed, pasted: d.pasted });
    from('import', null);
    const m = latest();
    if (m) showFeedback(accountFeedback(d, m));
  };

  /** "Thêm tài khoản"'s Tiếp tục. */
  const add = (how: CodexAddHow) => {
    if (how === 'chatgpt') return void login(null);
    if (how === 'json') return void pick();
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
  const reopenLogin = () => void api.codexReopenLogin().catch(() => {});

  /** The actions on rows, banners and lines. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'codex:add') setDialog({ kind: 'add', how: 'chatgpt', error: null });
    else if (a.startsWith('codex:relogin:')) setDialog({ kind: 'relogin', id: a.slice('codex:relogin:'.length) });
    else return false;
    return true;
  };

  return { api, dialog, open: setDialog, close, pending, setPending, act, add, login, cancelLogin, reopenLogin, rename, remove, pick, readText, importFiles };
}

export type CodexAccountsState = ReturnType<typeof useCodexAccounts>;
