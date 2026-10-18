// The calls the windows make to the Rust side. Components take an `Api`
// so tests can pass a fake one instead of Tauri's IPC.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Model, Outcome, Sessions, Tab } from './model';

export interface Api {
  getModel(): Promise<Model>;
  /** Asks for a read now; resolves to the count of finished reads at the time of asking. */
  refresh(): Promise<number>;
  /** Opens the main window on a tab, or brings it forward. */
  openWindow(tab: Tab): Promise<void>;
  /** The tab the main window was opened for. */
  mainTab(): Promise<Tab>;
  hidePopover(): Promise<void>;
  resizePopover(height: number): Promise<void>;
  /** Empty goes back to finding quota-axi in the usual places. Rejects with `notFound`. */
  setQuotaAxiPath(path: string): Promise<Model>;
  /** Empty goes back to the name macOS gives this Mac. */
  setMachineName(name: string): Promise<Model>;
  /** Lets quota-axi ask macOS for Keychain access once. */
  grantKeychain(): Promise<Model>;
  /** Live Claude Code sessions on this Mac; null when they cannot be counted. */
  claudeSessions(): Promise<Sessions | null>;
  /**
   * Runs `cswap switch N --json`, pauses the switcher until `holdUntil` (RFC 3339) when given, and
   * reports what the re-read five seconds later shows. Rejects with `busy` (a switch is running),
   * `unknownSlot` (not one of claude-swap's accounts) or `badTime` (`holdUntil` is not RFC 3339).
   */
  switchClaude(slot: number, holdUntil: string | null): Promise<Outcome>;
  /** Pauses automatic switching until `until` (RFC 3339), or until resumed when null. */
  pauseSwitching(until: string | null): Promise<Model>;
  /** Moves the end of a timed pause later. */
  extendPause(minutes: number): Promise<Model>;
  resumeSwitching(): Promise<Model>;
  onModel(handler: (model: Model) => void): Promise<() => void>;
  /** The main window is asked to show a tab while it is open. */
  onTab(handler: (tab: Tab) => void): Promise<() => void>;
}

export const tauriApi: Api = {
  getModel: () => invoke<Model>('get_model'),
  refresh: () => invoke<number>('refresh'),
  openWindow: (tab) => invoke('open_window', { tab }),
  mainTab: () => invoke<Tab>('main_tab'),
  hidePopover: () => invoke('hide_popover'),
  resizePopover: (height) => invoke('popover_resize', { height }),
  setQuotaAxiPath: (path) => invoke<Model>('set_quota_axi_path', { path }),
  setMachineName: (name) => invoke<Model>('set_machine_name', { name }),
  grantKeychain: () => invoke<Model>('grant_keychain'),
  claudeSessions: () => invoke<Sessions | null>('claude_sessions'),
  switchClaude: (slot, holdUntil) => invoke<Outcome>('switch_claude', { slot, holdUntil }),
  pauseSwitching: (until) => invoke<Model>('pause_switching', { until }),
  extendPause: (minutes) => invoke<Model>('extend_pause', { minutes }),
  resumeSwitching: () => invoke<Model>('resume_switching'),
  onModel: (handler) => listen<Model>('model', (event) => handler(event.payload)),
  onTab: (handler) => listen<Tab>('tab', (event) => handler(event.payload)),
};
