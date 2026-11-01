// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Plugin tab (M11) on the fake Rust side: nothing here runs `claude` or
// `codex`. Every plugin and marketplace is made up. The page is laid out as
// the approved redraw (plugins-R2): one line under the title with the rest
// behind "Chi tiết", one table of every agent's plugins grouped by agent,
// then Marketplace and the pack's contents side by side.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { pluginsBrandsFixture, pluginsFixture } from '../fixtures/fakeSkills';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

async function openTab(model: Model = fixtureModel('fresh')) {
  const fake = fakeApi(model, 'plugins');
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { level: 2, name: 'Plugin' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

const dialog = () => screen.getByRole('dialog');
const card = (title: string | RegExp) => screen.getByRole('heading', { level: 4, name: title }).closest('[data-slot=card]') as HTMLElement;

afterEach(cleanup);

describe('the Plugin tab', () => {
  it("lists each agent's plugins and marketplaces, read only", async () => {
    const fake = await openTab();
    expect(fake.api.pluginsRead).toHaveBeenCalled();
    const claude = screen.getByRole('list', { name: 'Plugin của Claude Code' });
    expect(within(claude).getAllByRole('listitem').map((r) => r.getAttribute('aria-label'))).toEqual([
      'issue-hub@acme-plugins, bật',
      'formatter@tools-market, bật',
      'docs-pack@acme-plugins, bật',
      'review-kit@tools-market, tắt',
    ]);
    expect(within(claude).getByText('1 MCP, 1 skill, 3 lệnh')).toBeTruthy();
    // No box, no switch: nothing here changes a plugin.
    expect(screen.queryByRole('checkbox')).toBeNull();
    expect(screen.queryByRole('switch')).toBeNull();
    await press(screen.getByRole('button', { name: 'Lệnh cài lại' }));
    expect(within(dialog()).getByText('claude plugin install issue-hub@acme-plugins')).toBeTruthy();
    expect(within(dialog()).getByText('codex plugin add sheets@codex-market')).toBeTruthy();
  });

  it('says it in one line under the title, the rest behind Chi tiết', async () => {
    await openTab();
    expect(screen.getByText(/^Chỉ xem plugin · Khôi phục qua lệnh của agent\./)).toBeTruthy();
    // The long words are not on the page until asked for.
    expect(screen.queryByText(/App không cài, gỡ, bật hay tắt plugin/)).toBeNull();
    await press(screen.getByRole('button', { name: 'Chi tiết: Plugin' }));
    expect(screen.getByText(/App không cài, gỡ, bật hay tắt plugin ở đây/)).toBeTruthy();
    expect(screen.getByText(/Gói sao lưu chỉ ghi tên@marketplace/)).toBeTruthy();
    expect(screen.getByText(/Plugin có sẵn của agent không cần cài lại/)).toBeTruthy();
  });

  it('is one table, grouped by agent under one head row, with Marketplace and the pack side by side', async () => {
    await openTab();
    const table = card('Plugin đã cài');
    // One head row for every agent.
    expect(within(table).getAllByText('Mang theo')).toHaveLength(1);
    expect(within(table).getAllByText('Trạng thái')).toHaveLength(1);
    const groups = within(table).getAllByRole('region');
    expect(groups.map((g) => g.getAttribute('aria-label'))).toEqual(['Claude Code', 'Codex']);
    expect(within(groups[0]).getByRole('heading', { level: 5 }).textContent).toBe('Claude Code· 4');
    expect(within(groups[1]).getByRole('heading', { level: 5 }).textContent).toBe('Codex· 2');
    expect(within(groups[1]).getAllByRole('listitem')).toHaveLength(2);
    // The two cards under the table share one row.
    const markets = card(/^Marketplace/);
    const pack = card('Trong gói sao lưu');
    expect(markets.parentElement).toBe(pack.parentElement);
    expect(within(markets).getAllByRole('listitem')).toHaveLength(4);
    expect(within(pack).getByText('Không ghi')).toBeTruthy();
  });

  it("shows a known service's logo before a plugin, never from its marketplace's name", async () => {
    await openTab(fixtureModel('fresh', { plugins: pluginsBrandsFixture() }));
    const row = (name: string) => screen.getByRole('listitem', { name: name + ', bật' });
    expect(row('github@acme-plugins').querySelector('.logo')).not.toBeNull();
    expect(row('google-calendar@codex-market').querySelector('.logo')).not.toBeNull();
    expect(row('formatter@tools-market').querySelector('.logo')).toBeNull();
  });

  it('without claude: says the pack keeps the old list, and the group counts nothing', async () => {
    const p = pluginsFixture();
    const sides = p.sides!.map((s) => (s.agent === 'claude' ? { ...s, state: 'noCli' as const, plugins: [], marketplaces: [] } : s));
    await openTab(fixtureModel('fresh', { plugins: { ...p, sides } }));
    expect(screen.getByText('Không đọc được plugin của Claude Code')).toBeTruthy();
    expect(screen.getByText(/Gói sao lưu giữ danh sách cũ của Claude Code/)).toBeTruthy();
    const claude = screen.getByRole('region', { name: 'Claude Code' });
    expect(within(claude).getByRole('heading', { level: 5 }).textContent).toBe('Claude Code· ?');
    expect(within(claude).getByText('Chưa đọc được.')).toBeTruthy();
  });
});
