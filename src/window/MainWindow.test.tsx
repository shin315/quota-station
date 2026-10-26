// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, fakeEmail, hideNames } from '../fixtures/fakeApi';
import { agyAccount, agyFixture, claudeAccountsFixture, communityModel, FIXTURE_NOW, fixtureModel, heldSwitcher, switcherFixture } from '../fixtures/models';
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
    for (const tab of ['rules', 'history', 'firstmate', 'settings', 'codex', 'mcp', 'skills', 'plugins', 'instr'] as Tab[]) {
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

  it('Tổng quan is one section per service, every account by its windows, automatic switching in the Claude head', async () => {
    await open(fixtureModel('fresh'));
    expect(heading()).toBe('Tổng quan');
    expect(screen.getByText('Mac mini · cập nhật 40 giây trước')).toBeTruthy();
    // No tiles and no Sắp tới: the reset times stay under each bar, the account leaving the rotation keeps its chip.
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
    expect(screen.getByText(/^% đã dùng của từng hạn mức/)).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chi tiết: % đã dùng của từng hạn mức' }));
    });
    expect(screen.getByText(/Dấu — là chưa có số, không phải 0\./)).toBeTruthy();

    const claude = screen.getByRole('list', { name: 'Claude Code' });
    const rows = within(claude).getAllByRole('listitem');
    expect(rows).toHaveLength(4);
    expect(rows[1].getAttribute('aria-label')).toMatch(/^lan\.acme1, Acme Studio, đang dùng\./);
    expect(within(rows[1]).queryByRole('button')).toBeNull();
    // Fixed columns: 5 giờ · Tuần · Fable tuần, whatever window is tightest.
    const header = claude.querySelector('.wt-r.h')!;
    expect(Array.from(header.children).map((c) => c.textContent)).toEqual(['Tài khoản', '5 giờ', 'Tuần', 'Fable tuần', '']);
    expect(Array.from(rows[0].querySelectorAll('.wc b')).map((b) => b.textContent)).toEqual(['0%', '0%', '4%']);
    expect(rows[1].getAttribute('aria-label')).toMatch(/rời vòng 17:45$/);
    expect(within(screen.getByRole('list', { name: 'Antigravity' })).getAllByRole('listitem')).toHaveLength(2);

    // Automatic switching in the Claude section's head: its switch and why Claude is on its account.
    const head = sections[0].querySelector('[data-slot=card-title]') as HTMLElement;
    expect(within(head).getByText('14:19 · Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.')).toBeTruthy();
    expect(screen.queryByText(/^Vì sao đang ở/)).toBeNull();
    expect(within(head).getByRole('switch', { name: 'Tự chuyển Claude' }).getAttribute('aria-checked')).toBe('true');

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
    expect(rows.map((r) => r.querySelector('.an b')!.textContent)).toEqual(['Cá nhân', 'Team A', 'Team B', 'Team C']);
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
    expect(screen.queryByText('Cập nhật')).toBeNull();
    expect(screen.getAllByText('số 41 phút trước').length).toBeGreaterThan(0);
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
    expect(screen.getByText('Hạn mức Claude trong vòng đã cạn')).toBeTruthy();
    expect(screen.getAllByText('96%').length).toBeGreaterThan(0);
  });
});

describe('with Nâng cao off', () => {
  const plain = (m: Model): Model => ({ ...m, advanced: { on: false, firstmate: true } });

  it('the sidebar has no Luật xoay vòng, Firstmate or Chép sang Mac khác, and a hidden tab gives way to Tổng quan', async () => {
    await open(plain(fixtureModel('fresh')), 'rules');
    expect(heading()).toBe('Tổng quan');
    const sidebar = screen.getByRole('navigation', { name: 'Các tab' });
    for (const name of ['Luật xoay vòng', 'Firstmate', 'Chép sang Mac khác']) expect(within(sidebar).queryByRole('button', { name })).toBeNull();
    expect(nav('Claude')).toBeTruthy();
    expect(nav('Sao lưu')).toBeTruthy();
  });

  it('the Claude tab has no Tự chuyển card or pause, and accounts go by number', async () => {
    await open(hideNames(plain(fixtureModel('fresh'))), 'claude');
    expect(screen.queryByRole('group', { name: 'Tạm dừng' })).toBeNull();
    expect(screen.queryByText(/chưa tới ngưỡng tự chuyển/)).toBeNull();
    const rows = within(screen.getByRole('list', { name: 'Claude Code' })).getAllByRole('listitem');
    const names = rows.map((r) => r.querySelector('.an b')!.textContent);
    expect(names.every((n) => /^Tài khoản \d+$/.test(n ?? ''))).toBe(true);
    expect(document.body.textContent).not.toMatch(/Team A|Cá nhân/);
  });

  it('the Firstmate switch hides only the Firstmate tab', async () => {
    await open({ ...fixtureModel('fresh'), advanced: { on: true, firstmate: false } });
    const sidebar = screen.getByRole('navigation', { name: 'Các tab' });
    expect(within(sidebar).queryByRole('button', { name: 'Firstmate' })).toBeNull();
    expect(nav('Luật xoay vòng')).toBeTruthy();
  });

  it('a build without the Antigravity accounts reads Antigravity without logging in', async () => {
    await open({ ...fixtureModel('fresh'), capabilities: { agyAccounts: false, authorTools: true } }, 'agy');
    expect(screen.queryByRole('button', { name: 'Thêm tài khoản' })).toBeNull();
    expect(screen.getByText(/Bản này chỉ đọc tài khoản đang đăng nhập/)).toBeTruthy();
    expect(screen.getByText('Đổi tài khoản trong chính Antigravity; Quota Station đọc lại ở lần đọc sau.')).toBeTruthy();
  });
});

