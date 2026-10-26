// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { API_KEY, envFixture } from '../fixtures/fakeEnv';
import { agyFixture, alertsFixture, appFixture, codexFixture, communityModel, firstmateFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

async function open(model: Model) {
  const fake = fakeApi(model, 'settings');
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { name: 'Cài đặt' });
  return fake;
}

afterEach(cleanup);

/** Shows one of Cài đặt's tabs, as a press on it does. */
async function showGroup(name: string) {
  await act(async () => {
    fireEvent.mouseDown(screen.getByRole('tab', { name }));
  });
}

describe('settings tab', () => {
  it('Thanh menu picks the service the menu bar title follows', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Hiển thị');
    const group = screen.getByRole('group', { name: 'Thanh menu' });
    const [claude, gemini, codex] = within(group).getAllByRole('button');
    expect([claude, gemini, codex].map((b) => [b.textContent, b.getAttribute('aria-pressed'), (b as HTMLButtonElement).disabled])).toEqual([
      ['Claude', 'true', false],
      ['Gemini', 'false', false],
      ['Codex', 'false', false],
    ]);
    expect(screen.getByText('Tên và phần trăm đã dùng của tài khoản đang dùng.')).toBeTruthy();
    await act(async () => {
      fireEvent.click(codex);
    });
    expect(fake.api.setMenubar).toHaveBeenCalledWith('codex');
    expect(codex.getAttribute('aria-pressed')).toBe('true');
  });

  it('a service this Mac lacks is greyed out with the reason, and the title falls back to Claude', async () => {
    await open(fixtureModel('no-antigravity', { menubar: 'gemini', codex: codexFixture({ present: false }) }));
    await showGroup('Hiển thị');
    const group = screen.getByRole('group', { name: 'Thanh menu' });
    const [, gemini, codex] = within(group).getAllByRole('button') as HTMLButtonElement[];
    expect([gemini.disabled, codex.disabled]).toEqual([true, true]);
    expect(gemini.title).toBe('Máy này chưa có Antigravity.');
    expect(codex.title).toBe('Máy này chưa có Codex.');
    expect(screen.getByText('Máy này chưa có Antigravity: thanh menu theo Claude.')).toBeTruthy();
    cleanup();
    // Codex on its ChatGPT login without one signed in.
    await open(fixtureModel('no-antigravity', { codex: codexFixture({ current: 'chatgpt' }) }));
    await showGroup('Hiển thị');
    const [, , codex2] = within(screen.getByRole('group', { name: 'Thanh menu' })).getAllByRole('button') as HTMLButtonElement[];
    expect([codex2.disabled, codex2.title]).toEqual([true, 'Codex chưa đăng nhập ChatGPT.']);
  });

  it('Số trên thanh menu picks the window the title shows, five hours by default', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Hiển thị');
    const group = screen.getByRole('group', { name: 'Số trên thanh menu' });
    const buttons = within(group).getAllByRole('button') as HTMLButtonElement[];
    expect(buttons.map((b) => [b.textContent, b.getAttribute('aria-pressed'), b.disabled])).toEqual([
      ['5 giờ', 'true', false],
      ['Tuần', 'false', false],
      ['Fable tuần', 'false', false],
      ['Chặt nhất', 'false', false],
    ]);
    await act(async () => {
      fireEvent.click(buttons[2]);
    });
    expect(fake.api.setMenubarWindow).toHaveBeenCalledWith('fableWeek');
    expect(buttons[2].getAttribute('aria-pressed')).toBe('true');
  });

  it('Fable tuần is greyed out unless the title follows Claude, and a saved Fable falls back to five hours', async () => {
    await open(fixtureModel('fresh', { menubar: 'gemini', menubarWindow: 'fableWeek' }));
    await showGroup('Hiển thị');
    const [, , fable] = within(screen.getByRole('group', { name: 'Số trên thanh menu' })).getAllByRole('button') as HTMLButtonElement[];
    expect([fable.disabled, fable.title]).toEqual([true, 'Gemini không có Fable tuần.']);
    expect(screen.getByText('Gemini không có Fable tuần: thanh menu hiện 5 giờ.')).toBeTruthy();
    cleanup();
    // This Mac lacks the service picked: the title follows Claude, which has Fable.
    await open(fixtureModel('no-antigravity', { menubar: 'gemini', menubarWindow: 'fableWeek' }));
    await showGroup('Hiển thị');
    const [, , fable2] = within(screen.getByRole('group', { name: 'Số trên thanh menu' })).getAllByRole('button') as HTMLButtonElement[];
    expect(fable2.disabled).toBe(false);
    expect(screen.queryByText(/thanh menu hiện 5 giờ/)).toBeNull();
  });

  it('shows that Keychain access is granted, and no path to another program', async () => {
    await open(fixtureModel('fresh'));
    await showGroup('Dịch vụ và nguồn số');
    expect(screen.queryByLabelText(/quota-axi/)).toBeNull();
    expect(screen.getByText('đã cấp')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Cấp quyền một lần' })).toBeNull();
  });

  it('the Keychain button runs only when pressed, through the fake', async () => {
    const fake = await open(fixtureModel('keychain'));
    await showGroup('Dịch vụ và nguồn số');
    expect(screen.getByText('chưa cấp')).toBeTruthy();
    expect(screen.getByText('Đến khi cấp, dòng Claude chỉ hiện số đọc được lần trước.')).toBeTruthy();
    expect(fake.api.grantKeychain).not.toHaveBeenCalled();
    vi.mocked(fake.api.grantKeychain).mockImplementation(async () => fixtureModel('fresh'));
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Cấp quyền một lần' }));
    });
    expect(fake.api.grantKeychain).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Đã cấp quyền Keychain')).toBeTruthy();
  });

  it('while macOS asks, the button waits and says what to press', async () => {
    await open(fixtureModel('keychain', { granting: true }));
    await showGroup('Dịch vụ và nguồn số');
    expect((screen.getByRole('button', { name: 'Cấp quyền một lần' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('macOS sẽ hỏi. Bấm "Luôn cho phép". App chỉ đọc đăng nhập của Claude Code để lấy số, không làm mới hay sửa nó.')).toBeTruthy();
  });

  it('says whether oauth-client.json is in place and where it goes', async () => {
    await open(fixtureModel('fresh'));
    await showGroup('Dịch vụ và nguồn số');
    expect(screen.getByText('có, dùng được')).toBeTruthy();
    expect(screen.getByText('Quyền 600, chỉ có trên Mac này.')).toBeTruthy();
    // The path waits behind "Chi tiết".
    expect(screen.queryByText('~/Library/Application Support/com.shin315.quota-station/oauth-client.json')).toBeNull();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chi tiết: OAuth client Antigravity' }));
    });
    const more = screen.getByRole('dialog');
    expect(within(more).getByText('~/Library/Application Support/com.shin315.quota-station/oauth-client.json')).toBeTruthy();
    expect(more.textContent).toMatch(/quyền 600, chỉ có trên Mac này\. README ghi định dạng\.$/);
    cleanup();
    await open(fixtureModel('fresh', { antigravity: agyFixture({ client: 'tooOpen' }) }));
    await showGroup('Dịch vụ và nguồn số');
    expect(screen.getByText('cần quyền 600')).toBeTruthy();
    cleanup();
    await open(fixtureModel('fresh', { antigravity: agyFixture({ client: 'missing' }) }));
    await showGroup('Dịch vụ và nguồn số');
    expect(screen.getByText('chưa có')).toBeTruthy();
  });

  it('the email switch is on by default and turning it off saves through the fake', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Hiển thị');
    const toggle = screen.getByRole('switch', { name: 'Hiện email' });
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    await act(async () => {
      fireEvent.click(toggle);
    });
    expect(fake.api.setShowNames).toHaveBeenCalledWith(false);
    expect(screen.getByRole('switch', { name: 'Hiện email' }).getAttribute('aria-checked')).toBe('false');
    // Hidden, no email or organization is left in the model the window draws.
    expect(JSON.stringify(fake.current().snapshot)).not.toMatch(/@example\.com|Acme/);
  });

  it('the look follows macOS by default and saves Sáng or Tối through the fake', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Hiển thị');
    const group = screen.getByRole('group', { name: 'Giao diện' });
    const pressed = () => within(group).getAllByRole('button').filter((b) => b.getAttribute('aria-pressed') === 'true').map((b) => b.textContent);
    expect(within(group).getAllByRole('button').map((b) => b.textContent)).toEqual(['Theo macOS', 'Sáng', 'Tối']);
    expect(pressed()).toEqual(['Theo macOS']);
    await act(async () => {
      fireEvent.click(within(group).getByRole('button', { name: 'Tối' }));
    });
    expect(fake.api.setTheme).toHaveBeenCalledWith('dark');
    expect(pressed()).toEqual(['Tối']);
    expect(document.documentElement.dataset.theme).toBe('dark');
    document.documentElement.dataset.theme = 'auto';
  });

  it('Chủ đề shows each theme pack drawn in its own tokens, and the one chosen redraws the window', async () => {
    const fake = await open(fixtureModel('fresh', { theme: 'dark' }));
    await showGroup('Hiển thị');
    const group = screen.getByRole('radiogroup', { name: 'Chủ đề' });
    const radios = within(group).getAllByRole('radio');
    expect(radios.map((r) => r.lastElementChild!.textContent)).toEqual(['Mặc định', 'Kính Nhịp', 'Dải sáng tĩnh', 'Một nhịp nhìn', 'Bàn điều khiển', 'Chiều lặng']);
    expect(radios.map((r) => r.getAttribute('aria-checked'))).toEqual(['true', 'false', 'false', 'false', 'false', 'false']);
    // Each picture is drawn by its own pack, in the look on screen.
    const swatches = radios.map((r) => r.querySelector<HTMLElement>('[data-pack]')!);
    expect(swatches.map((s) => s.dataset.pack)).toEqual(['default', 'kinhNhip', 'daiSangTinh', 'motNhipNhin', 'banDieuKhien', 'chieuLang']);
    expect(swatches.every((s) => s.dataset.theme === 'dark')).toBe(true);
    await act(async () => {
      fireEvent.click(within(group).getByRole('radio', { name: 'Chiều lặng' }));
    });
    expect(fake.api.setThemePack).toHaveBeenCalledWith('chieuLang');
    expect(within(group).getByRole('radio', { name: 'Chiều lặng' }).getAttribute('aria-checked')).toBe('true');
    expect(document.documentElement.dataset.pack).toBe('chieuLang');
    document.documentElement.dataset.pack = 'default';
    document.documentElement.dataset.theme = 'auto';
  });

  it('names this Mac and goes back to the name macOS gives when emptied', async () => {
    const fake = await open(fixtureModel('fresh'));
    const input = screen.getByLabelText(/Tên máy này/) as HTMLInputElement;
    expect(input.value).toBe('Mac mini');
    expect(input.placeholder).toBe('Mac mini');
    expect(screen.getByText('Để trống để dùng tên trong macOS: Mac mini.')).toBeTruthy();

    fireEvent.change(input, { target: { value: 'Mac phòng khách' } });
    await act(async () => {
      fireEvent.keyDown(input, { key: 'Enter' });
    });
    expect(fake.api.setMachineName).toHaveBeenCalledWith('Mac phòng khách');
    expect(screen.getByText('Mac phòng khách · cập nhật 40 giây trước')).toBeTruthy();
    expect(screen.getAllByText('Mac phòng khách').length).toBeGreaterThan(0);

    fireEvent.change(input, { target: { value: '' } });
    await act(async () => {
      fireEvent.blur(input);
    });
    expect(fake.api.setMachineName).toHaveBeenLastCalledWith('');
    expect(input.value).toBe('Mac mini');
  });

  it('a name that cannot be saved says so and goes back to the saved one', async () => {
    const fake = await open(fixtureModel('fresh'));
    vi.mocked(fake.api.setMachineName).mockRejectedValue('denied');
    const input = screen.getByLabelText(/Tên máy này/) as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'Mac phòng khách' } });
    await act(async () => {
      fireEvent.keyDown(input, { key: 'Enter' });
    });
    expect(screen.getByText('Chưa lưu được cài đặt')).toBeTruthy();
    expect(input.value).toBe('Mac mini');
    expect(screen.getByText('Mac mini · cập nhật 40 giây trước')).toBeTruthy();
  });

  it('an unchanged name is not saved again', async () => {
    const fake = await open(fixtureModel('fresh'));
    const input = screen.getByLabelText(/Tên máy này/);
    fireEvent.change(input, { target: { value: ' Mac mini ' } });
    fireEvent.blur(input);
    expect(fake.api.setMachineName).not.toHaveBeenCalled();
  });

  it('lists the other Macs with their SSH alias, and adds, renames and removes them', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Nâng cao');
    expect(screen.getByText('Các Mac khác')).toBeTruthy();
    const alias = screen.getByLabelText('Tên kết nối SSH của Mac B') as HTMLInputElement;
    expect(alias.value).toBe('mac-b');
    expect((screen.getByLabelText('Tên kết nối SSH của Mac C') as HTMLInputElement).value).toBe('mac-c');

    // A new row is saved once it has an alias.
    fireEvent.click(screen.getByRole('button', { name: 'Thêm Mac' }));
    expect(screen.getByText('Nhập tên kết nối SSH')).toBeTruthy();
    const name = screen.getByLabelText('Tên của Mac mới') as HTMLInputElement;
    fireEvent.change(name, { target: { value: 'Mac D' } });
    fireEvent.blur(name);
    expect(fake.api.setPeers).not.toHaveBeenCalled();
    const newAlias = screen.getByLabelText('Tên kết nối SSH của Mac D');
    fireEvent.change(newAlias, { target: { value: 'mac-d' } });
    await act(async () => {
      fireEvent.keyDown(newAlias, { key: 'Enter' });
    });
    expect(fake.api.setPeers).toHaveBeenLastCalledWith([
      { name: 'Mac B', alias: 'mac-b' },
      { name: 'Mac C', alias: 'mac-c' },
      { name: 'Mac D', alias: 'mac-d' },
    ]);
    expect(fake.current().rotation.peers.map((p) => p.alias)).toEqual(['mac-b', 'mac-c', 'mac-d']);

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Bỏ Mac C' }));
    });
    expect(fake.api.setPeers).toHaveBeenLastCalledWith([
      { name: 'Mac B', alias: 'mac-b' },
      { name: 'Mac D', alias: 'mac-d' },
    ]);
    expect(screen.queryByLabelText('Tên kết nối SSH của Mac C')).toBeNull();

    const rename = screen.getByLabelText('Tên của Mac D');
    fireEvent.change(rename, { target: { value: 'Mac phòng làm việc' } });
    await act(async () => {
      fireEvent.blur(rename);
    });
    expect(fake.api.setPeers).toHaveBeenLastCalledWith([
      { name: 'Mac B', alias: 'mac-b' },
      { name: 'Mac phòng làm việc', alias: 'mac-d' },
    ]);
    expect(fake.current().rotation.peers.map((p) => p.name)).toEqual(['Mac B', 'Mac phòng làm việc']);
  });

  it('a new row with no alias yet neither blocks nor loses the edits to the others', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Nâng cao');
    fireEvent.click(screen.getByRole('button', { name: 'Thêm Mac' }));
    const name = screen.getByLabelText('Tên của Mac B');
    fireEvent.change(name, { target: { value: 'Mac B2' } });
    await act(async () => {
      fireEvent.blur(name);
    });
    expect(fake.api.setPeers).toHaveBeenLastCalledWith([
      { name: 'Mac B2', alias: 'mac-b' },
      { name: 'Mac C', alias: 'mac-c' },
    ]);
    // The new row is still there, waiting for its alias.
    expect(screen.getByLabelText('Tên kết nối SSH của Mac mới')).toBeTruthy();
    expect(screen.getByText('Nhập tên kết nối SSH')).toBeTruthy();
  });

  it('an alias listed twice is refused on the row that repeats it', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Nâng cao');
    const alias = screen.getByLabelText('Tên kết nối SSH của Mac C');
    fireEvent.change(alias, { target: { value: 'MAC-B' } });
    await act(async () => {
      fireEvent.blur(alias);
    });
    expect(screen.getByText('Tên kết nối này đã có ở dòng trên.')).toBeTruthy();
    expect(alias.getAttribute('data-invalid')).toBe('true');
    expect(fake.current().rotation.peers.map((p) => p.alias)).toEqual(['mac-b', 'mac-c']);
  });

  it('refuses an alias ssh would read as an option and says why', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Nâng cao');
    const alias = screen.getByLabelText('Tên kết nối SSH của Mac B');
    fireEvent.change(alias, { target: { value: '-oProxyCommand=x' } });
    await act(async () => {
      fireEvent.blur(alias);
    });
    expect(fake.api.setPeers).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Tên kết nối chỉ gồm chữ, số và . - _ @, không bắt đầu bằng dấu gạch.')).toBeTruthy();
    expect(fake.current().rotation.peers[0].alias).toBe('mac-b');
  });

  it('Thử SSH reads each Mac and shows which one it reached', async () => {
    const fake = fakeApi(fixtureModel('fresh'), 'settings', { unreachable: ['mac-c'] });
    vi.spyOn(fake.api, 'testPeer');
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('heading', { name: 'Cài đặt' });
    await showGroup('Nâng cao');
    expect(screen.getAllByText('Chưa thử')).toHaveLength(2);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Thử SSH' }));
    });
    expect(fake.api.testPeer).toHaveBeenCalledWith('mac-b');
    expect(fake.api.testPeer).toHaveBeenCalledWith('mac-c');
    expect(screen.getByText('SSH ổn')).toBeTruthy();
    expect(screen.getByText('SSH hết giờ chờ. Kiểm tên kết nối hoặc bật máy đó.')).toBeTruthy();
  });

  it('with no other Mac the rules stay on this Mac', async () => {
    await open(fixtureModel('fresh', { rotation: { ...fixtureModel('fresh').rotation, peers: [] } }));
    await showGroup('Nâng cao');
    expect(screen.getByText('Chưa có Mac nào. Luật xoay vòng chỉ lưu trên máy này.')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Thử SSH' }) as HTMLButtonElement).disabled).toBe(true);
  });
});

