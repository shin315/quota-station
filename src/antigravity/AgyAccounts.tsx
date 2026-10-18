// The Antigravity accounts this app logs in on this Mac (M4a), merged into
// the Antigravity tab's account table: "Thêm tài khoản" in the card's
// header, "Đăng xuất" with a confirmation under the row, "Đăng nhập lại" and
// "Mở trang xác minh" on the accounts that need them, and whether the app is
// reading each account's quota or its last read failed (M4b). While a login
// waits for the browser, the card offers to paste the callback URL and to
// cancel. Logins quota-axi does not list yet stay in a short list under the
// table.

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
import type { ChipView, FeedbackView } from '../lib/view';

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

export interface AgyLogins {
  api: Api;
  view: AgyAccountsModel;
  rows: AgyRowView[];
  /** A login or a logout runs. */
  busy: boolean;
  /** The login whose logout is being confirmed. */
  confirm: string | null;
  notice: ReturnType<typeof clientNotice>;
  login(): void;
  verify(row: AgyRowView): void;
  logout(row: AgyRowView): void;
  ask(id: string | null): void;
}

export function useAgyLogins(view: AgyAccountsModel, api: Api, onFeedback: (fb: FeedbackView | null) => void): AgyLogins {
  const [confirm, setConfirm] = useState<string | null>(null);
  const [leaving, setLeaving] = useState(false);
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
  return {
    api,
    view,
    rows: agyRows(view),
    busy: view.loggingIn || leaving,
    confirm,
    notice: clientNotice(view),
    login: () => void login(),
    verify: (row) => void verify(row),
    logout: (row) => void logout(row),
    ask: setConfirm,
  };
}

/** The login's state as chips beside its name: what it needs, and the app's read of it. */
export function loginChips(row: AgyRowView | undefined): ChipView[] {
  if (!row) return [];
  const chips: ChipView[] = [];
  if (row.chip.k !== 'ok') chips.push({ ...row.chip, title: row.sub || undefined });
  if (row.read)
    chips.push({
      t: row.read.tone === 'info' ? row.read.text : 'đọc lỗi',
      k: row.read.tone === 'bad' ? 'bad' : row.read.tone === 'warn' ? 'warn' : 'info',
      title: row.read.text,
    });
  return chips;
}

export function AddAccount({ logins }: { logins: AgyLogins }) {
  return (
    <Button primary disabled={!!logins.notice || logins.view.indexError || logins.busy} onClick={logins.login}>
      Thêm tài khoản
    </Button>
  );
}

/** A login's buttons in its table row. */
export function LoginActions({ logins, row }: { logins: AgyLogins; row: AgyRowView }) {
  return (
    <>
      {row.relogin ? (
        <Button primary disabled={logins.busy} onClick={logins.login}>
          Đăng nhập lại
        </Button>
      ) : null}
      {row.verify ? (
        <Button primary onClick={() => logins.verify(row)}>
          Mở trang xác minh
        </Button>
      ) : null}
      <Button label={'Đăng xuất ' + row.name} disabled={logins.busy} onClick={() => logins.ask(row.id)}>
        Đăng xuất
      </Button>
    </>
  );
}

/** Under the row whose logout is asked: what it does, and the two answers. */
export function LogoutConfirm({ logins, row }: { logins: AgyLogins; row: AgyRowView }) {
  return (
    <div className="arow confirm" role="group" aria-label={'Đăng xuất ' + row.name}>
      <div>
        <b>Đăng xuất {row.name} khỏi Quota Station?</b>
        <small>Antigravity.app và agy vẫn đăng nhập như cũ.</small>
      </div>
      <Button onClick={() => logins.ask(null)}>Thôi</Button>
      <Button primary disabled={logins.busy} onClick={() => logins.logout(row)}>
        Đăng xuất
      </Button>
    </div>
  );
}

/**
 * What the table cannot say: a missing OAuth client, an unreadable list,
 * the logins quota-axi does not list yet, and a login waiting for the browser.
 */
export function LoginNotes({ logins, listed }: { logins: AgyLogins; listed: number[] }) {
  const { notice, view } = logins;
  const extra = logins.rows.filter((r) => !listed.includes(r.number));
  return (
    <>
      {notice ? (
        <div className={'banner ' + notice.tone} role="status" style={{ margin: '8px 0' }}>
          <b>{notice.title}</b>
          <p>{notice.body}</p>
        </div>
      ) : null}
      {view.indexError ? (
        <div className="banner bad" role="status" style={{ margin: '8px 0' }}>
          <b>Không đọc được danh sách tài khoản</b>
          <p>File accounts.json trong thư mục antigravity của app bị hỏng. App để nguyên file, không ghi đè.</p>
        </div>
      ) : null}
      {extra.length ? (
        <div role="list" aria-label="Tài khoản đăng nhập trong app">
          {extra.map((r) =>
            logins.confirm === r.id ? (
              <LogoutConfirm key={r.id} logins={logins} row={r} />
            ) : (
              <div className="arow" role="listitem" aria-label={r.name} key={r.id}>
                <div>
                  <b title={r.full || undefined}>{r.name}</b>
                  {r.sub ? <small>{r.sub}</small> : null}
                  {r.read ? (
                    <small className={'aread ' + r.read.tone} role="status">
                      {r.read.tone === 'info' ? <span className="spin" aria-hidden="true" /> : null}
                      {r.read.text}
                    </small>
                  ) : null}
                </div>
                <Chip chip={r.chip} />
                <div className="acts" style={{ display: 'flex', gap: 6 }}>
                  <LoginActions logins={logins} row={r} />
                </div>
              </div>
            ),
          )}
        </div>
      ) : null}
      {!logins.rows.length && !view.indexError ? <p className="note">Chưa có tài khoản nào đăng nhập trong app.</p> : null}
      {view.loggingIn ? <Waiting api={logins.api} /> : null}
    </>
  );
}
