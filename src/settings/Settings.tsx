// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Cài đặt tab of the main window: Keychain access for Claude Code's
// login (on a Mac whose Claude accounts the app does not keep), this Mac's name, the Antigravity OAuth client file (in a
// build with the Antigravity accounts), whether account names show, the
// light or dark look, Codex's folder (M8), where the backup packs go, the
// macOS notifications with the kinds of alert (M6) and alert 1's thresholds,
// launching at login and the version (M7). Rows speak for themselves: no
// line of explanation under them. Each Mac runs its own copy and switches
// only itself. First comes the card of the variables that override a login
// (M14); last the Nâng cao card: its switch, and while it is on, Firstmate
// and its state file, the other Macs the rotation file is copied to (an SSH
// alias each), and the clone "Cập nhật từ mã nguồn" builds from. Only a
// build with the author's tools has that card (`capabilities.authorTools`).

import { useState } from 'react';
import { AlertKinds } from '../alerts/AlertsTab';
import { EnvCard } from './EnvCard';
import { Card as Panel, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Segment, Segmented } from '@/components/ui/segmented';
import { IdMark } from '../components/accounts';
import { LABEL, rowClass, TabHead, TabSub, TabTitle } from '../components/page';
import { Button, Chip, CODE, Dot, Feedback, FieldHint, LinkButton, Toggle } from '../components/parts';
import { PERMISSION_CHIPS, UNREAD } from '../lib/alerts';
import { authorModel, type AuthorModel } from '../lib/author';
import type { ChipView } from '../lib/view';
import type { AlertKind, AppView, ClientStatus, Menubar, MenubarWindow, Model, Peer, PeerTest, SourceView, Theme, UpdateError } from '../lib/model';
import type { Action, FeedbackView, PopoverView } from '../lib/view';

const THEMES: [Theme, string][] = [
  ['auto', 'Theo macOS'],
  ['light', 'Sáng'],
  ['dark', 'Tối'],
];

export interface SettingsProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback?: FeedbackView | null;
  /** The buttons on a feedback line, the same as on the other tabs. */
  onAction(act: Action): void;
  onGrant(): void;
  /** Resolves to false when the name could not be saved. */
  onSetMachine(name: string): Promise<boolean>;
  /** Resolves to null once saved, else the refusal (`badAlias:1`, `sameAlias:1`, `tooMany`). */
  onSetPeers(peers: Peer[]): Promise<string | null>;
  onTestPeer(alias: string): Promise<PeerTest>;
  /** Names (emails, organizations) on or off, in both windows. */
  onShowNames(on: boolean): void;
  onTheme(theme: Theme): void;
  /** The service the menu bar title follows. */
  onMenubar(menubar: Menubar): void;
  /** The window the menu bar title shows. */
  onMenubarWindow(window: MenubarWindow): void;
  /** Resolves to null once saved, else the refusal (`notPoolFile`, `notFound`). */
  onSetFirstmate(path: string): Promise<string | null>;
  /** Codex's folder; empty is `~/.codex`. Resolves to what it was refused with, or null. */
  onSetCodexHome?(path: string): Promise<string | null>;
  onTestAlert(kind: AlertKind): void;
  onAlertSwitch(kind: AlertKind, on: boolean): void;
  /** Resolves to null once changed, else the refusal (`unavailable`, or macOS's message). */
  onLaunchAtLogin(on: boolean): Promise<string | null>;
  onOpenLoginItems(): void;
  /** Resolves to null once saved, else the refusal (`missing`, `notSource`). */
  onSetSource(path: string): Promise<string | null>;
  /** Resolves to null once the update ran (how it went is in the model), else the refusal. */
  onUpdate(): Promise<string | null>;
  /** The app quits on success; resolves to the refusal (`rulesDirty`, …) otherwise. */
  onRelaunch(): Promise<string | null>;
  /** Skills (M11): "Kiểm bản mới" once a day or never, and whether a macOS card says so too. */
  onSkillChecks?(daily: boolean, notify: boolean): void;
  /** Nâng cao on or off. */
  onAdvanced(on: boolean): void;
  /** The Firstmate switch inside Nâng cao. */
  onFirstmate(on: boolean): void;
  /** Alert 1's thresholds; resolves to null once saved, else the refusal (`range`, `order`). */
  onPoolAlert(dryAtMost: number, roomAtLeast: number): Promise<string | null>;
  /** Where the backup packs go; empty is iCloud Drive. Resolves to the refusal (`notFound`), or null. */
  onSetBackupFolder?(path: string): Promise<string | null>;
}

const CLIENT_CHIPS: Record<ClientStatus, ChipView> = {
  ok: { t: 'có, dùng được', k: 'ok' },
  missing: { t: 'chưa có', k: 'warn' },
  tooOpen: { t: 'cần quyền 600', k: 'bad' },
  invalid: { t: 'chưa đúng', k: 'bad' },
  unreadable: { t: 'không đọc được', k: 'bad' },
};

