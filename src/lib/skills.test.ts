// "Cập nhật tất cả" and "Cập nhật cả bộ" (M11): the sets by repository, the
// skills ticked when the preview opens, and the line after the run, on the
// fixture table. Every skill and repository is made up.

import { describe, expect, it } from 'vitest';
import { skillTableFixture } from '../fixtures/fakeSkills';
import type { SkillRow, SkillUpdate } from './model';
import { bulkPicked, bulkUpdatedFeedback, updateGroups } from './skills';

const newer: SkillUpdate = { fromAt: '2025-12-02T09:00:00Z', seenAt: 1768537000, plain: 'Sửa hướng dẫn', link: 'https://github.com/acme/agent-skills/commits/HEAD' };

/** The fixture with pdf-tools and release-notes of acme/agent-skills newer too, and a skill from nowhere known. */
function rows(): SkillRow[] {
  const base = skillTableFixture().rows.map((r) => (r.name === 'pdf-tools' || r.name === 'release-notes' ? { ...r, update: newer } : r));
  return [...base, { ...base.find((r) => r.name === 'pdf-tools')!, name: 'zip-tools', source: 'other', repo: null, update: newer }];
}

describe('updateGroups', () => {
  it('gathers the skills with a newer version by repository, those from nowhere known left out', () => {
    expect(updateGroups(rows())).toEqual([
      { repo: 'acme/agent-skills', names: ['pdf-tools', 'release-notes'] },
      { repo: 'dataworks/skills', names: ['sql-helper'] },
      { repo: 'uikit-lab/ui-skills', names: ['ui-polish'] },
    ]);
  });

  it('has no set for a repository with no newer version', () => {
    expect(updateGroups(skillTableFixture().rows).map((g) => g.repo)).toEqual(['dataworks/skills', 'uikit-lab/ui-skills']);
    expect(updateGroups(skillTableFixture().rows.map((r) => ({ ...r, update: null })))).toEqual([]);
  });
});

describe('bulkPicked', () => {
  it('ticks every skill with a newer version except those edited on this Mac', () => {
    expect(bulkPicked(rows(), ['pdf-tools', 'release-notes', 'sql-helper', 'ui-polish', 'zip-tools'])).toEqual(['pdf-tools', 'release-notes', 'ui-polish', 'zip-tools']);
  });

  it('never ticks a skill with no newer version', () => {
    expect(bulkPicked(skillTableFixture().rows, ['pdf-tools', 'ui-polish'])).toEqual(['ui-polish']);
  });
});

describe('bulkUpdatedFeedback', () => {
  const backups = '~/Library/Application Support/com.shin315.quota-station/backups/skills';

  it('says how many updated and where the copies are', () => {
    const fb = bulkUpdatedFeedback({
      results: [
        { name: 'pdf-tools', backup: 'pdf-tools.folder.1', error: null },
        { name: 'ui-polish', backup: 'ui-polish.folder.1', error: null },
      ],
      backups,
    });
    expect(fb).toMatchObject({ tone: 'info', title: 'Đã cập nhật 2 skill.' });
    expect(fb.body).toBe('Agent nào đang bật vẫn bật. Bản cũ đã lưu ở ' + backups + '.');
  });

  it('names each skill that failed and why, beside those updated', () => {
    const fb = bulkUpdatedFeedback({
      results: [
        { name: 'pdf-tools', backup: 'pdf-tools.folder.1', error: null },
        { name: 'release-notes', backup: 'release-notes.folder.1', error: { kind: 'failed', detail: 'Skill release-notes not found' } },
        { name: 'sql-helper', backup: 'sql-helper.folder.1', error: { kind: 'timeout' } },
      ],
      backups,
    });
    expect(fb).toMatchObject({ tone: 'warn', title: 'Đã cập nhật 1 skill, 2 skill chưa cập nhật.' });
    expect(fb.body).toBe('Chưa cập nhật release-notes (Skill release-notes not found), sql-helper (npx skills chạy quá lâu). Agent nào đang bật vẫn bật. Bản cũ đã lưu ở ' + backups + '.');
  });

  it('says nothing updated, and no copy when none was made', () => {
    const fb = bulkUpdatedFeedback({ results: [{ name: 'find-notes', backup: null, error: { kind: 'failed', detail: 'không có trong file khoá' } }], backups });
    expect(fb).toMatchObject({ tone: 'bad', title: 'Chưa cập nhật skill nào.', body: 'Chưa cập nhật find-notes (không có trong file khoá).' });
  });
});
