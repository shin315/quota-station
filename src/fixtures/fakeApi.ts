// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// A fake Rust side on fixture data, for the tests and the browser preview.
// Never reads a real quota or runs claude-swap, never opens a browser
// or the Keychain, and writes no file.
// Plain code: the preview bundle must not load Vitest.

import type { SessionView } from '../session/steps';
import type { Api } from '../lib/api';
import { fakeParts, fakeRestore, planFixture, summaryFixture } from './fakeBackup';
import { fakeApply, fakePaste, fakePreview } from './fakeMcp';
import { fakeSetChanges, fakeSetDrift } from './fakeSets';
import { SKIPPED_NONE, statsReport } from './fakeStats';
import { claudeAccountsFixture } from './models';
import { nextWake, wakeTimes } from '../lib/codexWake';
import { fakeSkillApply, fakeSkillPreview, instrDiffFixture, instrShownFixture, repoFixture, SKILL_MD } from './fakeSkills';
import type {
  Account,
  ReleaseView,
  Agent,
  AgyApp,
  AgyHow,
  AgyLogin,
  AgyLoginDone,
  AgyLoginFailure,
  AgyOutcome,
  Alert,
  AlertKind,
  AlertRecord,
  AlertSaid,
  ClaudeAccountAction,
  CliFailure,
  CliTool,
  CliToolView,
  ClaudeAccountRow,
  ClaudeAccountsView,
  ClaudeActionDone,
  CodexAccount,
  CodexAccountDone,
  CodexCustom,
  CodexImportItem,
  CodexImportPreview,
  CodexKeyOutcome,
  CodexKeyPreview,
  CodexLoginFail,
  CodexOutcome,
  CodexProviderCheck,
  CodexRestart,
  CodexTarget,
  CodexTriggerRun,
  Line,
  EnvFound,
  StatsSkipped,
  HistoryEntry,
  McpOutcome,
  Model,
  SetOutcome,
  SetsView,
  SkillOutcome,
  Outcome,
  Sessions,
  SettingsGroup,
  Tab,
} from '../lib/model';
// @private:begin
import type { AuthorModel } from '@/private/author';
import { comparisonFixture, copyDoneFixture, copyUndoneFixture } from '@/private/copy/fakeCopy';
import type { CopyComparison, FirstmateView, PeerView, RotationView, RulesError, UpdateError, UpdateOutcome } from '../lib/model';
// @private:end

export interface FakeOptions {
  /** A test build's guided session: where the guide was, and what the PC has. */
  session?: Partial<SessionView>;
  /** macOS reduces transparency: the windows show the solid ground. */
  reduceTransparency?: boolean;
  /** macOS increases contrast: the edges are drawn stronger. */
  increaseContrast?: boolean;
  // @private:begin
  /** Aliases SSH cannot reach: a save leaves them behind, and "Thử SSH" fails. */
  unreachable?: string[];
  /** What a save, adopt or keep rejects with, instead of writing. */
  rulesError?: RulesError;
  /** How long a save takes; the preview waits to show "Đang lưu…". */
  saveDelayMs?: number;
  // @private:end
  /** When an action happens, for the stamps it writes. */
  clock?: () => Date;
  // @private:begin
  /** `Thoát Quota Station` was chosen before the window listened. */
  quitPending?: boolean;
  // @private:end
  /** How long a switch takes before it reports; the preview waits to show the working sheet. */
  switchDelayMs?: number;
  sessions?: Sessions | null;
  /** What a switch reports; by default it lands on the slot asked for. */
  outcome?: (
    slot: number,
    // @private:begin
    holdUntil: string | null,
    // @private:end
  ) => Outcome;
  /** The error kind a Claude account action ends with; none lands it. */
  claudeFail?: (action: ClaudeAccountAction) => string | null;
  /** How long a Claude account action takes; the preview waits to show it running. */
  claudeDelayMs?: number;
  /** A sign-in or the account Claude Code is on turns out to be this kept account: it is renewed, not added. */
  claudeRenews?: number;
  /** What "Kiểm tra bản mới" of the app finds: a release (by default the one the model has), or an error kind. */
  releaseFinds?: ReleaseView['latest'] | 'offline';
  /** How long that check takes. */
  releaseDelayMs?: number;
  /** How a tool's "Cập nhật" ends; by default it lands on the newest. */
  cliFails?: Partial<Record<CliTool, { failure: CliFailure; detail: string }>>;
  /** How long a check or an update of a command-line tool takes. */
  cliDelayMs?: number;
  /** What "Kiểm lại" finds: the tools' sessions closed meanwhile. */
  cliClosed?: boolean;
  /**
   * How an Antigravity login ends when no callback is pasted and it is not cancelled; by default
   * it keeps the next account after `agyLoginDelayMs`.
   */
  agyLogin?: () => AgyLoginFailure | null;
  /** How long a login waits for the browser before it ends on its own; none waits for a paste. */
  agyLoginDelayMs?: number;
  /** What the switch sheet learns about Antigravity.app; null makes the question fail. By default it runs. */
  agyApp?: AgyApp | null;
  /** What an Antigravity switch reports; by default it lands on the account asked for. */
  agyOutcome?: (number: number, how: AgyHow) => AgyOutcome;
  /** How long an Antigravity switch takes before it reports. */
  agySwitchDelayMs?: number;
  /** What a Codex switch reports; by default it lands where it was asked to. */
  codexOutcome?: (target: CodexTarget) => CodexOutcome;
  /** What a restart of Codex's background process reports; by default it restarts. */
  codexRestart?: CodexRestart;
  /** How long a Codex switch or restart takes; the preview waits to show the working sheet. */
  codexDelayMs?: number;
  /** How a ChatGPT sign-in for a Codex account ends: `dup` signs in to an account already listed; by default it adds one. */
  codexLoginEnd?: CodexLoginFail | 'dup';
  /** How long a ChatGPT sign-in waits for the browser before it lands. */
  codexLoginDelayMs?: number;
  /** "Kiểm tra" on a new provider: no answer, or the key refused (also an address with `down` in it, a key starting `sk-bad`). */
  codexProviderDown?: boolean;
  codexKeyRejected?: boolean;
  /** "Nhập file đăng nhập (JSON)": the owner closes the picker without a file. */
  codexImportNone?: boolean;
  /** The History tab's lines; none by default. */
  history?: HistoryEntry[];
  // @private:begin
  /** What a `quota-pool.last` set in Cài đặt reads as; without it a path finds no file. */
  firstmate?: FirstmateView;
  // @private:end
  /** macOS waits for the owner's approval after the login switch is turned on. */
  loginApproval?: boolean;
  // @private:begin
  /** How "Cập nhật từ mã nguồn" ends; by default it installs 0.1.1 from a new commit. */
  update?: () => UpdateOutcome | UpdateError;
  /** How long each update step takes; the preview waits to show them. */
  updateDelayMs?: number;
  // @private:end
  /** A write finds a file changed since the preview: nothing is written. */
  mcpChanged?: boolean;
  /** How long an MCP preview or write takes. */
  mcpDelayMs?: number;
  /** A restore finds Codex's file changed since its preview: nothing is written. */
  restoreChanged?: boolean;
  /** The undo of a restore finds Codex's file changed since: it asks first. */
  restoreUndoChanged?: boolean;
  /** How long a restore takes; the preview waits to show the running step. */
  restoreDelayMs?: number;
  /** A restore's formatter plugin needs a login: "Thử lại" gets it done. */
  restorePartial?: boolean;
  /** A skills write finds a folder changed since the preview: nothing is written. */
  skillsChanged?: boolean;
  /** How long `npx skills` and a skills write take. */
  skillsDelayMs?: number;
  /** Skills `npx skills update` fails for, in "Cập nhật tất cả" and "Cập nhật cả bộ". */
  skillsUpdateFail?: string[];
  // @private:begin
  /** Chép sang Mac khác (M13): Macs that do not answer, run version 1, or have no app. */
  copyUnreach?: string[];
  copyOld?: string[];
  copyNoApp?: string[];
  /** A copy finds the other Mac's file changed since the comparison: nothing is written. */
  copyChanged?: boolean;
  /** How long a comparison, a copy and its undo take. */
  copyDelayMs?: number;
  // @private:end
  /** How long "Kiểm lại" takes (M14). */
  envDelayMs?: number;
  /** "Áp bộ" finds an agent's file changed since the preview (M15): nothing is written. */
  setsChanged?: Agent;
  /** How long a set's preview, apply and undo take. */
  setsDelayMs?: number;
  /** Thống kê (M16): the logs skipped, as the report says them. */
  statsSkipped?: StatsSkipped[];
  /** "Đọc lại" walks a first read's progress, a step this often. */
  statsStepMs?: number;
  /** The report cannot be read. */
  statsFails?: boolean;
}

const said = (title: string, body: string): AlertSaid => ({ tone: 'info', title, body });

/** The sample "Gửi thử" sends: modelled on the Rust side's `alerts::sample`, with shorter words. */
function sample(kind: AlertKind, at: number): Alert {
  const base = { key: 'test:' + kind + ':' + at, kind, at, test: true };
  switch (kind) {
    case 'pool': {
      let pool = { title: 'Hạn mức Claude sắp cạn', body: 'Thử: tài khoản đang dùng còn 8%.', to: 'Sang tài khoản 3 (còn 19%)' };
      // @private:begin
      pool = { title: 'Hạn mức Claude trong vòng sắp cạn', body: 'Thử: còn tương đương 0,4 tài khoản.', to: 'Sang Cá nhân (còn 19%)' };
      // @private:end
      return { ...base, tone: 'bad', title: pool.title, body: pool.body, actions: [{ id: 'switch:0', label: pool.to }, { id: 'dismiss', label: 'Bỏ qua' }] };
    }
    // @private:begin
    case 'leaving':
      return { ...base, tone: 'warn', title: 'Team A rời vòng sau 30 phút', body: 'Thử: còn 30 phút.', actions: [{ id: 'dismiss', label: 'Ổn' }, { id: 'switch:0', label: 'Chuyển ngay' }] };
    // @private:end
    case 'login':
      return { ...base, tone: 'warn', title: 'Team B cần đăng nhập lại', body: 'Thử: đăng nhập hết hạn sau 2 ngày.', actions: [{ id: 'snooze', label: 'Nhắc ngày mai' }, { id: 'dismiss', label: 'Đã biết' }] };
    case 'overwrite':
      return { ...base, tone: 'warn', title: 'Antigravity đổi về tài khoản cũ', body: 'Thử: Antigravity.app đổi về Ultra 1.', actions: [{ id: 'agy:quitFirst:0', label: 'Thoát app rồi chuyển lại' }, { id: 'agy:keepRunning:0', label: 'Chuyển lại' }, { id: 'agy:keep', label: 'Bỏ qua' }] };
    // @private:begin
    case 'firstmate':
      return { ...base, tone: 'info', title: 'Firstmate đang chờ hạn mức Claude', body: 'Thử: hạn mức trong vòng dưới ngưỡng.', actions: [] };
    // @private:end
    case 'skill':
      return { ...base, tone: 'info', title: '2 skill có bản mới', body: 'Thử: ui-polish, sql-helper. App không tự cập nhật.', actions: [{ id: 'skills:open', label: 'Xem' }, { id: 'dismiss', label: 'Để sau' }] };
    case 'env':
      return { ...base, tone: 'bad', title: 'ANTHROPIC_API_KEY đang đè đăng nhập Claude Code', body: 'Thử: đặt ở ~/.zshrc dòng 42. App không đọc giá trị.', actions: [{ id: 'env:settings', label: 'Xem' }, { id: 'env:ignore:ANTHROPIC_API_KEY', label: 'Bỏ qua' }] };
    case 'cli':
      return { ...base, tone: 'info', title: 'Codex CLI có bản 0.164.0', body: 'Thử: Codex CLI 0.163.0 → 0.164.0. App chỉ cập nhật khi bạn bấm.', actions: [{ id: 'cli:settings', label: 'Xem' }, { id: 'dismiss', label: 'Để sau' }] };
  }
}

const unanswered = (recent: AlertRecord[]) => recent.filter((r) => !r.alert.test && !r.answer && r.alert.actions.length).length;
/** The bell's count: the cards that came since its panel was last opened. */
const unread = (recent: AlertRecord[]) => recent.filter((r) => !r.read).length;

/** The made-up email of a fake account. */
export function fakeEmail(number: number): string {
  return number === 1 ? 'lanpham.ultra@example.com' : 'lan.ultra' + number + '@example.com';
}

/** The snapshot without names, as Rust sends it when the owner hides them. */
function withoutNames(m: Model): Model['snapshot'] {
  const strip = (s: Model['snapshot']['claude']) => ({
    ...s,
    accounts: s.accounts.map(({ email: _e, org: _o, ...a }) => a),
  });
  return { ...m.snapshot, claude: strip(m.snapshot.claude), agy: strip(m.snapshot.agy) };
}

/** The model as Rust sends it with names hidden: no email or organization anywhere. */
export function hideNames(m: Model): Model {
  return {
    ...m,
    showNames: false,
    snapshot: withoutNames(m),
    antigravity: m.antigravity && { ...m.antigravity, accounts: m.antigravity.accounts.map((a) => ({ ...a, email: null, name: null })) },
    claudeAccounts: { ...m.claudeAccounts, accounts: m.claudeAccounts.accounts.map((a) => ({ ...a, email: null })) },
  };
}