/**
 * A text field that follows `value` when it changes on the Rust side. The
 * reset happens during render, not in an effect: an effect that runs late
 * would overwrite what the user just typed.
 */
function useField(value: string): [string, (next: string) => void] {
  const [text, setText] = useState(value);
  const [seen, setSeen] = useState(value);
  if (seen !== value) {
    setSeen(value);
    setText(value);
  }
  return [text, setText];
}

function MachineName({ model, onSet }: { model: Model; onSet(name: string): Promise<boolean> }) {
  const [name, setName] = useField(model.machine);
  const commit = async () => {
    if (name.trim() !== model.machine && !(await onSet(name))) setName(model.machine);
  };
  return (
    <div className={rowClass()}>
      <label className={LABEL} htmlFor="qs-name">
        Tên máy này
      </label>
      <div>
        <Input
          id="qs-name"
          value={name}
          placeholder={model.machineDefault}
          maxLength={40}
          spellCheck={false}
          autoComplete="off"
          onChange={(e) => setName(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void commit();
          }}
        />
        <FieldHint>Để trống để dùng tên trong macOS: {model.machineDefault}.</FieldHint>
      </div>
    </div>
  );
}

type SshTest = { kind: 'testing' } | { kind: 'ok'; hasFile: boolean } | { kind: 'fail'; error: string | null };

function testText(t: SshTest | undefined, alias: string): { tone: string; text: string } {
  if (!alias.trim()) return { tone: 'bad', text: 'Nhập tên kết nối SSH' };
  if (!t) return { tone: 'muted', text: 'Chưa thử' };
  if (t.kind === 'testing') return { tone: 'muted', text: 'Đang thử…' };
  if (t.kind === 'ok') return { tone: 'ok', text: t.hasFile ? 'SSH ổn' : 'SSH ổn · máy đó chưa có file luật' };
  if (t.error === 'badAlias') return { tone: 'bad', text: PEER_ERRORS.badAlias };
  return {
    tone: 'bad',
    text: t.error === 'timeout' || !t.error ? 'SSH hết giờ chờ. Kiểm tên kết nối hoặc bật máy đó.' : 'SSH không tới được: ' + t.error,
  };
}

const PEER_ERRORS: Record<string, string> = {
  badAlias: 'Tên kết nối chỉ gồm chữ, số và . - _ @, không bắt đầu bằng dấu gạch.',
  sameAlias: 'Tên kết nối này đã có ở dòng trên.',
  tooMany: 'Tối đa 8 Mac.',
};

/** "Các Mac khác": a name and an SSH alias for each, saved when a field is left. */
function OtherMacs({ model, onSet, onTest }: { model: AuthorModel; onSet: SettingsProps['onSetPeers']; onTest: SettingsProps['onTestPeer'] }) {
  const saved = model.rotation.peers.map((p) => ({ name: p.name, alias: p.alias }));
  const key = JSON.stringify(saved);
  const [rows, setRows] = useState<Peer[]>(saved);
  const [seen, setSeen] = useState(key);
  const [error, setError] = useState<{ row: number; text: string } | null>(null);
  const [tests, setTests] = useState<Record<string, SshTest>>({});
  if (seen !== key) {
    // A new row still waiting for its alias stays under the saved ones.
    setSeen(key);
    setRows([...saved, ...rows.filter((r) => !r.alias.trim())]);
  }
  const commit = async (next: Peer[]) => {
    // A new row waits for its alias; the others are saved without it.
    const rowOf = next.flatMap((p, i) => (p.alias.trim() ? [i] : []));
    const complete = rowOf.map((i) => next[i]);
    if (JSON.stringify(complete) === key) return setError(null);
    const refused = await onSet(complete);
    if (!refused) return setError(null);
    const [code, at] = refused.split(':');
    setError({ row: at === undefined ? -1 : (rowOf[Number(at)] ?? -1), text: PEER_ERRORS[code] ?? 'Chưa lưu được: ' + refused });
  };
  const set = (i: number, field: keyof Peer, value: string) => setRows(rows.map((r, j) => (j === i ? { ...r, [field]: value } : r)));
  const onKey = (e: { key: string }) => {
    if (e.key === 'Enter') void commit(rows);
  };
  const test = async () => {
    const aliases = rows.map((r) => r.alias.trim()).filter(Boolean);
    setTests(Object.fromEntries(aliases.map((a) => [a, { kind: 'testing' } as SshTest])));
    await Promise.all(
      aliases.map(async (a) => {
        const r = await onTest(a).catch((e: unknown) => ({ reached: false, error: String(e), hasFile: false, stamp: null }) as PeerTest);
        const t: SshTest = r.reached ? { kind: 'ok', hasFile: r.hasFile } : { kind: 'fail', error: r.error };
        setTests((cur) => ({ ...cur, [a]: t }));
      }),
    );
  };
  return (
    <div className={rowClass()}>
      <div className={LABEL}>
        Các Mac khác
      </div>
      <div className="grid gap-1.5">
        {rows.map((r, i) => {
          const t = testText(tests[r.alias.trim()], r.alias);
          const label = r.name.trim() || 'Mac mới';
          return (
            <div className="flex items-center gap-2 text-sm" key={i}>
              <Dot tone={t.tone} />
              <Input
                className="w-[150px]"
                value={r.name}
                placeholder="Tên Mac"
                maxLength={40}
                spellCheck={false}
                autoComplete="off"
                aria-label={'Tên của ' + label}
                onChange={(e) => set(i, 'name', e.target.value)}
                onBlur={() => void commit(rows)}
                onKeyDown={onKey}
              />
              <span className="whitespace-nowrap text-xs text-ink2">tên kết nối</span>
              <Input
                mono
                invalid={error?.row === i}
                className="w-[150px]"
                value={r.alias}
                placeholder="vd. mac-b"
                maxLength={64}
                spellCheck={false}
                autoComplete="off"
                aria-label={'Tên kết nối SSH của ' + label}
                onChange={(e) => set(i, 'alias', e.target.value)}
                onBlur={() => void commit(rows)}
                onKeyDown={onKey}
              />
              <small className="text-ink2">{t.text}</small>
              <LinkButton
                aria-label={'Bỏ ' + label}
                onClick={() => {
                  const next = rows.filter((_, j) => j !== i);
                  setRows(next);
                  void commit(next);
                }}
              >
                Bỏ
              </LinkButton>
            </div>
          );
        })}
        {error ? <FieldHint tone="bad">{error.text}</FieldHint> : null}
        {!rows.length ? <FieldHint>Chưa có Mac nào. Luật xoay vòng chỉ lưu trên máy này.</FieldHint> : null}
        {!model.rotation.ssh ? <FieldHint tone="bad">Không thấy ssh trên máy này.</FieldHint> : null}
        <div>
          <Button disabled={rows.length >= 8} onClick={() => setRows([...rows, { name: '', alias: '' }])}>
            Thêm Mac
          </Button>{' '}
          <Button disabled={!rows.some((r) => r.alias.trim())} onClick={() => void test()}>
            Thử SSH
          </Button>
        </div>
      </div>
    </div>
  );
}

