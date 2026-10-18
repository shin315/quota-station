// The Cài đặt tab of the main window: where quota-axi is, Keychain access
// for quota-axi, and this Mac's name. Rows and copy come from the
// prototype's Cài đặt tab. Each Mac runs its own copy, so there is no row
// for other Macs; email, login, refresh, theme and Codex come in later
// milestones.

import { useState } from 'react';
import { Button, Chip, Dot, Feedback } from '../components/parts';
import type { Model } from '../lib/model';
import type { FeedbackView, PopoverView } from '../lib/view';

export interface SettingsProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback?: FeedbackView | null;
  onSetPath(path: string): Promise<boolean>;
  onFind(): void;
  onGrant(): void;
  /** Resolves to false when the name could not be saved. */
  onSetMachine(name: string): Promise<boolean>;
}

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

export function Settings({ model, fresh, feedback, onSetPath, onFind, onGrant, onSetMachine }: SettingsProps) {
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
      {feedback ? <Feedback fb={feedback} /> : null}
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
                      ? 'macOS sẽ hỏi. Bấm "Luôn cho phép". App không đọc Keychain.'
                      : 'Đến khi cấp, dòng Claude dùng số chưa được cập nhật từ claude-swap.'}
                  </div>
                </>
              ) : (
                <Chip chip={{ t: 'chưa rõ', k: '' }} />
              )}
            </div>
          </div>
          <MachineName model={model} onSet={onSetMachine} />
        </div>
      </div>
    </>
  );
}
