// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cập nhật Agent in words: what each tool's row says and offers, and the
// line at the top of the page, from the made-up Mac of fakeCliTools.

import { describe, expect, it } from 'vitest';
import { CLI_STATES, cliToolFixture, cliToolsFixture } from '../fixtures/fakeCliTools';
import { FIXTURE_NOW } from '../fixtures/models';
import type { CliToolView } from './model';
import { cliRow, cliSummary, failureText } from './cliTools';

/** Checked at 14:02 the same day, nothing checking now. */
const CHECKED = { checking: false, checkedAt: cliToolsFixture().checkedAt };
const row = (v: CliToolView, view = CHECKED) => cliRow(v, view, FIXTURE_NOW);

describe('Cập nhật Agent, a row', () => {
  it('a tool on the newest version says so in one line; how and when are behind Chi tiết', () => {
    const r = row(cliToolFixture('claude', { running: 0 }));
    expect(r.line).toEqual({ tone: 'ok', text: '2.1.311 · Đã là bản mới nhất' });
    expect(r.check).toBe('latest');
    expect(r.action).toEqual({ kind: 'none' });
    expect(r.more).toEqual(['Cài bằng trình cài chính thức.', 'Kiểm tra lúc 14:02.']);
  });

  it('a check from another day says its day', () => {
    const r = row(cliToolFixture('claude'), { checking: false, checkedAt: Date.parse('2026-01-14T13:41:00Z') / 1000 });
    expect(r.more).toContain('Kiểm tra lúc 14/01 20:41.');
  });

  it('a check running says so on every row, and "Cập nhật" waits for it', () => {
    const r = row(cliToolFixture('codex'), { ...CHECKED, checking: true });
    expect(r.line).toEqual({ tone: '', text: '0.163.0 → 0.164.0 · Đang kiểm tra…' });
    expect(r.action).toEqual({ kind: 'none' });
  });

  it('a feed that did not answer says why on the row', () => {
    const r = row(cliToolFixture('codex', { latest: null, newer: false, failed: '503' }));
    expect(r.line).toEqual({ tone: 'bad', text: '0.163.0 · Không kiểm tra được: máy chủ trả lỗi 503' });
    expect(r.check).toBe('failed');
    expect(row(cliToolFixture('agy', { latest: null, newer: false, failed: 'offline' })).line.text).toBe('1.4.7 · Không kiểm tra được: không tới được máy chủ');
  });

  it('a tool never checked says so', () => {
    const r = row(cliToolFixture('claude', { latest: null }), { checking: false, checkedAt: null });
    expect(r.line).toEqual({ tone: 'off', text: '2.1.311 · Chưa kiểm tra' });
  });

  it('a newer version offers "Cập nhật" and shows both versions', () => {
    const r = row(cliToolFixture('codex'));
    expect(r.line).toEqual({ tone: 'info', text: '0.163.0 → 0.164.0 · Có bản mới' });
    expect(r.more[0]).toBe('Cài bằng Homebrew.');
    expect(r.action).toEqual({ kind: 'update' });
  });

  it('Claude Code updates while its sessions run, and says behind Chi tiết they keep the old version', () => {
    const r = row(cliToolFixture('claude', { latest: '2.1.312', newer: true }));
    expect(r.action).toEqual({ kind: 'update' });
    expect(r.more).toContain('3 phiên Claude Code đang chạy giữ bản cũ tới khi mở lại.');
  });

  it('Codex waits while one of its sessions is open, and asks to close them first', () => {
    const r = row(cliToolFixture('codex', { running: 2 }));
    expect(r.action).toEqual({ kind: 'recheck' });
    expect(r.line).toEqual({ tone: 'warn', text: '0.163.0 → 0.164.0 · Chờ đóng 2 phiên Codex CLI' });
    expect(r.more).toContain('2 phiên Codex CLI đang mở. Đóng chúng rồi bấm Kiểm tra lại; cập nhật lúc này có thể làm chúng lỗi.');
  });

  it('Claude Code from npm is left to the owner with the command that moves it', () => {
    const r = row(cliToolFixture('claude', { install: 'npm', handsOff: 'npm', latest: '2.1.312', newer: true, command: 'claude install' }));
    expect(r.line).toEqual({ tone: 'warn', text: '2.1.311 → 2.1.312 · Cập nhật trong Terminal' });
    expect(r.command).toBe('claude install');
    expect(r.action).toEqual({ kind: 'none' });
    expect(r.more.some((l) => l.startsWith('Cài qua npm, cách cũ'))).toBe(true);
  });

  it('a copy inside another app offers no command', () => {
    const r = row(cliToolFixture('codex', { install: null, latest: null, newer: false, handsOff: 'bundled', bundledBy: 'Codex.app', command: null }));
    expect(r.line).toEqual({ tone: '', text: '0.163.0 · Do Codex.app tự cập nhật' });
    expect(r.command).toBeNull();
    expect(r.more).toContain('Bản này nằm trong Codex.app và do ứng dụng đó tự cập nhật. App không đụng vào.');
  });

  it('a folder that needs an administrator gives the command to run in Terminal', () => {
    const command = 'sudo npm install -g @openai/codex@latest';
    const r = row(cliToolFixture('codex', { install: 'npm', handsOff: 'unwritable', command }));
    expect(r.command).toBe(command);
    expect(r.action).toEqual({ kind: 'none' });
  });

  it('a tool not installed gives its official install command', () => {
    const r = row(cliToolFixture('agy', { version: null, latest: null, newer: false, install: null, path: null, command: null }));
    expect(r.line).toEqual({ tone: 'off', text: 'Chưa cài trên Mac này' });
    expect(r.check).toBeNull();
    expect(r.command).toBe('curl -fsSL https://antigravity.google/cli/install.sh | bash');
  });

  it('an update running shows its command behind Chi tiết', () => {
    const r = row(cliToolFixture('agy', { updating: true }));
    expect(r.action).toEqual({ kind: 'updating' });
    expect(r.line.text).toBe('1.4.7 → 1.4.8 · Đang cập nhật…');
    expect(r.more).toContain('Đang chạy `agy update`.');
  });

  it('an update that worked says from which version', () => {
    const r = row(cliToolFixture('codex', { version: '0.164.0', newer: false, last: { ok: true, from: '0.163.0', to: '0.164.0', at: 1 } }));
    expect(r.line).toEqual({ tone: 'ok', text: '0.164.0 · Đã cập nhật từ 0.163.0' });
    expect(r.more).toContain('Phiên mở sau dùng bản mới.');
  });

  it('an update that failed says so, keeps "Cập nhật", and has why behind Chi tiết', () => {
    const v = cliToolFixture('codex', { last: { ok: false, failure: 'permission', detail: '/opt/homebrew/Caskroom/codex', log: '~/x.log', at: 1 } });
    const r = row(v);
    expect(r.action).toEqual({ kind: 'update' });
    expect(r.line).toEqual({ tone: 'bad', text: '0.163.0 → 0.164.0 · Cập nhật không xong' });
    expect(r.more).toContain('Không ghi được vào /opt/homebrew/Caskroom/codex. Cần quyền quản trị: chạy `sudo brew upgrade --cask codex` trong Terminal.');
  });

  it('a session that opened before the update, or sessions that could not be read, are said', () => {
    const v = cliToolFixture('codex');
    expect(failureText(v, 'started', '1')).toBe('Có 1 phiên Codex CLI vừa mở nên chưa cập nhật. Đóng rồi bấm Kiểm tra lại.');
    expect(failureText(v, 'sessions', '')).toMatch(/^Không đọc được các tiến trình đang chạy nên chưa cập nhật/);
  });
});

