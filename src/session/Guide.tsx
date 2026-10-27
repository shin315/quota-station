// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The guide's window of a test build (src-tauri/src/session.rs): one step
// on screen at a time, what to do and what the right result looks like,
// buttons that do part of it, and "Xong". The owner never answers a
// question or reports a result: each step's facts and self-checks go to the
// app's session folder, and the last step of each part writes the zip.

import { useEffect, useRef, useState } from 'react';
import { cn } from '@/lib/utils';
import { useLang, useModel, usePlatform, useTheme, useThemePack } from '../App';
import { Button, LinkButton } from '../components/parts';
import type { Api } from '../lib/api';
import { STEPS, facts, lineText, type Ctx, type Help, type SessionView, type Step } from './steps';

const PART_NAMES = { 1: 'Phần 1 · Nhìn', 2: 'Phần 2 · Lõi' } as const;

/** Whether `font` draws differently from the fallback: it is on this PC. */
function fontThere(font: string): boolean | null {
  if (/jsdom/i.test(navigator.userAgent)) return null;
  const c = document.createElement('canvas').getContext('2d');
  if (!c) return null;
  const text = 'mmmmmmmmmmlli Quota Station 0123 Đường';
  c.font = '32px monospace';
  const plain = c.measureText(text).width;
  c.font = `32px "${font}", monospace`;
  return c.measureText(text).width !== plain;
}

/** What this page sees itself, once: its type, a contrast theme, a reload. */
function useOwnNotes(api: Api) {
  useEffect(() => {
    const fonts = Object.fromEntries(
      ['Segoe UI Variable Text', 'Segoe UI', 'Cascadia Mono', 'Consolas', 'Inter QS', 'Arial'].map((f) => [f, fontThere(f)]),
    );
    const contrast = typeof window.matchMedia === 'function' ? window.matchMedia('(forced-colors: active)') : null;
    void api.sessionNote('guide', { fonts, family: getComputedStyle(document.body).fontFamily, forcedColors: contrast?.matches ?? null }).catch(() => {});
    if (!contrast) return;
    const on = (e: MediaQueryListEvent) => void api.sessionNote('contrast', { forcedColors: e.matches }).catch(() => {});
    contrast.addEventListener?.('change', on);
    return () => contrast.removeEventListener?.('change', on);
  }, [api]);
}

export function GuideApp({ api }: { api: Api }) {
  const model = useModel(api);
  useTheme(model?.theme);
  useThemePack(model?.themePack);
  useLang('vi');
  usePlatform(model?.capabilities.platform);
  useOwnNotes(api);
  const [session, setSession] = useState<SessionView | null>(null);
  const [at, setAt] = useState<number | null>(null);

  // Where the guide was: the step saved, or the first. A step the owner
  // left by signing out is done once the app came back by itself.
  const resumed = useRef(false);
  useEffect(() => {
    void api.sessionGet().then(setSession, () => {});
  }, [api]);
  useEffect(() => {
    if (!session || !model || resumed.current) return;
    resumed.current = true;
    const saved = STEPS.findIndex((s) => s.id === session.step);
    const index = Math.max(saved, 0);
    const step = STEPS[index];
    if (step.restart && session.restarted) {
      void api.sessionDone(step.id, false, facts(model, session), step.checks ?? []).finally(() => setAt(index + 1));
    } else setAt(index);
  }, [api, model, session]);

  const step = at === null ? null : STEPS[at];
  useEffect(() => {
    if (step) void api.sessionStep(step.id, step.captures ?? []).catch(() => {});
  }, [api, step]);

  if (!model || !session || !step || at === null) {
    return <div className="p-4 text-ink2">Đang mở…</div>;
  }
  const ctx: Ctx = { model, session };
  const go = (skipped: boolean, why?: string) =>
    void api
      .sessionDone(step.id, skipped, { ...facts(model, session), ...(why ? { why } : {}) }, skipped ? [] : (step.checks ?? []))
      .finally(() => setAt(Math.min(at + 1, STEPS.length - 1)));
  return (
    <GuideStep
      key={step.id}
      api={api}
      ctx={ctx}
      step={step}
      onDone={() => go(false)}
      onSkip={(why) => go(true, why)}
      onBack={at > 0 ? () => setAt(at - 1) : undefined}
      onSession={setSession}
    />
  );
}

/** Where the step sits in its part: `Bước 3/17`. */
function position(step: Step): string {
  const part = STEPS.filter((s) => s.part === step.part);
  return `Bước ${part.indexOf(step) + 1}/${part.length}`;
}

