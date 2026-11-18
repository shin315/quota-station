// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Antigravity accounts the app logs in (M4a) and reads (M4b), as the Rust side sends them
// (`src-tauri/src/antigravity.rs`); `model.ts` passes them on.

/**
 * Whether the app has an OAuth client to sign in with (`quota-station-core::antigravity::client`):
 * `ok`, a usable `oauth-client.json` or the installed Antigravity's client; `missing`, no file and
 * no Antigravity installed; `unrecognized`, no file and an Antigravity build whose client the app
 * does not know; `tooOpen`, `invalid` and `unreadable` are about the file.
 */
export type ClientStatus = 'ok' | 'missing' | 'unrecognized' | 'unreadable' | 'tooOpen' | 'invalid';

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
  /** Held out of the Antigravity rotation by the owner ("Tạm dừng tự chuyển"): it never switches to it. */
  rotationPaused: boolean;
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
  /** The account kept: the one Google's page was signed in to, maybe not the one meant. */
  id: string;
  number: number;
  /** A new account, not a new login of one already listed. */
  added: boolean;
}

/** A login that kept nothing: `LoginError::kind` in Rust, plus `busy` and `spawn`. */
export interface AgyLoginFailure {
  kind: string;
  detail: string;
}
