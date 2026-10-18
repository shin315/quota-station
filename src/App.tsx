// Window roots: the popover and the main window share one model, pushed by
// the Rust reader after every read, so both always show the same snapshot.
// Each window opens its own sheets; a switch running from one shows in both.

import { useCallback, useEffect, useRef, useState } from 'react';
import { AlertsTab } from './alerts/AlertsTab';
import { AgyAccounts } from './antigravity/AgyAccounts';
import { SheetBody, type SheetState } from './components/Sheets';
import { agyLoginOf, agySwitchErrorFeedback, agySwitchFeedback, type AppAnswer } from './lib/agySwitch';
import type { Api } from './lib/api';
import { alertFailedFeedback } from './lib/alerts';
import type { AgyHow, AlertKind, Model, Sessions, Tab } from './lib/model';
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
import { keychainFeedback, popoverView, refreshedFeedback, settingsFailedFeedback, type Action, type FeedbackView } from './lib/view';
import { windowView } from './lib/window';
import { Popover } from './popover/Popover';
import { FirstmateTab } from './firstmate/FirstmateTab';
import { HistoryTab } from './history/HistoryTab';
import { CompareDialog, LeaveDialog, RulesTab } from './rules/RulesTab';
import { useRules } from './rules/useRules';
import { Settings } from './settings/Settings';
import { MainWindow } from './window/MainWindow';

/** Feedback lines clear themselves after this long. */
const FEEDBACK_MS = 8000;

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

function useFeedback(): [FeedbackView | null, (fb: FeedbackView | null) => void] {
  const [fb, setFb] = useState<FeedbackView | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const show = useCallback((next: FeedbackView | null) => {
    clearTimeout(timer.current);
    setFb(next);
    // A line with buttons waits for one of them.
    if (next && !next.actions?.length) timer.current = setTimeout(() => setFb(null), FEEDBACK_MS);
  }, []);
  useEffect(() => () => clearTimeout(timer.current), []);
  return [fb, show];
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
 * Switching Claude and Antigravity and pausing the switcher: the open sheet,
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
      showFeedback(agySwitchFeedback(await api.switchAgy(login.id, how)));
    } catch (e) {
      showFeedback(agySwitchErrorFeedback(number, e));
    } finally {
      setWorking(false);
      setAgyHow(null);
      setSheet(null);
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
    else if (a.startsWith('agy:switch:')) {
      showFeedback(null);
      open({ kind: 'agy', number: Number(a.slice('agy:switch:'.length)) });
    } else if (a.startsWith('switch:')) {
      showFeedback(null);
      open({ kind: 'claude', slot: Number(a.slice('switch:'.length)) });
    } else return false;
    return true;
  };
  const body = (now: Date, onSettings: () => void) =>
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
          (sheet.kind === 'agy' && model.agySwitch.busy === sheet.number)
        }
        agyHow={agyHow}
        onClose={close}
        onSwitch={(slot, until) => void doSwitch(slot, until)}
        onSwitchAgy={(number, how) => void doSwitchAgy(number, how)}
        onPause={(until) => void pause(until)}
        onSettings={onSettings}
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

export function PopoverApp({ api, clock = systemClock }: AppProps) {
  const model = useModel(api);
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
    if (act === 'dismiss') return setKeychainLater(true);
    if (act === 'refresh') return void refresh();
    if (act === 'keychain') return void grant();
  };

  if (!model) return null;
  const view = popoverView(model, now, { keychainLater });
  return (
    <Popover
      view={view}
      auto={autoView(model, now)}
      sync={syncSummary(model)}
      feedback={feedback}
      busy={busy}
      onAction={onAction}
      onSwitch={switching.openSwitch}
      onSwitchAgy={switching.openSwitchAgy}
      onToggle={switching.toggle}
      sheet={switching.body(now, () => void api.openWindow('settings'))}
      onCloseSheet={switching.close}
      onHeight={onHeight}
    />
  );
}

export function MainWindowApp({ api, clock = systemClock }: AppProps) {
  const model = useModel(api);
  const now = useNow(clock);
  const [feedback, showFeedback] = useFeedback();
  const [keychainLater, setKeychainLater] = useState(false);
  const [tab, setTab] = useState<Tab | null>(null);
  const { busy, refresh, grant } = useActions(api, model, clock, showFeedback);
  const switching = useSwitching(api, model, clock, showFeedback);
  const rules = useRules(api, model, clock, showFeedback, tab);
  const ruleDialog = rules.leave ? 'leave' : rules.compare ? 'compare' : null;
  const sheetOpen = !!switching.sheet || !!ruleDialog;
  const closeSheet = useRef(switching.close);
  // Escape closes the sheet on show: the unsaved-changes one, then a switch, then the comparison.
  closeSheet.current = () => {
    if (rules.leave) void rules.answerLeave('cancel', setTab);
    else if (switching.sheet) switching.close();
    else if (rules.compare) rules.closeCompare();
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
    // Whether macOS shows the cards may have changed in System Settings.
    if (tab === 'alerts' || tab === 'settings') void api.notifyRefresh().catch(() => {});
  }, [api, tab]);

  const onAction = (act: Action) => {
    if (switching.act(act)) return;
    if (rules.act(act)) return;
    if (act === 'notify:settings') return void api.openNotificationSettings().catch(() => showFeedback(settingsFailedFeedback()));
    if (act === 'settings') return rules.guardTab(tab, 'settings', setTab);
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
  const testAlert = (kind: AlertKind) => void api.testAlert(kind).catch(() => showFeedback(settingsFailedFeedback()));
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
      onShowEmail={(on) => void api.setShowEmail(on).catch(() => showFeedback(settingsFailedFeedback()))}
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
      onTestAlert={testAlert}
      onAlertSwitch={alertSwitch}
    />
  );
  const goTab = (next: Tab) => rules.guardTab(tab, next, setTab);
  const dialog =
    ruleDialog === 'leave' ? (
      <LeaveDialog rules={rules} onAnswer={(a) => void rules.answerLeave(a, setTab)} />
    ) : (
      (switching.body(now, () => {
        switching.close();
        goTab('settings');
      }) ?? (ruleDialog === 'compare' ? <CompareDialog model={model} rules={rules} now={now} /> : null))
    );
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
      agyAccounts={<AgyAccounts view={model.antigravity} api={api} onFeedback={showFeedback} />}
      dialog={dialog}
    />
  );
}
