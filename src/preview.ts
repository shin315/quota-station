// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Browser preview for `npm run dev`: the popover or the main window on a
// fixture, without Tauri. `?state=keychain&view=window&tab=claude`.
// Switching (M3): `switcher=held|noguard|auto|stopped|missing` sets the
// switcher, `outcome=raced|failed|busy|unconfirmed` what a switch reports, and
// `accounts=none` shows a Mac where Quota Station keeps no Claude account yet. A switch takes 1.5 s, so the working sheet shows.
// Claude accounts: `claude=relogin` has account 3 to sign in again, `claude=renew` makes a sign-in land on
// account 1 (already here), `claude=fail` ends a sign-in without one; an action takes 3 s.
// GitHub Releases: `release=avail|later|skipped|offline|none|off` (up to date by default); "Kiểm bản mới"
// takes 1.5 s and finds 0.3.0, or `releasefinds=offline`. `build=release` is the downloaded app
// (no author's tools, no Antigravity accounts), whose dialog offers the DMG.
// Rotation rules (M5): `tab=rules` with `rotation=unreachable|newer|edited|unstamped|missing|invalid|unwritable|single`;
// a save takes 1 s, and `unreachable` leaves Mac C behind.
// Antigravity logins (M4a) and reads (M4b): `agy=two|relogin|verify|noclient|tooopen|invalid|broken|reading|failing|forbidden`
// sets the login card, `login=timeout|noRefreshToken|keychain|…` how a login ends
// (it keeps the next account after 3 s by default, or at once on any pasted
// URL with `code=` and `state=`), and `email=on` shows the made-up emails.
// Switching Antigravity (M4c): `agyswitch=on|overwritten` lists the two
// accounts as the app's (the second one undone by Antigravity.app with
// `overwritten`), `agyapp=closed|missing|old|unreadable|unknown` is what the sheet learns
// about Antigravity.app (it runs by default), and
// `agyoutcome=unconfirmed|notquit|write` what a switch reports. A switch takes 1.5 s.
// History, alerts and Firstmate (M6): `tab=history|alerts|firstmate`, with
// `history=empty`, `alerts=on|off|unbundled|empty` (two cards by default on
// `tab=alerts`; `off` is macOS refusing the cards) and
// `firstmate=ok|holding|stale|missing|unset` (unset by default).
// The app itself (M7), on `tab=settings`: `app=on|approval|unavailable|nosource|unbundled`
// sets the login switch and the clone, and `update=uptodate|pull|install|timeout` how
// "Cập nhật từ mã nguồn" ends (by default it installs 0.1.1). Each step takes 1.5 s.
// Codex (M8): `codex=chatgpt|ccswitch|down|external|nologin|nokey|placeholder|header|envkey|noplugins|broken|none`
// sets the six states of the design and more (Codex goes through the made-up `relay` by
// default), and `codexoutcome=changed|refused` what a switch reports. A switch or a
// restart takes 1.5 s; `tab=codex` opens the Codex tab.
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
// Chép sang Mac khác (M13): `tab=copy` with the made-up Macs "MacBook Air" (`travel-air`) and
// "Studio iMac" (`studio-imac`), and `copy=unreach|old|noapp|none|changed`: the first Mac does
// not answer, runs version 1, has no app; no Mac in settings; a copy finds a file changed.
// A comparison, a copy and its undo take 0.9 s.
// Variables that override a login (M14): `env=found|claude2|launchd|unread|unchecked` (none set by
// default), shown on `tab=settings` (the card), `tab=claude` (the banner) and with `alerts=env`.
// Bộ cấu hình (M15): `tab=sets` with three made-up sets, Web app applied and drifted by one
// entry; `sets=empty|fresh|even` for none, none applied, or none drifted, and
// `setsoutcome=changed` for an apply that finds Codex's file changed. A preview, an apply and
// an undo take 0.5 s.
// Thống kê (M16): `tab=stats` with made-up counts for made-up projects; `stats=first` shows the
// first read halfway, `stats=new` starts one that runs (a step every 0.4 s), `stats=skip` has
// 38 Codex sessions in a format the app cannot read, `stats=fail` a report that cannot be read.
// The glass (community edition, 24/10/2026): `rt=1` is macOS with Reduce Transparency on (the
// solid ground), and `desk=plain` a plain desktop behind the glass instead of the made-up one.
// `lang=vi|en` sets the language; without it the preview follows the browser's, as the app follows macOS.

