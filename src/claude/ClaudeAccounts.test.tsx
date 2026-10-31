// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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
  await screen.findByRole('navigation', { name: 'Các tab' });
  return fake;
}

function withAccounts(over: Partial<ClaudeAccountsView>, model: Partial<Model> = {}): Model {
  return fixtureModel('fresh', { ...model, claudeAccounts: claudeAccountsFixture(over) });
}

const table = () => screen.getByRole('list', { name: 'Claude Code' });
const names = () => within(table()).getAllByRole('listitem').map((r) => r.querySelector('b')!.textContent);
const dialog = () => screen.getByRole('dialog');
const inDialog = (name: string | RegExp) => within(dialog()).getByRole('button', { name }) as HTMLButtonElement;

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

/** Opens an account's ⋯ menu as a pointer does: Radix opens a menu on pointerdown. */
async function menu(name: string) {
  await act(async () => {
    fireEvent.pointerDown(screen.getByRole('button', { name: 'Thêm thao tác cho ' + name }), { button: 0, ctrlKey: false });
  });
  return screen.getByRole('menu');
}

async function choose(account: string, item: string) {
  await menu(account);
  await press(screen.getByRole('menuitem', { name: item }));
}

afterEach(cleanup);

describe('the Claude accounts Quota Station keeps', () => {
  it('each account has a ⋯ menu: details, sign in again, remove', async () => {
    await open(withAccounts({}));
    expect(names()).toEqual(['lanpham.dev', 'lan.acme1', 'lan.acme2', 'lan.acme3']);
    const items = within(await menu('lan.acme2')).getAllByRole('menuitem');
    const want = ['Chi tiết và tên gọi…', 'Đăng nhập lại…', 'Xoá khỏi máy này…'];
    // @private:begin
    // And out of the rotation.
    want.splice(1, 0, 'Tạm dừng tự chuyển…');
    // @private:end
    expect(items.map((i) => i.textContent)).toEqual(want);
  });

  // @private:begin
  it('without the rotation and without Nâng cao, the menu has nothing to hold out of it', async () => {
    await open(withAccounts({ rotation: false }, { advanced: { on: false, firstmate: false } }));
    const items = within(await menu('lan.acme2')).getAllByRole('menuitem');
    expect(items.map((i) => i.textContent)).toEqual(['Chi tiết và tên gọi…', 'Đăng nhập lại…', 'Xoá khỏi máy này…']);
    // Nor a card for a rotation that does not run.
    expect(screen.queryByRole('switch', { name: 'Tự chuyển bằng Quota Station' })).toBeNull();
  });
  // @private:end

  it('before any account is kept, importing from claude-swap brings them over', async () => {
    const fake = await open(
      withAccounts({
        ready: false,
        accounts: [],
        // @private:begin
        rotation: false,
        // @private:end
      }),
    );
    expect(screen.getByText('Đưa các tài khoản Claude vào Quota Station')).toBeTruthy();
    // @private:begin
    // Nothing to rotate yet.
    expect((screen.getByRole('switch', { name: 'Tự chuyển bằng Quota Station' }) as HTMLButtonElement).disabled).toBe(true);
    // @private:end
    await press(screen.getByRole('button', { name: 'Nhập từ claude-swap' }));
    expect(fake.api.claudeImport).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/^Đã nhập từ claude-swap: 4 tài khoản mới/)).toBeTruthy();
    expect(screen.queryByText('Đưa các tài khoản Claude vào Quota Station')).toBeNull();
    // Importing again stays among the ways in.
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    expect(within(dialog()).getByRole('radio', { name: /^Nhập lại từ claude-swap/ })).toBeTruthy();
  });

  it('with no account and no claude-swap, the card is empty and offers the ways in', async () => {
    const empty = withAccounts({ ready: false, importable: false, accounts: [] });
    await open({ ...empty, snapshot: { ...empty.snapshot, claude: { ...empty.snapshot.claude, accounts: [] } } });
    expect(screen.getByText('Chưa có tài khoản Claude nào')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Thêm tài khoản…' }));
    expect(within(dialog()).getAllByRole('radio').map((r) => r.querySelector('b')!.textContent)).toEqual([
      'Đăng nhập trong trình duyệt',
      'Lấy tài khoản Claude Code đang đăng nhập',
      'Dán setup-token',
    ]);
  });

  it('the account in use cannot be removed; another one goes after asking', async () => {
    const fake = await open(withAccounts({}));
    await choose('lan.acme1', 'Xoá khỏi máy này…');
    expect(within(dialog()).getByRole('heading', { name: 'Chưa xoá được lan.acme1' })).toBeTruthy();
    await press(inDialog('Đóng'));
    await choose('lan.acme2', 'Xoá khỏi máy này…');
    expect(within(dialog()).getByText(/Tài khoản Claude vẫn còn nguyên/)).toBeTruthy();
    await press(inDialog('Thôi'));
    expect(fake.api.claudeRemove).not.toHaveBeenCalled();
    await choose('lan.acme2', 'Xoá khỏi máy này…');
    await press(inDialog('Xoá khỏi máy này'));
    expect(fake.api.claudeRemove).toHaveBeenCalledWith(3);
    expect(names()).toEqual(['lanpham.dev', 'lan.acme1', 'lan.acme3']);
    // Said once, as a toast that goes by itself.
    expect(screen.getByRole('status').textContent).toBe('Đã xoá lan.acme2 khỏi máy này');
  });

  it('the details give an account a nickname the tables use, and clearing it brings back the email', async () => {
    const fake = await open(withAccounts({}));
    await choose('lan.acme2', 'Chi tiết và tên gọi…');
    const d = within(dialog());
    expect(d.getByRole('heading', { name: 'lan.acme2' })).toBeTruthy();
    expect(d.getByText('lan.acme2@example.com')).toBeTruthy();
    const nick = d.getByRole('textbox', { name: 'Tên gọi' }) as HTMLInputElement;
    expect(nick.value).toBe('');
    expect(nick.placeholder).toBe('lan.acme2');
    fireEvent.change(nick, { target: { value: 'Team phụ' } });
    await press(inDialog('Lưu'));
    expect(fake.api.claudeSetAlias).toHaveBeenCalledWith(3, 'Team phụ');
    expect(names()).toEqual(['lanpham.dev', 'lan.acme1', 'Team phụ', 'lan.acme3']);
    expect(screen.getByRole('status').textContent).toBe('Đã lưu tên gọi Team phụ');
    await choose('Team phụ', 'Chi tiết và tên gọi…');
    fireEvent.change(within(dialog()).getByRole('textbox', { name: 'Tên gọi' }), { target: { value: '' } });
    await press(inDialog('Lưu'));
    expect(fake.api.claudeSetAlias).toHaveBeenLastCalledWith(3, '');
    expect(names()).toContain('lan.acme2');
  });

  // @private:begin
  it('an account leaves the rotation after asking, and Hoàn tác puts it back', async () => {
    const fake = await open(withAccounts({}));
    await choose('lan.acme3', 'Tạm dừng tự chuyển…');
    expect(within(dialog()).getByRole('heading', { name: 'Tạm dừng tự chuyển cho lan.acme3?' })).toBeTruthy();
    await press(inDialog('Tạm dừng'));
    expect(fake.api.claudeSetEnabled).toHaveBeenCalledWith(4, false);
    const rested = within(table()).getAllByRole('listitem')[3];
    expect(within(rested).getByText('nghỉ tự chuyển')).toBeTruthy();
    expect(screen.getByText('Đã tạm dừng tự chuyển cho lan.acme3.')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Hoàn tác' }));
    expect(fake.api.claudeSetEnabled).toHaveBeenLastCalledWith(4, true);
    expect(within(within(table()).getAllByRole('listitem')[3]).queryByText('nghỉ tự chuyển')).toBeNull();
  });
  // @private:end

  it('a setup token goes to the app once and is never shown; the added account takes a nickname', async () => {
    const fake = await open(withAccounts({}));
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    await press(within(dialog()).getByRole('radio', { name: /^Dán setup-token/ }));
    await press(inDialog('Tiếp tục'));
    const token = within(dialog()).getByLabelText('Token') as HTMLInputElement;
    expect(token.type).toBe('password');
    expect(inDialog('Thêm').disabled).toBe(true);
    fireEvent.change(token, { target: { value: 'sk-ant-oat01-made-up' } });
    fireEvent.change(within(dialog()).getByLabelText('Email của tài khoản (không bắt buộc)'), { target: { value: 'ci@team.example' } });
    await press(inDialog('Thêm'));
    expect(fake.api.claudeAddToken).toHaveBeenCalledWith('sk-ant-oat01-made-up', 'ci@team.example');
    expect(within(dialog()).getByRole('heading', { name: 'Đã thêm ci' })).toBeTruthy();
    expect(document.body.innerHTML).not.toContain('sk-ant-oat01-made-up');
    fireEvent.change(within(dialog()).getByRole('textbox', { name: 'Tên gọi' }), { target: { value: 'CI' } });
    await press(inDialog('Xong'));
    expect(fake.api.claudeSetAlias).toHaveBeenCalledWith(5, 'CI');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(names()).toContain('CI');
  });

  it('a token with no email is sent without one', async () => {
    const fake = await open(withAccounts({}));
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    await press(within(dialog()).getByRole('radio', { name: /^Dán setup-token/ }));
    await press(inDialog('Tiếp tục'));
    fireEvent.change(within(dialog()).getByLabelText('Token'), { target: { value: 'sk-ant-oat01-made-up' } });
    await press(inDialog('Thêm'));
    expect(fake.api.claudeAddToken).toHaveBeenCalledWith('sk-ant-oat01-made-up', null);
  });

  it('a sign-in waits for the browser with its countdown, and Thôi stops it', async () => {
    // Long enough that a slow runner still sees the wait before the sign-in ends.
    const fake = await open(withAccounts({}), { claudeDelayMs: 400 });
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    await press(inDialog('Tiếp tục'));
    expect(fake.api.claudeAddLogin).toHaveBeenCalledTimes(1);
    expect(within(dialog()).getByRole('heading', { name: 'Đang chờ trình duyệt…' })).toBeTruthy();
    expect(within(dialog()).getByText(/^Chờ tối đa 10 phút · còn \d+:\d\d$/)).toBeTruthy();
    await press(inDialog('Thôi'));
    expect(fake.api.claudeCancelLogin).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(names()).toHaveLength(4);
  });

  it('a sign-in comes in as the next account', async () => {
    const fake = await open(withAccounts({}));
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    await press(inDialog('Tiếp tục'));
    expect(fake.api.claudeAddLogin).toHaveBeenCalledTimes(1);
    expect(within(dialog()).getByRole('heading', { name: 'Đã thêm lan.new5' })).toBeTruthy();
    await press(inDialog('Xong'));
    expect(fake.api.claudeSetAlias).not.toHaveBeenCalled();
    expect(names().at(-1)).toBe('lan.new5');
  });

  it('a sign-in into an account already here renews it instead of adding it twice', async () => {
    await open(withAccounts({}), { claudeRenews: 1 });
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    await press(within(dialog()).getByRole('radio', { name: /^Lấy tài khoản Claude Code/ }));
    await press(inDialog('Tiếp tục'));
    expect(within(dialog()).getByRole('heading', { name: 'Tài khoản này đã có' })).toBeTruthy();
    expect(within(dialog()).getByText('Đăng nhập của lanpham.dev được làm mới bằng lần đăng nhập này.')).toBeTruthy();
    await press(inDialog('Đóng'));
    expect(names()).toHaveLength(4);
  });

  it('a sign-in that does not finish saves nothing and can be tried again', async () => {
    const fake = await open(withAccounts({}), { claudeFail: (a) => (a === 'login' ? 'login' : null) });
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    await press(inDialog('Tiếp tục'));
    expect(within(dialog()).getByRole('heading', { name: 'Chưa đăng nhập được' })).toBeTruthy();
    expect(within(dialog()).getByText(/Không có gì được lưu\.$/)).toBeTruthy();
    await press(inDialog('Thử lại'));
    expect(fake.api.claudeAddLogin).toHaveBeenCalledTimes(2);
  });

  it('an account to sign in again: the banner opens its dialog, and the sign-in renews it', async () => {
    const relogin = claudeAccountsFixture().accounts.map((a) => (a.slot === 3 ? { ...a, status: 'reloginRequired' as const } : a));
    const fake = await open(withAccounts({ accounts: relogin }), { claudeRenews: 3 });
    await press(screen.getByRole('button', { name: 'Đăng nhập lại…' }));
    expect(within(dialog()).getByRole('heading', { name: 'Đăng nhập lại lan.acme2?' })).toBeTruthy();
    expect(within(dialog()).getByText('lan.acme2@example.com')).toBeTruthy();
    await press(inDialog('Mở trang đăng nhập'));
    expect(fake.api.claudeAddLogin).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText('lan.acme2 đã đăng nhập lại')).toBeTruthy();
  });

  it('without the claude command there is no sign-in with the browser', async () => {
    await open(withAccounts({ claude: false }));
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    // The account Claude Code is on comes first then.
    expect(within(dialog()).getByRole('radio', { name: /^Lấy tài khoản Claude Code/ }).getAttribute('aria-checked')).toBe('true');
    await press(within(dialog()).getByRole('radio', { name: /^Đăng nhập trong trình duyệt/ }));
    expect(inDialog('Tiếp tục').disabled).toBe(true);
    expect(within(dialog()).getByText('Không thấy lệnh claude trên máy này.')).toBeTruthy();
  });

  // @private:begin
  it('an action that fails is reported and changes nothing', async () => {
    await open(withAccounts({}), { claudeFail: (a) => (a === 'enabled' ? 'busy' : null) });
    await choose('lan.acme3', 'Tạm dừng tự chuyển…');
    await press(inDialog('Tạm dừng'));
    expect(within(within(table()).getAllByRole('listitem')[3]).queryByText('nghỉ tự chuyển')).toBeNull();
    expect(screen.getByText('Một thao tác khác đang giữ tài khoản. Thử lại sau vài giây.')).toBeTruthy();
  });
  // @private:end

  // @private:begin
  it('the rotation turns on from the Switcher card', async () => {
    const fake = await open(withAccounts({ rotation: false }));
    const toggle = () => screen.getByRole('switch', { name: 'Tự chuyển bằng Quota Station' });
    expect(toggle().getAttribute('aria-checked')).toBe('false');
    await press(toggle());
    expect(fake.api.claudeSetRotation).toHaveBeenCalledWith(true);
    expect(toggle().getAttribute('aria-checked')).toBe('true');
    expect(screen.getByText('Đã bật tự chuyển bằng Quota Station')).toBeTruthy();
  });
  // @private:end

  // @private:begin
  it('while the old switcher is loaded the rotation cannot turn on, and the guide says how to stop it', async () => {
    await open(withAccounts({ rotation: false, legacy: 'com.cswap.prefer-team' }, { advanced: { on: false, firstmate: false } }));
    expect((screen.getByRole('switch', { name: 'Tự chuyển bằng Quota Station' }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Hướng dẫn tắt' }));
    const guide = screen.getByRole('dialog', { name: 'Tắt bộ tự chuyển cũ' });
    expect(within(guide).getByText('launchctl bootout gui/$(id -u)/com.cswap.prefer-team')).toBeTruthy();
  });
  // @private:end

  it('with names hidden no email shows, in the table or the details', async () => {
    await open(hideNames(withAccounts({})));
    expect(names()).toHaveLength(4);
    let third = 'Tài khoản 3';
    // @private:begin
    third = 'Team B';
    // @private:end
    await choose(third, 'Chi tiết và tên gọi…');
    expect(dialog().textContent).not.toMatch(/\w@\w/);
    await press(inDialog('Thôi'));
    expect(table().textContent).not.toMatch(/@/);
  });
});
