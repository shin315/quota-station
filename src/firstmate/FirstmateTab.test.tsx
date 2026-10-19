// The Firstmate tab (M6) on fixture data: what Firstmate's quota-pool.last
// says, next to quota-axi's pool. The file itself is read on the Rust side.

import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { firstmateFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { FirstmateView } from '../lib/model';

async function open(firstmate?: FirstmateView) {
  const model = fixtureModel('fresh', firstmate ? { firstmate } : {});
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

/** The "Firstmate đang thấy" rows as `key: value`. */
function seen(): string[] {
  const kvt = document.querySelector('.kvt');
  if (!kvt) return [];
  const cells = Array.from(kvt.children).map((c) => c.textContent);
  return cells.flatMap((c, i) => (i % 2 ? [] : [c + ': ' + cells[i + 1]]));
}

afterEach(cleanup);

describe('the Firstmate tab', () => {
  it('Firstmate sees room: not holding, with what it last wrote', async () => {
    await open(firstmateFixture());
    expect(screen.getByText('Firstmate thấy đủ hạn mức Claude')).toBeTruthy();
    expect(screen.getByText('Không chờ hạn mức.')).toBeTruthy();
    expect(seen()).toEqual([
      'Chờ hạn mức Claude: không',
      'Tài khoản đang dùng: lan.acme1',
      'Đã dùng từng tài khoản: lanpham.dev đã dùng 71% · lan.acme1 đã dùng 100% · lan.acme2 đã dùng 0% · lan.acme3 đã dùng 2%',
      'Có lại lúc: —',
      'quota-axi có +cswap: có',
      'Đọc được claude-swap: được',
      'Cập nhật: 14:19 · 1 phút trước',
    ]);
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
    expect(screen.getByText(/^App tính từ số quota-axi lúc 14:20 thì hạn mức trong vòng đã cạn\./)).toBeTruthy();
  });

  it('without a file set, it points to Cài đặt', async () => {
    await open();
    expect(screen.getByText('Chưa nối với firstmate')).toBeTruthy();
    expect(seen()).toEqual([]);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Mở Cài đặt' }));
    });
    expect(screen.getByRole('heading', { name: 'Cài đặt' })).toBeTruthy();
  });

  it('a file that is not there', async () => {
    await open(firstmateFixture({ status: 'missing', line: null, writtenAt: null, fresh: false }));
    expect(screen.getByText('Không thấy firstmate trên máy này')).toBeTruthy();
    expect(screen.getByText('Không có file ~/example/quota-pool.last. Luật "cạn" vẫn tự tính từ số quota-axi để báo.')).toBeTruthy();
  });
});
