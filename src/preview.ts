// Browser preview for `npm run dev`: the popover or the main window on a
// fixture, without Tauri. `?state=keychain&view=window&tab=claude&error=notFound`.

import type { Api } from './lib/api';
import type { ErrorView, Tab } from './lib/model';
import { fakeApi } from './fixtures/fakeApi';
import { failing, FIXTURE_NOW, fixtureModel, SNAPSHOTS } from './fixtures/models';

const TABS: Tab[] = ['overview', 'claude', 'agy', 'settings'];

export function previewSetup(params: URLSearchParams): { api: Api; view: 'popover' | 'window'; clock: () => Date } {
  const state = params.get('state') ?? 'fresh';
  const error = params.get('error') as ErrorView['kind'] | null;
  const tab = TABS.find((t) => t === params.get('tab')) ?? 'overview';
  const view = params.get('view') === 'window' ? 'window' : 'popover';
  const { api } = fakeApi(fixtureModel(SNAPSHOTS[state] ? state : 'fresh', error ? { errors: failing(error) } : {}), tab);
  const openWindow = async (next: Tab) => void window.open('?view=window&tab=' + next + '&state=' + state, '_blank');
  return { api: view === 'popover' ? { ...api, openWindow } : api, view, clock: () => FIXTURE_NOW };
}
