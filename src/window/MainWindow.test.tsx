// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, fakeEmail, hideNames } from '../fixtures/fakeApi';
import { agyAccount, agyFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
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
  await screen.findByRole('navigation', { name: 'Các tab' });
  return fake;
}

const heading = () => screen.getByRole('heading', { level: 2 }).textContent;
const nav = (name: string) => within(screen.getByRole('navigation', { name: 'Các tab' })).getByRole('button', { name });

afterEach(cleanup);

describe('main window', () => {
  it('opens on the tab it was asked for', async () => {
    await open(fixtureModel('fresh'), 'claude');
    expect(heading()).toBe('Claude');
    expect(nav('Claude').getAttribute('aria-current')).toBe('page');
  });

  it('drags from the whole top strip and the title row, its buttons still buttons', async () => {
    await open(fixtureModel('fresh'));
    const strip = document.querySelector('.win-o > .dragbar')!;
    expect(strip.getAttribute('data-tauri-drag-region')).toBe('true');
    // Deep: the title and the freshness line drag too, not only the gaps between them.
    const bar = screen.getByRole('heading', { level: 2 }).parentElement!;
    expect(bar.getAttribute('data-tauri-drag-region')).toBe('deep');
    const refresh = within(bar).getByRole('button', { name: 'Làm mới' });
    expect(refresh.hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(within(bar).getByRole('button', { name: 'Thông báo' }).hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(screen.getByRole('navigation', { name: 'Các tab' }).hasAttribute('data-tauri-drag-region')).toBe(true);
    let tabs: Tab[] = ['history', 'settings', 'codex', 'mcp', 'skills', 'plugins', 'instr'];
    for (const tab of tabs) {
      cleanup();
      await open(fixtureModel('fresh'), tab);
      const row = screen.getByRole('heading', { level: 2 }).parentElement!;
      expect(row.getAttribute('data-tauri-drag-region')).toBe('deep');
      // The bell ends every page's title row.
      expect(within(row).getAllByRole('button').at(-1)!.getAttribute('aria-label')).toBe('Thông báo');
    }
  });

  it('the notifications are a bell in the title row, not a tab: asked for, they open over Tổng quan', async () => {
    await open(fixtureModel('fresh'), 'alerts');
    expect(within(screen.getByRole('navigation', { name: 'Các tab' })).queryByRole('button', { name: /Thông báo|Cảnh báo/ })).toBeNull();
    expect(heading()).toBe('Tổng quan');
    expect(nav('Tổng quan').getAttribute('aria-current')).toBe('page');
    expect(screen.getByRole('dialog', { name: 'Thông báo' })).toBeTruthy();
  });

  it('Tổng quan is one section per service, every account by its windows', async () => {
    await open(fixtureModel('fresh'));
    expect(heading()).toBe('Tổng quan');
    expect(screen.getByText('Mac Studio · cập nhật 40 giây trước')).toBeTruthy();
    // No tiles and no Sắp tới: the reset times stay under each bar.
    expect(screen.queryByRole('list', { name: 'Đang dùng' })).toBeNull();
    expect(screen.queryByText('Sắp tới')).toBeNull();
    expect(screen.queryByText(/còn dùng được|tài khoản Claude/)).toBeNull();
    // Three sections, each the whole width, in this order.
    const sections = Array.from(document.querySelectorAll('[data-slot=card]'));
    expect(sections).toHaveLength(3);
    expect(sections.map((c) => within(c as HTMLElement).getByRole('list').getAttribute('aria-label'))).toEqual(['Claude Code', 'Antigravity', 'Codex']);
    expect(new Set(sections.map((c) => c.parentElement))).toHaveProperty('size', 1);
    // The names and the buttons on the same columns in all three.
    const cols = sections.map((c) => within(c as HTMLElement).getByRole('list').className.match(/\[--wt-cols:([^\]]+)\]/)![1].split('_'));
    for (const c of cols) {
      expect(c[0]).toBe('minmax(120px,22%)');
      expect(c[c.length - 1]).toBe('96px');
    }
    // What the numbers are, once for the page, behind Chi tiết.
    expect(screen.getByText(/^Phần trăm đã dùng của từng hạn mức/)).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chi tiết: Phần trăm đã dùng của từng hạn mức' }));
    });
    expect(screen.getByText(/Dấu — là chưa có số, không phải 0\./)).toBeTruthy();

    const claude = screen.getByRole('list', { name: 'Claude Code' });
    const rows = within(claude).getAllByRole('listitem');
    expect(rows).toHaveLength(4);
    let active = /^lan\.acme1, đang dùng\./;
    active = /^lan\.acme1, Acme Studio, đang dùng\./;
    expect(rows[1].getAttribute('aria-label')).toMatch(active);
    expect(within(rows[1]).queryByRole('button')).toBeNull();
    // Fixed columns: 5 giờ · Tuần · Fable tuần, whatever window is tightest.
    const header = claude.querySelector('.wt-r.h')!;
    expect(Array.from(header.children).map((c) => c.textContent)).toEqual(['Tài khoản', '5 giờ', 'Tuần', 'Fable tuần', '']);
    expect(Array.from(rows[0].querySelectorAll('.wc b')).map((b) => b.textContent)).toEqual(['0%', '0%', '4%']);
    // The account leaving the rotation keeps its chip.
    expect(rows[1].getAttribute('aria-label')).toMatch(/rời vòng 17:45$/);
    expect(within(screen.getByRole('list', { name: 'Antigravity' })).getAllByRole('listitem')).toHaveLength(2);


    // Every Claude account but the active one switches; Antigravity only once the app's account list is read.
    for (const name of ['lanpham.dev', 'lan.acme2', 'lan.acme3']) {
      expect((screen.getByRole('button', { name: 'Chuyển sang ' + name }) as HTMLButtonElement).disabled).toBe(false);
    }
    const ultra = screen.getByRole('button', { name: 'Chuyển sang lanpham.ultra' }) as HTMLButtonElement;
    expect(ultra.disabled).toBe(true);
    expect(ultra.title).toBe('Chưa chuyển được: chưa đọc được danh sách tài khoản của Quota Station');
    expect(screen.queryByRole('button', { name: 'Chuyển sang lan.acme1' })).toBeNull();
  });

  it('"Chuyển" is a button to press; "Đang dùng" is a status that cannot be pressed', async () => {
    await open(fixtureModel('fresh'), 'claude');
    const rows = within(screen.getByRole('list', { name: 'Claude Code' })).getAllByRole('listitem');
    const inUse = within(rows[1]).getByText('Đang dùng');
    expect(inUse.closest('button')).toBeNull();
    expect(inUse.tagName).toBe('SPAN');
    expect(inUse.querySelector('[data-tone=ok]')).toBeTruthy();
    const press = within(rows[2]).getByRole('button', { name: 'Chuyển sang lan.acme2' });
    // The blue button that goes ahead.
    expect(press.getAttribute('data-variant')).toBe('primary');
    expect(press.textContent).toBe('Chuyển');
  });

  it('with names hidden, no email or organization reaches the window', async () => {
    await open(hideNames(fixtureModel('fresh')), 'claude');
    const rows = within(screen.getByRole('list', { name: 'Claude Code' })).getAllByRole('listitem');
    let names = ['Tài khoản 1', 'Tài khoản 2', 'Tài khoản 3', 'Tài khoản 4'];
    names = ['Cá nhân', 'Team A', 'Team B', 'Team C'];
    expect(rows.map((r) => r.querySelector('.an b')!.textContent)).toEqual(names);
    expect(document.body.textContent).not.toContain('@');
    expect(document.body.textContent).not.toContain('Acme');
    expect(document.body.innerHTML).not.toContain('example.com');
  });

  it('Codex has its own tab, which looks at Codex when it opens, and a section on Tổng quan', async () => {
    const fake = await open(fixtureModel('fresh'));
    // The ChatGPT accounts by their windows, as Claude's are, then the provider Codex goes through.
    const codex = within(screen.getByRole('list', { name: 'Codex' })).getAllByRole('listitem');
    expect(codex.map((r) => r.getAttribute('aria-label'))).toEqual([
      'ChatGPT, ChatGPT · plugin dùng đăng nhập này. 5 giờ đã dùng 23%, đặt lại sau 2 giờ 09 phút. Tuần đã dùng 41%, đặt lại sau 4 ngày 5 giờ 39 phút',
      'relay, Custom API · relay.example.com · trả lời 180 ms, đang dùng. Không báo hạn mức',
    ]);
    expect(within(codex[1]).getByText('Không báo hạn mức')).toBeTruthy();
    expect(within(codex[1]).getByText('Đang dùng')).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(codex[0]).getByRole('button', { name: 'Chuyển sang ChatGPT' }));
    });
    // The same sheet as the Codex tab's, before anything changes.
    expect(screen.getByRole('dialog', { name: 'Chuyển Codex sang ChatGPT?' })).toBeTruthy();
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
    expect(fake.api.codexUse).not.toHaveBeenCalled();
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Thôi' }));
    fireEvent.click(nav('Codex'));
    expect(heading()).toBe('Codex');
    expect(fake.api.codexCheck).toHaveBeenCalled();
  });

  it('asks for a tab\'s read in the commit that shows the tab, even on a slow machine', async () => {
    // Every look at the clock finds 10 ms gone, so React's scheduler yields
    // after each piece of work: a passive effect would run after the tab is
    // on screen, as it did now and then on a busy CI runner.
    let t = performance.now();
    const clock = vi.spyOn(performance, 'now').mockImplementation(() => (t += 10));
    try {
      const reads: [Tab, string, keyof Api][] = [
        ['backup', 'Sao lưu', 'backupRead'],
        ['mcp', 'MCP', 'mcpRead'],
        ['settings', 'Cài đặt', 'settingsOpened'],
      ];
      for (const [tab, name, read] of reads) {
        const fake = fakeApi(fixtureModel('fresh'), tab);
        const asked = vi.spyOn(fake.api, read);
        render(
          <div className="qs app-window">
            <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
          </div>,
        );
        await screen.findByRole('heading', { level: 2, name });
        expect(asked).toHaveBeenCalled();
        cleanup();
      }
    } finally {
      clock.mockRestore();
    }
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
    expect(screen.getByText('4 tài khoản Quota Station')).toBeTruthy();
    expect(screen.queryByText(/tài khoản claude-swap/)).toBeNull();
    // No column of when each was updated; the sidebar's Cập nhật is another place.
    expect(within(screen.getByRole('main')).queryByText('Cập nhật')).toBeNull();
    expect(screen.getAllByText('cập nhật 41 phút trước').length).toBeGreaterThan(0);
    expect(screen.getByText('Số liệu Claude đã cũ 41 phút')).toBeTruthy();
  });

  it('the Claude tab names claude-swap as the keeper only until Quota Station keeps the accounts', async () => {
    await open(fixtureModel('fresh', { claudeSwitch: { ready: false, busy: null } }), 'claude');
    expect(screen.getByText('4 tài khoản claude-swap')).toBeTruthy();
    expect(screen.queryByText('4 tài khoản Quota Station')).toBeNull();
  });

  it('the Antigravity tab merges the app’s logins into its table', async () => {
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
      ['Tài khoản', '5 giờ', 'Tuần', '5 giờ', 'Tuần', ''],
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

  it('while the app’s account list is not read, its logins stay in a short list under the table', async () => {
    await open(fixtureModel('unavailable', { antigravity: agyFixture({ accounts: [agyAccount(1), agyAccount(2)] }) }), 'agy');
    expect(screen.getByText('Không đọc được danh sách tài khoản')).toBeTruthy();
    expect(within(screen.getByRole('list', { name: 'Tài khoản đăng nhập trong app' })).getAllByRole('listitem')).toHaveLength(2);
  });

  it('a Mac without an account list or without Antigravity says so on the Antigravity tab', async () => {
    await open(fixtureModel('single'), 'agy');
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

  it('follows the reader like the popover does', async () => {
    const fake = await open(fixtureModel('fresh'));
    act(() => fake.push(fixtureModel('exhausted')));
    let dry = 'Hạn mức Claude sắp cạn';
    dry = 'Hạn mức Claude trong vòng đã cạn';
    expect(screen.getByText(dry)).toBeTruthy();
    expect(screen.getAllByText('96%').length).toBeGreaterThan(0);
  });
});


describe('a build without the Antigravity accounts', () => {
  it('reads Antigravity without logging in', async () => {
    const m = fixtureModel('fresh');
    await open({ ...m, capabilities: { ...m.capabilities, agyAccounts: false } }, 'agy');
    expect(screen.queryByRole('button', { name: 'Thêm tài khoản' })).toBeNull();
    expect(screen.getByText('Đổi tài khoản trong chính Antigravity; Quota Station đọc lại ở lần đọc sau.')).toBeTruthy();
  });
});


describe('switching Claude from the main window', () => {




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





  it('Antigravity rows do not switch while the app\'s account list is not read', async () => {
    await open(fixtureModel('fresh'), 'agy');
    for (const b of screen.getAllByRole('button', { name: /^Chuyển sang / })) {
      expect((b as HTMLButtonElement).disabled).toBe(true);
    }
  });
});
