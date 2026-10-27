// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Sao lưu, Chép sang Mac khác and the card of variables that override a
// sign-in, in English: the owner's language set to English, on the fake
// Rust side. Every pack, Mac, key and variable is made up.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { API_KEY, envFixture } from '../fixtures/fakeEnv';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import { changedLines, keysLine, restoreError } from '../lib/backup';
import { copyError, macChip } from '../lib/copy';
import { checkedLine, envBanner } from '../lib/env';
import { setLang } from '../lib/i18n';
import type { Model, Tab } from '../lib/model';
import { EnvCard } from '../settings/EnvCard';

async function open(tab: Tab, model: Model) {
  const fake = fakeApi(model, tab);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const english = (over: Partial<Model> = {}) => fixtureModel('fresh', { language: 'en', ...over });
const dialog = () => screen.getByRole('dialog');
const button = (name: string | RegExp) => screen.getByRole('button', { name });

afterEach(cleanup);

describe('Backup, Copy to another Mac and the variables card in English', () => {
  it('draws the Backup tab and the restore sheet in English', async () => {
    await open('backup', english());
    await screen.findByRole('heading', { level: 2, name: 'Backup' });
    expect(screen.getByRole('heading', { level: 4, name: 'Latest pack' })).toBeTruthy();
    expect(screen.getByText('No keys inside. On restore, the app asks for 2 keys again.')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'After every change in the app' })).toBeTruthy();
    expect(screen.getByText('Stored in: Docs Search API')).toBeTruthy();
    expect(button('Back up now')).toBeTruthy();
    await press(button('Restore…'));
    expect(within(dialog()).getByRole('heading', { name: 'Restore' })).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Next' }));
    await screen.findByRole('heading', { name: /^Preview: pack from / });
    expect(within(dialog()).getByText('Does not change sign-ins, accounts or the Codex provider.')).toBeTruthy();
    expect(within(dialog()).getByRole('button', { name: 'Cancel' })).toBeTruthy();
  });

  it('compares with another Mac in English', async () => {
    await open('copy', english());
    await screen.findByRole('heading', { level: 2, name: 'Copy to another Mac' });
    expect(screen.getByText('Pick a Mac and press Compare. The app only reads; nothing is written yet.')).toBeTruthy();
    await press(button('Compare'));
    await screen.findByRole('heading', { level: 4, name: /^Differences/ });
    expect(screen.getByRole('switch', { name: 'Also send keys over SSH' }).getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText('Same on both: 11 items.')).toBeTruthy();
    await press(button(/^Copy \d+ items? to /));
    expect(within(dialog()).getByRole('heading', { name: /^Copy \d+ items? to .+\?$/ })).toBeTruthy();
    expect(within(dialog()).getByText('Sends no keys.')).toBeTruthy();
  });

  it('titles the variables card in English', () => {
    setLang('en');
    render(<EnvCard model={english({ env: envFixture({ found: [] }) })} onAction={() => {}} />);
    expect(screen.getByText('Environment variables that override sign-in')).toBeTruthy();
    expect(screen.getByText('No variable overrides a sign-in.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Check again' })).toBeTruthy();
  });

  it('builds the lines of the logic in English', () => {
    setLang('en');
    expect(keysLine(1)).toBe('No keys inside. On restore, the app asks for 1 key again.');
    expect(restoreError('busy').title).toBe('Not restored');
    expect(copyError('timeout', 'Mac B').title).toBe('Mac B is not answering over SSH');
    expect(macChip('ok')?.t).toBe('SSH OK');
    expect(changedLines('changed:codex\nskill:notes-helper', 'copy')).toEqual(['Codex files were edited after the copy.', 'Skill notes-helper was edited after the copy.']);
    expect(checkedLine(envFixture({ found: [], checkedAt: null, checking: false }))).toMatch(/not checked yet\. The app only knows whether each is set, never its value\.$/);
    expect(envBanner(fixtureModel('fresh', { env: envFixture({ found: [API_KEY] }) }))?.title).toBe('ANTHROPIC_API_KEY is overriding the Claude sign-in');
  });

  it('still speaks Vietnamese: a copy, not a restore, in the undo of a copy, and the variables', () => {
    expect(changedLines('changed:codex\nskill:notes-helper', 'copy')).toEqual(['File của Codex đã sửa sau lần chép.', 'Skill notes-helper đã sửa sau lần chép.']);
    expect(checkedLine(envFixture({ ignored: 2 }))).toMatch(/^Đã kiểm tra 11 biến ở 6 chỗ lúc \d\d:\d\d\. Chỉ biết biến có đặt hay không, không đọc giá trị\. Đã bỏ qua 2 chỗ\.$/);
    expect(envBanner(fixtureModel('fresh', { env: envFixture({ found: [API_KEY] }) }))?.title).toBe('ANTHROPIC_API_KEY đang đè đăng nhập Claude');
  });
});
