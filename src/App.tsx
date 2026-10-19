// Window roots: the popover and the main window share one model, pushed by
// the Rust reader after every read, so both always show the same snapshot.
// Each window opens its own sheets; a switch running from one shows in both.

import { useCallback, useEffect, useRef, useState } from 'react';
import { McpDialogBody, mcpDialogSize } from './agents/McpDialogs';
import { McpTab } from './agents/McpTab';
import { useMcp } from './agents/useMcp';
import { BackupTab } from './backup/BackupTab';
import { BackupDialogBody, backupDialogSize } from './backup/RestoreSheet';
import { useBackup } from './backup/useBackup';
import { AlertsTab } from './alerts/AlertsTab';
import { CodexCard, CodexTab } from './codex/CodexTab';
import { useAgyLogins } from './antigravity/AgyAccounts';
import { SheetBody, type SheetState } from './components/Sheets';
import { agyLoginOf, agySwitchErrorFeedback, agySwitchFeedback, type AppAnswer } from './lib/agySwitch';
import { codexErrorFeedback, codexFeedback, codexRow, codexRestartFeedback, nextTarget } from './lib/codex';
import type { Api } from './lib/api';
import { alertFailedFeedback, testFailedFeedback } from './lib/alerts';
import type { AgyHow, AlertKind, CodexOutcome, CodexTarget, Model, Sessions, Tab, Theme } from './lib/model';
import {
  autoView,
  extendedFeedback,
  pauseFailedFeedback,
  pauseOf,
  pausedFeedback,
  resumedFeedback,
  switchErrorFeedback,
  switchFeedback,
  type PauseChoice,
} from './lib/switching';
import { sidebarLine, syncSummary } from './lib/rules';
import { isQuiet, keychainFeedback, popoverView, refreshedFeedback, settingsFailedFeedback, type Action, type FeedbackView } from './lib/view';
import { windowView } from './lib/window';
import { Popover } from './popover/Popover';
import { FirstmateTab } from './firstmate/FirstmateTab';
import { HistoryTab } from './history/HistoryTab';
import { CompareDialog, LeaveDialog, RulesTab } from './rules/RulesTab';
import { useRules } from './rules/useRules';
import { Settings } from './settings/Settings';
import { MainWindow } from './window/MainWindow';

/** Feedback lines without buttons clear themselves after this long. */
const FEEDBACK_MS = 8000;
/** A line that only says something went well goes sooner. */
const QUIET_MS = 6000;

function useModel(api: Api): Model | null {
  const [model, setModel] = useState<Model | null>(null);
  useEffect(() => {
    let live = true;
    let stop: (() => void) | undefined;
    void api.onModel((m) => live && setModel(m)).then((unlisten) => {
      if (live) stop = unlisten;
      else unlisten();
    });
    void api.getModel().then((m) => live && setModel((cur) => cur ?? m));
    return () => {
      live = false;
      stop?.();
    };
  }, [api]);
  return model;
}

/** The clock for ages and countdowns, ticking while the window is open. */
function useNow(clock: () => Date, everyMs = 15000): Date {
  const [now, setNow] = useState(clock);
  useEffect(() => {
    const id = setInterval(() => setNow(clock()), everyMs);
    return () => clearInterval(id);
  }, [clock, everyMs]);
  return now;
}

/** macOS's own look, where the page can ask for it. */
const systemLook = (): MediaQueryList | null => (typeof window.matchMedia === 'function' ? window.matchMedia('(prefers-color-scheme: dark)') : null);

/**
 * Puts the owner's look on the page (`data-theme` drives every token) and
 * says whether what is on screen now is dark: the setting, or macOS's own
 * look while the setting follows it.
 */
