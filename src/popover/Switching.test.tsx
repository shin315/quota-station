// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The switch sheet (F04) in the popover, on the fake Rust side: nothing
// here runs claude-swap.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PopoverApp } from '../App';
import { MORE_KEY } from './Popover';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
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

// Every section shows its other accounts, as the owner opens them.
beforeEach(() => localStorage.setItem(MORE_KEY, JSON.stringify({ c: true, g: true, x: true })));
afterEach(() => {
  cleanup();
  localStorage.removeItem(MORE_KEY);
});

describe('the Claude switch sheet', () => {






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
    await act(async () => finish({ kind: 'switched', slot: 3, hold: null } as Outcome));
    expect(screen.queryByRole('dialog', { name: /Đang chuyển/ })).toBeNull();
    expect(screen.getByText('Đã chuyển sang lan.acme2')).toBeTruthy();
  });


  it('another switch winning says which account won', async () => {
    await open(fixtureModel('fresh'), { outcome: (slot) => ({ kind: 'raced', slot, active: 4, bySwitcherAt: null, hold: null }) as Outcome });
    const sheet = await sheetFor('lan.acme2');
    await act(async () => {
      fireEvent.click(within(sheet).getByRole('button', { name: 'Chuyển sang lan.acme2' }));
    });
    expect(screen.getByText('Một lần chuyển khác vừa chuyển sang lan.acme3')).toBeTruthy();
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
    expect(document.querySelector('[data-active]')!.getAttribute('aria-label')).toMatch(/^lan\.acme1, (Acme Studio, )?đang dùng\./);
  });

  it('an unconfirmed switch asks for a refresh', async () => {
    await open(fixtureModel('fresh'), { outcome: (slot) => ({ kind: 'unconfirmed', slot, hold: null }) as Outcome });
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