const FIRSTMATE_REFUSED: Record<string, string> = {
  notPoolFile: UNREAD.notPoolFile,
  notFound: 'Không có file ở đường dẫn này.',
};

function firstmateHint(model: AuthorModel, refused: string | null): { tone: 'ok' | 'bad' | ''; text: string } {
  if (refused) return { tone: 'bad', text: FIRSTMATE_REFUSED[refused] ?? 'Chưa lưu được: ' + refused };
  const f = model.firstmate;
  switch (f.status) {
    case 'notSet':
      return { tone: '', text: 'Để trống nếu máy này không có firstmate.' };
    case 'ok':
      return f.fresh ? { tone: 'ok', text: 'Đọc được ✓' } : { tone: '', text: 'Đọc được, nhưng firstmate chưa ghi lại file trong 30 phút qua.' };
    case 'missing':
      return { tone: 'bad', text: 'Không có file ở đường dẫn này.' };
    default:
      return { tone: 'bad', text: UNREAD[f.status] ?? UNREAD.unreadable };
  }
}

/** Firstmate's `quota-pool.last`: no path is built in, the owner sets it. */
function FirstmateFile({ model, onSet }: { model: AuthorModel; onSet: SettingsProps['onSetFirstmate'] }) {
  const [path, setPath] = useField(model.firstmate.file ?? '');
  const [refused, setRefused] = useState<string | null>(null);
  const commit = async () => {
    if (path.trim() === (model.firstmate.file ?? '')) return setRefused(null);
    setRefused(await onSet(path.trim()));
  };
  const hint = firstmateHint(model, refused);
  return (
    <div className={rowClass()}>
      <label className={LABEL} htmlFor="qs-fm">
        File của firstmate
      </label>
      <div>
        <Input
          id="qs-fm"
          mono
          invalid={hint.tone === 'bad'}
          value={path}
          placeholder="đường dẫn tới quota-pool.last"
          spellCheck={false}
          autoComplete="off"
          onChange={(e) => {
            setPath(e.target.value);
            setRefused(null);
          }}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void commit();
          }}
        />
        {hint.text ? <FieldHint tone={hint.tone}>{hint.text}</FieldHint> : null}
      </div>
    </div>
  );
}

