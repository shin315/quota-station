// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { claudeAccountsFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

/** A new install: nothing kept yet, Claude Code signed in to one account. */
function newMac(over: (m: Model) => void = () => {}): Model {
  const m = fixtureModel('fresh', { claudeAccounts: claudeAccountsFixture({ accounts: [], ready: false }), firstRun: true });
  over(m);
  return m;
}

async function open(model: Model, options: FakeOptions = {}) {
  const fake = fakeApi(model, 'overview', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { name: 'Chào bạn. Chọn ngôn ngữ' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const button = (name: string | RegExp) => screen.getByRole('button', { name });
const steps = () => within(screen.getByRole('list', { name: 'Các bước' })).getAllByRole('listitem').map((li) => li.textContent);
const current = () => screen.getByRole('listitem', { current: 'step' }).textContent;

afterEach(cleanup);

describe('the first run', () => {
  it('opens on its steps, without the sidebar, and ends on the popover', async () => {
    const fake = await open(newMac());
    expect(screen.queryByRole('navigation', { name: 'Các tab' })).toBeNull();
    expect(steps()).toEqual(['1. Ngôn ngữ', '2. Máy này có gì', '3. Tài khoản Claude', '4. Quyền và thanh menu', '5. Xong']);
    // 1: the language, saved at once.
    await press(screen.getByRole('radio', { name: 'English' }));
    expect(fake.api.setLanguage).toHaveBeenCalledWith('en');
    await press(screen.getByRole('radio', { name: 'Tiếng Việt' }));
    await press(button('Tiếp tục'));
    // 2: what this Mac has.
    expect(current()).toBe('2. Máy này có gì');
    const found = screen.getByRole('list', { name: 'Máy này có gì' });
    expect(within(found).getAllByRole('listitem').map((li) => li.querySelector('b')!.textContent)).toEqual(['Claude Code', 'Codex', 'Antigravity']);
    expect(within(found).getByText(/đang đăng nhập lan\.acme1@example\.com/)).toBeTruthy();
    await press(button('Kiểm lại'));
    expect(fake.api.refresh).toHaveBeenCalled();
    await press(button('Tiếp tục'));
    // 3: the account Claude Code is signed in to is kept at once, and takes a nickname.
    expect(fake.api.claudeAddCurrent).toHaveBeenCalledTimes(1);
    const accounts = await screen.findByRole('region', { name: 'Tài khoản Claude' });
    expect(within(accounts).getByText('1 tài khoản')).toBeTruthy();
    expect(within(accounts).getByText(/Claude Code đang đăng nhập/)).toBeTruthy();
    // The account in use cannot be dropped here.
    expect(within(accounts).queryByRole('button', { name: /^Bỏ / })).toBeNull();
    const nickname = within(accounts).getByRole('textbox');
    fireEvent.change(nickname, { target: { value: 'Công việc' } });
    await act(async () => {
      fireEvent.blur(nickname);
    });
    expect(fake.api.claudeSetAlias).toHaveBeenCalledWith(expect.any(Number), 'Công việc');
    await press(button('Tiếp tục'));
    // 4: Keychain, notifications, opening at login, the menu bar.
    expect(current()).toBe('4. Quyền và thanh menu');
    const perms = screen.getByRole('region', { name: 'Quyền và thanh menu' });
    expect(within(perms).getByText('Quyền Keychain')).toBeTruthy();
    expect(within(perms).getByRole('switch', { name: 'Mở khi đăng nhập' })).toBeTruthy();
    await press(within(perms).getByRole('button', { name: /Codex/ }));
    expect(fake.api.setMenubar).toHaveBeenCalledWith('codex');
    await press(button('Tiếp tục'));
    // 5: what the app found, then the popover.
    expect(screen.getByText('Claude: 1 tài khoản')).toBeTruthy();
    await press(button('Mở popover'));
    expect(fake.finished()).toEqual(['popover']);
    expect(await screen.findByRole('navigation', { name: 'Các tab' })).toBeTruthy();
  });

  it('Quay lại goes back a step, and Mở cửa sổ chính stays in the window', async () => {
    const fake = await open(newMac());
    await press(button('Tiếp tục'));
    await press(button('Quay lại'));
    expect(current()).toBe('1. Ngôn ngữ');
    await press(button('Tiếp tục'));
    await press(button('Tiếp tục'));
    await screen.findByText('1 tài khoản');
    await press(button('Tiếp tục'));
    await press(button('Tiếp tục'));
    await press(button('Mở cửa sổ chính'));
    expect(fake.finished()).toEqual(['window']);
    expect(await screen.findByRole('heading', { name: 'Tổng quan' })).toBeTruthy();
  });

  it('a Mac with no agent says so, skips the Claude step and still goes on', async () => {
    await open(
      newMac((m) => {
        m.snapshot.claude = { ...m.snapshot.claude, status: 'notSetUp', accounts: [] };
        m.snapshot.agy = { ...m.snapshot.agy, status: 'notSetUp', accounts: [] };
        m.codex = { ...m.codex, present: false };
      }),
    );
    expect(steps()).toEqual(['1. Ngôn ngữ', '2. Máy này có gì', '3. Quyền và thanh menu', '4. Xong']);
    await press(button('Tiếp tục'));
    expect(screen.getByText('Chưa thấy agent nào')).toBeTruthy();
    expect(screen.getByText('Không thấy Claude Code')).toBeTruthy();
    await press(button('Vẫn tiếp tục'));
    expect(current()).toBe('3. Quyền và thanh menu');
  });

  it('a section the app could not read says why', async () => {
    await open(
      newMac((m) => {
        m.snapshot.agy = { ...m.snapshot.agy, status: 'unavailable', reason: 'hết giờ chờ' };
      }),
    );
    await press(button('Tiếp tục'));
    expect(screen.getByText('Chưa đọc được hạn mức: hết giờ chờ.')).toBeTruthy();
  });

  it('Claude Code signed in to none: sign in with the browser on the page, more after it', async () => {
    const fake = await open(
      newMac((m) => {
        for (const a of m.snapshot.claude.accounts) a.active = false;
      }),
      { claudeDelayMs: 50 },
    );
    await press(button('Tiếp tục'));
    expect(screen.getByText(/đã cài, chưa đăng nhập tài khoản nào/)).toBeTruthy();
    await press(button('Tiếp tục'));
    expect(fake.api.claudeAddCurrent).not.toHaveBeenCalled();
    expect(screen.getByText('Claude Code chưa đăng nhập tài khoản nào')).toBeTruthy();
    expect(button('Bỏ qua, thêm sau')).toBeTruthy();
    await press(button('Đăng nhập trong trình duyệt'));
    expect(fake.api.claudeAddLogin).toHaveBeenCalled();
    // The wait is on the page, not in a dialog.
    expect(screen.getByText('Đang chờ trình duyệt…')).toBeTruthy();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(await screen.findByText('1 tài khoản')).toBeTruthy();
    // No "Đã thêm" dialog either: the list shows it, with its nickname field.
    expect(screen.queryByRole('dialog')).toBeNull();
    await press(button('Thêm tài khoản khác…'));
    expect(await screen.findByText('2 tài khoản')).toBeTruthy();
    // One added here can be dropped here.
    await press(screen.getAllByRole('button', { name: /^Bỏ / })[0]);
    expect(fake.api.claudeRemove).toHaveBeenCalled();
  });

  it('Thôi stops the wait for the browser', async () => {
    const fake = await open(
      newMac((m) => {
        for (const a of m.snapshot.claude.accounts) a.active = false;
      }),
      { claudeDelayMs: 50 },
    );
    await press(button('Tiếp tục'));
    await press(button('Tiếp tục'));
    await press(button('Đăng nhập trong trình duyệt'));
    await press(button('Thôi'));
    expect(fake.api.claudeCancelLogin).toHaveBeenCalled();
    expect(await screen.findByText('Claude Code chưa đăng nhập tài khoản nào')).toBeTruthy();
  });

  it('a setup-token goes through the add dialog', async () => {
    await open(
      newMac((m) => {
        for (const a of m.snapshot.claude.accounts) a.active = false;
      }),
    );
    await press(button('Tiếp tục'));
    await press(button('Tiếp tục'));
    await press(button('Dán setup-token'));
    expect(within(screen.getByRole('dialog')).getByText('Dán setup-token')).toBeTruthy();
  });

  it('in English', async () => {
    const fake = fakeApi(newMac((m) => (m.language = 'en')));
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('heading', { name: 'Welcome. Pick a language' });
    expect(within(screen.getByRole('list', { name: 'Steps' })).getAllByRole('listitem').map((li) => li.textContent)).toEqual([
      '1. Language',
      '2. This Mac',
      '3. Claude accounts',
      '4. Permissions',
      '5. Done',
    ]);
    await press(button('Continue'));
    expect(screen.getByRole('heading', { name: 'What this Mac has' })).toBeTruthy();
    expect(screen.getByText(/signed in as lan\.acme1@example\.com/)).toBeTruthy();
    await press(button('Continue'));
    expect(await screen.findByText('1 account')).toBeTruthy();
    await press(button('Continue'));
    expect(screen.getByRole('heading', { name: 'Permissions and menu bar' })).toBeTruthy();
    await press(button('Continue'));
    expect(screen.getByRole('heading', { name: 'All set' })).toBeTruthy();
    expect(button('Open the popover')).toBeTruthy();
  });

  it('a Mac that finished the first run opens on the tabs', async () => {
    const fake = fakeApi(fixtureModel('fresh'));
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    expect(await screen.findByRole('navigation', { name: 'Các tab' })).toBeTruthy();
    expect(screen.queryByRole('list', { name: 'Các bước' })).toBeNull();
  });
});
