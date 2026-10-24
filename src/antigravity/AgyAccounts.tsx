// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// The Antigravity accounts this app logs in on this Mac (M4a), merged into
// the Antigravity tab's account table: "Thêm tài khoản" in the card's
// header, "Đăng xuất" with a confirmation under the row, "Đăng nhập lại" and
// "Mở trang xác minh" on the accounts that need them, and whether the app is
// reading each account's quota or its last read failed (M4b). While a login
// waits for the browser, the card offers to paste the callback URL and to
// cancel. Logins the row does not list yet stay in a short list under the
// table.

import { useState } from 'react';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { LIST_ROW } from '../components/page';
import { BannerBox, Button, Chip, FeedbackLine, FieldHint, Spinner } from '../components/parts';
import type { Api } from '../lib/api';
import { t } from '../lib/i18n';
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
} from './agyLogin';
import type { AgyAccounts as AgyAccountsModel } from '../lib/model';
import type { ChipView, FeedbackView } from '../lib/view';

/** A login's row under the table, or the question under its row. */
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
    <FeedbackLine tone="info" style={{ margin: '8px 0 0' }}>
      <Spinner />
      <div style={{ minWidth: 0, flex: 1 }}>
        <b className="block font-semi">{t('Đang chờ đăng nhập trong trình duyệt…', 'Waiting for sign-in in the browser…')}</b>
        <small className="block text-xs text-ink2">
          {t(
            'Trang của Google mở trong trình duyệt mặc định. Màn hình đồng ý ghi tên ứng dụng của Antigravity, không phải Quota Station: app đăng nhập bằng client của Antigravity, như Cockpit. Chờ tối đa 10 phút.',
            'Google’s page opens in the default browser. The consent screen names Antigravity’s app, not Quota Station: the app signs in with Antigravity’s client, as Cockpit does. Waits up to 10 minutes.',
          )}
        </small>
        <label htmlFor="qs-agy-cb" className="mt-1 text-xs text-ink2" style={{ display: 'block', marginTop: 8 }}>
          {t('Trình duyệt không quay lại được app? Dán URL trên thanh địa chỉ vào đây.', 'The browser did not come back to the app? Paste the URL from its address bar here.')}
        </label>
        <div className="flex items-center justify-between gap-2.5" style={{ marginTop: 4 }}>
          <Input
            id="qs-agy-cb"
            mono
            invalid={!!hint}
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
            {t('Gửi', 'Send')}
          </Button>
          <Button onClick={() => void api.agyLoginCancel()}>{t('Huỷ', 'Cancel')}</Button>
        </div>
        {hint ? <FieldHint tone="bad">{hint}</FieldHint> : null}
      </div>
    </FeedbackLine>
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
      t: row.read.tone === 'info' ? row.read.text : t('đọc lỗi', 'read failed'),
      k: row.read.tone === 'bad' ? 'bad' : row.read.tone === 'warn' ? 'warn' : 'info',
      title: row.read.text,
    });
  return chips;
}

export function AddAccount({ logins }: { logins: AgyLogins }) {
  return (
    <Button primary className="text-s" disabled={!!logins.notice || logins.view.indexError || logins.busy} onClick={logins.login}>
      {t('Thêm tài khoản', 'Add account')}
    </Button>
  );
}

/** A login's buttons in its table row. */
export function LoginActions({ logins, row }: { logins: AgyLogins; row: AgyRowView }) {
  return (
    <>
      {row.relogin ? (
        <Button primary disabled={logins.busy} onClick={logins.login}>
          {t('Đăng nhập lại', 'Sign in again')}
        </Button>
      ) : null}
      {row.verify ? (
        <Button primary onClick={() => logins.verify(row)}>
          {t('Mở trang xác minh', 'Open the verification page')}
        </Button>
      ) : null}
      <Button label={t('Đăng xuất ', 'Sign out ') + row.name} disabled={logins.busy} onClick={() => logins.ask(row.id)}>
        {t('Đăng xuất', 'Sign out')}
      </Button>
    </>
  );
}

/** Under the row whose logout is asked: what it does, and the two answers. */
export function LogoutConfirm({ logins, row, inTable }: { logins: AgyLogins; row: AgyRowView; inTable?: boolean }) {
  return (
    <div className={cn(LIST_ROW, inTable && 'col-span-full mt-0 mb-1.5 px-3')} role="group" aria-label={t('Đăng xuất ', 'Sign out ') + row.name}>
      <div>
        <b>{t('Đăng xuất ' + row.name + ' khỏi Quota Station?', 'Sign ' + row.name + ' out of Quota Station?')}</b>
        <small className="block text-xs text-ink2">{t('Antigravity.app và agy vẫn đăng nhập như cũ.', 'Antigravity.app and agy stay signed in as before.')}</small>
      </div>
      <Button onClick={() => logins.ask(null)}>{t('Thôi', 'Cancel')}</Button>
      <Button primary disabled={logins.busy} onClick={() => logins.logout(row)}>
        {t('Đăng xuất', 'Sign out')}
      </Button>
    </div>
  );
}

/**
 * What the table cannot say: a missing OAuth client, an unreadable list,
 * the logins the row does not list yet, and a login waiting for the browser.
 */
export function LoginNotes({ logins, listed }: { logins: AgyLogins; listed: number[] }) {
  const { notice, view } = logins;
  const extra = logins.rows.filter((r) => !listed.includes(r.number));
  return (
    <>
      {notice ? (
        <BannerBox tone={notice.tone} title={notice.title} body={notice.body} style={{ margin: '8px 0' }} />
      ) : null}
      {view.indexError ? (
        <BannerBox
          tone="bad"
          title={t('Không đọc được danh sách tài khoản', 'Cannot read the account list')}
          body={t(
            'File accounts.json trong thư mục antigravity của app bị hỏng. App để nguyên file, không ghi đè.',
            'The accounts.json file in the app’s antigravity folder is damaged. The app leaves it as it is and does not overwrite it.',
          )}
          style={{ margin: '8px 0' }}
        />
      ) : null}
      {extra.length ? (
        <div role="list" aria-label={t('Tài khoản đăng nhập trong app', 'Accounts signed in to the app')}>
          {extra.map((r) =>
            logins.confirm === r.id ? (
              <LogoutConfirm key={r.id} logins={logins} row={r} />
            ) : (
              <div className={LIST_ROW} role="listitem" aria-label={r.name} key={r.id}>
                <div>
                  <b title={r.full || undefined}>{r.name}</b>
                  {r.sub ? <small className="block text-xs text-ink2">{r.sub}</small> : null}
                  {r.read ? (
                    <small
                      className={cn(
                        'mt-0.5 flex items-center gap-1.5 text-xs',
                        r.read.tone === 'warn' ? 'text-warn-ink' : r.read.tone === 'bad' ? 'text-bad-ink' : 'text-ink2',
                      )}
                      role="status"
                    >
                      {r.read.tone === 'info' ? <Spinner className="mt-0 size-2.5 border-[1.5px]" /> : null}
                      {r.read.text}
                    </small>
                  ) : null}
                </div>
                <Chip chip={r.chip} />
                <div style={{ display: 'flex', gap: 6 }}>
                  <LoginActions logins={logins} row={r} />
                </div>
              </div>
            ),
          )}
        </div>
      ) : null}
      {!logins.rows.length && !view.indexError ? <p className="mt-2 mb-0 text-sm text-ink2">{t('Chưa có tài khoản nào đăng nhập trong app.', 'No accounts signed in to the app yet.')}</p> : null}
      {view.loggingIn ? <Waiting api={logins.api} /> : null}
    </>
  );
}
