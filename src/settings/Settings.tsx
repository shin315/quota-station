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

import { useState, type ReactNode } from 'react';
import { AlertKinds } from '../alerts/AlertsTab';
import { EnvCard } from './EnvCard';
import { Card as Panel, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Segment, Segmented } from '@/components/ui/segmented';
import { IdMark } from '../components/accounts';
import { LABEL, rowClass, STACK, TabHead, TabSub, TabTitle } from '../components/page';
import { Button, Chip, CODE, Dot, Feedback, FieldHint, LinkButton, Toggle } from '../components/parts';
import { permissionChip, unreadText } from '../lib/alerts';
import { authorModel, type AuthorModel } from '../lib/author';
import type { ChipView } from '../lib/view';
import type { AlertKind, AppView, ClientStatus, Language, Menubar, MenubarWindow, Model, Peer, PeerTest, SourceView, Theme, UpdateError } from '../lib/model';
import { t } from '../lib/i18n';
import { Icon, type IconName } from '../components/icons';
import type { Action, FeedbackView, PopoverView } from '../lib/view';

const themes = (): [Theme, string][] => [
  ['auto', t('Theo macOS', 'Match macOS')],
  ['light', t('Sáng', 'Light')],
  ['dark', t('Tối', 'Dark')],
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
  /** The app's language: macOS's, Vietnamese or English. */
  onLanguage?(language: Language): void;
}

/** A card's rows, half the page wide: each row's name over its controls, as the design's block rows. */
const CARD_ROWS = 'grid [&>*]:grid-cols-[minmax(0,1fr)] [&>*]:gap-y-1.5';

/** A card of settings: its icon and name, then its rows. */
function SettingsCard(props: { title: string; icon: IconName; children: ReactNode }) {
  return (
    <Panel>
      <CardTitle>
        <Icon name={props.icon} />
        {props.title}
      </CardTitle>
      <div className={CARD_ROWS}>{props.children}</div>
    </Panel>
  );
}

/** "Ngôn ngữ": macOS's preferred language until one is picked here. */
function LanguageRow({ model, onPick }: { model: Model; onPick(language: Language): void }) {
  const options: [Language, string][] = [
    ['auto', t('Theo macOS', 'Match macOS')],
    ['vi', 'Tiếng Việt'],
    ['en', 'English'],
  ];
  return (
    <div className={rowClass()}>
      <div className={LABEL} id="qs-lang-l">
        {t('Ngôn ngữ', 'Language')}
      </div>
      <div>
        <Segmented variant="inline" className="justify-self-start" aria-labelledby="qs-lang-l">
          {options.map(([v, label]) => (
            <Segment key={v} className="px-3.5" pressed={model.language === v} onClick={() => onPick(v)}>
              {label}
            </Segment>
          ))}
        </Segmented>
        <FieldHint>{t('Theo ngôn ngữ ưu tiên của macOS cho tới khi bạn chọn.', 'Follows macOS’s preferred language until you pick one.')}</FieldHint>
      </div>
    </div>
  );
}

const clientChips = (): Record<ClientStatus, ChipView> => ({
  ok: { t: t('có, dùng được', 'present, usable'), k: 'ok' },
  missing: { t: t('chưa có', 'missing'), k: 'warn' },
  tooOpen: { t: t('cần quyền 600', 'needs mode 600'), k: 'bad' },
  invalid: { t: t('chưa đúng', 'invalid'), k: 'bad' },
  unreadable: { t: t('không đọc được', 'unreadable'), k: 'bad' },
});

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
        {t('Tên máy này', 'This Mac’s name')}
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
        <FieldHint>
          {t('Để trống để dùng tên trong macOS: ' + model.machineDefault + '.', 'Leave empty to use the macOS name: ' + model.machineDefault + '.')}
        </FieldHint>
      </div>
    </div>
  );
}

type SshTest = { kind: 'testing' } | { kind: 'ok'; hasFile: boolean } | { kind: 'fail'; error: string | null };

