// The Sao lưu tab's words and the keys a restore asks for (M10).

import { describe, expect, it } from 'vitest';
import { planFixture } from '../fixtures/fakeBackup';
import {
  changedLines,
  fileKey,
  fileLabel,
  initialTicks,
  itemKey,
  itemLine,
  keysFor,
  moreFor,
  packWhen,
  partLines,
  partNotes,
  PLUGINS,
  resultLine,
  SKILLS_GITHUB,
  skippedLines,
  todoLines,
  undoneFeedback,
} from './backup';
import type { RestorePartResult } from './model';

const NOW = new Date(1768548000 * 1000);

describe('the restore preview', () => {
  it('ticks the adds, not the replaces, and asks only the ticked servers\' keys', () => {
    const plan = planFixture('agents-20260116-141000-studio-mini');
    const ticks = initialTicks(plan);
    expect(plan.items.filter((i) => ticks[itemKey(i)]).map((i) => i.name)).toEqual(['docs-search', 'files', 'tracker', 'notes']);
    expect(keysFor(plan, ticks).map((k) => k.mcp)).toEqual(['docs-search']);
    expect(keysFor(plan, { ...ticks, 'docs-search|add': false })).toEqual([]);
  });

  it('says each line in plain words', () => {
    const plan = planFixture('x');
    const line = (name: string) => itemLine(plan.items.find((i) => i.name === name)!);
    expect(line('docs-search')).toBe('Thêm cho cả ba agent · cần dán khoá');
    expect(line('files')).toBe('Thêm cho Claude Code, Codex · app tự chỉnh cho hợp máy này');
    expect(line('design-kit')).toBe('Máy này đã có, khác gói. Bỏ chọn là giữ bản trên máy.');
    expect(skippedLines({ ...plan, skipped: [{ agent: 'agy', reason: 'absent', name: null }] })).toEqual(['Máy này chưa cài Antigravity: bỏ qua phần của Antigravity.']);
  });

  it('reads a pack\'s time as the prototype does', () => {
    expect(packWhen(1768548000 - 600, NOW)).toBe('Hôm nay 14:10');
    expect(packWhen(1768548000 - 60_000, NOW)).toBe('Hôm qua 21:40');
    expect(packWhen(1768548000 - 200_000, NOW)).toBe('14/01 06:46');
  });
});

describe('the result', () => {
  it('says what was added off and what is left to do', () => {
    const done = {
      at: 0,
      items: [
        { name: 'docs', kind: 'add' as const, done: [], kept: ['agy' as const], why: 'noKey' as const, failed: [], skipped: null, login: false, command: null },
        { name: 'tracker', kind: 'add' as const, done: ['codex' as const], kept: [], why: null, failed: [], skipped: null, login: true, command: null },
      ],
      failed: null,
      written: [{ agent: 'codex' as const, changes: 1, backup: null }],
      keys: 0,
      canUndo: true,
    };
    expect(resultLine(done.items[0])).toEqual(['w', 'MCP docs · thêm nhưng tắt', 'chưa có khoá']);
    expect(resultLine(done.items[1])).toEqual(['', 'MCP tracker · 1 agent', 'có thể cần đăng nhập lại trong agent']);
    expect(todoLines(done)).toEqual([
      'Dán khoá cho docs ở tab MCP, rồi bật docs.',
      'Nếu agent hỏi, đăng nhập lại tracker trong Codex.',
      'Mở lại agent để dùng MCP vừa khôi phục.',
    ]);
  });
});

