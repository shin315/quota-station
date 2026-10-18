import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, fakeEmail, type FakeOptions } from '../fixtures/fakeApi';
import { agyAccount, agyFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { AgyAccounts, Model } from '../lib/model';

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

const list = () => screen.getByRole('list', { name: 'Tài khoản Antigravity đăng nhập trong app' });
const addButton = () => screen.getByRole('button', { name: 'Thêm tài khoản' }) as HTMLButtonElement;

afterEach(cleanup);

describe('Antigravity login card', () => {
  it('starts empty, ready to add an account', async () => {
    await open(withAgy({}));
    expect(screen.getByText('Đăng nhập trong Quota Station')).toBeTruthy();
    expect(screen.getByText('Chưa có tài khoản Antigravity nào đăng nhập trong app trên máy này.')).toBeTruthy();
    expect(addButton().disabled).toBe(false);
  });

  it('a Mac without Antigravity in quota-axi still offers the card', async () => {
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

  it('lists accounts by number and keeps emails hidden until the switch is on', async () => {
    const accounts = [agyAccount(1), agyAccount(2)];
    await open(withAgy({ accounts }));
    const rows = within(list()).getAllByRole('listitem');
    expect(rows.map((r) => r.getAttribute('aria-label'))).toEqual(['Ultra 1', 'Ultra 2']);
    expect(screen.getAllByText('đã đăng nhập')).toHaveLength(2);
    expect(screen.getByText('2 tài khoản')).toBeTruthy();
    expect(document.body.textContent).not.toContain('@');
    cleanup();

    await open(withAgy({ accounts: accounts.map((a) => ({ ...a, email: fakeEmail(a.number) })) }, { showEmail: true }));
    expect(within(list()).getByText(fakeEmail(1))).toBeTruthy();
  });

  it('turning emails on in Cài đặt shows them on the card', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1)] }));
    fireEvent.click(screen.getByRole('button', { name: 'Cài đặt' }));
    await act(async () => {
      fireEvent.click(screen.getByRole('switch', { name: 'Hiện email' }));
    });
    expect(fake.api.setShowEmail).toHaveBeenCalledWith(true);
    fireEvent.click(screen.getByRole('button', { name: 'Antigravity' }));
    expect(within(list()).getByText(new RegExp(fakeEmail(1)))).toBeTruthy();
  });

  it('shows the accounts that need a new login or a verification', async () => {
    const fake = await open(
      withAgy({ accounts: [agyAccount(1, { status: 'needsLogin' }), agyAccount(2, { status: 'needsVerification', verify: true })] }),
    );
    const [first, second] = within(list()).getAllByRole('listitem');
    expect(within(first).getByText('cần đăng nhập lại')).toBeTruthy();
    expect(within(first).getByText(/Google không nhận đăng nhập cũ nữa/)).toBeTruthy();
    expect(within(second).getByText('cần xác minh')).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(second).getByRole('button', { name: 'Mở trang xác minh' }));
    });
    expect(fake.api.agyOpenVerification).toHaveBeenCalledWith('antigravity_fake2');
    expect(screen.queryByText(/trang xác minh cho/)).toBeNull();

    // Logging in again answers it.
    await act(async () => {
      fireEvent.click(within(first).getByRole('button', { name: 'Đăng nhập lại' }));
    });
    expect(fake.api.agyLogin).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/Đang chờ đăng nhập trong trình duyệt/)).toBeTruthy();
    await act(async () => {
      fireEvent.change(screen.getByLabelText(/Dán URL/), { target: { value: 'http://localhost:1/oauth-callback?state=s&code=c' } });
      fireEvent.click(screen.getByRole('button', { name: 'Gửi' }));
    });
    expect(screen.getByText('Đã đăng nhập lại Ultra 1')).toBeTruthy();
    expect(within(list()).queryByText('cần đăng nhập lại')).toBeNull();
  });

  it('a verification page that does not open says so', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1, { status: 'needsVerification', verify: true })] }));
    vi.mocked(fake.api.agyOpenVerification).mockRejectedValueOnce('noPage');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở trang xác minh' }));
    });
    expect(screen.getByText('Không còn trang xác minh cho Ultra 1')).toBeTruthy();
    vi.mocked(fake.api.agyOpenVerification).mockRejectedValueOnce('browser');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở trang xác minh' }));
    });
    expect(screen.getByText('Không mở được trình duyệt')).toBeTruthy();
  });

  it('adds an account through the browser and says so', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1)] }), { agyLoginDelayMs: 10 });
    await act(async () => {
      fireEvent.click(addButton());
    });
    expect(screen.getByText(/Màn hình đồng ý ghi tên ứng dụng của Antigravity, không phải Quota Station/)).toBeTruthy();
    expect(addButton().disabled).toBe(true);
    await act(async () => {
      await new Promise((r) => setTimeout(r, 30));
    });
    expect(fake.api.agyLogin).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Đã thêm Ultra 2')).toBeTruthy();
    expect(within(list()).getAllByRole('listitem')).toHaveLength(2);
    expect(screen.queryByText(/Đang chờ đăng nhập/)).toBeNull();
  });

  it('a login while an Antigravity switch runs says so and starts nothing', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1)] }, { agySwitch: { busy: 1, listed: true, overwritten: null } }));
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
  ])('a login that ends with %s says so', async (kind, title) => {
    await open(withAgy({}), { agyLoginDelayMs: 0, agyLogin: () => ({ kind, detail: '' }) });
    await act(async () => {
      fireEvent.click(addButton());
      await new Promise((r) => setTimeout(r, 10));
    });
    expect(screen.getByText(title)).toBeTruthy();
    expect(screen.getByText('Chưa có tài khoản Antigravity nào đăng nhập trong app trên máy này.')).toBeTruthy();
  });

  it('the advice for a missing refresh token names the page and the app to revoke', async () => {
    await open(withAgy({}), { agyLoginDelayMs: 0, agyLogin: () => ({ kind: 'noRefreshToken', detail: '' }) });
    await act(async () => {
      fireEvent.click(addButton());
      await new Promise((r) => setTimeout(r, 10));
    });
    expect(screen.getByText(/myaccount\.google\.com\/permissions, thu hồi quyền của "Antigravity Tools"/)).toBeTruthy();
  });

  it('logging out asks in the row first, then removes only that account', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1), agyAccount(2)] }));
    const second = () => within(list()).getAllByRole('listitem')[1];
    fireEvent.click(within(second()).getByRole('button', { name: 'Đăng xuất Ultra 2' }));
    expect(within(second()).getByText('Đăng xuất Ultra 2 khỏi Quota Station?')).toBeTruthy();
    expect(within(second()).getByText(/Antigravity\.app và agy vẫn đăng nhập như cũ/)).toBeTruthy();
    fireEvent.click(within(second()).getByRole('button', { name: 'Thôi' }));
    expect(fake.api.agyLogout).not.toHaveBeenCalled();

    fireEvent.click(within(second()).getByRole('button', { name: 'Đăng xuất Ultra 2' }));
    await act(async () => {
      fireEvent.click(within(second()).getByRole('button', { name: 'Đăng xuất' }));
    });
    expect(fake.api.agyLogout).toHaveBeenCalledWith('antigravity_fake2');
    expect(screen.getByText('Đã đăng xuất Ultra 2')).toBeTruthy();
    expect(within(list()).getAllByRole('listitem')).toHaveLength(1);
  });

  it('a logout the Keychain refuses says the account stays', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1)] }));
    vi.mocked(fake.api.agyLogout).mockRejectedValue('keychain');
    fireEvent.click(screen.getByRole('button', { name: 'Đăng xuất Ultra 1' }));
    await act(async () => {
      fireEvent.click(within(list()).getByRole('button', { name: 'Đăng xuất' }));
    });
    expect(screen.getByText('Chưa đăng xuất được Ultra 1')).toBeTruthy();
    expect(screen.getByText(/tài khoản vẫn trong danh sách/)).toBeTruthy();
    expect(within(list()).getAllByRole('listitem')).toHaveLength(1);
  });

  it('a logout refused while an Antigravity switch runs says to try again after it', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1)] }));
    vi.mocked(fake.api.agyLogout).mockRejectedValue('busy');
    fireEvent.click(screen.getByRole('button', { name: 'Đăng xuất Ultra 1' }));
    await act(async () => {
      fireEvent.click(within(list()).getByRole('button', { name: 'Đăng xuất' }));
    });
    expect(screen.getByText('Một lần chuyển Antigravity đang chạy. Chờ nó xong rồi thử lại.')).toBeTruthy();
    expect(within(list()).getAllByRole('listitem')).toHaveLength(1);
  });

  it('a logout while another runs says so', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1)] }));
    vi.mocked(fake.api.agyLogout).mockRejectedValue('loggingOut');
    fireEvent.click(screen.getByRole('button', { name: 'Đăng xuất Ultra 1' }));
    await act(async () => {
      fireEvent.click(within(list()).getByRole('button', { name: 'Đăng xuất' }));
    });
    expect(screen.getByText('Một lần đăng xuất khác đang chạy. Chờ nó xong rồi thử lại.')).toBeTruthy();
  });

  it('an unreadable account list is reported and nothing can be added', async () => {
    await open(withAgy({ indexError: true }));
    expect(screen.getByText('Không đọc được danh sách tài khoản')).toBeTruthy();
    expect(addButton().disabled).toBe(true);
  });
});

