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

const codexRow = () => within(screen.getByRole('region', { name: 'Codex' })).getByRole('button');
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
    expect(within(section).getByText('Codex App đang mở')).toBeTruthy();
    expect(codexRow().textContent).toBe('CodexrelayTrả lời 180 ms · giữ đăng nhập ChatGPT→ ChatGPT');
    expect(codexRow().getAttribute('aria-label')).toBe('Codex đang dùng relay. Bấm để chuyển sang tài khoản ChatGPT');
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
    const box = within(sheet).getByRole('checkbox', { name: /Khởi động lại tiến trình nền Codex sau khi chuyển/ }) as HTMLInputElement;
    expect(box.checked).toBe(false);
  });

  it('switches on confirm with the file hash the sheet showed, and says to restart Codex', async () => {
    const fake = await openPopover(withCodex({ current: 'chatgpt' }));
    expect(codexRow().textContent).toContain('→ relay');
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
    expect(codexRow().textContent).toContain('→ ChatGPT');
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
    expect(codexRow().textContent).toContain('CC Switch cũng quản lý');
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
    expect(codexRow().textContent).toContain('không trả lời');
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
    expect(codexRow().textContent).toContain('relay không trả lời');
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
    expect(codexRow().textContent).toContain('đổi ngoài app');
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
    expect(codexRow().textContent).toContain('chưa đăng nhập');
    await press(screen.getByRole('button', { name: 'Hướng dẫn' }));
    expect(screen.getByRole('dialog', { name: 'Đăng nhập ChatGPT cho Codex' })).toBeTruthy();
  });

  it('a provider without its own key: the row is grey, says why, and opens nothing', async () => {
    const fake = await openPopover(
      withCodex({ current: 'chatgpt', custom: { ...codexFixture().custom!, key: 'none' }, refusal: { kind: 'noKey' }, wantsRequires: null }),
    );
    expect(codexRow().getAttribute('aria-disabled')).toBe('true');
    expect(screen.getByText(/không có khoá API riêng\. App không chuyển sang relay/)).toBeTruthy();
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

describe('the Codex tab', () => {
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

  it('shows the provider, Codex on this Mac, the sessions, the MCP list, the CC Switch guide and the file it writes', async () => {
    const fake = await openTab(withCodex());
    expect(fake.api.codexCheck).toHaveBeenCalled();
    const choice = screen.getByRole('group', { name: 'Nhà cung cấp Codex' });
    expect(within(choice).getByRole('button', { name: 'relay' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByText('https://relay.example.com/v1')).toBeTruthy();
    expect(screen.getByText('có · experimental_bearer_token')).toBeTruthy();
    expect(screen.getByText('0.163.0 · tiến trình nền đang chạy')).toBeTruthy();
    expect(screen.getByText('128')).toBeTruthy();
    expect(screen.getByText('54')).toBeTruthy();
    expect(screen.getByText('docs-search')).toBeTruthy();
    expect(screen.getByText('auth.json · [mcp_servers]')).toBeTruthy();
    expect(screen.getByText('mã băm khớp')).toBeTruthy();
    const guide = document.getElementById('cc-card')!;
    expect(within(guide).getAllByRole('listitem')).toHaveLength(8);
    expect(within(guide).getByText('Kiểm skill của Codex trong ~/.codex/skills')).toBeTruthy();
    expect(within(guide).getByText(/Máy này: 1\/3 ngày từ lần chuyển đầu/)).toBeTruthy();

    await press(within(choice).getByRole('button', { name: 'Tài khoản ChatGPT' }));
    expect(screen.getByRole('dialog', { name: 'Chuyển Codex về tài khoản ChatGPT?' })).toBeTruthy();
  });

  it('exports the MCP list after a preview, and restarts the background process only after its own sheet', async () => {
    const fake = await openTab(withCodex());
    await press(screen.getByRole('button', { name: 'Xuất danh sách MCP…' }));
    await press(within(screen.getByRole('dialog', { name: 'Xuất danh sách MCP của Codex' })).getByRole('button', { name: 'Lưu file' }));
    expect(fake.api.codexExportMcp).toHaveBeenCalled();
    expect(screen.getByText(/4 máy chủ MCP → ~\/Downloads\/codex-mcp-16-01\.txt/)).toBeTruthy();

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
