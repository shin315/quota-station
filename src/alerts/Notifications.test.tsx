// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Thông báo (M6) on the fake Rust side: the bell in every page's title row
// and its count, the panel it opens with the cards and their buttons, what
// opening it reads, the way to Cài đặt › Thông báo, and the popover's bell.
// Nothing here posts a macOS notification or switches an account.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp, PopoverApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { agyAccount, agyFixture, alertsFixture, FIXTURE_NOW, fixtureModel, overwriteAlert, poolAlert } from '../fixtures/models';
// @private:begin
import { leavingAlert } from '../fixtures/models';
// @private:end
import type { Api } from '../lib/api';
import type { AlertRecord, AlertsView, Model, Tab } from '../lib/model';
import { bellCount, PANEL_FIRST } from './Notifications';

function withAlerts(over: Partial<AlertsView> = {}): Model {
  const recent = over.recent ?? [overwriteAlert(), poolAlert()];
  return fixtureModel('fresh', {
    antigravity: agyFixture({ accounts: [agyAccount(1), agyAccount(2)] }),
    agySwitch: { busy: null, listed: true, overwritten: null },
    alerts: alertsFixture({ recent, unanswered: recent.filter((r) => !r.answer && r.alert.actions.length).length, ...over }),
  });
}

/** The main window on `tab`; `alerts` is what a macOS card asks for: the bell's panel, open. */
async function open(model: Model, tab: Tab = 'alerts', options: FakeOptions = {}) {
  const fake = fakeApi(model, tab, options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Các tab' });
  if (tab === 'alerts') await screen.findByRole('dialog', { name: 'Thông báo' });
  return fake;
}

const bell = () => within(screen.getByRole('heading', { level: 2 }).parentElement!).getByRole('button', { name: /^Thông báo/ });
const panel = () => screen.getByRole('dialog', { name: 'Thông báo' });
const card = (title: string) => within(panel()).getByRole('group', { name: 'Thông báo: ' + title });
const count = () => screen.queryByTestId('bell-count')?.textContent ?? '';
const press = async (el: Element) => {
  await act(async () => {
    fireEvent.click(el);
  });
};

/** A notification of the same kind as `poolAlert`, `n` minutes later. */
function later(n: number, over: Partial<AlertRecord> = {}): AlertRecord {
  const at = 1768547700 + n * 60;
  return poolAlert({ id: 'pool:' + n + '@' + at, alert: { ...poolAlert().alert, key: 'pool:' + n, title: 'Thông báo số ' + n, at }, ...over });
}

afterEach(cleanup);

describe('the bell', () => {
  it('sits at the right end of every page’s title row, and the sidebar has no Thông báo', async () => {
    await open(withAlerts(), 'overview');
    expect(screen.queryByRole('dialog', { name: 'Thông báo' })).toBeNull();
    const nav = screen.getByRole('navigation', { name: 'Các tab' });
    expect(within(nav).queryByRole('button', { name: /Thông báo|Cảnh báo/ })).toBeNull();
    for (const name of ['Lịch sử', 'MCP', 'Cài đặt']) {
      await press(within(nav).getByRole('button', { name }));
      const row = screen.getByRole('heading', { level: 2, name }).parentElement!;
      const buttons = within(row).getAllByRole('button');
      expect(buttons[buttons.length - 1]).toBe(bell());
    }
    // Beside the machine and how fresh the numbers are, and "Làm mới" on Tổng quan.
    await press(within(nav).getByRole('button', { name: 'Tổng quan' }));
    const row = screen.getByRole('heading', { level: 2, name: 'Tổng quan' }).parentElement!;
    expect(within(row).getAllByRole('button').map((b) => b.getAttribute('aria-label') ?? b.textContent)).toEqual(['Làm mới', 'Thông báo, 2 chưa đọc']);
    expect(within(row).getByText('Mac Studio · cập nhật 40 giây trước')).toBeTruthy();
  });

  it('counts the notifications not read yet: none at zero, 9+ past nine', async () => {
    expect([0, 1, 9, 10, 30].map(bellCount)).toEqual(['', '1', '9', '9+', '9+']);
    await open(withAlerts(), 'overview');
    expect(count()).toBe('2');
    expect(bell().getAttribute('aria-label')).toBe('Thông báo, 2 chưa đọc');
    cleanup();
    await open(withAlerts({ recent: Array.from({ length: 12 }, (_, i) => later(i)) }), 'overview');
    expect(count()).toBe('9+');
    expect(bell().getAttribute('aria-label')).toBe('Thông báo, 12 chưa đọc');
    cleanup();
    await open(withAlerts({ recent: [overwriteAlert({ read: true }), poolAlert({ read: true })] }), 'overview');
    expect(count()).toBe('');
    expect(bell().getAttribute('aria-label')).toBe('Thông báo');
  });

  it('opens its panel, and opening reads them: the count goes, the new ones keep a dot while it stays open', async () => {
    const fake = await open(withAlerts({ recent: [overwriteAlert(), poolAlert({ read: true })] }), 'overview');
    expect(count()).toBe('1');
    expect(bell().getAttribute('aria-expanded')).toBe('false');
    await press(bell());
    expect(bell().getAttribute('aria-expanded')).toBe('true');
    expect(fake.api.alertsRead).toHaveBeenCalledTimes(1);
    expect(fake.api.notifyRefresh).toHaveBeenCalled();
    expect(count()).toBe('');
    expect(fake.current().alerts.recent.every((r) => r.read)).toBe(true);
    expect(within(card('Antigravity đổi về tài khoản cũ')).getByText(', mới')).toBeTruthy();
    expect(within(card('Hạn mức Claude sắp cạn')).queryByText(', mới')).toBeNull();
    // One that comes while it is open is read at once.
    await act(async () => {
      fake.push({ ...fake.current(), alerts: alertsFixture({ ...fake.current().alerts, recent: [later(30), ...fake.current().alerts.recent], unread: 1 }) });
    });
    expect(fake.api.alertsRead).toHaveBeenCalledTimes(2);
    expect(count()).toBe('');
    expect(within(card('Thông báo số 30')).getByText(', mới')).toBeTruthy();
  });

  it('shows the recent ones a day at a time, one line each with the buttons of its macOS card, newest first', async () => {
    const yesterday = poolAlert({
      id: 'pool:dry:1768461300@1768461300',
      alert: { ...poolAlert().alert, key: 'pool:dry:1768461300', title: 'Hạn mức Claude sắp cạn hôm qua', at: 1768461300 },
      answer: { action: 'dismiss', label: 'Bỏ qua', at: 1768461400, result: null },
    });
    await open(withAlerts({ recent: [overwriteAlert(), poolAlert(), yesterday] }));
    const recent = within(panel()).getByRole('region', { name: 'Gần đây' });
    expect(Array.from(recent.children).map((c) => (c.getAttribute('role') === 'group' ? '· ' + c.getAttribute('aria-label') : c.textContent))).toEqual([
      'Hôm nay',
      '· Thông báo: Antigravity đổi về tài khoản cũ',
      '· Thông báo: Hạn mức Claude sắp cạn',
      'Hôm qua',
      '· Thông báo: Hạn mức Claude sắp cạn hôm qua',
    ]);
    const o = card('Antigravity đổi về tài khoản cũ');
    expect(within(o).getByText('Lúc 14:10 Antigravity.app đổi về Ultra 1. Bạn đã chuyển sang Ultra 2 lúc 13:50.')).toBeTruthy();
    expect(within(o).getAllByRole('button').map((b) => b.textContent)).toEqual(['Thoát app rồi chuyển lại', 'Chuyển lại', 'Bỏ qua']);
    expect(within(o).getByText('14:10 · đã gửi thông báo macOS')).toBeTruthy();
    expect(within(card('Hạn mức Claude sắp cạn')).getAllByRole('button').map((b) => b.textContent)).toEqual(['Sang lan.acme2 (đã dùng 2%)', 'Bỏ qua']);
    expect(within(card('Hạn mức Claude sắp cạn hôm qua')).getByText('14:15 · đã gửi thông báo macOS')).toBeTruthy();
    expect(within(card('Hạn mức Claude sắp cạn hôm qua')).queryByRole('button')).toBeNull();
  });

  it('a long list shows the newest first and the rest behind "Xem tất cả"', async () => {
    const recent = Array.from({ length: PANEL_FIRST + 2 }, (_, i) => later(PANEL_FIRST + 2 - i));
    await open(withAlerts({ recent }));
    expect(within(panel()).getAllByRole('group', { name: /^Thông báo: / })).toHaveLength(PANEL_FIRST);
    expect(within(panel()).getByText(PANEL_FIRST + 2 + ' gần đây')).toBeTruthy();
    await press(within(panel()).getByRole('button', { name: 'Xem tất cả (' + (PANEL_FIRST + 2) + ')' }));
    expect(within(panel()).getAllByRole('group', { name: /^Thông báo: / })).toHaveLength(PANEL_FIRST + 2);
    expect(within(panel()).queryByRole('button', { name: /^Xem tất cả/ })).toBeNull();
  });

  it('with none yet, the panel says so', async () => {
    await open(withAlerts({ recent: [] }));
    expect(within(panel()).getByText('Chưa có thông báo')).toBeTruthy();
    expect(within(panel()).getByText('Khi có, thông báo sẽ hiện ở đây với cùng các nút bấm như thông báo macOS.')).toBeTruthy();
  });

  it('Esc and a click outside close it, and the focus goes back to the bell', async () => {
    await open(withAlerts(), 'overview');
    // From the keyboard: the bell takes the focus, Enter opens it.
    bell().focus();
    await press(bell());
    // The panel takes the focus, not its first button: Enter answers no card.
    expect(document.activeElement).toBe(panel());
    await act(async () => {
      fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    });
    expect(screen.queryByRole('dialog', { name: 'Thông báo' })).toBeNull();
    // The focus goes back once the panel is gone.
    await waitFor(() => expect(document.activeElement).toBe(bell()));
    await press(bell());
    expect(panel()).toBeTruthy();
    await act(async () => {
      // The panel listens for a click outside from the tick after it opens; a press there closes it once let go.
      await new Promise((r) => setTimeout(r, 0));
      const outside = screen.getByRole('heading', { level: 2 });
      fireEvent.pointerDown(outside, { button: 0 });
      fireEvent.pointerUp(outside, { button: 0 });
      fireEvent.click(outside);
    });
    expect(screen.queryByRole('dialog', { name: 'Thông báo' })).toBeNull();
  });

  it('"Cài đặt thông báo" opens Cài đặt at its Thông báo tab and closes the panel', async () => {
    const fake = await open(withAlerts());
    await press(within(panel()).getByRole('button', { name: 'Cài đặt thông báo' }));
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Cài đặt');
    expect(screen.queryByRole('dialog', { name: 'Thông báo' })).toBeNull();
    expect(screen.getByRole('tab', { name: 'Thông báo' }).getAttribute('aria-selected')).toBe('true');
    expect(fake.api.setSettingsGroup).toHaveBeenLastCalledWith('notify');
    // The kinds, "Gửi thử" and their switches are there.
    const group = screen.getByRole('region', { name: 'Thông báo' });
    let kinds = 3;
    // @private:begin
    kinds = 5;
    // @private:end
    expect(within(group).getAllByRole('button', { name: /^Gửi thử: / })).toHaveLength(kinds);
    // Asked again from another page after another tab was picked, it opens Thông báo again.
    fireEvent.mouseDown(screen.getByRole('tab', { name: 'Chung' }));
    await press(within(screen.getByRole('navigation', { name: 'Các tab' })).getByRole('button', { name: 'Tổng quan' }));
    await press(bell());
    await press(within(panel()).getByRole('button', { name: 'Cài đặt thông báo' }));
    expect(screen.getByRole('tab', { name: 'Thông báo' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByRole('region', { name: 'Thông báo' })).toBeTruthy();
  });
  it('a press on the macOS card opens the panel over the page in view', async () => {
    const fake = await open(withAlerts(), 'history');
    expect(screen.queryByRole('dialog', { name: 'Thông báo' })).toBeNull();
    await act(async () => {
      fake.cardOpened();
    });
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Lịch sử');
    expect(panel()).toBeTruthy();
    // Following a link from it, to another page, closes it.
    await press(within(screen.getByRole('navigation', { name: 'Các tab' })).getByRole('button', { name: 'Claude' }));
    expect(screen.queryByRole('dialog', { name: 'Thông báo' })).toBeNull();
  });
});

describe('a notification’s buttons in the panel', () => {
  it('a press counts once: the line says what was chosen and how it went', async () => {
    const fake = await open(withAlerts());
    await press(within(card('Hạn mức Claude sắp cạn')).getByRole('button', { name: 'Bỏ qua' }));
    expect(fake.api.alertAction).toHaveBeenCalledWith('pool:dry:1768547700@1768547700', 'dismiss');
    const c = card('Hạn mức Claude sắp cạn');
    expect(within(c).queryByRole('button')).toBeNull();
    expect(within(c).getByText('Bạn đã chọn “Bỏ qua”.')).toBeTruthy();
    expect(within(c).getByText(/^Không lặp lại cho cùng sự kiện\./)).toBeTruthy();
    expect(fake.current().alerts.unanswered).toBe(1);
  });

  // @private:begin
  it('"Ổn" on a card about an account leaving the rotation is not said again for that leave', async () => {
    const fake = await open(withAlerts({ recent: [leavingAlert()] }));
    await press(within(card('Team A rời vòng lúc 14:45')).getByRole('button', { name: 'Ổn' }));
    expect(fake.api.alertAction).toHaveBeenCalledWith('leaving:2:2026-01-16T14:45@1768547700', 'dismiss');
    const c = card('Team A rời vòng lúc 14:45');
    expect(within(c).getByText('Bạn đã chọn “Ổn”.')).toBeTruthy();
    expect(within(c).getByText('Không nhắc lại cho lần rời vòng này.')).toBeTruthy();
  });
  // @private:end

  it('"Sang …" opens the switch sheet in this window, closes the panel and switches nothing yet', async () => {
    const fake = await open(withAlerts());
    await press(within(card('Hạn mức Claude sắp cạn')).getByRole('button', { name: 'Sang lan.acme2 (đã dùng 2%)' }));
    expect(screen.getByRole('dialog', { name: /lan\.acme2/ })).toBeTruthy();
    expect(screen.queryByRole('dialog', { name: 'Thông báo' })).toBeNull();
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });

  it('"Thoát app rồi chuyển lại" switches back and says how it went on the line', async () => {
    const fake = await open(withAlerts(), 'alerts', { agySwitchDelayMs: 30 });
    await press(within(card('Antigravity đổi về tài khoản cũ')).getByRole('button', { name: 'Thoát app rồi chuyển lại' }));
    expect(fake.api.alertAction).toHaveBeenCalledWith('overwrite:antigravity_fake2:1768547400@1768547400', 'agy:quitFirst:2');
    await screen.findByText('Đã chuyển lại sang Ultra 2');
    const o = card('Antigravity đổi về tài khoản cũ');
    expect(within(o).getByText('Bạn đã chọn “Thoát app rồi chuyển lại”.')).toBeTruthy();
    // The result stays written on the line, not as a passing toast.
    expect(within(o).getByText('Antigravity.app đã mở lại với tài khoản này.')).toBeTruthy();
  });

  it('while the switch back runs, the line says so', async () => {
    const running = overwriteAlert({ answer: { action: 'agy:keepRunning:2', label: 'Chuyển lại', at: 1768547500, result: null } });
    await open(withAlerts({ recent: [running] }));
    const o = card('Antigravity đổi về tài khoản cũ');
    expect(within(o).getByText('Đang chuyển tài khoản Antigravity…')).toBeTruthy();
    expect(within(o).getByText('Chờ tối đa 60 giây.')).toBeTruthy();
  });

  it('one answered on macOS meanwhile says so in the panel instead of pressing twice', async () => {
    const fake = await open(withAlerts());
    vi.mocked(fake.api.alertAction).mockRejectedValueOnce('answered');
    await press(within(card('Hạn mức Claude sắp cạn')).getByRole('button', { name: 'Bỏ qua' }));
    expect(within(panel()).getByText('Thông báo này đã có câu trả lời, có thể từ thông báo macOS.')).toBeTruthy();
  });

  it('"Đăng nhập lại" on an Antigravity one opens the Antigravity tab and closes the panel', async () => {
    const relogin = poolAlert({
      id: 'agy:login:antigravity_fake2@1768547700',
      alert: {
        ...poolAlert().alert,
        key: 'agy:login:antigravity_fake2',
        kind: 'login',
        title: 'Ultra 2 cần đăng nhập lại',
        body: 'Google không nhận đăng nhập của tài khoản này nữa. Đăng nhập lại trong tab Antigravity.',
        actions: [
          { id: 'agy:login', label: 'Đăng nhập lại' },
          { id: 'dismiss', label: 'Đã biết' },
        ],
      },
    });
    await open(withAlerts({ recent: [relogin] }));
    await press(within(card('Ultra 2 cần đăng nhập lại')).getByRole('button', { name: 'Đăng nhập lại' }));
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Antigravity');
    expect(screen.queryByRole('dialog', { name: 'Thông báo' })).toBeNull();
  });

  it('"Gửi thử" in Cài đặt sends a test that lands under the bell, its buttons changing nothing', async () => {
    const fake = await open(withAlerts({ recent: [] }), 'settings');
    fireEvent.mouseDown(screen.getByRole('tab', { name: 'Thông báo' }));
    expect(count()).toBe('');
    await press(screen.getByRole('button', { name: 'Gửi thử: Tài khoản cần đăng nhập lại' }));
    expect(fake.api.testAlert).toHaveBeenCalledWith('login');
    expect(count()).toBe('1');
    await press(bell());
    const t = within(panel()).getByRole('group', { name: /cần đăng nhập lại$/ });
    expect(within(t).getByText(/thông báo thử · đã gửi thông báo macOS$/)).toBeTruthy();
    await press(within(t).getByRole('button', { name: 'Nhắc ngày mai' }));
    expect(within(t).getByText('Bạn đã bấm “Nhắc ngày mai”. Thông báo thử không làm gì cả.')).toBeTruthy();
    expect(screen.queryByRole('dialog', { name: /lan\.acme2/ })).toBeNull();
  });

  it('with notifications off in macOS, the cards stay under the bell and System Settings is one press away', async () => {
    const fake = await open(withAlerts({ permission: 'denied' }));
    expect(within(panel()).getByText('Thông báo của Quota Station đang tắt')).toBeTruthy();
    expect(within(panel()).getAllByRole('group', { name: /^Thông báo: / })).toHaveLength(2);
    await press(within(panel()).getByRole('button', { name: 'Mở Cài đặt hệ thống' }));
    expect(fake.api.openNotificationSettings).toHaveBeenCalledTimes(1);
  });
});

describe('the popover', () => {
  async function openPopover(model: Model) {
    const fake = fakeApi(model);
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-popover">
        <PopoverApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('dialog', { name: 'Quota Station' });
    return fake;
  }

  it('has a bell with the count in its header, opening the main window with the panel open', async () => {
    const fake = await openPopover(withAlerts());
    const b = screen.getByRole('button', { name: 'Thông báo, 2 chưa đọc' });
    expect(b.textContent).toBe('2');
    await press(b);
    expect(fake.api.openWindow).toHaveBeenCalledWith('alerts');
    cleanup();
    await openPopover(withAlerts({ recent: [] }));
    expect(screen.getByRole('button', { name: 'Thông báo' }).textContent).toBe('');
  });

  it('"Sang …" on a macOS card opens the switch sheet in the popover', async () => {
    const fake = await openPopover(withAlerts());
    await act(async () => {
      fake.cardOpensSwitch(3);
    });
    expect(screen.getByRole('dialog', { name: /lan\.acme2/ })).toBeTruthy();
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });
});
