// The switch sheet (F04) and the pause control (F07) in the popover, on the
// fake Rust side: nothing here runs claude-swap or writes a pause flag.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PopoverApp } from '../App';
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
  const row = within(screen.getByRole('list', { name: 'CLAUDE CODE' }))
    .getAllByRole('button')
    .find((b) => b.getAttribute('aria-label')!.startsWith(name + ' '))!;
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

afterEach(cleanup);

describe('the Claude switch sheet', () => {
  it('names the target, counts the live sessions and offers the four holds, one hour by default', async () => {
    const fake = await open(fixtureModel('fresh'));
    const sheet = await sheetFor('Team B');
    expect(fake.api.claudeSessions).toHaveBeenCalledTimes(1);
    expect(facts(sheet)).toEqual([
      'Team B còn 100% hạn mức 5 giờ, 50% hạn mức Fable tuần.',
      '5 phiên Claude đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo. Việc này dùng thêm hạn mức.',
    ]);
    // Team B's five-hour window has not started: "Đến lúc đặt lại" is five hours from now.
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
    const sheet = await sheetFor('Team C');
    expect(within(sheet).getByRole('radio', { name: /Đến lúc đặt lại/ }).textContent).toBe('Đến lúc đặt lạiđến 17:09');
    expect(facts(sheet)[0]).toBe('Team C còn 98% hạn mức 5 giờ, ≥98% hạn mức Fable tuần.');
  });

  it('confirming switches with the hold chosen and reports what the re-read shows', async () => {
    const fake = await open(fixtureModel('fresh'));
    const sheet = await sheetFor('Team B');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    expect(fake.api.switchClaude).toHaveBeenCalledWith(3, '2026-01-16T08:20:40.000Z');
    expect(screen.queryByRole('dialog', { name: /Chuyển Claude/ })).toBeNull();
    expect(screen.getByText('Đã chuyển sang Team B. Giữ tài khoản này đến 15:20.')).toBeTruthy();
    expect(screen.getByText('Đọc lại sau 5 giây: Team B đang dùng ✓. Tự chuyển Claude tạm dừng tới lúc đó.')).toBeTruthy();
    // The model the fake pushed: Team B is active and the switcher is held.
    expect(screen.getByText('Giữ Team B đến 15:20')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude, đang tạm dừng' }).getAttribute('aria-checked')).toBe('false');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    });
    expect(screen.queryByText(/Đã chuyển sang Team B/)).toBeNull();
  });

  it('"Để tự chuyển" switches without a hold and says the switcher may switch again', async () => {
    const fake = await open(fixtureModel('fresh'));
    const sheet = await sheetFor('Team B');
    fireEvent.click(within(sheet).getByRole('radio', { name: /Để tự chuyển/ }));
    expect(within(sheet).getByText('Tự chuyển vẫn bật. Switcher có thể đổi sang tài khoản khác ở lần kiểm sau (trong vòng 3 phút).')).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    expect(fake.api.switchClaude).toHaveBeenCalledWith(3, null);
    expect(screen.getByText('Đã chuyển sang Team B.')).toBeTruthy();
    expect(
      screen.getByText('Đọc lại sau 5 giây: Team B đang dùng ✓. Tự chuyển vẫn bật: switcher kiểm lại trong vòng 3 phút và có thể đổi tài khoản.'),
    ).toBeTruthy();
  });

  it('the hold ends at the time its choice shows, even when the clock has moved on since', async () => {
    let now = FIXTURE_NOW;
    const fake = await open(fixtureModel('fresh'), {}, () => now);
    const sheet = await sheetFor('Team B');
    expect(within(sheet).getByRole('radio', { name: /1 giờ/ }).textContent).toBe('1 giờđến 15:20');
    now = new Date('2026-01-16T07:21:50.000Z'); // a minute later, before the window's clock ticks
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    expect(fake.api.switchClaude).toHaveBeenCalledWith(3, '2026-01-16T08:20:40.000Z');
  });

  it('the arrow keys move the hold choice', async () => {
    await open(fixtureModel('fresh'));
    const sheet = await sheetFor('Team B');
    const checked = within(sheet).getByRole('radio', { name: /1 giờ/ });
    fireEvent.keyDown(checked, { key: 'ArrowRight' });
    expect(within(sheet).getByRole('radio', { name: /Đến lúc đặt lại/ }).getAttribute('aria-checked')).toBe('true');
    expect(within(sheet).getByText('Tạm dừng tự chuyển đến 19:20 để giữ tài khoản này.')).toBeTruthy();
  });

  it('shows the switch running until the result is in', async () => {
    let finish: (o: Outcome) => void = () => {};
    const fake = await open(fixtureModel('fresh'));
    vi.mocked(fake.api.switchClaude).mockImplementation(() => new Promise((r) => (finish = r)));
    const sheet = await sheetFor('Team B');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    const working = screen.getByRole('dialog', { name: 'Đang chuyển sang Team B…' });
    expect(within(working).getByText('cswap switch 3 đang chuyển tài khoản')).toBeTruthy();
    // Escape does not leave a running switch.
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.getByRole('dialog', { name: 'Đang chuyển sang Team B…' })).toBeTruthy();
    expect(fake.api.hidePopover).not.toHaveBeenCalled();
    await act(async () => finish({ kind: 'switched', slot: 3, hold: null }));
    expect(screen.queryByRole('dialog', { name: /Đang chuyển/ })).toBeNull();
    expect(screen.getByText('Đã chuyển sang Team B.')).toBeTruthy();
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
    const sheet = await sheetFor('Team B');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    expect(screen.getByText('Switcher vừa chuyển sang Team A lúc 14:21')).toBeTruthy();
    expect(screen.getByText('Hai lần ghi gần nhau, lần sau thắng. Đang dùng Team A. Tự chuyển Claude tạm dừng tới 15:20.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Cho phép tự chuyển lại' })).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Chuyển lại sang Team B' }));
    });
    expect(screen.getByRole('dialog', { name: 'Chuyển Claude sang Team B?' })).toBeTruthy();
  });

  it('another switch winning is not blamed on the switcher', async () => {
    await open(fixtureModel('fresh'), { outcome: (slot) => ({ kind: 'raced', slot, active: 4, bySwitcherAt: null, hold: null }) });
    const sheet = await sheetFor('Team B');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    expect(screen.getByText('Một lần chuyển khác vừa chuyển sang Team C')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Cho phép tự chuyển lại' })).toBeNull();
  });

  it('an error from cswap says nothing changed and offers to try again', async () => {
    await open(fixtureModel('fresh'), {
      outcome: (slot) => ({ kind: 'failed', slot, error: 'refused', detail: 'ClaudeSwitchError: Account-3 … needs /login', active: null }),
    });
    const sheet = await sheetFor('Team B');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    expect(screen.getByText('Chưa chuyển được sang Team B')).toBeTruthy();
    expect(screen.getByText('cswap báo: ClaudeSwitchError: Account-3 … needs /login. Chưa đổi gì.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Thử lại' })).toBeTruthy();
    // Nothing changed: Team A is still the one in use.
    expect(screen.getByText('Claude còn tương đương 2,3 tài khoản')).toBeTruthy();
  });

  it('an unconfirmed switch asks for a refresh', async () => {
    await open(fixtureModel('fresh'), { outcome: (slot) => ({ kind: 'unconfirmed', slot, hold: null }) });
    const sheet = await sheetFor('Team B');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang Team B' }));
    });
    expect(screen.getByText('Đã gửi lệnh chuyển sang Team B')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Làm mới' })).toBeTruthy();
  });

  it('Thôi and Escape close the sheet without switching, and Escape then hides the popover', async () => {
    const fake = await open(fixtureModel('fresh'));
    let sheet = await sheetFor('Team B');
    fireEvent.click(within(sheet).getByRole('button', { name: 'Thôi' }));
    expect(screen.queryByRole('dialog', { name: /Chuyển Claude/ })).toBeNull();
    sheet = await sheetFor('Team B');
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: /Chuyển Claude/ })).toBeNull();
    expect(fake.api.hidePopover).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(fake.api.hidePopover).toHaveBeenCalledTimes(1);
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });

  it('session counts: none, unreadable records, and a folder that cannot be read', async () => {
    await open(fixtureModel('fresh'), { sessions: { live: 0, unreadable: 0 } });
    expect(facts(await sheetFor('Team B'))[1]).toBe(
      'Không có phiên Claude nào đang chạy trên máy này, nên không phiên nào phải tạo lại bộ nhớ đệm.',
    );
    cleanup();
    await open(fixtureModel('fresh'), { sessions: { live: 3, unreadable: 1 } });
    expect(facts(await sheetFor('Team B'))[1]).toMatch(/^Ít nhất 3 phiên Claude đang chạy/);
    cleanup();
    await open(fixtureModel('fresh'), { sessions: null });
    expect(facts(await sheetFor('Team B'))[1]).toMatch(/^Các phiên Claude đang chạy sẽ tạo lại bộ nhớ đệm/);
  });

  it('the personal account outside its hours warns that the switcher will move away', async () => {
    // 00:30 at UTC+7: outside the personal hours 8-23.
    await open(fixtureModel('fresh'), {}, () => new Date('2026-01-16T17:30:00.000Z'));
    const sheet = await sheetFor('Cá nhân');
    expect(facts(sheet)).toContain(
      'Bây giờ là 00:30, ngoài giờ 08–23 của tài khoản Cá nhân. Nếu để tự chuyển, switcher sẽ đổi sang tài khoản khác trong vòng 3 phút.',
    );
    fireEvent.click(within(sheet).getByRole('radio', { name: /Để tự chuyển/ }));
    expect(within(sheet).getByText('Ngoài giờ dùng tài khoản Cá nhân. Chọn thời lượng giữ để tránh tự chuyển sang tài khoản khác.').className).toBe(
      'hint warn',
    );
  });

  it('without cswap nothing can switch', async () => {
    await open(fixtureModel('fresh', { claudeSwitch: { cswap: false, busy: null } }));
    const rows = within(screen.getByRole('list', { name: 'CLAUDE CODE' })).getAllByRole('button');
    expect(rows.every((r) => r.getAttribute('aria-disabled') === 'true')).toBe(true);
    expect(rows[2].getAttribute('aria-label')).toMatch(/\. Chưa chuyển được: không tìm thấy cswap$/);
    expect(screen.getByText('Không tìm thấy cswap trên máy này: chưa chuyển được tài khoản Claude.')).toBeTruthy();
  });

  it('a switch from the other window that lands on this sheet’s account closes the sheet', async () => {
    const fake = await open(fixtureModel('fresh'));
    await sheetFor('Team B');
    const model = fixtureModel('fresh');
    act(() => fake.push({ ...model, claudeSwitch: { cswap: true, busy: 3 } }));
    expect(screen.getByRole('dialog', { name: 'Đang chuyển sang Team B…' })).toBeTruthy();
    const accounts = model.snapshot.claude.accounts.map((a) => ({ ...a, active: a.number === 3 }));
    act(() => fake.push({ ...model, snapshot: { ...model.snapshot, claude: { ...model.snapshot.claude, accounts } } }));
    expect(screen.queryByRole('dialog', { name: /Team B/ })).toBeNull();
    expect(fake.api.switchClaude).not.toHaveBeenCalled();
  });

  it('while a switch runs from another window, rows wait', async () => {
    await open(fixtureModel('fresh', { claudeSwitch: { cswap: true, busy: 3 } }));
    const rows = within(screen.getByRole('list', { name: 'CLAUDE CODE' })).getAllByRole('button');
    expect(rows[3].getAttribute('aria-label')).toMatch(/\. Đang chuyển tài khoản$/);
  });
});

