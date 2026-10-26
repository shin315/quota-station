// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Window roots: the popover and the main window share one model, pushed by
// the Rust reader after every read, so both always show the same snapshot.
// Each window opens its own sheets; a switch running from one shows in both.

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { McpDialogBody, mcpDialogSize } from './agents/McpDialogs';
import { McpTab } from './agents/McpTab';
import { useMcp } from './agents/useMcp';
import { InstrDialogBody, InstrTab, instrDialogSize, useInstr } from './agents/InstrTab';
import { PluginDialogBody, PluginTab, usePlugins } from './agents/PluginTab';
import { SkillDialogBody, skillDialogSize } from './agents/SkillDialogs';
import { SetDialogBody, SetsTab, setDialogSize } from './agents/SetsTab';
import { SkillsTab } from './agents/SkillsTab';
import { useSets } from './agents/useSets';
import { StatsTab, useStats } from './stats/StatsTab';
import { useSkills } from './agents/useSkills';
import { BackupTab } from './backup/BackupTab';
import { BackupDialogBody, backupDialogSize } from './backup/RestoreSheet';
import { CopyTab } from './backup/CopyTab';
import { CopyDialogBody, copyDialogSize } from './backup/CopySheet';
import { useCopy } from './backup/useCopy';
import { useBackup } from './backup/useBackup';
import { NotificationBell } from './alerts/Notifications';
import { CodexCard, CodexTab } from './codex/CodexTab';
import { OV_COLS } from './components/page';
import { useAgyLogins } from '@agy/AgyAccounts';
import { claudeManage } from './claude/ClaudeAccounts';
import { accountName } from './lib/claudeAccounts';
import { releaseBanner, releaseLine } from './lib/release';
import { DiagDialogBody, ReleaseDialogBody } from './release/ReleaseParts';
import { useRelease } from './release/useRelease';
import { ClaudeDialogBody } from './claude/ClaudeDialogs';
import { useClaudeAccounts } from './claude/useClaudeAccounts';
import { Feedback } from './components/parts';
import { SheetBody, type SheetState } from './components/Sheets';
import { agyLoginOf, agySwitchErrorFeedback, agySwitchFeedback, type AppAnswer } from '@agy/agySwitch';
import { codexErrorFeedback, codexFeedback, codexPopover, codexRestartFeedback, keyFeedback, nextTarget, pluginsNeedApply } from './lib/codex';
import type { Api } from './lib/api';
import { authorModel } from './lib/author';
import { alertFailedFeedback, testFailedFeedback } from './lib/alerts';
import type { AgyHow, AlertKind, CodexOutcome, CodexTarget, Language, Model, Sessions, SettingsGroup, Tab, Theme, ThemePack } from './lib/model';
import { resolveLang, setLang, systemLanguages, t, type Lang } from './lib/i18n';
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
import { EnvListBody, useEnv } from './settings/EnvCard';
import { Settings } from './settings/Settings';
import { MainWindow } from './window/MainWindow';
import { Setup } from './setup/Setup';

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
 * Puts the owner's theme pack on the page: `data-pack` beside `data-theme`
 * redefines the tokens (tokens.css), in this window as in every other.
 */
export function useThemePack(pack: ThemePack | undefined) {
  useEffect(() => {
    if (!pack) return;
    document.documentElement.dataset.pack = pack;
    document.querySelectorAll<HTMLElement>('.qs[data-pack]').forEach((el) => (el.dataset.pack = pack));
  }, [pack]);
}

/**
 * Sets the language every string below the root is written in: the owner's
 * choice, or macOS's. It is set during the root's render, before any child
 * builds its text, and `<html lang>` follows for VoiceOver.
 */
export function useLang(language: Language | undefined): Lang {
  const next = resolveLang(language, systemLanguages());
  setLang(next);
  useEffect(() => {
    document.documentElement.lang = next;
  }, [next]);
  return next;
}

/**
 * The clear glass, or the solid ground while macOS reduces transparency:
 * `data-material` on the page drives the glass tokens. The stronger edges
 * while macOS increases contrast: `data-contrast`. macOS says so itself
 * (WebKit's media queries cannot be counted on), when the window opens and
 * whenever it comes forward after a setting changed.
 */
