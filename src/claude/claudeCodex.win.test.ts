// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude and Codex words on Windows: no Keychain, LaunchAgent, ⌘Q or
// Terminal, and a PC where macOS has a Mac. The setup file puts the system
// back to macOS and the language back to Vietnamese after each test.

import { describe, expect, it } from 'vitest';
import { fixtureModel } from '../fixtures/models';
import { permissionBanner } from '../lib/alerts';
import { errorText } from '../lib/claudeAccounts';
// @private:begin
import { claudeAccountsFixture } from '../fixtures/models';
import { rotationRow } from '../lib/claudeAccounts';
// @private:end
import { codexSheet } from '../lib/codex';
import { loginFailText } from '../lib/codexAccounts';
import { setLang, setPlatform } from '../lib/i18n';
import type { ClaudeActionDone } from '../lib/model';

const failed = (error: string): ClaudeActionDone => ({ action: 'login', ok: false, error, detail: null, slot: null, renewed: false, imported: null });

describe('Claude and Codex on Windows', () => {
  // @private:begin
  it('the rotation runs on the app’s timer, not a LaunchAgent', () => {
    setPlatform('windows');
    const row = rotationRow(claudeAccountsFixture({ rotation: true }));
    expect(row.line).toBe('Hẹn giờ trong app chạy mỗi 3 phút khi app đang chạy, kể cả khi chỉ còn ở khay.');
    setLang('en');
    expect(errorText(failed('launchctl'))).toBe('Could not start the app’s timer.');
  });
  // @private:end

  it('the app’s own sign-ins are in its encrypted store', () => {
    setPlatform('windows');
    expect(errorText(failed('keychain'))).toBe('Kho mã hoá không cho ghi.');
    setLang('en');
    expect(errorText(failed('keychain'))).toBe('The encrypted store refused the write.');
  });

  it('a PC, Alt+F4 and Windows Terminal for Codex', () => {
    setPlatform('windows');
    setLang('en');
    expect(loginFailText('noCodex', null)).toBe('The codex command was not found on this PC. Install the Codex CLI and try again.');
    const facts = codexSheet(fixtureModel('fresh'), 'chatGpt').facts.map((f) => f.text);
    const restart = facts.find((f) => f.includes('open it again'));
    expect(restart).toContain('quit Codex App (Alt+F4)');
    expect(restart).toContain('in Windows Terminal');
    expect(facts.join(' ')).not.toContain('⌘Q');
  });

  it('Windows notifications and Windows Settings', () => {
    setPlatform('windows');
    const denied = permissionBanner('denied')!;
    expect(denied.body).toBe('Windows không hiện thông báo nào. Các thông báo vẫn nằm ở đây, kèm các nút bấm tương ứng.');
    expect(denied.actions!.map((a) => a.label)).toEqual(['Mở Cài đặt Windows']);
  });

  it('macOS keeps its words', () => {
    setLang('en');
    expect(loginFailText('noCodex', null)).toBe('The codex command was not found on this Mac. Install the Codex CLI and try again.');
    expect(errorText(failed('keychain'))).toBe('The Keychain refused the write.');
    expect(permissionBanner('denied')!.actions!.map((a) => a.label)).toEqual(['Open System Settings']);
  });
});
