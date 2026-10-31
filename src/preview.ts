// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Browser preview for `npm run dev`: the popover or the main window on a
// fixture, without Tauri. `?state=keychain&view=window&tab=claude`.
// Switching (M3): `outcome=raced|failed|busy|unconfirmed` is what a switch reports, and
// `accounts=none` shows a Mac where Quota Station keeps no Claude account yet. A switch takes 1.5 s, so the working sheet shows.
// @private:begin
// `switcher=held|noguard|auto|stopped|missing` sets the switcher.
// @private:end
// Claude accounts: `claude=relogin` has account 3 to sign in again, `claude=renew` makes a sign-in land on
// account 1 (already here), `claude=fail` ends a sign-in without one; an action takes 3 s.
// GitHub Releases: `release=avail|later|skipped|offline|none|off` (up to date by default); "Kiểm tra bản mới"
// takes 1.5 s and finds 0.3.0, or `releasefinds=offline`. `build=release` is the downloaded app
// (no Antigravity accounts), whose dialog offers the DMG.
// The first run: `setup=on` (the fixture's Mac, its accounts not kept yet: the one signed in is
// taken at the Claude step), `setup=signedout` (Claude Code signed in to none), `setup=none` (no agent).
// @private:begin
// Rotation rules (M5): `tab=rules` with `rotation=unreachable|newer|edited|unstamped|missing|invalid|unwritable|single`;
// a save takes 1 s, and `unreachable` leaves Mac C behind.
// @private:end
// Antigravity logins (M4a) and reads (M4b): `agy=two|relogin|verify|noclient|tooopen|invalid|broken|reading|failing|forbidden`
// sets the login card, `login=timeout|noRefreshToken|keychain|…` how a login ends
// (it keeps the next account after 3 s by default, or at once on any pasted
// URL with `code=` and `state=`), and `email=on` shows the made-up emails.
// Switching Antigravity (M4c): `agyswitch=on|overwritten` lists the two
// accounts as the app's (the second one undone by Antigravity.app with
// `overwritten`), `agyapp=closed|missing|old|unreadable|unknown` is what the sheet learns
// about Antigravity.app (it runs by default), and
// `agyoutcome=unconfirmed|notquit|write` what a switch reports. A switch takes 1.5 s.
// History and notifications (M6): `tab=history`, and
// `tab=alerts`, Tổng quan with the bell's panel open (as a macOS card opens
// it), with `history=empty` and `alerts=on|off|unbundled|empty|many` (two cards
// by default on `tab=alerts`; `off` is macOS refusing the cards; `many`,
// twelve not read yet, `9+` on the bell).
// The app itself (M7), on `tab=settings`: `app=on|approval|unavailable` sets the login switch.
// @private:begin
// Firstmate (M6): `tab=firstmate` and `firstmate=ok|holding|stale|missing|unset` (unset by default).
// `app=nosource|unbundled` sets the clone, and `update=uptodate|pull|install|timeout` how
// "Cập nhật từ mã nguồn" ends (by default it installs 0.1.1). Each step takes 1.5 s.
// @private:end
// Codex (M8): `codex=chatgpt|ccswitch|down|external|nologin|nokey|placeholder|header|envkey|noplugins|broken|none`
// sets the six states of the design and more (Codex goes through the made-up `relay` by
// default), and `codexoutcome=changed|refused` what a switch reports. A switch or a
// restart takes 1.5 s; `tab=codex` opens the Codex tab.
// Codex accounts: four made-up ChatGPT sign-ins and three custom providers by default;
// `codexaccounts=one` is one of each (`chatgpts=3` then lists three ChatGPT accounts),
// `codexaccounts=empty` neither. A sign-in waits 3 s and adds an account, or ends with
// `codexlogin=dup|timeout|port|noCodex|denied|other`; "Kiểm tra" on a new provider finds
// it answering with the key accepted, or `codexprovider=down|rejected` (also an address
// with `down` in it, a key starting `sk-bad`). "Nhập file đăng nhập (JSON)" finds five made-up
// files, or none with `codeximport=none`, and any JSON pasted reads as two made-up sign-ins;
// `codextrigger=on|off` shows "Tự đánh thức tài khoản ChatGPT" set up (on, or off) instead of
// as it starts, off with no account chosen.
// The menu bar's service (19/10/2026): `menubar=claude|gemini|codex` on `tab=settings`,
// `mbwindow=fiveHours|week|fableWeek|tightest` the window its title shows,
// and `agyhot=on` puts the Antigravity account in use at 86 % of its Claude/GPT week,
// which Đang dùng then shows beside Gemini.
// MCP (M9): `tab=mcp` with `mcp=empty|parse|noagy|nocli|unread`, and `mcpoutcome=changed`
// for a write that finds a file changed. A preview or a write takes 0.6 s.
// The services' logos: `mcp=brands` and `plugins=brands` name the made-up servers, connectors
// and plugins after real services.
// Sao lưu (M10): `tab=backup` with `backup=empty|icloud|noicloud|restored|unread`
// (`icloud`: a pack iCloud Drive has not downloaded), and `restoreoutcome=changed|undochanged`
// for a restore, or its undo, that finds a file changed. A restore takes 1.2 s.
// M12: `restoreoutcome=partial` for a restore whose formatter plugin needs a login.
// Skills, Plugin and File hướng dẫn (M11): `tab=skills|plugins|instr`, with
// `skills=empty|nonpx|offline|unread|sets`, `plugins=noclaude` and `instr=clean`
// (`sets`: acme/agent-skills has two newer versions too, for "Cập nhật cả bộ");
// `skillsoutcome=changed` for a write that finds a folder changed, `skillsoutcome=updatefail`
// for an update of release-notes that fails. npx and a write take 0.8 s.
// @private:begin
// Chép sang Mac khác (M13): `tab=copy` with the made-up Macs "MacBook Air" (`travel-air`) and
// "Studio iMac" (`studio-imac`), and `copy=unreach|old|noapp|none|changed`: the first Mac does
// not answer, runs version 1, has no app; no Mac in settings; a copy finds a file changed.
// A comparison, a copy and its undo take 0.9 s.
// @private:end
// Variables that override a login (M14): `env=found|claude2|launchd|unread|unchecked` (none set by
// default), shown on `tab=settings` (the card), `tab=claude` (the banner) and with `alerts=env`.
// Bộ cấu hình (M15): `tab=sets` with three made-up sets, Web app applied and drifted by one
// entry; `sets=empty|fresh|even` for none, none applied, or none drifted, and
// `setsoutcome=changed` for an apply that finds Codex's file changed. A preview, an apply and
// an undo take 0.5 s.
// Thống kê (M16): `tab=stats` with made-up counts for made-up projects; `stats=first` shows the
// first read halfway, `stats=new` starts one that runs (a step every 0.4 s), `stats=skip` has
// 38 Codex sessions in a format the app cannot read, `stats=fail` a report that cannot be read.
// Cập nhật Agent (`tab=cliTools`): `cli=running|missing|npm|bundled|unwritable|unknown|offline|never|updated|updating|latest|failed|checking`,
// several joined with commas; `clifail=offline|permission|busy|exit|timeout|unchanged|started|sessions`
// is how every "Cập nhật" ends (it lands on the newest by default), and `cliclosed=1` has "Kiểm tra lại" find the sessions closed.
// A check takes 1 s and an update 2 s.
// The glass: `rt=1` is macOS with Reduce Transparency on (the solid ground).
// Windows: `os=win` is the app on Windows: its type, corners and scroll bars, no traffic
// lights, and Windows' words wherever the text names a part of the system.
// `lang=vi|en` sets the language; without it the preview follows the browser's, as the app follows macOS.