export function useMaterial(api: Api) {
  useEffect(() => {
    let live = true;
    const stops: (() => void)[] = [];
    const set = (key: 'material' | 'contrast', value: string) => {
      if (!live) return;
      document.documentElement.dataset[key] = value;
      document.querySelectorAll<HTMLElement>(`.qs[data-${key}]`).forEach((el) => (el.dataset[key] = value));
    };
    const material = (reduce: boolean) => set('material', reduce ? 'solid' : 'glass');
    const contrast = (more: boolean) => set('contrast', more ? 'more' : 'normal');
    const keep = (unlisten: () => void) => {
      if (live) stops.push(unlisten);
      else unlisten();
    };
    void api.reduceTransparency().then(material, () => {});
    void api.onReduceTransparency(material).then(keep);
    void api.increaseContrast().then(contrast, () => {});
    void api.onIncreaseContrast(contrast).then(keep);
    return () => {
      live = false;
      stops.forEach((stop) => stop());
    };
  }, [api]);
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
  const landed = !!target?.active && model?.claudeSwitch.busy == null && model?.agySwitch?.busy == null && !working;
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
  const doCodexSetKey = async (key: string, baseUrl: string | null, base: string, undo: NonNullable<Model['codex']['keyUndo']> | null) => {
    setWorking(true);
    try {
      const out = await api.codexSetKey(key, baseUrl, base);
      if (latest.current) showFeedback(keyFeedback(out, latest.current, undo));
    } catch (e) {
      showFeedback({
        tone: 'bad',
        title: t('Chưa ghi khoá', 'The key is not written'),
        body: String(e) === 'busy' ? t('Một lần chuyển hoặc ghi khác đang chạy. Chờ xong rồi thử lại.', 'Another switch or write is running. Wait for it, then try again.') : t('Không gọi được lệnh ghi khoá. Chưa đổi gì.', 'Could not run the key write. Nothing changed.'),
        actions: [{ label: t('Đóng', 'Close'), act: 'closeFeedback' }],
      });
    } finally {
      setWorking(false);
      setSheet(null);
    }
  };
  const undoCodexKey = async () => {
    const base = latest.current?.codex.hash;
    if (!base) return;
    try {
      const out = await api.codexUndoKey(base);
      if (latest.current) showFeedback(keyFeedback(out, latest.current));
    } catch (e) {
      showFeedback({
        tone: 'bad',
        title: t('Chưa hoàn tác', 'Not undone'),
        body: String(e) === 'busy' ? t('Một lần chuyển hoặc ghi khác đang chạy. Chờ xong rồi thử lại.', 'Another switch or write is running. Wait for it, then try again.') : t('Không gọi được lệnh. Chưa đổi gì.', 'Could not run the command. Nothing changed.'),
        actions: [{ label: t('Đóng', 'Close'), act: 'closeFeedback' }],
      });
    }
  };
  /** The plugins box: kept at once; on the custom provider in use, "Áp dụng ngay?" follows. */
  const setCodexPlugins = async (on: boolean, offer: boolean) => {
    const before = latest.current?.codex;
    try {
      await api.setCodexPlugins(on);
    } catch {
      return showFeedback(settingsFailedFeedback());
    }
    if (offer && before && pluginsNeedApply(before, on)) open({ kind: 'codexPlugins', on });
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
    else if (a === 'claude:rotation:on') void api.claudeSetRotation(true).catch(() => {});
    else if (a === 'claude:import') void api.claudeImport().catch(() => {});
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
    else if (a === 'codex:key') {
      showFeedback(null);
      open({ kind: 'codexKey' });
    } else if (a === 'codex:plugins:apply' && model) {
      showFeedback(null);
      open({ kind: 'codexPlugins', on: model.codex.plugins });
    } else if (a === 'codex:undoKey') {
      showFeedback(null);
      void undoCodexKey();
    }
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
          (sheet.kind === 'agy' && model.agySwitch?.busy === sheet.number) ||
          ((sheet.kind === 'codex' || sheet.kind === 'codexKey' || sheet.kind === 'codexPlugins') && model.codex.busy === 'switching') ||
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
        onCodexPlugins={(on) => void setCodexPlugins(on, false)}
        onCodexKeyPreview={(key, baseUrl) => api.codexKeyPreview(key, baseUrl)}
        onCodexSetKey={(key, baseUrl, base, undo) => void doCodexSetKey(key, baseUrl, base, undo)}
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
    setCodexPlugins: (on: boolean) => void setCodexPlugins(on, true),
  };
}

