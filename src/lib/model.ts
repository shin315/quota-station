// The model the Rust side sends (`src-tauri/src/state.rs`, `quota-station-core::snapshot`).
// Field names are serde's camelCase; `Option` arrives as null.

export type LineKind = 'session' | 'week' | 'fableWeek' | 'gemini' | 'claudeGpt';

export interface Line {
  kind: LineKind;
  /** Percent left. */
  percent: number | null;
  resetsAt: string | null;
  /** Past its reset after the reading: no number until the next reading. */
  pending: boolean;
  /** `percent` is a lower bound: another window limits this account first. */
  atLeast: boolean;
}

export type Reading = 'fresh' | 'stale' | 'resetPending' | 'noData';
export type Role = 'team' | 'personal' | 'unknown';
export type Source = 'live' | 'claudeSwap' | 'cockpit';
export type Membership = 'in' | 'out' | 'ended' | 'unknown';

export interface Account {
  /** claude-swap slot or Cockpit account number; 0 for the only account a row reads. */
  number: number;
  role: Role;
  active: boolean;
  disabled: boolean;
  reading: Reading;
  fetchedAt: string | null;
  source: Source;
  membership: Membership;
  leavesAt: string | null;
  lines: Line[];
}

export type PoolKind = 'rotation' | 'active' | 'none';

export interface Room {
  number: number;
  percent: number;
}

export interface Pool {
  kind: PoolKind;
  /** Rotation: room left in fresh accounts. */
  room: number | null;
  /** 0-100, for the bar. */
  percent: number | null;
  line: LineKind | null;
  scope: string | null;
  members: number;
  rooms: Room[];
  nextRefillAt: string | null;
  refills: number[];
  exhausted: boolean;
  reason: string | null;
}

export type SectionStatus = 'ok' | 'noSidecar' | 'unavailable' | 'notSetUp' | 'unread';

export interface Section {
  status: SectionStatus;
  reason: string | null;
  accounts: Account[];
  pool: Pool;
  newestReadingAt: string | null;
  stale: boolean;
  /** `8-23` */
  personalHours: string | null;
}

export type Keychain = 'granted' | 'required' | 'unknown';

export interface Snapshot {
  generatedAt: string | null;
  claude: Section;
  agy: Section;
  keychain: Keychain;
}

export type ErrorKind = 'notConfigured' | 'notFound' | 'spawn' | 'timeout' | 'exit' | 'parse';

export interface ErrorView {
  kind: ErrorKind;
  detail: string;
}

export interface QuotaAxi {
  /** `~/…` for paths under home. */
  path: string | null;
  custom: boolean;
  exists: boolean;
  version: string | null;
  fork: boolean | null;
}

export interface Model {
  snapshot: Snapshot;
  errors: { claude: ErrorView | null; agy: ErrorView | null };
  reading: boolean;
  reads: number;
  machine: string;
  quotaAxi: QuotaAxi;
  granting: boolean;
}
