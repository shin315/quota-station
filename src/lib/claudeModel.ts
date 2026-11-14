// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, as the Rust side sends them
// (`src-tauri/src/claude.rs`, `quota-station-core::claude::state`), part of
// the model (`model.ts` exports them all). Field names are serde's
// camelCase; `Option` arrives as null.

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
  /** `liveExpired`: the account in use, whose login in Claude Code itself expired over 15 minutes ago and was not refreshed. */
  status: 'fresh' | 'stale' | 'noData' | 'reloginRequired' | 'liveExpired';
  fetchedAt: string | null;
  ageSeconds: number | null;
  windows: { fiveHour: ClaudeWindowOut | null; sevenDay: ClaudeWindowOut | null; fable: ClaudeWindowOut | null };
  /** The last read's error kind (`http-429`, `invalid_grant`…). */
  error: string | null;
  /** Only while names show. */
  email?: string | null;
}

export type ClaudeAccountAction =
  | 'import'
  | 'login'
  | 'current'
  | 'token'
  | 'remove'
  | 'alias'
  | 'enabled'
  | 'rotation'
  ;

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
  /** The renewed account is the one Claude Code is on: whether Claude Code's own login got the new sign-in too. Absent otherwise. */
  live?: boolean;
  imported: ClaudeImported | null;
}

export interface ClaudeAccountsView {
  /** Quota Station keeps the accounts on this Mac. */
  ready: boolean;
  /** The other account switcher's folder is on this Mac, to import from. */
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
  /** "Tự chuyển khi hết quota": the percent of the fullest window (5-hour or week) at which it switches. */
  autoSwitchAt: number;
  /**
   * What the rotation follows: `auto`, the threshold rule (no rotation rules
   * file); `rules`, the rotation rules file (Nâng cao); `stopped`, a rules
   * file that cannot be followed.
   */
  autoPolicy: 'auto' | 'rules' | 'stopped';
}