export function useTheme(theme: Theme | undefined): boolean {
  const [systemDark, setSystemDark] = useState(() => !!systemLook()?.matches);
  useEffect(() => {
    const media = systemLook();
    if (!media) return;
    const on = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    media.addEventListener?.('change', on);
    return () => media.removeEventListener?.('change', on);
  }, []);
  useEffect(() => {
    if (!theme) return;
    document.documentElement.dataset.theme = theme;
    document.querySelectorAll<HTMLElement>('.qs[data-theme]').forEach((el) => (el.dataset.theme = theme));
  }, [theme]);
  return theme === 'dark' || (theme !== 'light' && systemDark);
}

/**
 * The line after an action. It belongs to the tab it was asked from (`where`
 * says which tab is on screen when it is shown) and shows only there. A line
 * that only says something went well goes after six seconds; one with
 * buttons waits for one of them; any other goes after eight.
 */
function useFeedback(where?: Tab | null): [FeedbackView | null, (fb: FeedbackView | null) => void] {
  const [fb, setFb] = useState<{ view: FeedbackView; tab: Tab | null } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const here = useRef(where ?? null);
  here.current = where ?? null;
  const show = useCallback((next: FeedbackView | null) => {
    clearTimeout(timer.current);
    setFb(next ? { view: next, tab: here.current } : null);
    if (!next) return;
    if (isQuiet(next)) timer.current = setTimeout(() => setFb(null), QUIET_MS);
    else if (!next.actions?.length) timer.current = setTimeout(() => setFb(null), FEEDBACK_MS);
  }, []);
  useEffect(() => () => clearTimeout(timer.current), []);
  return [fb && fb.tab === (where ?? null) ? fb.view : null, show];
}

/** Refresh and Keychain buttons: busy while they run, then a feedback line. */
function useActions(api: Api, model: Model | null, clock: () => Date, showFeedback: (fb: FeedbackView | null) => void) {
  const [busy, setBusy] = useState<Action | null>(null);
  const waitingFor = useRef<number | null>(null);

  // A refresh is done once the reader has finished a read after it was asked.
  useEffect(() => {
    if (model && waitingFor.current != null && model.reads > waitingFor.current && !model.reading) {
      waitingFor.current = null;
      setBusy(null);
      showFeedback(refreshedFeedback(clock()));
    }
  }, [model, clock, showFeedback]);

  const refresh = async () => {
    setBusy('refresh');
    waitingFor.current = await api.refresh();
  };
  const grant = async () => {
    setBusy('keychain');
    const result = await api.grantKeychain().catch(() => null);
    setBusy(null);
    showFeedback(keychainFeedback(result));
  };
  return { busy: model?.granting ? ('keychain' as const) : busy, refresh, grant };
}

/**
 * Switching Claude, Antigravity and Codex and pausing the switcher: the open sheet,
 * the live session count or Antigravity.app's state it shows, the running
 * switch, and the feedback after each action.
 */
