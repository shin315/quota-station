// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AgyRotation as RotationView, Api } from '../lib/api';
import { AgyRotation, rotationLine } from './AgyRotation';

const OFF: RotationView = { supported: true, on: false, threshold: 5, cooldownMinutes: 30, minLead: 10 };
const NAME = 'Tự chuyển khi gần hết Gemini';

/** An api with only the rotation's two calls, answering as Rust would. */
function rotationApi(first: RotationView, set?: (on: boolean) => Promise<RotationView>) {
  return {
    agyRotation: vi.fn(async () => first),
    agySetRotation: vi.fn(set ?? (async (on: boolean) => ({ ...first, on }))),
  };
}

async function show(api: Partial<Api>) {
  render(<AgyRotation api={api as Api} />);
  await act(async () => {});
}

afterEach(cleanup);

describe('the Antigravity rotation switch', () => {
  it('starts off for a new owner, and turns its LaunchAgent on and off', async () => {
    const api = rotationApi(OFF);
    await show(api);
    const toggle = screen.getByRole('switch', { name: NAME });
    expect(toggle.getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText(/còn 5% Gemini trở xuống/)).toBeTruthy();

    await act(async () => void fireEvent.click(toggle));
    expect(api.agySetRotation).toHaveBeenLastCalledWith(true);
    expect(screen.getByRole('switch', { name: NAME }).getAttribute('aria-checked')).toBe('true');

    await act(async () => void fireEvent.click(screen.getByRole('switch', { name: NAME })));
    expect(api.agySetRotation).toHaveBeenLastCalledWith(false);
    expect(screen.getByRole('switch', { name: NAME }).getAttribute('aria-checked')).toBe('false');
  });

  it('says why when launchd refused, and stays as it was', async () => {
    const api = rotationApi(OFF, async () => {
      throw 'launchctl: Bootstrap failed: 5: Input/output error';
    });
    await show(api);
    await act(async () => void fireEvent.click(screen.getByRole('switch', { name: NAME })));
    expect(screen.getByRole('alert').textContent).toContain('Bootstrap failed');
    expect(screen.getByRole('switch', { name: NAME }).getAttribute('aria-checked')).toBe('false');
  });

  it('cannot be turned on where no LaunchAgent runs it', async () => {
    await show(rotationApi({ ...OFF, supported: false }));
    expect((screen.getByRole('switch', { name: NAME }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('Chỉ có trên macOS.')).toBeTruthy();
  });

  it('shows nothing without the rotation in the api', async () => {
    await show({});
    expect(screen.queryByRole('switch')).toBeNull();
  });

  it('names its settings', () => {
    const line = rotationLine({ ...OFF, threshold: 8, cooldownMinutes: 45 });
    expect(line).toContain('8%');
    expect(line).toContain('45 phút');
  });
});
