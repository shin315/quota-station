// A fake Rust side for component tests: records calls, never runs quota-axi.

import { vi } from 'vitest';
import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

export function fakeApi(initial: Model) {
  let model = initial;
  const listeners = new Set<(m: Model) => void>();
  const push = (next: Model) => {
    model = next;
    listeners.forEach((l) => l(model));
  };
  const api = {
    getModel: vi.fn(async () => model),
    refresh: vi.fn(async () => model.reads),
    openSettings: vi.fn(async () => {}),
    hidePopover: vi.fn(async () => {}),
    resizePopover: vi.fn(async (_height: number) => {}),
    setQuotaAxiPath: vi.fn(async (_path: string) => model),
    grantKeychain: vi.fn(async () => model),
    onModel: vi.fn(async (handler: (m: Model) => void) => {
      listeners.add(handler);
      return () => listeners.delete(handler);
    }),
  } satisfies Api;
  return { api, push, current: () => model };
}
