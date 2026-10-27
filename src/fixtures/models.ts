// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Models built from the snapshot fixtures that Rust generates
// (`crates/quota-station-core/tests/ui_fixtures.rs`). Synthetic data only.

import { envFixture } from './fakeEnv';
import { backupFixture } from './fakeBackup';
import { instrFixture, pluginsFixture, skillsFixture } from './fakeSkills';
import { setsFixture } from './fakeSets';
import { statsFixture } from './fakeStats';
import type { AuthorModel } from '../lib/author';
import type {
  AgyAccounts,
  ReleaseView,
  AgyLogin,
  AlertRecord,
  AlertsView,
  AppView,
  ClaudeAccountRow,
  ClaudeAccountsView,
  Account,
  CodexAccount,
  CodexCustom,
  CodexReach,
  CodexTrigger,
  CodexView,
  Line,
  FirstmateView,
  HistoryEntry,
  McpDef,
  McpRow,
  McpTable,
  McpView,
  Model,
  PeerView,
  RotationView,
  Rules,
  Snapshot,
  SourceView,
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
    legacy: null,
    ...over,
  };
}

/** The four accounts of the pool fixture, kept by Quota Station; Team A (2) in use. */
export function claudeAccountsFixture(over: Partial<ClaudeAccountsView> = {}): ClaudeAccountsView {
  const row = (slot: number, email: string, five: number | null, over: Partial<ClaudeAccountRow> = {}): ClaudeAccountRow => ({
    slot,
    alias: null,
    enabled: true,
    active: false,
    kind: 'login',
    status: 'fresh',
    fetchedAt: '2026-01-16T07:18:00Z',
    ageSeconds: 120,
    windows: { fiveHour: five == null ? null : { usedPercent: five, resetsAt: null }, sevenDay: null, fable: null },
    error: null,
    email,
    ...over,
  });
  return {
    ready: true,
    importable: true,
    accounts: [
      row(1, 'lanpham.dev@example.com', 0),
      row(2, 'lan.acme1@example.com', 73, { active: true }),
      row(3, 'lan.acme2@example.com', 0),
      row(4, 'lan.acme3@example.com', 2),
    ],
    working: null,
    last: null,
    rotation: true,
    legacy: null,
    claude: true,
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
    poolAlert: { dryAtMost: 10, roomAtLeast: 15 },
    unanswered: 0,
    // The bell counts the cards not read yet unless told otherwise.
    unread: (over.recent ?? []).filter((r) => !r.read).length,
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
    read: false,
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
    read: false,
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
/** GitHub Releases checked at 09:14 (UTC+7) this morning; 0.3.0 is out when `newer`. */
export function releaseFixture(over: Partial<ReleaseView> = {}): ReleaseView {
  return {
    auto: true,
    checking: false,
    checkedAt: Date.parse('2026-01-16T02:14:00Z') / 1000,
    error: null,
    none: false,
    latest: {
      version: '0.1.0',
      publishedAt: '2026-01-02T08:00:00Z',
      notes: ['Bản cộng đồng đầu tiên'],
      url: 'https://github.com/shin315/quota-station/releases/tag/v0.1.0',
      dmg: 'https://github.com/shin315/quota-station/releases/download/v0.1.0/Quota.Station_0.1.0_aarch64.dmg',
      size: 13_800_000,
    },
    newer: false,
    skipped: null,
    later: null,
    ...over,
  };
}

/** 0.3.0 on GitHub, newer than the fixture's 0.1.0. */
export function releaseAvailable(over: Partial<ReleaseView> = {}): ReleaseView {
  return releaseFixture({
    latest: {
      version: '0.3.0',
      publishedAt: '2026-01-15T08:00:00Z',
      notes: ['Tab Claude quản lý tài khoản: thêm, đặt tên gọi, cho nghỉ, xoá', 'Giao diện bằng tiếng Anh', 'Đọc hạn mức nhanh hơn khi mở popover'],
      url: 'https://github.com/shin315/quota-station/releases/tag/v0.3.0',
      dmg: 'https://github.com/shin315/quota-station/releases/download/v0.3.0/Quota.Station_0.3.0_aarch64.dmg',
      size: 14_200_000,
    },
    newer: true,
    ...over,
  });
}

export function appFixture(over: Partial<AppView> = {}): AppView & SourceView {
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
  const v: CodexView = {
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
      proxy: false,
    },
    wantsRequires: true,
    plugins: true,
    takesKey: true,
    keyUndo: null,
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
    accounts: [],
    accountsDir: '~/Library/Application Support/com.shin315.quota-station/codex/accounts',
    trigger: { on: false, mode: 'daily', at: '06:00', keepAlive: true, ids: [], next: null, running: null, runs: [] },
    ...over,
  };
  return over.accounts ? v : { ...v, accounts: codexAccountsOf(v) };
}

/** Codex's accounts as the app lists them before any was added: the sign-in in the Codex folder and the custom table. */
export function codexAccountsOf(v: CodexView): CodexAccount[] {
  const out: CodexAccount[] = [];
  if (v.login === 'chatGpt' || v.login === 'keychain')
    out.push({ id: 'chatgpt:0', kind: 'chatgpt', name: 'ChatGPT', nickname: false, number: 0, email: null, plan: null, active: v.current === 'chatgpt', signedIn: true, login: 'ok', custom: null, reach: null, refusal: null, addedAt: null });
  if (v.custom)
    out.push({ id: v.custom.id, kind: 'custom', name: v.custom.name, nickname: false, number: null, email: null, plan: null, active: v.current === 'custom', signedIn: false, login: 'ok', custom: v.custom, reach: v.reach, refusal: v.refusal, addedAt: null });
  return out;
}

const relayEu: CodexCustom = { id: 'relay_eu', name: 'relay-eu', baseUrl: 'https://eu.relay.example.com/v1', key: 'bearer', envKey: null, requiresOpenaiAuth: true, plain: true, proxy: false };
const backupApi: CodexCustom = { id: 'backup_api', name: 'backup-api', baseUrl: 'https://api.backup.example.net/v1', key: 'bearer', envKey: null, requiresOpenaiAuth: false, plain: true, proxy: false };

/** One made-up ChatGPT account's quota, as the snapshot's Codex section holds it (percent left). */
function codexQuota(number: number, email: string, left: [number, number], resets: [string, string], fetchedAt: string, reading: Account['reading'] = 'fresh'): Account {
  const lines: Line[] = [
    { kind: 'session', percent: left[0], resetsAt: resets[0], pending: false, atLeast: false },
    { kind: 'week', percent: left[1], resetsAt: resets[1], pending: false, atLeast: false },
  ];
  return { number, role: 'unknown', active: false, disabled: false, reading, fetchedAt, source: 'list', membership: 'unknown', leavesAt: null, lines, windows: lines, email };
}

/**
 * Several Codex accounts on made-up data: four ChatGPT sign-ins (one in the
 * Codex folder kept for plugins, one idle with its last reading, one that
 * must sign in again) and three custom providers (relay in use, relay-eu,
 * backup-api not answering).
 */
export function codexMany(model: Model): Model {
  const c = model.codex;
  const first = model.snapshot.codex.accounts[0];
  const quota = [
    { ...(first ?? codexQuota(1, '', [77, 59], ['2026-01-16T09:30:00.000Z', '2026-01-20T13:00:00.000Z'], '2026-01-16T07:19:00.000Z')), number: 1, email: 'lan.chatgpt1@example.com', source: 'list' as const },
    codexQuota(2, 'lan.chatgpt2@example.com', [62, 78], ['2026-01-16T08:40:00.000Z', '2026-01-18T10:20:00.000Z'], '2026-01-16T07:17:00.000Z'),
    codexQuota(3, 'lan.team@example.com', [4, 12], ['2026-01-14T11:55:00.000Z', '2026-01-19T08:20:00.000Z'], '2026-01-14T09:10:00.000Z', 'stale'),
    { ...codexQuota(4, 'lan.side@example.com', [0, 0], ['', ''], '2026-01-09T09:00:00.000Z', 'noData'), lines: [], windows: [] },
  ];
  const reach = (ms: number) => ({ state: 'up' as const, ms, since: null, checkedAt: '2026-01-16T07:19:00Z' });
  const on = (id: string) => (c.current === 'custom' ? c.custom?.id === id : id === 'chatgpt:1' && c.current === 'chatgpt');
  const gpt = (n: number, email: string, plan: string, login: CodexAccount['login'], extra: Partial<CodexAccount> = {}): CodexAccount => ({
    id: 'chatgpt:' + n,
    kind: 'chatgpt',
    name: email.slice(0, email.indexOf('@')),
    nickname: false,
    number: n,
    email,
    plan,
    active: on('chatgpt:' + n),
    signedIn: n === 1,
    login,
    custom: null,
    reach: null,
    refusal: null,
    addedAt: n === 1 ? null : '2026-01-1' + n + 'T08:00:00Z',
    ...extra,
  });
  const custom = (cu: CodexCustom, r: CodexReach, refusal: CodexView['refusal'] = null): CodexAccount => ({
    id: cu.id,
    kind: 'custom',
    name: cu.name,
    nickname: false,
    number: null,
    email: null,
    plan: null,
    active: on(cu.id),
    signedIn: false,
    login: 'ok',
    custom: cu,
    reach: r,
    refusal,
    addedAt: cu.id === 'custom' ? null : '2026-01-15T10:00:00Z',
  });
  const accounts: CodexAccount[] = [
    gpt(1, 'lan.chatgpt1@example.com', 'Plus', 'ok'),
    gpt(2, 'lan.chatgpt2@example.com', 'Pro', 'ok'),
    gpt(3, 'lan.team@example.com', 'Team', 'idle'),
    gpt(4, 'lan.side@example.com', 'Plus', 'relogin', { name: 'Tài khoản phụ', nickname: true }),
    ...(c.custom ? [custom(c.custom, c.reach, c.refusal)] : []),
    custom(relayEu, reach(240)),
    custom(backupApi, { state: 'down', ms: null, since: '2026-01-16T06:52:00Z', checkedAt: '2026-01-16T07:19:00Z' }),
  ];
  return {
    ...model,
    snapshot: { ...model.snapshot, codex: { ...model.snapshot.codex, accounts: quota.map((q) => ({ ...q, active: on('chatgpt:' + q.number) })) } },
    codex: { ...c, accounts },
  };
}

/**
 * "Tự đánh thức tài khoản ChatGPT" on `codexMany`'s accounts: each day at
 * 06:00 (the Mac's time, here UTC+7), the three that sign in; this morning
 * one opened its window, one had it open already, one opened it after a
 * keep-alive; yesterday the side account was refused and left out since.
 */
export function codexTriggerFixture(on: boolean): CodexTrigger {
  return {
    on,
    mode: 'daily',
    at: '06:00',
    keepAlive: true,
    ids: ['chatgpt:1', 'chatgpt:2', 'chatgpt:3'],
    next: on ? '2026-01-16T23:00:00Z' : null,
    running: null,
    runs: [
      { at: '2026-01-15T23:01:10Z', id: 'chatgpt:3', why: 'daily', result: 'started', resetsAt: '2026-01-16T04:01:10Z', detail: null },
      { at: '2026-01-15T23:00:40Z', id: 'chatgpt:2', why: 'daily', result: 'open', resetsAt: null, detail: null },
      { at: '2026-01-15T23:00:05Z', id: 'chatgpt:1', why: 'daily', result: 'started', resetsAt: '2026-01-16T04:00:05Z', detail: null },
      { at: '2026-01-14T23:00:30Z', id: 'chatgpt:4', why: 'daily', result: 'relogin', resetsAt: null, detail: null },
    ],
  };
}

/** A Codex folder with nothing in it yet: no sign-in, no provider. */
export function codexEmpty(model: Model): Model {
  return {
    ...model,
    snapshot: { ...model.snapshot, codex: { ...model.snapshot.codex, status: 'single', reason: null, accounts: [] } },
    codex: { ...model.codex, current: 'chatgpt', custom: null, login: 'missing', config: 'missing', hash: null, refusal: { kind: 'noCustom' }, accounts: [], resume: {}, recent: [], lastWrite: null, backups: [] },
  };
}

const ON = { state: 'on', lock: null, parked: false } as const;
const OFF = { state: 'off', lock: null, parked: false } as const;
const ABSENT = { state: 'absent', lock: null, parked: false } as const;
const view = (def: McpDef, slots: string[] = []) => ({ def, slots, extra: [] });
const stdio = (command: string, args: string[] = [], env: { name: string; value: string }[] = []): McpDef => ({ kind: 'stdio', command, args, env, cwd: null });
const http = (url: string, headers: { name: string; value: string }[] = []): McpDef => ({ kind: 'http', url, headers, sse: false });
const MASK = '••••••';

function mcpRow(over: Partial<McpRow> & Pick<McpRow, 'name' | 'cells'>): McpRow {
  return { kind: 'stdio', program: null, host: null, secrets: [], differs: false, missingCommand: null, defs: {}, sameSecrets: {}, ...over };
}

/**
 * The MCP table (M9) on made-up servers: one on everywhere, one with a key kept out of sight, one
 * that differs between Claude Code and Antigravity, one turned off in Claude Code (the app keeps
 * it), one short of its key, one whose command is not found, and Codex App's own two.
 */
export function mcpTableFixture(over: Partial<McpTable> = {}): McpTable {
  const files = stdio('npx', ['-y', 'files-mcp', '~/Projects']);
  const browser = stdio('browser-mcp');
  const docs = http('https://docs.example.com/mcp', [{ name: 'Authorization', value: MASK }]);
  const kitClaude = stdio('design-kit-mcp', ['--theme', 'dark'], [{ name: 'DESIGN_KIT_TOKEN', value: MASK }]);
  const kitAgy = stdio('design-kit-mcp', [], [{ name: 'DESIGN_KIT_TOKEN', value: MASK }]);
  const tracker = http('https://tracker.example.com/mcp', [{ name: 'Authorization', value: '<redacted>' }]);
  const notes = stdio('notes-mcp', ['--vault', 'work']);
  const desk = stdio('npx', ['-y', 'ticket-desk-mcp'], [{ name: 'TICKET_DESK_KEY', value: '<redacted>' }]);
  return {
    columns: [
      { agent: 'claude', present: true, file: '~/.claude.json', state: 'ok', reason: null, writable: true },
      { agent: 'codex', present: true, file: '~/.codex/config.toml', state: 'ok', reason: null, writable: true },
      { agent: 'agy', present: true, file: '~/.gemini/config/mcp_config.json', state: 'ok', reason: null, writable: true },
    ],
    rows: [
      mcpRow({ name: 'browser', program: 'browser-mcp', cells: { claude: ON, codex: ON, agy: OFF }, defs: { claude: view(browser), codex: view(browser), agy: view(browser) } }),
      mcpRow({
        name: 'design-kit',
        program: 'design-kit-mcp',
        secrets: [{ id: 'env:DESIGN_KIT_TOKEN', label: 'DESIGN_KIT_TOKEN', place: 'env', have: true, bitwarden: 'Design Kit' }],
        cells: { claude: ON, codex: ABSENT, agy: ON },
        differs: true,
        defs: { claude: view(kitClaude, ['env:DESIGN_KIT_TOKEN']), agy: view(kitAgy, ['env:DESIGN_KIT_TOKEN']) },
        sameSecrets: { 'env:DESIGN_KIT_TOKEN': true },
      }),
      mcpRow({
        name: 'docs-search',
        kind: 'http',
        host: 'docs.example.com',
        secrets: [{ id: 'header:Authorization', label: 'Authorization', place: 'header', have: true, bitwarden: 'Docs search' }],
        cells: { claude: ON, codex: ON, agy: ABSENT },
        defs: { claude: view(docs, ['header:Authorization']), codex: view(docs, ['header:Authorization']) },
        sameSecrets: { 'header:Authorization': true },
      }),
      mcpRow({ name: 'files', program: 'files-mcp', cells: { claude: ON, codex: ON, agy: ON }, defs: { claude: view(files), codex: view(files), agy: view(files) } }),
      mcpRow({
        name: 'notes',
        program: 'notes-mcp',
        cells: { claude: ABSENT, codex: OFF, agy: ON },
        missingCommand: { command: 'notes-mcp', path: false },
        defs: { codex: view(notes), agy: view(notes) },
      }),
      mcpRow({
        name: 'ticket-desk',
        program: 'ticket-desk-mcp',
        secrets: [{ id: 'env:TICKET_DESK_KEY', label: 'TICKET_DESK_KEY', place: 'env', have: false, bitwarden: 'Ticket Desk' }],
        cells: { claude: { state: 'off', lock: 'missingKey', parked: true }, codex: { state: 'absent', lock: 'missingKey', parked: false }, agy: { state: 'absent', lock: 'missingKey', parked: false } },
        defs: { claude: view(desk, ['env:TICKET_DESK_KEY']) },
      }),
      mcpRow({
        name: 'tracker',
        kind: 'http',
        host: 'tracker.example.com',
        secrets: [{ id: 'header:Authorization', label: 'Authorization', place: 'header', have: true, bitwarden: null }],
        cells: { claude: { state: 'off', lock: null, parked: true }, codex: OFF, agy: ABSENT },
        defs: { claude: view(tracker, ['header:Authorization']), codex: view(http('https://tracker.example.com/mcp', [{ name: 'Authorization', value: MASK }]), ['header:Authorization']) },
      }),
    ],
    owned: [
      { name: 'js-repl', owner: 'Codex', agents: ['codex'] },
      { name: 'screen-control', owner: 'Codex', agents: ['codex'] },
    ],
    stored: [
      {
        name: 'old-wiki',
        kind: 'http',
        program: null,
        host: 'wiki.example.com',
        at: 1768380000,
        agents: ['claude', 'codex'],
        def: view(http('https://wiki.example.com/mcp')),
        missing: [],
      },
    ],
    ...over,
  };
}

/**
 * The same table on servers named after real services, so their logos show: one by its name,
 * one only by its package, one only by its host, the rest by name, and `files` with none.
 * The connectors Claude Code brings itself name services too.
 */
export function mcpBrandsFixture(): McpTable {
  const t = mcpTableFixture();
  const gh = stdio('npx', ['-y', '@modelcontextprotocol/server-github'], [{ name: 'GITHUB_PERSONAL_ACCESS_TOKEN', value: MASK }]);
  const repo = mcpRow({
    name: 'repo-tools',
    program: '@modelcontextprotocol/server-github',
    secrets: [{ id: 'env:GITHUB_PERSONAL_ACCESS_TOKEN', label: 'GITHUB_PERSONAL_ACCESS_TOKEN', place: 'env', have: true, bitwarden: null }],
    cells: { claude: ON, codex: ON, agy: OFF },
    defs: { claude: view(gh, ['env:GITHUB_PERSONAL_ACCESS_TOKEN']), codex: view(gh, ['env:GITHUB_PERSONAL_ACCESS_TOKEN']) },
  });
  const remote = (name: string, host: string, cells: McpRow['cells']) =>
    mcpRow({ name, kind: 'http', host, cells, defs: { claude: view(http('https://' + host + '/mcp')) } });
  return {
    ...t,
    rows: [
      remote('figma', 'mcp.figma.com', { claude: ON, codex: ON, agy: ON }),
      t.rows.find((r) => r.name === 'files')!,
      remote('issues', 'mcp.linear.app', { claude: ON, codex: OFF, agy: ABSENT }),
      remote('notion', 'mcp.notion.com', { claude: ON, codex: ABSENT, agy: ON }),
      repo,
      remote('supabase', 'mcp.supabase.com', { claude: OFF, codex: ON, agy: ABSENT }),
      remote('vercel', 'mcp.vercel.com', { claude: ON, codex: ABSENT, agy: ABSENT }),
    ],
    owned: [
      { name: 'claude.ai Gmail', owner: 'Claude Code', agents: ['claude'] },
      { name: 'claude.ai Google Drive', owner: 'Claude Code', agents: ['claude'] },
      { name: 'claude.ai Slack', owner: 'Claude Code', agents: ['claude'] },
      { name: 'js-repl', owner: 'Codex', agents: ['codex'] },
    ],
  };
}

export function mcpFixture(over: Partial<McpView> = {}): McpView {
  return {
    table: mcpTableFixture(),
    readAt: '2026-01-16T07:19:30Z',
    busy: null,
    last: null,
    backups: [
      { agent: 'codex', at: '2026-01-16T06:52:00Z', file: 'config.toml.20260116-135200', changes: ['docs-search:on'] },
      { agent: 'codex', at: '2026-01-16T06:40:00Z', file: 'config.toml.20260116-134000', changes: [] },
      { agent: 'claude', at: '2026-01-16T06:30:00Z', file: 'mcpServers.json.20260116-133000', changes: ['tracker:off'] },
      { agent: 'agy', at: '2026-01-15T15:40:00Z', file: 'mcp_config.json.20260115-224000', changes: ['files:add'] },
    ],
    ...over,
  };
}

/**
 * The same Mac on a build without the author's tools (a release build), whatever its settings
 * file says: Nâng cao off, and no rules editor, Firstmate or update from source in the model.
 */
export function communityModel(m: Model): Model {
  const app: AppView = { version: m.app.version, commit: m.app.commit, launchAtLogin: m.app.launchAtLogin };
  const out: Model = { ...m, app, advanced: { on: false, firstmate: false }, capabilities: { ...m.capabilities, authorTools: false } };
  delete out.rotation;
  delete out.firstmate;
  return out;
}

/** The author's Mac, as the fixtures were drawn: a build with the author's tools and the Antigravity accounts. */
export function fixtureModel(name: string, over: Partial<Model> = {}): AuthorModel {
  const snapshot = SNAPSHOTS[name];
  if (!snapshot) throw new Error('no snapshot fixture ' + name);
  return {
    snapshot: structuredClone(snapshot),
    reading: false,
    reads: 1,
    machine: 'Mac mini',
    machineDefault: 'Mac mini',
    granting: false,
    switcher: switcherFixture(),
    claudeSwitch: { ready: true, busy: null },
    claudeAccounts: claudeAccountsFixture(),
    rotation: rotationFixture(),
    showNames: true,
    theme: 'auto',
    themePack: 'default',
    language: 'auto',
    menubar: 'claude',
    menubarWindow: 'fiveHours',
    // The author's Mac, as the fixtures were drawn: Nâng cao on, a build with the Antigravity accounts.
    advanced: { on: true, firstmate: true },
    capabilities: { agyAccounts: true, authorTools: true, platform: 'macos' },
    antigravity: agyFixture(),
    agySwitch: { busy: null, listed: false, overwritten: null },
    alerts: alertsFixture(),
    firstmate: { file: null, status: 'notSet', line: null, writtenAt: null, fresh: false, ownDry: false },
    app: appFixture(),
    codex: codexFixture(),
    mcp: mcpFixture(),
    backup: backupFixture(),
    skills: skillsFixture(),
    release: releaseFixture(),
    firstRun: false,
    plugins: pluginsFixture(),
    instructions: instrFixture(),
    env: envFixture(),
    sets: setsFixture(),
    stats: statsFixture(),
    ...over,
    // An override of the app's row comes with the update from source, as every fixture's does.
  } as AuthorModel;
}

