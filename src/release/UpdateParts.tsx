// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The release dialog's lower half for a copy that does not just open the
// download: a release build that installs releases itself ("Tải và cài"
// downloads, checks the signature, installs and reopens the app, with its
// progress and a clear error that falls back to the download), and a copy
// Homebrew installed (Homebrew's command to copy). A command to copy is
// drawn the same way for the build from source.

import { useState, type ReactNode } from 'react';
import { DialogButtons } from '@/components/ui/dialog';
import { Command } from '../components/blocks';
import { Glyph } from '../components/icons';
import { Button, Facts, LinkButton, Spinner } from '../components/parts';
import { os, t } from '../lib/i18n';
import { megabytes } from '../lib/release';
import { BREW_UPGRADE } from '../lib/updater';
import type { InstallState, ReleaseState } from './useRelease';

/** A command and "Chép lệnh", which says "Đã chép" once it has. */
export function CopyCommand({ command }: { command: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="flex items-start gap-2 [&>div]:mb-0 [&>div]:flex-1">
      <Command>{command}</Command>
      <Button
        small
        onClick={() =>
          navigator.clipboard.writeText(command).then(
            () => setCopied(true),
            () => setCopied(false),
          )
        }
      >
        <Glyph name={copied ? 'check' : 'copy'} bold />
        {copied ? t('Đã chép', 'Copied') : t('Chép lệnh', 'Copy command')}
      </Button>
    </div>
  );
}

/** `4 / 14 MB`, or `4 MB` before the size is known. */
function progressText(got: number, total: number | null): string {
  const mb = (n: number) => (n / 1_000_000).toFixed(n < 10_000_000 ? 1 : 0);
  return total ? mb(got) + ' / ' + mb(total) + ' MB' : mb(got) + ' MB';
}

function Progress({ got, total }: { got: number; total: number | null }) {
  const pct = total ? Math.min(100, Math.round((got / total) * 100)) : null;
  return (
    <div className="grid gap-1.5">
      <div className="flex items-center gap-2 text-sm text-ink2" role="status">
        <Spinner />
        {t('Đang tải bản mới… ', 'Downloading the new version… ')}
        <span className="tabular-nums">{progressText(got, total)}</span>
      </div>
      <div
        className="h-1.5 overflow-hidden rounded-full bg-fill"
        role="progressbar"
        aria-label={t('Tiến độ tải', 'Download progress')}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct ?? undefined}
      >
        <div className="h-full rounded-full bg-accent transition-[width] motion-reduce:transition-none" style={{ width: (pct ?? 8) + '%' }} />
      </div>
    </div>
  );
}

/** What the install is doing, and its buttons. */
function InstallSteps({ install, release }: { install: InstallState; release: ReleaseState }) {
  const closeButton = <Button onClick={release.close}>{t('Đóng', 'Close')}</Button>;
  switch (install.step) {
    case 'downloading':
      return (
        <>
          <Progress got={install.got} total={install.total} />
          <DialogButtons>{closeButton}</DialogButtons>
        </>
      );
    case 'installing':
      return (
        <>
          <div className="flex items-center gap-2 text-sm text-ink2" role="status">
            <Spinner />
            {os(t('Đang cài bản mới…', 'Installing the new version…'), t('Đang mở bộ cài…', 'Starting the installer…'))}
          </div>
          <DialogButtons>{closeButton}</DialogButtons>
        </>
      );
    case 'failed':
      return (
        <>
          <Facts
            items={[
              { k: 'x', text: install.reason },
              ...(install.detail ? [{ k: '', text: <span className="font-mono text-xs [overflow-wrap:anywhere]">{install.detail}</span>, key: 'detail' }] : []),
              {
                k: '',
                text: os(
                  t('Bạn vẫn cài được như lần đầu: mở bản DMG trên GitHub rồi kéo Quota Station vào Applications.', 'You can still install it as the first time: open the DMG on GitHub and drag Quota Station into Applications.'),
                  t('Bạn vẫn cài được như lần đầu: chạy bản cài đặt trên GitHub để cài đè lên bản này.', 'You can still install it as the first time: run the installer from GitHub over this version.'),
                ),
              },
            ]}
          />
          <DialogButtons>
            <Button onClick={release.download}>{t('Thử lại', 'Try again')}</Button>
            <Button primary onClick={release.openDownload}>
              {t('Mở trang tải về', 'Open the download')}
            </Button>
          </DialogButtons>
        </>
      );
    default:
      return null;
  }
}

/** The dialog for a release build that installs releases itself. */
export function InstallBody({ head, notes, release, size }: { head: ReactNode; notes: ReactNode; release: ReleaseState; size: number | null }) {
  const install = release.install;
  return (
    <div className="grid gap-3.5">
      {head}
      {notes}
      {install.step === 'idle' ? (
        <>
          <Facts
            items={[
              {
                k: '',
                text: os(
                  t(
                    'Tải và cài tải bản mới từ GitHub, kiểm tra chữ ký rồi cài đè lên bản này: app sẽ tự mở lại với bản mới.',
                    'Download and install gets the new version from GitHub, checks its signature and installs it over this one: the app will reopen itself on the new version.',
                  ),
                  t(
                    'Tải và cài tải bản cài đặt' + (size ? ' ' + megabytes(size) : '') + ' từ GitHub, kiểm tra chữ ký rồi cài ngay: app sẽ tự đóng lại và mở lại để cài xong.',
                    'Download and install gets the' + (size ? ' ' + megabytes(size) : '') + ' installer from GitHub, checks its signature and installs it right away: the app will close and reopen itself to finish installing.',
                  ),
                ),
              },
              { k: 'ok', text: t('Thiết lập và tài khoản giữ nguyên.', 'Settings and accounts stay as they are.') },
            ]}
          />
          <DialogButtons className="items-center">
            <LinkButton className="mr-auto" onClick={release.skip}>
              {t('Bỏ qua bản này', 'Skip this version')}
            </LinkButton>
            <Button onClick={release.later}>{t('Để sau', 'Later')}</Button>
            <Button primary onClick={release.download}>
              {t('Tải và cài', 'Download and install')}
            </Button>
          </DialogButtons>
        </>
      ) : (
        <InstallSteps install={install} release={release} />
      )}
    </div>
  );
}

/** The dialog for a copy Homebrew installed: Homebrew updates it. */
export function BrewBody({ head, notes, release }: { head: ReactNode; notes: ReactNode; release: ReleaseState }) {
  return (
    <div className="grid gap-3.5">
      {head}
      {notes}
      <Facts items={[{ k: '', text: t('Bản này cài qua Homebrew, nên cập nhật bằng Homebrew trong Terminal:', 'This copy came from Homebrew, so update it with Homebrew in Terminal:') }]} />
      <CopyCommand command={BREW_UPGRADE} />
      <DialogButtons className="items-center">
        <LinkButton className="mr-auto" onClick={release.skip}>
          {t('Bỏ qua bản này', 'Skip this version')}
        </LinkButton>
        <Button onClick={release.later}>{t('Để sau', 'Later')}</Button>
        <Button primary onClick={release.close}>
          {t('Xong', 'Done')}
        </Button>
      </DialogButtons>
    </div>
  );
}
