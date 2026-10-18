// Models built from the snapshot fixtures that Rust generates
// (`crates/quota-station-core/tests/ui_fixtures.rs`). Synthetic data only.

import type { ErrorView, Model, Snapshot } from '../lib/model';

const files = import.meta.glob('./snapshots/*.json', { eager: true, import: 'default' }) as Record<string, Snapshot>;

export const SNAPSHOTS: Record<string, Snapshot> = Object.fromEntries(
  Object.entries(files).map(([path, snap]) => [path.replace('./snapshots/', '').replace('.json', ''), snap]),
);

/** 40 seconds after the fixtures were generated (14:20:40 at UTC+7). */
export const FIXTURE_NOW = new Date('2026-01-16T07:20:40.000Z');

export function fixtureModel(name: string, over: Partial<Model> = {}): Model {
  const snapshot = SNAPSHOTS[name];
  if (!snapshot) throw new Error('no snapshot fixture ' + name);
  return {
    snapshot: structuredClone(snapshot),
    errors: { claude: null, agy: null },
    reading: false,
    reads: 1,
    machine: 'Mac mini',
    quotaAxi: { path: '~/.local/bin/quota-axi', custom: false, exists: true, version: '0.1.61+cswap.4', fork: true },
    granting: false,
    ...over,
  };
}

export function failing(kind: ErrorView['kind'], detail = ''): Model['errors'] {
  return { claude: { kind, detail }, agy: { kind, detail } };
}
