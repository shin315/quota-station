// Models built from the snapshot fixtures that Rust generates
// (`crates/quota-station-core/tests/ui_fixtures.rs`). Synthetic data only.

import type {
  AgyAccounts,
  AppView,
  AgyLogin,
  AlertRecord,
  AlertsView,
  CodexView,
  ErrorView,
  FirstmateView,
  HistoryEntry,
  Model,
  PeerView,
  RotationView,
  Rules,
  Snapshot,
  SwitcherView,
} from '../lib/model';

const files = import.meta.glob('./snapshots/*.json', { eager: true, import: 'default' }) as Record<string, Snapshot>;

export const SNAPSHOTS: Record<string, Snapshot> = Object.fromEntries(
  Object.entries(files).map(([path, snap]) => [path.replace('./snapshots/', '').replace('.json', ''), snap]),
);

/** 40 seconds after the fixtures were generated (14:20:40 at UTC+7). */
export const FIXTURE_NOW = new Date('2026-01-16T07:20:40.000Z');

/**
 * The switcher on a Mac where all is well: the dry run a minute ago kept
 * Team A, and the last switch it logged was to Team A this morning.
 */
export function switcherFixture(over: Partial<SwitcherView> = {}): SwitcherView {
  return {
    installed: true,
    pause: null,
    dryRun: { decision: { kind: 'stay' }, raw: 'stay', at: '2026-01-16T07:19:40.000Z' },
    dryRunError: null,
    guard: null,
    lastLog: {
      at: '2026-01-16T02:12:05.000Z',
      event: { kind: 'switch', slot: 2 },
      raw: '2026-01-16 09:12:05 switch to slot 2 (ends for good at 17:45, use it first)',
    },
    service: true,
    autoService: false,
    ...over,
  };
}

/** Paused by a manual switch to Team A at 14:12, held until 15:20 (UTC+7). */
export function heldSwitcher(over: Partial<SwitcherView> = {}): SwitcherView {
  return switcherFixture({
    pause: { until: '2026-01-16T08:20:00.000Z', reason: 'chuyển tay sang tài khoản 2 lúc 14:12', holds: true },
    dryRun: {
      decision: { kind: 'paused', until: '2026-01-16T15:20', reason: 'chuyển tay sang tài khoản 2 lúc 14:12' },
      raw: 'paused until 2026-01-16T15:20: chuyển tay sang tài khoản 2 lúc 14:12',
      at: '2026-01-16T07:19:40.000Z',
    },
    guard: true,
    ...over,
  });
}

/** The rotation in the fixtures: Team A, B and C in turn, Cá nhân 08:00–24:00, Team A leaving at 17:45. */
export function rulesFixture(over: Partial<Rules> = {}): Rules {
  return {
    team: [2, 3, 4],
    personal: 1,
    personalFrom: 8,
    personalTo: 23,
    full: 90,
    fableFull: 95,
    expire: [{ slot: 2, at: '2026-01-16T17:45' }],
    ...over,
  };
}

/** Saved on this Mac at 14:02 (UTC+7); Mac B and Mac C have it. */
export const SAVED_AT = '2026-01-16T14:02:00+07:00';

export function peerFixture(name: string, alias: string, over: Partial<PeerView> = {}): PeerView {
  return { name, alias, state: { kind: 'synced', at: '2026-01-16T07:02:05.000Z' }, current: true, theirRules: null, pushing: false, ...over };
}

export function rotationFixture(over: Partial<RotationView> = {}): RotationView {
  return {
    path: '~/.config/cswap-prefer-team/rotation',
    status: 'ok',
    rules: rulesFixture(),
    reason: null,
    version: 'v1',
    stamp: { at: SAVED_AT, machine: 'Mac mini' },
    ignoredKeys: [],
    defaults: rulesFixture({ expire: [] }),
    saving: false,
    peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c')],
    ssh: true,
    ...over,
  };
}

/** The Antigravity login card on a Mac with the OAuth client in place and no account yet. */
export function agyFixture(over: Partial<AgyAccounts> = {}): AgyAccounts {
  return {
    client: 'ok',
    clientPath: '~/Library/Application Support/com.shin315.quota-station/oauth-client.json',
    accounts: [],
    indexError: false,
    loggingIn: false,
    ...over,
  };
}

/** An account logged in here, numbered `n`; emails stay hidden unless given. */
export function agyAccount(n: number, over: Partial<AgyLogin> = {}): AgyLogin {
  return {
    id: 'antigravity_fake' + n,
    number: n,
    email: null,
    name: null,
    status: 'ok',
    verify: false,
    reading: false,
    readError: null,
    ...over,
  };
}

