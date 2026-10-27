// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { MainWindowApp, PopoverApp } from './App';
import { tauriApi, type Api } from './lib/api';
import { GuideApp } from './session/Guide';
import { THEME_PACKS } from './lib/model';
import './styles/globals.css';

type View = 'popover' | 'window' | 'guide';

function tauriLabel(): string | null {
  const meta = (window as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } }).__TAURI_INTERNALS__;
  return meta?.metadata?.currentWindow?.label ?? null;
}

async function start() {
  const label = tauriLabel();
  let api: Api = tauriApi;
  let view: View = label === 'main' ? 'window' : label === 'guide' ? 'guide' : 'popover';
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
  const pack = params?.get('pack');
  document.documentElement.dataset.pack = THEME_PACKS.find((p) => p === pack) ?? 'default';
  // macOS until the model says otherwise (usePlatform); the preview's `os=win` at once.
  document.documentElement.dataset.os = params?.get('os') === 'win' ? 'windows' : 'macos';
  // Glass and plain edges until macOS says otherwise (useMaterial).
  document.documentElement.dataset.material = 'glass';
  document.documentElement.dataset.contrast = 'normal';
  if (view === 'guide') {
    // A test build's guided session (session.rs): its own window and words.
    createRoot(document.getElementById('root')!).render(
      <StrictMode>
        <div className="qs h-full w-full" data-os="windows">
          <GuideApp api={api} />
        </div>
      </StrictMode>,
    );
    return;
  }
  // A page that reloaded (F5 where WebView2's browser keys should be off):
  // a test build notes it; any other build has no session to note it in.
  const navigation = performance.getEntriesByType?.('navigation')[0] as PerformanceNavigationTiming | undefined;
  if (label && navigation?.type === 'reload') void api.sessionNote('reload', { view }).catch(() => {});
  document.body.classList.add(view === 'window' ? 'main-window' : 'popover-window');
  if (preview) document.body.classList.add('preview');
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <div
        className={'qs app-' + view + (view === 'window' ? ' h-full w-full' : ' in-[.preview]:ml-auto in-[.preview]:w-(--pop-w)')}
        data-theme={document.documentElement.dataset.theme}
        data-material={document.documentElement.dataset.material}
        data-pack={document.documentElement.dataset.pack}
        data-contrast={document.documentElement.dataset.contrast}
        data-os={document.documentElement.dataset.os}
      >
        {view === 'window' ? <MainWindowApp api={api} clock={clock} /> : <PopoverApp api={api} clock={clock} />}
      </div>
    </StrictMode>,
  );
}

void start();
