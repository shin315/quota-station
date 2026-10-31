// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, fakeEmail, hideNames, type FakeOptions } from '../fixtures/fakeApi';
import { agyAccount, agyFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { AgyAccounts, AgyLogin, Model } from '../lib/model';

async function open(model: Model, options: FakeOptions = {}) {
  const fake = fakeApi(model, 'agy', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { name: 'Antigravity' });
  return fake;
}

const withAgy = (over: Partial<AgyAccounts>, model: Partial<Model> = {}) =>
  fixtureModel('fresh', { antigravity: agyFixture(over), ...model });

/** An account logged in here, its email sent as Rust sends it while names show. */
const login = (n: number, over: Partial<AgyLogin> = {}) => agyAccount(n, { email: fakeEmail(n), ...over });

// The fresh fixture's Antigravity row lists this app's accounts 1 and 2: their
// logins sit in the account table; any other login is listed under it.
/** An email anywhere in the page; the layout's own `@container` classes are not one. */
const EMAIL = /\w@\w/;
const table = () => screen.getByRole('list', { name: 'Antigravity' });
const row = (name: string) =>
  within(table())
    .getAllByRole('listitem')
    .find((r) => new RegExp('^' + name.replace(/\./g, '\\.') + '[,.]').test(r.getAttribute('aria-label')!))!;
const others = () => screen.getByRole('list', { name: 'Tài khoản đăng nhập trong app' });
const addButton = () => screen.getByRole('button', { name: 'Thêm tài khoản' }) as HTMLButtonElement;
const confirmFor = (name: string) => screen.getByRole('group', { name: 'Đăng xuất ' + name });
/** What a row says, on screen or on hover. */
const said = (el: HTMLElement) => [el.textContent, ...Array.from(el.querySelectorAll('[title]')).map((t) => t.getAttribute('title'))].join(' ');

/** An account's name while names are hidden. */
let hidden = (n: number) => 'Antigravity ' + n;
// @private:begin
hidden = (n: number) => 'Ultra ' + n;
// @private:end

afterEach(cleanup);

describe('Antigravity accounts logged in here', () => {
  it('starts empty, ready to add an account', async () => {
    await open(withAgy({}));
    expect(screen.queryByText('Đăng nhập trong Quota Station')).toBeNull();
    expect(screen.getByText('Chưa có tài khoản nào đăng nhập trong app.')).toBeTruthy();
    expect(within(row('lanpham.ultra')).queryByRole('button', { name: /Đăng xuất/ })).toBeNull();
    expect(addButton().disabled).toBe(false);
  });

  it('a Mac without Antigravity still offers to add an account', async () => {
    await open(fixtureModel('no-antigravity', { antigravity: agyFixture() }));
    expect(screen.getByText('Máy này chưa có Antigravity')).toBeTruthy();
    expect(addButton().disabled).toBe(false);
  });

  it('without oauth-client.json it explains what is missing and cannot add', async () => {
    const fake = await open(withAgy({ client: 'missing' }));
    expect(screen.getByText('Chưa có oauth-client.json')).toBeTruthy();
    expect(screen.getByText(/~\/Library\/Application Support\/com\.shin315\.quota-station\/oauth-client\.json, quyền 600/)).toBeTruthy();
    expect(addButton().disabled).toBe(true);
    fireEvent.click(addButton());
    expect(fake.api.agyLogin).not.toHaveBeenCalled();
  });

  it('a client file others can read, or one still holding placeholders, is refused', async () => {
    await open(withAgy({ client: 'tooOpen' }));
    expect(screen.getByText('oauth-client.json đang để người khác đọc được')).toBeTruthy();
    expect(addButton().disabled).toBe(true);
    cleanup();
    await open(withAgy({ client: 'invalid' }));
    expect(screen.getByText('oauth-client.json chưa đúng')).toBeTruthy();
  });

  it('puts each login in its table row by name, and by number while names are hidden', async () => {
    const accounts = [login(1), login(2)];
    await open(withAgy({ accounts }));
    for (const name of ['lanpham.ultra', 'lan.ultra2']) {
      expect(within(row(name)).getByRole('button', { name: 'Đăng xuất ' + name })).toBeTruthy();
    }
    expect(within(row('lanpham.ultra')).getByRole('button', { name: 'Chuyển sang lanpham.ultra' })).toBeTruthy();
    expect(within(row('lan.ultra2')).getByText('Đang dùng')).toBeTruthy();
    // A login that is fine needs no chip; nothing else is listed under the table.
    expect(screen.queryByText('đã đăng nhập')).toBeNull();
    expect(screen.queryByRole('list', { name: 'Tài khoản đăng nhập trong app' })).toBeNull();
    expect(row('lanpham.ultra').getAttribute('title')).toMatch(/^lanpham\.ultra@example\.com · /);
    cleanup();

    await open(hideNames(withAgy({ accounts })));
    expect(within(row(hidden(1))).getByRole('button', { name: 'Đăng xuất ' + hidden(1) })).toBeTruthy();
    expect(within(row(hidden(2))).getByRole('button', { name: 'Đăng xuất ' + hidden(2) })).toBeTruthy();
    expect(document.body.innerHTML).not.toMatch(EMAIL);
  });

  it('turning names off in Cài đặt hides them in the table', async () => {
    const fake = await open(withAgy({ accounts: [login(1)] }));
    expect(row('lanpham.ultra')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Cài đặt' }));
    fireEvent.mouseDown(screen.getByRole('tab', { name: 'Hiển thị' }));
    await act(async () => {
      fireEvent.click(screen.getByRole('switch', { name: 'Hiện email' }));
    });
    expect(fake.api.setShowNames).toHaveBeenCalledWith(false);
    fireEvent.click(screen.getByRole('button', { name: 'Antigravity' }));
    expect(within(row(hidden(1))).getByRole('button', { name: 'Đăng xuất ' + hidden(1) })).toBeTruthy();
    expect(document.body.innerHTML).not.toMatch(EMAIL);
  });

  it('shows the accounts that need a new login or a verification', async () => {
    const fake = await open(
      withAgy({ accounts: [login(1, { status: 'needsLogin' }), login(2, { status: 'needsVerification', verify: true })] }),
    );
    const first = () => row('lanpham.ultra');
    const second = () => row('lan.ultra2');
    expect(within(first()).getByText('cần đăng nhập lại')).toBeTruthy();
    expect(within(second()).getByText('cần xác minh')).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(second()).getByRole('button', { name: 'Mở trang xác minh' }));
    });
    expect(fake.api.agyOpenVerification).toHaveBeenCalledWith('antigravity_fake2');
    expect(screen.queryByText(/trang xác minh cho/)).toBeNull();

    // Logging in again answers it.
    await act(async () => {
      fireEvent.click(within(first()).getByRole('button', { name: 'Đăng nhập lại' }));
    });
    expect(fake.api.agyLogin).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/Đang chờ đăng nhập trong trình duyệt/)).toBeTruthy();
    await act(async () => {
      fireEvent.change(screen.getByLabelText(/Dán URL/), { target: { value: 'http://localhost:1/oauth-callback?state=s&code=c' } });
      fireEvent.click(screen.getByRole('button', { name: 'Gửi' }));
    });
    expect(screen.getByText(/^Đã đăng nhập lại /)).toBeTruthy();
    expect(within(first()).queryByText('cần đăng nhập lại')).toBeNull();
  });

  it('a verification page that does not open says so', async () => {
    const fake = await open(withAgy({ accounts: [login(1, { status: 'needsVerification', verify: true })] }));
    vi.mocked(fake.api.agyOpenVerification).mockRejectedValueOnce('noPage');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở trang xác minh' }));
    });
    expect(screen.getByText('Không còn trang xác minh cho lanpham.ultra')).toBeTruthy();
    vi.mocked(fake.api.agyOpenVerification).mockRejectedValueOnce('browser');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở trang xác minh' }));
    });
    expect(screen.getByText('Không mở được trình duyệt')).toBeTruthy();
  });

  it('adds an account through the browser and says so', async () => {
    const fake = await open(withAgy({ accounts: [login(1)] }), { agyLoginDelayMs: 10 });
    expect(within(row('lan.ultra2')).queryByRole('button', { name: /Đăng xuất/ })).toBeNull();
    await act(async () => {
      fireEvent.click(addButton());
    });
    expect(screen.getByText(/Màn hình đồng ý ghi tên ứng dụng của Antigravity, không phải Quota Station/)).toBeTruthy();
    expect(addButton().disabled).toBe(true);
    await act(async () => {
      await new Promise((r) => setTimeout(r, 30));
    });
    expect(fake.api.agyLogin).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/^Đã thêm /)).toBeTruthy();
    expect(within(row('lan.ultra2')).getByRole('button', { name: 'Đăng xuất lan.ultra2' })).toBeTruthy();
    expect(screen.queryByText(/Đang chờ đăng nhập/)).toBeNull();
  });

  it('a login while an Antigravity switch runs says so and starts nothing', async () => {
    const fake = await open(withAgy({ accounts: [login(1)] }, { agySwitch: { busy: 1, listed: true, overwritten: null } }));
    await act(async () => {
      fireEvent.click(addButton());
    });
    expect(fake.api.agyLogin).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Đang chuyển Antigravity')).toBeTruthy();
    expect(screen.getByText('Chờ lần chuyển xong rồi đăng nhập.')).toBeTruthy();
    expect(screen.queryByText(/Đang chờ đăng nhập/)).toBeNull();
    expect(addButton().disabled).toBe(false);
  });

  it('a pasted URL the login refuses keeps it waiting, and Huỷ ends it', async () => {
    const fake = await open(withAgy({}));
    await act(async () => {
      fireEvent.click(addButton());
    });
    const field = screen.getByLabelText(/Dán URL/);
    await act(async () => {
      fireEvent.change(field, { target: { value: 'http://localhost:1/oauth-callback?state=wrong&code=c' } });
      fireEvent.click(screen.getByRole('button', { name: 'Gửi' }));
    });
    expect(screen.getByText('URL này không thuộc lần đăng nhập đang chờ.')).toBeTruthy();
    await act(async () => {
      fireEvent.change(field, { target: { value: 'http://localhost:1/oauth-callback?state=s' } });
      fireEvent.keyDown(field, { key: 'Enter' });
    });
    expect(screen.getByText('URL không có mã đăng nhập (code=…).')).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Huỷ' }));
    });
    expect(fake.api.agyLoginCancel).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Đã huỷ đăng nhập')).toBeTruthy();
    expect(screen.queryByText(/Đang chờ đăng nhập/)).toBeNull();
  });

  it.each([
    ['timeout', 'Hết 10 phút chờ đăng nhập'],
    ['noRefreshToken', 'Google không trả refresh token'],
    ['stateMismatch', 'Đăng nhập không khớp'],
    ['invalidGrant', 'Google từ chối mã đăng nhập'],
    ['keychain', 'Không lưu được vào Keychain'],
    ['network', 'Không kết nối được Google'],
    ['loggingOut', 'Đang đăng xuất một tài khoản'],
  ])('a login that ends with %s says so', async (kind, title) => {
    await open(withAgy({}), { agyLoginDelayMs: 0, agyLogin: () => ({ kind, detail: '' }) });
    await act(async () => {
      fireEvent.click(addButton());
      await new Promise((r) => setTimeout(r, 10));
    });
    expect(screen.getByText(title)).toBeTruthy();
    expect(screen.getByText('Chưa có tài khoản nào đăng nhập trong app.')).toBeTruthy();
  });

  it('the advice for a missing refresh token names the page and the app to revoke', async () => {
    await open(withAgy({}), { agyLoginDelayMs: 0, agyLogin: () => ({ kind: 'noRefreshToken', detail: '' }) });
    await act(async () => {
      fireEvent.click(addButton());
      await new Promise((r) => setTimeout(r, 10));
    });
    expect(screen.getByText(/myaccount\.google\.com\/permissions, thu hồi quyền của "Antigravity Tools"/)).toBeTruthy();
  });

  it('logging out asks under the row first, then removes only that account', async () => {
    const fake = await open(withAgy({ accounts: [login(1), login(2)] }));
    fireEvent.click(within(row('lan.ultra2')).getByRole('button', { name: 'Đăng xuất lan.ultra2' }));
    const ask = confirmFor('lan.ultra2');
    expect(within(ask).getByText('Đăng xuất lan.ultra2 khỏi Quota Station?')).toBeTruthy();
    expect(within(ask).getByText(/Antigravity\.app và agy vẫn đăng nhập như cũ/)).toBeTruthy();
    fireEvent.click(within(ask).getByRole('button', { name: 'Huỷ' }));
    expect(screen.queryByRole('group', { name: 'Đăng xuất lan.ultra2' })).toBeNull();
    expect(fake.api.agyLogout).not.toHaveBeenCalled();

    fireEvent.click(within(row('lan.ultra2')).getByRole('button', { name: 'Đăng xuất lan.ultra2' }));
    await act(async () => {
      fireEvent.click(within(confirmFor('lan.ultra2')).getByRole('button', { name: 'Đăng xuất' }));
    });
    expect(fake.api.agyLogout).toHaveBeenCalledWith('antigravity_fake2');
    expect(screen.getByText('Đã đăng xuất lan.ultra2')).toBeTruthy();
    expect(within(row('lan.ultra2')).queryByRole('button', { name: /Đăng xuất/ })).toBeNull();
    expect(within(row('lanpham.ultra')).getByRole('button', { name: 'Đăng xuất lanpham.ultra' })).toBeTruthy();
  });

  /** Asks to log lanpham.ultra out and confirms, with the fake refusing as `refusal`. */
  async function refusedLogout(refusal: string) {
    const fake = await open(withAgy({ accounts: [login(1)] }));
    vi.mocked(fake.api.agyLogout).mockRejectedValue(refusal);
    fireEvent.click(screen.getByRole('button', { name: 'Đăng xuất lanpham.ultra' }));
    await act(async () => {
      fireEvent.click(within(confirmFor('lanpham.ultra')).getByRole('button', { name: 'Đăng xuất' }));
    });
  }

  it('a logout the Keychain refuses says the account stays', async () => {
    await refusedLogout('keychain');
    expect(screen.getByText('Chưa đăng xuất được lanpham.ultra')).toBeTruthy();
    expect(screen.getByText(/tài khoản vẫn trong danh sách/)).toBeTruthy();
    expect(within(row('lanpham.ultra')).getByRole('button', { name: 'Đăng xuất lanpham.ultra' })).toBeTruthy();
  });

  it('a logout refused while an Antigravity switch runs says to try again after it', async () => {
    await refusedLogout('busy');
    expect(screen.getByText('Một lần chuyển Antigravity đang chạy. Chờ nó xong rồi thử lại.')).toBeTruthy();
    expect(within(row('lanpham.ultra')).getByRole('button', { name: 'Đăng xuất lanpham.ultra' })).toBeTruthy();
  });

  it('a logout while another runs says so', async () => {
    await refusedLogout('loggingOut');
    expect(screen.getByText('Một lần đăng xuất khác đang chạy. Chờ nó xong rồi thử lại.')).toBeTruthy();
  });

  it('a logout refused while a login waits says to finish or cancel it first', async () => {
    await refusedLogout('loggingIn');
    expect(screen.getByText('Một lần đăng nhập đang chờ. Xong lần đó, hoặc bấm Huỷ, rồi thử lại.')).toBeTruthy();
    expect(within(row('lanpham.ultra')).getByRole('button', { name: 'Đăng xuất lanpham.ultra' })).toBeTruthy();
  });

  it('a login the row does not list yet stays listed under the table', async () => {
    await open(withAgy({ accounts: [login(1), login(2), login(3)] }));
    const extra = within(others()).getAllByRole('listitem');
    expect(extra.map((r) => r.getAttribute('aria-label'))).toEqual(['lan.ultra3']);
    expect(within(extra[0]).getByRole('button', { name: 'Đăng xuất lan.ultra3' })).toBeTruthy();
  });

  it('an unreadable account list is reported and nothing can be added', async () => {
    await open(withAgy({ indexError: true }));
    expect(screen.getByText('Không đọc được danh sách tài khoản')).toBeTruthy();
    expect(addButton().disabled).toBe(true);
  });
});

