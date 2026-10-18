// Browser preview for `npm run dev`: the popover or the main window on a
// fixture, without Tauri. `?state=keychain&view=window&tab=claude&error=notFound`.
// Switching (M3): `switcher=held|noguard|auto|stopped|missing` sets the
// switcher, `outcome=raced|failed|unconfirmed` what a switch reports, and
// `cswap=none` hides claude-swap. A switch takes 1.5 s, so the working sheet shows.

import type { Api } from './lib/api';
import type { ErrorView, Model, Outcome, Tab } from './lib/model';
import { fakeApi } from './fixtures/fakeApi';
import { failing, FIXTURE_NOW, fixtureModel, heldSwitcher, SNAPSHOTS, switcherFixture } from './fixtures/models';

const TABS: Tab[] = ['overview', 'claude', 'agy', 'settings'];

const SWITCHERS: Record<string, () => Model['switcher']> = {
  held: () => heldSwitcher(),
  noguard: () => heldSwitcher({ dryRun: { decision: { kind: 'stay' }, raw: 'stay', at: '2026-01-16T07:19:40.000Z' }, guard: false }),
  auto: () => switcherFixture({ autoService: true }),
  stopped: () => switcherFixture({ service: false }),
  missing: () => switcherFixture({ installed: false, dryRun: null, lastLog: null, service: null }),
};

const OUTCOMES: Record<string, (slot: number, holdUntil: string | null) => Outcome> = {
  raced: (slot) => ({ kind: 'raced', slot, active: 2, bySwitcherAt: '2026-01-16T07:21:00.000Z', hold: null }),
  failed: (slot) => ({ kind: 'failed', slot, error: 'refused', detail: 'ClaudeSwitchError: Account-' + slot + ' … needs /login' }),
  unconfirmed: (slot) => ({ kind: 'unconfirmed', slot, hold: null }),
};

export function previewSetup(params: URLSearchParams): { api: Api; view: 'popover' | 'window'; clock: () => Date } {
  const state = params.get('state') ?? 'fresh';
  const error = params.get('error') as ErrorView['kind'] | null;
  const tab = TABS.find((t) => t === params.get('tab')) ?? 'overview';
  const view = params.get('view') === 'window' ? 'window' : 'popover';
  const over: Partial<Model> = error ? { errors: failing(error) } : {};
  const switcher = SWITCHERS[params.get('switcher') ?? ''];
  if (switcher) over.switcher = switcher();
  if (params.get('cswap') === 'none') over.claudeSwitch = { cswap: false, busy: null };
  const { api } = fakeApi(fixtureModel(SNAPSHOTS[state] ? state : 'fresh', over), tab, {
    switchDelayMs: 1500,
    outcome: OUTCOMES[params.get('outcome') ?? ''],
  });
  const openWindow = async (next: Tab) => void window.open('?view=window&tab=' + next + '&state=' + state, '_blank');
  return { api: view === 'popover' ? { ...api, openWindow } : api, view, clock: () => FIXTURE_NOW };
}
