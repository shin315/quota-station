// The Cài đặt tab of the main window: where quota-axi is, Keychain access
// for quota-axi, this Mac's name, the other Macs the rotation file is
// copied to (an SSH alias each), the Antigravity OAuth client file,
// whether account names show, the light or dark look, Firstmate's state
// file, the macOS notifications with the five kinds of alert (M6), and the
// app itself (M7): launching at login, the clone it updates from, its
// version and "Cập nhật từ mã nguồn". Rows speak for themselves: no line of
// explanation under them. Each Mac runs its own copy and switches only itself.

import { useState } from 'react';
import { AlertKinds } from '../alerts/AlertsTab';
import { Button, Chip, Dot, Feedback, Toggle } from '../components/parts';
import { PERMISSION_CHIPS, UNREAD } from '../lib/alerts';
import type { ChipView } from '../lib/view';
import type { AlertKind, AppView, ClientStatus, Model, Peer, PeerTest, Theme, UpdateError } from '../lib/model';
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
  onSetPath(path: string): Promise<boolean>;
  onFind(): void;
  onGrant(): void;
  /** Resolves to false when the name could not be saved. */
  onSetMachine(name: string): Promise<boolean>;
  /** Resolves to null once saved, else the refusal (`badAlias:1`, `sameAlias:1`, `tooMany`). */
  onSetPeers(peers: Peer[]): Promise<string | null>;
  onTestPeer(alias: string): Promise<PeerTest>;
  /** Names (emails, organizations) on or off, in both windows. */
  onShowNames(on: boolean): void;
  onTheme(theme: Theme): void;
  /** Resolves to null once saved, else the refusal (`notPoolFile`, `notFound`). */
  onSetFirstmate(path: string): Promise<string | null>;
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
}

const CLIENT_CHIPS: Record<ClientStatus, ChipView> = {
  ok: { t: 'có, dùng được', k: 'ok' },
  missing: { t: 'chưa có', k: 'warn' },
  tooOpen: { t: 'cần quyền 600', k: 'bad' },
  invalid: { t: 'chưa đúng', k: 'bad' },
  unreadable: { t: 'không đọc được', k: 'bad' },
};

