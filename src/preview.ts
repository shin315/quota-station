// Browser preview for `npm run dev`: the popover or settings window on a
// fixture, without Tauri. `?state=keychain&view=settings&error=notFound`.

import type { Api } from './lib/api';
import type { ErrorView } from './lib/model';
import { fakeApi } from './fixtures/fakeApi';
import { failing, FIXTURE_NOW, fixtureModel, SNAPSHOTS } from './fixtures/models';

export function previewSetup(params: URLSearchParams): { api: Api; view: 'popover' | 'settings'; clock: () => Date } {
  const state = params.get('state') ?? 'fresh';
  const error = params.get('error') as ErrorView['kind'] | null;
  const { api } = fakeApi(fixtureModel(SNAPSHOTS[state] ? state : 'fresh', error ? { errors: failing(error) } : {}));
  return {
    api: { ...api, openSettings: async () => void window.open('?view=settings&state=' + state, '_blank') },
    view: params.get('view') === 'settings' ? 'settings' : 'popover',
    clock: () => FIXTURE_NOW,
  };
}
