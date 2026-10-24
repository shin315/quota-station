// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The main window's release and diagnostic dialogs, and what their buttons
// do: check GitHub Releases now, put a version off or skip it, open its disk
// image or page in the browser, and copy the diagnostic info for a bug
// report. Nothing is downloaded, installed or sent from here.

import { useState } from 'react';
import type { Api } from '../lib/api';
import { t } from '../lib/i18n';
import type { Model } from '../lib/model';
import { newerRelease } from '../lib/release';
import type { Action, FeedbackView } from '../lib/view';

export type ReleaseDialog = 'release' | 'diag';

const failed = (): FeedbackView => ({ tone: 'bad', title: t('Không làm được. Thử lại sau.', 'Could not do it. Try again later.'), body: '', actions: [] });

export function useRelease(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const [dialog, setDialog] = useState<ReleaseDialog | null>(null);
  /** The diagnostic info as the app gave it; null while it comes. */
  const [diag, setDiag] = useState<string | null>(null);
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
  const download = () => {
    setDialog(null);
    quiet(api.releaseOpen('download'));
  };

  /** The actions on banners, lines and Cài đặt. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'release:open') setDialog('release');
    else if (a === 'release:later') later();
    else if (a === 'release:check') quiet(api.releaseCheck());
    else if (a === 'release:unskip') quiet(api.releaseSkip(null));
    else if (a === 'release:issue') quiet(api.releaseOpen('issue'));
    else if (a === 'diag:open') openDiag();
    else return false;
    return true;
  };

  return {
    dialog,
    diag,
    close,
    act,
    copy,
    later,
    skip,
    download,
    open: (what: 'notes' | 'releases' | 'issue') => quiet(api.releaseOpen(what)),
    setAuto: (on: boolean) => quiet(api.setReleaseChecks(on)),
  };
}

export type ReleaseState = ReturnType<typeof useRelease>;
