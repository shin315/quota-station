// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Chép sang Mac khác lines and choices (M13) as plain functions. Every
// Mac, item and key is made up.

import { describe, expect, it } from 'vitest';
import { comparisonFixture } from './fakeCopy';
import { copyError, initialTicks, itemKey, itemNote, macState, picked, selectable, selectionOf, wrongWay } from './copy';
import type { CopyItem } from '@/lib/model';

const cmp = () => comparisonFixture('mac-b', 'Mac B');
const find = (id: string): CopyItem => cmp().diff.items.find((i) => i.id === id)!;

describe('which way an item can go', () => {
  it('sends only what the sending Mac has, and leaves the rest for the other way', () => {
    expect(wrongWay(find('design-kit'), true)).toBe(false);
    expect(wrongWay(find('design-kit'), false)).toBe(true);
    expect(wrongWay(find('tracker'), true)).toBe(true);
    expect(wrongWay(find('files'), true)).toBe(false);
    expect(wrongWay(find('files'), false)).toBe(false);
  });

  it('never brings a newer skill down to an older one', () => {
    const skill = find('ui-polish');
    expect(selectable(skill, true)).toBe(true);
    expect(selectable(skill, false)).toBe(false);
    expect(itemNote(skill, false, 'Mac Studio', 'Mac B', false)).toBe('Mac B có bản 12/09, máy này 17/10. Bản trên Mac Studio mới hơn.');
  });

  it('never copies a file that holds a key', () => {
    const held: CopyItem = { ...find('~/.claude/CLAUDE.md'), push: 'key', pull: 'key' };
    expect(selectable(held, true)).toBe(false);
    expect(selectable(held, false)).toBe(false);
    expect(itemNote(held, true, 'Mac Studio', 'Mac B', false)).toBe('Có dòng giống khoá: không chép');
  });
});

describe('the ticks', () => {
  it('ticks only what the sending Mac alone has; what both have, the owner ticks', () => {
    const push = initialTicks(cmp(), true);
    expect(Object.keys(push).sort()).toEqual(
      [itemKey(find('design-kit')), itemKey(find('deploy-check')), itemKey(find('formatter@tools-market'))].sort(),
    );
    const pull = initialTicks(cmp(), false);
    expect(Object.keys(pull)).toEqual([itemKey(find('tracker'))]);
  });

  it('sends a skill of another version as an update, and nothing that cannot go this way', () => {
    const c = cmp();
    const ticks = Object.fromEntries(c.diff.items.map((i) => [itemKey(i), true]));
    const sel = selectionOf(picked(c, ticks, true));
    expect(sel).toEqual({
      mcp: ['design-kit', 'files'],
      skills: ['deploy-check'],
      updates: ['ui-polish'],
      plugins: [{ agent: 'claude', id: 'formatter@tools-market' }],
      files: ['~/.claude/CLAUDE.md'],
    });
    expect(selectionOf(picked(c, ticks, false)).mcp).toEqual(['files', 'tracker']);
  });

  it('says where a key will be asked for, or that it goes along', () => {
    expect(itemNote(find('design-kit'), true, 'Mac Studio', 'Mac B', false)).toBe('Cần dán khoá trên Mac B');
    expect(itemNote(find('design-kit'), true, 'Mac Studio', 'Mac B', true)).toBe('Gửi kèm khoá qua SSH');
  });
});

describe('how the other Mac answered', () => {
  it('tells an unreachable Mac from one on version 1 or one without the app', () => {
    expect(macState('unreachable:ssh: connect to host mac-b port 22: Operation timed out')).toBe('unreach');
    expect(macState('timeout')).toBe('unreach');
    expect(macState('old')).toBe('old');
    expect(macState('noApp')).toBe('noApp');
    expect(macState('changed:codex')).toBeUndefined();
    expect(copyError('timeout', 'Mac B').title).toBe('Mac B không trả lời qua SSH');
    expect(copyError('old', 'Mac B').body).toMatch(/^Máy đó chạy bản 1\./);
    expect(copyError('changed:codex', 'Mac B').body).toMatch(/Chưa ghi gì/);
  });

  it('never shows the reply of ssh itself', () => {
    const e = copyError('unreachable:ssh: connect to host mac-b port 22: Operation timed out', 'Mac B');
    expect(e.title + e.body).not.toMatch(/port|ssh:|mac-b/);
  });
});
