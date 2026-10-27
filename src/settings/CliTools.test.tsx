// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cài đặt › Chung › Cập nhật Agent on the fake Rust side: the rows, "Cập
// nhật", "Kiểm ngay", the daily check, and the bell's card leading there.
// Nothing here runs a tool or reaches a feed.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { cliToolsFixture, CLI_STATES } from '../fixtures/fakeCliTools';
import { alertsFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { CliToolsView, Model, Tab } from '../lib/model';

async function openTab(tab: Tab, model: Model, options: FakeOptions = {}) {
  const fake = fakeApi(model, tab, options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Các tab' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const withCli = (cliTools: CliToolsView = cliToolsFixture(), over: Partial<Model> = {}) => fixtureModel('fresh', { cliTools, ...over });
const card = () => screen.getByRole('region', { name: 'Cập nhật Agent' });
const row = (name: string) => within(card()).getByText(name).closest<HTMLElement>('.srow')!;

afterEach(cleanup);

describe('Cài đặt › Chung › Cập nhật Agent', () => {
  it('sits under Cập nhật in Chung, a row per tool, with no automatic update', async () => {
    await openTab('settings', withCli());
    const titles = [...document.querySelectorAll('.main [data-slot=card] h4')].map((h) => h.textContent);
    expect(titles.indexOf('Cập nhật Agent')).toBe(titles.indexOf('Cập nhật') + 1);
    expect(within(row('Claude Code')).getByText('mới nhất')).toBeTruthy();
    expect(within(row('Codex CLI')).getByText(/0\.163\.0 → 0\.164\.0 · Homebrew/)).toBeTruthy();
    expect(within(row('Antigravity CLI')).getByRole('button', { name: 'Cập nhật' })).toBeTruthy();
    expect(within(card()).getByText('Kiểm bản mới')).toBeTruthy();
    expect(within(card()).queryByText('Tự cập nhật')).toBeNull();
  });

  it('is not shown on Windows, whose installs it does not read yet', async () => {
    const model = withCli();
    await openTab('settings', { ...model, capabilities: { ...model.capabilities, platform: 'windows' } });
    expect(screen.queryByRole('region', { name: 'Cập nhật Agent' })).toBeNull();
  });

  it('"Cập nhật" runs that tool’s update and says how it went', async () => {
    const fake = await openTab('settings', withCli());
    await press(within(row('Codex CLI')).getByRole('button', { name: 'Cập nhật' }));
    expect(fake.api.cliToolUpdate).toHaveBeenCalledWith('codex');
    expect(within(row('Codex CLI')).getByText('Đã cập nhật 0.163.0 → 0.164.0. Phiên mở sau dùng bản mới.')).toBeTruthy();
  });

  it('a failed update says why and where its log is', async () => {
    await openTab('settings', withCli(), { cliFails: { agy: { failure: 'busy', detail: '' } } });
    await press(within(row('Antigravity CLI')).getByRole('button', { name: 'Cập nhật' }));
    expect(within(row('Antigravity CLI')).getByText('Công cụ đang tự cập nhật ở nền. Đợi một phút rồi thử lại.')).toBeTruthy();
  });

  it('Codex with a session open offers "Kiểm lại" instead, which looks again', async () => {
    const fake = await openTab('settings', withCli(CLI_STATES.running(cliToolsFixture())), { cliClosed: true });
    const codex = row('Codex CLI');
    expect(within(codex).queryByRole('button', { name: 'Cập nhật' })).toBeNull();
    await press(within(codex).getByRole('button', { name: 'Kiểm lại' }));
    expect(fake.api.cliToolsCheck).toHaveBeenCalledTimes(1);
    expect(within(row('Codex CLI')).getByRole('button', { name: 'Cập nhật' })).toBeTruthy();
    // Claude Code updates with its sessions open.
    expect(within(row('Claude Code')).getByRole('button', { name: 'Cập nhật' })).toBeTruthy();
  });

  it('an install left to the owner offers its command to copy, not "Cập nhật"', async () => {
    await openTab('settings', withCli(CLI_STATES.npm(cliToolsFixture())));
    const claude = row('Claude Code');
    expect(within(claude).getByText('chạy trong Terminal')).toBeTruthy();
    expect(within(claude).getByRole('button', { name: 'Chép lệnh' })).toBeTruthy();
    expect(within(claude).queryByRole('button', { name: 'Cập nhật' })).toBeNull();
  });

  it('"Kiểm ngay" asks for the newest versions, and "Tự kiểm mỗi ngày" turns off', async () => {
    const fake = await openTab('settings', withCli());
    await press(within(card()).getByRole('button', { name: 'Kiểm ngay' }));
    expect(fake.api.cliToolsCheck).toHaveBeenCalledTimes(1);
    await press(within(card()).getByRole('switch', { name: 'Tự kiểm mỗi ngày' }));
    expect(fake.api.setCliChecks).toHaveBeenCalledWith(false);
  });

  it('the bell’s card opens Cài đặt at Chung', async () => {
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
    expect(await screen.findByRole('region', { name: 'Cập nhật Agent' })).toBeTruthy();
  });
});
