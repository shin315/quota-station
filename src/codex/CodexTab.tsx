// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Codex tab of the main window (M8) and Codex's compact card on Tổng
// quan. The tab reads like the Claude tab: one table of every Codex account
// (the ChatGPT sign-ins the app keeps, then the custom providers in
// config.toml), each "Chuyển" opening the confirmation sheet and each ⋯ its
// details, a new sign-in and its removal, "Thêm tài khoản" over it; the
// plugins box and "Dán khoá…" for the provider in use under it; then Codex
// on this Mac with the technical detail behind "Chi tiết". CC Switch shows
// only as the banner that warns when it also switches Codex.

import type { ReactNode } from 'react';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Glyph } from '../components/icons';
import { accountRows, type CodexAccountRowView } from '../lib/codexAccounts';
import type { CodexAccountsState } from './useCodexAccounts';
import { EmptyBox } from '../window/MainWindow';
import { cn } from '@/lib/utils';
import { AccountName, IdMark, InUse, WindowCell } from '../components/accounts';
import { CheckLine } from '../components/blocks';
import { CARD_END, COUNT, Empty, TABLE, TABLE_BOX, TABLE_CELLS, TABLE_END, TABLE_HEAD, TABLE_ROW, TABLE_ROW_ON, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, BannerBox, Button, Chip, CODE, Dot, Feedback, KVT, LinkButton, More, Rich } from '../components/parts';
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
import { t } from '../lib/i18n';
import type { CodexView, Model } from '../lib/model';
import { dayClock } from '../lib/switching';
import type { Action, BannerView, ChipView, FeedbackView, PopoverView } from '../lib/view';

function Card(props: { title: ReactNode; id?: string; x?: boolean; children: ReactNode }) {
  return (
    <Panel id={props.id}>
      <CardTitle>
        {props.x ? <IdMark id="x" /> : null}
        {props.title}
      </CardTitle>
      {props.children}
    </Panel>
  );
}

/** A line with its button at the far end. */
const ROW2 = 'flex items-center justify-between gap-2.5';
const DETAILS = 'mt-2 text-sm text-ink2';
const SUMMARY = 'cursor-pointer list-none text-accent-ink [&::-webkit-details-marker]:hidden';

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
      return { t: t('còn trong máy', 'still on this Mac'), k: 'ok' };
    case 'keychain':
      return {
        t: t('trong Keychain', 'in the Keychain'),
        k: 'info',
        title: t('Codex giữ đăng nhập trong Keychain; app không mở Keychain để kiểm.', 'Codex keeps the sign-in in the Keychain; the app does not open the Keychain to check.'),
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
          {row.kind === 'chatgpt' ? t('Xoá khỏi máy này…', 'Remove from this Mac…') : t('Xoá khỏi config.toml…', 'Remove from config.toml…')}
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
        <CheckLine className="mt-2.5 mb-0" checked={v.plugins} disabled={busy} onChange={onPlugins}>
          <span>
            {pluginsLabel()}
            <small>
              {t('Bỏ tick: plugin cần đăng nhập ChatGPT sẽ tắt khi chạy Custom API.', 'Unticked: plugins that need the ChatGPT sign-in turn off on a Custom API.')} <PluginsMore />
            </small>
          </span>
        </CheckLine>
      ) : null}
      {pluginsNeedApply(v, v.plugins) ? (
        <div className={cn(ROW2, 'mt-1.5 text-sm text-warn-ink')}>
          <span>{t('config.toml chưa theo lựa chọn này.', 'config.toml does not follow this choice yet.')}</span>
          <LinkButton disabled={busy} onClick={() => onAction('codex:plugins:apply')}>
            {t('Áp dụng ngay…', 'Apply now…')}
          </LinkButton>
        </div>
      ) : null}
      {v.current === 'custom' && mode && v.custom ? (
        <div className={cn(ROW2, 'mt-1.5 text-sm text-ink2')}>
          <span>{t('Khoá của ' + v.custom.name + ' nằm trong config.toml.', v.custom.name + '’s key is in config.toml.')}</span>
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

/** What "Dùng plugin khi chạy Custom API" does, said in full behind "Chi tiết". */
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

/** What the tab used to spell out, behind one "Chi tiết". */
function Details({ v, now, onAction }: { v: CodexView; now: Date; onAction(a: Action): void }) {
  const c = v.custom;
  const w = v.lastWrite;
  const requires = c?.requiresOpenaiAuth == null ? t('chưa có', 'not set') : String(c.requiresOpenaiAuth);
  const wants = v.wantsRequires != null && v.wantsRequires !== c?.requiresOpenaiAuth ? ' → ' + v.wantsRequires + t(' khi chuyển', ' on switch') : '';
  const r = v.resume;
  return (
    <details className={DETAILS}>
      <summary className={SUMMARY}>{t('Chi tiết', 'Details')}</summary>
      <div className="mt-2 grid gap-3">
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
            t('Chưa có lần chuyển nào trên máy này.', 'No switches on this Mac yet.')
          )}
        </div>
      </div>
    </details>
  );
}

function OnThisMac({ v, now, onAction }: { v: CodexView; now: Date; onAction(a: Action): void }) {
  const p = v.procs;
  const on = v.mcp.filter((m) => m.enabled).length;
  const line = [
    'CLI ' + (v.version ?? t('không tìm thấy', 'not found')),
    p ? t(p.sessions + ' phiên Terminal', p.sessions + (p.sessions === 1 ? ' Terminal session' : ' Terminal sessions')) : null,
    p ? (p.app ? t('Codex App đang mở', 'Codex App is open') : t('Codex App không mở', 'Codex App is not open')) : null,
  ]
    .filter(Boolean)
    .join(' · ');
  return (
    <Card title={t('Codex trên máy này', 'Codex on this Mac')}>
      <div className="text-sm text-ink2">
        <Dot tone={p?.daemon ? 'ok' : 'muted'} className="mr-1.5" />
        {line}
      </div>
      <div className={cn(ROW2, 'mt-1.5 text-sm text-ink2')}>
        <span>{v.mcp.length ? v.mcp.length + ' MCP · ' + on + t(' đang bật', ' on') : t('Chưa có MCP', 'No MCP yet')}</span>
        <LinkButton onClick={() => onAction('mcp:tab')}>{t('Mở tab MCP', 'Open the MCP tab')}</LinkButton>
      </div>
      <div className={cn(ROW2, 'mt-2.5')}>
        <small className="text-xs text-ink2">{t('Đổi xong: ⌘Q Codex App rồi mở lại.', 'After a switch: ⌘Q Codex App and open it again.')}</small>
        <Button disabled={v.busy != null} onClick={() => onAction('codex:daemon')}>
          {t('Khởi động lại tiến trình nền…', 'Restart background process…')}
        </Button>
      </div>
      <Details v={v} now={now} onAction={onAction} />
    </Card>
  );
}

export interface CodexTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  onAction(act: Action): void;
  /** "Dùng plugin khi chạy Custom API". */
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
            <b>{t('Máy này chưa có Codex', 'This Mac has no Codex')}</b>
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
      <OnThisMac v={v} now={now} onAction={onAction} />
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
