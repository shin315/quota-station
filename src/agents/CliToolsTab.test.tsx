// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Agent › Cập nhật (Cập nhật Agent) on the fake Rust side: its place in the
// sidebar, the line at the top, the rows, "Cập nhật", "Kiểm tra ngay", the
// check when the page opens, "Tự động kiểm tra", and the bell's card leading
// there. Nothing here runs a tool or reaches a feed.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { cliToolsFixture, CLI_STATES } from '../fixtures/fakeCliTools';
import { alertsFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { CliToolsView, Model, Tab } from '../lib/model';

async function openTab(tab: Tab, model: Model, options: FakeOptions = {}) {
  const fake = fakeApi(model, tab, { clock: () => FIXTURE_NOW, ...options });
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: /^(Các tab|Tabs)$/ });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const withCli = (cliTools: CliToolsView = cliToolsFixture(), over: Partial<Model> = {}) => fixtureModel('fresh', { cliTools, ...over });
const tools = () => screen.getByRole('region', { name: 'Công cụ' });
const checks = () => screen.getByRole('region', { name: 'Kiểm tra bản mới' });
/** A row's one line under its name. */
const line = (name: string) => row(name).querySelector('[data-tone], .tabular-nums > span')!.textContent;
const summary = () => screen.getByRole('status', { name: 'Tóm tắt' });
const row = (name: string) => within(tools()).getByText(name).closest<HTMLElement>('.srow')!;
const sidebar = () => screen.getByRole('navigation', { name: /^(Các tab|Tabs)$/ });

afterEach(cleanup);

