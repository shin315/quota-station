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
// The layout is settings-R2 (25/10/2026) with its groups as tabs: a bar of
// tabs under the title that stays while the page scrolls, and under it the
// one group picked (Chung at first), its cards the page's whole width; in
// each card a row's name and one line on the left, its control on the right
// (SetRow). A longer description waits behind its "Chi tiết". Five tabs
// (26/10/2026): Chung (with the backup folder, and the Cập nhật card: new
// versions and Chẩn đoán), Hiển thị, Thông báo, Dịch vụ (with the
// variables' card) and Nâng cao.

import { Fragment, useEffect, useRef, useState, type ReactNode } from 'react';
import { Tabs as TabsPrimitive } from 'radix-ui';
import { DiagnosticsRows, ReleaseRows } from '../release/ReleaseParts';
import { EnvCard } from './EnvCard';
import { ThemePicker } from './ThemePicker';
import { SET_CTL, SetRow } from './SetRow';
import { Card as Panel, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Segment, Segmented, segmentedVariants, segmentVariants } from '@/components/ui/segmented';
import { IdMark } from '../components/accounts';
import { LABEL, rowClass, STACK, TabHead, TabSub, TabTitle } from '../components/page';
import { Button, Chip, CODE, Dot, Feedback, FieldHint, LinkButton, More, Toggle } from '../components/parts';
import { alertKinds, permissionChip, unreadText } from '../lib/alerts';
import { authorModel, type AuthorModel } from '../lib/author';
import type { ChipView } from '../lib/view';
import type { AlertKind, AppView, ClientStatus, Language, Menubar, MenubarWindow, Model, Peer, PeerTest, SettingsGroup, SourceView, Theme, ThemePack, UpdateError } from '../lib/model';
import { t } from '../lib/i18n';
import { SectionGlyph, type Section } from '../components/icons';
import type { Action, FeedbackView, PopoverView } from '../lib/view';
import { cn } from '@/lib/utils';

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
  /** The theme pack of every window. */
  onThemePack?(pack: ThemePack): void;
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
  /** Cài đặt › Cập nhật: "Tự kiểm bản mới của app"; with it come the release rows and Chẩn đoán. */
  onReleaseChecks?(on: boolean): void;
  /** The group shown, kept by the window while it is open; Chung when it is not one this build has. */
  group?: SettingsGroup;
  onGroup?(group: SettingsGroup): void;
}

/** A card's rows: each row's name on the left, its controls on the right. */
const CARD_ROWS = 'grid';

/** A card of settings: its icon and name, then its rows; a region named by its title. */
function SettingsCard(props: { id: string; title: string; icon: Section; children: ReactNode }) {
  return (
    <Panel id={props.id} role="region" aria-labelledby={props.id + '-t'}>
      <CardTitle id={props.id + '-t'}>
        <SectionGlyph section={props.icon} />
        {props.title}
      </CardTitle>
      <div className={CARD_ROWS}>{props.children}</div>
    </Panel>
  );
}

/** Where a row of the first run (`form`, the form's rowClass) and of Cài đặt (`set`, SetRow) differ. */
type Layout = 'form' | 'set';