/** No alert yet, the five switches as they ship, and macOS letting the cards show. */
export function alertsFixture(over: Partial<AlertsView> = {}): AlertsView {
  return {
    recent: [],
    switches: { pool: true, leaving: true, login: true, overwrite: true, firstmate: false },
    unanswered: 0,
    permission: 'allowed',
    ...over,
  };
}

/**
 * Team A leaving the rotation at 14:45, sent at 14:15 (UTC+7) on the fixture day:
 * the prototype's card 2, with its two buttons.
 */
export function leavingAlert(over: Partial<AlertRecord> = {}): AlertRecord {
  return {
    id: 'leaving:2:2026-01-16T14:45@1768547700',
    alert: {
      key: 'leaving:2:2026-01-16T14:45',
      kind: 'leaving',
      tone: 'warn',
      title: 'Team A rời vòng lúc 14:45',
      body: 'Còn 30 phút. Sau đó switcher chuyển sang tài khoản khác trong vòng.',
      actions: [
        { id: 'dismiss', label: 'Ổn' },
        { id: 'switch:3', label: 'Chuyển ngay' },
      ],
      at: 1768547700,
    },
    notified: true,
    answer: null,
    ...over,
  };
}

/** Antigravity.app wrote Ultra 1 back at 14:10, 20 minutes after a switch to Ultra 2. */
export function overwriteAlert(over: Partial<AlertRecord> = {}): AlertRecord {
  return {
    id: 'overwrite:antigravity_fake2:1768547400@1768547400',
    alert: {
      key: 'overwrite:antigravity_fake2:1768547400',
      kind: 'overwrite',
      tone: 'warn',
      title: 'Antigravity đổi về tài khoản cũ',
      body: 'Lúc 14:10 Antigravity.app đổi về Ultra 1. Bạn đã chuyển sang Ultra 2 lúc 13:50.',
      actions: [
        { id: 'agy:quitFirst:2', label: 'Thoát app rồi chuyển lại' },
        { id: 'agy:keepRunning:2', label: 'Chuyển lại' },
        { id: 'agy:keep', label: 'Thôi' },
      ],
      at: 1768547400,
    },
    notified: true,
    answer: null,
    ...over,
  };
}

/** Firstmate's file is set and was written a minute ago: the pool has room. */
export function firstmateFixture(over: Partial<FirstmateView> = {}): FirstmateView {
  return {
    file: '~/example/quota-pool.last',
    status: 'ok',
    line: {
      fork: true,
      autoSwitcher: false,
      claudeSwap: true,
      active: 2,
      dry: false,
      slots: [
        { slot: 1, percent: 29, reset: false },
        { slot: 2, percent: 0, reset: false },
        { slot: 3, percent: 100, reset: false },
        { slot: 4, percent: 98, reset: false },
      ],
      nextReset: null,
    },
    writtenAt: '2026-01-16T07:19:40.000Z',
    fresh: true,
    ownDry: false,
    ...over,
  };
}

/**
 * A morning on this Mac, newest first as the Rust side sends them (UTC+7): a manual
 * switch, an Antigravity switch Antigravity.app undid, the switcher's switch and prime,
 * and a change seen last night that nothing here made.
 */
export function historyFixture(): HistoryEntry[] {
  return [
    {
      at: '2026-01-16T07:12:00Z',
      app: 'claude',
      source: 'app',
      from: 3,
      to: 2,
      cause: { kind: 'manual', holdUntil: '2026-01-16T08:20:00Z', result: 'switched' },
    },
    {
      at: '2026-01-16T06:50:00Z',
      app: 'codex',
      source: 'app',
      from: null,
      to: null,
      cause: { kind: 'codexSwitch', from: 'openai', to: 'custom', restarted: true },
    },
    {
      at: '2026-01-16T06:40:00Z',
      app: 'antigravity',
      source: 'observed',
      from: 2,
      to: 1,
      cause: { kind: 'agyOverwrite', overwrites: 1 },
    },
    {
      at: '2026-01-16T06:20:00Z',
      app: 'antigravity',
      source: 'app',
      from: 1,
      to: 2,
      cause: { kind: 'agySwitch', how: 'quitFirst', result: 'switched', reopened: true },
    },
    {
      at: '2026-01-16T02:12:05Z',
      app: 'claude',
      source: 'switcher',
      from: null,
      to: 3,
      cause: {
        kind: 'switcher',
        why: { kind: 'room', used: 4, ends: '15:00' },
        raw: '2026-01-16 09:12:05 switch to slot 3 (4.0%, ends 15:00)',
      },
    },
    {
      at: '2026-01-16T01:01:00Z',
      app: 'claude',
      source: 'switcher',
      from: null,
      to: 3,
      cause: { kind: 'primed', raw: '2026-01-16 08:01:00 primed slot 3 (exit 0)' },
    },
    { at: '2026-01-15T15:47:00Z', app: 'claude', source: 'observed', from: 2, to: 1, cause: { kind: 'observed' } },
  ];
}