describe('settings for M6', () => {
  async function openWith(model: Parameters<typeof open>[0], firstmate = firstmateFixture()) {
    const fake = fakeApi(model, 'settings', { firstmate });
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('heading', { name: 'Cài đặt' });
    return fake;
  }
  const fmInput = () => screen.getByLabelText(/^File của firstmate/) as HTMLInputElement;

  it('Firstmate\'s file has no path built in, and only quota-pool.last is taken', async () => {
    const fake = await openWith(fixtureModel('fresh'));
    await showGroup('Nâng cao');
    expect(fmInput().value).toBe('');
    expect(screen.getByText('Để trống nếu máy này không có firstmate.')).toBeTruthy();

    fireEvent.change(fmInput(), { target: { value: '~/notes/secrets.txt' } });
    await act(async () => {
      fireEvent.blur(fmInput());
    });
    expect(fake.api.setFirstmateFile).toHaveBeenCalledWith('~/notes/secrets.txt');
    expect(screen.getByText('File phải tên là quota-pool.last.')).toBeTruthy();
    expect(fake.current().firstmate.status).toBe('notSet');

    fireEvent.change(fmInput(), { target: { value: '~/example/watch/quota-pool.last' } });
    await act(async () => {
      fireEvent.keyDown(fmInput(), { key: 'Enter' });
    });
    expect(fake.current().firstmate.file).toBe('~/example/watch/quota-pool.last');
    expect(screen.getByText('Đọc được ✓')).toBeTruthy();
  });

  it('a path with no file is refused and the one before stays', async () => {
    const fake = fakeApi(fixtureModel('fresh', { firstmate: firstmateFixture() }), 'settings');
    vi.spyOn(fake.api, 'setFirstmateFile');
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('heading', { name: 'Cài đặt' });
    await showGroup('Nâng cao');
    fireEvent.change(fmInput(), { target: { value: '~/nowhere/quota-pool.last' } });
    await act(async () => {
      fireEvent.blur(fmInput());
    });
    expect(screen.getByText('Không có file ở đường dẫn này.')).toBeTruthy();
    expect(fake.current().firstmate.file).toBe('~/example/quota-pool.last');
    expect(fake.current().firstmate.status).toBe('ok');
  });

  it('Thông báo shows whether macOS lets the cards show, with "Gửi thử" and a switch for each kind', async () => {
    const fake = await openWith(fixtureModel('fresh'));
    await showGroup('Thông báo');
    expect(screen.getByText('đang bật')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: /^Gửi thử: / })).toHaveLength(5);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Gửi thử: Hạn mức Claude trong vòng sắp cạn / có lại' }));
    });
    expect(fake.api.testAlert).toHaveBeenCalledWith('pool');
    await act(async () => {
      fireEvent.click(screen.getByRole('switch', { name: 'Tài khoản cần đăng nhập lại' }));
    });
    expect(fake.api.setAlertSwitch).toHaveBeenCalledWith('login', false);
    expect(fake.current().alerts.switches.login).toBe(false);
    expect(fake.api.notifyRefresh).toHaveBeenCalled();
  });

  it('with notifications off, it says so and opens System Settings', async () => {
    const fake = await openWith(fixtureModel('fresh', { alerts: alertsFixture({ permission: 'denied' }) }));
    await showGroup('Thông báo');
    expect(screen.getByText('đang tắt')).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở Cài đặt hệ thống' }));
    });
    expect(fake.api.openNotificationSettings).toHaveBeenCalledTimes(1);
  });

  it('a run outside the packaged app says macOS shows no cards', async () => {
    await openWith(fixtureModel('fresh', { alerts: alertsFixture({ permission: 'unbundled' }) }));
    await showGroup('Thông báo');
    expect(screen.getByText('bản chạy thử')).toBeTruthy();
    expect(screen.getByText('Chỉ app đã đóng gói mới gửi được thẻ macOS. Thông báo vẫn có ở chuông trên đầu trang.')).toBeTruthy();
  });
});

