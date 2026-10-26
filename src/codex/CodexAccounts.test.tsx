// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Several Codex accounts (ChatGPT sign-ins and custom providers) in the
// main window's Codex tab, and the line after a switch between two ChatGPT
// sign-ins, on the fake Rust side: nothing here moves a sign-in or writes
// config.toml.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { codexFixture, codexMany, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import { codexFeedback } from '../lib/codex';
import type { Model } from '../lib/model';

async function openTab(model: Model) {
  const fake = fakeApi(model, 'codex');
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { level: 2, name: 'Codex' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

afterEach(cleanup);

describe('the Codex accounts', () => {
  it('the tab lists the ChatGPT sign-ins, then the Custom API, the one in use and the one to sign in again as such', async () => {
    const fake = await openTab(codexMany(fixtureModel('fresh')));
    expect(screen.getByText('4 ChatGPT · 3 Custom API')).toBeTruthy();
    const rows = within(screen.getByRole('list', { name: 'Tài khoản Codex' })).getAllByRole('listitem');
    const labels = rows.map((r) => r.getAttribute('aria-label')!);
    expect(labels.map((l) => l.split(', ')[0])).toEqual(['lan.chatgpt1', 'lan.chatgpt2', 'lan.team', 'Tài khoản phụ', 'relay', 'relay-eu', 'backup-api']);
    expect(labels.map((l) => (/^[^,]+, ChatGPT/.test(l) ? 'ChatGPT' : /^[^,]+, Custom API/.test(l) ? 'Custom API' : l))).toEqual([
      ...Array<string>(4).fill('ChatGPT'),
      ...Array<string>(3).fill('Custom API'),
    ]);
    // relay is in use: a status in place of "Chuyển", and the only one.
    expect(rows.filter((r) => within(r).queryByText('Đang dùng')).map((r) => r.getAttribute('aria-label')!.split(', ')[0])).toEqual(['relay']);
    expect(within(rows[4]).queryByRole('button', { name: /^Chuyển sang/ })).toBeNull();
    for (const [i, name] of [[1, 'lan.chatgpt2'], [5, 'relay-eu']] as const)
      expect((within(rows[i]).getByRole('button', { name: 'Chuyển sang ' + name }) as HTMLButtonElement).disabled).toBe(false);
    // ChatGPT refused the side account's sign-in: "Đăng nhập lại" stands where "Chuyển" would.
    const side = rows[3];
    expect(within(side).getByText('cần đăng nhập lại')).toBeTruthy();
    expect(within(side).queryByRole('button', { name: /^Chuyển sang/ })).toBeNull();
    await press(within(side).getByRole('button', { name: 'Đăng nhập lại' }));
    // It asks first: nothing signs in until the page is opened.
    expect(screen.getByRole('dialog', { name: 'Đăng nhập lại Tài khoản phụ?' })).toBeTruthy();
    expect(fake.api.codexLogin).not.toHaveBeenCalled();
  });

  it('the line after a switch between two ChatGPT sign-ins names the account and says its sign-in moved; a refusal says nothing changed', () => {
    const model = codexMany(fixtureModel('fresh', { codex: codexFixture({ current: 'chatgpt' }) }));
    const moved = codexFeedback({ kind: 'switched', target: 'chatGpt', from: 'chatgpt:1', to: 'chatgpt:2', backup: '' }, model);
    expect(moved.title).toBe('Đã chuyển Codex sang lan.chatgpt2');
    expect(moved.body).toBe(
      'Đăng nhập của lan.chatgpt2 đã vào ~/.codex, đăng nhập trước về thư mục riêng của nó. Thoát hẳn Codex App (⌘Q) rồi mở lại; 3 phiên codex trong Terminal cần mở lại để dùng lan.chatgpt2.',
    );
    // Only the sign-ins moved: config.toml was not written, so there is no old copy of it to point to.
    expect(moved.body).not.toContain('config.toml');
    const refused = codexFeedback({ kind: 'account', target: 'chatGpt', refusal: { kind: 'keychain' } }, model);
    expect(refused.tone).toBe('bad');
    expect(refused.title).toBe('Chưa chuyển Codex sang tài khoản ChatGPT');
    expect(refused.body).toBe(
      'Codex giữ đăng nhập trong Keychain (cli_auth_credentials_store trong ~/.codex/config.toml), nên không có file nào để dời. Chưa đổi gì.',
    );
    expect(refused.actions!.map((a) => a.label)).toEqual(['Đóng']);
  });

  it('"Thêm tài khoản" offers to import a sign-in file only where the model says imports are on', async () => {
    const many = codexMany(fixtureModel('fresh'));
    const json = /^Nhập file đăng nhập \(JSON\)/;
    await openTab(many);
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    let dialog = screen.getByRole('dialog', { name: 'Thêm tài khoản Codex' });
    expect(within(dialog).getAllByRole('radio')).toHaveLength(2);
    expect(within(dialog).queryByRole('radio', { name: json })).toBeNull();
    cleanup();

    const fake = await openTab({ ...many, codex: { ...many.codex, imports: true } });
    await press(screen.getByRole('button', { name: 'Thêm tài khoản' }));
    dialog = screen.getByRole('dialog', { name: 'Thêm tài khoản Codex' });
    expect(within(dialog).getAllByRole('radio')).toHaveLength(3);
    await press(within(dialog).getByRole('radio', { name: json }));
    // The macOS file picker opens only on "Chọn file…".
    expect(within(dialog).getByRole('button', { name: 'Chọn file…' })).toBeTruthy();
    expect(fake.api.codexImportPick).not.toHaveBeenCalled();
  });
});
