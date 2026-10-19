// The Sao lưu tab's words and the keys a restore asks for (M10).

import { describe, expect, it } from 'vitest';
import { planFixture } from '../fixtures/fakeBackup';
import { initialTicks, itemKey, itemLine, keysFor, packWhen, resultLine, skippedLines, todoLines } from './backup';

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
