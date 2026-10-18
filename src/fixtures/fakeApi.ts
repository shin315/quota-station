// A fake Rust side on fixture data, for the tests and the browser preview.
// Never runs quota-axi, claude-swap or the switcher, never opens a browser
// or the Keychain, and writes no file.
// Plain code: the preview bundle must not load Vitest.

import type { Api } from '../lib/api';
import type { AgyApp, AgyHow, AgyLogin, AgyLoginDone, AgyLoginFailure, AgyOutcome, Model, Outcome, PeerView, RotationView, RulesError, Sessions, Tab } from '../lib/model';

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
}

/** The made-up email of a fake account. */
export function fakeEmail(number: number): string {
  return 'ultra' + number + '@example.com';
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
  let tab = initialTab;
  let version = 1;
  const now = options.clock ?? (() => new Date());
  const listeners = new Set<(m: Model) => void>();
  const tabListeners = new Set<(t: Tab) => void>();
  const closeListeners = new Set<() => void>();
  const quitListeners = new Set<() => void>();
  /** What the last `closeMain` did: the tests check a quit really quits. */
  let closed: 'window' | 'app' | null = null;
  let quitPending = options.quitPending ?? false;
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
      push({ ...model, antigravity: agy({ loggingIn: false, accounts: accounts(next, model.showEmail) }) });
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
      push({ ...model, antigravity: agy({ accounts: accounts(list.filter((a) => a.id !== id), model.showEmail) }) });
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
    agyDismissOverwrite: async () => {
      push({ ...model, agySwitch: { ...model.agySwitch, overwritten: null } });
      return model;
    },
    setShowEmail: async (on) => {
      push({ ...model, showEmail: on, antigravity: agy({ accounts: accounts(model.antigravity.accounts, on) }) });
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
  return { api, push, current: () => model, requestClose, requestQuit, closed: () => closed };
}