describe('Antigravity quota reads', () => {
  it('opening the tab asks the app to read every account', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1), agyAccount(2)] }));
    expect(fake.api.agyTabOpened).toHaveBeenCalledTimes(1);
    cleanup();

    const other = fakeApi(withAgy({ accounts: [agyAccount(1)] }), 'overview');
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
    await open(withAgy({ accounts: [agyAccount(1, { reading: true }), agyAccount(2)] }));
    const [first, second] = within(list()).getAllByRole('listitem');
    expect(within(first).getByRole('status').textContent).toBe('đang đọc…');
    expect(within(second).queryByRole('status')).toBeNull();
    expect(within(second).getByText('đã đăng nhập')).toBeTruthy();
  });

  it('a failed read says why and that the last numbers stay', async () => {
    await open(
      withAgy({
        accounts: [
          agyAccount(1, { readError: { kind: 'network', detail: 'fetchAvailableModels: no answer', retryAt: null } }),
          agyAccount(2, { readError: { kind: 'rateLimited', detail: '', retryAt: '2026-01-16T07:24:00.000Z' } }),
          agyAccount(3, { readError: { kind: 'rejected', detail: 'fetchAvailableModels: Google refused (400)', retryAt: null } }),
          agyAccount(4, { readError: { kind: 'unauthorized', detail: '', retryAt: null } }),
        ],
      }),
    );
    const rows = within(list()).getAllByRole('listitem');
    const line = (i: number) => within(rows[i]).getByRole('status');
    expect(line(0).textContent).toBe('đọc lỗi: không kết nối được Google. Giữ số cũ.');
    expect(line(0).className).toContain('bad');
    expect(line(1).textContent).toBe('đọc lỗi: Google báo gọi quá nhiều (429). Giữ số cũ; app đọc lại lúc 14:24.');
    expect(line(1).className).toContain('warn');
    expect(line(2).textContent).toBe('đọc lỗi: Google từ chối (fetchAvailableModels: Google refused (400)). Giữ số cũ.');
    expect(line(3).textContent).toBe('đọc lỗi: Google không nhận đăng nhập (401). Giữ số cũ.');
    // The login chips stay as they are: a failed read is not a logout.
    expect(within(list()).getAllByText('đã đăng nhập')).toHaveLength(4);
  });

  it('an account Google refused is no longer read and can log in again', async () => {
    const fake = await open(withAgy({ accounts: [agyAccount(1), agyAccount(2, { status: 'forbidden', verify: true })] }));
    const second = within(list()).getAllByRole('listitem')[1];
    expect(within(second).getByText('Google chặn')).toBeTruthy();
    expect(within(second).getByText(/từ chối đọc hạn mức của tài khoản này \(403\), nên app thôi đọc nó/)).toBeTruthy();
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
    expect(screen.queryByText(/đang đọc|đọc lỗi/)).toBeNull();
  });
});