// @private:begin
/** RFC 3339 cut to the minute, as the pause flag keeps it. */
function minute(iso: string): string {
  const d = new Date(iso);
  d.setSeconds(0, 0);
  return d.toISOString();
}
// @private:end

// @private:begin
/** The stamp's form: this Mac's offset, to the second. */
function stampAt(d: Date): string {
  const p = (n: number) => String(Math.abs(n)).padStart(2, '0');
  const off = -d.getTimezoneOffset();
  return (
    d.getFullYear() + '-' + p(d.getMonth() + 1) + '-' + p(d.getDate()) + 'T' + p(d.getHours()) + ':' + p(d.getMinutes()) + ':' + p(d.getSeconds()) +
    (off < 0 ? '-' : '+') + p(Math.trunc(off / 60)) + ':' + p(off % 60)
  );
}
// @private:end

/** The model of a build with the Antigravity accounts, as the fixtures are. */
type Full = Model &
  // @private:begin
  AuthorModel &
  // @private:end
  Required<Pick<Model, 'antigravity' | 'agySwitch'>>;

export function fakeApi(initial: Model, initialTab: Tab = 'overview', options: FakeOptions = {}) {
  let model = initial as Full;
  // The names Rust would send again when the owner shows them.
  const named = initial.snapshot;
  let tab = initialTab;
  let settingsGroup: SettingsGroup = 'general';
  // @private:begin
  let version = 1;
  // @private:end
  const now = options.clock ?? (() => new Date());
  const listeners = new Set<(m: Model) => void>();
  const tabListeners = new Set<(t: Tab) => void>();
  // @private:begin
  const closeListeners = new Set<() => void>();
  // @private:end
  const openSwitchListeners = new Set<(slot: number) => void>();
  // @private:begin
  const quitListeners = new Set<() => void>();
  // @private:end
  const materialListeners = new Set<(on: boolean) => void>();
  let reduceTransparency = options.reduceTransparency ?? false;
  const contrastListeners = new Set<(on: boolean) => void>();
  let increaseContrast = options.increaseContrast ?? false;
  // @private:begin
  /** What the last `closeMain` did: the tests check a quit really quits. */
  let closed: 'window' | 'app' | null = null;
  let quitPending = options.quitPending ?? false;
  let relaunched = 0;
  // @private:end
  let session: SessionView = {
    on: false,
    step: null,
    done: [],
    restarted: false,
    zip: null,
    claudeProgram: null,
    exe: null,
    ...options.session,
  };
  /** What the guide asked of the session, in order. */
  const sessionCalls: { call: string; id?: string; captures?: string[]; skipped?: boolean; facts?: unknown; checks?: string[] }[] = [];
  let lastMcpUndo: McpOutcome['undo'] = null;
  let lastSkillUndo: SkillOutcome['undo'] = null;
  /** The last apply's undo, and the set deleted last. */
  let lastSetUndo: { mcp: McpOutcome['undo']; skills: SkillOutcome['undo']; codexBefore: string | null } | null = null;
  let removedSet: { set: SetsView['sets'][number]; index: number; applied: SetsView['applied'] } | null = null;
  const setsWait = () => (options.setsDelayMs ? new Promise((r) => setTimeout(r, options.setsDelayMs)) : Promise.resolve());
  // @private:begin
  const delay = () => (options.saveDelayMs ? new Promise((r) => setTimeout(r, options.saveDelayMs)) : Promise.resolve());
  /** Every Mac SSH reaches takes the file; the others wait 5 minutes. */
  const sent = (peers: PeerView[]): PeerView[] =>
    peers.map((p) => ({
      ...p,
      current: true,
      pushing: false,
      theirRules: null,
      state: options.unreachable?.includes(p.alias)
        ? { kind: 'unreachable', error: 'ssh: connect to host ' + p.alias + ' port 22: Operation timed out', retryAt: new Date(now().getTime() + 300000).toISOString() }
        : { kind: 'synced', at: now().toISOString() },
    }));
  const write = async (base: string | null, next: (r: RotationView) => Partial<RotationView>) => {
    await delay();
    if (options.rulesError) throw options.rulesError;
    if (base !== model.rotation.version) throw { kind: 'changed', detail: null, restored: false } satisfies RulesError;
    version += 1;
    const r = model.rotation;
    push({ ...model, rotation: { ...r, status: 'ok', reason: null, version: 'v' + version, peers: sent(r.peers), ...next(r) } });
    return model;
  };
  // @private:end
  const push = (next: Model) => {
    model = next as Full;
    listeners.forEach((l) => l(model));
  };
  /** The accounts renumbered from 1, emails filled in or hidden as the switch says. */
  const accounts = (list: AgyLogin[], show: boolean): AgyLogin[] =>
    list.map((a, i) => ({ ...a, number: i + 1, email: show ? fakeEmail(i + 1) : null, name: show ? 'Người thử ' + (i + 1) : null }));
  const agy = (over: Partial<Model['antigravity']>) => ({ ...model.antigravity, ...over });
  /** Ends the waiting login: with a paste, a cancel or its own end. */
  let endLogin: ((how: 'paste' | 'cancel' | 'self') => void) | null = null;
  /** Ends the waiting Codex sign-in: its own end, or "Thôi". */
  let codexLoginEnd: ((how: 'self' | 'cancel') => void) | null = null;
  // @private:begin
  let lastCopy: CopyComparison | null = null;
  // @private:end
  /** The pages "Tải và cài", "Mở trang Releases" and the bug form opened. */
  const released: string[] = [];
  const finished: string[] = [];
  /** "Bỏ qua" keeps what it hid, for "Hoàn tác"; "Mở file" what it opened. */
  const envHidden = new Map<string, EnvFound>();
  const envOpened: string[] = [];
  let statsSkipped = structuredClone(options.statsSkipped ?? SKIPPED_NONE);
  /** One Claude account action: running, then landed (`change`) or failed (`claudeFail`). */
  const claudeRun = async (
    action: ClaudeAccountAction,
    change: (v: ClaudeAccountsView) => { view: Partial<ClaudeAccountsView>; done?: Partial<ClaudeActionDone>; model?: Partial<Model> } | string,
  ) => {
    push({ ...model, claudeAccounts: { ...model.claudeAccounts, working: action, last: null } });
    if (options.claudeDelayMs) await new Promise((r) => setTimeout(r, options.claudeDelayMs));
    const base: ClaudeActionDone = { action, ok: true, error: null, detail: null, slot: null, renewed: false, imported: null };
    const failed = options.claudeFail?.(action) ?? null;
    const landed = failed ? failed : change(model.claudeAccounts);
    if (typeof landed === 'string') {
      push({ ...model, claudeAccounts: { ...model.claudeAccounts, working: null, last: { ...base, ok: false, error: landed } } });
    } else {
      const next = { ...model, ...landed.model };
      push({
        ...next,
        snapshot: landed.view.accounts ? keptSnapshot(next.snapshot, landed.view.accounts) : next.snapshot,
        claudeAccounts: { ...model.claudeAccounts, ...landed.view, working: null, last: { ...base, ...landed.done } },
      });
    }
    return model;
  };
  /** The quota rows follow the kept accounts, as the app's list does: a new one has no numbers yet. */
  const keptSnapshot = (snap: Model['snapshot'], kept: ClaudeAccountRow[]): Model['snapshot'] => {
    const had = new Map(snap.claude.accounts.map((a) => [a.number, a]));
    const accounts = kept.map(
      (r): Account =>
        had.has(r.slot)
          ? {
              ...had.get(r.slot)!,
              // @private:begin
              disabled: !r.enabled,
              // @private:end
            }
          : {
              number: r.slot,
              // @private:begin
              role: 'unknown',
              disabled: !r.enabled,
              membership: 'unknown',
              leavesAt: null,
              // @private:end
              active: r.active,
              reading: 'noData',
              fetchedAt: null,
              source: 'list',
              lines: [],
              windows: [],
              ...(r.email ? { email: r.email } : {}),
            },
    );
    return { ...snap, claude: { ...snap.claude, accounts } };
  };
  const claudeRow = (slot: number, over: Partial<ClaudeAccountRow> = {}): ClaudeAccountRow => ({
    slot,
    alias: null,
    // @private:begin
    enabled: true,
    // @private:end
    active: false,
    kind: 'login',
    status: 'noData',
    fetchedAt: null,
    ageSeconds: null,
    windows: { fiveHour: null, sevenDay: null, fable: null },
    error: null,
    email: model.showNames ? 'lan.new' + slot + '@example.com' : null,
    ...over,
  });
  const nextSlot = (v: ClaudeAccountsView) => Math.max(0, ...v.accounts.map((a) => a.slot)) + 1;
  const editRow = (slot: number, f: (r: ClaudeAccountRow) => ClaudeAccountRow) => (v: ClaudeAccountsView) =>
    v.accounts.some((a) => a.slot === slot)
      ? { view: { accounts: v.accounts.map((a) => (a.slot === slot ? f(a) : a)) }, done: { slot } }
      : 'unknownSlot';
  const api: Api = {
    getModel: async () => model,
    refresh: async () => {
      const asked = model.reads;
      setTimeout(() => push({ ...model, reads: asked + 1 }), 300);
      return asked;
    },
    openWindow: async (next) => {
      tab = next;
      tabListeners.forEach((l) => l(tab));
    },
    mainTab: async () => tab,
    settingsGroup: async () => settingsGroup,
    setSettingsGroup: async (group) => {
      settingsGroup = group;
    },
    hidePopover: async () => {},
    resizePopover: async () => {},
    setMachineName: async (name) => {
      push({ ...model, machine: name.trim() || model.machineDefault });
      return model;
    },
    grantKeychain: async () => {
      push({ ...model, snapshot: { ...model.snapshot, keychain: 'granted' } });
      return model;
    },
    claudeSessions: async () => (options.sessions === undefined ? { live: 5, unreadable: 0 } : options.sessions),
    switchClaude: async (
      slot,
      // @private:begin
      holdUntil,
      // @private:end
    ) => {
      push({ ...model, claudeSwitch: { ...model.claudeSwitch, busy: slot } });
      if (options.switchDelayMs) await new Promise((r) => setTimeout(r, options.switchDelayMs));
      const outcome: Outcome = options.outcome?.(
        slot,
        // @private:begin
        holdUntil,
        // @private:end
      ) ?? {
        kind: 'switched',
        slot,
        // @private:begin
        hold: holdUntil ? { until: minute(holdUntil), failed: false } : null,
        // @private:end
      };
      const lands = outcome.kind === 'switched' || outcome.kind === 'unconfirmed' ? slot : outcome.kind === 'raced' ? outcome.active : null;
      // @private:begin
      const hold = outcome.kind !== 'failed' && outcome.hold && !outcome.hold.failed ? outcome.hold : null;
      // @private:end
      const accounts = model.snapshot.claude.accounts.map((a) => (lands == null ? a : { ...a, active: a.number === lands }));
      push({
        ...model,
        snapshot: { ...model.snapshot, claude: { ...model.snapshot.claude, accounts } },
        claudeSwitch: { ...model.claudeSwitch, busy: null },
        // @private:begin
        switcher: hold
          ? { ...model.switcher, pause: { until: hold.until, reason: 'chuyển tay sang tài khoản ' + slot + ' lúc 14:20', holds: true } }
          : model.switcher,
        // @private:end
      });
      return outcome;
    },
    claudeImport: () =>
      claudeRun('import', () => {
        const ready = claudeAccountsReady();
        return {
          view: { ready: true, accounts: ready },
          done: {
            imported: {
              added: ready.map((a) => a.slot),
              updated: [],
              unchanged: [],
              skipped: [],
              // @private:begin
              rotation: true,
              prime: false,
              // @private:end
            },
          },
          model: {
            claudeSwitch: { ...model.claudeSwitch, ready: true },
            // @private:begin
            switcher: { ...model.switcher, installed: true },
            // @private:end
          },
        };
      }),
    claudeAddLogin: () =>
      claudeRun('login', (v) => {
        if (options.claudeRenews != null) return { view: {}, done: { slot: options.claudeRenews, renewed: true } };
        const slot = nextSlot(v);
        return { view: { ready: true, accounts: [...v.accounts, claudeRow(slot)] }, done: { slot } };
      }),
    claudeCancelLogin: async () => {
      if (model.claudeAccounts.working === 'login') options.claudeFail = () => 'cancelled';
    },
    claudeAddCurrent: () =>
      claudeRun('current', (v) => {
        if (options.claudeRenews != null) return { view: {}, done: { slot: options.claudeRenews, renewed: true } };
        const slot = nextSlot(v);
        return { view: { ready: true, accounts: [...v.accounts, claudeRow(slot, { active: true })] }, done: { slot } };
      }),
    claudeAddToken: (token, email) =>
      claudeRun('token', (v) => {
        if (!token.trim() || /\s/.test(token.trim())) return 'badToken';
        const slot = nextSlot(v);
        const row = claudeRow(slot, { kind: 'setupToken', email: model.showNames ? email || 'setup-token-' + slot + '@token.local' : null });
        return { view: { ready: true, accounts: [...v.accounts, row] }, done: { slot } };
      }),
    claudeRemove: (slot) =>
      claudeRun('remove', (v) =>
        v.accounts.find((a) => a.slot === slot)?.active ? 'active' : { view: { accounts: v.accounts.filter((a) => a.slot !== slot) }, done: { slot } },
      ),
    claudeSetAlias: (slot, alias) => claudeRun('alias', editRow(slot, (r) => ({ ...r, alias: alias.trim() || null }))),
    // @private:begin
    claudeSetEnabled: (slot, enabled) => claudeRun('enabled', editRow(slot, (r) => ({ ...r, enabled }))),
    claudeSetRotation: (on) =>
      claudeRun('rotation', (v) =>
        on && v.legacy
          ? 'legacy'
          : { view: { rotation: on }, model: { switcher: { ...model.switcher, service: on } } },
      ),
    pauseSwitching: async (until) => {
      push({
        ...model,
        switcher: {
          ...model.switcher,
          pause: { until: until && minute(until), reason: 'tạm dừng trong Quota Station lúc 14:20', holds: true },
        },
      });
      return model;
    },
    extendPause: async (minutes) => {
      const p = model.switcher.pause;
      if (!p?.until) throw new Error('noEnd');
      const until = new Date(new Date(p.until).getTime() + minutes * 60000).toISOString();
      push({ ...model, switcher: { ...model.switcher, pause: { ...p, until } } });
      return model;
    },
    resumeSwitching: async () => {
      push({ ...model, switcher: { ...model.switcher, pause: null } });
      return model;
    },
    saveRotation: (rules, base) => write(base, () => ({ rules, stamp: { at: stampAt(now()), machine: model.machine } })),
    adoptRotation: (alias, base) => {
      const peer = model.rotation.peers.find((p) => p.alias === alias);
      // Stamped again by this Mac, as the Rust side does, so it goes out as the newest copy.
      return write(base, (r) => ({ rules: peer?.theirRules ?? r.rules, stamp: { at: stampAt(now()), machine: model.machine } }));
    },
    keepRotation: (base) => write(base, () => ({ stamp: { at: stampAt(now()), machine: model.machine } })),
    retryRotation: async () => {
      // A newer or hand-edited copy waits for the owner, as on the Rust side.
      const waits = (p: PeerView) => p.current && (p.state?.kind === 'newer' || p.state?.kind === 'edited');
      const tried = sent(model.rotation.peers);
      push({ ...model, rotation: { ...model.rotation, peers: model.rotation.peers.map((p, i) => (waits(p) ? p : tried[i])) } });
      return model;
    },
    setPeers: async (peers) => {
      if (peers.length > 8) throw 'tooMany';
      const bad = peers.findIndex((p) => !/^[A-Za-z0-9][A-Za-z0-9._@-]*$/.test(p.alias.trim()));
      if (bad >= 0) throw 'badAlias:' + bad;
      const lower = peers.map((p) => p.alias.trim().toLowerCase());
      const same = lower.findIndex((a, i) => lower.indexOf(a) < i);
      if (same >= 0) throw 'sameAlias:' + same;
      const known = new Map(model.rotation.peers.map((p) => [p.alias, p]));
      const next = peers.map((p): PeerView => {
        const alias = p.alias.trim();
        const was = known.get(alias);
        return { ...(was ?? { state: null, current: false, theirRules: null, pushing: false }), name: p.name.trim() || alias, alias };
      });
      push({ ...model, rotation: { ...model.rotation, peers: next } });
      return model;
    },
    testPeer: async (alias) =>
      options.unreachable?.includes(alias)
        ? { reached: false, error: 'timeout', hasFile: false, stamp: null }
        : { reached: true, error: null, hasFile: true, stamp: model.rotation.stamp },
    previewRotation: async (rules) => {
      const active = model.snapshot.claude.accounts.find((a) => a.active)?.number ?? 0;
      if (rules.team.includes(active) || rules.personal === active) return { decision: { kind: 'stay' }, raw: 'stay' };
      const next = rules.team[0];
      return { decision: { kind: 'switch', slot: next, why: { kind: 'room', used: 0, ends: null } }, raw: 'switch ' + next + ' (0.0%, window not started)' };
    },
    rulesDirty: async () => {},
    closeMain: async (quit) => {
      closed = quit ? 'app' : 'window';
    },
    onCloseRequested: async (handler) => {
      closeListeners.add(handler);
      return () => closeListeners.delete(handler);
    },
    takeQuit: async () => {
      const asked = quitPending;
      quitPending = false;
      return asked;
    },
    onQuitRequested: async (handler) => {
      quitListeners.add(handler);
      return () => quitListeners.delete(handler);
    },
    // @private:end
    agyLogin: async () => {
      if (model.antigravity.loggingIn) throw { kind: 'busy', detail: '' } satisfies AgyLoginFailure;
      if (model.agySwitch.busy != null) throw { kind: 'switching', detail: '' } satisfies AgyLoginFailure;
      if (model.antigravity.client !== 'ok') throw { kind: 'client', detail: model.antigravity.client } satisfies AgyLoginFailure;
      push({ ...model, antigravity: agy({ loggingIn: true }) });
      const how = await new Promise<'paste' | 'cancel' | 'self'>((resolve) => {
        endLogin = resolve;
        if (options.agyLoginDelayMs != null) setTimeout(() => resolve('self'), options.agyLoginDelayMs);
      });
      endLogin = null;
      const failure: AgyLoginFailure | null =
        how === 'cancel' ? { kind: 'cancelled', detail: '' } : how === 'self' ? (options.agyLogin?.() ?? null) : null;
      if (failure) {
        push({ ...model, antigravity: agy({ loggingIn: false }) });
        throw failure;
      }
      // A login of an account that needs one answers it; otherwise the next account is added.
      const list = model.antigravity.accounts;
      const again = list.find((a) => a.status !== 'ok');
      const next = again
        ? list.map((a) => (a.id === again.id ? { ...a, status: 'ok' as const, verify: false } : a))
        : [
            ...list,
            {
              id: 'antigravity_fake' + (list.length + 1),
              number: 0,
              email: null,
              name: null,
              status: 'ok' as const,
              verify: false,
              reading: false,
              readError: null,
            },
          ];
      push({ ...model, antigravity: agy({ loggingIn: false, accounts: accounts(next, model.showNames) }) });
      const done: AgyLoginDone = again ? { number: again.number, added: false } : { number: next.length, added: true };
      return done;
    },
    agyLoginCallback: async (url) => {
      if (!endLogin) throw new Error('noLogin');
      if (!url.includes('code=')) throw new Error('noCode');
      if (!url.includes('state=')) throw new Error('noState');
      if (url.includes('state=wrong')) throw new Error('stateMismatch');
      endLogin('paste');
    },
    agyLoginCancel: async () => {
      endLogin?.('cancel');
    },
    agyLogout: async (id) => {
      const list = model.antigravity.accounts;
      if (!list.some((a) => a.id === id)) throw new Error('notFound');
      push({ ...model, antigravity: agy({ accounts: accounts(list.filter((a) => a.id !== id), model.showNames) }) });
      return model;
    },
    agyOpenVerification: async () => {},
    agyTabOpened: async () => {},
    agyAppState: async () => {
      if (options.agyApp === null) throw new Error('spawn');
      return options.agyApp ?? { installed: 'supported', version: '2.24.0', running: true };
    },
    switchAgy: async (id, how) => {
      if (model.agySwitch.busy != null) throw 'busy';
      if (model.antigravity.loggingIn) throw 'loggingIn';
      const login = model.antigravity.accounts.find((a) => a.id === id);
      if (!login) throw 'unknownAccount';
      const number = login.number;
      push({ ...model, agySwitch: { ...model.agySwitch, busy: number } });
      if (options.agySwitchDelayMs) await new Promise((r) => setTimeout(r, options.agySwitchDelayMs));
      const outcome: AgyOutcome = options.agyOutcome?.(number, how) ?? {
        kind: 'switched',
        number,
        how,
        basis: 'reading',
        reopened: how === 'quitFirst' ? true : null,
        saved: true,
      };
      const lands = outcome.kind === 'switched' ? number : outcome.kind === 'unconfirmed' ? outcome.active : null;
      const g = model.snapshot.agy;
      push({
        ...model,
        snapshot: { ...model.snapshot, agy: { ...g, accounts: g.accounts.map((a) => (lands == null ? a : { ...a, active: a.number === lands })) } },
        agySwitch: { ...model.agySwitch, busy: null, overwritten: outcome.kind === 'failed' ? model.agySwitch.overwritten : null },
      });
      return outcome;
    },
    codexSwitch: async (target, base) => {
      const c = model.codex;
      push({ ...model, codex: { ...c, busy: 'switching' } });
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, options.codexDelayMs));
      const from = c.current === 'custom' && c.custom ? c.custom.id : c.current === 'other' && c.other ? c.other : 'openai';
      const to = target === 'custom' ? (c.custom?.id ?? 'custom') : 'openai';
      const requires = target === 'custom' ? c.wantsRequires : null;
      const outcome: CodexOutcome =
        options.codexOutcome?.(target) ??
        (base !== c.hash
          ? { kind: 'failed', target, error: { kind: 'changed' } }
          : { kind: 'switched', target, from, to, backup: 'config.toml.20260116-142040' });
      const at = now().toISOString();
      push({
        ...model,
        codex:
          outcome.kind === 'switched'
            ? {
                ...c,
                busy: null,
                current: target === 'custom' ? 'custom' : 'chatgpt',
                custom: c.custom && requires != null ? { ...c.custom, requiresOpenaiAuth: requires } : c.custom,
                keyUndo: null,
                hash: c.hash + "'",
                external: null,
                lastWrite: { at, provider: to, matches: true },
                backups: [at, ...c.backups].slice(0, 10),
                recent: [{ at, from, to, result: from === to ? 'plugins' : 'switched', restarted: null, requires }, ...c.recent].slice(0, 5),
              }
            : { ...c, busy: null },
      });
      return outcome;
    },
    codexRestart: async (afterSwitch) => {
      push({ ...model, codex: { ...model.codex, busy: 'restarting' } });
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, options.codexDelayMs));
      const out: CodexRestart = options.codexRestart ?? (model.codex.procs?.daemon ? { kind: 'restarted' } : { kind: 'notRunning' });
      const recent = afterSwitch && model.codex.recent[0] ? [{ ...model.codex.recent[0], restarted: out.kind === 'restarted' }, ...model.codex.recent.slice(1)] : model.codex.recent;
      push({ ...model, codex: { ...model.codex, busy: null, recent } });
      return out;
    },
    codexKeep: async () => {
      push({ ...model, codex: { ...model.codex, external: null } });
      return model;
    },
    codexCheck: async () => {},
    codexKeyPreview: async (key, baseUrl) => {
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, 300));
      return fakeKeyPreview(model.codex, key, baseUrl);
    },
    codexSetKey: async (key, baseUrl, base) => {
      const c = model.codex;
      if (c.busy) throw 'busy';
      const p = fakeKeyPreview(c, key, baseUrl);
      if (p.refusal) return { kind: 'refused', refusal: p.refusal } satisfies CodexKeyOutcome;
      push({ ...model, codex: { ...c, busy: 'switching' } });
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, options.codexDelayMs));
      if (base !== c.hash) {
        push({ ...model, codex: { ...c, busy: null } });
        return { kind: 'failed', error: { kind: 'changed' } } satisfies CodexKeyOutcome;
      }
      const at = now().toISOString();
      const id = c.custom!.id;
      const keyUndo = p.undo ? { at, token: p.undo, baseUrl: !!baseUrl } : null;
      push({
        ...model,
        codex: {
          ...c,
          busy: null,
          hash: c.hash + 'k',
          custom: { ...c.custom!, key: 'bearer', baseUrl: baseUrl || c.custom!.baseUrl, proxy: baseUrl ? false : c.custom!.proxy },
          refusal: c.refusal?.kind === 'noKey' || c.refusal?.kind === 'placeholder' ? null : c.refusal,
          wantsRequires: c.login === 'chatGpt' && c.plugins,
          keyUndo,
          backups: [at, ...c.backups].slice(0, 10),
          recent: [{ at, from: id, to: id, result: 'key', restarted: null }, ...c.recent].slice(0, 5),
        },
      });
      return { kind: 'written', provider: id, backup: 'config.toml.20260116-142040', undo: !!keyUndo } satisfies CodexKeyOutcome;
    },
    codexUndoKey: async (base) => {
      const c = model.codex;
      if (!c.keyUndo) return { kind: 'refused', refusal: { kind: 'noUndo' } } satisfies CodexKeyOutcome;
      if (base !== c.hash) return { kind: 'failed', error: { kind: 'changed' } } satisfies CodexKeyOutcome;
      const at = now().toISOString();
      const id = c.custom!.id;
      const key = c.keyUndo.token === 'placeholder' ? 'placeholder' : 'none';
      push({
        ...model,
        codex: {
          ...c,
          hash: c.hash + 'u',
          custom: { ...c.custom!, key },
          refusal: c.current === 'custom' ? c.refusal : { kind: key === 'placeholder' ? 'placeholder' : 'noKey' },
          wantsRequires: null,
          keyUndo: null,
          recent: [{ at, from: id, to: id, result: 'keyUndo', restarted: null }, ...c.recent].slice(0, 5),
        },
      });
      return { kind: 'undone', provider: id, backup: 'config.toml.20260116-142100' } satisfies CodexKeyOutcome;
    },
    codexUse: async (id, base) => {
      const c = model.codex;
      if (c.busy) throw 'busy';
      const to = c.accounts.find((a) => a.id === id);
      const target: CodexTarget = to?.kind === 'custom' ? 'custom' : 'chatGpt';
      push({ ...model, codex: { ...c, busy: 'switching' } });
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, options.codexDelayMs));
      const from = c.accounts.find((a) => a.active)?.id ?? 'openai';
      const outcome: CodexOutcome =
        options.codexOutcome?.(target) ??
        (!to
          ? { kind: 'refused', target, refusal: { kind: 'noCustom' } }
          : base !== c.hash
            ? { kind: 'failed', target, error: { kind: 'changed' } }
            : { kind: 'switched', target, from, to: id, backup: 'config.toml.20260116-142040' });
      if (outcome.kind !== 'switched' || !to) {
        push({ ...model, codex: { ...c, busy: null } });
        return outcome;
      }
      const at = now().toISOString();
      const accounts = c.accounts.map((a) => ({ ...a, active: a.id === id, signedIn: to.kind === 'chatgpt' ? a.id === id : a.signedIn }));
      const s = model.snapshot.codex;
      push({
        ...model,
        snapshot: { ...model.snapshot, codex: { ...s, accounts: s.accounts.map((q) => ({ ...q, active: to.kind === 'chatgpt' && q.number === to.number })) } },
        codex: {
          ...c,
          busy: null,
          accounts,
          current: to.kind === 'custom' ? 'custom' : 'chatgpt',
          custom: to.kind === 'custom' ? to.custom : c.custom,
          reach: to.kind === 'custom' && to.reach ? to.reach : c.reach,
          refusal: to.kind === 'custom' ? to.refusal : c.refusal,
          login: 'chatGpt',
          hash: c.hash + "'",
          external: null,
          lastWrite: { at, provider: to.kind === 'custom' ? id : 'openai', matches: true },
          recent: [{ at, from, to: id, result: 'switched', restarted: null }, ...c.recent].slice(0, 5),
        },
      });
      return outcome;
    },
    codexLogin: async (id, device) => {
      const c = model.codex;
      if (c.busy || codexLoginEnd) return { kind: 'refused', reason: 'busy' } satisfies CodexAccountDone;
      // A device code shows while it waits, as `codex login --device-auth` prints one.
      if (device) push({ ...model, codex: { ...model.codex, loginCode: { url: 'https://auth.openai.com/codex/device', code: 'QSFX-27K4D' } } });
      const ended = await new Promise<'self' | 'cancel'>((resolve) => {
        codexLoginEnd = resolve;
        setTimeout(() => resolve('self'), options.codexLoginDelayMs ?? 0);
      });
      codexLoginEnd = null;
      if (device) push({ ...model, codex: { ...model.codex, loginCode: null } });
      if (ended === 'cancel') return { kind: 'failed', reason: 'cancelled', detail: null } satisfies CodexAccountDone;
      const end = options.codexLoginEnd;
      if (end === 'dup') return { kind: 'dup', id: c.accounts.find((a) => a.kind === 'chatgpt' && !a.signedIn)?.id ?? 'chatgpt:1' } satisfies CodexAccountDone;
      if (end) return { kind: 'failed', reason: end, detail: end === 'other' ? 'exit 1: Error logging in: unexpected response' : null } satisfies CodexAccountDone;
      const at = now().toISOString();
      const s = model.snapshot.codex;
      const fresh = (n: number, email: string, left: [number, number]) => {
        const lines: Line[] = [
          { kind: 'session', percent: left[0], resetsAt: new Date(now().getTime() + 4.5 * 3600e3).toISOString(), pending: false, atLeast: false },
          { kind: 'week', percent: left[1], resetsAt: new Date(now().getTime() + 6 * 86400e3).toISOString(), pending: false, atLeast: false },
        ];
        return { number: n, role: 'unknown' as const, active: false, disabled: false, reading: 'fresh' as const, fetchedAt: at, source: 'list' as const, membership: 'unknown' as const, leavesAt: null, lines, windows: lines, email };
      };
      if (id) {
        const a = c.accounts.find((x) => x.id === id);
        if (!a) return { kind: 'failed', reason: 'other', detail: null } satisfies CodexAccountDone;
        push({
          ...model,
          snapshot: { ...model.snapshot, codex: { ...s, accounts: [...s.accounts.filter((q) => q.number !== a.number), fresh(a.number ?? 0, a.email ?? '', [100, 91])] } },
          codex: { ...c, accounts: c.accounts.map((x) => (x.id === id ? { ...x, login: 'ok' } : x)) },
        });
        return { kind: 'renewed', id } satisfies CodexAccountDone;
      }
      const n = Math.max(0, ...c.accounts.map((a) => a.number ?? 0)) + 1;
      const email = 'lan.chatgpt' + n + '@example.com';
      const added: CodexAccount = { id: 'chatgpt:' + n, kind: 'chatgpt', name: email.slice(0, email.indexOf('@')), nickname: false, number: n, email, plan: 'Plus', active: false, signedIn: !c.accounts.some((a) => a.signedIn), login: 'ok', custom: null, reach: null, refusal: null, addedAt: at };
      const gpts = c.accounts.filter((a) => a.kind === 'chatgpt');
      const rest = c.accounts.filter((a) => a.kind !== 'chatgpt');
      push({
        ...model,
        snapshot: { ...model.snapshot, codex: { ...s, status: 'ok', accounts: [...s.accounts, fresh(n, email, [100, 96])] } },
        codex: { ...c, login: 'chatGpt', accounts: [...gpts, added, ...rest] },
      });
      return { kind: 'added', id: added.id } satisfies CodexAccountDone;
    },
    codexCancelLogin: async () => {
      codexLoginEnd?.('cancel');
    },
    codexReopenLogin: async () => {},
    codexCheckProvider: async (baseUrl, key) => {
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, 900));
      if (!/^https?:\/\/[^\s@]+$/.test(baseUrl.trim())) return { reach: 'badUrl', ms: null, key: 'unknown', status: null } satisfies CodexProviderCheck;
      if (options.codexProviderDown || /down/i.test(baseUrl)) return { reach: 'down', ms: null, key: 'unknown', status: null } satisfies CodexProviderCheck;
      if (options.codexKeyRejected || key.trim().startsWith('sk-bad')) return { reach: 'up', ms: 230, key: 'rejected', status: 401 } satisfies CodexProviderCheck;
      return { reach: 'up', ms: 210, key: 'accepted', status: 200 } satisfies CodexProviderCheck;
    },
    codexProviderPreview: async (name, baseUrl, key) => {
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, 300));
      return fakeProviderPreview(model.codex, name, baseUrl, key);
    },
    codexAddProvider: async (name, baseUrl, key, base) => {
      const c = model.codex;
      if (c.busy) return { kind: 'refused', reason: 'busy' } satisfies CodexAccountDone;
      const p = fakeProviderPreview(c, name, baseUrl, key);
      if (p.refusal) return { kind: 'refused', reason: p.refusal.kind === 'name' ? 'name' : p.refusal.kind === 'badUrl' ? 'badUrl' : p.refusal.kind === 'empty' ? 'empty' : 'key' } satisfies CodexAccountDone;
      push({ ...model, codex: { ...c, busy: 'switching' } });
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, options.codexDelayMs));
      if (base !== c.hash) {
        push({ ...model, codex: { ...c, busy: null } });
        return { kind: 'refused', reason: 'changed' } satisfies CodexAccountDone;
      }
      const at = now().toISOString();
      const id = providerId(name);
      const custom: CodexCustom = { id, name: name.trim(), baseUrl: baseUrl.trim(), key: 'bearer', envKey: null, requiresOpenaiAuth: null, plain: true, proxy: false };
      const added: CodexAccount = { id, kind: 'custom', name: name.trim(), nickname: false, number: null, email: null, plan: null, active: false, signedIn: false, login: 'ok', custom, reach: { state: 'up', ms: 210, since: null, checkedAt: at }, refusal: null, addedAt: at };
      push({
        ...model,
        codex: {
          ...c,
          busy: null,
          config: 'ok',
          hash: (c.hash ?? 'h') + 'p',
          custom: c.custom ?? custom,
          refusal: c.custom ? c.refusal : null,
          accounts: [...c.accounts, added],
          backups: [at, ...c.backups].slice(0, 10),
          recent: [{ at, from: id, to: id, result: 'added', restarted: null }, ...c.recent].slice(0, 5),
        },
      });
      return { kind: 'added', id } satisfies CodexAccountDone;
    },
    codexRename: async (id, name) => {
      const c = model.codex;
      const a = c.accounts.find((x) => x.id === id);
      if (!a) return { kind: 'refused', reason: 'changed' } satisfies CodexAccountDone;
      const own = a.kind === 'custom' ? (a.custom?.name ?? a.id) : a.email ? a.email.slice(0, a.email.indexOf('@')) : 'ChatGPT ' + (a.number ?? '');
      const next = name.trim();
      push({ ...model, codex: { ...c, accounts: c.accounts.map((x) => (x.id === id ? { ...x, name: next || own, nickname: !!next } : x)) } });
      return { kind: 'renamed', id } satisfies CodexAccountDone;
    },
    codexRemove: async (id, logout, base) => {
      const c = model.codex;
      const a = c.accounts.find((x) => x.id === id);
      if (!a) return { kind: 'refused', reason: 'changed' } satisfies CodexAccountDone;
      if (a.active) return { kind: 'refused', reason: 'inUse' } satisfies CodexAccountDone;
      if (a.signedIn) return { kind: 'refused', reason: 'signedIn' } satisfies CodexAccountDone;
      if (a.kind === 'custom' && base !== c.hash) return { kind: 'refused', reason: 'changed' } satisfies CodexAccountDone;
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, 600));
      const s = model.snapshot.codex;
      const at = now().toISOString();
      const accounts = c.accounts.filter((x) => x.id !== id);
      const nextCustom = a.kind === 'custom' && c.custom?.id === id ? (accounts.find((x) => x.kind === 'custom')?.custom ?? null) : c.custom;
      push({
        ...model,
        snapshot: a.kind === 'chatgpt' ? { ...model.snapshot, codex: { ...s, accounts: s.accounts.filter((q) => q.number !== a.number) } } : model.snapshot,
        codex: {
          ...c,
          accounts,
          custom: nextCustom,
          hash: a.kind === 'custom' ? c.hash + 'r' : c.hash,
          backups: a.kind === 'custom' ? [at, ...c.backups].slice(0, 10) : c.backups,
        },
      });
      return { kind: 'removed', id, name: a.name, loggedOut: a.kind === 'chatgpt' && logout } satisfies CodexAccountDone;
    },
    codexImportPick: async () => {
      if (options.codexImportNone) return null;
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, 400));
      return fakeImportPreview(model.codex);
    },
    codexImportText: async (text) => {
      if (text.length > 1024 * 1024) throw 'large';
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, 400));
      return fakePastePreview(model.codex);
    },
    codexImport: async (token, removeSource) => {
      const c = model.codex;
      if (c.busy) return { kind: 'refused', reason: 'busy' } satisfies CodexAccountDone;
      if (token !== IMPORT_TOKEN && token !== PASTE_TOKEN) return { kind: 'refused', reason: 'expired' } satisfies CodexAccountDone;
      if (options.codexDelayMs) await new Promise((r) => setTimeout(r, 900));
      const at = now().toISOString();
      const pasted = token === PASTE_TOKEN;
      const items = (pasted ? fakePastePreview(c) : fakeImportPreview(c)).items;
      const s = model.snapshot.codex;
      let n = Math.max(0, ...c.accounts.map((a) => a.number ?? 0));
      const added: CodexAccount[] = items
        .filter((i) => i.state === 'new')
        .map((i) => {
          n += 1;
          const email = i.email ?? '';
          return { id: 'chatgpt:' + n, kind: 'chatgpt', name: email.slice(0, email.indexOf('@')), nickname: false, number: n, email, plan: i.plan, active: false, signedIn: false, login: 'idle', custom: null, reach: null, refusal: null, addedAt: at };
        });
      const renewed = items.filter((i) => i.state === 'renew').map((i) => i.id ?? '');
      const gpts = c.accounts.filter((a) => a.kind === 'chatgpt').map((a) => (renewed.includes(a.id) ? { ...a, login: 'idle' as const } : a));
      const rest = c.accounts.filter((a) => a.kind !== 'chatgpt');
      push({
        ...model,
        snapshot: { ...model.snapshot, codex: { ...s, accounts: [...s.accounts, ...added.map((a) => ({ number: a.number ?? 0, role: 'unknown' as const, active: false, disabled: false, reading: 'noData' as const, fetchedAt: at, source: 'list' as const, membership: 'unknown' as const, leavesAt: null, lines: [], windows: [], email: a.email ?? '' }))] } },
        codex: { ...c, accounts: [...gpts, ...added, ...rest] },
      });
      return { kind: 'imported', ids: [...added.map((a) => a.id), ...renewed], removed: removeSource && !pasted, pasted } satisfies CodexAccountDone;
    },
    codexTriggerSet: async (next) => {
      const c = model.codex;
      if (!c.trigger) return;
      // Refused as Rust refuses it: `at`, `same`, `empty` or `gap`.
      const kept = wakeTimes(next.times);
      if ('error' in kept) throw kept.error;
      const due = next.on && next.mode === 'daily' ? nextWake(kept.times, now()) : null;
      push({ ...model, codex: { ...c, trigger: { ...c.trigger, ...next, times: kept.times, next: due } } });
    },
    codexTriggerNow: async () => {
      const first = model.codex.trigger;
      if (!first || first.running) return;
      for (const id of first.ids) {
        const tr = model.codex.trigger!;
        push({ ...model, codex: { ...model.codex, trigger: { ...tr, running: id } } });
        await new Promise((r) => setTimeout(r, options.codexDelayMs ? 1200 : 0));
        const a = model.codex.accounts.find((x) => x.id === id);
        const at = now().toISOString();
        const run: CodexTriggerRun =
          a?.login === 'relogin'
            ? { at, id, why: 'now', result: 'relogin', resetsAt: null, detail: null }
            : { at, id, why: 'now', result: 'started', resetsAt: new Date(now().getTime() + 5 * 3600e3).toISOString(), detail: null };
        const t2 = model.codex.trigger!;
        push({ ...model, codex: { ...model.codex, trigger: { ...t2, running: null, runs: [run, ...t2.runs].slice(0, 8) } } });
      }
    },
    setCodexPlugins: async (on) => {
      const c = model.codex;
      const own = c.custom?.key === 'bearer' || c.custom?.key === 'envKey';
      const wantsRequires = c.wantsRequires == null ? null : own && (c.login === 'chatGpt' || c.login === 'keychain') && on;
      push({ ...model, codex: { ...c, plugins: on, wantsRequires } });
      return model;
    },
    mcpRead: async () => {
      push({ ...model, mcp: { ...model.mcp, busy: 'reading' } });
      if (options.mcpDelayMs) await new Promise((r) => setTimeout(r, options.mcpDelayMs));
      push({ ...model, mcp: { ...model.mcp, busy: null, readAt: now().toISOString() } });
    },
    mcpPreview: async (changes) => {
      if (options.mcpDelayMs) await new Promise((r) => setTimeout(r, options.mcpDelayMs));
      if (!model.mcp.table) throw 'notRead';
      return fakePreview(model.mcp.table, changes);
    },
    mcpWrite: async (changes, bases) => {
      if (model.mcp.busy === 'writing' || model.codex.busy) throw 'busy';
      const table = model.mcp.table;
      if (!table) throw 'notRead';
      const at = Math.floor(now().getTime() / 1000);
      if (options.mcpChanged) {
        const agent = (Object.keys(bases)[0] ?? 'codex') as Agent;
        return { done: [], failed: { agent, error: { kind: 'changed' } }, undo: null, at };
      }
      push({ ...model, mcp: { ...model.mcp, busy: 'writing' } });
      if (options.mcpDelayMs) await new Promise((r) => setTimeout(r, options.mcpDelayMs));
      const { table: next, outcome } = fakeApply(table, changes, at);
      const iso = new Date(at * 1000).toISOString();
      const backups = outcome.done.map((d) => ({ agent: d.agent, at: iso, file: d.backup ?? '', changes: changes.filter((c) => c.agent === d.agent).map((c) => c.name + ':' + c.to.kind) }));
      push({ ...model, mcp: { ...model.mcp, busy: null, table: next, readAt: iso, last: { at: iso, outcome, canUndo: !!outcome.undo }, backups: [...backups, ...model.mcp.backups] } });
      lastMcpUndo = outcome.undo;
      return outcome;
    },
    mcpUndo: async () => {
      const undo = lastMcpUndo;
      if (!undo || !model.mcp.table) throw 'nothing';
      lastMcpUndo = null;
      const at = Math.floor(now().getTime() / 1000);
      const { table, outcome } = fakeApply(model.mcp.table, undo.changes, at);
      push({ ...model, mcp: { ...model.mcp, table, last: model.mcp.last ? { ...model.mcp.last, canUndo: false } : null } });
      return { ...outcome, undo: null };
    },
    mcpSaveSecret: async (name, slot) => {
      const table = model.mcp.table;
      if (!table) return;
      const rows = table.rows.map((r) => {
        if (r.name !== name) return r;
        const secrets = r.secrets.map((s) => (s.id === slot ? { ...s, have: true } : s));
        const full = secrets.every((s) => s.have);
        const cells = Object.fromEntries(
          Object.entries(r.cells).map(([a, c]) => [a, full && c.lock === 'missingKey' ? { ...c, lock: null } : c]),
        ) as typeof r.cells;
        return { ...r, secrets, cells };
      });
      const stored = table.stored.map((k) => (k.name === name ? { ...k, missing: k.missing.filter((m) => m !== slot) } : k));
      push({ ...model, mcp: { ...model.mcp, table: { ...table, rows, stored } } });
    },
    mcpPaste: async (text) => fakePaste(text),
    mcpForget: async (name) => {
      const table = model.mcp.table;
      if (table) push({ ...model, mcp: { ...model.mcp, table: { ...table, stored: table.stored.filter((k) => k.name !== name) } } });
    },
    mcpOpenBackups: async () => {},
    skillsRead: async () => {
      push({ ...model, skills: { ...model.skills, busy: 'reading' } });
      if (options.skillsDelayMs) await new Promise((r) => setTimeout(r, options.skillsDelayMs));
      push({ ...model, skills: { ...model.skills, busy: null, readAt: now().toISOString() } });
    },
    skillsPreview: async (changes) => {
      if (!model.skills.table) throw 'notRead';
      return fakeSkillPreview(model.skills.table, changes);
    },
    skillsWrite: async (changes) => {
      if (model.skills.busy || model.mcp.busy === 'writing' || model.codex.busy) throw 'busy';
      const table = model.skills.table;
      if (!table) throw 'notRead';
      const at = Math.floor(now().getTime() / 1000);
      if (options.skillsChanged) return { done: [], failed: { agent: changes[0]?.agent ?? 'codex', error: { kind: 'changed' } }, undo: null, at };
      push({ ...model, skills: { ...model.skills, busy: 'writing' } });
      if (options.skillsDelayMs) await new Promise((r) => setTimeout(r, options.skillsDelayMs));
      const { table: next, outcome } = fakeSkillApply(table, changes, at);
      const iso = new Date(at * 1000).toISOString();
      lastSkillUndo = outcome.undo;
      push({ ...model, skills: { ...model.skills, busy: null, table: next, readAt: iso, last: { at: iso, kind: 'batch', names: changes.map((c) => c.name), outcome, canUndo: true } } });
      return outcome;
    },
    skillsUndo: async () => {
      const undo = lastSkillUndo;
      const table = model.skills.table;
      if (!undo || !table) throw 'nothing';
      lastSkillUndo = null;
      const { table: next } = fakeSkillApply(table, undo.changes, Math.floor(now().getTime() / 1000));
      push({ ...model, skills: { ...model.skills, table: next, last: model.skills.last ? { ...model.skills.last, canUndo: false } : null } });
      return 'undone';
    },
    skillsDropLink: async (agent, name) => {
      const table = model.skills.table;
      if (table) push({ ...model, skills: { ...model.skills, table: { ...table, broken: table.broken.filter((b) => !(b.agent === agent && b.name === name)) } } });
    },
    skillsListRepo: async (repo) => {
      if (!model.skills.npx) throw 'noNpx';
      if (options.skillsDelayMs) await new Promise((r) => setTimeout(r, options.skillsDelayMs));
      const r = repo.trim().replace(/^https:\/\/github\.com\//, '').replace(/\.git$/, '');
      if (!/^[\w.-]+\/[\w.-]+$/.test(r)) throw 'badRepo';
      if (r.endsWith('skils')) throw 'failed:Repository not found';
      return repoFixture(r);
    },
    skillsReadMd: async () => SKILL_MD,
    skillsInstall: async (repo, names, agents) => {
      if (model.skills.busy) throw 'busy';
      const table = model.skills.table;
      if (!table) throw 'notRead';
      push({ ...model, skills: { ...model.skills, busy: 'installing' } });
      if (options.skillsDelayMs) await new Promise((r) => setTimeout(r, options.skillsDelayMs));
      const at = Math.floor(now().getTime() / 1000);
      const cell = (a: 'claude' | 'codex' | 'agy') => ({ state: agents[a] ? ('on' as const) : ('off' as const), lock: null });
      const added = names.map((name) => ({
        name,
        source: 'github' as const,
        repo,
        home: null,
        cells: { claude: cell('claude'), codex: cell('codex'), agy: cell('agy') },
        edited: false,
        update: null,
        size: 12_000,
        importable: false,
      }));
      const rows = [...table.rows, ...added].sort((a, b) => a.name.localeCompare(b.name));
      const outcome: SkillOutcome = { done: [], failed: null, undo: null, at };
      const iso = new Date(at * 1000).toISOString();
      push({ ...model, skills: { ...model.skills, busy: null, table: { ...table, rows }, last: { at: iso, kind: 'install', names, outcome, canUndo: true } } });
      return { names, outcome, error: null };
    },
    skillsUpdate: async (name) => {
      const table = model.skills.table;
      if (!table) throw 'notRead';
      push({ ...model, skills: { ...model.skills, busy: 'updating' } });
      if (options.skillsDelayMs) await new Promise((r) => setTimeout(r, options.skillsDelayMs));
      const rows = table.rows.map((r) => (r.name === name ? { ...r, update: null, edited: false } : r));
      push({ ...model, skills: { ...model.skills, busy: null, table: { ...table, rows } } });
      return { name, backup: name + '.folder.20260116-142000', error: null };
    },
    skillsUpdateMany: async (names) => {
      const table = model.skills.table;
      if (!table) throw 'notRead';
      if (model.skills.busy) throw 'busy';
      const fail = options.skillsUpdateFail ?? [];
      const results = names.map((name) => ({ name, backup: name + '.folder.20260116-142000', error: fail.includes(name) ? { kind: 'failed' as const, detail: 'Skill ' + name + ' not found' } : null }));
      for (const [i, name] of names.entries()) {
        push({ ...model, skills: { ...model.skills, busy: 'updating', progress: { done: i, total: names.length, name } } });
        if (options.skillsDelayMs) await new Promise((r) => setTimeout(r, options.skillsDelayMs));
      }
      const ok = new Set(results.filter((r) => !r.error).map((r) => r.name));
      const rows = table.rows.map((r) => (ok.has(r.name) ? { ...r, update: null, edited: false } : r));
      push({ ...model, skills: { ...model.skills, busy: null, progress: null, table: { ...table, rows } } });
      return { results, backups: '~/Library/Application Support/com.shin315.quota-station/backups/skills' };
    },
    skillsSkip: async (name) => {
      const table = model.skills.table;
      if (table) push({ ...model, skills: { ...model.skills, table: { ...table, rows: table.rows.map((r) => (r.name === name ? { ...r, update: null } : r)) } } });
    },
    skillsRemove: async (name) => {
      const table = model.skills.table;
      if (!table) throw 'notRead';
      push({ ...model, skills: { ...model.skills, table: { ...table, rows: table.rows.filter((r) => r.name !== name) } } });
      return { names: [name], backups: [name + '.folder.20260116-142000'], error: null };
    },
    skillsImport: async (names) => {
      const table = model.skills.table;
      if (!table) throw 'notRead';
      const at = Math.floor(now().getTime() / 1000);
      const moved = table.rows.filter((r) => names.includes(r.name) && r.home).map((r) => ({ name: r.name, home: r.home! }));
      const rows = table.rows.map((r) => {
        if (!names.includes(r.name) || !r.home) return r;
        const cells = { ...r.cells };
        for (const a of ['claude', 'codex', 'agy'] as const) cells[a] = { state: a === r.home ? 'on' : 'off', lock: null };
        return { ...r, home: null, importable: false, cells };
      });
      push({ ...model, skills: { ...model.skills, table: { ...table, rows }, last: { at: new Date(at * 1000).toISOString(), kind: 'import', names, outcome: null, canUndo: true } } });
      return { moved, failed: null, at };
    },
    skillsOpenLink: async () => {},
    skillsCheck: async () => {
      push({ ...model, skills: { ...model.skills, updates: { ...model.skills.updates, checking: true } } });
      if (options.skillsDelayMs) await new Promise((r) => setTimeout(r, options.skillsDelayMs));
      push({ ...model, skills: { ...model.skills, updates: { ...model.skills.updates, checking: false, checkedAt: now().toISOString(), error: null } } });
    },
    releaseCheck: async () => {
      push({ ...model, release: { ...model.release, checking: true } });
      if (options.releaseDelayMs) await new Promise((r) => setTimeout(r, options.releaseDelayMs));
      const found = options.releaseFinds;
      const checkedAt = Math.floor(now().getTime() / 1000);
      if (found === 'offline') return push({ ...model, release: { ...model.release, checking: false, checkedAt, error: 'offline' } });
      const latest = found === undefined ? model.release.latest : found;
      const newer = !!latest && latest.version.localeCompare(model.app.version, undefined, { numeric: true }) > 0;
      push({ ...model, release: { ...model.release, checking: false, checkedAt, error: null, none: !latest, latest, newer } });
    },
    cliToolsCheck: async () => {
      const c = model.cliTools;
      if (!c) return;
      push({ ...model, cliTools: { ...c, checking: true } });
      if (options.cliDelayMs) await new Promise((r) => setTimeout(r, options.cliDelayMs));
      const tools = model.cliTools!.tools.map((v) => ({ ...v, failed: null, ...(options.cliClosed ? { running: 0 } : {}) }));
      push({ ...model, cliTools: { ...model.cliTools!, checking: false, checkedAt: Math.floor(now().getTime() / 1000), offline: false, tools } });
    },
    cliToolsOpened: async () => {
      // As the Rust side: while "Tự động kiểm tra" is on, a check when the last one is missing or a day old.
      const c = model.cliTools;
      if (!c?.daily || c.tools.some((v) => v.updating)) return;
      const nowS = Math.floor(now().getTime() / 1000);
      if (c.checkedAt && nowS - c.checkedAt < (c.offline ? 3600 : 86400)) return;
      await api.cliToolsCheck();
    },
    cliToolUpdate: async (tool) => {
      const set = (over: Partial<CliToolView>) =>
        push({ ...model, cliTools: { ...model.cliTools!, tools: model.cliTools!.tools.map((v) => (v.tool === tool ? { ...v, ...over } : v)) } });
      const v = model.cliTools?.tools.find((x) => x.tool === tool);
      if (!v?.version) throw 'notInstalled';
      if (v.handsOff) throw 'handsOff';
      if (v.updating) throw 'busy';
      set({ updating: true, last: null });
      if (options.cliDelayMs) await new Promise((r) => setTimeout(r, options.cliDelayMs! * 2));
      const at = Math.floor(now().getTime() / 1000);
      const fail = options.cliFails?.[tool];
      if (fail) return set({ updating: false, last: { ok: false, ...fail, log: '~/Library/Application Support/com.shin315.quota-station/cli-updates/' + tool + '.log', at } });
      // As the app does: a tool that waits does not update while one of its sessions runs.
      if (v.running > 0 && !v.safeWhileRunning) return set({ updating: false, last: { ok: false, failure: 'started', detail: String(v.running), at } });
      set({ updating: false, version: v.latest, newer: false, last: { ok: true, from: v.version, to: v.latest ?? v.version, at } });
    },
    setCliChecks: async (daily) => {
      const c = model.cliTools;
      if (c) push({ ...model, cliTools: { ...c, daily } });
    },
    releaseSkip: async (version) => push({ ...model, release: { ...model.release, skipped: version } }),
    releaseLater: async (version) => push({ ...model, release: { ...model.release, later: version } }),
    setReleaseChecks: async (on) => push({ ...model, release: { ...model.release, auto: on } }),
    releaseOpen: async (what) => {
      released.push(what);
    },
    releaseDiagnostics: async () =>
      [
        'Quota Station ' + model.app.version + ' (' + (model.app.commit ?? 'no commit') + ') · MIT build',
        'macOS 26.1 · aarch64 · language auto' +
          // @private:begin
          ' · Advanced ' + (model.advanced.on ? 'on' : 'off') +
          // @private:end
          '',
        'Claude Code: found · ' + model.claudeAccounts.accounts.length + ' account(s) kept · Keychain ' + model.snapshot.keychain,
        'Claude quota: ' + model.snapshot.claude.status + ', ' + model.snapshot.claude.accounts.length + ' account(s)',
        'Antigravity quota: ' + model.snapshot.agy.status + ', ' + model.snapshot.agy.accounts.length + ' account(s)',
        'Codex quota: ' + model.snapshot.codex.status + ', ' + model.snapshot.codex.accounts.length + ' account(s)',
      ].join('\n'),
    skillsSetChecks: async (daily, notify) => {
      push({ ...model, skills: { ...model.skills, updates: { ...model.skills.updates, settings: { daily, notify } } } });
    },
    statsRead: async () => {
      if (model.stats.reading) return;
      const step = options.statsStepMs;
      const done = () => push({ ...model, stats: { ...model.stats, reading: false, first: false, progress: [], generation: model.stats.generation + 1, readAt: now().toISOString() } });
      if (!step) return done();
      // A first read: Claude Code's logs, then Codex's, a tenth at a time.
      const first = !model.stats.readAt;
      push({ ...model, stats: { ...model.stats, reading: true, first, progress: [] } });
      const totals = { claude: 2140, codex: 960 } as const;
      for (const agent of ['claude', 'codex'] as const) {
        for (let i = 0; i <= 10; i++) {
          await new Promise((r) => setTimeout(r, step));
          const progress = model.stats.progress.filter((p) => p.agent !== agent);
          progress.push({ agent, done: Math.round((totals[agent] * i) / 10), total: totals[agent] });
          push({ ...model, stats: { ...model.stats, progress } });
        }
      }
      done();
    },
    statsReport: async (period) => {
      if (options.statsFails) throw 'io';
      return statsReport(period, statsSkipped);
    },
    statsAck: async () => {
      statsSkipped = statsSkipped.map((s) => ({ ...s, acknowledged: s.sessions }));
      push({ ...model, stats: { ...model.stats, generation: model.stats.generation + 1 } });
    },
    envCheck: async () => {
      push({ ...model, env: { ...model.env, checking: true } });
      if (options.envDelayMs) await new Promise((r) => setTimeout(r, options.envDelayMs));
      push({ ...model, env: { ...model.env, checking: false, checkedAt: now().toISOString() } });
    },
    envOpen: async (id) => {
      const f = model.env.found.find((x) => x.id === id);
      if (!f) throw 'notFound';
      if (f.place === 'launchctl') throw 'noFile';
      envOpened.push(id);
      return 'Visual Studio Code';
    },
    envIgnore: async (id) => {
      const f = model.env.found.find((x) => x.id === id);
      if (!f) throw 'notFound';
      envHidden.set(id, f);
      const name = f.name;
      const recent = model.alerts.recent.map((r) =>
        r.alert.kind === 'env' && !r.answer && r.alert.actions.some((a) => a.id === 'env:ignore:' + name)
          ? { ...r, answer: { action: 'env:ignore:' + name, label: 'Bỏ qua', at: Math.floor(now().getTime() / 1000), result: said('Đã bỏ qua ' + name, 'Báo lại nếu biến được đặt ở chỗ khác. App không sửa file shell.') } }
          : r,
      );
      push({
        ...model,
        env: { ...model.env, found: model.env.found.filter((x) => x.id !== id), ignored: model.env.ignored + 1 },
        alerts: { ...model.alerts, recent, unanswered: unanswered(recent), unread: unread(recent) },
      });
    },
    envUnignore: async (id) => {
      const f = envHidden.get(id);
      if (!f) return;
      envHidden.delete(id);
      push({ ...model, env: { ...model.env, found: [...model.env.found, f], ignored: Math.max(0, model.env.ignored - 1) } });
    },
    setsSave: async (set) => {
      const name = set.name.trim();
      if (!name || name.length > 60) throw 'name';
      if (model.sets.sets.some((s) => s.id !== set.id && s.name.toLowerCase() === name.toLowerCase())) throw 'taken';
      const saved = { ...set, name, id: set.id || 's' + now().getTime() };
      const known = model.sets.sets.some((s) => s.id === saved.id);
      const list = known ? model.sets.sets.map((s) => (s.id === saved.id ? saved : s)) : [...model.sets.sets, saved];
      const applied = model.sets.applied;
      const drift = applied?.id === saved.id ? fakeSetDrift(saved, model.mcp.table, model.skills.table) : applied?.drift;
      push({ ...model, sets: { ...model.sets, sets: list, removed: null, applied: applied ? { ...applied, drift: drift ?? null } : null } });
      return saved;
    },
    setsDelete: async (id) => {
      const index = model.sets.sets.findIndex((s) => s.id === id);
      if (index < 0) throw 'missing';
      const set = model.sets.sets[index];
      const applied = model.sets.applied?.id === id ? model.sets.applied : null;
      removedSet = { set, index, applied };
      push({ ...model, sets: { ...model.sets, sets: model.sets.sets.filter((s) => s.id !== id), applied: applied ? null : model.sets.applied, removed: set.name } });
    },
    setsUndelete: async () => {
      const r = removedSet;
      if (!r) throw 'nothing';
      removedSet = null;
      const list = [...model.sets.sets];
      list.splice(Math.min(r.index, list.length), 0, r.set);
      push({ ...model, sets: { ...model.sets, sets: list, applied: model.sets.applied ?? r.applied, removed: null } });
    },
    setsPreview: async (id) => {
      await setsWait();
      const set = model.sets.sets.find((s) => s.id === id);
      if (!set) throw 'missing';
      const mcp = model.mcp.table;
      const skills = model.skills.table;
      const ch = fakeSetChanges(set, mcp, skills);
      return {
        id,
        name: set.name,
        mcp: mcp ? fakePreview(mcp, ch.mcp) : { groups: [], refused: [], changes: [], bases: {} },
        skills: skills ? fakeSkillPreview(skills, ch.skills) : { groups: [], refused: [], changes: [], bases: {} },
        skipped: ch.skipped,
        codex: set.codex,
      };
    },
    setsApply: async (id, sent) => {
      if (model.sets.busy || model.mcp.busy === 'writing' || model.skills.busy || model.codex.busy) throw 'busy';
      const set = model.sets.sets.find((s) => s.id === id);
      if (!set) throw 'missing';
      if (options.setsChanged) throw 'changed:' + options.setsChanged;
      push({ ...model, sets: { ...model.sets, busy: 'applying' } });
      await setsWait();
      const at = Math.floor(now().getTime() / 1000);
      const out: SetOutcome = { mcp: null, skills: null, ok: true, at };
      let mcp = model.mcp.table;
      let skills = model.skills.table;
      if (mcp && sent.mcp.length) {
        const r = fakeApply(mcp, sent.mcp, at);
        mcp = r.table;
        out.mcp = r.outcome;
      }
      if (skills && sent.skills.length) {
        const r = fakeSkillApply(skills, sent.skills, at);
        skills = r.table;
        out.skills = r.outcome;
      }
      const side = set.codex === 'chatGpt' ? 'chatgpt' : set.codex;
      const codexBefore = set.codex && side !== model.codex.current ? model.codex.current : null;
      lastSetUndo = { mcp: out.mcp?.undo ?? null, skills: out.skills?.undo ?? null, codexBefore };
      const iso = new Date(at * 1000).toISOString();
      push({
        ...model,
        mcp: { ...model.mcp, table: mcp, readAt: iso },
        skills: { ...model.skills, table: skills, readAt: iso },
        sets: {
          ...model.sets,
          busy: null,
          applied: { id, name: set.name, at: iso, drift: fakeSetDrift(set, mcp, skills), canUndo: !!(sent.mcp.length || sent.skills.length), codexBefore },
        },
      });
      return out;
    },
    setsUndo: async () => {
      const u = lastSetUndo ?? (model.sets.applied?.canUndo ? { mcp: null, skills: null, codexBefore: model.sets.applied.codexBefore } : null);
      if (!u || !model.sets.applied) throw 'nothing';
      push({ ...model, sets: { ...model.sets, busy: 'undoing' } });
      await setsWait();
      lastSetUndo = null;
      const at = Math.floor(now().getTime() / 1000);
      let mcp = model.mcp.table;
      let skills = model.skills.table;
      const done = { skills: null as SkillOutcome | null, mcp: null as McpOutcome | null };
      if (skills && u.skills?.changes.length) {
        const r = fakeSkillApply(skills, u.skills.changes, at);
        skills = r.table;
        done.skills = { ...r.outcome, undo: null };
      }
      if (mcp && u.mcp?.changes.length) {
        const r = fakeApply(mcp, u.mcp.changes, at);
        mcp = r.table;
        done.mcp = { ...r.outcome, undo: null };
      }
      push({ ...model, mcp: { ...model.mcp, table: mcp }, skills: { ...model.skills, table: skills }, sets: { ...model.sets, busy: null, applied: null } });
      return { ...done, left: [], ok: true, codexBefore: u.codexBefore };
    },
    pluginsRead: async () => {
      push({ ...model, plugins: { ...model.plugins, readAt: now().toISOString() } });
    },
    instrRead: async () => {
      push({ ...model, instructions: { ...model.instructions, readAt: now().toISOString() } });
    },
    instrShow: async (path) => instrShownFixture(path),
    instrCompare: async () => instrDiffFixture(),
    instrInclude: async (path, on) => {
      const v = model.instructions.view;
      if (v) push({ ...model, instructions: { ...model.instructions, view: { ...v, files: v.files.map((f) => (f.path === path ? { ...f, include: on } : f)) } } });
    },
    instrNotKey: async (path, line) => {
      const v = model.instructions.view;
      if (v) push({ ...model, instructions: { ...model.instructions, view: { ...v, files: v.files.map((f) => (f.path === path ? { ...f, keyLines: f.keyLines.filter((l) => l !== line) } : f)) } } });
    },
    instrAdd: async (path) => {
      const p = path.trim().replace(/^\/Users\/[^/]+/, '~');
      if (!p.startsWith('~/')) throw 'outside';
      if (!/\.(md|txt)$/.test(p)) throw 'type';
      const v = model.instructions.view;
      const file = { path: p, agent: null, modifiedAt: now().toISOString(), size: 800, lines: 20, keyLines: [], error: null, include: true, extra: true, vsPack: 'new' as const };
      if (v) push({ ...model, instructions: { ...model.instructions, view: { ...v, files: [...v.files, file] } } });
      return p;
    },
    instrForget: async (path) => {
      const v = model.instructions.view;
      if (v) push({ ...model, instructions: { ...model.instructions, view: { ...v, files: v.files.filter((f) => f.path !== path) } } });
    },
    instrOpen: async () => {},
    backupRead: async () => {
      push({ ...model, backup: { ...model.backup, readAt: now().toISOString() } });
    },
    backupNow: async () => {
      if (!model.backup.icloud) throw 'noICloud';
      const at = Math.floor(now().getTime() / 1000);
      const id = 'agents-' + at + '-' + model.machine.toLowerCase().replace(/[^a-z0-9]+/g, '-');
      const count = model.mcp.table?.rows.length ?? 0;
      const keys = model.mcp.table?.rows.reduce((n, r) => n + r.secrets.length, 0) ?? 0;
      const pack = { id, state: 'ok' as const, at, mac: model.machine, mine: true, mcp: count, keys, newer: false };
      push({ ...model, backup: { ...model.backup, packs: [pack, ...(model.backup.packs ?? [])].slice(0, 10), autoError: null } });
      return id;
    },
    backupSetAuto: async (afterChange, daily) => {
      push({ ...model, backup: { ...model.backup, auto: { afterChange, daily } } });
    },
    backupSetFolder: async (path) => {
      const folder = path.trim();
      if (folder && !folder.startsWith('/') && !folder.startsWith('~')) throw 'notFound';
      const icloud = '~/Library/Mobile Documents/com~apple~CloudDocs/Quota Station Agents';
      push({ ...model, backup: { ...model.backup, folder: folder || icloud, customFolder: !!folder, packs: null } });
    },
    backupOpenFolder: async () => {},
    backupDeleteKey: async (mcp, slot) => {
      push({ ...model, backup: { ...model.backup, keys: (model.backup.keys ?? []).filter((k) => !(k.mcp === mcp && k.slot === slot)) } });
    },
    packSummary: async (id) => {
      const p = model.backup.packs?.find((x) => x.id === id);
      if (p?.state === 'cloud') throw 'cloud';
      return summaryFixture(id);
    },
    restorePlan: async (id) => {
      const p = model.backup.packs?.find((x) => x.id === id);
      if (p?.state === 'cloud') throw 'cloud';
      return planFixture(id);
    },
    restoreRun: async (id, picks, more, keys) => {
      if (model.mcp.busy === 'writing' || model.codex.busy) throw 'busy';
      if (options.restoreChanged) throw 'changed:codex';
      push({ ...model, backup: { ...model.backup, busy: 'restoring' } });
      if (options.restoreDelayMs) await new Promise((r) => setTimeout(r, options.restoreDelayMs));
      const at = Math.floor(now().getTime() / 1000);
      const base = fakeRestore(planFixture(id), picks, keys.map((k) => k.mcp + '|' + k.slot), at);
      const parts = fakeParts(planFixture(id), more, !!options.restorePartial);
      const done = { ...base, parts, canUndo: base.canUndo || parts.some((r) => r.ok) };
      const mac = planFixture(id).mac;
      push({ ...model, backup: { ...model.backup, busy: null, restore: done.canUndo ? { at, pack: id, mac } : model.backup.restore } });
      return done;
    },
    restoreRetry: async (_id, part, item, agent) => {
      if (options.restoreDelayMs) await new Promise((r) => setTimeout(r, options.restoreDelayMs));
      return { part, id: item, agent, kind: '', ok: true, why: null, retry: false };
    },
    // @private:begin
    copyCompare: async (alias) => {
      if (options.copyDelayMs) await new Promise((r) => setTimeout(r, options.copyDelayMs));
      if (options.copyUnreach?.includes(alias)) throw 'unreachable:ssh: connect to host ' + alias + ' port 22: Operation timed out';
      if (options.copyOld?.includes(alias)) throw 'old';
      if (options.copyNoApp?.includes(alias)) throw 'noApp';
      const peer = model.rotation.peers.find((p) => p.alias === alias);
      return (lastCopy = comparisonFixture(alias, peer?.name || alias));
    },
    copyRun: async (alias, _push, selection, keys) => {
      if (options.copyDelayMs) await new Promise((r) => setTimeout(r, options.copyDelayMs));
      if (options.copyChanged) throw 'changed:codex';
      const c = lastCopy ?? comparisonFixture(alias, alias);
      return copyDoneFixture(c, selection, keys, Math.floor(now().getTime() / 1000));
    },
    copyUndo: async () => {
      if (options.copyDelayMs) await new Promise((r) => setTimeout(r, options.copyDelayMs));
      return copyUndoneFixture();
    },
    // @private:end
    restoreUndo: async (force) => {
      if (!model.backup.restore) throw 'nothing';
      if (options.restoreUndoChanged && !force) throw 'changed:codex';
      push({ ...model, backup: { ...model.backup, restore: null } });
      return { outcome: null, kept: 1, keys: 1 };
    },
    setCodexHome: async (path) => {
      if (path && !path.startsWith('~')) throw new Error('notFound');
      push({ ...model, codex: { ...model.codex, home: path || '~/.codex', customHome: !!path } });
      return model;
    },
    agyDismissOverwrite: async () => {
      push({ ...model, agySwitch: { ...model.agySwitch, overwritten: null } });
      return model;
    },
    alertAction: async (id, action) => {
      const record = model.alerts.recent.find((r) => r.id === id);
      if (!record) throw 'unknown';
      if (record.answer) throw 'answered';
      const pressed = record.alert.actions.find((a) => a.id === action);
      if (!pressed) throw 'noSuchAction';
      const at = Math.floor(now().getTime() / 1000);
      let result: AlertSaid | null;
      let openSwitch: number | null = null;
      let switchBack: { number: number; how: AgyHow } | null = null;
      const agy = /^agy:(plain|quitFirst|keepRunning):(\d+)$/.exec(action);
      if (record.alert.test) result = said('Thông báo thử', 'Bạn đã bấm “' + pressed.label + '”. Thông báo thử không làm gì cả.');
      else if (action === 'dismiss') {
        let again = 'Không nhắc lại cho lần này.';
        let back = 'Không lặp lại cho cùng sự kiện. Quota Station báo lại khi có lại hạn mức.';
        // @private:begin
        if (record.alert.kind === 'leaving') again = 'Không nhắc lại cho lần rời vòng này.';
        back = 'Không lặp lại cho cùng sự kiện. Quota Station báo lại khi hạn mức trong vòng có lại.';
        // @private:end
        result = record.alert.kind === 'pool' ? said('Đã bỏ qua thông báo này', back) : said('Đã ghi nhận', again);
      }
      else if (action === 'snooze') result = said('Sẽ nhắc lại lúc 09:00 ngày mai', 'Thứ Bảy 17/01.');
      else if (action === 'agy:keep') result = said('Đã bỏ qua', 'Antigravity giữ tài khoản đang dùng. Ứng dụng ngừng theo dõi lần chuyển này.');
      else if (action === 'agy:login') {
        result = said('Đã mở tab Antigravity', 'Bấm Đăng nhập lại ở dòng của tài khoản này.');
        // The Rust side opens the main window on the Antigravity tab.
        tab = 'agy';
        tabListeners.forEach((l) => l('agy'));
      }
      else if (action === 'env:settings') {
        result = said('Đã mở Cài đặt', 'Ở tab Dịch vụ. App không sửa file shell.');
        // The Rust side opens the main window on Cài đặt's Dịch vụ, the variables' card first.
        settingsGroup = 'services';
        tab = 'settings';
        tabListeners.forEach((l) => l('settings'));
      } else if (action.startsWith('env:ignore:')) {
        const name = action.slice('env:ignore:'.length);
        result = said('Đã bỏ qua ' + name, 'Báo lại nếu biến được đặt ở chỗ khác. App không sửa file shell.');
        for (const f of model.env.found.filter((x) => x.name === name)) envHidden.set(f.id, f);
        const gone = model.env.found.filter((x) => x.name === name).length;
        push({ ...model, env: { ...model.env, found: model.env.found.filter((x) => x.name !== name), ignored: model.env.ignored + gone } });
      } else if (action === 'cli:settings') {
        result = said('Đã mở Cập nhật Agent', 'Ở nhóm Agent, mục Cập nhật. App chỉ cập nhật khi bạn bấm.');
        // The Rust side opens the main window on Cập nhật Agent.
        tab = 'cliTools';
        tabListeners.forEach((l) => l('cliTools'));
      } else if (action === 'skills:open') {
        result = said('Đã mở tab Skills', 'Bấm Cập nhật ở skill bạn muốn. App không tự cập nhật.');
        tab = 'skills';
        tabListeners.forEach((l) => l('skills'));
      } else if (action.startsWith('agy:verify:')) result = said('Đã mở trang xác minh', 'Trang của Google mở trong trình duyệt.');
      else if (action.startsWith('switch:')) {
        result = said('Đã mở hộp thoại chuyển tài khoản', 'Xác nhận trong hộp thoại để chuyển. Chưa có gì thay đổi.');
        openSwitch = Number(action.slice('switch:'.length));
      } else if (agy) {
        result = null;
        switchBack = { number: Number(agy[2]), how: agy[1] as AgyHow };
      } else throw 'noSuchAction';
      const answer = (r: AlertSaid | null) => {
        const recent = model.alerts.recent.map((x) =>
          x.id === id ? { ...x, answer: { action, label: pressed.label, at, result: r } } : x,
        );
        push({
          ...model,
          alerts: { ...model.alerts, recent, unanswered: unanswered(recent), unread: unread(recent) },
          agySwitch: action === 'agy:keep' ? { ...model.agySwitch, overwritten: null } : model.agySwitch,
        });
      };
      answer(result);
      if (switchBack) {
        const n = switchBack.number;
        // The same switch as the sheet's: busy, then the account in use, the watch over.
        push({ ...model, agySwitch: { ...model.agySwitch, busy: n } });
        if (options.agySwitchDelayMs) await new Promise((r) => setTimeout(r, options.agySwitchDelayMs));
        const g = model.snapshot.agy;
        push({
          ...model,
          snapshot: { ...model.snapshot, agy: { ...g, accounts: g.accounts.map((a) => ({ ...a, active: a.number === n })) } },
          agySwitch: { ...model.agySwitch, busy: null, overwritten: null },
        });
        answer({
          tone: 'ok',
          title: 'Đã chuyển lại sang Ultra ' + n,
          body:
            switchBack.how === 'quitFirst'
              ? 'Antigravity.app đã mở lại với tài khoản này.'
              : 'Antigravity.app vẫn mở nên có thể đổi về lần nữa. Nếu vậy, app báo thêm.',
        });
      }
      return { openSwitch };
    },
    alertsRead: async () => {
      if (!model.alerts.unread) return;
      const recent = model.alerts.recent.map((r) => (r.read ? r : { ...r, read: true }));
      push({ ...model, alerts: { ...model.alerts, recent, unread: 0 } });
    },
    testAlert: async (kind) => {
      const alert = sample(kind, Math.floor(now().getTime() / 1000));
      const record: AlertRecord = { id: alert.key + '@' + alert.at, alert, notified: model.alerts.permission === 'allowed', answer: null, read: false };
      const recent = [record, ...model.alerts.recent.filter((r) => r.id !== record.id)].slice(0, 30);
      push({ ...model, alerts: { ...model.alerts, recent, unanswered: unanswered(recent), unread: unread(recent) } });
      return model;
    },
    setAlertSwitch: async (kind, on) => {
      push({ ...model, alerts: { ...model.alerts, switches: { ...model.alerts.switches, [kind]: on } } });
      return model;
    },
    setPoolAlert: async (dryAtMost, roomAtLeast) => {
      const ok = (v: number) => Number.isInteger(v) && v >= 1 && v <= 99;
      if (!ok(dryAtMost) || !ok(roomAtLeast)) throw 'range';
      if (roomAtLeast <= dryAtMost) throw 'order';
      push({ ...model, alerts: { ...model.alerts, poolAlert: { dryAtMost, roomAtLeast } } });
      return model;
    },
    // @private:begin
    setAdvanced: async (on) => {
      push({ ...model, advanced: { ...model.advanced, on } });
      return model;
    },
    setFirstmate: async (firstmate) => {
      push({ ...model, advanced: { ...model.advanced, firstmate } });
      return model;
    },
    setFirstmateFile: async (path) => {
      if (!path.trim()) {
        push({ ...model, firstmate: { file: null, status: 'notSet', line: null, writtenAt: null, fresh: false, ownDry: model.firstmate.ownDry } });
        return model;
      }
      if (!/(^|\/)quota-pool\.last$/.test(path.trim())) throw 'notPoolFile';
      if (!options.firstmate) throw 'notFound';
      push({ ...model, firstmate: { ...options.firstmate, file: path.trim() } });
      return model;
    },
    // @private:end
    settingsOpened: async () => {},
    setLaunchAtLogin: async (on) => {
      if (model.app.launchAtLogin === 'unavailable') throw 'unavailable';
      push({ ...model, app: { ...model.app, launchAtLogin: !on ? 'off' : options.loginApproval ? 'needsApproval' : 'on' } });
      return model;
    },
    openLoginItems: async () => {},
    // @private:begin
    // A clone is a path ending in `quota-station`; `nowhere` has no folder.
    setSourceDir: async (path) => {
      const p = path.trim();
      if (!p) {
        push({ ...model, app: { ...model.app, source: null, sourceStatus: 'notSet' } });
        return model;
      }
      if (p.includes('nowhere')) throw 'missing';
      if (!/quota-station\/?$/.test(p)) throw 'notSource';
      push({ ...model, app: { ...model.app, source: p, sourceStatus: 'ok' } });
      return model;
    },
    updateFromSource: async () => {
      if (model.app.updating) throw 'busy';
      if (!model.app.bundled) throw 'unbundled';
      if (model.app.sourceStatus !== 'ok') throw 'noSource';
      const ends = options.update?.() ?? { kind: 'installed', commit: 'b2c3d4e5f60718293a4b5c6d7e8f901234567890', version: '0.1.1' };
      const wait = () => (options.updateDelayMs ? new Promise((r) => setTimeout(r, options.updateDelayMs)) : Promise.resolve());
      push({ ...model, app: { ...model.app, updating: 'pulling', outcome: null, error: null } });
      await wait();
      const pulled = ends.kind !== 'upToDate' && ends.kind !== 'pull' && ends.kind !== 'source' && !(ends.kind === 'timeout' && ends.step === 'pulling');
      if (pulled) {
        push({ ...model, app: { ...model.app, updating: 'installing' } });
        await wait();
      }
      const done = ends.kind === 'upToDate' || ends.kind === 'installed';
      push({
        ...model,
        app: { ...model.app, updating: null, outcome: done ? (ends as UpdateOutcome) : null, error: done ? null : (ends as UpdateError) },
      });
      return model;
    },
    relaunch: async () => {
      if (model.app.outcome?.kind !== 'installed') throw 'notInstalled';
      relaunched += 1;
    },
    // @private:end
    getHistory: async () => options.history ?? [],
    notifyRefresh: async () => {},
    openNotificationSettings: async () => {},
    onOpenSwitch: async (handler) => {
      openSwitchListeners.add(handler);
      return () => openSwitchListeners.delete(handler);
    },
    setShowNames: async (on) => {
      const snapshot = on ? named : withoutNames(model);
      push({ ...model, showNames: on, snapshot, antigravity: agy({ accounts: accounts(model.antigravity.accounts, on) }) });
      return model;
    },
    setLanguage: async (language) => {
      push({ ...model, language });
      return model;
    },
    finishSetup: async (open) => {
      finished.push(open);
      push({ ...model, firstRun: false });
      return model;
    },
    setTheme: async (theme) => {
      push({ ...model, theme });
      return model;
    },
    setThemePack: async (themePack) => {
      push({ ...model, themePack });
      return model;
    },
    setMenubar: async (menubar) => {
      push({ ...model, menubar });
      return model;
    },
    setMenubarWindow: async (menubarWindow) => {
      push({ ...model, menubarWindow });
      return model;
    },
    onModel: async (handler) => {
      listeners.add(handler);
      return () => listeners.delete(handler);
    },
    onTab: async (handler) => {
      tabListeners.add(handler);
      return () => tabListeners.delete(handler);
    },
    reduceTransparency: async () => reduceTransparency,
    onReduceTransparency: async (handler) => {
      materialListeners.add(handler);
      return () => materialListeners.delete(handler);
    },
    increaseContrast: async () => increaseContrast,
    onIncreaseContrast: async (handler) => {
      contrastListeners.add(handler);
      return () => contrastListeners.delete(handler);
    },
    sessionGet: async () => session,
    sessionStep: async (id, captures) => {
      session = { ...session, step: id };
      sessionCalls.push({ call: 'step', id, captures });
    },
    sessionDone: async (id, skipped, facts, checks) => {
      session = { ...session, done: [...session.done, id] };
      sessionCalls.push({ call: 'done', id, skipped, facts, checks });
    },
    sessionShot: async (what) => {
      sessionCalls.push({ call: 'shot', id: what });
    },
    sessionNote: async (kind) => {
      sessionCalls.push({ call: 'note', id: kind });
    },
    sessionOpen: async (what) => {
      sessionCalls.push({ call: 'open', id: what });
    },
    sessionZip: async () => {
      sessionCalls.push({ call: 'zip' });
      session = { ...session, zip: { name: 'diag-2026-10-28.zip', folder: 'Desktop' } };
      return session.zip!;
    },
    sessionReveal: async () => {
      sessionCalls.push({ call: 'reveal' });
    },
  };
  // @private:begin
  /** The window's close button, as macOS would press it with unsaved rules. */
  const requestClose = () => closeListeners.forEach((l) => l());
  /** `Thoát Quota Station` in the menu bar menu with unsaved rules. */
  const requestQuit = () => {
    quitPending = true;
    quitListeners.forEach((l) => l());
  };
  // @private:end
  /** The owner turns Reduce Transparency on or off in macOS's settings. */
  const setReduceTransparency = (on: boolean) => {
    reduceTransparency = on;
    materialListeners.forEach((l) => l(on));
  };
  /** The owner turns Increase Contrast on or off in macOS's settings. */
  const setIncreaseContrast = (on: boolean) => {
    increaseContrast = on;
    contrastListeners.forEach((l) => l(on));
  };
  /** A button on a macOS card that opens the popover's switch sheet. */
  const cardOpensSwitch = (slot: number) => openSwitchListeners.forEach((l) => l(slot));
  /** The macOS card itself pressed: the Rust side opens the main window with the bell's panel open. */
  const cardOpened = () => {
    tab = 'alerts';
    tabListeners.forEach((l) => l('alerts'));
  };
  return {
    api,
    push,
    current: () => model,
    // @private:begin
    requestClose,
    requestQuit,
    closed: () => closed,
    /** How many times "Mở lại" quit the app. */
    relaunched: () => relaunched,
    // @private:end
    cardOpensSwitch,
    cardOpened,
    setReduceTransparency,
    setIncreaseContrast,
    /** The ids "Mở file" opened (M14). */
    envOpened: () => envOpened,
    /** What the release buttons opened: `download`, `notes`, `releases`, `issue`. */
    released: () => released,
    /** Where the first run's last step opened the app: `window` or `popover`. */
    finished: () => finished,
    /** What the guide asked of the session (a test build's). */
    sessionCalls: () => sessionCalls,
  };
}