import type { Api } from './lib/api';
import type { AgyAccounts, AgyApp, AgyHow, AgyOutcome, CodexOutcome, CodexTarget, McpTable, Model, Outcome, Tab, UpdateError, UpdateOutcome } from './lib/model';
import { fakeApi, fakeEmail, hideNames } from './fixtures/fakeApi';
import { backupFixture, packsFixture } from './fixtures/fakeBackup';
import { API_KEY, BASE_URL, CLAUDE_BASE, envAlert, envFixture, LAUNCHD_GEMINI } from './fixtures/fakeEnv';
import { setsFixture } from './fixtures/fakeSets';
import { firstReadFixture, SKIPPED_CODEX, statsFixture } from './fixtures/fakeStats';
import { instrFixture, pluginsBrandsFixture, pluginsFixture, skillsFixture, skillTableFixture } from './fixtures/fakeSkills';
import {
  agyAccount,
  agyFixture,
  FIXTURE_NOW,
  fixtureModel,
  heldSwitcher,
  peerFixture,
  rotationFixture,
  rulesFixture,
  SNAPSHOTS,
  switcherFixture,
  claudeAccountsFixture,
  releaseAvailable,
  releaseFixture,
  communityModel,
  alertsFixture,
  appFixture,
  codexFixture,
  firstmateFixture,
  mcpBrandsFixture,
  mcpFixture,
  mcpTableFixture,
  historyFixture,
  leavingAlert,
  overwriteAlert,
} from './fixtures/models';

const TABS: Tab[] = ['overview', 'stats', 'claude', 'agy', 'codex', 'rules', 'history', 'alerts', 'firstmate', 'mcp', 'skills', 'plugins', 'instr', 'sets', 'backup', 'copy', 'settings'];

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
  restored: () => backupFixture({ restore: { at: 1768547700, pack: 'agents-20260116-141000-studio-mini', mac: 'Mac mini' } }),
  unread: () => backupFixture({ packs: null, keys: null, readAt: null }),
};

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

const ALERTS: Record<string, () => Model['alerts']> = {
  on: () => alertsFixture({ recent: [overwriteAlert(), leavingAlert()], unanswered: 2 }),
  off: () => alertsFixture({ recent: [overwriteAlert(), leavingAlert()], unanswered: 2, permission: 'denied' }),
  unbundled: () => alertsFixture({ recent: [overwriteAlert(), leavingAlert()], unanswered: 2, permission: 'unbundled' }),
  empty: () => alertsFixture(),
  env: () => alertsFixture({ recent: [envAlert(API_KEY), envAlert(BASE_URL, 1768547640)], unanswered: 2 }),
};

const FIRSTMATES: Record<string, () => Model['firstmate']> = {
  ok: () => firstmateFixture(),
  holding: () =>
    firstmateFixture({ line: { ...firstmateFixture().line!, dry: true, nextReset: '2026-01-16T09:40:00.000Z' }, ownDry: true }),
  stale: () => firstmateFixture({ writtenAt: '2026-01-16T05:20:00.000Z', fresh: false }),
  missing: () => firstmateFixture({ status: 'missing', line: null, writtenAt: null, fresh: false }),
  unset: () => fixtureModel('fresh').firstmate,
};

const APPS: Record<string, () => Model['app']> = {
  on: () => appFixture({ launchAtLogin: 'on' }),
  approval: () => appFixture({ launchAtLogin: 'needsApproval' }),
  unavailable: () => appFixture({ launchAtLogin: 'unavailable', bundled: false, commit: null }),
  nosource: () => appFixture({ source: null, sourceStatus: 'notSet' }),
  unbundled: () => appFixture({ bundled: false, launchAtLogin: 'unavailable', commit: null }),
};

const UPDATES: Record<string, () => UpdateOutcome | UpdateError> = {
  uptodate: () => ({ kind: 'upToDate', commit: 'a1b2c3d4e5f60718293a4b5c6d7e8f9012345678' }),
  pull: () => ({ kind: 'pull', detail: 'fatal: Not possible to fast-forward, aborting.' }),
  install: () => ({ kind: 'install', detail: 'cargo tauri build failed' }),
  timeout: () => ({ kind: 'timeout', step: 'installing' }),
};