/** Codex's folder (M8): `~/.codex` unless set; the switch edits `config.toml` there. */
function CodexHome({ model, onSet }: { model: Model; onSet: NonNullable<SettingsProps['onSetCodexHome']> }) {
  const shown = model.codex.customHome ? model.codex.home : '';
  const [path, setPath] = useField(shown);
  const [refused, setRefused] = useState<string | null>(null);
  const commit = async () => {
    if (path.trim() === shown) return setRefused(null);
    setRefused(await onSet(path.trim()));
  };
  const bad = refused != null;
  const hint = bad
    ? refused === 'notFound'
      ? 'Không có thư mục ở đường dẫn này.'
      : 'Chưa lưu được: ' + refused
    : !model.codex.present
      ? 'Không thấy thư mục Codex ở ' + model.codex.home + '.'
      : 'Đọc được ✓ · app chỉ sửa config.toml ở đây khi bạn chuyển Codex.';
  return (
    <div className={rowClass()}>
      <label className={LABEL} htmlFor="qs-codex">Thư mục Codex</label>
      <div>
        <Input
          id="qs-codex"
          mono
          invalid={bad}
          value={path}
          placeholder="~/.codex"
          spellCheck={false}
          autoComplete="off"
          onChange={(e) => {
            setPath(e.target.value);
            setRefused(null);
          }}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void commit();
          }}
        />
        <FieldHint tone={bad ? 'bad' : model.codex.present ? 'ok' : ''}>{hint}</FieldHint>
      </div>
    </div>
  );
}

function Notifications(props: { model: Model; onAction: SettingsProps['onAction']; onTest: SettingsProps['onTestAlert']; onSwitch: SettingsProps['onAlertSwitch'] }) {
  const a = props.model.alerts;
  return (
    <div className={rowClass()}>
      <div className={LABEL}>
        Thông báo
      </div>
      <div>
        <Chip chip={PERMISSION_CHIPS[a.permission]} />{' '}
        {a.permission === 'denied' ? <Button onClick={() => props.onAction('notify:settings')}>Mở Cài đặt hệ thống</Button> : null}
        {a.permission === 'unbundled' ? <FieldHint>Chỉ app đã đóng gói mới gửi được thẻ macOS. Cảnh báo vẫn có ở tab Cảnh báo.</FieldHint> : null}
        <div className="mt-1.5">
          <AlertKinds model={props.model} switches={a.switches} testOff={a.permission === 'denied'} onTest={props.onTest} onSwitch={props.onSwitch} />
        </div>
      </div>
    </div>
  );
}

/** "Mở khi đăng nhập": macOS's own login item, off until turned on here. */
function LaunchAtLogin({ app, onSet, onOpen }: { app: AppView; onSet: SettingsProps['onLaunchAtLogin']; onOpen(): void }) {
  const [refused, setRefused] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const state = app.launchAtLogin;
  const on = state === 'on' || state === 'needsApproval';
  const toggle = async () => {
    setBusy(true);
    setRefused(await onSet(!on));
    setBusy(false);
  };
  return (
    <div className={rowClass()}>
      <div className={LABEL}>
        Mở khi đăng nhập
      </div>
      <div>
        <Toggle on={on} label="Mở khi đăng nhập" disabled={busy || state === 'unavailable'} onClick={() => void toggle()} />
        {state === 'needsApproval' ? (
          <>
            {' '}
            <Chip chip={{ t: 'chờ duyệt', k: 'warn' }} />{' '}
            <Button label="Mở Cài đặt hệ thống: Mục đăng nhập" onClick={onOpen}>
              Mở Cài đặt hệ thống
            </Button>
            <FieldHint>macOS cần bạn bật Quota Station trong Cài đặt hệ thống › Cài đặt chung › Mục đăng nhập.</FieldHint>
          </>
        ) : null}
        {state === 'unavailable' ? <FieldHint>Chỉ bản .app đã cài mới mở khi đăng nhập được.</FieldHint> : null}
        {refused && refused !== 'unavailable' ? <FieldHint tone="bad">Chưa đổi được: {refused}</FieldHint> : null}
      </div>
    </div>
  );
}

const SOURCE_REFUSED: Record<string, string> = {
  missing: 'Không có thư mục ở đường dẫn này.',
  notSource: 'Thư mục này không phải bản clone Quota Station: cần .git và scripts/install.sh.',
};

function sourceHint(app: SourceView, refused: string | null): { tone: 'ok' | 'bad' | ''; text: string } {
  if (refused) return { tone: 'bad', text: SOURCE_REFUSED[refused] ?? 'Chưa lưu được: ' + refused };
  if (app.sourceStatus === 'ok') return { tone: 'ok', text: 'Có scripts/install.sh ✓' };
  if (app.sourceStatus === 'notSet') return { tone: '', text: 'Để trống trên Mac cài bằng gói build từ Mac khác.' };
  return { tone: 'bad', text: SOURCE_REFUSED[app.sourceStatus] };
}

