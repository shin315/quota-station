// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// The Antigravity accounts this app logs in on this Mac (M4a), merged into
// the Antigravity tab's account table: "Thêm tài khoản" in the card's
// header, "Đăng xuất" with a confirmation under the row, "Đăng nhập lại" and
// "Mở trang xác minh" on the accounts that need them, each account's ⋯ menu
// (a new login at any time, and the account out of the rotation or back in,
// as on the Claude tab), and whether the app is
// reading each account's quota or its last read failed (M4b). While a login
// waits for the browser, the card offers to paste the callback URL and to
// cancel. Logins the row does not list yet stay in a short list under the
// table, and with two accounts or more the rotation's switch (AgyRotation).

import { useState, type ReactNode } from 'react';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { LIST_ROW } from '../components/page';
import { BannerBox, Button, Chip, FeedbackLine, FieldHint, Spinner } from '../components/parts';
import { Glyph } from '../components/icons';
import type { Api } from '../lib/api';
import { onWindows, t } from '../lib/i18n';
import {
  agyLoginErrorFeedback,
  agyLoginFeedback,
  agyLogoutErrorFeedback,
  agyLogoutFeedback,
  agyPauseErrorFeedback,
  agyPauseFeedback,
  agyReloginFeedback,
  agyRows,
  agyVerifyErrorFeedback,
  clientNotice,
  desk,
  pasteHint,
  type AgyRowView,
} from './agyLogin';
import type { AgyAccounts as AgyAccountsModel } from '../lib/model';
import { AgyRotation } from './AgyRotation';
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
            'Trang của Google mở trong trình duyệt mặc định. Màn hình đồng ý ghi tên ứng dụng của Antigravity, không phải Quota Station: app đăng nhập bằng OAuth client của Antigravity. Chờ tối đa 10 phút.',
            'Google’s page opens in the default browser. The consent screen names Antigravity’s app, not Quota Station: the app signs in with Antigravity’s OAuth client. Waits up to 10 minutes.',
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

/** The question asked under a row: a logout, a new login, or a pause of the rotation. */
export type AgyConfirmKind = 'logout' | 'relogin' | 'pause';

export interface AgyLogins {
  api: Api;
  view: AgyAccountsModel;
  rows: AgyRowView[];
  /** A login or a logout runs. */
  busy: boolean;
  /** The login whose question shows under its row, and which question. */
  confirm: { id: string; kind: AgyConfirmKind } | null;
  notice: ReturnType<typeof clientNotice>;
  /** "Tạm dừng tự chuyển…" is offered: two accounts or more, on macOS, with an api that keeps it. */
  canPause: boolean;
  login(): void;
  /** A new login meant for this account; another Google account picked in the browser is said, not taken for it. */
  relogin(row: AgyRowView): void;
  verify(row: AgyRowView): void;
  logout(row: AgyRowView): void;
  /** Holds the account out of the rotation (`true`) or puts it back. */
  setPaused(row: AgyRowView, paused: boolean): void;
  ask(id: string | null, kind?: AgyConfirmKind): void;
}

