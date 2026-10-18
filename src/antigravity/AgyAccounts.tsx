// The Antigravity tab's login card (M4a): the accounts this app logs in on
// this Mac, "Thêm tài khoản", "Đăng xuất" with a confirmation in the row,
// and the accounts that need a new login or a verification. While a login
// waits for the browser, the card offers to paste the callback URL and to
// cancel. Reading their quota and switching come in later milestones.

import { useState } from 'react';
import { Button, Chip } from '../components/parts';
import type { Api } from '../lib/api';
import {
  agyLoginErrorFeedback,
  agyLoginFeedback,
  agyLogoutErrorFeedback,
  agyLogoutFeedback,
  agyRows,
  agyVerifyErrorFeedback,
  clientNotice,
  pasteHint,
  type AgyRowView,
} from '../lib/agyLogin';
import type { AgyAccounts as AgyAccountsModel } from '../lib/model';
import type { FeedbackView } from '../lib/view';

export interface AgyAccountsProps {
  view: AgyAccountsModel;
  api: Api;
  onFeedback(fb: FeedbackView | null): void;
}

function Waiting({ api }: { api: Api }) {
  const [url, setUrl] = useState('');
  const [hint, setHint] = useState('');
  const send = async () => {
    if (!url.trim()) return;
    try {
      await api.agyLoginCallback(url.trim());
      setHint('');
    } catch (e) {
      setHint(pasteHint(e));
    }
  };
  return (
    <div className="fb info" role="status" style={{ margin: '8px 0 0' }}>
      <span className="spin" aria-hidden="true" />
      <div style={{ minWidth: 0, flex: 1 }}>
        <b>Đang chờ đăng nhập trong trình duyệt…</b>
        <small>
          Trang của Google mở trong trình duyệt mặc định. Màn hình đồng ý ghi tên ứng dụng của Antigravity, không phải Quota Station: app
          đăng nhập bằng client của Antigravity, như Cockpit. Chờ tối đa 10 phút.
        </small>
        <label htmlFor="qs-agy-cb" className="fhint" style={{ display: 'block', marginTop: 8 }}>
          Trình duyệt không quay lại được app? Dán URL trên thanh địa chỉ vào đây.
        </label>
        <div className="row2" style={{ marginTop: 4 }}>
          <input
            id="qs-agy-cb"
            className={'inp mono' + (hint ? ' bad' : '')}
            value={url}
            placeholder="http://localhost:…/oauth-callback?state=…&code=…"
            spellCheck={false}
            autoComplete="off"
            onChange={(e) => {
              setUrl(e.target.value);
              setHint('');
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void send();
            }}
          />
          <Button disabled={!url.trim()} onClick={() => void send()}>
            Gửi
          </Button>
          <Button onClick={() => void api.agyLoginCancel()}>Huỷ</Button>
        </div>
        {hint ? <div className="fhint bad">{hint}</div> : null}
      </div>
    </div>
  );
}

function Row(props: {
  row: AgyRowView;
  busy: boolean;
  confirming: boolean;
  onAsk(): void;
  onKeep(): void;
  onLogout(): void;
  onLogin(): void;
  onVerify(): void;
}) {
  const r = props.row;
  if (props.confirming) {
    return (
      <div className="arow" role="listitem" aria-label={r.name}>
        <div>
          <b>Đăng xuất {r.name} khỏi Quota Station?</b>
          <small>App xoá đăng nhập của mình (mục Keychain riêng của app). Antigravity.app và agy vẫn đăng nhập như cũ; Google không bị thu hồi gì.</small>
        </div>
        <Button onClick={props.onKeep}>Thôi</Button>
        <Button primary disabled={props.busy} onClick={props.onLogout}>
          Đăng xuất
        </Button>
      </div>
    );
  }
  return (
    <div className="arow" role="listitem" aria-label={r.name}>
      <div>
        <b>{r.name}</b>
        <small>{r.sub}</small>
      </div>
      <Chip chip={r.chip} />
      <div className="acts" style={{ display: 'flex', gap: 6 }}>
        {r.relogin ? (
          <Button primary disabled={props.busy} onClick={props.onLogin}>
            Đăng nhập lại
          </Button>
        ) : null}
        {r.verify ? (
          <Button primary onClick={props.onVerify}>
            Mở trang xác minh
          </Button>
        ) : null}
        <Button label={'Đăng xuất ' + r.name} disabled={props.busy} onClick={props.onAsk}>
          Đăng xuất
        </Button>
      </div>
    </div>
  );
}

export function AgyAccounts({ view, api, onFeedback }: AgyAccountsProps) {
  const [confirm, setConfirm] = useState<string | null>(null);
  const [leaving, setLeaving] = useState(false);
  const rows = agyRows(view);
  const notice = clientNotice(view);
  const busy = view.loggingIn || leaving;

  const login = async () => {
    onFeedback(null);
    try {
      onFeedback(agyLoginFeedback(await api.agyLogin()));
    } catch (e) {
      onFeedback(agyLoginErrorFeedback(e));
    }
  };
  const verify = async (row: AgyRowView) => {
    try {
      await api.agyOpenVerification(row.id);
    } catch (e) {
      onFeedback(agyVerifyErrorFeedback(row.name, e));
    }
  };
  const logout = async (row: AgyRowView) => {
    setLeaving(true);
    try {
      await api.agyLogout(row.id);
      onFeedback(agyLogoutFeedback(row.name));
    } catch (e) {
      onFeedback(agyLogoutErrorFeedback(row.name, e));
    } finally {
      setLeaving(false);
      setConfirm(null);
    }
  };

  return (
    <div className="card">
      <h4>
        <span className="idm g" />
        Đăng nhập trong Quota Station
        <span className="r">{rows.length ? rows.length + ' tài khoản' : ''}</span>
      </h4>
      {notice ? (
        <div className={'banner ' + notice.tone} role="status" style={{ margin: '4px 0 8px' }}>
          <b>{notice.title}</b>
          <p>{notice.body}</p>
        </div>
      ) : null}
      {view.indexError ? (
        <div className="banner bad" role="status" style={{ margin: '4px 0 8px' }}>
          <b>Không đọc được danh sách tài khoản</b>
          <p>File accounts.json trong thư mục antigravity của app bị hỏng. App để nguyên file, không ghi đè.</p>
        </div>
      ) : null}
      {rows.length ? (
        <div role="list" aria-label="Tài khoản Antigravity đăng nhập trong app">
          {rows.map((r) => (
            <Row
              key={r.id}
              row={r}
              busy={busy}
              confirming={confirm === r.id}
              onAsk={() => setConfirm(r.id)}
              onKeep={() => setConfirm(null)}
              onLogout={() => void logout(r)}
              onLogin={() => void login()}
              onVerify={() => void verify(r)}
            />
          ))}
        </div>
      ) : !view.indexError ? (
        <p>Chưa có tài khoản Antigravity nào đăng nhập trong app trên máy này.</p>
      ) : null}
      {view.loggingIn ? <Waiting api={api} /> : null}
      <div className="row2" style={{ marginTop: 10 }}>
        <p style={{ margin: 0, flex: 1 }}>
          Mỗi Mac giữ đăng nhập của riêng nó. Đọc hạn mức và chuyển Antigravity từ app có ở bản sau.
        </p>
        <Button primary disabled={!!notice || view.indexError || busy} onClick={() => void login()}>
          Thêm tài khoản
        </Button>
      </div>
    </div>
  );
}
