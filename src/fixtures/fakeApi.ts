// A fake Rust side on fixture data, for the tests and the browser preview.
// Never runs quota-axi, claude-swap or the switcher, never opens a browser
// or the Keychain, and writes no file.
// Plain code: the preview bundle must not load Vitest.

import type { Api } from '../lib/api';
import type {
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
  CodexOutcome,
  CodexRestart,
  CodexTarget,
  FirstmateView,
  HistoryEntry,
  Model,
  Outcome,
  PeerView,
  RotationView,
  RulesError,
  Sessions,
  Tab,
  UpdateError,
  UpdateOutcome,
} from '../lib/model';

export interface FakeOptions {
  /** Aliases SSH cannot reach: a save leaves them behind, and "Thử SSH" fails. */
  unreachable?: string[];
  /** What a save, adopt or keep rejects with, instead of writing. */
  rulesError?: RulesError;
  /** How long a save takes; the preview waits to show "Đang lưu…". */
  saveDelayMs?: number;
  /** When an action happens, for the stamps it writes. */
  clock?: () => Date;
  /** `Thoát Quota Station` was chosen before the window listened. */
  quitPending?: boolean;
  /** How long a switch takes before it reports; the preview waits to show the working sheet. */
  switchDelayMs?: number;
  sessions?: Sessions | null;
  /** What a switch reports; by default it lands on the slot asked for. */
  outcome?: (slot: number, holdUntil: string | null) => Outcome;
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
  /** The History tab's lines; none by default. */
  history?: HistoryEntry[];
  /** What a `quota-pool.last` set in Cài đặt reads as; without it a path finds no file. */
  firstmate?: FirstmateView;
  /** macOS waits for the owner's approval after the login switch is turned on. */
  loginApproval?: boolean;
  /** How "Cập nhật từ mã nguồn" ends; by default it installs 0.1.1 from a new commit. */
  update?: () => UpdateOutcome | UpdateError;
  /** How long each update step takes; the preview waits to show them. */
  updateDelayMs?: number;
}

const said = (title: string, body: string): AlertSaid => ({ tone: 'info', title, body });

/** The sample "Gửi thử" sends: modelled on the Rust side's `alerts::sample`, with shorter words. */
function sample(kind: AlertKind, at: number): Alert {
  const base = { key: 'test:' + kind + ':' + at, kind, at, test: true };
  switch (kind) {
    case 'pool':
      return { ...base, tone: 'bad', title: 'Hạn mức Claude trong vòng sắp cạn', body: 'Thử: còn tương đương 0,4 tài khoản.', actions: [{ id: 'switch:0', label: 'Sang Cá nhân (còn 19%)' }, { id: 'dismiss', label: 'Thôi' }] };
    case 'leaving':
      return { ...base, tone: 'warn', title: 'Team A rời vòng sau 30 phút', body: 'Thử: còn 30 phút.', actions: [{ id: 'dismiss', label: 'Ổn' }, { id: 'switch:0', label: 'Chuyển ngay' }] };
    case 'login':
      return { ...base, tone: 'warn', title: 'Team B cần đăng nhập lại', body: 'Thử: đăng nhập hết hạn sau 2 ngày.', actions: [{ id: 'snooze', label: 'Nhắc ngày mai' }, { id: 'dismiss', label: 'Đã biết' }] };
    case 'overwrite':
      return { ...base, tone: 'warn', title: 'Antigravity đổi về tài khoản cũ', body: 'Thử: Antigravity.app đổi về Ultra 1.', actions: [{ id: 'agy:quitFirst:0', label: 'Thoát app rồi chuyển lại' }, { id: 'agy:keepRunning:0', label: 'Chuyển lại' }, { id: 'agy:keep', label: 'Thôi' }] };
    case 'firstmate':
      return { ...base, tone: 'info', title: 'Firstmate đang chờ hạn mức Claude', body: 'Thử: hạn mức trong vòng dưới ngưỡng.', actions: [] };
  }
}

