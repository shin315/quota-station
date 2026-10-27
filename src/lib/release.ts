// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// A newer Quota Station on GitHub Releases, in words: the banner on Tổng
// quan, the line at the popover's foot, Cài đặt › Cập nhật's rows and the
// dialog. The app only reads the latest release; "Tải và cài" opens its
// disk image in the browser. A build from source (the Antigravity accounts
// or the author's tools) is told about a release but pointed at its clone,
// since the download has neither part.

import { clock } from './format';
import { mac, t } from './i18n';
import type { Model, ReleaseLatest } from './model';
import type { BannerView } from './view';

/** This copy was built from source with a part the download leaves out. */
export function fromSource(model: Pick<Model, 'capabilities'>): boolean {
  return model.capabilities.agyAccounts || model.capabilities.authorTools;
}

/** The newer release to tell about, unless the owner skipped it. */
export function newerRelease(model: Pick<Model, 'release'>): ReleaseLatest | null {
  const r = model.release;
  return r.newer && r.latest && r.skipped !== r.latest.version ? r.latest : null;
}

/** The newer release the banner and the popover tell about: not put off either. */
export function toldRelease(model: Pick<Model, 'release'>): ReleaseLatest | null {
  const l = newerRelease(model);
  return l && model.release.later !== l.version ? l : null;
}

/** `15/01/2026`, `15 Jan 2026`. */
export function releaseDate(iso: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  const pad = (n: number) => String(n).padStart(2, '0');
  const en = d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
  return t(pad(d.getDate()) + '/' + pad(d.getMonth() + 1) + '/' + d.getFullYear(), en);
}

/** `14 MB`. */
export function megabytes(bytes: number | null): string {
  return bytes ? Math.round(bytes / 1_000_000) + ' MB' : '';
}

/** Tổng quan's banner (u-overview): a plain note, under the error banners. */
export function releaseBanner(model: Model): BannerView | null {
  const l = toldRelease(model);
  if (!l) return null;
  return {
    tone: 'info',
    icon: 'note',
    title: t('Có bản mới: Quota Station ' + l.version, 'New version: Quota Station ' + l.version),
    body: fromSource(model)
      ? mac(
          t(
            'Máy này chạy bản build từ mã nguồn, nên cập nhật từ mã nguồn để giữ phần chỉ có ở bản build này.',
            'This Mac runs a build from source, so update from source to keep what only this build has.',
          ),
        )
      : t('Bạn đang dùng ' + model.app.version + '. Không có gì tự cài: app hỏi trước.', 'You have ' + model.app.version + '. Nothing installs by itself: the app asks first.'),
    actions: [
      { label: t('Xem có gì mới', 'See what’s new'), act: 'release:open' },
      { label: t('Để sau', 'Later'), act: 'release:later' },
    ],
  };
}

/** The popover's foot (u-pop): one quiet line, no red dot, no macOS card. */
export function releaseLine(model: Model): { title: string; body: string } | null {
  const l = toldRelease(model);
  if (!l) return null;
  return {
    title: t('Có bản mới ' + l.version, 'Version ' + l.version + ' is out'),
    body: fromSource(model) ? t('Bản build từ mã nguồn: cập nhật từ mã nguồn.', 'Built from source: update from source.') : t('Hỏi trước khi cài.', 'Asks before installing.'),
  };
}

/** Cài đặt › Cập nhật's version row: what the last check found, and its button. */
export interface VersionRowView {
  /** `0.1.0 · bản tải về (MIT)`. */
  build: string;
  status: string;
  button: { label: string; act: 'release:check' | 'release:open'; primary?: boolean; disabled?: boolean } | null;
  /** "Bỏ qua bản này" can be undone here. */
  undo: boolean;
}

export function versionRow(model: Model): VersionRowView {
  const r = model.release;
  const build =
    model.app.version +
    (model.app.commit ? ' (' + model.app.commit + ')' : '') +
    ' · ' +
    (fromSource(model) ? t('build từ mã nguồn', 'built from source') : t('bản tải về (MIT)', 'download (MIT)'));
  const at = r.checkedAt ? clock(new Date(r.checkedAt * 1000).toISOString()) : '';
  const check = { label: t('Kiểm bản mới', 'Check now'), act: 'release:check' as const };
  if (r.checking) return { build, status: t('Đang kiểm GitHub Releases…', 'Checking GitHub Releases…'), button: { ...check, disabled: true }, undo: false };
  if (r.error)
    return {
      build,
      status:
        r.error === 'rateLimited'
          ? t('Chưa kiểm được: GitHub đang giới hạn. Tự thử lại sau 1 giờ.', 'Could not check: GitHub is rate-limiting. Tries again in 1 hour.')
          : t('Chưa kiểm được: không kết nối được GitHub. Tự thử lại sau 1 giờ.', 'Could not check: GitHub did not answer. Tries again in 1 hour.'),
      button: { ...check, label: t('Thử lại', 'Try again') },
      undo: false,
    };
  const l = r.latest;
  if (r.newer && l && r.skipped === l.version)
    return { build, status: t('Đã bỏ qua ' + l.version + '. Báo lại khi có bản sau.', 'Skipped ' + l.version + '. You will hear about the next one.'), button: check, undo: true };
  if (r.newer && l) {
    const date = releaseDate(l.publishedAt);
    return {
      build,
      status: date ? t('Có bản ' + l.version + ' (phát hành ' + date + ').', 'Version ' + l.version + ' is out (released ' + date + ').') : t('Có bản ' + l.version + '.', 'Version ' + l.version + ' is out.'),
      button: { label: t('Xem có gì mới', 'See what’s new'), act: 'release:open', primary: true },
      undo: false,
    };
  }
  if (!r.checkedAt) return { build, status: t('Chưa kiểm lần nào.', 'Not checked yet.'), button: check, undo: false };
  if (r.none) return { build, status: t('Chưa có bản phát hành nào trên GitHub · kiểm lúc ', 'No release on GitHub yet · checked at ') + at, button: check, undo: false };
  return { build, status: t('Đang dùng bản mới nhất · kiểm lúc ', 'Up to date · checked at ') + at, button: check, undo: false };
}