describe('a build without the author\'s tools', () => {
  it('has none of their tabs, and no automatic switching, pause or Firstmate line', async () => {
    await open(communityModel(fixtureModel('fresh')), 'rules');
    expect(heading()).toBe('Tổng quan');
    const sidebar = screen.getByRole('navigation', { name: 'Các tab' });
    for (const name of ['Luật xoay vòng', 'Firstmate', 'Chép sang Mac khác']) expect(within(sidebar).queryByRole('button', { name })).toBeNull();
    expect(nav('Claude')).toBeTruthy();
    expect(nav('Sao lưu')).toBeTruthy();
    cleanup();
    await open(communityModel(fixtureModel('fresh')), 'claude');
    expect(screen.queryByRole('group', { name: 'Tạm dừng' })).toBeNull();
    expect(screen.queryByText(/Firstmate/)).toBeNull();
    // The Claude accounts' own rotation switch stays: it is not one of the author's tools.
    expect(screen.getByRole('switch', { name: 'Tự chuyển bằng Quota Station' })).toBeTruthy();
  });

  it('the same Mac on a build with them keeps the tabs and the Firstmate line', async () => {
    await open(fixtureModel('fresh'), 'claude');
    for (const name of ['Luật xoay vòng', 'Firstmate', 'Chép sang Mac khác']) expect(nav(name)).toBeTruthy();
    expect(screen.getByText(/^Firstmate dùng cùng bộ luật\./)).toBeTruthy();
    // The commands drawn as code, without their backticks, behind Chi tiết.
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chi tiết: Chạy nền' }));
    });
    expect(screen.getByText('quota-station claude status --json', { selector: 'code' }).closest('p')!.textContent).toMatch(/^Firstmate đọc quota-station claude status --json và chạy/);
  });
});

describe('switching Claude and pausing from the main window', () => {
  const details = async () => {
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chi tiết: Chạy nền' }));
    });
  };
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

  it('one card in two halves: Tự chuyển Claude with its switch and pause, and what runs it', async () => {
    const fake = await open(fixtureModel('fresh'), 'claude');
    const auto = screen.getByRole('region', { name: /^Tự chuyển Claude/ });
    const run = screen.getByRole('region', { name: /^Chạy nền/ });
    expect(auto.parentElement).toBe(run.parentElement);
    expect(within(auto).getByText('14:19 · Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.')).toBeTruthy();
    expect(within(auto).getByRole('switch', { name: 'Tự chuyển Claude' })).toBeTruthy();
    expect(within(run).getByRole('switch', { name: 'Tự chuyển bằng Quota Station' })).toBeTruthy();
    expect(within(auto).getByRole('group', { name: 'Tạm dừng' })).toBeTruthy();
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
    await details();
    expect(kv()['Tạm dừng tự chuyển']).toBe('đến 18:00');
  });

  it('Chạy nền says when it last ran; whether it runs, its last decision and log sit under Chi tiết', async () => {
    await open(fixtureModel('fresh', { switcher: heldSwitcher() }), 'claude');
    const run = screen.getByRole('region', { name: /^Chạy nền/ });
    expect(within(run).getByText(/^Kiểm lúc \d\d:\d\d$/)).toBeTruthy();
    expect(within(run).getByText('LaunchAgent chạy mỗi 3 phút, kể cả khi đã đóng app.')).toBeTruthy();
    expect(kv()).toEqual({});
    await details();
    const rows = kv();
    expect(rows).toMatchObject({
      'LaunchAgent tự chuyển': 'chạy mỗi 3 phút',
      'Bộ tự chuyển cũ': 'tắt',
      'Tạm dừng tự chuyển': 'đến 15:20',
      'Switcher đọc cờ tạm dừng': 'có',
      'Nhật ký 09:12': '2026-01-16 09:12:05 switch to slot 2 (ends for good at 17:45, use it first)',
    });
    // While paused, the decision is the switcher's own paused line.
    expect(Object.entries(rows).find(([k]) => k.startsWith('Quyết định gần nhất'))?.[1]).toMatch(/^paused until/);
  });

  it('a rotation that is not loaded gets a banner that turns it on', async () => {
    const fake = await open(
      fixtureModel('fresh', { switcher: switcherFixture({ service: false }), claudeAccounts: claudeAccountsFixture({ rotation: false }) }),
      'claude',
    );
    expect(screen.getByText('Tự chuyển Claude đang tắt')).toBeTruthy();
    await details();
    expect(kv()['LaunchAgent tự chuyển']).toBe('tắt');
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Bật tự chuyển' }));
    });
    expect(fake.api.claudeSetRotation).toHaveBeenCalledWith(true);
  });

  it('an older switcher still loaded is a warning with a guide to stop it', async () => {
    await open(fixtureModel('fresh', { switcher: switcherFixture({ legacy: 'com.cswap.prefer-team' }) }), 'claude');
    await details();
    expect(kv()['Bộ tự chuyển cũ']).toBe('com.cswap.prefer-team đang nạp');
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    fireEvent.click(screen.getAllByRole('button', { name: 'Hướng dẫn tắt' })[0]);
    const guide = screen.getByRole('dialog', { name: 'Tắt bộ tự chuyển cũ' });
    expect(within(guide).getByText('launchctl bootout gui/$(id -u)/com.cswap.prefer-team')).toBeTruthy();
    fireEvent.click(within(guide).getByRole('button', { name: 'Đóng' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('Antigravity rows do not switch while the app\'s account list is not read', async () => {
    await open(fixtureModel('fresh'), 'agy');
    for (const b of screen.getAllByRole('button', { name: /^Chuyển sang / })) {
      expect((b as HTMLButtonElement).disabled).toBe(true);
    }
  });
});
