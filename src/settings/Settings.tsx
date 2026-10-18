// The Cài đặt tab of the main window: where quota-axi is, Keychain access
// for quota-axi, this Mac's name, the other Macs the rotation file is
// copied to (an SSH alias each), the Antigravity OAuth client file and
// whether emails show. Rows and copy come from the prototype's Cài đặt tab.
// Each Mac runs its own copy and switches only itself; login, refresh,
// theme and Codex come in later milestones.

import { useState } from 'react';
import { Button, Chip, Dot, Feedback, Toggle } from '../components/parts';
import type { ChipView } from '../lib/view';
import type { ClientStatus, Model, Peer, PeerTest } from '../lib/model';
import type { Action, FeedbackView, PopoverView } from '../lib/view';

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
  onShowEmail(on: boolean): void;
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
        Tên máy này<small>Hiện ở đầu popover và cửa sổ</small>
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
        <small>Tên kết nối SSH để gửi luật xoay vòng. Mỗi Mac chạy bản Quota Station riêng; app không điều khiển máy khác.</small>
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

export function Settings({ model, fresh, feedback, onAction, onSetPath, onFind, onGrant, onSetMachine, onSetPeers, onTestPeer, onShowEmail }: SettingsProps) {
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
              quota-axi<small>Tự tìm đường dẫn. Cần phiên bản có hỗ trợ +cswap.</small>
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
              Quyền Keychain<small>Để quota-axi đọc dòng Claude trực tiếp</small>
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
              OAuth client Antigravity<small>File oauth-client.json để đăng nhập Antigravity trong app</small>
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
              Hiện email<small>Mặc định tắt. Tài khoản gọi theo số</small>
            </div>
            <div>
              <Toggle on={model.showEmail} label="Hiện email" onClick={() => onShowEmail(!model.showEmail)} />
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
