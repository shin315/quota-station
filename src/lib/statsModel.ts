// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Thống kê tab's types (M16), as the Rust side sends them
// (`src-tauri/src/stats.rs`, `stats::report`): `model.ts` re-exports them.

import type { Agent } from './model';

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
  agy: number;
}

export interface StatsModelRow {
  model: string;
  agent: Agent;
  /** Input, written to a cache or not. */
  input: number;
  cacheRead: number;
  output: number;
  total: number;
  /** null: the price table does not have the model; Antigravity's are never priced (its quota is not by the token). */
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
  /** The account's slot (Antigravity: its number in the app's list); null when the switch history cannot tell. */
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
  agy: number;
  total: number;
  usd: number;
  unpriced: string[];
  models: StatsModelRow[];
  projects: StatsProjectRow[];
  accounts: StatsAccountRow[];
  /** Antigravity's, by the number the app gives its accounts. */
  agyAccounts: StatsAccountRow[];
  /** `2026-10-19`. */
  pricesAsOf: string;
  pricesSource: string;
  skipped: StatsSkipped[];
  /** Unix seconds. */
  readAt: number | null;
}