import type { Api } from './lib/api';
import { THEME_PACKS, type AgyAccounts, type AgyApp, type AgyHow, type AgyOutcome, type CliFailure, type CodexOutcome, type CodexTarget, type McpTable, type Model, type Tab, type AlertRecord } from './lib/model';
import { fakeApi, fakeEmail, hideNames, type FakeOptions } from './fixtures/fakeApi';
import { backupFixture, packsFixture } from './fixtures/fakeBackup';
import { API_KEY, BASE_URL, CLAUDE_BASE, envAlert, envFixture, LAUNCHD_GEMINI } from './fixtures/fakeEnv';
import { setsFixture } from './fixtures/fakeSets';
import { CLI_STATES, cliToolsFixture } from './fixtures/fakeCliTools';
import { firstReadFixture, SKIPPED_CODEX, statsFixture } from './fixtures/fakeStats';
import { instrFixture, pluginsBrandsFixture, pluginsFixture, skillsFixture, skillTableFixture } from './fixtures/fakeSkills';
import {
  agyAccount,
  agyFixture,
  FIXTURE_NOW,
  fixtureModel,
  SNAPSHOTS,
  claudeAccountsFixture,
  releaseAvailable,
  releaseFixture,
  alertsFixture,
  appFixture,
  codexFixture,
  codexEmpty,
  codexMany,
  codexTriggerFixture,
  mcpBrandsFixture,
  mcpFixture,
  mcpTableFixture,
  historyFixture,
  overwriteAlert,
} from './fixtures/models';
// @private:begin
import type { UpdateError, UpdateOutcome } from './lib/model';
import { communityModel, firstmateFixture, heldSwitcher, leavingAlert, peerFixture, rotationFixture, rulesFixture, switcherFixture } from './fixtures/models';
// @private:end

const TABS: Tab[] = [
  'overview',
  'stats',
  'claude',
  'agy',
  'codex',
  'history',
  'alerts',
  'mcp',
  'skills',
  'plugins',
  'instr',
  'sets',
  'cliTools',
  'backup',
  'settings',
  // @private:begin
  'rules',
  'firstmate',
  'copy',
  // @private:end
];

