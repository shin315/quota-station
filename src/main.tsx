import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { MainWindowApp, PopoverApp } from './App';
import { tauriApi, type Api } from './lib/api';
import './styles/tokens.css';
import './styles/components.css';
import './styles/redesign.css';
import './styles/app.css';
import './styles/agents.css';
import './styles/backup.css';

type View = 'popover' | 'window';

function tauriLabel(): string | null {
  const meta = (window as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } }).__TAURI_INTERNALS__;
  return meta?.metadata?.currentWindow?.label ?? null;
}

async function start() {
  const label = tauriLabel();
  let api: Api = tauriApi;
  let view: View = label === 'main' ? 'window' : 'popover';
  let clock: (() => Date) | undefined;
  let preview = false;

  // `npm run dev` in a browser: fixture data, no Tauri. Not in the app build.
  if (!label && import.meta.env.DEV) {
    const p = await import('./preview');
    const setup = p.previewSetup(new URLSearchParams(location.search));
    api = setup.api;
    view = setup.view;
    clock = setup.clock;
    preview = true;
  }

  const theme = preview ? new URLSearchParams(location.search).get('theme') : null;
  document.documentElement.dataset.theme = theme === 'light' || theme === 'dark' ? theme : 'auto';
  document.body.classList.add(view === 'window' ? 'main-window' : 'popover-window');
  if (preview) document.body.classList.add('preview');
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <div className={'qs app-' + view} data-theme={document.documentElement.dataset.theme} data-material="glass">
        {view === 'window' ? <MainWindowApp api={api} clock={clock} /> : <PopoverApp api={api} clock={clock} />}
      </div>
    </StrictMode>,
  );
}

void start();
