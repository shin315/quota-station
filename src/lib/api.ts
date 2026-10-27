// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The calls the windows make to the Rust side. Components take an `Api`
// so tests can pass a fake one instead of Tauri's IPC.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { SessionView } from '../session/steps';
import type {
  CopyComparison,
  CopySelection,
  PackSummary,
  RestoreDone,
  RestoreMore,
  RestorePartResult,
  RestorePlan,
  RestoreUndone,
  CodexAccountDone,
  CodexImportPreview,
  CodexKeyOutcome,
  CodexKeyPreview,
  CodexOutcome,
  CodexProviderCheck,
  CodexRestart,
  CodexTarget,
  CodexTrigger,
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
  SettingsGroup,
  StatsPeriod,
  StatsReport,
  Tab,
  Theme,
  ThemePack,
  Language,
  Menubar,
  MenubarWindow,
  Agent,
  CliTool,
  McpChange,
  McpOutcome,
  McpPaste,
  McpPreview,
  McpLine,
  InstrShown,
  RepoList,
  SkillChange,
  SkillGathered,
  SkillInstalled,
  SkillOutcome,
  SkillPreview,
  SkillRemoved,
  SkillUpdated,
  SkillUpdatedMany,
  ConfigSet,
  SetOutcome,
  SetPreview,
  SetSent,
  SetUndone,
} from './model';

