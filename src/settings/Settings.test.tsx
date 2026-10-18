import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { SettingsApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Model } from '../lib/model';

async function open(model: Model) {
  const fake = fakeApi(model);
  render(
    <div className="qs app-settings">
      <SettingsApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { name: 'Cài đặt' });
  return fake;
}

afterEach(cleanup);

describe('settings', () => {
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
    fake.api.grantKeychain.mockImplementation(async () => fixtureModel('fresh'));
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
    fake.api.setQuotaAxiPath.mockImplementation(async (path: string) => {
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
});
