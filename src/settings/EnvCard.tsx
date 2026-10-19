// Cài đặt's card "Biến môi trường đè đăng nhập" (M14, I): each variable set
// in a shell file or launchd, the agent it overrides and how, with "Mở file"
// at its line and "Bỏ qua"; a green line when there is none. "Xem danh
// sách" says what the app looks at. The app never reads a value, never
// edits a shell file and never unsets anything.

import { useRef, useState } from 'react';
import { Facts } from '../agents/McpDialogs';
import { Button, Chip } from '../components/parts';
import { SHEET_TITLE_ID } from '../components/Sheets';
import type { Api } from '../lib/api';
import {
  agentChip,
  checkedFeedback,
  checkedLine,
  envFailedFeedback,
  ignoredFeedback,
  openedFeedback,
  openFailedFeedback,
  unreadNote,
  whereLine,
} from '../lib/env';
import type { Model } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';

/** The list dialog, and what the card's, banner's and feedback's buttons do. */
export function useEnv(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const [list, setList] = useState(false);
  const latest = useRef(model);
  latest.current = model;
  const found = (id: string) => latest.current?.env.found.find((f) => f.id === id);
  const check = async () => {
    try {
      await api.envCheck();
      if (latest.current) showFeedback(checkedFeedback(latest.current.env));
    } catch {
      showFeedback(envFailedFeedback());
    }
  };
  const open = async (id: string) => {
    const f = found(id);
    try {
      const editor = await api.envOpen(id);
      showFeedback(f ? openedFeedback(f, editor) : null);
    } catch {
      showFeedback(openFailedFeedback(f));
    }
  };
  const ignore = async (id: string) => {
    const f = found(id);
    try {
      await api.envIgnore(id);
      if (f) showFeedback(ignoredFeedback(f));
    } catch {
      showFeedback(envFailedFeedback());
    }
  };
  const back = async (id: string) => {
    showFeedback(null);
    await api.envUnignore(id).catch(() => showFeedback(envFailedFeedback()));
  };
  /** Its own actions; false for the others. `env:settings` is the window's. */
  const act = (a: Action): boolean => {
    if (a === 'env:check') void check();
    else if (a === 'env:list') setList(true);
    else if (a.startsWith('env:open:')) void open(a.slice('env:open:'.length));
    else if (a.startsWith('env:ignore:')) void ignore(a.slice('env:ignore:'.length));
    else if (a.startsWith('env:back:')) void back(a.slice('env:back:'.length));
    else return false;
    return true;
  };
  return { list, act, close: () => setList(false) };
}

export type EnvState = ReturnType<typeof useEnv>;

export function EnvCard({ model, onAction }: { model: Model; onAction(act: Action): void }) {
  const env = model.env;
  return (
    <div className="card env-card">
      <h4>Biến môi trường đè đăng nhập</h4>
      {env.found.length ? (
        <div role="list" aria-label="Biến đè đăng nhập">
          {env.found.map((f) => (
            <div key={f.id} className="ev" role="listitem" aria-label={f.name + ', ' + whereLine(f)}>
              <span>
                <b>{f.name}</b> <Chip chip={agentChip(f)} />
                <small>{whereLine(f)}</small>
                <small>{f.effect}</small>
              </span>
              <span className="a">
                {f.place !== 'launchctl' ? (
                  <Button label={'Mở file: ' + whereLine(f)} onClick={() => onAction(`env:open:${f.id}`)}>
                    Mở file
                  </Button>
                ) : null}
                <Button label={'Bỏ qua ' + f.name + ' ở ' + whereLine(f)} onClick={() => onAction(`env:ignore:${f.id}`)}>
                  Bỏ qua
                </Button>
              </span>
            </div>
          ))}
        </div>
      ) : env.checkedAt ? (
        <p className="ev-none">
          <Chip chip={{ t: 'không có', k: 'ok' }} /> Không có biến nào đè đăng nhập.
        </p>
      ) : null}
      {unreadNote(env) ? <p className="note warn ev-unread">{unreadNote(env)}</p> : null}
      <p className="note ev-foot">
        {checkedLine(env)}{' '}
        <button type="button" className="lnk sm" disabled={env.checking} onClick={() => onAction('env:check')}>
          Kiểm lại
        </button>
        {' · '}
        <button type="button" className="lnk sm" onClick={() => onAction('env:list')}>
          Xem danh sách
        </button>
      </p>
    </div>
  );
}

/** "Xem danh sách": the names and the places, and what the app does with them. */
export function EnvListBody({ model, env }: { model: Model; env: EnvState }) {
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>App kiểm những gì</h3>
      <div className="sec-t">Biến</div>
      <p className="mono ev-vars">{model.env.vars.join(' · ')}</p>
      <div className="sec-t">Chỗ</div>
      <Facts items={model.env.places.map((p) => ['', p === 'launchctl' ? 'launchctl (app mở từ Dock)' : p])} />
      <p className="tech">
        App tìm tên biến ở đầu dòng trong từng file và hỏi launchctl biến có đặt hay không. App không đọc giá trị, không chạy file shell, không sửa
        file và không tự bỏ biến.
      </p>
      <div className="btns">
        <Button large primary onClick={env.close}>
          Đóng
        </Button>
      </div>
    </>
  );
}
