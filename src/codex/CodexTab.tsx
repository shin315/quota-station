// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Codex tab of the main window (M8) and Codex's compact card on Tổng
// quan. The tab reads like the Claude tab: one table of every Codex account
// (the ChatGPT sign-ins the app keeps, then the custom providers in
// config.toml), each "Chuyển" opening the confirmation sheet and each ⋯ its
// details, a new sign-in and its removal, "Thêm tài khoản" over it; the
// plugins box and "Đổi khoá…" for the provider in use under it, one line
// each; "Tự đánh thức ChatGPT" once the app keeps a ChatGPT account, resting
// on one line that opens in place (direction C, 27/10/2026); then Codex on
// this Mac as one line, the technical detail on its "Chi tiết…" sheet. Every
// longer explanation is behind a Chi tiết. CC Switch shows only as the
// banner that warns when it also switches Codex.

import { useState, type ReactNode } from 'react';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { Glyph } from '../components/icons';
import { accountName, accountOf, accountRows, type CodexAccountRowView } from '../lib/codexAccounts';
import type { CodexAccountsState } from './useCodexAccounts';
import { EmptyBox } from '../window/MainWindow';
import { cn } from '@/lib/utils';
import { AccountName, IdMark, InUse, WindowCell } from '../components/accounts';
import { CheckLine, CheckRow, GROUP_PANEL } from '../components/blocks';
import { CARD_END, COUNT, Empty, TABLE, TABLE_BOX, TABLE_CELLS, TABLE_END, TABLE_HEAD, TABLE_ROW, TABLE_ROW_ON, TabHead, TabSub, TabTitle } from '../components/page';
import { Input } from '@/components/ui/input';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Banner, BannerBox, Button, Chip, ChoiceLabel, CODE, Dot, Feedback, KVT, LinkButton, More, Rich, Spinner, Toggle } from '../components/parts';
import {
  CHATGPT_ID,
  ccSwitchManages,
  chatGptNote,
  codexBanner,
  externalBanner,
  keyMode,
  pluginsHint,
  pluginsNeedApply,
  pluginsLabel,
  recentText,
  shortName,
  undoLabel,
} from '../lib/codex';
import { plusFive, runLine, suggestWake, triggerError, triggerLine, wakeErrorText, wakeTimes } from '../lib/codexWake';
import { mac, os, quitKey, t, terminal, toolStore } from '../lib/i18n';
import type { CodexTrigger, CodexView, Model } from '../lib/model';
import { dayClock } from '../lib/switching';
import type { Action, BannerView, ChipView, FeedbackView, PopoverView } from '../lib/view';

/** A line with its button at the far end. */
const ROW2 = 'flex items-center justify-between gap-2.5';
/** A line under the accounts, a hairline above it. */
const LINE = 'mt-2 border-t-[.5px] border-card-line pt-2 text-sm text-ink2';

/** A note at the end of a card; red or amber when it warns. */
function Note({ tone, children }: { tone: string; children: ReactNode }) {
  return <CardText className={cn('mt-2 mb-0', tone === 'warn' ? 'text-warn-ink' : tone === 'bad' ? 'text-bad-ink' : undefined)}>{children}</CardText>;
}

function Rows({ rows }: { rows: { key: ReactNode; value: ReactNode; id: string }[] }) {
  return <div className={KVT}>{rows.flatMap((r) => [<span key={r.id + ':k'}>{r.key}</span>, <span key={r.id + ':v'}>{r.value}</span>])}</div>;
}

function loginChip(v: CodexView): ChipView {
  switch (v.login) {
    case 'chatGpt':
      return { t: mac(t('còn trong máy', 'still on this Mac')), k: 'ok' };
    case 'keychain':
      return {
        t: os(t('trong Keychain', 'in the Keychain'), t('trong ' + toolStore(), 'in ' + toolStore())),
        k: 'info',
        title: os(
          t('Codex giữ đăng nhập trong Keychain; app không mở Keychain để kiểm.', 'Codex keeps the sign-in in the Keychain; the app does not open the Keychain to check.'),
          t('Codex giữ đăng nhập trong ' + toolStore() + '; app không mở nó để kiểm.', 'Codex keeps the sign-in in ' + toolStore() + '; the app does not open it to check.'),
        ),
      };
    case 'other':
      return { t: t('đăng nhập khác', 'another sign-in'), k: 'warn' };
    case 'apiKey':
      return { t: t('chỉ có khoá API', 'API key only'), k: 'warn' };
    case 'unreadable':
      return { t: t('không đọc được', 'unreadable'), k: 'bad' };
    case 'ephemeral':
      return { t: t('không lưu', 'not stored'), k: 'warn' };
    default:
      return { t: t('chưa có', 'none'), k: 'warn' };
  }
}

function keyChip(v: CodexView): ChipView {
  const c = v.custom;
  if (!c) return { t: t('không có bảng', 'no table'), k: 'bad' };
  switch (c.key) {
    case 'bearer':
      return { t: t('có · experimental_bearer_token', 'yes · experimental_bearer_token'), k: 'ok' };
    case 'envKey':
      return { t: t('có · biến ', 'yes · variable ') + (c.envKey ?? ''), k: 'ok' };
    case 'headers':
      return { t: t('có · header', 'yes · header'), k: 'ok' };
    case 'placeholder':
      return { t: t('chỉ khoá giữ chỗ của CC Switch', 'only CC Switch’s placeholder key'), k: 'bad' };
    default:
      return { t: t('thiếu', 'missing'), k: 'bad' };
  }
}