describe('settings for M7: the app itself', () => {
  async function openApp(over: Partial<Model['app']> = {}, options: FakeOptions = {}) {
    const fake = fakeApi(fixtureModel('fresh', { app: appFixture(over) }), 'settings', options);
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('heading', { name: 'Cài đặt' });
    return fake;
  }
  const loginSwitch = () => screen.getByRole('switch', { name: 'Mở khi đăng nhập' });
  const updateButton = () => screen.getByRole('button', { name: /Cập nhật từ mã nguồn|Đang cập nhật…/ }) as HTMLButtonElement;
  const sourceInput = () => screen.getByLabelText(/^Mã nguồn/) as HTMLInputElement;

  it('shows the version and the commit it was built from', async () => {
    await openApp();
    await showGroup('Cập nhật');
    expect(screen.getByText('0.1.0 (a1b2c3d) · build từ mã nguồn')).toBeTruthy();
    cleanup();
    await openApp({ commit: null });
    await showGroup('Cập nhật');
    expect(screen.getByText('0.1.0 · build từ mã nguồn')).toBeTruthy();
  });

  it('launching at login is off by default, and opening the tab only asks macOS', async () => {
    const fake = await openApp();
    expect(loginSwitch().getAttribute('aria-checked')).toBe('false');
    expect(fake.api.settingsOpened).toHaveBeenCalled();
    expect(fake.api.setLaunchAtLogin).not.toHaveBeenCalled();
  });

  it('the switch turns the login item on and off through the fake', async () => {
    const fake = await openApp();
    await act(async () => {
      fireEvent.click(loginSwitch());
    });
    expect(fake.api.setLaunchAtLogin).toHaveBeenLastCalledWith(true);
    expect(loginSwitch().getAttribute('aria-checked')).toBe('true');
    await act(async () => {
      fireEvent.click(loginSwitch());
    });
    expect(fake.api.setLaunchAtLogin).toHaveBeenLastCalledWith(false);
    expect(loginSwitch().getAttribute('aria-checked')).toBe('false');
  });

  it('a login item waiting for approval says where to allow it', async () => {
    const fake = await openApp({}, { loginApproval: true });
    await act(async () => {
      fireEvent.click(loginSwitch());
    });
    expect(screen.getByText('chờ duyệt')).toBeTruthy();
    expect(screen.getByText('macOS cần bạn bật Quota Station trong Cài đặt hệ thống › Cài đặt chung › Mục đăng nhập.')).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở Cài đặt hệ thống: Mục đăng nhập' }));
    });
    expect(fake.api.openLoginItems).toHaveBeenCalledTimes(1);
  });

  it('outside the installed app the switch and the update are off', async () => {
    const fake = await openApp({ launchAtLogin: 'unavailable', bundled: false });
    expect((loginSwitch() as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('Chỉ bản .app đã cài mới mở khi đăng nhập được.')).toBeTruthy();
    await showGroup('Nâng cao');
    expect(updateButton().disabled).toBe(true);
    expect(screen.getByText('Bản đang chạy không phải .app đã cài, nên không có gì để cập nhật.')).toBeTruthy();
    expect(fake.api.updateFromSource).not.toHaveBeenCalled();
  });

  it('a refusal from macOS is shown and the switch stays off', async () => {
    const fake = await openApp();
    vi.mocked(fake.api.setLaunchAtLogin).mockRejectedValue('Operation not permitted');
    await act(async () => {
      fireEvent.click(loginSwitch());
    });
    expect(screen.getByText('Chưa đổi được: Operation not permitted')).toBeTruthy();
    expect(loginSwitch().getAttribute('aria-checked')).toBe('false');
  });

  it('the source folder must be a clone with the install script', async () => {
    const fake = await openApp({ source: null, sourceStatus: 'notSet' });
    await showGroup('Nâng cao');
    expect(screen.getByText('Để trống trên Mac cài bằng gói build từ Mac khác.')).toBeTruthy();
    expect(screen.getByText('Đặt thư mục mã nguồn ở trên để cập nhật.')).toBeTruthy();
    expect(updateButton().disabled).toBe(true);

    fireEvent.change(sourceInput(), { target: { value: '~/Documents' } });
    await act(async () => {
      fireEvent.keyDown(sourceInput(), { key: 'Enter' });
    });
    expect(fake.api.setSourceDir).toHaveBeenLastCalledWith('~/Documents');
    expect(screen.getByText('Thư mục này không phải bản clone Quota Station: cần .git và scripts/install.sh.')).toBeTruthy();

    fireEvent.change(sourceInput(), { target: { value: '~/nowhere/quota-station' } });
    await act(async () => {
      fireEvent.blur(sourceInput());
    });
    expect(screen.getByText('Không có thư mục ở đường dẫn này.')).toBeTruthy();

    fireEvent.change(sourceInput(), { target: { value: '~/Code/quota-station' } });
    await act(async () => {
      fireEvent.blur(sourceInput());
    });
    expect(fake.current().app.source).toBe('~/Code/quota-station');
    expect(screen.getByText('Có scripts/install.sh ✓')).toBeTruthy();
    expect(updateButton().disabled).toBe(false);
  });

  it('an update pulls, installs and offers to open the new copy', async () => {
    const fake = await openApp();
    await showGroup('Nâng cao');
    await act(async () => {
      fireEvent.click(updateButton());
    });
    expect(fake.api.updateFromSource).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('Đã cài 0.1.1 (b2c3d4e). Mở lại để dùng bản mới.')).toBeTruthy();
    expect(fake.relaunched()).toBe(0);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở lại' }));
    });
    expect(fake.api.relaunch).toHaveBeenCalledTimes(1);
    expect(fake.relaunched()).toBe(1);
  });

  it('while it runs it says which step it is on', async () => {
    await openApp({ updating: 'pulling' });
    await showGroup('Nâng cao');
    expect(screen.getByText('Đang lấy mã mới (git pull --ff-only)…')).toBeTruthy();
    expect(updateButton().disabled).toBe(true);
    expect(sourceInput().disabled).toBe(true);
    cleanup();
    await openApp({ updating: 'installing' });
    await showGroup('Nâng cao');
    expect(screen.getByText('Đang build và cài. Mất vài phút; app vẫn chạy như thường.')).toBeTruthy();
  });

  it('nothing new installs nothing', async () => {
    const fake = await openApp({}, { update: () => ({ kind: 'upToDate', commit: 'a1b2c3d4e5f60718293a4b5c6d7e8f9012345678' }) });
    await showGroup('Nâng cao');
    await act(async () => {
      fireEvent.click(updateButton());
    });
    expect(await screen.findByText('Đã là bản mới nhất (a1b2c3d).')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Mở lại' })).toBeNull();
    expect(fake.api.relaunch).not.toHaveBeenCalled();
  });

  it('a pull or an install that fails says why, and where the log is', async () => {
    await openApp({ error: { kind: 'pull', detail: 'fatal: Not possible to fast-forward, aborting.' } });
    await showGroup('Nâng cao');
    expect(screen.getByText('git pull không chạy được: fatal: Not possible to fast-forward, aborting.')).toBeTruthy();
    cleanup();
    await openApp({ error: { kind: 'install', detail: 'cargo is not installed' } });
    await showGroup('Nâng cao');
    expect(
      screen.getByText(
        'Build hoặc cài lỗi: cargo is not installed. Log đầy đủ: ~/Library/Application Support/com.shin315.quota-station/update/install.log',
      ),
    ).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Mở lại' })).toBeNull();
  });

  it('opening again waits for unsaved rules', async () => {
    const fake = await openApp({ outcome: { kind: 'installed', commit: 'b2c3d4e5f6', version: '0.1.1' } });
    await showGroup('Nâng cao');
    vi.mocked(fake.api.relaunch).mockRejectedValue('rulesDirty');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở lại' }));
    });
    expect(screen.getByText('Luật xoay vòng còn thay đổi chưa lưu. Lưu hoặc bỏ trước khi mở lại.')).toBeTruthy();
  });
});

