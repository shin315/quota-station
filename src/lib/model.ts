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
  /** When quota-axi wrote the report behind this section. */
  generatedAt: string | null;
  newestReadingAt: string | null;
  stale: boolean;
  /** `8-23` */
  personalHours: string | null;
}

export type Keychain = 'granted' | 'required' | 'unknown';

export interface Snapshot {
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

// ===== The switcher and manual switching (M3) =====

/** Why the switcher would switch (`quota-station-core::switcher::Why`). Times are the switcher's local `HH:MM`. */
export type Why =
  | { kind: 'expiring'; at: string }
  | { kind: 'early'; at: string; left: number; before: number }
  | { kind: 'room'; used: number; ends: string | null }
  | { kind: 'personal'; used: number }
  | { kind: 'park'; at: string }
  | { kind: 'other' };

/** The switcher's `--dry-run` decision line, in parts. */
export type Decision =
  | { kind: 'stay' }
  | { kind: 'unknown'; slot: number }
  | { kind: 'waiting'; slot: number; at: string }
  | { kind: 'switch'; slot: number; why: Why }
  | { kind: 'wait' }
  | { kind: 'paused'; until: string | null; reason: string | null }
  | { kind: 'other' };

export type LogEvent =
  | { kind: 'switch'; slot: number }
  | { kind: 'switchFailed'; slot: number }
  | { kind: 'primed'; slot: number }
  | { kind: 'pauseEnded' }
  | { kind: 'other' };

export interface LogLine {
  /** RFC 3339. */
  at: string | null;
  event: LogEvent;
  raw: string;
}

export interface PauseView {
  /** RFC 3339; null pauses until the owner resumes. */
  until: string | null;
  reason: string | null;
  /** The flag held when the model was built. */
  holds: boolean;
}

export interface SwitcherView {
  /** `cswap-prefer-team` is on this Mac. */
  installed: boolean;
  /** The pause flag; null when there is none. */
  pause: PauseView | null;
  dryRun: { decision: Decision; raw: string; at: string } | null;
  dryRunError: string | null;
  /** False: the switcher decided as if not paused while the flag held, so it does not read the flag yet. */
  guard: boolean | null;
  lastLog: LogLine | null;
  /** `com.cswap.prefer-team` is loaded. */
  service: boolean | null;
  /** `com.cswap.auto`, claude-swap's own switcher, is loaded. */
  autoService: boolean | null;
}

export interface ClaudeSwitchView {
  /** claude-swap is on this Mac. */
  cswap: boolean;
  /** The slot a switch is running for. */
  busy: number | null;
}

export interface Sessions {
  live: number;
  unreadable: number;
}

export interface Hold {
  /** RFC 3339. */
  until: string;
  /** The pause flag could not be written. */
  failed: boolean;
}

/** What a switch really did (`quota-station-core::switching::Outcome`). */
export type Outcome =
  | { kind: 'switched'; slot: number; hold: Hold | null }
  | { kind: 'raced'; slot: number; active: number; bySwitcherAt: string | null; hold: Hold | null }
  | { kind: 'unconfirmed'; slot: number; hold: Hold | null }
  /**
   * `active`: the account read again when cswap stopped without an answer (timed out, or ended
   * without one). Null when cswap answered, and when that re-read found no active account.
   */
  | { kind: 'failed'; slot: number; error: string; detail: string; active: number | null };

// ===== The rotation file and the other Macs (M5) =====

/** The six keys of the rotation file (`quota-station-core::rotation::Rules`). */
export interface Rules {
  /** Team slots in the switcher's order. */
  team: number[];
  personal: number | null;
  /** `personal_hours = 8-23`: hour 8 through hour 23, both included. */
  personalFrom: number;
  personalTo: number;
  /** Percent of a window used at which the switcher moves on. */
  full: number;
  fableFull: number;
  /** `at` as written: `2026-10-16T17:45`, local time. */
  expire: { slot: number; at: string }[];
}

/** The `# updated = <time> by <machine>` line. */
export interface Stamp {
  /** RFC 3339 with the writing Mac's offset. */
  at: string;
  machine: string;
}

/** Where another Mac stands with this Mac's file (`quota-station-core::peers::PeerState`). Times are RFC 3339. */
export type PeerState =
  | { kind: 'synced'; at: string }
  | { kind: 'unreachable'; error: string; retryAt: string }
  | { kind: 'mismatch'; retryAt: string }
  | { kind: 'failed'; error: string; retryAt: string }
  | { kind: 'newer'; their: Stamp }
  /** `their` is null for a copy with no `updated` line, or one the app cannot read. */
  | { kind: 'edited'; their: Stamp | null };

export interface PeerView {
  name: string;
  alias: string;
  /** Null: nothing sent to it yet. */
  state: PeerState | null;
  /** The state is about this Mac's file as it is now. */
  current: boolean;
  /** Its rules, when it holds another copy. */
  theirRules: Rules | null;
  pushing: boolean;
}

export interface RotationView {
  /** `~/.config/cswap-prefer-team/rotation` */
  path: string;
  status: 'ok' | 'missing' | 'invalid' | 'unreadable';
  rules: Rules | null;
  /** quota-axi's reason for an invalid file (`rotation_bad_value:full`), or the read error. */
  reason: string | null;
  /** Sent back with a save, so a file changed meanwhile is not overwritten. */
  version: string | null;
  stamp: Stamp | null;
  ignoredKeys: string[];
  defaults: Rules;
  saving: boolean;
  peers: PeerView[];
  /** An `ssh` to copy the file with. */
  ssh: boolean;
}

/** Why a rotation command did nothing, or put the file back. */
export interface RulesError {
  kind: 'busy' | 'changed' | 'invalid' | 'write' | 'refused' | 'unchecked' | 'unreachable' | 'noFile' | 'unknownMac';
  detail: string | null;
  /** The file before is back on disk. */
  restored: boolean;
}

/** What "Thử SSH" found. */
export interface PeerTest {
  reached: boolean;
  error: string | null;
  hasFile: boolean;
  stamp: Stamp | null;
}

export interface Peer {
  name: string;
  alias: string;
}

/** The main window's tabs in this build (`src-tauri/src/state.rs` `Tab`). */
export type Tab = 'overview' | 'claude' | 'agy' | 'rules' | 'settings';

export interface Model {
  snapshot: Snapshot;
  errors: { claude: ErrorView | null; agy: ErrorView | null };
  reading: boolean;
  reads: number;
  /** The name shown for this Mac: set in settings, else the one macOS gives. */
  machine: string;
  /** The name macOS gives this Mac. */
  machineDefault: string;
  quotaAxi: QuotaAxi;
  granting: boolean;
  switcher: SwitcherView;
  claudeSwitch: ClaudeSwitchView;
  rotation: RotationView;
}