/** The table's head: the windows' names over their columns. */
function ChatGptHead() {
  return (
    <div className={TABLE_HEAD} aria-hidden="true">
      <span>{t('Tài khoản', 'Account')}</span>
      <span>{t('5 giờ', '5-hour')}</span>
      <span>{t('Tuần', 'Week')}</span>
      <span />
    </div>
  );
}

/** The accounts' table: the name, 5 giờ and Tuần (a provider: one line), the button, ⋯. */
const ACCOUNT_COLS = '[--wt-cols:minmax(120px,1.4fr)_repeat(2,minmax(64px,1fr))_minmax(66px,auto)_28px]';
const ITEM = 'px-2.5 py-[7px] data-[highlighted]:bg-ctl-hover data-[highlighted]:text-ink';

/** An account's ⋯ menu: its details and nickname, a new sign-in, its removal. */
function AccountMenu({ row, accounts, busy }: { row: CodexAccountRowView; accounts: CodexAccountsState; busy: boolean }) {
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="grid size-6 cursor-pointer place-items-center rounded-ctl border-0 bg-fill2 text-ink hover:brightness-[.97] data-[state=open]:brightness-[.94] [&>svg]:size-3.5"
          aria-label={t('Thêm thao tác cho ', 'More actions for ') + row.name}
          title={t('Thêm thao tác cho ', 'More actions for ') + row.name}
        >
          <Glyph name="dots-three" bold />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem className={ITEM} onSelect={() => accounts.open({ kind: 'detail', id: row.id })}>
          {t('Chi tiết và tên gọi…', 'Details and nickname…')}
        </DropdownMenuItem>
        {row.kind === 'chatgpt' ? (
          <DropdownMenuItem className={ITEM} disabled={busy} onSelect={() => accounts.open({ kind: 'relogin', id: row.id })}>
            {t('Đăng nhập lại…', 'Sign in again…')}
          </DropdownMenuItem>
        ) : null}
        <DropdownMenuSeparator />
        <DropdownMenuItem className={ITEM + ' font-med text-bad-ink'} onSelect={() => accounts.open({ kind: 'remove', id: row.id })}>
          {row.kind === 'chatgpt' ? mac(t('Xoá khỏi máy này…', 'Remove from this Mac…')) : t('Xoá khỏi config.toml…', 'Remove from config.toml…')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** One account: a ChatGPT sign-in with its windows, or a provider, which reports no quota. */
function AccountRow({ row, tight, menu, onAction }: { row: CodexAccountRowView; tight?: boolean; menu?: ReactNode; onAction(a: Action): void }) {
  return (
    <div className={cn(TABLE_ROW, row.active && TABLE_ROW_ON)} role="listitem" aria-label={row.label} data-stale={row.dim || undefined}>
      <AccountName name={row.name} sub={row.sub} full={row.full} chips={row.chips} />
      <span className={TABLE_CELLS}>
        {row.cells ? (
          row.cells.map((c) => <WindowCell key={c.kind} cell={c} dim={row.dim} tight={tight} />)
        ) : (
          <span className="col-span-2 self-center text-sm text-ink3">{t('Không báo hạn mức', 'Reports no quota')}</span>
        )}
      </span>
      <span className={cn('flex flex-wrap items-center justify-end gap-1.5', TABLE_END)}>
        {row.active ? (
          <InUse />
        ) : row.relogin ? (
          <Button onClick={() => onAction(`codex:relogin:${row.id}`)}>{t('Đăng nhập lại', 'Sign in again')}</Button>
        ) : (
          <Button primary disabled={!!row.off} label={t('Chuyển sang ', 'Switch to ') + row.name} title={row.off || undefined} onClick={() => onAction(`codex:use:${row.id}`)}>
            {t('Chuyển', 'Switch')}
          </Button>
        )}
      </span>
      {menu !== undefined ? <span className="flex items-center justify-end">{menu}</span> : null}
    </div>
  );
}

function AccountsHead({ menu }: { menu: boolean }) {
  return (
    <div className={TABLE_HEAD} aria-hidden="true">
      <span>{t('Tài khoản', 'Account')}</span>
      <span>{t('5 giờ', '5-hour')}</span>
      <span>{t('Tuần', 'Week')}</span>
      <span />
      {menu ? <span /> : null}
    </div>
  );
}

/** Every Codex account in one list, with "Thêm tài khoản", the plugins box and the provider's key under it. */
function AccountsCard({ model, now, accounts, onAction, onPlugins }: { model: Model; now: Date; accounts: CodexAccountsState; onAction(a: Action): void; onPlugins(on: boolean): void }) {
  const v = model.codex;
  const rows = accountRows(model, now);
  const note = chatGptNote(model);
  const busy = v.busy != null;
  const gpts = rows.filter((r) => r.kind === 'chatgpt').length;
  const add = (
    <Button small lead="plus" onClick={() => onAction('codex:add')}>
      {t('Thêm tài khoản', 'Add account')}
    </Button>
  );
  const mode = keyMode(v);
  return (
    <Panel>
      <CardTitle>
        <IdMark id="x" />
        {t('Tài khoản', 'Accounts')}
        <span className={COUNT}>{rows.length ? t(gpts + ' ChatGPT · ' + (rows.length - gpts) + ' Custom API', gpts + ' ChatGPT · ' + (rows.length - gpts) + ' Custom API') : null}</span>
        <span className={CARD_END}>{add}</span>
      </CardTitle>
      {rows.length ? (
        <div className={TABLE_BOX}>
          <div className={cn(TABLE, ACCOUNT_COLS)} role="list" aria-label={t('Tài khoản Codex', 'Codex accounts')}>
            <AccountsHead menu />
            {rows.map((r) => (
              <AccountRow key={r.key} row={r} menu={<AccountMenu row={r} accounts={accounts} busy={busy} />} onAction={onAction} />
            ))}
          </div>
        </div>
      ) : (
        <EmptyBox
          icon={<IdMark id="x" className="size-6" />}
          title={t('Chưa có tài khoản Codex nào', 'No Codex accounts yet')}
          body={t(
            'Đăng nhập một tài khoản ChatGPT hoặc thêm nhà cung cấp API riêng để xem hạn mức và chuyển Codex giữa chúng.',
            'Sign in to a ChatGPT account or add your own API provider to see their quota and switch Codex between them.',
          )}
        >
          <Button small primary onClick={() => accounts.login(null)}>
            {t('Đăng nhập ChatGPT…', 'Sign in to ChatGPT…')}
          </Button>
          <Button small onClick={() => accounts.open({ kind: 'provider' })}>
            {t('Thêm Custom API…', 'Add a Custom API…')}
          </Button>
        </EmptyBox>
      )}
      {note ? <Note tone={note.tone}>{note.text}</Note> : null}
      {v.current === 'other' ? <Note tone="warn">{t('Codex đang dùng nhà cung cấp "' + v.other + '" không có trong danh sách. Chọn một tài khoản để chuyển.', 'Codex is using the provider "' + v.other + '", which is not in the list. Pick an account to switch.')}</Note> : null}
      {v.openaiBaseUrl ? <Note tone="warn">{t('config.toml có openai_base_url: tài khoản ChatGPT đang được gửi tới địa chỉ khác.', 'config.toml has openai_base_url: the ChatGPT account is being sent to another address.')}</Note> : null}
      {rows.some((r) => r.kind === 'custom') ? (
        <div className={cn(ROW2, LINE)}>
          <span className="flex min-w-0 flex-wrap items-center gap-x-1.5">
            <CheckLine className="mb-0" checked={v.plugins} disabled={busy} onChange={onPlugins}>
              <span>{pluginsLabel()}</span>
            </CheckLine>
            <span aria-hidden="true">·</span>
            {pluginsNeedApply(v, v.plugins) ? (
              <span className="text-warn-ink" title={t('config.toml chưa theo lựa chọn này.', 'config.toml does not follow this choice yet.')}>
                {t('Chưa áp dụng', 'Not applied yet')}
              </span>
            ) : (
              <span className={v.plugins ? 'text-ok-ink' : 'text-ink2'}>{v.plugins ? t('Đang bật', 'On') : t('Đang tắt', 'Off')}</span>
            )}
          </span>
          <span className="flex items-center gap-2.5">
            {pluginsNeedApply(v, v.plugins) ? (
              <LinkButton disabled={busy} onClick={() => onAction('codex:plugins:apply')}>
                {t('Áp dụng ngay…', 'Apply now…')}
              </LinkButton>
            ) : null}
            <PluginsMore />
          </span>
        </div>
      ) : null}
      {v.current === 'custom' && mode && v.custom ? (
        <div className={cn(ROW2, LINE)}>
          <span>
            {mode === 'env' ? t('Khoá API qua biến ' + (v.custom.envKey ?? ''), 'API key from the variable ' + (v.custom.envKey ?? '')) : t('Khoá API trong config.toml', 'API key in config.toml')}
            {' · '}
            <span className={keyChip(v).k === 'ok' ? 'text-ok-ink' : 'text-bad-ink'}>{keyChip(v).k === 'ok' ? t('Đã có', 'Set') : v.custom.key === 'placeholder' ? t('Chưa có · khoá giữ chỗ', 'Not set · placeholder') : t('Chưa có', 'Not set')}</span>
          </span>
          <LinkButton disabled={busy} onClick={() => onAction('codex:key')}>
            {mode === 'replace' ? t('Đổi khoá…', 'Change key…') : mode === 'env' ? t('Cách đặt khoá…', 'How to set the key…') : t('Dán khoá…', 'Paste key…')}
          </LinkButton>
        </div>
      ) : null}
      {v.keyUndo ? (
        <div className={cn(ROW2, 'mt-2 text-sm text-ink2')}>
          <span>{t('Đã dán khoá', 'Pasted a key') + (v.keyUndo.at ? t(' lúc ', ' at ') + dayClock(v.keyUndo.at, now) : '') + '.'}</span>
          <LinkButton disabled={busy} onClick={() => onAction('codex:undoKey')}>
            {undoLabel(v.keyUndo)}
          </LinkButton>
        </div>
      ) : null}
    </Panel>
  );
}

/** A label and its control, as the expanded card lines them up. */
const FORM = 'mt-2.5 grid grid-cols-[88px_minmax(0,1fr)] items-start gap-x-3 gap-y-2.5 text-sm @max-[420px]/main:grid-cols-[minmax(0,1fr)]';
const FORM_LABEL = 'pt-[5px] font-semi';
/** A wake time in the list: the time, then its ×. */
const TIME_BOX = 'inline-flex h-[26px] w-[110px] items-center rounded-ctl bg-fill px-2 tabular-nums shadow-[inset_0_0_0_.5px_var(--card-line)]';

/** "Tốn hạn mức · Có thể trái điều khoản": always on the card, the whole of it behind Chi tiết. */
function TriggerWarn() {
  return (
    <div className="mt-2 flex flex-wrap items-center gap-x-1.5 text-sm text-warn-ink">
      <span aria-hidden="true" className="font-black">
        !
      </span>
      <span>{t('Tốn hạn mức · Có thể trái điều khoản', 'Spends quota · May break the terms')}</span>
      <span aria-hidden="true">·</span>
      <More about={t('Tự đánh thức ChatGPT', 'Wake ChatGPT')}>
        <p>
          <Rich
            text={t(
              'Mỗi lần chạy `codex exec` với một câu "hi" trong thư mục của tài khoản đó, không dùng config, MCP hay plugin của bạn: Codex tự dùng và làm mới đăng nhập, app không cầm token. Mỗi lần tốn một tin nhắn ngắn của hạn mức. Không chạy cùng lúc với Chuyển hay đăng nhập.',
              'Each run is `codex exec` with a "hi" in that account’s folder, without your config, MCP servers or plugins: Codex uses and renews the sign-in itself, the app holds no token. Each run spends one short message of quota. It never runs during a switch or a sign-in.',
            )}
          />
        </p>
        <p className="text-warn-ink">
          {t(
            'OpenAI không nói tới việc tự gửi yêu cầu theo lịch: có thể trái điều khoản sử dụng. Chỉ dùng cho tài khoản của chính bạn.',
            'OpenAI does not cover sending requests on a schedule: it may break the terms of use. Use it only on your own accounts.',
          )}
        </p>
      </More>
    </div>
  );
}

/** "Giờ chạy": the times, each with its ×, and "Thêm giờ"; a list Rust would refuse is not sent, and says why. */
function WakeTimes({ times, onSet }: { times: string[]; onSet(times: string[]): Promise<string | null> }) {
  const [adding, setAdding] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const keep = async (list: string[]) => {
    const kept = wakeTimes(list);
    if ('error' in kept) return setError(wakeErrorText(kept.error)), false;
    const refused = await onSet(kept.times);
    setError(refused ? wakeErrorText(refused) : null);
    return !refused;
  };
  return (
    <div className="grid justify-items-start gap-1.5" role="group" aria-label={t('Giờ chạy', 'Wake times')}>
      {times.map((at) => (
        <span key={at} className="flex items-center gap-1.5">
          <span className={TIME_BOX}>{at}</span>
          <Button quiet icon small lead="x" label={t('Xoá giờ ', 'Remove ') + at} onClick={() => void keep(times.filter((x) => x !== at))}>
            {null}
          </Button>
        </span>
      ))}
      {adding == null ? (
        <Button small lead="plus" onClick={() => (setAdding(suggestWake(times)), setError(null))}>
          {t('Thêm giờ', 'Add time')}
        </Button>
      ) : (
        <span className="flex items-center gap-1.5">
          <Input type="time" aria-label={t('Giờ mới', 'New time')} className="w-[110px]" value={adding} onChange={(e) => setAdding(e.target.value)} />
          <Button small primary onClick={() => void keep([...times, adding]).then((ok) => ok && setAdding(null))}>
            {t('Thêm', 'Add')}
          </Button>
          <Button small quiet onClick={() => (setAdding(null), setError(null))}>
            {t('Thôi', 'Cancel')}
          </Button>
        </span>
      )}
      <small className={cn('text-xs', error ? 'text-bad-ink' : 'text-ink2')} role={error ? 'alert' : undefined}>
        {error ?? t('Các mốc cách nhau ít nhất 5 giờ.', 'Times at least 5 hours apart.')}
      </small>
    </div>
  );
}

/**
 * "Tự đánh thức ChatGPT", off until turned on: a short message sent with
 * Codex in each chosen account's own folder, each day at its times or as a
 * window resets, and to an account whose sign-in ran out while it waited.
 * It rests on one line (on or off, its times, how many accounts) and opens
 * in place on "Cài đặt"; the explanations are behind Chi tiết.
 */
function TriggerCard({ model, now, tr, accounts }: { model: Model; now: Date; tr: CodexTrigger; accounts: CodexAccountsState }) {
  const v = model.codex;
  const [open, setOpen] = useState(false);
  const set = (p: Partial<Pick<CodexTrigger, 'on' | 'mode' | 'times' | 'keepAlive' | 'ids'>>) =>
    accounts.api.codexTriggerSet({ on: tr.on, mode: tr.mode, times: tr.times, keepAlive: tr.keepAlive, ids: tr.ids, ...p }).then(
      () => null,
      (e: unknown) => (typeof e === 'string' ? e : 'at'),
    );
  const gpts = v.accounts.filter((a) => a.kind === 'chatgpt' && (a.number ?? 0) > 0);
  const chosen = gpts.filter((a) => tr.ids.includes(a.id) && a.login !== 'relogin').length;
  const runner = tr.running ? accountOf(v, tr.running) : undefined;
  const failed = runner ? null : triggerError(tr);
  const failedOf = failed ? accountOf(v, failed.id) : undefined;
  const runNow = (
    <Button disabled={!tr.ids.length || !!tr.running || v.version == null} onClick={() => void accounts.api.codexTriggerNow().catch(() => {})}>
      {t('Chạy ngay', 'Run now')}
    </Button>
  );
  return (
    <Panel>
      <CardTitle>
        {t('Tự đánh thức ChatGPT', 'Wake ChatGPT')}
        <span className={CARD_END}>
          <Toggle on={tr.on} label={t('Tự đánh thức', 'Wake accounts')} disabled={v.version == null} onClick={() => void set({ on: !tr.on })} />
          <Button small quiet={open} expanded={open} label={open ? t('Thu gọn Tự đánh thức', 'Collapse Wake ChatGPT') : t('Cài đặt Tự đánh thức', 'Wake ChatGPT settings')} onClick={() => setOpen(!open)}>
            {open ? t('Thu gọn', 'Collapse') : t('Cài đặt', 'Settings')}
            <Glyph name={open ? 'caret-up' : 'caret-down'} bold />
          </Button>
          {open ? null : runNow}
        </span>
      </CardTitle>
      <div className="text-sm text-ink2 tabular-nums">
        {runner ? <Spinner className="mr-1.5 inline-block align-[-2px]" /> : null}
        {triggerLine(tr, chosen, gpts.length, now, runner ? accountName(model, runner) : null)}
        {failed && failedOf ? (
          <span className="text-bad-ink">
            {' · ' + t('Lỗi', 'Error') + ' · ' + accountName(model, failedOf) + ': '}
            {failed.result === 'relogin' ? t('cần đăng nhập lại', 'must sign in again') : (failed.detail ?? t('Codex báo lỗi', 'Codex reported an error'))}
          </span>
        ) : null}
      </div>
      {open ? (
        <>
          <div className="mt-0.5 text-sm text-ink2">
            {t('Mở sớm khung 5 giờ', 'Opens the 5-hour window early')}
            {' · '}
            <More about={t('Tự đánh thức ChatGPT', 'Wake ChatGPT')}>
              <p>
                {t(
                  'Gửi một câu ngắn bằng Codex cho từng tài khoản đã chọn: khung 5 giờ mở từ sớm nên đặt lại sớm.',
                  'Sends a short message with Codex for each chosen account: its 5-hour window opens early, so it resets early.',
                )}
              </p>
              <p>
                <b>{t('Mỗi ngày', 'Every day')}</b>
                {mac(
                  t(
                    ': chạy ở từng giờ đã chọn; mở lúc ' + tr.times[0] + ' thì đặt lại lúc ' + plusFive(tr.times[0] ?? '06:00') + '. Mac ngủ lúc đó thì chạy khi thức, trễ tối đa 3 giờ; giờ đã lỡ không chạy bù.',
                    ': runs at each time chosen; opened at ' + tr.times[0] + ', it resets at ' + plusFive(tr.times[0] ?? '06:00') + '. A Mac asleep then runs it on waking, up to 3 hours late; a time missed is not made up.',
                  ),
                )}
              </p>
              <p>
                <b>{t('Khi đặt lại', 'On reset')}</b>
                {t(': chạy ngay khi khung 5 giờ của tài khoản đã đặt lại.', ': runs as soon as the account’s 5-hour window has reset.')}
              </p>
              <p>
                {t(
                  'Mỗi tài khoản cách nhau ít nhất 5 giờ, tối đa 5 lần trong 24 giờ, kể cả khi lỗi; không gửi khi khung đang mở hay hết hạn mức tuần. Giờ nào rơi vào lúc đó thì bỏ qua và ghi lý do.',
                  'Each account at least 5 hours apart, at most 5 times in 24 hours, failed runs included; nothing sent while a window is open or the week is used up. A time that falls then is skipped, with its reason.',
                )}
              </p>
            </More>
          </div>
          <div className={FORM}>
            <span className={FORM_LABEL} id="qs-cx-trig-mode-l">
              {t('Khi nào', 'When')}
            </span>
            <RadioGroup className="grid-cols-[repeat(2,max-content)]" aria-labelledby="qs-cx-trig-mode-l" value={tr.mode} onValueChange={(mode) => void set({ mode: mode as CodexTrigger['mode'] })}>
              <RadioGroupItem value="daily" className="items-center">
                {t('Mỗi ngày', 'Every day')}
              </RadioGroupItem>
              <RadioGroupItem value="reset" className="items-center">
                {t('Khi đặt lại', 'On reset')}
              </RadioGroupItem>
            </RadioGroup>
            {tr.mode === 'daily' ? (
              <>
                <span className={FORM_LABEL}>{t('Giờ chạy', 'Wake times')}</span>
                <WakeTimes times={tr.times} onSet={(times) => set({ times })} />
              </>
            ) : null}
          </div>
          <div className={cn(ROW2, 'mt-2.5')}>
            <CheckLine className="mb-0" checked={tr.keepAlive} onChange={(keepAlive) => void set({ keepAlive })}>
              <span>{t('Giữ đăng nhập', 'Keep sign-ins')}</span>
            </CheckLine>
            <More small about={t('Giữ đăng nhập', 'Keep sign-ins')}>
              <p>
                {t(
                  'Cũng đánh thức tài khoản đang chờ trong thư mục riêng khi token của nó đã hết hạn (app không đọc được hạn mức), để Codex làm mới; tối đa 2 lần một ngày. Không hứa đăng nhập luôn còn hạn: ChatGPT từ chối thì cần đăng nhập lại.',
                  'Also wakes an account waiting in its own folder once its token has run out (the app cannot read its quota then), so Codex renews it; at most twice a day. It does not promise a sign-in never runs out: one ChatGPT refuses must sign in again.',
                )}
              </p>
            </More>
          </div>
          <ChoiceLabel className="mt-2.5">{t('Tài khoản', 'Accounts')}</ChoiceLabel>
          <div className={GROUP_PANEL} role="list" aria-label={t('Tài khoản được đánh thức', 'Accounts woken')}>
            {gpts.map((a) => {
              const r = tr.runs.find((x) => x.id === a.id);
              const refused = a.login === 'relogin';
              return (
                <CheckRow
                  key={a.id}
                  checked={tr.ids.includes(a.id) && !refused}
                  disabled={refused}
                  off={refused}
                  onChange={(on) => void set({ ids: on ? [...tr.ids, a.id] : tr.ids.filter((x) => x !== a.id) })}
                >
                  <span className="min-w-0 truncate" title={r?.detail ?? undefined}>
                    <b>{accountName(model, a)}</b>
                    <span className="text-ink2 tabular-nums">{' · ' + runLine(r, now)}</span>
                  </span>
                  {tr.running === a.id ? <Spinner /> : refused ? <Chip chip={{ t: t('cần đăng nhập lại', 'must sign in again'), k: 'bad' }} /> : <span />}
                </CheckRow>
              );
            })}
          </div>
          <div className={cn(ROW2, 'items-end')}>
            <TriggerWarn />
            {runNow}
          </div>
        </>
      ) : (
        <TriggerWarn />
      )}
    </Panel>
  );
}

/** What "Dùng plugin với Custom API" does, said in full behind "Chi tiết". */
function PluginsMore() {
  return (
    <More about={pluginsLabel()}>
      <p>{pluginsHint()}</p>
      <p>
        {t(
          'Giữ đăng nhập ChatGPT không có nghĩa token của mô hình được tính vào gói ChatGPT: khi chạy Custom API, mô hình vẫn chạy qua nhà cung cấp đó.',
          'Keeping the ChatGPT sign-in does not bill the model’s tokens to ChatGPT: on a Custom API the model still runs through that provider.',
        )}
      </p>
      <p>
        <Rich
          text={t(
            'Với khoá riêng dạng bearer hoặc biến môi trường, ô này quyết định `requires_openai_auth` app ghi khi chuyển. Khoá dạng header luôn ghi `requires_openai_auth = false`: plugin và điều khiển từ xa cần ChatGPT có thể không chạy.',
            'With a bearer key or an environment variable, this box decides the `requires_openai_auth` the app writes on a switch. A header key always writes `requires_openai_auth = false`: plugins and remote control that need ChatGPT may not run.',
          )}
        />
      </p>
      <p>
        {t(
          'Bỏ chọn không xoá auth.json. Lựa chọn được lưu cho lần chuyển sau; muốn áp dụng ngay khi đang chạy Custom API thì app hỏi xác nhận trước. Chuyển nhà cung cấp chỉ sửa config.toml, sao lưu trước khi ghi, và không đụng tới MCP.',
          'Unticking does not delete auth.json. The choice is kept for the next switch; to apply it now while on a Custom API, the app asks first. Switching providers edits only config.toml, backs it up before writing, and leaves the MCP servers alone.',
        )}
      </p>
    </More>
  );
}

/** What Codex on this Mac used to spell out, on its "Chi tiết…" sheet. */
function Details({ v, now, onAction }: { v: CodexView; now: Date; onAction(a: Action): void }) {
  const c = v.custom;
  const w = v.lastWrite;
  const requires = c?.requiresOpenaiAuth == null ? t('chưa có', 'not set') : String(c.requiresOpenaiAuth);
  const wants = v.wantsRequires != null && v.wantsRequires !== c?.requiresOpenaiAuth ? ' → ' + v.wantsRequires + t(' khi chuyển', ' on switch') : '';
  const r = v.resume;
  return (
    <div className="grid gap-3 text-sm text-ink2">
      <Rows
        rows={[
          { id: 'login', key: t('Đăng nhập ChatGPT (auth.json)', 'ChatGPT sign-in (auth.json)'), value: <Chip chip={loginChip(v)} /> },
          ...(c
            ? [
                { id: 'url', key: t('Địa chỉ ' + c.name, c.name + ' address'), value: c.baseUrl ? <span className={CODE}>{c.baseUrl}</span> : t('không có', 'none') },
                { id: 'key', key: t('Khoá API riêng', 'Own API key'), value: <Chip chip={keyChip(v)} /> },
                { id: 'req', key: 'requires_openai_auth', value: <span className={CODE}>{requires + wants}</span> },
              ]
            : []),
          { id: 'w', key: t('File app ghi', 'File the app writes'), value: <span className={CODE}>{v.home}/config.toml</span> },
          { id: 'n', key: t('Không ghi', 'Never written'), value: <span className={CODE}>auth.json · [mcp_servers]</span> },
          {
            id: 'b',
            key: t('Sao lưu', 'Backups'),
            value: (
              <>
                {t(v.backups.length + ' bản · không chứa khoá', v.backups.length + (v.backups.length === 1 ? ' copy' : ' copies') + ' · no keys inside')}{' '}
                <LinkButton onClick={() => onAction('codex:backup')}>{t('Xem', 'View')}</LinkButton>
              </>
            ),
          },
          {
            id: 'l',
            key: t('App ghi lần cuối', 'Last written by the app'),
            value: w ? (
              <>
                {(w.at ? dayClock(w.at, now) + ' · ' : '') + shortName(v, w.provider) + ' '}
                <Chip chip={w.matches ? { t: t('mã băm khớp', 'hash matches'), k: 'ok' } : { t: t('file đã khác', 'file has changed'), k: 'warn' }} />
              </>
            ) : (
              t('chưa ghi lần nào', 'never written')
            ),
          },
          {
            id: 'r',
            key: t('Phiên tiếp tục được', 'Resumable sessions'),
            value: r
              ? 'ChatGPT ' +
                (r[CHATGPT_ID] ?? 0) +
                (c ? ' · ' + c.name + ' ' + (r[c.id] ?? 0) : '') +
                t(' (codex resume tách theo nhà cung cấp)', ' (codex resume keeps them per provider)')
              : '…',
          },
        ]}
      />
      <div>
        <b className="mb-1 block text-xs font-semi text-ink2">{t('Lần chuyển gần đây', 'Recent switches')}</b>
        {v.recent.length ? (
          // One grid: the time column as wide as its longest time ("hôm qua 22:47" on one line).
          <div className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-2 gap-y-1">
            {v.recent.map((e) => (
              <div key={e.at + e.to + e.result} className="col-span-full grid grid-cols-subgrid">
                <time className="tabular-nums">{dayClock(e.at, now)}</time>
                <span className="text-ink">{recentText(v, e)}</span>
              </div>
            ))}
          </div>
        ) : (
          mac(t('Chưa có lần chuyển nào trên máy này.', 'No switches on this Mac yet.'))
        )}
      </div>
    </div>
  );
}

/** Codex on this Mac in one line: the CLI, the sessions, the app, the MCP servers on. */
function machineLine(v: CodexView): string {
  const p = v.procs;
  const on = v.mcp.filter((m) => m.enabled).length;
  return [
    'CLI ' + (v.version ?? t('không tìm thấy', 'not found')),
    p ? t(p.sessions + ' phiên ' + terminal(), p.sessions + ' ' + terminal() + (p.sessions === 1 ? ' session' : ' sessions')) : null,
    p ? (p.app ? t('App đang mở', 'App open') : t('App đã đóng', 'App closed')) : null,
    v.mcp.length ? 'MCP ' + on + '/' + v.mcp.length : t('Chưa có MCP', 'No MCP yet'),
  ]
    .filter(Boolean)
    .join(' · ');
}

/**
 * "Codex trên máy này"'s sheet: the line in full, the technical detail and
 * the recent switches, the reminder after a switch, and the restart of the
 * background process.
 */
export function MachineSheet({ v, now, onAction, onClose }: { v: CodexView; now: Date; onAction(a: Action): void; onClose(): void }) {
  const go = (a: Action) => {
    onClose();
    onAction(a);
  };
  return (
    <div className="grid gap-3.5">
      <div className="grid gap-1">
        <DialogTitle className="mb-0">{mac(t('Codex trên máy này', 'Codex on this Mac'))}</DialogTitle>
        <DialogLead className="mb-0 flex items-center">
          <Dot tone={v.procs?.daemon ? 'ok' : 'muted'} className="mr-1.5" />
          {machineLine(v)}
        </DialogLead>
      </div>
      <Details v={v} now={now} onAction={go} />
      <small className="text-xs text-ink2">{t('Đổi xong: ' + quitKey() + ' Codex App rồi mở lại.', 'After a switch: ' + quitKey() + ' Codex App and open it again.')}</small>
      <DialogButtons>
        <Button disabled={v.busy != null} onClick={() => go('codex:daemon')}>
          {t('Khởi động lại tiến trình nền…', 'Restart background process…')}
        </Button>
        <Button primary onClick={onClose}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </div>
  );
}

function OnThisMac({ v, accounts, onAction }: { v: CodexView; accounts: CodexAccountsState; onAction(a: Action): void }) {
  return (
    <Panel>
      <CardTitle>
        {mac(t('Codex trên máy này', 'Codex on this Mac'))}
        <span className={CARD_END}>
          <Button small onClick={() => accounts.open({ kind: 'mac' })}>
            {t('Chi tiết…', 'Details…')}
          </Button>
          <Button small primary onClick={() => onAction('mcp:tab')}>
            {t('Mở MCP', 'Open MCP')}
          </Button>
        </span>
      </CardTitle>
      <div className="flex items-center text-sm text-ink2 tabular-nums">
        <Dot tone={v.procs?.daemon ? 'ok' : 'muted'} className="mr-1.5" />
        {machineLine(v)}
      </div>
    </Panel>
  );
}

export interface CodexTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  onAction(act: Action): void;
  /** "Dùng plugin với Custom API". */
  onPlugins(on: boolean): void;
  /** The accounts' dialogs and ⋯ menus. */
  accounts: CodexAccountsState;
}

export function CodexTab({ model, fresh, feedback, now, onAction, onPlugins, accounts }: CodexTabProps) {
  const v = model.codex;
  const top = (
    <TabHead>
      <TabTitle>Codex</TabTitle>
      <TabSub>
        <Dot tone={fresh.tone} />
        {model.machine} · {fresh.text}
      </TabSub>
      <Button disabled={v.busy != null} onClick={() => onAction('codex:check')}>
        {t('Kiểm lại', 'Check again')}
      </Button>
    </TabHead>
  );
  if (!v.present)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <b>{mac(t('Máy này chưa có Codex', 'This Mac has no Codex'))}</b>
            {t(
              'Không thấy thư mục ' + v.home + '. Nếu Codex dùng thư mục khác, đặt nó ở Cài đặt › Thư mục Codex.',
              'No ' + v.home + ' folder found. If Codex uses another folder, set it in Settings › Codex folder.',
            )}
          </Empty>
        </Panel>
      </>
    );
  const banners: BannerView[] = [];
  const main = codexBanner(model);
  if (main) banners.push(main);
  // The main banner is the external one when it carries its action.
  const mainIsExternal = !!main?.actions.some((a) => a.act === 'codex:external');
  if (v.external && !mainIsExternal) banners.push(externalBanner(v)!);
  if (ccSwitchManages(v))
    banners.push({
      tone: 'warn',
      title: t('CC Switch đang chạy và cũng đổi Codex', 'CC Switch is running and also switches Codex'),
      body: t(
        'Chỉ chuyển Codex ở một nơi: lần chuyển sau của CC Switch ghi đè lựa chọn ở đây.',
        'Switch Codex in one place only: CC Switch’s next switch overwrites the choice made here.',
      ),
      actions: [],
    });
  return (
    <>
      {top}
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {banners.map((b) => (
        <Banner key={b.title} banner={b} onAction={onAction} />
      ))}
      {v.config === 'invalid' || v.config === 'unreadable' ? (
        <BannerBox
          tone="bad"
          title={t('Không đọc được config.toml của Codex', 'Cannot read Codex’s config.toml')}
          body={(v.reason ?? t('Không rõ lý do', 'Unknown reason')) + t('. App không sửa file này cho tới khi Codex đọc được nó.', '. The app does not edit this file until Codex can read it.')}
        />
      ) : null}
      <AccountsCard model={model} now={now} accounts={accounts} onAction={onAction} onPlugins={onPlugins} />
      {v.accounts.some((a) => a.kind === 'chatgpt' && (a.number ?? 0) > 0) ? <TriggerCard model={model} now={now} tr={v.trigger} accounts={accounts} /> : null}
      <OnThisMac v={v} accounts={accounts} onAction={onAction} />
    </>
  );
}

/**
 * Codex on Tổng quan, a section like Claude's and Antigravity's: the ChatGPT
 * accounts with their five hours and week, and the custom provider Codex can
 * go through instead, each with its "Chuyển". `cols`: the columns Tổng
 * quan's tables share.
 */
export function CodexCard({ model, now, cols, onAction }: { model: Model; now: Date; cols: string; onAction(act: Action): void }) {
  const v = model.codex;
  if (!v.present) return null;
  const rows = accountRows(model, now);
  const note = chatGptNote(model);
  return (
    <Panel>
      <CardTitle>
        <IdMark id="x" />
        Codex
        <span className={CARD_END}>{v.procs?.app ? t('Codex App đang mở', 'Codex App is open') : ''}</span>
      </CardTitle>
      <div className={TABLE_BOX}>
        <div className={cn(TABLE, cols)} role="list" aria-label="Codex">
          <ChatGptHead />
          {rows.map((r) => (
            <AccountRow key={r.key} row={r} tight onAction={onAction} />
          ))}
        </div>
      </div>
      {note ? <Note tone={note.tone}>{note.text}</Note> : null}
    </Panel>
  );
}
