// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Firstmate tab (M6) on fixture data: what Firstmate's quota-pool.last
// says, next to the app's own pool. The file itself is read on the Rust side.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '@/App';
import { fakeApi } from '@/fixtures/fakeApi';
import { firstmateFixture, FIXTURE_NOW, fixtureModel } from '@/fixtures/models';
import type { Api } from '@/lib/api';
import type { FirstmateView, Model } from '@/lib/model';

async function open(firstmate?: FirstmateView, language: Model['language'] = 'auto') {
  const model = fixtureModel('fresh', { ...(firstmate ? { firstmate } : {}), language });
  const fake = fakeApi(model, 'firstmate');
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { name: 'Firstmate' });
  return fake;
}

/** A Firstmate card's lines as `key: value`; none while the card is not there. */
function lines(card: string): string[] {
  const list = screen.queryByRole('list', { name: card });
  if (!list) return [];
  return within(list)
    .getAllByRole('listitem')
    .map((r) => r.children[0].textContent + ': ' + r.children[1].textContent);
}

const seen = () => lines('Firstmate đang thấy');

afterEach(cleanup);

describe('the Firstmate tab', () => {
  it('Firstmate sees room: not holding, with what it last wrote', async () => {
    await open(firstmateFixture());
    expect(screen.getByText('Firstmate thấy đủ hạn mức Claude')).toBeTruthy();
    expect(screen.getByText('Không chờ hạn mức.')).toBeTruthy();
    expect(seen()).toEqual([
      'Chờ hạn mức Claude: không',
      'Tài khoản đang dùng: lan.acme1',
      'Có lại lúc: —',
      'Cập nhật: 14:19 · 1 phút trước',
    ]);
    expect(lines('Đã dùng từng tài khoản')).toEqual(['lanpham.dev: 71%', 'lan.acme1: 100%', 'lan.acme2: 0%', 'lan.acme3: 2%']);
    expect(lines('Kết nối')).toEqual(['quota-axi của firstmate có +cswap: có', 'Đọc được claude-swap: được']);
  });

  it('three cards side by side, one question each, and the note under them on the glass', async () => {
    await open(firstmateFixture());
    const cards = ['Firstmate đang thấy', 'Đã dùng từng tài khoản', 'Kết nối'].map((n) => screen.getByRole('list', { name: n }).closest('[data-slot=card]')!);
    // One row of three: the same grid holds all of them.
    expect(new Set(cards.map((c) => c.parentElement)).size).toBe(1);
    expect(cards[0].parentElement!.children).toHaveLength(3);
    // An account used up reads red; the others stay plain.
    const used = within(screen.getByRole('list', { name: 'Đã dùng từng tài khoản' })).getAllByRole('listitem');
    expect(used[1].children[1].className).toMatch(/text-bad-ink/);
    expect(used[0].children[1].className).not.toMatch(/text-bad-ink/);
    // The note is not in a card.
    const note = screen.getByText(/^Firstmate ghi file này sau mỗi lần kiểm/);
    expect(note.closest('[data-slot=card]')).toBeNull();
  });

  it('pool=dry: Firstmate holds new Claude work until the pool has room', async () => {
    const line = { ...firstmateFixture().line!, dry: true, nextReset: '2026-01-16T09:40:00Z' };
    await open(firstmateFixture({ line, ownDry: true }));
    expect(screen.getByText('Firstmate đang chờ hạn mức Claude')).toBeTruthy();
    expect(screen.getByText('Hạn mức trong vòng đã cạn: firstmate giữ việc Claude mới đến khi có lại. Có lại lúc 16:40.')).toBeTruthy();
    expect(seen()).toContain('Chờ hạn mức Claude: đang chờ');
  });

  it('a file not written for long says Firstmate may not run', async () => {
    await open(firstmateFixture({ writtenAt: '2026-01-16T05:20:00Z', fresh: false }));
    expect(screen.getByText('Thông tin firstmate chưa cập nhật trong 2 giờ')).toBeTruthy();
    expect(screen.getByText('File của firstmate chưa được ghi lại từ 12:20. Firstmate có thể không chạy.')).toBeTruthy();
  });

  it('when the app and Firstmate disagree, it says why', async () => {
    await open(firstmateFixture({ ownDry: true }));
    expect(screen.getByText(/^App tính từ số của nó lúc 14:20 thì hạn mức trong vòng đã cạn\./)).toBeTruthy();
  });

  it('without a file set, it points to Cài đặt', async () => {
    await open();
    expect(screen.getByText('Chưa nối với firstmate')).toBeTruthy();
    expect(seen()).toEqual([]);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở Cài đặt' }));
    });
    expect(screen.getByRole('heading', { name: 'Cài đặt' })).toBeTruthy();
    // On Nâng cao, where Firstmate's file is set.
    expect(screen.getByRole('tab', { name: 'Nâng cao' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByLabelText(/^File của firstmate/)).toBeTruthy();
  });

  it('a file that is not there', async () => {
    await open(firstmateFixture({ status: 'missing', line: null, writtenAt: null, fresh: false }));
    expect(screen.getByText('Không thấy firstmate trên máy này')).toBeTruthy();
    expect(screen.getByText('Không có file ~/example/quota-pool.last. Luật "cạn" vẫn tự tính từ số của app để báo.')).toBeTruthy();
  });
});

describe('the Firstmate tab in English', () => {
  it('has no Vietnamese left: the button and the card title', async () => {
    await open(firstmateFixture(), 'en');
    expect(screen.getByRole('heading', { name: 'What Firstmate sees' })).toBeTruthy();
    cleanup();
    await open(undefined, 'en');
    expect(screen.getByRole('button', { name: 'Open Settings' })).toBeTruthy();
  });
});
