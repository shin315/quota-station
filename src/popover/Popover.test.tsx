import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PopoverApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { failing, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

const clock = () => FIXTURE_NOW;

async function open(model: Model) {
  const fake = fakeApi(model);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-popover">
      <PopoverApp api={fake.api} clock={clock} />
    </div>,
  );
  await screen.findByRole('dialog', { name: 'Quota Station' });
  return fake;
}

afterEach(cleanup);

describe('popover', () => {
  it('shows every account: Claude rows switch, the active row and Antigravity rows do not', async () => {
    const fake = await open(fixtureModel('fresh'));
    expect(screen.getByText('Mac mini · cập nhật 40 giây trước')).toBeTruthy();
    expect(screen.getByText('Claude còn tương đương 2,3 tài khoản')).toBeTruthy();

    const claude = screen.getByRole('list', { name: 'CLAUDE CODE' });
    const rows = within(claude).getAllByRole('listitem').map((li) => within(li).getByRole('button'));
    expect(rows).toHaveLength(4);
    expect(rows[1].getAttribute('aria-label')).toMatch(/^Team A \(claude-swap, tài khoản 2\), đang dùng\./);
    expect(rows[1].getAttribute('aria-disabled')).toBe('true');
    expect(rows[2].getAttribute('aria-label')).toMatch(/^Team B .*\. Bấm để chuyển$/);
    expect(rows[2].getAttribute('aria-disabled')).toBeNull();

    const agy = within(screen.getByRole('list', { name: 'ANTIGRAVITY' })).getAllByRole('button');
    expect(agy).toHaveLength(2);
    const ultra1 = agy.find((b) => !b.getAttribute('aria-label')!.includes('đang dùng'))!;
    expect(ultra1.getAttribute('aria-disabled')).toBe('true');
    expect(ultra1.getAttribute('title')).toBe('Cockpit · tài khoản 1 · Chuyển Antigravity có ở bản sau');
    fireEvent.click(ultra1);
    fireEvent.click(rows[1]);
    expect(screen.queryByRole('dialog', { name: /Chuyển/ })).toBeNull();
    expect(fake.api.claudeSessions).not.toHaveBeenCalled();
    expect(fake.api.resizePopover).toHaveBeenCalled();
  });

  it('the footer has the automatic switching toggle and why Claude is on its account', async () => {
    await open(fixtureModel('fresh'));
    const toggle = screen.getByRole('switch', { name: 'Tự chuyển Claude' });
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    expect(screen.getByText('14:19 · Vẫn dùng Team A: chưa tới ngưỡng tự chuyển.')).toBeTruthy();
    expect(screen.getByText('09:12 · Switcher chuyển sang Team A.')).toBeTruthy();
  });

  it('opens the main window on settings from the gear, on Tổng quan from the window button, and hides on Escape', async () => {
    const fake = await open(fixtureModel('fresh'));
    fireEvent.click(screen.getByRole('button', { name: 'Cài đặt' }));
    expect(fake.api.openWindow).toHaveBeenLastCalledWith('settings');
    fireEvent.click(screen.getByRole('button', { name: 'Mở cửa sổ chính' }));
    expect(fake.api.openWindow).toHaveBeenLastCalledWith('overview');
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(fake.api.hidePopover).toHaveBeenCalledTimes(1);
  });

  it('asks for Keychain access only when the button is pressed', async () => {
    const fake = await open(fixtureModel('keychain'));
    expect(fake.api.grantKeychain).not.toHaveBeenCalled();
    const granted = fixtureModel('fresh');
    vi.mocked(fake.api.grantKeychain).mockImplementation(async () => granted);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Cấp quyền một lần' }));
    });
    expect(fake.api.grantKeychain).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Đã cấp quyền Keychain')).toBeTruthy();
    expect(screen.getByText('Dòng Claude giờ đọc trực tiếp từ quota-axi.')).toBeTruthy();
  });

  it('a refused Keychain prompt says so', async () => {
    const fake = await open(fixtureModel('keychain'));
    vi.mocked(fake.api.grantKeychain).mockImplementation(async () => fake.current());
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Cấp quyền một lần' }));
    });
    expect(fake.api.grantKeychain).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Chưa cấp được quyền Keychain')).toBeTruthy();
  });

  it('"Để sau" hides the Keychain banner', async () => {
    await open(fixtureModel('keychain'));
    fireEvent.click(screen.getByRole('button', { name: 'Để sau' }));
    expect(screen.queryByText('Chưa cấp quyền Keychain cho quota-axi')).toBeNull();
  });

  it('"Làm mới" reports once the read it asked for is done', async () => {
    const fake = await open(fixtureModel('stale', { reads: 3 }));
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Làm mới' }));
    });
    expect(fake.api.refresh).toHaveBeenCalledTimes(1);
    expect(screen.queryByText('Đã làm mới')).toBeNull();
    act(() => fake.push(fixtureModel('fresh', { reads: 4 })));
    expect(screen.getByText('Đã làm mới')).toBeTruthy();
    expect(screen.getByText('Đọc lại số lúc 14:20.')).toBeTruthy();
  });

  it('quota-axi missing offers the settings and keeps the last numbers', async () => {
    const fake = await open(fixtureModel('fresh', { errors: failing('notFound') }));
    expect(screen.getByText('Không chạy được quota-axi')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Chọn đường dẫn…' }));
    expect(fake.api.openWindow).toHaveBeenCalledWith('settings');
    expect(screen.getAllByRole('listitem').length).toBeGreaterThan(0);
  });

  it('updates when the reader pushes a new model', async () => {
    const fake = await open(fixtureModel('fresh'));
    act(() => fake.push(fixtureModel('exhausted')));
    expect(screen.getByText('Hạn mức Claude trong vòng đã cạn')).toBeTruthy();
    expect(screen.getByText('Có lại lúc 16:40 khi Cá nhân và Team B được đặt lại.')).toBeTruthy();
  });
});