const SKILLS: Record<string, () => Model['skills']> = {
  empty: () => skillsFixture({ table: skillTableFixture({ rows: [], owned: [], broken: [] }) }),
  nonpx: () => skillsFixture({ npx: false }),
  offline: () => skillsFixture({ updates: { ...skillsFixture().updates, error: 'offline' } }),
  unread: () => skillsFixture({ table: null, readAt: null }),
  sets: () => {
    const t = skillTableFixture();
    const update = { fromAt: '2025-11-20T08:00:00Z', seenAt: 1768537000, plain: 'Sửa hướng dẫn', link: 'https://github.com/acme/agent-skills/commits/HEAD' };
    return skillsFixture({ table: { ...t, rows: t.rows.map((r) => (r.name === 'pdf-tools' || r.name === 'release-notes' ? { ...r, update } : r)) } });
  },
};
const PLUGINS: Record<string, () => Model['plugins']> = {
  brands: pluginsBrandsFixture,
  noclaude: () => {
    const p = pluginsFixture();
    return { ...p, sides: p.sides!.map((s) => (s.agent === 'claude' ? { ...s, state: 'noCli', plugins: [], marketplaces: [] } : s)), reinstall: { codex: p.reinstall.codex } };
  },
};
const INSTRS: Record<string, () => Model['instructions']> = {
  clean: () => {
    const i = instrFixture();
    return { ...i, view: { ...i.view!, files: i.view!.files.filter((f) => !f.keyLines.length) } };
  },
};

const SETS: Record<string, () => Model['sets']> = {
  empty: () => setsFixture({ sets: [], applied: null }),
  fresh: () => setsFixture({ applied: null }),
  even: () => setsFixture({ applied: { ...setsFixture().applied!, drift: [] } }),
};

const ENVS: Record<string, () => Model['env']> = {
  found: () => envFixture({ found: [API_KEY, BASE_URL] }),
  claude2: () => envFixture({ found: [API_KEY, CLAUDE_BASE, BASE_URL] }),
  launchd: () => envFixture({ found: [API_KEY, BASE_URL, LAUNCHD_GEMINI] }),
  unread: () => envFixture({ found: [API_KEY], unread: ['zprofile'], ignored: 1 }),
  unchecked: () => envFixture({ checkedAt: null }),
};

const MCP_COLUMN = (agent: 'claude' | 'codex' | 'agy', over: Partial<McpTable['columns'][number]>) => (t: McpTable) =>
  t.columns.map((c) => (c.agent === agent ? { ...c, ...over } : c));
const MCP: Record<string, () => Model['mcp']> = {
  empty: () => mcpFixture({ table: mcpTableFixture({ rows: [], stored: [] }), backups: [] }),
  parse: () => {
    const t = mcpTableFixture();
    return mcpFixture({ table: { ...t, columns: MCP_COLUMN('codex', { state: 'invalid', reason: 'TOML parse error at line 48, column 1', writable: false })(t) } });
  },
  noagy: () => {
    const t = mcpTableFixture();
    return mcpFixture({ table: { ...t, columns: MCP_COLUMN('agy', { state: 'absent', present: false, writable: false })(t) } });
  },
  nocli: () => {
    const t = mcpTableFixture();
    return mcpFixture({ table: { ...t, columns: MCP_COLUMN('claude', { state: 'noCli', writable: false })(t) } });
  },
  unread: () => mcpFixture({ table: null, readAt: null }),
  brands: () => mcpFixture({ table: mcpBrandsFixture() }),
};

const BACKUP: Record<string, () => Model['backup']> = {
  empty: () => backupFixture({ packs: [], keys: [] }),
  icloud: () => backupFixture({ packs: packsFixture() }),
  noicloud: () => backupFixture({ icloud: false, packs: [] }),
  restored: () => backupFixture({ restore: { at: 1768547700, pack: 'agents-20260116-141000-studio-mini', mac: 'Mac Studio' } }),
  unread: () => backupFixture({ packs: null, keys: null, readAt: null }),
};

/** How a ChatGPT sign-in for Codex ends (`codexlogin=`); by default it adds an account after 3 s. */
const CODEX_LOGIN_ENDS = ['dup', 'timeout', 'port', 'noCodex', 'denied', 'other'] as const;

