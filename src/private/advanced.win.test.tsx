// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The author's tools on Windows: the rotation, its rules, the pause and
// Firstmate come with them, as on a Mac; what only a Mac does (the other
// Macs, Chép sang Mac khác and the update from source) is never offered.
// The setup file puts the system back to macOS after each test.

import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MainWindowApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { appFixture, FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Model, Tab } from '../lib/model';

/** A Windows build with the author's tools, as Rust sends it: no update from source, no other Mac. */
function windowsModel(): Model {
  const m = fixtureModel('fresh');
  const { version, commit, launchAtLogin } = appFixture();
  return {
    ...m,
    capabilities: { ...m.capabilities, platform: 'windows' },
    app: { version, commit, launchAtLogin },
    rotation: { ...m.rotation!, peers: [], ssh: false },
  };
}

async function open(model: Model, tab: Tab) {
  const fake = fakeApi(model, tab);
  for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
  render(
    <div className="qs app-window">
      <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
    </div>,
  );
  await screen.findByRole('navigation', { name: 'Các tab' });
  return fake;
}

afterEach(cleanup);

describe('the author\'s tools on Windows', () => {
  it('keep the rules and Firstmate in the sidebar, without Chép sang Mac khác', async () => {
    await open(windowsModel(), 'overview');
    const nav = screen.getByRole('navigation', { name: 'Các tab' });
    expect(within(nav).getByRole('button', { name: /Luật xoay vòng/ })).toBeTruthy();
    expect(within(nav).getByRole('button', { name: /Firstmate/ })).toBeTruthy();
    expect(within(nav).queryByRole('button', { name: /Chép sang Mac khác/ })).toBeNull();
  });

  it('a Mac still has Chép sang Mac khác', async () => {
    await open(fixtureModel('fresh'), 'overview');
    const nav = screen.getByRole('navigation', { name: 'Các tab' });
    expect(within(nav).getByRole('button', { name: /Chép sang Mac khác/ })).toBeTruthy();
  });

  it('Nâng cao has Firstmate, and no other Macs or update from source', async () => {
    const fake = await open(windowsModel(), 'settings');
    await act(async () => {
      fireEvent.mouseDown(screen.getByRole('tab', { name: 'Nâng cao' }));
    });
    const card = screen.getByRole('region', { name: 'Nâng cao' });
    expect(within(card).getByText('Luật, tạm dừng và Firstmate.')).toBeTruthy();
    expect(within(card).getByText('File của firstmate')).toBeTruthy();
    expect(within(card).queryByText('Các Mac khác')).toBeNull();
    expect(within(card).queryByText('Mã nguồn')).toBeNull();
    expect(within(card).queryByRole('button', { name: 'Cập nhật từ mã nguồn' })).toBeNull();
    expect(fake.api.setPeers).not.toHaveBeenCalled();
  });

  it('the rules tab shows this computer alone', async () => {
    await open(windowsModel(), 'rules');
    const macs = await screen.findByRole('group', { name: 'Các Mac' });
    expect(within(macs).getAllByText(/\(máy này\)|\(PC này\)/)).toHaveLength(1);
  });
});
