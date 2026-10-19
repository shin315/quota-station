// The Skills, Plugin and File hướng dẫn tabs (M11) on the fake Rust side:
// nothing here reads or writes a skill folder, runs `npx`, `claude` or
// `codex`, or reaches GitHub. Every skill, plugin and file is made up.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { pluginsFixture, skillsFixture } from '../fixtures/fakeSkills';
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
  await screen.findByRole('navigation', { name: 'Mục' });
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
    const nav = screen.getByRole('navigation', { name: 'Mục' });
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
    expect(screen.getByText('sql-helper, ui-polish. App không tự cập nhật: bạn xem trước rồi mới cập nhật.')).toBeTruthy();
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
    const seg = screen.getByRole('group', { name: 'Kiểm bản mới' });
    expect(within(seg).getByRole('button', { name: 'Mỗi ngày' }).getAttribute('aria-pressed')).toBe('true');
    const notify = screen.getByRole('switch', { name: 'Báo skill có bản mới bằng thông báo macOS' });
    expect(notify.getAttribute('aria-checked')).toBe('false');
    await press(within(seg).getByRole('button', { name: 'Tắt' }));
    expect(fake.api.skillsSetChecks).toHaveBeenCalledWith(false, false);
    await press(notify);
    expect(fake.api.skillsSetChecks).toHaveBeenLastCalledWith(false, true);
  });
});

describe('the Plugin tab', () => {
  it('lists each agent\'s plugins and marketplaces, read only', async () => {
    const fake = await openTab('plugins');
    expect(fake.api.pluginsRead).toHaveBeenCalled();
    const claude = screen.getByRole('list', { name: 'Plugin của Claude Code' });
    expect(within(claude).getAllByRole('listitem').map((r) => r.getAttribute('aria-label'))).toEqual([
      'issue-hub@acme-plugins, bật',
      'formatter@tools-market, bật',
      'docs-pack@acme-plugins, bật',
      'review-kit@tools-market, tắt',
    ]);
    expect(within(claude).getByText('1 MCP, 1 skill, 3 lệnh')).toBeTruthy();
    expect(screen.getByText(/Chỉ xem. App không cài, gỡ, bật hay tắt plugin/)).toBeTruthy();
    // No box, no switch: nothing here changes a plugin.
    expect(screen.queryByRole('checkbox')).toBeNull();
    expect(screen.queryByRole('switch')).toBeNull();
    await press(screen.getByRole('button', { name: 'Lệnh cài lại' }));
    expect(within(dialog()).getByText('claude plugin install issue-hub@acme-plugins')).toBeTruthy();
    expect(within(dialog()).getByText('codex plugin add sheets@codex-market')).toBeTruthy();
  });

  it('without claude: says the pack keeps the old list', async () => {
    const p = pluginsFixture();
    const sides = p.sides!.map((s) => (s.agent === 'claude' ? { ...s, state: 'noCli' as const, plugins: [], marketplaces: [] } : s));
    await openTab('plugins', fixtureModel('fresh', { plugins: { ...p, sides } }));
    expect(screen.getByText('Không đọc được plugin của Claude Code')).toBeTruthy();
    expect(screen.getByText(/Gói sao lưu giữ danh sách cũ của Claude Code/)).toBeTruthy();
  });
});

describe('the File hướng dẫn tab', () => {
  it('lists the files, keeps one with a key out of the pack, and shows the key masked', async () => {
    const fake = await openTab('instr');
    expect(fake.api.instrRead).toHaveBeenCalled();
    const rows = within(screen.getByRole('list', { name: 'File hướng dẫn' })).getAllByRole('listitem');
    expect(rows.map((r) => r.getAttribute('aria-label'))).toEqual(['~/.claude/CLAUDE.md', '~/.claude/rules/deploy.md', '~/.claude/rules/style.md', '~/.codex/AGENTS.md', '~/.gemini/GEMINI.md']);
    expect(screen.getByText('deploy.md chưa vào gói sao lưu')).toBeTruthy();
    const deploy = screen.getByRole('switch', { name: 'Đưa ~/.claude/rules/deploy.md vào gói' }) as HTMLButtonElement;
    expect(deploy.disabled).toBe(true);
    expect(deploy.getAttribute('aria-checked')).toBe('false');
    await press(screen.getByRole('button', { name: 'Xem dòng 12' }));
    expect(within(dialog()).getByText('DEPLOY_TOKEN=••••••')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Dòng 12 không phải khoá' }));
    expect(fake.api.instrNotKey).toHaveBeenCalledWith('~/.claude/rules/deploy.md', 12);
  });

  it('compares a changed file with the pack, and "Trong gói" is the only thing it changes', async () => {
    const fake = await openTab('instr');
    await press(screen.getByRole('button', { name: 'So sánh ~/.claude/rules/style.md' }));
    expect(fake.api.instrCompare).toHaveBeenCalledWith('~/.claude/rules/style.md');
    expect(within(dialog()).getByText('- No comments when the code is clear.')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Đóng' }));
    await press(screen.getByRole('switch', { name: 'Đưa ~/.gemini/GEMINI.md vào gói' }));
    expect(fake.api.instrInclude).toHaveBeenCalledWith('~/.gemini/GEMINI.md', true);
  });

  it('"Thêm file…" takes a file in the home folder only', async () => {
    const fake = await openTab('instr');
    await press(screen.getByRole('button', { name: 'Thêm file…' }));
    const input = within(dialog()).getByRole('textbox');
    fireEvent.change(input, { target: { value: '/etc/hosts.md' } });
    await press(within(dialog()).getByRole('button', { name: 'Thêm' }));
    expect(within(dialog()).getByText('File phải nằm trong thư mục nhà, ngoài thư mục của app.')).toBeTruthy();
    fireEvent.change(input, { target: { value: '~/notes/team.md' } });
    await press(within(dialog()).getByRole('button', { name: 'Thêm' }));
    expect(fake.api.instrAdd).toHaveBeenLastCalledWith('~/notes/team.md');
    expect(screen.getByRole('listitem', { name: '~/notes/team.md' })).toBeTruthy();
  });
});
