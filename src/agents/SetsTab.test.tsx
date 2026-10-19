// The Bộ cấu hình tab and its dialogs (M15) on the fake Rust side: nothing
// here reads an agent's file, runs `claude` or `npx`, or touches the
// Keychain. Every set, project and server name is made up.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { setsFixture } from '../fixtures/fakeSets';
import { codexFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

async function openTab(model: Model = fixtureModel('fresh'), options: FakeOptions = {}) {
  const fake = fakeApi(model, 'sets', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { level: 2, name: 'Bộ cấu hình' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const dialog = () => screen.getByRole('dialog');
const button = (name: string | RegExp) => screen.getByRole('button', { name });
const want = (entry: string, w: 'Bật' | 'Tắt' | 'Không đổi') => within(within(dialog()).getByRole('group', { name: entry })).getByRole('button', { name: w });

afterEach(cleanup);

describe('the Bộ cấu hình tab', () => {
  it('lists the sets, the one in use and how far it drifted, with the title row dragging the window', async () => {
    const fake = await openTab();
    const nav = screen.getByRole('navigation', { name: 'Mục' });
    expect(within(nav).getByRole('button', { name: 'Bộ cấu hình' }).getAttribute('aria-current')).toBe('page');
    expect(screen.getByRole('heading', { level: 2 }).parentElement!.getAttribute('data-tauri-drag-region')).toBe('deep');
    // Opening the tab reads the tables the editor lists and the drift needs.
    expect(fake.api.mcpRead).toHaveBeenCalled();
    expect(fake.api.skillsRead).toHaveBeenCalled();
    expect(screen.getByText('đã lệch 1 mục')).toBeTruthy();
    expect(screen.getByText(/sau đó browser đang tắt ở Antigravity/)).toBeTruthy();
    expect(screen.getByText('Bật 1 MCP, tắt 2 · bật 2 skill · Codex về tài khoản ChatGPT')).toBeTruthy();
    expect(screen.getByText('Bật 3 MCP, tắt 1 · bật 2 skill · Codex giữ nguyên')).toBeTruthy();
    expect(fake.api.setsPreview).not.toHaveBeenCalled();
    expect(fake.api.setsApply).not.toHaveBeenCalled();
  });

  it('says a set as applied when nothing drifted', async () => {
    await openTab(fixtureModel('fresh', { sets: setsFixture({ applied: { ...setsFixture().applied!, drift: [] } }) }));
    expect(screen.getByText('đúng như bộ')).toBeTruthy();
  });

  it('counts Codex on the other side as drift', async () => {
    const applied = { ...setsFixture().applied!, id: 'docs', name: 'Viết tài liệu', drift: [] };
    await openTab(fixtureModel('fresh', { sets: setsFixture({ applied }) }));
    expect(screen.getByText('đã lệch 1 mục')).toBeTruthy();
    expect(screen.getByText(/Codex không ở tài khoản ChatGPT/)).toBeTruthy();
  });

  it('offers a first set from the configuration as it is', async () => {
    const fake = await openTab(fixtureModel('fresh', { sets: setsFixture({ sets: [], applied: null }) }));
    await press(button('Tạo bộ từ cấu hình hiện tại'));
    const d = within(dialog());
    expect(d.getByRole('heading', { name: 'Tạo bộ' })).toBeTruthy();
    // On everywhere it is: Bật. Off everywhere: Tắt. Some of each: Không đổi.
    expect(want('files', 'Bật').getAttribute('aria-pressed')).toBe('true');
    expect(want('tracker', 'Tắt').getAttribute('aria-pressed')).toBe('true');
    expect(want('browser', 'Không đổi').getAttribute('aria-pressed')).toBe('true');
    expect(want('pdf-tools', 'Bật').getAttribute('aria-pressed')).toBe('true');
    expect(want('release-notes', 'Không đổi').getAttribute('aria-pressed')).toBe('true');
    // A name is needed.
    await press(d.getByRole('button', { name: 'Lưu bộ' }));
    expect(d.getByText(/Đặt tên cho bộ/)).toBeTruthy();
    expect(fake.api.setsSave).not.toHaveBeenCalled();
    await act(async () => {
      fireEvent.change(d.getByRole('textbox', { name: 'Tên bộ' }), { target: { value: 'Ứng dụng ghi chú' } });
    });
    await press(d.getByRole('button', { name: 'Lưu bộ' }));
    expect(fake.api.setsSave).toHaveBeenCalledTimes(1);
    const saved = vi.mocked(fake.api.setsSave).mock.calls[0][0];
    expect(saved.mcp).toMatchObject({ files: 'on', tracker: 'off' });
    expect(saved.mcp.browser).toBeUndefined();
    expect(saved.codex).toBeNull();
    expect(screen.getByText('Đã lưu bộ Ứng dụng ghi chú.')).toBeTruthy();
    // Saving writes no agent file.
    expect(fake.api.mcpWrite).not.toHaveBeenCalled();
    expect(fake.api.skillsWrite).not.toHaveBeenCalled();
    expect(fake.api.setsApply).not.toHaveBeenCalled();
  });

  it('edits a set: Bật, Tắt, Không đổi and the Codex side', async () => {
    const fake = await openTab();
    await press(button('Sửa bộ Web app'));
    expect(within(dialog()).getByRole('heading', { name: 'Sửa bộ Web app' })).toBeTruthy();
    expect(want('browser', 'Bật').getAttribute('aria-pressed')).toBe('true');
    await press(want('browser', 'Không đổi'));
    await press(want('files', 'Tắt'));
    await press(within(dialog()).getByRole('radio', { name: /relay/ }));
    await press(within(dialog()).getByRole('button', { name: 'Lưu bộ' }));
    const saved = vi.mocked(fake.api.setsSave).mock.calls[0][0];
    expect(saved.id).toBe('web');
    expect(saved.mcp.browser).toBeUndefined();
    expect(saved.mcp.files).toBe('off');
    expect(saved.codex).toBe('custom');
    expect(screen.getByText('Bật 2 MCP, tắt 2 · bật 2 skill · Codex sang relay')).toBeTruthy();
  });

  it('previews by agent, reminds that running sessions keep the old configuration, and writes only what it showed', async () => {
    const fake = await openTab();
    await press(button('Áp bộ Tối giản'));
    const d = within(dialog());
    await d.findByText('Claude Code');
    expect(fake.api.setsPreview).toHaveBeenCalledWith('min');
    expect(d.getByText('MCP: tắt browser, tắt design-kit, tắt files')).toBeTruthy();
    expect(d.getByText('5 phiên Claude Code, 3 phiên Codex và Codex App đang chạy giữ cấu hình cũ đến khi mở lại.')).toBeTruthy();
    // tracker is off already everywhere it is: nothing to skip or write.
    expect(dialog().textContent).not.toMatch(/tracker|Bỏ qua/);
    expect(dialog().textContent).not.toMatch(/Bước 2/);
    expect(fake.api.setsApply).not.toHaveBeenCalled();
    const p = await vi.mocked(fake.api.setsPreview).mock.results[0].value;
    await press(d.getByRole('button', { name: 'Áp bộ' }));
    expect(fake.api.setsApply).toHaveBeenCalledWith('min', { mcp: p.mcp.changes, mcpBases: p.mcp.bases, skills: p.skills.changes, skillBases: p.skills.bases });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText(/^Đã áp bộ Tối giản: \d+ thay đổi\.$/)).toBeTruthy();
    // No Codex side: the Codex switch is not opened.
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
    expect(screen.getByText('đúng như bộ')).toBeTruthy();

    // Hoàn tác bộ: every entry back.
    await press(screen.getAllByRole('button', { name: 'Hoàn tác bộ' })[0]);
    expect(fake.api.setsUndo).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Đã hoàn tác bộ Tối giản.')).toBeTruthy();
    expect(screen.getByText('Chưa áp bộ nào.')).toBeTruthy();
  });

  it('opens the Codex switch of version 1 as step 2, and never switches Codex itself', async () => {
    const fake = await openTab();
    await press(button('Áp bộ Viết tài liệu'));
    const d = within(dialog());
    await d.findByText('Bước 2: mở tấm chuyển Codex về tài khoản ChatGPT');
    await press(d.getByRole('button', { name: 'Áp bộ' }));
    expect(fake.api.setsApply).toHaveBeenCalledTimes(1);
    // The sheet is the Codex tab's own: switching waits for its button.
    expect(within(dialog()).getByRole('heading', { name: /Chuyển Codex về tài khoản ChatGPT/ })).toBeTruthy();
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
    expect(screen.getByText(/Bước 2: chuyển Codex về tài khoản ChatGPT ở tấm chuyển Codex\./)).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Thôi' }));
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
    // Later: the line's button opens the same sheet again.
    await press(button('Chuyển Codex'));
    await press(within(dialog()).getByRole('button', { name: 'Chuyển về ChatGPT' }));
    expect(fake.api.codexSwitch).toHaveBeenCalledWith('chatGpt', expect.anything());
  });

  it('goes straight to step 2 when only Codex differs', async () => {
    const only = { id: 'cx', name: 'Chỉ Codex', mcp: {}, skills: {}, codex: 'chatGpt' as const };
    const fake = await openTab(fixtureModel('fresh', { sets: setsFixture({ sets: [only], applied: null }) }));
    await press(button('Áp bộ Chỉ Codex'));
    const go = await within(dialog()).findByRole('button', { name: 'Tiếp: chuyển Codex' });
    await press(go);
    expect(fake.api.setsApply).toHaveBeenCalledWith('cx', { mcp: [], mcpBases: {}, skills: [], skillBases: {} });
    expect(within(dialog()).getByRole('heading', { name: /Chuyển Codex về tài khoản ChatGPT/ })).toBeTruthy();
  });

  it('says which entries it cannot change', async () => {
    await openTab();
    await press(button('Áp bộ Web app'));
    const d = within(dialog());
    await d.findByText('Antigravity');
    expect(d.getByText('Bỏ qua deploy-check cho Antigravity: nằm ở thư mục riêng của agent, gom về một chỗ để bật tắt.')).toBeTruthy();
    expect(d.getByText('MCP: bật browser')).toBeTruthy();
  });

  it('has nothing to write when the Mac is as the set says', async () => {
    const even = { id: 'even', name: 'Như hiện tại', mcp: { files: 'on' as const }, skills: {}, codex: 'custom' as const };
    await openTab(fixtureModel('fresh', { sets: setsFixture({ sets: [even], applied: null }), codex: codexFixture() }));
    await press(button('Áp bộ Như hiện tại'));
    const d = within(dialog());
    await d.findByText('Máy đang đúng như bộ này. Không có gì để ghi.');
    expect((d.getByRole('button', { name: 'Áp bộ' }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('writes nothing when a file changed since the preview', async () => {
    const fake = await openTab(fixtureModel('fresh'), { setsChanged: 'codex' });
    await press(button('Áp bộ Tối giản'));
    await within(dialog()).findByText('Claude Code');
    await press(within(dialog()).getByRole('button', { name: 'Áp bộ' }));
    expect(within(dialog()).getByText(/Chưa đổi gì: file của Codex vừa đổi sau lúc xem trước/)).toBeTruthy();
    expect(fake.api.mcpWrite).not.toHaveBeenCalled();
    expect(screen.getByText('đã lệch 1 mục')).toBeTruthy();
  });

  it('deletes a set without touching an agent file, with Hoàn tác', async () => {
    const fake = await openTab();
    await press(button('Xoá bộ Tối giản'));
    expect(fake.api.setsDelete).toHaveBeenCalledWith('min');
    expect(screen.getByText('Không đổi gì trong file của agent.')).toBeTruthy();
    expect(screen.queryByText('Tối giản')).toBeNull();
    await press(button('Hoàn tác'));
    expect(fake.api.setsUndelete).toHaveBeenCalled();
    expect(screen.getByText('Tối giản')).toBeTruthy();
    expect(fake.api.mcpWrite).not.toHaveBeenCalled();
    expect(fake.api.skillsWrite).not.toHaveBeenCalled();
  });

  it('says what to do when there is no set', async () => {
    await openTab(fixtureModel('fresh', { sets: setsFixture({ sets: [], applied: null }) }));
    expect(screen.getByText('Chưa có bộ nào')).toBeTruthy();
    expect(button('Tạo bộ từ cấu hình hiện tại')).toBeTruthy();
  });
});
