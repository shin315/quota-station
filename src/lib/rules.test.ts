// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { describe, expect, it } from 'vitest';
import { hideNames } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel, peerFixture, rotationFixture, rulesFixture } from '../fixtures/models';
import type { Model, PeerView } from './model';
import {
  changedBanner,
  compareRows,
  hoursText,
  keyWords,
  markLeft,
  peerLine,
  ruleChanges,
  ruleEmails,
  ruleNames,
  rulesErrorBanner,
  rulesProblem,
  rotationBanner,
  saveFeedback,
  sidebarLine,
  syncSummary,
} from './rules';

const RETRY = '2026-01-16T07:25:40.000Z'; // 14:25 at UTC+7

function withPeers(...peers: PeerView[]): Model {
  return fixtureModel('fresh', { rotation: rotationFixture({ peers }) });
}

describe('rules view', () => {
  it('the file hours take their last hour in', () => {
    expect(hoursText(8, 23)).toBe('08:00–24:00');
    expect(hoursText(0, 6)).toBe('00:00–07:00');
  });

  it("names the rules' reasons by key", () => {
    expect(keyWords('rotation_bad_value:personal_hours')).toBe('khoá personal_hours sai');
    expect(keyWords('rotation_missing_key:full')).toBe('thiếu khoá full');
    expect(keyWords('rotation_malformed_line')).toBe('có dòng không đọc được');
    expect(keyWords('rotation_personal_in_team')).toBe('tài khoản cá nhân cũng nằm trong vòng Team');
  });

  it('lists every change by account name', () => {
    const nameOf = ruleNames(fixtureModel('fresh'));
    const before = rulesFixture();
    const after = rulesFixture({
      team: [3, 2],
      personal: null,
      personalFrom: 9,
      personalTo: 21,
      full: 85,
      fableFull: 97,
      expire: [{ slot: 3, at: '2026-01-17T09:00' }],
    });
    expect(ruleChanges(before, after, nameOf, FIXTURE_NOW)).toEqual([
      'Tài khoản Team được xoay vòng: lan.acme1, lan.acme2, lan.acme3 → lan.acme2, lan.acme1',
      'Tài khoản cá nhân: lanpham.dev → không có',
      'Giờ dùng tài khoản cá nhân: 08:00–24:00 → 09:00–22:00',
      'Ngưỡng tự chuyển: 90% → 85%',
      'Ngưỡng Fable tuần: 95% → 97%',
      'Giờ rời vòng của lan.acme1: 17:45 → chưa đặt',
      'Giờ rời vòng của lan.acme2: chưa đặt → ngày mai 09:00',
    ]);
    expect(ruleChanges(before, rulesFixture(), nameOf, FIXTURE_NOW)).toEqual([]);
  });

  it('names accounts by email while names show, else by place, never capitalized', () => {
    const shown = ruleNames(fixtureModel('fresh'));
    expect([1, 2, 3, 4, 9].map(shown)).toEqual(['lanpham.dev', 'lan.acme1', 'lan.acme2', 'lan.acme3', 'Tài khoản 9']);
    expect(ruleEmails(fixtureModel('fresh'))(2)).toBe('lan.acme1@example.com');
    const hidden = hideNames(fixtureModel('fresh'));
    expect([1, 2, 3, 4].map(ruleNames(hidden))).toEqual(['Cá nhân', 'Team A', 'Team B', 'Team C']);
    expect(ruleEmails(hidden)(2)).toBe('');
  });

  it('places each slider mark where its value is', () => {
    expect([80, 85, 90, 95, 98].map((v) => markLeft(v, 80, 98))).toEqual([0, 27.78, 55.56, 83.33, 100]);
    expect(markLeft(90, 90, 90)).toBe(0);
  });

  it('says why rules cannot be saved yet', () => {
    expect(rulesProblem(rulesFixture())).toBeNull();
    expect(rulesProblem(rulesFixture({ team: [] }))).toBe('Chọn ít nhất một tài khoản Team.');
    expect(rulesProblem(rulesFixture({ personalFrom: 20, personalTo: 7 }))).toBe('Giờ kết thúc phải sau giờ bắt đầu.');
    expect(rulesProblem(rulesFixture({ team: [1, 2], personal: 1 }))).toBe('Tài khoản cá nhân không được nằm trong vòng Team.');
  });

  it('one status per Mac', () => {
    const line = (over: Partial<PeerView>) => peerLine(peerFixture('Mac C', 'mac-c', over));
    expect(line({})).toMatchObject({ tone: 'ok', text: 'đã nhận 14:02', short: 'Mac C: 14:02' });
    expect(line({ state: { kind: 'unreachable', error: 'ssh: connect to host mac-c port 22: Operation timed out', retryAt: RETRY } })).toMatchObject({
      tone: 'warn',
      text: 'chưa tới được · thử lại 14:25',
      short: 'Mac C: chưa tới được, sẽ thử lại',
      title: 'ssh: connect to host mac-c port 22: Operation timed out',
    });
    expect(line({ state: { kind: 'mismatch', retryAt: RETRY } }).short).toBe('Mac C: đọc lại không khớp, sẽ thử lại');
    expect(line({ state: { kind: 'failed', error: 'Permission denied', retryAt: RETRY } }).tone).toBe('bad');
    expect(line({ state: { kind: 'newer', their: { at: '2026-01-16T14:11:00+07:00', machine: 'Mac C' } } })).toMatchObject({
      tone: 'info',
      text: 'có bản mới hơn 14:11',
    });
    expect(line({ state: { kind: 'edited', their: { at: '2026-01-16T14:02:00+07:00', machine: 'Mac mini' } } }).short).toBe('Mac C: bản đã bị sửa tay');
    expect(line({ state: { kind: 'edited', their: null } }).short).toBe('Mac C: bản đã bị sửa tay');
    expect(line({ current: false }).short).toBe('Mac C: chưa nhận bản mới, sẽ gửi');
    expect(line({ state: null }).short).toBe('Mac C: chưa gửi lần nào');
    expect(line({ pushing: true }).text).toBe('đang gửi…');
  });

  it('the status line and the sidebar sum the Macs up', () => {
    const fine = fixtureModel('fresh');
    expect(syncSummary(fine)).toEqual({ tone: 'ok', text: 'Mac B: 14:02 · Mac C: 14:02' });
    expect(sidebarLine(fine)).toBe('luật đã gửi tới 2 Mac khác lúc 14:02');
    const down = withPeers(peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'unreachable', error: 'x', retryAt: RETRY } }));
    expect(syncSummary(down)).toEqual({ tone: 'warn', text: 'Mac B: 14:02 · Mac C: chưa tới được, sẽ thử lại' });
    expect(sidebarLine(down)).toBe('1 Mac chưa nhận luật mới');
    expect(syncSummary(withPeers())).toBeNull();
    expect(sidebarLine(withPeers())).toBeNull();
  });

  it('banners for the file on disk and the other Macs', () => {
    expect(rotationBanner(fixtureModel('fresh'))).toBeNull();
    const missing = fixtureModel('fresh', { rotation: rotationFixture({ status: 'missing', rules: null, version: null, stamp: null }) });
    expect(rotationBanner(missing)).toMatchObject({ title: 'Chưa có file luật trên máy này', actions: [{ act: 'rules:create' }] });
    const invalid = fixtureModel('fresh', { rotation: rotationFixture({ status: 'invalid', rules: null, reason: 'rotation_bad_value:personal_hours' }) });
    expect(rotationBanner(invalid)?.body).toMatch(/^Khoá personal_hours sai\./);

    const newer = withPeers(
      peerFixture('Mac B', 'mac-b', { state: { kind: 'newer', their: { at: '2026-01-16T14:11:00+07:00', machine: 'Mac B' } }, theirRules: rulesFixture({ full: 88 }) }),
    );
    expect(rotationBanner(newer)).toEqual({
      tone: 'info',
      title: 'Bản trên Mac B mới hơn (14:11)',
      body: 'Có người sửa luật trên Mac B sau bản 14:02 của máy này. App không tự gộp hai bản. Tải bản đó về?',
      actions: [
        { label: 'Dùng bản trên Mac B', act: 'rules:adopt:mac-b' },
        { label: 'So sánh', act: 'rules:compare:mac-b' },
      ],
    });

    const unread = withPeers(peerFixture('Mac B', 'mac-b', { state: { kind: 'edited', their: null }, theirRules: null }));
    expect(rotationBanner(unread)).toMatchObject({
      title: 'Bản trên Mac B đã bị sửa tay',
      body: 'File luật trên Mac B không có dòng updated của app, hoặc app không đọc được nó; có thể đã được sửa tay. App không ghi đè. App không tự gộp hai bản. Tải bản đó về?',
      actions: [{ label: 'So sánh', act: 'rules:compare:mac-b' }],
    });

    const down = withPeers(peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'unreachable', error: 'x', retryAt: RETRY } }));
    expect(rotationBanner(down)).toEqual({
      tone: 'warn',
      title: 'Chưa gửi được luật tới Mac C',
      body: 'Đã lưu trên máy này và Mac B lúc 14:02. App thử lại mỗi 5 phút.',
      actions: [{ label: 'Thử lại', act: 'rules:retry' }],
    });
  });

  it('a newer copy landing while the editor holds changes: the newer one wins until the owner chooses', () => {
    const m = fixtureModel('fresh', { rotation: rotationFixture({ version: 'v2', stamp: { at: '2026-01-16T14:15:00+07:00', machine: 'Mac B' } }) });
    expect(changedBanner(m)).toMatchObject({
      title: 'Bản trên Mac B mới hơn, tải lại?',
      actions: [
        { label: 'Tải lại', act: 'rules:reload' },
        { label: 'Giữ bản đang sửa', act: 'rules:overwrite' },
      ],
    });
  });

  it('a save the rules refuse says which key, and nothing was written', () => {
    const b = rulesErrorBanner({ kind: 'invalid', detail: 'rotation_bad_value:personal_hours', restored: false });
    expect(b.title).toBe('Luật chưa hợp lệ');
    expect(b.body).toBe('Khoá personal_hours sai. Chưa ghi gì.');
    const w = rulesErrorBanner({ kind: 'write', detail: 'Permission denied', restored: false });
    expect(w).toMatchObject({ title: 'Không ghi được file luật', body: 'Permission denied. Máy này giữ nguyên.' });
  });

  it('the feedback after a save names the Macs left behind', () => {
    const now = rotationFixture({ stamp: { at: '2026-01-16T14:21:00+07:00', machine: 'Mac mini' } });
    expect(saveFeedback(fixtureModel('fresh', { rotation: now }))).toEqual({
      tone: '',
      title: 'Đã lưu luật cho cả ba Mac lúc 14:21',
      body: 'Đã kiểm tra đủ 6 mục của luật ✓',
    });
    const down = { ...now, peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'unreachable', error: 'x', retryAt: RETRY } })] } as Model['rotation'];
    expect(saveFeedback(fixtureModel('fresh', { rotation: down }))).toEqual({
      tone: 'warn',
      title: 'Đã lưu luật trên máy này lúc 14:21',
      body: 'Đã kiểm tra đủ 6 mục ✓. Mac C: chưa tới được, sẽ thử lại.',
    });
    expect(saveFeedback(fixtureModel('fresh', { rotation: { ...now, peers: [] } })).title).toBe('Đã lưu luật cho máy này lúc 14:21');
  });

  it('compares two copies row by row', () => {
    const rows = compareRows(rulesFixture(), rulesFixture({ full: 88, personalTo: 21 }), ruleNames(fixtureModel('fresh')), FIXTURE_NOW);
    expect(rows.filter((r) => r.diff).map((r) => [r.label, r.mine, r.theirs])).toEqual([
      ['Giờ dùng tài khoản cá nhân', '08:00–24:00', '08:00–22:00'],
      ['Ngưỡng tự chuyển', '90%', '88%'],
    ]);
    expect(rows.find((r) => r.label === 'Giờ rời vòng')?.mine).toBe('lan.acme1 17:45');
  });
});
