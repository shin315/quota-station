import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { agyFixture, alertsFixture, appFixture, firstmateFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
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

describe('settings tab', () => {
  it('shows the quota-axi in use and that Keychain access is granted', async () => {
    await open(fixtureModel('fresh'));
    expect((screen.getByLabelText(/quota-axi/) as HTMLInputElement).value).toBe('~/.local/bin/quota-axi');
    expect(screen.getByText('0.1.61+cswap.4 ✓')).toBeTruthy();
    expect(screen.getByText('đã cấp')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Cấp quyền một lần' })).toBeNull();
  });

  it('the Keychain button runs only when pressed, through the fake', async () => {
    const fake = await open(fixtureModel('keychain'));
    expect(screen.getByText('chưa cấp')).toBeTruthy();
    expect(screen.getByText('Đến khi cấp, dòng Claude dùng số chưa được cập nhật từ claude-swap.')).toBeTruthy();
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
    expect((screen.getByRole('button', { name: 'Cấp quyền một lần' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('macOS sẽ hỏi. Bấm "Luôn cho phép". App không đọc mục Keychain của Claude.')).toBeTruthy();
  });

  it('says whether oauth-client.json is in place and where it goes', async () => {
    await open(fixtureModel('fresh'));
    expect(screen.getByText('có, dùng được')).toBeTruthy();
    expect(screen.getByText('~/Library/Application Support/com.shin315.quota-station/oauth-client.json')).toBeTruthy();
    cleanup();
    await open(fixtureModel('fresh', { antigravity: agyFixture({ client: 'tooOpen' }) }));
    expect(screen.getByText('cần quyền 600')).toBeTruthy();
    cleanup();
    await open(fixtureModel('fresh', { antigravity: agyFixture({ client: 'missing' }) }));
    expect(screen.getByText('chưa có')).toBeTruthy();
  });

  it('the email switch is on by default and turning it off saves through the fake', async () => {
    const fake = await open(fixtureModel('fresh'));
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

  it('refuses a path with no file and offers to find quota-axi again', async () => {
    const fake = await open(fixtureModel('fresh'));
    vi.mocked(fake.api.setQuotaAxiPath).mockImplementation(async (path: string) => {
      if (path) throw new Error('notFound');
      return fake.current();
    });
    const input = screen.getByLabelText(/quota-axi/);
    fireEvent.change(input, { target: { value: '~/nowhere/quota-axi' } });
    await act(async () => {
      fireEvent.keyDown(input, { key: 'Enter' });
    });
    expect(fake.api.setQuotaAxiPath).toHaveBeenCalledWith('~/nowhere/quota-axi');
    expect(screen.getByText('Không có file ở đường dẫn này.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Tự tìm lại' }));
    expect(fake.api.setQuotaAxiPath).toHaveBeenLastCalledWith('');
  });

  it('names a quota-axi without the +cswap fork', async () => {
    await open(fixtureModel('fresh', { quotaAxi: { path: '/opt/homebrew/bin/quota-axi', custom: true, exists: true, version: '0.1.61', fork: false } }));
    expect(screen.getByText('Bản 0.1.61 không có +cswap: thiếu số claude-swap và luật xoay vòng.')).toBeTruthy();
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
    const alias = screen.getByLabelText('Tên kết nối SSH của Mac C');
    fireEvent.change(alias, { target: { value: 'MAC-B' } });
    await act(async () => {
      fireEvent.blur(alias);
    });
    expect(screen.getByText('Tên kết nối này đã có ở dòng trên.')).toBeTruthy();
    expect(alias.className).toContain('bad');
    expect(fake.current().rotation.peers.map((p) => p.alias)).toEqual(['mac-b', 'mac-c']);
  });

  it('refuses an alias ssh would read as an option and says why', async () => {
    const fake = await open(fixtureModel('fresh'));
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
    expect(screen.getByText('đang tắt')).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở Cài đặt hệ thống' }));
    });
    expect(fake.api.openNotificationSettings).toHaveBeenCalledTimes(1);
  });

  it('a run outside the packaged app says macOS shows no cards', async () => {
    await openWith(fixtureModel('fresh', { alerts: alertsFixture({ permission: 'unbundled' }) }));
    expect(screen.getByText('bản chạy thử')).toBeTruthy();
    expect(screen.getByText('Chỉ app đã đóng gói mới gửi được thẻ macOS. Cảnh báo vẫn có ở tab Cảnh báo.')).toBeTruthy();
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
    expect(screen.getByText('0.1.0 · a1b2c3d')).toBeTruthy();
    cleanup();
    await openApp({ commit: null });
    expect(screen.getByText('0.1.0')).toBeTruthy();
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
    expect(screen.getByText('Đang lấy mã mới (git pull --ff-only)…')).toBeTruthy();
    expect(updateButton().disabled).toBe(true);
    expect(sourceInput().disabled).toBe(true);
    cleanup();
    await openApp({ updating: 'installing' });
    expect(screen.getByText('Đang build và cài. Mất vài phút; app vẫn chạy như thường.')).toBeTruthy();
  });

  it('nothing new installs nothing', async () => {
    const fake = await openApp({}, { update: () => ({ kind: 'upToDate', commit: 'a1b2c3d4e5f60718293a4b5c6d7e8f9012345678' }) });
    await act(async () => {
      fireEvent.click(updateButton());
    });
    expect(await screen.findByText('Đã là bản mới nhất (a1b2c3d).')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Mở lại' })).toBeNull();
    expect(fake.api.relaunch).not.toHaveBeenCalled();
  });

  it('a pull or an install that fails says why, and where the log is', async () => {
    await openApp({ error: { kind: 'pull', detail: 'fatal: Not possible to fast-forward, aborting.' } });
    expect(screen.getByText('git pull không chạy được: fatal: Not possible to fast-forward, aborting.')).toBeTruthy();
    cleanup();
    await openApp({ error: { kind: 'install', detail: 'cargo is not installed' } });
    expect(
      screen.getByText(
        'Build hoặc cài lỗi: cargo is not installed. Log đầy đủ: ~/Library/Application Support/com.shin315.quota-station/update/install.log',
      ),
    ).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Mở lại' })).toBeNull();
  });

  it('opening again waits for unsaved rules', async () => {
    const fake = await openApp({ outcome: { kind: 'installed', commit: 'b2c3d4e5f6', version: '0.1.1' } });
    vi.mocked(fake.api.relaunch).mockRejectedValue('rulesDirty');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở lại' }));
    });
    expect(screen.getByText('Luật xoay vòng còn thay đổi chưa lưu. Lưu hoặc bỏ trước khi mở lại.')).toBeTruthy();
  });
});
