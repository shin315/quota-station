import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { agyFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
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

  it('the email switch is off by default and saves through the fake', async () => {
    const fake = await open(fixtureModel('fresh'));
    const toggle = screen.getByRole('switch', { name: 'Hiện email' });
    expect(toggle.getAttribute('aria-checked')).toBe('false');
    await act(async () => {
      fireEvent.click(toggle);
    });
    expect(fake.api.setShowEmail).toHaveBeenCalledWith(true);
    expect(screen.getByRole('switch', { name: 'Hiện email' }).getAttribute('aria-checked')).toBe('true');
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