export interface Api {
  getModel(): Promise<Model>;
  /** Asks for a read now; resolves to the count of finished reads at the time of asking. */
  refresh(): Promise<number>;
  /** Opens the main window on a tab, or brings it forward; `alerts` opens the bell's panel over the page. */
  openWindow(tab: Tab): Promise<void>;
  /** The tab the main window was opened for. */
  mainTab(): Promise<Tab>;
  /** The group Cài đặt shows: the last one picked while the app runs, or the one a link opened it on. */
  settingsGroup(): Promise<SettingsGroup>;
  setSettingsGroup(group: SettingsGroup): Promise<void>;
  hidePopover(): Promise<void>;
  resizePopover(height: number): Promise<void>;
  /** Empty goes back to the name macOS gives this Mac. */
  setMachineName(name: string): Promise<Model>;
  /** Reads Claude Code's login once, waiting while macOS asks the owner for Keychain access. */
  grantKeychain(): Promise<Model>;
  /** Live Claude Code sessions on this Mac; null when they cannot be counted. */
  claudeSessions(): Promise<Sessions | null>;
  /**
   * Switches Claude Code's login to the app's account `slot`, pauses the rotation until `holdUntil`
   * (RFC 3339) when given, and reports what the second look five seconds later shows. Rejects with
   * `busy` (a switch is running), `unknownSlot` or `badTime` (`holdUntil` is not RFC 3339).
   */
  switchClaude(slot: number, holdUntil: string | null): Promise<Outcome>;
  /** Carries claude-swap's accounts over; claude-swap is left as it is. The result is in `claudeAccounts.last`. */
  claudeImport(): Promise<Model>;
  /** Logs a new account in with `claude auth login` in a folder of its own; waits for the browser. */
  claudeAddLogin(): Promise<Model>;
  claudeCancelLogin(): Promise<void>;
  /** Adds the login Claude Code is on now. */
  claudeAddCurrent(): Promise<Model>;
  /** Adds a `claude setup-token` token, with the account's email when given. */
  claudeAddToken(token: string, email: string | null): Promise<Model>;
  claudeRemove(slot: number): Promise<Model>;
  claudeSetAlias(slot: number, alias: string): Promise<Model>;
  claudeSetEnabled(slot: number, enabled: boolean): Promise<Model>;
  /** Loads or unloads the rotation's LaunchAgent. */
  claudeSetRotation(on: boolean): Promise<Model>;
  /** Pauses automatic switching until `until` (RFC 3339), or until resumed when null. */
  pauseSwitching(until: string | null): Promise<Model>;
  /** Moves the end of a timed pause later. */
  extendPause(minutes: number): Promise<Model>;
  resumeSwitching(): Promise<Model>;
  /**
   * Writes the rules on this Mac once they keep the file's rules, and copies them to the other Macs.
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
   * The switcher's `--dry-run` with these rules. Rejects with the rules' reason
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
  /** The theme pack of every window. */
  setThemePack(pack: ThemePack): Promise<Model>;
  /** Vietnamese, English, or back to macOS's language. */
  setLanguage(language: Language): Promise<Model>;
  /** The first run's last step: it does not show again; `popover` closes the window and opens the popover. */
  finishSetup(open: 'window' | 'popover'): Promise<Model>;
  /** The service the menu bar title follows; stays on this Mac. */
  setMenubar(menubar: Menubar): Promise<Model>;
  /** The window the menu bar title shows; stays on this Mac. */
  setMenubarWindow(window: MenubarWindow): Promise<Model>;
  /** Is Antigravity.app installed, which version, does it run; asked when the switch sheet opens. */
  agyAppState(): Promise<AgyApp>;
  /**
   * Switches Antigravity to the account `id` the way `how` says, and reports what agy shows
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
  /** "Dán khoá": what writing `key` (and `baseUrl`) would change; the key comes back only as its hint. */
  codexKeyPreview(key: string, baseUrl: string | null): Promise<CodexKeyPreview>;
  /** Writes the key after the owner confirmed the preview whose hash is `base`. Rejects with `busy`. */
  codexSetKey(key: string, baseUrl: string | null, base: string): Promise<CodexKeyOutcome>;
  /** "Hoàn tác" after "Dán khoá". Rejects with `busy`. */
  codexUndoKey(base: string): Promise<CodexKeyOutcome>;
  /** "Dùng plugin với Custom API": kept for the next switch; writes nothing to Codex. */
  setCodexPlugins(on: boolean): Promise<Model>;
  /**
   * Switches Codex to one account after the owner confirmed it: a ChatGPT
   * sign-in (its `auth.json` moves into the Codex folder, the one there moves
   * back to its own folder) or a custom provider (`model_provider`). Rejects with `busy`.
   */
  codexUse(id: string, base: string | null): Promise<CodexOutcome>;
  /**
   * Signs a new ChatGPT account in, in a Codex folder of its own; `id` signs that account in again.
   * `device`: with a code typed on ChatGPT's page (`model.codex.loginCode`) instead of the browser coming back.
   */
  codexLogin(id: string | null, device?: boolean): Promise<CodexAccountDone>;
  /** Stops the sign-in that waits for the browser. */
  codexCancelLogin(): Promise<void>;
  /** "Mở lại trang đăng nhập": the page the sign-in under way printed, in the browser. */
  codexReopenLogin(): Promise<void>;
  /** "Kiểm tra" on a new provider: its address with no key, then with the key. The key goes only there. */
  codexCheckProvider(baseUrl: string, key: string): Promise<CodexProviderCheck>;
  /** The lines a new provider adds to `config.toml`; the key only as its hint. */
  codexProviderPreview(name: string, baseUrl: string, key: string): Promise<CodexKeyPreview>;
  /** Adds the provider the preview showed, when `config.toml` still has the hash `base`. */
  codexAddProvider(name: string, baseUrl: string, key: string, base: string): Promise<CodexAccountDone>;
  /** A nickname for one account, kept by the app; empty goes back to its own name. */
  codexRename(id: string, name: string): Promise<CodexAccountDone>;
  /** Removes an account not in use; `logout` signs a ChatGPT one out first. */
  codexRemove(id: string, logout: boolean, base: string | null): Promise<CodexAccountDone>;
  /** "Nhập file đăng nhập (JSON)": the system's file picker, then what the files hold; null when the owner picked none. */
  codexImportPick(): Promise<CodexImportPreview | null>;
  /** "Dán JSON": what JSON pasted in the window holds. Rejects with `large` over 1 MB. */
  codexImportText(text: string): Promise<CodexImportPreview>;
  /** Imports what the preview read; `removeSource` deletes the files picked once their sign-ins are in the app. */
  codexImport(token: string, removeSource: boolean): Promise<CodexAccountDone>;
  /** "Tự đánh thức": its settings, kept by the app. */
  codexTriggerSet(t: Pick<CodexTrigger, 'on' | 'mode' | 'times' | 'keepAlive' | 'ids'>): Promise<void>;
  /** "Chạy ngay": wakes the chosen accounts now, one after another. */
  codexTriggerNow(): Promise<void>;
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
  /** The Skills tab opened, or "Đọc lại": every agent's skills read again; the table comes in the model. */
  skillsRead(): Promise<void>;
  /** What turning the boxes on or off would write, read from the folders and `config.toml` now. */
  skillsPreview(changes: SkillChange[]): Promise<SkillPreview>;
  /** Writes the batch the preview showed. Rejects with `busy`. */
  skillsWrite(changes: SkillChange[], bases: SkillPreview['bases']): Promise<SkillOutcome>;
  /** "Hoàn tác" of the last batch, install or gathering: `undone` or `failed`. Rejects with `nothing` or `busy`. */
  skillsUndo(): Promise<'undone' | 'failed'>;
  /** "Bỏ liên kết": a link to nothing in an agent's folder; never a folder. */
  skillsDropLink(agent: Agent, name: string): Promise<void>;
  /** The skills in a GitHub repository, through `npx skills`. Rejects with `badRepo`, `noNpx`, `noSkills`, `timeout`, `failed:…`. */
  skillsListRepo(repo: string): Promise<RepoList>;
  /** "Đọc SKILL.md" before installing. */
  skillsReadMd(repo: string, skill: string): Promise<string>;
  /** "Cài": `npx skills add`, then each agent's box as asked. Rejects with `busy`, `noNpx`, `badRepo`. */
  skillsInstall(repo: string, names: string[], agents: Record<Agent, boolean>): Promise<SkillInstalled>;
  /** "Cập nhật": a copy kept first; the boxes stay. */
  skillsUpdate(name: string): Promise<SkillUpdated>;
  /** "Cập nhật tất cả" and "Cập nhật cả bộ": a copy of each kept first, then one update at a time; the boxes stay. */
  skillsUpdateMany(names: string[]): Promise<SkillUpdatedMany>;
  /** "Bỏ qua bản này". */
  skillsSkip(name: string): Promise<void>;
  /** "Gỡ hẳn": a copy kept first, then `npx skills remove`. */
  skillsRemove(name: string): Promise<SkillRemoved>;
  /** "Gom": skills installed by hand moved into the shared folder. */
  skillsImport(names: string[]): Promise<SkillGathered>;
  /** A skill's history on GitHub, in the browser. */
  skillsOpenLink(url: string): Promise<void>;
  /** "Kiểm tra bản mới": one question to GitHub per repository in the lock file. */
  skillsCheck(): Promise<void>;
  /** "Kiểm tra bản mới" of the app: asks GitHub Releases now. */
  releaseCheck(): Promise<void>;
  /** "Bỏ qua bản này" (a version), or its Hoàn tác (null). */
  releaseSkip(version: string | null): Promise<void>;
  /** "Để sau": the version's banner and popover line go quiet. */
  releaseLater(version: string): Promise<void>;
  setReleaseChecks(on: boolean): Promise<void>;
  /** Cập nhật Agent's "Kiểm tra ngay" (and a row's "Kiểm tra lại"): each tool's version, install, sessions and newest, read again. */
  cliToolsCheck(): Promise<void>;
  /** "Cập nhật": the tool's own update command. Rejects with `busy`, `handsOff`, `notInstalled`; how it ended is in the model. */
  cliToolUpdate(tool: CliTool): Promise<void>;
  /** Cập nhật Agent opened: while "Tự động kiểm tra" is on, a check when the last one is missing or a day old. */
  cliToolsOpened(): Promise<void>;
  /** Cập nhật Agent's "Tự động kiểm tra". */
  setCliChecks(daily: boolean): Promise<void>;
  /** Opens the latest disk image (`download`), its page (`notes`), the Releases page, or the bug form. */
  releaseOpen(what: 'download' | 'notes' | 'releases' | 'issue'): Promise<void>;
  /** The diagnostic info, exactly as "Chép" copies it. */
  releaseDiagnostics(): Promise<string>;
  /** Cài đặt › Skills. */
  skillsSetChecks(daily: boolean, notify: boolean): Promise<void>;
  /** "Kiểm lại", and Cài đặt opening: the shell files and launchd, by name (M14). */
  envCheck(): Promise<void>;
  /** "Mở file" at the line. Resolves to the editor that went to the line, or null (the default editor, no line). Rejects with `notFound`, `noFile`, `open`. */
  envOpen(id: string): Promise<string | null>;
  /** "Bỏ qua" on one place, and its "Hoàn tác". */
  envIgnore(id: string): Promise<void>;
  envUnignore(id: string): Promise<void>;
  /** "Lưu bộ" (M15): a new set (empty id) or one edited. Rejects with `name`, `taken`, `badEntry:…`, `full`, `io`. */
  setsSave(set: ConfigSet): Promise<ConfigSet>;
  /** "Xoá": the set goes; no agent file changes. */
  setsDelete(id: string): Promise<void>;
  /** "Hoàn tác" of the last delete. */
  setsUndelete(): Promise<void>;
  /** "Áp…": what the set would write, read from the files now. */
  setsPreview(id: string): Promise<SetPreview>;
  /** "Áp bộ": what the preview showed. Rejects with `busy`, `missing`, `changed:<agent>`, `notTheSet`. */
  setsApply(id: string, sent: SetSent): Promise<SetOutcome>;
  /** "Hoàn tác bộ": every entry the last apply changed, put back. Rejects with `busy`, `nothing`. */
  setsUndo(): Promise<SetUndone>;
  /** The Thống kê tab opened, or "Đọc lại" (M16): what the session logs gained, read off the UI thread. */
  statsRead(): Promise<void>;
  /** The tab's numbers for a period, from the index. */
  statsReport(period: StatsPeriod): Promise<StatsReport>;
  /** "Đã biết" on the notice of skipped logs. Rejects with `busy` while a read runs. */
  statsAck(): Promise<void>;
  /** The Plugin tab opened, or "Đọc lại": each agent's own `plugin list`. */
  pluginsRead(): Promise<void>;
  /** The File hướng dẫn tab opened, or "Đọc lại". */
  instrRead(): Promise<void>;
  /** "Xem": the file, keys masked. Rejects with `missing`, `large`, `binary`. */
  instrShow(path: string): Promise<InstrShown>;
  /** "So sánh": the file against the newest pack's copy, keys masked. Rejects with `noPack` or `notInPack`. */
  instrCompare(path: string): Promise<McpLine[]>;
  /** "Trong gói". */
  instrInclude(path: string, on: boolean): Promise<void>;
  /** "Không phải khoá" for one line (from 1). */
  instrNotKey(path: string, line: number): Promise<void>;
  /** "Thêm file…": a `.md` or `.txt` in the home folder. Resolves to how it is shown; rejects with `outside`, `type`, `missing`, `large`. */
  instrAdd(path: string): Promise<string>;
  /** A file added by hand leaves the list. */
  instrForget(path: string): Promise<void>;
  /** "Mở file": in the default text editor. */
  instrOpen(path: string): Promise<void>;
  /** The Sao lưu tab opened, or "Kiểm lại": the packs and the keys listed again; they come in the model. */
  backupRead(): Promise<void>;
  /** "Sao lưu bây giờ": the new pack's name. Rejects with `busy`, `noICloud`, `secret` or `io:…`. */
  backupNow(): Promise<string>;
  /** "Tự sao lưu": after each change in the app, and once a day when an agent changed. */
  backupSetAuto(afterChange: boolean, daily: boolean): Promise<void>;
  /** Where the packs go from now on; empty is iCloud Drive. Rejects with `notFound`. */
  backupSetFolder(path: string): Promise<void>;
  /** The packs' folder in Finder. Rejects with `missing`. */
  backupOpenFolder(): Promise<void>;
  /** "Xoá khoá": the Keychain item goes; the agents keep their own copy. */
  backupDeleteKey(mcp: string, slot: string): Promise<void>;
  /** "Xem trong gói". Rejects with `cloud`, `missing`, `newer`, `invalid:…`. */
  packSummary(id: string): Promise<PackSummary>;
  /** Khôi phục, step 2: what the pack would do on this Mac. Writes nothing. */
  restorePlan(id: string): Promise<RestorePlan>;
  /**
   * Khôi phục, step 4. The pasted keys go to the Keychain and are never sent back; `bases` are
   * the files as the plan read them. Rejects with `busy`, `changed:<agents>`, `keychain` or a pack error.
   */
  restoreRun(
    id: string,
    picks: { name: string; kind: string }[],
    more: RestoreMore,
    keys: { mcp: string; slot: string; value: string }[],
    bases: RestorePlan['bases'],
  ): Promise<RestoreDone>;
  /** "Thử lại" on one skill, plugin or instruction file of the last restore. Rejects with `busy` or a pack error. */
  restoreRetry(id: string, part: RestorePartResult['part'], item: string, agent: Agent | null): Promise<RestorePartResult>;
  /**
   * "Hoàn tác lần khôi phục"; `force` once the owner agreed to what changed since. Rejects with `nothing`,
   * `changed:<agents>` (then `file:<~/…>` and `skill:<name>` lines), `busy`, `failed`.
   */
  restoreUndo(force: boolean): Promise<RestoreUndone>;
  /**
   * Chép sang Mac khác (M13), "So sánh": reads the other Mac over SSH and this one; writes nothing.
   * Rejects with `unreachable:<line>`, `timeout`, `noApp`, `old`, `noSsh`, `unknownMac`, `busy`,
   * `refused:<why>`, `failed:<line>` or `reply`.
   */
  copyCompare(alias: string): Promise<CopyComparison>;
  /**
   * "Chép": the items ticked, `push` from this Mac to the other (written there by its own app),
   * else from it to this Mac. Keys go only with `keys`. Resolves to what the restore did there or
   * here; rejects as `copyCompare` does, or with `stale`, `changed:<agents>`, `keychain`, `skill`,
   * `lost` or `waiting`.
   */
  copyRun(alias: string, push: boolean, selection: CopySelection, keys: boolean): Promise<RestoreDone>;
  /** "Hoàn tác" of the copy that ran at `at`, on the Mac it was written to. Rejects with `gone`, `changed:…`, … */
  copyUndo(alias: string, push: boolean, at: number, force: boolean): Promise<RestoreUndone>;
  /**
   * A button on a card in the bell's panel (Thông báo), the same as on the macOS card. A switch back
   * runs on the Rust side and its result comes in the model; `openSwitch` asks this window to open
   * its switch sheet. Rejects with `answered`, `unknown` or `noSuchAction`.
   */
  alertAction(id: string, action: string): Promise<{ openSwitch: number | null }>;
  /** The bell's panel opened: every card in it is read, and the count goes. */
  alertsRead(): Promise<void>;
  /** "Gửi thử": a sample card of this kind, with buttons that change nothing. */
  testAlert(kind: AlertKind): Promise<Model>;
  setAlertSwitch(kind: AlertKind, on: boolean): Promise<Model>;
  /** Alert 1's thresholds in percent left; rejects with `range` or `order`. */
  setPoolAlert(dryAtMost: number, roomAtLeast: number): Promise<Model>;
  /** Cài đặt › Nâng cao on or off. */
  setAdvanced(on: boolean): Promise<Model>;
  /** The Firstmate switch inside Nâng cao. */
  setFirstmate(on: boolean): Promise<Model>;
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
  /** The main window is asked to show a tab while it is open (`alerts`: the bell's panel). */
  onTab(handler: (tab: Tab) => void): Promise<() => void>;
  /** Whether macOS reduces transparency (Accessibility › Display): the windows then show the solid ground. */
  reduceTransparency(): Promise<boolean>;
  /** macOS's Reduce Transparency changed, as a window learns when it comes forward. */
  onReduceTransparency(handler: (on: boolean) => void): Promise<() => void>;
  /** macOS increases contrast (Accessibility › Display): the edges are drawn stronger. */
  increaseContrast(): Promise<boolean>;
  /** macOS's Increase Contrast changed, as a window learns when it comes forward. */
  onIncreaseContrast(handler: (on: boolean) => void): Promise<() => void>;
  /** A test build's guided session (`session.rs`): where the guide is, and what this PC has. */
  sessionGet(): Promise<SessionView>;
  /** The guide shows step `id`; the app takes the pictures it wants (`popover`, `main`, `toast`) as they show. */
  sessionStep(id: string, captures: string[]): Promise<void>;
  /** The step is done (or skipped): its facts and self-checks go to the session's folder. */
  sessionDone(id: string, skipped: boolean, facts: unknown, checks: string[]): Promise<void>;
  /** "Chụp sau 3 giây": a picture of `tray`, `main` or `popover`. */
  sessionShot(what: string, delayMs: number): Promise<void>;
  /** What a page saw itself (its fonts, a contrast theme, a reload). */
  sessionNote(kind: string, data: unknown): Promise<void>;
  /** A Windows Settings page, or Codex's `config.toml` in Notepad. */
  sessionOpen(what: string): Promise<void>;
  /** "Ghi gói chẩn đoán": `diag-<date>.zip`, and where it went. */
  sessionZip(): Promise<{ name: string; folder: string }>;
  /** Shows the zip in File Explorer. */
  sessionReveal(): Promise<void>;
}