/** The clone "Cập nhật từ mã nguồn" pulls and builds. */
function SourceDir({ app, onSet }: { app: SourceView; onSet: SettingsProps['onSetSource'] }) {
  const [path, setPath] = useField(app.source ?? '');
  const [refused, setRefused] = useState<string | null>(null);
  const commit = async () => {
    if (path.trim() === (app.source ?? '')) return setRefused(null);
    setRefused(await onSet(path.trim()));
  };
  const hint = sourceHint(app, refused);
  return (
    <div className={rowClass()}>
      <label className={LABEL} htmlFor="qs-src">
        Mã nguồn
      </label>
      <div>
        <Input
          id="qs-src"
          mono
          invalid={hint.tone === 'bad'}
          value={path}
          placeholder="vd. ~/Code/quota-station"
          spellCheck={false}
          autoComplete="off"
          disabled={!!app.updating}
          onChange={(e) => {
            setPath(e.target.value);
            setRefused(null);
          }}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void commit();
          }}
        />
        {hint.text ? <FieldHint tone={hint.tone}>{hint.text}</FieldHint> : null}
      </div>
    </div>
  );
}

function errorText(e: UpdateError, log: string): string {
  switch (e.kind) {
    case 'source':
      return SOURCE_REFUSED[e.reason];
    case 'pull':
      return 'git pull không chạy được: ' + e.detail;
    case 'install':
      return 'Build hoặc cài lỗi: ' + e.detail + '. Log đầy đủ: ' + log;
    case 'timeout':
      return e.step === 'pulling' ? 'git pull hết giờ chờ. Kiểm mạng rồi thử lại.' : 'Build quá 30 phút nên đã dừng. Log: ' + log;
    case 'spawn':
      return 'Không chạy được: ' + e.detail;
  }
}

const UPDATE_REFUSED: Record<string, string> = {
  busy: 'Đang cập nhật.',
  noSource: 'Đặt thư mục mã nguồn ở trên để cập nhật.',
  unbundled: 'Bản đang chạy không phải .app đã cài, nên không có gì để cập nhật.',
  rulesDirty: 'Luật xoay vòng còn thay đổi chưa lưu. Lưu hoặc bỏ trước khi mở lại.',
  notInstalled: 'Chưa có bản mới để mở.',
};

/** The version this copy is. */
function Version({ app }: { app: AppView }) {
  return (
    <div className={rowClass()}>
      <div className={LABEL}>Phiên bản</div>
      <div>
        <span className={CODE}>
          {app.version}
          {app.commit ? ' · ' + app.commit : ''}
        </span>
      </div>
    </div>
  );
}

/** "Cập nhật từ mã nguồn" (Nâng cao): git pull --ff-only, then scripts/install.sh. */
function UpdateFromSource({ app, onUpdate, onRelaunch }: { app: SourceView; onUpdate: SettingsProps['onUpdate']; onRelaunch: SettingsProps['onRelaunch'] }) {
  const [refused, setRefused] = useState<string | null>(null);
  const run = async (act: () => Promise<string | null>) => {
    setRefused(null);
    setRefused(await act());
  };
  const short = (commit: string) => commit.slice(0, 7);
  let line: { tone: 'ok' | 'bad' | ''; text: string } | null = null;
  if (app.updating === 'pulling') line = { tone: '', text: 'Đang lấy mã mới (git pull --ff-only)…' };
  else if (app.updating === 'installing') line = { tone: '', text: 'Đang build và cài. Mất vài phút; app vẫn chạy như thường.' };
  else if (app.error) line = { tone: 'bad', text: errorText(app.error, app.log) };
  else if (app.outcome?.kind === 'upToDate') line = { tone: 'ok', text: 'Đã là bản mới nhất (' + short(app.outcome.commit) + ').' };
  else if (app.outcome?.kind === 'installed')
    line = {
      tone: 'ok',
      text: 'Đã cài ' + (app.outcome.version ?? 'bản mới') + ' (' + short(app.outcome.commit) + '). Mở lại để dùng bản mới.',
    };
  else if (!app.bundled) line = { tone: '', text: UPDATE_REFUSED.unbundled };
  else if (app.sourceStatus === 'notSet') line = { tone: '', text: UPDATE_REFUSED.noSource };
  const ready = app.bundled && app.sourceStatus === 'ok' && !app.updating;
  return (
    <div className={rowClass()}>
      <div className={LABEL}>Cập nhật</div>
      <div>
        <div>
          <Button primary disabled={!ready} onClick={() => void run(onUpdate)}>
            {app.updating ? 'Đang cập nhật…' : 'Cập nhật từ mã nguồn'}
          </Button>
          {app.outcome?.kind === 'installed' && !app.updating ? (
            <>
              {' '}
              <Button onClick={() => void run(onRelaunch)}>Mở lại</Button>
            </>
          ) : null}
        </div>
        {line ? <FieldHint tone={line.tone}>{line.text}</FieldHint> : null}
        {refused ? <FieldHint tone="bad">{UPDATE_REFUSED[refused] ?? 'Chưa chạy được: ' + refused}</FieldHint> : null}
      </div>
    </div>
  );
}

const POOL_REFUSED: Record<string, string> = {
  range: 'Mỗi ngưỡng từ 1 tới 99%.',
  order: 'Ngưỡng của tài khoản khác phải lớn hơn ngưỡng cạn.',
};

