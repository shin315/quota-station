// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PopoverApp } from '../App';
import { FOLD_KEY } from './Popover';
import { fakeApi } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel, heldSwitcher, peerFixture, rotationFixture } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

const clock = () => FIXTURE_NOW;

async function open(model: Model) {
  const fake = fakeApi(model);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-popover">
      <PopoverApp api={fake.api} clock={clock} />
    </div>,
  );
  await screen.findByRole('dialog', { name: 'Quota Station' });
  return fake;
}

afterEach(cleanup);

describe('popover', () => {
  it('shows a card per account: Claude cards switch, the card in use and unlisted Antigravity ones do not', async () => {
    const fake = await open(fixtureModel('fresh'));
    // One header line: this Mac, no title and no age while the numbers are fresh.
    expect(screen.getByText('Mac mini')).toBeTruthy();
    expect(screen.queryByText(/cập nhật .* trước/)).toBeNull();
    expect(screen.queryByText('Quota Station')).toBeNull();
    expect(screen.queryByText(/còn tương đương/)).toBeNull();
    expect(screen.queryByText(/n = ngày/)).toBeNull();

    const claude = screen.getByRole('list', { name: 'CLAUDE' });
    const cards = within(claude).getAllByRole('listitem').map((li) => within(li).getByRole('button'));
    expect(cards).toHaveLength(4);
    // The main window's order: the personal account, then the Team seats; the one in use keeps its place.
    expect(cards.map((c) => c.getAttribute('aria-label')!.split(',')[0])).toEqual(['lanpham.dev', 'lan.acme1', 'lan.acme2', 'lan.acme3']);
    // The spoken label keeps every word the screen no longer writes.
    expect(cards[1].getAttribute('aria-label')).toMatch(/^lan\.acme1, Acme Studio, đang dùng\. 5 giờ đã dùng 74%/);
    expect(within(cards[1]).queryByText('đang dùng')).toBeNull();
    expect(within(cards[1]).queryByText('Acme Studio')).toBeNull();
    expect(within(cards[2]).queryByText('Tuần')).toBeNull();
    expect(within(cards[0]).queryByText('chưa dùng')).toBeNull();
    expect(cards[1].getAttribute('aria-disabled')).toBe('true');
    expect(cards[1].getAttribute('title')).toBe('lan.acme1@example.com · Quota Station · tài khoản 2');
    expect(cards[2].getAttribute('aria-label')).toMatch(/^lan\.acme2, Acme Studio\. .*\. Bấm để chuyển$/);
    expect(cards[2].getAttribute('aria-disabled')).toBeNull();
    expect(within(cards[1]).getByText('74%')).toBeTruthy();
    expect(screen.queryByText('2,3')).toBeNull();

    const agy = within(screen.getByRole('list', { name: 'ANTIGRAVITY' })).getAllByRole('button');
    expect(agy).toHaveLength(2);
    const ultra1 = agy.find((b) => !b.getAttribute('aria-label')!.includes('đang dùng'))!;
    expect(ultra1.getAttribute('aria-disabled')).toBe('true');
    expect(ultra1.getAttribute('title')).toBe(
      'lanpham.ultra@example.com · Quota Station · tài khoản 1 · Chưa chuyển được: chưa đọc được danh sách tài khoản của Quota Station',
    );
    fireEvent.click(ultra1);
    fireEvent.click(cards[1]);
    expect(screen.queryByRole('dialog', { name: /Chuyển/ })).toBeNull();
    expect(fake.api.claudeSessions).not.toHaveBeenCalled();
    fireEvent.click(cards[2]);
    expect(fake.api.claudeSessions).toHaveBeenCalledTimes(1);
    expect(fake.api.resizePopover).toHaveBeenCalled();
  });

  it('folds a group to one line from its header, and keeps it folded the next time', async () => {
    localStorage.clear();
    await open(fixtureModel('fresh'));
    const head = screen.getByRole('button', { name: /CLAUDE/ });
    expect(head.getAttribute('aria-expanded')).toBe('true');
    fireEvent.click(head);
    expect(head.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('list', { name: 'CLAUDE' })).toBeNull();
    // The header is the group's name alone: Đang dùng above says whose account and how much.
    expect(head.textContent).toBe('CLAUDE');
    // Antigravity stays open.
    expect(screen.getByRole('list', { name: 'ANTIGRAVITY' })).toBeTruthy();
    expect(JSON.parse(localStorage.getItem(FOLD_KEY)!)).toEqual({ c: true, g: false });

    cleanup();
    await open(fixtureModel('fresh'));
    expect(screen.getByRole('button', { name: /CLAUDE/ }).getAttribute('aria-expanded')).toBe('false');
    const agyHead = screen.getByRole('button', { name: /ANTIGRAVITY/ });
    fireEvent.click(agyHead);
    expect(agyHead.textContent).toBe('ANTIGRAVITY');
    expect(screen.getByRole('list', { name: 'Đang dùng' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /CLAUDE/ }));
    expect(screen.getByRole('list', { name: 'CLAUDE' })).toBeTruthy();
    expect(JSON.parse(localStorage.getItem(FOLD_KEY)!)).toEqual({ c: false, g: true });
    localStorage.clear();
  });

  it('a fold that cannot be kept still works', async () => {
    const get = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('denied');
    });
    const set = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('denied');
    });
    await open(fixtureModel('fresh'));
    const head = screen.getByRole('button', { name: /CLAUDE/ });
    fireEvent.click(head);
    expect(head.getAttribute('aria-expanded')).toBe('false');
    get.mockRestore();
    set.mockRestore();
  });

  it('the footer has the automatic switching toggle and why Claude is on its account', async () => {
    await open(fixtureModel('fresh'));
    const toggle = screen.getByRole('switch', { name: 'Tự chuyển Claude' });
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    // Claude simply staying on its account needs no words here; the Claude tab has them.
    expect(screen.queryByText(/Vẫn dùng lan\.acme1/)).toBeNull();
    expect(screen.queryByText('09:12 · Switcher chuyển sang lan.acme1.')).toBeNull();
    cleanup();
    // A pause is something to act on: its line shows.
    await open(fixtureModel('fresh', { switcher: heldSwitcher() }));
    expect(screen.getByRole('switch', { name: /Tự chuyển Claude/ }).closest('.li')!.querySelector('small')).toBeTruthy();
  });

  it('without Nâng cao the footer has no automatic switching', async () => {
    await open({ ...fixtureModel('fresh'), advanced: { on: false, firstmate: true } });
    expect(screen.queryByRole('switch', { name: /Tự chuyển Claude/ })).toBeNull();
  });

  it('Đang dùng heads the popover: each service, its account, share used and reset', async () => {
    await open(fixtureModel('fresh'));
    const now = within(screen.getByRole('list', { name: 'Đang dùng' })).getAllByRole('listitem');
    expect(now.map((r) => r.textContent)).toEqual(['Claudelan.acme174%↻ 3h 8m', 'Geminilan.ultra22%↻ 1h 57m', 'Codexrelay—']);
    expect(now[0].getAttribute('title')).toBe('Claude: lan.acme1 đã dùng 74% 5 giờ');
    expect(now[2].getAttribute('title')).toBe('Codex: relay, ChatGPT đã dùng 23% 5 giờ');
    cleanup();
    // Old numbers: the age comes back in the header.
    await open(fixtureModel('stale'));
    expect(screen.getByText('· cập nhật 41 phút trước')).toBeTruthy();
  });

  it('one header button turns the look dark or light, the other opens the main window; Escape hides', async () => {
    const fake = await open(fixtureModel('fresh', { theme: 'light' }));
    expect(screen.queryByRole('button', { name: 'Cài đặt' })).toBeNull();
    const theme = screen.getByRole('button', { name: 'Chuyển sang giao diện tối' });
    expect(theme.getAttribute('title')).toBe('Chuyển sang giao diện tối');
    expect(document.documentElement.dataset.theme).toBe('light');
    await act(async () => {
      fireEvent.click(theme);
    });
    expect(fake.api.setTheme).toHaveBeenLastCalledWith('dark');
    expect(fake.api.openWindow).not.toHaveBeenCalled();
    expect(document.documentElement.dataset.theme).toBe('dark');
    const light = screen.getByRole('button', { name: 'Chuyển sang giao diện sáng' });
    await act(async () => {
      fireEvent.click(light);
    });
    expect(fake.api.setTheme).toHaveBeenLastCalledWith('light');

    const app = screen.getByRole('button', { name: 'Mở Quota Station' });
    expect(app.getAttribute('title')).toBe('Mở Quota Station');
    fireEvent.click(app);
    expect(fake.api.openWindow).toHaveBeenCalledTimes(1);
    expect(fake.api.openWindow).toHaveBeenLastCalledWith('overview');
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(fake.api.hidePopover).toHaveBeenCalledTimes(1);
  });

  it('following macOS, the theme button offers the other look', async () => {
    // macOS is dark (jsdom has no matchMedia of its own).
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      value: (q: string) => ({ matches: true, media: q, addEventListener: () => {}, removeEventListener: () => {} }),
    });
    const fake = await open(fixtureModel('fresh', { theme: 'auto' }));
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển sang giao diện sáng' }));
    });
    expect(fake.api.setTheme).toHaveBeenLastCalledWith('light');
    delete (window as { matchMedia?: unknown }).matchMedia;
  });

  it('asks for Keychain access only when the button is pressed', async () => {
    const fake = await open(fixtureModel('keychain'));
    expect(fake.api.grantKeychain).not.toHaveBeenCalled();
    const granted = fixtureModel('fresh');
    vi.mocked(fake.api.grantKeychain).mockImplementation(async () => granted);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Cấp quyền một lần' }));
    });
    expect(fake.api.grantKeychain).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Đã cấp quyền Keychain').closest('.toast')?.getAttribute('title')).toBe('Dòng Claude giờ có số mới của Claude Code.');
  });

  it('a refused Keychain prompt says so', async () => {
    const fake = await open(fixtureModel('keychain'));
    vi.mocked(fake.api.grantKeychain).mockImplementation(async () => fake.current());
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Cấp quyền một lần' }));
    });
    expect(fake.api.grantKeychain).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Chưa cấp được quyền Keychain')).toBeTruthy();
  });

  it('"Để sau" hides the Keychain banner', async () => {
    await open(fixtureModel('keychain'));
    fireEvent.click(screen.getByRole('button', { name: 'Để sau' }));
    expect(screen.queryByText('Chưa cấp quyền Keychain cho Quota Station')).toBeNull();
  });

  it('"Làm mới" reports once the read it asked for is done', async () => {
    const fake = await open(fixtureModel('stale', { reads: 3 }));
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Làm mới' }));
    });
    expect(fake.api.refresh).toHaveBeenCalledTimes(1);
    expect(screen.queryByText('Đã làm mới')).toBeNull();
    act(() => fake.push(fixtureModel('fresh', { reads: 4 })));
    // A short line that goes by itself, with the time on hover.
    expect(screen.getByText('Đã làm mới').closest('.toast')?.getAttribute('title')).toBe('Đọc lại số lúc 14:20.');
  });

  it('updates when the reader pushes a new model', async () => {
    const fake = await open(fixtureModel('fresh'));
    act(() => fake.push(fixtureModel('exhausted')));
    expect(screen.getByText('Hạn mức Claude trong vòng đã cạn')).toBeTruthy();
    expect(screen.getByText('Có lại lúc 16:40 khi Cá nhân và Team B được đặt lại.')).toBeTruthy();
  });

  it('the footer says which Macs have the rotation rules only when one is behind', async () => {
    await open(fixtureModel('fresh'));
    expect(screen.queryByText(/^Luật xoay vòng ·/)).toBeNull();
    cleanup();
    const down = rotationFixture({
      peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'unreachable', error: 'x', retryAt: '2026-01-16T07:25:40.000Z' } })],
    });
    await open(fixtureModel('fresh', { rotation: down }));
    expect(screen.getByText('Luật xoay vòng · Mac B: 14:02 · Mac C: chưa tới được, sẽ thử lại')).toBeTruthy();
    cleanup();
    await open(fixtureModel('fresh', { rotation: rotationFixture({ peers: [] }) }));
    expect(screen.queryByText(/^Luật xoay vòng ·/)).toBeNull();
  });
});