/** "Ngôn ngữ": macOS's preferred language until one is picked here. */
function LanguageRow({ model, onPick }: { model: Model; onPick(language: Language): void }) {
  const options: [Language, string][] = [
    ['auto', t('Theo macOS', 'Match macOS')],
    ['vi', 'Tiếng Việt'],
    ['en', 'English'],
  ];
  return (
    <SetRow label={t('Ngôn ngữ', 'Language')} labelId="qs-lang-l">
      <Segmented className={SET_CTL} aria-labelledby="qs-lang-l">
        {options.map(([v, label]) => (
          <Segment key={v} pressed={model.language === v} onClick={() => onPick(v)}>
            {label}
          </Segment>
        ))}
      </Segmented>
    </SetRow>
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
    <SetRow label={t('Tên máy này', 'This Mac’s name')} htmlFor="qs-name">
      <Input
        id="qs-name"
        className={SET_CTL}
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
    </SetRow>
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
    <SetRow label={t('Các Mac khác', 'Other Macs')} block>
      <div className="grid gap-1.5">
        {rows.map((r, i) => {
          const st = testText(tests[r.alias.trim()], r.alias);
          const label = r.name.trim() || t('Mac mới', 'New Mac');
          return (
            // What does not fit (the test's result, Bỏ) goes on the next line instead of past the card.
            <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm" key={i}>
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
              <small className="whitespace-nowrap text-ink2">{st.text}</small>
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
    </SetRow>
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
  // A plain line says what to put here; what the file said back goes under the field.
  return (
    <SetRow
      label={t('File của firstmate', 'Firstmate file')}
      htmlFor="qs-fm"
      hint={hint.tone === '' ? hint.text : null}
      under={hint.tone !== '' && hint.text ? <FieldHint tone={hint.tone}>{hint.text}</FieldHint> : null}
    >
      <Input
        id="qs-fm"
        className={SET_CTL}
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
    </SetRow>
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
      : t('Đọc được ✓ · app chỉ sửa config.toml và dời auth.json ở đây khi bạn chuyển Codex.', 'Readable ✓ · the app only edits config.toml and moves auth.json here when you switch Codex.');
  return (
    <SetRow
      label={t('Thư mục Codex', 'Codex folder')}
      htmlFor="qs-codex"
      under={<FieldHint tone={bad ? 'bad' : model.codex.present ? 'ok' : ''}>{hint}</FieldHint>}
    >
      <Input
        id="qs-codex"
        className={SET_CTL}
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
    </SetRow>
  );
}

/** Where the Codex accounts the app keeps are, and the way to the tab that manages them. */
function CodexAccountsRow({ model, onAction }: { model: Model; onAction: SettingsProps['onAction'] }) {
  const all = model.codex.accounts;
  const gpts = all.filter((a) => a.kind === 'chatgpt').length;
  return (
    <SetRow
      label={t('Tài khoản Codex', 'Codex accounts')}
      inline
      hint={
        <span>
          {t(gpts + ' ChatGPT · ' + (all.length - gpts) + ' Custom API. Đăng nhập ChatGPT đang chờ nằm trong thư mục của app, chỉ bạn đọc được.', gpts + ' ChatGPT · ' + (all.length - gpts) + ' Custom API. ChatGPT sign-ins not in use stay in the app’s folder, readable only by you.')}{' '}
          <More small about={t('Tài khoản Codex', 'Codex accounts')}>
            <p>
              <span className={CODE}>{model.codex.accountsDir}</span>
              {t(
                ': mỗi tài khoản ChatGPT một thư mục (quyền 700, file 600). Tài khoản đang đăng nhập nằm ở ' + model.codex.home + '/auth.json. Khoá Custom API nằm trong config.toml của Codex.',
                ': one folder per ChatGPT account (mode 700, files 600). The account signed in is at ' + model.codex.home + '/auth.json. Custom API keys are in Codex’s config.toml.',
              )}
            </p>
          </More>
        </span>
      }
    >
      <Button onClick={() => onAction('codex:tab')}>{t('Mở tab Codex', 'Open the Codex tab')}</Button>
    </SetRow>
  );
}

/**
 * A kind of alert's line, one line long: the rule in short, the whole of it
 * behind "Chi tiết". Kinds whose rule is already one line say it as it is.
 */
function kindHint(model: Model, kind: { kind: string; title: string; text: string }): ReactNode {
  const p = model.alerts.poolAlert;
  const short =
    kind.kind === 'pool'
      ? model.advanced.on
        ? t('Đang dùng còn ≤ ' + p.dryAtMost + '%, mọi tài khoản trong vòng còn < ' + p.roomAtLeast + '%.', 'In use ≤ ' + p.dryAtMost + '% left, every account in rotation < ' + p.roomAtLeast + '% left.')
        : t('Đang dùng còn ≤ ' + p.dryAtMost + '%, mọi tài khoản còn < ' + p.roomAtLeast + '%.', 'In use ≤ ' + p.dryAtMost + '% left, every account < ' + p.roomAtLeast + '% left.')
      : kind.kind === 'login'
        ? t('Sắp hết hạn hoặc lỗi đăng nhập.', 'Sign-in about to expire or failing.')
        : null;
  if (!short) return kind.text;
  return (
    <span>
      {short}{' '}
      <More small about={kind.title}>
        <p>{kind.text}</p>
      </More>
    </span>
  );
}

/**
 * Thông báo: whether macOS shows the cards (its own permission, apart from
 * the kinds), then each kind with "Gửi thử" and its switch; alert 1's
 * thresholds right under it.
 */
function NotifyRows(props: {
  model: Model;
  onAction: SettingsProps['onAction'];
  onTest: SettingsProps['onTestAlert'];
  onSwitch: SettingsProps['onAlertSwitch'];
  onPool: SettingsProps['onPoolAlert'];
}) {
  const a = props.model.alerts;
  return (
    <>
      <SetRow
        label={t('Thông báo macOS', 'macOS notifications')}
        inline
        hint={
          a.permission === 'unbundled'
            ? t('Chỉ app đã đóng gói mới gửi được thẻ macOS. Thông báo vẫn có ở chuông trên đầu trang.', 'Only the packaged app can send macOS notifications. Notifications still show under the bell at the top of the page.')
            : null
        }
      >
        {a.permission === 'denied' ? (
          <Button onClick={() => props.onAction('notify:settings')}>{t('Mở Cài đặt hệ thống', 'Open System Settings')}</Button>
        ) : null}
        <Chip chip={permissionChip(a.permission)} />
      </SetRow>
      {alertKinds(props.model).map((k) => (
        <Fragment key={k.kind}>
          <SetRow label={k.title} hint={kindHint(props.model, k)} inline>
            <Button disabled={a.permission === 'denied'} label={t('Gửi thử: ', 'Send a test: ') + k.title} onClick={() => props.onTest(k.kind)}>
              {t('Gửi thử', 'Send a test')}
            </Button>
            <Toggle on={a.switches[k.kind]} label={k.title} onClick={() => props.onSwitch(k.kind, !a.switches[k.kind])} />
          </SetRow>
          {k.kind === 'pool' ? <PoolAlertRow model={props.model} onSet={props.onPool} /> : null}
        </Fragment>
      ))}
    </>
  );
}

/** "Mở khi đăng nhập": macOS's own login item, off until turned on here. */
export function LaunchAtLogin({ app, onSet, onOpen, layout = 'form' }: { app: AppView; onSet: SettingsProps['onLaunchAtLogin']; onOpen(): void; layout?: Layout }) {
  const [refused, setRefused] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const state = app.launchAtLogin;
  const on = state === 'on' || state === 'needsApproval';
  const toggle = async () => {
    setBusy(true);
    setRefused(await onSet(!on));
    setBusy(false);
  };
  const failed =
    refused && refused !== 'unavailable' ? <FieldHint tone="bad">{t('Chưa đổi được: ' + refused, 'Could not change: ' + refused)}</FieldHint> : null;
  if (layout === 'set')
    return (
      <SetRow
        label={t('Mở khi đăng nhập', 'Open at login')}
        inline
        hint={
          state === 'needsApproval'
            ? t('macOS cần bạn bật Quota Station trong Cài đặt hệ thống › Cài đặt chung › Mục đăng nhập.', 'macOS needs you to turn on Quota Station in System Settings › General › Login Items.')
            : state === 'unavailable'
              ? t('Chỉ bản .app đã cài mới mở khi đăng nhập được.', 'Only the installed .app can open at login.')
              : null
        }
        under={failed}
      >
        {state === 'needsApproval' ? (
          <>
            <Chip chip={{ t: t('chờ duyệt', 'awaiting approval'), k: 'warn' }} />
            <Button label={t('Mở Cài đặt hệ thống: Mục đăng nhập', 'Open System Settings: Login Items')} onClick={onOpen}>
              {t('Mở Cài đặt hệ thống', 'Open System Settings')}
            </Button>
          </>
        ) : null}
        <Toggle on={on} label={t('Mở khi đăng nhập', 'Open at login')} disabled={busy || state === 'unavailable'} onClick={() => void toggle()} />
      </SetRow>
    );
  return (
    <div className={rowClass(false, true)}>
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
        {failed}
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
    <SetRow
      label={t('Mã nguồn', 'Source code')}
      htmlFor="qs-src"
      hint={hint.tone === '' ? hint.text : null}
      under={hint.tone !== '' && hint.text ? <FieldHint tone={hint.tone}>{hint.text}</FieldHint> : null}
    >
      <Input
        id="qs-src"
        className={SET_CTL}
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
    </SetRow>
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
    <SetRow label={t('Phiên bản', 'Version')} inline>
      <span className={CODE}>
        {app.version}
        {app.commit ? ' · ' + app.commit : ''}
      </span>
    </SetRow>
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
  // What the run says goes under the row's name: it can be a long line (a log's path).
  return (
    <SetRow
      label={t('Cập nhật', 'Update')}
      inline
      hint={
        line || refused ? (
          <>
            {line ? <FieldHint tone={line.tone}>{line.text}</FieldHint> : null}
            {refused ? <FieldHint tone="bad">{updateRefused()[refused] ?? t('Chưa chạy được: ' + refused, 'Could not run: ' + refused)}</FieldHint> : null}
          </>
        ) : null
      }
    >
      {app.outcome?.kind === 'installed' && !app.updating ? <Button onClick={() => void run(onRelaunch)}>{t('Mở lại', 'Restart')}</Button> : null}
      <Button primary disabled={!ready} onClick={() => void run(onUpdate)}>
        {app.updating ? t('Đang cập nhật…', 'Updating…') : t('Cập nhật từ mã nguồn', 'Update from source')}
      </Button>
    </SetRow>
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
    <SetRow
      label={t('Ngưỡng sắp cạn', 'Running-out thresholds')}
      under={refused ? <FieldHint tone="bad">{poolRefused()[refused] ?? notSavedText(refused)}</FieldHint> : null}
    >
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
    </SetRow>
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
  const note = b.customFolder ? t('Gói mới vào thư mục này. Gói cũ vẫn ở chỗ cũ.', 'New packs go to this folder. Old packs stay where they are.') : null;
  return (
    <SetRow
      label={t('Thư mục sao lưu', 'Backup folder')}
      htmlFor="qs-backup"
      hint={note}
      under={refused ? <FieldHint tone="bad">{refused === 'notFound' ? noFolderText() : notSavedText(refused)}</FieldHint> : null}
    >
      <Input
        id="qs-backup"
        className={SET_CTL}
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
    </SetRow>
  );
}

/**
 * Nâng cao, the last card: what was made for the author's own Macs. Off, its
 * tabs (Luật xoay vòng, Firstmate, Chép sang Mac khác), the automatic switch
 * and the pause, and alerts 2 and 5 are hidden; the rows below come with it.
 */
function Advanced(props: SettingsProps & { model: AuthorModel; id: string }) {
  const { model } = props;
  const a = model.advanced;
  return (
    <SettingsCard id={props.id} title={t('Nâng cao', 'Advanced')} icon="advanced">
      <SetRow
        label={t('Hiện phần nâng cao', 'Show advanced features')}
        inline
        hint={
          <span>
            {t('Luật, tạm dừng, các Mac khác và Firstmate.', 'Rules, pausing, other Macs and Firstmate.')}{' '}
            <More small about={t('Hiện phần nâng cao', 'Show advanced features')}>
              <p>
                {t(
                  'Tự chuyển Claude theo luật xoay vòng, tạm dừng, các Mac khác, cập nhật từ mã nguồn và Firstmate.',
                  'Automatic Claude switching by rotation rules, pausing, other Macs, updating from source, and Firstmate.',
                )}
              </p>
            </More>
          </span>
        }
      >
        <Toggle on={a.on} label={t('Hiện phần nâng cao', 'Show advanced features')} onClick={() => props.onAdvanced(!a.on)} />
      </SetRow>
      {a.on ? (
        <>
          <SetRow
            label="Firstmate"
            inline
            hint={t('Tab Firstmate và thông báo khi Firstmate chờ hạn mức Claude.', 'The Firstmate tab and a notification when Firstmate is waiting for Claude quota.')}
          >
            <Toggle on={a.firstmate} label="Firstmate" onClick={() => props.onFirstmate(!a.firstmate)} />
          </SetRow>
          {a.firstmate ? <FirstmateFile model={model} onSet={props.onSetFirstmate} /> : null}
          <OtherMacs model={model} onSet={props.onSetPeers} onTest={props.onTestPeer} />
          <SourceDir app={model.app} onSet={props.onSetSource} />
          <UpdateFromSource app={model.app} onUpdate={props.onUpdate} onRelaunch={props.onRelaunch} />
        </>
      ) : null}
    </SettingsCard>
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
  const label = t('Kiểm bản mới của skill', 'Check for skill updates');
  return (
    <>
      <SetRow
        label={label}
        labelId="qs-skcheck-l"
        hint={
          <span>
            {t('Chỉ kiểm tra, không tự cập nhật skill.', 'Only checks; never updates a skill itself.')}{' '}
            <More small about={label}>
              <p>
                {t(
                  'Hỏi GitHub một lần cho mỗi repo có skill đã cài. App không tự cập nhật skill.',
                  'Asks GitHub once for each repo with an installed skill. The app never updates skills itself.',
                )}
              </p>
            </More>
          </span>
        }
      >
        <Segmented variant="inline" aria-labelledby="qs-skcheck-l">
          <Segment className="px-3.5" pressed={c.daily} onClick={() => onSet(true, c.notify)}>
            {t('Mỗi ngày', 'Daily')}
          </Segment>
          <Segment className="px-3.5" pressed={!c.daily} onClick={() => onSet(false, c.notify)}>
            {t('Tắt', 'Off')}
          </Segment>
        </Segmented>
      </SetRow>
      <SetRow
        label={t('Báo skill mới bằng thông báo macOS', 'Show skill updates as macOS notifications')}
        inline
        hint={t('Tắt thì chỉ báo ở chuông Thông báo.', 'When off, updates show only under the Notifications bell.')}
      >
        <Toggle
          on={c.notify}
          label={t('Báo skill có bản mới bằng thông báo macOS', 'Show a macOS notification when a skill has an update')}
          onClick={() => onSet(c.daily, !c.notify)}
        />
      </SetRow>
    </>
  );
}

export function MenubarRow({ model, onPick, layout = 'form' }: { model: Model; onPick(menubar: Menubar): void; layout?: Layout }) {
  const offs = MENUBARS.map(([v]) => menubarOff(model, v)).filter(Boolean);
  const picked = menubarOff(model, model.menubar);
  const set = layout === 'set';
  const control = (
    <Segmented variant="menubar" className={set ? SET_CTL : undefined} aria-labelledby="qs-menubar-l">
      {MENUBARS.map(([v, name, id]) => {
        const off = menubarOff(model, v);
        return (
          <Segment
            key={v}
            className={set ? 'flex-1 justify-center px-2' : undefined}
            pressed={model.menubar === v}
            disabled={!!off}
            title={off || undefined}
            onClick={() => onPick(v)}
          >
            <IdMark id={id} />
            {name}
          </Segment>
        );
      })}
    </Segmented>
  );
  const off = offs.length ? (
    <FieldHint tone="off">
      {picked
        ? t(picked.replace(/\.$/, '') + ': thanh menu theo Claude.', picked.replace(/\.$/, '') + ': the menu bar follows Claude.')
        : offs.join(' ')}
    </FieldHint>
  ) : null;
  if (set)
    return (
      <SetRow label={t('Dịch vụ trên thanh menu', 'Service in the menu bar')} labelId="qs-menubar-l" hint={off}>
        {control}
      </SetRow>
    );
  return (
    <div className={rowClass()}>
      <div className={LABEL} id="qs-menubar-l">
        {t('Thanh menu', 'Menu bar')}
      </div>
      <div>
        {control}
        <FieldHint>{t('Tên và phần trăm đã dùng của tài khoản đang dùng.', 'Name and percent used of the account in use.')}</FieldHint>
        {off}
      </div>
    </div>
  );
}

const menubarWindows = (): [MenubarWindow, string][] => [
  ['fiveHours', t('5 giờ', '5-hour')],
  ['week', t('Tuần', 'Week')],
  ['fableWeek', t('Fable tuần', 'Fable week')],
  ['tightest', t('Cao nhất', 'Highest')],
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
export function MenubarWindowRow({ model, onPick, layout = 'form' }: { model: Model; onPick(window: MenubarWindow): void; layout?: Layout }) {
  const picked = menubarWindowOff(model, model.menubarWindow);
  const set = layout === 'set';
  const control = (
    <Segmented variant={set ? 'fill' : 'inline'} className={set ? SET_CTL : undefined} aria-labelledby="qs-mbwindow-l">
      {menubarWindows().map(([v, label]) => {
        const off = menubarWindowOff(model, v);
        return (
          <Segment key={v} pressed={model.menubarWindow === v} disabled={!!off} title={off || undefined} onClick={() => onPick(v)}>
            {label}
          </Segment>
        );
      })}
    </Segmented>
  );
  const off = picked ? (
    <FieldHint tone="off">
      {t(picked.replace(/\.$/, '') + ': thanh menu hiện 5 giờ.', picked.replace(/\.$/, '') + ': the menu bar shows the 5-hour window.')}
    </FieldHint>
  ) : null;
  if (set)
    return (
      <SetRow label={t('Số trên thanh menu', 'Number in the menu bar')} labelId="qs-mbwindow-l" hint={off}>
        {control}
      </SetRow>
    );
  return (
    <div className={rowClass()}>
      <div className={LABEL} id="qs-mbwindow-l">
        {t('Số trên thanh menu', 'Number in the menu bar')}
      </div>
      <div>
        {control}
        <FieldHint>{t('Cao nhất: hạn mức đã dùng nhiều nhất.', 'Highest: whichever quota is most used.')}</FieldHint>
        {off}
      </div>
    </div>
  );
}

/** Each card's id. */
const CARDS = {
  general: 'qs-set-general',
  updates: 'qs-set-updates',
  display: 'qs-set-display',
  notify: 'qs-set-notify',
  services: 'qs-set-services',
  env: 'qs-set-env',
  advanced: 'qs-set-advanced',
};

/** How far the bar fades out at an end with more tabs past it. */
const FADE = 24;

/**
 * The bar of tabs under the title, one per group, the shown one lit as the
 * segmented controls light theirs. Where the page is too narrow for all of
 * them it scrolls sideways, an end with more past it faded out; it never
 * wraps or cuts a name short. It stays at the page's top while the page
 * scrolls, frosted once the cards pass under it. The arrows, Home and End
 * move between the tabs (Radix Tabs).
 */
function GroupBar(props: { groups: { key: SettingsGroup; title: string }[]; current: SettingsGroup }) {
  const bar = useRef<HTMLDivElement>(null);
  const strip = useRef<HTMLDivElement>(null);
  const [stuck, setStuck] = useState(false);
  const [fade, setFade] = useState({ start: 0, end: 0 });
  useEffect(() => {
    const page = bar.current?.closest('main');
    const el = strip.current;
    if (!page || !el) return;
    const read = () => {
      setStuck(bar.current!.getBoundingClientRect().top <= page.getBoundingClientRect().top + 0.5 && page.scrollTop > 0);
      const room = el.scrollWidth - el.clientWidth;
      setFade({ start: Math.min(FADE, el.scrollLeft), end: Math.min(FADE, Math.max(0, room - el.scrollLeft)) });
    };
    read();
    page.addEventListener('scroll', read, { passive: true });
    el.addEventListener('scroll', read, { passive: true });
    const sized = typeof ResizeObserver === 'function' ? new ResizeObserver(read) : null;
    sized?.observe(el);
    sized?.observe(page);
    return () => {
      page.removeEventListener('scroll', read);
      el.removeEventListener('scroll', read);
      sized?.disconnect();
    };
  }, []);
  // Another group starts at its top, and its tab in sight: one opened from elsewhere may be past the bar's end.
  useEffect(() => {
    const page = bar.current?.closest('main');
    if (page) page.scrollTop = 0;
    const el = strip.current;
    const tab = el?.querySelector<HTMLElement>('[data-state=active]');
    if (!el || !tab) return;
    const left = tab.offsetLeft - FADE;
    const right = tab.offsetLeft + tab.offsetWidth + FADE - el.clientWidth;
    if (el.scrollLeft > left) el.scrollLeft = left;
    else if (el.scrollLeft < right) el.scrollLeft = right;
  }, [props.current]);
  return (
    // Edge to edge over the page's side padding, as the window's foot is; over the drag strip at the window's top.
    <div
      ref={bar}
      data-tauri-drag-region
      className={cn(
        'sticky -top-4 z-[11] -mt-2.5 -mr-[22px] -mb-2 -ml-3.5 border-b-[.5px] pt-2.5 pr-[22px] pb-2 pl-3.5',
        stuck ? 'border-line bg-glass-win backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-sat)]' : 'border-transparent',
      )}
    >
      <div
        ref={strip}
        className="-m-1 overflow-x-auto overscroll-x-contain p-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
        style={{
          maskImage: `linear-gradient(to right, transparent, #000 ${fade.start}px, #000 calc(100% - ${fade.end}px), transparent)`,
        }}
      >
        <TabsPrimitive.List className={segmentedVariants({ variant: 'tab' })} aria-label={t('Các mục cài đặt', 'Settings sections')}>
          {props.groups.map((g) => (
            <TabsPrimitive.Trigger key={g.key} value={g.key} className={segmentVariants({ variant: 'tab' })}>
              {g.title}
            </TabsPrimitive.Trigger>
          ))}
        </TabsPrimitive.List>
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
  onThemePack,
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
  onReleaseChecks,
  } = props;
  const keychain = model.snapshot.keychain;
  // Nâng cao is one of the author's tools: a release build has no such card.
  const author = authorModel(model);
  const env = <EnvCard id={CARDS.env} model={model} onAction={onAction} />;
  // A variable overriding a login is something to act on: while one does, its card leads Dịch vụ and Dịch vụ leads the bar.
  const envFound = model.env.found.length > 0;
  const services = {
    key: 'services' as const,
    title: t('Dịch vụ', 'Services'),
    card: (
      <>
        {envFound ? env : null}
        <SettingsCard id={CARDS.services} title={t('Dịch vụ', 'Services')} icon="services">
          <SetRow
            label={t('Quyền Keychain', 'Keychain access')}
            inline
            hint={
              keychain === 'required'
                ? model.granting
                  ? t(
                      'macOS sẽ hỏi. Bấm "Luôn cho phép". App chỉ đọc đăng nhập của Claude Code để lấy số, không làm mới hay sửa nó.',
                      'macOS will ask. Click "Always Allow". The app only reads Claude Code’s sign-in to get the numbers; it never refreshes or changes it.',
                    )
                  : t('Đến khi cấp, dòng Claude chỉ hiện số đọc được lần trước.', 'Until you grant it, the Claude row shows only the numbers read last time.')
                : null
            }
          >
            {keychain === 'granted' ? (
              <Chip chip={{ t: t('đã cấp', 'granted'), k: 'ok' }} />
            ) : keychain === 'required' ? (
              <>
                <Chip chip={{ t: t('chưa cấp', 'not granted'), k: 'warn' }} />
                <Button primary disabled={model.granting} onClick={onGrant}>
                  {t('Cấp quyền một lần', 'Grant once')}
                </Button>
              </>
            ) : (
              <Chip chip={{ t: t('chưa rõ', 'unknown'), k: '' }} />
            )}
          </SetRow>
          {onSetCodexHome ? <CodexHome model={model} onSet={onSetCodexHome} /> : null}
          {model.codex.present ? <CodexAccountsRow model={model} onAction={onAction} /> : null}
          {model.capabilities.agyAccounts && model.antigravity ? (
            <SetRow
              label={t('OAuth client Antigravity', 'Antigravity OAuth client')}
              inline
              hint={
                <span>
                  {t('Quyền 600, chỉ có trên Mac này.', 'Mode 600, on this Mac only.')}{' '}
                  <More small about={t('OAuth client Antigravity', 'Antigravity OAuth client')}>
                    <p>
                      <span className={CODE}>{model.antigravity.clientPath}</span>
                      {t(', quyền 600, chỉ có trên Mac này. README ghi định dạng.', ', mode 600, on this Mac only. The README describes the format.')}
                    </p>
                  </More>
                </span>
              }
            >
              <Chip chip={clientChips()[model.antigravity.client]} />
            </SetRow>
          ) : null}
        </SettingsCard>
        {envFound ? null : env}
      </>
    ),
  };

  // The tabs in their order, each with its cards.
  const groups: { key: SettingsGroup; title: string; card: ReactNode }[] = [
    ...(envFound ? [services] : []),
    {
      key: 'general',
      title: t('Chung', 'General'),
      card: (
        <>
          <SettingsCard id={CARDS.general} title={t('Chung', 'General')} icon="general">
            {onLanguage ? <LanguageRow model={model} onPick={onLanguage} /> : null}
            <MachineName model={model} onSet={onSetMachine} />
            <LaunchAtLogin layout="set" app={model.app} onSet={onLaunchAtLogin} onOpen={onOpenLoginItems} />
            {onSetBackupFolder ? <BackupFolder model={model} onSet={onSetBackupFolder} /> : null}
          </SettingsCard>
          <SettingsCard id={CARDS.updates} title={t('Cập nhật', 'Updates')} icon="updates">
            {onReleaseChecks ? <ReleaseRows model={model} onAction={onAction} onAuto={onReleaseChecks} /> : <Version app={model.app} />}
            {onSkillChecks ? <SkillChecksRows model={model} onSet={onSkillChecks} /> : null}
            {onReleaseChecks ? <DiagnosticsRows onAction={onAction} /> : null}
          </SettingsCard>
        </>
      ),
    },
    {
      key: 'display',
      title: t('Hiển thị', 'Display'),
      card: (
        <SettingsCard id={CARDS.display} title={t('Hiển thị', 'Display')} icon="display">
          <SetRow label={t('Giao diện', 'Appearance')} labelId="qs-theme-l">
            <Segmented className={SET_CTL} aria-labelledby="qs-theme-l">
              {themes().map(([v, label]) => (
                <Segment key={v} pressed={model.theme === v} onClick={() => onTheme(v)}>
                  {label}
                </Segment>
              ))}
            </Segmented>
          </SetRow>
          {onThemePack ? <ThemePicker value={model.themePack} theme={model.theme} onPick={onThemePack} /> : null}
          <SetRow label={t('Hiện email', 'Show emails')} labelId="qs-email-l" inline>
            <Toggle on={model.showNames} label={t('Hiện email', 'Show emails')} onClick={() => onShowNames(!model.showNames)} />
          </SetRow>
          <MenubarRow layout="set" model={model} onPick={onMenubar} />
          <MenubarWindowRow layout="set" model={model} onPick={onMenubarWindow} />
        </SettingsCard>
      ),
    },
    {
      key: 'notify',
      title: t('Thông báo', 'Notifications'),
      card: (
        <SettingsCard id={CARDS.notify} title={t('Thông báo', 'Notifications')} icon="notify">
          <NotifyRows model={model} onAction={onAction} onTest={onTestAlert} onSwitch={onAlertSwitch} onPool={onPoolAlert} />
        </SettingsCard>
      ),
    },
    ...(envFound ? [] : [services]),
    ...(author ? [{ key: 'advanced' as const, title: t('Nâng cao', 'Advanced'), card: <Advanced {...props} model={author} id={CARDS.advanced} /> }] : []),
  ];
  // Chung unless another tab was picked, or asked for by a link from elsewhere.
  const [own, setOwn] = useState<SettingsGroup>('general');
  const asked = props.group ?? own;
  const current = groups.some((g) => g.key === asked) ? asked : 'general';
  const pick = (key: string) => {
    setOwn(key as SettingsGroup);
    props.onGroup?.(key as SettingsGroup);
  };

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
      <TabsPrimitive.Root value={current} onValueChange={pick} className="contents">
        <GroupBar groups={groups} current={current} />
        {groups.map((g) => (
          // Radix keeps an empty panel for each tab not shown: it takes no room in the page's gaps.
          <TabsPrimitive.Content key={g.key} value={g.key} className={cn(STACK, 'data-[state=inactive]:hidden')}>
            {g.card}
          </TabsPrimitive.Content>
        ))}
      </TabsPrimitive.Root>
    </>
  );
}
