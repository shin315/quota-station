// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// The Antigravity words on Windows: the app is Antigravity, not
// Antigravity.app; the shared sign-in is in Credential Manager and the app's
// own in its encrypted store; the OAuth client file goes under %APPDATA%
// with no mode. The setup file puts the system back to macOS and the
// language back to Vietnamese after each test.

import { describe, expect, it } from 'vitest';
import { agyFixture } from '../fixtures/models';
import { setLang, setPlatform } from '../lib/i18n';
import { agyLoginFeedback, clientNotice } from './agyLogin';
import { agySwitchFeedback, agyWorking } from './agySwitch';

const WIN_PATH = '%APPDATA%\\com.shin315.quota-station\\oauth-client.json';

describe('Antigravity on Windows', () => {
  it('a missing OAuth client file says where it goes, with no mode', () => {
    setPlatform('windows');
    const notice = clientNotice(agyFixture({ client: 'missing', clientPath: WIN_PATH }))!;
    expect(notice.body).toContain(WIN_PATH);
    expect(notice.body).toContain('PowerShell');
    expect(notice.body).not.toContain('600');
    setLang('en');
    const en = clientNotice(agyFixture({ client: 'missing', clientPath: WIN_PATH }))!;
    expect(en.body).toContain('on this PC');
    expect(en.body).not.toMatch(/\bMac\b|mode 600/);
    const unreadable = clientNotice(agyFixture({ client: 'unreadable', clientPath: WIN_PATH }))!;
    expect(unreadable.body).toContain('your Windows account');
  });

  it('macOS keeps its own words', () => {
    const notice = clientNotice(agyFixture({ client: 'missing' }))!;
    expect(notice.body).toContain(', quyền 600. README ghi định dạng.');
    expect(agyWorking('quitFirst')).toContain('Thoát Antigravity.app');
  });

  it('the switch names Antigravity and Credential Manager', () => {
    setPlatform('windows');
    setLang('en');
    expect(agyWorking('quitFirst')).toBe(
      'Quits Antigravity, writes the shared sign-in of Antigravity and agy, reopens the app, then waits 5 seconds to read back the account agy uses.',
    );
    const write = agySwitchFeedback({ kind: 'failed', number: 2, error: 'write', detail: 'Access is denied. (os error 5)', reopened: true });
    expect(write.body).toBe('Could not write the shared sign-in to the Credential Manager (Access is denied. (os error 5)). The old sign-in is unchanged. Antigravity has reopened.');
    const notQuit = agySwitchFeedback({ kind: 'failed', number: 2, error: 'notQuit', detail: '', reopened: null });
    expect(notQuit.body).not.toContain('Antigravity.app');
    expect(notQuit.body).toContain('Close Antigravity and try again');
  });

  it('a new sign-in is kept in the app’s encrypted store on this PC', () => {
    setPlatform('windows');
    setLang('en');
    const done = agyLoginFeedback({ number: 1, added: true });
    expect(done.body).toBe('The sign-in is kept in the app’s encrypted store on this PC. Antigravity and agy are unchanged.');
  });
});
