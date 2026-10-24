// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// Switching Antigravity from the popover and the main window (M4c), on the
// fake Rust side: nothing here writes a Keychain item or touches
// Antigravity.app.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp, PopoverApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { agyAccount, agyFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { AgyOverwrite, AgyStatus, Model } from '../lib/model';

/**
 * The fresh fixture with lanpham.ultra and lan.ultra2 logged in here and listed on
 * the Antigravity row; lan.ultra2 is in use.
 */
function listed(over: { status?: AgyStatus; overwritten?: AgyOverwrite } = {}): Model {
  return fixtureModel('fresh', {
    antigravity: agyFixture({ accounts: [agyAccount(1, { status: over.status ?? 'ok' }), agyAccount(2)] }),
    agySwitch: { busy: null, listed: true, overwritten: over.overwritten ?? null },
  });
}

async function openPopover(model: Model, options: FakeOptions = {}) {
  const fake = fakeApi(model, 'overview', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-popover">
      <PopoverApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('dialog', { name: 'Quota Station' });
  return fake;
}

const agyRows = () => within(screen.getByRole('list', { name: 'Antigravity' })).getAllByRole('button');
const ultra1 = () => agyRows().find((b) => /^lanpham\.ultra[,.]/.test(b.getAttribute('aria-label')!))!;
const ultra2 = () => agyRows().find((b) => /^lan\.ultra2[,.]/.test(b.getAttribute('aria-label')!))!;

async function sheetForUltra1() {
  await act(async () => {
    fireEvent.click(ultra1());
  });
  return screen.getByRole('dialog', { name: 'Chuyển Antigravity sang lanpham.ultra?' });
}

const facts = (sheet: HTMLElement) => within(sheet).getAllByRole('listitem').map((li) => li.textContent);
const buttons = (sheet: HTMLElement) => within(sheet).getAllByRole('button').map((b) => b.textContent);

afterEach(cleanup);

describe('the Antigravity switch sheet', () => {
  it('a row of an account logged in here opens it, with the three buttons while Antigravity.app runs', async () => {
    const fake = await openPopover(listed());
    expect(ultra1().getAttribute('aria-disabled')).toBeNull();
    expect(ultra1().getAttribute('aria-label')).toMatch(/^lanpham\.ultra\. Gemini 5 giờ đã dùng .*\. Bấm để chuyển$/);
    expect(ultra1().getAttribute('title')).toBe('lanpham.ultra@example.com · Quota Station · tài khoản 1');
    const sheet = await sheetForUltra1();
    expect(fake.api.agyAppState).toHaveBeenCalledTimes(1);
    expect(facts(sheet)).toEqual([
      'lanpham.ultra đã dùng 37% Gemini tuần, 18% Claude/GPT tuần.',
      'Antigravity.app đang chạy. App đang chạy có thể ghi lại tài khoản cũ, nên thoát app rồi chuyển là chắc nhất.',
      'Phiên agy đang chạy giữ tài khoản cũ cho tới khi khởi động lại.',
    ]);
    expect(buttons(sheet)).toEqual(['Thôi', 'Chuyển, để app chạy', 'Thoát app rồi chuyển']);
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thôi' }));
    });
    expect(screen.queryByRole('dialog', { name: /Chuyển Antigravity/ })).toBeNull();
    expect(fake.api.switchAgy).not.toHaveBeenCalled();
  });

  it('"Thoát app rồi chuyển" switches that way and reports the check', async () => {
    const fake = await openPopover(listed());
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    expect(fake.api.switchAgy).toHaveBeenCalledWith('antigravity_fake1', 'quitFirst');
    expect(screen.queryByRole('dialog', { name: /Chuyển Antigravity/ })).toBeNull();
    // Done with nothing to answer: a toast, the check on hover.
    const toast = screen.getByText('Đã chuyển Antigravity sang lanpham.ultra');
    expect(toast.getAttribute('role')).toBe('status');
    expect(toast.getAttribute('title')).toBe(
      'Đọc lại sau 5 giây: agy đang dùng lanpham.ultra ✓. Phiên agy đang chạy vẫn dùng tài khoản cũ cho tới khi khởi động lại.',
    );
    // The model the fake pushed: lanpham.ultra is in use.
    expect(ultra1().getAttribute('aria-label')).toMatch(/đang dùng/);
  });

  it('"Chuyển, để app chạy" warns that the running app may write the old account back', async () => {
    const fake = await openPopover(listed());
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển, để app chạy' }));
    });
    expect(fake.api.switchAgy).toHaveBeenCalledWith('antigravity_fake1', 'keepRunning');
    // Not a passing toast: the line stays with the warning in view.
    expect(screen.getByText('Đã chuyển Antigravity sang lanpham.ultra.')).toBeTruthy();
    expect(screen.getByText(/Antigravity.app vẫn chạy và có thể ghi lại tài khoản cũ; app sẽ báo nếu bị ghi đè trong 6 giờ tới\./)).toBeTruthy();
  });

  it('with Antigravity.app not running there is only "Chuyển"', async () => {
    const fake = await openPopover(listed(), { agyApp: { installed: 'supported', version: '2.24.0', running: false } });
    const sheet = await sheetForUltra1();
    expect(facts(sheet)[1]).toBe('Antigravity.app không chạy.');
    expect(buttons(sheet)).toEqual(['Thôi', 'Chuyển']);
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển' }));
    });
    expect(fake.api.switchAgy).toHaveBeenCalledWith('antigravity_fake1', 'plain');
  });

  it('without Antigravity.app only agy changes', async () => {
    await openPopover(listed(), { agyApp: { installed: 'missing', version: null, running: false } });
    const sheet = await sheetForUltra1();
    expect(facts(sheet)[1]).toBe('Máy này không có Antigravity.app: chỉ đổi tài khoản của agy.');
    expect(buttons(sheet)).toEqual(['Thôi', 'Chuyển']);
  });

  it('an Antigravity.app older than 2.0 is named and cannot be switched', async () => {
    await openPopover(listed(), { agyApp: { installed: 'tooOld', version: '1.20.5', running: true } });
    const sheet = await sheetForUltra1();
    expect(facts(sheet)[1]).toBe(
      'Antigravity.app bản 1.20.5 quá cũ: app chỉ chuyển được Antigravity.app 2.0 trở lên. Cập nhật Antigravity.app rồi thử lại.',
    );
    expect(buttons(sheet)).toEqual(['Thôi']);
  });

  it('an Antigravity.app too old stays blocked when its process list cannot be read', async () => {
    await openPopover(listed(), { agyApp: { installed: 'tooOld', version: '1.20.5', running: null } });
    const sheet = await sheetForUltra1();
    expect(facts(sheet)[1]).toMatch(/^Antigravity\.app bản 1\.20\.5 quá cũ/);
    expect(buttons(sheet)).toEqual(['Thôi']);
  });

  it('while it asks about Antigravity.app it shows both choices, switched off', async () => {
    const fake = await openPopover(listed());
    vi.mocked(fake.api.agyAppState).mockReturnValue(new Promise(() => {}));
    const sheet = await sheetForUltra1();
    expect(facts(sheet)[1]).toBe('Đang xem Antigravity.app có đang chạy không…');
    expect(buttons(sheet)).toEqual(['Thôi', 'Chuyển, để app chạy', 'Thoát app rồi chuyển']);
    for (const name of ['Chuyển, để app chạy', 'Thoát app rồi chuyển']) {
      expect((within(sheet).getByRole('button', { name }) as HTMLButtonElement).disabled).toBe(true);
    }
  });

  it('an Antigravity.app whose version cannot be read is named and cannot be switched', async () => {
    await openPopover(listed(), { agyApp: { installed: 'unreadable', version: null, running: true } });
    const sheet = await sheetForUltra1();
    expect(facts(sheet)[1]).toMatch(/^Không đọc được phiên bản Antigravity\.app/);
    expect(buttons(sheet)).toEqual(['Thôi']);
  });

  it('a version found unreadable during the switch is reported, nothing changed', async () => {
    await openPopover(listed(), {
      agyOutcome: (number) => ({ kind: 'failed', number, error: 'appVersion', detail: 'plutil exited Some(1): file does not exist', reopened: null }),
    });
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    expect(
      screen.getByText('Không đọc được phiên bản Antigravity.app (plutil exited Some(1): file does not exist), nên app không chuyển. Chưa đổi gì.'),
    ).toBeTruthy();
    expect(ultra2().getAttribute('aria-label')).toMatch(/đang dùng/);
  });

  it('a switch while an Antigravity logout runs says so', async () => {
    const fake = await openPopover(listed());
    const sheet = await sheetForUltra1();
    vi.mocked(fake.api.switchAgy).mockRejectedValue('loggingOut');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    expect(screen.getByText('Đang đăng xuất một tài khoản Antigravity. Chờ xong rồi thử lại.')).toBeTruthy();
  });

  it('a switch while an Antigravity login runs says so', async () => {
    const fake = await openPopover(listed());
    const sheet = await sheetForUltra1();
    vi.mocked(fake.api.switchAgy).mockRejectedValue('loggingIn');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    expect(screen.getByText('Đang có một lần đăng nhập Antigravity. Chờ xong rồi thử lại.')).toBeTruthy();
  });

  it('when the app cannot tell whether Antigravity.app runs it offers all three', async () => {
    await openPopover(listed(), { agyApp: null });
    const sheet = await sheetForUltra1();
    expect(facts(sheet)[1]).toBe('Không biết Antigravity.app có đang chạy không.');
    expect(buttons(sheet)).toEqual(['Thôi', 'Chuyển, để app chạy', 'Thoát app rồi chuyển']);
  });

  it('shows the switch running until the result is in', async () => {
    await openPopover(listed(), { agySwitchDelayMs: 50 });
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    const working = screen.getByRole('dialog', { name: 'Đang chuyển Antigravity sang lanpham.ultra…' });
    expect(
      within(working).getByText(
        'Thoát Antigravity.app, ghi đăng nhập dùng chung của Antigravity.app và agy, mở lại app, rồi đợi 5 giây để đọc lại tài khoản agy đang dùng.',
      ),
    ).toBeTruthy();
    await act(async () => {
      await new Promise((r) => setTimeout(r, 80));
    });
    expect(screen.getByText('Đã chuyển Antigravity sang lanpham.ultra')).toBeTruthy();
  });

  it('agy still on the old account reads as not confirmed', async () => {
    await openPopover(listed(), {
      agyOutcome: (number, how) => ({ kind: 'unconfirmed', number, how, active: 2, reopened: true, saved: true }),
    });
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    expect(screen.getByText('Đã ghi tài khoản lanpham.ultra cho Antigravity')).toBeTruthy();
    expect(
      screen.getByText(
        'Nhưng đọc lại sau 5 giây, agy vẫn đang dùng lan.ultra2. Antigravity.app hoặc phiên agy đang chạy có thể vẫn giữ tài khoản cũ.',
      ),
    ).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Làm mới' })).toBeTruthy();
  });

  it('an app that does not quit is reported, nothing written, and "Thử lại" opens the sheet again', async () => {
    await openPopover(listed(), {
      agyOutcome: (number) => ({ kind: 'failed', number, error: 'notQuit', detail: '', reopened: null }),
    });
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    expect(screen.getByText('Chưa chuyển được Antigravity sang lanpham.ultra')).toBeTruthy();
    expect(
      screen.getByText(/Antigravity.app không thoát trong 20 giây, có thể đang hỏi lưu việc\. App không ép thoát và chưa đổi đăng nhập nào\./),
    ).toBeTruthy();
    expect(ultra2().getAttribute('aria-label')).toMatch(/đang dùng/);
    expect(ultra1().getAttribute('aria-label')).not.toMatch(/đang dùng/);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    });
    expect(screen.getByRole('dialog', { name: 'Chuyển Antigravity sang lanpham.ultra?' })).toBeTruthy();
  });

  it('a failed write keeps the old login, says the app is back, and offers "Thử lại"', async () => {
    await openPopover(listed(), {
      agyOutcome: (number) => ({ kind: 'failed', number, error: 'write', detail: 'security exited 36: User interaction is not allowed.', reopened: true }),
    });
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    expect(
      screen.getByText(
        'Không ghi được đăng nhập dùng chung vào Keychain (security exited 36: User interaction is not allowed.). Đăng nhập cũ vẫn giữ nguyên. Antigravity.app đã mở lại.',
      ),
    ).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Thử lại' })).toBeTruthy();
    expect(ultra2().getAttribute('aria-label')).toMatch(/đang dùng/);
  });

  it('a write that timed out says so and keeps the old login', async () => {
    await openPopover(listed(), {
      agyApp: { installed: 'supported', version: '2.24.0', running: false },
      agyOutcome: (number) => ({ kind: 'failed', number, error: 'writeTimeout', detail: '', reopened: null }),
    });
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển' }));
    });
    expect(screen.getByText('Không ghi được đăng nhập dùng chung vào Keychain (security không trả lời trong 15 giây). Đăng nhập cũ vẫn giữ nguyên.')).toBeTruthy();
    expect(ultra2().getAttribute('aria-label')).toMatch(/đang dùng/);
  });

  it('an app opened again after the sheet looked stops "Chuyển", and "Thử lại" asks again', async () => {
    const fake = await openPopover(listed(), {
      agyApp: { installed: 'supported', version: '2.24.0', running: false },
      agyOutcome: (number) => ({ kind: 'failed', number, error: 'appRunning', detail: '', reopened: null }),
    });
    const sheet = await sheetForUltra1();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển' }));
    });
    expect(screen.getByText(/^Antigravity\.app vừa mở lại sau khi bảng chuyển kiểm tra\. Chưa đổi gì\./)).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    });
    expect(screen.getByRole('dialog', { name: 'Chuyển Antigravity sang lanpham.ultra?' })).toBeTruthy();
    expect(fake.api.agyAppState).toHaveBeenCalledTimes(2);
  });

  it('a switch started in the other window shows without guessing its button', async () => {
    const fake = await openPopover(listed());
    await sheetForUltra1();
    await act(async () => {
      fake.push({ ...fake.current(), agySwitch: { ...fake.current().agySwitch, busy: 1 } });
    });
    const working = screen.getByRole('dialog', { name: 'Đang chuyển Antigravity sang lanpham.ultra…' });
    expect(
      within(working).getByText('Ghi đăng nhập dùng chung của Antigravity.app và agy, rồi đợi 5 giây để đọc lại tài khoản agy đang dùng.'),
    ).toBeTruthy();
  });

  it('a switch already running is refused as busy', async () => {
    const fake = await openPopover(listed());
    const sheet = await sheetForUltra1();
    fake.push({ ...fake.current(), agySwitch: { ...fake.current().agySwitch, busy: 2 } });
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Thoát app rồi chuyển' }));
    });
    expect(screen.getByText('Một lần chuyển khác đang chạy. Chờ nó xong rồi thử lại.')).toBeTruthy();
  });

  it('an account that needs a new login does not switch', async () => {
    const fake = await openPopover(listed({ status: 'needsLogin' }));
    expect(ultra1().getAttribute('aria-disabled')).toBe('true');
    expect(ultra1().getAttribute('title')).toBe('lanpham.ultra@example.com · Quota Station · tài khoản 1 · Cần đăng nhập lại trong tab Antigravity');
    fireEvent.click(ultra1());
    expect(screen.queryByRole('dialog', { name: /Chuyển Antigravity/ })).toBeNull();
    expect(fake.api.agyAppState).not.toHaveBeenCalled();
  });

  it('a row of an account not logged in here does not switch', async () => {
    await openPopover(
      fixtureModel('fresh', {
        antigravity: agyFixture({ accounts: [agyAccount(2)] }),
        agySwitch: { busy: null, listed: true, overwritten: null },
      }),
    );
    expect(ultra1().getAttribute('aria-disabled')).toBe('true');
    expect(ultra1().getAttribute('title')).toBe('lanpham.ultra@example.com · Quota Station · tài khoản 1 · Chưa đăng nhập tài khoản này trong Quota Station');
  });
});