/** Alert 1: the account in use at this share left or less, and no other with that much. */
function PoolAlertRow({ model, onSet }: { model: Model; onSet: SettingsProps['onPoolAlert'] }) {
  const p = model.alerts.poolAlert;
  const [dry, setDry] = useField(String(p.dryAtMost));
  const [room, setRoom] = useField(String(p.roomAtLeast));
  const [refused, setRefused] = useState<string | null>(null);
  const commit = async () => {
    const d = Number(dry);
    const r = Number(room);
    if (d === p.dryAtMost && r === p.roomAtLeast) return setRefused(null);
    if (!Number.isInteger(d) || !Number.isInteger(r)) return setRefused('range');
    setRefused(await onSet(d, r));
  };
  const field = (id: string, value: string, set: (v: string) => void, label: string) => (
    <Input
      id={id}
      className="w-[56px] text-right tabular-nums"
      inputMode="numeric"
      invalid={refused != null}
      value={value}
      aria-label={label}
      onChange={(e) => {
        set(e.target.value.replace(/[^0-9]/g, '').slice(0, 2));
        setRefused(null);
      }}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') void commit();
      }}
    />
  );
  return (
    <div className={rowClass()}>
      <div className={LABEL}>Hạn mức sắp cạn</div>
      <div>
        <div className="flex flex-wrap items-center gap-1.5 text-sm">
          <span>Tài khoản đang dùng còn ≤</span>
          {field('qs-pool-dry', dry, setDry, 'Còn bao nhiêu phần trăm thì coi là sắp cạn')}
          <span>% và không tài khoản nào còn ≥</span>
          {field('qs-pool-room', room, setRoom, 'Tài khoản khác còn bao nhiêu phần trăm thì coi là còn hạn mức')}
          <span>%</span>
        </div>
        {refused ? (
          <FieldHint tone="bad">{POOL_REFUSED[refused] ?? 'Chưa lưu được: ' + refused}</FieldHint>
        ) : (
          <FieldHint>Cảnh báo 1 dùng hai ngưỡng này.</FieldHint>
        )}
      </div>
    </div>
  );
}

/** Where the backup packs go: iCloud Drive's folder unless another is chosen. */
function BackupFolder({ model, onSet }: { model: Model; onSet: NonNullable<SettingsProps['onSetBackupFolder']> }) {
  const b = model.backup;
  const shown = b.customFolder ? b.folder : '';
  const [path, setPath] = useField(shown);
  const [refused, setRefused] = useState<string | null>(null);
  const commit = async () => {
    if (path.trim() === shown) return setRefused(null);
    setRefused(await onSet(path.trim()));
  };
  const hint = refused
    ? refused === 'notFound'
      ? 'Không có thư mục ở đường dẫn này.'
      : 'Chưa lưu được: ' + refused
    : b.customFolder
      ? 'Gói mới vào thư mục này. Gói cũ vẫn ở chỗ cũ.'
      : 'Để trống để dùng iCloud Drive › Quota Station Agents.';
  return (
    <div className={rowClass()}>
      <label className={LABEL} htmlFor="qs-backup">
        Thư mục sao lưu
      </label>
      <div>
        <Input
          id="qs-backup"
          mono
          invalid={refused != null}
          value={path}
          placeholder="iCloud Drive › Quota Station Agents"
          spellCheck={false}
          autoComplete="off"
          onChange={(e) => {
            setPath(e.target.value);
            setRefused(null);
          }}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void commit();
          }}
        />
        <FieldHint tone={refused ? 'bad' : ''}>{hint}</FieldHint>
      </div>
    </div>
  );
}

/**
 * Nâng cao, the last card: what was made for the author's own Macs. Off, its
 * tabs (Luật xoay vòng, Firstmate, Chép sang Mac khác), the automatic switch
 * and the pause, and alerts 2 and 5 are hidden; the rows below come with it.
 */
function Advanced(props: SettingsProps & { model: AuthorModel }) {
  const { model } = props;
  const a = model.advanced;
  return (
    <Panel>
      <CardTitle>Nâng cao</CardTitle>
      <div className="grid">
        <div className={rowClass()}>
          <div className={LABEL}>Hiện phần nâng cao</div>
          <div>
            <Toggle on={a.on} label="Hiện phần nâng cao" onClick={() => props.onAdvanced(!a.on)} />
            <FieldHint>Tự chuyển Claude theo luật xoay vòng, tạm dừng, các Mac khác, cập nhật từ mã nguồn và Firstmate.</FieldHint>
          </div>
        </div>
        {a.on ? (
          <>
            <div className={rowClass()}>
              <div className={LABEL}>Firstmate</div>
              <div>
                <Toggle on={a.firstmate} label="Firstmate" onClick={() => props.onFirstmate(!a.firstmate)} />
                <FieldHint>Tab Firstmate và cảnh báo khi Firstmate chờ hạn mức Claude.</FieldHint>
              </div>
            </div>
            {a.firstmate ? <FirstmateFile model={model} onSet={props.onSetFirstmate} /> : null}
            <OtherMacs model={model} onSet={props.onSetPeers} onTest={props.onTestPeer} />
            <SourceDir app={model.app} onSet={props.onSetSource} />
            <UpdateFromSource app={model.app} onUpdate={props.onUpdate} onRelaunch={props.onRelaunch} />
          </>
        ) : null}
      </div>
    </Panel>
  );
}