const RELAY = codexFixture().custom!;
const CODEX: Record<string, () => Model['codex']> = {
  chatgpt: () => codexFixture({ current: 'chatgpt' }),
  ccswitch: () =>
    codexFixture({
      ccSwitch: { installed: true, running: true, data: true, traces: ['model_catalog_json = "cc-switch-model-catalog.json"'] },
      procs: { sessions: 3, app: true, daemon: true, ccSwitch: true },
    }),
  down: () => codexFixture({ reach: { state: 'down', ms: null, since: '2026-01-16T07:12:00Z', checkedAt: '2026-01-16T07:20:00Z' } }),
  external: () =>
    codexFixture({
      current: 'chatgpt',
      external: { at: '2026-01-16T07:02:00Z', lastAt: '2026-01-16T06:40:00Z', lastProvider: 'custom', now: 'openai', mcpChanged: true, mcpCount: 4 },
      lastWrite: { at: '2026-01-16T06:40:00Z', provider: 'custom', matches: false },
    }),
  nologin: () => codexFixture({ current: 'chatgpt', login: 'missing', wantsRequires: false }),
  nokey: () => codexFixture({ current: 'chatgpt', custom: { ...RELAY, key: 'none' }, wantsRequires: null, refusal: { kind: 'noKey' } }),
  placeholder: () =>
    codexFixture({
      current: 'chatgpt',
      custom: { ...RELAY, name: 'custom', baseUrl: 'http://127.0.0.1:15721/v1', key: 'placeholder', requiresOpenaiAuth: null, proxy: true },
      wantsRequires: null,
      refusal: { kind: 'placeholder' },
      ccSwitch: { installed: true, running: false, data: true, traces: ['model_catalog_json = "cc-switch-model-catalog.json"', '[model_providers.custom] experimental_bearer_token = "PROXY_MANAGED"'] },
    }),
  header: () => codexFixture({ current: 'chatgpt', custom: { ...RELAY, key: 'headers' }, wantsRequires: false, takesKey: false }),
  envkey: () => codexFixture({ current: 'chatgpt', custom: { ...RELAY, key: 'envKey', envKey: 'RELAY_API_KEY' }, takesKey: false }),
  noplugins: () => codexFixture({ plugins: false, wantsRequires: false, custom: { ...RELAY, requiresOpenaiAuth: false } }),
  broken: () => codexFixture({ config: 'invalid', reason: 'TOML parse error at line 3', refusal: { kind: 'noCustom' }, custom: null, mcp: [] }),
  none: () => codexFixture({ present: false }),
};

const CODEX_OUTCOMES: Record<string, (target: CodexTarget) => CodexOutcome> = {
  changed: (target) => ({ kind: 'failed', target, error: { kind: 'changed' } }),
  refused: (target) => ({ kind: 'refused', target, refusal: { kind: 'noKey' } }),
};

/** The bell's other card beside the overwritten Antigravity switch: a variable over a login. */
let otherAlert = (): AlertRecord => envAlert(API_KEY);
// @private:begin
// Team A leaving the rotation, with the author's tools.
otherAlert = () => leavingAlert();
// @private:end

const ALERTS: Record<string, () => Model['alerts']> = {
  on: () => alertsFixture({ recent: [overwriteAlert(), otherAlert()], unanswered: 2 }),
  off: () => alertsFixture({ recent: [overwriteAlert(), otherAlert()], unanswered: 2, permission: 'denied' }),
  unbundled: () => alertsFixture({ recent: [overwriteAlert(), otherAlert()], unanswered: 2, permission: 'unbundled' }),
  empty: () => alertsFixture(),
  many: () =>
    alertsFixture({
      recent: Array.from({ length: 12 }, (_, i) => {
        const at = 1768547700 - i * 2400;
        const base = i % 2 ? overwriteAlert() : otherAlert();
        let title = base.alert.title;
        // @private:begin
        if (!(i % 2)) title = 'Team A rời vòng lúc ' + new Date((at + 7 * 3600) * 1000).toISOString().slice(11, 16);
        // @private:end
        return { ...base, id: base.alert.key + ':' + i + '@' + at, alert: { ...base.alert, key: base.alert.key + ':' + i, title, at } };
      }),
      unanswered: 12,
    }),
  env: () => alertsFixture({ recent: [envAlert(API_KEY), envAlert(BASE_URL, 1768547640)], unanswered: 2 }),
};

// @private:begin
const FIRSTMATES: Record<string, () => Model['firstmate']> = {
  ok: () => firstmateFixture(),
  holding: () =>
    firstmateFixture({ line: { ...firstmateFixture().line!, dry: true, nextReset: '2026-01-16T09:40:00.000Z' }, ownDry: true }),
  stale: () => firstmateFixture({ writtenAt: '2026-01-16T05:20:00.000Z', fresh: false }),
  missing: () => firstmateFixture({ status: 'missing', line: null, writtenAt: null, fresh: false }),
  unset: () => fixtureModel('fresh').firstmate,
};
// @private:end

const APPS: Record<string, () => Model['app']> = {
  on: () => appFixture({ launchAtLogin: 'on' }),
  approval: () => appFixture({ launchAtLogin: 'needsApproval' }),
  unavailable: () =>
    appFixture({
      launchAtLogin: 'unavailable',
      commit: null,
      // @private:begin
      bundled: false,
      // @private:end
    }),
  // @private:begin
  nosource: () => appFixture({ source: null, sourceStatus: 'notSet' }),
  unbundled: () => appFixture({ bundled: false, launchAtLogin: 'unavailable', commit: null }),
  // @private:end
};

const CLI_FAILS: CliFailure[] = ['offline', 'permission', 'busy', 'exit', 'timeout', 'unchanged', 'started', 'sessions'];
/** What each failure says, as the tool would. */
const CLI_FAIL_DETAIL: Record<string, string> = {
  permission: '/opt/homebrew/Caskroom/codex',
  started: '1',
  exit: 'Error: Download failed on Cask \'codex\' with message: SHA256 mismatch (mã 1)',
};

