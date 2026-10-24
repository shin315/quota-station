// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, in the Claude tab: import from
// claude-swap, log a new account in, add the login Claude Code is on or a
// setup token, give an account a name, hold it out of the rotation, remove
// it, and turn the rotation's LaunchAgent on or off. A token typed here goes
// to the app and is never shown again.

import { useState } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { LIST_ROW } from '../components/page';
import { BannerBox, Button, Chip, Feedback, FeedbackLine, LinkButton, Spinner, Toggle } from '../components/parts';
import type { Api } from '../lib/api';
import { t } from '../lib/i18n';
import { accountRows, doneFeedback, rotationRow, setupText, workingText, type AccountRowView } from '../lib/claudeAccounts';
import type { ClaudeAccountsView } from '../lib/model';
import type { Action } from '../lib/view';

interface Props {
  view: ClaudeAccountsView;
  api: Api;
  onAction(act: Action): void;
  /** A build with the author's tools: the line on what Firstmate runs shows. */
  authorTools?: boolean;
}

/** The calls reject only when another action runs; the model says the rest. */
function quiet(p: Promise<unknown>) {
  void p.catch(() => {});
}

function Row({ row, view, api }: { row: AccountRowView; view: ClaudeAccountsView; api: Api }) {
  const [renaming, setRenaming] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  const busy = view.working != null;
  if (asking)
    return (
      <div className={LIST_ROW} role="group" aria-label={t('Xoá ', 'Remove ') + row.name}>
        <div>
          <b>{t('Xoá ' + row.name + ' khỏi Quota Station?', 'Remove ' + row.name + ' from Quota Station?')}</b>
          <small className="block text-xs text-ink2">
            {t(
              'Bản đăng nhập app giữ trong Keychain bị xoá. Claude Code và claude-swap không đổi.',
              'The sign-in the app keeps in the Keychain is deleted. Claude Code and claude-swap are unchanged.',
            )}
          </small>
        </div>
        <Button onClick={() => setAsking(false)}>{t('Thôi', 'Cancel')}</Button>
        <Button
          danger
          disabled={busy}
          onClick={() => {
            setAsking(false);
            quiet(api.claudeRemove(row.slot));
          }}
        >
          {t('Xoá', 'Remove')}
        </Button>
      </div>
    );
  if (renaming != null)
    return (
      <div className={LIST_ROW} role="group" aria-label={t('Đổi tên ', 'Rename ') + row.name}>
        <Input
          aria-label={t('Tên mới', 'New name')}
          value={renaming}
          maxLength={24}
          placeholder={t('Tài khoản ', 'Account ') + row.slot}
          onChange={(e) => setRenaming(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              quiet(api.claudeSetAlias(row.slot, renaming));
              setRenaming(null);
            }
          }}
        />
        <Button onClick={() => setRenaming(null)}>{t('Thôi', 'Cancel')}</Button>
        <Button
          primary
          disabled={busy}
          onClick={() => {
            quiet(api.claudeSetAlias(row.slot, renaming));
            setRenaming(null);
          }}
        >
          {t('Lưu', 'Save')}
        </Button>
      </div>
    );
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto_auto_auto] items-center gap-2.5 border-t-[.5px] border-card-line py-[9px] text-sm first:border-t-0">
      <div className="min-w-0">
        <b className="font-semi">{row.name}</b>
        {row.chips.map((c) => (
          <Chip key={c.t} chip={c} className="ml-1.5" />
        ))}
        <small className="block truncate text-xs text-ink2">{row.sub}</small>
      </div>
      <Toggle
        on={row.enabled}
        label={t('Trong vòng xoay: ', 'In rotation: ') + row.name}
        disabled={busy}
        onClick={() => quiet(api.claudeSetEnabled(row.slot, !row.enabled))}
      />
      <Button disabled={busy} onClick={() => setRenaming(row.alias)}>
        {t('Đổi tên', 'Rename')}
      </Button>
      <Button label={t('Xoá ', 'Remove ') + row.name} disabled={busy || !!row.keep} title={row.keep ?? undefined} onClick={() => setAsking(true)}>
        {t('Xoá', 'Remove')}
      </Button>
    </div>
  );
}

function TokenForm({ api, busy, onDone }: { api: Api; busy: boolean; onDone(): void }) {
  const [token, setToken] = useState('');
  const [email, setEmail] = useState('');
  const add = () => {
    quiet(api.claudeAddToken(token, email.trim() || null));
    setToken('');
    onDone();
  };
  return (
    <div className="mt-2 grid gap-2" role="group" aria-label={t('Thêm setup token', 'Add a setup token')}>
      <small className="text-xs text-ink2">
        {t(
          'Chạy `claude setup-token` trong Terminal rồi dán token vào đây. Token không có email, nên ghi email để dễ nhận ra.',
          'Run `claude setup-token` in Terminal, then paste the token here. A token carries no email, so add one to tell it apart.',
        )}
      </small>
      <Input type="password" aria-label="Setup token" mono value={token} autoComplete="off" spellCheck={false} onChange={(e) => setToken(e.target.value)} />
      <Input
        aria-label={t('Email của tài khoản (không bắt buộc)', 'The account’s email (optional)')}
        placeholder={t('Email (không bắt buộc)', 'Email (optional)')}
        value={email}
        onChange={(e) => setEmail(e.target.value)}
      />
      <div className="flex justify-end gap-2.5">
        <Button onClick={onDone}>{t('Thôi', 'Cancel')}</Button>
        <Button primary disabled={busy || !token.trim()} onClick={add}>
          {t('Thêm token', 'Add token')}
        </Button>
      </div>
    </div>
  );
}

