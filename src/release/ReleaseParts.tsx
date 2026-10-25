// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// A newer release and the diagnostic info, as the prototype draws them
// (u-dialog, u-source, u-set…, u-diag): the dialog with what is new and
// "Tải và cài", the build from source pointed at its clone, Cài đặt ›
// Cập nhật's rows, and the Chẩn đoán card with its dialog.

import { useState } from 'react';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { Command } from '../components/blocks';
import { rowClass } from '../components/page';
import { Icon } from '../components/icons';
import { Button, Facts, FieldHint, LinkButton, Rich, Spinner, Toggle } from '../components/parts';
import { t } from '../lib/i18n';
import type { Model } from '../lib/model';
import { fromSource, megabytes, newerRelease, releaseDate, versionRow } from '../lib/release';
import type { Action } from '../lib/view';
import type { ReleaseState } from './useRelease';

const LABEL = 'fl pt-[3px] text-sm font-semi';
const PULL = 'git pull --ff-only && scripts/install.sh';

/** The clone's command and "Chép lệnh", which says "Đã chép" once it has. */
function PullCommand() {
  const [copied, setCopied] = useState(false);
  return (
    <div className="flex items-start gap-2 [&>div]:mb-0 [&>div]:flex-1">
      <Command>{PULL}</Command>
      <Button
        small
        onClick={() =>
          navigator.clipboard.writeText(PULL).then(
            () => setCopied(true),
            () => setCopied(false),
          )
        }
      >
        <Icon name={copied ? 'check' : 'copy'} />
        {copied ? t('Đã chép', 'Copied') : t('Chép lệnh', 'Copy command')}
      </Button>
    </div>
  );
}

function Head({ title, lead, cap }: { title: string; lead?: string; cap?: string }) {
  return (
    <div className="grid gap-1">
      {cap ? <span className="text-xs font-semi text-ink3">{cap}</span> : null}
      <DialogTitle className="mb-0">{title}</DialogTitle>
      {lead ? (
        <DialogLead className="mb-0">
          <Rich text={lead} />
        </DialogLead>
      ) : null}
    </div>
  );
}