/** What the Rust side shows for "Dán khoá", on a fixture: the key only as its hint. */
/** A provider's table id from the name the owner typed: `relay-eu` → `relay_eu`. */
function providerId(name: string): string {
  return name.trim().toLowerCase().replace(/[^a-z0-9_]+/g, '_').replace(/^_+|_+$/g, '') || 'provider';
}

/** The lines a new provider adds to `config.toml`, the key only as its hint. */
const IMPORT_TOKEN = 'import-1';
const PASTE_TOKEN = 'paste-1';

/**
 * The files "Nhập file đăng nhập (JSON)" finds on made-up data: a new
 * CLIProxyAPI account, Codex's own auth.json for the account that must sign
 * in again, one already here, a refresh token alone, and a file that is not
 * a sign-in.
 */
export function fakeImportPreview(c: Model['codex']): CodexImportPreview {
  const by = (email: string) => c.accounts.find((a) => a.email === email);
  const side = c.accounts.find((a) => a.kind === 'chatgpt' && a.login === 'relogin');
  const here = c.accounts.find((a) => a.kind === 'chatgpt' && a.login === 'ok' && a.email);
  const items: CodexImportItem[] = [
    by('lan.work@example.com')
      ? { file: 'codex-lan.work@example.com-plus.json', format: 'cliproxy', email: 'lan.work@example.com', plan: 'Plus', state: 'dup', id: by('lan.work@example.com')!.id }
      : { file: 'codex-lan.work@example.com-plus.json', format: 'cliproxy', email: 'lan.work@example.com', plan: 'Plus', state: 'new', id: null },
    ...(side ? [{ file: 'auth.json', format: 'codex', email: side.email, plan: side.plan, state: 'renew', id: side.id } satisfies CodexImportItem] : []),
    ...(here ? [{ file: 'codex-' + here.email + '-' + (here.plan ?? '').toLowerCase() + '.json', format: 'cliproxy', email: here.email, plan: here.plan, state: 'dup', id: here.id } satisfies CodexImportItem] : []),
    { file: 'codex-old.team@example.com.json', format: 'cliproxy', email: 'old.team@example.com', plan: null, state: 'refreshOnly', id: null },
    { file: 'notes.json', format: null, email: null, plan: null, state: 'broken', id: null },
  ];
  return { token: IMPORT_TOKEN, items };
}

