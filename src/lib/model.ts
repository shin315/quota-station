// The model the Rust side sends (`src-tauri/src/state.rs`, `quota-station-core::snapshot`).
// Field names are serde's camelCase; `Option` arrives as null.

export type LineKind =
  | 'session'
  | 'week'
  | 'fableWeek'
  | 'gemini'
  | 'claudeGpt'
  /** Antigravity's four windows, each on its own (`Account.windows`). */
  | 'geminiSession'
  | 'geminiWeek'
  | 'claudeGptSession'
  | 'claudeGptWeek';

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
  /** quota-axi's numbers: the tighter window only, when it reports one. */
  lines: Line[];
  /** Every window on its own, from claude-swap's list (Claude) or the app's own reading (Antigravity); empty when neither has it. */
  windows: Line[];
  /** Only while the owner shows names. */
  email?: string;
  /** The Claude account's organization, only while the owner shows names. */
  org?: string;
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
  /** Who read the Antigravity accounts quota-axi lists: `quota-station` (this app) or Cockpit Tools. */
  source: string | null;
}

export type Keychain = 'granted' | 'required' | 'unknown';

export interface Snapshot {
  claude: Section;
  agy: Section;
  /** Codex's own ChatGPT login (`quota-axi --provider codex`): one account, five hours and a week. */
  codex: Section;
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

// ===== Antigravity accounts the app logs in (M4a) and reads (M4b) =====

/** What the app found in `oauth-client.json` (`quota-station-core::antigravity::client`). */
export type ClientStatus = 'ok' | 'missing' | 'unreadable' | 'tooOpen' | 'invalid';

export type AgyStatus = 'ok' | 'needsLogin' | 'needsVerification' | 'forbidden';

/** The app's last read of an account failed (`src-tauri/src/antigravity.rs` `ReadErrorView`). */
export interface AgyReadError {
  /**
   * `quota::ReadError::kind`: `network`, `unauthorized`, `rateLimited`, `server`, `rejected`,
   * `verification`, `parse`, `keychain`, `client`, `refresh`, `cache` or `index`.
   */
  kind: string;
  detail: string;
  /** After a 429 or a 5xx: when the app reads it again (RFC 3339). */
  retryAt: string | null;
}

/** One account logged in here (`src-tauri/src/antigravity.rs` `AccountView`). */
export type Theme = 'auto' | 'light' | 'dark';
export type Menubar = 'claude' | 'gemini' | 'codex';
/** The window the menu bar title shows; one the service lacks falls back to five hours, then the tightest. */
export type MenubarWindow = 'fiveHours' | 'week' | 'fableWeek' | 'tightest';

export interface AgyLogin {
  id: string;
  /** Its place in the app's list, from 1, as quota-axi numbers it. */
  number: number;
  /** Only when the owner turned emails on. */
  email: string | null;
  name: string | null;
  status: AgyStatus;
  /** Google sent a verification page for it. */
  verify: boolean;
  /** The app is reading its quota now. */
  reading: boolean;
  /** The app's last read failed; quota-axi keeps the last numbers, with their age. */
  readError: AgyReadError | null;
}

export interface AgyAccounts {
  client: ClientStatus;
  /** Where `oauth-client.json` goes, `~/…`. */
  clientPath: string;
  accounts: AgyLogin[];
  /** `accounts.json` is there and cannot be read. */
  indexError: boolean;
  /** A login is waiting for the browser. */
  loggingIn: boolean;
}

/** A login that kept an account. */
export interface AgyLoginDone {
  number: number;
  /** A new account, not a new login of one already listed. */
  added: boolean;
}

/** A login that kept nothing: `LoginError::kind` in Rust, plus `busy` and `spawn`. */
export interface AgyLoginFailure {
  kind: string;
  detail: string;
}

// ===== Switching the Antigravity account (M4c) =====

/** The button pressed on the sheet (`quota-station-core::antigravity::switch::How`). */
export type AgyHow = 'plain' | 'quitFirst' | 'keepRunning';

/** Antigravity.app as the sheet sees it (`src-tauri/src/agy_switch.rs` `AppView`). */
export interface AgyApp {
  /** `unreadable`: there, but its version could not be read; the sheet offers no switch. */
  installed: 'missing' | 'supported' | 'tooOld' | 'unreadable';
  version: string | null;
  /** Null when the process list could not be read. */
  running: boolean | null;
}

/** What an Antigravity switch did (`quota-station-core::antigravity::switch::Outcome`). */
export type AgyOutcome =
  /**
   * `basis`: quota-axi's `identity` when agy's login named the account,
   * `reading` when agy's own numbers showed it, `cockpit` when only the
   * binding the switch wrote answered.
   */
  | { kind: 'switched'; number: number; how: AgyHow; basis: string | null; reopened: boolean | null; saved: boolean }
  /** The login was written; the check showed `active` in use, or could not tell (null). */
  | { kind: 'unconfirmed'; number: number; how: AgyHow; active: number | null; reopened: boolean | null; saved: boolean }
  /** Nothing switched. `reopened`: Antigravity.app was quit for the switch and opened again (or not). */
  | { kind: 'failed'; number: number; error: string; detail: string; reopened: boolean | null };

/** A switch a running Antigravity.app undid (the six-hour watch). */
export interface AgyOverwrite {
  /** The account switched to, and the one back in use. */
  to: number;
  from: number;
  /** RFC 3339. */
  at: string;
  overwrites: number;
}

export interface AgySwitchView {
  /** The account number a switch is running for. */
  busy: number | null;
  /** quota-axi lists this app's accounts, so a row's number is the account's number here. */
  listed: boolean;
  overwritten: AgyOverwrite | null;
}

// ===== History, alerts and Firstmate (M6) =====

/** Where a history line comes from (`quota-station-core::history::Source`). */
export type HistorySource = 'app' | 'switcher' | 'observed';

export type HistoryCause =
  /** The owner switched Claude in this app; `result` is what the check after it found. */
  | { kind: 'manual'; holdUntil: string | null; result: 'switched' | 'raced' | 'unconfirmed' | string }
  | { kind: 'switcher'; why: Why; raw: string }
  /** The switcher started a five-hour window early. */
  | { kind: 'primed'; raw: string }
  /** The account in use changed between two readings, and nothing here or in the switcher's log switched it. */
  | { kind: 'observed' }
  | { kind: 'agySwitch'; how: AgyHow; result: 'switched' | 'unconfirmed' | string; reopened: boolean | null }
  /** Antigravity.app wrote the old login back after a switch. */
  | { kind: 'agyOverwrite'; overwrites: number }
  /** A Codex switch from this app (M8), by provider id: `openai` is the ChatGPT login. */
  | { kind: 'codexSwitch'; from: string; to: string; restarted: boolean | null }
  /** `config.toml` chose another provider and the app did not write it. */
  | { kind: 'codexObserved'; from: string; to: string };

/** One line of the History tab (`quota-station-core::history::Entry`). */
export interface HistoryEntry {
  /** RFC 3339. */
  at: string;
  app: 'claude' | 'antigravity' | 'codex';
  source: HistorySource;
  from: number | null;
  /** Null for an Antigravity account no longer logged in here. */
  to: number | null;
  cause: HistoryCause;
}

export type AlertKind = 'pool' | 'leaving' | 'login' | 'overwrite' | 'firstmate';
export type AlertTone = 'ok' | 'warn' | 'bad' | 'info';

export interface AlertAct {
  /** `dismiss`, `snooze`, `switch:N`, `agy:keep`, `agy:<how>:N`, `agy:login` or `agy:verify:N`. */
  id: string;
  label: string;
}

/** One alert, as the macOS card shows it (`quota-station-core::alerts::Alert`). */
export interface Alert {
  key: string;
  kind: AlertKind;
  tone: AlertTone;
  title: string;
  body: string;
  actions: AlertAct[];
  /** Unix seconds. */
  at: number;
  /** Sent with "Gửi thử": its buttons change nothing. */
  test?: boolean;
}

export interface AlertSaid {
  tone: AlertTone;
  title: string;
  body: string;
}

export interface AlertAnswer {
  action: string;
  label: string;
  /** Unix seconds. */
  at: number;
  /** Null while a switch back runs. */
  result: AlertSaid | null;
}

export interface AlertRecord {
  /** The card's id, the same on the macOS card. */
  id: string;
  alert: Alert;
  /** macOS took the card. */
  notified: boolean;
  answer: AlertAnswer | null;
}

export type AlertSwitches = Record<AlertKind, boolean>;

/** `unbundled`: the app does not run as a packaged `.app`, so macOS shows no cards. */
export type NotifyPermission = 'unknown' | 'allowed' | 'denied' | 'unbundled';

export interface AlertsView {
  /** Newest first. */
  recent: AlertRecord[];
  switches: AlertSwitches;
  /** Cards with buttons nobody pressed yet. */
  unanswered: number;
  permission: NotifyPermission;
}

/** Firstmate's `quota-pool.last`, in parts (`quota-station-core::firstmate::PoolLine`). */
export interface PoolLine {
  fork: boolean | null;
  autoSwitcher: boolean | null;
  claudeSwap: boolean | null;
  active: number | null;
  /** `pool=dry`: Firstmate holds new Claude work. */
  dry: boolean | null;
  slots: { slot: number; percent: number | null; reset: boolean }[];
  nextReset: string | null;
}

export interface FirstmateView {
  /** `~/…`; null when not set. */
  file: string | null;
  status: 'notSet' | 'ok' | 'missing' | 'notPlain' | 'notPoolFile' | 'unreadable';
  line: PoolLine | null;
  /** When Firstmate last wrote it, RFC 3339. */
  writtenAt: string | null;
  /** Written within 30 minutes: Firstmate is running. */
  fresh: boolean;
  /** The same dry rule, counted by the app from quota-axi's numbers. */
  ownDry: boolean | null;
}

/** The app as a login item (`quota-station-core::login_item::Status`). */
export type LaunchAtLogin = 'off' | 'on' | 'needsApproval' | 'unavailable';

/** Where "Cập nhật từ mã nguồn" is (`quota-station-core::update::Step`). */
export type UpdateStep = 'pulling' | 'installing';

export type UpdateOutcome =
  /** The clone had nothing new, and this copy was built from it. */
  | { kind: 'upToDate'; commit: string }
  /** The new copy is in place; it runs once the app opens again. */
  | { kind: 'installed'; commit: string; version: string | null };

export type UpdateError =
  | { kind: 'source'; reason: 'missing' | 'notSource' }
  /** `git pull --ff-only` refused or failed: its last line. */
  | { kind: 'pull'; detail: string }
  /** `scripts/install.sh` failed: its own error line, or its last line. */
  | { kind: 'install'; detail: string }
  | { kind: 'timeout'; step: UpdateStep }
  | { kind: 'spawn'; detail: string };

/** The app itself (`src-tauri/src/install.rs` `AppView`, M7). */
export interface AppView {
  version: string;
  /** The commit it was built from, 7 characters, `-dirty` kept; null when not built by `scripts/install.sh`. */
  commit: string | null;
  launchAtLogin: LaunchAtLogin;
  /** The clone to update from, `~/…`; null when not set. */
  source: string | null;
  sourceStatus: 'notSet' | 'ok' | 'missing' | 'notSource';
  /** Runs from an installed `.app`, so an update has somewhere to go. */
  bundled: boolean;
  updating: UpdateStep | null;
  outcome: UpdateOutcome | null;
  error: UpdateError | null;
  /** The last install's output, `~/…`. */
  log: string;
}

/** The main window's tabs in this build (`src-tauri/src/state.rs` `Tab`). */
export type Tab = 'overview' | 'claude' | 'agy' | 'codex' | 'rules' | 'history' | 'alerts' | 'firstmate' | 'mcp' | 'backup' | 'settings';

// ===== Codex (M8) =====

/** Where the custom provider's credentials come from (`codex::config::KeyKind`); never the key itself. */
export type CodexKey = 'bearer' | 'envKey' | 'headers' | 'placeholder' | 'none';

/** The custom provider table in `config.toml` a switch goes to. */
export interface CodexCustom {
  id: string;
  /** Its `name`, else the id. */
  name: string;
  baseUrl: string | null;
  key: CodexKey;
  /** The variable `env_key` names. */
  envKey: string | null;
  requiresOpenaiAuth: boolean | null;
  /** A plain `[model_providers.<id>]` table; other shapes are not edited. */
  plain: boolean;
}

/** Why a switch cannot run (`codex::config::Refusal`). */
export type CodexRefusal =
  | { kind: 'noCustom' }
  | { kind: 'noKey' }
  | { kind: 'placeholder' }
  | { kind: 'profile'; profile: string; key: string }
  | { kind: 'shape' };

/** `codex::login::Login`: the shape of `auth.json`, never its values. */
export type CodexLogin = 'chatGpt' | 'other' | 'apiKey' | 'missing' | 'unreadable' | 'keychain' | 'ephemeral';

export interface CodexMcp {
  name: string;
  kind: 'stdio' | 'http' | 'other';
  enabled: boolean;
}

export interface CodexReach {
  state: 'unknown' | 'up' | 'down';
  ms: number | null;
  /** When it stopped answering. */
  since: string | null;
  checkedAt: string | null;
}

export interface CodexProcs {
  /** `codex` running in a terminal. */
  sessions: number;
  app: boolean;
  daemon: boolean;
  ccSwitch: boolean;
}

export interface CodexRecent {
  at: string;
  /** Provider ids; `openai` is the ChatGPT login. */
  from: string;
  to: string;
  result: 'switched' | 'observed' | string;
  restarted: boolean | null;
}

export interface CodexView {
  /** `~/.codex` */
  home: string;
  customHome: boolean;
  present: boolean;
  config: 'ok' | 'missing' | 'invalid' | 'unreadable';
  reason: string | null;
  /** Sent back with a switch: a file changed since is not overwritten. */
  hash: string | null;
  current: 'chatgpt' | 'custom' | 'other';
  other: string | null;
  custom: CodexCustom | null;
  /** The `requires_openai_auth` a switch to the custom provider writes. */
  wantsRequires: boolean | null;
  refusal: CodexRefusal | null;
  backRefusal: CodexRefusal | null;
  login: CodexLogin;
  reach: CodexReach;
  ccSwitch: { installed: boolean; running: boolean; data: boolean; traces: string[] };
  /** `config.toml` changed outside the app since its last write. */
  external: {
    at: string | null;
    lastAt: string | null;
    lastProvider: string;
    now: string;
    mcpChanged: boolean;
    mcpCount: number;
  } | null;
  procs: CodexProcs | null;
  version: string | null;
  mcp: CodexMcp[];
  /** Resumable sessions by provider id (`''`: unknown); counted when the Codex tab opens. */
  resume: Record<string, number> | null;
  lastWrite: { at: string | null; provider: string; matches: boolean } | null;
  /** When each backup was taken, newest first. */
  backups: string[];
  backupDir: string;
  recent: CodexRecent[];
  firstSwitchAt: string | null;
  busy: 'switching' | 'restarting' | null;
  openaiBaseUrl: boolean;
}

export type CodexTarget = 'chatGpt' | 'custom';

export type CodexWriteError = { kind: 'changed' } | { kind: 'io'; detail: string } | { kind: 'verify' };

/** `codex::Outcome`. */
export type CodexOutcome =
  | { kind: 'switched'; target: CodexTarget; from: string; to: string; backup: string }
  | { kind: 'already'; target: CodexTarget }
  | { kind: 'refused'; target: CodexTarget; refusal: CodexRefusal }
  | { kind: 'invalid'; target: CodexTarget; detail: string }
  | { kind: 'failed'; target: CodexTarget; error: CodexWriteError };

/** `codex::procs::Restart`. */
export type CodexRestart = { kind: 'restarted' } | { kind: 'notRunning' } | { kind: 'noCodex' } | { kind: 'failed'; detail: string };

// ===== MCP (M9) =====

/** `agents::Agent`: a column of the MCP table. */
export type Agent = 'claude' | 'codex' | 'agy';

/** One `NAME = value` of an environment or of headers; a secret's value is the mask. */
export interface McpPair {
  name: string;
  value: string;
}

/** `agents::def::Def`: how a server runs. Its secrets are the mask: the windows never hold one. */
export type McpDef =
  | { kind: 'stdio'; command: string; args: string[]; env: McpPair[]; cwd: string | null }
  | { kind: 'http'; url: string; headers: McpPair[]; sse: boolean };

export type McpPlace = 'env' | 'header' | 'arg' | 'url';

export interface McpColumn {
  agent: Agent;
  present: boolean;
  /** `~/.codex/config.toml` */
  file: string;
  /** `missing`: no file yet; `absent`: the agent is not on this Mac; `noCli`: no `claude`. */
  state: 'ok' | 'missing' | 'invalid' | 'unreadable' | 'absent' | 'noCli';
  reason: string | null;
  writable: boolean;
}

export type McpLock = 'readOnly' | 'shape' | 'missingKey' | 'noCommand' | 'sse';

export interface McpCell {
  /** `off`: in the file and off, or kept by the app for Claude Code (`parked`). */
  state: 'on' | 'off' | 'absent';
  lock: McpLock | null;
  parked: boolean;
}

export interface McpSecret {
  /** `env:NAME`, `header:NAME`, `arg:2`, `url`: also its Keychain item. */
  id: string;
  label: string;
  place: McpPlace;
  /** In an agent's file or the app's Keychain item. */
  have: boolean;
  bitwarden: string | null;
}

export interface McpDefView {
  def: McpDef;
  /** The slots holding a secret (masked in `def`). */
  slots: string[];
  /** Members the agent's entry has that the app does not carry. */
  extra: string[];
}

export interface McpRow {
  name: string;
  kind: 'stdio' | 'http' | 'sse' | 'other';
  /** The command, or the package `npx` runs. */
  program: string | null;
  host: string | null;
  secrets: McpSecret[];
  cells: Record<Agent, McpCell>;
  /** Not the same server in every agent that has it. */
  differs: boolean;
  /** `path`: a full path that is not there; else a name not found in the usual folders. */
  missingCommand: { command: string; path: boolean } | null;
  defs: Partial<Record<Agent, McpDefView>>;
  /** For a secret two agents hold: only whether they hold the same. */
  sameSecrets: Record<string, boolean>;
}

export interface McpOwned {
  name: string;
  owner: string;
  agents: Agent[];
}

export interface McpStored {
  name: string;
  kind: McpRow['kind'];
  program: string | null;
  host: string | null;
  /** Unix seconds. */
  at: number;
  agents: Agent[];
  def: McpDefView;
  /** Secrets the Keychain no longer holds. */
  missing: string[];
}

export interface McpTable {
  columns: McpColumn[];
  rows: McpRow[];
  owned: McpOwned[];
  stored: McpStored[];
}

/** `agents::batch::To`. */
export type McpTo =
  | { kind: 'on' }
  | { kind: 'off' }
  | { kind: 'absent'; store: boolean }
  | { kind: 'def'; def: McpDef; from: Agent | null; keychain: string[]; off: boolean };

export interface McpChange {
  name: string;
  agent: Agent;
  to: McpTo;
}

export type McpRefusal = 'readOnly' | 'shape' | 'owned' | 'missingKey' | 'noCommand' | 'sse' | 'badName' | 'unknown' | 'nothing';

export interface McpLine {
  op: 'ctx' | 'add' | 'del' | 'gap';
  text: string;
  /** The line holds a masked secret. */
  secret: boolean;
}

export interface McpBlock {
  name: string;
  action: 'on' | 'off' | 'add' | 'remove' | 'replace';
  lines: McpLine[];
  /** Claude Code: the commands run, secrets masked. */
  commands: string[];
  secret: boolean;
  dropped: string[];
  from: Agent | null;
  /** The app keeps it: Claude Code's off, a delete into the store. */
  kept: boolean;
}

export interface McpPreview {
  groups: { agent: Agent; file: string; backups: string; blocks: McpBlock[] }[];
  refused: { name: string; agent: Agent; reason: McpRefusal }[];
  /** Sent back with the write. */
  changes: McpChange[];
  bases: Partial<Record<Agent, string>>;
}

export type McpEditError =
  | { kind: 'parse'; detail: string }
  | { kind: 'missing' }
  | { kind: 'exists' }
  | { kind: 'shape' }
  | { kind: 'unsupported' }
  | { kind: 'check'; detail: string };

export type McpFailure =
  | { kind: 'changed' }
  | { kind: 'refused'; refused: McpPreview['refused'] }
  | { kind: 'edit'; error: McpEditError }
  | { kind: 'write'; error: CodexWriteError }
  | { kind: 'cli'; detail: string }
  | { kind: 'keychain'; detail: string }
  | { kind: 'verify' };

/** `agents::batch::Outcome`. */
export interface McpOutcome {
  done: { agent: Agent; changes: number; backup: string | null }[];
  failed: { agent: Agent; error: McpFailure } | null;
  undo: { changes: McpChange[]; bases: Partial<Record<Agent, string>> } | null;
  at: number;
}

/** `agents::paste::Paste`: what a pasted JSON or TOML holds, secrets masked. */
export interface McpPaste {
  servers: { name: string; def: McpDef; slots: { id: string; label: string; place: McpPlace }[] }[];
  skipped: string[];
  error: 'empty' | 'tooLong' | 'parse' | 'noName' | 'none' | 'keychain' | null;
  detail: string | null;
}

export interface McpBackup {
  agent: Agent;
  at: string;
  file: string;
  /** `name:action` of the write it came before. */
  changes: string[];
}

export interface McpView {
  /** `null` until the tab first reads. */
  table: McpTable | null;
  readAt: string | null;
  busy: 'reading' | 'writing' | null;
  last: { at: string; outcome: McpOutcome; canUndo: boolean } | null;
  backups: McpBackup[];
}

export interface Model {
  snapshot: Snapshot;
  errors: { claude: ErrorView | null; agy: ErrorView | null; codex: ErrorView | null };
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
  /** Account names (emails, organizations) may show; off, none reaches the window. */
  showNames: boolean;
  theme: Theme;
  /** The service the menu bar title follows. */
  menubar: Menubar;
  /** The window whose share used the menu bar title and Đang dùng show. */
  menubarWindow: MenubarWindow;
  antigravity: AgyAccounts;
  agySwitch: AgySwitchView;
  /** The five alerts (M6). */
  alerts: AlertsView;
  /** What Firstmate sees (M6). */
  firstmate: FirstmateView;
  /** Version, launching at login and the update from source (M7). */
  app: AppView;
  /** Switching Codex (M8). */
  codex: CodexView;
  /** Every agent's MCP servers (M9). */
  mcp: McpView;
  /** The backup packs and the last restore (M10). */
  backup: BackupView;
}

// ===== Backup and restore (M10) =====

/** `agents::pack::Info`: a pack in iCloud Drive's folder. */
export interface PackInfo {
  /** The folder's name. */
  id: string;
  /** `cloud`: iCloud Drive has not downloaded it yet. */
  state: 'ok' | 'cloud' | 'broken';
  /** Unix seconds. */
  at: number | null;
  mac: string | null;
  mine: boolean;
  mcp: number;
  keys: number;
  /** Written by a later version of the app. */
  newer: boolean;
}

/** A key the app keeps in the Keychain: names only. */
export interface BackupKey {
  mcp: string;
  slot: string;
  label: string;
  bitwarden: string | null;
}

export interface BackupView {
  /** iCloud Drive is on on this Mac. */
  icloud: boolean;
  /** `null` until the tab first reads. */
  packs: PackInfo[] | null;
  keys: BackupKey[] | null;
  readAt: string | null;
  busy: 'packing' | 'restoring' | null;
  auto: { afterChange: boolean; daily: boolean };
  /** The last restore, while it can be undone. */
  restore: { at: number; pack: string; mac: string } | null;
  autoError: { kind: 'noICloud' | 'secret' | 'io'; detail?: string } | null;
}

/** `agents::pack::Summary`: "Xem trong gói". */
export interface PackSummary {
  id: string;
  mac: string;
  at: number;
  servers: { name: string; agents: Agent[]; on: Agent[] }[];
  keys: { mcp: string; slot: string; label: string; bitwarden: string | null }[];
}

/** `agents::restore::Item`: one line of the restore preview. */
export interface RestoreItem {
  name: string;
  kind: 'add' | 'replace';
  agents: Agent[];
  checked: boolean;
  /** Agents it goes to turned off, as in the pack. */
  off: Agent[];
  missingCommand: string | null;
  moved: boolean;
  /** Slots this Mac has no key for. */
  keys: string[];
  login: boolean;
  compare: { agent: Agent; here: McpDefView; pack: McpDefView }[];
}

/** `agents::restore::Plan`. */
export interface RestorePlan {
  pack: string;
  mac: string;
  at: number;
  items: RestoreItem[];
  same: string[];
  keys: { mcp: string; slot: string; label: string; bitwarden: string | null }[];
  skipped: { agent: Agent; reason: 'absent' | 'readOnly' | 'sse'; name: string | null }[];
  bases: Partial<Record<Agent, string>>;
}

export interface RestoreItemResult {
  name: string;
  kind: 'add' | 'replace';
  done: Agent[];
  kept: Agent[];
  why: 'noKey' | 'noCommand' | null;
  failed: Agent[];
  skipped: 'noKey' | 'noCommand' | null;
  login: boolean;
  command: string | null;
}

/** `agents::restore::Done`. */
export interface RestoreDone {
  at: number;
  items: RestoreItemResult[];
  failed: { agent: Agent; error: McpFailure } | null;
  written: { agent: Agent; changes: number; backup: string | null }[];
  keys: number;
  canUndo: boolean;
}

/** `agents::restore::Undone`. */
export interface RestoreUndone {
  outcome: McpOutcome | null;
  kept: number;
  keys: number;
}