/** The release's dialog (u-dialog), or the build from source's (u-source). */
export function ReleaseDialogBody({ model, release, onUpdateFromSource }: { model: Model; release: ReleaseState; onUpdateFromSource?: () => void }) {
  const l = newerRelease(model) ?? model.release.latest;
  if (!l)
    return (
      <div className="grid gap-3.5">
        <Head title={t('Không còn bản mới', 'No newer version')} lead={t('Kiểm lại trong Cài đặt › Cập nhật.', 'Check again in Settings › Updates.')} />
        <DialogButtons>
          <Button primary onClick={release.close}>
            {t('Đóng', 'Close')}
          </Button>
        </DialogButtons>
      </div>
    );
  const date = releaseDate(l.publishedAt);
  const source = fromSource(model);
  const notes = (
    <div>
      <div className="mb-1.5 text-xs font-bold text-ink2">{t('Có gì mới', 'What’s new')}</div>
      <ul className="m-0 grid list-disc gap-2 pl-3.5 text-sm text-ink marker:text-ink3">
        {l.notes.map((n) => (
          <li key={n}>{n}</li>
        ))}
      </ul>
      <div className="mt-2 text-sm">
        <LinkButton small onClick={() => release.open('notes')}>
          {t('Ghi chú đầy đủ trên GitHub', 'Full notes on GitHub')}
        </LinkButton>
      </div>
    </div>
  );
  if (source)
    return (
      <div className="grid gap-3.5">
        <Head
          cap={t('Bản mới', 'New version')}
          title={'Quota Station ' + l.version}
          lead={t('Máy này chạy bản build từ mã nguồn.', 'This Mac runs a build from source.') + (date ? t(' Phát hành ' + date + '.', ' Released ' + date + '.') : '')}
        />
        {notes}
        <Facts
          items={[
            {
              k: 'w',
              text: t(
                'Bản tải về không có phần chỉ có ở bản build này (tài khoản Antigravity, công cụ của tác giả), nên đừng cài nó đè lên.',
                'The download lacks what only this build has (Antigravity accounts, the author’s tools), so do not install it over this one.',
              ),
            },
            { k: '', text: t('Cập nhật từ bản clone của bạn:', 'Update from your clone:') },
          ]}
        />
        <PullCommand />
        <DialogButtons>
          <Button onClick={release.later}>{t('Để sau', 'Later')}</Button>
          {onUpdateFromSource ? (
            <Button
              primary
              onClick={() => {
                release.close();
                onUpdateFromSource();
              }}
            >
              {t('Cập nhật từ mã nguồn', 'Update from source')}
            </Button>
          ) : (
            <Button primary onClick={release.close}>
              {t('Xong', 'Done')}
            </Button>
          )}
        </DialogButtons>
      </div>
    );
  return (
    <div className="grid gap-3.5">
      <Head
        cap={t('Bản mới', 'New version')}
        title={'Quota Station ' + l.version}
        lead={t('Bạn đang dùng ' + model.app.version + '.', 'You have ' + model.app.version + '.') + (date ? t(' Phát hành ' + date + ' trên GitHub Releases.', ' Released ' + date + ' on GitHub Releases.') : '')}
      />
      {notes}
      <Facts
        items={[
          {
            k: '',
            text: l.dmg
              ? t(
                  'Tải và cài mở bản DMG' + (l.size ? ' ' + megabytes(l.size) : '') + ' trên GitHub. Mở nó rồi kéo Quota Station vào Applications như lần đầu.',
                  'Download and install opens the' + (l.size ? ' ' + megabytes(l.size) : '') + ' DMG on GitHub. Open it and drag Quota Station into Applications, as the first time.',
                )
              : t('Tải và cài mở trang phát hành trên GitHub.', 'Download and install opens the release page on GitHub.'),
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
    </div>
  );
}

/** "Chép thông tin chẩn đoán" (u-diag): exactly what is copied, nothing sent. */
export function DiagDialogBody({ release }: { release: ReleaseState }) {
  return (
    <div className="grid gap-3.5">
      <Head
        title={t('Thông tin chẩn đoán', 'Diagnostic info')}
        lead={t(
          'Đây đúng là phần sẽ được chép. Không có email, tên tài khoản, khoá hay đường dẫn ngoài thư mục nhà.',
          'This is exactly what gets copied. No email, account names, keys or paths outside your home folder.',
        )}
      />
      {release.diag == null ? (
        <div className="flex items-center gap-2 text-sm text-ink2">
          <Spinner />
          {t('Đang gom…', 'Gathering…')}
        </div>
      ) : (
        <pre
          className="m-0 max-h-[240px] overflow-auto rounded-tile bg-fill px-[11px] py-2 font-mono text-[11px]/[1.5] whitespace-pre-wrap text-ink"
          tabIndex={0}
          aria-label={t('Thông tin sẽ chép', 'What gets copied')}
        >
          {release.diag || t('Không gom được thông tin.', 'Could not gather the info.')}
        </pre>
      )}
      <Facts items={[{ k: '', text: t('Không gửi gì đi đâu. Dán phần này vào issue trên GitHub.', 'Nothing is sent anywhere. Paste this into a GitHub issue.') }]} />
      <DialogButtons>
        <Button onClick={() => release.open('issue')}>{t('Mở trang báo lỗi', 'Open the bug form')}</Button>
        <Button primary disabled={!release.diag} onClick={() => void release.copy()}>
          {t('Chép', 'Copy')}
        </Button>
      </DialogButtons>
    </div>
  );
}

/** Cài đặt › Cập nhật (u-set…): the version and what the last check found, and the daily check. */
export function ReleaseRows({ model, onAction, onAuto }: { model: Model; onAction(act: Action): void; onAuto(on: boolean): void }) {
  const row = versionRow(model);
  const source = fromSource(model);
  const label = source ? t('Báo khi có bản mới', 'Tell me about new versions') : t('Tự kiểm bản mới của app', 'Check for app updates');
  return (
    <>
      <div className={rowClass()}>
        <div className={LABEL}>{t('Phiên bản', 'Version')}</div>
        <div className="flex items-start gap-3">
          <div className="grid min-w-0 flex-1 gap-0.5 text-xs text-ink2">
            <span>{row.build}</span>
            <span role="status">
              {row.status}{' '}
              {row.undo ? (
                <LinkButton small onClick={() => onAction('release:unskip')}>
                  {t('Hoàn tác', 'Undo')}
                </LinkButton>
              ) : null}
            </span>
          </div>
          {row.button ? (
            <Button small primary={row.button.primary} disabled={row.button.disabled} onClick={() => onAction(row.button!.act)}>
              {row.button.label}
            </Button>
          ) : null}
        </div>
      </div>
      <div className={rowClass()}>
        <div className={LABEL}>{label}</div>
        <div>
          <Toggle on={model.release.auto} label={label} onClick={() => onAuto(!model.release.auto)} />
          <FieldHint>
            {source
              ? t(
                  'Mỗi ngày một lần qua GitHub Releases. Bản build từ mã nguồn không cài bản tải về: chỉ báo.',
                  'Once a day through GitHub Releases. A build from source never installs the download: it only tells you.',
                )
              : t(
                  'Mỗi ngày một lần qua GitHub Releases, không token, không gửi gì về máy này. Tắt thì chỉ kiểm khi bạn bấm.',
                  'Once a day through GitHub Releases, no token, nothing about this Mac sent. Off means it only checks when you press.',
                )}
          </FieldHint>
        </div>
      </div>
    </>
  );
}

/** Cài đặt › Chẩn đoán: copy the diagnostic info, open the bug form. */
export function DiagnosticsRows({ onAction }: { onAction(act: Action): void }) {
  return (
    <div className="grid gap-2.5">
      <p className="m-0 text-sm text-ink2">
        {t('Không gửi gì đi đâu. Khi báo lỗi, dán phần đã chép vào issue trên GitHub.', 'Nothing is sent anywhere. When reporting a bug, paste what you copied into a GitHub issue.')}
      </p>
      <div className="flex flex-wrap items-center gap-3">
        <Button small onClick={() => onAction('diag:open')}>
          {t('Chép thông tin chẩn đoán…', 'Copy diagnostic info…')}
        </Button>
        <LinkButton small onClick={() => onAction('release:issue')}>
          {t('Mở trang báo lỗi trên GitHub', 'Open the GitHub bug form')}
        </LinkButton>
      </div>
    </div>
  );
}