function GuideStep(props: {
  api: Api;
  ctx: Ctx;
  step: Step;
  onDone(): void;
  onSkip(why?: string): void;
  onBack?: () => void;
  onSession(s: SessionView): void;
}) {
  const { api, ctx, step } = props;
  const cannot = step.cannot?.(ctx) ?? null;
  const last = step.end === 2;
  return (
    <main className="flex h-screen flex-col gap-3 overflow-y-auto bg-(--glass-solid) p-4 text-ink" aria-labelledby="guide-title">
      <header>
        <p className="m-0 text-xs text-ink2">
          {PART_NAMES[step.part]} · {position(step)}
        </p>
        <h1 id="guide-title" className="m-0 mt-0.5 text-lg font-semibold">
          {step.title}
        </h1>
      </header>
      {cannot ? (
        <p className="m-0">{cannot}</p>
      ) : (
        <>
          <section aria-label="Làm">
            <ol className="m-0 flex list-decimal flex-col gap-1.5 pl-5">
              {step.do.map((line, i) => (
                <li key={i}>{lineText(line, ctx)}</li>
              ))}
            </ol>
          </section>
          <section aria-label="Đúng là khi" className="rounded-tile bg-fill px-3 py-2">
            <p className="m-0 mb-1 text-xs font-semibold text-ink2">Đúng là khi</p>
            <ul className="m-0 flex list-disc flex-col gap-1 pl-5">
              {step.right.map((line, i) => (
                <li key={i}>{lineText(line, ctx)}</li>
              ))}
            </ul>
          </section>
          {step.help?.length ? (
            <div className="flex flex-wrap gap-1.5">
              {step.help.map((h) => (
                <HelpButton key={h.label} api={api} ctx={ctx} help={h} />
              ))}
            </div>
          ) : null}
          {step.end ? <ZipPart api={api} ctx={ctx} onSession={props.onSession} /> : null}
        </>
      )}
      <footer className="mt-auto flex flex-wrap items-center gap-2 pt-2">
        {cannot ? (
          <Button primary onClick={() => props.onSkip(cannot)}>
            Sang bước tiếp
          </Button>
        ) : step.end === 1 ? (
          <Button primary onClick={props.onDone}>
            Sang phần 2
          </Button>
        ) : last ? null : (
          <Button primary onClick={props.onDone}>
            Xong
          </Button>
        )}
        {step.skip && !cannot ? <Button onClick={() => props.onSkip()}>{step.skip}</Button> : null}
        {props.onBack ? (
          <LinkButton small className="ml-auto" onClick={props.onBack}>
            Quay lại bước trước
          </LinkButton>
        ) : null}
      </footer>
    </main>
  );
}

/** What each help button does, through the commands the app already has. */
function act(api: Api, ctx: Ctx, help: Help): Promise<unknown> {
  switch (help.kind) {
    case 'page':
      return api.sessionOpen(help.page);
    case 'window':
      return (help.group ? api.setSettingsGroup(help.group) : Promise.resolve()).then(() => api.openWindow(help.tab));
    case 'shot':
      return api.sessionShot(help.what, 3000);
    case 'toast':
      return api.testAlert('pool');
    case 'open':
      if (help.what === 'releases') return api.releaseOpen('releases');
      if (help.what === 'backup') return api.backupOpenFolder();
      if (help.what === 'mcpBackups') return api.mcpOpenBackups('claude');
      return api.openNotificationSettings();
    case 'copy':
      return navigator.clipboard.writeText(help.text(ctx));
  }
}

function HelpButton({ api, ctx, help }: { api: Api; ctx: Ctx; help: Help }) {
  const [said, setSaid] = useState<string | null>(null);
  const press = () =>
    void act(api, ctx, help).then(
      () => setSaid(help.kind === 'copy' ? 'Đã chép' : help.kind === 'shot' ? 'Sẽ chụp sau 3 giây' : null),
      () => setSaid('Không mở được'),
    );
  return (
    <span className="inline-flex items-center gap-1.5">
      <Button small tint onClick={press}>
        {help.label}
      </Button>
      {said ? (
        <span role="status" className="text-xs text-ink2">
          {said}
        </span>
      ) : null}
    </span>
  );
}

/** "Ghi gói chẩn đoán", then where the zip is and a button to show it. */
function ZipPart({ api, ctx, onSession }: { api: Api; ctx: Ctx; onSession(s: SessionView): void }) {
  const [state, setState] = useState<'idle' | 'writing' | 'failed'>('idle');
  const zip = ctx.session.zip;
  const write = () => {
    setState('writing');
    void api.sessionZip().then(
      (z) => {
        setState('idle');
        onSession({ ...ctx.session, zip: z });
      },
      () => setState('failed'),
    );
  };
  return (
    <section aria-label="Gói chẩn đoán" className={cn('rounded-tile px-3 py-2', zip ? 'bg-fill' : undefined)}>
      <div className="flex flex-wrap items-center gap-2">
        <Button primary={!zip} disabled={state === 'writing'} onClick={write}>
          {state === 'writing' ? 'Đang ghi…' : 'Ghi gói chẩn đoán'}
        </Button>
        {zip ? <Button onClick={() => void api.sessionReveal().catch(() => {})}>Mở thư mục chứa file</Button> : null}
      </div>
      {zip ? (
        <p role="status" className="m-0 mt-2">
          Đã ghi <b>{zip.name}</b> vào thư mục <b>{zip.folder}</b>. Kéo file đó vào chat.
        </p>
      ) : null}
      {state === 'failed' ? (
        <p role="alert" className="m-0 mt-2 text-bad-ink">
          Chưa ghi được. Bấm lại một lần nữa.
        </p>
      ) : null}
    </section>
  );
}
