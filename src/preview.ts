// Browser preview for `npm run dev`: the popover or the main window on a
// fixture, without Tauri. `?state=keychain&view=window&tab=claude&error=notFound`.
// Switching (M3): `switcher=held|noguard|auto|stopped|missing` sets the
// switcher, `outcome=raced|failed|stopped|unconfirmed` what a switch reports, and
// `cswap=none` hides claude-swap. A switch takes 1.5 s, so the working sheet shows.
// Rotation rules (M5): `tab=rules` with `rotation=unreachable|newer|edited|unstamped|missing|invalid|refused|single`;
// a save takes 1 s, and `unreachable` leaves Mac C behind.
// Antigravity logins (M4a) and reads (M4b): `agy=two|relogin|verify|noclient|tooopen|invalid|broken|reading|failing|forbidden`
// sets the login card, `login=timeout|noRefreshToken|keychain|…` how a login ends
// (it keeps the next account after 3 s by default, or at once on any pasted
// URL with `code=` and `state=`), and `email=on` shows the made-up emails.
// Switching Antigravity (M4c): `agyswitch=on|overwritten` lists the two
// accounts as the app's (the second one undone by Antigravity.app with
// `overwritten`), `agyapp=closed|missing|old|unreadable|unknown` is what the sheet learns
// about Antigravity.app (it runs by default), and
// `agyoutcome=unconfirmed|notquit|write` what a switch reports. A switch takes 1.5 s.
// History, alerts and Firstmate (M6): `tab=history|alerts|firstmate`, with
// `history=empty`, `alerts=on|off|unbundled|empty` (two cards by default on
// `tab=alerts`; `off` is macOS refusing the cards) and
// `firstmate=ok|holding|stale|missing|unset` (unset by default).

import type { Api } from './lib/api';
import type { AgyAccounts, AgyApp, AgyHow, AgyOutcome, ErrorView, Model, Outcome, Tab } from './lib/model';
import { fakeApi, fakeEmail } from './fixtures/fakeApi';
import {
  agyAccount,
  agyFixture,
  failing,
  FIXTURE_NOW,
  fixtureModel,
  heldSwitcher,
  peerFixture,
  rotationFixture,
  rulesFixture,
  SNAPSHOTS,
  switcherFixture,
  alertsFixture,
  firstmateFixture,
  historyFixture,
  leavingAlert,
  overwriteAlert,
} from './fixtures/models';

const TABS: Tab[] = ['overview', 'claude', 'agy', 'rules', 'history', 'alerts', 'firstmate', 'settings'];

const ALERTS: Record<string, () => Model['alerts']> = {
  on: () => alertsFixture({ recent: [overwriteAlert(), leavingAlert()], unanswered: 2 }),
  off: () => alertsFixture({ recent: [overwriteAlert(), leavingAlert()], unanswered: 2, permission: 'denied' }),
  unbundled: () => alertsFixture({ recent: [overwriteAlert(), leavingAlert()], unanswered: 2, permission: 'unbundled' }),
  empty: () => alertsFixture(),
};

const FIRSTMATES: Record<string, () => Model['firstmate']> = {
  ok: () => firstmateFixture(),
  holding: () =>
    firstmateFixture({ line: { ...firstmateFixture().line!, dry: true, nextReset: '2026-01-16T09:40:00.000Z' }, ownDry: true }),
  stale: () => firstmateFixture({ writtenAt: '2026-01-16T05:20:00.000Z', fresh: false }),
  missing: () => firstmateFixture({ status: 'missing', line: null, writtenAt: null, fresh: false }),
  unset: () => fixtureModel('fresh').firstmate,
};

const RETRY = '2026-01-16T07:21:40.000Z';
const NEWER = { at: '2026-01-16T14:11:00+07:00', machine: 'Mac B' };

const ROTATIONS: Record<string, () => Model['rotation']> = {
  unreachable: () =>
    rotationFixture({
      stamp: { at: '2026-01-16T14:16:00+07:00', machine: 'Mac mini' },
      peers: [
        peerFixture('Mac B', 'mac-b', { state: { kind: 'synced', at: '2026-01-16T07:16:04.000Z' } }),
        peerFixture('Mac C', 'mac-c', { state: { kind: 'unreachable', error: 'ssh: connect to host mac-c port 22: Operation timed out', retryAt: RETRY } }),
      ],
    }),
  unstamped: () =>
    rotationFixture({
      peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'edited', their: null }, theirRules: null })],
    }),
  newer: () =>
    rotationFixture({
      peers: [peerFixture('Mac B', 'mac-b', { state: { kind: 'newer', their: NEWER }, theirRules: rulesFixture({ full: 88, personalTo: 21 }) }), peerFixture('Mac C', 'mac-c')],
    }),
  edited: () =>
    rotationFixture({
      peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'edited', their: { at: '2026-01-16T14:02:00+07:00', machine: 'Mac mini' } }, theirRules: rulesFixture({ team: [3, 4] }) })],
    }),
  missing: () => rotationFixture({ status: 'missing', rules: null, version: null, stamp: null, peers: [peerFixture('Mac B', 'mac-b', { state: null, current: false }), peerFixture('Mac C', 'mac-c', { state: null, current: false })] }),
  invalid: () => rotationFixture({ status: 'invalid', rules: null, reason: 'rotation_bad_value:personal_hours', stamp: null }),
  single: () => rotationFixture({ peers: [] }),
};