describe('the pause control', () => {
  it('the toggle opens the pause sheet; pausing holds the active account for the time chosen', async () => {
    const fake = await open(fixtureModel('fresh'));
    fireEvent.click(screen.getByRole('switch', { name: 'Tự chuyển Claude' }));
    const sheet = screen.getByRole('dialog', { name: 'Tạm dừng tự chuyển Claude?' });
    expect(within(sheet).getByText('Giữ Team A. Switcher sẽ không đổi tài khoản Claude trong thời gian này. Hạn mức vẫn cập nhật.')).toBeTruthy();
    expect(radios(sheet)).toEqual(['● 1 giờđến 15:20', '○ Đến 18:00hôm nay', '○ Đến khi tôi bật lạikhông có giờ hết hạn']);
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Tạm dừng' }));
    });
    expect(fake.api.pauseSwitching).toHaveBeenCalledWith('2026-01-16T08:20:40.000Z');
    expect(screen.getByText('Đã tạm dừng tự chuyển Claude')).toBeTruthy();
    expect(screen.getByText('Giữ Team A đến 15:20. Hết thời gian này, tự chuyển sẽ bật lại.')).toBeTruthy();
    // The why line, the toggle and the summary say so; the banner waits for the line to close.
    expect(screen.getByText('Tự chuyển đang tạm dừng')).toBeTruthy();
    expect(screen.getByText('Đang tạm dừng tới 15:20: tạm dừng trong Quota Station lúc 14:20')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Tự chuyển Claude, đang tạm dừng' }).getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText('Giữ Team A đến 15:20')).toBeTruthy();
    expect(screen.queryByText('Đang giữ Team A đến 15:20')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    expect(screen.getByText('Đang giữ Team A đến 15:20')).toBeTruthy();
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
    expect(screen.getByText('Giữ Team A đến khi bạn bật lại. Bật lại bằng công tắc Tự chuyển Claude.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    expect(screen.getByText('Đang giữ Team A đến khi bạn bật lại')).toBeTruthy();
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
    expect(screen.getAllByText('Giữ Team A đến 16:20')).toHaveLength(2);
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
    await open(fixtureModel('fresh', { switcher: switcherFixture({ autoService: true }) }));
    expect(screen.getByText('Hai bộ tự chuyển cùng chạy')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Hướng dẫn tắt' }));
    const guide = screen.getByRole('dialog', { name: 'Tắt bộ tự chuyển của claude-swap' });
    expect(within(guide).getByText('launchctl bootout gui/$(id -u)/com.cswap.auto')).toBeTruthy();
    fireEvent.click(within(guide).getByRole('button', { name: 'Đóng' }));
    expect(screen.queryByRole('dialog', { name: /Tắt bộ/ })).toBeNull();
  });

  it('a Mac without the switcher cannot pause', async () => {
    await open(fixtureModel('fresh', { switcher: switcherFixture({ installed: false, dryRun: null, lastLog: null }) }));
    const toggle = screen.getByRole('switch', { name: 'Tự chuyển Claude' }) as HTMLButtonElement;
    expect(toggle.disabled).toBe(true);
    expect(screen.getByText('Không thấy cswap-prefer-team trên máy này: không có tự chuyển Claude.')).toBeTruthy();
  });
});