export function ClaudeAccounts({ view, api, onAction, authorTools = false }: Props) {
  const [tokenOpen, setTokenOpen] = useState(false);
  const busy = view.working != null;
  const setup = setupText(view);
  const rows = accountRows(view);
  const rotation = rotationRow(view);
  const working = workingText(view);
  const done = doneFeedback(view);
  return (
    <Panel role="region" aria-label={t('Tài khoản Quota Station giữ', 'Accounts Quota Station keeps')}>
      <CardTitle>
        {t('Tài khoản Quota Station giữ', 'Accounts Quota Station keeps')}
        <span className="ml-auto font-reg">
          <LinkButton onClick={() => onAction('guide:switcher')}>{t('Hướng dẫn chuyển từ claude-swap', 'How to move from claude-swap')}</LinkButton>
        </span>
      </CardTitle>
      {setup ? <BannerBox tone="info" title={setup.title} body={setup.body} style={{ margin: '4px 0 8px' }} /> : null}
      {rows.length ? (
        <div role="list" aria-label={t('Tài khoản Claude', 'Claude accounts')}>
          {rows.map((row) => (
            <div key={row.slot} role="listitem" aria-label={row.name} className="border-t-[.5px] border-card-line first:border-t-0 *:border-t-0">
              <Row row={row} view={view} api={api} />
            </div>
          ))}
        </div>
      ) : null}
      <div className="mt-2 flex flex-wrap items-center gap-2.5">
        {view.importable ? (
          <Button primary={!view.ready} disabled={busy} onClick={() => quiet(api.claudeImport())}>
            {view.ready ? t('Nhập lại từ claude-swap', 'Import again from claude-swap') : t('Nhập từ claude-swap', 'Import from claude-swap')}
          </Button>
        ) : null}
        <Button disabled={busy || !view.claude} title={view.claude ? undefined : t('Không thấy lệnh claude trên máy này', 'The claude command was not found on this Mac')} onClick={() => quiet(api.claudeAddLogin())}>
          {t('Đăng nhập tài khoản mới', 'Sign in to a new account')}
        </Button>
        <Button disabled={busy} onClick={() => quiet(api.claudeAddCurrent())}>
          {t('Thêm tài khoản đang đăng nhập', 'Add the signed-in account')}
        </Button>
        <Button disabled={busy} onClick={() => setTokenOpen(true)}>
          {t('Thêm setup token', 'Add a setup token')}
        </Button>
      </div>
      {tokenOpen ? <TokenForm api={api} busy={busy} onDone={() => setTokenOpen(false)} /> : null}
      {working ? (
        <FeedbackLine tone="info" style={{ margin: '8px 0 0' }}>
          <Spinner />
          <div className="flex min-w-0 flex-1 items-center justify-between gap-2.5">
            <span className="text-sm">{working}</span>
            {view.working === 'login' ? <Button onClick={() => void api.claudeCancelLogin()}>{t('Huỷ', 'Cancel')}</Button> : null}
          </div>
        </FeedbackLine>
      ) : null}
      {done ? <Feedback fb={done} inline onAction={onAction} style={{ margin: '8px 0 0' }} /> : null}
      <div className="mt-3 flex items-center justify-between gap-2.5 border-t-[.5px] border-card-line pt-3">
        <div className="min-w-0">
          <b className="block text-body font-semi">{t('Tự chuyển bằng Quota Station', 'Auto-switch with Quota Station')}</b>
          <small className="block text-xs text-ink2">{rotation.line}</small>
        </div>
        <Toggle on={rotation.on} label={t('Tự chuyển bằng Quota Station', 'Auto-switch with Quota Station')} disabled={rotation.disabled} onClick={() => quiet(api.claudeSetRotation(!rotation.on))} />
      </div>
      {rotation.legacy ? (
        <BannerBox tone="warn" title={t('Bộ tự chuyển cũ còn chạy', 'The old auto-switcher is still running')} body={rotation.legacy} style={{ margin: '8px 0 0' }}>
          <LinkButton onClick={() => onAction('guide:auto')}>{t('Hướng dẫn tắt', 'How to turn it off')}</LinkButton>
        </BannerBox>
      ) : null}
      {authorTools ? (
        <CardText className="mt-2 mb-0 text-xs text-ink2">
          {t(
            'Firstmate đọc `quota-station claude status --json` và chạy `quota-station claude run N -- …`.',
            'Firstmate reads `quota-station claude status --json` and runs `quota-station claude run N -- …`.',
          )}
        </CardText>
      ) : null}
    </Panel>
  );
}
