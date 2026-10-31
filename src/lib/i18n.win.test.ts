// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The words for the system's own parts: as written on macOS, Windows' own on
// Windows. The setup file puts the system back to macOS and the language
// back to Vietnamese after each test.

import { describe, expect, it } from 'vitest';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { isFile, openLabel, placeLabel, whereLine } from './env';
import { appName, cloudDrive, fileManager, mac, onWindows, os, osName, platform, quitKey, secretStore, setLang, setPlatform, systemSettings, terminal, toolStore } from './i18n';
// @private:begin
import { guide } from '@/private/switching';
// @private:end
import { popoverView } from './view';
import type { EnvFound } from './model';

const found = (place: EnvFound['place'], line: number | null): EnvFound => ({
  id: 'ANTHROPIC_API_KEY@' + place,
  name: 'ANTHROPIC_API_KEY',
  agent: 'claude',
  weight: 'bad',
  place,
  line,
  effect: '',
});

describe('the words for the system', () => {
  it('give macOS its text back unchanged', () => {
    expect(platform()).toBe('macos');
    expect(onWindows()).toBe(false);
    expect(mac('Xoá khỏi máy này. Mac khác giữ nguyên.')).toBe('Xoá khỏi máy này. Mac khác giữ nguyên.');
    setLang('en');
    expect(mac('Remove from this Mac’s Keychain; another Mac keeps its own.')).toBe('Remove from this Mac’s Keychain; another Mac keeps its own.');
    expect([osName(), quitKey(), fileManager(), systemSettings(), terminal(), appName('Antigravity'), secretStore(), toolStore(), cloudDrive()]).toEqual([
      'macOS',
      '⌘Q',
      'Finder',
      'System Settings',
      'Terminal',
      'Antigravity.app',
      'Keychain',
      'Keychain',
      'iCloud Drive',
    ]);
  });

  it('a Mac is a PC in English and a máy in Vietnamese on Windows', () => {
    setPlatform('windows');
    expect(mac('Mac khác')).toBe('Máy khác');
    expect(mac('Tối đa 8 Mac. Mỗi Mac khác: Mac kia.')).toBe('Tối đa 8 máy. Mỗi máy khác: máy kia.');
    setLang('en');
    expect(mac('This Mac’s name')).toBe('This PC’s name');
    expect(mac('Up to 8 Macs on this Mac.')).toBe('Up to 8 PCs on this PC.');
    // Other words that start with "Mac" stay.
    expect(mac('MacBook Air, macOS and this Mac')).toBe('MacBook Air, macOS and this PC');
  });

  it('Windows names its own parts', () => {
    setPlatform('windows');
    expect(os('mac', 'win')).toBe('win');
    expect([osName(), quitKey(), fileManager(), systemSettings(), terminal(), appName('Antigravity'), secretStore(), toolStore(), cloudDrive()]).toEqual([
      'Windows',
      'Alt+F4',
      'File Explorer',
      'Cài đặt Windows',
      'Windows Terminal',
      'Antigravity',
      'kho mã hoá',
      'Credential Manager',
      'OneDrive',
    ]);
  });

  it('no Keychain banner on Windows: nothing asks for access there', () => {
    expect(popoverView(fixtureModel('keychain'), FIXTURE_NOW).banner?.title).toBe('Chưa cấp quyền Keychain cho Quota Station');
    setPlatform('windows');
    expect(popoverView(fixtureModel('keychain'), FIXTURE_NOW).banner).toBeNull();
  });

  // @private:begin
  it('the move from claude-swap has no LaunchAgent to turn off on Windows', () => {
    const steps = guide('switcher', false).steps;
    expect(steps[0]).toContain('launchctl bootout');
    setPlatform('windows');
    const win = guide('switcher', false);
    expect(win.steps.some((s) => s.includes('launchctl'))).toBe(false);
    expect(win.steps).toHaveLength(steps.length - 1);
    expect(win.lead).toBe('Làm lần lượt trên từng máy. claude-swap và dữ liệu của nó được giữ nguyên, để quay lại được khi cần.');
  });
  // @private:end

  it('a variable set in Windows’ user or system variables, or in PowerShell’s profile', () => {
    setPlatform('windows');
    expect(placeLabel('userEnv')).toBe('biến người dùng');
    expect(whereLine(found('userEnv', null))).toBe('biến người dùng · app mở từ Start');
    expect(openLabel(found('machineEnv', null))).toBeNull();
    expect(isFile('psProfile')).toBe(true);
    expect(openLabel(found('psProfile', 12))).toBe('Mở $PROFILE ở dòng 12');
    setLang('en');
    expect(whereLine(found('machineEnv', null))).toBe('system variables · apps opened from Start');
    // macOS's places read as before.
    expect(whereLine(found('launchctl', null))).toBe('launchctl · apps opened from the Dock');
    expect(whereLine(found('zshrc', 42))).toBe('~/.zshrc · line 42');
  });
});
