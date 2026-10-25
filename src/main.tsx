// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { MainWindowApp, PopoverApp } from './App';
import { tauriApi, type Api } from './lib/api';
import './styles/globals.css';

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

  const params = preview ? new URLSearchParams(location.search) : null;
  const theme = params?.get('theme');
  document.documentElement.dataset.theme = theme === 'light' || theme === 'dark' ? theme : 'auto';
  // Glass until macOS says it reduces transparency (useMaterial).
  document.documentElement.dataset.material = 'glass';
  document.body.classList.add(view === 'window' ? 'main-window' : 'popover-window');
  if (preview) document.body.classList.add('preview');
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <div
        className={'qs app-' + view + (view === 'window' ? ' h-full w-full' : ' in-[.preview]:ml-auto in-[.preview]:w-(--pop-w)')}
        data-theme={document.documentElement.dataset.theme}
        data-material={document.documentElement.dataset.material}
      >
        {view === 'window' ? <MainWindowApp api={api} clock={clock} /> : <PopoverApp api={api} clock={clock} />}
      </div>
    </StrictMode>,
  );
}

void start();
