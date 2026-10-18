// Window roots: the popover and the main window share one model, pushed by
// the Rust reader after every read, so both always show the same snapshot.

import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api } from './lib/api';
import type { Model, Tab } from './lib/model';
import { keychainFeedback, popoverView, refreshedFeedback, settingsFailedFeedback, type Action, type FeedbackView } from './lib/view';
import { windowView } from './lib/window';
import { Popover } from './popover/Popover';
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
    if (next) timer.current = setTimeout(() => setFb(null), FEEDBACK_MS);
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

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') void api.hidePopover();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [api]);

  const onHeight = useCallback((h: number) => void api.resizePopover(h), [api]);

  const onAction = (act: Action) => {
    if (act === 'settings') return void api.openWindow('settings');
    if (act === 'window') return void api.openWindow('overview');
    if (act === 'dismiss') return setKeychainLater(true);
    if (act === 'refresh') return void refresh();
    if (act === 'keychain') return void grant();
  };

  if (!model) return null;
  const view = popoverView(model, now, { keychainLater });
  return <Popover view={view} feedback={feedback} busy={busy} onAction={onAction} onHeight={onHeight} />;
}

export function MainWindowApp({ api, clock = systemClock }: AppProps) {
  const model = useModel(api);
  const now = useNow(clock);
  const [feedback, showFeedback] = useFeedback();
  const [keychainLater, setKeychainLater] = useState(false);
  const [tab, setTab] = useState<Tab | null>(null);
  const { busy, refresh, grant } = useActions(api, model, clock, showFeedback);

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

  const onAction = (act: Action) => {
    if (act === 'settings') return setTab('settings');
    if (act === 'dismiss') return setKeychainLater(true);
    if (act === 'refresh') return void refresh();
    if (act === 'keychain') return void grant();
  };

  if (!model || !tab) return null;
  const view = windowView(model, now, { keychainLater });
  const settings = (
    <Settings
      model={model}
      fresh={view.fresh}
      feedback={feedback}
      onSetPath={(path) =>
        api.setQuotaAxiPath(path).then(
          () => true,
          () => false,
        )
      }
      onFind={() => void api.setQuotaAxiPath('').catch(() => showFeedback(settingsFailedFeedback()))}
      onGrant={() => void grant()}
      onSetMachine={(name) =>
        api.setMachineName(name).then(
          () => true,
          () => {
            showFeedback(settingsFailedFeedback());
            return false;
          },
        )
      }
    />
  );
  return (
    <MainWindow
      tab={tab}
      view={view}
      busy={busy}
      feedback={feedback}
      onTab={setTab}
      onAction={onAction}
      settings={settings}
    />
  );
}
