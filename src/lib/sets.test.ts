// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Bộ cấu hình tab's words and rules (M15) on the made-up fixtures.

import { describe, expect, it } from 'vitest';
import { DOCS, fakeSetChanges, WEB } from '../fixtures/fakeSets';
import { skillTableFixture } from '../fixtures/fakeSkills';
import { codexFixture, mcpTableFixture } from '../fixtures/models';
import type { ConfigSet } from './model';
import { codexStep, driftChip, fromCurrent, sessionsLine, setErrorText, setSummary, undoneFeedback } from './sets';

describe('configuration sets', () => {
  it('takes the configuration as it is, so that applying it changes nothing', () => {
    const mcp = mcpTableFixture();
    const skills = skillTableFixture();
    const cur = fromCurrent(mcp, skills);
    expect(cur.mcp).toEqual({ 'design-kit': 'on', 'docs-search': 'on', files: 'on', 'ticket-desk': 'off', tracker: 'off' });
    expect(cur.mcp.browser).toBeUndefined();
    expect(cur.skills['pdf-tools']).toBe('on');
    expect(cur.skills['ui-polish']).toBeUndefined();
    const set: ConfigSet = { id: 'x', name: 'Hiện tại', ...cur, codex: null };
    const ch = fakeSetChanges(set, mcp, skills);
    expect(ch.mcp).toEqual([]);
    expect(ch.skills).toEqual([]);
  });

  it('sums a set up in one line', () => {
    const codex = codexFixture();
    expect(setSummary(WEB, codex)).toBe('Bật 3 MCP, tắt 1 · bật 2 skill · Codex giữ nguyên');
    expect(setSummary({ ...WEB, mcp: {}, skills: {}, codex: 'custom' }, codex)).toBe('Codex sang relay');
    expect(setSummary({ ...WEB, mcp: { a: 'off' }, skills: {} }, codex)).toBe('Tắt 1 MCP · Codex giữ nguyên');
  });

  it('opens the Codex switch only when Codex is on the other side', () => {
    expect(codexStep(DOCS, codexFixture())).toBe('chatGpt');
    expect(codexStep(DOCS, codexFixture({ current: 'chatgpt' }))).toBeNull();
    expect(codexStep(WEB, codexFixture())).toBeNull();
    expect(codexStep(DOCS, codexFixture({ present: false }))).toBeNull();
  });

  it('offers the Codex switch back after an undo, never switching itself', () => {
    const fb = undoneFeedback('Viết tài liệu', { skills: null, mcp: null, left: ['docs'], ok: true, codexBefore: 'custom' }, codexFixture({ current: 'chatgpt' }));
    expect(fb.body).toBe('MCP và skills trở về như trước khi áp dụng. Giữ docs: đã đổi sau khi áp dụng. Codex vẫn ở tài khoản ChatGPT.');
    expect(fb.actions).toEqual([{ label: 'Chuyển Codex về relay', act: 'codex:switch:custom' }]);
    const same = undoneFeedback('Viết tài liệu', { skills: null, mcp: null, left: [], ok: true, codexBefore: 'custom' }, codexFixture());
    expect(same.actions).toBeUndefined();
  });

  it('says who keeps the old configuration, and why a write did not run', () => {
    expect(sessionsLine(0, codexFixture({ procs: null }))).toBe('Phiên đang chạy giữ cấu hình cũ đến khi mở lại.');
    expect(sessionsLine(2, codexFixture({ procs: { sessions: 0, app: false, daemon: true, ccSwitch: false } }))).toBe(
      '2 phiên Claude Code đang chạy giữ cấu hình cũ đến khi mở lại.',
    );
    expect(driftChip(0)).toEqual({ t: 'đúng như bộ', k: 'ok' });
    expect(driftChip(2).t).toBe('đã lệch 2 mục');
    expect(setErrorText('changed:agy')).toMatch(/file của Antigravity vừa đổi/);
    expect(setErrorText('taken')).toBe('Đã có bộ cùng tên.');
  });
});
