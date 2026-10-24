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
      <div className={LIST_ROW} role="group" aria-label={'Xoá ' + row.name}>
        <div>
          <b>Xoá {row.name} khỏi Quota Station?</b>
          <small className="block text-xs text-ink2">Bản đăng nhập app giữ trong Keychain bị xoá. Claude Code và claude-swap không đổi.</small>
        </div>
        <Button onClick={() => setAsking(false)}>Thôi</Button>
        <Button
          danger
          disabled={busy}
          onClick={() => {
            setAsking(false);
            quiet(api.claudeRemove(row.slot));
          }}
        >
          Xoá
        </Button>
      </div>
    );
  if (renaming != null)
    return (
      <div className={LIST_ROW} role="group" aria-label={'Đổi tên ' + row.name}>
        <Input
          aria-label="Tên mới"
          value={renaming}
          maxLength={24}
          placeholder={'Tài khoản ' + row.slot}
          onChange={(e) => setRenaming(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              quiet(api.claudeSetAlias(row.slot, renaming));
              setRenaming(null);
            }
          }}
        />
        <Button onClick={() => setRenaming(null)}>Thôi</Button>
        <Button
          primary
          disabled={busy}
          onClick={() => {
            quiet(api.claudeSetAlias(row.slot, renaming));
            setRenaming(null);
          }}
        >
          Lưu
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
        label={'Trong vòng xoay: ' + row.name}
        disabled={busy}
        onClick={() => quiet(api.claudeSetEnabled(row.slot, !row.enabled))}
      />
      <Button disabled={busy} onClick={() => setRenaming(row.name.startsWith('Tài khoản ') ? '' : row.name)}>
        Đổi tên
      </Button>
      <Button label={'Xoá ' + row.name} disabled={busy || !!row.keep} title={row.keep ?? undefined} onClick={() => setAsking(true)}>
        Xoá
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
    <div className="mt-2 grid gap-2" role="group" aria-label="Thêm setup token">
      <small className="text-xs text-ink2">
        Chạy `claude setup-token` trong Terminal rồi dán token vào đây. Token không có email, nên ghi email để dễ nhận ra.
      </small>
      <Input type="password" aria-label="Setup token" mono value={token} autoComplete="off" spellCheck={false} onChange={(e) => setToken(e.target.value)} />
      <Input aria-label="Email của tài khoản (không bắt buộc)" placeholder="Email (không bắt buộc)" value={email} onChange={(e) => setEmail(e.target.value)} />
      <div className="flex justify-end gap-2.5">
        <Button onClick={onDone}>Thôi</Button>
        <Button primary disabled={busy || !token.trim()} onClick={add}>
          Thêm token
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
    <Panel role="region" aria-label="Tài khoản Quota Station giữ">
      <CardTitle>
        Tài khoản Quota Station giữ
        <span className="ml-auto font-reg">
          <LinkButton onClick={() => onAction('guide:switcher')}>Hướng dẫn chuyển từ claude-swap</LinkButton>
        </span>
      </CardTitle>
      {setup ? <BannerBox tone="info" title={setup.title} body={setup.body} style={{ margin: '4px 0 8px' }} /> : null}
      {rows.length ? (
        <div role="list" aria-label="Tài khoản Claude">
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
            {view.ready ? 'Nhập lại từ claude-swap' : 'Nhập từ claude-swap'}
          </Button>
        ) : null}
        <Button disabled={busy || !view.claude} title={view.claude ? undefined : 'Không thấy lệnh claude trên máy này'} onClick={() => quiet(api.claudeAddLogin())}>
          Đăng nhập tài khoản mới
        </Button>
        <Button disabled={busy} onClick={() => quiet(api.claudeAddCurrent())}>
          Thêm tài khoản đang đăng nhập
        </Button>
        <Button disabled={busy} onClick={() => setTokenOpen(true)}>
          Thêm setup token
        </Button>
      </div>
      {tokenOpen ? <TokenForm api={api} busy={busy} onDone={() => setTokenOpen(false)} /> : null}
      {working ? (
        <FeedbackLine tone="info" style={{ margin: '8px 0 0' }}>
          <Spinner />
          <div className="flex min-w-0 flex-1 items-center justify-between gap-2.5">
            <span className="text-sm">{working}</span>
            {view.working === 'login' ? <Button onClick={() => void api.claudeCancelLogin()}>Huỷ</Button> : null}
          </div>
        </FeedbackLine>
      ) : null}
      {done ? <Feedback fb={done} inline onAction={onAction} style={{ margin: '8px 0 0' }} /> : null}
      <div className="mt-3 flex items-center justify-between gap-2.5 border-t-[.5px] border-card-line pt-3">
        <div className="min-w-0">
          <b className="block text-body font-semi">Tự chuyển bằng Quota Station</b>
          <small className="block text-xs text-ink2">{rotation.line}</small>
        </div>
        <Toggle on={rotation.on} label="Tự chuyển bằng Quota Station" disabled={rotation.disabled} onClick={() => quiet(api.claudeSetRotation(!rotation.on))} />
      </div>
      {rotation.legacy ? (
        <BannerBox tone="warn" title="Bộ tự chuyển cũ còn chạy" body={rotation.legacy} style={{ margin: '8px 0 0' }}>
          <LinkButton onClick={() => onAction('guide:auto')}>Hướng dẫn tắt</LinkButton>
        </BannerBox>
      ) : null}
      {authorTools ? (
        <CardText className="mt-2 mb-0 text-xs text-ink2">
          Firstmate đọc `quota-station claude status --json` và chạy `quota-station claude run N -- …`.
        </CardText>
      ) : null}
    </Panel>
  );
}
