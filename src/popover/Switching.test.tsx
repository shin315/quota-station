// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The switch sheet (F04) and the pause control (F07) in the popover, on the
// fake Rust side: nothing here runs claude-swap or writes a pause flag.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PopoverApp } from '../App';
import { MORE_KEY } from './Popover';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel, heldSwitcher, switcherFixture } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model, Outcome } from '../lib/model';

async function open(model: Model, options: FakeOptions = {}, clock = () => FIXTURE_NOW) {
  const fake = fakeApi(model, 'overview', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-popover">
      <PopoverApp api={fake.api} clock={clock} />
    </div>,
  );
  await screen.findByRole('dialog', { name: 'Quota Station' });
  return fake;
}

/** Opens the switch sheet from a Claude row and waits for the session count. */
async function sheetFor(name: string) {
  const row = within(screen.getByRole('list', { name: 'Claude' }))
    .getAllByRole('button')
    .find((b) => /^[,.]/.test(b.getAttribute('aria-label')!.slice(name.length)) && b.getAttribute('aria-label')!.startsWith(name))!;
  await act(async () => {
    fireEvent.click(row);
  });
  return screen.getByRole('dialog', { name: 'Chuyển Claude sang ' + name + '?' });
}

const facts = (sheet: HTMLElement) => within(sheet).getAllByRole('listitem').map((li) => li.textContent);
const radios = (sheet: HTMLElement) =>
  within(sheet)
    .getAllByRole('radio')
    .map((r) => (r.getAttribute('aria-checked') === 'true' ? '● ' : '○ ') + r.textContent);

// Every section shows its other accounts, as the owner opens them.
beforeEach(() => localStorage.setItem(MORE_KEY, JSON.stringify({ c: true, g: true, x: true })));
afterEach(() => {
  cleanup();
  localStorage.removeItem(MORE_KEY);
});