describe('the skills, plugins and instruction files (M12)', () => {
  const plan = planFixture('agents-20260116-141000-studio-mini');

  it('ticks the groups and the files this Mac has not got, and sends them', () => {
    const ticks = initialTicks(plan);
    expect([ticks[SKILLS_GITHUB], ticks[PLUGINS], ticks[fileKey('~/.claude/CLAUDE.md')], ticks[fileKey('~/.codex/AGENTS.md')]]).toEqual([true, true, true, false]);
    const more = moreFor(plan, { ...ticks, [SKILLS_GITHUB]: false });
    expect(more.skills).toEqual(['deploy-check', 'trace-reader', 'notes-helper']);
    expect(more.plugins).toHaveLength(7);
    expect(more.files).toEqual(['~/.claude/CLAUDE.md']);
  });

  it('names a file by its agent, never by its folder', () => {
    expect(fileLabel('~/.claude/CLAUDE.md', 'claude')).toBe('CLAUDE.md của Claude Code');
    expect(fileLabel('~/.claude/rules/style.md', 'claude')).toBe('quy tắc style.md của Claude Code');
    expect(fileLabel('~/notes/team.md', null)).toBe('team.md');
  });

  it('says what is left out, once each', () => {
    const p = {
      ...plan,
      skills: { items: [{ ...plan.skills!.items[0], blocked: 'noNpx' as const, checked: false }], same: [], leftOut: [{ name: 'secret-skill', why: 'key' as const }] },
      plugins: { ...plan.plugins!, items: [{ ...plan.plugins!.items[0], note: 'noSource' as const, checked: false }], skipped: [{ agent: 'codex' as const, reason: 'noCli' as const, name: null }] },
      files: { ...plan.files!, kept: [{ path: '~/.gemini/GEMINI.md', agent: 'agy' as const, why: 'key' as const }] },
    };
    expect(partNotes(p)).toEqual([
      'Máy này chưa có Node: chưa cài lại được pdf-tools. Cài Node rồi khôi phục lại.',
      'Skill secret-skill có dòng giống khoá nên không có trong gói.',
      'Máy này chưa cài Codex: bỏ qua plugin của Codex.',
      'Plugin issue-hub@acme-plugins lấy từ thư mục trên Mac kia: cài tay nếu cần.',
      'GEMINI.md của Antigravity trên máy có dòng giống khoá nên app giữ nguyên.',
    ]);
  });

  it('puts what was done together and each failure on its own with Thử lại', () => {
    const r = (over: Partial<RestorePartResult>): RestorePartResult => ({ part: 'skill', id: 'x', agent: null, kind: 'github', ok: true, why: null, retry: false, ...over });
    const parts = [
      r({ id: 'pdf-tools' }),
      r({ id: 'ui-polish' }),
      r({ id: 'notes-helper', kind: 'hand', ok: false, why: 'cloud', retry: true }),
      r({ part: 'plugin', id: 'issue-hub@acme-plugins', agent: 'claude', kind: '' }),
      r({ part: 'file', id: '~/.codex/AGENTS.md', agent: 'codex', kind: 'replace' }),
      r({ part: 'file', id: '~/.claude/CLAUDE.md', agent: 'claude', kind: 'add', ok: false, why: 'changed' }),
    ];
    const lines = partLines(parts).map((l) => [l.tone, l.title, l.sub, l.retry ? 'retry' : '']);
    expect(lines).toEqual([
      ['', '2 skill từ GitHub', 'pdf-tools, ui-polish', ''],
      ['', '1 plugin', 'issue-hub', ''],
      ['', 'AGENTS.md của Codex · thay bằng bản trong gói', '', ''],
      ['x', 'Skill notes-helper', 'iCloud Drive chưa tải về', 'retry'],
      ['x', 'CLAUDE.md của Claude Code', 'file vừa đổi trên máy', ''],
    ]);
    const done = { at: 0, items: [], failed: null, written: [], keys: 0, parts, canUndo: true };
    expect(todoLines(done)).toEqual([
      'Mở thư mục gói trong Finder để iCloud Drive tải về, rồi bấm Thử lại.',
      'CLAUDE.md của Claude Code vừa đổi trên máy: mở Khôi phục lần nữa để xem trước lại.',
      'Mở lại agent để dùng skill, plugin vừa khôi phục.',
    ]);
  });

  it('says what the undo found changed, and what it did', () => {
    expect(changedLines('changed:codex\nfile:~/.codex/AGENTS.md\nskill:notes-helper')).toEqual([
      'File của Codex đã sửa sau lần khôi phục.',
      'AGENTS.md của Codex đã sửa sau lần khôi phục.',
      'Skill notes-helper đã sửa sau lần khôi phục.',
    ]);
    expect(changedLines('changed:\nfile:~/notes/team.md')).toEqual(['team.md đã sửa sau lần khôi phục.']);
    expect(undoneFeedback({ outcome: null, kept: 0, keys: 0, skills: 2, plugins: 4, files: 1, left: 1 }).body).toBe(
      'Gỡ 2 skill, gỡ 4 plugin, trả 1 file hướng dẫn về như trước. Giữ nguyên 1 mục bạn đã sửa hoặc chưa gỡ được. Mở lại agent để dùng.',
    );
  });
});
