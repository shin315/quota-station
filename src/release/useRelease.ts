// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The main window's release and diagnostic dialogs, and what their buttons
// do: check GitHub Releases now, put a version off or skip it, "Tải và cài"
// (installed in place on a release build that updates itself, else the
// installer or page opened in the browser), and copy the diagnostic info
// for a bug report. Nothing is sent from here.

import { useRef, useState } from 'react';
import type { Api } from '../lib/api';
import { onWindows, t } from '../lib/i18n';
import type { Model } from '../lib/model';
import { newerRelease, updateWay } from '../lib/release';
import * as updater from '../lib/updater';
import type { Action, FeedbackView } from '../lib/view';

export type ReleaseDialog = 'release' | 'diag';

/** Where "Tải và cài" stands on a copy that installs releases itself. */
export type InstallState =
  | { step: 'idle' }
  | { step: 'downloading'; got: number; total: number | null }
  | { step: 'installing' }
  /** macOS: in place; runs once the app opens again. */
  | { step: 'installed'; version: string }
  /** Windows: downloaded and checked; installing closes the app and opens the new copy. */
  | { step: 'downloaded'; version: string }
  | { step: 'failed'; reason: string; detail: string };

/** The progress shown at most this often, in milliseconds. */
const PROGRESS_EVERY = 100;

const failed = (): FeedbackView => ({ tone: 'bad', title: t('Không làm được. Thử lại sau.', 'Could not do it. Try again later.'), body: '', actions: [] });

/** What went wrong, in words, and the updater's own message, short. */
export function installFailure(e: unknown): { reason: string; detail: string } {
  const raw = (e instanceof Error ? e.message : typeof e === 'string' ? e : '').trim();
  const detail = raw.length > 160 ? raw.slice(0, 160) + '…' : raw;
  if (raw === 'noUpdate')
    return { reason: t('GitHub chưa có tệp cập nhật cho máy này.', 'GitHub has no update file for this computer yet.'), detail: '' };
  if (/signature/i.test(raw))
    return { reason: t('Chữ ký của bản tải về không khớp, nên app không cài nó.', 'The download’s signature did not match, so the app did not install it.'), detail };
  return { reason: t('Không tải hoặc cài được bản mới.', 'Could not download or install the new version.'), detail };
}

export function useRelease(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const [dialog, setDialog] = useState<ReleaseDialog | null>(null);
  /** The diagnostic info as the app gave it; null while it comes. */
  const [diag, setDiag] = useState<string | null>(null);
  const [install, setInstall] = useState<InstallState>({ step: 'idle' });
  const shown = useRef(0);
  const close = () => setDialog(null);
  const quiet = (p: Promise<unknown>) => void p.catch(() => showFeedback(failed()));

  const openDiag = () => {
    setDiag(null);
    setDialog('diag');
    api.releaseDiagnostics().then(setDiag, () => setDiag(''));
  };

  /** "Chép": the text the dialog shows, then the line with the bug form. */
  const copy = async () => {
    if (!diag) return;
    try {
      await navigator.clipboard.writeText(diag);
    } catch {
      return showFeedback(failed());
    }
    setDialog(null);
    showFeedback({
      tone: '',
      title: t('Đã chép thông tin chẩn đoán.', 'Copied the diagnostic info.'),
      body: t('Dán vào issue trên GitHub. Không có gì được gửi đi.', 'Paste it into a GitHub issue. Nothing was sent.'),
      actions: [
        { label: t('Mở trang báo lỗi trên GitHub', 'Open the GitHub bug form'), act: 'release:issue' },
        { label: t('Đóng', 'Close'), act: 'closeFeedback' },
      ],
    });
  };

  const version = model ? newerRelease(model)?.version : undefined;
  const later = () => {
    setDialog(null);
    if (version) quiet(api.releaseLater(version));
  };
  const skip = () => {
    setDialog(null);
    if (version) quiet(api.releaseSkip(version));
  };
  const busy = install.step === 'downloading' || install.step === 'installing';

  /** Downloads and checks the update, then installs it on macOS; Windows waits for "Cài và mở lại". */
  const selfInstall = async () => {
    if (busy) return;
    shown.current = 0;
    setInstall({ step: 'downloading', got: 0, total: null });
    try {
      const v = await updater.downloadUpdate((got, total) => {
        const now = Date.now();
        if (now - shown.current < PROGRESS_EVERY && got !== total) return;
        shown.current = now;
        setInstall({ step: 'downloading', got, total });
      });
      if (onWindows()) return setInstall({ step: 'downloaded', version: v });
      setInstall({ step: 'installing' });
      await updater.installUpdate();
      setInstall({ step: 'installed', version: v });
    } catch (e) {
      setInstall({ step: 'failed', ...installFailure(e) });
    }
  };

  /** "Tải và cài": in place where this copy installs releases itself, else the installer in the browser. */
  const download = () => {
    if (model && updateWay(model) === 'install') return void selfInstall();
    setDialog(null);
    quiet(api.releaseOpen('download'));
  };

  /** After the download: "Mở lại" on macOS, "Cài và mở lại" on Windows (the app closes there). */
  const finish = async () => {
    try {
      if (install.step === 'downloaded') {
        setInstall({ step: 'installing' });
        await updater.installUpdate();
      } else {
        await updater.relaunch();
      }
    } catch (e) {
      setInstall({ step: 'failed', ...installFailure(e) });
    }
  };

  /** The fallback when the install failed: the installer or page in the browser, as before. */
  const openDownload = () => {
    setDialog(null);
    setInstall({ step: 'idle' });
    quiet(api.releaseOpen('download'));
  };

  /** The actions on banners, lines and Cài đặt. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'release:open') setDialog('release');
    else if (a === 'release:later') later();
    else if (a === 'release:check') quiet(api.releaseCheck());
    else if (a === 'release:unskip') quiet(api.releaseSkip(null));
    else if (a === 'release:issue') quiet(api.releaseOpen('issue'));
    else if (a === 'release:notice') quiet(api.releaseOpen('notice'));
    else if (a === 'diag:open') openDiag();
    else return false;
    return true;
  };

  return {
    dialog,
    diag,
    install,
    close,
    act,
    copy,
    later,
    skip,
    download,
    finish,
    openDownload,
    open: (what: 'notes' | 'releases' | 'issue') => quiet(api.releaseOpen(what)),
    setAuto: (on: boolean) => quiet(api.setReleaseChecks(on)),
  };
}

export type ReleaseState = ReturnType<typeof useRelease>;
