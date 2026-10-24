// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Cài đặt tab in English: the cards, the rows and the language picker.
// The setup file puts the language back to Vietnamese after each test.

import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fixtureModel } from '../fixtures/models';
import { setLang } from '../lib/i18n';
import type { Model } from '../lib/model';
import { Settings, type SettingsProps } from './Settings';

function renderSettings(model: Model) {
  const props: SettingsProps = {
    model,
    fresh: { tone: 'ok', text: 'just now' },
    onAction: vi.fn(),
    onGrant: vi.fn(),
    onSetMachine: vi.fn(async () => true),
    onSetPeers: vi.fn(async () => null),
    onTestPeer: vi.fn(),
    onShowNames: vi.fn(),
    onTheme: vi.fn(),
    onMenubar: vi.fn(),
    onMenubarWindow: vi.fn(),
    onSetFirstmate: vi.fn(async () => null),
    onSetCodexHome: vi.fn(async () => null),
    onTestAlert: vi.fn(),
    onAlertSwitch: vi.fn(),
    onLaunchAtLogin: vi.fn(async () => null),
    onOpenLoginItems: vi.fn(),
    onSetSource: vi.fn(async () => null),
    onUpdate: vi.fn(async () => null),
    onRelaunch: vi.fn(async () => null),
    onSkillChecks: vi.fn(),
    onAdvanced: vi.fn(),
    onFirstmate: vi.fn(),
    onPoolAlert: vi.fn(async () => null),
    onSetBackupFolder: vi.fn(async () => null),
    onLanguage: vi.fn(),
  };
  render(
    <div className="qs app-window">
      <Settings {...props} />
    </div>,
  );
  return props;
}

afterEach(cleanup);

describe('settings tab in English', () => {
  it('names its cards and rows in English', () => {
    setLang('en');
    renderSettings(fixtureModel('fresh'));
    expect(screen.getByRole('heading', { name: 'Settings' })).toBeTruthy();
    const titles = Array.from(document.querySelectorAll('[data-slot="card-title"]')).map((h) => h.textContent);
    expect(titles).toEqual(expect.arrayContaining(['General', 'Display', 'Notifications', 'Updates', 'Services and data', 'Backup', 'Advanced']));
    expect(titles.at(-1)).toBe('Advanced');

    expect(screen.getByText('This Mac’s name')).toBeTruthy();
    expect(screen.getByText('Open at login')).toBeTruthy();
    expect(screen.getByText('Keychain access')).toBeTruthy();
    expect(screen.getByText('Codex folder')).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Show emails' })).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Show advanced features' })).toBeTruthy();
    expect(within(screen.getByRole('group', { name: 'Appearance' })).getAllByRole('button').map((b) => b.textContent)).toEqual([
      'Match macOS',
      'Light',
      'Dark',
    ]);
    expect(within(screen.getByRole('group', { name: 'Number in the menu bar' })).getAllByRole('button').map((b) => b.textContent)).toEqual([
      '5-hour',
      'Week',
      'Fable week',
      'Highest usage',
    ]);
    expect(screen.getByRole('group', { name: 'Menu bar' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Add Mac' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Update from source' })).toBeTruthy();
    // No Vietnamese left in the cards' own names.
    expect(screen.queryByText('Cài đặt')).toBeNull();
    expect(screen.queryByText('Chung')).toBeNull();
  });

  it('the Language row offers macOS’s language, Vietnamese and English', () => {
    setLang('en');
    const props = renderSettings(fixtureModel('fresh'));
    const group = screen.getByRole('group', { name: 'Language' });
    const buttons = within(group).getAllByRole('button');
    expect(buttons.map((b) => b.textContent)).toEqual(['Match macOS', 'Tiếng Việt', 'English']);
    for (const b of buttons) fireEvent.click(b);
    expect((props.onLanguage as ReturnType<typeof vi.fn>).mock.calls).toEqual([['auto'], ['vi'], ['en']]);
  });
});