const RETRY = '2026-01-16T07:21:40.000Z';
const NEWER = { at: '2026-01-16T14:11:00+07:00', machine: 'Mac B' };

const ROTATIONS: Record<string, () => Model['rotation']> = {
  unreachable: () =>
    rotationFixture({
      stamp: { at: '2026-01-16T14:16:00+07:00', machine: 'Mac mini' },
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
      peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'edited', their: { at: '2026-01-16T14:02:00+07:00', machine: 'Mac mini' } }, theirRules: rulesFixture({ team: [3, 4] }) })],
    }),
  missing: () => rotationFixture({ status: 'missing', rules: null, version: null, stamp: null, peers: [peerFixture('Mac B', 'mac-b', { state: null, current: false }), peerFixture('Mac C', 'mac-c', { state: null, current: false })] }),
  invalid: () => rotationFixture({ status: 'invalid', rules: null, reason: 'rotation_bad_value:personal_hours', stamp: null }),
  single: () => rotationFixture({ peers: [] }),
};

const SWITCHERS: Record<string, () => Model['switcher']> = {
  held: () => heldSwitcher(),
  noguard: () => heldSwitcher({ dryRun: { decision: { kind: 'stay' }, raw: 'stay', at: '2026-01-16T07:19:40.000Z' }, guard: false }),
  auto: () => switcherFixture({ legacy: 'com.cswap.prefer-team' }),
  stopped: () => switcherFixture({ service: false }),
  missing: () => switcherFixture({ installed: false, dryRun: null, lastLog: null, service: null }),
};

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

/** The hold a switch writes: to the minute, as the pause flag keeps it. */
const holdOf = (until: string | null) =>
  until ? { until: new Date(Math.floor(Date.parse(until) / 60000) * 60000).toISOString(), failed: false } : null;

