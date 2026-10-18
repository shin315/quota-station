// Browser preview for `npm run dev`: the popover or the main window on a
// fixture, without Tauri. `?state=keychain&view=window&tab=claude&error=notFound`.
// Switching (M3): `switcher=held|noguard|auto|stopped|missing` sets the
// switcher, `outcome=raced|failed|stopped|unconfirmed` what a switch reports, and
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

/** The hold a switch writes: to the minute, as the pause flag keeps it. */
const holdOf = (until: string | null) =>
  until ? { until: new Date(Math.floor(Date.parse(until) / 60000) * 60000).toISOString(), failed: false } : null;

const OUTCOMES: Record<string, (slot: number, holdUntil: string | null) => Outcome> = {
  // The switcher puts back an account other than the one asked for.
  raced: (slot, holdUntil) => ({
    kind: 'raced',
    slot,
    active: slot === 2 ? 3 : 2,
    bySwitcherAt: '2026-01-16T07:21:00.000Z',
    hold: holdOf(holdUntil),
  }),
  failed: (slot) => ({ kind: 'failed', slot, error: 'refused', detail: 'ClaudeSwitchError: Account-' + slot + ' … needs /login', active: null }),
  // cswap timed out, and the account read again is still the one before.
  stopped: (slot) => ({ kind: 'failed', slot, error: 'timeout', detail: 'cswap did not answer within 60s', active: slot === 2 ? 3 : 2 }),
  unconfirmed: (slot, holdUntil) => ({ kind: 'unconfirmed', slot, hold: holdOf(holdUntil) }),
};

/** A table entry by query value, never one of Object.prototype's keys. */
function pick<T>(table: Record<string, T>, key: string | null): T | undefined {
  return key !== null && Object.hasOwn(table, key) ? table[key] : undefined;
}

export function previewSetup(params: URLSearchParams): { api: Api; view: 'popover' | 'window'; clock: () => Date } {
  const state = params.get('state') ?? 'fresh';
  const error = params.get('error') as ErrorView['kind'] | null;
  const tab = TABS.find((t) => t === params.get('tab')) ?? 'overview';
  const view = params.get('view') === 'window' ? 'window' : 'popover';
  const over: Partial<Model> = error ? { errors: failing(error) } : {};
  const switcher = pick(SWITCHERS, params.get('switcher'));
  if (switcher) over.switcher = switcher();
  if (params.get('cswap') === 'none') over.claudeSwitch = { cswap: false, busy: null };
  const { api } = fakeApi(fixtureModel(SNAPSHOTS[state] ? state : 'fresh', over), tab, {
    switchDelayMs: 1500,
    outcome: pick(OUTCOMES, params.get('outcome')),
  });
  // The window opens on the same fixture and switcher states as the popover.
  const openWindow = async (next: Tab) => {
    const query = new URLSearchParams(params);
    query.set('view', 'window');
    query.set('tab', next);
    window.open('?' + query.toString(), '_blank');
  };
  return { api: view === 'popover' ? { ...api, openWindow } : api, view, clock: () => FIXTURE_NOW };
}