const SWITCHERS: Record<string, () => Model['switcher']> = {
  held: () => heldSwitcher(),
  noguard: () => heldSwitcher({ dryRun: { decision: { kind: 'stay' }, raw: 'stay', at: '2026-01-16T07:19:40.000Z' }, guard: false }),
  auto: () => switcherFixture({ autoService: true }),
  stopped: () => switcherFixture({ service: false }),
  missing: () => switcherFixture({ installed: false, dryRun: null, lastLog: null, service: null }),
};

const AGY: Record<string, () => AgyAccounts> = {
  two: () => agyFixture({ accounts: [agyAccount(1), agyAccount(2)] }),
  relogin: () => agyFixture({ accounts: [agyAccount(1), agyAccount(2, { status: 'needsLogin' })] }),
  verify: () => agyFixture({ accounts: [agyAccount(1, { status: 'needsVerification', verify: true }), agyAccount(2)] }),
  noclient: () => agyFixture({ client: 'missing' }),
  tooopen: () => agyFixture({ client: 'tooOpen' }),
  invalid: () => agyFixture({ client: 'invalid' }),
  broken: () => agyFixture({ indexError: true }),
  reading: () => agyFixture({ accounts: [agyAccount(1, { reading: true }), agyAccount(2)] }),
  failing: () =>
    agyFixture({
      accounts: [
        agyAccount(1, { readError: { kind: 'network', detail: '', retryAt: null } }),
        agyAccount(2, { readError: { kind: 'rateLimited', detail: '', retryAt: '2026-01-16T07:24:00.000Z' } }),
      ],
    }),
  forbidden: () => agyFixture({ accounts: [agyAccount(1), agyAccount(2, { status: 'forbidden', verify: true })] }),
};

const LOGIN_ENDS = ['timeout', 'cancelled', 'noCode', 'stateMismatch', 'noRefreshToken', 'invalidGrant', 'network', 'keychain', 'index', 'browser'];

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

const AGY_APPS: Record<string, AgyApp | null> = {
  closed: { installed: 'supported', version: '2.24.0', running: false },
  missing: { installed: 'missing', version: null, running: false },
  old: { installed: 'tooOld', version: '1.20.5', running: true },
  unreadable: { installed: 'unreadable', version: null, running: true },
  unknown: null,
};

const AGY_OUTCOMES: Record<string, (number: number, how: AgyHow) => AgyOutcome> = {
  // agy still reads the account from before.
  unconfirmed: (number, how) => ({ kind: 'unconfirmed', number, how, active: number === 1 ? 2 : 1, reopened: how === 'quitFirst' ? true : null, saved: true }),
  notquit: (number) => ({ kind: 'failed', number, error: 'notQuit', detail: '', reopened: null }),
  write: (number, how) => ({
    kind: 'failed',
    number,
    error: 'write',
    detail: 'security exited 36: User interaction is not allowed.',
    reopened: how === 'quitFirst' ? true : null,
  }),
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
  const rotation = params.get('rotation');
  const rules = pick(ROTATIONS, rotation);
  if (rules) over.rotation = rules();
  const agy = pick(AGY, params.get('agy'));
  if (agy) over.antigravity = agy();
  const agySwitch = params.get('agyswitch');
  if (agySwitch === 'on' || agySwitch === 'overwritten') {
    if (!over.antigravity?.accounts.length) over.antigravity = AGY.two();
    over.agySwitch = {
      busy: null,
      listed: true,
      overwritten: agySwitch === 'overwritten' ? { to: 1, from: 2, at: '2026-01-16T07:10:00.000Z', overwrites: 1 } : null,
    };
  }
  if (params.get('email') === 'on') {
    over.showEmail = true;
    if (over.antigravity) over.antigravity.accounts = over.antigravity.accounts.map((a) => ({ ...a, email: fakeEmail(a.number) }));
  }
  const alerts = pick(ALERTS, params.get('alerts') ?? (tab === 'alerts' ? 'on' : null));
  if (alerts) over.alerts = alerts();
  const firstmate = pick(FIRSTMATES, params.get('firstmate'));
  if (firstmate) over.firstmate = firstmate();
  const agyApp = params.get('agyapp');
  const ends = LOGIN_ENDS.find((k) => k.toLowerCase() === params.get('login')?.toLowerCase());
  const { api } = fakeApi(fixtureModel(SNAPSHOTS[state] ? state : 'fresh', over), tab, {
    switchDelayMs: 1500,
    outcome: pick(OUTCOMES, params.get('outcome')),
    saveDelayMs: 1000,
    clock: () => FIXTURE_NOW,
    unreachable: rotation === 'unreachable' ? ['mac-c'] : [],
    rulesError: rotation === 'refused' ? { kind: 'refused', detail: 'rotation_bad_value:personal_hours', restored: true } : undefined,
    agyLoginDelayMs: 3000,
    agyLogin: ends ? () => ({ kind: ends, detail: '' }) : undefined,
    agyApp: agyApp !== null && Object.hasOwn(AGY_APPS, agyApp) ? AGY_APPS[agyApp] : undefined,
    agyOutcome: pick(AGY_OUTCOMES, params.get('agyoutcome')),
    agySwitchDelayMs: 1500,
    history: params.get('history') === 'empty' ? [] : historyFixture(),
    firstmate: firstmateFixture(),
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
