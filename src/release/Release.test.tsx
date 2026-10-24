// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp, PopoverApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { communityModel, FIXTURE_NOW, fixtureModel, releaseAvailable, releaseFixture } from '../fixtures/models';
import type { Api } from '../lib/api';
import { setLang } from '../lib/i18n';
import type { Model, ReleaseView, Tab } from '../lib/model';
import { releaseBanner, releaseLine, versionRow } from '../lib/release';

async function open(model: Model, tab: Tab = 'overview', options: FakeOptions = {}) {
  const fake = fakeApi(model, tab, options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Các tab' });
  return fake;
}

/** The app as downloaded: no author's tools, no Antigravity accounts. */
function downloaded(release: ReleaseView = releaseAvailable()): Model {
  const m = communityModel(fixtureModel('fresh', { release }));
  return { ...m, capabilities: { agyAccounts: false, authorTools: false } };
}

const dialog = () => screen.getByRole('dialog');
const inDialog = (name: string | RegExp) => within(dialog()).getByRole('button', { name });

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

afterEach(cleanup);

describe('a newer release on GitHub', () => {
  it('Tổng quan says so at its foot; its dialog lists what is new and opens the DMG', async () => {
    const fake = await open(downloaded());
    expect(screen.getByText('Có bản mới: Quota Station 0.3.0')).toBeTruthy();
    expect(screen.getByText('Bạn đang dùng 0.1.0. Không có gì tự cài: app hỏi trước.')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Xem có gì mới' }));
    expect(within(dialog()).getByText('Quota Station 0.3.0')).toBeTruthy();
    expect(within(dialog()).getByText('Bạn đang dùng 0.1.0. Phát hành 15/01/2026 trên GitHub Releases.')).toBeTruthy();
    expect(within(dialog()).getAllByRole('listitem').map((li) => li.textContent)).toContain('Giao diện bằng tiếng Anh');
    expect(within(dialog()).getByText(/mở bản DMG 14 MB trên GitHub/)).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Ghi chú đầy đủ trên GitHub' }));
    await press(inDialog('Tải và cài'));
    expect(fake.released()).toEqual(['notes', 'download']);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('Để sau hides the banner and the popover line until the next version', async () => {
    const fake = await open(downloaded());
    await press(screen.getByRole('button', { name: 'Để sau' }));
    expect(fake.api.releaseLater).toHaveBeenCalledWith('0.3.0');
    expect(screen.queryByText('Có bản mới: Quota Station 0.3.0')).toBeNull();
  });

  it('Bỏ qua bản này skips it; Cài đặt › Cập nhật says so and undoes it', async () => {
    const fake = await open(downloaded());
    await press(screen.getByRole('button', { name: 'Xem có gì mới' }));
    await press(inDialog('Bỏ qua bản này'));
    expect(fake.api.releaseSkip).toHaveBeenCalledWith('0.3.0');
    expect(screen.queryByText('Có bản mới: Quota Station 0.3.0')).toBeNull();
    await press(screen.getByRole('button', { name: 'Cài đặt' }));
    expect(screen.getByText(/Đã bỏ qua 0\.3\.0\. Báo lại khi có bản sau\./)).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Hoàn tác' }));
    expect(fake.api.releaseSkip).toHaveBeenLastCalledWith(null);
    expect(screen.getByText('Có bản 0.3.0 (phát hành 15/01/2026).')).toBeTruthy();
  });

  it('a build from source is pointed at its clone, not the download', async () => {
    const fake = await open(fixtureModel('fresh', { release: releaseAvailable() }));
    expect(screen.getByText(/Máy này chạy bản build từ mã nguồn, nên cập nhật từ mã nguồn/)).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Xem có gì mới' }));
    expect(within(dialog()).getByText('git pull --ff-only && scripts/install.sh')).toBeTruthy();
    expect(within(dialog()).queryByRole('button', { name: 'Tải và cài' })).toBeNull();
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    await press(inDialog('Chép lệnh'));
    expect(writeText).toHaveBeenCalledWith('git pull --ff-only && scripts/install.sh');
    expect(inDialog('Đã chép')).toBeTruthy();
    await press(inDialog('Cập nhật từ mã nguồn'));
    expect(fake.api.updateFromSource).toHaveBeenCalled();
  });

  it('the popover has one quiet line; Xem opens Tổng quan', async () => {
    const fake = fakeApi(downloaded());
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-popover">
        <PopoverApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('dialog', { name: 'Quota Station' });
    expect(screen.getByText('Có bản mới 0.3.0 · Hỏi trước khi cài.')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Xem' }));
    expect(fake.api.openWindow).toHaveBeenCalledWith('overview');
  });

  it('nothing shows while the app is up to date, or for a version put off', async () => {
    await open(downloaded(releaseFixture()));
    expect(screen.queryByText(/Có bản mới/)).toBeNull();
    cleanup();
    await open(downloaded(releaseAvailable({ later: '0.3.0' })));
    expect(screen.queryByText(/Có bản mới/)).toBeNull();
  });
});

describe('Cài đặt › Cập nhật', () => {
  it('Kiểm bản mới asks GitHub and shows what it found', async () => {
    const fake = await open(downloaded(releaseFixture()), 'settings', { releaseFinds: releaseAvailable().latest });
    expect(screen.getByText('0.1.0 (a1b2c3d) · bản tải về (MIT)')).toBeTruthy();
    expect(screen.getByText('Đang dùng bản mới nhất · kiểm lúc 09:14')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Kiểm bản mới' }));
    expect(fake.api.releaseCheck).toHaveBeenCalled();
    expect(screen.getByText('Có bản 0.3.0 (phát hành 15/01/2026).')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Xem có gì mới' }));
    expect(within(dialog()).getByText('Quota Station 0.3.0')).toBeTruthy();
  });

  it('GitHub out of reach: the line says so and Thử lại checks again', async () => {
    const fake = await open(downloaded(releaseFixture()), 'settings', { releaseFinds: 'offline' });
    await press(screen.getByRole('button', { name: 'Kiểm bản mới' }));
    expect(screen.getByText('Chưa kiểm được: không kết nối được GitHub. Tự thử lại sau 1 giờ.')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Thử lại' }));
    expect(fake.api.releaseCheck).toHaveBeenCalledTimes(2);
  });

  it('the daily check turns off', async () => {
    const fake = await open(downloaded(releaseFixture()), 'settings');
    const toggle = screen.getByRole('switch', { name: 'Tự kiểm bản mới của app' });
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    await press(toggle);
    expect(fake.api.setReleaseChecks).toHaveBeenCalledWith(false);
    expect(screen.getByRole('switch', { name: 'Tự kiểm bản mới của app' }).getAttribute('aria-checked')).toBe('false');
  });
});

describe('Cài đặt › Chẩn đoán', () => {
  it('shows exactly what it copies, then offers the bug form', async () => {
    const fake = await open(downloaded(releaseFixture()), 'settings');
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    await press(screen.getByRole('button', { name: 'Chép thông tin chẩn đoán…' }));
    const text = await within(dialog()).findByLabelText('Thông tin sẽ chép');
    expect(text.textContent).toMatch(/^Quota Station 0\.1\.0 \(a1b2c3d\)/);
    expect(text.textContent).not.toMatch(/\w@\w/);
    await press(inDialog('Chép'));
    expect(writeText).toHaveBeenCalledWith(text.textContent);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText('Đã chép thông tin chẩn đoán.')).toBeTruthy();
    // The line's button, above the card's link of the same name.
    await press(screen.getAllByRole('button', { name: 'Mở trang báo lỗi trên GitHub' })[0]);
    expect(fake.released()).toContain('issue');
  });

  it('a clipboard that refuses leaves nothing copied', async () => {
    await open(downloaded(releaseFixture()), 'settings');
    Object.defineProperty(navigator, 'clipboard', { value: { writeText: vi.fn(async () => Promise.reject(new Error('denied'))) }, configurable: true });
    await press(screen.getByRole('button', { name: 'Chép thông tin chẩn đoán…' }));
    await within(dialog()).findByLabelText('Thông tin sẽ chép');
    await press(inDialog('Chép'));
    expect(screen.queryByText('Đã chép thông tin chẩn đoán.')).toBeNull();
  });
});

describe('the release in English', () => {
  beforeEach(() => setLang('en'));
  afterEach(() => setLang('vi'));

  it('the banner, the popover line and the version row', () => {
    const m = downloaded();
    expect(releaseBanner(m)?.title).toBe('New version: Quota Station 0.3.0');
    expect(releaseBanner(m)?.body).toBe('You have 0.1.0. Nothing installs by itself: the app asks first.');
    expect(releaseLine(m)).toEqual({ title: 'Version 0.3.0 is out', body: 'Asks before installing.' });
    expect(versionRow(m).status).toBe('Version 0.3.0 is out (released 15 Jan 2026).');
    expect(versionRow(downloaded(releaseFixture({ error: 'rateLimited' }))).status).toBe('Could not check: GitHub is rate-limiting. Tries again in 1 hour.');
    expect(versionRow(downloaded(releaseFixture({ none: true, latest: null }))).status).toBe('No release on GitHub yet · checked at 09:14');
  });
});