const unanswered = (recent: AlertRecord[]) => recent.filter((r) => !r.alert.test && !r.answer && r.alert.actions.length).length;

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
    antigravity: { ...m.antigravity, accounts: m.antigravity.accounts.map((a) => ({ ...a, email: null, name: null })) },
  };
}

/** RFC 3339 cut to the minute, as the pause flag keeps it. */
function minute(iso: string): string {
  const d = new Date(iso);
  d.setSeconds(0, 0);
  return d.toISOString();
}

/** The stamp's form: this Mac's offset, to the second. */
function stampAt(d: Date): string {
  const p = (n: number) => String(Math.abs(n)).padStart(2, '0');
  const off = -d.getTimezoneOffset();
  return (
    d.getFullYear() + '-' + p(d.getMonth() + 1) + '-' + p(d.getDate()) + 'T' + p(d.getHours()) + ':' + p(d.getMinutes()) + ':' + p(d.getSeconds()) +
    (off < 0 ? '-' : '+') + p(Math.trunc(off / 60)) + ':' + p(off % 60)
  );
}

export function fakeApi(initial: Model, initialTab: Tab = 'overview', options: FakeOptions = {}) {
  let model = initial;
  // The names Rust would send again when the owner shows them.
  const named = initial.snapshot;
  let tab = initialTab;
  let version = 1;
  const now = options.clock ?? (() => new Date());
  const listeners = new Set<(m: Model) => void>();
  const tabListeners = new Set<(t: Tab) => void>();
  const closeListeners = new Set<() => void>();
  const openSwitchListeners = new Set<(slot: number) => void>();
  const quitListeners = new Set<() => void>();
  /** What the last `closeMain` did: the tests check a quit really quits. */
  let closed: 'window' | 'app' | null = null;
  let quitPending = options.quitPending ?? false;
  let relaunched = 0;
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
  const push = (next: Model) => {
    model = next;
    listeners.forEach((l) => l(model));
  };
  /** The accounts renumbered from 1, emails filled in or hidden as the switch says. */
  const accounts = (list: AgyLogin[], show: boolean): AgyLogin[] =>
    list.map((a, i) => ({ ...a, number: i + 1, email: show ? fakeEmail(i + 1) : null, name: show ? 'Người thử ' + (i + 1) : null }));
  const agy = (over: Partial<Model['antigravity']>) => ({ ...model.antigravity, ...over });
  /** Ends the waiting login: with a paste, a cancel or its own end. */
  let endLogin: ((how: 'paste' | 'cancel' | 'self') => void) | null = null;
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
    hidePopover: async () => {},
    resizePopover: async () => {},
    setQuotaAxiPath: async (path) => {
      if (path && !path.includes('quota-axi')) throw new Error('notFound');
      push({ ...model, quotaAxi: { ...model.quotaAxi, path: path || '~/.local/bin/quota-axi', custom: !!path } });
      return model;
    },
    setMachineName: async (name) => {
      push({ ...model, machine: name.trim() || model.machineDefault });
      return model;
    },
    grantKeychain: async () => {
      push({ ...model, snapshot: { ...model.snapshot, keychain: 'granted' } });
      return model;
    },
    claudeSessions: async () => (options.sessions === undefined ? { live: 5, unreadable: 0 } : options.sessions),
    switchClaude: async (slot, holdUntil) => {
      push({ ...model, claudeSwitch: { ...model.claudeSwitch, busy: slot } });
      if (options.switchDelayMs) await new Promise((r) => setTimeout(r, options.switchDelayMs));
      const outcome: Outcome = options.outcome?.(slot, holdUntil) ?? {
        kind: 'switched',
        slot,
        hold: holdUntil ? { until: minute(holdUntil), failed: false } : null,
      };
      const lands = outcome.kind === 'switched' || outcome.kind === 'unconfirmed' ? slot : outcome.kind === 'raced' ? outcome.active : null;
      const hold = outcome.kind !== 'failed' && outcome.hold && !outcome.hold.failed ? outcome.hold : null;
      const accounts = model.snapshot.claude.accounts.map((a) => (lands == null ? a : { ...a, active: a.number === lands }));
      push({
        ...model,
        snapshot: { ...model.snapshot, claude: { ...model.snapshot.claude, accounts } },
        claudeSwitch: { ...model.claudeSwitch, busy: null },
        switcher: hold
          ? { ...model.switcher, pause: { until: hold.until, reason: 'chuyển tay sang tài khoản ' + slot + ' lúc 14:20', holds: true } }
          : model.switcher,
      });
      return outcome;
    },
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
                hash: c.hash + "'",
                external: null,
                lastWrite: { at, provider: to, matches: true },
                backups: [at, ...c.backups].slice(0, 10),
                recent: [{ at, from, to, result: 'switched', restarted: null }, ...c.recent].slice(0, 5),
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
    codexExportMcp: async () => '~/Downloads/codex-mcp-16-01.txt',
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
      else if (action === 'dismiss')
        result =
          record.alert.kind === 'pool'
            ? said('Đã bỏ qua cảnh báo này', 'Không lặp lại cho cùng sự kiện. Quota Station báo lại khi hạn mức trong vòng có lại.')
            : said('Đã ghi nhận', record.alert.kind === 'leaving' ? 'Không nhắc lại cho lần rời vòng này.' : 'Không nhắc lại cho lần này.');
      else if (action === 'snooze') result = said('Sẽ nhắc lại lúc 09:00 ngày mai', 'Thứ Bảy 17/01.');
      else if (action === 'agy:keep') result = said('Đã bỏ qua', 'Antigravity giữ tài khoản đang dùng. App thôi canh lần chuyển này.');
      else if (action === 'agy:login') {
        result = said('Đã mở tab Antigravity', 'Bấm Đăng nhập lại ở dòng của tài khoản này.');
        // The Rust side opens the main window on the Antigravity tab.
        tab = 'agy';
        tabListeners.forEach((l) => l('agy'));
      }
      else if (action.startsWith('agy:verify:')) result = said('Đã mở trang xác minh', 'Trang của Google mở trong trình duyệt.');
      else if (action.startsWith('switch:')) {
        result = said('Đã mở tấm hỏi chuyển', 'Xác nhận trên tấm để chuyển. Chưa có gì thay đổi.');
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
          alerts: { ...model.alerts, recent, unanswered: unanswered(recent) },
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
    testAlert: async (kind) => {
      const alert = sample(kind, Math.floor(now().getTime() / 1000));
      const record: AlertRecord = { id: alert.key + '@' + alert.at, alert, notified: model.alerts.permission === 'allowed', answer: null };
      const recent = [record, ...model.alerts.recent.filter((r) => r.id !== record.id)].slice(0, 30);
      push({ ...model, alerts: { ...model.alerts, recent, unanswered: unanswered(recent) } });
      return model;
    },
    setAlertSwitch: async (kind, on) => {
      push({ ...model, alerts: { ...model.alerts, switches: { ...model.alerts.switches, [kind]: on } } });
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
    settingsOpened: async () => {},
    setLaunchAtLogin: async (on) => {
      if (model.app.launchAtLogin === 'unavailable') throw 'unavailable';
      push({ ...model, app: { ...model.app, launchAtLogin: !on ? 'off' : options.loginApproval ? 'needsApproval' : 'on' } });
      return model;
    },
    openLoginItems: async () => {},
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
    setTheme: async (theme) => {
      push({ ...model, theme });
      return model;
    },
    setMenubar: async (menubar) => {
      push({ ...model, menubar });
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
  };
  /** The window's close button, as macOS would press it with unsaved rules. */
  const requestClose = () => closeListeners.forEach((l) => l());
  /** `Thoát Quota Station` in the menu bar menu with unsaved rules. */
  const requestQuit = () => {
    quitPending = true;
    quitListeners.forEach((l) => l());
  };
  /** A button on a macOS card that opens the popover's switch sheet. */
  const cardOpensSwitch = (slot: number) => openSwitchListeners.forEach((l) => l(slot));
  return {
    api,
    push,
    current: () => model,
    requestClose,
    requestQuit,
    closed: () => closed,
    cardOpensSwitch,
    /** How many times "Mở lại" quit the app. */
    relaunched: () => relaunched,
  };
}
