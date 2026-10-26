// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PopoverApp } from '../App';
import { FOLD_KEY, MORE_KEY } from './Popover';
import { fakeApi } from '../fixtures/fakeApi';
import { claudeAccountsFixture, codexFixture, FIXTURE_NOW, fixtureModel, heldSwitcher, peerFixture, rotationFixture } from '../fixtures/models';
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

/** A section by its service's name. */
const section = (name: string) => screen.getByRole('region', { name });
/** A section's account in use: a status tile, not a button. */
const inUse = (name: string) => within(section(name)).getByRole('group');
/** The line under it that shows the other accounts. */
const disclosure = (name: string) => within(section(name)).getByRole('button', { name: /tài khoản|accounts?$/ });
/** The other accounts' buttons, once the line is open. */
const others = (name: string) => within(screen.getByRole('list', { name })).getAllByRole('button');
/** Every section opens on its other accounts, as the owner leaves them. */
const showOthers = () => localStorage.setItem(MORE_KEY, JSON.stringify({ c: true, g: true, x: true }));

beforeEach(() => localStorage.clear());
afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe('popover', () => {
  it('shows each service’s account in use, the others behind one line: Claude rows switch, unlisted Antigravity ones do not', async () => {
    const fake = await open(fixtureModel('fresh'));
    // One header line: this Mac, with no age while the numbers are fresh.
    expect(screen.getByText('Mac mini')).toBeTruthy();
    expect(screen.queryByText(/cập nhật .* trước/)).toBeNull();
    expect(screen.queryByText(/còn tương đương/)).toBeNull();
    expect(screen.queryByText(/n = ngày/)).toBeNull();

    // The account in use first, a status with every window; the spoken label keeps every word the screen no longer writes.
    const acme1 = inUse('Claude');
    expect(acme1.getAttribute('aria-label')).toMatch(/^lan\.acme1, Acme Studio, đang dùng\. 5 giờ đã dùng 74%/);
    expect(acme1.getAttribute('data-active')).toBe('true');
    // Its rotation note is no chip here: the hover has it, and the main window.
    expect(acme1.getAttribute('title')).toBe('lan.acme1@example.com · Quota Station · tài khoản 2 · rời vòng 17:45');
    expect(screen.queryByText(/rời vòng/)).toBeNull();
    expect(within(acme1).getByText('Đang dùng')).toBeTruthy();
    expect(within(acme1).getByText('74%')).toBeTruthy();
    expect(within(acme1).queryByText('Acme Studio')).toBeNull();
    expect(acme1.textContent).toBe('lan.acme1Đang dùng5 giờ74%3h 8mTuần39%4d 19hFable87%1d 17h');

    // The other accounts wait behind one line, closed until the owner opens it.
    const more = disclosure('Claude');
    expect(more.textContent).toBe('3 tài khoản khác');
    expect(more.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('list', { name: 'Claude' })).toBeNull();
    fireEvent.click(more);
    expect(more.getAttribute('aria-expanded')).toBe('true');
    const list = screen.getByRole('list', { name: 'Claude' });
    expect(more.getAttribute('aria-controls')).toBe(list.id);
    const cards = others('Claude');
    // The main window's order, without the one in use: the personal account, then the Team seats.
    expect(cards.map((c) => c.getAttribute('aria-label')!.split(/,|\. /)[0])).toEqual(['lanpham.dev', 'lan.acme2', 'lan.acme3']);
    expect(within(cards[0]).getByText('chưa dùng')).toBeTruthy();
    expect(within(cards[1]).queryByText('Tuần')).toBeNull();
    expect(cards[1].getAttribute('aria-label')).toMatch(/^lan\.acme2, Acme Studio\. .*\. Bấm để chuyển$/);
    expect(cards[1].getAttribute('aria-disabled')).toBeNull();
    expect(screen.queryByText('2,3')).toBeNull();
    expect(JSON.parse(localStorage.getItem(MORE_KEY)!)).toEqual({ c: true, g: false, x: false });

    // Antigravity: lan.ultra2 in use, lanpham.ultra under the line, grey while the list is not read.
    expect(inUse('Antigravity').getAttribute('aria-label')).toMatch(/^lan\.ultra2/);
    fireEvent.click(disclosure('Antigravity'));
    const agy = others('Antigravity');
    expect(agy).toHaveLength(1);
    expect(agy[0].getAttribute('aria-disabled')).toBe('true');
    expect(agy[0].getAttribute('title')).toBe(
      'lanpham.ultra@example.com · Quota Station · tài khoản 1 · Chưa chuyển được: chưa đọc được danh sách tài khoản của Quota Station · số 25 phút trước',
    );
    // Its numbers are 25 minutes old: drawn dim, the age on hover, no chip.
    expect(agy[0].getAttribute('data-stale')).toBe('true');
    expect(screen.queryByText(/phút trước/)).toBeNull();
    fireEvent.click(agy[0]);
    fireEvent.click(acme1);
    expect(screen.queryByRole('dialog', { name: /Chuyển/ })).toBeNull();
    expect(fake.api.claudeSessions).not.toHaveBeenCalled();
    fireEvent.click(cards[1]);
    expect(fake.api.claudeSessions).toHaveBeenCalledTimes(1);
    expect(fake.api.resizePopover).toHaveBeenCalled();
  });

  it('every disclosure is a button the keyboard reaches, and says whether it is open', async () => {
    await open(fixtureModel('fresh'));
    const lines = ['Claude', 'Antigravity', 'Codex'].flatMap((name) => within(section(name)).getAllByRole('button').filter((b) => b.hasAttribute('aria-expanded')));
    expect(lines.map((b) => b.textContent)).toEqual(['Claude', '3 tài khoản khác', 'Antigravity', '1 tài khoản khác', 'Codex', '1 tài khoản khác']);
    for (const b of lines) {
      expect(b.tagName).toBe('BUTTON');
      expect(b.tabIndex).toBe(0);
      expect(b.getAttribute('aria-expanded')).toMatch(/^(true|false)$/);
    }
    lines[1].focus();
    expect(document.activeElement).toBe(lines[1]);
  });

  it('folds a section to one line from its header, and keeps it folded the next time', async () => {
    await open(fixtureModel('fresh'));
    const head = screen.getByRole('button', { name: /^Claude/ });
    expect(head.getAttribute('aria-expanded')).toBe('true');
    expect(head.textContent).toBe('Claude');
    fireEvent.click(head);
    expect(head.getAttribute('aria-expanded')).toBe('false');
    expect(within(section('Claude')).queryByRole('group')).toBeNull();
    // Folded, the header says whose account is in use and how much of it.
    expect(head.textContent).toBe('Claudelan.acme174%');
    // Antigravity stays open.
    expect(inUse('Antigravity')).toBeTruthy();
    expect(JSON.parse(localStorage.getItem(FOLD_KEY)!)).toEqual({ c: true, g: false, x: false });

    cleanup();
    await open(fixtureModel('fresh'));
    expect(screen.getByRole('button', { name: /^Claude/ }).getAttribute('aria-expanded')).toBe('false');
    const agyHead = screen.getByRole('button', { name: /^Antigravity/ });
    fireEvent.click(agyHead);
    expect(agyHead.textContent).toBe('Antigravitylan.ultra22%');
    fireEvent.click(screen.getByRole('button', { name: /^Claude/ }));
    expect(inUse('Claude')).toBeTruthy();
    expect(JSON.parse(localStorage.getItem(FOLD_KEY)!)).toEqual({ c: false, g: true, x: false });
  });

  it('a fold that cannot be kept still works', async () => {
    const get = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('denied');
    });
    const set = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('denied');
    });
    await open(fixtureModel('fresh'));
    const head = screen.getByRole('button', { name: /^Claude/ });
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
    expect(screen.getByRole('switch', { name: /Tự chuyển Claude/ }).parentElement!.querySelector('small')).toBeTruthy();
  });

  it('without Nâng cao the footer has no automatic switching', async () => {
    await open({ ...fixtureModel('fresh'), advanced: { on: false, firstmate: true } });
    expect(screen.queryByRole('switch', { name: /Tự chuyển Claude/ })).toBeNull();
  });

  it('each section opens on its account in use, with its share used and reset', async () => {
    await open(fixtureModel('fresh'));
    expect(['Claude', 'Antigravity', 'Codex'].map((name) => inUse(name).textContent)).toEqual(['lan.acme1Đang dùng5 giờ74%3h 8mTuần39%4d 19hFable87%1d 17h', 'lan.ultra2Đang dùngGemini5 giờ2%1h 57mTuần6%3d 23hClaude/GPT5 giờ0%4h 59mTuần0%5d 23h', 'relayĐang dùngCustom API · không báo hạn mức']);
    cleanup();
    // Old numbers: the age comes back in the header.
    await open(fixtureModel('stale'));
    expect(screen.getByText('· cập nhật 41 phút trước')).toBeTruthy();
  });

  it('writes no notes on the accounts: an old number is dim with its age on hover, the rotation stays in the main window', async () => {
    const m = fixtureModel('fresh');
    const accounts = m.snapshot.claude.accounts;
    const old = new Date(FIXTURE_NOW.getTime() - 25 * 60_000).toISOString();
    accounts[2] = { ...accounts[2], reading: 'stale', fetchedAt: old };
    accounts[3] = { ...accounts[3], membership: 'ended' };
    showOthers();
    await open(m);
    const [, acme2, acme3] = others('Claude');
    expect(acme2.getAttribute('data-stale')).toBe('true');
    expect(acme2.getAttribute('title')).toBe('lan.acme2@example.com · Quota Station · tài khoản 3 · số 25 phút trước');
    // The bars and numbers go grey, not red or orange.
    const numbers = within(acme2).getAllByText(/%$/);
    expect(numbers.length).toBeGreaterThan(0);
    for (const n of numbers) expect(n.className).toContain('text-ink3');
    expect(acme3.getAttribute('title')).toBe('lan.acme3@example.com · Quota Station · tài khoản 4 · đã rời vòng');
    expect(acme3.getAttribute('data-stale')).toBeNull();
    for (const note of [/phút trước/, /rời vòng/, /ngoài vòng/, /nghỉ tự chuyển/]) expect(screen.queryByText(note)).toBeNull();
  });

  it('shows the glass, and the solid ground while macOS reduces transparency', async () => {
    const fake = fakeApi(fixtureModel('fresh'), 'overview', { reduceTransparency: true });
    render(
      <div className="qs app-popover" data-material="glass">
        <PopoverApp api={fake.api} clock={clock} />
      </div>,
    );
    await screen.findByRole('dialog', { name: 'Quota Station' });
    const root = document.querySelector<HTMLElement>('.qs')!;
    expect(document.documentElement.dataset.material).toBe('solid');
    expect(root.dataset.material).toBe('solid');
    // The owner turns it off in macOS's settings and comes back to the popover.
    act(() => fake.setReduceTransparency(false));
    expect(document.documentElement.dataset.material).toBe('glass');
    expect(root.dataset.material).toBe('glass');
  });

  it('draws stronger edges while macOS increases contrast, and the theme pack the owner chose', async () => {
    const fake = fakeApi(fixtureModel('fresh', { themePack: 'banDieuKhien' }), 'overview', { increaseContrast: true });
    render(
      <div className="qs app-popover" data-contrast="normal" data-pack="default">
        <PopoverApp api={fake.api} clock={clock} />
      </div>,
    );
    await screen.findByRole('dialog', { name: 'Quota Station' });
    const root = document.querySelector<HTMLElement>('.qs')!;
    expect(document.documentElement.dataset.contrast).toBe('more');
    expect(root.dataset.contrast).toBe('more');
    expect(document.documentElement.dataset.pack).toBe('banDieuKhien');
    expect(root.dataset.pack).toBe('banDieuKhien');
    // The owner turns it off in macOS's settings, and picks another pack in the main window.
    act(() => fake.setIncreaseContrast(false));
    expect(root.dataset.contrast).toBe('normal');
    await act(async () => {
      await fake.api.setThemePack('kinhNhip');
    });
    expect(root.dataset.pack).toBe('kinhNhip');
    document.documentElement.dataset.pack = 'default';
    document.documentElement.dataset.contrast = 'normal';
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

  it('no Claude account yet: an empty card whose button opens the Claude tab', async () => {
    const m = fixtureModel('fresh');
    m.snapshot.claude.accounts = [];
    const fake = await open(m);
    expect(screen.getByText('Chưa có tài khoản Claude nào')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Thêm tài khoản…' }));
    expect(fake.api.openWindow).toHaveBeenCalledWith('claude');
  });

  it('no agent on this Mac: one empty card with Kiểm lại, and no groups', async () => {
    const m = fixtureModel('fresh', { codex: codexFixture({ present: false }) });
    m.snapshot.claude.status = 'notSetUp';
    m.snapshot.agy.status = 'notSetUp';
    const fake = await open(m);
    expect(screen.getByText('Chưa thấy Claude Code, Codex hay Antigravity trên máy này')).toBeTruthy();
    expect(screen.queryByRole('list', { name: 'Claude' })).toBeNull();
    expect(screen.queryByRole('list', { name: 'Antigravity' })).toBeNull();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Kiểm lại' }));
    });
    expect(fake.api.refresh).toHaveBeenCalled();
  });

  it('the first read says so and draws grey bars instead of numbers', async () => {
    const m = fixtureModel('fresh', { reading: true });
    m.snapshot.claude.newestReadingAt = null;
    m.snapshot.agy.newestReadingAt = null;
    await open(m);
    expect(screen.getByText('Đang đọc hạn mức lần đầu…')).toBeTruthy();
    expect(screen.queryByText('74%')).toBeNull();
    // The other accounts wait: none of them switches before the numbers are in.
    fireEvent.click(disclosure('Claude'));
    expect(others('Claude').every((c) => c.getAttribute('aria-disabled') === 'true')).toBe(true);
  });

  it('a card to sign in again opens the sign-in in the Claude tab instead of switching', async () => {
    const accounts = claudeAccountsFixture();
    accounts.accounts[2] = { ...accounts.accounts[2], status: 'reloginRequired' };
    const fake = await open(fixtureModel('fresh', { claudeAccounts: accounts }));
    // Closed, the line still says one must sign in again: something to act on.
    expect(disclosure('Claude').textContent).toBe('3 tài khoản khác· 1 cần đăng nhập lại');
    fireEvent.click(disclosure('Claude'));
    const card = screen.getByRole('button', { name: 'lan.acme2, cần đăng nhập lại. Bấm để đăng nhập lại' });
    expect(within(card).getByText('Cần đăng nhập lại')).toBeTruthy();
    fireEvent.click(card);
    expect(fake.api.claudeSessions).not.toHaveBeenCalled();
    expect(fake.api.openWindow).toHaveBeenCalledWith('claude');
  });

  it('speaks English when the setting says so', async () => {
    await open(fixtureModel('fresh', { language: 'en' }));
    expect(within(inUse('Claude')).getByText('In use')).toBeTruthy();
    expect(screen.getByRole('button', { name: '3 other accounts' })).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Claude auto-switch' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Switch to the dark look' })).toBeTruthy();
    expect(document.documentElement.lang).toBe('en');
  });
});