const MENUBARS: [Menubar, string, 'c' | 'g' | 'x'][] = [
  ['claude', 'Claude', 'c'],
  ['gemini', 'Gemini', 'g'],
  ['codex', 'Codex', 'x'],
];

/**
 * Why this Mac cannot follow a service in the menu bar, as the Rust side
 * decides it (`icon::followed`): no Antigravity, or no Codex reached through
 * a ChatGPT login or a provider of its own. '' when it can.
 */
export function menubarOff(model: Model, menubar: Menubar): string {
  if (menubar === 'gemini' && model.snapshot.agy.status === 'notSetUp') return 'Máy này chưa có Antigravity.';
  if (menubar === 'codex') {
    if (!model.codex.present) return 'Máy này chưa có Codex.';
    if (model.codex.current === 'chatgpt' && model.snapshot.codex.status === 'notSetUp') return 'Codex chưa đăng nhập ChatGPT.';
  }
  return '';
}

/** Thanh menu: which service's account and percent used the menu bar shows. */
/** Skills (M11, G): looking for newer versions, and how the app says it found one. */
function SkillChecksRows({ model, onSet }: { model: Model; onSet: NonNullable<SettingsProps['onSkillChecks']> }) {
  const c = model.skills.updates.settings;
  return (
    <>
      <div className={rowClass()}>
        <div className={LABEL} id="qs-skcheck-l">
          Kiểm bản mới
        </div>
        <div>
          <Segmented variant="inline" aria-labelledby="qs-skcheck-l">
            <Segment pressed={c.daily} onClick={() => onSet(true, c.notify)}>
              Mỗi ngày
            </Segment>
            <Segment pressed={!c.daily} onClick={() => onSet(false, c.notify)}>
              Tắt
            </Segment>
          </Segmented>
          <FieldHint>Hỏi GitHub một lần cho mỗi repo có skill đã cài. App không tự cập nhật skill.</FieldHint>
        </div>
      </div>
      <div className={rowClass()}>
        <div className={LABEL}>Báo bằng thông báo macOS</div>
        <div>
          <Toggle on={c.notify} label="Báo skill có bản mới bằng thông báo macOS" onClick={() => onSet(c.daily, !c.notify)} />
          <FieldHint>Tắt thì chỉ báo trong tab Cảnh báo.</FieldHint>
        </div>
      </div>
    </>
  );
}

function MenubarRow({ model, onPick }: { model: Model; onPick(menubar: Menubar): void }) {
  const offs = MENUBARS.map(([v]) => menubarOff(model, v)).filter(Boolean);
  const picked = menubarOff(model, model.menubar);
  return (
    <div className={rowClass()}>
      <div className={LABEL} id="qs-menubar-l">
        Thanh menu
      </div>
      <div>
        <Segmented variant="menubar" aria-labelledby="qs-menubar-l">
          {MENUBARS.map(([v, t, id]) => {
            const off = menubarOff(model, v);
            return (
              <Segment key={v} pressed={model.menubar === v} disabled={!!off} title={off || undefined} onClick={() => onPick(v)}>
                <IdMark id={id} />
                {t}
              </Segment>
            );
          })}
        </Segmented>
        <FieldHint>Tên và phần trăm đã dùng của tài khoản đang dùng.</FieldHint>
        {offs.length ? <FieldHint tone="off">{picked ? picked.replace(/\.$/, '') + ': thanh menu theo Claude.' : offs.join(' ')}</FieldHint> : null}
      </div>
    </div>
  );
}

const MENUBAR_WINDOWS: [MenubarWindow, string][] = [
  ['fiveHours', '5 giờ'],
  ['week', 'Tuần'],
  ['fableWeek', 'Fable tuần'],
  ['tightest', 'Chặt nhất'],
];

/**
 * Why the menu bar cannot show a window: only Claude has Fable tuần, and the
 * title follows Claude when this Mac lacks the service picked. '' when it can.
 */
export function menubarWindowOff(model: Model, window: MenubarWindow): string {
  const follows = menubarOff(model, model.menubar) ? 'claude' : model.menubar;
  if (window !== 'fableWeek' || follows === 'claude') return '';
  return (follows === 'gemini' ? 'Gemini' : 'Codex') + ' không có Fable tuần.';
}