describe('the Claude switch sheet', () => {
  it('names the target, counts the live sessions and offers the four holds, one hour by default', async () => {
    const fake = await open(fixtureModel('fresh'));
    const sheet = await sheetFor('lan.acme2');
    expect(fake.api.claudeSessions).toHaveBeenCalledTimes(1);
    expect(facts(sheet)).toEqual([
      'lan.acme2 đã dùng 0% 5 giờ, 50% Fable.',
      '5 phiên Claude đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo. Việc này dùng thêm hạn mức.',
    ]);
    // lan.acme2's five-hour window has not started: "Đến lúc đặt lại" is five hours from now.
    expect(radios(sheet)).toEqual([
      '○ 30 phútđến 14:50',
      '● 1 giờđến 15:20',
      '○ Đến lúc đặt lạiđến 19:20',
      '○ Để tự chuyểnkhông giữ',
    ]);
    expect(within(sheet).getByText('Tạm dừng tự chuyển đến 15:20 để giữ tài khoản này.')).toBeTruthy();
    // The rest of the popover sits behind the sheet.
    expect(screen.getByRole('dialog', { name: 'Quota Station' }).querySelector('.body')!.hasAttribute('inert')).toBe(true);
  });

  it('"Đến lúc đặt lại" ends at the five-hour reset of an account whose window runs', async () => {
    await open(fixtureModel('fresh'));
    const sheet = await sheetFor('lan.acme3');
    expect(within(sheet).getByRole('radio', { name: /Đến lúc đặt lại/ }).textContent).toBe('Đến lúc đặt lạiđến 17:09');
    expect(facts(sheet)[0]).toBe('lan.acme3 đã dùng 2% 5 giờ, 2% Fable.');
  });

  it('confirming switches with the hold chosen and reports what the re-read shows', async () => {
    const fake = await open(fixtureModel('fresh'));
    const sheet = await sheetFor('lan.acme2');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    expect(fake.api.switchClaude).toHaveBeenCalledWith(3, '2026-01-16T08:20:40.000Z');
    expect(screen.queryByRole('dialog', { name: /Chuyển Claude/ })).toBeNull();
    expect(screen.getByText('Đã chuyển sang lan.acme2. Giữ tài khoản này đến 15:20.')).toBeTruthy();
    expect(screen.getByText('Đọc lại sau 5 giây: lan.acme2 đang dùng ✓. Tự chuyển Claude tạm dừng tới lúc đó.')).toBeTruthy();
    // The model the fake pushed: lan.acme2 is active and the switcher is held.
    expect(document.querySelector('[data-active]')!.getAttribute('aria-label')).toMatch(/^lan\.acme2,/);
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude, đang tạm dừng' }).getAttribute('aria-checked')).toBe('false');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    });
    expect(screen.queryByText(/Đã chuyển sang lan.acme2/)).toBeNull();
  });

  it('"Để tự chuyển" switches without a hold and says the switcher may switch again', async () => {
    const fake = await open(fixtureModel('fresh'));
    const sheet = await sheetFor('lan.acme2');
    fireEvent.click(within(sheet).getByRole('radio', { name: /Để tự chuyển/ }));
    expect(within(sheet).getByText('Tự chuyển vẫn bật. Switcher có thể đổi sang tài khoản khác ở lần kiểm sau (trong vòng 3 phút).')).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    expect(fake.api.switchClaude).toHaveBeenCalledWith(3, null);
    // Done with nothing to answer: a toast that says so, the details on hover.
    const toast = screen.getByText('Đã chuyển sang lan.acme2');
    expect(toast.getAttribute('role')).toBe('status');
    expect(toast.getAttribute('title')).toBe(
      'Đọc lại sau 5 giây: lan.acme2 đang dùng ✓. Tự chuyển vẫn bật: switcher kiểm lại trong vòng 3 phút và có thể đổi tài khoản.',
    );
  });

  it('the hold ends at the time its choice shows, even when the clock has moved on since', async () => {
    let now = FIXTURE_NOW;
    const fake = await open(fixtureModel('fresh'), {}, () => now);
    const sheet = await sheetFor('lan.acme2');
    expect(within(sheet).getByRole('radio', { name: /1 giờ/ }).textContent).toBe('1 giờđến 15:20');
    now = new Date('2026-01-16T07:21:50.000Z'); // a minute later, before the window's clock ticks
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    expect(fake.api.switchClaude).toHaveBeenCalledWith(3, '2026-01-16T08:20:40.000Z');
  });

  it('the arrow keys move the hold choice', async () => {
    await open(fixtureModel('fresh'));
    const sheet = await sheetFor('lan.acme2');
    const checked = within(sheet).getByRole('radio', { name: /1 giờ/ });
    // Radix moves to the next choice, and so picks it, a tick after the key.
    await act(async () => {
      fireEvent.keyDown(checked, { key: 'ArrowRight' });
      await new Promise((r) => setTimeout(r, 0));
    });
    expect(within(sheet).getByRole('radio', { name: /Đến lúc đặt lại/ }).getAttribute('aria-checked')).toBe('true');
    expect(within(sheet).getByText('Tạm dừng tự chuyển đến 19:20 để giữ tài khoản này.')).toBeTruthy();
  });

  it('shows the switch running until the result is in', async () => {
    let finish: (o: Outcome) => void = () => {};
    const fake = await open(fixtureModel('fresh'));
    vi.mocked(fake.api.switchClaude).mockImplementation(() => new Promise((r) => (finish = r)));
    const sheet = await sheetFor('lan.acme2');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    const working = screen.getByRole('dialog', { name: 'Đang chuyển sang lan.acme2…' });
    expect(within(working).getByText('Đang đổi đăng nhập Claude Code sang lan.acme2')).toBeTruthy();
    // Escape does not leave a running switch.
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.getByRole('dialog', { name: 'Đang chuyển sang lan.acme2…' })).toBeTruthy();
    expect(fake.api.hidePopover).not.toHaveBeenCalled();
    await act(async () => finish({ kind: 'switched', slot: 3, hold: null }));
    expect(screen.queryByRole('dialog', { name: /Đang chuyển/ })).toBeNull();
    expect(screen.getByText('Đã chuyển sang lan.acme2')).toBeTruthy();
  });

  it('when the switcher switches back, says so and offers to switch again', async () => {
    await open(fixtureModel('fresh'), {
      outcome: (slot, holdUntil) => ({
        kind: 'raced',
        slot,
        active: 2,
        bySwitcherAt: '2026-01-16T07:21:00.000Z',
        hold: holdUntil ? { until: '2026-01-16T08:20:00.000Z', failed: false } : null,
      }),
    });
    const sheet = await sheetFor('lan.acme2');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    expect(screen.getByText('Switcher vừa chuyển sang lan.acme1 lúc 14:21')).toBeTruthy();
    expect(screen.getByText('Hai lần ghi gần nhau, lần sau thắng. Đang dùng lan.acme1. Tự chuyển Claude tạm dừng tới 15:20.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Cho phép tự chuyển lại' })).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển lại sang lan.acme2' }));
    });
    expect(screen.getByRole('dialog', { name: 'Chuyển Claude sang lan.acme2?' })).toBeTruthy();
  });

  it('another switch winning is not blamed on the switcher', async () => {
    await open(fixtureModel('fresh'), { outcome: (slot) => ({ kind: 'raced', slot, active: 4, bySwitcherAt: null, hold: null }) });
    const sheet = await sheetFor('lan.acme2');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    expect(screen.getByText('Một lần chuyển khác vừa chuyển sang lan.acme3')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Cho phép tự chuyển lại' })).toBeNull();
  });

  it('a switch that does not land says nothing changed and offers to try again', async () => {
    await open(fixtureModel('fresh'), {
      outcome: (slot) => ({ kind: 'failed', slot, error: 'busy', detail: '', active: null }),
    });
    const sheet = await sheetFor('lan.acme2');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    expect(screen.getByText('Chưa chuyển được sang lan.acme2')).toBeTruthy();
    expect(screen.getByText('Claude Code đang giữ khoá đăng nhập (đang làm mới hoặc ghi đăng nhập). Thử lại sau vài giây. Chưa đổi gì.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Thử lại' })).toBeTruthy();
    // Nothing changed: lan.acme1 is still the one in use.
    expect(document.querySelector('[data-active]')!.getAttribute('aria-label')).toMatch(/^lan\.acme1, Acme Studio, đang dùng\./);
  });

  it('an unconfirmed switch asks for a refresh', async () => {
    await open(fixtureModel('fresh'), { outcome: (slot) => ({ kind: 'unconfirmed', slot, hold: null }) });
    const sheet = await sheetFor('lan.acme2');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    expect(screen.getByText('Đã gửi lệnh chuyển sang lan.acme2')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Làm mới' })).toBeTruthy();
  });

  it('Thôi and Escape close the sheet without switching, and Escape then hides the popover', async () => {
    const fake = await open(fixtureModel('fresh'));
    let sheet = await sheetFor('lan.acme2');
    fireEvent.click(within(sheet).getByRole('button', { name: 'Thôi' }));
    expect(screen.queryByRole('dialog', { name: /Chuyển Claude/ })).toBeNull();
    sheet = await sheetFor('lan.acme2');
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: /Chuyển Claude/ })).toBeNull();
    expect(fake.api.hidePopover).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(fake.api.hidePopover).toHaveBeenCalledTimes(1);
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });

  it('session counts: none, unreadable records, and a folder that cannot be read', async () => {
    await open(fixtureModel('fresh'), { sessions: { live: 0, unreadable: 0 } });
    expect(facts(await sheetFor('lan.acme2'))[1]).toBe(
      'Không có phiên Claude nào đang chạy trên máy này, nên không phiên nào phải tạo lại bộ nhớ đệm.',
    );
    cleanup();
    await open(fixtureModel('fresh'), { sessions: { live: 3, unreadable: 1 } });
    expect(facts(await sheetFor('lan.acme2'))[1]).toMatch(/^Ít nhất 3 phiên Claude đang chạy/);
    cleanup();
    await open(fixtureModel('fresh'), { sessions: null });
    expect(facts(await sheetFor('lan.acme2'))[1]).toMatch(/^Các phiên Claude đang chạy sẽ tạo lại bộ nhớ đệm/);
  });

  it('the personal account outside its hours warns that the switcher will move away', async () => {
    // 00:30 at UTC+7: outside the personal hours 8-23.
    await open(fixtureModel('fresh'), {}, () => new Date('2026-01-16T17:30:00.000Z'));
    const sheet = await sheetFor('lanpham.dev');
    expect(facts(sheet)).toContain(
      'Bây giờ là 00:30, ngoài giờ 08–23 của tài khoản Cá nhân. Nếu để tự chuyển, switcher sẽ đổi sang tài khoản khác trong vòng 3 phút.',
    );
    fireEvent.click(within(sheet).getByRole('radio', { name: /Để tự chuyển/ }));
    expect(within(sheet).getByText('Ngoài giờ dùng tài khoản Cá nhân. Chọn thời lượng giữ để tránh tự chuyển sang tài khoản khác.').getAttribute('data-tone')).toBe('warn');
  });

  it('before Quota Station keeps any account nothing can switch', async () => {
    await open(fixtureModel('fresh', { claudeSwitch: { ready: false, busy: null } }));
    const rows = within(screen.getByRole('list', { name: 'Claude' })).getAllByRole('button');
    expect(rows.every((r) => r.getAttribute('aria-disabled') === 'true')).toBe(true);
    expect(rows[2].getAttribute('aria-label')).toMatch(/\. Chưa chuyển được: Quota Station chưa giữ tài khoản Claude$/);
    expect(screen.getByText('Quota Station chưa giữ tài khoản Claude nào trên máy này: nhập từ claude-swap ở tab Claude để chuyển được tài khoản.')).toBeTruthy();
  });

  it('a switch from the other window that lands on this sheet’s account closes the sheet', async () => {
    const fake = await open(fixtureModel('fresh'));
    await sheetFor('lan.acme2');
    const model = fixtureModel('fresh');
    act(() => fake.push({ ...model, claudeSwitch: { ready: true, busy: 3 } }));
    expect(screen.getByRole('dialog', { name: 'Đang chuyển sang lan.acme2…' })).toBeTruthy();
    const accounts = model.snapshot.claude.accounts.map((a) => ({ ...a, active: a.number === 3 }));
    act(() => fake.push({ ...model, snapshot: { ...model.snapshot, claude: { ...model.snapshot.claude, accounts } } }));
    expect(screen.queryByRole('dialog', { name: /lan.acme2/ })).toBeNull();
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });

  it('while a switch runs from another window, rows wait', async () => {
    await open(fixtureModel('fresh', { claudeSwitch: { ready: true, busy: 3 } }));
    const rows = within(screen.getByRole('list', { name: 'Claude' })).getAllByRole('button');
    expect(rows[2].getAttribute('aria-label')).toMatch(/\. Đang chuyển tài khoản$/);
  });
});

