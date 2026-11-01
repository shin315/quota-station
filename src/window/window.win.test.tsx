// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The main window and Cài đặt on Windows (`Capabilities.platform`): Windows'
// own title bar instead of the traffic lights, `data-os` for the type and
// scroll bars, no Keychain row, and the system tray in its own card where
// macOS has the menu bar. The setup file puts the system back to macOS and
// the language back to Vietnamese after each test.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { appFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model, Tab } from '../lib/model';

const onWindows = (m: Model): Model => ({ ...m, capabilities: { ...m.capabilities, platform: 'windows' } });

async function open(model: Model, tab: Tab = 'overview') {
  const fake = fakeApi(model, tab);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  const { container } = render(
    <div className="qs app-window" data-os="macos">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Các tab' });
  return { fake, root: container.firstElementChild as HTMLElement };
}

async function showGroup(name: string) {
  await act(async () => {
    fireEvent.mouseDown(screen.getByRole('tab', { name }));
  });
}

afterEach(cleanup);

describe('the main window on Windows', () => {
  it('puts data-os on the page; macOS by default', async () => {
    const mac = await open(fixtureModel('fresh'));
    expect(document.documentElement.dataset.os).toBe('macos');
    expect(mac.root.dataset.os).toBe('macos');
    cleanup();
    const win = await open(onWindows(fixtureModel('fresh')));
    expect(document.documentElement.dataset.os).toBe('windows');
    expect(win.root.dataset.os).toBe('windows');
  });

  it('draws no traffic lights and no drag strip: Windows draws its title bar', async () => {
    await open(fixtureModel('fresh'));
    const nav = screen.getByRole('navigation', { name: 'Các tab' });
    expect(nav.querySelectorAll('i.rounded-full')).toHaveLength(3);
    expect(document.querySelector('.win-o > .dragbar')).not.toBeNull();
    cleanup();
    await open(onWindows(fixtureModel('fresh')));
    expect(screen.getByRole('navigation', { name: 'Các tab' }).querySelectorAll('i.rounded-full')).toHaveLength(0);
    expect(document.querySelector('.win-o > .dragbar')).toBeNull();
  });

  it('Cài đặt › Hiển thị has a Khay hệ thống card for the tray', async () => {
    await open(onWindows(fixtureModel('fresh')), 'settings');
    await showGroup('Hiển thị');
    const tray = screen.getByRole('region', { name: 'Khay hệ thống' });
    expect(within(tray).getByRole('group', { name: 'Dịch vụ trên icon khay' })).toBeTruthy();
    expect(within(tray).getByRole('group', { name: 'Số trên icon khay' })).toBeTruthy();
    // Not in the Hiển thị card, and nothing says menu bar.
    const display = screen.getByRole('region', { name: 'Hiển thị' });
    expect(within(display).queryByRole('group', { name: /thanh menu|icon khay/ })).toBeNull();
    expect(screen.queryByText(/thanh menu/)).toBeNull();
    expect(within(display).getByRole('button', { name: 'Theo Windows' })).toBeTruthy();
  });

  it('macOS keeps the menu bar rows in Hiển thị, and no tray card', async () => {
    await open(fixtureModel('fresh'), 'settings');
    await showGroup('Hiển thị');
    expect(screen.queryByRole('region', { name: 'Khay hệ thống' })).toBeNull();
    const display = screen.getByRole('region', { name: 'Hiển thị' });
    expect(within(display).getByRole('group', { name: 'Dịch vụ trên thanh menu' })).toBeTruthy();
    expect(within(display).getByRole('group', { name: 'Số trên thanh menu' })).toBeTruthy();
  });

  it('Dịch vụ has no Keychain row on Windows', async () => {
    await open(fixtureModel('keychain'), 'settings');
    await showGroup('Dịch vụ');
    expect(screen.getByText('Quyền Keychain')).toBeTruthy();
    cleanup();
    await open(onWindows(fixtureModel('keychain')), 'settings');
    await showGroup('Dịch vụ');
    expect(screen.queryByText('Quyền Keychain')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Cấp quyền một lần' })).toBeNull();
  });

  it('Mở khi đăng nhập sends the owner to Windows’ Startup apps', async () => {
    await open(onWindows(fixtureModel('fresh', { app: appFixture({ launchAtLogin: 'needsApproval' }) })), 'settings');
    expect(screen.getByText('Windows đang tắt Quota Station trong Cài đặt Windows › Ứng dụng › Khởi động: bật lại ở đó.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Mở Cài đặt Windows: Khởi động' }).textContent).toBe('Mở Cài đặt Windows');
  });
});
