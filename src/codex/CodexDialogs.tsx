// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Codex tab's account dialogs, laid out as the Claude tab's are: the
// ways to add an account, the wait for the browser, added with its
// nickname, already here, not signed in; a custom provider's form with
// "Kiểm tra" and the preview of the lines it adds; an account's details and
// nickname; removing it; signing it in again. A key typed here lives only
// in its dialog, goes one way to the app and comes back only as its hint.

import { useEffect, useState, type ReactNode } from 'react';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { DiffFile, DiffLine, DiffView, KeyMasked } from '../components/blocks';
import { Button, Chip, ChoiceGroup, Facts, FeedbackLine, Hint, LinkButton, Rich, Spinner } from '../components/parts';
import { t } from '../lib/i18n';
import { accountName, accountOf, accountRows, checkText, loginFailText, refusedText } from '../lib/codexAccounts';
import { keyRefusalText } from '../lib/codex';
import { dayClock } from '../lib/switching';
import type { CodexAccount, CodexKeyPreview, CodexProviderCheck, Model } from '../lib/model';
import type { Action } from '../lib/view';
import type { CodexAccountsState, CodexAddHow, CodexDialog } from './useCodexAccounts';

/** How long `codex login` waits for the browser (the app's own limit). */
export const LOGIN_WAIT_MS = 10 * 60 * 1000;

interface BodyProps {
  codex: CodexAccountsState;
  model: Model;
  now: Date;
  onAction(a: Action): void;
  /** The newest model, for the line after an action. */
  latest(): Model | null;
}

const LABEL = 'mb-1 block text-xs font-semi text-ink2';

