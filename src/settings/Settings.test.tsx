import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
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
    expect(screen.getByText('macOS sẽ hỏi. Bấm "Luôn cho phép". App không đọc Keychain.')).toBeTruthy();
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

  it('has no row that reaches another Mac', async () => {
    await open(fixtureModel('fresh'));
    expect(screen.queryByText(/SSH/)).toBeNull();
    expect(screen.queryByText('Các Mac khác')).toBeNull();
  });
});
