// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The File hướng dẫn tab (M11) on the fake Rust side: nothing here reads or
// writes an agent's file. Every file is made up. The page is laid out as the
// approved redraw (instr-A): one table, its files by agent under group heads,
// "Thêm file…" at the table's head and the two notes at its foot.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

async function openTab(model: Model = fixtureModel('fresh')) {
  const fake = fakeApi(model, 'instr');
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { level: 2, name: 'File hướng dẫn' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

const dialog = () => screen.getByRole('dialog');
const files = () => within(screen.getByRole('group', { name: 'File hướng dẫn' })).getAllByRole('listitem');

afterEach(cleanup);

describe('the File hướng dẫn tab', () => {
  it('lists the files, keeps one with a key out of the pack, and shows the key masked', async () => {
    const fake = await openTab();
    expect(fake.api.instrRead).toHaveBeenCalled();
    expect(files().map((r) => r.getAttribute('aria-label'))).toEqual(['~/.claude/CLAUDE.md', '~/.claude/rules/deploy.md', '~/.claude/rules/style.md', '~/.codex/AGENTS.md', '~/.gemini/GEMINI.md']);
    expect(screen.getByText('deploy.md chưa vào gói sao lưu')).toBeTruthy();
    const deploy = screen.getByRole('switch', { name: 'Đưa ~/.claude/rules/deploy.md vào gói' }) as HTMLButtonElement;
    expect(deploy.disabled).toBe(true);
    expect(deploy.getAttribute('aria-checked')).toBe('false');
    await press(screen.getByRole('button', { name: 'Xem dòng 12' }));
    expect(within(dialog()).getByText('DEPLOY_TOKEN=••••••')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Dòng 12 không phải khoá' }));
    expect(fake.api.instrNotKey).toHaveBeenCalledWith('~/.claude/rules/deploy.md', 12);
  });

  it('groups the files by agent, with Thêm file… on the table and the notes at its foot', async () => {
    await openTab();
    const table = screen.getByRole('heading', { level: 4, name: /^Trong máy này/ }).closest('[data-slot=card]') as HTMLElement;
    // "Thêm file…" sits on the table's head, not in the title row; "Đọc lại" stays up there.
    expect(within(table).getByRole('button', { name: 'Thêm file…' })).toBeTruthy();
    const title = screen.getByRole('heading', { level: 2 }).parentElement!;
    expect(within(title).queryByRole('button', { name: 'Thêm file…' })).toBeNull();
    expect(within(title).getByRole('button', { name: 'Làm mới' })).toBeTruthy();
    // A group per agent; the agent is no longer a column.
    const groups = within(table).getAllByRole('region');
    expect(groups.map((g) => [g.getAttribute('aria-label'), within(g).getAllByRole('listitem').length])).toEqual([
      ['Claude Code', 3],
      ['Codex', 1],
      ['Antigravity', 1],
    ]);
    expect(within(groups[0]).getByRole('heading', { level: 5 }).textContent).toBe('Claude Code · 3');
    expect(within(table).queryByText('Agent')).toBeNull();
    // The two notes are the table's foot.
    expect(within(table).getByText(/^App không sửa các file này/)).toBeTruthy();
    expect(within(table).getByText(/mặc định giữ bản trên máy/)).toBeTruthy();
  });

  it('compares a changed file with the pack, and "Trong gói" is the only thing it changes', async () => {
    const fake = await openTab();
    await press(screen.getByRole('button', { name: 'So sánh ~/.claude/rules/style.md' }));
    expect(fake.api.instrCompare).toHaveBeenCalledWith('~/.claude/rules/style.md');
    expect(within(dialog()).getByText('- No comments when the code is clear.')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Đóng' }));
    await press(screen.getByRole('switch', { name: 'Đưa ~/.gemini/GEMINI.md vào gói' }));
    expect(fake.api.instrInclude).toHaveBeenCalledWith('~/.gemini/GEMINI.md', true);
  });

  it('"Thêm file…" takes a file in the home folder only, and lists it as added by hand', async () => {
    const fake = await openTab();
    await press(screen.getByRole('button', { name: 'Thêm file…' }));
    const input = within(dialog()).getByRole('textbox');
    fireEvent.change(input, { target: { value: '/etc/hosts.md' } });
    await press(within(dialog()).getByRole('button', { name: 'Thêm' }));
    expect(within(dialog()).getByText('File phải nằm trong thư mục nhà, ngoài thư mục của app.')).toBeTruthy();
    fireEvent.change(input, { target: { value: '~/notes/team.md' } });
    await press(within(dialog()).getByRole('button', { name: 'Thêm' }));
    expect(fake.api.instrAdd).toHaveBeenLastCalledWith('~/notes/team.md');
    const hand = screen.getByRole('region', { name: 'Thêm tay' });
    expect(within(hand).getByRole('listitem', { name: '~/notes/team.md' })).toBeTruthy();
  });
});
