// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Cảnh báo tab (M6) on the fake Rust side: the cards and their buttons,
// "Gửi thử" and the switches, and the count on the sidebar. Nothing here
// posts a macOS notification or switches an account.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp, PopoverApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { agyAccount, agyFixture, alertsFixture, FIXTURE_NOW, fixtureModel, leavingAlert, overwriteAlert } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { AlertsView, Model } from '../lib/model';

function withAlerts(over: Partial<AlertsView> = {}): Model {
  const recent = over.recent ?? [overwriteAlert(), leavingAlert()];
  return fixtureModel('fresh', {
    antigravity: agyFixture({ accounts: [agyAccount(1), agyAccount(2)] }),
    agySwitch: { busy: null, listed: true, overwritten: null },
    alerts: alertsFixture({ recent, unanswered: recent.filter((r) => !r.answer && r.alert.actions.length).length, ...over }),
  });
}

async function open(model: Model, options: FakeOptions = {}) {
  const fake = fakeApi(model, 'alerts', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { name: 'Cảnh báo' });
  return fake;
}

const card = (title: string) => screen.getByRole('group', { name: 'Cảnh báo: ' + title });
/** The title row's switch between the two views. */
const show = async (view: 'Gần đây' | 'Loại cảnh báo') => {
  await act(async () => {
    fireEvent.click(within(screen.getByRole('group', { name: 'Xem cảnh báo' })).getByRole('button', { name: view }));
  });
};
const nav = (name: RegExp) => within(screen.getByRole('navigation', { name: 'Các tab' })).getByRole('button', { name });

afterEach(cleanup);