describe('Cập nhật Agent, the line at the top', () => {
  const summary = (v = cliToolsFixture()) => cliSummary(v, FIXTURE_NOW);

  it('says when every tool is the newest, and when that was checked', () => {
    expect(summary(CLI_STATES.latest(cliToolsFixture()))).toEqual({ tone: 'ok', text: 'Tất cả đã là bản mới nhất', when: 'kiểm tra lúc 14:02' });
  });

  it('counts the tools with a new version, naming one alone', () => {
    expect(summary()).toEqual({ tone: 'info', text: '2 công cụ có bản mới', when: 'kiểm tra lúc 14:02' });
    const one = CLI_STATES.latest(cliToolsFixture());
    expect(summary({ ...one, tools: one.tools.map((t) => (t.tool === 'agy' ? { ...t, version: '1.4.7', newer: true } : t)) }).text).toBe('Antigravity CLI có bản mới 1.4.8');
  });

  it('names a tool it could not check', () => {
    expect(summary(CLI_STATES.failed(CLI_STATES.latest(cliToolsFixture())))).toEqual({ tone: 'warn', text: 'Không kiểm tra được Codex CLI', when: 'kiểm tra lúc 14:02' });
    expect(summary(CLI_STATES.failed(cliToolsFixture())).text).toBe('Antigravity CLI có bản mới 1.4.8 · không kiểm tra được Codex CLI');
    expect(summary(CLI_STATES.offline(cliToolsFixture()))).toEqual({ tone: 'bad', text: 'Không kiểm tra được: không tới được máy chủ nào', when: 'kiểm tra lúc 14:02' });
  });

  it('says a check is running, or that none has run', () => {
    expect(summary(CLI_STATES.checking(cliToolsFixture()))).toEqual({ tone: 'stale', text: 'Đang kiểm tra bản mới…', when: null });
    expect(summary(CLI_STATES.never(cliToolsFixture()))).toEqual({ tone: 'off', text: 'Chưa kiểm tra lần nào.', when: null });
  });
});
