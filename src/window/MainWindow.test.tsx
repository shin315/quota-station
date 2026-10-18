import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { failing, FIXTURE_NOW, fixtureModel, heldSwitcher, switcherFixture } from '../fixtures/models';
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

  it('Tổng quan shows the pools, every account, why Claude is on its account and what comes next', async () => {
    await open(fixtureModel('fresh'));
    expect(heading()).toBe('Tổng quan');
    expect(screen.getByText('Mac mini · cập nhật 40 giây trước')).toBeTruthy();
    expect(screen.getByText('Hạn mức Claude trong vòng')).toBeTruthy();
    expect(screen.getByText('Tương đương, cộng phần còn lại · Team A 0% + Team B 100% + Team C 98% + Cá nhân 29%')).toBeTruthy();

    const claude = screen.getByRole('list', { name: 'Claude Code' });
    const rows = within(claude).getAllByRole('listitem');
    expect(rows).toHaveLength(4);
    expect(rows[1].getAttribute('aria-label')).toMatch(/^Team A \(claude-swap, tài khoản 2\), đang dùng\./);
    expect(within(rows[1]).queryByRole('button')).toBeNull();
    expect(within(screen.getByRole('list', { name: 'Antigravity' })).getAllByRole('listitem')).toHaveLength(2);
    expect(screen.getByText('Đặt lại sau 3 giờ 08 phút')).toBeTruthy();

    expect(screen.getByText('Vì sao đang ở Team A')).toBeTruthy();
    expect(screen.getByText('Vẫn dùng Team A: chưa tới ngưỡng tự chuyển.')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude' }).getAttribute('aria-checked')).toBe('true');

    expect(screen.getByText('Sắp tới')).toBeTruthy();
    expect(screen.getByText('Team A rời vòng')).toBeTruthy();
    expect(screen.getByText('ngày mai 03:20')).toBeTruthy();

    // Every Claude account but the active one switches; Antigravity comes later.
    for (const name of ['Cá nhân', 'Team B', 'Team C']) {
      expect((screen.getByRole('button', { name: 'Chuyển sang ' + name }) as HTMLButtonElement).disabled).toBe(false);
    }
    const ultra = screen.getByRole('button', { name: 'Chuyển sang Ultra 1' }) as HTMLButtonElement;
    expect(ultra.disabled).toBe(true);
    expect(ultra.title).toBe('Chuyển Antigravity có ở bản sau');
    expect(screen.queryByRole('button', { name: 'Chuyển sang Team A' })).toBeNull();
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

describe('switching Claude and pausing from the main window', () => {
  const kv = () => {
    const out: Record<string, string> = {};
    const cells = Array.from(document.querySelectorAll('.kvt > span'));
    for (let i = 0; i < cells.length; i += 2) out[cells[i].textContent!] = cells[i + 1].textContent!;
    return out;
  };

  it('"Chuyển" on the Claude tab opens the confirmation and switches with the hold chosen', async () => {
    const fake = await open(fixtureModel('fresh'), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang Team C' }));
    });
    const dialog = screen.getByRole('dialog', { name: 'Chuyển Claude sang Team C?' });
    expect(within(dialog).getAllByRole('listitem')[1].textContent).toMatch(/^5 phiên Claude đang chạy/);
    fireEvent.click(within(dialog).getByRole('radio', { name: /30 phút/ }));
    await act(async () => {
      fireEvent.click(within(dialog).getByRole('button', { name: 'Chuyển sang Team C' }));
    });
    expect(fake.api.switchClaude).toHaveBeenCalledWith(4, '2026-01-16T07:50:40.000Z');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText('Đã chuyển sang Team C. Giữ tài khoản này đến 14:50.')).toBeTruthy();
  });

  it('a refresh while paused keeps the paused banner and its buttons', async () => {
    const fake = await open(fixtureModel('fresh', { reads: 3, switcher: heldSwitcher() }), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Làm mới' }));
    });
    act(() => fake.push(fixtureModel('fresh', { reads: 4, switcher: heldSwitcher() })));
    expect(screen.getByText('Đã làm mới')).toBeTruthy();
    expect(screen.getByText('Đang giữ Team A đến 15:20')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Thêm 1 giờ' })).toBeTruthy();
  });

  it('a feedback line keeps its buttons on the Cài đặt tab', async () => {
    await open(fixtureModel('fresh'), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang Team C' }));
    });
    await act(async () => {
      fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Chuyển sang Team C' }));
    });
    fireEvent.click(nav('Cài đặt'));
    expect(screen.getByText('Đã chuyển sang Team C. Giữ tài khoản này đến 15:20.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Cho phép tự chuyển lại' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    expect(screen.queryByText(/Đã chuyển sang Team C/)).toBeNull();
  });

  it('Escape closes the dialog and Thôi does too, without switching', async () => {
    const fake = await open(fixtureModel('fresh'), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Thôi' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });

  it('the Tự chuyển card shows the switcher’s last decision and log, and quick pause opens the sheet on its choice', async () => {
    const fake = await open(fixtureModel('fresh'), 'claude');
    expect(screen.getByText('Quyết định gần nhất của switcher (14:19): Vẫn dùng Team A: chưa tới ngưỡng tự chuyển.')).toBeTruthy();
    expect(screen.getByText('stay')).toBeTruthy();
    expect(screen.getByText('2026-01-16 09:12:05 switch to slot 2 (ends for good at 17:45, use it first)')).toBeTruthy();
    const quick = within(screen.getByRole('group', { name: 'Tạm dừng nhanh' }))
      .getAllByRole('button')
      .map((b) => b.textContent);
    expect(quick).toEqual(['1 giờ', 'Đến 18:00', 'Đến khi tôi bật lại']);
    fireEvent.click(screen.getByRole('button', { name: 'Đến 18:00' }));
    const sheet = screen.getByRole('dialog', { name: 'Tạm dừng tự chuyển Claude?' });
    expect(within(sheet).getByRole('radio', { name: /Đến 18:00/ }).getAttribute('aria-checked')).toBe('true');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Tạm dừng' }));
    });
    expect(fake.api.pauseSwitching).toHaveBeenCalledWith('2026-01-16T11:00:00.000Z');
    expect(kv()['Tạm dừng tự chuyển']).toBe('đến 18:00');
  });

  it('the Switcher card says whether it runs, when it last wrote, and whether it reads the pause flag', async () => {
    await open(fixtureModel('fresh', { switcher: heldSwitcher() }), 'claude');
    expect(kv()).toEqual({
      'LaunchAgent cswap-prefer-team': 'chạy mỗi 3 phút',
      'Nhật ký gần nhất': '09:12',
      'com.cswap.auto': 'tắt',
      'Tạm dừng tự chuyển': 'đến 15:20',
      'Switcher đọc cờ tạm dừng': 'có',
    });
    // While paused, the decision is the switcher's own paused line.
    expect(screen.getByText(/^Quyết định gần nhất của switcher \(\d\d:\d\d\): paused until/)).toBeTruthy();
  });

  it('a switcher that is not loaded gets a banner and a guide to load it again', async () => {
    await open(fixtureModel('fresh', { switcher: switcherFixture({ service: false }) }), 'claude');
    expect(screen.getByText('Switcher không chạy')).toBeTruthy();
    expect(kv()['LaunchAgent cswap-prefer-team']).toBe('chưa nạp');
    fireEvent.click(screen.getByRole('button', { name: 'Hướng dẫn nạp lại' }));
    const guide = screen.getByRole('dialog', { name: 'Nạp lại switcher' });
    expect(within(guide).getByText('launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.cswap.prefer-team.plist')).toBeTruthy();
    fireEvent.click(within(guide).getByRole('button', { name: 'Đóng' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('Antigravity rows keep their switch for a later milestone', async () => {
    await open(fixtureModel('fresh'), 'agy');
    for (const b of screen.getAllByRole('button', { name: /^Chuyển sang / })) {
      expect((b as HTMLButtonElement).disabled).toBe(true);
    }
  });
});