describe('the Cảnh báo tab', () => {
  it('shows the recent cards with the buttons of their macOS card, newest first', async () => {
    await open(withAlerts());
    const cards = screen.getAllByRole('group', { name: /^Cảnh báo: / });
    expect(cards.map((c) => c.getAttribute('aria-label'))).toEqual([
      'Cảnh báo: Antigravity đổi về tài khoản cũ',
      'Cảnh báo: Team A rời vòng lúc 14:45',
    ]);
    const o = card('Antigravity đổi về tài khoản cũ');
    expect(within(o).getByText('Lúc 14:10 Antigravity.app đổi về Ultra 1. Bạn đã chuyển sang Ultra 2 lúc 13:50.')).toBeTruthy();
    expect(within(o).getAllByRole('button').map((b) => b.textContent)).toEqual(['Thoát app rồi chuyển lại', 'Chuyển lại', 'Thôi']);
    expect(within(o).getByText('14:10 · đã gửi thông báo macOS')).toBeTruthy();
    expect(within(card('Team A rời vòng lúc 14:45')).getAllByRole('button').map((b) => b.textContent)).toEqual(['Ổn', 'Chuyển ngay']);
  });

  it('two views from the title row: Gần đây a day at a time, a line per alert with its buttons on the right; Loại cảnh báo apart', async () => {
    const yesterday = leavingAlert({
      id: 'leaving:2:2026-01-15T14:45@1768461300',
      alert: { ...leavingAlert().alert, key: 'leaving:2:2026-01-15T14:45', title: 'Team A rời vòng lúc 14:45 hôm qua', at: 1768461300 },
      answer: { action: 'dismiss', label: 'Ổn', at: 1768461400, result: null },
    });
    await open(withAlerts({ recent: [overwriteAlert(), leavingAlert(), yesterday] }));
    const views = within(screen.getByRole('group', { name: 'Xem cảnh báo' }));
    // In the title row, Gần đây first.
    expect(screen.getByRole('group', { name: 'Xem cảnh báo' }).closest('[data-tauri-drag-region]')).toBeTruthy();
    expect(views.getByRole('button', { name: 'Gần đây' }).getAttribute('aria-pressed')).toBe('true');
    const recent = screen.getByRole('region', { name: 'Gần đây' });
    // One card: the days as heads, the alerts under each.
    expect(Array.from(recent.children).map((c) => (c.getAttribute('role') === 'group' ? '· ' + c.getAttribute('aria-label') : c.textContent))).toEqual([
      'Hôm nay',
      '· Cảnh báo: Antigravity đổi về tài khoản cũ',
      '· Cảnh báo: Team A rời vòng lúc 14:45',
      'Hôm qua',
      '· Cảnh báo: Team A rời vòng lúc 14:45 hôm qua',
    ]);
    // A line: its mark, its words and when, and its buttons on the right; the day is the head, the line keeps the time.
    const line = card('Team A rời vòng lúc 14:45');
    expect(line.children).toHaveLength(3);
    expect(within(line.children[2] as HTMLElement).getAllByRole('button').map((b) => b.textContent)).toEqual(['Ổn', 'Chuyển ngay']);
    expect(within(card('Team A rời vòng lúc 14:45 hôm qua')).getByText('14:15 · đã gửi thông báo macOS')).toBeTruthy();
    // The kinds are the other view.
    expect(screen.queryByRole('switch')).toBeNull();
    await show('Loại cảnh báo');
    expect(screen.queryByRole('region', { name: 'Gần đây' })).toBeNull();
    expect(screen.getAllByRole('button', { name: /^Gửi thử: / })).toHaveLength(5);
    expect(views.getByRole('button', { name: 'Loại cảnh báo' }).getAttribute('aria-pressed')).toBe('true');
  });

  it('with no alert yet, Gần đây says so and leads to the kinds', async () => {
    await open(withAlerts({ recent: [] }));
    expect(screen.getByText('Chưa có cảnh báo')).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Xem các loại cảnh báo' }));
    });
    expect(screen.getAllByRole('switch')).toHaveLength(5);
  });

  it('the sidebar counts the cards nobody answered, and a press counts once', async () => {
    const fake = await open(withAlerts());
    expect(nav(/^Cảnh báo/).textContent).toBe('Cảnh báo2, 2 cảnh báo mới');
    await act(async () => {
      fireEvent.click(within(card('Team A rời vòng lúc 14:45')).getByRole('button', { name: 'Ổn' }));
    });
    expect(fake.api.alertAction).toHaveBeenCalledWith('leaving:2:2026-01-16T14:45@1768547700', 'dismiss');
    const c = card('Team A rời vòng lúc 14:45');
    expect(within(c).queryByRole('button')).toBeNull();
    expect(within(c).getByText('Bạn đã chọn “Ổn”.')).toBeTruthy();
    expect(within(c).getByText('Không nhắc lại cho lần rời vòng này.')).toBeTruthy();
    expect(nav(/^Cảnh báo/).textContent).toBe('Cảnh báo1, 1 cảnh báo mới');
  });

  it('"Chuyển ngay" opens the switch sheet in this window and switches nothing yet', async () => {
    const fake = await open(withAlerts());
    await act(async () => {
      fireEvent.click(within(card('Team A rời vòng lúc 14:45')).getByRole('button', { name: 'Chuyển ngay' }));
    });
    expect(screen.getByRole('dialog', { name: /lan\.acme2/ })).toBeTruthy();
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });

  it('"Thoát app rồi chuyển lại" switches back and says how it went under the card', async () => {
    const fake = await open(withAlerts(), { agySwitchDelayMs: 30 });
    await act(async () => {
      fireEvent.click(within(card('Antigravity đổi về tài khoản cũ')).getByRole('button', { name: 'Thoát app rồi chuyển lại' }));
    });
    expect(fake.api.alertAction).toHaveBeenCalledWith('overwrite:antigravity_fake2:1768547400@1768547400', 'agy:quitFirst:2');
    await screen.findByText('Đã chuyển lại sang Ultra 2');
    const o = card('Antigravity đổi về tài khoản cũ');
    expect(within(o).getByText('Bạn đã chọn “Thoát app rồi chuyển lại”.')).toBeTruthy();
    // The result stays written on the card, not as a passing toast.
    expect(within(o).getByText('Antigravity.app đã mở lại với tài khoản này.')).toBeTruthy();
  });

  it('while the switch back runs, the card says so', async () => {
    const running = overwriteAlert({ answer: { action: 'agy:keepRunning:2', label: 'Chuyển lại', at: 1768547500, result: null } });
    await open(withAlerts({ recent: [running] }));
    const o = card('Antigravity đổi về tài khoản cũ');
    expect(within(o).getByText('Đang chuyển tài khoản Antigravity…')).toBeTruthy();
    expect(within(o).getByText('Chờ tối đa 60 giây.')).toBeTruthy();
  });

  it('a card answered on macOS meanwhile says so instead of pressing twice', async () => {
    const fake = await open(withAlerts());
    vi.mocked(fake.api.alertAction).mockRejectedValueOnce('answered');
    await act(async () => {
      fireEvent.click(within(card('Team A rời vòng lúc 14:45')).getByRole('button', { name: 'Ổn' }));
    });
    expect(screen.getByText('Cảnh báo này đã có câu trả lời, có thể từ thẻ macOS.')).toBeTruthy();
  });

  it('"Gửi thử" sends a test card whose buttons change nothing', async () => {
    const fake = await open(withAlerts({ recent: [] }));
    expect(screen.getByText('Chưa có cảnh báo')).toBeTruthy();
    await show('Loại cảnh báo');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Gửi thử: Tài khoản sắp rời vòng' }));
    });
    expect(fake.api.testAlert).toHaveBeenCalledWith('leaving');
    const t = screen.getByRole('group', { name: /^Cảnh báo: Team A rời vòng/ });
    expect(within(t).getByText(/thông báo thử · đã gửi thông báo macOS$/)).toBeTruthy();
    expect(nav(/^Cảnh báo/).textContent).toBe('Cảnh báo');
    await act(async () => {
      fireEvent.click(within(t).getByRole('button', { name: 'Chuyển ngay' }));
    });
    expect(within(t).getByText('Bạn đã bấm “Chuyển ngay”. Thông báo thử không làm gì cả.')).toBeTruthy();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('a test card that does not go out says so, not that settings were not saved', async () => {
    const fake = await open(withAlerts());
    vi.mocked(fake.api.testAlert).mockRejectedValueOnce('io');
    await show('Loại cảnh báo');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Gửi thử: Tài khoản sắp rời vòng' }));
    });
    expect(screen.getByText('Chưa gửi được thông báo thử')).toBeTruthy();
    expect(screen.queryByText(/Chưa lưu được/)).toBeNull();
  });

  it('"Đăng nhập lại" on an Antigravity card opens the Antigravity tab', async () => {
    const relogin = leavingAlert({
      id: 'agy:login:antigravity_fake2@1768547700',
      alert: {
        ...leavingAlert().alert,
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
    await act(async () => {
      fireEvent.click(within(card('Ultra 2 cần đăng nhập lại')).getByRole('button', { name: 'Đăng nhập lại' }));
    });
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Antigravity');
  });

  it('each kind has its switch, and Firstmate starts off', async () => {
    const fake = await open(withAlerts());
    await show('Loại cảnh báo');
    const states = [
      'Hạn mức Claude trong vòng sắp cạn / có lại',
      'Tài khoản sắp rời vòng',
      'Tài khoản cần đăng nhập lại',
      'Antigravity đổi về tài khoản cũ',
      'Firstmate đang chờ hạn mức Claude',
    ].map((name) => screen.getByRole('switch', { name }).getAttribute('aria-checked'));
    expect(states).toEqual(['true', 'true', 'true', 'true', 'false']);
    expect(screen.getAllByRole('switch', { name: /./ }).filter((s) => s.closest('.arow'))).toHaveLength(5);
    const firstmate = screen.getByRole('switch', { name: 'Firstmate đang chờ hạn mức Claude' });
    await act(async () => {
      fireEvent.click(firstmate);
    });
    expect(fake.api.setAlertSwitch).toHaveBeenCalledWith('firstmate', true);
    expect(firstmate.getAttribute('aria-checked')).toBe('true');
  });

  it('with notifications off in macOS, the cards stay here and System Settings is one press away', async () => {
    const fake = await open(withAlerts({ permission: 'denied' }));
    expect(screen.getByText('Thông báo của Quota Station đang tắt')).toBeTruthy();
    expect(screen.getAllByRole('group', { name: /^Cảnh báo: / })).toHaveLength(2);
    await show('Loại cảnh báo');
    expect(screen.getByText('Thông báo của Quota Station đang tắt')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Gửi thử: Tài khoản sắp rời vòng' }) as HTMLButtonElement).disabled).toBe(true);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở Cài đặt hệ thống' }));
    });
    expect(fake.api.openNotificationSettings).toHaveBeenCalledTimes(1);
    expect(fake.api.notifyRefresh).toHaveBeenCalled();
  });
});

describe('a button on a macOS card', () => {
  it('"Sang …" opens the switch sheet in the popover', async () => {
    const fake = fakeApi(withAlerts());
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-popover">
        <PopoverApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('dialog', { name: 'Quota Station' });
    await act(async () => {
      fake.cardOpensSwitch(3);
    });
    expect(screen.getByRole('dialog', { name: /lan\.acme2/ })).toBeTruthy();
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });
});
