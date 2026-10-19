// The calls the windows make to the Rust side. Components take an `Api`
// so tests can pass a fake one instead of Tauri's IPC.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  CodexOutcome,
  CodexRestart,
  CodexTarget,
  AgyApp,
  AgyHow,
  AgyLoginDone,
  AgyOutcome,
  AlertKind,
  Decision,
  HistoryEntry,
  Model,
  Outcome,
  Peer,
  PeerTest,
  Rules,
  Sessions,
  Tab,
  Theme,
  Menubar,
  Agent,
  McpChange,
  McpOutcome,
  McpPaste,
  McpPreview,
} from './model';

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
  /**
   * Writes the rules on this Mac, has quota-axi check them and copies them to the other Macs.
   * `base` is the file version the editor started from. Rejects with a `RulesError`.
   */
  saveRotation(rules: Rules, base: string | null): Promise<Model>;
  /** Takes the other Mac's copy as this Mac's file, then copies it out. Rejects with a `RulesError`. */
  adoptRotation(alias: string, base: string | null): Promise<Model>;
  /** Stamps this Mac's file again so it is the newest, then copies it out. Rejects with a `RulesError`. */
  keepRotation(base: string | null): Promise<Model>;
  /** Tries every Mac that is behind now. */
  retryRotation(): Promise<Model>;
  /** Rejects with `badAlias:<i>`, `sameAlias:<i>` or `tooMany`. */
  setPeers(peers: Peer[]): Promise<Model>;
  /** Reads the other Mac's rotation file over SSH; changes nothing. */
  testPeer(alias: string): Promise<PeerTest>;
  /**
   * The switcher's `--dry-run` with these rules. Rejects with a quota-axi reason
   * (`rotation_bad_value:<key>`, `rotation_personal_in_team`) for rules that
   * cannot be written, or with the dry run's error kind (`notFound`, `timeout`, …).
   */
  previewRotation(rules: Rules): Promise<{ decision: Decision; raw: string }>;
  /** The editor holds unsaved changes: closing the main window asks first. */
  rulesDirty(dirty: boolean): Promise<void>;
  /** Closes the main window, or quits the app with `quit`, without asking again. */
  closeMain(quit: boolean): Promise<void>;
  /** The main window was asked to close while the editor held unsaved changes. */
  onCloseRequested(handler: () => void): Promise<() => void>;
  /** `Thoát Quota Station` was chosen while the editor held unsaved changes. */
  onQuitRequested(handler: () => void): Promise<() => void>;
  /** Takes a pending quit request: true once per request, also when the window opened for it before listening. */
  takeQuit(): Promise<boolean>;
  /**
   * Adds an Antigravity account, or logs one in again, through Google's consent screen in the
   * browser; resolves once the account is kept. Rejects with an `AgyLoginFailure`.
   */
  agyLogin(): Promise<AgyLoginDone>;
  /**
   * The callback URL pasted for the waiting login. Rejects with `noLogin`, `stateMismatch`,
   * `noCode`, `noState` or `badUrl`, and the login keeps waiting.
   */
  agyLoginCallback(url: string): Promise<void>;
  agyLoginCancel(): Promise<void>;
  /** Logs an account out of this app. Rejects with `notFound`, `keychain` or `index`. */
  agyLogout(id: string): Promise<Model>;
  agyOpenVerification(id: string): Promise<void>;
  /** The Antigravity tab opened: the app reads every account now, unless read in the last minute. */
  agyTabOpened(): Promise<void>;
  /** Shows or hides account names (emails, organizations) in both windows. */
  setShowNames(on: boolean): Promise<Model>;
  setTheme(theme: Theme): Promise<Model>;
  /** The service the menu bar title follows; stays on this Mac. */
  setMenubar(menubar: Menubar): Promise<Model>;
  /** Is Antigravity.app installed, which version, does it run; asked when the switch sheet opens. */
  agyAppState(): Promise<AgyApp>;
  /**
   * Switches Antigravity to the account `id` the way `how` says, and reports what quota-axi shows
   * five seconds later. Rejects with `busy` (a switch is running), `loggingOut`, `loggingIn`,
   * `unknownAccount` or `index`.
   */
  switchAgy(id: string, how: AgyHow): Promise<AgyOutcome>;
  /** "Thôi" on the overwrite banner: stops watching the last switch. */
  agyDismissOverwrite(): Promise<Model>;
  /**
   * Switches Codex after the owner confirmed it, when `config.toml` still has the hash the sheet
   * showed. Rejects with `busy` while a switch or a restart runs.
   */
  codexSwitch(target: CodexTarget, base: string | null): Promise<CodexOutcome>;
  /** Restarts Codex's background process, after its own confirmation. Rejects with `busy`. */
  codexRestart(afterSwitch: boolean): Promise<CodexRestart>;
  /** Keeps `config.toml` as changed outside the app; it is not reported again. */
  codexKeep(): Promise<Model>;
  /** The Codex tab opened, or "Kiểm lại": look at Codex now and count the resumable sessions. */
  codexCheck(): Promise<void>;
  /** Codex's folder; empty goes back to `~/.codex`. Rejects with `notFound`. */
  setCodexHome(path: string): Promise<Model>;
  /** The MCP tab opened, or "Đọc lại": every agent's list read again; the table comes in the model. */
  mcpRead(): Promise<void>;
  /** What `changes` would write, read from the files now, secrets masked. */
  mcpPreview(changes: McpChange[]): Promise<McpPreview>;
  /** Writes the batch the preview showed; `bases` are the files as it read them. Rejects with `busy`. */
  mcpWrite(changes: McpChange[], bases: McpPreview['bases']): Promise<McpOutcome>;
  /** "Hoàn tác": the last write put back. Rejects with `nothing` or `busy`. */
  mcpUndo(): Promise<McpOutcome>;
  /** A key typed or pasted: straight to the Keychain, with the Bitwarden item it is in. Never sent back. */
  mcpSaveSecret(name: string, slot: string, value: string, bitwarden: string | null): Promise<void>;
  /** The servers in a pasted JSON or TOML; with `save`, their secrets go to the Keychain first, with the Bitwarden item. */
  mcpPaste(text: string, save: boolean, bitwarden: string | null): Promise<McpPaste>;
  /** "Xoá hẳn": out of the app's store, and with `keys` out of the Keychain. */
  mcpForget(name: string, slots: string[], keys: boolean): Promise<void>;
  /** An agent's backups folder in Finder. Rejects with `missing`. */
  mcpOpenBackups(agent: Agent): Promise<void>;
  /**
   * A button on an alert card in the Cảnh báo tab, the same as on the macOS card. A switch back
   * runs on the Rust side and its result comes in the model; `openSwitch` asks this window to open
   * its switch sheet. Rejects with `answered`, `unknown` or `noSuchAction`.
   */
  alertAction(id: string, action: string): Promise<{ openSwitch: number | null }>;
  /** "Gửi thử": a sample card of this kind, with buttons that change nothing. */
  testAlert(kind: AlertKind): Promise<Model>;
  setAlertSwitch(kind: AlertKind, on: boolean): Promise<Model>;
  /** Firstmate's `quota-pool.last`; empty forgets it. Rejects with `notPoolFile` or `notFound`. */
  setFirstmateFile(path: string): Promise<Model>;
  /** The History tab's lines, newest first. */
  getHistory(): Promise<HistoryEntry[]>;
  /** Asks macOS again whether cards may show; the answer comes in the model. */
  notifyRefresh(): Promise<void>;
  /** System Settings › Notifications. */
  openNotificationSettings(): Promise<void>;
  /** The Cài đặt tab opened: asks macOS again whether the app launches at login. */
  settingsOpened(): Promise<void>;
  /** Turns launching at login on or off. Rejects with `unavailable` or macOS's message. */
  setLaunchAtLogin(on: boolean): Promise<Model>;
  /** System Settings › General › Login Items, where macOS asks for approval. */
  openLoginItems(): Promise<void>;
  /** The clone to update from; empty forgets it. Rejects with `missing` or `notSource`. */
  setSourceDir(path: string): Promise<Model>;
  /**
   * `git pull --ff-only` in the clone, then its `scripts/install.sh`; the steps and the result come
   * in the model. Rejects with `busy`, `noSource` or `unbundled`.
   */
  updateFromSource(): Promise<Model>;
  /** Quits and opens the copy just installed. Rejects with `notInstalled`, `rulesDirty` or `unbundled`. */
  relaunch(): Promise<void>;
  /** A button on a macOS card asks the popover to open its switch sheet for a Claude account. */
  onOpenSwitch(handler: (slot: number) => void): Promise<() => void>;
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
  saveRotation: (rules, base) => invoke<Model>('save_rotation', { rules, base }),
  adoptRotation: (alias, base) => invoke<Model>('adopt_rotation', { alias, base }),
  keepRotation: (base) => invoke<Model>('keep_rotation', { base }),
  retryRotation: () => invoke<Model>('retry_rotation'),
  setPeers: (peers) => invoke<Model>('set_peers', { peers }),
  testPeer: (alias) => invoke<PeerTest>('test_peer', { alias }),
  previewRotation: (rules) => invoke('preview_rotation', { rules }),
  rulesDirty: (dirty) => invoke('rules_dirty', { dirty }),
  closeMain: (quit) => invoke('close_main', { quit }),
  onCloseRequested: (handler) => listen('closeRequested', () => handler()),
  onQuitRequested: (handler) => listen('quitRequested', () => handler()),
  takeQuit: () => invoke<boolean>('take_quit'),
  agyLogin: () => invoke<AgyLoginDone>('agy_login'),
  agyLoginCallback: (url) => invoke('agy_login_callback', { url }),
  agyLoginCancel: () => invoke('agy_login_cancel'),
  agyLogout: (id) => invoke<Model>('agy_logout', { id }),
  agyOpenVerification: (id) => invoke('agy_open_verification', { id }),
  agyTabOpened: () => invoke('agy_tab_opened'),
  setShowNames: (on) => invoke<Model>('set_show_names', { on }),
  setTheme: (theme) => invoke<Model>('set_theme', { theme }),
  setMenubar: (menubar) => invoke<Model>('set_menubar', { menubar }),
  agyAppState: () => invoke<AgyApp>('agy_app_state'),
  switchAgy: (id, how) => invoke<AgyOutcome>('switch_agy', { id, how }),
  agyDismissOverwrite: () => invoke<Model>('agy_dismiss_overwrite'),
  codexSwitch: (target, base) => invoke<CodexOutcome>('codex_switch', { target, base }),
  codexRestart: (afterSwitch) => invoke<CodexRestart>('codex_restart', { afterSwitch }),
  codexKeep: () => invoke<Model>('codex_keep'),
  codexCheck: () => invoke('codex_check'),
  setCodexHome: (path) => invoke<Model>('set_codex_home', { path }),
  mcpRead: () => invoke('mcp_read'),
  mcpPreview: (changes) => invoke<McpPreview>('mcp_preview', { changes }),
  mcpWrite: (changes, bases) => invoke<McpOutcome>('mcp_write', { changes, bases }),
  mcpUndo: () => invoke<McpOutcome>('mcp_undo'),
  mcpSaveSecret: (name, slot, value, bitwarden) => invoke('mcp_save_secret', { name, slot, value, bitwarden }),
  mcpPaste: (text, save, bitwarden) => invoke<McpPaste>('mcp_paste', { text, save, bitwarden }),
  mcpForget: (name, slots, keys) => invoke('mcp_forget', { name, slots, keys }),
  mcpOpenBackups: (agent) => invoke('mcp_open_backups', { agent }),
  alertAction: (id, action) => invoke('alert_action', { id, action }),
  testAlert: (kind) => invoke<Model>('test_alert', { kind }),
  setAlertSwitch: (kind, on) => invoke<Model>('set_alert_switch', { kind, on }),
  setFirstmateFile: (path) => invoke<Model>('set_firstmate_file', { path }),
  getHistory: () => invoke<HistoryEntry[]>('get_history'),
  notifyRefresh: () => invoke('notify_refresh'),
  openNotificationSettings: () => invoke('open_notification_settings'),
  settingsOpened: () => invoke('settings_opened'),
  setLaunchAtLogin: (on) => invoke<Model>('set_launch_at_login', { on }),
  openLoginItems: () => invoke('open_login_items'),
  setSourceDir: (path) => invoke<Model>('set_source_dir', { path }),
  updateFromSource: () => invoke<Model>('update_from_source'),
  relaunch: () => invoke('relaunch'),
  onOpenSwitch: (handler) => listen<number>('openSwitch', (event) => handler(event.payload)),
  onModel: (handler) => listen<Model>('model', (event) => handler(event.payload)),
  onTab: (handler) => listen<Tab>('tab', (event) => handler(event.payload)),
};