function useSwitching(api: Api, model: Model | null, clock: () => Date, showFeedback: (fb: FeedbackView | null) => void) {
  const [sheet, setSheet] = useState<SheetState | null>(null);
  const [sessions, setSessions] = useState<Sessions | null | 'loading'>('loading');
  const [agyApp, setAgyApp] = useState<AppAnswer>('loading');
  const [working, setWorking] = useState(false);
  const [agyHow, setAgyHow] = useState<AgyHow | null>(null);
  // The feedback after a switch names accounts from the newest model.
  const latest = useRef(model);
  latest.current = model;
  /** The Codex switch the restart sheet follows, for "Không khởi động lại". */
  const codexDone = useRef<CodexOutcome | null>(null);

  const open = (next: SheetState) => {
    setSheet(next);
    if (next.kind === 'agy') {
      setAgyApp('loading');
      void api.agyAppState().then(
        (a) => setAgyApp(a),
        () => setAgyApp(null),
      );
    }
    if (next.kind !== 'claude') return;
    setSessions('loading');
    void api.claudeSessions().then(
      (n) => setSessions(n),
      () => setSessions(null),
    );
  };
  const close = () => {
    if (!working) setSheet(null);
  };
  // A switch from the other window landed on this sheet's account: there is
  // nothing left to confirm.
  const target =
    sheet?.kind === 'claude'
      ? model?.snapshot.claude.accounts.find((a) => a.number === sheet.slot)
      : sheet?.kind === 'agy'
        ? model?.snapshot.agy.accounts.find((a) => a.number === sheet.number)
        : undefined;
  const landed = !!target?.active && model?.claudeSwitch.busy == null && model?.agySwitch.busy == null && !working;
  useEffect(() => {
    if (landed) setSheet(null);
  }, [landed]);
  const doSwitch = async (slot: number, holdUntil: Date | null) => {
    setWorking(true);
    try {
      const outcome = await api.switchClaude(slot, holdUntil ? holdUntil.toISOString() : null);
      if (latest.current) showFeedback(switchFeedback(outcome, latest.current, clock()));
    } catch (e) {
      if (latest.current) showFeedback(switchErrorFeedback(slot, e, latest.current));
    } finally {
      setWorking(false);
      setSheet(null);
    }
  };
  const doSwitchAgy = async (number: number, how: AgyHow) => {
    const login = latest.current && agyLoginOf(latest.current, number);
    if (!login) return setSheet(null);
    setWorking(true);
    setAgyHow(how);
    try {
      const outcome = await api.switchAgy(login.id, how);
      showFeedback(agySwitchFeedback(outcome, latest.current));
    } catch (e) {
      showFeedback(agySwitchErrorFeedback(number, e, latest.current));
    } finally {
      setWorking(false);
      setAgyHow(null);
      setSheet(null);
    }
  };
  const doCodexSwitch = async (target: CodexTarget, restart: boolean) => {
    setWorking(true);
    let next: SheetState | null = null;
    try {
      const outcome = await api.codexSwitch(target, latest.current?.codex.hash ?? null);
      if (outcome.kind === 'switched' && restart) {
        // G14: the box asks once more before anything is interrupted.
        codexDone.current = outcome;
        next = { kind: 'daemon', after: true };
      } else if (latest.current) showFeedback(codexFeedback(outcome, latest.current));
    } catch (e) {
      if (latest.current) showFeedback(codexErrorFeedback(target, e, latest.current));
    } finally {
      setWorking(false);
      setSheet(next);
    }
  };
  const doCodexRestart = async (after: boolean) => {
    setWorking(true);
    try {
      const out = await api.codexRestart(after);
      if (latest.current) showFeedback(codexRestartFeedback(out, latest.current, after));
    } catch (e) {
      if (latest.current) showFeedback(codexRestartFeedback(String(e) === 'busy' ? 'busy' : 'error', latest.current, after));
    } finally {
      setWorking(false);
      setSheet(null);
      codexDone.current = null;
    }
  };
  const skipCodexRestart = () => {
    setSheet(null);
    if (codexDone.current && latest.current) showFeedback(codexFeedback(codexDone.current, latest.current));
    codexDone.current = null;
  };
  const keepCodex = async () => {
    setSheet(null);
    try {
      await api.codexKeep();
    } catch {
      // The banner stays; nothing else to say.
    }
  };
  const keepOverwrite = async () => {
    try {
      await api.agyDismissOverwrite();
    } catch {
      // The banner stays; nothing else to say.
    }
  };
  const pause = async (until: Date | null) => {
    setSheet(null);
    try {
      showFeedback(pausedFeedback(await api.pauseSwitching(until ? until.toISOString() : null), clock()));
    } catch {
      showFeedback(pauseFailedFeedback(false));
    }
  };
  const resume = async () => {
    try {
      await api.resumeSwitching();
      showFeedback(resumedFeedback());
    } catch {
      showFeedback(pauseFailedFeedback(true));
    }
  };
  const addHour = async () => {
    try {
      showFeedback(extendedFeedback(await api.extendPause(60), clock()));
    } catch {
      showFeedback(pauseFailedFeedback(false));
    }
  };
  /** The toggle: on pauses through the sheet, off resumes at once. */
  const toggle = () => {
    if (model && pauseOf(model, clock())) void resume();
    else open({ kind: 'pause', preset: '60' });
  };
  /** The switcher's actions on banners and feedback lines. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'resume') void resume();
    else if (a === 'addHour') void addHour();
    else if (a === 'closeFeedback') showFeedback(null);
    else if (a === 'guide:auto' || a === 'guide:switcher') open({ kind: 'guide', id: a === 'guide:auto' ? 'auto' : 'switcher' });
    else if (a === 'agy:keep') void keepOverwrite();
    else if (a === 'codex:open' && model) open(model.codex.external ? { kind: 'codexExternal' } : { kind: 'codex', target: nextTarget(model.codex) });
    else if (a === 'codex:switch:chatGpt' || a === 'codex:switch:custom') {
      showFeedback(null);
      open({ kind: 'codex', target: a === 'codex:switch:custom' ? 'custom' : 'chatGpt' });
    } else if (a === 'codex:daemon') {
      showFeedback(null);
      open({ kind: 'daemon', after: false });
    } else if (a === 'codex:external') open({ kind: 'codexExternal' });
    else if (a === 'codex:keep') void keepCodex();
    else if (a === 'codex:check') void api.codexCheck().catch(() => {});
    else if (a === 'codex:guide:login') open({ kind: 'codexInfo', id: 'login' });
    else if (a === 'codex:backup') {
      showFeedback(null);
      open({ kind: 'codexInfo', id: 'backup' });
    }
    else if (a.startsWith('agy:switch:')) {
      showFeedback(null);
      open({ kind: 'agy', number: Number(a.slice('agy:switch:'.length)) });
    } else if (a.startsWith('switch:')) {
      showFeedback(null);
      open({ kind: 'claude', slot: Number(a.slice('switch:'.length)) });
    } else return false;
    return true;
  };
  const body = (now: Date, onSettings: () => void, onCodexGuide?: () => void) =>
    sheet && model ? (
      <SheetBody
        sheet={sheet}
        model={model}
        now={now}
        clock={clock}
        sessions={sessions}
        agyApp={agyApp}
        working={
          working ||
          (sheet.kind === 'claude' && model.claudeSwitch.busy === sheet.slot) ||
          (sheet.kind === 'agy' && model.agySwitch.busy === sheet.number) ||
          (sheet.kind === 'codex' && model.codex.busy === 'switching') ||
          (sheet.kind === 'daemon' && model.codex.busy === 'restarting')
        }
        agyHow={agyHow}
        onClose={close}
        onSwitch={(slot, until) => void doSwitch(slot, until)}
        onSwitchAgy={(number, how) => void doSwitchAgy(number, how)}
        onPause={(until) => void pause(until)}
        onSettings={onSettings}
        onCodexSwitch={(target, restart) => void doCodexSwitch(target, restart)}
        onCodexRestart={(after) => void doCodexRestart(after)}
        onCodexSkipRestart={skipCodexRestart}
        onCodexKeep={() => void keepCodex()}
        onCodexOpen={open}
        onCodexGuide={onCodexGuide}
      />
    ) : null;
  return {
    sheet,
    open,
    close,
    toggle,
    act,
    body,
    openSwitch: (slot: number) => open({ kind: 'claude', slot }),
    openSwitchAgy: (number: number) => open({ kind: 'agy', number }),
    openPause: (preset: PauseChoice) => open({ kind: 'pause', preset }),
  };
}

export interface AppProps {
  api: Api;
  clock?: () => Date;
}

const systemClock = () => new Date();

/** Before the first model: no account logged in yet. */
const NO_LOGINS: Model['antigravity'] = { client: 'ok', clientPath: '', accounts: [], indexError: false, loggingIn: false };

/** Null once a call went through, else what it was refused with. */
const refusal = (call: Promise<unknown>): Promise<string | null> =>
  call.then(
    () => null,
    (e: unknown) => String(e),
  );

export function PopoverApp({ api, clock = systemClock }: AppProps) {
  const model = useModel(api);
  const dark = useTheme(model?.theme);
  const now = useNow(clock);
  const [feedback, showFeedback] = useFeedback();
  const [keychainLater, setKeychainLater] = useState(false);
  const { busy, refresh, grant } = useActions(api, model, clock, showFeedback);
  const switching = useSwitching(api, model, clock, showFeedback);
  const sheetOpen = !!switching.sheet;
  const closeSheet = useRef(switching.close);
  closeSheet.current = switching.close;

  // Escape closes an open sheet first, then the popover. Losing focus hides
  // the popover, and a sheet that is not switching goes with it.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      if (sheetOpen) closeSheet.current();
      else void api.hidePopover();
    };
    const onBlur = () => closeSheet.current();
    window.addEventListener('keydown', onKey);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('blur', onBlur);
    };
  }, [api, sheetOpen]);

  // A button on a macOS card ("Sang …", "Chuyển ngay") opens the switch sheet here.
  const openSwitch = useRef(switching.openSwitch);
  openSwitch.current = switching.openSwitch;
  useEffect(() => {
    let live = true;
    let stop: (() => void) | undefined;
    void api.onOpenSwitch((slot) => live && openSwitch.current(slot)).then((unlisten) => {
      if (live) stop = unlisten;
      else unlisten();
    });
    return () => {
      live = false;
      stop?.();
    };
  }, [api]);

  const onHeight = useCallback((h: number) => void api.resizePopover(h), [api]);

  const onAction = (act: Action) => {
    if (switching.act(act)) return;
    if (act === 'settings') return void api.openWindow('settings');
    if (act === 'window') return void api.openWindow('overview');
    if (act === 'codex:tab') return void api.openWindow('codex');
    if (act === 'theme') return void api.setTheme(dark ? 'light' : 'dark').catch(() => showFeedback(settingsFailedFeedback()));
    if (act === 'dismiss') return setKeychainLater(true);
    if (act === 'refresh') return void refresh();
    if (act === 'keychain') return void grant();
  };

  if (!model) return null;
  const view = popoverView(model, now, { keychainLater });
  return (
    <Popover
      view={view}
      codex={codexRow(model)}
      auto={autoView(model, now)}
      sync={syncSummary(model)}
      feedback={feedback}
      busy={busy}
      dark={dark}
      onAction={onAction}
      onSwitch={switching.openSwitch}
      onSwitchAgy={switching.openSwitchAgy}
      onToggle={switching.toggle}
      sheet={switching.body(
        now,
        () => void api.openWindow('settings'),
        () => {
          switching.close();
          void api.openWindow('codex');
        },
      )}
      onCloseSheet={switching.close}
      onHeight={onHeight}
    />
  );
}