describe('the pause control', () => {
  it('the toggle opens the pause sheet; pausing holds the active account for the time chosen', async () => {
    const fake = await open(fixtureModel('fresh'));
    fireEvent.click(screen.getByRole('switch', { name: 'Tự chuyển Claude' }));
    const sheet = screen.getByRole('dialog', { name: 'Tạm dừng tự chuyển Claude?' });
    expect(within(sheet).getByText('Giữ lan.acme1. Switcher sẽ không đổi tài khoản Claude trong thời gian này. Hạn mức vẫn cập nhật.')).toBeTruthy();
    expect(radios(sheet)).toEqual(['● 1 giờđến 15:20', '○ Đến 18:00hôm nay', '○ Đến khi tôi bật lạikhông có giờ hết hạn']);
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Tạm dừng' }));
    });
    expect(fake.api.pauseSwitching).toHaveBeenCalledWith('2026-01-16T08:20:40.000Z');
    expect(screen.getByText('Đã tạm dừng tự chuyển Claude')).toBeTruthy();
    expect(screen.getByText('Giữ lan.acme1 đến 15:20. Hết thời gian này, tự chuyển sẽ bật lại.')).toBeTruthy();
    // The why line and the toggle say so; the banner waits for the line to close.
    expect(screen.getByText('Tự chuyển đang tạm dừng')).toBeTruthy();
    expect(screen.getByText('Đang tạm dừng tới 15:20: tạm dừng trong Quota Station lúc 14:20')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude, đang tạm dừng' }).getAttribute('aria-checked')).toBe('false');
    expect(screen.queryByText('Đang giữ lan.acme1 đến 15:20')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    expect(screen.getByText('Đang giữ lan.acme1 đến 15:20')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Thêm 1 giờ' })).toBeTruthy();
  });

  it('the pause ends at the time its choice shows, even when the clock has moved on since', async () => {
    let now = FIXTURE_NOW;
    const fake = await open(fixtureModel('fresh'), {}, () => now);
    fireEvent.click(screen.getByRole('switch', { name: 'Tự chuyển Claude' }));
    const sheet = screen.getByRole('dialog', { name: 'Tạm dừng tự chuyển Claude?' });
    now = new Date('2026-01-16T07:21:50.000Z');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Tạm dừng' }));
    });
    expect(fake.api.pauseSwitching).toHaveBeenCalledWith('2026-01-16T08:20:40.000Z');
  });

  it('"Đến khi tôi bật lại" pauses with no end and no "Thêm 1 giờ"', async () => {
    const fake = await open(fixtureModel('fresh'));
    fireEvent.click(screen.getByRole('switch', { name: 'Tự chuyển Claude' }));
    const sheet = screen.getByRole('dialog', { name: 'Tạm dừng tự chuyển Claude?' });
    fireEvent.click(within(sheet).getByRole('radio', { name: /Đến khi tôi bật lại/ }));
    expect(within(sheet).getByText('Tự chuyển chỉ bật lại khi bạn bật công tắc. Firstmate vẫn thấy hạn mức trong vòng.')).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Tạm dừng' }));
    });
    expect(fake.api.pauseSwitching).toHaveBeenCalledWith(null);
    expect(screen.getByText('Giữ lan.acme1 đến khi bạn bật lại. Bật lại bằng công tắc Tự chuyển Claude.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    expect(screen.getByText('Đang giữ lan.acme1 đến khi bạn bật lại')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Thêm 1 giờ' })).toBeNull();
    expect(screen.getByText('Đang tạm dừng đến khi bạn bật lại: tạm dừng trong Quota Station lúc 14:20')).toBeTruthy();
  });

  it('"Đến 18:00" confirmed after 18:00 shows the choices again instead of pausing with no end', async () => {
    let now = new Date('2026-01-16T10:59:50.000Z'); // 17:59:50
    const fake = await open(fixtureModel('fresh'), {}, () => now);
    fireEvent.click(screen.getByRole('switch', { name: 'Tự chuyển Claude' }));
    let sheet = screen.getByRole('dialog', { name: 'Tạm dừng tự chuyển Claude?' });
    fireEvent.click(within(sheet).getByRole('radio', { name: /Đến 18:00/ }));
    now = new Date('2026-01-16T11:00:05.000Z');
    fireEvent.click(within(sheet).getByRole('button', { name: 'Tạm dừng' }));
    expect(fake.api.pauseSwitching).not.toHaveBeenCalled();
    sheet = screen.getByRole('dialog', { name: 'Tạm dừng tự chuyển Claude?' });
    expect(radios(sheet)).toEqual(['● 1 giờđến 19:00', '○ Đến khi tôi bật lạikhông có giờ hết hạn']);
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Tạm dừng' }));
    });
    expect(fake.api.pauseSwitching).toHaveBeenCalledWith('2026-01-16T12:00:05.000Z');
  });

  it('after 18:00 there is no "Đến 18:00"', async () => {
    await open(fixtureModel('fresh'), {}, () => new Date('2026-01-16T11:30:00.000Z'));
    fireEvent.click(screen.getByRole('switch', { name: 'Tự chuyển Claude' }));
    expect(radios(screen.getByRole('dialog', { name: 'Tạm dừng tự chuyển Claude?' }))).toEqual([
      '● 1 giờđến 19:30',
      '○ Đến khi tôi bật lạikhông có giờ hết hạn',
    ]);
  });

  it('while paused, the toggle resumes at once and "Thêm 1 giờ" moves the end', async () => {
    const fake = await open(fixtureModel('fresh', { switcher: heldSwitcher() }));
    expect(screen.getByText('Đang tạm dừng tới 15:20: chuyển tay sang tài khoản 2 lúc 14:12')).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Thêm 1 giờ' }));
    });
    expect(fake.api.extendPause).toHaveBeenCalledWith(60);
    expect(screen.getByText('Đã thêm 1 giờ. Tự chuyển Claude vẫn tạm dừng.')).toBeTruthy();
    expect(screen.getAllByText('Giữ lan.acme1 đến 16:20')).toHaveLength(1);
    await act(async () => {
      fireEvent.click(screen.getByRole('switch', { name: 'Tự chuyển Claude, đang tạm dừng' }));
    });
    expect(fake.api.resumeSwitching).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Tự chuyển Claude đã bật lại')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude' }).getAttribute('aria-checked')).toBe('true');
  });

  it('a pause whose end has passed no longer counts', async () => {
    await open(fixtureModel('fresh', { switcher: heldSwitcher() }), {}, () => new Date('2026-01-16T08:21:00.000Z'));
    expect(screen.queryByText(/Đang giữ/)).toBeNull();
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude' }).getAttribute('aria-checked')).toBe('true');
  });

  it('a switcher that ignores the pause flag is reported, not shown as paused', async () => {
    const fake = await open(
      fixtureModel('fresh', {
        switcher: heldSwitcher({ dryRun: { decision: { kind: 'stay' }, raw: 'stay', at: '2026-01-16T07:19:40.000Z' }, guard: false }),
      }),
    );
    expect(screen.getByText('Switcher chưa đọc cờ tạm dừng')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude' }).getAttribute('aria-checked')).toBe('true');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Cho phép tự chuyển lại' }));
    });
    expect(fake.api.resumeSwitching).toHaveBeenCalledTimes(1);
  });

  it('claude-swap’s own switcher loaded again is a warning with a guide', async () => {
    await open(fixtureModel('fresh', { switcher: switcherFixture({ legacy: 'com.cswap.prefer-team' }) }));
    expect(screen.getByText('Bộ tự chuyển cũ còn chạy')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Hướng dẫn tắt' }));
    const guide = screen.getByRole('dialog', { name: 'Tắt bộ tự chuyển cũ' });
    expect(within(guide).getByText('launchctl bootout gui/$(id -u)/com.cswap.prefer-team')).toBeTruthy();
    fireEvent.click(within(guide).getByRole('button', { name: 'Đóng' }));
    expect(screen.queryByRole('dialog', { name: /Tắt bộ/ })).toBeNull();
  });

  it('a Mac without the switcher cannot pause', async () => {
    await open(fixtureModel('fresh', { switcher: switcherFixture({ installed: false, dryRun: null, lastLog: null }) }));
    const toggle = screen.getByRole('switch', { name: 'Tự chuyển Claude' }) as HTMLButtonElement;
    expect(toggle.disabled).toBe(true);
    expect(screen.getByText('Quota Station chưa giữ tài khoản Claude nào trên máy này: chưa có tự chuyển Claude.')).toBeTruthy();
  });
});