describe('settings: Nâng cao, alert 1 and the backup folder', () => {
  it('Nâng cao is the last tab; off, it hides its rows and its tabs', async () => {
    const fake = await open(fixtureModel('fresh'));
    expect(screen.getAllByRole('tab').at(-1)!.textContent).toBe('Nâng cao');
    await showGroup('Nâng cao');
    expect(screen.getByLabelText(/^Mã nguồn/)).toBeTruthy();
    expect(screen.getByText('Các Mac khác')).toBeTruthy();
    expect(screen.getByLabelText(/^File của firstmate/)).toBeTruthy();

    await act(async () => {
      fireEvent.click(screen.getByRole('switch', { name: 'Firstmate' }));
    });
    expect(fake.api.setFirstmate).toHaveBeenCalledWith(false);
    expect(screen.queryByLabelText(/^File của firstmate/)).toBeNull();

    await act(async () => {
      fireEvent.click(screen.getByRole('switch', { name: 'Hiện phần nâng cao' }));
    });
    expect(fake.api.setAdvanced).toHaveBeenCalledWith(false);
    expect(screen.queryByLabelText(/^Mã nguồn/)).toBeNull();
    expect(screen.queryByText('Các Mac khác')).toBeNull();
    expect(screen.queryByRole('switch', { name: 'Firstmate' })).toBeNull();
    const sidebar = screen.getByRole('navigation', { name: 'Các tab' });
    expect(within(sidebar).queryByRole('button', { name: 'Luật xoay vòng' })).toBeNull();
    // The version stays where everyone sees it.
    await showGroup('Cập nhật');
    expect(screen.getByText('0.1.0 (a1b2c3d) · build từ mã nguồn')).toBeTruthy();
  });

  it('a build without the author\'s tools has no Nâng cao card, switch or row', async () => {
    await open(communityModel(fixtureModel('fresh')));
    expect(screen.getAllByRole('tab').map((t) => t.textContent)).not.toContain('Nâng cao');
    for (const tab of screen.getAllByRole('tab').map((t) => t.textContent!)) {
      await showGroup(tab);
      expect(screen.queryByRole('switch', { name: 'Hiện phần nâng cao' })).toBeNull();
      expect(screen.queryByRole('switch', { name: 'Firstmate' })).toBeNull();
      expect(screen.queryByText('Các Mac khác')).toBeNull();
      expect(screen.queryByLabelText(/^Mã nguồn/)).toBeNull();
      expect(screen.queryByLabelText(/^File của firstmate/)).toBeNull();
      expect(screen.queryByRole('button', { name: 'Cập nhật từ mã nguồn' })).toBeNull();
    }
    // Alerts 2 and 5 are theirs too.
    await showGroup('Thông báo');
    expect(screen.queryByText('Tài khoản sắp rời vòng')).toBeNull();
    expect(screen.queryByText('Firstmate đang chờ hạn mức Claude')).toBeNull();
    // What every build has stays.
    await showGroup('Cập nhật');
    expect(screen.getByText('0.1.0 (a1b2c3d) · build từ mã nguồn')).toBeTruthy();
    await showGroup('Sao lưu');
    expect(screen.getByLabelText('Thư mục sao lưu')).toBeTruthy();
  });

  it('alert 1 takes its two thresholds here, and says why one is refused', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Thông báo');
    const dry = screen.getByLabelText('Còn bao nhiêu phần trăm thì coi là sắp cạn') as HTMLInputElement;
    const room = screen.getByLabelText('Tài khoản khác còn bao nhiêu phần trăm thì coi là còn hạn mức') as HTMLInputElement;
    expect([dry.value, room.value]).toEqual(['10', '15']);
    fireEvent.change(dry, { target: { value: '20' } });
    await act(async () => {
      fireEvent.blur(dry);
    });
    expect(fake.api.setPoolAlert).toHaveBeenCalledWith(20, 15);
    expect(screen.getByText('Ngưỡng của tài khoản khác phải lớn hơn ngưỡng cạn.')).toBeTruthy();
    fireEvent.change(room, { target: { value: '30' } });
    await act(async () => {
      fireEvent.blur(room);
    });
    expect(fake.api.setPoolAlert).toHaveBeenLastCalledWith(20, 30);
    expect(screen.getByText('Thông báo 1 dùng hai ngưỡng này.')).toBeTruthy();
  });

  it('the backup folder is iCloud Drive’s until another is chosen', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Sao lưu');
    const input = screen.getByLabelText('Thư mục sao lưu') as HTMLInputElement;
    expect(input.value).toBe('');
    expect(screen.getByText('Để trống để dùng iCloud Drive › Quota Station Agents.')).toBeTruthy();
    fireEvent.change(input, { target: { value: 'Backups' } });
    await act(async () => {
      fireEvent.blur(input);
    });
    expect(screen.getByText('Không có thư mục ở đường dẫn này.')).toBeTruthy();
    fireEvent.change(input, { target: { value: '/Volumes/Backup/QS' } });
    await act(async () => {
      fireEvent.blur(input);
    });
    expect(fake.api.backupSetFolder).toHaveBeenLastCalledWith('/Volumes/Backup/QS');
    expect(screen.getByText('Gói mới vào thư mục này. Gói cũ vẫn ở chỗ cũ.')).toBeTruthy();
  });
});

