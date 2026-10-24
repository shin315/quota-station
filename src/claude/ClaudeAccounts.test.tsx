// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, hideNames, type FakeOptions } from '../fixtures/fakeApi';
import { claudeAccountsFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { ClaudeAccountsView, Model } from '../lib/model';

async function open(model: Model, options: FakeOptions = {}) {
  const fake = fakeApi(model, 'claude', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Mục' });
  return fake;
}

function withAccounts(over: Partial<ClaudeAccountsView>): Model {
  return fixtureModel('fresh', { claudeAccounts: claudeAccountsFixture(over) });
}

const card = () => screen.getByRole('region', { name: 'Tài khoản Quota Station giữ' });
const rows = () => within(within(card()).getByRole('list', { name: 'Tài khoản Claude' })).getAllByRole('listitem');
const row = (name: string) => within(card()).getByRole('listitem', { name });
const button = (name: string) => within(card()).getByRole('button', { name }) as HTMLButtonElement;

afterEach(cleanup);

describe('the Claude accounts Quota Station keeps', () => {
  it('before any account is kept, importing from claude-swap brings them over', async () => {
    const fake = await open(withAccounts({ ready: false, accounts: [], rotation: false }));
    expect(within(card()).queryByRole('list')).toBeNull();
    // Nothing to rotate yet.
    expect((within(card()).getByRole('switch', { name: 'Tự chuyển bằng Quota Station' }) as HTMLButtonElement).disabled).toBe(true);
    await act(async () => {
      fireEvent.click(button('Nhập từ claude-swap'));
    });
    expect(fake.api.claudeImport).toHaveBeenCalledTimes(1);
    expect(rows().map((r) => r.getAttribute('aria-label'))).toEqual(['Tài khoản 1', 'Tài khoản 2', 'Tài khoản 3', 'Tài khoản 4']);
    // Running it again stays offered.
    expect(button('Nhập lại từ claude-swap')).toBeTruthy();
  });

  it('a Mac without claude-swap offers only the other ways in', async () => {
    await open(withAccounts({ ready: false, importable: false, accounts: [] }));
    expect(within(card()).queryByRole('button', { name: /^Nhập/ })).toBeNull();
    expect(button('Đăng nhập tài khoản mới').disabled).toBe(false);
    expect(button('Thêm tài khoản đang đăng nhập').disabled).toBe(false);
  });

  it('the account in use cannot be removed; another one goes after asking', async () => {
    const fake = await open(withAccounts({}));
    expect(button('Xoá Tài khoản 2').disabled).toBe(true);
    fireEvent.click(button('Xoá Tài khoản 3'));
    const ask = within(card()).getByRole('group', { name: 'Xoá Tài khoản 3' });
    fireEvent.click(within(ask).getByRole('button', { name: 'Thôi' }));
    expect(fake.api.claudeRemove).not.toHaveBeenCalled();
    fireEvent.click(button('Xoá Tài khoản 3'));
    await act(async () => {
      fireEvent.click(within(within(card()).getByRole('group', { name: 'Xoá Tài khoản 3' })).getByRole('button', { name: 'Xoá' }));
    });
    expect(fake.api.claudeRemove).toHaveBeenCalledWith(3);
    expect(rows().map((r) => r.getAttribute('aria-label'))).toEqual(['Tài khoản 1', 'Tài khoản 2', 'Tài khoản 4']);
  });

  it('an account gets a name, and clearing it brings back its number', async () => {
    const fake = await open(withAccounts({}));
    fireEvent.click(within(row('Tài khoản 3')).getByRole('button', { name: 'Đổi tên' }));
    const name = within(card()).getByRole('textbox', { name: 'Tên mới' });
    expect((name as HTMLInputElement).value).toBe('');
    fireEvent.change(name, { target: { value: 'Team phụ' } });
    await act(async () => {
      fireEvent.keyDown(name, { key: 'Enter' });
    });
    expect(fake.api.claudeSetAlias).toHaveBeenCalledWith(3, 'Team phụ');
    fireEvent.click(within(row('Team phụ')).getByRole('button', { name: 'Đổi tên' }));
    const again = within(card()).getByRole('textbox', { name: 'Tên mới' });
    expect((again as HTMLInputElement).value).toBe('Team phụ');
    fireEvent.change(again, { target: { value: '' } });
    await act(async () => {
      fireEvent.click(button('Lưu'));
    });
    expect(row('Tài khoản 3')).toBeTruthy();
  });

  it('an account leaves the rotation and comes back', async () => {
    const fake = await open(withAccounts({}));
    const toggle = () => within(card()).getByRole('switch', { name: 'Trong vòng xoay: Tài khoản 4' });
    expect(toggle().getAttribute('aria-checked')).toBe('true');
    await act(async () => {
      fireEvent.click(toggle());
    });
    expect(fake.api.claudeSetEnabled).toHaveBeenCalledWith(4, false);
    expect(toggle().getAttribute('aria-checked')).toBe('false');
    await act(async () => {
      fireEvent.click(toggle());
    });
    expect(fake.api.claudeSetEnabled).toHaveBeenLastCalledWith(4, true);
    expect(toggle().getAttribute('aria-checked')).toBe('true');
  });

  it('a setup token goes to the app once and is never shown', async () => {
    const fake = await open(withAccounts({}));
    fireEvent.click(button('Thêm setup token'));
    const form = within(card()).getByRole('group', { name: 'Thêm setup token' });
    const add = within(form).getByRole('button', { name: 'Thêm token' }) as HTMLButtonElement;
    expect(add.disabled).toBe(true);
    const token = within(form).getByLabelText('Setup token') as HTMLInputElement;
    expect(token.type).toBe('password');
    fireEvent.change(token, { target: { value: 'sk-ant-oat01-made-up' } });
    fireEvent.change(within(form).getByRole('textbox', { name: 'Email của tài khoản (không bắt buộc)' }), { target: { value: ' ci@team.example ' } });
    await act(async () => {
      fireEvent.click(add);
    });
    expect(fake.api.claudeAddToken).toHaveBeenCalledWith('sk-ant-oat01-made-up', 'ci@team.example');
    expect(within(card()).queryByRole('group', { name: 'Thêm setup token' })).toBeNull();
    expect(rows()).toHaveLength(5);
    expect(document.body.innerHTML).not.toContain('sk-ant-oat01-made-up');
  });

  it('a token with no email is sent without one', async () => {
    const fake = await open(withAccounts({}));
    fireEvent.click(button('Thêm setup token'));
    const form = within(card()).getByRole('group', { name: 'Thêm setup token' });
    fireEvent.change(within(form).getByLabelText('Setup token'), { target: { value: 'sk-ant-oat01-made-up' } });
    await act(async () => {
      fireEvent.click(within(form).getByRole('button', { name: 'Thêm token' }));
    });
    expect(fake.api.claudeAddToken).toHaveBeenCalledWith('sk-ant-oat01-made-up', null);
  });

  it('while a login waits every other action waits, and the login can be cancelled', async () => {
    const fake = await open(withAccounts({ working: 'login' }));
    for (const name of ['Nhập lại từ claude-swap', 'Đăng nhập tài khoản mới', 'Thêm tài khoản đang đăng nhập', 'Thêm setup token', 'Xoá Tài khoản 3'])
      expect(button(name).disabled, name).toBe(true);
    expect((within(card()).getByRole('switch', { name: 'Tự chuyển bằng Quota Station' }) as HTMLButtonElement).disabled).toBe(true);
    await act(async () => {
      fireEvent.click(button('Huỷ'));
    });
    expect(fake.api.claudeCancelLogin).toHaveBeenCalledTimes(1);
  });

  it('a login comes in as the next account', async () => {
    const fake = await open(withAccounts({}));
    await act(async () => {
      fireEvent.click(button('Đăng nhập tài khoản mới'));
    });
    expect(fake.api.claudeAddLogin).toHaveBeenCalledTimes(1);
    expect(rows().at(-1)!.getAttribute('aria-label')).toBe('Tài khoản 5');
  });

  it('without the claude command there is no new login', async () => {
    await open(withAccounts({ claude: false }));
    expect(button('Đăng nhập tài khoản mới').disabled).toBe(true);
    expect(button('Thêm setup token').disabled).toBe(false);
  });

  it('an action that fails is reported and changes nothing', async () => {
    await open(withAccounts({}), { claudeFail: (a) => (a === 'enabled' ? 'busy' : null) });
    await act(async () => {
      fireEvent.click(within(card()).getByRole('switch', { name: 'Trong vòng xoay: Tài khoản 4' }));
    });
    expect(within(card()).getByRole('switch', { name: 'Trong vòng xoay: Tài khoản 4' }).getAttribute('aria-checked')).toBe('true');
    expect(card().querySelector('[data-tone="bad"]')).not.toBeNull();
  });

  it('the rotation turns on from the card', async () => {
    const fake = await open(withAccounts({ rotation: false }));
    const toggle = () => within(card()).getByRole('switch', { name: 'Tự chuyển bằng Quota Station' });
    expect(toggle().getAttribute('aria-checked')).toBe('false');
    await act(async () => {
      fireEvent.click(toggle());
    });
    expect(fake.api.claudeSetRotation).toHaveBeenCalledWith(true);
    expect(toggle().getAttribute('aria-checked')).toBe('true');
  });

  it('while the old switcher is loaded the rotation cannot turn on, and the guide says how to stop it', async () => {
    await open(withAccounts({ rotation: false, legacy: 'com.cswap.prefer-team' }));
    expect((within(card()).getByRole('switch', { name: 'Tự chuyển bằng Quota Station' }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(within(card()).getByRole('button', { name: 'Hướng dẫn tắt' }));
    const guide = screen.getByRole('dialog', { name: 'Tắt bộ tự chuyển cũ' });
    expect(within(guide).getByText('launchctl bootout gui/$(id -u)/com.cswap.prefer-team')).toBeTruthy();
  });

  it('with names hidden no email shows', async () => {
    await open(hideNames(withAccounts({})));
    expect(rows()).toHaveLength(4);
    expect(card().textContent).not.toMatch(/@/);
  });
});
