// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

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
/** `live`: the one login read; `list`: the accounts the app keeps. */
export type Source = 'live' | 'list';
export type Membership = 'in' | 'out' | 'ended' | 'unknown';

export interface Account {
  /** The account's number in the app's list; 0 for the only account a row reads. */
  number: number;
  role: Role;
  active: boolean;
  disabled: boolean;
  reading: Reading;
  fetchedAt: string | null;
  source: Source;
  membership: Membership;
  leavesAt: string | null;
  /** The row's numbers: the tighter window only, when there is one. */
  lines: Line[];
  /** Every window on its own, from the app's own reading of the account; empty when it has none. */
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

/** `single`: only the login signed in on this Mac, without the app's list. */
export type SectionStatus = 'ok' | 'single' | 'unavailable' | 'notSetUp' | 'unread';

export interface Section {
  status: SectionStatus;
  /** Why the list could not be read (`unavailable`), or why the one login read gave no number (`single`). */
  reason: string | null;
  accounts: Account[];
  pool: Pool;
  /** When the app made the report behind this section. */
  generatedAt: string | null;
  newestReadingAt: string | null;
  stale: boolean;
  /** `8-23` */
  personalHours: string | null;
  /** `quota-station` when the section lists the Antigravity accounts this app logs in. */
  source: string | null;
}

export type Keychain = 'granted' | 'required' | 'unknown';

export interface Snapshot {
  claude: Section;
  agy: Section;
  /** Codex's own ChatGPT login: one account, five hours and a week. */
  codex: Section;
  keychain: Keychain;
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
  /** `blocked (com.cswap.prefer-team is loaded)`: an older switcher runs, so nothing switches. */
  | { kind: 'blocked'; label: string }
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
  /** Quota Station keeps the Claude accounts on this Mac (imported or added), so its rotation can run. */
  installed: boolean;
  /** The pause flag; null when there is none. */
  pause: PauseView | null;
  dryRun: { decision: Decision; raw: string; at: string } | null;
  dryRunError: string | null;
  /** False: the switcher decided as if not paused while the flag held, so it does not read the flag yet. */
  guard: boolean | null;
  lastLog: LogLine | null;
  /** The rotation's LaunchAgent (`com.shin315.quota-station.rotation`) is loaded. */
  service: boolean | null;
  /** An older switcher still loaded (`com.cswap.prefer-team`, `com.cswap.auto`): nothing switches while it is. */
  legacy: string | null;
}

export interface ClaudeSwitchView {
  /** Quota Station keeps the Claude accounts on this Mac; before that nothing can switch. */
  ready: boolean;
  /** The slot a switch is running for. */
  busy: number | null;
}

export interface Sessions {
  live: number;
  unreadable: number;
}

// ===== The Claude accounts Quota Station keeps =====

export interface ClaudeWindowOut {
  usedPercent: number;
  /** RFC 3339. */
  resetsAt: string | null;
}

/** One account, as `claude/state.json` and the Claude tab list it (`quota-station-core::claude::state::AccountOut`). */
export interface ClaudeAccountRow {
  slot: number;
  alias: string | null;
  /** In the rotation; held out, it is still switched to by hand. */
  enabled: boolean;
  active: boolean;
  kind: 'login' | 'setupToken';
  status: 'fresh' | 'stale' | 'noData' | 'reloginRequired';
  fetchedAt: string | null;
  ageSeconds: number | null;
  windows: { fiveHour: ClaudeWindowOut | null; sevenDay: ClaudeWindowOut | null; fable: ClaudeWindowOut | null };
  /** The last read's error kind (`http-429`, `invalid_grant`…). */
  error: string | null;
  /** Only while names show. */
  email?: string | null;
}

export type ClaudeAccountAction = 'import' | 'login' | 'current' | 'token' | 'remove' | 'alias' | 'enabled' | 'rotation';

export interface ClaudeImported {
  added: number[];
  updated: number[];
  unchanged: number[];
  /** `[slot, why]`: `slotTaken`, `elsewhere`, `apiKey`, `noConfig`, `noCredential`, `keychain`. */
  skipped: [number, string][];
  rotation: boolean;
  prime: boolean;
}

export interface ClaudeActionDone {
  action: ClaudeAccountAction;
  ok: boolean;
  error: string | null;
  detail: string | null;
  slot: number | null;
  /** An add renewed an account the app had. */
  renewed: boolean;
  imported: ClaudeImported | null;
}

export interface ClaudeAccountsView {
  /** Quota Station keeps the accounts on this Mac. */
  ready: boolean;
  /** claude-swap's folder is on this Mac, to import from. */
  importable: boolean;
  accounts: ClaudeAccountRow[];
  working: ClaudeAccountAction | null;
  last: ClaudeActionDone | null;
  /** The rotation's LaunchAgent is loaded; null when that cannot be told. */
  rotation: boolean | null;
  /** An older switcher still loaded. */
  legacy: string | null;
  /** `claude` is on this Mac, to log a new account in with. */
  claude: boolean;
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
   * `error`: the switch's error kind (`busy`, `noCredential`, `unmanaged`, `unconfirmed`, `keychain`, …),
   * or `legacy` while an older switcher is loaded (its label in `detail`). `active`: the account in use
   * when the login could not be put back as it was.
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
  /** The rule's reason for an invalid file (`rotation_bad_value:full`), or the read error. */
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
  kind: 'busy' | 'changed' | 'invalid' | 'write' | 'unreachable' | 'noFile' | 'unknownMac';
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
/** The theme pack every window is drawn in (`src/styles/tokens.css`). */
export type ThemePack = 'default' | 'kinhNhip' | 'daiSangTinh' | 'motNhipNhin' | 'banDieuKhien' | 'chieuLang';
export const THEME_PACKS: readonly ThemePack[] = ['default', 'kinhNhip', 'daiSangTinh', 'motNhipNhin', 'banDieuKhien', 'chieuLang'];
/** The windows' language; `auto` follows macOS. */
export type Language = 'auto' | 'vi' | 'en';
export type Menubar = 'claude' | 'gemini' | 'codex';
/** The window the menu bar title shows; one the service lacks falls back to five hours, then the tightest. */
export type MenubarWindow = 'fiveHours' | 'week' | 'fableWeek' | 'tightest';

export interface AgyLogin {
  id: string;
  /** Its place in the app's list, from 1. */
  number: number;
  /** Only when the owner turned emails on. */
  email: string | null;
  name: string | null;
  status: AgyStatus;
  /** Google sent a verification page for it. */
  verify: boolean;
  /** The app is reading its quota now. */
  reading: boolean;
  /** The app's last read failed; the row keeps the last numbers, with their age. */
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
   * `basis`: `identity` when agy's login named the account, `reading` when
   * agy's own numbers showed it, `binding` when only the binding the switch
   * wrote answered.
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
  /** The row lists this app's accounts, so a row's number is the account's number here. */
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

/** The five kinds with a switch each (M6). */
export type SwitchKind = 'pool' | 'leaving' | 'login' | 'overwrite' | 'firstmate';
/**
 * `skill`: skills with a newer version (M11), shown in the app only unless Cài đặt says otherwise.
 * `env`: a variable overriding a login (M14), in the app only.
 */
export type AlertKind = SwitchKind | 'skill' | 'env' | 'cli';
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
  /** The bell's panel was opened since it came. */
  read: boolean;
}

export type AlertSwitches = Record<SwitchKind, boolean>;

/** `unbundled`: the app does not run as a packaged `.app`, so macOS shows no cards. */
export type NotifyPermission = 'unknown' | 'allowed' | 'denied' | 'unbundled';

/** Alert 1: the account in use at `dryAtMost` % left or less, and no other with `roomAtLeast` %. */
export interface PoolAlert {
  dryAtMost: number;
  roomAtLeast: number;
}

export interface AlertsView {
  /** Newest first. */
  recent: AlertRecord[];
  switches: AlertSwitches;
  /** Alert 1's thresholds, in percent left. */
  poolAlert: PoolAlert;
  /** Cards with buttons nobody pressed yet. */
  unanswered: number;
  /** Cards that came since the bell's panel was last opened: its count. */
  unread: number;
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
  /** The same dry rule, counted by the app from its own numbers. */
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

/**
 * "Mã nguồn" and "Cập nhật từ mã nguồn" (`src-tauri/src/install.rs`
 * `SourceView`): only in a build with the author's tools.
 */
export interface SourceView {
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

/** The app itself (`src-tauri/src/install.rs` `AppView`, M7), with the update from source in a build with the author's tools. */
export interface AppView extends Partial<SourceView> {
  version: string;
  /** The commit it was built from, 7 characters, `-dirty` kept; null when not built by `scripts/install.sh`. */
  commit: string | null;
  launchAtLogin: LaunchAtLogin;
}

/** The latest release on GitHub (`quota-station-core::release::Latest`). */
export interface ReleaseLatest {
  /** `0.3.0`. */
  version: string;
  publishedAt: string | null;
  /** The notes' points, in plain words. */
  notes: string[];
  url: string;
  /** The disk image, when the release has one. */
  dmg: string | null;
  /** The installer for this system, when the release has one: on Windows the `-setup.exe`. Older apps send only `dmg`. */
  asset?: string | null;
  size: number | null;
}

/** A newer Quota Station on GitHub Releases (`src-tauri/src/release.rs` `ReleaseView`). */
export interface ReleaseView {
  /** "Tự kiểm tra bản mới của app": once a day. */
  auto: boolean;
  checking: boolean;
  /** Unix seconds of the last check that ended. */
  checkedAt: number | null;
  /** Why the last check did not reach GitHub: `offline`, `rateLimited`, a status. */
  error: string | null;
  /** The repository has no release yet. */
  none: boolean;
  latest: ReleaseLatest | null;
  /** The latest release is newer than this copy. */
  newer: boolean;
  /** "Bỏ qua bản này". */
  skipped: string | null;
  /** "Để sau". */
  later: string | null;
}

/** A command-line tool the app can update: Claude Code, the Codex CLI, the Antigravity CLI. */
export type CliTool = 'claude' | 'codex' | 'agy';

/** How it was installed (`quota-station-core::cli_tools::Install`). */
export type CliInstall =
  /** Claude Code's own installer: `~/.local/bin/claude` into `~/.local/share/claude/versions/`. */
  | 'native'
  | 'homebrew'
  | 'npm'
  /** Codex's `install.sh`: `~/.local/bin/codex` into `~/.codex/packages/standalone/`. */
  | 'standalone'
  /** Antigravity's `install.sh`: one file, `~/.local/bin/agy`. */
  | 'script';

/** Why the app leaves a tool's update to the owner, who is told what to do instead. */
export type CliHandsOff =
  /** Claude Code from npm: the old way, which updates over the files a session runs from. */
  | 'npm'
  /** A copy inside another app, which updates it itself. */
  | 'bundled'
  /** Its folder needs an administrator to write. */
  | 'unwritable'
  /** Installed in a way the app does not know. */
  | 'unknown';

/**
 * Why an update failed (`quota-station-core::cli_tools::Failure`). `started`: a Codex session opened before it ran
 * (`detail` is how many); `sessions`: the processes could not be listed, so Codex did not update.
 */
export type CliFailure = 'offline' | 'permission' | 'busy' | 'exit' | 'timeout' | 'unchanged' | 'started' | 'sessions';

export type CliLast =
  | { ok: true; from: string; to: string; at: number }
  | { ok: false; failure: CliFailure; detail: string; log?: string | null; at: number };

/** One tool (`quota-station-core::cli_tools::ToolView`). */
export interface CliToolView {
  tool: CliTool;
  /** Null: not found on this Mac. */
  version: string | null;
  /** The newest for the way it was installed; null before a check, or when the check failed. */
  latest: string | null;
  newer: boolean;
  /** Why the last check did not reach its feed: `offline`, a status such as `503`, `unreadable`. */
  failed: string | null;
  install: CliInstall | null;
  /** Where the command is, `~` for the home folder. */
  path: string | null;
  /** What "Cập nhật" runs, or what to run in Terminal when the app leaves it to the owner. */
  command: string | null;
  handsOff: CliHandsOff | null;
  /** The app that brings it, for `bundled`. */
  bundledBy?: string | null;
  /** Its sessions running now. */
  running: number;
  /** An update while they run is safe: they keep their version until opened again. */
  safeWhileRunning: boolean;
  updating: boolean;
  last: CliLast | null;
}

/** Agent › Cập nhật, the Cập nhật Agent page (`src-tauri/src/cli_tools.rs` `CliToolsView`). */
export interface CliToolsView {
  /** "Tự động kiểm tra", once a day: only looks and tells the bell, never updates. */
  daily: boolean;
  checking: boolean;
  /** Unix seconds of the last check that ended. */
  checkedAt: number | null;
  /** The last check reached no server. */
  offline: boolean;
  tools: CliToolView[];
}

/** The main window's tabs in this build (`src-tauri/src/state.rs` `Tab`). */
export type Tab =
  | 'overview'
  | 'stats'
  | 'claude'
  | 'agy'
  | 'codex'
  | 'rules'
  | 'history'
  /** Not a page: the bell's panel (Thông báo), open over the page in view. */
  | 'alerts'
  | 'firstmate'
  | 'mcp'
  | 'skills'
  | 'plugins'
  | 'instr'
  | 'sets'
  /** Cập nhật Agent: Claude Code, the Codex CLI and the Antigravity CLI. */
  | 'cliTools'
  | 'backup'
  | 'copy'
  | 'settings';

/** A tab of Cài đặt: Chung holds Cập nhật and Sao lưu too, Dịch vụ the variables that override a login. */
export type SettingsGroup = 'general' | 'display' | 'notify' | 'services' | 'advanced';

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
  /** `baseUrl` points at CC Switch's local proxy (port 15721). */
  proxy: boolean;
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
  /** `plugins`: the plugins choice applied on the provider in use; `key`, `keyUndo`: "Dán khoá" and its undo. */
  result: 'switched' | 'observed' | 'plugins' | 'key' | 'keyUndo' | string;
  restarted: boolean | null;
  /** `requires_openai_auth` written on the way to the custom provider. */
  requires?: boolean | null;
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
  /** "Dùng plugin với Custom API": kept in the settings, on by default. */
  plugins: boolean;
  /** The custom provider's table takes a pasted key ("Dán khoá"). */
  takesKey: boolean;
  /** "Hoàn tác" after "Dán khoá", while the file is still the one it left. */
  keyUndo: { at: string | null; token: 'placeholder' | 'empty' | 'none'; baseUrl: boolean } | null;
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
  /** `importing`: sign-ins read from files go in; `waking`: "Tự đánh thức" runs Codex for one account. */
  busy: 'switching' | 'restarting' | 'importing' | 'waking' | null;
  openaiBaseUrl: boolean;
  /** Every Codex account the app lists: the ChatGPT sign-ins it keeps, then the provider tables in `config.toml`. */
  accounts: CodexAccount[];
  /** Where the ChatGPT sign-ins not in the Codex folder wait, each in its own folder. */
  accountsDir: string;
  /** A sign-in with a device code: the page and the code to type there. */
  loginCode?: { url: string; code: string } | null;
  /** "Tự đánh thức tài khoản ChatGPT", off until the owner turns it on. */
  trigger: CodexTrigger;
}

/**
 * One sign-in read in "Nhập file đăng nhập (JSON)", from a file picked or
 * from JSON pasted. The window gets the file's name, whose sign-in it is and
 * what importing it would do, never a token.
 */
export interface CodexImportItem {
  /** The file's name, as picked; null for JSON pasted. */
  file: string | null;
  /** `codex`: Codex's own `auth.json`; `cliproxy`: a CLIProxyAPI `codex-*.json`. */
  format: 'codex' | 'cliproxy' | null;
  email: string | null;
  plan: string | null;
  /**
   * `new`: added as a new account; `renew`: an account here that must sign in
   * again takes this sign-in; `dup`: already here and signed in, left out;
   * `refreshOnly`: only a refresh token, left out (using it would sign the
   * other copy out); `apiKey`: an API key, not a ChatGPT sign-in; `broken`:
   * not a sign-in the app can read.
   */
  state: 'new' | 'renew' | 'dup' | 'refreshOnly' | 'apiKey' | 'broken';
  /** `renew`, `dup`: the account here. */
  id: string | null;
}

/** The files picked, read; `token` names them when the owner presses Nhập. */
export interface CodexImportPreview {
  token: string;
  items: CodexImportItem[];
}

/**
 * "Tự đánh thức tài khoản ChatGPT": a short message sent with Codex in each
 * chosen account's own folder, so its five-hour window starts early and its
 * sign-in does not run out. Codex reads and renews the sign-in itself.
 */
export interface CodexTrigger {
  on: boolean;
  /** `daily`: each day at each of `times`; `reset`: once a five-hour window has reset, at most once per window. */
  mode: 'daily' | 'reset';
  /** `07:00`, `12:00`: the Mac's times, earliest first, five hours apart at least. */
  times: string[];
  /** Also wake an unused account whose sign-in is about to run out, so Codex renews it. */
  keepAlive: boolean;
  /** The accounts it wakes. */
  ids: string[];
  /** When the next run is due; null while off. */
  next: string | null;
  /** The account being woken now. */
  running: string | null;
  /** Newest first. */
  runs: CodexTriggerRun[];
}

export interface CodexTriggerRun {
  at: string;
  id: string;
  why: 'daily' | 'reset' | 'keepAlive' | 'now';
  /**
   * `started`: a five-hour window opened; `open`: one was open already, so
   * it ran nothing; `relogin`: ChatGPT refused the sign-in; `limited`: no
   * quota left this week; `failed`: Codex ended with an error; `soon`: run
   * less than five hours before, and `capped`: five times in 24 hours, so
   * it ran nothing.
   */
  result: 'started' | 'open' | 'relogin' | 'limited' | 'failed' | 'soon' | 'capped';
  /** `started`: when the window it opened resets. */
  resetsAt: string | null;
  detail: string | null;
}

/**
 * One Codex account: a ChatGPT sign-in the app keeps (its own Codex folder,
 * `auth.json` moved into the Codex folder while it is the one signed in), or
 * a custom provider table in `config.toml`. Never a token or a key.
 */
export interface CodexAccount {
  /** `chatgpt:<n>` for a sign-in, the table's id for a custom provider. */
  id: string;
  kind: 'chatgpt' | 'custom';
  /** The owner's nickname, else the email's name (ChatGPT) or the table's `name`. */
  name: string;
  /** The owner gave it a nickname. */
  nickname: boolean;
  /** ChatGPT: its number in the snapshot's Codex section, where its quota is. */
  number: number | null;
  /** ChatGPT: the email, only while the owner shows names. */
  email: string | null;
  /** ChatGPT: `Plus`, `Pro`, `Team`… as the usage answer names it. */
  plan: string | null;
  /** Codex sends its requests through this account now. */
  active: boolean;
  /** ChatGPT: its sign-in is the one in the Codex folder (in use, or kept for plugins on a custom provider). */
  signedIn: boolean;
  /** ChatGPT: `relogin` when ChatGPT refused the sign-in; `idle` when its token ran out while not in use (the numbers wait until it is used). */
  login: 'ok' | 'relogin' | 'idle';
  /** Custom: its table, how its key is given, whether it answers. */
  custom: CodexCustom | null;
  reach: CodexReach | null;
  /** Why "Chuyển" to it cannot run. */
  refusal: CodexRefusal | null;
  /** When it was added here; null for what was already in Codex. */
  addedAt: string | null;
}

/** What an account action did. Holds no token or key. */
export type CodexAccountDone =
  | { kind: 'added'; id: string }
  /** The sign-in is an account already in the list: nothing added. */
  | { kind: 'dup'; id: string }
  | { kind: 'renewed'; id: string }
  | { kind: 'renamed'; id: string }
  /**
   * "Nhập file đăng nhập (JSON)": the accounts added or renewed; `removed`:
   * every file they came from was deleted, as asked; `pasted`: they came from
   * JSON pasted in the window.
   */
  | { kind: 'imported'; ids: string[]; removed: boolean; pasted: boolean }
  | { kind: 'removed'; id: string; name: string; loggedOut: boolean }
  | { kind: 'failed'; reason: CodexLoginFail; detail: string | null }
  /** `expired`: "Nhập" came more than ten minutes after the files were read, or after another read. */
  | { kind: 'refused'; reason: 'busy' | 'inUse' | 'signedIn' | 'changed' | 'badUrl' | 'name' | 'empty' | 'key' | 'expired' };

/** Why a ChatGPT sign-in ended without an account. */
export type CodexLoginFail = 'cancelled' | 'timeout' | 'port' | 'noCodex' | 'denied' | 'other';

/** "Kiểm tra" on a new provider: its address asked with no key, then with the key. */
export interface CodexProviderCheck {
  reach: 'up' | 'down' | 'badUrl';
  ms: number | null;
  /** `rejected`: 401 or 403 with the key; `unknown`: the address did not answer, or answered both the same. */
  key: 'accepted' | 'rejected' | 'unknown';
  status: number | null;
}

export type CodexTarget = 'chatGpt' | 'custom';

export type CodexWriteError = { kind: 'changed' } | { kind: 'io'; detail: string } | { kind: 'verify' };

/** `codex::Outcome`. */
export type CodexOutcome =
  | { kind: 'switched'; target: CodexTarget; from: string; to: string; backup: string }
  | { kind: 'already'; target: CodexTarget }
  | { kind: 'refused'; target: CodexTarget; refusal: CodexRefusal }
  | { kind: 'invalid'; target: CodexTarget; detail: string }
  | { kind: 'failed'; target: CodexTarget; error: CodexWriteError }
  /** A ChatGPT account's sign-in could not be moved: nothing changed. */
  | { kind: 'account'; target: CodexTarget; refusal: CodexAccountRefusal };

/** Why a ChatGPT account's sign-in cannot move (`codex::accounts::Refusal`). */
export type CodexAccountRefusal =
  | { kind: 'unknown' }
  | { kind: 'noLogin' }
  | { kind: 'inUse' }
  | { kind: 'conflict' }
  | { kind: 'keychain' }
  | { kind: 'name' }
  | { kind: 'io'; detail: string };

/** Why a pasted key is not written (`codex::config::KeyRefusal`). */
export type CodexKeyRefusal =
  | { kind: 'noCustom' }
  | { kind: 'envKey'; var: string }
  | { kind: 'headers' }
  | { kind: 'shape' }
  | { kind: 'empty' }
  | { kind: 'notAKey' }
  | { kind: 'tooLong' }
  | { kind: 'placeholder' }
  | { kind: 'reference' }
  | { kind: 'badUrl' }
  | { kind: 'noUndo' }
  /** A new provider's name is taken, or one Codex reserves. */
  | { kind: 'name' };

/** `codex::KeyPreview`: the lines "Dán khoá" changes; the new key only as `sk-…abcd`. */
export interface CodexKeyPreview {
  hash: string | null;
  lines: { op: 'ctx' | 'add' | 'del' | 'gap'; text: string; secret: boolean }[];
  refusal: CodexKeyRefusal | null;
  detail: string | null;
  /** A key already sits there: it is replaced and cannot be put back. */
  replaces: boolean;
  /** What "Hoàn tác" puts back; null: no undo. */
  undo: 'placeholder' | 'empty' | 'none' | null;
}

/** `codex::KeyOutcome`: never the key. */
export type CodexKeyOutcome =
  | { kind: 'written'; provider: string; backup: string; undo: boolean }
  | { kind: 'undone'; provider: string; backup: string }
  | { kind: 'already' }
  | { kind: 'refused'; refusal: CodexKeyRefusal }
  | { kind: 'invalid'; detail: string }
  | { kind: 'failed'; error: CodexWriteError };

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

/**
 * Cài đặt › Nâng cao (`src-tauri/src/state.rs` `AdvancedView`): on, the
 * tabs and switches made for the author's own Macs show (the rotation, the
 * pause and the switcher, the other Macs, the update from source); Firstmate
 * has its own switch inside. Off on a new install, on where the app was before.
 * Always off in a build without the author's tools.
 */
export interface AdvancedView {
  on: boolean;
  firstmate: boolean;
}

/** The system the app runs on: every difference the windows draw follows it. */
export type Platform = 'macos' | 'windows';

/** The parts a build may leave out, and the system it was built for. */
export interface Capabilities {
  /**
   * The Antigravity accounts the app logs in, reads and switches itself (the
   * `antigravity-accounts` feature, CC BY-NC-SA 4.0). Without them
   * Antigravity is the one account signed in here, read-only.
   */
  agyAccounts: boolean;
  /**
   * The tools made for the author's own Macs (the `author-tools` feature,
   * which a release build leaves out): Cài đặt › Nâng cao, Luật xoay vòng
   * and the other Macs, Chép sang Mac khác, Firstmate and the update from
   * source. Without them the model has none of their parts and the windows
   * show no switch, tab or setting for them.
   */
  authorTools: boolean;
  /**
   * The system the app was built for: `data-os` on the page (fonts,
   * corners, scroll bars) and the words for the system's own parts
   * (`os()`, `mac()` in i18n.ts). Absent is macOS, as in the fixtures
   * and the tests written before Windows.
   */
  platform?: Platform;
  /**
   * A test build for a guided session on a Windows PC (`session.rs`): the
   * guide's window, its self-checks and pictures. Absent in every other build.
   */
  session?: boolean;
}

export interface Model {
  snapshot: Snapshot;
  reading: boolean;
  reads: number;
  /** The name shown for this Mac: set in settings, else the one macOS gives. */
  machine: string;
  /** The name macOS gives this Mac. */
  machineDefault: string;
  granting: boolean;
  switcher: SwitcherView;
  claudeSwitch: ClaudeSwitchView;
  /** The Claude accounts Quota Station keeps, and their actions. */
  claudeAccounts: ClaudeAccountsView;
  /** The rules editor and the other Macs; only in a build with the author's tools. */
  rotation?: RotationView;
  /** Account names (emails, organizations) may show; off, none reaches the window. */
  showNames: boolean;
  theme: Theme;
  /** The theme pack: colours, glass and type, never what is shown. */
  themePack: ThemePack;
  language: Language;
  /** The service the menu bar title follows. */
  menubar: Menubar;
  /** The window whose share used the menu bar title and Đang dùng show. */
  menubarWindow: MenubarWindow;
  /** Cài đặt › Nâng cao, and the Firstmate switch inside it. */
  advanced: AdvancedView;
  /** What this build has (`src-tauri/src/state.rs` `Capabilities`). */
  capabilities: Capabilities;
  /** The Antigravity accounts the app logs in; only in a build with them. */
  antigravity?: AgyAccounts;
  /** Switching them; only in a build with them. */
  agySwitch?: AgySwitchView;
  /** The five alerts (M6). */
  alerts: AlertsView;
  /** What Firstmate sees (M6); only in a build with the author's tools. */
  firstmate?: FirstmateView;
  /** Version, launching at login and the update from source (M7). */
  app: AppView;
  /** Switching Codex (M8). */
  codex: CodexView;
  /** Every agent's MCP servers (M9). */
  mcp: McpView;
  /** The backup packs and the last restore (M10). */
  backup: BackupView;
  /** The skills, the plugins and the instruction files (M11). */
  skills: SkillsView;
  /** A newer version on GitHub Releases. */
  release: ReleaseView;
  /** Claude Code, Codex and Antigravity's command-line tools: their versions and updates. */
  cliTools?: CliToolsView;
  /** A new install: the main window shows the first run's steps. */
  firstRun: boolean;
  plugins: PluginsView;
  instructions: InstrView;
  /** The variables that override a login (M14). */
  env: EnvView;
  /** The configuration sets (M15). */
  sets: SetsView;
  /** The token statistics' read (M16); the numbers come with `statsReport`. */
  stats: StatsView;
}

// ===== Thống kê (M16) =====

/** `stats::index::Progress`: logs read of the logs found. */
export interface StatsProgress {
  agent: Agent;
  done: number;
  total: number;
}

/** `src-tauri/src/stats.rs` `StatsView`. */
export interface StatsView {
  reading: boolean;
  /** The read running is the first: no log was read before. */
  first: boolean;
  progress: StatsProgress[];
  /** Bumped after each read and "Đã biết": the tab asks for its numbers again. */
  generation: number;
  /** The last read could not save the index. */
  failed: boolean;
  /** RFC 3339; none before the first read. */
  readAt: string | null;
}

/** `stats::report::Period`: Hôm nay by hour, 7 ngày, 30 ngày. */
export type StatsPeriod = 'today' | 'week' | 'month';

/** One column: an hour (`14:00`) or a day (`18/10`). */
export interface StatsPoint {
  label: string;
  claude: number;
  codex: number;
}

export interface StatsModelRow {
  model: string;
  agent: Agent;
  /** Input, written to a cache or not. */
  input: number;
  cacheRead: number;
  output: number;
  total: number;
  /** null: the price table does not have the model. */
  usd: number | null;
}

export interface StatsProjectRow {
  /** The folder's own name; empty when the logs do not say. */
  name: string;
  /** `~/Projects/web-shop`. */
  path: string;
  tokens: number;
}

export interface StatsAccountRow {
  /** claude-swap's slot; null when the switch history cannot tell. */
  slot: number | null;
  tokens: number;
}

/** `stats::index::Skipped`: the logs in a format the app does not know. */
export interface StatsSkipped {
  agent: Agent;
  sessions: number;
  lines: number;
  versions: string[];
  /** Unix seconds. */
  since: number | null;
  /** Sessions counted when "Đã biết" was pressed. */
  acknowledged: number;
}

/** `stats::report::Report`. */
export interface StatsReport {
  period: StatsPeriod;
  points: StatsPoint[];
  claude: number;
  codex: number;
  total: number;
  usd: number;
  unpriced: string[];
  models: StatsModelRow[];
  projects: StatsProjectRow[];
  accounts: StatsAccountRow[];
  /** `2026-10-19`. */
  pricesAsOf: string;
  pricesSource: string;
  skipped: StatsSkipped[];
  /** Unix seconds. */
  readAt: number | null;
}

// ===== Variables that override a login (M14) =====

/**
 * `agents::env_check::Place`: on macOS five shell files in the home folder,
 * and launchd; on Windows the user's and the system's environment variables
 * (the registry), Windows PowerShell's and PowerShell 7's profiles, and Git
 * Bash's `~/.bash_profile`, `~/.bashrc` and `~/.profile`.
 */
export type EnvPlace =
  | 'zshenv'
  | 'zprofile'
  | 'zshrc'
  | 'bashProfile'
  | 'profile'
  | 'launchctl'
  | 'userEnv'
  | 'machineEnv'
  | 'psProfile'
  | 'pwshProfile'
  | 'bashrc';

/** `agents::env_check::Found`: a name set in one place. Never its value. */
export interface EnvFound {
  /** `ANTHROPIC_API_KEY@zshrc`. */
  id: string;
  name: string;
  agent: Agent;
  /** `bad`: a key or token replaces the login; `warn`: an address, folder or project. */
  weight: 'bad' | 'warn';
  place: EnvPlace;
  /** The line that sets it, from 1; none in launchd. */
  line: number | null;
  /** "Trong Terminal, Claude Code dùng khoá này thay cho …". */
  effect: string;
}

/** `env::EnvView`: Cài đặt's card and the Claude tab's banner. */
export interface EnvView {
  /** RFC 3339; none before the first check. */
  checkedAt: string | null;
  checking: boolean;
  /** The eleven names and the six places' labels. */
  vars: string[];
  places: string[];
  /** Set and not ignored. */
  found: EnvFound[];
  /** How many "Bỏ qua" hides. */
  ignored: number;
  /** Places that could not be looked at. */
  unread: EnvPlace[];
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
  /** Where the packs go, `~/…`. */
  folder: string;
  /** Chosen in Cài đặt; otherwise iCloud Drive's `Quota Station Agents`. */
  customFolder: boolean;
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
  /** The skills, plugins and instruction files in the pack (M11); none in an older one. */
  skills?: string[];
  plugins?: string[];
  instructions?: string[];
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
  /** The pack's skills, plugins and instruction files (M12); none in an older pack. */
  skills?: RestoreSkillPlan | null;
  plugins?: RestorePluginPlan | null;
  files?: RestoreFilePlan | null;
}

/** `agents::restore::SkillItem`: a skill of the pack this Mac has not got. */
export interface RestoreSkillItem {
  name: string;
  /** `github`: installed from its repository again; `hand`: copied from the pack. */
  kind: 'github' | 'hand';
  repo: string | null;
  on: Agent[];
  checked: boolean;
  blocked: 'noNpx' | 'cloud' | null;
}

export interface RestoreSkillPlan {
  items: RestoreSkillItem[];
  same: string[];
  /** Skills installed by hand whose files are not in the pack. */
  leftOut: { name: string; why: 'key' | 'link' | 'large' | 'unreadable' | 'old' }[];
}

/** `agents::restore::PluginItem`. */
export interface RestorePluginItem {
  agent: Agent;
  /** `name@marketplace`. */
  id: string;
  enabled: boolean;
  /** The marketplace added first. */
  market: string | null;
  checked: boolean;
  note: 'noSource' | 'codexOff' | null;
}

export interface RestorePluginPlan {
  items: RestorePluginItem[];
  same: string[];
  skipped: { agent: Agent; reason: 'noCli' | 'failed'; name: string | null }[];
}

/** `agents::restore::FileItem`: an instruction file to write. */
export interface RestoreFileItem {
  /** `~/…`. */
  path: string;
  agent: Agent | null;
  kind: 'add' | 'replace';
  checked: boolean;
  /** For a replace: the lines that differ; `del` this Mac's, `add` the pack's, keys masked. */
  changed: number;
  diff: McpLine[];
}

export interface RestoreFilePlan {
  items: RestoreFileItem[];
  same: string[];
  kept: { path: string; agent: Agent | null; why: 'key' | 'absent' | 'unreadable' | 'path' }[];
  bases: Record<string, string>;
}

/** `agents::restore::More`: the skills, plugins and files ticked. */
export interface RestoreMore {
  skills: string[];
  plugins: { agent: Agent; id: string }[];
  files: string[];
  fileBases: Record<string, string>;
}

/** `agents::restore::PartResult`: how one skill, plugin or file ended. */
export interface RestorePartResult {
  part: 'skill' | 'plugin' | 'file';
  id: string;
  agent: Agent | null;
  kind: string;
  ok: boolean;
  why: 'noNpx' | 'npx' | 'cloud' | 'damaged' | 'taken' | 'links' | 'noCli' | 'login' | 'command' | 'changed' | 'io' | null;
  retry: boolean;
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
  /** The skills, plugins and instruction files (M12). */
  parts?: RestorePartResult[];
  canUndo: boolean;
}

/** `agents::restore::Undone`. */
export interface RestoreUndone {
  outcome: McpOutcome | null;
  kept: number;
  keys: number;
  /** Skills removed, plugins uninstalled, instruction files put back (M12). */
  skills?: number;
  plugins?: number;
  files?: number;
  /** What stays: changed since, or its undo did not run. */
  left?: number;
}

// ===== Chép sang Mac khác (M13) =====

/** `agents::copy::DiffItem`: one item the two Macs do not have alike. */
export interface CopyItem {
  group: 'mcp' | 'skill' | 'plugin' | 'file';
  /** The server's or skill's name, the plugin's `name@marketplace`, the file's `~/…`. */
  id: string;
  agent: Agent | null;
  /** `mine`: only on this Mac; `theirs`: only on the other; `diff`, `ver`, `text`: both, not alike. */
  state: 'mine' | 'theirs' | 'diff' | 'ver' | 'text';
  /** What differs: `command`, `args`, `env`, `cwd`, `url`, `headers`, `agents`, `source`. */
  what: string[];
  /** Why it cannot go from this Mac (`push`) or come to it (`pull`). */
  push: CopyBlock | null;
  pull: CopyBlock | null;
  /** A skill left out: `key`, `link`, `large`, `unreadable`. */
  note: string | null;
  /** A skill installed by hand: its files go along. */
  hand: boolean;
  /** A server's secrets, by their names. */
  keys: string[];
  mineOn: Agent[];
  theirsOn: Agent[];
  /** A server's definitions on each Mac, secrets masked. */
  compare: { agent: Agent; mine: McpDef | null; theirs: McpDef | null }[];
  /** A skill's version on each Mac, RFC 3339. */
  mineAt: string | null;
  theirsAt: string | null;
  /** A file's lines, this Mac's against the other's. */
  lines: McpLine[];
  changed: number;
}

export type CopyBlock = 'leftOut' | 'older' | 'hand' | 'source' | 'key' | 'excluded' | 'unreadable' | 'absent';

/** What "So sánh" found (`src-tauri/src/copy.rs` `Comparison`). */
export interface CopyComparison {
  alias: string;
  /** The other Mac's name in its app. */
  mac: string;
  /** When it was read, Unix seconds. */
  at: number;
  diff: { items: CopyItem[]; same: string[] };
}

/** The items ticked (`agents::copy::Selection`); `updates` are skills to bring to their newest version. */
export interface CopySelection {
  mcp: string[];
  skills: string[];
  updates: string[];
  plugins: { agent: Agent; id: string }[];
  files: string[];
}

// ===== Skills, plugins and instruction files (M11) =====

/** `agents::skills::Column`. */
export interface SkillColumn {
  agent: Agent;
  present: boolean;
  /** The folder, or Codex's `config.toml`. */
  file: string;
  /** `missing`: the agent is here, its skills folder not yet. */
  state: 'ok' | 'missing' | 'absent' | 'invalid' | 'unreadable';
  reason: string | null;
  writable: boolean;
}

/** Why a box cannot be ticked: the column is read only, the skill is in an agent's own folder, another skill of that name is there, or its link points to nothing. */
export type SkillLock = 'readOnly' | 'hand' | 'taken' | 'broken';

export interface SkillCell {
  state: 'on' | 'off' | 'absent';
  lock: SkillLock | null;
}

/** A newer version on GitHub (G). */
export interface SkillUpdate {
  /** When the version on this Mac was installed (the lock file's date). */
  fromAt: string | null;
  /** Unix seconds: when the app first saw the newer version. */
  seenAt: number;
  /** "Sửa hướng dẫn, thêm 1 file". */
  plain: string;
  /** The skill's history on GitHub. */
  link: string;
}

export interface SkillRow {
  name: string;
  /** `github`: in the lock file; `hand`: installed by hand; `other`: in the lock file from elsewhere. */
  source: 'github' | 'other' | 'hand';
  repo: string | null;
  /** In an agent's own folder rather than the shared one: gather it first. */
  home: Agent | null;
  cells: Record<Agent, SkillCell>;
  /** Changed on this Mac since it was installed. */
  edited: boolean;
  update: SkillUpdate | null;
  /** Bytes. */
  size: number;
  importable: boolean;
}

/** Skills an agent or a plugin brings: read only. */
export interface SkillOwned {
  what: string;
  agent: Agent;
  names: string[];
}

export interface SkillBroken {
  agent: Agent;
  name: string;
  target: string;
}

export interface SkillTable {
  columns: SkillColumn[];
  rows: SkillRow[];
  owned: SkillOwned[];
  broken: SkillBroken[];
  /** The lock file could not be read. */
  lockError: string | null;
}

export interface SkillChange {
  name: string;
  agent: Agent;
  on: boolean;
}

export type SkillRefusal = SkillLock | 'badName' | 'unknown' | 'nothing';

export interface SkillPreview {
  groups: { agent: Agent; file: string; count: number; lines: McpLine[] }[];
  refused: { name: string; agent: Agent; reason: SkillRefusal }[];
  changes: SkillChange[];
  bases: Partial<Record<Agent, string>>;
}

export type SkillFailure =
  | { kind: 'changed' }
  | { kind: 'refused'; refused: SkillPreview['refused'] }
  | { kind: 'edit'; detail: string }
  | { kind: 'write'; detail: string };

export interface SkillOutcome {
  done: { agent: Agent; changes: number; backup: string | null }[];
  failed: { agent: Agent; error: SkillFailure } | null;
  undo: { changes: SkillChange[]; bases: Partial<Record<Agent, string>> } | null;
  at: number;
}

/** What `npx skills` said when it failed. */
export type NpxError = { kind: 'noNpx' } | { kind: 'badRepo' } | { kind: 'noSkills' } | { kind: 'timeout' } | { kind: 'failed'; detail: string };

export interface SkillInstalled {
  names: string[];
  outcome: SkillOutcome;
  error: NpxError | null;
}

export interface SkillUpdated {
  name: string;
  backup: string | null;
  error: NpxError | null;
}

/** "Cập nhật tất cả" or "Cập nhật cả bộ": how each skill ended, and the folder the copies are in. */
export interface SkillUpdatedMany {
  results: SkillUpdated[];
  backups: string;
}

export interface SkillRemoved {
  names: string[];
  backups: string[];
  error: NpxError | null;
}

export interface SkillGathered {
  moved: { name: string; home: Agent }[];
  /** The skill that could not be moved, and why. */
  failed: [string, string] | null;
  at: number;
}

export interface RepoList {
  repo: string;
  skills: { name: string; description: string; installed: boolean }[];
}

export interface SkillChecks {
  daily: boolean;
  notify: boolean;
}

export interface SkillsView {
  /** `null` until the tab first reads. */
  table: SkillTable | null;
  readAt: string | null;
  busy: 'reading' | 'writing' | 'installing' | 'updating' | 'removing' | 'importing' | null;
  /** While several skills update: `done` of `total` updated, `name` now. */
  progress: { done: number; total: number; name: string } | null;
  /** `npx` is on this Mac. */
  npx: boolean;
  last: { at: string; kind: 'batch' | 'install' | 'import' | 'update' | 'remove'; names: string[]; outcome: SkillOutcome | null; canUndo: boolean } | null;
  updates: {
    checkedAt: string | null;
    /** `offline`, `rateLimited`, … when the whole check failed. */
    error: string | null;
    /** Repositories GitHub did not answer for. */
    failed: string[];
    checking: boolean;
    settings: SkillChecks;
  };
}

/** `agents::plugins::Plugin`. */
export interface Plugin {
  /** `name@marketplace`. */
  id: string;
  enabled: boolean;
  version: string | null;
  brings: { mcp: number; skills: string[]; commands: number; hooks: boolean; agents: number };
}

export interface Marketplace {
  name: string;
  kind: 'github' | 'git' | 'local' | 'builtin' | 'other';
  source: string | null;
}

/** One agent's plugins: `noCli` without its command, `kept` from the newest pack. */
export interface PluginSide {
  agent: Agent;
  state: 'ok' | 'noCli' | 'failed' | 'kept';
  detail: string | null;
  plugins: Plugin[];
  marketplaces: Marketplace[];
}

export interface PluginsView {
  sides: PluginSide[] | null;
  readAt: string | null;
  busy: boolean;
  /** What a restore would run, by agent. */
  reinstall: Partial<Record<Agent, string[]>>;
}

/** `agents::instructions::FileView`. */
export interface InstrFile {
  /** `~/…`. */
  path: string;
  agent: Agent | null;
  modifiedAt: string | null;
  size: number;
  lines: number;
  /** Lines (from 1) shaped like a key: the file stays out of the pack. */
  keyLines: number[];
  error: string | null;
  include: boolean;
  /** Added with "Thêm file…". */
  extra: boolean;
  vsPack: 'same' | 'changed' | 'new' | 'noPack';
}

export interface InstrView {
  view: { files: InstrFile[]; packAt: number | null } | null;
  readAt: string | null;
}

export interface InstrShown {
  path: string;
  lines: { text: string; key: boolean; notKey: boolean }[];
}

// ===== Configuration sets (M15) =====

/** `agents::sets::Want`: an entry a set turns on or off; one not named stays as it is. */
export type SetWant = 'on' | 'off';

/** `agents::sets::Set`: named after a project, applied for the whole Mac. */
export interface ConfigSet {
  /** Empty for a new set: the Rust side gives it one. */
  id: string;
  name: string;
  mcp: Record<string, SetWant>;
  skills: Record<string, SetWant>;
  /** The side of the Codex switch the set wants; null: Giữ nguyên. */
  codex: CodexTarget | null;
}

/** `agents::sets::Drifted`: an entry no longer as the applied set says. */
export interface SetDrifted {
  kind: 'mcp' | 'skill';
  name: string;
  want: SetWant;
  agents: Agent[];
}

export interface SetsView {
  sets: ConfigSet[];
  applied: {
    id: string;
    name: string;
    at: string;
    /** null until the MCP and Skills tables are read. */
    drift: SetDrifted[] | null;
    canUndo: boolean;
    /** The side Codex was on before the set asked for the other (`chatgpt`, `custom`, `other`). */
    codexBefore: string | null;
  } | null;
  busy: 'applying' | 'undoing' | null;
  /** The name of the set deleted last, while it can be put back. */
  removed: string | null;
}

/** `agents::sets::Skipped`: an entry of the set that cannot be made as it says. */
export interface SetSkipped {
  kind: 'mcp' | 'skill';
  name: string;
  /** null: for every agent. */
  agent: Agent | null;
  /** `notHere`, `owned`, or the box's lock. */
  reason: string;
}

/** `agents::sets::Preview`: the MCP and Skills tabs' own previews of the set. */
export interface SetPreview {
  id: string;
  name: string;
  mcp: McpPreview;
  skills: SkillPreview;
  skipped: SetSkipped[];
  codex: CodexTarget | null;
}

/** `agents::sets::Sent`: what the preview showed, sent back with the write. */
export interface SetSent {
  mcp: McpChange[];
  mcpBases: Partial<Record<Agent, string>>;
  skills: SkillChange[];
  skillBases: Partial<Record<Agent, string>>;
}

/** `agents::sets::Outcome`. */
export interface SetOutcome {
  mcp: McpOutcome | null;
  skills: SkillOutcome | null;
  ok: boolean;
  at: number;
}

/** `agents::sets::Undone`. */
export interface SetUndone {
  skills: SkillOutcome | null;
  mcp: McpOutcome | null;
  /** Entries changed since the apply: left as they are. */
  left: string[];
  ok: boolean;
  codexBefore: string | null;
}
