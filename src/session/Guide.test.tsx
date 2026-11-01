// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The guided sessions M1 and M2 of a test build (plan 4.3): every item the
// Windows PRs left for the PC is in a step, each step says what to do and
// what is right without asking anything, the guide goes on by "Xong",
// passes a step this PC cannot do by itself, comes back after a sign-out,
// and its facts carry no name or email.

import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { fakeApi } from '../fixtures/fakeApi';
import { claudeAccountsFixture, fixtureModel } from '../fixtures/models';
import type { Model } from '../lib/model';
import { GuideApp } from './Guide';
import { ITEMS, STEPS, facts, type SessionView } from './steps';

const SESSION: SessionView = { on: true, step: null, done: [], restarted: false, zip: null, claudeProgram: 'claude.exe', exe: null };

function windows(over: Partial<Model> = {}): Model {
  const m = fixtureModel('fresh', over);
  return { ...m, capabilities: { ...m.capabilities, platform: 'windows', session: true } };
}

async function open(model: Model, session: Partial<SessionView> = {}) {
  const fake = fakeApi(model, 'overview', { session: { ...SESSION, ...session } });
  render(<GuideApp api={fake.api} />);
  await screen.findByRole('heading', { level: 1 });
  return fake;
}

const press = async (name: string) => {
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name }));
  });
};

afterEach(cleanup);

describe('the steps', () => {
  it('cover every M1 and M2 item of the four Windows PRs, and nothing else', () => {
    const covered = new Set(STEPS.flatMap((s) => s.covers));
    for (const item of ITEMS) expect(covered.has(item), item).toBe(true);
    for (const item of covered) expect((ITEMS as readonly string[]).includes(item), item).toBe(true);
  });

  it('put M1 first, end each part with the zip, and keep ids unique', () => {
    const parts = STEPS.map((s) => s.part);
    expect(parts).toEqual([...parts].sort());
    expect(STEPS.filter((s) => s.end).map((s) => [s.id, s.end])).toEqual([
      ['m1-end', 1],
      ['m2-end', 2],
    ]);
    expect(STEPS[STEPS.length - 1].end).toBe(2);
    expect(STEPS.findIndex((s) => s.id === 'm1-end')).toBe(STEPS.filter((s) => s.part === 1).length - 1);
    expect(new Set(STEPS.map((s) => s.id)).size).toBe(STEPS.length);
  });

  it('say what to do and what is right, and never ask the owner to count or report', () => {
    for (const s of STEPS) {
      expect(s.do.length, s.id).toBeGreaterThan(0);
      expect(s.right.length, s.id).toBeGreaterThan(0);
      const words = [...s.do, ...s.right].filter((l) => typeof l === 'string').join(' ');
      expect(words, s.id).not.toMatch(/\?|ghi lại|đếm|báo lại|báo kết quả|có đúng không/i);
    }
  });

  it('leave the sign-out for the end of M2, after which the app comes back by itself', () => {
    const restart = STEPS.findIndex((s) => s.restart);
    expect(STEPS[restart].id).toBe('m2-restart');
    expect(STEPS.slice(restart + 1).map((s) => s.id)).toEqual(['m2-after', 'm2-end']);
  });

  it('read facts without a name, an email or a path', () => {
    const f = JSON.stringify(facts(windows(), SESSION));
    expect(f).not.toMatch(/@|example\.com|\\|\/Users\//);
    expect(JSON.parse(f).claude.kept).toBe(4);
  });
});

describe('the guide', () => {
  it('starts on the first step, takes the pictures it asks for, and goes on with Xong', async () => {
    const fake = await open(windows());
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Bắt đầu');
    expect(screen.getByText('Phần 1 · Nhìn · Bước 1/18')).toBeTruthy();
    // The guide tells the session its step from an effect after the step is
    // drawn, so the heading can be there a moment before the call is.
    await waitFor(() => expect(fake.sessionCalls()).toContainEqual({ call: 'step', id: 'm1-start', captures: ['main'] }));
    await press('Xong');
    const done = fake.sessionCalls().find((c) => c.call === 'done');
    expect(done).toMatchObject({ id: 'm1-start', skipped: false, checks: ['system', 'mainWindow'] });
    expect(await screen.findByRole('heading', { name: 'Icon khay' })).toBeTruthy();
    await press('Chụp góc khay sau 3 giây');
    expect(fake.sessionCalls()).toContainEqual({ call: 'shot', id: 'tray' });
    expect(screen.getByRole('status').textContent).toBe('Sẽ chụp sau 3 giây');
  });

  it('opens the Windows Settings page and the app window a step points to', async () => {
    const fake = await open(windows(), { step: 'm1-transparency' });
    await press('Mở Cài đặt › Màu');
    expect(fake.sessionCalls()).toContainEqual({ call: 'open', id: 'colors' });
    await press('Mở cửa sổ chính');
    expect(fake.current()).toBeTruthy();
  });

  it('resumes where it was, and lets an optional step be left out', async () => {
    const fake = await open(windows(), { step: 'm1-screens' });
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Hai màn hình');
    await press('Máy chỉ có một màn hình: bỏ qua');
    expect(fake.sessionCalls().find((c) => c.call === 'done')).toMatchObject({ id: 'm1-screens', skipped: true, checks: [] });
    expect(await screen.findByRole('heading', { name: 'Taskbar tự ẩn' })).toBeTruthy();
  });

  it('passes a step this PC cannot do by itself, saying why', async () => {
    const fake = await open(windows({ codex: { ...fixtureModel('fresh').codex, present: false } }), { step: 'm2-codex' });
    expect(screen.getByText(/chưa có Codex/)).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Xong' })).toBeNull();
    await press('Sang bước tiếp');
    expect(fake.sessionCalls().find((c) => c.call === 'done')).toMatchObject({ id: 'm2-codex', skipped: true });
  });

  it('writes the zip at the end of M1, says where it is, and goes on to M2', async () => {
    const fake = await open(windows(), { step: 'm1-end' });
    await press('Ghi gói chẩn đoán');
    expect(await screen.findByText(/Kéo file đó vào chat/)).toBeTruthy();
    expect(screen.getByRole('status').textContent).toContain('diag-2026-10-28.zip');
    await press('Mở thư mục chứa file');
    expect(fake.sessionCalls().map((c) => c.call)).toEqual(expect.arrayContaining(['zip', 'reveal']));
    await press('Sang phần 2');
    expect(await screen.findByRole('heading', { name: 'Phần 2: tài khoản Claude' })).toBeTruthy();
  });

  it('ends M2 on the zip, with nothing after it', async () => {
    await open(windows(), { step: 'm2-end' });
    expect(screen.queryByRole('button', { name: 'Xong' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Ghi gói chẩn đoán' })).toBeTruthy();
  });

  it('after a sign-out and in, the sign-out step is done and the next one shows', async () => {
    const fake = await open(windows(), { step: 'm2-restart', restarted: true });
    expect(await screen.findByRole('heading', { name: 'Sau khi đăng nhập lại' })).toBeTruthy();
    expect(fake.sessionCalls().find((c) => c.call === 'done')).toMatchObject({ id: 'm2-restart', skipped: false, checks: ['autostart'] });
  });

  it('gives the claude run command for an account not in use', async () => {
    await open(windows({ claudeAccounts: claudeAccountsFixture() }), { step: 'm2-claude-folder', exe: 'C:\\Apps\\quota-station.exe' });
    expect(screen.getByText(/bằng tài khoản số 1,/)).toBeTruthy();
  });
});