function pathHint(model: Model, refused: boolean): { tone: 'ok' | 'bad' | ''; text: string } {
  const q = model.quotaAxi;
  if (refused) return { tone: 'bad', text: 'Không có file ở đường dẫn này.' };
  if (!q.path) return { tone: 'bad', text: 'Không tìm thấy quota-axi ở các chỗ thường cài.' };
  if (!q.exists) return { tone: 'bad', text: 'Không có file ở đường dẫn này.' };
  const err = model.errors.claude;
  if (err && err.kind !== 'notFound' && err.kind !== 'notConfigured')
    return { tone: 'bad', text: 'quota-axi không chạy được: ' + (err.detail || err.kind) };
  if (q.fork === false) return { tone: 'bad', text: 'Bản ' + q.version + ' không có +cswap: thiếu số claude-swap và luật xoay vòng.' };
  if (q.version) return { tone: 'ok', text: q.version + ' ✓' };
  return { tone: '', text: '' };
}

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
    <div className="fr">
      <label htmlFor="qs-name">
        Tên máy này
      </label>
      <div>
        <input
          id="qs-name"
          className="inp"
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
        <div className="fhint">Để trống để dùng tên trong macOS: {model.machineDefault}.</div>
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
function OtherMacs({ model, onSet, onTest }: { model: Model; onSet: SettingsProps['onSetPeers']; onTest: SettingsProps['onTestPeer'] }) {
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
    <div className="fr">
      <div className="fl">
        Các Mac khác
      </div>
      <div className="macs">
        {rows.map((r, i) => {
          const t = testText(tests[r.alias.trim()], r.alias);
          const label = r.name.trim() || 'Mac mới';
          return (
            <div className="mac" key={i}>
              <Dot tone={t.tone} />
              <input
                className="inp s2"
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
              <span className="mal">tên kết nối</span>
              <input
                className={'inp mono s2' + (error?.row === i ? ' bad' : '')}
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
              <small>{t.text}</small>
              <button
                type="button"
                className="lnk"
                aria-label={'Bỏ ' + label}
                onClick={() => {
                  const next = rows.filter((_, j) => j !== i);
                  setRows(next);
                  void commit(next);
                }}
              >
                Bỏ
              </button>
            </div>
          );
        })}
        {error ? <div className="fhint bad">{error.text}</div> : null}
        {!rows.length ? <div className="fhint">Chưa có Mac nào. Luật xoay vòng chỉ lưu trên máy này.</div> : null}
        {!model.rotation.ssh ? <div className="fhint bad">Không thấy ssh trên máy này.</div> : null}
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

function firstmateHint(model: Model, refused: string | null): { tone: 'ok' | 'bad' | ''; text: string } {
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
function FirstmateFile({ model, onSet }: { model: Model; onSet: SettingsProps['onSetFirstmate'] }) {
  const [path, setPath] = useField(model.firstmate.file ?? '');
  const [refused, setRefused] = useState<string | null>(null);
  const commit = async () => {
    if (path.trim() === (model.firstmate.file ?? '')) return setRefused(null);
    setRefused(await onSet(path.trim()));
  };
  const hint = firstmateHint(model, refused);
  return (
    <div className="fr">
      <label htmlFor="qs-fm">
        File của firstmate
      </label>
      <div>
        <input
          id="qs-fm"
          className={'inp mono' + (hint.tone === 'bad' ? ' bad' : '')}
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
        {hint.text ? <div className={('fhint ' + hint.tone).trim()}>{hint.text}</div> : null}
      </div>
    </div>
  );
}

function Notifications(props: { model: Model; onAction: SettingsProps['onAction']; onTest: SettingsProps['onTestAlert']; onSwitch: SettingsProps['onAlertSwitch'] }) {
  const a = props.model.alerts;
  return (
    <div className="fr">
      <div className="fl">
        Thông báo
      </div>
      <div>
        <Chip chip={PERMISSION_CHIPS[a.permission]} />{' '}
        {a.permission === 'denied' ? <Button onClick={() => props.onAction('notify:settings')}>Mở Cài đặt hệ thống</Button> : null}
        {a.permission === 'unbundled' ? <div className="fhint">Chỉ app đã đóng gói mới gửi được thẻ macOS. Cảnh báo vẫn có ở tab Cảnh báo.</div> : null}
        <div style={{ marginTop: 6 }}>
          <AlertKinds switches={a.switches} testOff={a.permission === 'denied'} onTest={props.onTest} onSwitch={props.onSwitch} />
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
    <div className="fr">
      <div className="fl">
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
            <div className="fhint">macOS cần bạn bật Quota Station trong Cài đặt hệ thống › Cài đặt chung › Mục đăng nhập.</div>
          </>
        ) : null}
        {state === 'unavailable' ? <div className="fhint">Chỉ bản .app đã cài mới mở khi đăng nhập được.</div> : null}
        {refused && refused !== 'unavailable' ? <div className="fhint bad">Chưa đổi được: {refused}</div> : null}
      </div>
    </div>
  );
}

const SOURCE_REFUSED: Record<string, string> = {
  missing: 'Không có thư mục ở đường dẫn này.',
  notSource: 'Thư mục này không phải bản clone Quota Station: cần .git và scripts/install.sh.',
};

function sourceHint(app: AppView, refused: string | null): { tone: 'ok' | 'bad' | ''; text: string } {
  if (refused) return { tone: 'bad', text: SOURCE_REFUSED[refused] ?? 'Chưa lưu được: ' + refused };
  if (app.sourceStatus === 'ok') return { tone: 'ok', text: 'Có scripts/install.sh ✓' };
  if (app.sourceStatus === 'notSet') return { tone: '', text: 'Để trống trên Mac cài bằng gói build từ Mac khác.' };
  return { tone: 'bad', text: SOURCE_REFUSED[app.sourceStatus] };
}

