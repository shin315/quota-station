// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// MCP, Bộ cấu hình and Sao lưu on Windows: the Mac is a PC ("máy" in
// Vietnamese), the packs go to OneDrive, folders open in File Explorer and
// the app's keys sit in its encrypted store. The setup file puts the system
// back to macOS and the language back to Vietnamese after each test.

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { backupNowError, macOf, restoreError } from '../lib/backup';
import { setLang, setPlatform } from '../lib/i18n';
import { whereLine } from '../lib/mcp';
import type { PackInfo } from '../lib/model';

afterEach(cleanup);

const other = { mac: null, mine: false } as unknown as PackInfo;

describe('agents and backup on Windows', () => {
  it('names the other computer a PC, or a máy', () => {
    setPlatform('windows');
    setLang('en');
    expect(macOf(other, 'Desk')).toBe('Another PC');
    expect(whereLine({ kind: 'stdio', program: 'npx', host: null })).toBe('on this PC · npx');
    setLang('vi');
    expect(macOf(other, 'Desk')).toBe('Máy khác');
  });

  it('backs up to OneDrive and opens folders in File Explorer', () => {
    setPlatform('windows');
    setLang('en');
    const off = backupNowError('noICloud');
    expect(off.title).toBe('Not backed up: OneDrive is off');
    expect(off.body).toBe('Sign in to OneDrive, then try again.');
    expect(restoreError('cloud').body).toBe('OneDrive only has the file name so far. Open the folder in File Explorer to download it, or wait.');
    setLang('vi');
    expect(restoreError('keychain').body).toBe('Kho mã hoá không nhận khoá lúc này. Chưa ghi gì.');
  });

  it('leaves the macOS text as it was', () => {
    setLang('en');
    expect(macOf(other, 'Desk')).toBe('Another Mac');
    expect(backupNowError('noICloud').body).toBe('Turn on iCloud Drive in System Settings › Apple ID › iCloud, then try again.');
    expect(restoreError('keychain').body).toBe('Keychain is not accepting keys right now. Nothing was written.');
  });

  it('heads the Sao lưu tab with OneDrive', async () => {
    const model = fixtureModel('fresh', { language: 'en' });
    model.capabilities = { ...model.capabilities, platform: 'windows' };
    const fake = fakeApi(model, 'backup');
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    expect(await screen.findByText('Keeps the 10 latest packs per PC.')).toBeTruthy();
    expect(screen.getByText(/· OneDrive$/)).toBeTruthy();
    expect(screen.queryByText(/iCloud Drive/)).toBeNull();
  });
});
