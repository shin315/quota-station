import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { failing, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model, Tab } from '../lib/model';

async function open(model: Model, tab: Tab = 'overview') {
  const fake = fakeApi(model, tab);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Mục' });
  return fake;
}

const heading = () => screen.getByRole('heading', { level: 2 }).textContent;
const nav = (name: string) => within(screen.getByRole('navigation', { name: 'Mục' })).getByRole('button', { name });

afterEach(cleanup);

describe('main window', () => {
  it('opens on the tab it was asked for', async () => {
    await open(fixtureModel('fresh'), 'claude');
    expect(heading()).toBe('Claude');
    expect(nav('Claude').getAttribute('aria-current')).toBe('page');
  });

  it('Tổng quan shows the pools, every account and what comes next, with nothing to switch', async () => {
    await open(fixtureModel('fresh'));
    expect(heading()).toBe('Tổng quan');
    expect(screen.getByText('Mac mini · cập nhật 40 giây trước')).toBeTruthy();
    expect(screen.getByText('Hạn mức Claude trong vòng')).toBeTruthy();
    expect(screen.getByText('Tương đương, cộng phần còn lại · Team A 0% + Team B 100% + Team C 98% + Cá nhân 29%')).toBeTruthy();

    const claude = screen.getByRole('list', { name: 'Claude Code' });
    const rows = within(claude).getAllByRole('listitem');
    expect(rows).toHaveLength(4);
    expect(rows[1].getAttribute('aria-label')).toMatch(/^Team A \(claude-swap, tài khoản 2\), đang dùng\./);
    expect(within(screen.getByRole('list', { name: 'Antigravity' })).getAllByRole('listitem')).toHaveLength(2);
    expect(screen.getByText('Đặt lại sau 3 giờ 08 phút')).toBeTruthy();

    expect(screen.getByText('Sắp tới')).toBeTruthy();
    expect(screen.getByText('Team A rời vòng')).toBeTruthy();
    expect(screen.getByText('ngày mai 03:20')).toBeTruthy();

    // Only the title bar's refresh and the sidebar act; no account can be switched here.
    expect(screen.queryByRole('button', { name: /Chuyển/ })).toBeNull();
    const main = screen.getByRole('main');
    expect(within(main).getAllByRole('button').map((b) => b.textContent)).toEqual(['Làm mới']);
  });

  it('later tabs sit in the sidebar but cannot be opened', async () => {
    const fake = await open(fixtureModel('fresh'));
    for (const name of ['Codex', 'Luật xoay vòng', 'Lịch sử', 'Cảnh báo', 'Firstmate']) {
      const b = nav(name) as HTMLButtonElement;
      expect(b.disabled).toBe(true);
      expect(b.title).toBe('Có ở bản sau');
      fireEvent.click(b);
    }
    expect(heading()).toBe('Tổng quan');
    expect(fake.api.openWindow).not.toHaveBeenCalled();
  });

  it('switches tabs from the sidebar and when asked from outside', async () => {
    const fake = await open(fixtureModel('fresh'));
    fireEvent.click(nav('Antigravity'));
    expect(heading()).toBe('Antigravity');
    fireEvent.click(nav('Cài đặt'));
    expect(heading()).toBe('Cài đặt');
    await act(async () => {
      await fake.api.openWindow('claude');
    });
    expect(heading()).toBe('Claude');
  });

  it('the Claude tab has a freshness column from each account’s own reading', async () => {
    await open(fixtureModel('stale'), 'claude');
    expect(screen.getByText('4 tài khoản claude-swap trên máy này')).toBeTruthy();
    expect(screen.getByText('Cập nhật')).toBeTruthy();
    expect(screen.getAllByText('41 phút trước')).toHaveLength(4);
    // The column replaces the chip next to the name.
    expect(screen.queryByText('cập nhật 41 phút trước')).toBeNull();
    expect(screen.getByText('Hạn mức Claude chưa cập nhật trong 41 phút')).toBeTruthy();
  });

  it('the Antigravity tab says how fresh the other accounts are and that Cockpit runs', async () => {
    await open(fixtureModel('fresh'), 'agy');
    expect(
      screen.getByText(
        'Số của tài khoản không dùng do Cockpit làm mới không đều: Ultra 1 đọc 25 phút trước. App không cảnh báo khi Antigravity xuống thấp.',
      ),
    ).toBeTruthy();
    expect(screen.getByText('Cockpit Tools')).toBeTruthy();
    expect(screen.getByText('đang chạy')).toBeTruthy();
  });

  it('a Mac without Cockpit or without Antigravity says so on the Antigravity tab', async () => {
    await open(fixtureModel('no-cockpit'), 'agy');
    expect(screen.getByText('Chưa thấy Cockpit trên máy này')).toBeTruthy();
    expect(within(screen.getByRole('list', { name: 'Antigravity' })).getAllByRole('listitem')).toHaveLength(1);
    cleanup();

    await open(fixtureModel('no-antigravity'), 'agy');
    expect(screen.getByText('Máy này chưa có Antigravity')).toBeTruthy();
    expect(screen.queryByRole('list', { name: 'Antigravity' })).toBeNull();
    fireEvent.click(nav('Tổng quan'));
    expect(screen.queryByRole('list', { name: 'Antigravity' })).toBeNull();
    expect(screen.getByText('Không có trên máy này')).toBeTruthy();
  });

  it('"Làm mới" reads again and reports when done', async () => {
    const fake = await open(fixtureModel('fresh', { reads: 3 }), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Làm mới' }));
    });
    expect(fake.api.refresh).toHaveBeenCalledTimes(1);
    expect((screen.getByRole('button', { name: 'Làm mới' }) as HTMLButtonElement).disabled).toBe(true);
    act(() => fake.push(fixtureModel('fresh', { reads: 4 })));
    expect(screen.getByText('Đã làm mới')).toBeTruthy();
  });

  it('quota-axi missing points to the settings tab and keeps the last numbers', async () => {
    const fake = await open(fixtureModel('fresh', { errors: failing('notFound') }));
    expect(screen.getByText('Mac mini · không đọc được số')).toBeTruthy();
    expect(screen.getAllByText('Số lúc 14:20')).toHaveLength(2);
    fireEvent.click(screen.getByRole('button', { name: 'Chọn đường dẫn…' }));
    expect(heading()).toBe('Cài đặt');
    expect(fake.api.openWindow).not.toHaveBeenCalled();
  });

  it('the Antigravity tab says quota-axi is not running instead of calling Cockpit running', async () => {
    await open(fixtureModel('fresh', { errors: failing('notFound') }), 'agy');
    expect(screen.getByText('Không chạy được quota-axi')).toBeTruthy();
    expect(screen.queryByText('đang chạy')).toBeNull();
    expect(screen.getByText('không rõ')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Chọn đường dẫn…' }));
    expect(heading()).toBe('Cài đặt');
  });

  it('follows the reader like the popover does', async () => {
    const fake = await open(fixtureModel('fresh'));
    act(() => fake.push(fixtureModel('exhausted')));
    expect(screen.getByText('Hạn mức Claude trong vòng đã cạn')).toBeTruthy();
    expect(screen.getByText('Cạn · có lại lúc 16:40')).toBeTruthy();
  });
});