// @private:begin
const UPDATES: Record<string, () => UpdateOutcome | UpdateError> = {
  uptodate: () => ({ kind: 'upToDate', commit: 'a1b2c3d4e5f60718293a4b5c6d7e8f9012345678' }),
  pull: () => ({ kind: 'pull', detail: 'fatal: Not possible to fast-forward, aborting.' }),
  install: () => ({ kind: 'install', detail: 'cargo tauri build failed' }),
  timeout: () => ({ kind: 'timeout', step: 'installing' }),
};
// @private:end

// @private:begin
const RETRY = '2026-01-16T07:21:40.000Z';
const NEWER = { at: '2026-01-16T14:11:00+07:00', machine: 'Mac B' };
// @private:end

// @private:begin
const ROTATIONS: Record<string, () => Model['rotation']> = {
  unreachable: () =>
    rotationFixture({
      stamp: { at: '2026-01-16T14:16:00+07:00', machine: 'Mac Studio' },
      peers: [
        peerFixture('Mac B', 'mac-b', { state: { kind: 'synced', at: '2026-01-16T07:16:04.000Z' } }),
        peerFixture('Mac C', 'mac-c', { state: { kind: 'unreachable', error: 'ssh: connect to host mac-c port 22: Operation timed out', retryAt: RETRY } }),
      ],
    }),
  unstamped: () =>
    rotationFixture({
      peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'edited', their: null }, theirRules: null })],
    }),
  newer: () =>
    rotationFixture({
      peers: [peerFixture('Mac B', 'mac-b', { state: { kind: 'newer', their: NEWER }, theirRules: rulesFixture({ full: 88, personalTo: 21 }) }), peerFixture('Mac C', 'mac-c')],
    }),
  edited: () =>
    rotationFixture({
      peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'edited', their: { at: '2026-01-16T14:02:00+07:00', machine: 'Mac Studio' } }, theirRules: rulesFixture({ team: [3, 4] }) })],
    }),
  missing: () => rotationFixture({ status: 'missing', rules: null, version: null, stamp: null, peers: [peerFixture('Mac B', 'mac-b', { state: null, current: false }), peerFixture('Mac C', 'mac-c', { state: null, current: false })] }),
  invalid: () => rotationFixture({ status: 'invalid', rules: null, reason: 'rotation_bad_value:personal_hours', stamp: null }),
  single: () => rotationFixture({ peers: [] }),
};
// @private:end

// @private:begin
const SWITCHERS: Record<string, () => Model['switcher']> = {
  held: () => heldSwitcher(),
  noguard: () => heldSwitcher({ dryRun: { decision: { kind: 'stay' }, raw: 'stay', at: '2026-01-16T07:19:40.000Z' }, guard: false }),
  auto: () => switcherFixture({ legacy: 'com.cswap.prefer-team' }),
  stopped: () => switcherFixture({ service: false }),
  missing: () => switcherFixture({ installed: false, dryRun: null, lastLog: null, service: null }),
};
// @private:end

const AGY: Record<string, () => AgyAccounts> = {
  two: () => agyFixture({ accounts: [agyAccount(1), agyAccount(2)] }),
  relogin: () => agyFixture({ accounts: [agyAccount(1), agyAccount(2, { status: 'needsLogin' })] }),
  verify: () => agyFixture({ accounts: [agyAccount(1, { status: 'needsVerification', verify: true }), agyAccount(2)] }),
  noclient: () => agyFixture({ client: 'missing' }),
  tooopen: () => agyFixture({ client: 'tooOpen' }),
  invalid: () => agyFixture({ client: 'invalid' }),
  broken: () => agyFixture({ indexError: true }),
  reading: () => agyFixture({ accounts: [agyAccount(1, { reading: true }), agyAccount(2)] }),
  failing: () =>
    agyFixture({
      accounts: [
        agyAccount(1, { readError: { kind: 'network', detail: '', retryAt: null } }),
        agyAccount(2, { readError: { kind: 'rateLimited', detail: '', retryAt: '2026-01-16T07:24:00.000Z' } }),
      ],
    }),
  forbidden: () => agyFixture({ accounts: [agyAccount(1), agyAccount(2, { status: 'forbidden', verify: true })] }),
};

const LOGIN_ENDS = ['timeout', 'cancelled', 'noCode', 'stateMismatch', 'noRefreshToken', 'invalidGrant', 'network', 'keychain', 'index', 'browser'];

// @private:begin
/** The hold a switch writes: to the minute, as the pause flag keeps it. */
const holdOf = (until: string | null) =>
  until ? { until: new Date(Math.floor(Date.parse(until) / 60000) * 60000).toISOString(), failed: false } : null;
// @private:end

