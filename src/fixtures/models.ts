// Models built from the snapshot fixtures that Rust generates
// (`crates/quota-station-core/tests/ui_fixtures.rs`). Synthetic data only.

import type { ErrorView, Model, Snapshot, SwitcherView } from '../lib/model';

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
    ...over,
  };
}

export function failing(kind: ErrorView['kind'], detail = ''): Model['errors'] {
  return { claude: { kind, detail }, agy: { kind, detail } };
}