/** The clone "Cập nhật từ mã nguồn" pulls and builds. */
function SourceDir({ app, onSet }: { app: AppView; onSet: SettingsProps['onSetSource'] }) {
  const [path, setPath] = useField(app.source ?? '');
  const [refused, setRefused] = useState<string | null>(null);
  const commit = async () => {
    if (path.trim() === (app.source ?? '')) return setRefused(null);
    setRefused(await onSet(path.trim()));
  };
  const hint = sourceHint(app, refused);
  return (
    <div className="fr">
      <label htmlFor="qs-src">
        Mã nguồn
      </label>
      <div>
        <input
          id="qs-src"
          className={'inp mono' + (hint.tone === 'bad' ? ' bad' : '')}
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
        {hint.text ? <div className={('fhint ' + hint.tone).trim()}>{hint.text}</div> : null}
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

/** The version and "Cập nhật từ mã nguồn": git pull --ff-only, then scripts/install.sh. */
function Version({ app, onUpdate, onRelaunch }: { app: AppView; onUpdate: SettingsProps['onUpdate']; onRelaunch: SettingsProps['onRelaunch'] }) {
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
    <div className="fr">
      <div className="fl">
        Phiên bản
      </div>
      <div>
        <span className="mono">
          {app.version}
          {app.commit ? ' · ' + app.commit : ''}
        </span>
        <div style={{ marginTop: 6 }}>
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
        {line ? <div className={('fhint ' + line.tone).trim()}>{line.text}</div> : null}
        {refused ? <div className="fhint bad">{UPDATE_REFUSED[refused] ?? 'Chưa chạy được: ' + refused}</div> : null}
      </div>
    </div>
  );
}

export function Settings({
  model,
  fresh,
  feedback,
  onAction,
  onSetPath,
  onFind,
  onGrant,
  onSetMachine,
  onSetPeers,
  onTestPeer,
  onShowNames,
  onTheme,
  onSetFirstmate,
  onTestAlert,
  onAlertSwitch,
  onLaunchAtLogin,
  onOpenLoginItems,
  onSetSource,
  onUpdate,
  onRelaunch,
}: SettingsProps) {
  const [path, setPath] = useField(model.quotaAxi.path ?? '');
  const [refused, setRefused] = useState(false);

  const hint = pathHint(model, refused);
  const commit = async () => {
    if (path === (model.quotaAxi.path ?? '')) return;
    setRefused(!(await onSetPath(path)));
  };
  const keychain = model.snapshot.keychain;

  return (
    <>
      <div className="tb" data-tauri-drag-region>
        <h2>Cài đặt</h2>
        <small>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </small>
      </div>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      <div className="card">
        <div className="form">
          <div className="fr">
            <label htmlFor="qs-axi">
              quota-axi
            </label>
            <div>
              <input
                id="qs-axi"
                className={'inp mono' + (hint.tone === 'bad' ? ' bad' : '')}
                value={path}
                spellCheck={false}
                autoComplete="off"
                onChange={(e) => {
                  setPath(e.target.value);
                  setRefused(false);
                }}
                onBlur={commit}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void commit();
                }}
              />
              {hint.text ? <div className={('fhint ' + hint.tone).trim()}>{hint.text}</div> : null}
              {hint.tone === 'bad' ? (
                <div style={{ marginTop: 6 }}>
                  <Button onClick={onFind}>Tự tìm lại</Button>
                </div>
              ) : null}
            </div>
          </div>
          <div className="fr">
            <div className="fl">
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
                  <div className="fhint">
                    {model.granting
                      ? 'macOS sẽ hỏi. Bấm "Luôn cho phép". App không đọc mục Keychain của Claude.'
                      : 'Đến khi cấp, dòng Claude dùng số chưa được cập nhật từ claude-swap.'}
                  </div>
                </>
              ) : (
                <Chip chip={{ t: 'chưa rõ', k: '' }} />
              )}
            </div>
          </div>
          <MachineName model={model} onSet={onSetMachine} />
          <OtherMacs model={model} onSet={onSetPeers} onTest={onTestPeer} />
          <div className="fr">
            <div className="fl">
              OAuth client Antigravity
            </div>
            <div>
              <Chip chip={CLIENT_CHIPS[model.antigravity.client]} />
              <div className="fhint">
                <span className="mono">{model.antigravity.clientPath}</span>, quyền 600, chỉ có trên Mac này. README ghi định dạng.
              </div>
            </div>
          </div>
          <div className="fr">
            <div className="fl" id="qs-email-l">
              Hiện email
            </div>
            <div>
              <Toggle on={model.showNames} label="Hiện email" onClick={() => onShowNames(!model.showNames)} />
            </div>
          </div>
          <div className="fr">
            <div className="fl" id="qs-theme-l">
              Giao diện
            </div>
            <div className="seg inline" role="group" aria-labelledby="qs-theme-l">
              {THEMES.map(([v, t]) => (
                <button key={v} type="button" aria-pressed={model.theme === v} onClick={() => onTheme(v)}>
                  {t}
                </button>
              ))}
            </div>
          </div>
          <FirstmateFile model={model} onSet={onSetFirstmate} />
          <Notifications model={model} onAction={onAction} onTest={onTestAlert} onSwitch={onAlertSwitch} />
          <LaunchAtLogin app={model.app} onSet={onLaunchAtLogin} onOpen={onOpenLoginItems} />
          <SourceDir app={model.app} onSet={onSetSource} />
          <Version app={model.app} onUpdate={onUpdate} onRelaunch={onRelaunch} />
        </div>
      </div>
    </>
  );
}