const OUTCOMES: Record<string, NonNullable<FakeOptions['outcome']>> = {
  // Another switch puts back an account other than the one asked for.
  raced: (
    slot,
    // @private:begin
    holdUntil,
    // @private:end
  ) => ({
    kind: 'raced',
    slot,
    active: slot === 2 ? 3 : 2,
    // @private:begin
    bySwitcherAt: '2026-01-16T07:21:00.000Z',
    hold: holdOf(holdUntil),
    // @private:end
  }),
  failed: (slot) => ({ kind: 'failed', slot, error: 'noCredential', detail: '', active: null }),
  // Claude Code held its login locks, so nothing was written.
  busy: (slot) => ({ kind: 'failed', slot, error: 'busy', detail: '', active: null }),
  unconfirmed: (
    slot,
    // @private:begin
    holdUntil,
    // @private:end
  ) => ({
    kind: 'unconfirmed',
    slot,
    // @private:begin
    hold: holdOf(holdUntil),
    // @private:end
  }),
};

const AGY_APPS: Record<string, AgyApp | null> = {
  closed: { installed: 'supported', version: '2.24.0', running: false },
  missing: { installed: 'missing', version: null, running: false },
  old: { installed: 'tooOld', version: '1.20.5', running: true },
  unreadable: { installed: 'unreadable', version: null, running: true },
  unknown: null,
};

const AGY_OUTCOMES: Record<string, (number: number, how: AgyHow) => AgyOutcome> = {
  // agy still reads the account from before.
  unconfirmed: (number, how) => ({ kind: 'unconfirmed', number, how, active: number === 1 ? 2 : 1, reopened: how === 'quitFirst' ? true : null, saved: true }),
  notquit: (number) => ({ kind: 'failed', number, error: 'notQuit', detail: '', reopened: null }),
  write: (number, how) => ({
    kind: 'failed',
    number,
    error: 'write',
    detail: 'security exited 36: User interaction is not allowed.',
    reopened: how === 'quitFirst' ? true : null,
  }),
};

/** A table entry by query value, never one of Object.prototype's keys. */
function pick<T>(table: Record<string, T>, key: string | null): T | undefined {
  return key !== null && Object.hasOwn(table, key) ? table[key] : undefined;
}