describe('the overwrite banner', () => {
  const overwritten: AgyOverwrite = { to: 1, from: 2, at: '2026-01-16T07:10:00.000Z', overwrites: 1 };

  it('is not in the popover: the macOS card with its buttons replaces it', async () => {
    await openPopover(listed({ overwritten }));
    expect(screen.queryByText('Chuyển Antigravity bị ghi đè')).toBeNull();
  });

  it('on the Antigravity tab, "Chuyển lại" opens the sheet again', async () => {
    const fake = fakeApi(listed({ overwritten }), 'agy');
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('navigation', { name: 'Mục' });
    const banner = screen.getByText('Chuyển Antigravity bị ghi đè').closest('.banner') as HTMLElement;
    expect(
      within(banner).getByText(
        'Lúc 14:10 agy lại dùng lan.ultra2 sau khi chuyển sang lanpham.ultra. Antigravity.app đang chạy có thể đã ghi lại đăng nhập cũ.',
      ),
    ).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(banner).getByRole('button', { name: 'Chuyển lại' }));
    });
    expect(screen.getByRole('dialog', { name: 'Chuyển Antigravity sang lanpham.ultra?' })).toBeTruthy();
    expect(fake.api.agyAppState).toHaveBeenCalledTimes(1);
  });

  it('on the Antigravity tab, "Thôi" stops the watch and the banner goes', async () => {
    const fake = fakeApi(listed({ overwritten: { ...overwritten, overwrites: 2 } }), 'agy');
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('navigation', { name: 'Mục' });
    expect(screen.getByText(/Đây là lần thứ 2\./)).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Thôi' }));
    });
    expect(fake.api.agyDismissOverwrite).toHaveBeenCalledTimes(1);
    expect(screen.queryByText('Chuyển Antigravity bị ghi đè')).toBeNull();
  });

  it('shows on the main window\'s Antigravity tab too, and its rows switch', async () => {
    const fake = fakeApi(listed({ overwritten }), 'agy');
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('navigation', { name: 'Mục' });
    expect(screen.getByText('Chuyển Antigravity bị ghi đè')).toBeTruthy();
    const button = screen.getByRole('button', { name: 'Chuyển sang lanpham.ultra' }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
    await act(async () => {
      fireEvent.click(button);
    });
    expect(screen.getByRole('dialog', { name: 'Chuyển Antigravity sang lanpham.ultra?' })).toBeTruthy();
  });
});
