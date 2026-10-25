// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Skills tab (M11) on the fake Rust side (the Plugin and File hướng dẫn
// tabs have their own tests, PluginTab.test.tsx and InstrTab.test.tsx):
// nothing here reads or writes a skill folder, runs `npx`, `claude` or
// `codex`, or reaches GitHub. Every skill, plugin and file is made up.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { skillsFixture, skillTableFixture } from '../fixtures/fakeSkills';
import { alertsFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model, Tab } from '../lib/model';

async function openTab(tab: Tab, model: Model = fixtureModel('fresh'), options: FakeOptions = {}) {
  const fake = fakeApi(model, tab, options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Các tab' });
  return fake;
}

/** Opens a column's menu as a pointer does: Radix opens a menu on pointerdown, not on click. */
async function openMenu(el: HTMLElement) {
  await act(async () => {
    fireEvent.pointerDown(el, { button: 0, ctrlKey: false });
  });
}

/** Radix moves focus a tick after React commits: the menu's arrows, the icon again when the menu closes. */
async function tick() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
  await tick();
}

const box = (name: string) => screen.getByRole('checkbox', { name: new RegExp('^' + name) });
const dialog = () => screen.getByRole('dialog');

afterEach(cleanup);

describe('the Skills tab', () => {
  it('is under Agent, reads when it opens, and draws a box per agent', async () => {
    const fake = await openTab('skills');
    expect(fake.api.skillsRead).toHaveBeenCalled();
    const nav = screen.getByRole('navigation', { name: 'Các tab' });
    expect(within(nav).getAllByRole('button').map((b) => b.textContent)).toEqual(expect.arrayContaining(['MCP', 'Skills', 'Plugin', 'File hướng dẫn']));
    expect(screen.getByRole('heading', { level: 2, name: 'Skills' })).toBeTruthy();
    expect(screen.getByText('GitHub · uikit-lab/ui-skills')).toBeTruthy();
    expect(screen.getByText('cài tay · thư mục của Claude Code')).toBeTruthy();
    expect(box('ui-polish cho Antigravity').getAttribute('aria-checked')).toBe('false');
    // In an agent's own folder: gather it first.
    expect(box('deploy-check cho Claude Code').getAttribute('aria-disabled')).toBe('true');
    expect(screen.getByText(/2 skill cài tay đang nằm rải/)).toBeTruthy();
    expect(screen.getByText('5 skill có sẵn trong agent và plugin. App để yên, không sửa.')).toBeTruthy();
  });

  it('says which skills have a newer version, and never updates on its own', async () => {
    const fake = await openTab('skills');
    expect(screen.getByText('2 skill có bản mới')).toBeTruthy();
    expect(screen.getByText(/^Chỉ cập nhật sau khi bạn xem trước\./)).toBeTruthy();
    expect(fake.api.skillsUpdate).not.toHaveBeenCalled();
    await press(screen.getByRole('button', { name: 'Xem sql-helper' }));
    const d = dialog();
    expect(within(d).getByRole('heading', { name: 'Cập nhật sql-helper?' })).toBeTruthy();
    // Edited on this Mac: said before anything is replaced.
    expect(within(d).getByText('Bạn đã sửa skill này trên máy')).toBeTruthy();
    await press(within(d).getByRole('button', { name: 'Xem thay đổi trên GitHub' }));
    expect(fake.api.skillsOpenLink).toHaveBeenCalledWith('https://github.com/dataworks/skills/commits/HEAD/sql-helper');
    await press(within(d).getByRole('button', { name: 'Cập nhật' }));
    expect(fake.api.skillsUpdate).toHaveBeenCalledWith('sql-helper');
    expect(screen.getByText('Đã cập nhật sql-helper.')).toBeTruthy();
  });

  it('"Bỏ qua bản này" keeps the version on this Mac', async () => {
    const fake = await openTab('skills');
    await press(screen.getByRole('button', { name: 'Cập nhật ui-polish' }));
    await press(within(dialog()).getByRole('button', { name: 'Bỏ qua bản này' }));
    expect(fake.api.skillsSkip).toHaveBeenCalledWith('ui-polish');
    expect(fake.api.skillsUpdate).not.toHaveBeenCalled();
    expect(screen.getByText('1 skill có bản mới')).toBeTruthy();
  });

  it('"Cập nhật tất cả" previews every newer version, a skill edited on this Mac not ticked', async () => {
    const fake = await openTab('skills');
    await press(screen.getByRole('button', { name: 'Cập nhật tất cả (2)' }));
    const d = dialog();
    expect(within(d).getByRole('heading', { name: 'Cập nhật tất cả skill có bản mới?' })).toBeTruthy();
    expect(within(d).getByText('Bạn đã sửa skill này trên máy')).toBeTruthy();
    const group = within(d).getByRole('group', { name: 'Skill sẽ cập nhật' });
    expect(within(group).getByRole('checkbox', { name: /sql-helper/ }).getAttribute('aria-checked')).toBe('false');
    expect(within(group).getByRole('checkbox', { name: /ui-polish/ }).getAttribute('aria-checked')).toBe('true');
    expect(within(d).getByText('12/12 → 16/01 · Sửa hướng dẫn, thêm 1 file · uikit-lab/ui-skills')).toBeTruthy();
    expect(within(d).getByText('Cập nhật 1 skill, lần lượt bằng npx skills update <tên> -g -y. Không chạy script nào của skill.')).toBeTruthy();
    expect(fake.api.skillsUpdateMany).not.toHaveBeenCalled();
    await press(within(group).getByRole('checkbox', { name: /sql-helper/ }));
    await press(within(d).getByRole('button', { name: 'Cập nhật 2 skill' }));
    expect(fake.api.skillsUpdateMany).toHaveBeenCalledWith(['sql-helper', 'ui-polish']);
    expect(fake.api.skillsUpdate).not.toHaveBeenCalled();
    expect(screen.getByText('Đã cập nhật 2 skill.')).toBeTruthy();
    expect(screen.getByText('Agent nào đang bật vẫn bật. Bản cũ đã lưu ở ~/Library/Application Support/com.shin315.quota-station/backups/skills.')).toBeTruthy();
    expect(screen.queryByRole('button', { name: /Cập nhật tất cả/ })).toBeNull();
  });

  it('"Cập nhật cả bộ" for each repository with a newer version, with progress and each skill\'s result', async () => {
    const newer = { fromAt: '2025-12-02T09:00:00Z', seenAt: 1768537000, plain: 'Sửa hướng dẫn', link: 'https://github.com/acme/agent-skills/commits/HEAD' };
    const rows = skillTableFixture().rows.map((r) => (r.name === 'pdf-tools' || r.name === 'release-notes' ? { ...r, update: newer } : r));
    const fake = await openTab('skills', fixtureModel('fresh', { skills: skillsFixture({ table: skillTableFixture({ rows }) }) }), { skillsDelayMs: 200, skillsUpdateFail: ['release-notes'] });
    await waitFor(() => expect(screen.queryByText(/đang đọc…/)).toBeNull());
    expect(screen.getByRole('region', { name: '4 skill có bản mới' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Cập nhật cả bộ dataworks/skills' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Cập nhật cả bộ uikit-lab/ui-skills' })).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Cập nhật cả bộ acme/agent-skills' }));
    const d = dialog();
    expect(within(d).getByRole('heading', { name: 'Cập nhật cả bộ acme/agent-skills?' })).toBeTruthy();
    expect(within(within(d).getByRole('group', { name: 'Skill sẽ cập nhật' })).getAllByRole('checkbox').map((b) => b.getAttribute('aria-label') ?? b.closest('label')?.textContent)).toEqual([
      expect.stringContaining('pdf-tools'),
      expect.stringContaining('release-notes'),
    ]);
    expect(within(d).queryByText('Bạn đã sửa skill này trên máy')).toBeNull();
    await press(within(d).getByRole('button', { name: 'Cập nhật 2 skill' }));
    expect(fake.api.skillsUpdateMany).toHaveBeenCalledWith(['pdf-tools', 'release-notes']);
    // While it runs: the one at hand, and nothing else to press.
    expect(within(dialog()).getByRole('heading', { name: /^Đang cập nhật \d\/2: / })).toBeTruthy();
    expect((within(dialog()).getByRole('button', { name: 'Đang cập nhật…' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: /^Đang cập nhật \d\/2…$/, hidden: true }) as HTMLButtonElement).disabled).toBe(true);
    expect(await screen.findByText('Đã cập nhật 1 skill, 1 skill chưa cập nhật.')).toBeTruthy();
    expect(screen.getByText(/^Chưa cập nhật release-notes \(Skill release-notes not found\)\. Agent nào đang bật vẫn bật\. Bản cũ đã lưu ở /)).toBeTruthy();
    // The set still has one newer version; pdf-tools has none.
    expect(screen.getByRole('button', { name: 'Cập nhật cả bộ acme/agent-skills' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Cập nhật tất cả (3)' })).toBeTruthy();
  });

  it('writes the ticks through one preview, then offers Hoàn tác', async () => {
    const fake = await openTab('skills');
    await press(box('ui-polish cho Antigravity'));
    await press(box('pdf-tools cho Codex'));
    const bar = screen.getByText('2 thay đổi chưa ghi').closest('.pendbar') as HTMLElement;
    expect(within(bar).getByText('pdf-tools: tắt cho Codex · ui-polish: bật cho Antigravity')).toBeTruthy();
    await press(within(bar).getByRole('button', { name: 'Xem trước và ghi…' }));
    const d = dialog();
    expect(within(d).getByRole('heading', { name: 'Ghi 2 thay đổi?' })).toBeTruthy();
    expect(within(d).getByText('Không chép, không sửa nội dung skill.')).toBeTruthy();
    expect(within(d).getByText('~/.gemini/config/skills/ui-polish → ~/.agents/skills/ui-polish')).toBeTruthy();
    expect(within(d).getByText('name = "pdf-tools"')).toBeTruthy();
    expect(fake.api.skillsWrite).not.toHaveBeenCalled();
    await press(within(d).getByRole('button', { name: 'Ghi 2 thay đổi' }));
    expect(fake.api.skillsWrite).toHaveBeenCalledWith(
      [
        { name: 'pdf-tools', agent: 'codex', on: false },
        { name: 'ui-polish', agent: 'agy', on: true },
      ],
      { codex: 'base-codex', agy: 'base-agy' },
    );
    expect(screen.queryByText(/thay đổi chưa ghi/)).toBeNull();
    expect(screen.getByText('Đã ghi 2 thay đổi.')).toBeTruthy();
    expect(box('ui-polish cho Antigravity').getAttribute('aria-checked')).toBe('true');
    await press(screen.getByRole('button', { name: 'Hoàn tác' }));
    expect(fake.api.skillsUndo).toHaveBeenCalled();
    expect(box('ui-polish cho Antigravity').getAttribute('aria-checked')).toBe('false');
  });

  it('writes nothing when a folder changed since the preview', async () => {
    await openTab('skills', fixtureModel('fresh'), { skillsChanged: true });
    await press(box('ui-polish cho Antigravity'));
    await press(screen.getByRole('button', { name: 'Xem trước và ghi…' }));
    await press(within(dialog()).getByRole('button', { name: 'Ghi 1 thay đổi' }));
    expect(screen.getByText(/Chưa đổi gì: liên kết của Antigravity vừa bị chương trình khác đổi/)).toBeTruthy();
    expect(screen.getByText('1 thay đổi chưa ghi')).toBeTruthy();
  });

  it('filters the table, and a column menu only touches the rows shown', async () => {
    await openTab('skills');
    await press(screen.getByRole('button', { name: 'Có bản mới 2' }));
    expect(screen.queryByRole('group', { name: 'pdf-tools' })).toBeNull();
    expect(screen.getByRole('group', { name: 'ui-polish' })).toBeTruthy();
    await openMenu(screen.getByRole('button', { name: 'Antigravity: tuỳ chọn cột' }));
    expect(within(screen.getByRole('menu')).getByText('0/2 skill đang tick')).toBeTruthy();
    await press(screen.getByRole('menuitem', { name: 'Bật tất cả' }));
    expect(screen.getByText('2 thay đổi chưa ghi')).toBeTruthy();
  });

  it('installs from GitHub in three steps, reading a SKILL.md first, and runs nothing of it', async () => {
    const fake = await openTab('skills');
    await press(screen.getAllByRole('button', { name: 'Cài từ GitHub' })[0]);
    expect(within(dialog()).getByText('Skill có thể chạy lệnh trên máy khi agent dùng nó. Chỉ cài từ nguồn bạn tin.')).toBeTruthy();
    fireEvent.change(screen.getByRole('textbox', { name: 'Repo hoặc link GitHub' }), { target: { value: 'https://github.com/acme/agent-skills' } });
    await press(within(dialog()).getByRole('button', { name: 'Tìm skill' }));
    expect(fake.api.skillsListRepo).toHaveBeenCalledWith('https://github.com/acme/agent-skills');
    expect(within(dialog()).getByRole('heading', { name: 'Chọn skill trong acme/agent-skills' })).toBeTruthy();
    expect((within(dialog()).getByRole('checkbox', { name: /pdf-tools/ }) as HTMLInputElement).disabled).toBe(true);
    await press(within(dialog()).getByRole('button', { name: 'Đọc SKILL.md của changelog-writer' }));
    expect(fake.api.skillsReadMd).toHaveBeenCalledWith('acme/agent-skills', 'changelog-writer');
    expect(within(dialog()).getByText('# Changelog writer')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Quay lại danh sách' }));
    fireEvent.click(within(dialog()).getByRole('checkbox', { name: 'Codex' }));
    await press(within(dialog()).getByRole('button', { name: 'Tiếp: xem trước' }));
    expect(within(dialog()).getByText('npx skills add acme/agent-skills --skill changelog-writer -g -y -a claude-code')).toBeTruthy();
    expect(within(dialog()).getByText('Không chạy script nào của skill lúc cài.')).toBeTruthy();
    expect(fake.api.skillsInstall).not.toHaveBeenCalled();
    await press(within(dialog()).getByRole('button', { name: 'Cài' }));
    expect(fake.api.skillsInstall).toHaveBeenCalledWith('acme/agent-skills', ['changelog-writer'], { claude: true, codex: false, agy: true });
    expect(screen.getByText('Đã cài changelog-writer.')).toBeTruthy();
    expect(screen.getByRole('group', { name: 'changelog-writer' })).toBeTruthy();
  });

  it('says a repository was not found, in the first step', async () => {
    await openTab('skills');
    await press(screen.getAllByRole('button', { name: 'Cài từ GitHub' })[0]);
    fireEvent.change(screen.getByRole('textbox', { name: 'Repo hoặc link GitHub' }), { target: { value: 'acme/agent-skils' } });
    await press(within(dialog()).getByRole('button', { name: 'Tìm skill' }));
    expect(within(dialog()).getByText('npx skills báo lỗi. Repository not found')).toBeTruthy();
  });

  it('without npx: says so, and installing waits', async () => {
    await openTab('skills', fixtureModel('fresh', { skills: skillsFixture({ npx: false }) }));
    expect(screen.getByText('Không tìm thấy npx')).toBeTruthy();
    expect((screen.getAllByRole('button', { name: 'Cài từ GitHub' })[0] as HTMLButtonElement).disabled).toBe(true);
    expect(screen.queryByRole('button', { name: 'Cập nhật ui-polish' })).toBeNull();
  });

  it('gathers the skills installed by hand, with Hoàn tác', async () => {
    const fake = await openTab('skills');
    await press(screen.getByRole('button', { name: 'Gom về…' }));
    expect(within(dialog()).getByText('Lưu lại bản cũ trước khi chuyển. Hoàn tác được.')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Gom 2 skill' }));
    expect(fake.api.skillsImport).toHaveBeenCalledWith(['deploy-check', 'trace-reader']);
    expect(screen.getByText('Đã gom 2 skill về thư mục chung.')).toBeTruthy();
    expect(box('deploy-check cho Claude Code').getAttribute('aria-disabled')).toBeNull();
  });

  it('"Gỡ" turns off everywhere through the preview by default; "Gỡ hẳn" is the tool\'s remove', async () => {
    const fake = await openTab('skills');
    await press(screen.getByRole('button', { name: 'Gỡ release-notes' }));
    await press(within(dialog()).getByRole('button', { name: 'Tiếp: xem trước' }));
    expect(within(dialog()).getByRole('heading', { name: 'Tắt release-notes ở mọi agent?' })).toBeTruthy();
    expect(fake.api.skillsRemove).not.toHaveBeenCalled();
    await press(within(dialog()).getByRole('button', { name: 'Quay lại' }));
    await press(screen.getByRole('button', { name: 'Gỡ release-notes' }));
    await press(within(dialog()).getByRole('radio', { name: /Gỡ hẳn/ }));
    await press(within(dialog()).getByRole('button', { name: 'Gỡ hẳn' }));
    expect(fake.api.skillsRemove).toHaveBeenCalledWith('release-notes');
  });

  it('a skill installed by hand is never removed for good', async () => {
    const fake = await openTab('skills');
    await press(screen.getByRole('button', { name: 'Gỡ find-notes' }));
    await press(within(dialog()).getByRole('radio', { name: /Gỡ hẳn/ }));
    expect(within(dialog()).getByText('Skill cài tay: app không xoá thư mục')).toBeTruthy();
    expect(within(dialog()).queryByRole('button', { name: 'Gỡ hẳn' })).toBeNull();
    expect(fake.api.skillsRemove).not.toHaveBeenCalled();
  });

  it('a link to nothing is dropped, never a folder', async () => {
    const fake = await openTab('skills');
    await press(screen.getByRole('button', { name: 'Sửa liên kết' }));
    expect(within(dialog()).getByText('Chỉ bỏ liên kết, không xoá thư mục nào.')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Bỏ liên kết' }));
    expect(fake.api.skillsDropLink).toHaveBeenCalledWith('agy', 'old-linter');
  });

  it('keeps the newer versions in one card above the table: Cập nhật tất cả, a line per repository, the details behind Chi tiết', async () => {
    await openTab('skills');
    const card = screen.getByRole('region', { name: '2 skill có bản mới' });
    // The tab's head keeps only "Kiểm bản mới"; installing sits at the table's head.
    const head = screen.getByRole('heading', { level: 2, name: 'Skills' }).parentElement!;
    expect(within(head).getAllByRole('button').map((b) => b.textContent)).toEqual(['Kiểm bản mới']);
    expect(within(card).getByRole('button', { name: 'Cập nhật tất cả (2)' })).toBeTruthy();
    expect(Array.from(card.querySelectorAll('.grid > div')).map((r) => r.textContent)).toEqual([
      'dataworks/skills · 1 skill có bản mới: sql-helperXem sql-helperCập nhật cả bộ',
      'uikit-lab/ui-skills · 1 skill có bản mới: ui-polishXem ui-polishCập nhật cả bộ',
    ]);
    await press(within(card).getByRole('button', { name: 'Chi tiết: cập nhật skill' }));
    expect(screen.getByText(/Skill bạn đã sửa trên máy để chưa tick/)).toBeTruthy();
    const table = screen.getByRole('heading', { name: /^Skills của bạn/ });
    expect(within(table).getByRole('button', { name: 'Cài từ GitHub' })).toBeTruthy();
    // Each column names its agent next to its logo; a box an agent cannot change says why.
    expect(screen.getByRole('button', { name: 'Claude Code: tuỳ chọn cột' }).textContent).toBe('Claude Code');
    expect(box('deploy-check cho Claude Code').getAttribute('aria-label')).toMatch(/, khoá: deploy-check: /);
  });

  it('has no update card while nothing is newer, nor without npx', async () => {
    await openTab('skills', fixtureModel('fresh', { skills: skillsFixture({ table: skillTableFixture({ rows: skillTableFixture().rows.map((r) => ({ ...r, update: null })) }) }) }));
    await screen.findByRole('heading', { name: /^Skills của bạn/ });
    expect(screen.queryByText(/skill có bản mới/)).toBeNull();
    expect(screen.queryByRole('button', { name: /Cập nhật tất cả/ })).toBeNull();
    cleanup();
    await openTab('skills', fixtureModel('fresh', { skills: skillsFixture({ npx: false }) }));
    await screen.findByRole('heading', { name: /^Skills của bạn/ });
    expect(screen.queryByRole('region', { name: /skill có bản mới/ })).toBeNull();
    expect((within(screen.getByRole('heading', { name: /^Skills của bạn/ })).getByRole('button', { name: 'Cài từ GitHub' }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('the alert for newer versions opens this tab', async () => {
    const alert = {
      id: 'skills:abc@1',
      alert: { key: 'skills:abc', kind: 'skill' as const, tone: 'info' as const, title: '2 skill có bản mới', body: 'sql-helper, ui-polish. App không tự cập nhật.', actions: [{ id: 'skills:open', label: 'Xem' }, { id: 'dismiss', label: 'Để sau' }], at: 1768540000 },
      notified: false,
      answer: null,
    };
    await openTab('alerts', fixtureModel('fresh', { alerts: { ...alertsFixture(), recent: [alert], unanswered: 1 } }));
    const card = screen.getByRole('group', { name: 'Cảnh báo: 2 skill có bản mới' });
    await press(within(card).getByRole('button', { name: 'Xem' }));
    expect(await screen.findByRole('heading', { level: 2, name: 'Skills' })).toBeTruthy();
  });
});

describe('Cài đặt › Skills', () => {
  it('has "Kiểm bản mới" daily or off, and the macOS card off by default', async () => {
    const fake = await openTab('settings');
    const seg = screen.getByRole('group', { name: 'Kiểm bản mới của skill' });
    expect(within(seg).getByRole('button', { name: 'Mỗi ngày' }).getAttribute('aria-pressed')).toBe('true');
    const notify = screen.getByRole('switch', { name: 'Báo skill có bản mới bằng thông báo macOS' });
    expect(notify.getAttribute('aria-checked')).toBe('false');
    await press(within(seg).getByRole('button', { name: 'Tắt' }));
    expect(fake.api.skillsSetChecks).toHaveBeenCalledWith(false, false);
    await press(notify);
    expect(fake.api.skillsSetChecks).toHaveBeenLastCalledWith(false, true);
  });
});
