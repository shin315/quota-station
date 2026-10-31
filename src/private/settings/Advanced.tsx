// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cài đặt › Nâng cao (the author's tools): its switch, and while it is on,
// Firstmate and its state file, the other Macs the rotation file is copied
// to (an SSH alias each), and the clone "Cập nhật từ mã nguồn" builds from.
// Windows has no other Macs and no update from source: their rows stay out.

import { useState } from 'react';
import { Input } from '@/components/ui/input';
import { Button, Dot, FieldHint, LinkButton, More, Toggle } from '@/components/parts';
import { unreadText } from '@/lib/alerts';
import { macTools, sourceView, type AuthorModel } from '@/private/author';
import type { Peer, PeerTest, SourceView, UpdateError } from '@/lib/model';
import { t } from '@/lib/i18n';
import { SET_CTL, SetRow } from '@/settings/SetRow';
import { noFileText, noFolderText, notSavedText, SettingsCard, useField, type SettingsProps } from '@/settings/Settings';

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
        ? t('SSH hết thời gian chờ. Kiểm tra tên kết nối hoặc bật máy đó.', 'SSH timed out. Check the SSH name or turn that Mac on.')
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
        ? t('git pull hết thời gian chờ. Kiểm tra mạng rồi thử lại.', 'git pull timed out. Check the network and try again.')
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

/**
 * Nâng cao, the last card: what was made for the author's own Macs. Off, its
 * tabs (Luật xoay vòng, Firstmate, Chép sang Mac khác), the automatic switch
 * and the pause, and alerts 2 and 5 are hidden; the rows below come with it.
 */
export function Advanced(props: SettingsProps & { model: AuthorModel; id: string }) {
  const { model } = props;
  const a = model.advanced;
  const mac = macTools(model);
  const source = mac ? sourceView(model.app) : null;
  return (
    <SettingsCard id={props.id} title={t('Nâng cao', 'Advanced')} icon="advanced">
      <SetRow
        label={t('Hiện phần nâng cao', 'Show advanced features')}
        inline
        hint={
          <span>
            {mac
              ? t('Luật, tạm dừng, các Mac khác và Firstmate.', 'Rules, pausing, other Macs and Firstmate.')
              : t('Luật, tạm dừng và Firstmate.', 'Rules, pausing and Firstmate.')}{' '}
            <More small about={t('Hiện phần nâng cao', 'Show advanced features')}>
              <p>
                {mac
                  ? t(
                      'Tự chuyển Claude theo luật xoay vòng, tạm dừng, các Mac khác, cập nhật từ mã nguồn và Firstmate.',
                      'Automatic Claude switching by rotation rules, pausing, other Macs, updating from source, and Firstmate.',
                    )
                  : t('Tự chuyển Claude theo luật xoay vòng, tạm dừng và Firstmate.', 'Automatic Claude switching by rotation rules, pausing, and Firstmate.')}
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
          {mac ? <OtherMacs model={model} onSet={props.onSetPeers} onTest={props.onTestPeer} /> : null}
          {source ? <SourceDir app={source} onSet={props.onSetSource} /> : null}
          {source ? <UpdateFromSource app={source} onUpdate={props.onUpdate} onRelaunch={props.onRelaunch} /> : null}
        </>
      ) : null}
    </SettingsCard>
  );
}
