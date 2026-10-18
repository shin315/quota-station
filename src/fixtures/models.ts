// Models built from the snapshot fixtures that Rust generates
// (`crates/quota-station-core/tests/ui_fixtures.rs`). Synthetic data only.

import type { ErrorView, Model, PeerView, RotationView, Rules, Snapshot, SwitcherView } from '../lib/model';

const files = import.meta.glob('./snapshots/*.json', { eager: true, import: 'default' }) as Record<string, Snapshot>;

export const SNAPSHOTS: Record<string, Snapshot> = Object.fromEntries(
  Object.entries(files).map(([path, snap]) => [path.replace('./snapshots/', '').replace('.json', ''), snap]),
);

/** 40 seconds after the fixtures were generated (14:20:40 at UTC+7). */
export const FIXTURE_NOW = new Date('2026-01-16T07:20:40.000Z');

/**
 * The switcher on a Mac where all is well: the dry run a minute ago kept
 * Team A, and the last switch it logged was to Team A this morning.
 */
export function switcherFixture(over: Partial<SwitcherView> = {}): SwitcherView {
  return {
    installed: true,
    pause: null,
    dryRun: { decision: { kind: 'stay' }, raw: 'stay', at: '2026-01-16T07:19:40.000Z' },
    dryRunError: null,
    guard: null,
    lastLog: {
      at: '2026-01-16T02:12:05.000Z',
      event: { kind: 'switch', slot: 2 },
      raw: '2026-01-16 09:12:05 switch to slot 2 (ends for good at 17:45, use it first)',
    },
    service: true,
    autoService: false,
    ...over,
  };
}

/** Paused by a manual switch to Team A at 14:12, held until 15:20 (UTC+7). */
export function heldSwitcher(over: Partial<SwitcherView> = {}): SwitcherView {
  return switcherFixture({
    pause: { until: '2026-01-16T08:20:00.000Z', reason: 'chuyển tay sang tài khoản 2 lúc 14:12', holds: true },
    dryRun: {
      decision: { kind: 'paused', until: '2026-01-16T15:20', reason: 'chuyển tay sang tài khoản 2 lúc 14:12' },
      raw: 'paused until 2026-01-16T15:20: chuyển tay sang tài khoản 2 lúc 14:12',
      at: '2026-01-16T07:19:40.000Z',
    },
    guard: true,
    ...over,
  });
}

/** The rotation in the fixtures: Team A, B and C in turn, Cá nhân 08:00–24:00, Team A leaving at 17:45. */
export function rulesFixture(over: Partial<Rules> = {}): Rules {
  return {
    team: [2, 3, 4],
    personal: 1,
    personalFrom: 8,
    personalTo: 23,
    full: 90,
    fableFull: 95,
    expire: [{ slot: 2, at: '2026-01-16T17:45' }],
    ...over,
  };
}

/** Saved on this Mac at 14:02 (UTC+7); Mac B and Mac C have it. */
export const SAVED_AT = '2026-01-16T14:02:00+07:00';

export function peerFixture(name: string, alias: string, over: Partial<PeerView> = {}): PeerView {
  return { name, alias, state: { kind: 'synced', at: '2026-01-16T07:02:05.000Z' }, current: true, theirRules: null, pushing: false, ...over };
}

export function rotationFixture(over: Partial<RotationView> = {}): RotationView {
  return {
    path: '~/.config/cswap-prefer-team/rotation',
    status: 'ok',
    rules: rulesFixture(),
    reason: null,
    version: 'v1',
    stamp: { at: SAVED_AT, machine: 'Mac mini' },
    ignoredKeys: [],
    defaults: rulesFixture({ expire: [] }),
    saving: false,
    peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c')],
    ssh: true,
    ...over,
  };
}

export function fixtureModel(name: string, over: Partial<Model> = {}): Model {
  const snapshot = SNAPSHOTS[name];
  if (!snapshot) throw new Error('no snapshot fixture ' + name);
  return {
    snapshot: structuredClone(snapshot),
    errors: { claude: null, agy: null },
    reading: false,
    reads: 1,
    machine: 'Mac mini',
    machineDefault: 'Mac mini',
    quotaAxi: { path: '~/.local/bin/quota-axi', custom: false, exists: true, version: '0.1.61+cswap.4', fork: true },
    granting: false,
    switcher: switcherFixture(),
    claudeSwitch: { cswap: true, busy: null },
    rotation: rotationFixture(),
    ...over,
  };
}

export function failing(kind: ErrorView['kind'], detail = ''): Model['errors'] {
  return { claude: { kind, detail }, agy: { kind, detail } };
}