/** Số trên thanh menu: which window's share used the menu bar shows. */
function MenubarWindowRow({ model, onPick }: { model: Model; onPick(window: MenubarWindow): void }) {
  const picked = menubarWindowOff(model, model.menubarWindow);
  return (
    <div className={rowClass()}>
      <div className={LABEL} id="qs-mbwindow-l">
        Số trên thanh menu
      </div>
      <div>
        <Segmented variant="inline" aria-labelledby="qs-mbwindow-l">
          {MENUBAR_WINDOWS.map(([v, t]) => {
            const off = menubarWindowOff(model, v);
            return (
              <Segment key={v} pressed={model.menubarWindow === v} disabled={!!off} title={off || undefined} onClick={() => onPick(v)}>
                {t}
              </Segment>
            );
          })}
        </Segmented>
        <FieldHint>Chặt nhất: hạn mức đã dùng nhiều nhất.</FieldHint>
        {picked ? <FieldHint tone="off">{picked.replace(/\.$/, '') + ': thanh menu hiện 5 giờ.'}</FieldHint> : null}
      </div>
    </div>
  );
}

export function Settings(props: SettingsProps) {
  const {
  model,
  fresh,
  feedback,
  onAction,
  onGrant,
  onSetMachine,
  onShowNames,
  onTheme,
  onMenubar,
  onMenubarWindow,
  onSetCodexHome,
  onTestAlert,
  onAlertSwitch,
  onLaunchAtLogin,
  onOpenLoginItems,
  onSkillChecks,
  onPoolAlert,
  onSetBackupFolder,
  } = props;
  const keychain = model.snapshot.keychain;
  // Nâng cao is one of the author's tools: a release build has no such card.
  const author = authorModel(model);

  return (
    <>
      <TabHead>
        <TabTitle>Cài đặt</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
      </TabHead>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      <EnvCard model={model} onAction={onAction} />
      <Panel>
        <div className="grid">
          <div className={rowClass()}>
            <div className={LABEL}>
              Quyền Keychain
            </div>
            <div>
              {keychain === 'granted' ? (
                <Chip chip={{ t: 'đã cấp', k: 'ok' }} />
              ) : keychain === 'required' ? (
                <>
                  <Chip chip={{ t: 'chưa cấp', k: 'warn' }} />{' '}
                  <Button primary disabled={model.granting} onClick={onGrant}>
                    Cấp quyền một lần
                  </Button>
                  <FieldHint>
                    {model.granting
                      ? 'macOS sẽ hỏi. Bấm "Luôn cho phép". App chỉ đọc đăng nhập của Claude Code để lấy số, không làm mới hay sửa nó.'
                      : 'Đến khi cấp, dòng Claude chỉ hiện số đọc được lần trước.'}
                  </FieldHint>
                </>
              ) : (
                <Chip chip={{ t: 'chưa rõ', k: '' }} />
              )}
            </div>
          </div>
          <MachineName model={model} onSet={onSetMachine} />
          {model.capabilities.agyAccounts && model.antigravity ? (
            <div className={rowClass()}>
              <div className={LABEL}>
                OAuth client Antigravity
              </div>
              <div>
                <Chip chip={CLIENT_CHIPS[model.antigravity.client]} />
                <FieldHint>
                  <span className={CODE}>{model.antigravity.clientPath}</span>, quyền 600, chỉ có trên Mac này. README ghi định dạng.
                </FieldHint>
              </div>
            </div>
          ) : null}
          <div className={rowClass()}>
            <div className={LABEL} id="qs-email-l">
              Hiện email
            </div>
            <div>
              <Toggle on={model.showNames} label="Hiện email" onClick={() => onShowNames(!model.showNames)} />
            </div>
          </div>
          <div className={rowClass()}>
            <div className={LABEL} id="qs-theme-l">
              Giao diện
            </div>
            <Segmented variant="inline" className="justify-self-start" aria-labelledby="qs-theme-l">
              {THEMES.map(([v, t]) => (
                <Segment key={v} className="px-3.5" pressed={model.theme === v} onClick={() => onTheme(v)}>
                  {t}
                </Segment>
              ))}
            </Segmented>
          </div>
          <MenubarRow model={model} onPick={onMenubar} />
          <MenubarWindowRow model={model} onPick={onMenubarWindow} />
          {onSetCodexHome ? <CodexHome model={model} onSet={onSetCodexHome} /> : null}
          {onSetBackupFolder ? <BackupFolder model={model} onSet={onSetBackupFolder} /> : null}
          <Notifications model={model} onAction={onAction} onTest={onTestAlert} onSwitch={onAlertSwitch} />
          <PoolAlertRow model={model} onSet={onPoolAlert} />
          {onSkillChecks ? <SkillChecksRows model={model} onSet={onSkillChecks} /> : null}
          <LaunchAtLogin app={model.app} onSet={onLaunchAtLogin} onOpen={onOpenLoginItems} />
          <Version app={model.app} />
        </div>
      </Panel>
      {author ? <Advanced {...props} model={author} /> : null}
    </>
  );
}
