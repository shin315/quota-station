// The calls the windows make to the Rust side. Components take an `Api`
// so tests can pass a fake one instead of Tauri's IPC.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Model, Tab } from './model';

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
  onModel: (handler) => listen<Model>('model', (event) => handler(event.payload)),
  onTab: (handler) => listen<Tab>('tab', (event) => handler(event.payload)),
};
