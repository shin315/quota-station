// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Thống kê tab (M16) on the fake Rust side: no session log is read.
// Every project, account and number is made up; `stats-trap.json` is what
// the Rust side counted from fixture logs whose conversation carries a
// planted text.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { firstReadFixture, SKIPPED_CODEX, statsFixture, statsReport } from '../fixtures/fakeStats';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import trap from '../fixtures/stats-trap.json';
import type { Api } from '../lib/api';
import type { Model, StatsReport } from '../lib/model';
import { num, tokensText, unitOf, usd } from '../lib/stats';

async function openTab(model: Model = fixtureModel('fresh'), options: FakeOptions = {}) {
  const fake = fakeApi(model, 'stats', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { level: 2, name: 'Thống kê' });
  await act(async () => {});
  return fake;
}

async function press(el: Element) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const chart = () => screen.getByRole('group', { name: /^Token mỗi/ });
const bars = () => within(chart()).getAllByRole('img');

afterEach(cleanup);

describe('the Thống kê tab', () => {
  it('follows Tổng quan in the sidebar and reads the logs when it opens', async () => {
    const fake = await openTab();
    const nav = screen.getByRole('navigation', { name: 'Các tab' });
    const items = within(nav).getAllByRole('button').map((b) => b.textContent);
    expect(items.slice(0, 2)).toEqual(['Tổng quan', 'Thống kê']);
    expect(within(nav).getByRole('button', { name: 'Thống kê' }).getAttribute('aria-current')).toBe('page');
    expect(fake.api.statsRead).toHaveBeenCalled();
    expect(fake.api.statsReport).toHaveBeenCalledWith('week');
    expect(screen.getByText('Đếm token từ log phiên trên máy này. Không đọc nội dung, không gửi đi.')).toBeTruthy();
  });

  it('shows the totals, the agents and the API price for comparison only', async () => {
    await openTab();
    const r = statsReport('week');
    expect(screen.getByText('Tổng · 7 ngày')).toBeTruthy();
    expect(screen.getByText('Giá API tương đương')).toBeTruthy();
    expect(screen.getByText('gói thuê bao: chỉ để so sánh')).toBeTruthy();
    expect(screen.getByText(usd(r.usd))).toBeTruthy();
    // Antigravity has no row of numbers.
    expect(screen.getByText('chưa có log token trên máy')).toBeTruthy();
    // The accounts by the window's names, approximate.
    expect(screen.getByText(/Gần đúng: ghép giờ của phiên/)).toBeTruthy();
    const accounts = screen.getByRole('table', { name: 'Theo tài khoản Claude' });
    expect(within(accounts).getAllByRole('row')).toHaveLength(3);
    // Projects and models.
    expect(within(screen.getByRole('table', { name: 'Theo dự án' })).getByText('web-shop').closest('th')!.getAttribute('title')).toBe('~/Projects/web-shop');
    expect(within(screen.getByRole('table', { name: 'Theo model' })).getByText('claude-opus-5-5')).toBeTruthy();
    expect(screen.getByText(/bảng giá API ngày 19\/10\/2026/)).toBeTruthy();
  });

  it('draws one bar a day in one colour, with a number on the tallest only', async () => {
    await openTab();
    expect(bars()).toHaveLength(7);
    const paths = chart().querySelectorAll('path');
    expect(paths).toHaveLength(7);
    for (const p of paths) expect(p.getAttribute('class')).toContain('fill-series-1');
    // Bars at most 24 wide, the top corners rounded by 4.
    for (const p of paths) {
      const d = p.getAttribute('d')!;
      const xs = [...d.matchAll(/[MH]([\d.]+)/g)].map((m) => Number(m[1]));
      expect(Math.max(...xs) - Math.min(...xs) + 4).toBeLessThanOrEqual(24.001);
      expect(d).toMatch(/Q/);
    }
    const r = statsReport('week');
    const max = Math.max(...r.points.map((p) => p.claude + p.codex));
    const u = unitOf(max);
    const labels = [...chart().querySelectorAll('text')].map((t) => t.textContent);
    const top = num(max / u.div, 1) + ' ' + u.short;
    expect(labels.filter((t) => t === top)).toHaveLength(1);
  });

  it('gives each bar on hover and from the keyboard, one Tab stop for the chart', async () => {
    await openTab();
    const all = bars();
    expect(all.filter((b) => b.getAttribute('tabindex') === '0')).toHaveLength(1);
    expect(all[6].getAttribute('tabindex')).toBe('0');
    const r = statsReport('week');
    expect(all[6].getAttribute('aria-label')).toBe('Ngày 16/01: ' + tokensText(r.points[6].claude + r.points[6].codex) + ', Claude Code ' + tokensText(r.points[6].claude) + ', Codex ' + tokensText(r.points[6].codex));
    fireEvent.pointerEnter(all[1]);
    expect(screen.getByTestId('chart-tip').textContent).toContain('Ngày 11/01');
    fireEvent.pointerLeave(all[1]);
    expect(screen.queryByTestId('chart-tip')).toBeNull();
    act(() => all[6].focus());
    expect(screen.getByTestId('chart-tip').textContent).toContain('Ngày 16/01');
    fireEvent.keyDown(all[6], { key: 'ArrowLeft' });
    expect(document.activeElement).toBe(all[5]);
    expect(all[5].getAttribute('tabindex')).toBe('0');
    expect(screen.getByTestId('chart-tip').textContent).toContain('Ngày 15/01');
    fireEvent.keyDown(all[5], { key: 'Home' });
    expect(document.activeElement).toBe(all[0]);
    fireEvent.keyDown(all[0], { key: 'End' });
    expect(document.activeElement).toBe(all[6]);
  });

  it('has every number of the chart in its table', async () => {
    await openTab();
    await press(screen.getByRole('button', { name: 'Xem bảng' }));
    const r = statsReport('week');
    const u = unitOf(Math.max(...r.points.map((p) => p.claude + p.codex)));
    const table = screen.getByRole('table', { name: 'Token mỗi ngày' });
    const rows = within(table).getAllByRole('row').slice(1);
    expect(rows).toHaveLength(8);
    r.points.forEach((p, i) => {
      const cells = within(rows[i]).getAllByRole('cell').map((c) => c.textContent);
      expect(cells).toEqual([p.label, num(p.claude / u.div), num(p.codex / u.div), num((p.claude + p.codex) / u.div)]);
    });
    expect(within(rows[7]).getAllByRole('cell')[0].textContent).toBe('Cộng');
    await press(screen.getByRole('button', { name: 'Xem biểu đồ' }));
    expect(chart()).toBeTruthy();
  });

  it('shows 30 days with labels five apart, and today by hour', async () => {
    const fake = await openTab();
    await press(screen.getByRole('button', { name: '30 ngày' }));
    expect(fake.api.statsReport).toHaveBeenLastCalledWith('month');
    expect(bars()).toHaveLength(30);
    expect(screen.getByText('Tổng · 30 ngày')).toBeTruthy();
    const axis = [...chart().querySelectorAll('text')].map((t) => t.textContent).filter((t) => /\/\d\d$/.test(t ?? ''));
    expect(axis).toEqual(['18/12', '23/12', '28/12', '02/01', '07/01', '12/01']);
    await press(screen.getByRole('button', { name: 'Hôm nay' }));
    expect(fake.api.statsReport).toHaveBeenLastCalledWith('today');
    expect(screen.getByRole('group', { name: 'Token mỗi giờ' })).toBeTruthy();
    expect(bars()).toHaveLength(24);
    expect(bars()[14].getAttribute('aria-label')).toMatch(/^Giờ 14:00: /);
  });

  it('shows each agent progress on the first read, then the numbers', async () => {
    await openTab(fixtureModel('fresh', { stats: statsFixture({ readAt: null }) }), { statsStepMs: 40 });
    expect(await screen.findByText(/Claude Code · \d/)).toBeTruthy();
    expect(screen.getByText('Đọc lần đầu mất vài phút. Lần sau chỉ đọc phần mới.')).toBeTruthy();
    expect(screen.getByText('Antigravity · chưa có log token trên máy')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Đọc lại' }).hasAttribute('disabled')).toBe(true);
    expect(await screen.findByText('Tổng · 7 ngày', {}, { timeout: 4000 })).toBeTruthy();
    expect(screen.getByText(/đọc log lúc/)).toBeTruthy();
  });

  it('draws the first read halfway as the prototype', async () => {
    await openTab(fixtureModel('fresh', { stats: firstReadFixture() }));
    expect(screen.getByText('Claude Code · 2.140 file log')).toBeTruthy();
    expect(screen.getByText('Codex · 640/960 file')).toBeTruthy();
    expect(screen.getByText(/đang đọc log…/)).toBeTruthy();
  });

  it('says how many sessions it skipped, never guessing them, until "Đã biết"', async () => {
    const fake = await openTab(fixtureModel('fresh'), { statsSkipped: SKIPPED_CODEX });
    expect(screen.getByText('Bỏ qua 38 phiên Codex')).toBeTruthy();
    expect(screen.getByText('Log của Codex 0.170.0 có dạng app chưa đọc được. Số Codex từ 14/01 thiếu các phiên này.')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Đã biết' }));
    expect(fake.api.statsAck).toHaveBeenCalled();
    expect(screen.queryByText('Bỏ qua 38 phiên Codex')).toBeNull();
  });

  it('says when the report cannot be read', async () => {
    await openTab(fixtureModel('fresh'), { statsFails: true });
    expect(screen.getByText('Chưa đọc được số')).toBeTruthy();
  });
});

/** The trap: `stats-trap.json` is the report the Rust side counted from logs carrying a planted text. */
describe('a planted conversation text', () => {
  const TRAP = 'TRAP-conversation';

  it('is not in what Rust sends the window', () => {
    expect(JSON.stringify(trap)).not.toContain(TRAP);
    expect((trap as unknown as StatsReport).total).toBeGreaterThan(0);
  });

  it('never reaches the tab, its table or its hover cards', async () => {
    const fake = await openTab();
    vi.mocked(fake.api.statsReport).mockImplementation(async () => trap as unknown as StatsReport);
    await press(screen.getByRole('button', { name: 'Hôm nay' }));
    await press(screen.getByRole('button', { name: 'Xem bảng' }));
    expect(screen.getByText('gpt-5.1-codex-max')).toBeTruthy();
    expect(document.body.innerHTML).not.toContain(TRAP);
    await press(screen.getByRole('button', { name: 'Xem biểu đồ' }));
    for (const b of bars()) fireEvent.pointerEnter(b);
    expect(document.body.innerHTML).not.toContain(TRAP);
  });
});
