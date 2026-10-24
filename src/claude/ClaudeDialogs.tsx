// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude tab's dialogs, as the prototype draws them (a-add … a-remove):
// the ways to add an account, the wait for the browser, a setup token, added
// with its nickname, already here, not signed in; an account's details and
// nickname; holding it out of the rotation; removing it; signing it in again.
// A token typed here goes to the app and is never shown again.

import { useEffect, useState, type ReactNode } from 'react';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Button, Chip, ChoiceGroup, Facts, FeedbackLine, Hint, LinkButton, Rich, Spinner, Toggle } from '../components/parts';
import { t } from '../lib/i18n';
import { localName, quotaLine, rotationMatters, signInChip } from '../lib/claudeAccounts';
import type { ClaudeAccountRow, ClaudeAccountsView, Tab } from '../lib/model';
import type { AddHow, ClaudeAccountsState, ClaudeDialog } from './useClaudeAccounts';

/** How long `claude auth login` waits for the browser (the app's own limit). */
export const LOGIN_WAIT_MS = 10 * 60 * 1000;

interface BodyProps {
  claude: ClaudeAccountsState;
  view: ClaudeAccountsView;
  /** Nâng cao is on: the rotation rules exist. */
  advanced: boolean;
  /** The account's name as the table shows it. */
  nameOf(slot: number): string;
  onTab(tab: Tab): void;
}

const LABEL = 'mb-1.5 block text-[12.5px] font-semi text-ink2';

/** The dialog's head: a small line over the title, the title, the line under it. */
function Head({ title, lead, cap }: { title: string; lead?: string; cap?: string }) {
  return (
    <div className="grid gap-1">
      {cap ? <span className="text-[12.5px] font-semi text-ink3">{cap}</span> : null}
      <DialogTitle className="mb-0 text-[19px] font-bold tracking-head text-heading">{title}</DialogTitle>
      {lead ? (
        <DialogLead className="mb-0">
          <Rich text={lead} />
        </DialogLead>
      ) : null}
    </div>
  );
}

function Stack({ children }: { children: ReactNode }) {
  return <div className="grid gap-3.5">{children}</div>;
}

function ErrorLine({ text }: { text: string | null }) {
  return text ? (
    <FeedbackLine tone="bad">
      <span className="min-w-0 flex-1">{text}</span>
    </FeedbackLine>
  ) : null;
}

/** A nickname box: empty, the account goes by the part of its email before @. */
function Nickname({ id, row, value, onChange }: { id: string; row: ClaudeAccountRow; value: string; onChange(v: string): void }) {
  const local = localName(row);
  return (
    <div>
      <label className={LABEL} htmlFor={id}>
        {t('Tên gọi', 'Nickname')}
      </label>
      <Input id={id} value={value} maxLength={24} placeholder={local} onChange={(e) => onChange(e.target.value)} />
      <Hint className="mb-0">
        {t(
          'Hiện thay cho phần trước @ ở popover, cửa sổ này và Lịch sử. Để trống để dùng ' + local + '.',
          'Shown instead of the part before @ in the popover, this window and History. Leave empty to use ' + local + '.',
        )}
      </Hint>
    </div>
  );
}