function testText(test: SshTest | undefined, alias: string): { tone: string; text: string } {
  if (!alias.trim()) return { tone: 'bad', text: t('Nhập tên kết nối SSH', 'Enter an SSH name') };
  if (!test) return { tone: 'muted', text: t('Chưa thử', 'Not tested') };
  if (test.kind === 'testing') return { tone: 'muted', text: t('Đang thử…', 'Testing…') };
  if (test.kind === 'ok')
    return { tone: 'ok', text: test.hasFile ? t('SSH ổn', 'SSH OK') : t('SSH ổn · máy đó chưa có file luật', 'SSH OK · that Mac has no rules file yet') };
  if (test.error === 'badAlias') return { tone: 'bad', text: peerErrors().badAlias };
  return {
    tone: 'bad',
    text:
      test.error === 'timeout' || !test.error
        ? t('SSH hết giờ chờ. Kiểm tên kết nối hoặc bật máy đó.', 'SSH timed out. Check the SSH name or turn that Mac on.')
        : t('SSH không tới được: ' + test.error, 'SSH could not connect: ' + test.error),
  };
}

const peerErrors = (): Record<string, string> => ({
  badAlias: t(
    'Tên kết nối chỉ gồm chữ, số và . - _ @, không bắt đầu bằng dấu gạch.',
    'An SSH name has only letters, digits and . - _ @, and does not start with a hyphen.',
  ),
  sameAlias: t('Tên kết nối này đã có ở dòng trên.', 'This SSH name is already on a row above.'),
  tooMany: t('Tối đa 8 Mac.', 'Up to 8 Macs.'),
});

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
    setError({ row: at === undefined ? -1 : (rowOf[Number(at)] ?? -1), text: peerErrors()[code] ?? t('Chưa lưu được: ' + refused, 'Could not save: ' + refused) });
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
        const res: SshTest = r.reached ? { kind: 'ok', hasFile: r.hasFile } : { kind: 'fail', error: r.error };
        setTests((cur) => ({ ...cur, [a]: res }));
      }),
    );
  };
  return (
    <div className={rowClass()}>
      <div className={LABEL}>
        {t('Các Mac khác', 'Other Macs')}
      </div>
      <div className="grid gap-1.5">
        {rows.map((r, i) => {
          const st = testText(tests[r.alias.trim()], r.alias);
          const label = r.name.trim() || t('Mac mới', 'New Mac');
          return (
            <div className="flex items-center gap-2 text-sm" key={i}>
              <Dot tone={st.tone} />
              <Input
                className="w-[150px]"
                value={r.name}
                placeholder={t('Tên Mac', 'Mac name')}
                maxLength={40}
                spellCheck={false}
                autoComplete="off"
                aria-label={t('Tên của ' + label, 'Name of ' + label)}
                onChange={(e) => set(i, 'name', e.target.value)}
                onBlur={() => void commit(rows)}
                onKeyDown={onKey}
              />
              <span className="whitespace-nowrap text-xs text-ink2">{t('tên kết nối', 'SSH name')}</span>
              <Input
                mono
                invalid={error?.row === i}
                className="w-[150px]"
                value={r.alias}
                placeholder={t('vd. mac-b', 'e.g. mac-b')}
                maxLength={64}
                spellCheck={false}
                autoComplete="off"
                aria-label={t('Tên kết nối SSH của ' + label, 'SSH name of ' + label)}
                onChange={(e) => set(i, 'alias', e.target.value)}
                onBlur={() => void commit(rows)}
                onKeyDown={onKey}
              />
              <small className="text-ink2">{st.text}</small>
              <LinkButton
                aria-label={t('Bỏ ' + label, 'Remove ' + label)}
                onClick={() => {
                  const next = rows.filter((_, j) => j !== i);
                  setRows(next);
                  void commit(next);
                }}
              >
                {t('Bỏ', 'Remove')}
              </LinkButton>
            </div>
          );
        })}
        {error ? <FieldHint tone="bad">{error.text}</FieldHint> : null}
        {!rows.length ? <FieldHint>{t('Chưa có Mac nào. Luật xoay vòng chỉ lưu trên máy này.', 'No other Macs yet. Rotation rules stay on this Mac.')}</FieldHint> : null}
        {!model.rotation.ssh ? <FieldHint tone="bad">{t('Không thấy ssh trên máy này.', 'ssh was not found on this Mac.')}</FieldHint> : null}
        <div>
          <Button disabled={rows.length >= 8} onClick={() => setRows([...rows, { name: '', alias: '' }])}>
            {t('Thêm Mac', 'Add Mac')}
          </Button>{' '}
          <Button disabled={!rows.some((r) => r.alias.trim())} onClick={() => void test()}>
            {t('Thử SSH', 'Test SSH')}
          </Button>
        </div>
      </div>
    </div>
  );
}