describe('Antigravity quota reads', () => {
  it('opening the tab asks the app to read every account', async () => {
    const fake = await open(withAgy({ accounts: [login(1), login(2)] }));
    expect(fake.api.agyTabOpened).toHaveBeenCalledTimes(1);
    cleanup();

    const other = fakeApi(withAgy({ accounts: [login(1)] }), 'overview');
    vi.spyOn(other.api, 'agyTabOpened');
    render(
      <div className="qs app-window">
        <MainWindowApp api={other.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('heading', { name: 'Tổng quan' });
    expect(other.api.agyTabOpened).not.toHaveBeenCalled();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Antigravity' }));
    });
    expect(other.api.agyTabOpened).toHaveBeenCalledTimes(1);
  });

  it('a row says while the app reads it, and nothing once it is read', async () => {
    await open(withAgy({ accounts: [login(1, { reading: true }), login(2)] }));
    expect(within(row('lanpham.ultra')).getByText('đang đọc…')).toBeTruthy();
    expect(within(row('lan.ultra2')).queryByText('đang đọc…')).toBeNull();
    expect(within(row('lan.ultra2')).queryByText('lỗi đọc dữ liệu')).toBeNull();
  });

  it('a failed read says why and that the last numbers stay', async () => {
    await open(
      withAgy({
        accounts: [
          login(1, { readError: { kind: 'network', detail: 'fetchAvailableModels: no answer', retryAt: null } }),
          login(2, { readError: { kind: 'rateLimited', detail: '', retryAt: '2026-01-16T07:24:00.000Z' } }),
          login(3, { readError: { kind: 'rejected', detail: 'fetchAvailableModels: Google refused (400)', retryAt: null } }),
          login(4, { readError: { kind: 'unauthorized', detail: '', retryAt: null } }),
        ],
      }),
    );
    // In the table: a chip, coloured by whether the app reads it again by itself.
    const chip = (name: string) => within(row(name)).getByText('lỗi đọc dữ liệu');
    expect(chip('lanpham.ultra').className).toContain('bad');
    expect(chip('lan.ultra2').className).toContain('warn');
    // Why it failed and that the last numbers stay must still be there to read.
    expect(said(row('lanpham.ultra'))).toContain('lỗi đọc dữ liệu: không kết nối được Google. Giữ số cũ.');
    expect(said(row('lan.ultra2'))).toContain('lỗi đọc dữ liệu: Google báo gọi quá nhiều (429). Giữ số cũ; app đọc lại lúc 14:24.');
    // Under the table, the logins the row does not list keep the whole line.
    const extra = within(others()).getAllByRole('listitem');
    const line = (i: number) => within(extra[i]).getByRole('status');
    expect(line(0).textContent).toBe('lỗi đọc dữ liệu: Google từ chối (fetchAvailableModels: Google refused (400)). Giữ số cũ.');
    expect(line(1).textContent).toBe('lỗi đọc dữ liệu: Google không nhận đăng nhập (401). Giữ số cũ.');
    expect(line(1).className).toContain('bad');
    // The login stays as it is: a failed read is not a logout.
    expect(screen.queryByText('cần đăng nhập lại')).toBeNull();
  });

  it('an account Google refused is no longer read and can log in again', async () => {
    const fake = await open(withAgy({ accounts: [login(1), login(2, { status: 'forbidden', verify: true })] }));
    const second = row('lan.ultra2');
    expect(within(second).getByText('Google chặn')).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(second).getByRole('button', { name: 'Mở trang xác minh' }));
    });
    expect(fake.api.agyOpenVerification).toHaveBeenCalledWith('antigravity_fake2');
    await act(async () => {
      fireEvent.click(within(second).getByRole('button', { name: 'Đăng nhập lại' }));
    });
    expect(fake.api.agyLogin).toHaveBeenCalledTimes(1);
  });

  it('a Mac with no account logged in still says it has no Antigravity, and reads nothing', async () => {
    await open(fixtureModel('no-antigravity', { antigravity: agyFixture() }));
    expect(screen.getByText('Máy này chưa có Antigravity')).toBeTruthy();
    expect(screen.queryByText(/đang đọc|lỗi đọc dữ liệu/)).toBeNull();
  });
});
