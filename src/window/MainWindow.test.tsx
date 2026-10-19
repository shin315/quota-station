import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, fakeEmail, hideNames } from '../fixtures/fakeApi';
import { agyAccount, agyFixture, failing, FIXTURE_NOW, fixtureModel, heldSwitcher, switcherFixture } from '../fixtures/models';
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

  it('Tổng quan shows the room left, every account by its windows, automatic switching and what comes next', async () => {
    await open(fixtureModel('fresh'));
    expect(heading()).toBe('Tổng quan');
    expect(screen.getByText('Mac mini · cập nhật 40 giây trước')).toBeTruthy();
    expect(screen.getByText('còn dùng được trước ngưỡng 90%')).toBeTruthy();
    expect(screen.getByText('lan.acme1 87%')).toBeTruthy();

    const claude = screen.getByRole('list', { name: 'Claude Code' });
    const rows = within(claude).getAllByRole('listitem');
    expect(rows).toHaveLength(4);
    expect(rows[1].getAttribute('aria-label')).toMatch(/^lan\.acme1, Acme Studio, đang dùng\./);
    expect(within(rows[1]).queryByRole('button')).toBeNull();
    // Fixed columns: 5 giờ · Tuần · Fable tuần, whatever window is tightest.
    const header = claude.querySelector('.wt-r.h')!;
    expect(Array.from(header.children).map((c) => c.textContent)).toEqual(['đã dùng', '5 giờ', 'Tuần', 'Fable tuần', '']);
    expect(Array.from(rows[0].querySelectorAll('.wc b')).map((b) => b.textContent)).toEqual(['0%', '0%', '4%']);
    expect(within(screen.getByRole('list', { name: 'Antigravity' })).getAllByRole('listitem')).toHaveLength(2);

    expect(screen.getByText('14:19 · Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.')).toBeTruthy();
    expect(screen.queryByText(/^Vì sao đang ở/)).toBeNull();
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude' }).getAttribute('aria-checked')).toBe('true');

    expect(screen.getByText('Sắp tới')).toBeTruthy();
    expect(screen.getByText('lan.acme1 rời vòng')).toBeTruthy();

    // Every Claude account but the active one switches; Antigravity only once quota-axi lists the app's accounts.
    for (const name of ['lanpham.dev', 'lan.acme2', 'lan.acme3']) {
      expect((screen.getByRole('button', { name: 'Chuyển sang ' + name }) as HTMLButtonElement).disabled).toBe(false);
    }
    const ultra = screen.getByRole('button', { name: 'Chuyển sang lanpham.ultra' }) as HTMLButtonElement;
    expect(ultra.disabled).toBe(true);
    expect(ultra.title).toBe('Chưa chuyển được: quota-axi chưa liệt kê tài khoản của Quota Station');
    expect(screen.queryByRole('button', { name: 'Chuyển sang lan.acme1' })).toBeNull();
  });

  it('"Chuyển" is a button to press; "Đang dùng" is a status that cannot be pressed', async () => {
    await open(fixtureModel('fresh'), 'claude');
    const rows = within(screen.getByRole('list', { name: 'Claude Code' })).getAllByRole('listitem');
    const inUse = within(rows[1]).getByText('Đang dùng');
    expect(inUse.closest('button')).toBeNull();
    expect(inUse.className).toBe('inuse');
    expect(inUse.querySelector('.dot.ok')).toBeTruthy();
    const press = within(rows[2]).getByRole('button', { name: 'Chuyển sang lan.acme2' });
    expect(press.className).toBe('btn pri');
    expect(press.textContent).toBe('Chuyển');
  });

  it('with names hidden, no email or organization reaches the window', async () => {
    await open(hideNames(fixtureModel('fresh')), 'claude');
    const rows = within(screen.getByRole('list', { name: 'Claude Code' })).getAllByRole('listitem');
    expect(rows.map((r) => r.querySelector('.an b')!.textContent)).toEqual(['Cá nhân', 'Team A', 'Team B', 'Team C']);
    expect(document.body.textContent).not.toContain('@');
    expect(document.body.textContent).not.toContain('Acme');
    expect(document.body.innerHTML).not.toContain('example.com');
  });

  it('Codex has its own tab, which looks at Codex when it opens, and a card on Tổng quan', async () => {
    const fake = await open(fixtureModel('fresh'));
    expect(screen.getByText('Qua relay')).toBeTruthy();
    expect(screen.getByText('trả lời 180 ms · giữ đăng nhập ChatGPT')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Về ChatGPT' })).toBeTruthy();
    fireEvent.click(nav('Codex'));
    expect(heading()).toBe('Codex');
    expect(fake.api.codexCheck).toHaveBeenCalled();
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

  it('the Claude tab marks old numbers next to the name, not in a column', async () => {
    await open(fixtureModel('stale'), 'claude');
    expect(screen.getByText('4 tài khoản claude-swap')).toBeTruthy();
    expect(screen.queryByText('Cập nhật')).toBeNull();
    expect(screen.getAllByText('số 41 phút trước').length).toBeGreaterThan(0);
    expect(screen.getByText('Hạn mức Claude chưa cập nhật trong 41 phút')).toBeTruthy();
  });

  it('the Antigravity tab merges the app’s logins into its table, with no Cockpit card', async () => {
    const logins = agyFixture({ accounts: [1, 2].map((n) => agyAccount(n, { email: fakeEmail(n) })) });
    const model = fixtureModel('fresh', { antigravity: logins, agySwitch: { busy: null, listed: true, overwritten: null } });
    const fake = await open(model, 'agy');
    expect(screen.queryByText('Cockpit Tools')).toBeNull();
    expect(screen.queryByText(/Cockpit làm mới không đều/)).toBeNull();
    expect(screen.queryByText('Đăng nhập trong Quota Station')).toBeNull();
    const table = screen.getByRole('list', { name: 'Antigravity' });
    const header = Array.from(table.querySelectorAll('.wt-r.h')).map((h) => Array.from(h.children).map((c) => c.textContent));
    expect(header).toEqual([
      ['', 'Gemini', 'Claude/GPT', ''],
      ['đã dùng', '5 giờ', 'Tuần', '5 giờ', 'Tuần', ''],
    ]);
    const [u1, u2] = within(table).getAllByRole('listitem');
    expect(within(u1).getByRole('button', { name: 'Chuyển sang lanpham.ultra' })).toBeTruthy();
    expect(within(u1).getByRole('button', { name: 'Đăng xuất lanpham.ultra' })).toBeTruthy();
    expect(within(u2).getByText('Đang dùng')).toBeTruthy();
    expect(within(u2).getByRole('button', { name: 'Đăng xuất lan.ultra2' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Thêm tài khoản' })).toBeTruthy();

    fireEvent.click(within(u1).getByRole('button', { name: 'Đăng xuất lanpham.ultra' }));
    const confirm = screen.getByRole('group', { name: 'Đăng xuất lanpham.ultra' });
    await act(async () => {
      fireEvent.click(within(confirm).getByRole('button', { name: 'Đăng xuất' }));
    });
    expect(fake.api.agyLogout).toHaveBeenCalledTimes(1);
  });

  it('a Cockpit Tools Mac keeps its Cockpit card and a short list of the app’s logins', async () => {
    const model = fixtureModel('cockpit', { antigravity: agyFixture({ accounts: [agyAccount(1), agyAccount(2)] }) });
    model.snapshot.agy.source = 'cockpit';
    await open(model, 'agy');
    expect(screen.getByText('Cockpit Tools', { selector: 'h4' })).toBeTruthy();
    expect(screen.getByText('đang chạy')).toBeTruthy();
    // Logins quota-axi does not list stay under the table.
    expect(within(screen.getByRole('list', { name: 'Tài khoản đăng nhập trong app' })).getAllByRole('listitem')).toHaveLength(2);
  });

  it('a Mac without an account list or without Antigravity says so on the Antigravity tab', async () => {
    await open(fixtureModel('no-cockpit'), 'agy');
    expect(screen.getByText('Chỉ thấy tài khoản đang dùng')).toBeTruthy();
    expect(within(screen.getByRole('list', { name: 'Antigravity' })).getAllByRole('listitem')).toHaveLength(1);
    cleanup();

    await open(fixtureModel('no-antigravity'), 'agy');
    expect(screen.getByText('Máy này chưa có Antigravity')).toBeTruthy();
    expect(screen.queryByRole('list', { name: 'Antigravity' })).toBeNull();
    fireEvent.click(nav('Tổng quan'));
    expect(screen.queryByRole('list', { name: 'Antigravity' })).toBeNull();
  });

  it('"Làm mới" reads again and reports when done', async () => {
    const fake = await open(fixtureModel('fresh', { reads: 3 }), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Làm mới' }));
    });
    expect(fake.api.refresh).toHaveBeenCalledTimes(1);
    expect((screen.getByRole('button', { name: 'Làm mới' }) as HTMLButtonElement).disabled).toBe(true);
    act(() => fake.push(fixtureModel('fresh', { reads: 4 })));
    expect(screen.getByText('Đã làm mới').closest('.toast')).toBeTruthy();
  });

  it('a line that only says it went well goes by itself after six seconds', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const fake = await open(fixtureModel('fresh', { reads: 3 }), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Làm mới' }));
    });
    act(() => fake.push(fixtureModel('fresh', { reads: 4 })));
    expect(screen.getByText('Đã làm mới')).toBeTruthy();
    act(() => vi.advanceTimersByTime(5900));
    expect(screen.getByText('Đã làm mới')).toBeTruthy();
    act(() => vi.advanceTimersByTime(200));
    expect(screen.queryByText('Đã làm mới')).toBeNull();
    vi.useRealTimers();
  });

  it('quota-axi missing points to the settings tab and keeps the last numbers', async () => {
    const fake = await open(fixtureModel('fresh', { errors: failing('notFound') }));
    expect(screen.getByText('Mac mini · không đọc được số')).toBeTruthy();
    expect(screen.getByText('số lúc 14:20')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Chọn đường dẫn…' }));
    expect(heading()).toBe('Cài đặt');
    expect(fake.api.openWindow).not.toHaveBeenCalled();
  });

  it('the Antigravity tab says quota-axi is not running', async () => {
    await open(fixtureModel('fresh', { errors: failing('notFound') }), 'agy');
    expect(screen.getByText('Không chạy được quota-axi')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Chọn đường dẫn…' }));
    expect(heading()).toBe('Cài đặt');
  });

  it('follows the reader like the popover does', async () => {
    const fake = await open(fixtureModel('fresh'));
    act(() => fake.push(fixtureModel('exhausted')));
    expect(screen.getByText('Hạn mức Claude trong vòng đã cạn')).toBeTruthy();
    expect(screen.getByText('cạn · có lại lúc 16:40')).toBeTruthy();
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
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang lan.acme3' }));
    });
    const dialog = screen.getByRole('dialog', { name: 'Chuyển Claude sang lan.acme3?' });
    expect(within(dialog).getAllByRole('listitem')[1].textContent).toMatch(/^5 phiên Claude đang chạy/);
    fireEvent.click(within(dialog).getByRole('radio', { name: /30 phút/ }));
    await act(async () => {
      fireEvent.click(within(dialog).getByRole('button', { name: 'Chuyển sang lan.acme3' }));
    });
    expect(fake.api.switchClaude).toHaveBeenCalledWith(4, '2026-01-16T07:50:40.000Z');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText('Đã chuyển sang lan.acme3. Giữ tài khoản này đến 14:50.')).toBeTruthy();
  });

  it('a refresh while paused keeps the paused banner and its buttons', async () => {
    const fake = await open(fixtureModel('fresh', { reads: 3, switcher: heldSwitcher() }), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Làm mới' }));
    });
    act(() => fake.push(fixtureModel('fresh', { reads: 4, switcher: heldSwitcher() })));
    expect(screen.getByText('Đã làm mới')).toBeTruthy();
    expect(screen.getByText('Đang giữ lan.acme1 đến 15:20')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Thêm 1 giờ' })).toBeTruthy();
  });

  it('a feedback line stays on the tab it was raised on, with its buttons', async () => {
    await open(fixtureModel('fresh'), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang lan.acme3' }));
    });
    await act(async () => {
      fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Chuyển sang lan.acme3' }));
    });
    expect(screen.getByText('Đã chuyển sang lan.acme3. Giữ tài khoản này đến 15:20.')).toBeTruthy();
    fireEvent.click(nav('Cài đặt'));
    expect(screen.queryByText(/Đã chuyển sang lan.acme3/)).toBeNull();
    fireEvent.click(nav('Antigravity'));
    expect(screen.queryByText(/Đã chuyển sang lan.acme3/)).toBeNull();
    fireEvent.click(nav('Claude'));
    expect(screen.getByText('Đã chuyển sang lan.acme3. Giữ tài khoản này đến 15:20.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Cho phép tự chuyển lại' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    expect(screen.queryByText(/Đã chuyển sang lan.acme3/)).toBeNull();
  });

  it('Escape closes the dialog and Thôi does too, without switching', async () => {
    const fake = await open(fixtureModel('fresh'), 'claude');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Thôi' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });

  it('the Tự chuyển card is one line with its switch, and pause opens the sheet on its choice', async () => {
    const fake = await open(fixtureModel('fresh'), 'claude');
    expect(screen.getByText('14:19 · Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.')).toBeTruthy();
    expect(screen.queryByText(/Dòng gốc/)).toBeNull();
    const quick = within(screen.getByRole('group', { name: 'Tạm dừng' }))
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

  it('the Switcher card is one line; whether it runs, its last decision and log sit under Chi tiết', async () => {
    await open(fixtureModel('fresh', { switcher: heldSwitcher() }), 'claude');
    expect(screen.getByText(/^Chạy mỗi 3 phút · kiểm lúc \d\d:\d\d$/)).toBeTruthy();
    expect(screen.getByText('Chi tiết').tagName).toBe('SUMMARY');
    const details = kv();
    expect(details).toMatchObject({
      'LaunchAgent cswap-prefer-team': 'chạy mỗi 3 phút',
      'com.cswap.auto': 'tắt',
      'Tạm dừng tự chuyển': 'đến 15:20',
      'Switcher đọc cờ tạm dừng': 'có',
      'Nhật ký 09:12': '2026-01-16 09:12:05 switch to slot 2 (ends for good at 17:45, use it first)',
    });
    // While paused, the decision is the switcher's own paused line.
    expect(Object.entries(details).find(([k]) => k.startsWith('Quyết định gần nhất'))?.[1]).toMatch(/^paused until/);
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

  it('Antigravity rows do not switch while quota-axi does not list the app\'s accounts', async () => {
    await open(fixtureModel('fresh'), 'agy');
    for (const b of screen.getAllByRole('button', { name: /^Chuyển sang / })) {
      expect((b as HTMLButtonElement).disabled).toBe(true);
    }
  });
});