const OUTCOMES: Record<string, (slot: number, holdUntil: string | null) => Outcome> = {
  // The switcher puts back an account other than the one asked for.
  raced: (slot, holdUntil) => ({
    kind: 'raced',
    slot,
    active: slot === 2 ? 3 : 2,
    bySwitcherAt: '2026-01-16T07:21:00.000Z',
    hold: holdOf(holdUntil),
  }),
  failed: (slot) => ({ kind: 'failed', slot, error: 'noCredential', detail: '', active: null }),
  // Claude Code held its login locks, so nothing was written.
  busy: (slot) => ({ kind: 'failed', slot, error: 'busy', detail: '', active: null }),
  unconfirmed: (slot, holdUntil) => ({ kind: 'unconfirmed', slot, hold: holdOf(holdUntil) }),
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
  const switcher = pick(SWITCHERS, params.get('switcher'));
  if (switcher) over.switcher = switcher();
  if (params.get('accounts') === 'none') {
    over.claudeSwitch = { ready: false, busy: null };
    over.claudeAccounts = claudeAccountsFixture({ ready: false, accounts: [], rotation: false });
    over.switcher = switcherFixture({ installed: false, service: false, dryRun: null, dryRunError: 'noAccounts' });
  }
  const rel = params.get('release');
  if (rel === 'avail' || rel === 'later' || rel === 'skipped') over.release = releaseAvailable(rel === 'later' ? { later: '0.3.0' } : rel === 'skipped' ? { skipped: '0.3.0' } : {});
  else if (rel === 'offline') over.release = releaseFixture({ error: 'offline' });
  else if (rel === 'none') over.release = releaseFixture({ none: true, latest: null });
  else if (rel === 'off') over.release = releaseFixture({ auto: false });
  const claude = params.get('claude');
  if (claude === 'relogin')
    over.claudeAccounts = claudeAccountsFixture({
      accounts: claudeAccountsFixture().accounts.map((a) => (a.slot === 3 ? { ...a, status: 'reloginRequired' as const } : a)),
    });
  const rotation = params.get('rotation');
  const rules = pick(ROTATIONS, rotation);
  if (rules) over.rotation = rules();
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
  const lang = params.get('lang');
  if (lang === 'vi' || lang === 'en') over.language = lang;
  const alerts = pick(ALERTS, params.get('alerts') ?? (tab === 'alerts' ? 'on' : null));
  if (alerts) over.alerts = alerts();
  const firstmate = pick(FIRSTMATES, params.get('firstmate'));
  if (firstmate) over.firstmate = firstmate();
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
  const copy = params.get('copy');
  if (tab === 'copy' || copy) {
    const peers = copy === 'none' ? [] : [peerFixture('MacBook Air', 'travel-air'), peerFixture('Studio iMac', 'studio-imac')];
    over.rotation = rotationFixture({ peers });
  }
  const model = fixtureModel(SNAPSHOTS[state] ? state : 'fresh', over);
  if (params.get('agyhot') === 'on') {
    for (const a of model.snapshot.agy.accounts.filter((x) => x.active)) {
      a.windows = a.windows.map((w) => (w.kind === 'claudeGptWeek' ? { ...w, percent: 14 } : w));
    }
  }
  const built: Model = params.get('build') === 'release' ? { ...communityModel(model), capabilities: { agyAccounts: false, authorTools: false } } : model;
  const { api } = fakeApi(params.get('email') === 'off' ? hideNames(built) : built, tab, {
    switchDelayMs: 1500,
    claudeDelayMs: 3000,
    releaseDelayMs: 1500,
    releaseFinds: params.get('releasefinds') === 'offline' ? 'offline' : releaseAvailable().latest,
    claudeRenews: claude === 'renew' ? 1 : undefined,
    claudeFail: claude === 'fail' ? (action) => (action === 'login' ? 'login' : null) : undefined,
    outcome: pick(OUTCOMES, params.get('outcome')),
    saveDelayMs: 1000,
    clock: () => FIXTURE_NOW,
    unreachable: rotation === 'unreachable' ? ['mac-c'] : [],
    rulesError: rotation === 'unwritable' ? { kind: 'write', detail: 'Permission denied', restored: true } : undefined,
    agyLoginDelayMs: 3000,
    agyLogin: ends ? () => ({ kind: ends, detail: '' }) : undefined,
    agyApp: agyApp !== null && Object.hasOwn(AGY_APPS, agyApp) ? AGY_APPS[agyApp] : undefined,
    agyOutcome: pick(AGY_OUTCOMES, params.get('agyoutcome')),
    agySwitchDelayMs: 1500,
    history: params.get('history') === 'empty' ? [] : historyFixture(),
    firstmate: firstmateFixture(),
    loginApproval: params.get('app') === 'approval',
    update: pick(UPDATES, params.get('update')),
    updateDelayMs: 1500,
    codexOutcome: pick(CODEX_OUTCOMES, params.get('codexoutcome')),
    codexDelayMs: 1500,
    mcpChanged: params.get('mcpoutcome') === 'changed',
    mcpDelayMs: 600,
    restoreChanged: params.get('restoreoutcome') === 'changed',
    restoreUndoChanged: params.get('restoreoutcome') === 'undochanged',
    restorePartial: params.get('restoreoutcome') === 'partial',
    restoreDelayMs: 1200,
    skillsChanged: params.get('skillsoutcome') === 'changed',
    skillsUpdateFail: params.get('skillsoutcome') === 'updatefail' ? ['release-notes'] : undefined,
    skillsDelayMs: 800,
    copyUnreach: copy === 'unreach' ? ['travel-air'] : [],
    copyOld: copy === 'old' ? ['travel-air'] : [],
    copyNoApp: copy === 'noapp' ? ['travel-air'] : [],
    copyChanged: copy === 'changed',
    copyDelayMs: 900,
    envDelayMs: 300,
    setsChanged: params.get('setsoutcome') === 'changed' ? 'codex' : undefined,
    setsDelayMs: 500,
    statsSkipped: stats === 'skip' ? SKIPPED_CODEX : undefined,
    statsStepMs: stats === 'new' ? 400 : undefined,
    statsFails: stats === 'fail',
    reduceTransparency: params.get('rt') === '1',
  });
  // The window opens on the same fixture and switcher states as the popover.
  const openWindow = async (next: Tab) => {
    const query = new URLSearchParams(params);
    query.set('view', 'window');
    query.set('tab', next);
    window.open('?' + query.toString(), '_blank');
  };
  return { api: view === 'popover' ? { ...api, openWindow } : api, view, clock: () => FIXTURE_NOW };
}