/** JSON pasted in "Nhập file đăng nhập (JSON)" on made-up data: two CLIProxyAPI sign-ins, one a line. */
export function fakePastePreview(c: Model['codex']): CodexImportPreview {
  const files = fakeImportPreview(c).items;
  const items = [files[0], files[3]].map((i) => ({ ...i, file: null }));
  return { token: PASTE_TOKEN, items };
}

export function fakeProviderPreview(c: Model['codex'], name: string, baseUrl: string, key: string): CodexKeyPreview {
  const none: CodexKeyPreview = { hash: c.hash, lines: [], refusal: null, detail: null, replaces: false, undo: null };
  const k = key.trim().replace(/^Bearer /, '');
  const id = providerId(name);
  if (!name.trim() || !baseUrl.trim() || !k) return { ...none, refusal: { kind: 'empty' } };
  if (['openai', 'ollama', 'lmstudio'].includes(id) || c.accounts.some((a) => a.id === id)) return { ...none, refusal: { kind: 'name' } };
  if (!/^https?:\/\/[^\s@]+$/.test(baseUrl.trim())) return { ...none, refusal: { kind: 'badUrl' } };
  if (/\s/.test(k)) return { ...none, refusal: { kind: 'notAKey' } };
  const hint = k.length < 16 ? '••••••' : k.slice(0, 3) + '…' + k.slice(-4);
  const requires = c.plugins && (c.login === 'chatGpt' || c.login === 'keychain');
  return {
    ...none,
    lines: [
      { op: 'gap', text: '', secret: false },
      { op: 'add', text: '[model_providers.' + id + ']', secret: false },
      { op: 'add', text: 'name = "' + name.trim() + '"', secret: false },
      { op: 'add', text: 'base_url = "' + baseUrl.trim() + '"', secret: false },
      { op: 'add', text: 'wire_api = "responses"', secret: false },
      { op: 'add', text: 'experimental_bearer_token = "' + hint + '"', secret: true },
      { op: 'add', text: 'requires_openai_auth = ' + requires, secret: false },
    ],
  };
}