export function useAgyLogins(view: AgyAccountsModel, api: Api, onFeedback: (fb: FeedbackView | null) => void): AgyLogins {
  const [confirm, setConfirm] = useState<AgyLogins['confirm']>(null);
  const [leaving, setLeaving] = useState(false);
  const rows = agyRows(view);
  const login = async () => {
    onFeedback(null);
    try {
      onFeedback(agyLoginFeedback(await api.agyLogin()));
    } catch (e) {
      onFeedback(agyLoginErrorFeedback(e));
    }
  };
  const relogin = async (row: AgyRowView) => {
    setConfirm(null);
    onFeedback(null);
    try {
      onFeedback(agyReloginFeedback(row, await api.agyLogin(), rows));
    } catch (e) {
      onFeedback(agyLoginErrorFeedback(e));
    }
  };
  const setPaused = async (row: AgyRowView, paused: boolean) => {
    setConfirm(null);
    if (!api.agySetRotationPaused) return;
    try {
      await api.agySetRotationPaused(row.id, paused);
      onFeedback(agyPauseFeedback(row.name, paused));
    } catch (e) {
      onFeedback(agyPauseErrorFeedback(row.name, paused, e));
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
    rows,
    busy: view.loggingIn || leaving,
    confirm,
    notice: clientNotice(view),
    // The rotation runs from a LaunchAgent: macOS only, and only with two accounts to rotate.
    canPause: !!api.agySetRotationPaused && rows.length >= 2 && !onWindows(),
    login: () => void login(),
    relogin: (row) => void relogin(row),
    verify: (row) => void verify(row),
    logout: (row) => void logout(row),
    setPaused: (row, paused) => void setPaused(row, paused),
    ask: (id, kind = 'logout') => setConfirm(id == null ? null : { id, kind }),
  };
}

/** An account held out of the rotation, as the Claude tab marks one. */
const PAUSED_CHIP = (): ChipView => ({
  t: t('nghỉ tự chuyển', 'out of auto-switch'),
  k: '',
  title: t('Tự chuyển không chuyển sang tài khoản này cho tới khi bạn cho vào lại.', 'Auto-switch does not switch to this account until you put it back.'),
});

/** The login's state as chips beside its name: what it needs, and the app's read of it. */
export function loginChips(row: AgyRowView | undefined): ChipView[] {
  if (!row) return [];
  const chips: ChipView[] = [];
  if (row.paused) chips.push(PAUSED_CHIP());
  if (row.chip.k !== 'ok') chips.push({ ...row.chip, title: row.sub || undefined });
  if (row.read)
    chips.push({
      t: row.read.tone === 'info' ? row.read.text : t('lỗi đọc dữ liệu', 'read failed'),
      k: row.read.tone === 'bad' ? 'bad' : row.read.tone === 'warn' ? 'warn' : 'info',
      title: row.read.text,
    });
  return chips;
}

export function AddAccount({ logins }: { logins: AgyLogins }) {
  return (
    <Button primary lead="plus" disabled={!!logins.notice || logins.view.indexError || logins.busy} onClick={logins.login}>
      {t('Thêm tài khoản', 'Add account')}
    </Button>
  );
}

/** A login's buttons in its table row. */
export function LoginActions({ logins, row }: { logins: AgyLogins; row: AgyRowView }) {
  return (
    <>
      {row.relogin ? (
        <Button primary disabled={logins.busy || !!logins.notice} onClick={() => logins.relogin(row)}>
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

const ITEM = 'px-2.5 py-[7px] data-[highlighted]:bg-ctl-hover data-[highlighted]:text-ink';

/** A login's ⋯ menu, as on the Claude tab: the account out of the rotation or back in, and a new login at any time. */
export function AgyMenu({ logins, row }: { logins: AgyLogins; row: AgyRowView }) {
  const more = t('Thêm thao tác cho ', 'More actions for ') + row.name;
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="grid size-6 cursor-pointer place-items-center rounded-ctl border-0 bg-fill2 text-ink hover:brightness-[.97] data-[state=open]:brightness-[.94] [&>svg]:size-3.5"
          aria-label={more}
          title={more}
        >
          <Glyph name="dots-three" bold />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {row.paused ? (
          <DropdownMenuItem className={ITEM} disabled={logins.busy} onSelect={() => logins.setPaused(row, false)}>
            {t('Đưa lại vào tự chuyển', 'Resume auto-switch')}
          </DropdownMenuItem>
        ) : logins.canPause ? (
          <DropdownMenuItem className={ITEM} disabled={logins.busy} onSelect={() => logins.ask(row.id, 'pause')}>
            {t('Tạm dừng tự chuyển…', 'Pause auto-switch…')}
          </DropdownMenuItem>
        ) : null}
        <DropdownMenuItem className={ITEM} disabled={logins.busy || !!logins.notice || logins.view.indexError} onSelect={() => logins.ask(row.id, 'relogin')}>
          {t('Đăng nhập lại…', 'Sign in again…')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Under the row whose logout, new login or pause is asked: what it does, and the two answers. */
export function RowConfirm({ logins, row, inTable }: { logins: AgyLogins; row: AgyRowView; inTable?: boolean }) {
  const ask = (label: string, title: string, body: ReactNode, yes: string, onYes: () => void) => (
    <div className={cn(LIST_ROW, inTable && 'col-span-full mt-0 mb-1.5 px-3')} role="group" aria-label={label}>
      <div>
        <b>{title}</b>
        <small className="block text-xs text-ink2">{body}</small>
      </div>
      <Button onClick={() => logins.ask(null)}>{t('Huỷ', 'Cancel')}</Button>
      <Button primary disabled={logins.busy} onClick={onYes}>
        {yes}
      </Button>
    </div>
  );
  const who = row.name;
  switch (logins.confirm?.kind ?? 'logout') {
    case 'relogin':
      return ask(
        t('Đăng nhập lại ', 'Sign in again: ') + who,
        t('Đăng nhập lại ' + who + '?', 'Sign ' + who + ' in again?'),
        t(
          'Trang của Google mở trong trình duyệt. Chọn đúng ' + (row.full || 'tài khoản này') + '; chọn tài khoản khác thì ' + who + ' không đổi.',
          'Google’s page opens in the browser. Pick ' + (row.full || 'this account') + '; with another account, ' + who + ' stays as it is.',
        ),
        t('Mở trang đăng nhập', 'Open the sign-in page'),
        () => logins.relogin(row),
      );
    case 'pause':
      return ask(
        t('Tạm dừng tự chuyển cho ', 'Pause auto-switch for ') + who,
        t('Tạm dừng tự chuyển cho ' + who + '?', 'Pause auto-switch for ' + who + '?'),
        t(
          'Tự chuyển bỏ qua ' + who + ' cho tới khi bạn cho vào lại. Bạn vẫn chuyển tay sang ' + who + ' được; hạn mức vẫn hiện như cũ.',
          'Auto-switch skips ' + who + ' until you put it back. You can still switch to ' + who + ' by hand; its quota still shows.',
        ),
        t('Tạm dừng', 'Pause'),
        () => logins.setPaused(row, true),
      );
    default:
      return ask(
        t('Đăng xuất ', 'Sign out ') + who,
        t('Đăng xuất ' + who + ' khỏi Quota Station?', 'Sign ' + who + ' out of Quota Station?'),
        desk(t('Antigravity.app và agy vẫn đăng nhập như cũ.', 'Antigravity.app and agy stay signed in as before.')),
        t('Đăng xuất', 'Sign out'),
        () => logins.logout(row),
      );
  }
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
            logins.confirm?.id === r.id ? (
              <RowConfirm key={r.id} logins={logins} row={r} />
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
                <span style={{ display: 'flex', gap: 4 }}>
                  {r.paused ? <Chip chip={PAUSED_CHIP()} /> : null}
                  <Chip chip={r.chip} />
                </span>
                <div style={{ display: 'flex', gap: 6 }}>
                  <LoginActions logins={logins} row={r} />
                  <AgyMenu logins={logins} row={r} />
                </div>
              </div>
            ),
          )}
        </div>
      ) : null}
      {!logins.rows.length && !view.indexError ? <p className="mt-2 mb-0 text-sm text-ink2">{t('Chưa có tài khoản nào đăng nhập trong app.', 'No accounts signed in to the app yet.')}</p> : null}
      {view.loggingIn ? <Waiting api={logins.api} /> : null}
      {logins.rows.length >= 2 ? <AgyRotation api={logins.api} /> : null}
    </>
  );
}
