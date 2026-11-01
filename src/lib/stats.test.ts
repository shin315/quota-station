// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { describe, expect, it } from 'vitest';
import { firstReadFixture, SKIPPED_CODEX, statsFixture, statsReport } from '../fixtures/fakeStats';
import { bigTokens, num, progressLines, projectRows, readLine, share, skippedNotices, tokensText, unitOf, usd } from './stats';

describe('the numbers of Thống kê', () => {
  it('are written in Vietnamese, in one unit', () => {
    expect(num(1234.56)).toBe('1.234,6');
    expect(unitOf(45_500_000).long).toBe('triệu');
    expect(unitOf(2_100_000_000).short).toBe('tỷ');
    expect(tokensText(45_500_000)).toBe('45,5 triệu token');
    expect(tokensText(820)).toBe('820 token');
    expect(tokensText(12_310)).toBe('12,3 nghìn token');
    expect(tokensText(780_000)).toBe('780 nghìn token');
    expect(bigTokens(30_900_000)).toEqual({ value: '30,9', unit: 'triệu token' });
    expect(usd(412.4)).toBe('$412');
    expect(usd(61.2)).toBe('$61');
    expect(usd(3.456)).toBe('$3,46');
    expect(share(1, 3)).toBe('33%');
    expect(share(0, 0)).toBe('0%');
  });

  it('say when the logs were read', () => {
    expect(readLine(statsFixture())).toBe('cập nhật lúc 14:30');
    expect(readLine(statsFixture({ readAt: null }))).toBe('chưa cập nhật');
    expect(readLine(firstReadFixture())).toBe('đang cập nhật…');
  });

  it('give each agent a line on the first read', () => {
    const p = progressLines(firstReadFixture());
    expect(p.lines).toEqual([
      ['', 'Claude Code · 2.140 file log'],
      ['run', 'Codex · 640/960 file'],
      ['wait', 'Antigravity · chưa có log token trên máy'],
    ]);
    expect(p.percent).toBe(90);
    expect(progressLines(statsFixture({ reading: true, first: true })).lines[0]).toEqual(['wait', 'Claude Code']);
  });

  it('tell skipped sessions until "Đã biết" counted them', () => {
    const r = statsReport('week', SKIPPED_CODEX);
    expect(skippedNotices(r).map((n) => n.title)).toEqual(['Bỏ qua 38 phiên Codex']);
    const seen = statsReport('week', SKIPPED_CODEX.map((s) => ({ ...s, acknowledged: s.sessions })));
    expect(skippedNotices(seen)).toEqual([]);
    // More sessions skipped after "Đã biết" bring it back.
    const more = statsReport('week', SKIPPED_CODEX.map((s) => ({ ...s, acknowledged: s.sessions ? s.sessions - 1 : 0 })));
    expect(skippedNotices(more)).toHaveLength(1);
  });

  it('keep seven projects and add up the rest', () => {
    const r = statsReport('week');
    r.projects = Array.from({ length: 10 }, (_, i) => ({ name: 'p' + i, path: '~/p' + i, tokens: 100 - i }));
    const rows = projectRows(r);
    expect(rows).toHaveLength(8);
    expect(rows[7]).toEqual({ name: 'Các dự án khác', path: '', tokens: 93 + 92 + 91 });
    r.projects = [{ name: '', path: '', tokens: 5 }];
    expect(projectRows(r)[0].name).toBe('Không rõ thư mục');
  });
});
