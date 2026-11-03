// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Chép sang Mac khác tab and its dialogs (M13) on the fake Rust side:
// nothing here opens ssh, reads an agent's file or touches the Keychain.
// Every Mac, item and key is made up.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '@/App';
import { fakeApi, type FakeOptions } from '@/fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel } from '@/fixtures/models';
import type { Api } from '@/lib/api';
import type { Model } from '@/lib/model';

async function openTab(model: Model = fixtureModel('fresh'), options: FakeOptions = {}) {
  const fake = fakeApi(model, 'copy', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { level: 2, name: 'Chép sang Mac khác' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const dialog = () => screen.getByRole('dialog');
const button = (name: string | RegExp) => screen.getByRole('button', { name });
const box = (name: RegExp) => screen.getByRole('checkbox', { name }) as HTMLButtonElement;

async function compare() {
  await press(button('So sánh'));
  await screen.findByRole('heading', { level: 4, name: /^Khác nhau/ });
}

afterEach(cleanup);

describe('the Chép sang Mac khác tab', () => {
  it('reads the other Mac only when asked, and ticks what this Mac alone has', async () => {
    const fake = await openTab();
    const nav = screen.getByRole('navigation', { name: 'Các tab' });
    expect(within(nav).getByRole('button', { name: 'Chép sang Mac khác' }).getAttribute('aria-current')).toBe('page');
    expect(fake.api.copyCompare).not.toHaveBeenCalled();
    expect(screen.getByRole('radio', { name: /Mac B/ }).getAttribute('aria-checked')).toBe('true');
    await compare();
    expect(fake.api.copyCompare).toHaveBeenCalledWith('mac-b');
    expect(box(/^design-kit/).getAttribute('aria-checked')).toBe('true');
    expect(box(/^deploy-check/).getAttribute('aria-checked')).toBe('true');
    expect(box(/^formatter/).getAttribute('aria-checked')).toBe('true');
    expect(box(/^files/).getAttribute('aria-checked')).toBe('false');
    expect(box(/^ui-polish/).getAttribute('aria-checked')).toBe('false');
    expect(box(/^tracker/).disabled).toBe(true);
    expect(screen.getByRole('switch', { name: 'Gửi cả khoá qua SSH' }).getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText('Giống nhau: 11 mục.')).toBeTruthy();
    expect(fake.api.copyRun).not.toHaveBeenCalled();
    expect(document.body.textContent).not.toMatch(/sha256|opt\/homebrew|Keychain/);
  });

  it('shows one item on both Macs, by the command name only', async () => {
    await openTab();
    await compare();
    await press(button('Xem files'));
    const d = within(dialog());
    expect(d.getByRole('heading', { name: 'files · khác cấu hình' })).toBeTruthy();
    expect(d.getAllByText('files-mcp --root ~/Workspace').length).toBe(2);
    expect(d.getAllByText('files-mcp --root ~/Code').length).toBe(2);
    expect(dialog().textContent).not.toMatch(/opt\/homebrew|\.local\/bin/);
    await press(d.getByRole('button', { name: 'Đóng' }));
    await press(button('Xem CLAUDE.md của Claude Code'));
    // Sending: the other Mac's file now, then after the copy.
    expect(within(dialog()).getByText('Trên Mac B bây giờ → sau khi chép')).toBeTruthy();
    expect(within(dialog()).getByText('- Dùng pnpm cho dự án mới.').getAttribute('data-op')).toBe('add');
  });

  it('copies the ticked items without keys, then undoes it on the other Mac', async () => {
    const fake = await openTab();
    await compare();
    await press(button('Chép 3 mục sang Mac B…'));
    const d = within(dialog());
    expect(d.getByRole('heading', { name: 'Chép 3 mục sang Mac B?' })).toBeTruthy();
    expect(d.getByText('Không gửi khoá.')).toBeTruthy();
    expect(d.getByText('design-kit: thêm cho các agent đang có nó · tắt đến khi dán khoá')).toBeTruthy();
    expect(d.getByText('Không đổi nhà cung cấp Codex, đăng nhập hay tài khoản.')).toBeTruthy();
    await press(d.getByRole('button', { name: 'Chép' }));
    expect(fake.api.copyRun).toHaveBeenCalledWith(
      'mac-b',
      true,
      { mcp: ['design-kit'], skills: ['deploy-check'], updates: [], plugins: [{ agent: 'claude', id: 'formatter@tools-market' }], files: [] },
      false,
    );
    await screen.findByRole('heading', { name: 'Đã chép 3 mục' });
    expect(within(dialog()).getByText('Mac B sẽ hỏi dán khoá')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Hoàn tác trên Mac B' }));
    await press(within(dialog()).getByRole('button', { name: 'Hoàn tác' }));
    const [alias, push, , force] = vi.mocked(fake.api.copyUndo).mock.calls[0];
    expect([alias, push, force]).toEqual(['mac-b', true, false]);
    expect(await screen.findByText('Đã hoàn tác trên Mac B.')).toBeTruthy();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('sends keys only when the box is ticked', async () => {
    const fake = await openTab();
    await compare();
    await press(screen.getByRole('switch', { name: 'Gửi cả khoá qua SSH' }));
    await press(button('Chép 3 mục sang Mac B…'));
    expect(within(dialog()).getByText('Gửi 1 khoá qua SSH vào Keychain của Mac B.')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Chép' }));
    expect(vi.mocked(fake.api.copyRun).mock.calls[0][3]).toBe(true);
    await screen.findByRole('heading', { name: 'Đã chép 3 mục' });
    await press(within(dialog()).getByRole('button', { name: 'Xong' }));
    // A new comparison starts with the box off again.
    await compare();
    expect(screen.getByRole('switch', { name: 'Gửi cả khoá qua SSH' }).getAttribute('aria-checked')).toBe('false');
  });

  it('brings items to this Mac the other way', async () => {
    const fake = await openTab();
    await press(button('Lấy từ Mac B về'));
    await compare();
    expect(box(/^tracker/).getAttribute('aria-checked')).toBe('true');
    expect(box(/^design-kit/).disabled).toBe(true);
    expect(box(/^ui-polish/).disabled).toBe(true);
    await press(button('Chép 1 mục về máy này…'));
    await press(within(dialog()).getByRole('button', { name: 'Chép' }));
    expect(fake.api.copyRun).toHaveBeenCalledWith('mac-b', false, { mcp: ['tracker'], skills: [], updates: [], plugins: [], files: [] }, false);
  });

  it('writes nothing when the other Mac changed since the comparison', async () => {
    await openTab(fixtureModel('fresh'), { copyChanged: true });
    await compare();
    await press(button('Chép 3 mục sang Mac B…'));
    await press(within(dialog()).getByRole('button', { name: 'Chép' }));
    expect(await within(dialog()).findByRole('alert')).toBeTruthy();
    expect(within(dialog()).getByRole('alert').textContent).toMatch(/Chưa chép: file vừa đổi/);
    expect(within(dialog()).getByRole('heading', { name: 'Chép 3 mục sang Mac B?' })).toBeTruthy();
  });

  it('says a Mac that does not answer, and offers to try again', async () => {
    const fake = await openTab(fixtureModel('fresh'), { copyUnreach: ['mac-b'] });
    await press(button('So sánh'));
    expect(await screen.findByText('Mac B không trả lời qua SSH')).toBeTruthy();
    expect(screen.getByText('không trả lời')).toBeTruthy();
    expect(button('Thử lại')).toBeTruthy();
    expect(screen.queryByRole('heading', { level: 4, name: /^Khác nhau/ })).toBeNull();
    expect(document.body.textContent).not.toMatch(/port 22/);
    expect(fake.api.copyRun).not.toHaveBeenCalled();
  });

  it('never writes to a Mac still on version 1', async () => {
    const fake = await openTab(fixtureModel('fresh'), { copyOld: ['mac-b'] });
    await press(button('So sánh'));
    expect(await screen.findByText('Quota Station trên Mac B chưa có phần MCP và skills')).toBeTruthy();
    expect(screen.getByText('bản cũ')).toBeTruthy();
    expect(screen.queryByRole('button', { name: /^Chép \d/ })).toBeNull();
    expect(fake.api.copyRun).not.toHaveBeenCalled();
  });

  it('keeps this Mac, the way, the other Mac and the comparison in one bar', async () => {
    await openTab();
    const bar = screen.getByRole('group', { name: 'Chép với Mac nào' });
    expect(bar.textContent).toMatch(/^Mac Studio\(máy này\)/);
    expect(within(bar).getByRole('group', { name: 'Chiều chép' })).toBeTruthy();
    expect(within(bar).getAllByRole('radio').map((r) => r.textContent)).toEqual(['Mac B tên kết nối mac-b', 'Mac C tên kết nối mac-c']);
    expect(within(bar).getByRole('button', { name: 'So sánh' })).toBeTruthy();
  });

  it('lists the differences by kind, and ends with the keys beside the copy', async () => {
    await openTab();
    await compare();
    const card = screen.getByRole('heading', { level: 4, name: /^Khác nhau · Mac Studio → Mac B/ }).parentElement!;
    const kinds = within(card).getAllByRole('group').filter((g) => g.getAttribute('aria-label') !== 'Chép');
    expect(kinds.map((g) => g.getAttribute('aria-label'))).toEqual(['MCP', 'Skills', 'Plugin', 'File hướng dẫn']);
    expect(within(kinds[0]).getAllByRole('checkbox')).toHaveLength(3);
    // The keys and the copy are the card's last part, one bar.
    const foot = within(card).getByRole('group', { name: 'Chép' });
    expect(card.lastElementChild).toBe(foot);
    expect(within(foot).getByText('Khoá')).toBeTruthy();
    expect(within(foot).getByRole('switch', { name: 'Gửi cả khoá qua SSH' })).toBeTruthy();
    expect(within(foot).getByRole('button', { name: 'Chép 3 mục sang Mac B…' })).toBeTruthy();
    // Bringing items here turns the arrow round.
    await press(button('Lấy từ Mac B về'));
    expect(screen.getByRole('heading', { level: 4, name: /^Khác nhau · Mac Studio ← Mac B/ })).toBeTruthy();
  });

  it('points to Cài đặt when there is no other Mac', async () => {
    const model = fixtureModel('fresh');
    await openTab({ ...model, rotation: { ...model.rotation, peers: [] } });
    expect(screen.getByText('Chưa có Mac nào để chép')).toBeTruthy();
    expect(button('Mở Cài đặt')).toBeTruthy();
  });
});
