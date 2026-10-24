// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi, hideNames, type FakeOptions } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel, peerFixture, rotationFixture, rulesFixture } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model, Tab } from '../lib/model';

async function open(model: Model = fixtureModel('fresh'), options: FakeOptions = {}, tab: Tab = 'rules') {
  const fake = fakeApi(model, tab, { clock: () => FIXTURE_NOW, ...options });
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Mục' });
  return fake;
}

const heading = () => screen.getByRole('heading', { level: 2 }).textContent;
const team = () => within(screen.getByRole('group', { name: /^Team trong vòng/ }));
const personal = () => within(screen.getByRole('group', { name: /^Cá nhân/ }));
const macs = () => within(screen.getByLabelText('Các Mac'));
const saveButton = () => screen.queryByRole('button', { name: /^Lưu cho/ }) as HTMLButtonElement | null;
const press = async (el: HTMLElement) => {
  await act(async () => {
    fireEvent.click(el);
  });
};

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('rules editor', () => {
  it('shows the rules on this Mac and one status per Mac', async () => {
    await open();
    expect(heading()).toBe('Luật xoay vòng');
    expect(screen.queryByText('~/.config/cswap-prefer-team/rotation')).toBeNull();
    const chips = team().getAllByRole('button');
    expect(chips.map((b) => [b.textContent, b.getAttribute('aria-pressed')])).toEqual([
      ['1lan.acme1, thứ tự 1', 'true'],
      ['2lan.acme2, thứ tự 2', 'true'],
      ['3lan.acme3, thứ tự 3', 'true'],
    ]);
    // The whole email on hover.
    expect(chips[0].getAttribute('title')).toBe('lan.acme1@example.com');
    expect(personal().getByRole('button', { name: /lanpham\.dev/ }).getAttribute('aria-pressed')).toBe('true');
    expect((screen.getByLabelText('Giờ bắt đầu dùng tài khoản cá nhân') as HTMLSelectElement).value).toBe('8');
    expect((screen.getByLabelText('Giờ kết thúc dùng tài khoản cá nhân') as HTMLSelectElement).value).toBe('24');
    expect((screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng') as HTMLInputElement).value).toBe('90');
    expect((screen.getByLabelText('Ngưỡng Fable tuần, phần trăm đã dùng') as HTMLInputElement).value).toBe('95');
    expect((screen.getByLabelText('Giờ rời vòng của lan.acme1') as HTMLInputElement).value).toBe('2026-01-16T17:45');
    expect(screen.getByRole('button', { name: 'Đặt giờ rời vòng cho lan.acme2' })).toBeTruthy();

    expect(macs().getByText('Mac mini (máy này)').textContent).toBe('Mac mini (máy này) · lưu 14:02');
    expect(macs().getByText('Mac B').textContent).toBe('Mac B · đã nhận 14:02');
    expect(saveButton()?.textContent).toBe('Lưu cho cả ba Mac');
    expect(saveButton()?.disabled).toBe(true);
    // The sidebar says the other Macs have it.
    expect(screen.getByText('luật đã gửi tới 2 Mac khác lúc 14:02', { exact: false })).toBeTruthy();
  });

  it('chips and sliders edit the draft, and Huỷ thay đổi puts the file back', async () => {
    const fake = await open();
    await press(team().getByRole('button', { name: /lan\.acme2/ }));
    expect(team().getAllByRole('button').map((b) => b.getAttribute('aria-pressed'))).toEqual(['true', 'false', 'true']);
    expect(team().getByRole('button', { name: /lan\.acme3/ }).textContent).toBe('2lan.acme3, thứ tự 2');
    // A slot out of the Team rotation can become the personal one.
    expect(personal().getByRole('button', { name: /lan\.acme2/ })).toBeTruthy();
    fireEvent.change(screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng'), { target: { value: '85' } });
    expect(screen.getByText('85%')).toBeTruthy();
    expect(screen.getByText('Có 2 thay đổi chưa lưu.')).toBeTruthy();
    expect(macs().getByText('Mac mini (máy này)').textContent).toBe('Mac mini (máy này) · chưa lưu');
    expect(saveButton()?.disabled).toBe(false);
    expect(fake.api.rulesDirty).toHaveBeenLastCalledWith(true);

    await press(screen.getByRole('button', { name: 'Huỷ thay đổi' }));
    expect(screen.queryByText(/thay đổi chưa lưu/)).toBeNull();
    expect(team().getAllByRole('button').every((b) => b.getAttribute('aria-pressed') === 'true')).toBe(true);
    expect(fake.api.rulesDirty).toHaveBeenLastCalledWith(false);
    expect(fake.api.saveRotation).not.toHaveBeenCalled();
  });

  it('saves for every Mac and says when', async () => {
    const fake = await open();
    fireEvent.change(screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng'), { target: { value: '85' } });
    await press(screen.getByRole('button', { name: 'Đặt giờ rời vòng cho lan.acme2' }));
    expect((screen.getByLabelText('Giờ rời vòng của lan.acme2') as HTMLInputElement).value).toBe('2026-01-16T18:00');
    await press(saveButton()!);
    expect(fake.api.saveRotation).toHaveBeenCalledWith(
      rulesFixture({ full: 85, expire: [{ slot: 2, at: '2026-01-16T17:45' }, { slot: 3, at: '2026-01-16T18:00' }] }),
      'v1',
    );
    // A save that went well is a toast; the check is on hover.
    const toast = screen.getByText('Đã lưu luật cho cả ba Mac lúc 14:20').closest('.toast')!;
    expect(toast.getAttribute('title')).toContain('Đã kiểm tra đủ 6 mục của luật ✓');
    expect(screen.queryByText(/thay đổi chưa lưu/)).toBeNull();
    expect(macs().getByText('Mac C').textContent).toBe('Mac C · đã nhận 14:20');
  });

  it('a Mac that cannot be reached is named, tried again, and retried on Thử lại', async () => {
    const fake = await open(fixtureModel('fresh'), { unreachable: ['mac-c'] });
    fireEvent.change(screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng'), { target: { value: '85' } });
    await press(saveButton()!);
    expect(screen.getByText('Đã lưu luật trên máy này lúc 14:20')).toBeTruthy();
    expect(screen.getByText('Chưa gửi được luật tới Mac C')).toBeTruthy();
    expect(screen.getByText('Đã lưu trên máy này và Mac B lúc 14:20. App thử lại mỗi 5 phút.')).toBeTruthy();
    expect(macs().getByText('Mac C').textContent).toBe('Mac C · chưa tới được · thử lại 14:25');
    expect(screen.getByText('1 Mac chưa nhận luật mới', { exact: false })).toBeTruthy();
    await press(screen.getByRole('button', { name: 'Thử lại' }));
    expect(fake.api.retryRotation).toHaveBeenCalledTimes(1);
  });

  it('Thử lại keeps the edits the owner has not saved', async () => {
    const down = rotationFixture({
      peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'unreachable', error: 'x', retryAt: '2026-01-16T07:25:40.000Z' } })],
    });
    const fake = await open(fixtureModel('fresh', { rotation: down }));
    fireEvent.change(screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng'), { target: { value: '85' } });
    await press(screen.getByRole('button', { name: 'Thử lại' }));
    expect(fake.api.retryRotation).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Có 1 thay đổi chưa lưu.')).toBeTruthy();
    expect((screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng') as HTMLInputElement).value).toBe('85');
  });

  it('nothing can be edited while a save runs', async () => {
    await open(fixtureModel('fresh'), { saveDelayMs: 30 });
    const slider = screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng') as HTMLInputElement;
    fireEvent.change(slider, { target: { value: '85' } });
    await press(saveButton()!);
    expect(slider.disabled).toBe(true);
    expect((screen.getByLabelText('Ngưỡng Fable tuần, phần trăm đã dùng') as HTMLInputElement).disabled).toBe(true);
    expect(await screen.findByText('Đã lưu luật cho cả ba Mac lúc 14:20')).toBeTruthy();
    expect(slider.disabled).toBe(false);
  });

  it('a file with the personal account inside Team is shown as invalid, to save over', async () => {
    await open(fixtureModel('fresh', { rotation: rotationFixture({ status: 'invalid', rules: null, reason: 'rotation_personal_in_team' }) }));
    expect(screen.getByText('File luật trên máy này không hợp lệ')).toBeTruthy();
    expect(screen.getByText(/^Tài khoản cá nhân cũng nằm trong vòng Team\./)).toBeTruthy();
  });

  it('a save the rules refuse shows the key and writes nothing', async () => {
    await open(fixtureModel('fresh'), { rulesError: { kind: 'invalid', detail: 'rotation_bad_value:personal_hours', restored: false } });
    fireEvent.change(screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng'), { target: { value: '85' } });
    await press(saveButton()!);
    expect(screen.getByText('Luật chưa hợp lệ')).toBeTruthy();
    expect(screen.getByText('Khoá personal_hours sai. Chưa ghi gì.')).toBeTruthy();
    // The draft stays, to fix and save again.
    expect(screen.getByText('Có 1 thay đổi chưa lưu.')).toBeTruthy();
  });

  it('hours that end before they start cannot be saved', async () => {
    await open();
    fireEvent.change(screen.getByLabelText('Giờ bắt đầu dùng tài khoản cá nhân'), { target: { value: '20' } });
    fireEvent.change(screen.getByLabelText('Giờ kết thúc dùng tài khoản cá nhân'), { target: { value: '9' } });
    expect(screen.getByText('Giờ kết thúc phải sau giờ bắt đầu.')).toBeTruthy();
    expect(saveButton()?.disabled).toBe(true);
  });

  it('another Mac with a newer copy: compare, then take it', async () => {
    const theirs = rulesFixture({ full: 88, personalTo: 21 });
    const model = fixtureModel('fresh', {
      rotation: rotationFixture({
        peers: [
          peerFixture('Mac B', 'mac-b', { state: { kind: 'newer', their: { at: '2026-01-16T14:11:00+07:00', machine: 'Mac B' } }, theirRules: theirs }),
          peerFixture('Mac C', 'mac-c'),
        ],
      }),
    });
    const fake = await open(model);
    expect(screen.getByText('Bản trên Mac B mới hơn (14:11)')).toBeTruthy();
    expect(macs().getByText('Mac B').textContent).toBe('Mac B · có bản mới hơn 14:11');
    await press(screen.getByRole('button', { name: 'So sánh' }));
    const dialog = screen.getByRole('dialog', { name: 'So sánh hai bản luật' });
    expect(within(dialog).getByText('Máy này lưu lúc 14:02. Mac B lưu lúc 14:11. App không tự gộp hai bản.')).toBeTruthy();
    expect(within(dialog).getAllByText('khác')).toHaveLength(2);
    expect(within(dialog).getByText('88%')).toBeTruthy();
    await press(within(dialog).getByRole('button', { name: 'Dùng bản trên Mac B' }));
    expect(fake.api.adoptRotation).toHaveBeenCalledWith('mac-b', 'v1');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText('Đã dùng bản luật của Mac B (14:11)')).toBeTruthy();
    expect((screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng') as HTMLInputElement).value).toBe('88');
  });

  it('keeping this Mac’s copy over a newer one stamps it again and sends it', async () => {
    const model = fixtureModel('fresh', {
      rotation: rotationFixture({
        peers: [peerFixture('Mac B', 'mac-b', { state: { kind: 'newer', their: { at: '2026-01-16T14:11:00+07:00', machine: 'Mac B' } }, theirRules: rulesFixture({ full: 88 }) })],
      }),
    });
    const fake = await open(model);
    await press(screen.getByRole('button', { name: 'So sánh' }));
    await press(screen.getByRole('button', { name: 'Giữ bản máy này' }));
    expect(fake.api.keepRotation).toHaveBeenCalledWith('v1');
    const toast = screen.getByText('Đã giữ bản của máy này').closest('.toast')!;
    expect(toast.getAttribute('title')).toBe('Đã gửi bản 14:20 tới Mac B. Bản của Mac B được sao lưu trên máy đó.');
  });

  it('two Macs editing: a newer copy arriving during an edit wins until the owner chooses', async () => {
    const fake = await open();
    fireEvent.change(screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng'), { target: { value: '85' } });
    // Mac B saves and pushes its copy to this Mac meanwhile.
    await act(async () => {
      fake.push({
        ...fake.current(),
        rotation: rotationFixture({ version: 'v9', rules: rulesFixture({ full: 92 }), stamp: { at: '2026-01-16T14:15:00+07:00', machine: 'Mac B' } }),
      });
    });
    expect(screen.getByText('Bản trên Mac B mới hơn, tải lại?')).toBeTruthy();
    expect(saveButton()?.disabled).toBe(true);
    // Keeping the edit saves it over the newer copy, knowingly.
    await press(screen.getByRole('button', { name: 'Giữ bản đang sửa' }));
    expect(fake.api.saveRotation).toHaveBeenCalledWith(rulesFixture({ full: 85 }), 'v9');
  });

  it('Tải lại drops the edit for the newer copy', async () => {
    const fake = await open();
    fireEvent.change(screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng'), { target: { value: '85' } });
    await act(async () => {
      fake.push({ ...fake.current(), rotation: rotationFixture({ version: 'v9', rules: rulesFixture({ full: 92 }), stamp: { at: '2026-01-16T14:15:00+07:00', machine: 'Mac B' } }) });
    });
    await press(screen.getByRole('button', { name: 'Tải lại' }));
    expect((screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng') as HTMLInputElement).value).toBe('92');
    expect(screen.queryByText(/thay đổi chưa lưu/)).toBeNull();
    expect(fake.api.saveRotation).not.toHaveBeenCalled();
  });

  it('leaving the tab with unsaved changes asks first', async () => {
    await open();
    await press(team().getByRole('button', { name: /lan\.acme3/ }));
    await press(screen.getByRole('button', { name: 'Claude' }));
    const dialog = screen.getByRole('dialog', { name: 'Lưu thay đổi luật trước khi rời trang này?' });
    expect(within(dialog).getByText('Tài khoản Team được xoay vòng: lan.acme1, lan.acme2, lan.acme3 → lan.acme1, lan.acme2')).toBeTruthy();
    await press(within(dialog).getByRole('button', { name: 'Huỷ' }));
    expect(heading()).toBe('Luật xoay vòng');
    await press(screen.getByRole('button', { name: 'Claude' }));
    await press(screen.getByRole('button', { name: 'Bỏ thay đổi' }));
    expect(heading()).toBe('Claude');
    await press(screen.getByRole('button', { name: 'Luật xoay vòng' }));
    expect(screen.queryByText(/thay đổi chưa lưu/)).toBeNull();
  });

  it('closing the window with unsaved changes asks, and Lưu saves before it closes', async () => {
    const fake = await open();
    await press(team().getByRole('button', { name: /lan\.acme3/ }));
    await act(async () => fake.requestClose());
    const dialog = screen.getByRole('dialog', { name: 'Lưu thay đổi luật trước khi đóng cửa sổ?' });
    await press(within(dialog).getByRole('button', { name: 'Lưu' }));
    expect(fake.api.saveRotation).toHaveBeenCalledWith(rulesFixture({ team: [2, 3] }), 'v1');
    expect(fake.api.closeMain).toHaveBeenCalledWith(false);
  });

  it('quitting from the menu bar with unsaved changes asks first', async () => {
    const fake = await open();
    await press(team().getByRole('button', { name: /lan\.acme3/ }));
    await act(async () => fake.requestQuit());
    const dialog = screen.getByRole('dialog', { name: 'Lưu thay đổi luật trước khi thoát Quota Station?' });
    await press(within(dialog).getByRole('button', { name: 'Bỏ thay đổi' }));
    expect(fake.api.saveRotation).not.toHaveBeenCalled();
    expect(fake.api.closeMain).toHaveBeenCalledWith(true);
    expect(fake.closed()).toBe('app');
  });

  it('a quit chosen before the window listened is still answered once', async () => {
    const fake = await open(fixtureModel('fresh'), { quitPending: true });
    await act(async () => {});
    // Nothing unsaved in this window: it quits.
    expect(fake.api.takeQuit).toHaveBeenCalled();
    expect(fake.api.closeMain).toHaveBeenCalledTimes(1);
    expect(fake.api.closeMain).toHaveBeenCalledWith(true);
  });

  it('another Mac whose copy has no updated line is not overwritten until the owner keeps this one', async () => {
    const model = fixtureModel('fresh', {
      rotation: rotationFixture({ peers: [peerFixture('Mac B', 'mac-b'), peerFixture('Mac C', 'mac-c', { state: { kind: 'edited', their: null }, theirRules: null })] }),
    });
    const fake = await open(model);
    expect(screen.getByText('Bản trên Mac C đã bị sửa tay')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Dùng bản trên Mac C' })).toBeNull();
    await press(screen.getByRole('button', { name: 'So sánh' }));
    const dialog = screen.getByRole('dialog', { name: 'So sánh hai bản luật' });
    await press(within(dialog).getByRole('button', { name: 'Giữ bản máy này' }));
    expect(fake.api.keepRotation).toHaveBeenCalledWith('v1');
    expect(macs().getByText('Mac C').textContent).toBe('Mac C · đã nhận 14:20');
  });

  it('closing with nothing unsaved closes at once', async () => {
    const fake = await open();
    await act(async () => fake.requestClose());
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(fake.api.closeMain).toHaveBeenCalledTimes(1);
  });

  it('a Mac with no rotation file starts from the switcher defaults', async () => {
    const model = fixtureModel('fresh', { rotation: rotationFixture({ status: 'missing', rules: null, version: null, stamp: null, peers: [] }) });
    const fake = await open(model);
    expect(screen.getByText('Chưa có file luật trên máy này')).toBeTruthy();
    expect(saveButton()?.textContent).toBe('Lưu cho máy này');
    expect(saveButton()?.disabled).toBe(true);
    await press(screen.getByRole('button', { name: 'Tạo từ mặc định' }));
    expect(screen.getByText('Có 1 thay đổi chưa lưu.')).toBeTruthy();
    await press(saveButton()!);
    expect(fake.api.saveRotation).toHaveBeenCalledWith(rulesFixture({ expire: [] }), null);
  });

  it('previews what the switcher would do with the draft', async () => {
    const fake = await open();
    expect(await screen.findByText('Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.', {}, { timeout: 2000 })).toBeTruthy();
    await press(team().getByRole('button', { name: /lan\.acme1/ }));
    expect(await screen.findByText(/^Lần kiểm tới sẽ chuyển sang lan\.acme2/, {}, { timeout: 2000 })).toBeTruthy();
    expect(fake.api.previewRotation).toHaveBeenLastCalledWith(rulesFixture({ team: [3, 4] }));
  });

  it('rows have short names and no line of explanation', async () => {
    await open();
    for (const label of ['Team trong vòng', 'Cá nhân', 'Chuyển khi 5 giờ đã dùng', 'Bỏ qua khi Fable tuần đã dùng', 'Giờ rời vòng', 'Xem trước'])
      expect(screen.getAllByText(label).length).toBeGreaterThan(0);
    const card = document.querySelector('.form')!;
    expect(card.textContent).not.toMatch(/Khoá/);
    expect(card.querySelectorAll('.fl small')).toHaveLength(0);
  });

  it('each slider mark sits where its value is', async () => {
    await open();
    const slider = screen.getByLabelText('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng').closest('.slider')!;
    const marks = Array.from(slider.querySelectorAll<HTMLElement>('.marks span')).map((m) => [m.textContent, m.style.left]);
    expect(marks).toEqual([
      ['80', '0%'],
      ['85', '27.78%'],
      ['90', '55.56%'],
      ['95', '83.33%'],
      ['98', '100%'],
    ]);
  });

  it('the preview is one sentence, with the switcher’s line on hover', async () => {
    await open();
    const line = await screen.findByText('Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.', {}, { timeout: 2000 });
    const preview = line.closest('.preview')!;
    expect(preview.textContent).toBe('Vẫn dùng lan.acme1: chưa tới ngưỡng tự chuyển.');
    expect(preview.getAttribute('title')).toBeTruthy();
  });

  it('goes by place when names are hidden', async () => {
    await open(hideNames(fixtureModel('fresh')));
    expect(team().getAllByRole('button').map((b) => [b.textContent, b.getAttribute('title')])).toEqual([
      ['1Team A, thứ tự 1', 'tài khoản 2'],
      ['2Team B, thứ tự 2', 'tài khoản 3'],
      ['3Team C, thứ tự 3', 'tài khoản 4'],
    ]);
    expect(personal().getByRole('button', { name: /Cá nhân/ })).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/@|lan\.|Acme/);
  });
});
