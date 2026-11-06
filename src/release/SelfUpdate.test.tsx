// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel, releaseAvailable, releaseModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';
import { releaseLine } from '../lib/release';
import type { UpdateWay } from '../lib/updater';
import { installFailure } from './useRelease';

const updater = vi.hoisted(() => ({
  downloadUpdate: vi.fn<(onProgress: (got: number, total: number | null) => void) => Promise<string>>(),
  installUpdate: vi.fn<() => Promise<void>>(),
  relaunch: vi.fn<() => Promise<void>>(),
}));
vi.mock('../lib/updater', () => ({ ...updater, BREW_UPGRADE: 'brew upgrade --cask shin315/tap/quota-station' }));

/** The release workflow's build, told how "Tải và cài" goes (`way`), on `platform`. */
function release(way: UpdateWay | undefined, platform: 'macos' | 'windows' = 'macos'): Model {
  const m = releaseModel(fixtureModel('fresh', { release: { ...releaseAvailable(), way } }));
  return { ...m, capabilities: { ...m.capabilities, platform } };
}

async function open(model: Model) {
  const fake = fakeApi(model, 'overview');
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Các tab' });
  return fake;
}

const dialog = () => screen.getByRole('dialog');
const inDialog = (name: string | RegExp) => within(dialog()).getByRole('button', { name });

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

async function openDialog() {
  await press(screen.getByRole('button', { name: 'Xem có gì mới' }));
}

beforeEach(() => {
  updater.downloadUpdate.mockReset();
  updater.installUpdate.mockReset().mockResolvedValue(undefined);
  updater.relaunch.mockReset().mockResolvedValue(undefined);
});

afterEach(cleanup);

describe('Tải và cài on a release build that updates itself', () => {
  it('downloads with its progress, installs, and offers to reopen (macOS)', async () => {
    let finish: (v: string) => void = () => {};
    updater.downloadUpdate.mockImplementation(async (onProgress) => {
      onProgress(4_000_000, 14_000_000);
      return new Promise<string>((resolve) => (finish = resolve));
    });
    const fake = await open(release('install'));
    await openDialog();
    expect(within(dialog()).getByText(/kiểm tra chữ ký rồi cài đè lên bản này/)).toBeTruthy();
    await press(inDialog('Tải và cài'));
    expect(within(dialog()).getByText('4.0 / 14 MB')).toBeTruthy();
    expect(within(dialog()).getByRole('progressbar').getAttribute('aria-valuenow')).toBe('29');
    expect(updater.installUpdate).not.toHaveBeenCalled();
    await act(async () => finish('0.3.0'));
    expect(updater.installUpdate).toHaveBeenCalledTimes(1);
    expect(within(dialog()).getByText('Đã cài Quota Station 0.3.0. Mở lại để dùng bản mới.')).toBeTruthy();
    await press(inDialog('Mở lại'));
    expect(updater.relaunch).toHaveBeenCalledTimes(1);
    // Nothing went to the browser.
    expect(fake.released()).toEqual([]);
  });

  it('on Windows installs only when asked, since the app closes for the installer', async () => {
    updater.downloadUpdate.mockResolvedValue('0.3.0');
    await open(release('install', 'windows'));
    await openDialog();
    await press(inDialog('Tải và cài'));
    expect(within(dialog()).getByText('Đã tải Quota Station 0.3.0 và kiểm tra chữ ký.')).toBeTruthy();
    expect(updater.installUpdate).not.toHaveBeenCalled();
    await press(inDialog('Cài và mở lại'));
    expect(updater.installUpdate).toHaveBeenCalledTimes(1);
    expect(updater.relaunch).not.toHaveBeenCalled();
  });

  it('a failure says why and falls back to the download page', async () => {
    updater.downloadUpdate.mockRejectedValue(new Error('The signature verification failed'));
    const fake = await open(release('install'));
    await openDialog();
    await press(inDialog('Tải và cài'));
    expect(within(dialog()).getByText('Chữ ký của bản tải về không khớp, nên app không cài nó.')).toBeTruthy();
    expect(within(dialog()).getByText('The signature verification failed')).toBeTruthy();
    expect(updater.installUpdate).not.toHaveBeenCalled();
    await press(inDialog('Thử lại'));
    expect(updater.downloadUpdate).toHaveBeenCalledTimes(2);
    await press(inDialog('Mở trang tải về'));
    expect(fake.released()).toEqual(['download']);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('keeps Để sau and Bỏ qua bản này', async () => {
    const fake = await open(release('install'));
    await openDialog();
    await press(inDialog('Bỏ qua bản này'));
    expect(fake.api.releaseSkip).toHaveBeenCalledWith('0.3.0');
    expect(updater.downloadUpdate).not.toHaveBeenCalled();
  });
});

describe('the other ways', () => {
  it('a copy Homebrew installed shows the brew command and installs nothing', async () => {
    const fake = await open(release('brew'));
    expect(screen.getByText(/cài qua Homebrew: cập nhật bằng Homebrew\./)).toBeTruthy();
    await openDialog();
    expect(within(dialog()).getByText('brew upgrade --cask shin315/tap/quota-station')).toBeTruthy();
    expect(within(dialog()).queryByRole('button', { name: 'Tải và cài' })).toBeNull();
    await press(inDialog('Xong'));
    expect(updater.downloadUpdate).not.toHaveBeenCalled();
    expect(fake.released()).toEqual([]);
    expect(releaseLine(release('brew'))?.body).toBe('Cập nhật bằng Homebrew.');
  });

  it('a release build without the update key, or an older app, opens the download as before', async () => {
    for (const way of ['download', undefined] as const) {
      const fake = await open(release(way));
      await openDialog();
      expect(within(dialog()).getByText(/mở bản DMG 14 MB trên GitHub/)).toBeTruthy();
      await press(inDialog('Tải và cài'));
      expect(fake.released()).toEqual(['download']);
      cleanup();
    }
    expect(updater.downloadUpdate).not.toHaveBeenCalled();
  });

  it('a build from source never downloads the release, even if told to', async () => {
    const m = fixtureModel('fresh', { release: { ...releaseAvailable(), way: 'source' } });
    await open(m);
    await openDialog();
    expect(within(dialog()).queryByRole('button', { name: 'Tải và cài' })).toBeNull();
    expect(within(dialog()).getByRole('button', { name: 'Cập nhật từ mã nguồn' })).toBeTruthy();
    expect(updater.downloadUpdate).not.toHaveBeenCalled();
  });
});

describe('installFailure', () => {
  it('names the missing update file, a bad signature, and anything else', () => {
    expect(installFailure(new Error('noUpdate'))).toEqual({ reason: 'GitHub chưa có tệp cập nhật cho máy này.', detail: '' });
    expect(installFailure('Signature mismatch').reason).toBe('Chữ ký của bản tải về không khớp, nên app không cài nó.');
    const long = installFailure(new Error('x'.repeat(400)));
    expect(long.reason).toBe('Không tải hoặc cài được bản mới.');
    expect(long.detail.length).toBe(161);
  });
});