/** The app's own row: an installed copy, built by `scripts/install.sh` from a made-up commit. */
export function appFixture(over: Partial<AppView> = {}): AppView {
  return {
    version: '0.1.0',
    commit: 'a1b2c3d',
    launchAtLogin: 'off',
    source: '~/src/quota-station',
    sourceStatus: 'ok',
    bundled: true,
    updating: null,
    outcome: null,
    error: null,
    log: '~/Library/Application Support/com.shin315.quota-station/update/install.log',
    ...over,
  };
}

/**
 * Codex on this Mac (M8): it goes through a made-up custom provider, `relay`, with its own key,
 * keeps the ChatGPT login, and the app wrote the file last. No real host, key or path.
 */
export function codexFixture(over: Partial<CodexView> = {}): CodexView {
  return {
    home: '~/.codex',
    customHome: false,
    present: true,
    config: 'ok',
    reason: null,
    hash: 'h1',
    current: 'custom',
    other: null,
    custom: {
      id: 'custom',
      name: 'relay',
      baseUrl: 'https://relay.example.com/v1',
      key: 'bearer',
      envKey: null,
      requiresOpenaiAuth: true,
      plain: true,
    },
    wantsRequires: true,
    refusal: null,
    backRefusal: null,
    login: 'chatGpt',
    reach: { state: 'up', ms: 180, since: null, checkedAt: '2026-01-16T07:19:00Z' },
    ccSwitch: { installed: true, running: false, data: true, traces: [] },
    external: null,
    procs: { sessions: 3, app: true, daemon: true, ccSwitch: false },
    version: '0.163.0',
    mcp: [
      { name: 'files', kind: 'stdio', enabled: true },
      { name: 'browser', kind: 'stdio', enabled: true },
      { name: 'docs-search', kind: 'http', enabled: true },
      { name: 'tracker', kind: 'http', enabled: false },
    ],
    resume: { openai: 128, custom: 54 },
    lastWrite: { at: '2026-01-16T06:40:00Z', provider: 'custom', matches: true },
    backups: ['2026-01-16T06:40:00Z', '2026-01-15T02:05:00Z', '2026-01-14T11:22:00Z'],
    backupDir: '~/Library/Application Support/com.shin315.quota-station/backups/codex',
    recent: [
      { at: '2026-01-16T06:40:00Z', from: 'openai', to: 'custom', result: 'switched', restarted: true },
      { at: '2026-01-15T15:47:00Z', from: 'custom', to: 'openai', result: 'observed', restarted: null },
      { at: '2026-01-15T02:05:00Z', from: 'openai', to: 'custom', result: 'switched', restarted: null },
    ],
    firstSwitchAt: '2026-01-14T11:22:00Z',
    busy: null,
    openaiBaseUrl: false,
    ...over,
  };
}

export function fixtureModel(name: string, over: Partial<Model> = {}): Model {
  const snapshot = SNAPSHOTS[name];
  if (!snapshot) throw new Error('no snapshot fixture ' + name);
  return {
    snapshot: structuredClone(snapshot),
    errors: { claude: null, agy: null, codex: null },
    reading: false,
    reads: 1,
    machine: 'Mac mini',
    machineDefault: 'Mac mini',
    quotaAxi: { path: '~/.local/bin/quota-axi', custom: false, exists: true, version: '0.1.61+cswap.4', fork: true },
    granting: false,
    switcher: switcherFixture(),
    claudeSwitch: { cswap: true, busy: null },
    rotation: rotationFixture(),
    showNames: true,
    theme: 'auto',
    menubar: 'claude',
    antigravity: agyFixture(),
    agySwitch: { busy: null, listed: false, overwritten: null },
    alerts: alertsFixture(),
    firstmate: { file: null, status: 'notSet', line: null, writtenAt: null, fresh: false, ownDry: false },
    app: appFixture(),
    codex: codexFixture(),
    ...over,
  };
}

export function failing(kind: ErrorView['kind'], detail = ''): Model['errors'] {
  return { claude: { kind, detail }, agy: { kind, detail }, codex: { kind, detail } };
}
