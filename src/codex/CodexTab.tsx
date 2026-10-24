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
import { AccountName, IdMark, InUse } from '../components/accounts';
import { CheckLine } from '../components/blocks';
import { Empty, GRID2, TABLE_ROW, TABLE_ROW_ON, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, BannerBox, Button, Chip, CODE, Dot, Feedback, KVT, LinkButton } from '../components/parts';
import {
  CHATGPT_ID,
  ccSwitchManages,
  codexBanner,
  codexLine,
  currentId,
  externalBanner,
  hasLogin,
  keyMode,
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

function Providers({ v, now, onAction, onPlugins }: { v: CodexView; now: Date; onAction(a: Action): void; onPlugins(on: boolean): void }) {
  const c = v.custom;
  const busy = v.busy != null;
  const refusal = v.current === 'custom' ? v.backRefusal : v.refusal;
  const mode = keyMode(v);
  const loginSub = hasLogin(v) ? t('đăng nhập còn trong máy', 'sign-in kept on this Mac') : '';
  const chatGpt = t('Tài khoản ChatGPT', 'ChatGPT account');
  return (
    <Card title={t('Nhà cung cấp', 'Providers')} x>
      <div className="grid [--wt-cols:minmax(0,1fr)_auto]" role="list" aria-label={t('Nhà cung cấp Codex', 'Codex providers')}>
        <div className={cn(TABLE_ROW, v.current === 'chatgpt' && TABLE_ROW_ON)} role="listitem" aria-label={chatGpt}>
          <AccountName name={chatGpt} sub={loginSub} full="" chips={hasLogin(v) ? [] : [{ t: t('chưa đăng nhập', 'not signed in'), k: 'warn' }]} />
          <span className="flex items-center justify-end gap-2.5">
            <SideCell
              on={v.current === 'chatgpt'}
              off={!!v.backRefusal || busy}
              label={t('Chuyển sang tài khoản ChatGPT', 'Switch to the ChatGPT account')}
              title={v.backRefusal ? refusalText(v, v.backRefusal) : undefined}
              act="codex:switch:chatGpt"
              onAction={onAction}
            />
          </span>
        </div>
        {c ? (
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
        ) : null}
      </div>
      {v.current === 'other' ? <Note tone="warn">{t('Codex đang dùng nhà cung cấp "' + v.other + '". Chọn một bên để chuyển.', 'Codex is using the provider "' + v.other + '". Pick a side to switch.')}</Note> : null}
      {refusal ? <Note tone={refusal.kind === 'noCustom' ? '' : 'bad'}>{refusalText(v, refusal)}</Note> : null}
      {v.openaiBaseUrl ? <Note tone="warn">{t('config.toml có openai_base_url: tài khoản ChatGPT đang được gửi tới địa chỉ khác.', 'config.toml has openai_base_url: the ChatGPT account is being sent to another address.')}</Note> : null}
      {c ? (
        <CheckLine className="mt-2.5 mb-0" checked={v.plugins} disabled={busy} onChange={onPlugins}>
          <span>
            {pluginsLabel()}
            <small>{pluginsHint()}</small>
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
      <Details v={v} now={now} onAction={onAction} />
    </Card>
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
            <div className="grid gap-1">
              {v.recent.map((e) => (
                <div key={e.at + e.to + e.result} className="grid grid-cols-[64px_minmax(0,1fr)] gap-2">
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

function OnThisMac({ v, onAction }: { v: CodexView; onAction(a: Action): void }) {
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
      <Providers v={v} now={now} onAction={onAction} onPlugins={onPlugins} />
      <div className={GRID2}>
        <OnThisMac v={v} onAction={onAction} />
      </div>
    </>
  );
}

/** Codex on Tổng quan: the provider in use, one line, and the switch to the other side. */
export function CodexCard({ model, onAction }: { model: Model; onAction(act: Action): void }) {
  const v = model.codex;
  if (!v.present) return null;
  const cur = currentId(v);
  const line = codexLine(v);
  const toCustom = v.current !== 'custom';
  const refusal = toCustom ? v.refusal : v.backRefusal;
  const sub =
    (v.current === 'custom'
      ? v.reach.state === 'down'
        ? v.custom!.name + t(' không trả lời', ' is not answering')
        : v.reach.state === 'up'
          ? t('trả lời ' + v.reach.ms + ' ms', 'answers in ' + v.reach.ms + ' ms')
          : v.custom!.name
      : v.current === 'other'
        ? t('nhà cung cấp ', 'provider ') + cur
        : t('đăng nhập chính thức', 'official sign-in')) +
    (hasLogin(v) ? t(' · giữ đăng nhập ChatGPT', ' · keeps the ChatGPT sign-in') : t(' · chưa đăng nhập ChatGPT', ' · not signed in to ChatGPT'));
  return (
    <Panel>
      <CardTitle>
        <IdMark id="x" />
        Codex
        <span className="ml-auto font-reg">{v.procs?.app ? t('Codex App đang mở', 'Codex App is open') : ''}</span>
      </CardTitle>
      <div className={ROW2}>
        <div>
          <b className={line.tone === 'bad' ? 'text-bad-ink!' : undefined}>
            {v.current === 'custom'
              ? t('Qua ', 'Through ') + v.custom!.name
              : v.current === 'other'
                ? t('Nhà cung cấp ', 'Provider ') + cur
                : t('Tài khoản ChatGPT', 'ChatGPT account')}
          </b>
          <small style={{ display: 'block', color: 'var(--ink2)', fontSize: 'var(--fs-xs)' }}>{sub}</small>
        </div>
        {v.custom || v.current !== 'chatgpt' ? (
          <Button
            disabled={!!refusal || v.busy != null}
            title={refusal ? refusalText(v, refusal) : undefined}
            onClick={() => onAction(toCustom ? 'codex:switch:custom' : 'codex:switch:chatGpt')}
          >
            {toCustom ? t('Sang ', 'To ') + (v.custom?.name ?? '…') : t('Về ChatGPT', 'Back to ChatGPT')}
          </Button>
        ) : null}
      </div>
    </Panel>
  );
}