const noFileText = () => t('Không có file ở đường dẫn này.', 'No file at this path.');
const noFolderText = () => t('Không có thư mục ở đường dẫn này.', 'No folder at this path.');
const notSavedText = (refused: string) => t('Chưa lưu được: ' + refused, 'Could not save: ' + refused);

const firstmateRefused = (): Record<string, string> => ({
  notPoolFile: unreadText('notPoolFile'),
  notFound: noFileText(),
});

function firstmateHint(model: AuthorModel, refused: string | null): { tone: 'ok' | 'bad' | ''; text: string } {
  if (refused) return { tone: 'bad', text: firstmateRefused()[refused] ?? notSavedText(refused) };
  const f = model.firstmate;
  switch (f.status) {
    case 'notSet':
      return { tone: '', text: t('Để trống nếu máy này không có firstmate.', 'Leave empty if this Mac has no firstmate.') };
    case 'ok':
      return f.fresh
        ? { tone: 'ok', text: t('Đọc được ✓', 'Readable ✓') }
        : { tone: '', text: t('Đọc được, nhưng firstmate chưa ghi lại file trong 30 phút qua.', 'Readable, but firstmate has not written the file in the last 30 minutes.') };
    case 'missing':
      return { tone: 'bad', text: noFileText() };
    default:
      return { tone: 'bad', text: unreadText(f.status) };
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
        {t('File của firstmate', 'Firstmate file')}
      </label>
      <div>
        <Input
          id="qs-fm"
          mono
          invalid={hint.tone === 'bad'}
          value={path}
          placeholder={t('đường dẫn tới quota-pool.last', 'path to quota-pool.last')}
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
      ? noFolderText()
      : notSavedText(refused)
    : !model.codex.present
      ? t('Không thấy thư mục Codex ở ' + model.codex.home + '.', 'No Codex folder at ' + model.codex.home + '.')
      : t('Đọc được ✓ · app chỉ sửa config.toml ở đây khi bạn chuyển Codex.', 'Readable ✓ · the app only edits config.toml here when you switch Codex.');
  return (
    <div className={rowClass()}>
      <label className={LABEL} htmlFor="qs-codex">
        {t('Thư mục Codex', 'Codex folder')}
      </label>
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
        {t('Thông báo', 'Notifications')}
      </div>
      <div>
        <Chip chip={permissionChip(a.permission)} />{' '}
        {a.permission === 'denied' ? (
          <Button onClick={() => props.onAction('notify:settings')}>{t('Mở Cài đặt hệ thống', 'Open System Settings')}</Button>
        ) : null}
        {a.permission === 'unbundled' ? (
          <FieldHint>
            {t(
              'Chỉ app đã đóng gói mới gửi được thẻ macOS. Cảnh báo vẫn có ở tab Cảnh báo.',
              'Only the packaged app can send macOS notifications. Alerts still show in the Alerts tab.',
            )}
          </FieldHint>
        ) : null}
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
        {t('Mở khi đăng nhập', 'Open at login')}
      </div>
      <div>
        <Toggle on={on} label={t('Mở khi đăng nhập', 'Open at login')} disabled={busy || state === 'unavailable'} onClick={() => void toggle()} />
        {state === 'needsApproval' ? (
          <>
            {' '}
            <Chip chip={{ t: t('chờ duyệt', 'awaiting approval'), k: 'warn' }} />{' '}
            <Button label={t('Mở Cài đặt hệ thống: Mục đăng nhập', 'Open System Settings: Login Items')} onClick={onOpen}>
              {t('Mở Cài đặt hệ thống', 'Open System Settings')}
            </Button>
            <FieldHint>
              {t(
                'macOS cần bạn bật Quota Station trong Cài đặt hệ thống › Cài đặt chung › Mục đăng nhập.',
                'macOS needs you to turn on Quota Station in System Settings › General › Login Items.',
              )}
            </FieldHint>
          </>
        ) : null}
        {state === 'unavailable' ? <FieldHint>{t('Chỉ bản .app đã cài mới mở khi đăng nhập được.', 'Only the installed .app can open at login.')}</FieldHint> : null}
        {refused && refused !== 'unavailable' ? (
          <FieldHint tone="bad">{t('Chưa đổi được: ' + refused, 'Could not change: ' + refused)}</FieldHint>
        ) : null}
      </div>
    </div>
  );
}

const sourceRefused = (): Record<string, string> => ({
  missing: noFolderText(),
  notSource: t(
    'Thư mục này không phải bản clone Quota Station: cần .git và scripts/install.sh.',
    'This folder is not a Quota Station clone: it needs .git and scripts/install.sh.',
  ),
});

function sourceHint(app: SourceView, refused: string | null): { tone: 'ok' | 'bad' | ''; text: string } {
  if (refused) return { tone: 'bad', text: sourceRefused()[refused] ?? notSavedText(refused) };
  if (app.sourceStatus === 'ok') return { tone: 'ok', text: t('Có scripts/install.sh ✓', 'Has scripts/install.sh ✓') };
  if (app.sourceStatus === 'notSet')
    return { tone: '', text: t('Để trống trên Mac cài bằng gói build từ Mac khác.', 'Leave empty on a Mac installed from a build made on another Mac.') };
  return { tone: 'bad', text: sourceRefused()[app.sourceStatus] };
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
        {t('Mã nguồn', 'Source code')}
      </label>
      <div>
        <Input
          id="qs-src"
          mono
          invalid={hint.tone === 'bad'}
          value={path}
          placeholder={t('vd. ~/Code/quota-station', 'e.g. ~/Code/quota-station')}
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
      return sourceRefused()[e.reason];
    case 'pull':
      return t('git pull không chạy được: ' + e.detail, 'git pull failed: ' + e.detail);
    case 'install':
      return t('Build hoặc cài lỗi: ' + e.detail + '. Log đầy đủ: ' + log, 'Build or install failed: ' + e.detail + '. Full log: ' + log);
    case 'timeout':
      return e.step === 'pulling'
        ? t('git pull hết giờ chờ. Kiểm mạng rồi thử lại.', 'git pull timed out. Check the network and try again.')
        : t('Build quá 30 phút nên đã dừng. Log: ' + log, 'The build took over 30 minutes and was stopped. Log: ' + log);
    case 'spawn':
      return t('Không chạy được: ' + e.detail, 'Could not run: ' + e.detail);
  }
}

