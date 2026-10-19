// The Lịch sử tab (M6) on the fake Rust side: synthetic lines from the
// app's journal, the switcher's log and the readings, a day at a time.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, hideNames } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel, historyFixture } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { HistoryEntry } from '../lib/model';

const ENTRIES = historyFixture();

async function open(history: HistoryEntry[], names = true) {
  const model = fixtureModel('fresh');
  const fake = fakeApi(names ? model : hideNames(model), 'history', { history });
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { name: 'Lịch sử' });
  return fake;
}

const rows = (day: string) =>
  Array.from(screen.getByRole('region', { name: day }).querySelectorAll('.hrow')).map((r) =>
    Array.from(r.children)
      .map((c) => c.textContent)
      .join(' | '),
  );

afterEach(cleanup);

describe('the Lịch sử tab', () => {
  it('lists every switch a day at a time, with why and where it came from', async () => {
    const fake = await open(ENTRIES);
    await screen.findByText('Hôm nay');
    expect(fake.api.getHistory).toHaveBeenCalled();
    expect(rows('Hôm nay')).toEqual([
      '14:12 | Chuyển | lan.acme2→lan.acme1Bạn chuyển tay · giữ đến 15:20 | app',
      '13:50 | Codex | ChatGPT→relayBạn chuyển trong app · giữ đăng nhập ChatGPT · đã khởi động lại tiến trình nền | app',
      '13:40 | Cảnh báo | lan.ultra2→lanpham.ultraAntigravity.app đổi về tài khoản cũ (lần 1/2) | quan sát',
      '13:20 | Antigravity | lanpham.ultra→lan.ultra2Bạn chuyển trong app · thoát app rồi chuyển | app',
      '09:12 | Chuyển | lan.acme25 giờ đã dùng 4% · đặt lại 15:00 | switcher',
      '08:01 | Mồi | lan.acme2Bắt đầu tính 5 giờ sớm | switcher',
    ]);
    expect(rows('Hôm qua')).toEqual(['22:47 | Chuyển | lan.acme1→lanpham.devĐổi giữa hai lần đọc, không do app hay switcher | quan sát']);
    // No line of explanation, no exit code; the switcher's own line is on hover only.
    expect(screen.queryByText(/^Lịch sử của/)).toBeNull();
    expect(screen.queryByText(/exit 0/)).toBeNull();
    const primed = screen.getByRole('region', { name: 'Hôm nay' }).querySelectorAll('.hrow')[5];
    expect(primed.getAttribute('title')).toMatch(/exit 0/);
    expect(primed.className).toBe('hrow r3');
  });

  it('goes by place when names are hidden', async () => {
    await open(ENTRIES, false);
    await screen.findByText('Hôm nay');
    expect(rows('Hôm nay')[0]).toBe('14:12 | Chuyển | Team B→Team ABạn chuyển tay · giữ đến 15:20 | app');
    expect(rows('Hôm nay')[2]).toMatch(/^13:40 \| Cảnh báo \| Ultra 2→Ultra 1/);
    expect(document.body.textContent).not.toMatch(/@|lan\.|Acme/);
  });

  it('filters by app', async () => {
    await open(ENTRIES);
    await screen.findByText('Hôm nay');
    const filter = screen.getByRole('group', { name: 'Lọc lịch sử' });
    expect(within(filter).getByRole('button', { name: 'Tất cả' }).getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(within(filter).getByRole('button', { name: 'Antigravity' }));
    expect(rows('Hôm nay')).toHaveLength(2);
    expect(screen.queryByRole('region', { name: 'Hôm qua' })).toBeNull();
    fireEvent.click(within(filter).getByRole('button', { name: 'Claude' }));
    expect(rows('Hôm nay')).toHaveLength(3);
    expect(rows('Hôm qua')).toHaveLength(1);
    fireEvent.click(within(filter).getByRole('button', { name: 'Codex' }));
    expect(rows('Hôm nay')).toEqual([
      '13:50 | Codex | ChatGPT→relayBạn chuyển trong app · giữ đăng nhập ChatGPT · đã khởi động lại tiến trình nền | app',
    ]);
  });

  it('says when there is nothing yet, and when only the filter leaves nothing', async () => {
    await open([]);
    expect(await screen.findByText('Chưa có lần chuyển nào')).toBeTruthy();
    cleanup();
    await open(ENTRIES.filter((e) => e.app === 'claude'));
    await screen.findByText('Hôm nay');
    fireEvent.click(within(screen.getByRole('group', { name: 'Lọc lịch sử' })).getByRole('button', { name: 'Antigravity' }));
    expect(screen.getByText('Chưa có lần chuyển Antigravity nào')).toBeTruthy();
    expect(screen.queryByText('Chưa có lần chuyển nào')).toBeNull();
  });

  it('opened while a switch runs, it still reads the list', async () => {
    const fake = fakeApi(fixtureModel('fresh', { claudeSwitch: { cswap: true, busy: 3 } }), 'history', { history: ENTRIES });
    vi.spyOn(fake.api, 'getHistory');
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    expect(await screen.findByText('Hôm nay')).toBeTruthy();
    expect(fake.api.getHistory).toHaveBeenCalled();
  });

  it('reads the list again after the next read', async () => {
    const fake = await open([]);
    await screen.findByText('Chưa có lần chuyển nào');
    vi.mocked(fake.api.getHistory).mockResolvedValue(ENTRIES.slice(0, 1));
    await act(async () => {
      fake.push({ ...fake.current(), reads: fake.current().reads + 1 });
    });
    expect(rows('Hôm nay')).toHaveLength(1);
  });
});