describe('settings: the layout of settings-R2, its groups as tabs', () => {
  const bar = () => screen.getByRole('tablist', { name: 'Các mục cài đặt' });
  const tabs = () => within(bar()).getAllByRole('tab');
  const shown = () => screen.getAllByRole('region').filter((r) => r.closest('main'));
  const regionName = (r: HTMLElement) => r.getAttribute('aria-label') ?? document.getElementById(r.getAttribute('aria-labelledby')!)!.textContent;

  it('has a tab for every card, in the cards’ order, and shows Chung alone at first', async () => {
    await open(fixtureModel('fresh'));
    expect(tabs().map((b) => b.textContent)).toEqual([
      'Chung',
      'Hiển thị',
      'Thông báo',
      'Cập nhật',
      'Dịch vụ và nguồn số',
      'Biến môi trường',
      'Sao lưu',
      'Chẩn đoán',
      'Nâng cao',
    ]);
    expect(tabs().map((b) => b.getAttribute('aria-selected'))).toEqual(['true', 'false', 'false', 'false', 'false', 'false', 'false', 'false', 'false']);
    expect(shown().map(regionName)).toEqual(['Chung']);
    // The panel shown is named by its tab.
    const panel = screen.getByRole('tabpanel');
    expect(panel.getAttribute('aria-labelledby')).toBe(tabs()[0].id);
    // Each tab shows its card alone, a region named by the card's title.
    const cards: string[] = [];
    for (const tab of tabs().map((b) => b.textContent!)) {
      await showGroup(tab);
      expect(shown()).toHaveLength(1);
      cards.push(regionName(shown()[0])!);
    }
    expect(cards).toEqual([
      'Chung',
      'Hiển thị',
      'Thông báo',
      'Cập nhật',
      'Dịch vụ và nguồn số',
      'Biến môi trường đè đăng nhập',
      'Sao lưu',
      'Chẩn đoán',
      'Nâng cao',
    ]);
  });

  it('a tab shows its card and marks itself; the arrows, Home and End move along the bar', async () => {
    await open(fixtureModel('fresh'));
    await showGroup('Dịch vụ và nguồn số');
    expect(shown().map(regionName)).toEqual(['Dịch vụ và nguồn số']);
    expect(within(bar()).getByRole('tab', { name: 'Dịch vụ và nguồn số' }).getAttribute('aria-selected')).toBe('true');
    expect(within(bar()).getByRole('tab', { name: 'Chung' }).getAttribute('aria-selected')).toBe('false');
    // One stop in the Tab order: the tab shown.
    expect(tabs().map((b) => b.tabIndex)).toEqual([-1, -1, -1, -1, 0, -1, -1, -1, -1]);
    // Radix moves the focus on the next turn.
    const key = async (k: string) => {
      await act(async () => {
        fireEvent.keyDown(document.activeElement!, { key: k });
        await new Promise((r) => setTimeout(r, 0));
      });
    };
    tabs()[4].focus();
    await key('ArrowRight');
    expect(document.activeElement!.textContent).toBe('Biến môi trường');
    expect(shown().map(regionName)).toEqual(['Biến môi trường đè đăng nhập']);
    await key('ArrowLeft');
    await key('ArrowLeft');
    expect(document.activeElement!.textContent).toBe('Cập nhật');
    await key('End');
    expect(document.activeElement!.textContent).toBe('Nâng cao');
    await key('Home');
    expect(document.activeElement!.textContent).toBe('Chung');
    expect(shown().map(regionName)).toEqual(['Chung']);
  });

  it('the window keeps the tab picked: away to another page and back, the same one shows', async () => {
    const fake = await open(fixtureModel('fresh'));
    await showGroup('Thông báo');
    expect(fake.api.setSettingsGroup).toHaveBeenLastCalledWith('notify');
    const sidebar = screen.getByRole('navigation', { name: 'Các tab' });
    await act(async () => {
      fireEvent.click(within(sidebar).getByRole('button', { name: 'Tổng quan' }));
    });
    expect(screen.queryByRole('tablist', { name: 'Các mục cài đặt' })).toBeNull();
    await act(async () => {
      fireEvent.click(within(sidebar).getByRole('button', { name: 'Cài đặt' }));
    });
    expect(shown().map(regionName)).toEqual(['Thông báo']);
    // A window opened again asks the app which one it was.
    cleanup();
    vi.mocked(fake.api.settingsGroup).mockClear();
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('heading', { name: 'Cài đặt' });
    expect(fake.api.settingsGroup).toHaveBeenCalled();
    expect(shown().map(regionName)).toEqual(['Thông báo']);
  });

  it('links from elsewhere open the tab they are about', async () => {
    // Firstmate's "Mở Cài đặt" opens Nâng cao, where its file is.
    const fake = fakeApi(fixtureModel('fresh'), 'firstmate');
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByText('Chưa nối với firstmate');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở Cài đặt' }));
    });
    expect(shown().map(regionName)).toEqual(['Nâng cao']);
    expect(fake.api.setSettingsGroup).toHaveBeenLastCalledWith('advanced');
    // Chép sang Mac khác with no Mac yet: its "Mở Cài đặt" opens Nâng cao too, where the Macs are.
    cleanup();
    const copy = fakeApi(fixtureModel('fresh', { rotation: { ...fixtureModel('fresh').rotation, peers: [] } }), 'copy');
    render(
      <div className="qs app-window">
        <MainWindowApp api={copy.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByText('Chưa có Mac nào để chép');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở Cài đặt' }));
    });
    expect(shown().map(regionName)).toEqual(['Nâng cao']);
    // The variables' banners and alerts open Biến môi trường (EnvCard.test.tsx).
  });

  it('a variable overriding a login puts its card, and its tab, first; a release build has no Nâng cao', async () => {
    await open(fixtureModel('fresh', { env: envFixture({ found: [API_KEY] }) }));
    expect(tabs()[0].textContent).toBe('Biến môi trường');
    expect(tabs().filter((b) => b.textContent === 'Biến môi trường')).toHaveLength(1);
    // Chung still shows first.
    expect(shown().map(regionName)).toEqual(['Chung']);
    cleanup();
    await open(communityModel(fixtureModel('fresh')));
    expect(tabs().map((b) => b.textContent)).not.toContain('Nâng cao');
  });

  it('a row has its name and its line on the left, and its switch at the right end', async () => {
    await open(fixtureModel('fresh'));
    for (const tab of tabs().map((b) => b.textContent!)) {
      await showGroup(tab);
      for (const sw of screen.queryAllByRole('switch')) {
        const row = sw.closest('.srow')!;
        const [name, control] = Array.from(row.children);
        // The name comes first; the switch is the last thing in the control's cell.
        expect(name.contains(sw)).toBe(false);
        expect(control.lastElementChild).toBe(sw);
      }
    }
    await showGroup('Thông báo');
    const kinds = screen.getByRole('region', { name: 'Thông báo' });
    const login = within(kinds).getByRole('switch', { name: 'Tài khoản cần đăng nhập lại' }).closest('.srow')!;
    expect(login.children[0].textContent).toBe('Tài khoản cần đăng nhập lạiSắp hết hạn hoặc lỗi đăng nhập. Chi tiết');
  });

  it('a long rule waits behind its "Chi tiết", whole', async () => {
    await open(fixtureModel('fresh'));
    await showGroup('Thông báo');
    expect(screen.getByText('Đang dùng còn ≤ 10%, mọi tài khoản trong vòng còn < 15%.')).toBeTruthy();
    expect(screen.queryByText(/^Khi tài khoản đang dùng còn 10% trở xuống/)).toBeNull();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chi tiết: Hạn mức Claude trong vòng sắp cạn / có lại' }));
    });
    expect(within(screen.getByRole('dialog')).getByText(/^Khi tài khoản đang dùng còn 10% trở xuống và không tài khoản nào trong vòng còn 15%, và khi có lại\./)).toBeTruthy();
    cleanup();
    await open(fixtureModel('fresh'));
    await showGroup('Cập nhật');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chi tiết: Báo khi có bản mới' }));
    });
    expect(within(screen.getByRole('dialog')).getByText('Mỗi ngày một lần qua GitHub Releases. Bản build từ mã nguồn không cài bản tải về: chỉ báo.')).toBeTruthy();
  });
});