function AddBody({ d, view, claude, nameOf }: { d: Extract<ClaudeDialog, { kind: 'add' }> } & Omit<BodyProps, 'advanced' | 'onTab'>) {
  const using = view.accounts.find((a) => a.active);
  const options: { v: AddHow; t: string; s: string }[] = [
    {
      v: 'browser',
      t: t('Đăng nhập trong trình duyệt', 'Sign in with the browser'),
      s: view.claude
        ? t('Mở trang đăng nhập của Claude. Xong thì quay lại đây.', 'Opens Claude’s sign-in page. Come back here when done.')
        : t('Không thấy lệnh claude trên máy này.', 'The claude command was not found on this Mac.'),
    },
    {
      v: 'current',
      t: t('Lấy tài khoản Claude Code đang đăng nhập', 'Use the account Claude Code is signed in to'),
      s: using
        ? t('Claude Code đang dùng ' + nameOf(using.slot) + ', đã có trong danh sách.', 'Claude Code uses ' + nameOf(using.slot) + ', already in the list.')
        : t('Tài khoản `claude` trong Terminal đang đăng nhập.', 'The account `claude` in Terminal is signed in to.'),
    },
    {
      v: 'token',
      t: t('Dán setup-token', 'Paste a setup-token'),
      s: t('Token từ `claude setup-token`, cho máy không mở được trình duyệt.', 'A token from `claude setup-token`, for a Mac that cannot open a browser.'),
    },
  ];
  if (view.importable)
    options.push({
      v: 'import',
      t: view.ready ? t('Nhập lại từ claude-swap', 'Import again from claude-swap') : t('Nhập từ claude-swap', 'Import from claude-swap'),
      s: t('Mang các tài khoản sang, giữ nguyên số, không phải đăng nhập lại. claude-swap vẫn để nguyên.', 'Brings the accounts over with the same numbers and no new sign-in. claude-swap stays as it is.'),
    });
  const blocked = (d.how === 'browser' && !view.claude) || claude.pending || view.working != null;
  return (
    <Stack>
      <Head title={t('Thêm tài khoản Claude', 'Add a Claude account')} lead={t('Đăng nhập chỉ lưu trên Mac này.', 'The sign-in is stored on this Mac only.')} />
      <div>
        <ChoiceGroup
          id="qs-addhow"
          label={t('Cách thêm', 'How to add')}
          labelClassName="sr-only"
          columns={1}
          options={options}
          value={d.how}
          onChange={(how) => claude.open({ kind: 'add', how, error: null })}
        />
      </div>
      <ErrorLine text={d.error} />
      <DialogButtons>
        <Button onClick={claude.close}>{t('Thôi', 'Cancel')}</Button>
        <Button primary disabled={blocked} onClick={() => void claude.add(d.how)}>
          {t('Tiếp tục', 'Continue')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

/** `9:12`. */
export function minutes(ms: number): string {
  const s = Math.max(0, Math.ceil(ms / 1000));
  return Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0');
}

function WaitBody({ d, view, claude, nameOf }: { d: Extract<ClaudeDialog, { kind: 'wait' }> } & Omit<BodyProps, 'advanced' | 'onTab'>) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  const row = d.slot != null ? view.accounts.find((a) => a.slot === d.slot) : undefined;
  const who = d.slot != null ? nameOf(d.slot) : '';
  return (
    <Stack>
      {d.slot != null ? (
        <Head
          title={t('Đăng nhập lại ' + who, 'Sign ' + who + ' in again')}
          lead={
            row?.email
              ? t('Đăng nhập đúng **' + row.email + '** trong tab vừa mở.', 'Sign in as **' + row.email + '** in the tab that opened.')
              : t('Đăng nhập đúng tài khoản này trong tab vừa mở.', 'Sign in to this account in the tab that opened.')
          }
        />
      ) : (
        <Head
          title={t('Đang chờ trình duyệt…', 'Waiting for the browser…')}
          lead={t('Đăng nhập Claude trong tab vừa mở. Cửa sổ này tự cập nhật khi xong.', 'Sign in to Claude in the tab that opened. This window updates by itself when you are done.')}
        />
      )}
      <div className="flex items-start gap-2.5 rounded-card border border-line bg-card-2 px-3 py-2.5 text-sm" role="status">
        <Spinner />
        <div className="grid gap-0.5">
          <b className="font-semi text-heading">{t('Chờ đăng nhập', 'Waiting for sign-in')}</b>
          <small className="text-[12.5px] text-ink2 tabular-nums">
            {t('Chờ tối đa 10 phút · còn ' + minutes(LOGIN_WAIT_MS - (now - d.started)), 'Waits up to 10 minutes · ' + minutes(LOGIN_WAIT_MS - (now - d.started)) + ' left')}
          </small>
        </div>
      </div>
      <DialogButtons>
        <Button onClick={claude.cancelLogin}>{t('Thôi', 'Cancel')}</Button>
      </DialogButtons>
    </Stack>
  );
}

function TokenBody({ d, claude, view }: { d: Extract<ClaudeDialog, { kind: 'token' }> } & Omit<BodyProps, 'advanced' | 'nameOf' | 'onTab'>) {
  const [token, setToken] = useState('');
  const [email, setEmail] = useState('');
  const add = () => {
    void claude.addToken(token, email.trim() || null);
    setToken('');
  };
  return (
    <Stack>
      <Head
        title={t('Dán setup-token', 'Paste a setup-token')}
        lead={t(
          'Chạy `claude setup-token` ở máy có trình duyệt, rồi dán token vào đây. Token chỉ lưu trong Keychain của máy này.',
          'Run `claude setup-token` on a computer with a browser, then paste the token here. It is stored only in this Mac’s Keychain.',
        )}
      />
      <div>
        <label className={LABEL} htmlFor="qs-tok">
          Token
        </label>
        <Input id="qs-tok" type="password" mono value={token} autoComplete="off" spellCheck={false} placeholder="sk-ant-oat01-…" onChange={(e) => setToken(e.target.value)} />
      </div>
      <div>
        <label className={LABEL} htmlFor="qs-tok-email">
          {t('Email của tài khoản (không bắt buộc)', 'The account’s email (optional)')}
        </label>
        <Input id="qs-tok-email" value={email} onChange={(e) => setEmail(e.target.value)} />
        <Hint className="mb-0">{t('Token không có email, nên ghi email để dễ nhận ra.', 'A token carries no email, so add one to tell it apart.')}</Hint>
      </div>
      <ErrorLine text={d.error} />
      <DialogButtons>
        <Button onClick={claude.close}>{t('Thôi', 'Cancel')}</Button>
        <Button primary disabled={!token.trim() || claude.pending || view.working != null} onClick={add}>
          {t('Thêm', 'Add')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function DoneBody({ d, view, claude, advanced, onTab }: { d: Extract<ClaudeDialog, { kind: 'done' }> } & Omit<BodyProps, 'nameOf'>) {
  const [alias, setAlias] = useState('');
  const row = view.accounts.find((a) => a.slot === d.slot);
  if (!row) return <Gone claude={claude} />;
  const local = localName(row);
  const auto = rotationMatters(view, advanced);
  return (
    <Stack>
      <Head
        title={t('Đã thêm ' + local, 'Added ' + local)}
        lead={row.kind === 'setupToken' ? t('Setup token · lưu trên Mac này.', 'Setup token · stored on this Mac.') : t('Đăng nhập lưu trên Mac này.', 'Sign-in stored on this Mac.')}
      />
      <Facts
        items={[
          {
            k: 'ok',
            text: t(
              'Đăng nhập lưu trong Keychain của Quota Station trên Mac này. Hạn mức có ở lần đọc tới.',
              'The sign-in is in Quota Station’s Keychain on this Mac. Quota shows up on the next read.',
            ),
          },
          ...(auto
            ? [
                {
                  k: 'w',
                  text: advanced
                    ? t(
                        'Luật xoay vòng chưa có tài khoản này: switcher chưa tự chuyển sang nó. Thêm vào Team trong vòng nếu muốn.',
                        'The rotation rules do not have this account yet, so the switcher will not move to it. Add it to Team in rotation if you want.',
                      )
                    : t(
                        'Tự chuyển dùng được tài khoản này. Muốn để nó ngoài, chọn Cho nghỉ khỏi tự chuyển trong menu ⋯.',
                        'Auto-switch may use this account. To keep it out, choose Leave out of auto-switch in its ⋯ menu.',
                      ),
                },
              ]
            : []),
        ]}
      />
      <Nickname id="qs-al-new" row={row} value={alias} onChange={setAlias} />
      <DialogButtons>
        {auto && advanced ? (
          <Button
            onClick={() => {
              void claude.finish(d.slot, alias);
              onTab('rules');
            }}
          >
            {t('Mở Luật xoay vòng', 'Open Rotation rules')}
          </Button>
        ) : null}
        <Button primary onClick={() => void claude.finish(d.slot, alias)}>
          {t('Xong', 'Done')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function DupBody({ d, claude, nameOf }: { d: Extract<ClaudeDialog, { kind: 'dup' }> } & Pick<BodyProps, 'claude' | 'nameOf'>) {
  const who = nameOf(d.slot);
  return (
    <Stack>
      <Head
        title={t('Tài khoản này đã có', 'This account is already here')}
        lead={t('Đăng nhập vừa xong là ' + who + '. Không thêm lần nữa.', 'The sign-in you just finished is ' + who + '. It is not added twice.')}
      />
      <Facts items={[{ k: 'ok', text: t('Đăng nhập của ' + who + ' được làm mới bằng lần đăng nhập này.', who + '’s sign-in was refreshed with this one.') }]} />
      <DialogButtons>
        <Button onClick={() => claude.open({ kind: 'add', how: 'browser', error: null })}>{t('Đăng nhập tài khoản khác', 'Sign in to another account')}</Button>
        <Button primary onClick={claude.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function FailBody({ d, claude, view }: { d: Extract<ClaudeDialog, { kind: 'fail' }> } & Pick<BodyProps, 'claude' | 'view'>) {
  return (
    <Stack>
      <Head title={t('Chưa đăng nhập được', 'Sign-in did not finish')} lead={d.text + t(' Không có gì được lưu.', ' Nothing was saved.')} />
      <DialogButtons>
        <Button onClick={() => claude.open({ kind: 'token', error: null })}>{t('Dán setup-token', 'Paste a setup-token')}</Button>
        <Button onClick={claude.close}>{t('Đóng', 'Close')}</Button>
        <Button primary disabled={!view.claude || view.working != null} onClick={() => void claude.login(null)}>
          {t('Thử lại', 'Try again')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function DetailBody({ d, view, claude, advanced, nameOf }: { d: Extract<ClaudeDialog, { kind: 'detail' }> } & Omit<BodyProps, 'onTab'>) {
  const row = view.accounts.find((a) => a.slot === d.slot);
  const [alias, setAlias] = useState(row?.alias ?? '');
  if (!row) return <Gone claude={claude} />;
  const who = nameOf(row.slot);
  const quota = quotaLine(row);
  return (
    <Stack>
      <Head title={who} cap={t('Tài khoản Claude', 'Claude account')} />
      <Nickname id="qs-al" row={row} value={alias} onChange={setAlias} />
      <dl className="m-0 grid grid-cols-[max-content_minmax(0,1fr)] gap-x-4 gap-y-1.5 text-sm [&_dd]:m-0 [&_dt]:text-ink2">
        {row.email ? (
          <>
            <dt>Email</dt>
            <dd>{row.email}</dd>
          </>
        ) : null}
        <dt>{t('Loại', 'Kind')}</dt>
        <dd>{row.kind === 'setupToken' ? 'Setup token' : t('Đăng nhập trình duyệt', 'Browser sign-in')}</dd>
        <dt>{t('Đăng nhập', 'Sign-in')}</dt>
        <dd>
          <Chip chip={signInChip(row)} />
        </dd>
        <dt>{t('Hạn mức', 'Quota')}</dt>
        <dd className="tabular-nums">{quota || t('chưa có số', 'no numbers yet')}</dd>
      </dl>
      {rotationMatters(view, advanced) ? (
        <div className="flex items-center gap-3">
          <div className="min-w-0 flex-1">
            <b className="block text-body font-med text-heading">{t('Cho tự chuyển dùng tài khoản này', 'Let auto-switch use this account')}</b>
            <small className="block text-[12.5px] text-ink2">
              {t('Tắt để tự chuyển trên máy này bỏ qua ' + who + '. Bạn vẫn chuyển tay được.', 'Turn off so auto-switch on this Mac skips ' + who + '. You can still switch by hand.')}
            </small>
          </div>
          <Toggle
            on={row.enabled}
            label={t('Cho tự chuyển dùng tài khoản này', 'Let auto-switch use this account')}
            disabled={view.working != null}
            onClick={() => (row.enabled ? claude.open({ kind: 'rest', slot: row.slot }) : claude.setEnabled(row.slot, true))}
          />
        </div>
      ) : null}
      <DialogButtons className="items-center">
        <LinkButton bad className="mr-auto" onClick={() => claude.open({ kind: 'remove', slot: row.slot })}>
          {t('Xoá khỏi máy này…', 'Remove from this Mac…')}
        </LinkButton>
        <Button onClick={claude.close}>{t('Thôi', 'Cancel')}</Button>
        <Button
          primary
          disabled={view.working != null}
          onClick={() => (alias.trim() === (row.alias ?? '') ? claude.close() : claude.setAlias(row.slot, alias))}
        >
          {t('Lưu', 'Save')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function RestBody({ d, claude, nameOf, view }: { d: Extract<ClaudeDialog, { kind: 'rest' }> } & Pick<BodyProps, 'claude' | 'nameOf' | 'view'>) {
  const who = nameOf(d.slot);
  return (
    <Stack>
      <Head title={t('Cho ' + who + ' nghỉ khỏi tự chuyển?', 'Leave ' + who + ' out of auto-switch?')} />
      <Facts
        items={[
          { k: '', text: t('Tự chuyển bỏ qua ' + who + ' cho tới khi bạn cho vào lại.', 'Auto-switch skips ' + who + ' until you put it back.') },
          { k: '', text: t('Bạn vẫn chuyển tay sang ' + who + ' được; hạn mức vẫn hiện như cũ.', 'You can still switch to ' + who + ' by hand; its quota still shows.') },
        ]}
      />
      <DialogButtons>
        <Button onClick={claude.close}>{t('Thôi', 'Cancel')}</Button>
        <Button primary disabled={view.working != null} onClick={() => claude.setEnabled(d.slot, false)}>
          {t('Cho nghỉ', 'Leave out')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function RemoveBody({ d, claude, nameOf, view, advanced }: { d: Extract<ClaudeDialog, { kind: 'remove' }> } & Omit<BodyProps, 'onTab'>) {
  const who = nameOf(d.slot);
  const row = view.accounts.find((a) => a.slot === d.slot);
  if (!row) return <Gone claude={claude} />;
  if (row.active)
    return (
      <Stack>
        <Head title={t('Chưa xoá được ' + who, 'Cannot remove ' + who + ' yet')} />
        <Facts items={[{ k: 'x', text: t(who + ' đang dùng. Chuyển sang tài khoản khác trước khi xoá.', who + ' is in use. Switch to another account before removing it.') }]} />
        <DialogButtons>
          <Button primary onClick={claude.close}>
            {t('Đóng', 'Close')}
          </Button>
        </DialogButtons>
      </Stack>
    );
  return (
    <Stack>
      <Head title={t('Xoá ' + who + ' khỏi máy này?', 'Remove ' + who + ' from this Mac?')} />
      <Facts
        items={[
          {
            k: '',
            text: t(
              'Quota Station quên đăng nhập của ' + who + ' trên Mac này. Tài khoản Claude vẫn còn nguyên.',
              'Quota Station forgets ' + who + '’s sign-in on this Mac. The Claude account itself is untouched.',
            ),
          },
          ...(advanced
            ? [
                {
                  k: 'w',
                  text: t(
                    'Luật xoay vòng vẫn có tài khoản này trong Team trong vòng. Sửa luật sau khi xoá.',
                    'The rotation rules still list this account in Team in rotation. Edit the rules after removing it.',
                  ),
                },
              ]
            : []),
          {
            k: '',
            text: t(
              'Lịch sử giữ các lần chuyển cũ của ' + who + '. Muốn dùng lại: Thêm tài khoản rồi đăng nhập.',
              'History keeps ' + who + '’s past switches. To use it again: Add account and sign in.',
            ),
          },
        ]}
      />
      <DialogButtons>
        <Button onClick={claude.close}>{t('Thôi', 'Cancel')}</Button>
        <Button danger disabled={view.working != null} onClick={() => claude.remove(d.slot)}>
          {t('Xoá khỏi máy này', 'Remove from this Mac')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function ReloginBody({ d, claude, nameOf, view }: { d: Extract<ClaudeDialog, { kind: 'relogin' }> } & Pick<BodyProps, 'claude' | 'nameOf' | 'view'>) {
  const who = nameOf(d.slot);
  const row = view.accounts.find((a) => a.slot === d.slot);
  return (
    <Stack>
      <Head
        title={t('Đăng nhập lại ' + who + '?', 'Sign ' + who + ' in again?')}
        lead={
          row?.email
            ? t(
                'Đăng nhập của ' + who + ' trong Quota Station đã hết hạn. Mở trang đăng nhập của Claude và đăng nhập đúng **' + row.email + '**.',
                who + '’s sign-in in Quota Station has expired. Open Claude’s sign-in page and sign in as **' + row.email + '**.',
              )
            : t(
                'Đăng nhập của ' + who + ' trong Quota Station đã hết hạn. Mở trang đăng nhập của Claude và đăng nhập đúng tài khoản này.',
                who + '’s sign-in in Quota Station has expired. Open Claude’s sign-in page and sign in to this account.',
              )
        }
      />
      <DialogButtons>
        <Button onClick={claude.close}>{t('Thôi', 'Cancel')}</Button>
        <Button
          primary
          disabled={!view.claude || view.working != null}
          title={view.claude ? undefined : t('Không thấy lệnh claude trên máy này', 'The claude command was not found on this Mac')}
          onClick={() => void claude.login(d.slot)}
        >
          {t('Mở trang đăng nhập', 'Open the sign-in page')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

/** The account went while its dialog was open. */
function Gone({ claude }: { claude: ClaudeAccountsState }) {
  return (
    <Stack>
      <Head title={t('Không còn tài khoản này', 'This account is gone')} lead={t('Danh sách vừa đổi.', 'The list just changed.')} />
      <DialogButtons>
        <Button primary onClick={claude.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

export function ClaudeDialogBody(props: BodyProps) {
  const d = props.claude.dialog;
  if (!d) return null;
  switch (d.kind) {
    case 'add':
      return <AddBody d={d} {...props} />;
    case 'wait':
      return <WaitBody d={d} {...props} />;
    case 'token':
      return <TokenBody d={d} {...props} />;
    case 'done':
      return <DoneBody key={d.slot} d={d} {...props} />;
    case 'dup':
      return <DupBody d={d} {...props} />;
    case 'fail':
      return <FailBody d={d} {...props} />;
    case 'detail':
      return <DetailBody key={d.slot} d={d} {...props} />;
    case 'rest':
      return <RestBody d={d} {...props} />;
    case 'remove':
      return <RemoveBody d={d} {...props} />;
    case 'relogin':
      return <ReloginBody d={d} {...props} />;
  }
}