function Head({ title, lead, cap }: { title: string; lead?: string; cap?: string }) {
  return (
    <div className="grid gap-1">
      {cap ? <span className="text-xs font-semi text-ink3">{cap}</span> : null}
      <DialogTitle className="mb-0">{title}</DialogTitle>
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

/** `9:12`. */
function minutes(ms: number): string {
  const s = Math.max(0, Math.ceil(ms / 1000));
  return Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0');
}

/** The name the account goes by without a nickname. */
function ownName(a: CodexAccount): string {
  if (a.kind === 'custom') return a.custom?.name ?? a.id;
  return a.email ? a.email.slice(0, a.email.indexOf('@')) : 'ChatGPT ' + (a.number ?? '');
}

function Nickname({ id, account, value, onChange }: { id: string; account: CodexAccount; value: string; onChange(v: string): void }) {
  const own = ownName(account);
  return (
    <div>
      <label className={LABEL} htmlFor={id}>
        {t('Tên gọi', 'Nickname')}
      </label>
      <Input id={id} value={value} maxLength={24} placeholder={own} onChange={(e) => onChange(e.target.value)} />
      <Hint className="mb-0">
        {account.kind === 'custom'
          ? t('Chỉ đổi tên app hiện. Bảng trong config.toml giữ tên ' + own + '. Để trống để dùng ' + own + '.', 'Only changes the name the app shows. The table in config.toml keeps the name ' + own + '. Leave empty to use ' + own + '.')
          : t('Hiện thay cho phần trước @ ở popover, cửa sổ này và Lịch sử. Để trống để dùng ' + own + '.', 'Shown instead of the part before @ in the popover, this window and History. Leave empty to use ' + own + '.')}
      </Hint>
    </div>
  );
}

function AddBody({ d, codex, model }: { d: Extract<CodexDialog, { kind: 'add' }> } & BodyProps) {
  const v = model.codex;
  const cli = v.version != null;
  const options: { v: CodexAddHow; t: string; s: string }[] = [
    {
      v: 'chatgpt',
      t: t('Đăng nhập ChatGPT trong trình duyệt', 'Sign in to ChatGPT in the browser'),
      s: cli
        ? t(
            'Mở trang đăng nhập của ChatGPT. Đăng nhập nằm trong thư mục riêng của tài khoản này; Codex đang chạy không bị đụng tới.',
            'Opens ChatGPT’s sign-in page. The sign-in goes into this account’s own folder; the Codex that runs is not touched.',
          )
        : t('Không thấy lệnh codex trên máy này.', 'The codex command was not found on this Mac.'),
    },
    {
      v: 'custom',
      t: t('Nhà cung cấp API riêng (Custom API)', 'Your own API provider (Custom API)'),
      s: t('Tên, địa chỉ và khoá API. Khoá được ghi vào config.toml của Codex và không hiện lại.', 'A name, an address and an API key. The key goes into Codex’s config.toml and is never shown again.'),
    },
  ];
  const blocked = (d.how === 'chatgpt' && !cli) || codex.pending || v.busy != null;
  return (
    <Stack>
      <Head title={t('Thêm tài khoản Codex', 'Add a Codex account')} lead={t('Chỉ lưu trên Mac này.', 'Stored on this Mac only.')} />
      <ChoiceGroup
        id="qs-codex-addhow"
        label={t('Cách thêm', 'How to add')}
        labelClassName="sr-only"
        columns={1}
        options={options}
        value={d.how}
        onChange={(how) => codex.open({ kind: 'add', how, error: null })}
      />
      <ErrorLine text={d.error} />
      <DialogButtons>
        <Button onClick={codex.close}>{t('Thôi', 'Cancel')}</Button>
        <Button primary disabled={blocked} onClick={() => codex.add(d.how)}>
          {t('Tiếp tục', 'Continue')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function WaitBody({ d, codex, model }: { d: Extract<CodexDialog, { kind: 'wait' }> } & BodyProps) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  const a = d.id ? accountOf(model.codex, d.id) : undefined;
  const who = a ? accountName(model, a) : '';
  return (
    <Stack>
      {a ? (
        <Head
          title={t('Đăng nhập lại ' + who, 'Sign ' + who + ' in again')}
          lead={a.email ? t('Đăng nhập đúng **' + a.email + '** trong tab vừa mở.', 'Sign in as **' + a.email + '** in the tab that opened.') : t('Đăng nhập đúng tài khoản này trong tab vừa mở.', 'Sign in to this account in the tab that opened.')}
        />
      ) : (
        <Head
          title={t('Đang chờ trình duyệt…', 'Waiting for the browser…')}
          lead={t('Đăng nhập ChatGPT trong tab vừa mở. Cửa sổ này tự cập nhật khi xong.', 'Sign in to ChatGPT in the tab that opened. This window updates by itself when you are done.')}
        />
      )}
      <div className="flex items-start gap-2.5 rounded-tile bg-fill px-[11px] py-2 text-sm" role="status">
        <Spinner />
        <div className="grid gap-0.5">
          <b className="font-semi text-heading">{t('Chờ đăng nhập', 'Waiting for sign-in')}</b>
          <small className="text-xs text-ink2 tabular-nums">
            {t('Chờ tối đa 10 phút · còn ' + minutes(LOGIN_WAIT_MS - (now - d.started)), 'Waits up to 10 minutes · ' + minutes(LOGIN_WAIT_MS - (now - d.started)) + ' left')}
          </small>
        </div>
      </div>
      <Hint className="mb-0">
        {t('Không thấy tab? ', 'No tab? ')}
        <LinkButton small onClick={() => {}}>
          {t('Mở lại trang đăng nhập', 'Open the sign-in page again')}
        </LinkButton>
      </Hint>
      <DialogButtons>
        <Button onClick={codex.cancelLogin}>{t('Thôi', 'Cancel')}</Button>
      </DialogButtons>
    </Stack>
  );
}

/** Added: its nickname, and the switch when the owner wants it now. */
function DoneBody({ d, codex, model, onAction, latest }: { d: Extract<CodexDialog, { kind: 'done' | 'providerDone' }> } & BodyProps) {
  const [alias, setAlias] = useState('');
  const a = accountOf(model.codex, d.id);
  if (!a) return <Gone codex={codex} />;
  const own = ownName(a);
  const using = model.codex.accounts.find((x) => x.active);
  const finish = (then?: Action) => {
    if (alias.trim()) codex.rename(a.id, alias, latest);
    else codex.close();
    if (then) onAction(then);
  };
  return (
    <Stack>
      <Head
        title={t('Đã thêm ' + own, 'Added ' + own)}
        lead={a.kind === 'chatgpt' ? 'ChatGPT' + (a.plan ? ' ' + a.plan : '') + t(' · đăng nhập lưu trên Mac này.', ' · sign-in stored on this Mac.') : 'Custom API · ' + (a.custom?.baseUrl ?? '')}
      />
      <Facts
        items={[
          a.kind === 'chatgpt'
            ? {
                k: 'ok',
                text: t(
                  'Đăng nhập nằm trong thư mục riêng của tài khoản này trong app, chỉ bạn đọc được. Hạn mức có ngay ở lần đọc tới.',
                  'The sign-in is in this account’s own folder in the app, readable only by you. Its quota shows on the next read.',
                ),
              }
            : {
                k: 'ok',
                text: t('Bảng `[model_providers.' + a.id + ']` đã vào config.toml, kèm khoá. Bản sao lưu trước đó không chứa khoá.', 'The `[model_providers.' + a.id + ']` table is in config.toml, with its key. The backup taken first holds no key.'),
              },
          {
            k: '',
            text: t(
              'Codex chưa đổi: vẫn dùng ' + (using ? accountName(model, using) : t('tài khoản hiện tại', 'the current account')) + '. Bấm Chuyển khi muốn dùng ' + own + '.',
              'Codex has not changed: it still uses ' + (using ? accountName(model, using) : 'the current account') + '. Press Switch when you want ' + own + '.',
            ),
          },
        ]}
      />
      <Nickname id="qs-cx-al-new" account={a} value={alias} onChange={setAlias} />
      <DialogButtons>
        <Button onClick={() => finish('codex:use:' + a.id)}>{t('Chuyển sang ' + (alias.trim() || own) + '…', 'Switch to ' + (alias.trim() || own) + '…')}</Button>
        <Button primary onClick={() => finish()}>
          {t('Xong', 'Done')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function DupBody({ d, codex, model }: { d: Extract<CodexDialog, { kind: 'dup' }> } & BodyProps) {
  const a = accountOf(model.codex, d.id);
  const who = a ? accountName(model, a) : d.id;
  return (
    <Stack>
      <Head title={t('Tài khoản này đã có', 'This account is already here')} lead={t('Đăng nhập vừa xong là ' + who + '. Không thêm lần nữa.', 'The sign-in you just finished is ' + who + '. It is not added twice.')} />
      <Facts items={[{ k: 'ok', text: t('Đăng nhập vừa tạo đã được đăng xuất; đăng nhập cũ của ' + who + ' giữ nguyên.', 'The new sign-in was signed out; ' + who + '’s own sign-in stays as it was.') }]} />
      <DialogButtons>
        <Button onClick={() => void codex.login(null)}>{t('Đăng nhập tài khoản khác', 'Sign in to another account')}</Button>
        <Button primary onClick={codex.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function FailBody({ d, codex, model }: { d: Extract<CodexDialog, { kind: 'fail' }> } & BodyProps) {
  return (
    <Stack>
      <Head title={t('Chưa đăng nhập được', 'Sign-in did not finish')} lead={loginFailText(d.reason, d.detail) + t(' Không có gì được lưu.', ' Nothing was saved.')} />
      <DialogButtons>
        {d.reason === 'port' ? (
          <Button onClick={() => void codex.login(d.id)}>{t('Dùng mã thiết bị', 'Use a device code')}</Button>
        ) : null}
        <Button onClick={codex.close}>{t('Đóng', 'Close')}</Button>
        <Button primary disabled={model.codex.version == null || model.codex.busy != null} onClick={() => void codex.login(d.id)}>
          {t('Thử lại', 'Try again')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

/** A custom provider: the form, then the lines it adds. The key lives only here. */
function ProviderBody({ codex, model, latest }: BodyProps) {
  const v = model.codex;
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');
  const [key, setKey] = useState('');
  const [check, setCheck] = useState<CodexProviderCheck | 'checking' | null>(null);
  const [preview, setPreview] = useState<CodexKeyPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [writing, setWriting] = useState(false);
  const api = codex.api;
  const filled = !!name.trim() && !!url.trim() && !!key.trim();
  const edit = (f: (s: string) => void) => (e: { target: { value: string } }) => {
    f(e.target.value);
    setCheck(null);
    setError(null);
  };
  const runCheck = async () => {
    setCheck('checking');
    try {
      setCheck(await api.codexCheckProvider(url.trim(), key));
    } catch {
      setCheck(null);
      setError(t('Không kiểm được. Chưa đổi gì.', 'Could not check. Nothing changed.'));
    }
  };
  const look = async () => {
    setError(null);
    try {
      const p = await api.codexProviderPreview(name, url, key);
      if (p.refusal) setError(keyRefusalText(v, p.refusal));
      else setPreview(p);
    } catch {
      setError(t('Không gọi được lệnh xem trước. Chưa đổi gì.', 'Could not run the preview. Nothing changed.'));
    }
  };
  const write = async () => {
    if (!preview?.hash && v.config !== 'missing') return;
    setWriting(true);
    const d = await api.codexAddProvider(name, url, key, preview?.hash ?? '').catch(() => ({ kind: 'refused', reason: 'busy' }) as const);
    setWriting(false);
    if (d.kind === 'added') {
      setKey('');
      codex.open({ kind: 'providerDone', id: d.id });
    } else if (d.kind === 'refused') {
      setPreview(null);
      setError(refusedText(d.reason));
    }
    void latest;
  };
  if (writing)
    return (
      <Stack>
        <Head title={t('Đang thêm ' + name.trim() + '…', 'Adding ' + name.trim() + '…')} />
        <FeedbackLine tone="info">
          <Spinner />
          <span className="min-w-0 flex-1">{t('Sao lưu bản cũ (không chứa khoá) → ghi file tạm → kiểm mã băm → thay file → đọc lại.', 'Back up the old file (without keys) → write a temporary file → check its hash → replace the file → read it again.')}</span>
        </FeedbackLine>
      </Stack>
    );
  const checked = check && check !== 'checking' ? checkText(check) : null;
  if (preview)
    return (
      <Stack>
        <Head title={t('Thêm ' + name.trim() + ' vào config.toml?', 'Add ' + name.trim() + ' to config.toml?')} />
        <DiffView head={<DiffFile>{v.home}/config.toml</DiffFile>}>
          {preview.lines.map((l, i) => (
            <DiffLine key={i} op={l.op}>
              {l.text}
              {l.secret ? <KeyMasked /> : null}
            </DiffLine>
          ))}
        </DiffView>
        <Facts
          items={[
            { k: 'ok', text: t('Thêm một bảng mới ở cuối file; mọi dòng khác giữ nguyên. Sao lưu bản cũ (không chứa khoá) trước khi ghi.', 'Adds one new table at the end of the file; every other line stays. Backs up the old file (without keys) first.') },
            { k: '', text: t('Codex chưa chuyển sang ' + name.trim() + ': bấm Chuyển khi muốn dùng.', 'Codex does not switch to ' + name.trim() + ' yet: press Switch when you want it.') },
            ...(checked?.tone === 'ok'
              ? []
              : [{ k: 'w' as const, text: checked ? checked.text : t('Chưa kiểm tra địa chỉ và khoá. Vẫn thêm được; Codex sẽ báo lỗi nếu chúng sai.', 'The address and key are not checked. You can still add it; Codex reports an error if they are wrong.') }]),
          ]}
        />
        <DialogButtons>
          <Button onClick={() => setPreview(null)}>{t('Sửa lại', 'Edit')}</Button>
          <Button primary disabled={v.busy != null} onClick={() => void write()}>
            {t('Thêm vào config.toml', 'Add to config.toml')}
          </Button>
        </DialogButtons>
      </Stack>
    );
  return (
    <Stack>
      <Head
        title={t('Thêm Custom API', 'Add a Custom API')}
        lead={t('Codex gửi yêu cầu qua địa chỉ này bằng khoá của bạn. Khoá chỉ ghi vào config.toml, không hiện lại, không vào log hay bản sao lưu.', 'Codex sends its requests to this address with your key. The key goes only into config.toml; it is never shown again or written to a log or backup.')}
      />
      <div className="grid gap-3">
        <div>
          <label className={LABEL} htmlFor="qs-cx-name">
            {t('Tên', 'Name')}
          </label>
          <Input id="qs-cx-name" value={name} maxLength={32} autoFocus placeholder="relay-2" onChange={edit(setName)} />
        </div>
        <div>
          <label className={LABEL} htmlFor="qs-cx-url">
            {t('Địa chỉ (base URL)', 'Address (base URL)')}
          </label>
          <Input id="qs-cx-url" mono value={url} spellCheck={false} placeholder="https://api.example.com/v1" onChange={edit(setUrl)} />
        </div>
        <div>
          <label className={LABEL} htmlFor="qs-cx-key">
            {t('Khoá API', 'API key')}
          </label>
          <Input id="qs-cx-key" type="password" mono value={key} autoComplete="off" spellCheck={false} placeholder="sk-…" onChange={edit(setKey)} />
          <Hint className="mb-0">{t('Kiểm tra gửi khoá tới đúng địa chỉ trên, như Codex sẽ gửi. Không gửi đi đâu khác.', 'Check sends the key to the address above only, as Codex will. Nowhere else.')}</Hint>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2.5">
        <Button disabled={!url.trim() || !key.trim() || check === 'checking'} onClick={() => void runCheck()}>
          {check === 'checking' ? t('Đang kiểm…', 'Checking…') : t('Kiểm tra', 'Check')}
        </Button>
        {checked ? <span className={checked.tone === 'ok' ? 'text-sm text-ok-ink' : checked.tone === 'bad' ? 'text-sm text-bad-ink' : 'text-sm text-warn-ink'}>{checked.text}</span> : null}
      </div>
      <ErrorLine text={error} />
      <DialogButtons>
        <Button onClick={codex.close}>{t('Thôi', 'Cancel')}</Button>
        <Button primary disabled={!filled || v.busy != null} onClick={() => void look()}>
          {t('Xem trước…', 'Preview…')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function DetailBody({ d, codex, model, now, latest }: { d: Extract<CodexDialog, { kind: 'detail' }> } & BodyProps) {
  const a = accountOf(model.codex, d.id);
  const [alias, setAlias] = useState(a?.nickname ? a.name : '');
  if (!a) return <Gone codex={codex} />;
  const row = accountRows(model, now).find((r) => r.id === a.id);
  const who = accountName(model, a);
  const rows: [string, ReactNode][] =
    a.kind === 'chatgpt'
      ? [
          ...(a.email ? [['Email', a.email] as [string, ReactNode]] : []),
          [t('Gói', 'Plan'), a.plan ?? t('chưa biết', 'unknown')],
          [
            t('Đăng nhập', 'Sign-in'),
            a.signedIn ? t('đang nằm trong ', 'in ') + model.codex.home : t('thư mục riêng trong app · chỉ bạn đọc được', 'its own folder in the app · readable only by you'),
          ],
          [t('Hạn mức', 'Quota'), row?.cells ? row.cells.map((c) => c.label + ' ' + c.text).join(' · ') : '—'],
          ...(a.addedAt ? [[t('Thêm lúc', 'Added'), dayClock(a.addedAt, now)] as [string, ReactNode]] : []),
        ]
      : [
          [t('Bảng', 'Table'), '[model_providers.' + a.id + ']'],
          [t('Địa chỉ', 'Address'), a.custom?.baseUrl ?? '—'],
          [t('Khoá', 'Key'), a.custom?.key === 'bearer' ? t('có · trong config.toml', 'yes · in config.toml') : a.custom?.key === 'envKey' ? t('biến ', 'variable ') + (a.custom.envKey ?? '') : t('chưa có', 'none')],
          [t('Trả lời', 'Answers'), row?.sub.split(' · ').slice(2).join(' · ') || '—'],
        ];
  return (
    <Stack>
      <Head title={who} cap={a.kind === 'chatgpt' ? t('Tài khoản ChatGPT', 'ChatGPT account') : 'Custom API'} />
      <Nickname id="qs-cx-al" account={a} value={alias} onChange={setAlias} />
      <dl className="m-0 grid grid-cols-[max-content_minmax(0,1fr)] gap-x-4 gap-y-1.5 text-sm [&_dd]:m-0 [&_dd]:break-all [&_dt]:text-ink2">
        {rows.map(([k, val]) => (
          <div key={k} className="contents">
            <dt>{k}</dt>
            <dd>{val}</dd>
          </div>
        ))}
      </dl>
      {row?.chips.length ? (
        <div className="flex flex-wrap gap-1.5">
          {row.chips.map((c) => (
            <Chip key={c.t} chip={c} />
          ))}
        </div>
      ) : null}
      <DialogButtons>
        <LinkButton style={{ marginRight: 'auto' }} className="text-bad-ink" onClick={() => codex.open({ kind: 'remove', id: a.id })}>
          {t('Xoá khỏi máy này…', 'Remove from this Mac…')}
        </LinkButton>
        <Button onClick={codex.close}>{t('Thôi', 'Cancel')}</Button>
        <Button primary disabled={codex.pending} onClick={() => codex.rename(a.id, alias, latest)}>
          {t('Lưu', 'Save')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function RemoveBody({ d, codex, model, latest }: { d: Extract<CodexDialog, { kind: 'remove' }> } & BodyProps) {
  const [logout, setLogout] = useState(true);
  const a = accountOf(model.codex, d.id);
  if (!a) return <Gone codex={codex} />;
  const who = accountName(model, a);
  const cannot = a.active
    ? t(who + ' đang dùng. Chuyển sang tài khoản khác trước khi xoá.', who + ' is in use. Switch to another account before removing it.')
    : a.signedIn
      ? t('Đăng nhập của ' + who + ' đang nằm trong Codex (giữ cho plugin). Chuyển sang tài khoản ChatGPT khác trước khi xoá.', who + '’s sign-in sits in Codex (kept for plugins). Switch to another ChatGPT account before removing it.')
      : null;
  if (cannot)
    return (
      <Stack>
        <Head title={t('Chưa xoá được ' + who, 'Cannot remove ' + who + ' yet')} />
        <Facts items={[{ k: 'x', text: cannot }]} />
        <DialogButtons>
          <Button primary onClick={codex.close}>
            {t('Đóng', 'Close')}
          </Button>
        </DialogButtons>
      </Stack>
    );
  const sessions = model.codex.resume?.[a.id];
  return (
    <Stack>
      <Head title={a.kind === 'chatgpt' ? t('Xoá ' + who + ' khỏi máy này?', 'Remove ' + who + ' from this Mac?') : t('Xoá ' + who + ' khỏi config.toml?', 'Remove ' + who + ' from config.toml?')} />
      {a.kind === 'chatgpt' ? (
        <>
          <Facts
            items={[
              { k: '', text: t('Quota Station xoá thư mục đăng nhập của ' + who + ' trong app. Codex và tài khoản ChatGPT vẫn còn nguyên.', 'Quota Station deletes ' + who + '’s sign-in folder in the app. Codex and the ChatGPT account stay as they are.') },
              { k: '', text: t('Lịch sử giữ các lần chuyển cũ. Muốn dùng lại: Thêm tài khoản rồi đăng nhập.', 'History keeps the past switches. To use it again: Add account and sign in.') },
            ]}
          />
          <label className="flex cursor-pointer items-start gap-2 text-sm">
            <Checkbox checked={logout} onCheckedChange={(on) => setLogout(on === true)} />
            <span>
              {t('Đăng xuất ' + who + ' trước', 'Sign ' + who + ' out first')}
              <small className="block text-xs text-ink2">{t('Thu hồi đăng nhập này ở OpenAI (`codex logout` trong thư mục của nó).', 'Revokes this sign-in at OpenAI (`codex logout` in its folder).')}</small>
            </span>
          </label>
        </>
      ) : (
        <Facts
          items={[
            { k: '', text: t('Bỏ bảng `[model_providers.' + a.id + ']` khỏi config.toml. Sao lưu bản cũ (không chứa khoá) trước khi ghi.', 'Removes the `[model_providers.' + a.id + ']` table from config.toml. Backs up the old file (without keys) first.') },
            { k: 'w', text: t('Khoá của ' + who + ' đi theo bảng. App không giữ khoá, nên muốn dùng lại phải dán lại.', who + '’s key goes with the table. The app keeps no key, so using it again means pasting it again.') },
            {
              k: '',
              text: t(
                (sessions ? sessions + ' phiên làm trên ' + who : 'Phiên làm trên ' + who) + ' chỉ tiếp tục được (`codex resume`) khi thêm lại đúng tên này.',
                (sessions ? sessions + ' sessions made on ' + who : 'Sessions made on ' + who) + ' can only be resumed (`codex resume`) once a provider of this name is added again.',
              ),
            },
          ]}
        />
      )}
      <DialogButtons>
        <Button onClick={codex.close}>{t('Thôi', 'Cancel')}</Button>
        <Button danger disabled={model.codex.busy != null || codex.pending} onClick={() => codex.remove(a.id, logout, latest)}>
          {a.kind === 'chatgpt' ? t('Xoá khỏi máy này', 'Remove from this Mac') : t('Xoá khỏi config.toml', 'Remove from config.toml')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function ReloginBody({ d, codex, model }: { d: Extract<CodexDialog, { kind: 'relogin' }> } & BodyProps) {
  const a = accountOf(model.codex, d.id);
  if (!a) return <Gone codex={codex} />;
  const who = accountName(model, a);
  return (
    <Stack>
      <Head
        title={t('Đăng nhập lại ' + who + '?', 'Sign ' + who + ' in again?')}
        lead={
          a.email
            ? t('ChatGPT đã từ chối đăng nhập của ' + who + '. Mở trang đăng nhập và đăng nhập đúng **' + a.email + '**.', 'ChatGPT refused ' + who + '’s sign-in. Open the sign-in page and sign in as **' + a.email + '**.')
            : t('ChatGPT đã từ chối đăng nhập của ' + who + '. Mở trang đăng nhập và đăng nhập đúng tài khoản này.', 'ChatGPT refused ' + who + '’s sign-in. Open the sign-in page and sign in to this account.')
        }
      />
      <DialogButtons>
        <Button onClick={codex.close}>{t('Thôi', 'Cancel')}</Button>
        <Button primary disabled={model.codex.version == null || model.codex.busy != null} onClick={() => void codex.login(a.id)}>
          {t('Mở trang đăng nhập', 'Open the sign-in page')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

function Gone({ codex }: { codex: CodexAccountsState }) {
  return (
    <Stack>
      <Head title={t('Không còn tài khoản này', 'This account is gone')} lead={t('Danh sách vừa đổi.', 'The list just changed.')} />
      <DialogButtons>
        <Button primary onClick={codex.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </Stack>
  );
}

export function CodexDialogBody(props: BodyProps) {
  const d = props.codex.dialog;
  if (!d) return null;
  switch (d.kind) {
    case 'add':
      return <AddBody d={d} {...props} />;
    case 'wait':
      return <WaitBody d={d} {...props} />;
    case 'done':
    case 'providerDone':
      return <DoneBody key={d.id} d={d} {...props} />;
    case 'dup':
      return <DupBody d={d} {...props} />;
    case 'fail':
      return <FailBody d={d} {...props} />;
    case 'provider':
      return <ProviderBody {...props} />;
    case 'detail':
      return <DetailBody key={d.id} d={d} {...props} />;
    case 'remove':
      return <RemoveBody key={d.id} d={d} {...props} />;
    case 'relogin':
      return <ReloginBody d={d} {...props} />;
  }
}
