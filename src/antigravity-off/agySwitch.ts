// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Stands in for `src/antigravity/agySwitch.ts` in a build without the
// Antigravity accounts (see `AgyAccounts.tsx` here): Antigravity is the one
// account quota-axi reads, so there is nothing to switch to, no sheet and no
// overwrite to tell. Only types come from the real module.

import type * as Real from '../antigravity/agySwitch';

export type { AppAnswer } from '../antigravity/agySwitch';

export const agyLoginOf: typeof Real.agyLoginOf = () => null;
export const agySheet: typeof Real.agySheet = () => null;
export const agyWorking: typeof Real.agyWorking = () => '';
export const overwriteBanner: typeof Real.overwriteBanner = () => null;

const nothing = { tone: 'bad' as const, title: 'Bản này không chuyển tài khoản Antigravity', body: '' };
export const agySwitchFeedback: typeof Real.agySwitchFeedback = () => nothing;
export const agySwitchErrorFeedback: typeof Real.agySwitchErrorFeedback = () => nothing;
