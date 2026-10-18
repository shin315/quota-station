// Window roots: the popover and the settings window share one model, pushed
// by the Rust reader after every read.

import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api } from './lib/api';
import type { Model } from './lib/model';
import { keychainFeedback, popoverView, refreshedFeedback, type Action, type FeedbackView } from './lib/view';
import { Popover } from './popover/Popover';
import { Settings } from './settings/Settings';

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

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') void api.hidePopover();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [api]);

  const onHeight = useCallback((h: number) => void api.resizePopover(h), [api]);

  const onAction = async (act: Action) => {
    if (act === 'settings') return void api.openSettings();
    if (act === 'dismiss') return setKeychainLater(true);
    if (act === 'refresh') {
      setBusy('refresh');
      waitingFor.current = await api.refresh();
      return;
    }
    if (act === 'keychain') {
      setBusy('keychain');
      const result = await api.grantKeychain().catch(() => null);
      setBusy(null);
      showFeedback(keychainFeedback(result));
    }
  };

  if (!model) return null;
  const view = popoverView(model, now, { keychainLater });
  return <Popover view={view} feedback={feedback} busy={model.granting ? 'keychain' : busy} onAction={onAction} onHeight={onHeight} />;
}

export function SettingsApp({ api, clock = systemClock }: AppProps) {
  const model = useModel(api);
  const now = useNow(clock);
  const [feedback, showFeedback] = useFeedback();
  if (!model) return null;
  const fresh = popoverView(model, now).fresh;
  return (
    <Settings
      model={model}
      fresh={fresh}
      feedback={feedback}
      onSetPath={(path) =>
        api.setQuotaAxiPath(path).then(
          () => true,
          () => false,
        )
      }
      onFind={() => void api.setQuotaAxiPath('')}
      onGrant={async () => {
        const result = await api.grantKeychain().catch(() => null);
        showFeedback(keychainFeedback(result));
      }}
    />
  );
}