export function MainWindowApp({ api, clock = systemClock }: AppProps) {
  const model = useModel(api);
  useTheme(model?.theme);
  const now = useNow(clock);
  const [keychainLater, setKeychainLater] = useState(false);
  const [tab, setTab] = useState<Tab | null>(null);
  const [feedback, showFeedback] = useFeedback(tab);
  const agyLogins = useAgyLogins(model?.antigravity ?? NO_LOGINS, api, showFeedback);
  const { busy, refresh, grant } = useActions(api, model, clock, showFeedback);
  const switching = useSwitching(api, model, clock, showFeedback);
  const rules = useRules(api, model, clock, showFeedback, tab);
  const mcp = useMcp(api, model, showFeedback);
  const toTab = useRef<(t: Tab) => void>(() => {});
  const backup = useBackup(api, model, showFeedback, { rekey: mcp.rekey, onTab: (t) => toTab.current(t) });
  const ruleDialog = rules.leave ? 'leave' : rules.compare ? 'compare' : null;
  const sheetOpen = !!switching.sheet || !!ruleDialog || !!mcp.dialog || !!backup.dialog;
  const closeSheet = useRef(switching.close);
  // Escape closes the sheet on show: the unsaved-changes one, then a switch, then the comparison, then an MCP dialog.
  closeSheet.current = () => {
    if (rules.leave) void rules.answerLeave('cancel', setTab);
    else if (switching.sheet) switching.close();
    else if (rules.compare) rules.closeCompare();
    else if (mcp.dialog) mcp.close();
    else if (backup.dialog) backup.close();
  };

  useEffect(() => {
    if (!sheetOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeSheet.current();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [sheetOpen]);

  // The tab the window was opened for, then any tab asked for while it is open.
  useEffect(() => {
    let live = true;
    let stop: (() => void) | undefined;
    void api.onTab((t) => live && setTab(t)).then((unlisten) => {
      if (live) stop = unlisten;
      else unlisten();
    });
    void api.mainTab().then((t) => live && setTab((cur) => cur ?? t));
    return () => {
      live = false;
      stop?.();
    };
  }, [api]);

  // Opening the Antigravity tab reads every account now (at most once a minute each).
  useEffect(() => {
    if (tab === 'agy') void api.agyTabOpened().catch(() => {});
    // Opening the Codex tab looks at Codex now and counts the resumable sessions.
    if (tab === 'codex') void api.codexCheck().catch(() => {});
    // Whether macOS shows the cards may have changed in System Settings.
    if (tab === 'alerts' || tab === 'settings') void api.notifyRefresh().catch(() => {});
    // So may launching at login.
    if (tab === 'settings') void api.settingsOpened().catch(() => {});
    // Opening the MCP tab reads every agent's list now.
    if (tab === 'mcp') void api.mcpRead().catch(() => {});
    // Opening the Sao lưu tab lists the packs and the keys now; the MCP table names the keys.
    if (tab === 'backup') {
      void api.backupRead().catch(() => {});
      void api.mcpRead().catch(() => {});
    }
  }, [api, tab]);

  const onAction = (act: Action) => {
    if (switching.act(act)) return;
    if (rules.act(act)) return;
    if (mcp.act(act)) return;
    if (backup.act(act)) return;
    if (act === 'mcp:tab') return rules.guardTab(tab, 'mcp', setTab);
    if (act === 'notify:settings') return void api.openNotificationSettings().catch(() => showFeedback(settingsFailedFeedback()));
    if (act === 'settings') return rules.guardTab(tab, 'settings', setTab);
    if (act === 'codex:tab') return rules.guardTab(tab, 'codex', setTab);
    if (act === 'dismiss') return setKeychainLater(true);
    if (act === 'refresh') return void refresh();
    if (act === 'keychain') return void grant();
  };

  /** A button on a card in the Cảnh báo tab; "Sang …" opens this window's switch sheet. */
  const pressAlert = async (id: string, action: string) => {
    try {
      const pressed = await api.alertAction(id, action);
      if (pressed.openSwitch != null) switching.openSwitch(pressed.openSwitch);
    } catch (e) {
      showFeedback(alertFailedFeedback(e));
    }
  };
  const testAlert = (kind: AlertKind) => void api.testAlert(kind).catch((e: unknown) => showFeedback(testFailedFeedback(e)));
  const alertSwitch = (kind: AlertKind, on: boolean) =>
    void api.setAlertSwitch(kind, on).catch(() => showFeedback(settingsFailedFeedback()));

  if (!model || !tab) return null;
  const view = windowView(model, now, { keychainLater });
  const settings = (
    <Settings
      model={model}
      fresh={view.fresh}
      feedback={feedback}
      onAction={onAction}
      onSetPath={(path) =>
        api.setQuotaAxiPath(path).then(
          () => true,
          () => false,
        )
      }
      onFind={() => void api.setQuotaAxiPath('').catch(() => showFeedback(settingsFailedFeedback()))}
      onGrant={() => void grant()}
      onShowNames={(on) => void api.setShowNames(on).catch(() => showFeedback(settingsFailedFeedback()))}
      onTheme={(theme) => void api.setTheme(theme).catch(() => showFeedback(settingsFailedFeedback()))}
      onMenubar={(menubar) => void api.setMenubar(menubar).catch(() => showFeedback(settingsFailedFeedback()))}
      onMenubarWindow={(window) => void api.setMenubarWindow(window).catch(() => showFeedback(settingsFailedFeedback()))}
      onSetMachine={(name) =>
        api.setMachineName(name).then(
          () => true,
          () => {
            showFeedback(settingsFailedFeedback());
            return false;
          },
        )
      }
      onSetPeers={(peers) =>
        api.setPeers(peers).then(
          () => null,
          (e: unknown) => String(e),
        )
      }
      onTestPeer={(alias) => api.testPeer(alias)}
      onSetFirstmate={(path) =>
        api.setFirstmateFile(path).then(
          () => null,
          (e: unknown) => String(e),
        )
      }
      onSetCodexHome={(path) => refusal(api.setCodexHome(path))}
      onTestAlert={testAlert}
      onAlertSwitch={alertSwitch}
      onLaunchAtLogin={(on) => refusal(api.setLaunchAtLogin(on))}
      onOpenLoginItems={() => void api.openLoginItems().catch(() => showFeedback(settingsFailedFeedback()))}
      onSetSource={(path) => refusal(api.setSourceDir(path))}
      onUpdate={() => refusal(api.updateFromSource())}
      onRelaunch={() => refusal(api.relaunch())}
    />
  );
  const goTab = (next: Tab) => rules.guardTab(tab, next, setTab);
  toTab.current = goTab;
  const dialog =
    ruleDialog === 'leave' ? (
      <LeaveDialog rules={rules} onAnswer={(a) => void rules.answerLeave(a, setTab)} />
    ) : (
      (switching.body(
        now,
        () => {
          switching.close();
          goTab('settings');
        },
        () => {
          switching.close();
          goTab('codex');
        },
      ) ??
      (ruleDialog === 'compare' ? (
        <CompareDialog model={model} rules={rules} now={now} />
      ) : mcp.dialog ? (
        <McpDialogBody mcp={mcp} model={model} now={now} />
      ) : backup.dialog ? (
        <BackupDialogBody backup={backup} model={model} now={now} />
      ) : null))
    );
  const dialogSize =
    switching.sheet || ruleDialog ? undefined : mcp.dialog ? mcpDialogSize(mcp.dialog) : backup.dialog ? backupDialogSize(backup.dialog) : undefined;
  return (
    <MainWindow
      tab={tab}
      view={view}
      busy={busy}
      feedback={feedback}
      now={now}
      onTab={goTab}
      onAction={onAction}
      onSwitch={switching.openSwitch}
      onSwitchAgy={switching.openSwitchAgy}
      onToggle={switching.toggle}
      onPause={switching.openPause}
      settings={settings}
      syncNote={sidebarLine(model)}
      rules={<RulesTab model={model} fresh={view.fresh} rules={rules} feedback={feedback} now={now} onAction={onAction} />}
      codex={<CodexTab model={model} fresh={view.fresh} feedback={feedback} now={now} onAction={onAction} />}
      codexCard={<CodexCard model={model} onAction={onAction} />}
      mcp={<McpTab model={model} fresh={view.fresh} feedback={feedback} now={now} mcp={mcp} onAction={onAction} />}
      backup={<BackupTab model={model} fresh={view.fresh} feedback={feedback} now={now} backup={backup} onAction={onAction} />}
      history={<HistoryTab model={model} fresh={view.fresh} now={now} load={api.getHistory} />}
      alerts={
        <AlertsTab
          model={model}
          fresh={view.fresh}
          feedback={feedback}
          now={now}
          onAction={onAction}
          onPress={pressAlert}
          onTest={testAlert}
          onSwitch={alertSwitch}
        />
      }
      firstmate={<FirstmateTab model={model} fresh={view.fresh} now={now} onAction={onAction} />}
      unanswered={model.alerts.unanswered}
      agyLogins={agyLogins}
      dialog={dialog}
      dialogSize={dialogSize}
    />
  );
}