export interface AppProps {
  api: Api;
  clock?: () => Date;
}

const systemClock = () => new Date();

/** Before the first model: no account logged in yet. */
const NO_LOGINS: NonNullable<Model['antigravity']> = { client: 'ok', clientPath: '', accounts: [], indexError: false, loggingIn: false };

/** Null once a call went through, else what it was refused with. */
const refusal = (call: Promise<unknown>): Promise<string | null> =>
  call.then(
    () => null,
    (e: unknown) => String(e),
  );

export function PopoverApp({ api, clock = systemClock }: AppProps) {
  const model = useModel(api);
  const dark = useTheme(model?.theme);
  useThemePack(model?.themePack);
  useLang(model?.language);
  useMaterial(api);
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
    // The bell: the main window with its panel open.
    if (act === 'notifications') return void api.openWindow('alerts');
    if (act === 'codex:tab') return void api.openWindow('codex');
    // Signing in again happens in the main window's Claude tab.
    if (act.startsWith('claude:relogin:') || act === 'claude:add') return void api.openWindow('claude');
    // The release's banner, with "Xem có gì mới", is on Tổng quan.
    if (act === 'release:open') return void api.openWindow('overview');
    if (act === 'theme') return void api.setTheme(dark ? 'light' : 'dark').catch(() => showFeedback(settingsFailedFeedback()));
    if (act === 'dismiss') return setKeychainLater(true);
    if (act === 'refresh') return void refresh();
    if (act === 'keychain') return void grant();
  };

  if (!model) return null;
  const view = popoverView(model, now, { keychainLater });
  const author = authorModel(model);
  return (
    <Popover
      view={view}
      codex={codexPopover(model, now)}
      auto={model.advanced.on ? autoView(model, now) : null}
      sync={author && model.advanced.on ? syncSummary(author) : null}
      release={releaseLine(model)}
      unread={model.alerts.unread}
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
  useThemePack(model?.themePack);
  useLang(model?.language);
  useMaterial(api);
  const now = useNow(clock);
  const [keychainLater, setKeychainLater] = useState(false);
  const [tab, setTab] = useState<Tab | null>(null);
  // The bell's panel (Thông báo), over whichever page is in view.
  const [bell, setBell] = useState(false);
  // Cài đặt's tab: the app keeps it while it runs, so the window opens on it again.
  const [settingsGroup, setSettingsGroup] = useState<SettingsGroup>('general');
  const showGroup = (group: SettingsGroup) => {
    setSettingsGroup(group);
    void api.setSettingsGroup(group).catch(() => {});
  };
  const [feedback, showFeedback] = useFeedback(tab);
  const agyLogins = useAgyLogins(model?.antigravity ?? NO_LOGINS, api, showFeedback);
  const { busy, refresh, grant } = useActions(api, model, clock, showFeedback);
  const switching = useSwitching(api, model, clock, showFeedback);
  const rules = useRules(api, model, clock, showFeedback, tab);
  const mcp = useMcp(api, model, showFeedback);
  const skills = useSkills(api, model, showFeedback);
  const plugins = usePlugins();
  const instr = useInstr(api, model, showFeedback);
  const toTab = useRef<(t: Tab) => void>(() => {});
  const backup = useBackup(api, model, showFeedback, { rekey: mcp.rekey, onTab: (t) => toTab.current(t) });
  const copy = useCopy(api, model, showFeedback);
  const env = useEnv(api, model, showFeedback);
  const onActionRef = useRef<(a: Action) => void>(() => {});
  const sets = useSets(api, model, showFeedback, (a) => onActionRef.current(a));
  const stats = useStats(api, model, tab === 'stats');
  // The Claude tab's dialogs name an account as its table does.
  const claudeNames = useRef<(slot: number) => string>((slot) => t('Tài khoản ', 'Account ') + slot);
  const claude = useClaudeAccounts(api, model, showFeedback, (slot) => claudeNames.current(slot));
  const release = useRelease(api, model, showFeedback);
  const ruleDialog = rules.leave ? 'leave' : rules.compare ? 'compare' : null;
  const sheetOpen =
    !!switching.sheet || !!ruleDialog || !!mcp.dialog || !!backup.dialog || !!copy.dialog || !!skills.dialog || plugins.open || !!instr.dialog || env.list || !!sets.dialog || !!claude.dialog || !!release.dialog;
  const closeSheet = useRef(switching.close);
  // Escape closes the sheet on show: the unsaved-changes one, then a switch, then the comparison, then an MCP dialog.
  closeSheet.current = () => {
    if (rules.leave) void rules.answerLeave('cancel', setTab);
    else if (switching.sheet) switching.close();
    else if (rules.compare) rules.closeCompare();
    else if (mcp.dialog) mcp.close();
    else if (backup.dialog) backup.close();
    else if (copy.dialog) copy.close();
    else if (skills.dialog) skills.close();
    else if (plugins.open) plugins.close();
    else if (instr.dialog) instr.close();
    else if (env.list) env.close();
    else if (sets.dialog) sets.close();
    // The wait for the browser ends with its own Thôi, which stops the sign-in.
    else if (claude.dialog && claude.dialog.kind !== 'wait') claude.close();
    else if (release.dialog) release.close();
  };

  useEffect(() => {
    if (!sheetOpen) return;
    // A dialog takes the window: the bell's panel gives way.
    setBell(false);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeSheet.current();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [sheetOpen]);

  // The tab the window was opened for, then any tab asked for while it is open.
  // `alerts` (a macOS card, the popover's bell) opens the bell's panel over the
  // page in view; any other tab closes it, as following a link from it does.
  // Cài đặt opens on the group the app keeps: the last one, or the one an alert's "Xem" asked for.
  useEffect(() => {
    let live = true;
    let stop: (() => void) | undefined;
    const group = () =>
      api.settingsGroup().then(
        (g) => live && setSettingsGroup(g),
        () => {},
      );
    const arrive = (t: Tab, first: boolean) => {
      if (!live) return;
      if (t === 'alerts') {
        setBell(true);
        setTab((cur) => cur ?? 'overview');
      } else if (first) setTab((cur) => cur ?? t);
      else {
        setBell(false);
        setTab(t);
      }
    };
    void api
      .onTab((t) => {
        if (t === 'settings') void group().then(() => arrive(t, false));
        else arrive(t, false);
      })
      .then((unlisten) => {
        if (live) stop = unlisten;
        else unlisten();
      });
    void group()
      .then(() => api.mainTab())
      .then((t) => arrive(t, true));
    return () => {
      live = false;
      stop?.();
    };
  }, [api]);

  // Opening the Antigravity tab reads every account now (at most once a minute each).
  // A layout effect: the read is asked in the same commit that shows the tab,
  // not in a passive effect that may run some time after the tab is on screen.
  useLayoutEffect(() => {
    if (tab === 'agy') void api.agyTabOpened().catch(() => {});
    // Opening the Codex tab looks at Codex now and counts the resumable sessions.
    if (tab === 'codex') void api.codexCheck().catch(() => {});
    // Whether macOS shows the cards may have changed in System Settings.
    if (tab === 'settings') void api.notifyRefresh().catch(() => {});
    // So may launching at login, and the shell files: the variables are looked at again.
    if (tab === 'settings') {
      void api.settingsOpened().catch(() => {});
      void api.envCheck().catch(() => {});
    }
    // Opening Thống kê reads what the session logs gained since the last read.
    if (tab === 'stats') void api.statsRead().catch(() => {});
    // Opening the MCP tab reads every agent's list now.
    if (tab === 'mcp') void api.mcpRead().catch(() => {});
    // Opening Skills, Plugin or File hướng dẫn reads them now.
    if (tab === 'skills') void api.skillsRead().catch(() => {});
    if (tab === 'plugins') void api.pluginsRead().catch(() => {});
    if (tab === 'instr') void api.instrRead().catch(() => {});
    // Opening Bộ cấu hình reads the MCP and Skills tables: the editor lists them, the drift needs them.
    if (tab === 'sets') {
      void api.mcpRead().catch(() => {});
      void api.skillsRead().catch(() => {});
    }
    // Opening the Sao lưu tab lists the packs and the keys now; the MCP table names the keys.
    if (tab === 'backup') {
      void api.backupRead().catch(() => {});
      void api.mcpRead().catch(() => {});
    }
  }, [api, tab]);

  const onAction = (act: Action) => {
    // The Claude accounts' actions first: here they have the tab's dialogs and lines.
    if (claude.act(act)) {
      // A sign-in or an add asked from another tab's banner opens over the Claude tab.
      if (act === 'claude:add' || act.startsWith('claude:relogin:')) rules.guardTab(tab, 'claude', setTab);
      return;
    }
    if (release.act(act)) {
      // The release's dialog opens over Tổng quan, the diagnostic info's over Cài đặt.
      if (act === 'release:open') rules.guardTab(tab, 'overview', setTab);
      return;
    }
    if (switching.act(act)) return;
    if (rules.act(act)) return;
    if (mcp.act(act)) return;
    if (backup.act(act)) return;
    if (skills.act(act)) return;
    if (instr.act(act)) return;
    if (env.act(act)) return;
    if (sets.act(act)) return;
    if (act === 'env:settings') {
      showGroup('services');
      return rules.guardTab(tab, 'settings', setTab);
    }
    if (act === 'settings:advanced') {
      showGroup('advanced');
      return rules.guardTab(tab, 'settings', setTab);
    }
    if (act === 'plugins:reread') return void api.pluginsRead().catch(() => {});
    if (act === 'mcp:tab') return rules.guardTab(tab, 'mcp', setTab);
    if (act === 'notify:settings') return void api.openNotificationSettings().catch(() => showFeedback(settingsFailedFeedback()));
    if (act === 'settings') return rules.guardTab(tab, 'settings', setTab);
    if (act === 'codex:tab') return rules.guardTab(tab, 'codex', setTab);
    if (act === 'dismiss') return setKeychainLater(true);
    if (act === 'refresh') return void refresh();
    if (act === 'keychain') return void grant();
  };
  onActionRef.current = onAction;

  /** A button on a card under the bell; "Sang …" opens this window's switch sheet. What went wrong shows in the panel. */
  const pressAlert = async (id: string, action: string) => {
    try {
      const pressed = await api.alertAction(id, action);
      if (pressed.openSwitch != null) switching.openSwitch(pressed.openSwitch);
      return null;
    } catch (e) {
      return alertFailedFeedback(e);
    }
  };
  const openBell = (open: boolean) => {
    setBell(open);
    // Whether macOS shows the cards may have changed in System Settings.
    if (open) void api.notifyRefresh().catch(() => {});
  };
  const testAlert = (kind: AlertKind) => void api.testAlert(kind).catch((e: unknown) => showFeedback(testFailedFeedback(e)));
  const alertSwitch = (kind: AlertKind, on: boolean) =>
    void api.setAlertSwitch(kind, on).catch(() => showFeedback(settingsFailedFeedback()));

  if (!model || !tab) return null;
  const view = windowView(model, now, { keychainLater });
  // The rules editor, the copy to another Mac and Firstmate: only from a build with the author's tools.
  const author = authorModel(model);
  const settings = (
    <Settings
      model={model}
      fresh={view.fresh}
      feedback={feedback}
      onAction={onAction}
      onGrant={() => void grant()}
      onShowNames={(on) => void api.setShowNames(on).catch(() => showFeedback(settingsFailedFeedback()))}
      onTheme={(theme) => void api.setTheme(theme).catch(() => showFeedback(settingsFailedFeedback()))}
      onThemePack={(pack) => void api.setThemePack(pack).catch(() => showFeedback(settingsFailedFeedback()))}
      onLanguage={(language) => void api.setLanguage(language).catch(() => showFeedback(settingsFailedFeedback()))}
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
      onSetBackupFolder={(path) => refusal(api.backupSetFolder(path))}
      onPoolAlert={(dry, room) => refusal(api.setPoolAlert(dry, room))}
      onAdvanced={(on) => void api.setAdvanced(on).catch(() => showFeedback(settingsFailedFeedback()))}
      onFirstmate={(on) => void api.setFirstmate(on).catch(() => showFeedback(settingsFailedFeedback()))}
      onTestAlert={testAlert}
      onAlertSwitch={alertSwitch}
      onLaunchAtLogin={(on) => refusal(api.setLaunchAtLogin(on))}
      onOpenLoginItems={() => void api.openLoginItems().catch(() => showFeedback(settingsFailedFeedback()))}
      onSetSource={(path) => refusal(api.setSourceDir(path))}
      onUpdate={() => refusal(api.updateFromSource())}
      onRelaunch={() => refusal(api.relaunch())}
      onSkillChecks={(daily, notify) => void api.skillsSetChecks(daily, notify).catch(() => showFeedback(settingsFailedFeedback()))}
      onReleaseChecks={release.setAuto}
      group={settingsGroup}
      onGroup={showGroup}
    />
  );
  const goTab = (next: Tab) => {
    setBell(false);
    rules.guardTab(tab, next, setTab);
  };
  /** "Cài đặt thông báo" under the bell: Cài đặt, at its Thông báo group. */
  const notifySettings = () => {
    setBell(false);
    rules.guardTab(tab, 'settings', (t) => {
      showGroup('notify');
      setTab(t);
    });
  };
  toTab.current = goTab;
  const shownNames = new Map(view.claude.cards.map((c) => [c.slot, c.name]));
  const nameOf = (slot: number) => shownNames.get(slot) ?? accountName(model.claudeAccounts.accounts.find((a) => a.slot === slot) ?? { slot, alias: null });
  claudeNames.current = nameOf;
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
      (ruleDialog === 'compare' && author ? (
        <CompareDialog model={author} rules={rules} now={now} />
      ) : mcp.dialog ? (
        <McpDialogBody mcp={mcp} model={model} now={now} />
      ) : backup.dialog ? (
        <BackupDialogBody backup={backup} model={model} now={now} />
      ) : copy.dialog ? (
        <CopyDialogBody copy={copy} />
      ) : skills.dialog ? (
        <SkillDialogBody skills={skills} model={model} />
      ) : plugins.open ? (
        <PluginDialogBody model={model} plugins={plugins} />
      ) : instr.dialog ? (
        <InstrDialogBody instr={instr} model={model} now={now} />
      ) : env.list ? (
        <EnvListBody model={model} env={env} />
      ) : sets.dialog ? (
        <SetDialogBody sets={sets} model={model} />
      ) : release.dialog === 'release' ? (
        <ReleaseDialogBody model={model} release={release} onUpdateFromSource={author && model.advanced.on ? () => void refusal(api.updateFromSource()) : undefined} />
      ) : release.dialog === 'diag' ? (
        <DiagDialogBody release={release} />
      ) : claude.dialog ? (
        <ClaudeDialogBody claude={claude} view={model.claudeAccounts} advanced={model.advanced.on} nameOf={nameOf} onTab={goTab} />
      ) : null))
    );
  const dialogSize =
    switching.sheet || ruleDialog
      ? undefined
      : mcp.dialog
        ? mcpDialogSize(mcp.dialog)
        : backup.dialog
          ? backupDialogSize(backup.dialog)
          : copy.dialog
            ? copyDialogSize(copy.dialog, copy)
            : skills.dialog
            ? skillDialogSize(skills.dialog)
            : plugins.open
              ? 'xl'
              : instr.dialog
                ? instrDialogSize(instr.dialog)
                : env.list
                  ? 'lg'
                  : sets.dialog
                    ? setDialogSize(sets.dialog)
                    : claude.dialog || release.dialog
                      ? 'lg'
                      : undefined;
  return (
    <MainWindow
      tab={tab}
      release={releaseBanner(model)}
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
      syncNote={author ? sidebarLine(author) : null}
      rules={author ? <RulesTab model={author} fresh={view.fresh} rules={rules} feedback={feedback} now={now} onAction={onAction} /> : null}
      codex={<CodexTab model={model} fresh={view.fresh} feedback={feedback} now={now} onAction={onAction} onPlugins={switching.setCodexPlugins} />}
      codexCard={<CodexCard model={model} now={now} cols={OV_COLS.codex} onAction={onAction} />}
      mcp={<McpTab model={model} fresh={view.fresh} feedback={feedback} now={now} mcp={mcp} onAction={onAction} />}
      skills={<SkillsTab model={model} fresh={view.fresh} feedback={feedback} skills={skills} onAction={onAction} />}
      plugins={<PluginTab model={model} fresh={view.fresh} plugins={plugins} onAction={onAction} />}
      stats={<StatsTab model={model} fresh={view.fresh} stats={stats} />}
      sets={<SetsTab model={model} fresh={view.fresh} feedback={feedback} now={now} sets={sets} onAction={onAction} />}
      instr={<InstrTab model={model} fresh={view.fresh} feedback={feedback} now={now} instr={instr} onAction={onAction} />}
      backup={<BackupTab model={model} fresh={view.fresh} feedback={feedback} now={now} backup={backup} onAction={onAction} />}
      copy={author ? <CopyTab model={author} feedback={feedback} copy={copy} onAction={onAction} /> : null}
      history={<HistoryTab model={model} fresh={view.fresh} now={now} load={api.getHistory} />}
      firstmate={author ? <FirstmateTab model={author} fresh={view.fresh} now={now} onAction={onAction} /> : null}
      bell={
        <NotificationBell
          model={model}
          now={now}
          open={bell}
          onOpenChange={openBell}
          onRead={() => void api.alertsRead().catch(() => {})}
          onPress={pressAlert}
          onAction={onAction}
          onSettings={notifySettings}
        />
      }
      advanced={model.advanced.on}
      firstmateTab={model.advanced.on && model.advanced.firstmate}
      agyAccounts={model.capabilities.agyAccounts}
      agyLogins={agyLogins}
      claude={claudeManage({ view: model.claudeAccounts, claude, advanced: model.advanced.on, onAction, authorTools: model.capabilities.authorTools })}
      // The first run waits for the browser on its own page, not in a dialog.
      dialog={model.firstRun && claude.dialog?.kind === 'wait' ? null : dialog}
      dialogSize={dialogSize}
      setup={
        model.firstRun ? (
          <Setup
            model={model}
            claude={claude}
            busy={busy}
            notice={feedback && feedback.tone === 'bad' ? <Feedback fb={feedback} onAction={onAction} /> : null}
            onAction={onAction}
            onLanguage={(language) => void api.setLanguage(language).catch(() => showFeedback(settingsFailedFeedback()))}
            onAddCurrent={() => void api.claudeAddCurrent().catch(() => {})}
            onSetAlias={(slot, alias) => void api.claudeSetAlias(slot, alias).catch(() => showFeedback(settingsFailedFeedback()))}
            onRemove={(slot) => void api.claudeRemove(slot).catch(() => showFeedback(settingsFailedFeedback()))}
            onLaunchAtLogin={(on) => refusal(api.setLaunchAtLogin(on))}
            onOpenLoginItems={() => void api.openLoginItems().catch(() => showFeedback(settingsFailedFeedback()))}
            onMenubar={(menubar) => void api.setMenubar(menubar).catch(() => showFeedback(settingsFailedFeedback()))}
            onMenubarWindow={(window) => void api.setMenubarWindow(window).catch(() => showFeedback(settingsFailedFeedback()))}
            onFinish={(open) => void api.finishSetup(open).catch(() => showFeedback(settingsFailedFeedback()))}
          />
        ) : undefined
      }
    />
  );
}
