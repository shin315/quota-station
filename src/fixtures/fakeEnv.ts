// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Made-up variables for the M14 card, banner and alerts: names and places
// only, as the Rust side sends them. No value is anywhere here.

import type { AlertRecord, EnvFound, EnvView } from '../lib/model';

export const ENV_VARS = [
  'ANTHROPIC_API_KEY',
  'ANTHROPIC_AUTH_TOKEN',
  'ANTHROPIC_BASE_URL',
  'CLAUDE_CODE_OAUTH_TOKEN',
  'CLAUDE_CONFIG_DIR',
  'OPENAI_API_KEY',
  'OPENAI_BASE_URL',
  'CODEX_HOME',
  'GEMINI_API_KEY',
  'GOOGLE_API_KEY',
  'GOOGLE_CLOUD_PROJECT',
];
export const ENV_PLACES = ['~/.zshenv', '~/.zprofile', '~/.zshrc', '~/.bash_profile', '~/.profile', 'launchctl'];

export const API_KEY: EnvFound = {
  id: 'ANTHROPIC_API_KEY@zshrc',
  name: 'ANTHROPIC_API_KEY',
  agent: 'claude',
  weight: 'bad',
  place: 'zshrc',
  line: 42,
  effect: 'Trong Terminal, Claude Code dùng khoá này thay cho tài khoản claude-swap đang chọn.',
};
export const BASE_URL: EnvFound = {
  id: 'OPENAI_BASE_URL@zprofile',
  name: 'OPENAI_BASE_URL',
  agent: 'codex',
  weight: 'warn',
  place: 'zprofile',
  line: 7,
  effect: 'Trong Terminal, Codex gửi yêu cầu tới địa chỉ này thay cho nhà cung cấp trong config.toml.',
};
export const LAUNCHD_GEMINI: EnvFound = {
  id: 'GEMINI_API_KEY@launchctl',
  name: 'GEMINI_API_KEY',
  agent: 'agy',
  weight: 'bad',
  place: 'launchctl',
  line: null,
  effect: 'Trong các app mở từ Dock, Antigravity có thể dùng khoá này thay cho tài khoản Google đang đăng nhập.',
};
export const CLAUDE_BASE: EnvFound = {
  id: 'ANTHROPIC_BASE_URL@launchctl',
  name: 'ANTHROPIC_BASE_URL',
  agent: 'claude',
  weight: 'warn',
  place: 'launchctl',
  line: null,
  effect: 'Trong các app mở từ Dock, Claude Code gửi yêu cầu tới địa chỉ này thay cho máy chủ của Anthropic.',
};

/** Checked a minute before the fixture's now; nothing set by default. */
export function envFixture(over: Partial<EnvView> = {}): EnvView {
  return {
    checkedAt: '2026-01-16T07:19:30Z',
    checking: false,
    vars: ENV_VARS,
    places: ENV_PLACES,
    found: [],
    ignored: 0,
    unread: [],
    ...over,
  };
}

/** The card the Rust side's `alerts::env_alerts` keeps for one variable. */
export function envAlert(f: EnvFound, at = 1768547700): AlertRecord {
  const key = 'env:' + f.name + ':' + f.place;
  const where = f.place === 'launchctl' ? 'launchctl' : (f.place === 'zshrc' ? '~/.zshrc' : '~/.zprofile') + ' dòng ' + f.line;
  return {
    id: key + '@' + at,
    alert: {
      key,
      kind: 'env',
      tone: f.weight,
      title: f.name + ' đang đè đăng nhập ' + (f.agent === 'claude' ? 'Claude Code' : f.agent === 'codex' ? 'Codex' : 'Antigravity'),
      body: 'Đặt ở ' + where + '. ' + f.effect,
      actions: [
        { id: 'env:settings', label: 'Xem' },
        { id: 'env:ignore:' + f.name, label: 'Bỏ qua' },
      ],
      at,
    },
    notified: false,
    answer: null,
    read: false,
  };
}
