// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Codex tab of the main window (M8) and Codex's compact card on Tổng
// quan. The tab reads like the Claude and Antigravity tabs: one table with
// the two sides (each "Chuyển" opens the confirmation sheet), "Dán khoá…"
// for the custom provider and the plugins box under it, the technical
// detail behind "Chi tiết"; then Codex on this Mac. CC Switch shows only as
// the banner that warns when it also switches Codex.

import type { ReactNode } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { AccountName, IdMark, InUse, WindowCell } from '../components/accounts';
import { CheckLine } from '../components/blocks';
import { CARD_END, COUNT, Empty, GRID2_EVEN, TABLE, TABLE_BOX, TABLE_CELLS, TABLE_END, TABLE_HEAD, TABLE_ROW, TABLE_ROW_ON, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, BannerBox, Button, Chip, CODE, Dot, Feedback, KVT, LinkButton, More, Rich } from '../components/parts';
import {
  CHATGPT_ID,
  ccSwitchManages,
  chatGptNote,
  chatGptRows,
  codexBanner,
  currentId,
  externalBanner,
  keyMode,
  type ChatGptRowView,
  pluginsHint,
  pluginsNeedApply,
  pluginsLabel,
  recentText,
  refusalText,
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

/** The custom provider's row chips: only what needs acting on. */
function customChips(v: CodexView): ChipView[] {
  const c = v.custom;
  if (!c) return [];
  const chips: ChipView[] = [];
  if (c.key === 'placeholder') chips.push({ t: t('chỉ khoá giữ chỗ', 'placeholder key only'), k: 'bad' });
  if (c.key === 'none') chips.push({ t: t('chưa có khoá', 'no key yet'), k: 'bad' });
  if (c.proxy) chips.push({ t: t('qua proxy CC Switch', 'through the CC Switch proxy'), k: 'warn' });
  if (v.reach.state === 'down') chips.push({ t: t('không trả lời', 'not answering'), k: 'bad' });
  return chips;
}

function host(url: string | null): string {
  if (!url) return t('không có base_url', 'no base_url');
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

function reachText(v: CodexView, now: Date): string {
  if (v.reach.state === 'up') return t('trả lời ' + v.reach.ms + ' ms', 'answers in ' + v.reach.ms + ' ms');
  if (v.reach.state === 'down') return t('không trả lời từ ', 'not answering since ') + (v.reach.since ? dayClock(v.reach.since, now) : '?');
  return v.custom?.baseUrl ? t('đang kiểm…', 'checking…') : '';
}

/** "Chuyển" is a button to press; the side in use gets a status in its place. */
function SideCell({ on, off, label, title, act, onAction }: { on: boolean; off: boolean; label: string; title?: string; act: Action; onAction(a: Action): void }) {
  if (on) return <InUse />;
  return (
    <Button primary disabled={off} label={label} title={title} onClick={() => onAction(act)}>
      {t('Chuyển', 'Switch')}
    </Button>
  );
}

/** The ChatGPT accounts' table: the name, 5 giờ and Tuần, the button. */
const CHATGPT_COLS = '[--wt-cols:minmax(96px,1.25fr)_repeat(2,minmax(64px,1fr))_minmax(66px,auto)]';

/** One ChatGPT account, a row as the Claude and Antigravity accounts are. */
function ChatGptRow({ row, tight, onAction }: { row: ChatGptRowView; tight?: boolean; onAction(a: Action): void }) {
  return (
    <div className={cn(TABLE_ROW, row.active && TABLE_ROW_ON)} role="listitem" aria-label={row.label} title={row.full || undefined}>
      <AccountName name={row.name} sub={row.sub} full={row.full} chips={row.chips} />
      <span className={TABLE_CELLS}>
        {row.cells.map((c) => (
          <WindowCell key={c.kind} cell={c} dim={row.dim} tight={tight} />
        ))}
      </span>
      <span className={cn('flex flex-wrap items-center justify-end gap-1.5', TABLE_END)}>
        <SideCell
          on={row.active}
          off={!!row.off}
          label={t('Chuyển sang tài khoản ChatGPT', 'Switch to the ChatGPT account')}
          title={row.off || undefined}
          act="codex:switch:chatGpt"
          onAction={onAction}
        />
      </span>
    </div>
  );
}

/**
 * Tổng quan's row for the provider Codex goes through instead of ChatGPT: a
 * custom API reports no quota, so its windows' columns say so once.
 */
function ProviderRow({ v, now, onAction }: { v: CodexView; now: Date; onAction(a: Action): void }) {
  const c = v.custom;
  const other = v.current === 'other';
  if (!c && !other) return null;
  const name = other ? currentId(v) : c!.name;
  const sub = other ? t('nhà cung cấp khác trong config.toml', 'another provider in config.toml') : ['Custom API', reachText(v, now)].filter(Boolean).join(' · ');
  const active = other || v.current === 'custom';
  const refusal = v.refusal;
  return (
    <div className={cn(TABLE_ROW, active && TABLE_ROW_ON)} role="listitem" aria-label={name + ', ' + sub + (active ? t(', đang dùng', ', in use') : '')}>
      <AccountName name={name} sub={sub} full={other ? '' : (c!.baseUrl ?? '')} chips={other ? [] : customChips(v)} />
      <span className={TABLE_CELLS}>
        <span className="col-span-2 self-center text-sm text-ink3">{t('Không báo hạn mức', 'Reports no quota')}</span>
      </span>
      <span className={cn('flex flex-wrap items-center justify-end gap-1.5', TABLE_END)}>
        {other ? (
          <InUse />
        ) : (
          <SideCell
            on={v.current === 'custom'}
            off={!!refusal || v.busy != null}
            label={t('Chuyển sang ', 'Switch to ') + c!.name}
            title={refusal ? refusalText(v, refusal) : undefined}
            act="codex:switch:custom"
            onAction={onAction}
          />
        )}
      </span>
    </div>
  );
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

/** The ChatGPT accounts Codex can use, a list as Claude's and Antigravity's are. */
function ChatGptCard({ model, now, onAction }: { model: Model; now: Date; onAction(a: Action): void }) {
  const v = model.codex;
  const rows = chatGptRows(model, now);
  const note = chatGptNote(model);
  return (
    <Panel>
      <CardTitle>
        <IdMark id="x" />
        {t('Tài khoản ChatGPT', 'ChatGPT accounts')}
        <span className={COUNT}>{rows.length}</span>
      </CardTitle>
      <div className={TABLE_BOX}>
        <div className={cn(TABLE, CHATGPT_COLS)} role="list" aria-label={t('Tài khoản ChatGPT', 'ChatGPT accounts')}>
          <ChatGptHead />
          {rows.map((r) => (
            <ChatGptRow key={r.key} row={r} onAction={onAction} />
          ))}
        </div>
      </div>
      {note ? <Note tone={note.tone}>{note.text}</Note> : null}
      {v.current === 'other' ? <Note tone="warn">{t('Codex đang dùng nhà cung cấp "' + v.other + '". Chọn một bên để chuyển.', 'Codex is using the provider "' + v.other + '". Pick a side to switch.')}</Note> : null}
      {v.current === 'custom' && v.backRefusal ? <Note tone="bad">{refusalText(v, v.backRefusal)}</Note> : null}
      {v.openaiBaseUrl ? <Note tone="warn">{t('config.toml có openai_base_url: tài khoản ChatGPT đang được gửi tới địa chỉ khác.', 'config.toml has openai_base_url: the ChatGPT account is being sent to another address.')}</Note> : null}
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

/** The custom provider from config.toml: its row, its key, and the plugins box. */
function CustomCard({ v, now, onAction, onPlugins }: { v: CodexView; now: Date; onAction(a: Action): void; onPlugins(on: boolean): void }) {
  const c = v.custom;
  const busy = v.busy != null;
  const mode = keyMode(v);
  const refusal = v.current === 'custom' ? null : v.refusal;
  return (
    <Panel>
      <CardTitle>Custom API</CardTitle>
      {c ? (
        <div className="grid grid-cols-(--wt-cols) gap-x-6 [--wt-cols:minmax(0,1fr)_auto]" role="list" aria-label="Custom API">
          <div className={cn(TABLE_ROW, v.current === 'custom' && TABLE_ROW_ON)} role="listitem" aria-label={c.name}>
            <AccountName name={c.name} sub={[host(c.baseUrl), reachText(v, now)].filter(Boolean).join(' · ')} full={c.baseUrl ?? ''} chips={customChips(v)} />
            <span className="flex items-center justify-end gap-2.5">
              {mode ? (
                <LinkButton disabled={busy} onClick={() => onAction('codex:key')}>
                  {mode === 'replace' ? t('Đổi khoá…', 'Change key…') : mode === 'env' ? t('Cách đặt khoá…', 'How to set the key…') : t('Dán khoá…', 'Paste key…')}
                </LinkButton>
              ) : null}
              <SideCell
                on={v.current === 'custom'}
                off={!!v.refusal || busy}
                label={t('Chuyển sang ', 'Switch to ') + c.name}
                title={v.refusal ? refusalText(v, v.refusal) : undefined}
                act="codex:switch:custom"
                onAction={onAction}
              />
            </span>
          </div>
        </div>
      ) : null}
      {refusal ? <Note tone={refusal.kind === 'noCustom' ? '' : 'bad'}>{refusalText(v, refusal)}</Note> : null}
      {c ? (
        <CheckLine className="mt-2.5 mb-0" checked={v.plugins} disabled={busy} onChange={onPlugins}>
          <span>
            {pluginsLabel()}
            <small>
              {t('Bỏ tick: plugin cần đăng nhập ChatGPT sẽ tắt.', 'Unticked: plugins that need the ChatGPT sign-in turn off.')} <PluginsMore />
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
}

export function CodexTab({ model, fresh, feedback, now, onAction, onPlugins }: CodexTabProps) {
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
      <ChatGptCard model={model} now={now} onAction={onAction} />
      <div className={GRID2_EVEN}>
        <CustomCard v={v} now={now} onAction={onAction} onPlugins={onPlugins} />
        <OnThisMac v={v} now={now} onAction={onAction} />
      </div>
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
  const rows = chatGptRows(model, now);
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
            <ChatGptRow key={r.key} row={r} tight onAction={onAction} />
          ))}
          <ProviderRow v={v} now={now} onAction={onAction} />
        </div>
      </div>
      {note ? <Note tone={note.tone}>{note.text}</Note> : null}
    </Panel>
  );
}
