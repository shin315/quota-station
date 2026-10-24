// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Switching Codex from the popover and the main window (M8), on the fake
// Rust side: nothing here reads or writes Codex's files, restarts anything
// or opens CC Switch. The custom provider is the made-up `relay`.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp, PopoverApp } from '../App';
import { fakeApi, type FakeOptions } from '../fixtures/fakeApi';
import { codexFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { CodexView, Model } from '../lib/model';

const withCodex = (over: Partial<CodexView> = {}): Model => fixtureModel('fresh', { codex: codexFixture(over) });

async function openPopover(model: Model, options: FakeOptions = {}) {
  const fake = fakeApi(model, 'overview', options);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-popover">
      <PopoverApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('dialog', { name: 'Quota Station' });
  return fake;
}

const codexSection = () => screen.getByRole('region', { name: 'Codex' });
/** The full-width button under the Codex header: where a switch goes. */
const codexRow = () => within(codexSection()).getAllByRole('button')[0];
const facts = (sheet: HTMLElement) => within(sheet).getAllByRole('listitem').map((li) => li.textContent);

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

afterEach(cleanup);

describe('the Codex row', () => {
  it('shows the provider in use and where a switch goes, and opens the sheet instead of switching', async () => {
    const fake = await openPopover(withCodex());
    const section = screen.getByRole('region', { name: 'Codex' });
    // One line at the Gọn hơn level; what it leaves out is in the spoken label and the Codex tab.
    expect(within(section).queryByText('Codex App đang mở')).toBeNull();
    expect(codexSection().textContent).toBe('CodexrelayChatGPT');
    expect(codexRow().textContent).toBe('ChatGPT');
    expect(codexRow().getAttribute('aria-label')).toBe(
      'Codex đang dùng relay. Bấm để chuyển sang tài khoản ChatGPT. Trả lời 180 ms · giữ đăng nhập ChatGPT. Codex App đang mở',
    );
    await press(codexRow());
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex về tài khoản ChatGPT?' });
    expect(facts(sheet)).toEqual([
      'Giữ đăng nhập ChatGPT để Codex App tiếp tục dùng plugin và điều khiển từ xa.',
      'Chỉ sửa config.toml của Codex (dòng chọn nhà cung cấp). Sao lưu bản cũ trước khi ghi. Không đụng auth.json và danh sách MCP.',
      'Codex phải khởi động lại: thoát hẳn Codex App (⌘Q) rồi mở lại; 3 phiên codex trong Terminal cần mở lại.',
      'Hoàn tất việc đang chạy trước khi chuyển. Yêu cầu tiếp theo của phiên cũ có thể dùng nhà cung cấp mới và gặp lỗi.',
      'Các phiên có thể tiếp tục được tách theo nhà cung cấp: codex resume chỉ hiện phiên làm trên tài khoản ChatGPT. Codex App vẫn thấy hết.',
    ]);
    const box = within(sheet).getByRole('checkbox', { name: /Khởi động lại tiến trình nền Codex sau khi chuyển/ });
    expect(box.getAttribute('aria-checked')).toBe('false');
  });

  it('switches on confirm with the file hash the sheet showed, and says to restart Codex', async () => {
    const fake = await openPopover(withCodex({ current: 'chatgpt' }));
    expect(codexRow().textContent).toBe('relay');
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' });
    expect(facts(sheet)[1]).toBe(
      'Chỉ sửa config.toml của Codex (dòng chọn nhà cung cấp và requires_openai_auth trong bảng relay). Sao lưu bản cũ trước khi ghi. Không đụng auth.json và danh sách MCP.',
    );
    await press(within(sheet).getByRole('button', { name: 'Chuyển sang relay' }));
    expect(fake.api.codexSwitch).toHaveBeenCalledWith('custom', 'h1');
    expect(fake.api.codexRestart).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog', { name: /Chuyển Codex/ })).toBeNull();
    expect(screen.getByText('Đã chuyển Codex sang relay')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Khởi động lại tiến trình nền Codex…' })).toBeTruthy();
    expect(codexRow().textContent).toBe('ChatGPT');
  });

  it('with the box ticked, asks once more before restarting the background process', async () => {
    const fake = await openPopover(withCodex());
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex về tài khoản ChatGPT?' });
    await press(within(sheet).getByRole('checkbox'));
    await press(within(sheet).getByRole('button', { name: 'Chuyển về ChatGPT' }));
    expect(fake.api.codexSwitch).toHaveBeenCalledWith('chatGpt', 'h1');
    const again = screen.getByRole('dialog', { name: 'Khởi động lại tiến trình nền Codex?' });
    expect(fake.api.codexRestart).not.toHaveBeenCalled();
    expect(within(again).getByText(/việc đang chạy sẽ bị ngắt/)).toBeTruthy();
    await press(within(again).getByRole('button', { name: 'Khởi động lại' }));
    expect(fake.api.codexRestart).toHaveBeenCalledWith(true);
    expect(screen.getByText('Đã chuyển Codex sang tài khoản ChatGPT và khởi động lại tiến trình nền')).toBeTruthy();
  });

  it('"Không khởi động lại" leaves the background process and says only that it switched', async () => {
    const fake = await openPopover(withCodex());
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex về tài khoản ChatGPT?' });
    await press(within(sheet).getByRole('checkbox'));
    await press(within(sheet).getByRole('button', { name: 'Chuyển về ChatGPT' }));
    await press(screen.getByRole('button', { name: 'Không khởi động lại' }));
    expect(fake.api.codexRestart).not.toHaveBeenCalled();
    expect(screen.getByText('Đã chuyển Codex sang tài khoản ChatGPT')).toBeTruthy();
  });

  it('CC Switch also managing Codex: a yellow chip, a note, and "Vẫn chuyển" with the guide on the sheet', async () => {
    await openPopover(
      withCodex({
        ccSwitch: { installed: true, running: true, data: true, traces: [] },
        procs: { sessions: 3, app: true, daemon: true, ccSwitch: true },
      }),
    );
    expect(codexSection().textContent).toContain('CC Switch cũng quản lý');
    expect(screen.getByText('CC Switch cũng đang quản lý Codex. Lần chuyển sau của CC Switch sẽ ghi đè lựa chọn ở đây.')).toBeTruthy();
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex về tài khoản ChatGPT?' });
    expect(facts(sheet)[0]).toMatch(/^CC Switch vẫn đang quản lý Codex trên máy này/);
    expect(within(sheet).getByRole('button', { name: 'Vẫn chuyển' })).toBeTruthy();
    expect(within(sheet).getByRole('button', { name: 'Hướng dẫn ngừng dùng CC Switch' })).toBeTruthy();
  });

  it('the provider in use not answering: a red chip and a banner that offers the ChatGPT login', async () => {
    const down = { state: 'down' as const, ms: null, since: '2026-01-16T07:12:00Z', checkedAt: '2026-01-16T07:20:00Z' };
    const fake = await openPopover(withCodex({ reach: down }));
    expect(codexSection().textContent).toContain('không trả lời');
    expect(screen.getByText('relay không trả lời từ 14:12')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Chuyển về ChatGPT' }));
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex về tài khoản ChatGPT?' });
    expect(facts(sheet)[0]).toBe('relay không trả lời từ 14:12 (3 lần liền, mỗi lần chờ 5 giây).');
    expect(within(sheet).getByRole('button', { name: 'Chuyển về ChatGPT' })).toBeTruthy();
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
  });

  it('going to a provider that does not answer warns that Codex will fail at once', async () => {
    const down = { state: 'down' as const, ms: null, since: '2026-01-16T07:12:00Z', checkedAt: null };
    await openPopover(withCodex({ current: 'chatgpt', reach: down }));
    expect(codexSection().textContent).toContain('relay không trả lời');
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' });
    expect(facts(sheet)[0]).toBe('relay đang không trả lời. Codex sẽ lỗi ngay sau khi chuyển.');
    expect(within(sheet).getByRole('button', { name: 'Vẫn chuyển' })).toBeTruthy();
  });

  it('a change made outside the app: a blue banner, and the row opens what changed', async () => {
    const fake = await openPopover(
      withCodex({
        current: 'chatgpt',
        external: { at: '2026-01-16T07:02:00Z', lastAt: '2026-01-16T06:40:00Z', lastProvider: 'custom', now: 'openai', mcpChanged: true, mcpCount: 4 },
      }),
    );
    expect(screen.getByText('Cấu hình Codex vừa đổi ngoài app')).toBeTruthy();
    expect(codexSection().textContent).toContain('đổi ngoài app');
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Cấu hình Codex đã đổi ngoài app' });
    expect(within(sheet).getByText('4 máy chủ MCP. Danh sách đã thay đổi từ 13:40.')).toBeTruthy();
    await press(within(sheet).getByRole('button', { name: 'Về lại relay…' }));
    expect(screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' })).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Thôi' }));
    await press(screen.getByRole('button', { name: 'Đã hiểu' }));
    expect(fake.api.codexKeep).toHaveBeenCalled();
    expect(screen.queryByText('Cấu hình Codex vừa đổi ngoài app')).toBeNull();
  });

  it('no ChatGPT login: a banner, and the app does not log in', async () => {
    await openPopover(withCodex({ current: 'chatgpt', login: 'missing', wantsRequires: false }));
    expect(screen.getByText('Codex chưa có đăng nhập ChatGPT')).toBeTruthy();
    expect(codexSection().textContent).toContain('chưa đăng nhập');
    await press(screen.getByRole('button', { name: 'Hướng dẫn' }));
    expect(screen.getByRole('dialog', { name: 'Đăng nhập ChatGPT cho Codex' })).toBeTruthy();
  });

  it('a provider without its own key: the row is grey, says why, and opens nothing', async () => {
    const fake = await openPopover(
      withCodex({ current: 'chatgpt', custom: { ...codexFixture().custom!, key: 'none' }, refusal: { kind: 'noKey' }, wantsRequires: null }),
    );
    expect(codexRow().getAttribute('aria-disabled')).toBe('true');
    expect(screen.getByText('relay chưa có khoá API riêng nên chưa chuyển được. Dán khoá để dùng relay.')).toBeTruthy();
    await press(codexRow());
    expect(screen.queryByRole('dialog', { name: /Chuyển Codex/ })).toBeNull();
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
  });

  it('a file changed while writing: nothing changed, the backup folder is one press away', async () => {
    await openPopover(withCodex(), { codexOutcome: (target) => ({ kind: 'failed', target, error: { kind: 'changed' } }) });
    await press(codexRow());
    await press(screen.getByRole('button', { name: 'Chuyển về ChatGPT' }));
    expect(screen.getByText('Không ghi được config.toml')).toBeTruthy();
    expect(screen.getByText('File bị sửa trong lúc ghi (mã băm khác). Chưa đổi gì. Bản cũ vẫn nằm trong thư mục sao lưu.')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Xem thư mục sao lưu' }));
    expect(screen.getByRole('dialog', { name: 'Thư mục sao lưu Codex' })).toBeTruthy();
  });

  it('a Mac without Codex has no Codex row', async () => {
    await openPopover(withCodex({ present: false }));
    expect(screen.queryByRole('region', { name: 'Codex' })).toBeNull();
  });
});

async function openTab(model: Model) {
    const fake = fakeApi(model, 'codex');
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-window">
        <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('heading', { level: 2, name: 'Codex' });
    return fake;
}

describe('the Codex tab', () => {
  it('reads like the Claude tab: the two sides in one table, the detail behind Chi tiết, no CC Switch guide', async () => {
    const fake = await openTab(withCodex());
    expect(fake.api.codexCheck).toHaveBeenCalled();
    const sides = within(screen.getByRole('list', { name: 'Nhà cung cấp Codex' })).getAllByRole('listitem');
    expect(sides.map((r) => r.getAttribute('aria-label'))).toEqual(['Tài khoản ChatGPT', 'relay']);
    expect(within(sides[1]).getByText('Đang dùng')).toBeTruthy();
    expect(within(sides[1]).getByText('relay.example.com · trả lời 180 ms')).toBeTruthy();
    expect(within(sides[1]).getByRole('button', { name: 'Đổi khoá…' })).toBeTruthy();
    expect(screen.getByText('CLI 0.163.0 · 3 phiên Terminal · Codex App đang mở')).toBeTruthy();
    expect(screen.getByText('4 MCP · 3 đang bật')).toBeTruthy();
    // The technical words wait behind one Chi tiết.
    const details = screen.getByText('Chi tiết');
    expect(details.tagName).toBe('SUMMARY');
    const more = details.parentElement!;
    for (const t of ['có · experimental_bearer_token', 'auth.json · [mcp_servers]', 'mã băm khớp', 'ChatGPT 128 · relay 54 (codex resume tách theo nhà cung cấp)', 'ChatGPT → relay · đã khởi động lại tiến trình nền'])
      expect(within(more).getByText(t)).toBeTruthy();
    // The author's own guide to leave CC Switch is not part of the app.
    expect(document.getElementById('cc-card')).toBeNull();
    expect(screen.queryByText(/Ngừng dùng CC Switch/)).toBeNull();

    await press(within(sides[0]).getByRole('button', { name: 'Chuyển sang tài khoản ChatGPT' }));
    expect(screen.getByRole('dialog', { name: 'Chuyển Codex về tài khoản ChatGPT?' })).toBeTruthy();
  });

  it('sends the MCP list to the MCP tab, and restarts the background process only after its own sheet', async () => {
    const fake = await openTab(withCodex());
    expect(screen.queryByRole('button', { name: 'Xuất danh sách MCP…' })).toBeNull();
    await press(screen.getByRole('button', { name: 'Mở tab MCP' }));
    expect(await screen.findByRole('heading', { level: 2, name: 'MCP' })).toBeTruthy();
    expect(fake.api.mcpRead).toHaveBeenCalled();
    await press(screen.getByRole('button', { name: 'Codex' }));
    await screen.findByRole('heading', { level: 2, name: 'Codex' });

    await press(screen.getByRole('button', { name: 'Khởi động lại tiến trình nền…' }));
    expect(fake.api.codexRestart).not.toHaveBeenCalled();
    const sheet = screen.getByRole('dialog', { name: 'Khởi động lại tiến trình nền Codex?' });
    await press(within(sheet).getByRole('button', { name: 'Khởi động lại' }));
    expect(fake.api.codexRestart).toHaveBeenCalledWith(false);
  });

  it('a Mac without Codex says so', async () => {
    await openTab(withCodex({ present: false }));
    expect(screen.getByText('Máy này chưa có Codex')).toBeTruthy();
  });
});

describe('"Dán khoá" for the custom provider', () => {
  const KEY = 'sk-made-up-window-key-0000wxyz';
  const placeholder = () =>
    withCodex({
      current: 'chatgpt',
      custom: { ...codexFixture().custom!, name: 'custom', baseUrl: 'http://127.0.0.1:15721/v1', key: 'placeholder', requiresOpenaiAuth: null, proxy: true },
      wantsRequires: null,
      refusal: { kind: 'placeholder' },
      ccSwitch: { installed: true, running: false, data: true, traces: ['model_catalog_json = "cc-switch-model-catalog.json"'] },
    });

  it('is offered under the row\'s placeholder note, previews the key masked, and writes it only on confirm', async () => {
    const fake = await openPopover(placeholder());
    expect(codexRow().getAttribute('aria-disabled')).toBe('true');
    await press(screen.getByRole('button', { name: 'Dán khoá…' }));
    const sheet = screen.getByRole('dialog', { name: 'Dán khoá API cho custom' });
    const field = within(sheet).getByLabelText('Khoá API của custom') as HTMLInputElement;
    expect(field.type).toBe('password');
    expect(within(sheet).getByText(/model_catalog_json của CC Switch: giữ file danh mục đó, đừng xoá/)).toBeTruthy();
    await act(async () => {
      fireEvent.change(field, { target: { value: KEY } });
      fireEvent.change(within(sheet).getByLabelText('Địa chỉ thật của custom'), { target: { value: 'https://relay.example.com/v1' } });
    });
    await press(within(sheet).getByRole('button', { name: 'Xem trước' }));
    expect(fake.api.codexKeyPreview).toHaveBeenCalledWith(KEY, 'https://relay.example.com/v1');
    const preview = screen.getByRole('dialog', { name: 'Ghi khoá cho custom?' });
    expect(preview.textContent).not.toContain(KEY);
    expect(within(preview).getByText('experimental_bearer_token = "sk-…wxyz"')).toBeTruthy();
    expect(within(preview).getByText('base_url = "http://127.0.0.1:15721/v1"')).toBeTruthy();
    expect(facts(preview)).toContain('Có nút Hoàn tác: trả lại khoá giữ chỗ PROXY_MANAGED.');
    expect(fake.api.codexSetKey).not.toHaveBeenCalled();
    await press(within(preview).getByRole('button', { name: 'Ghi khoá' }));
    expect(fake.api.codexSetKey).toHaveBeenCalledWith(KEY, 'https://relay.example.com/v1', 'h1');
    expect(screen.getByText('Đã ghi khoá cho custom')).toBeTruthy();
    expect(document.body.textContent).not.toContain(KEY);
    // The undo says what it can put back, on the button.
    await press(screen.getAllByRole('button', { name: 'Hoàn tác: trả lại PROXY_MANAGED và địa chỉ cũ' })[0]);
    expect(fake.api.codexUndoKey).toHaveBeenCalledWith("h1k");
    expect(screen.getByText('Đã hoàn tác khoá của custom')).toBeTruthy();
  });

  it('says why a paste is not a key, and replacing a key says it cannot be undone', async () => {
    const fake = await openTab(withCodex());
    await press(screen.getByRole('button', { name: 'Đổi khoá…' }));
    const sheet = screen.getByRole('dialog', { name: 'Đổi khoá API của relay' });
    await act(async () => {
      fireEvent.change(within(sheet).getByLabelText('Khoá API của relay'), { target: { value: 'PROXY_MANAGED' } });
    });
    await press(within(sheet).getByRole('button', { name: 'Xem trước' }));
    expect(within(sheet).getByText('PROXY_MANAGED là khoá giữ chỗ của CC Switch, không phải khoá thật.')).toBeTruthy();
    await act(async () => {
      fireEvent.change(within(sheet).getByLabelText('Khoá API của relay'), { target: { value: KEY } });
    });
    await press(within(sheet).getByRole('button', { name: 'Xem trước' }));
    const preview = screen.getByRole('dialog', { name: 'Ghi khoá cho relay?' });
    expect(facts(preview)).toContain('Khoá cũ bị thay. App không giữ khoá cũ nên không hoàn tác được.');
    expect(fake.api.codexSetKey).not.toHaveBeenCalled();
  });

  it('a provider that reads its key from a variable gets the steps, and nothing is written', async () => {
    const fake = await openTab(withCodex({ current: 'chatgpt', custom: { ...codexFixture().custom!, key: 'envKey', envKey: 'RELAY_API_KEY' }, takesKey: false }));
    await press(screen.getByRole('button', { name: 'Cách đặt khoá…' }));
    const sheet = screen.getByRole('dialog', { name: 'relay đọc khoá từ biến RELAY_API_KEY' });
    expect(within(sheet).getByText('export RELAY_API_KEY=<khoá của bạn>')).toBeTruthy();
    expect(within(sheet).queryByLabelText(/Khoá API/)).toBeNull();
    expect(fake.api.codexKeyPreview).not.toHaveBeenCalled();
  });
});

describe('"Dùng plugin khi chạy Custom API"', () => {
  // `hidden`: a sheet on top leaves the tab behind it out of the accessibility tree.
  const box = () => screen.getByRole('checkbox', { name: /Dùng plugin khi chạy Custom API \(giữ đăng nhập ChatGPT\)/, hidden: true });

  it('is a ticked checkbox by default, kept when unticked, and offers to apply it on the provider in use', async () => {
    const fake = await openTab(withCodex());
    expect(box().getAttribute('role')).toBe('checkbox');
    expect(box().getAttribute('aria-checked')).toBe('true');
    await press(box());
    expect(fake.api.setCodexPlugins).toHaveBeenCalledWith(false);
    expect(box().getAttribute('aria-checked')).toBe('false');
    const sheet = screen.getByRole('dialog', { name: 'Áp dụng ngay cho relay?' });
    expect(facts(sheet)[0]).toBe('Không dùng đăng nhập ChatGPT khi chạy relay: plugin cần đăng nhập (apps, connector) sẽ tắt.');
    expect(facts(sheet)[1]).toBe('Chỉ đổi requires_openai_auth = false trong bảng relay. Sao lưu trước khi ghi. Không đụng auth.json.');
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
    await press(within(sheet).getByRole('button', { name: 'Áp dụng ngay' }));
    expect(fake.api.codexSwitch).toHaveBeenCalledWith('custom', 'h1');
    expect(screen.getByText('Đã tắt plugin khi chạy relay')).toBeTruthy();
    expect(screen.getByText(/Thoát hẳn Codex App \(⌘Q\) rồi mở lại/)).toBeTruthy();
  });

  it('"Để lần chuyển sau" keeps the choice and writes nothing; on the ChatGPT side there is nothing to ask', async () => {
    const fake = await openTab(withCodex());
    await press(box());
    await press(screen.getByRole('button', { name: 'Để lần chuyển sau' }));
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
    cleanup();
    const other = await openTab(withCodex({ current: 'chatgpt' }));
    await press(box());
    expect(other.api.setCodexPlugins).toHaveBeenCalledWith(false);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('a file that does not follow the box on the provider in use offers Áp dụng ngay', async () => {
    const fake = await openTab(withCodex({ custom: { ...codexFixture().custom!, requiresOpenaiAuth: false } }));
    await press(screen.getByRole('button', { name: 'Áp dụng ngay…' }));
    await press(within(screen.getByRole('dialog', { name: 'Áp dụng ngay cho relay?' })).getByRole('button', { name: 'Áp dụng ngay' }));
    expect(fake.api.codexSwitch).toHaveBeenCalledWith('custom', 'h1');
    expect(screen.getByText('Đã bật plugin khi chạy relay')).toBeTruthy();
  });

  it('the confirmation sheet\'s login line follows the box, and the box can be ticked there', async () => {
    const fake = await openPopover(withCodex({ current: 'chatgpt', plugins: false, wantsRequires: false }));
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' });
    expect(facts(sheet)[0]).toBe(
      'Không dùng đăng nhập ChatGPT khi chạy relay (ô "Dùng plugin" đang bỏ tick): plugin cần đăng nhập (apps, connector) sẽ tắt. auth.json vẫn giữ nguyên.',
    );
    await press(within(sheet).getByRole('checkbox', { name: /Dùng plugin khi chạy Custom API/ }));
    expect(fake.api.setCodexPlugins).toHaveBeenCalledWith(true);
    expect(facts(screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' }))[0]).toBe(
      'Giữ đăng nhập ChatGPT để Codex App tiếp tục dùng plugin và điều khiển từ xa.',
    );
  });
});
