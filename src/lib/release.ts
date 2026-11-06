// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// A newer Quota Station on GitHub Releases, in words: the banner on Tổng
// quan, the line at the popover's foot, Cài đặt › Cập nhật's rows and the
// dialog. The app reads the latest release; "Tải và cài" installs it on a
// release build that updates itself (`release.way`), shows Homebrew's command
// to a copy Homebrew installed, and else opens its installer in the
// browser. A build from source (every build but the release workflow's) is
// told about a release but pointed at its clone.

import { sourceView } from './advanced';
import { clock } from './format';
import { mac, t } from './i18n';
import type { Model, ReleaseLatest } from './model';
import type { BannerView } from './view';

/**
 * This copy was built from source: any build without the release marker
 * (`capabilities.releaseBuild`, set only by the release workflow). What the
 * build has does not count: the download has the advanced part and the
 * Antigravity accounts too.
 */
export function fromSource(model: Pick<Model, 'capabilities'>): boolean {
  return model.capabilities.releaseBuild !== true;
}

/** How a build from source is told to update: rebuilt from its clone, or updated from source where it has that. */
function sourceWords(model: Pick<Model, 'app'>): { banner: string; line: string } {
  let w = {
    banner: t(
      'Máy này chạy bản build từ bản clone của bạn, nên build lại từ bản clone đó.',
      'This Mac runs a build from your clone, so rebuild it from that clone.',
    ),
    line: t('Bản build từ mã nguồn: build lại từ bản clone.', 'Built from source: rebuild from your clone.'),
  };
  // Windows has no update from source: rebuilt from the clone, as above.
  if (sourceView(model.app))
    w = {
      banner: t(
        'Máy này chạy bản build từ bản clone của bạn, nên cập nhật nó từ mã nguồn.',
        'This Mac runs a build from your clone, so update it from source.',
      ),
      line: t('Bản build từ mã nguồn: cập nhật từ mã nguồn.', 'Built from source: update from source.'),
    };
  return w;
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
      ? mac(sourceWords(model).banner)
      : model.release.way === 'brew'
        ? t('Bạn đang dùng ' + model.app.version + ', cài qua Homebrew: cập nhật bằng Homebrew.', 'You have ' + model.app.version + ', installed with Homebrew: update it with Homebrew.')
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
    body: fromSource(model)
      ? sourceWords(model).line
      : model.release.way === 'brew'
        ? t('Cập nhật bằng Homebrew.', 'Update with Homebrew.')
        : t('Hỏi trước khi cài.', 'Asks before installing.'),
  };
}

/** Cài đặt › Cập nhật's version row: what the last check found, and its button. */
export interface VersionRowView {
  /** `0.1.0 (build 412 · d6ae958) · bản tải về`. */
  build: string;
  status: string;
  /** Whether this copy is the newest: `ok` yes, `info` a newer one is out, `bad` the check failed, `off` not checked or skipped. */
  tone: 'ok' | 'info' | 'bad' | 'off' | '';
  button: { label: string; act: 'release:check' | 'release:open'; primary?: boolean; disabled?: boolean } | null;
  /** "Bỏ qua bản này" can be undone here. */
  undo: boolean;
}

/** What tells one build from another after the version: `build 412 · d6ae958`, either part alone, or empty. */
export function buildTag(app: { build: string | null; commit: string | null }): string {
  return [app.build ? 'build ' + app.build : '', app.commit ?? ''].filter(Boolean).join(' · ');
}

/** `0.1.0 (build 412 · d6ae958)`, or the version alone when nothing tells the build. */
export function versionText(app: Pick<Model['app'], 'version' | 'build' | 'commit'>): string {
  const tag = buildTag(app);
  return app.version + (tag ? ' (' + tag + ')' : '');
}

export function versionRow(model: Model): VersionRowView {
  const r = model.release;
  const build =
    versionText(model.app) +
    ' · ' +
    (fromSource(model) ? t('build từ mã nguồn', 'built from source') : t('bản tải về', 'download'));
  const at = r.checkedAt ? clock(new Date(r.checkedAt * 1000).toISOString()) : '';
  const check = { label: t('Kiểm tra bản mới', 'Check for new versions'), act: 'release:check' as const };
  if (r.checking) return { build, status: t('Đang kiểm tra GitHub Releases…', 'Checking GitHub Releases…'), tone: '', button: { ...check, disabled: true }, undo: false };
  if (r.error)
    return {
      build,
      status:
        r.error === 'rateLimited'
          ? t('Chưa kiểm tra được: GitHub đang giới hạn. Tự thử lại sau 1 giờ.', 'Could not check: GitHub is rate-limiting. Tries again in 1 hour.')
          : t('Chưa kiểm tra được: không kết nối được GitHub. Tự thử lại sau 1 giờ.', 'Could not check: GitHub did not answer. Tries again in 1 hour.'),
      tone: 'bad',
      button: { ...check, label: t('Thử lại', 'Try again') },
      undo: false,
    };
  const l = r.latest;
  if (r.newer && l && r.skipped === l.version)
    return { build, status: t('Đã bỏ qua ' + l.version + '. Báo lại khi có bản sau.', 'Skipped ' + l.version + '. You will hear about the next one.'), tone: 'off', button: check, undo: true };
  if (r.newer && l) {
    const date = releaseDate(l.publishedAt);
    return {
      build,
      status: date ? t('Có bản ' + l.version + ' (phát hành ' + date + ').', 'Version ' + l.version + ' is out (released ' + date + ').') : t('Có bản ' + l.version + '.', 'Version ' + l.version + ' is out.'),
      tone: 'info',
      button: { label: t('Xem có gì mới', 'See what’s new'), act: 'release:open', primary: true },
      undo: false,
    };
  }
  if (!r.checkedAt) return { build, status: t('Chưa kiểm tra lần nào.', 'Not checked yet.'), tone: 'off', button: check, undo: false };
  // No release yet: nothing is newer than this copy.
  if (r.none) return { build, status: t('Đã là bản mới nhất: GitHub chưa có bản phát hành nào · kiểm tra lúc ', 'Already the newest: no release on GitHub yet · checked at ') + at, tone: 'ok', button: check, undo: false };
  return { build, status: t('Đã là bản mới nhất · kiểm tra lúc ', 'Already the newest · checked at ') + at, tone: 'ok', button: check, undo: false };
}
