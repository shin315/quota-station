// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The MCP tab (M9) on the fake Rust side: nothing here reads or writes an
// agent's file, runs `claude` or opens the Keychain. The servers are made up.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel, mcpBrandsFixture, mcpFixture, mcpTableFixture } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

async function openTab(model: Model = fixtureModel('fresh'), options: FakeOptions = {}) {
  const fake = fakeApi(model, 'mcp', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { level: 2, name: 'MCP' });
  return fake;
}

/** Radix moves focus a tick after React commits: the menu's arrows, the icon again when the menu closes. */
async function tick() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
  await tick();
}

async function key(el: Element | Window, k: string) {
  await act(async () => {
    fireEvent.keyDown(el, { key: k });
  });
  await tick();
}

/** Opens a column's menu as a pointer does: Radix opens a menu on pointerdown, not on click. */
async function openMenu(el: HTMLElement) {
  await act(async () => {
    fireEvent.pointerDown(el, { button: 0, ctrlKey: false });
  });
}

const box = (name: string) => screen.getByRole('checkbox', { name: new RegExp('^' + name) });

afterEach(cleanup);

describe('the MCP tab', () => {
  it('is in the sidebar under Agent and reads every agent\'s list when it opens', async () => {
    const fake = await openTab();
    expect(fake.api.mcpRead).toHaveBeenCalled();
    const nav = screen.getByRole('navigation', { name: 'Các tab' });
    expect(within(nav).getByText('Agent')).toBeTruthy();
    expect(within(nav).getByRole('button', { name: 'MCP' }).getAttribute('aria-current')).toBe('page');
    expect(screen.getByRole('heading', { level: 4, name: /MCP của bạn/ })).toBeTruthy();
    expect(screen.getByText('trên máy · files-mcp')).toBeTruthy();
    expect(screen.getByText('qua mạng · docs.example.com')).toBeTruthy();
    expect(screen.getByText('khác giữa các agent')).toBeTruthy();
    expect(screen.getByText('2 MCP có sẵn trong Codex. App để yên, không sửa.')).toBeTruthy();
  });

  it('draws a checkbox per agent that the keyboard can tick, and a locked one stays reachable', async () => {
    await openTab();
    const cell = box('browser cho Antigravity');
    expect(cell.tagName).toBe('BUTTON');
    expect(cell.getAttribute('aria-checked')).toBe('false');
    cell.focus();
    // A button takes Space and Enter as a press.
    await press(cell);
    expect(box('browser cho Antigravity').getAttribute('aria-checked')).toBe('true');
    expect(box('browser cho Antigravity').getAttribute('aria-label')).toBe('browser cho Antigravity, chưa ghi');
    const locked = box('ticket-desk cho Codex');
    expect(locked.getAttribute('aria-disabled')).toBe('true');
    expect(locked.hasAttribute('disabled')).toBe(false);
    await press(locked);
    expect(box('ticket-desk cho Codex').getAttribute('aria-checked')).toBe('false');
  });

  it('lists the ticks on the pending bar, and a tick taken back leaves it', async () => {
    await openTab();
    await press(box('browser cho Antigravity'));
    await press(box('notes cho Codex'));
    const bar = screen.getByText('2 thay đổi chưa ghi').closest('.pendbar') as HTMLElement;
    expect(within(bar).getByText('browser: bật cho Antigravity · notes: bật cho Codex')).toBeTruthy();
    await press(box('notes cho Codex'));
    expect(screen.getByText('1 thay đổi chưa ghi')).toBeTruthy();
    await press(within(bar).getByRole('button', { name: 'Bỏ' }));
    expect(screen.queryByText(/thay đổi chưa ghi/)).toBeNull();
  });

  it('opens a column\'s menu from its icon: menu items, arrows, Esc, focus back on the icon', async () => {
    await openTab();
    const icon = screen.getByRole('button', { name: 'Antigravity: tuỳ chọn cột' });
    expect(icon.getAttribute('aria-haspopup')).toBe('menu');
    expect(icon.getAttribute('aria-expanded')).toBe('false');
    await openMenu(icon);
    expect(icon.getAttribute('aria-expanded')).toBe('true');
    const menu = screen.getByRole('menu', { name: 'Antigravity' });
    const items = within(menu).getAllByRole('menuitem');
    expect(items.map((i) => i.textContent)).toEqual(['Bật tất cả', 'Tắt tất cả']);
    expect(within(menu).getByText('3/6 MCP đang tick')).toBeTruthy();
    // The keys go where the focus is, as a keyboard sends them.
    expect(document.activeElement).toBe(items[0]);
    await key(items[0], 'ArrowDown');
    expect(document.activeElement).toBe(items[1]);
    await key(items[1], 'ArrowDown');
    expect(document.activeElement).toBe(items[0]);
    await key(items[0], 'ArrowUp');
    expect(document.activeElement).toBe(items[1]);
    await key(items[1], 'Escape');
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(icon);
    expect(icon.getAttribute('aria-expanded')).toBe('false');
  });

  it('Bật tất cả skips a locked box; Bỏ N thay đổi chưa ghi takes the column back', async () => {
    await openTab();
    await openMenu(screen.getByRole('button', { name: 'Antigravity: tuỳ chọn cột' }));
    await press(screen.getByRole('menuitem', { name: 'Bật tất cả' }));
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Antigravity: tuỳ chọn cột' }));
    expect(screen.getByText('3 thay đổi chưa ghi')).toBeTruthy();
    expect(box('ticket-desk cho Antigravity').getAttribute('aria-checked')).toBe('false');
    await openMenu(screen.getByRole('button', { name: 'Antigravity: tuỳ chọn cột' }));
    expect(screen.getByRole('menuitem', { name: 'Bật tất cả' }).hasAttribute('disabled')).toBe(true);
    await press(screen.getByRole('menuitem', { name: 'Bỏ 3 thay đổi chưa ghi' }));
    expect(screen.queryByText(/thay đổi chưa ghi/)).toBeNull();
  });

  it('a click outside closes the menu', async () => {
    await openTab();
    await openMenu(screen.getByRole('button', { name: 'Codex: tuỳ chọn cột' }));
    expect(screen.getByRole('menu')).toBeTruthy();
    await act(async () => {
      // The menu listens for a press outside from the next tick on.
      await new Promise((r) => setTimeout(r, 0));
      fireEvent.pointerDown(document.body);
    });
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('a column that cannot be written has a dimmed icon and no menu', async () => {
    const t = mcpTableFixture();
    const columns = t.columns.map((c) => (c.agent === 'codex' ? { ...c, state: 'invalid' as const, reason: 'TOML parse error at line 48', writable: false } : c));
    await openTab(fixtureModel('fresh', { mcp: mcpFixture({ table: { ...t, columns } }) }));
    expect(screen.queryByRole('button', { name: 'Codex: tuỳ chọn cột' })).toBeNull();
    expect(screen.getByText('Codex, chỉ xem')).toBeTruthy();
    expect(screen.getByText('Không đọc được config.toml của Codex')).toBeTruthy();
    expect(screen.queryByRole('checkbox', { name: /cho Codex/ })).toBeNull();
  });

  it("shows a known service's logo before a server's or a connector's name, and leaves the others plain", async () => {
    await openTab(fixtureModel('fresh', { mcp: mcpFixture({ table: mcpBrandsFixture() }) }));
    const logo = (el: HTMLElement) => el.querySelector('.logo');
    expect(logo(screen.getByRole('group', { name: 'figma' }))).not.toBeNull();
    // By its package, and by its host.
    expect(logo(screen.getByRole('group', { name: 'repo-tools' }))).not.toBeNull();
    expect(logo(screen.getByRole('group', { name: 'issues' }))).not.toBeNull();
    expect(logo(screen.getByRole('group', { name: 'files' }))).toBeNull();
    await press(screen.getByRole('button', { name: 'Xem' }));
    const d = screen.getByRole('dialog', { name: 'MCP có sẵn trong agent và plugin' });
    expect(logo(within(d).getByRole('group', { name: 'claude.ai Google Drive' }))).not.toBeNull();
    expect(logo(within(d).getByRole('group', { name: 'js-repl' }))).toBeNull();
  });

  it('writes every tick in one preview, one file each, and offers Hoàn tác', async () => {
    const fake = await openTab();
    await press(box('browser cho Antigravity'));
    await press(box('notes cho Codex'));
    await press(box('files cho Claude Code'));
    await press(screen.getByRole('button', { name: 'Xem trước và ghi…' }));
    expect(fake.api.mcpPreview).toHaveBeenCalledTimes(1);
    const dialog = screen.getByRole('dialog', { name: 'Ghi 3 thay đổi?' });
    expect(within(dialog).getByText('claude mcp remove files --scope user')).toBeTruthy();
    expect(within(dialog).getByText('enabled = false')).toBeTruthy();
    expect(within(dialog).getAllByRole('region').map((r) => r.getAttribute('aria-label'))).toEqual(['Codex', 'Antigravity', 'Claude Code']);
    await press(within(dialog).getByRole('button', { name: 'Ghi 3 thay đổi' }));
    expect(fake.api.mcpWrite).toHaveBeenCalledTimes(1);
    const [changes, bases] = vi.mocked(fake.api.mcpWrite).mock.calls[0];
    expect(changes).toHaveLength(3);
    expect(Object.keys(bases).sort()).toEqual(['agy', 'claude', 'codex']);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText('Đã ghi 3 thay đổi.')).toBeTruthy();
    expect(screen.queryByText(/thay đổi chưa ghi/)).toBeNull();
    expect(box('browser cho Antigravity').getAttribute('aria-checked')).toBe('true');
    await press(screen.getByRole('button', { name: 'Hoàn tác' }));
    expect(fake.api.mcpUndo).toHaveBeenCalled();
    expect(screen.getByText('Đã hoàn tác lần ghi vừa rồi.')).toBeTruthy();
    expect(box('browser cho Antigravity').getAttribute('aria-checked')).toBe('false');
  });

  it('a file changed since the preview writes nothing and keeps the ticks', async () => {
    await openTab(fixtureModel('fresh'), { mcpChanged: true });
    await press(box('notes cho Codex'));
    await press(screen.getByRole('button', { name: 'Xem trước và ghi…' }));
    await press(screen.getByRole('button', { name: 'Ghi 1 thay đổi' }));
    expect(screen.getByText('Chưa đổi gì: config.toml vừa bị chương trình khác ghi.')).toBeTruthy();
    expect(screen.getByText('1 thay đổi chưa ghi')).toBeTruthy();
  });

  it('adds a server for the agents ticked; its key goes to the Keychain and leaves the window', async () => {
    const fake = await openTab();
    await press(screen.getAllByRole('button', { name: 'Thêm MCP' })[0]);
    const form = screen.getByRole('dialog', { name: 'Thêm MCP' });
    fireEvent.change(within(form).getByLabelText('Tên'), { target: { value: 'ticket-hub' } });
    fireEvent.change(within(form).getByLabelText('Lệnh và tham số'), { target: { value: 'npx -y ticket-hub-mcp' } });
    await press(within(form).getByRole('button', { name: '+ Thêm biến' }));
    fireEvent.change(within(form).getByLabelText('Biến 1: tên'), { target: { value: 'TICKET_HUB_TOKEN' } });
    fireEvent.change(within(form).getByLabelText('Biến 1: giá trị'), { target: { value: 'not-a-real-token' } });
    expect((within(form).getByLabelText('Biến 1: giá trị') as HTMLInputElement).type).toBe('password');
    fireEvent.click(within(form).getByLabelText('Antigravity'));
    await press(within(form).getByRole('button', { name: 'Tiếp: xem trước' }));
    expect(fake.api.mcpSaveSecret).toHaveBeenCalledWith('ticket-hub', 'env:TICKET_HUB_TOKEN', 'not-a-real-token', null);
    const changes = vi.mocked(fake.api.mcpPreview).mock.calls[0][0];
    expect(changes.map((c) => c.agent)).toEqual(['claude', 'codex']);
    expect(JSON.stringify(changes)).not.toContain('not-a-real-token');
    expect(changes[0].to).toMatchObject({ kind: 'def', keychain: ['env:TICKET_HUB_TOKEN'] });
    const preview = screen.getByRole('dialog', { name: 'Thêm ticket-hub?' });
    expect(document.body.innerHTML).not.toContain('not-a-real-token');
    // Back to the form: the key is kept, not shown.
    await press(within(preview).getByRole('button', { name: 'Quay lại' }));
    const again = screen.getByRole('dialog', { name: 'Thêm MCP' });
    expect((within(again).getByLabelText('Biến 1: giá trị') as HTMLInputElement).value).toBe('');
    expect((within(again).getByLabelText('Biến 1: giá trị') as HTMLInputElement).placeholder).toBe('giữ khoá đang có');
    await press(within(again).getByRole('button', { name: 'Tiếp: xem trước' }));
    await press(screen.getByRole('button', { name: 'Ghi 2 thay đổi' }));
    expect(screen.getByRole('checkbox', { name: 'ticket-hub cho Claude Code' }).getAttribute('aria-checked')).toBe('true');
  });

  it('adds from a pasted JSON, its keys saved only on Tiếp', async () => {
    const fake = await openTab();
    await press(screen.getAllByRole('button', { name: 'Thêm MCP' })[0]);
    await press(screen.getByRole('button', { name: 'Dán JSON hoặc TOML' }));
    vi.useFakeTimers();
    fireEvent.change(screen.getByLabelText('JSON hoặc TOML'), {
      target: { value: '{"mcpServers": {"wiki-search": {"command": "wiki-mcp", "env": {"WIKI_TOKEN": "not-a-real-token"}}}}' },
    });
    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    vi.useRealTimers();
    expect(fake.api.mcpPaste).toHaveBeenLastCalledWith(expect.any(String), false, null);
    expect(screen.getByText('Tìm thấy 1 MCP: wiki-search.')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Tiếp: xem trước' }));
    expect(fake.api.mcpPaste).toHaveBeenLastCalledWith(expect.any(String), true, null);
    expect(screen.getByRole('dialog', { name: 'Thêm wiki-search?' })).toBeTruthy();
    expect(document.body.innerHTML).not.toContain('not-a-real-token');
  });

  it('a server that differs is compared first, keys only as giống nhau', async () => {
    await openTab();
    await press(screen.getByRole('button', { name: 'Sửa design-kit' }));
    const d = screen.getByRole('dialog', { name: 'design-kit khác nhau giữa các agent' });
    expect(within(d).getAllByText('giống nhau')).toHaveLength(2);
    expect(within(d).getByText('--theme dark')).toBeTruthy();
    await press(within(d).getByRole('radio', { name: /Bản của Claude Code/ }));
    await press(within(d).getByRole('button', { name: 'Tiếp: xem trước' }));
    expect(screen.getByRole('dialog', { name: 'Dùng bản của Claude Code cho Antigravity?' })).toBeTruthy();
  });

  it('delete puts the server in the store by default, through the preview', async () => {
    const fake = await openTab();
    await press(screen.getByRole('button', { name: 'Xoá docs-search' }));
    await press(screen.getByRole('button', { name: 'Tiếp: xem trước' }));
    const changes = vi.mocked(fake.api.mcpPreview).mock.calls[0][0];
    expect(changes).toEqual([
      { name: 'docs-search', agent: 'claude', to: { kind: 'absent', store: true } },
      { name: 'docs-search', agent: 'codex', to: { kind: 'absent', store: true } },
    ]);
    expect(screen.getByRole('dialog', { name: 'Cất docs-search vào kho?' })).toBeTruthy();
  });

  it('a key pasted for a server short of one goes straight to the Keychain', async () => {
    const fake = await openTab();
    await press(screen.getByRole('button', { name: 'Dán khoá' }));
    const d = screen.getByRole('dialog', { name: 'Dán khoá cho ticket-desk' });
    expect(within(d).getByText('Lưu ở: Ticket Desk')).toBeTruthy();
    fireEvent.change(within(d).getByLabelText('TICKET_DESK_KEY'), { target: { value: 'not-a-real-key' } });
    await press(within(d).getByRole('button', { name: 'Lưu vào Keychain' }));
    expect(fake.api.mcpSaveSecret).toHaveBeenCalledWith('ticket-desk', 'env:TICKET_DESK_KEY', 'not-a-real-key', null);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(box('ticket-desk cho Codex').getAttribute('aria-disabled')).toBeNull();
  });

  it('Escape closes an MCP dialog', async () => {
    await openTab();
    await press(screen.getByRole('button', { name: 'Bản đã lưu' }));
    expect(screen.getByRole('dialog', { name: 'Bản đã lưu trước khi ghi' })).toBeTruthy();
    expect(screen.getByText('trước khi bật docs-search')).toBeTruthy();
    await key(window, 'Escape');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('shows nothing as a secret: the table and the dialogs hold only masks', async () => {
    await openTab();
    await press(screen.getByRole('button', { name: 'Sửa docs-search' }));
    const d = screen.getByRole('dialog', { name: 'Sửa docs-search' });
    expect((within(d).getByLabelText('Header 1: giá trị') as HTMLInputElement).value).toBe('');
    expect(document.body.innerHTML).not.toMatch(/\bsk-[A-Za-z0-9]{8}|ghp_[A-Za-z0-9]{8}|Bearer [A-Za-z0-9]/);
  });
});
