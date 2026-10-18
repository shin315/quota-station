// Browser preview for `npm run dev`: the popover or settings window on a
// fixture, without Tauri. `?state=keychain&view=settings&error=notFound`.

import type { Api } from './lib/api';
import type { ErrorView, Model } from './lib/model';
import { failing, FIXTURE_NOW, fixtureModel, SNAPSHOTS } from './fixtures/models';

export function previewSetup(params: URLSearchParams): { api: Api; view: 'popover' | 'settings'; clock: () => Date } {
  const state = params.get('state') ?? 'fresh';
  const error = params.get('error') as ErrorView['kind'] | null;
  let model: Model = fixtureModel(SNAPSHOTS[state] ? state : 'fresh', error ? { errors: failing(error) } : {});
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
    openSettings: async () => void window.open('?view=settings&state=' + state, '_blank'),
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
  return { api, view: params.get('view') === 'settings' ? 'settings' : 'popover', clock: () => FIXTURE_NOW };
}
