// A fake Rust side on fixture data, for the tests and the browser preview.
// Never runs quota-axi. Plain code: the preview bundle must not load Vitest.

import type { Api } from '../lib/api';
import type { Model } from '../lib/model';

export function fakeApi(initial: Model) {
  let model = initial;
  const listeners = new Set<(m: Model) => void>();
  const push = (next: Model) => {
    model = next;
    listeners.forEach((l) => l(model));
  };
  const api: Api = {
    getModel: async () => model,
    refresh: async () => {
      const asked = model.reads;
      setTimeout(() => push({ ...model, reads: asked + 1 }), 300);
      return asked;
    },
    openSettings: async () => {},
    hidePopover: async () => {},
    resizePopover: async () => {},
    setQuotaAxiPath: async (path) => {
      if (path && !path.includes('quota-axi')) throw new Error('notFound');
      push({ ...model, quotaAxi: { ...model.quotaAxi, path: path || '~/.local/bin/quota-axi', custom: !!path } });
      return model;
    },
    grantKeychain: async () => {
      push({ ...model, snapshot: { ...model.snapshot, keychain: 'granted' } });
      return model;
    },
    onModel: async (handler) => {
      listeners.add(handler);
      return () => listeners.delete(handler);
    },
  };
  return { api, push, current: () => model };
}
