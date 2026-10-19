// The Sao lưu tab and the Khôi phục sheet (M10) on the fake Rust side:
// nothing here reads or writes an agent's file, iCloud Drive or the
// Keychain. Every pack, Mac and key is made up.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { backupFixture, packsFixture } from '../fixtures/fakeBackup';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

async function openTab(model: Model = fixtureModel('fresh'), options: FakeOptions = {}) {
  const fake = fakeApi(model, 'backup', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('heading', { level: 2, name: 'Sao lưu' });
  return fake;
}

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const dialog = () => screen.getByRole('dialog');
const button = (name: string | RegExp) => screen.getByRole('button', { name });

/** Opens the sheet and goes to its preview. */
async function toPreview() {
  await press(button('Khôi phục…'));
  expect(within(dialog()).getByRole('heading', { name: 'Khôi phục' })).toBeTruthy();
  await press(within(dialog()).getByRole('button', { name: 'Tiếp' }));
  await screen.findByRole('heading', { name: /^Xem trước: gói/ });
}

afterEach(cleanup);

describe('the Sao lưu tab', () => {
  it('is in the sidebar, lists the packs and the keys, and says nothing technical', async () => {
    const fake = await openTab();
    expect(fake.api.backupRead).toHaveBeenCalled();
    expect(fake.api.mcpRead).toHaveBeenCalled();
    const nav = screen.getByRole('navigation', { name: 'Mục' });
    expect(within(nav).getByRole('button', { name: 'Sao lưu' }).getAttribute('aria-current')).toBe('page');
    expect(screen.getByRole('heading', { level: 4, name: 'Gói gần nhất' })).toBeTruthy();
    expect(screen.getByText('Không chứa khoá. Khi khôi phục, app hỏi lại 2 khoá.')).toBeTruthy();
    expect(screen.getByRole('heading', { level: 4, name: /Các gói/ })).toBeTruthy();
    expect(screen.getByText('Trong Bitwarden: Docs Search API')).toBeTruthy();
    const text = document.body.textContent ?? '';
    expect(text).not.toMatch(/Keychain|Mobile Documents|~\/|\.json|security/);
  });

  it('backs up now, and turns the packs made on their own on or off', async () => {
    const fake = await openTab();
    await press(button('Sao lưu bây giờ'));
    expect(fake.api.backupNow).toHaveBeenCalled();
    expect(await screen.findByText(/^Đã sao lưu lúc/)).toBeTruthy();
    await press(screen.getByRole('switch', { name: 'Sau mỗi thay đổi trong app' }));
    expect(fake.api.backupSetAuto).toHaveBeenCalledWith(false, true);
    expect(screen.getByRole('switch', { name: 'Sau mỗi thay đổi trong app' }).getAttribute('aria-checked')).toBe('false');
  });

  it('says a pack iCloud Drive has not downloaded and does not let it be picked', async () => {
    await openTab(fixtureModel('fresh', { backup: backupFixture({ packs: packsFixture() }) }));
    expect(screen.getByText('Gói hôm nay 04:55 chưa tải về máy')).toBeTruthy();
    expect(button('Mở trong Finder')).toBeTruthy();
    const row = button(/^Khôi phục gói Hôm nay 04:55/) as HTMLButtonElement;
    expect(row.disabled).toBe(true);
    await press(button('Khôi phục…'));
    const cloud = within(dialog()).getByRole('radio', { name: /chưa tải về máy/ }) as HTMLButtonElement;
    expect(cloud.disabled).toBe(true);
  });

  it('shows an empty state with the first backup', async () => {
    await openTab(fixtureModel('fresh', { backup: backupFixture({ packs: [], keys: [] }) }));
    expect(screen.getByText('Chưa có gói sao lưu nào')).toBeTruthy();
    expect(button('Sao lưu lần đầu')).toBeTruthy();
  });

  it('opens a pack in plain words', async () => {
    const fake = await openTab();
    await press(button('Xem trong gói'));
    expect(fake.api.packSummary).toHaveBeenCalled();
    expect(await within(dialog()).findByText('6 MCP, và agent nào đang bật')).toBeTruthy();
    expect(within(dialog()).getByText('Nhà cung cấp Codex')).toBeTruthy();
  });
});

describe('the Khôi phục sheet', () => {
  it('previews adds ticked and a different server unticked, with a side by side', async () => {
    await toPreviewOn();
    const add = screen.getByRole('checkbox', { name: /^docs-search/ }) as HTMLInputElement;
    expect(add.checked).toBe(true);
    expect(screen.getByText('Thêm cho cả ba agent · cần dán khoá')).toBeTruthy();
    expect(screen.getByText('Máy này chưa cài notes-mcp: thêm nhưng để tắt')).toBeTruthy();
    const replace = screen.getByRole('checkbox', { name: /^design-kit/ }) as HTMLInputElement;
    expect(replace.checked).toBe(false);
    await press(within(dialog()).getByRole('button', { name: 'So sánh' }));
    expect(within(dialog()).getByText('design-kit-mcp --theme dark')).toBeTruthy();
    expect(screen.getByText('Giống trên máy: browser.')).toBeTruthy();
    expect(within(dialog()).getByText('Không đổi đăng nhập, tài khoản hay nhà cung cấp Codex.')).toBeTruthy();
  });

  it('asks for the keys, sends a pasted one once and never keeps it', async () => {
    const fake = await toPreviewOn();
    await press(within(dialog()).getByRole('button', { name: 'Tiếp: dán khoá' }));
    const field = within(dialog()).getByLabelText('Khoá của docs-search') as HTMLInputElement;
    expect(field.type).toBe('password');
    expect(field.value).toBe('');
    await act(async () => {
      fireEvent.change(field, { target: { value: 'fake-pasted-value' } });
    });
    await press(within(dialog()).getByRole('button', { name: 'Khôi phục' }));
    expect(fake.api.restoreRun).toHaveBeenCalledWith(
      'agents-20260116-141000-studio-mini',
      [
        { name: 'docs-search', kind: 'add' },
        { name: 'files', kind: 'add' },
        { name: 'tracker', kind: 'add' },
        { name: 'notes', kind: 'add' },
      ],
      [{ mcp: 'docs-search', slot: 'header:Authorization', value: 'fake-pasted-value' }],
      { claude: 'b-claude', codex: 'b-codex', agy: 'b-agy' },
    );
    expect(await within(dialog()).findByRole('heading', { name: 'Đã khôi phục' })).toBeTruthy();
    expect(within(dialog()).getByText('MCP notes · thêm nhưng tắt')).toBeTruthy();
    expect(within(dialog()).getByText('Cài notes-mcp rồi bật notes ở tab MCP.')).toBeTruthy();
    expect(document.body.innerHTML).not.toContain('fake-pasted-value');
  });

  it('adds a server whose key is left empty turned off, and says so', async () => {
    await toPreviewOn();
    await press(within(dialog()).getByRole('button', { name: 'Tiếp: dán khoá' }));
    await press(within(dialog()).getByRole('button', { name: 'Khôi phục' }));
    expect(await within(dialog()).findByText('MCP docs-search · thêm nhưng tắt')).toBeTruthy();
    expect(within(dialog()).getByText('Dán khoá cho docs-search ở tab MCP, rồi bật docs-search.')).toBeTruthy();
  });

  it('stops when a file changed after the preview, writing nothing', async () => {
    await toPreviewOn(fixtureModel('fresh'), { restoreChanged: true });
    await press(within(dialog()).getByRole('button', { name: 'Tiếp: dán khoá' }));
    await press(within(dialog()).getByRole('button', { name: 'Khôi phục' }));
    expect(await within(dialog()).findByText(/Chưa khôi phục: file của Codex vừa đổi/)).toBeTruthy();
    expect(within(dialog()).getByRole('button', { name: 'Đọc lại' })).toBeTruthy();
  });

  it('undoes the whole restore, asking first when a file changed since', async () => {
    const fake = await toPreviewOn(fixtureModel('fresh'), { restoreUndoChanged: true });
    await press(within(dialog()).getByRole('button', { name: 'Tiếp: dán khoá' }));
    await press(within(dialog()).getByRole('button', { name: 'Khôi phục' }));
    await within(dialog()).findByRole('heading', { name: 'Đã khôi phục' });
    await press(within(dialog()).getByRole('button', { name: 'Hoàn tác lần khôi phục…' }));
    expect(within(dialog()).getByText('File bạn sửa sau lần khôi phục thì hỏi trước, không ghi đè.')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Hoàn tác' }));
    expect(fake.api.restoreUndo).toHaveBeenCalledWith(false);
    expect(await within(dialog()).findByText('File của Codex đã sửa sau lần khôi phục.')).toBeTruthy();
    await press(within(dialog()).getByRole('button', { name: 'Vẫn hoàn tác' }));
    expect(fake.api.restoreUndo).toHaveBeenLastCalledWith(true);
    expect(await screen.findByText('Đã hoàn tác lần khôi phục.')).toBeTruthy();
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});

async function toPreviewOn(model: Model = fixtureModel('fresh'), options: FakeOptions = {}) {
  const fake = await openTab(model, options);
  await toPreview();
  return fake;
}