describe('Agent › Cập nhật', () => {
  it('closes the Agent group in the sidebar and opens the Cập nhật Agent page', async () => {
    await openTab('overview', withCli());
    const items = [...sidebar().children].map((el) => el.textContent);
    expect(items.indexOf('Cập nhật')).toBe(items.indexOf('Bộ cấu hình') + 1);
    expect(items.indexOf('Cập nhật')).toBeGreaterThan(items.indexOf('Agent'));
    expect(items.indexOf('Cập nhật')).toBeLessThan(items.indexOf('Sao lưu'));
    await press(within(sidebar()).getByRole('button', { name: 'Cập nhật' }));
    expect(screen.getByRole('heading', { level: 2, name: 'Cập nhật Agent' })).toBeTruthy();
  });

  it('is no longer a card in Cài đặt › Chung', async () => {
    await openTab('settings', withCli());
    const titles = [...document.querySelectorAll('.main [data-slot=card] h4')].map((h) => h.textContent);
    expect(titles).toContain('Cập nhật');
    expect(titles).not.toContain('Cập nhật Agent');
    expect(screen.queryByText('Claude Code')).toBeNull();
  });

  it('says at the top whether the tools are the newest, then a row each: its name and one line', async () => {
    await openTab('cliTools', withCli());
    expect(summary().textContent).toBe('2 công cụ có bản mới· kiểm tra lúc 14:02');
    expect(line('Claude Code')).toBe('2.1.311 · Đã là bản mới nhất');
    expect(line('Codex CLI')).toBe('0.163.0 → 0.164.0 · Có bản mới');
    expect(within(row('Antigravity CLI')).getByRole('button', { name: 'Cập nhật' })).toBeTruthy();
    // How it was installed and when it was checked are behind Chi tiết.
    expect(screen.queryByText(/Homebrew/)).toBeNull();
    await press(within(row('Codex CLI')).getByRole('button', { name: 'Chi tiết: Codex CLI' }));
    expect(screen.getByText('Cài bằng Homebrew.')).toBeTruthy();
    expect(screen.getByText('Kiểm tra lúc 14:02.')).toBeTruthy();
    expect(screen.queryByText('Tự cập nhật')).toBeNull();
  });

  it('says plainly when every tool is the newest', async () => {
    await openTab('cliTools', withCli(CLI_STATES.latest(cliToolsFixture())));
    expect(summary().textContent).toBe('Tất cả đã là bản mới nhất· kiểm tra lúc 14:02');
    for (const name of ['Claude Code', 'Codex CLI', 'Antigravity CLI']) expect(line(name)).toMatch(/ · Đã là bản mới nhất$/);
    expect(within(tools()).queryByRole('button', { name: 'Cập nhật' })).toBeNull();
  });

  it('says which tool could not be checked, and why', async () => {
    await openTab('cliTools', withCli(CLI_STATES.failed(CLI_STATES.latest(cliToolsFixture()))));
    expect(summary().textContent).toBe('Không kiểm tra được Codex CLI· kiểm tra lúc 14:02');
    expect(line('Codex CLI')).toBe('0.164.0 · Không kiểm tra được: máy chủ trả lỗi 503');
  });

  // The Mac mini M6 at ca353c5: no check had ever been kept, and the page said only "Chưa kiểm lần nào.".
  it('opening it with no check kept checks at once and shows the result', async () => {
    const fake = await openTab('cliTools', withCli(CLI_STATES.never(cliToolsFixture())));
    expect(fake.api.cliToolsOpened).toHaveBeenCalledTimes(1);
    expect(fake.api.cliToolsCheck).toHaveBeenCalledTimes(1);
    expect(screen.queryByText('Chưa kiểm tra lần nào.')).toBeNull();
    expect(summary().textContent).toMatch(/· kiểm tra lúc 14:20$/);
  });

  it('opening it after a check today asks nothing, and with the daily check off neither', async () => {
    const fake = await openTab('cliTools', withCli());
    expect(fake.api.cliToolsOpened).toHaveBeenCalledTimes(1);
    expect(fake.api.cliToolsCheck).not.toHaveBeenCalled();
    cleanup();
    const off = await openTab('cliTools', withCli(CLI_STATES.never({ ...cliToolsFixture(), daily: false })));
    expect(off.api.cliToolsCheck).not.toHaveBeenCalled();
    expect(summary().textContent).toBe('Chưa kiểm tra lần nào.');
  });

  it('a check running says so at the top and on every row', async () => {
    await openTab('cliTools', withCli(CLI_STATES.checking(cliToolsFixture())));
    expect(summary().textContent).toBe('Đang kiểm tra bản mới…');
    expect(line('Codex CLI')).toBe('0.163.0 → 0.164.0 · Đang kiểm tra…');
    expect(screen.getByRole('button', { name: 'Đang kiểm tra…' })).toHaveProperty('disabled', true);
  });

  it('"Cập nhật" runs that tool’s update and says how it went', async () => {
    const fake = await openTab('cliTools', withCli());
    await press(within(row('Codex CLI')).getByRole('button', { name: 'Cập nhật' }));
    expect(fake.api.cliToolUpdate).toHaveBeenCalledWith('codex');
    expect(line('Codex CLI')).toBe('0.164.0 · Đã cập nhật từ 0.163.0');
  });

  it('a failed update says why and where its log is', async () => {
    await openTab('cliTools', withCli(), { cliFails: { agy: { failure: 'busy', detail: '' } } });
    await press(within(row('Antigravity CLI')).getByRole('button', { name: 'Cập nhật' }));
    expect(line('Antigravity CLI')).toBe('1.4.7 → 1.4.8 · Cập nhật không xong');
    await press(within(row('Antigravity CLI')).getByRole('button', { name: 'Chi tiết: Antigravity CLI' }));
    expect(screen.getByText('Công cụ đang tự cập nhật ở nền. Đợi một phút rồi thử lại.')).toBeTruthy();
    expect(screen.getByText(/cli-updates\/agy\.log/)).toBeTruthy();
  });

  it('Codex with a session open offers "Kiểm tra lại" instead, which looks again', async () => {
    const fake = await openTab('cliTools', withCli(CLI_STATES.running(cliToolsFixture())), { cliClosed: true });
    const codex = row('Codex CLI');
    expect(within(codex).queryByRole('button', { name: 'Cập nhật' })).toBeNull();
    await press(within(codex).getByRole('button', { name: 'Kiểm tra lại' }));
    expect(fake.api.cliToolsCheck).toHaveBeenCalledTimes(1);
    expect(within(row('Codex CLI')).getByRole('button', { name: 'Cập nhật' })).toBeTruthy();
    // Claude Code updates with its sessions open.
    expect(within(row('Claude Code')).getByRole('button', { name: 'Cập nhật' })).toBeTruthy();
  });

  it('an install left to the owner offers its command to copy, not "Cập nhật"', async () => {
    await openTab('cliTools', withCli(CLI_STATES.npm(cliToolsFixture())));
    const claude = row('Claude Code');
    expect(line('Claude Code')).toBe('2.1.311 → 2.1.312 · Cập nhật trong Terminal');
    expect(within(claude).queryByRole('button', { name: 'Cập nhật' })).toBeNull();
    await press(within(claude).getByRole('button', { name: 'Chi tiết: Claude Code' }));
    expect(screen.getByRole('button', { name: 'Chép lệnh' })).toBeTruthy();
    // Named in the why, and given to copy.
    expect(screen.getAllByText('claude install')).toHaveLength(2);
  });

  it('"Kiểm tra ngay" asks for the newest versions, and "Tự động kiểm tra" turns off', async () => {
    const fake = await openTab('cliTools', withCli());
    await press(screen.getByRole('button', { name: 'Kiểm tra ngay' }));
    expect(fake.api.cliToolsCheck).toHaveBeenCalledTimes(1);
    expect(within(checks()).getByText(/^Mỗi ngày, chỉ báo ở chuông\./)).toBeTruthy();
    await press(within(checks()).getByRole('switch', { name: 'Tự động kiểm tra' }));
    expect(fake.api.setCliChecks).toHaveBeenCalledWith(false);
  });

  it('is not in the sidebar on Windows, whose installs it does not read yet', async () => {
    const model = withCli();
    await openTab('overview', { ...model, capabilities: { ...model.capabilities, platform: 'windows' } });
    expect(within(sidebar()).queryByRole('button', { name: 'Cập nhật' })).toBeNull();
  });

  it('the bell’s card opens the page', async () => {
    const alert = {
      key: 'cli:codex@0.164.0',
      kind: 'cli' as const,
      tone: 'info' as const,
      title: 'Codex CLI có bản 0.164.0',
      body: 'Codex CLI 0.163.0 → 0.164.0. App chỉ cập nhật khi bạn bấm.',
      actions: [{ id: 'cli:settings', label: 'Xem' }, { id: 'dismiss', label: 'Để sau' }],
      at: 1768540000,
    };
    const recent = [{ id: 'cli:codex@0.164.0@1768540000', alert, notified: false, answer: null, read: false }];
    const fake = await openTab('overview', withCli(cliToolsFixture(), { alerts: alertsFixture({ recent, unanswered: 1, unread: 1 }) }));
    await press(screen.getByRole('button', { name: /Thông báo/ }));
    await press(screen.getByRole('button', { name: 'Xem' }));
    expect(fake.api.alertAction).toHaveBeenCalledWith('cli:codex@0.164.0@1768540000', 'cli:settings');
    expect(await screen.findByRole('heading', { level: 2, name: 'Cập nhật Agent' })).toBeTruthy();
    expect(within(sidebar()).getByRole('button', { name: 'Cập nhật' }).getAttribute('aria-current')).toBe('page');
  });
});

describe('Agent › Cập nhật in English', () => {
  it('is Updates in the sidebar, and its page says plainly what is the newest', async () => {
    await openTab('cliTools', withCli(cliToolsFixture(), { language: 'en' }));
    expect(within(sidebar()).getByRole('button', { name: 'Updates' }).getAttribute('aria-current')).toBe('page');
    expect(screen.getByRole('heading', { level: 2, name: 'Agent updates' })).toBeTruthy();
    expect(screen.getByRole('status', { name: 'Summary' }).textContent).toBe('2 tools have a new version· checked at 14:02');
    const claude = within(screen.getByRole('region', { name: 'Tools' })).getByText('Claude Code').closest<HTMLElement>('.srow')!;
    expect(within(claude).getByText('2.1.311 · Already the newest')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Check now' })).toBeTruthy();
    const checking = screen.getByRole('region', { name: 'Check for new versions' });
    expect(within(checking).getByRole('switch', { name: 'Check automatically' })).toBeTruthy();
  });
});
