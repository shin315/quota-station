// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Switching Codex from the popover and the main window (M8), on the fake
// Rust side: nothing here reads or writes Codex's files, restarts anything
// or opens CC Switch. The custom provider is the made-up `relay`.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp, PopoverApp } from '../App';
import { MORE_KEY } from './Popover';
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
/** The other Codex account, behind "1 tài khoản khác": the ChatGPT account while Codex goes through relay, relay while it is on ChatGPT. It opens its own switch sheet. */
const codexRow = () => within(within(codexSection()).getByRole('list', { name: 'Codex' })).getByRole('button');
/** What Codex uses now: a status tile, whose hover holds the notes the popover does not write. */
const inUse = () => within(codexSection()).getByRole('group');
const facts = (sheet: HTMLElement) => within(sheet).getAllByRole('listitem').map((li) => li.textContent);

async function press(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

// Every section shows its other accounts, as the owner opens them.
beforeEach(() => localStorage.setItem(MORE_KEY, JSON.stringify({ c: true, g: true, x: true })));
afterEach(() => {
  cleanup();
  localStorage.removeItem(MORE_KEY);
});

describe('the Codex row', () => {
  it('shows the provider in use, the ChatGPT account under it, and opens the sheet instead of switching', async () => {
    const fake = await openPopover(withCodex());
    const section = screen.getByRole('region', { name: 'Codex' });
    // relay in use, as a status; the ChatGPT account under "1 tài khoản khác" opens the sheet.
    expect(within(section).queryByText('Codex App đang mở')).toBeNull();
    expect(inUse().textContent).toBe('relayĐang dùngCustom API · không báo hạn mức');
    expect(codexRow().textContent).toBe('ChatGPT5 giờ23%2h 9mTuần41%4d 5h');
    expect(codexRow().getAttribute('aria-label')).toBe(
      'ChatGPT, ChatGPT · plugin dùng đăng nhập này. 5 giờ đã dùng 23%, đặt lại sau 2 giờ 09 phút. Tuần đã dùng 41%, đặt lại sau 4 ngày 5 giờ 39 phút',
    );
    expect(inUse().getAttribute('aria-label')).toBe('relay, Custom API · relay.example.com · trả lời 180 ms, đang dùng. Không báo hạn mức. Codex App đang mở');
    expect(inUse().getAttribute('title')).toBe('https://relay.example.com/v1');
    await press(codexRow());
    expect(fake.api.codexSwitch).not.toHaveBeenCalled();
    expect(fake.api.codexUse).not.toHaveBeenCalled();
    // The ChatGPT sign-in is in the Codex folder already: only the provider line goes.
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang ChatGPT?' });
    expect(facts(sheet)).toEqual([
      'Đăng nhập của ChatGPT đã nằm trong ~/.codex: không dời file nào.',
      'Bỏ dòng model_provider trong config.toml (sao lưu trước); bảng relay vẫn giữ.',
      'Phiên đang chạy vẫn đi qua relay tới khi mở lại. Thoát hẳn Codex App (⌘Q) rồi mở lại; 3 phiên codex trong Terminal mở lại bằng codex resume.',
    ]);
    // A ChatGPT account's sheet ticks the restart: running sessions keep the old sign-in until then.
    const box = within(sheet).getByRole('checkbox', { name: /Khởi động lại tiến trình nền Codex sau khi chuyển/ });
    expect(box.getAttribute('aria-checked')).toBe('true');
  });

  it('switches on confirm with the file hash the sheet showed, and says to restart Codex', async () => {
    const fake = await openPopover(withCodex({ current: 'chatgpt' }));
    expect(codexRow().textContent).toBe('relayCustom API · relay.example.com · trả lời 180 ms');
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' });
    expect(facts(sheet)[1]).toBe(
      'Chỉ sửa config.toml của Codex (dòng chọn nhà cung cấp và requires_openai_auth trong bảng relay). Sao lưu bản cũ trước khi ghi. Không đụng auth.json và danh sách MCP.',
    );
    await press(within(sheet).getByRole('button', { name: 'Chuyển sang relay' }));
    expect(fake.api.codexUse).toHaveBeenCalledWith('custom', 'h1');
    expect(fake.api.codexRestart).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog', { name: /Chuyển Codex/ })).toBeNull();
    expect(screen.getByText('Đã chuyển Codex sang relay')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Khởi động lại tiến trình nền Codex…' })).toBeTruthy();
    expect(inUse().textContent).toMatch(/^relayĐang dùng/);
    expect(codexRow().textContent).toMatch(/^ChatGPT5 giờ/);
  });

  it('with the box ticked, asks once more before restarting the background process', async () => {
    const fake = await openPopover(withCodex());
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang ChatGPT?' });
    expect(within(sheet).getByRole('checkbox').getAttribute('aria-checked')).toBe('true');
    await press(within(sheet).getByRole('button', { name: 'Chuyển sang ChatGPT' }));
    expect(fake.api.codexUse).toHaveBeenCalledWith('chatgpt:0', 'h1');
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
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang ChatGPT?' });
    await press(within(sheet).getByRole('button', { name: 'Chuyển sang ChatGPT' }));
    await press(screen.getByRole('button', { name: 'Không khởi động lại' }));
    expect(fake.api.codexRestart).not.toHaveBeenCalled();
    // The account goes by its own name.
    expect(screen.getByText('Đã chuyển Codex sang ChatGPT')).toBeTruthy();
  });

  const ccSwitch = { ccSwitch: { installed: true, running: true, data: true, traces: [] }, procs: { sessions: 3, app: true, daemon: true, ccSwitch: true } };

  it('CC Switch also managing Codex: a note, the hover of what is in use, and "Vẫn chuyển" on the sheet', async () => {
    await openPopover(withCodex({ current: 'chatgpt', ...ccSwitch }));
    // No chip: the note says it, and the hover of what is in use.
    expect(codexSection().textContent).not.toContain('CC Switch cũng quản lý');
    expect(inUse().getAttribute('title')).toContain('CC Switch cũng quản lý');
    expect(screen.getByText('CC Switch cũng đang quản lý Codex. Lần chuyển sau của CC Switch sẽ ghi đè lựa chọn ở đây.')).toBeTruthy();
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' });
    expect(facts(sheet)[0]).toMatch(/^CC Switch vẫn đang quản lý Codex trên máy này/);
    expect(within(sheet).getByRole('button', { name: 'Vẫn chuyển' })).toBeTruthy();
    // @private:begin
    expect(within(sheet).getByRole('button', { name: 'Hướng dẫn ngừng dùng CC Switch' })).toBeTruthy();
    // @private:end
  });

  it('CC Switch also managing Codex: going to a ChatGPT account warns on the sheet too', async () => {
    const fake = await openPopover(withCodex(ccSwitch));
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang ChatGPT?' });
    expect(facts(sheet)[0]).toMatch(/^CC Switch vẫn đang quản lý Codex trên máy này/);
    expect(within(sheet).getByRole('button', { name: 'Vẫn chuyển' })).toBeTruthy();
    // @private:begin
    expect(within(sheet).getByRole('button', { name: 'Hướng dẫn ngừng dùng CC Switch' })).toBeTruthy();
    // @private:end
    expect(fake.api.codexUse).not.toHaveBeenCalled();
  });

  it('the provider in use not answering: its hover and a banner that offers the ChatGPT login', async () => {
    const down = { state: 'down' as const, ms: null, since: '2026-01-16T07:12:00Z', checkedAt: '2026-01-16T07:20:00Z' };
    const fake = await openPopover(withCodex({ reach: down }));
    expect(within(codexSection()).queryByText(/không trả lời/)).toBeNull();
    expect(inUse().getAttribute('title')).toContain('không trả lời');
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
    // A provider's own line stands where a ChatGPT account's windows would; its hover says it too.
    expect(codexRow().textContent).toBe('relayCustom API · relay.example.com · không trả lời từ 14:12');
    expect(codexRow().getAttribute('title')).toBe('https://relay.example.com/v1 · không trả lời');
    await press(codexRow());
    const sheet = screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' });
    expect(facts(sheet)[0]).toBe('relay đang không trả lời. Codex sẽ lỗi ngay sau khi chuyển.');
    expect(within(sheet).getByRole('button', { name: 'Vẫn chuyển' })).toBeTruthy();
  });

  it('a change made outside the app: a blue banner, and "Xem thay đổi" opens what changed', async () => {
    const fake = await openPopover(
      withCodex({
        current: 'chatgpt',
        external: { at: '2026-01-16T07:02:00Z', lastAt: '2026-01-16T06:40:00Z', lastProvider: 'custom', now: 'openai', mcpChanged: true, mcpCount: 4 },
      }),
    );
    expect(screen.getByText('Cấu hình Codex vừa đổi ngoài app')).toBeTruthy();
    expect(codexSection().textContent).not.toContain('đổi ngoài app');
    expect(inUse().getAttribute('title')).toContain('đổi ngoài app');
    await press(screen.getByRole('button', { name: 'Xem thay đổi' }));
    const sheet = screen.getByRole('dialog', { name: 'Cấu hình Codex đã đổi ngoài app' });
    expect(within(sheet).getByText('4 máy chủ MCP. Danh sách đã thay đổi từ 13:40.')).toBeTruthy();
    await press(within(sheet).getByRole('button', { name: 'Về lại relay…' }));
    expect(screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' })).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Thôi' }));
    await press(screen.getByRole('button', { name: 'Đã hiểu' }));
    expect(fake.api.codexKeep).toHaveBeenCalled();
    expect(screen.queryByText('Cấu hình Codex vừa đổi ngoài app')).toBeNull();
  });

  const noLogin = () => withCodex({ current: 'chatgpt', login: 'missing', wantsRequires: false });

  it('no ChatGPT login: a banner that offers the sign-in, and no account in use', async () => {
    await openPopover(noLogin());
    expect(screen.getByText('Codex chưa có đăng nhập ChatGPT')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Đăng nhập ChatGPT…' })).toBeTruthy();
    // No ChatGPT account to show as in use: only relay, under "1 tài khoản"; the banner says the rest.
    expect(within(codexSection()).queryByRole('group')).toBeNull();
    expect(codexRow().textContent).toMatch(/^relay/);
    expect(codexSection().textContent).not.toContain('chưa đăng nhập');
  });

  it('no ChatGPT login: "Đăng nhập ChatGPT…" opens the Codex tab, and the popover does not log in', async () => {
    const fake = await openPopover(noLogin());
    await press(screen.getByRole('button', { name: 'Đăng nhập ChatGPT…' }));
    expect(fake.api.openWindow).toHaveBeenCalledWith('codex');
    expect(fake.api.codexLogin).not.toHaveBeenCalled();
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
    expect(fake.api.codexUse).not.toHaveBeenCalled();
  });

  it('a file changed while writing: nothing changed, the backup folder is one press away', async () => {
    await openPopover(withCodex(), { codexOutcome: (target) => ({ kind: 'failed', target, error: { kind: 'changed' } }) });
    await press(codexRow());
    await press(screen.getByRole('button', { name: 'Chuyển sang ChatGPT' }));
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
  it('lists every Codex account in one table as Claude lists its own, Codex on this Mac as one line under it, the detail on its sheet', async () => {
    const fake = await openTab(withCodex());
    expect(fake.api.codexCheck).toHaveBeenCalled();
    // One list, as Claude's and Antigravity's are: the ChatGPT sign-in with its two windows, then the custom provider.
    const accounts = within(screen.getByRole('list', { name: 'Tài khoản Codex' })).getAllByRole('listitem');
    expect(accounts).toHaveLength(2);
    expect(accounts[0].getAttribute('aria-label')).toMatch(/^ChatGPT, ChatGPT · plugin dùng đăng nhập này\. 5 giờ đã dùng 23%.*Tuần đã dùng 41%/);
    expect(within(accounts[0]).getByText('23%')).toBeTruthy();
    // The custom provider in use: its line where the windows would be, and its key under the table.
    expect(accounts[1].getAttribute('aria-label')).toBe('relay, Custom API · relay.example.com · trả lời 180 ms, đang dùng. Không báo hạn mức');
    expect(within(accounts[1]).getByText('Đang dùng')).toBeTruthy();
    expect(within(accounts[1]).getByText('Custom API · relay.example.com · trả lời 180 ms')).toBeTruthy();
    expect(screen.getByText('Khoá API trong config.toml ·', { exact: false }).textContent).toBe('Khoá API trong config.toml · Đã có');
    expect(screen.getByRole('button', { name: 'Đổi khoá…' })).toBeTruthy();
    // Codex on this Mac is one line; the technical words wait on its Chi tiết… sheet.
    expect(screen.getByText('CLI 0.163.0 · 3 phiên Terminal · App đang mở · MCP 3/4')).toBeTruthy();
    expect(screen.queryByText('auth.json · [mcp_servers]')).toBeNull();
    await press(screen.getByRole('button', { name: 'Chi tiết…' }));
    const more = screen.getByRole('dialog', { name: 'Codex trên máy này' });
    for (const t of ['có · experimental_bearer_token', 'auth.json · [mcp_servers]', 'mã băm khớp', 'ChatGPT 128 · relay 54 (codex resume tách theo nhà cung cấp)', 'ChatGPT → relay · đã khởi động lại tiến trình nền', 'Đổi xong: ⌘Q Codex App rồi mở lại.'])
      expect(within(more).getByText(t)).toBeTruthy();
    await press(within(more).getByRole('button', { name: 'Đóng' }));
    // The plugins box keeps one short line and its state; the rest is in its own Chi tiết.
    expect(screen.getByText('Đang bật')).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Chi tiết: Dùng plugin với Custom API' }));
    expect(screen.getByText(/Bỏ chọn không xoá auth\.json/)).toBeTruthy();
    // @private:begin
    // The author's own guide to leave CC Switch is not part of this tab.
    expect(document.getElementById('cc-card')).toBeNull();
    expect(screen.queryByText(/Ngừng dùng CC Switch/)).toBeNull();
    // @private:end

    await press(within(accounts[0]).getByRole('button', { name: 'Chuyển sang ChatGPT' }));
    expect(screen.getByRole('dialog', { name: 'Chuyển Codex sang ChatGPT?' })).toBeTruthy();
    expect(fake.api.codexUse).not.toHaveBeenCalled();
  });

  it('sends the MCP list to the MCP tab, and restarts the background process only after its own sheet', async () => {
    const fake = await openTab(withCodex());
    expect(screen.queryByRole('button', { name: 'Xuất danh sách MCP…' })).toBeNull();
    await press(screen.getByRole('button', { name: 'Mở MCP' }));
    expect(await screen.findByRole('heading', { level: 2, name: 'MCP' })).toBeTruthy();
    expect(fake.api.mcpRead).toHaveBeenCalled();
    await press(screen.getByRole('button', { name: 'Codex' }));
    await screen.findByRole('heading', { level: 2, name: 'Codex' });

    await press(screen.getByRole('button', { name: 'Chi tiết…' }));
    await press(within(screen.getByRole('dialog', { name: 'Codex trên máy này' })).getByRole('button', { name: 'Khởi động lại tiến trình nền…' }));
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
    // The key's line sits under the table for the provider in use.
    const fake = await openTab(withCodex({ custom: { ...codexFixture().custom!, key: 'envKey', envKey: 'RELAY_API_KEY' }, takesKey: false }));
    await press(screen.getByRole('button', { name: 'Cách đặt khoá…' }));
    const sheet = screen.getByRole('dialog', { name: 'relay đọc khoá từ biến RELAY_API_KEY' });
    expect(within(sheet).getByText('export RELAY_API_KEY=<khoá của bạn>')).toBeTruthy();
    expect(within(sheet).queryByLabelText(/Khoá API/)).toBeNull();
    expect(fake.api.codexKeyPreview).not.toHaveBeenCalled();
  });
});

describe('"Dùng plugin với Custom API"', () => {
  // `hidden`: a sheet on top leaves the tab behind it out of the accessibility tree.
  const box = () => screen.getByRole('checkbox', { name: /Dùng plugin với Custom API/, hidden: true });

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
    await press(within(sheet).getByRole('checkbox', { name: /Dùng plugin với Custom API/ }));
    expect(fake.api.setCodexPlugins).toHaveBeenCalledWith(true);
    expect(facts(screen.getByRole('dialog', { name: 'Chuyển Codex sang relay?' }))[0]).toBe(
      'Giữ đăng nhập ChatGPT để Codex App tiếp tục dùng plugin và điều khiển từ xa.',
    );
  });
});