export function previewSetup(params: URLSearchParams): { api: Api; view: 'popover' | 'window'; clock: () => Date } {
  const state = params.get('state') ?? 'fresh';
  const tab = TABS.find((t) => t === params.get('tab')) ?? 'overview';
  const view = params.get('view') === 'window' ? 'window' : 'popover';
  const over: Partial<Model> = {};
  // @private:begin
  const switcher = pick(SWITCHERS, params.get('switcher'));
  if (switcher) over.switcher = switcher();
  // @private:end
  if (params.get('accounts') === 'none') {
    over.claudeSwitch = { ready: false, busy: null };
    over.claudeAccounts = claudeAccountsFixture({ ready: false, accounts: [] });
    // @private:begin
    over.claudeAccounts.rotation = false;
    over.switcher = switcherFixture({ installed: false, service: false, dryRun: null, dryRunError: 'noAccounts' });
    // @private:end
  }
  const rel = params.get('release');
  if (rel === 'avail' || rel === 'later' || rel === 'skipped') over.release = releaseAvailable(rel === 'later' ? { later: '0.3.0' } : rel === 'skipped' ? { skipped: '0.3.0' } : {});
  else if (rel === 'offline') over.release = releaseFixture({ error: 'offline' });
  else if (rel === 'none') over.release = releaseFixture({ none: true, latest: null });
  else if (rel === 'off') over.release = releaseFixture({ auto: false });
  let cli = cliToolsFixture();
  for (const state of (params.get('cli') ?? '').split(',')) if (Object.hasOwn(CLI_STATES, state)) cli = CLI_STATES[state](cli);
  over.cliTools = cli;
  const claude = params.get('claude');
  if (claude === 'relogin')
    over.claudeAccounts = claudeAccountsFixture({
      accounts: claudeAccountsFixture().accounts.map((a) => (a.slot === 3 ? { ...a, status: 'reloginRequired' as const } : a)),
    });
  // @private:begin
  const rotation = params.get('rotation');
  const rules = pick(ROTATIONS, rotation);
  if (rules) over.rotation = rules();
  // @private:end
  const agy = pick(AGY, params.get('agy'));
  if (agy) over.antigravity = agy();
  const agySwitch = params.get('agyswitch');
  if (agySwitch === 'on' || agySwitch === 'overwritten') {
    if (!over.antigravity?.accounts.length) over.antigravity = AGY.two();
    over.agySwitch = {
      busy: null,
      listed: true,
      overwritten: agySwitch === 'overwritten' ? { to: 1, from: 2, at: '2026-01-16T07:10:00.000Z', overwrites: 1 } : null,
    };
  }
  // Names show unless `email=off`, as in the app.
  if (params.get('email') !== 'off') {
    if (over.antigravity) over.antigravity.accounts = over.antigravity.accounts.map((a) => ({ ...a, email: fakeEmail(a.number) }));
  }
  const theme = params.get('theme');
  if (theme === 'light' || theme === 'dark') over.theme = theme;
  const pack = THEME_PACKS.find((p) => p === params.get('pack'));
  if (pack) over.themePack = pack;
  const lang = params.get('lang');
  if (lang === 'vi' || lang === 'en') over.language = lang;
  const alerts = pick(ALERTS, params.get('alerts') ?? (tab === 'alerts' ? 'on' : null));
  if (alerts) over.alerts = alerts();
  // @private:begin
  const firstmate = pick(FIRSTMATES, params.get('firstmate'));
  if (firstmate) over.firstmate = firstmate();
  // @private:end
  const app = pick(APPS, params.get('app'));
  if (app) over.app = app();
  const codex = pick(CODEX, params.get('codex'));
  if (codex) over.codex = codex();
  const mcp = pick(MCP, params.get('mcp'));
  if (mcp) over.mcp = mcp();
  const backup = pick(BACKUP, params.get('backup'));
  if (backup) over.backup = backup();
  const skills = pick(SKILLS, params.get('skills'));
  if (skills) over.skills = skills();
  const plugins = pick(PLUGINS, params.get('plugins'));
  if (plugins) over.plugins = plugins();
  const env = pick(ENVS, params.get('env') ?? (params.get('alerts') === 'env' ? 'found' : null));
  if (env) over.env = env();
  const sets = pick(SETS, params.get('sets'));
  if (sets) over.sets = sets();
  const instr = pick(INSTRS, params.get('instr'));
  if (instr) over.instructions = instr();
  const stats = params.get('stats');
  if (stats === 'first') over.stats = firstReadFixture();
  if (stats === 'new') over.stats = statsFixture({ readAt: null });
  const agyApp = params.get('agyapp');
  const ends = LOGIN_ENDS.find((k) => k.toLowerCase() === params.get('login')?.toLowerCase());
  const menubar = params.get('menubar');
  if (menubar === 'claude' || menubar === 'gemini' || menubar === 'codex') over.menubar = menubar;
  const mbwindow = params.get('mbwindow');
  if (mbwindow === 'fiveHours' || mbwindow === 'week' || mbwindow === 'fableWeek' || mbwindow === 'tightest') over.menubarWindow = mbwindow;
  // @private:begin
  const copy = params.get('copy');
  if (tab === 'copy' || copy) {
    const peers = copy === 'none' ? [] : [peerFixture('MacBook Air', 'travel-air'), peerFixture('Studio iMac', 'studio-imac')];
    over.rotation = rotationFixture({ peers });
  }
  // @private:end
  const setup = params.get('setup');
  if (setup) over.claudeAccounts = claudeAccountsFixture({ accounts: [], ready: false });
  const model = fixtureModel(SNAPSHOTS[state] ? state : 'fresh', over);
  if (setup) {
    model.firstRun = true;
    if (setup === 'signedout' || setup === 'none') for (const a of model.snapshot.claude.accounts) a.active = false;
    if (setup === 'none') {
      model.snapshot.claude = { ...model.snapshot.claude, status: 'notSetUp', accounts: [] };
      model.snapshot.agy = { ...model.snapshot.agy, status: 'notSetUp', accounts: [] };
      model.codex = { ...model.codex, present: false };
    }
  }
  if (params.get('agyhot') === 'on') {
    for (const a of model.snapshot.agy.accounts.filter((x) => x.active)) {
      a.windows = a.windows.map((w) => (w.kind === 'claudeGptWeek' ? { ...w, percent: 14 } : w));
    }
  }
  // Several Codex accounts by default (four ChatGPT sign-ins, three custom providers);
  // `codexaccounts=one` is one of each, as before, `empty` a Codex folder with neither.
  const codexAccounts = params.get('codexaccounts');
  const withCodex = codexAccounts === 'one' || params.get('codex') === 'none' ? model : codexAccounts === 'empty' ? codexEmpty(model) : codexMany(model);
  Object.assign(model, withCodex);
  const trigger = params.get('codextrigger');
  if (trigger === 'on' || trigger === 'off') model.codex = { ...model.codex, trigger: codexTriggerFixture(trigger === 'on') };
  // `chatgpts=3` adds made-up accounts to the one ChatGPT sign-in, to show the popover's list of them.
  const chatgpts = codexAccounts === 'one' ? Math.min(Number(params.get('chatgpts')) || 0, 5) : 0;
  const codexSnap = model.snapshot.codex;
  if (chatgpts > 1 && codexSnap.accounts.length) {
    const first = codexSnap.accounts[0];
    const made = [
      [62, 78, '2026-01-16T08:40:00.000Z', '2026-01-18T10:20:00.000Z'],
      [4, 12, '2026-01-16T11:55:00.000Z', '2026-01-22T08:20:00.000Z'],
      [30, 45, '2026-01-16T10:00:00.000Z', '2026-01-19T09:00:00.000Z'],
      [91, 66, '2026-01-16T07:50:00.000Z', '2026-01-17T07:00:00.000Z'],
    ] as const;
    const extra = made.slice(0, chatgpts - 1).map(([five, week, fiveAt, weekAt], i) => {
      const lines = [
        { kind: 'session' as const, percent: five, resetsAt: fiveAt, pending: false, atLeast: false },
        { kind: 'week' as const, percent: week, resetsAt: weekAt, pending: false, atLeast: false },
      ];
      return { ...first, number: i + 2, active: false, email: 'lan.chatgpt' + (i + 2) + '@example.com', lines, windows: lines };
    });
    model.snapshot.codex = { ...codexSnap, accounts: [{ ...first, number: 1, email: 'lan.chatgpt1@example.com' }, ...extra] };
  }
  let release: Model = params.get('build') === 'release' ? { ...model, capabilities: { ...model.capabilities, agyAccounts: false } } : model;
  // @private:begin
  if (release !== model) release = { ...communityModel(model), capabilities: { ...model.capabilities, agyAccounts: false, authorTools: false } };
  // @private:end
  // `os=win`: the app as Windows draws it (W5); its title bar and tray are Windows' own.
  const built: Model = params.get('os') === 'win' ? { ...release, capabilities: { ...release.capabilities, platform: 'windows' } } : release;
  const { api } = fakeApi(params.get('email') === 'off' ? hideNames(built) : built, tab, {
    switchDelayMs: 1500,
    claudeDelayMs: 3000,
    releaseDelayMs: 1500,
    cliDelayMs: 1000,
    cliClosed: params.get('cliclosed') === '1',
    cliFails: CLI_FAILS.includes(params.get('clifail') as CliFailure)
      ? Object.fromEntries((['claude', 'codex', 'agy'] as const).map((tool) => [tool, { failure: params.get('clifail') as CliFailure, detail: CLI_FAIL_DETAIL[params.get('clifail')!] }]))
      : undefined,
    releaseFinds: params.get('releasefinds') === 'offline' ? 'offline' : releaseAvailable().latest,
    claudeRenews: claude === 'renew' ? 1 : undefined,
    claudeFail: claude === 'fail' ? (action) => (action === 'login' ? 'login' : null) : undefined,
    outcome: pick(OUTCOMES, params.get('outcome')),
    clock: () => FIXTURE_NOW,
    // @private:begin
    saveDelayMs: 1000,
    unreachable: rotation === 'unreachable' ? ['mac-c'] : [],
    rulesError: rotation === 'unwritable' ? { kind: 'write', detail: 'Permission denied', restored: true } : undefined,
    firstmate: firstmateFixture(),
    update: pick(UPDATES, params.get('update')),
    updateDelayMs: 1500,
    copyUnreach: copy === 'unreach' ? ['travel-air'] : [],
    copyOld: copy === 'old' ? ['travel-air'] : [],
    copyNoApp: copy === 'noapp' ? ['travel-air'] : [],
    copyChanged: copy === 'changed',
    copyDelayMs: 900,
    // @private:end
    agyLoginDelayMs: 3000,
    agyLogin: ends ? () => ({ kind: ends, detail: '' }) : undefined,
    agyApp: agyApp !== null && Object.hasOwn(AGY_APPS, agyApp) ? AGY_APPS[agyApp] : undefined,
    agyOutcome: pick(AGY_OUTCOMES, params.get('agyoutcome')),
    agySwitchDelayMs: 1500,
    history: params.get('history') === 'empty' ? [] : historyFixture(),
    loginApproval: params.get('app') === 'approval',
    codexOutcome: pick(CODEX_OUTCOMES, params.get('codexoutcome')),
    codexDelayMs: 1500,
    codexLoginDelayMs: 3000,
    codexLoginEnd: CODEX_LOGIN_ENDS.find((k) => k === params.get('codexlogin')),
    codexProviderDown: params.get('codexprovider') === 'down',
    codexKeyRejected: params.get('codexprovider') === 'rejected',
    codexImportNone: params.get('codeximport') === 'none',
    mcpChanged: params.get('mcpoutcome') === 'changed',
    mcpDelayMs: 600,
    restoreChanged: params.get('restoreoutcome') === 'changed',
    restoreUndoChanged: params.get('restoreoutcome') === 'undochanged',
    restorePartial: params.get('restoreoutcome') === 'partial',
    restoreDelayMs: 1200,
    skillsChanged: params.get('skillsoutcome') === 'changed',
    skillsUpdateFail: params.get('skillsoutcome') === 'updatefail' ? ['release-notes'] : undefined,
    skillsDelayMs: 800,
    envDelayMs: 300,
    setsChanged: params.get('setsoutcome') === 'changed' ? 'codex' : undefined,
    setsDelayMs: 500,
    statsSkipped: stats === 'skip' ? SKIPPED_CODEX : undefined,
    statsStepMs: stats === 'new' ? 400 : undefined,
    statsFails: stats === 'fail',
    reduceTransparency: params.get('rt') === '1',
  });
  // The window opens on the same fixture and states as the popover.
  const openWindow = async (next: Tab) => {
    const query = new URLSearchParams(params);
    query.set('view', 'window');
    query.set('tab', next);
    window.open('?' + query.toString(), '_blank');
  };
  return { api: view === 'popover' ? { ...api, openWindow } : api, view, clock: () => FIXTURE_NOW };
}