export const tauriApi: Api = {
  getModel: () => invoke<Model>('get_model'),
  refresh: () => invoke<number>('refresh'),
  openWindow: (tab) => invoke('open_window', { tab }),
  mainTab: () => invoke<Tab>('main_tab'),
  settingsGroup: () => invoke<SettingsGroup>('settings_group'),
  setSettingsGroup: (group) => invoke('set_settings_group', { group }),
  hidePopover: () => invoke('hide_popover'),
  resizePopover: (height) => invoke('popover_resize', { height }),
  setMachineName: (name) => invoke<Model>('set_machine_name', { name }),
  grantKeychain: () => invoke<Model>('grant_keychain'),
  claudeSessions: () => invoke<Sessions | null>('claude_sessions'),
  switchClaude: (slot, holdUntil) => invoke<Outcome>('switch_claude', { slot, holdUntil }),
  claudeImport: () => invoke<Model>('claude_import'),
  claudeAddLogin: () => invoke<Model>('claude_add_login'),
  claudeCancelLogin: () => invoke<void>('claude_cancel_login'),
  claudeAddCurrent: () => invoke<Model>('claude_add_current'),
  claudeAddToken: (token, email) => invoke<Model>('claude_add_token', { token, email }),
  claudeRemove: (slot) => invoke<Model>('claude_remove', { slot }),
  claudeSetAlias: (slot, alias) => invoke<Model>('claude_set_alias', { slot, alias }),
  claudeSetEnabled: (slot, enabled) => invoke<Model>('claude_set_enabled', { slot, enabled }),
  claudeSetRotation: (on) => invoke<Model>('claude_set_rotation', { on }),
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
  setThemePack: (pack) => invoke<Model>('set_theme_pack', { pack }),
  setLanguage: (language) => invoke<Model>('set_language', { language }),
  finishSetup: (open) => invoke<Model>('finish_setup', { open }),
  setMenubar: (menubar) => invoke<Model>('set_menubar', { menubar }),
  setMenubarWindow: (window) => invoke<Model>('set_menubar_window', { window }),
  agyAppState: () => invoke<AgyApp>('agy_app_state'),
  switchAgy: (id, how) => invoke<AgyOutcome>('switch_agy', { id, how }),
  agyDismissOverwrite: () => invoke<Model>('agy_dismiss_overwrite'),
  codexSwitch: (target, base) => invoke<CodexOutcome>('codex_switch', { target, base }),
  codexRestart: (afterSwitch) => invoke<CodexRestart>('codex_restart', { afterSwitch }),
  codexKeep: () => invoke<Model>('codex_keep'),
  codexCheck: () => invoke('codex_check'),
  setCodexHome: (path) => invoke<Model>('set_codex_home', { path }),
  codexKeyPreview: (key, baseUrl) => invoke<CodexKeyPreview>('codex_key_preview', { key, baseUrl }),
  codexSetKey: (key, baseUrl, base) => invoke<CodexKeyOutcome>('codex_set_key', { key, baseUrl, base }),
  codexUndoKey: (base) => invoke<CodexKeyOutcome>('codex_undo_key', { base }),
  setCodexPlugins: (on) => invoke<Model>('set_codex_plugins', { on }),
  codexUse: (id, base) => invoke<CodexOutcome>('codex_use', { id, base }),
  codexLogin: (id, device) => invoke<CodexAccountDone>('codex_login', { id, device: device ?? false }),
  codexCancelLogin: () => invoke('codex_cancel_login'),
  codexReopenLogin: () => invoke('codex_reopen_login'),
  codexCheckProvider: (baseUrl, key) => invoke<CodexProviderCheck>('codex_check_provider', { baseUrl, key }),
  codexProviderPreview: (name, baseUrl, key) => invoke<CodexKeyPreview>('codex_provider_preview', { name, baseUrl, key }),
  codexAddProvider: (name, baseUrl, key, base) => invoke<CodexAccountDone>('codex_add_provider', { name, baseUrl, key, base }),
  codexRename: (id, name) => invoke<CodexAccountDone>('codex_rename', { id, name }),
  codexRemove: (id, logout, base) => invoke<CodexAccountDone>('codex_remove', { id, logout, base }),
  codexImportPick: () => invoke<CodexImportPreview | null>('codex_import_pick'),
  codexImportText: (text) => invoke<CodexImportPreview>('codex_import_text', { text }),
  codexImport: (token, removeSource) => invoke<CodexAccountDone>('codex_import', { token, removeSource }),
  codexTriggerSet: (t) => invoke('codex_trigger_set', { trigger: t }),
  codexTriggerNow: () => invoke('codex_trigger_now'),
  mcpRead: () => invoke('mcp_read'),
  mcpPreview: (changes) => invoke<McpPreview>('mcp_preview', { changes }),
  mcpWrite: (changes, bases) => invoke<McpOutcome>('mcp_write', { changes, bases }),
  mcpUndo: () => invoke<McpOutcome>('mcp_undo'),
  mcpSaveSecret: (name, slot, value, bitwarden) => invoke('mcp_save_secret', { name, slot, value, bitwarden }),
  mcpPaste: (text, save, bitwarden) => invoke<McpPaste>('mcp_paste', { text, save, bitwarden }),
  mcpForget: (name, slots, keys) => invoke('mcp_forget', { name, slots, keys }),
  mcpOpenBackups: (agent) => invoke('mcp_open_backups', { agent }),
  skillsRead: () => invoke('skills_read'),
  skillsPreview: (changes) => invoke<SkillPreview>('skills_preview', { changes }),
  skillsWrite: (changes, bases) => invoke<SkillOutcome>('skills_write', { changes, bases }),
  skillsUndo: () => invoke<'undone' | 'failed'>('skills_undo'),
  skillsDropLink: (agent, name) => invoke('skills_drop_link', { agent, name }),
  skillsListRepo: (repo) => invoke<RepoList>('skills_list_repo', { repo }),
  skillsReadMd: (repo, skill) => invoke<string>('skills_read_md', { repo, skill }),
  skillsInstall: (repo, names, agents) => invoke<SkillInstalled>('skills_install', { repo, names, agents }),
  skillsUpdate: (name) => invoke<SkillUpdated>('skills_update', { name }),
  skillsUpdateMany: (names) => invoke<SkillUpdatedMany>('skills_update_many', { names }),
  skillsSkip: (name) => invoke('skills_skip', { name }),
  skillsRemove: (name) => invoke<SkillRemoved>('skills_remove', { name }),
  skillsImport: (names) => invoke<SkillGathered>('skills_import', { names }),
  skillsOpenLink: (url) => invoke('skills_open_link', { url }),
  skillsCheck: () => invoke('skills_check'),
  releaseCheck: () => invoke('release_check'),
  releaseSkip: (version) => invoke('release_skip', { version }),
  releaseLater: (version) => invoke('release_later', { version }),
  setReleaseChecks: (on) => invoke('set_release_checks', { on }),
  cliToolsCheck: () => invoke('cli_tools_check'),
  cliToolsOpened: () => invoke('cli_tools_opened'),
  cliToolUpdate: (tool) => invoke('cli_tool_update', { tool }),
  setCliChecks: (daily) => invoke('set_cli_checks', { daily }),
  releaseOpen: (what) => invoke('release_open', { what }),
  releaseDiagnostics: () => invoke<string>('release_diagnostics'),
  skillsSetChecks: (daily, notify) => invoke('skills_set_checks', { daily, notify }),
  envCheck: () => invoke('env_check'),
  envOpen: (id) => invoke<string | null>('env_open', { id }),
  envIgnore: (id) => invoke('env_ignore', { id }),
  envUnignore: (id) => invoke('env_unignore', { id }),
  setsSave: (set) => invoke<ConfigSet>('sets_save', { set }),
  setsDelete: (id) => invoke('sets_delete', { id }),
  setsUndelete: () => invoke('sets_undelete'),
  setsPreview: (id) => invoke<SetPreview>('sets_preview', { id }),
  setsApply: (id, sent) => invoke<SetOutcome>('sets_apply', { id, sent }),
  setsUndo: () => invoke<SetUndone>('sets_undo'),
  statsRead: () => invoke('stats_read'),
  statsReport: (period) => invoke<StatsReport>('stats_report', { period }),
  statsAck: () => invoke('stats_ack'),
  pluginsRead: () => invoke('plugins_read'),
  instrRead: () => invoke('instr_read'),
  instrShow: (path) => invoke<InstrShown>('instr_show', { path }),
  instrCompare: (path) => invoke('instr_compare', { path }),
  instrInclude: (path, on) => invoke('instr_include', { path, on }),
  instrNotKey: (path, line) => invoke('instr_not_key', { path, line }),
  instrAdd: (path) => invoke<string>('instr_add', { path }),
  instrForget: (path) => invoke('instr_forget', { path }),
  instrOpen: (path) => invoke('instr_open', { path }),
  backupRead: () => invoke('backup_read'),
  backupNow: () => invoke<string>('backup_now'),
  backupSetAuto: (afterChange, daily) => invoke('backup_set_auto', { afterChange, daily }),
  backupSetFolder: (path) => invoke('backup_set_folder', { path }),
  backupOpenFolder: () => invoke('backup_open_folder'),
  backupDeleteKey: (mcp, slot) => invoke('backup_delete_key', { mcp, slot }),
  packSummary: (id) => invoke<PackSummary>('pack_summary', { id }),
  restorePlan: (id) => invoke<RestorePlan>('restore_plan', { id }),
  restoreRun: (id, picks, more, keys, bases) => invoke<RestoreDone>('restore_run', { id, picks, more, keys, bases }),
  restoreRetry: (id, part, item, agent) => invoke<RestorePartResult>('restore_retry', { id, part, item, agent }),
  restoreUndo: (force) => invoke<RestoreUndone>('restore_undo', { force }),
  copyCompare: (alias) => invoke<CopyComparison>('copy_compare', { alias }),
  copyRun: (alias, push, selection, keys) => invoke<RestoreDone>('copy_run', { alias, push, selection, keys }),
  copyUndo: (alias, push, at, force) => invoke<RestoreUndone>('copy_undo', { alias, push, at, force }),
  alertAction: (id, action) => invoke('alert_action', { id, action }),
  alertsRead: () => invoke('alerts_read'),
  testAlert: (kind) => invoke<Model>('test_alert', { kind }),
  setAlertSwitch: (kind, on) => invoke<Model>('set_alert_switch', { kind, on }),
  setPoolAlert: (dryAtMost, roomAtLeast) => invoke<Model>('set_pool_alert', { dryAtMost, roomAtLeast }),
  setAdvanced: (on) => invoke<Model>('set_advanced', { on }),
  setFirstmate: (on) => invoke<Model>('set_firstmate', { on }),
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
  reduceTransparency: () => invoke<boolean>('reduce_transparency'),
  onReduceTransparency: (handler) => listen<boolean>('reduceTransparency', (event) => handler(event.payload)),
  increaseContrast: () => invoke<boolean>('increase_contrast'),
  onIncreaseContrast: (handler) => listen<boolean>('increaseContrast', (event) => handler(event.payload)),
  onTab: (handler) => listen<Tab>('tab', (event) => handler(event.payload)),
  sessionGet: () => invoke<SessionView>('session_get'),
  sessionStep: (id, captures) => invoke('session_step', { id, captures }),
  sessionDone: (id, skipped, facts, checks) => invoke('session_done', { id, skipped, facts, checks }),
  sessionShot: (what, delayMs) => invoke('session_shot', { what, delayMs }),
  sessionNote: (kind, data) => invoke('session_note', { kind, data }),
  sessionOpen: (what) => invoke('session_open', { what }),
  sessionZip: () => invoke<{ name: string; folder: string }>('session_zip'),
  sessionReveal: () => invoke('session_reveal'),
};