const updateRefused = (): Record<string, string> => ({
  busy: t('Đang cập nhật.', 'Already updating.'),
  noSource: t('Đặt thư mục mã nguồn ở trên để cập nhật.', 'Set the source folder above to update.'),
  unbundled: t('Bản đang chạy không phải .app đã cài, nên không có gì để cập nhật.', 'This copy is not the installed .app, so there is nothing to update.'),
  rulesDirty: t(
    'Luật xoay vòng còn thay đổi chưa lưu. Lưu hoặc bỏ trước khi mở lại.',
    'The rotation rules have unsaved changes. Save or discard them before restarting.',
  ),
  notInstalled: t('Chưa có bản mới để mở.', 'No new version to open yet.'),
});

/** The version this copy is. */
function Version({ app }: { app: AppView }) {
  return (
    <div className={rowClass()}>
      <div className={LABEL}>{t('Phiên bản', 'Version')}</div>
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
  if (app.updating === 'pulling') line = { tone: '', text: t('Đang lấy mã mới (git pull --ff-only)…', 'Fetching new code (git pull --ff-only)…') };
  else if (app.updating === 'installing')
    line = {
      tone: '',
      text: t('Đang build và cài. Mất vài phút; app vẫn chạy như thường.', 'Building and installing. This takes a few minutes; the app keeps running as usual.'),
    };
  else if (app.error) line = { tone: 'bad', text: errorText(app.error, app.log) };
  else if (app.outcome?.kind === 'upToDate') {
    const c = short(app.outcome.commit);
    line = { tone: 'ok', text: t('Đã là bản mới nhất (' + c + ').', 'Already up to date (' + c + ').') };
  } else if (app.outcome?.kind === 'installed') {
    const c = short(app.outcome.commit);
    line = {
      tone: 'ok',
      text: t(
        'Đã cài ' + (app.outcome.version ?? 'bản mới') + ' (' + c + '). Mở lại để dùng bản mới.',
        'Installed ' + (app.outcome.version ?? 'the new version') + ' (' + c + '). Restart to use it.',
      ),
    };
  } else if (!app.bundled) line = { tone: '', text: updateRefused().unbundled };
  else if (app.sourceStatus === 'notSet') line = { tone: '', text: updateRefused().noSource };
  const ready = app.bundled && app.sourceStatus === 'ok' && !app.updating;
  return (
    <div className={rowClass()}>
      <div className={LABEL}>{t('Cập nhật', 'Update')}</div>
      <div>
        <div>
          <Button primary disabled={!ready} onClick={() => void run(onUpdate)}>
            {app.updating ? t('Đang cập nhật…', 'Updating…') : t('Cập nhật từ mã nguồn', 'Update from source')}
          </Button>
          {app.outcome?.kind === 'installed' && !app.updating ? (
            <>
              {' '}
              <Button onClick={() => void run(onRelaunch)}>{t('Mở lại', 'Restart')}</Button>
            </>
          ) : null}
        </div>
        {line ? <FieldHint tone={line.tone}>{line.text}</FieldHint> : null}
        {refused ? (
          <FieldHint tone="bad">{updateRefused()[refused] ?? t('Chưa chạy được: ' + refused, 'Could not run: ' + refused)}</FieldHint>
        ) : null}
      </div>
    </div>
  );
}

const poolRefused = (): Record<string, string> => ({
  range: t('Mỗi ngưỡng từ 1 tới 99%.', 'Each threshold is from 1 to 99%.'),
  order: t('Ngưỡng của tài khoản khác phải lớn hơn ngưỡng cạn.', 'The threshold for other accounts must be higher than the running-out one.'),
});

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
      <div className={LABEL}>{t('Hạn mức sắp cạn', 'Quota running out')}</div>
      <div>
        <div className="flex flex-wrap items-center gap-1.5 text-sm">
          <span>{t('Tài khoản đang dùng còn ≤', 'The account in use has ≤')}</span>
          {field('qs-pool-dry', dry, setDry, t('Còn bao nhiêu phần trăm thì coi là sắp cạn', 'Percent left that counts as running out'))}
          <span>{t('% và không tài khoản nào còn ≥', '% left and no account has ≥')}</span>
          {field(
            'qs-pool-room',
            room,
            setRoom,
            t('Tài khoản khác còn bao nhiêu phần trăm thì coi là còn hạn mức', 'Percent left on another account that counts as having quota'),
          )}
          <span>%</span>
        </div>
        {refused ? (
          <FieldHint tone="bad">{poolRefused()[refused] ?? notSavedText(refused)}</FieldHint>
        ) : (
          <FieldHint>{t('Cảnh báo 1 dùng hai ngưỡng này.', 'Alert 1 uses these two thresholds.')}</FieldHint>
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
      ? noFolderText()
      : notSavedText(refused)
    : b.customFolder
      ? t('Gói mới vào thư mục này. Gói cũ vẫn ở chỗ cũ.', 'New packs go to this folder. Old packs stay where they are.')
      : t('Để trống để dùng iCloud Drive › Quota Station Agents.', 'Leave empty to use iCloud Drive › Quota Station Agents.');
  return (
    <div className={rowClass()}>
      <label className={LABEL} htmlFor="qs-backup">
        {t('Thư mục sao lưu', 'Backup folder')}
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
      <CardTitle>
        <Icon name="sliders-horizontal" />
        {t('Nâng cao', 'Advanced')}
      </CardTitle>
      <div className={CARD_ROWS}>
        <div className={rowClass()}>
          <div className={LABEL}>{t('Hiện phần nâng cao', 'Show advanced features')}</div>
          <div>
            <Toggle on={a.on} label={t('Hiện phần nâng cao', 'Show advanced features')} onClick={() => props.onAdvanced(!a.on)} />
            <FieldHint>
              {t(
                'Tự chuyển Claude theo luật xoay vòng, tạm dừng, các Mac khác, cập nhật từ mã nguồn và Firstmate.',
                'Automatic Claude switching by rotation rules, pausing, other Macs, updating from source, and Firstmate.',
              )}
            </FieldHint>
          </div>
        </div>
        {a.on ? (
          <>
            <div className={rowClass()}>
              <div className={LABEL}>Firstmate</div>
              <div>
                <Toggle on={a.firstmate} label="Firstmate" onClick={() => props.onFirstmate(!a.firstmate)} />
                <FieldHint>
                  {t('Tab Firstmate và cảnh báo khi Firstmate chờ hạn mức Claude.', 'The Firstmate tab and an alert when Firstmate is waiting for Claude quota.')}
                </FieldHint>
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
  if (menubar === 'gemini' && model.snapshot.agy.status === 'notSetUp') return t('Máy này chưa có Antigravity.', 'This Mac has no Antigravity.');
  if (menubar === 'codex') {
    if (!model.codex.present) return t('Máy này chưa có Codex.', 'This Mac has no Codex.');
    if (model.codex.current === 'chatgpt' && model.snapshot.codex.status === 'notSetUp')
      return t('Codex chưa đăng nhập ChatGPT.', 'Codex is not signed in to ChatGPT.');
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
          {t('Kiểm bản mới', 'Check for skill updates')}
        </div>
        <div>
          <Segmented variant="inline" aria-labelledby="qs-skcheck-l">
            <Segment pressed={c.daily} onClick={() => onSet(true, c.notify)}>
              {t('Mỗi ngày', 'Daily')}
            </Segment>
            <Segment pressed={!c.daily} onClick={() => onSet(false, c.notify)}>
              {t('Tắt', 'Off')}
            </Segment>
          </Segmented>
          <FieldHint>
            {t(
              'Hỏi GitHub một lần cho mỗi repo có skill đã cài. App không tự cập nhật skill.',
              'Asks GitHub once for each repo with an installed skill. The app never updates skills itself.',
            )}
          </FieldHint>
        </div>
      </div>
      <div className={rowClass()}>
        <div className={LABEL}>{t('Báo bằng thông báo macOS', 'Show a macOS notification')}</div>
        <div>
          <Toggle
            on={c.notify}
            label={t('Báo skill có bản mới bằng thông báo macOS', 'Show a macOS notification when a skill has an update')}
            onClick={() => onSet(c.daily, !c.notify)}
          />
          <FieldHint>{t('Tắt thì chỉ báo trong tab Cảnh báo.', 'When off, updates show only in the Alerts tab.')}</FieldHint>
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
        {t('Thanh menu', 'Menu bar')}
      </div>
      <div>
        <Segmented variant="menubar" aria-labelledby="qs-menubar-l">
          {MENUBARS.map(([v, name, id]) => {
            const off = menubarOff(model, v);
            return (
              <Segment key={v} pressed={model.menubar === v} disabled={!!off} title={off || undefined} onClick={() => onPick(v)}>
                <IdMark id={id} />
                {name}
              </Segment>
            );
          })}
        </Segmented>
        <FieldHint>{t('Tên và phần trăm đã dùng của tài khoản đang dùng.', 'Name and percent used of the account in use.')}</FieldHint>
        {offs.length ? (
          <FieldHint tone="off">
            {picked
              ? t(picked.replace(/\.$/, '') + ': thanh menu theo Claude.', picked.replace(/\.$/, '') + ': the menu bar follows Claude.')
              : offs.join(' ')}
          </FieldHint>
        ) : null}
      </div>
    </div>
  );
}

const menubarWindows = (): [MenubarWindow, string][] => [
  ['fiveHours', t('5 giờ', '5-hour')],
  ['week', t('Tuần', 'Week')],
  ['fableWeek', t('Fable tuần', 'Fable week')],
  ['tightest', t('Chặt nhất', 'Highest usage')],
];

/**
 * Why the menu bar cannot show a window: only Claude has Fable tuần, and the
 * title follows Claude when this Mac lacks the service picked. '' when it can.
 */
export function menubarWindowOff(model: Model, window: MenubarWindow): string {
  const follows = menubarOff(model, model.menubar) ? 'claude' : model.menubar;
  if (window !== 'fableWeek' || follows === 'claude') return '';
  const name = follows === 'gemini' ? 'Gemini' : 'Codex';
  return t(name + ' không có Fable tuần.', name + ' has no Fable week.');
}

/** Số trên thanh menu: which window's share used the menu bar shows. */
function MenubarWindowRow({ model, onPick }: { model: Model; onPick(window: MenubarWindow): void }) {
  const picked = menubarWindowOff(model, model.menubarWindow);
  return (
    <div className={rowClass()}>
      <div className={LABEL} id="qs-mbwindow-l">
        {t('Số trên thanh menu', 'Number in the menu bar')}
      </div>
      <div>
        <Segmented variant="inline" aria-labelledby="qs-mbwindow-l">
          {menubarWindows().map(([v, label]) => {
            const off = menubarWindowOff(model, v);
            return (
              <Segment key={v} pressed={model.menubarWindow === v} disabled={!!off} title={off || undefined} onClick={() => onPick(v)}>
                {label}
              </Segment>
            );
          })}
        </Segmented>
        <FieldHint>{t('Chặt nhất: hạn mức đã dùng nhiều nhất.', 'Highest usage: whichever quota is most used.')}</FieldHint>
        {picked ? (
          <FieldHint tone="off">
            {t(picked.replace(/\.$/, '') + ': thanh menu hiện 5 giờ.', picked.replace(/\.$/, '') + ': the menu bar shows the 5-hour window.')}
          </FieldHint>
        ) : null}
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
  onLanguage,
  } = props;
  const keychain = model.snapshot.keychain;
  // Nâng cao is one of the author's tools: a release build has no such card.
  const author = authorModel(model);

  return (
    <>
      <TabHead>
        <TabTitle>{t('Cài đặt', 'Settings')}</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
      </TabHead>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {/* A variable overriding a login is something to act on: its card comes first. */}
      {model.env.found.length ? <EnvCard model={model} onAction={onAction} /> : null}
      <div className="grid grid-cols-2 items-start gap-3 [@media(max-width:1000px)]:grid-cols-1">
        <div className={STACK}>
          <SettingsCard title={t('Chung', 'General')} icon="settings">
            {onLanguage ? <LanguageRow model={model} onPick={onLanguage} /> : null}
            <MachineName model={model} onSet={onSetMachine} />
            <LaunchAtLogin app={model.app} onSet={onLaunchAtLogin} onOpen={onOpenLoginItems} />
          </SettingsCard>
          <SettingsCard title={t('Hiển thị', 'Display')} icon="monitor">
            <div className={rowClass()}>
              <div className={LABEL} id="qs-theme-l">
                {t('Giao diện', 'Appearance')}
              </div>
              <Segmented variant="inline" className="justify-self-start" aria-labelledby="qs-theme-l">
                {themes().map(([v, label]) => (
                  <Segment key={v} className="px-3.5" pressed={model.theme === v} onClick={() => onTheme(v)}>
                    {label}
                  </Segment>
                ))}
              </Segmented>
            </div>
            <div className={rowClass()}>
              <div className={LABEL} id="qs-email-l">
                {t('Hiện email', 'Show emails')}
              </div>
              <div>
                <Toggle on={model.showNames} label={t('Hiện email', 'Show emails')} onClick={() => onShowNames(!model.showNames)} />
              </div>
            </div>
            <MenubarRow model={model} onPick={onMenubar} />
            <MenubarWindowRow model={model} onPick={onMenubarWindow} />
          </SettingsCard>
          <SettingsCard title={t('Thông báo', 'Notifications')} icon="bell">
            <Notifications model={model} onAction={onAction} onTest={onTestAlert} onSwitch={onAlertSwitch} />
            <PoolAlertRow model={model} onSet={onPoolAlert} />
          </SettingsCard>
          <SettingsCard title={t('Cập nhật', 'Updates')} icon="refresh-cw">
            <Version app={model.app} />
            {onSkillChecks ? <SkillChecksRows model={model} onSet={onSkillChecks} /> : null}
          </SettingsCard>
        </div>
        <div className={STACK}>
          <SettingsCard title={t('Dịch vụ và nguồn số', 'Services and data')} icon="database">
            <div className={rowClass()}>
              <div className={LABEL}>
                {t('Quyền Keychain', 'Keychain access')}
              </div>
              <div>
                {keychain === 'granted' ? (
                  <Chip chip={{ t: t('đã cấp', 'granted'), k: 'ok' }} />
                ) : keychain === 'required' ? (
                  <>
                    <Chip chip={{ t: t('chưa cấp', 'not granted'), k: 'warn' }} />{' '}
                    <Button primary disabled={model.granting} onClick={onGrant}>
                      {t('Cấp quyền một lần', 'Grant once')}
                    </Button>
                    <FieldHint>
                      {model.granting
                        ? t(
                            'macOS sẽ hỏi. Bấm "Luôn cho phép". App chỉ đọc đăng nhập của Claude Code để lấy số, không làm mới hay sửa nó.',
                            'macOS will ask. Click "Always Allow". The app only reads Claude Code’s sign-in to get the numbers; it never refreshes or changes it.',
                          )
                        : t('Đến khi cấp, dòng Claude chỉ hiện số đọc được lần trước.', 'Until you grant it, the Claude row shows only the numbers read last time.')}
                    </FieldHint>
                  </>
                ) : (
                  <Chip chip={{ t: t('chưa rõ', 'unknown'), k: '' }} />
                )}
              </div>
            </div>
            {onSetCodexHome ? <CodexHome model={model} onSet={onSetCodexHome} /> : null}
            {model.capabilities.agyAccounts && model.antigravity ? (
              <div className={rowClass()}>
                <div className={LABEL}>
                  {t('OAuth client Antigravity', 'Antigravity OAuth client')}
                </div>
                <div>
                  <Chip chip={clientChips()[model.antigravity.client]} />
                  <FieldHint>
                    <span className={CODE}>{model.antigravity.clientPath}</span>
                    {t(', quyền 600, chỉ có trên Mac này. README ghi định dạng.', ', mode 600, on this Mac only. The README describes the format.')}
                  </FieldHint>
                </div>
              </div>
            ) : null}
          </SettingsCard>
          {model.env.found.length ? null : <EnvCard model={model} onAction={onAction} />}
          {onSetBackupFolder ? (
            <SettingsCard title={t('Sao lưu', 'Backup')} icon="layers">
              <BackupFolder model={model} onSet={onSetBackupFolder} />
            </SettingsCard>
          ) : null}
          {author ? <Advanced {...props} model={author} /> : null}
        </div>
      </div>
    </>
  );
}
