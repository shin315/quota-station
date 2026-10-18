import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PopoverApp } from '../App';
import { FOLD_KEY } from './Popover';
import { fakeApi } from '../fixtures/fakeApi';
import { failing, FIXTURE_NOW, fixtureModel, peerFixture, rotationFixture } from '../fixtures/models';
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
    expect(screen.getByText('Mac mini · cập nhật 40 giây trước')).toBeTruthy();
    expect(screen.queryByText(/còn tương đương/)).toBeNull();
    expect(screen.queryByText(/n = ngày/)).toBeNull();

    const claude = screen.getByRole('list', { name: 'CLAUDE' });
    const cards = within(claude).getAllByRole('listitem').map((li) => within(li).getByRole('button'));
    expect(cards).toHaveLength(4);
    expect(cards[0].getAttribute('aria-label')).toMatch(/^lan\.acme1, Acme Studio, đang dùng\. 5 giờ đã dùng 74%/);
    expect(cards[0].getAttribute('aria-disabled')).toBe('true');
    expect(cards[0].getAttribute('title')).toBe('lan.acme1@example.com · claude-swap · tài khoản 2');
    expect(cards[2].getAttribute('aria-label')).toMatch(/^lan\.acme2, Acme Studio\. .*\. Bấm để chuyển$/);
    expect(cards[2].getAttribute('aria-disabled')).toBeNull();
    expect(within(cards[0]).getByText('74%')).toBeTruthy();
    expect(screen.getByText('2,3')).toBeTruthy();

    const agy = within(screen.getByRole('list', { name: 'ANTIGRAVITY' })).getAllByRole('button');
    expect(agy).toHaveLength(2);
    const ultra1 = agy.find((b) => !b.getAttribute('aria-label')!.includes('đang dùng'))!;
    expect(ultra1.getAttribute('aria-disabled')).toBe('true');
    expect(ultra1.getAttribute('title')).toBe(
      'lanpham.ultra@example.com · Cockpit Tools · tài khoản 1 · Chưa chuyển được: quota-axi chưa liệt kê tài khoản của Quota Station',
    );
    fireEvent.click(ultra1);
    fireEvent.click(cards[0]);
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
    expect(head.textContent).toBe('CLAUDElan.acme1 · 87% Fable');
    // Antigravity stays open.
    expect(screen.getByRole('list', { name: 'ANTIGRAVITY' })).toBeTruthy();
    expect(JSON.parse(localStorage.getItem(FOLD_KEY)!)).toEqual({ c: true, g: false });

    cleanup();
    await open(fixtureModel('fresh'));
    expect(screen.getByRole('button', { name: /CLAUDE/ }).getAttribute('aria-expanded')).toBe('false');
    const agyHead = screen.getByRole('button', { name: /ANTIGRAVITY/ });
    fireEvent.click(agyHead);
    expect(agyHead.textContent).toBe('ANTIGRAVITYlan.ultra2 · Gemini 6% · C/GPT 0%');
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
    expect(screen.getByText('14:19 · Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.')).toBeTruthy();
    expect(screen.queryByText('09:12 · Switcher chuyển sang lan.acme1.')).toBeNull();
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
    expect(screen.getByText('Đã cấp quyền Keychain').closest('.toast')?.getAttribute('title')).toBe('Dòng Claude giờ đọc trực tiếp từ quota-axi.');
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
    expect(screen.queryByText('Chưa cấp quyền Keychain cho quota-axi')).toBeNull();
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

  it('quota-axi missing offers the settings and keeps the last numbers', async () => {
    const fake = await open(fixtureModel('fresh', { errors: failing('notFound') }));
    expect(screen.getByText('Không chạy được quota-axi')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Chọn đường dẫn…' }));
    expect(fake.api.openWindow).toHaveBeenCalledWith('settings');
    expect(screen.getAllByRole('listitem').length).toBeGreaterThan(0);
  });

  it('updates when the reader pushes a new model', async () => {
    const fake = await open(fixtureModel('fresh'));
    act(() => fake.push(fixtureModel('exhausted')));
    expect(screen.getByText('Hạn mức Claude trong vòng đã cạn')).toBeTruthy();
    expect(screen.getByText('Có lại lúc 16:40 khi Cá nhân và Team B được đặt lại.')).toBeTruthy();
  });

  it('the footer says which Macs have the rotation rules', async () => {
    await open(fixtureModel('fresh'));
    expect(screen.getByText('Luật xoay vòng · Mac B: 14:02 · Mac C: 14:02')).toBeTruthy();
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
