// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Stands in for `src/antigravity/AgyAccounts.tsx` in a build without the
// Antigravity accounts (the `antigravity-accounts` feature, CC BY-NC-SA 4.0):
// `vite.config.ts` points `@agy/` here, so none of that code reaches the
// bundle. Nothing is logged in, so nothing is drawn. Only types come from the
// real module, and they are gone once compiled.

import type * as Real from '../antigravity/AgyAccounts';

export type { AgyLogins } from '../antigravity/AgyAccounts';

export const useAgyLogins: typeof Real.useAgyLogins = (view, api) => ({
  api,
  view,
  rows: [],
  busy: false,
  confirm: null,
  notice: null,
  login: () => {},
  verify: () => {},
  logout: () => {},
  ask: () => {},
});
export const loginChips: typeof Real.loginChips = () => [];
export const AddAccount: typeof Real.AddAccount = () => <></>;
export const LoginActions: typeof Real.LoginActions = () => <></>;
export const LogoutConfirm: typeof Real.LogoutConfirm = () => <></>;
export const LoginNotes: typeof Real.LoginNotes = () => <></>;
