// The variables that override a login (M14) on the fake Rust side: Cài
// đặt's card, the Claude tab's banner and the alerts. Nothing here reads a
// shell file or launchd; every variable is made up, and none has a value.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { API_KEY, BASE_URL, CLAUDE_BASE, envAlert, envFixture, LAUNCHD_GEMINI } from '../fixtures/fakeEnv';
import { alertsFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import trap from '../fixtures/env-trap.json';
import type { Api } from '../lib/api';
import type { AlertRecord, EnvFound, EnvPlace, Model, Tab } from '../lib/model';

async function openTab(tab: Tab, model: Model) {
  const fake = fakeApi(model, tab);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Mục' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const withEnv = (found: EnvFound[], over: Partial<Model> = {}) => fixtureModel('fresh', { env: envFixture({ found }), ...over });
const card = () => document.querySelector<HTMLElement>('.env-card')!;
const row = (name: string) => within(card()).getByRole('listitem', { name: new RegExp('^' + name) });

afterEach(cleanup);

describe('Cài đặt › Biến môi trường đè đăng nhập', () => {
  it('comes first, checks again when Cài đặt opens, and says each variable, its place and what it does', async () => {
    const fake = await openTab('settings', withEnv([API_KEY, BASE_URL, LAUNCHD_GEMINI]));
    expect(fake.api.envCheck).toHaveBeenCalled();
    const h4s = [...document.querySelectorAll('.main .card h4')].map((h) => h.textContent);
    expect(h4s[0]).toBe('Biến môi trường đè đăng nhập');
    const key = row('ANTHROPIC_API_KEY');
    expect(within(key).getByText('Claude Code').className).toBe('chip bad');
    expect(within(key).getByText('~/.zshrc · dòng 42')).toBeTruthy();
    expect(within(key).getByText('Trong Terminal, Claude Code dùng khoá này thay cho tài khoản claude-swap đang chọn.')).toBeTruthy();
    expect(within(row('OPENAI_BASE_URL')).getByText('Codex').className).toBe('chip warn');
    // launchd has no file to open.
    const launchd = row('GEMINI_API_KEY');
    expect(within(launchd).getByText('launchctl · app mở từ Dock')).toBeTruthy();
    expect(within(launchd).queryByRole('button', { name: /^Mở file/ })).toBeNull();
    expect(within(card()).getByText(/Đã kiểm 11 biến ở 6 chỗ lúc \d\d:\d\d\. Chỉ biết biến có đặt hay không, không đọc giá trị\./)).toBeTruthy();
    // The title row drags the window, as on every tab.
    expect(screen.getByRole('heading', { level: 2, name: 'Cài đặt' }).parentElement!.getAttribute('data-tauri-drag-region')).toBe('deep');
    // One sheet of glass: the card is not inside another card.
    expect(card().parentElement!.closest('.card')).toBeNull();
  });

  it('opens the file at the line', async () => {
    const fake = await openTab('settings', withEnv([API_KEY]));
    await press(within(row('ANTHROPIC_API_KEY')).getByRole('button', { name: 'Mở file: ~/.zshrc · dòng 42' }));
    expect(fake.api.envOpen).toHaveBeenCalledWith('ANTHROPIC_API_KEY@zshrc');
    const toast = screen.getByText('Đã mở ~/.zshrc dòng 42 trong Visual Studio Code').closest('.toast') as HTMLElement;
    expect(toast.title).toBe('App không sửa file shell.');
  });

  it('"Bỏ qua" hides one place, says how many are hidden, and "Hoàn tác" brings it back', async () => {
    const fake = await openTab('settings', withEnv([API_KEY, BASE_URL]));
    await press(within(row('ANTHROPIC_API_KEY')).getByRole('button', { name: 'Bỏ qua ANTHROPIC_API_KEY ở ~/.zshrc · dòng 42' }));
    expect(fake.api.envIgnore).toHaveBeenCalledWith('ANTHROPIC_API_KEY@zshrc');
    expect(within(card()).queryByRole('listitem', { name: /^ANTHROPIC_API_KEY/ })).toBeNull();
    expect(screen.getByText('Đã bỏ qua ANTHROPIC_API_KEY ở ~/.zshrc')).toBeTruthy();
    expect(within(card()).getByText(/Đã bỏ qua 1 chỗ\./)).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Hoàn tác' }));
    expect(fake.api.envUnignore).toHaveBeenCalledWith('ANTHROPIC_API_KEY@zshrc');
    expect(row('ANTHROPIC_API_KEY')).toBeTruthy();
  });

  it('none set: one green line, and "Kiểm lại" looks again', async () => {
    const fake = await openTab('settings', withEnv([]));
    expect(within(card()).queryByRole('list')).toBeNull();
    expect(within(card()).getByText('không có').className).toBe('chip ok');
    expect(within(card()).getByText('Không có biến nào đè đăng nhập.')).toBeTruthy();
    vi.mocked(fake.api.envCheck).mockClear();
    await press(within(card()).getByRole('button', { name: 'Kiểm lại' }));
    expect(fake.api.envCheck).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/Đã kiểm lúc \d\d:\d\d: không có biến nào đè đăng nhập/)).toBeTruthy();
  });

  it('a place it could not read is said', async () => {
    await openTab('settings', fixtureModel('fresh', { env: envFixture({ found: [API_KEY], unread: ['zprofile', 'launchctl'] }) }));
    expect(within(card()).getByText('Không đọc được ~/.zprofile, launchctl.')).toBeTruthy();
  });

  it('"Xem danh sách" lists the eleven names and the six places', async () => {
    await openTab('settings', withEnv([]));
    await press(within(card()).getByRole('button', { name: 'Xem danh sách' }));
    const d = screen.getByRole('dialog', { name: 'App kiểm những gì' });
    for (const n of envFixture().vars) expect(within(d).getByText(new RegExp(n + '( ·|$)'))).toBeTruthy();
    expect(within(d).getAllByRole('listitem').map((li) => li.textContent)).toEqual([
      '~/.zshenv',
      '~/.zprofile',
      '~/.zshrc',
      '~/.bash_profile',
      '~/.profile',
      'launchctl (app mở từ Dock)',
    ]);
    expect(within(d).getByText(/không đọc giá trị, không chạy file shell, không sửa file và không tự bỏ biến/)).toBeTruthy();
    await press(within(d).getByRole('button', { name: 'Đóng' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});

describe('the Claude tab', () => {
  it('has a red banner while a variable overrides Claude Code, with "Mở file", "Bỏ qua" and "Kiểm lại"', async () => {
    const fake = await openTab('claude', withEnv([API_KEY, BASE_URL]));
    const banner = screen.getByText('ANTHROPIC_API_KEY đang đè đăng nhập Claude').closest('.banner') as HTMLElement;
    expect(banner.className).toBe('banner bad');
    expect(
      within(banner).getByText('Đặt ở ~/.zshrc dòng 42. Trong Terminal, Claude Code dùng khoá này thay cho tài khoản claude-swap đang chọn.'),
    ).toBeTruthy();
    expect(within(banner).getAllByRole('button').map((b) => b.textContent)).toEqual(['Mở ~/.zshrc ở dòng 42', 'Bỏ qua', 'Kiểm lại']);
    await press(within(banner).getByRole('button', { name: 'Mở ~/.zshrc ở dòng 42' }));
    expect(fake.api.envOpen).toHaveBeenCalledWith('ANTHROPIC_API_KEY@zshrc');
    await press(within(banner).getByRole('button', { name: 'Bỏ qua' }));
    expect(fake.api.envIgnore).toHaveBeenCalledWith('ANTHROPIC_API_KEY@zshrc');
    expect(screen.queryByText('ANTHROPIC_API_KEY đang đè đăng nhập Claude')).toBeNull();
  });

  it('names how many when several override it, and sends to Cài đặt', async () => {
    await openTab('claude', withEnv([API_KEY, CLAUDE_BASE]));
    const banner = screen.getByText('2 biến đang đè đăng nhập Claude').closest('.banner') as HTMLElement;
    expect(within(banner).getByText(/ANTHROPIC_API_KEY ở ~\/\.zshrc dòng 42, ANTHROPIC_BASE_URL ở launchctl\./)).toBeTruthy();
    await press(within(banner).getByRole('button', { name: 'Xem trong Cài đặt' }));
    expect(screen.getByRole('heading', { level: 2, name: 'Cài đặt' })).toBeTruthy();
  });

  it('has no banner for a variable of another agent, or none', async () => {
    await openTab('claude', withEnv([BASE_URL, LAUNCHD_GEMINI]));
    expect(screen.queryByText(/đang đè đăng nhập Claude/)).toBeNull();
  });
});

describe('the alerts', () => {
  it('one card per variable, in the app only; "Bỏ qua" hides it and "Xem" opens Cài đặt', async () => {
    const recent = [envAlert(API_KEY), envAlert(BASE_URL, 1768547640)];
    const fake = await openTab('alerts', withEnv([API_KEY, BASE_URL], { alerts: alertsFixture({ recent, unanswered: 2 }) }));
    const key = screen.getByRole('group', { name: 'Cảnh báo: ANTHROPIC_API_KEY đang đè đăng nhập Claude Code' });
    expect(within(key).getByText(/chỉ có trong app/)).toBeTruthy();
    await press(within(key).getByRole('button', { name: 'Bỏ qua' }));
    expect(fake.api.alertAction).toHaveBeenCalledWith(recent[0].id, 'env:ignore:ANTHROPIC_API_KEY');
    expect(within(key).getByText('Đã bỏ qua ANTHROPIC_API_KEY')).toBeTruthy();
    expect(fake.current().env.found.map((f) => f.id)).toEqual(['OPENAI_BASE_URL@zprofile']);
    const base = screen.getByRole('group', { name: 'Cảnh báo: OPENAI_BASE_URL đang đè đăng nhập Codex' });
    await press(within(base).getByRole('button', { name: 'Xem' }));
    expect(screen.getByRole('heading', { level: 2, name: 'Cài đặt' })).toBeTruthy();
  });
});

/** The trap: `env-trap.json` is what the Rust side builds from shell files and launchd holding a planted value. */
describe('a planted value', () => {
  const TRAP = 'TRAP';
  const fromRust = () => {
    const found = trap.found as EnvFound[];
    const recent = trap.alerts as AlertRecord[];
    return fixtureModel('fresh', {
      env: envFixture({ found, unread: trap.unread as EnvPlace[] }),
      alerts: alertsFixture({ recent, unanswered: recent.length }),
    });
  };

  it('is not in what Rust sends the window', () => {
    expect(JSON.stringify(trap)).not.toContain(TRAP);
    expect(trap.found.length).toBe(11 * 6 - 1);
    expect(trap.alerts.length).toBe(11);
  });

  for (const tab of ['settings', 'claude', 'alerts'] as Tab[]) {
    it('never reaches the ' + tab + ' tab', async () => {
      await openTab(tab, fromRust());
      expect(document.body.textContent).toContain('ANTHROPIC_API_KEY');
      if (tab === 'settings') await press(within(card()).getByRole('button', { name: 'Xem danh sách' }));
      expect(document.body.innerHTML).not.toContain(TRAP);
    });
  }
});
