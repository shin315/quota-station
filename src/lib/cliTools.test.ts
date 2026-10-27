// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cập nhật Agent's rows in words: what each tool's row says and offers,
// from the made-up Mac of fakeCliTools.

import { describe, expect, it } from 'vitest';
import { cliToolFixture } from '../fixtures/fakeCliTools';
import { cliRow, failureText } from './cliTools';

describe('Cập nhật Agent, a row', () => {
  it('a tool on the newest version says so and offers nothing to run', () => {
    const r = cliRow(cliToolFixture('claude', { running: 0 }));
    expect(r.status).toBe('2.1.311 · mới nhất · trình cài chính thức');
    expect(r.action).toEqual({ kind: 'latest' });
    expect(r.note).toBeNull();
  });

  it('a newer version offers "Cập nhật" and shows both versions', () => {
    const r = cliRow(cliToolFixture('codex'));
    expect(r.status).toBe('0.163.0 → 0.164.0 · Homebrew');
    expect(r.action).toEqual({ kind: 'update' });
  });

  it('Claude Code updates while its sessions run, and says they keep the old version', () => {
    const r = cliRow(cliToolFixture('claude', { latest: '2.1.312', newer: true }));
    expect(r.action).toEqual({ kind: 'update' });
    expect(r.note?.text).toBe('3 phiên Claude Code đang chạy giữ bản cũ tới khi mở lại.');
  });

  it('Codex waits while one of its sessions is open, and asks to close them first', () => {
    const r = cliRow(cliToolFixture('codex', { running: 2 }));
    expect(r.action).toEqual({ kind: 'recheck' });
    expect(r.note).toEqual({ tone: 'warn', text: '2 phiên Codex CLI đang mở. Đóng chúng rồi bấm Kiểm lại; cập nhật lúc này có thể làm chúng lỗi.' });
  });

  it('Claude Code from npm is left to the owner with the command that moves it', () => {
    const r = cliRow(cliToolFixture('claude', { install: 'npm', handsOff: 'npm', latest: '2.1.312', newer: true, command: 'claude install' }));
    expect(r.action).toEqual({ kind: 'copy', command: 'claude install' });
    expect(r.note?.tone).toBe('warn');
    expect(r.note?.text).toMatch(/^Cài qua npm, cách cũ/);
  });

  it('a copy inside another app offers no command', () => {
    const r = cliRow(cliToolFixture('codex', { install: null, handsOff: 'bundled', bundledBy: 'Codex.app', command: null }));
    expect(r.action).toEqual({ kind: 'none' });
    expect(r.note?.text).toBe('Bản này nằm trong Codex.app và do ứng dụng đó tự cập nhật. App không đụng vào.');
  });

  it('a folder that needs an administrator gives the command to run in Terminal', () => {
    const command = 'sudo npm install -g @openai/codex@latest';
    const r = cliRow(cliToolFixture('codex', { install: 'npm', handsOff: 'unwritable', command }));
    expect(r.action).toEqual({ kind: 'copy', command });
  });

  it('a tool not installed gives its official install command', () => {
    const r = cliRow(cliToolFixture('agy', { version: null, latest: null, newer: false, install: null, path: null, command: null }));
    expect(r.status).toBe('Chưa cài trên Mac này.');
    expect(r.action).toEqual({ kind: 'install', command: 'curl -fsSL https://antigravity.google/cli/install.sh | bash' });
  });

  it('an update running shows its command', () => {
    const r = cliRow(cliToolFixture('agy', { updating: true }));
    expect(r.action).toEqual({ kind: 'updating' });
    expect(r.note?.text).toBe('Đang chạy `agy update`…');
  });

  it('an update that worked says from which version to which', () => {
    const r = cliRow(cliToolFixture('codex', { version: '0.164.0', newer: false, last: { ok: true, from: '0.163.0', to: '0.164.0', at: 1 } }));
    expect(r.note).toEqual({ tone: 'ok', text: 'Đã cập nhật 0.163.0 → 0.164.0. Phiên mở sau dùng bản mới.' });
  });

  it('an update that failed says why under the row and keeps "Cập nhật"', () => {
    const v = cliToolFixture('codex', { last: { ok: false, failure: 'permission', detail: '/opt/homebrew/Caskroom/codex', log: '~/x.log', at: 1 } });
    const r = cliRow(v);
    expect(r.action).toEqual({ kind: 'update' });
    expect(r.note).toEqual({ tone: 'bad', text: 'Không ghi được vào /opt/homebrew/Caskroom/codex. Cần quyền quản trị: chạy `sudo brew upgrade --cask codex` trong Terminal.' });
  });

  it('a session that opened before the update, or sessions that could not be read, are said', () => {
    const v = cliToolFixture('codex');
    expect(failureText(v, 'started', '1')).toBe('Có 1 phiên Codex CLI vừa mở nên chưa cập nhật. Đóng rồi bấm Kiểm lại.');
    expect(failureText(v, 'sessions', '')).toMatch(/^Không đọc được các tiến trình đang chạy nên chưa cập nhật/);
  });
});
