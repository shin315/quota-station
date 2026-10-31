// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { describe, expect, it } from 'vitest';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import { popoverView } from './view';
import { windowView } from './window';

const view = (name: string, over = {}) => windowView(fixtureModel(name, over), FIXTURE_NOW);

describe('main window view', () => {
  it('shows the same snapshot as the popover, both in slot order', () => {
    const model = fixtureModel('fresh');
    const w = windowView(model, FIXTURE_NOW);
    const p = popoverView(model, FIXTURE_NOW);
    expect(w.machine).toBe('Mac Studio');
    expect(w.fresh).toEqual(p.fresh);
    expect(w.banner).toEqual(p.banner);
    expect(w.claude.cards.map((c) => c.slot)).toEqual([1, 2, 3, 4]);
    // The popover lists the accounts in the same order, the one in use in its place.
    expect(w.claude.cards).toEqual(p.claude.cards);
    expect(w.agy.cards.map((c) => c.name)).toEqual(['lanpham.ultra', 'lan.ultra2']);
    expect(w.agy.cards).toEqual(p.agy.cards);
  });

  it('counts the Claude accounts by what the Mac has', () => {
    expect(view('fresh').claude.count).toBe('4 tài khoản Quota Station');
    // Before Quota Station keeps the accounts, claude-swap still does.
    expect(view('fresh', { claudeSwitch: { ready: false, busy: null } }).claude.count).toBe('4 tài khoản claude-swap');
    const unavailable = view('unavailable').claude;
    expect(unavailable).toMatchObject({ count: '', cards: [] });
    expect(unavailable.note?.text).toBe('Không đọc được hạn mức của Claude Code (network_unreachable).');
  });

  it('writes resets in the tables as the popover does, minutes only within a day', () => {
    const [personal, acme1, acme2] = view('fresh').claude.cards;
    expect(personal.cells.map((c) => c.reset)).toEqual(['chưa dùng', '6d 7h', '6d 7h']);
    expect(acme1.cells[0].reset).toBe('3h 8m');
    expect(acme2.cells[1]).toMatchObject({ none: true, reset: 'không có' });
    expect(view('reset-pending').claude.cards[3].cells[0].reset).toBe('chờ số');
  });

  it('a Mac without an account list sees why only one Antigravity account shows', () => {
    const a = view('single').agy;
    expect(a.banner).toEqual({
      tone: 'info',
      title: 'Chỉ thấy tài khoản đang dùng',
      body: 'Chưa có danh sách tài khoản Antigravity trên máy này. Đăng nhập tài khoản ở đây để thấy và chuyển mọi tài khoản.',
      actions: [],
    });
    expect(a.own).toBe(false);
    expect(view('fresh').agy).toMatchObject({ present: true, banner: null, own: true });
    expect(view('unavailable').agy.banner).toMatchObject({ tone: 'warn', title: 'Không đọc được danh sách tài khoản' });
  });

  it('the Antigravity tab says when the one login read gave no number', () => {
    const m = fixtureModel('single');
    m.snapshot.agy = { ...m.snapshot.agy, accounts: [], reason: 'Antigravity sign-in required' };
    const agy = windowView(m, FIXTURE_NOW).agy;
    expect(agy.banner).toMatchObject({ title: 'Chỉ thấy tài khoản đang dùng' });
    expect(agy.tabNote).toEqual({ tone: 'warn', text: 'Không đọc được hạn mức Antigravity (Antigravity sign-in required).' });
    expect(view('single').agy.tabNote).toBeNull();
    expect(view('fresh').agy.tabNote).toBeNull();
  });

  it("an undone Antigravity switch takes the tab's banner", () => {
    const overwritten = { busy: null, listed: true, overwritten: { to: 1, from: 2, at: '2026-01-16T07:10:00.000Z', overwrites: 1 } };
    for (const name of ['single', 'unavailable']) {
      expect(view(name, { agySwitch: overwritten }).agy.banner).toMatchObject({ tone: 'warn', title: 'Chuyển Antigravity bị ghi đè' });
    }
  });

  // @private:begin
  it('the switcher card is one line, the rest folded', () => {
    const s = view('fresh').switcher;
    // How often it runs is the LaunchAgent's own line beside it; this one says when it last checked.
    expect(s.line).toMatch(/^Kiểm tra lúc \d\d:\d\d$/);
    expect(s.tone).toBe('ok');
    expect(s.details.map((d) => d.key)).toContain('LaunchAgent tự chuyển');
    expect(s.details.find((d) => d.key.startsWith('Quyết định gần nhất'))).toMatchObject({ mono: true });
  });
  // @private:end

  it('a Mac without Antigravity has no Antigravity card', () => {
    expect(view('no-antigravity').agy.present).toBe(false);
  });
});