export function fakeKeyPreview(c: Model['codex'], key: string, baseUrl: string | null): CodexKeyPreview {
  const none: CodexKeyPreview = { hash: c.hash, lines: [], refusal: null, detail: null, replaces: false, undo: null };
  const k = key.trim().replace(/^Bearer /, '');
  const custom = c.custom;
  if (!custom) return { ...none, refusal: { kind: 'noCustom' } };
  if (custom.key === 'envKey') return { ...none, refusal: { kind: 'envKey', var: custom.envKey ?? '' } };
  if (custom.key === 'headers') return { ...none, refusal: { kind: 'headers' } };
  if (!k) return { ...none, refusal: { kind: 'empty' } };
  if (/\s/.test(k)) return { ...none, refusal: { kind: 'notAKey' } };
  if (k === 'PROXY_MANAGED') return { ...none, refusal: { kind: 'placeholder' } };
  if (baseUrl && !/^https?:\/\/[^\s@]+$/.test(baseUrl.trim())) return { ...none, refusal: { kind: 'badUrl' } };
  const hint = k.length < 16 ? '••••••' : k.slice(0, 3) + '…' + k.slice(-4);
  const lines: CodexKeyPreview['lines'] = [{ op: 'ctx', text: '[model_providers.' + custom.id + ']', secret: false }];
  if (baseUrl && baseUrl.trim() !== custom.baseUrl) {
    if (custom.baseUrl) lines.push({ op: 'del', text: 'base_url = "' + custom.baseUrl + '"', secret: false });
    lines.push({ op: 'add', text: 'base_url = "' + baseUrl.trim() + '"', secret: false });
  }
  if (custom.key === 'placeholder') lines.push({ op: 'del', text: 'experimental_bearer_token = "PROXY_MANAGED"', secret: false });
  if (custom.key === 'bearer') lines.push({ op: 'del', text: 'experimental_bearer_token = "••••••"', secret: true });
  lines.push({ op: 'add', text: 'experimental_bearer_token = "' + hint + '"', secret: true });
  const replaces = custom.key === 'bearer';
  return { ...none, lines, replaces, undo: replaces ? null : custom.key === 'placeholder' ? 'placeholder' : 'none' };
}

/** The accounts an import brings in: the pool fixture's four. */
function claudeAccountsReady() {
  return claudeAccountsFixture().accounts;
}
