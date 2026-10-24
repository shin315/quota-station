// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The main window: the prototype's window with the tabs Tổng quan, Claude,
// Antigravity, Luật xoay vòng and Cài đặt. Claude accounts switch from the
// tables (F04), Tổng quan and Claude show automatic switching and why Claude
// is on its account (F07), Luật xoay vòng edits the rotation for every Mac
// (F06), and Antigravity also shows the accounts the app logs in (M4a) and
// switches from its tables once its row lists them (M4c). Lịch sử, Cảnh
// báo and Firstmate come from M6, and Cảnh báo counts the alerts nobody
// answered yet. Codex (M8) has its own tab and a compact card on Tổng quan.
// The Agent group starts with MCP (M9): every agent's MCP servers in one table,
// then Skills, Plugin and File hướng dẫn (M11).
// The Sao lưu group has the backup packs and the restore (M10), and the copy
// to or from another Mac over SSH (M13).
// The Claude tab has a red banner while a variable overrides Claude Code's login (M14).
// Bộ cấu hình (M15) closes the Agent group: named sets applied for the whole Mac.
// Thống kê (M16) follows Tổng quan: the tokens counted from this Mac's session logs.
// Cài đặt › Nâng cao holds what was made for the author's own Macs: off, the
// sidebar has no Luật xoay vòng, Firstmate or Chép sang Mac khác (Firstmate
// also has its own switch), and Tổng quan and Claude show no automatic
// switching, pause or switcher. A build without the Antigravity accounts
// shows the one account signed in, with no login or switch.

import { Fragment, type ReactNode } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Dialog, DialogContent, DialogOverlay } from '@/components/ui/dialog';
import { Segment, Segmented } from '@/components/ui/segmented';
import { cn } from '@/lib/utils';
import { AddAccount, LoginActions, LoginNotes, LogoutConfirm, loginChips, type AgyLogins } from '@agy/AgyAccounts';
import { AccountName, IdMark, InUse, NowCard, WindowCell } from '../components/accounts';
import { Empty, GRID2, STACK, TABLE, TABLE_BOX, TABLE_CELLS, TABLE_END, TABLE_HEAD, TABLE_ROW, TABLE_ROW_ON, TABLE_TIGHT, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, CODE, Dot, Feedback, KVT, Toggle } from '../components/parts';
import type { Tab } from '../lib/model';
import { pauseOptions, type AutoView, type PauseChoice } from '../lib/switching';
import { bannerUnder, type Action, type BannerView, type CardView, type ChipView, type FeedbackView, type NoteView } from '../lib/view';
import type { KeyValue, WindowView } from '../lib/window';
import { t } from '../lib/i18n';
import { Icon, type IconName } from '../components/icons';
import { NAV_ICONS } from './icons';

type NavId = keyof typeof NAV_ICONS;

/** The sidebar, by group. `adv`: a tab Nâng cao holds, marked as such. */
function navs(advanced: boolean): (['-', string] | [NavId, string, 'adv'?])[] {
  return [
    ['overview', t('Tổng quan', 'Overview')],
    ['stats', t('Thống kê', 'Stats')],
    ['-', t('Tài khoản', 'Accounts')],
    ['claude', 'Claude'],
    ['agy', 'Antigravity'],
    ['codex', 'Codex'],
    // Without automatic switching the group only watches.
    ['-', advanced ? t('Tự động', 'Automation') : t('Theo dõi', 'Activity')],
    ['rules', t('Luật xoay vòng', 'Rotation rules'), 'adv'],
    ['history', t('Lịch sử', 'History')],
    ['alerts', t('Cảnh báo', 'Alerts')],
    ['firstmate', 'Firstmate', 'adv'],
    ['-', 'Agent'],
    ['mcp', 'MCP'],
    ['skills', 'Skills'],
    ['plugins', t('Plugin', 'Plugins')],
    ['instr', t('File hướng dẫn', 'Instruction files')],
    ['sets', t('Bộ cấu hình', 'Config sets')],
    ['-', t('Sao lưu', 'Backup')],
    ['backup', t('Sao lưu', 'Backup')],
    ['copy', t('Chép sang Mac khác', 'Copy to another Mac'), 'adv'],
  ];
}

/** The small tag on what Nâng cao holds. */
export function AdvancedTag({ className, hidden }: { className?: string; hidden?: boolean }) {
  return (
    <span aria-hidden={hidden || undefined} className={cn('inline-flex h-[18px] items-center rounded-chip bg-tag px-1.5 text-[10.5px] font-semi whitespace-nowrap text-tag-ink', className)}>
      {t('Nâng cao', 'Advanced')}
    </span>
  );
}

/**
 * The sidebar's mark on what Nâng cao holds: the Nâng cao card's icon on the
 * tag's fill, its name on hover, so the tab's own name keeps its one line.
 */
function AdvancedMark() {
  return (
    <span className="-ml-0.5 inline-grid size-4 flex-none place-items-center rounded-chip bg-tag text-tag-ink" title={t('Nâng cao', 'Advanced')} aria-hidden="true">
      <Icon name="sliders-horizontal" className="size-[11px]" strokeWidth={2.4} />
    </span>
  );
}

function NavButton(props: { id: NavId; label: string; tab: Tab; badge?: number; adv?: boolean; go?: boolean; onTab(tab: Tab): void }) {
  const on = props.id === props.tab;
  return (
    <button
      type="button"
      className={cn(
        'flex min-h-(--nav-h) w-full cursor-pointer items-center gap-2.5 rounded-nav border-0 px-2.5 text-left text-body font-med whitespace-nowrap text-nav [&>svg]:size-[18px] [&>svg]:flex-none',
        on ? 'bg-nav-sel text-nav-sel-ink [&>svg]:text-nav-sel-ink' : 'bg-transparent hover:bg-[color-mix(in_srgb,var(--text-1)_6%,transparent)] [&>svg]:text-ink2',
      )}
      aria-current={on ? 'page' : undefined}
      onClick={() => props.onTab(props.id as Tab)}
    >
      {NAV_ICONS[props.id]}
      <span className="min-w-0 flex-1 truncate">{props.label}</span>
      {/* The tab keeps its plain name for the screen reader. */}
      {props.adv ? <AdvancedMark /> : null}
      {props.badge ? (
        <>
          <span className="inline-flex h-[22px] min-w-[22px] items-center justify-center rounded-chip bg-accent px-1.5 text-xs font-semi text-white" aria-hidden="true">
            {props.badge}
          </span>
          <span className="sr-only">{t(', ' + props.badge + ' cảnh báo mới', ', ' + props.badge + ' new alerts')}</span>
        </>
      ) : null}
      {props.go ? <Icon name="chevron-right" className="size-[15px]! text-ink3" /> : null}
    </button>
  );
}

/** The tabs Nâng cao holds, and the one behind the Firstmate switch. */
function hiddenTabs(advanced: boolean, firstmate: boolean): Set<NavId> {
  const hidden = new Set<NavId>();
  if (!advanced) ['rules', 'copy'].forEach((t) => hidden.add(t as NavId));
  if (!advanced || !firstmate) hidden.add('firstmate');
  return hidden;
}

/** The app's mark at the head of the sidebar. */
function AppMark() {
  return (
    <span className="grid size-9 flex-none place-items-center rounded-lg bg-accent text-white shadow-[inset_0_1px_0_rgba(255,255,255,.25)]" aria-hidden="true">
      <Icon name="chart-column" className="size-[22px]" strokeWidth={2.6} />
    </span>
  );
}

function Sidebar(props: {
  tab: Tab;
  machine: string;
  version: string | null;
  note: string | null;
  alerts: number;
  hidden: Set<NavId>;
  advanced: boolean;
  onTab(tab: Tab): void;
}) {
  const { tab, machine, note, onTab } = props;
  const items = navs(props.advanced).filter((n) => n[0] === '-' || !props.hidden.has(n[0]));
  return (
    <nav
      className="material-side m-0 flex min-h-0 flex-col overflow-hidden rounded-none border-r border-line px-2.5 pt-3.5 pb-2.5"
      aria-label={t('Các tab', 'Tabs')}
      data-tauri-drag-region
    >
      {/* macOS draws the window's traffic lights here; the browser preview draws the prototype's. */}
      <div className="flex flex-none gap-2 pr-1.5 pl-1" data-tauri-drag-region aria-hidden="true">
        <i className={LIGHT} />
        <i className={LIGHT} />
        <i className={LIGHT} />
      </div>
      <div className="flex flex-none items-center gap-2.5 px-1.5 pt-2 pb-2" data-tauri-drag-region>
        <AppMark />
        <span className="grid leading-tight">
          <b className="text-[15px] font-heading tracking-head text-heading">Quota Station</b>
          {props.version ? <small className="text-[12.5px] text-ink2">v{props.version}</small> : null}
        </span>
      </div>
      <div className="-mx-1 flex min-h-0 flex-auto flex-col gap-px overflow-y-auto px-1">
        {items.map((n) =>
          n[0] === '-' ? (
            <div className="px-2.5 pt-2 pb-0.5 text-sm leading-4 text-ink3" key={n[1]}>
              {n[1]}
            </div>
          ) : (
            <NavButton
              key={n[0]}
              id={n[0]}
              label={n[1]}
              adv={n[2] === 'adv'}
              badge={n[0] === 'alerts' ? props.alerts : undefined}
              tab={tab}
              onTab={onTab}
            />
          ),
        )}
      </div>
      <div className="mt-2 flex-none border-t border-divider pt-2">
        <NavButton id="settings" label={t('Cài đặt', 'Settings')} tab={tab} go onTab={onTab} />
      </div>
      {/* One line; the whole of it on hover. */}
      <div className="flex-none truncate px-2.5 pt-1.5 pb-0.5 text-[12.5px] text-ink3" title={note ? machine + ' · ' + note : machine}>
        <span>{machine}</span>
        {note ? ' · ' + note : null}
      </div>
    </nav>
  );
}

function TitleBar(props: { title: string; view: WindowView; busy: boolean; onRefresh(): void }) {
  return (
    <TabHead>
      <TabTitle>{props.title}</TabTitle>
      <TabSub>
        <Dot tone={props.view.fresh.tone} />
        {props.view.machine} · {props.view.fresh.text}
      </TabSub>
      <Button lead="refresh-cw" className="min-h-[42px] px-5 text-[15px]" disabled={props.busy} onClick={props.onRefresh}>
        {t('Làm mới', 'Refresh')}
      </Button>
    </TabHead>
  );
}

/** The traffic lights the browser preview draws; macOS draws its own in the app. */
const LIGHT = 'size-3 rounded-full bg-traffic shadow-[inset_0_0_0_.5px_var(--line-2)] [body:not(.preview)_&]:invisible';

/** A card: a service's (`kind`) has its logo and a large name; `under`: a line under the name. */
function Card(props: { title: ReactNode; kind?: 'c' | 'g'; big?: boolean; icon?: IconName; tag?: ReactNode; under?: ReactNode; end?: ReactNode; children: ReactNode }) {
  const title = (
    <>
      {props.kind ? <IdMark id={props.kind} className="size-[26px]" /> : props.icon ? <Icon name={props.icon} /> : null}
      {props.title}
      {props.tag}
    </>
  );
  return (
    <Panel>
      <CardTitle big={!!props.kind || props.big}>
        {props.under ? (
          <span className="grid min-w-0 gap-0.5">
            <span className="flex items-center gap-2.5">{title}</span>
            <small className="text-sm font-reg tracking-normal text-ink2 tabular-nums">{props.under}</small>
          </span>
        ) : (
          title
        )}
        {props.end ? <span className="ml-auto flex items-center gap-2.5 text-body font-reg tracking-normal">{props.end}</span> : null}
      </CardTitle>
      {props.children}
    </Panel>
  );
}

/** "Chuyển" is a button to press; the account in use gets a status in its place. */
function SwitchCell({ card, onSwitch }: { card: CardView; onSwitch(slot: number): void }) {
  if (card.active) return <InUse />;
  if (card.slot === 0) return null;
  return (
    <Button small disabled={!card.switchable} label={t('Chuyển sang ' + card.name, 'Switch to ' + card.name)} title={card.off || undefined} onClick={() => onSwitch(card.slot)}>
      {t('Chuyển', 'Switch')}
    </Button>
  );
}

/** A row of a window table (see TABLE_ROW). */
const ROW = TABLE_ROW;
/** The table's head rows: the windows' names. */
const HEAD = TABLE_HEAD;

/** One account in a window table: its name, a fixed column per window, and its buttons. */
function TableRow(props: { card: CardView; extra?: ReactNode; menu?: ReactNode; chips?: ChipView[]; tight?: boolean; onSwitch(slot: number): void }) {
  const c = props.card;
  return (
    <div className={cn(ROW, c.active && TABLE_ROW_ON, c.resting && 'bg-card-2')} role="listitem" aria-label={c.label} title={c.title || undefined}>
      <AccountName name={c.name} sub={c.sub} full={c.full} dot={c.active ? 'on' : c.resting ? 'rest' : 'off'} chips={[...c.chips, ...(props.chips ?? [])]} />
      <span className={TABLE_CELLS}>
        {c.cells.map((cell) => (
          <WindowCell key={cell.label} cell={cell} dim={c.dim} />
        ))}
      </span>
      <span className={cn('flex flex-wrap items-center justify-end gap-1.5', TABLE_END)}>
        <SwitchCell card={c} onSwitch={props.onSwitch} />
        {props.extra}
      </span>
      {props.menu !== undefined ? <span className={cn('flex justify-end', TABLE_END)}>{props.menu}</span> : null}
    </div>
  );
}

/** Claude: 5 giờ · Tuần · Fable tuần, the same columns for every account. */
function ClaudeTable(props: { cards: CardView[]; tight?: boolean; menu?(card: CardView): ReactNode; onSwitch(slot: number): void }) {
  if (!props.cards.length) return null;
  // Whole class names, so Tailwind finds them; the Claude tab adds the ⋯ column.
  // The buttons' column is as wide as the widest row's buttons.
  const cols = props.menu
    ? '[--wt-cols:minmax(120px,1.4fr)_repeat(3,minmax(72px,1fr))_auto_28px]'
    : '[--wt-cols:minmax(120px,1.4fr)_repeat(3,minmax(72px,1fr))_auto]';
  return (
    <div className={cn(TABLE_BOX, props.tight && TABLE_TIGHT)}>
      <div className={cn(TABLE, cols)} role="list" aria-label="Claude Code">
        <div className={HEAD} aria-hidden="true">
          <span>{t('Tài khoản', 'Account')}</span>
          <span>{t('5 giờ', '5-hour')}</span>
          <span>{t('Tuần', 'Week')}</span>
          <span>{t('Fable tuần', 'Fable week')}</span>
          <span />
          {props.menu ? <span /> : null}
        </div>
        {props.cards.map((c) => (
          <TableRow key={c.key} card={c} tight={props.tight} menu={props.menu ? props.menu(c) : undefined} onSwitch={props.onSwitch} />
        ))}
      </div>
    </div>
  );
}

/**
 * Antigravity: Gemini and Claude/GPT, five hours and a week each. An
 * account the app has no reading of has one number per group instead.
 */
function AgyTable(props: { cards: CardView[]; logins?: AgyLogins; tight?: boolean; onSwitch(slot: number): void }) {
  if (!props.cards.length) return null;
  const four = props.cards.every((c) => c.cells.length === 4);
  const logins = props.logins;
  // Whole class names, so Tailwind finds them. The buttons' column (Chuyển, and
  // with the logins the verification page and Đăng xuất) is as wide as the widest row's.
  const cols = four
    ? '[--wt-cols:minmax(120px,1.6fr)_repeat(4,minmax(62px,1fr))_auto]'
    : '[--wt-cols:minmax(120px,1.3fr)_repeat(2,minmax(72px,1fr))_auto]';
  return (
    <div className={cn(TABLE_BOX, props.tight && TABLE_TIGHT)}>
      <div className={cn(TABLE, cols)} role="list" aria-label="Antigravity">
        {four ? (
          <>
            <div className={cn(HEAD, 'pb-0 text-ink2')} aria-hidden="true">
              <span />
              <span className={GROUP}>Gemini</span>
              <span className={GROUP}>Claude/GPT</span>
              <span />
            </div>
            <div className={cn(HEAD, 'rounded-none pt-px')} aria-hidden="true">
              <span>{t('Tài khoản', 'Account')}</span>
              <span>{t('5 giờ', '5-hour')}</span>
              <span>{t('Tuần', 'Week')}</span>
              <span>{t('5 giờ', '5-hour')}</span>
              <span>{t('Tuần', 'Week')}</span>
              <span />
            </div>
          </>
        ) : (
          <div className={HEAD} aria-hidden="true">
            <span>{t('Tài khoản', 'Account')}</span>
            <span>Gemini</span>
            <span>Claude/GPT</span>
            <span />
          </div>
        )}
        {props.cards.map((c) => {
          const row = logins?.rows.find((r) => r.number === c.slot);
          return (
            <Fragment key={c.key}>
              <TableRow
                card={c}
                tight={props.tight}
                chips={loginChips(row)}
                extra={logins && row ? <LoginActions logins={logins} row={row} /> : null}
                onSwitch={props.onSwitch}
              />
              {logins && row && logins.confirm === row.id ? <LogoutConfirm logins={logins} row={row} inTable /> : null}
            </Fragment>
          );
        })}
      </div>
    </div>
  );
}

/** Two head cells over a service's windows. */
const GROUP = 'col-span-2';

function Note({ note }: { note: NoteView | null }) {
  return note ? (
    <CardText className={cn('mt-2 mb-0', note.tone === 'warn' ? 'text-warn-ink' : note.tone === 'bad' ? 'text-bad-ink' : undefined)}>{note.text}</CardText>
  ) : null;
}

function KeyValues({ rows }: { rows: KeyValue[] }) {
  return (
    <div className={KVT}>
      {rows.map((r) => [
        <span key={r.key + ':k'}>{r.key}</span>,
        <span key={r.key + ':v'} className={r.mono ? CODE : undefined}>
          {r.chip ? <Chip chip={r.chip} /> : r.value}
        </span>,
      ])}
    </div>
  );
}

/** Automatic switching: its switch in the title row, then on or paused and why Claude is on its account, the card's whole width. */
function AutoCard({ auto, tag, onToggle, children }: { auto: AutoView; tag?: ReactNode; onToggle(): void; children?: ReactNode }) {
  return (
    <Card
      title={t('Tự chuyển Claude', 'Claude auto-switch')}
      icon="shuffle"
      tag={tag}
      end={<Toggle on={auto.toggle.on} label={auto.toggle.label} disabled={auto.toggle.disabled} onClick={onToggle} />}
    >
      <b className="block text-body font-med text-heading">{auto.paused ? auto.title : t('Đang bật', 'On')}</b>
      <small className="mt-px block text-[13px] text-ink2">{auto.line}</small>
      {children}
    </Card>
  );
}

/** A card with nothing in it yet: a dashed box, a bold line, a word, and its button. */
export function EmptyBox(props: { icon?: ReactNode; title: string; body?: string; children?: ReactNode }) {
  return (
    <div className="grid justify-items-center gap-1.5 rounded-card border border-dashed border-line-2 bg-card px-4 py-[22px] text-center">
      {props.icon}
      <b className="text-body font-semi text-heading">{props.title}</b>
      {props.body ? <small className="max-w-[420px] text-sm text-ink2">{props.body}</small> : null}
      {props.children ? <div className="mt-1.5 flex flex-wrap justify-center gap-2">{props.children}</div> : null}
    </div>
  );
}

// ===== Tabs =====

interface TabProps {
  view: WindowView;
  busy: Action | null;
  feedback: FeedbackView | null;
  onAction(act: Action): void;
  onSwitch(slot: number): void;
  /** An Antigravity row's switch button (M4c). */
  onSwitchAgy(number: number): void;
  onToggle(): void;
  onPause(preset: PauseChoice): void;
  now: Date;
  /** The accounts the app logs in, merged into the Antigravity table. */
  agyLogins?: AgyLogins;
  /** Codex's card on Tổng quan (M8). */
  codexCard?: ReactNode;
  /** The Claude accounts Quota Station keeps, in the Claude tab. */
  claude?: ClaudeManage;
}

/** The Claude tab's accounts, from `ClaudeAccounts`: the ⋯ menus, the add button, the rotation. */
export interface ClaudeManage {
  /** The ⋯ menu of an account Quota Station keeps; null for one it does not. */
  menu(card: CardView): ReactNode;
  onAdd(): void;
  /** No account yet: the empty box's other way in (claude-swap's import). */
  emptyExtra?: ReactNode;
  /** Before Quota Station keeps the accounts claude-swap lists: the banner to bring them in. */
  setup?: ReactNode;
  /** The rotation's LaunchAgent: its switch, the old switcher's warning. In the Switcher card with Nâng cao. */
  rotation: ReactNode;
  /** Without Nâng cao, a card of its own: only while the rotation runs or the old switcher is loaded. */
  rotationAlone: boolean;
}

function Top(props: TabProps & { title: string; banner?: boolean }) {
  const banner = props.banner !== false ? bannerUnder(props.view.banner, props.feedback) : null;
  return (
    <>
      <TitleBar title={props.title} view={props.view} busy={props.busy === 'refresh'} onRefresh={() => props.onAction('refresh')} />
      {props.feedback ? <Feedback fb={props.feedback} onAction={props.onAction} /> : null}
      {banner ? <Banner banner={banner} onAction={props.onAction} busy={props.busy} /> : null}
    </>
  );
}

function Overview(props: TabProps & { advanced: boolean; agyAccounts: boolean; release?: BannerView | null; onTab(tab: Tab): void }) {
  const v = props.view;
  return (
    <>
      <Top {...props} title={t('Tổng quan', 'Overview')} />

      <NowCard now={v.now} />
      {/* The rail takes its share of the page, as the old 2.6fr/1fr did, and goes
          under the accounts where the page (not the window) is too narrow for both. */}
      <div className="grid grid-cols-[minmax(0,2.6fr)_minmax(252px,1fr)] items-start gap-3 @max-[960px]/main:grid-cols-[minmax(0,1fr)]">
        <div className={STACK}>
          {v.claude.present ? (
            <Card title="Claude" kind="c">
              {v.claude.empty ? (
                <EmptyBox title={t('Chưa có tài khoản Claude nào', 'No Claude accounts yet')}>
                  <Button small primary onClick={() => props.onTab('claude')}>
                    {t('Thêm tài khoản…', 'Add account…')}
                  </Button>
                </EmptyBox>
              ) : (
                <ClaudeTable cards={v.claude.cards} tight onSwitch={props.onSwitch} />
              )}
              <Note note={v.claude.note} />
            </Card>
          ) : null}
          {v.agy.present ? (
            <Card title="Antigravity" kind="g" tag={props.agyAccounts ? null : <Chip chip={{ t: t('chỉ đọc', 'read-only'), k: 'tag' }} />}>
              <AgyTable cards={v.agy.cards} tight onSwitch={props.onSwitchAgy} />
              <Note note={v.agy.note} />
            </Card>
          ) : null}
          {props.codexCard}
        </div>
        <div className={cn(STACK, '@max-[960px]/main:grid-cols-[repeat(auto-fit,minmax(260px,1fr))] @max-[960px]/main:items-start')}>
          {props.advanced && v.claude.cards.length > 1 ? <AutoCard auto={v.auto} onToggle={props.onToggle} /> : null}
          <Card title={t('Sắp tới', 'Coming up')} icon="clock">
            {v.upcoming.length ? (
              <ul className="m-0 grid list-none p-0">
                {v.upcoming.map((u) => (
                  // The time first, in its own column (tomorrow under it), the event the rest of the width.
                  <li key={u.key} className="grid grid-cols-[3.75rem_minmax(0,1fr)] items-start gap-x-2.5 border-t border-divider py-2.5 text-sm first:border-t-0 first:pt-0.5">
                    <time className="grid tabular-nums">
                      <span className="flex items-center gap-1.5">
                        <Dot tone={u.tone || 'off'} />
                        <b className="text-body font-semi text-heading">{u.clock}</b>
                      </span>
                      {u.day ? <small className="pl-3.5 text-xs text-ink3">{u.day}</small> : null}
                    </time>
                    <span className="pt-px">{u.what}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <CardText className="mb-0">{t('Không có mốc đặt lại nào trong 24 giờ tới.', 'No resets in the next 24 hours.')}</CardText>
            )}
          </Card>
        </div>
      </div>
      {/* A newer release: a plain note held at the window's foot, under everything else (u-overview). */}
      {props.release ? (
        // Frosted, edge to edge, over the page's foot padding: the page scrolls by under it.
        <div className="sticky -bottom-[22px] z-[5] mt-auto -mr-[22px] -mb-[22px] -ml-3.5 border-t border-line bg-glass-win pt-2.5 pr-[22px] pb-3.5 pl-3.5 backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-sat)]">
          <div className="rounded-card bg-glass-solid shadow-card">
            <Banner banner={props.release} onAction={props.onAction} busy={props.busy} />
          </div>
        </div>
      ) : null}
    </>
  );
}

function ClaudeTab(props: TabProps & { advanced: boolean }) {
  const v = props.view;
  const a = v.auto;
  const sw = v.switcher;
  const m = props.claude;
  const cards = v.claude.cards;
  if (!v.claude.present && !cards.length)
    return (
      <>
        <Top {...props} title="Claude" />
        <EmptyBox icon={<IdMark id="c" className="size-[34px]" />} title={t('Máy này chưa có Claude Code', 'This Mac has no Claude Code')} body={t('Cài Claude Code rồi mở lại tab này. Quota Station đọc ~/.claude.', 'Install Claude Code, then open this tab again. Quota Station reads ~/.claude.')} />
      </>
    );
  return (
    <>
      <Top {...props} title="Claude" />
      {v.envBanner ? <Banner banner={v.envBanner} onAction={props.onAction} busy={props.busy} /> : null}
      {v.claudeBanner ? <Banner banner={v.claudeBanner} onAction={props.onAction} busy={props.busy} /> : null}
      <Card
        title={t('Tài khoản', 'Accounts')}
        big
        under={v.claude.count || null}
        end={
          m ? (
            <Button primary lead="plus" onClick={m.onAdd}>
              {t('Thêm tài khoản', 'Add account')}
            </Button>
          ) : null
        }
      >
        {cards.length && m?.setup ? <div className="mb-3">{m.setup}</div> : null}
        {cards.length ? (
          <ClaudeTable cards={cards} menu={m?.menu} onSwitch={props.onSwitch} />
        ) : (
          <EmptyBox
            icon={<IdMark id="c" className="size-[34px]" />}
            title={t('Chưa có tài khoản Claude nào', 'No Claude accounts yet')}
            body={t(
              'Quota Station giữ đăng nhập của từng tài khoản trong Keychain của máy này, để bạn chuyển Claude Code giữa các tài khoản.',
              'Quota Station keeps each account’s sign-in in this Mac’s Keychain so you can switch Claude Code between them.',
            )}
          >
            {m ? (
              <Button small primary onClick={m.onAdd}>
                {t('Thêm tài khoản…', 'Add account…')}
              </Button>
            ) : null}
            {m?.emptyExtra}
          </EmptyBox>
        )}
        {cards.length === 1 ? (
          <CardText className="mt-2.5 mb-0 text-sm text-ink2">{t('Thêm tài khoản thứ hai để chuyển nhanh từ popover.', 'Add a second account to switch quickly from the popover.')}</CardText>
        ) : null}
        <Note note={v.claude.note} />
      </Card>
      {props.advanced ? (
        <div className={GRID2}>
          <AutoCard auto={a} tag={<AdvancedTag />} onToggle={props.onToggle}>
            <CardText style={{ margin: '12px 0 4px' }} id="qs-qp">
              {t('Tạm dừng', 'Pause')}
            </CardText>
            <Segmented aria-labelledby="qs-qp">
              {pauseOptions(props.now).map((o) => (
                <Segment key={o.v} disabled={a.toggle.disabled} onClick={() => props.onPause(o.v)}>
                  {o.t}
                </Segment>
              ))}
            </Segmented>
          </AutoCard>
          <Card title="Switcher" tag={<AdvancedTag />}>
            <div className="text-sm text-ink2">
              <Dot tone={sw.tone} className="mr-1.5" />
              {sw.line}
            </div>
            <details className="mt-2 text-sm text-ink2">
              <summary className="cursor-pointer list-none text-accent-ink [&::-webkit-details-marker]:hidden">{t('Chi tiết', 'Details')}</summary>
              <div className="mt-2">
                <KeyValues rows={sw.details} />
              </div>
            </details>
            {m?.rotation}
          </Card>
        </div>
      ) : m?.rotationAlone ? (
        <Card title={t('Tự chuyển bằng Quota Station', 'Auto-switch with Quota Station')} icon="shuffle">
          {m.rotation}
        </Card>
      ) : null}
    </>
  );
}

function AgyTab(props: TabProps & { agyAccounts: boolean }) {
  const a = props.view.agy;
  const logins = props.agyLogins;
  return (
    <>
      <Top {...props} title="Antigravity" banner={false} />
      {a.banner ? <Banner banner={a.banner} onAction={props.onAction} busy={props.busy} /> : null}
      <Card title={props.agyAccounts ? t('Tài khoản', 'Accounts') : t('Tài khoản', 'Account')} kind="g" end={logins ? <AddAccount logins={logins} /> : null}>
        {a.present ? <AgyTable cards={a.cards} logins={a.own ? logins : undefined} onSwitch={props.onSwitchAgy} /> : null}
        {!a.present ? (
          <Empty>
            <b>{t('Máy này chưa có Antigravity', 'This Mac has no Antigravity')}</b>
          </Empty>
        ) : null}
        <Note note={a.tabNote} />
        {logins ? <LoginNotes logins={logins} listed={a.own ? a.cards.map((c) => c.slot) : []} /> : null}
        {!props.agyAccounts && a.present ? (
          <CardText className="mt-2 mb-0">
            {t(
              'Đổi tài khoản trong chính Antigravity; Quota Station đọc lại ở lần đọc sau.',
              'Change the account inside Antigravity itself; Quota Station picks it up on its next read.',
            )}
          </CardText>
        ) : null}
      </Card>
      {!props.agyAccounts && a.present ? (
        <Card title={t('Nhiều tài khoản Antigravity', 'Several Antigravity accounts')}>
          <CardText className="mb-0">
            {t(
              'Bản này chỉ đọc tài khoản đang đăng nhập. Thêm, đăng nhập và chuyển nhiều tài khoản có ở bản build từ mã nguồn với phần Antigravity, theo giấy phép CC BY-NC-SA 4.0 của phần đó.',
              'This build only reads the signed-in account. Adding, signing in and switching several accounts comes with a build from source that includes the Antigravity part, under that part’s CC BY-NC-SA 4.0 licence.',
            )}
          </CardText>
        </Card>
      ) : null}
    </>
  );
}

export interface MainWindowProps {
  tab: Tab;
  view: WindowView;
  busy: Action | null;
  feedback: FeedbackView | null;
  now: Date;
  onTab(tab: Tab): void;
  onAction(act: Action): void;
  onSwitch(slot: number): void;
  /** An Antigravity row's switch button; none leaves the rows without one that works. */
  onSwitchAgy?(number: number): void;
  onToggle(): void;
  onPause(preset: PauseChoice): void;
  /** The Cài đặt tab, drawn by the settings component. */
  settings: ReactNode;
  /** The Luật xoay vòng tab, drawn by the rules editor. */
  rules: ReactNode;
  /** The Codex tab (M8) and Codex's card on Tổng quan. */
  codex?: ReactNode;
  codexCard?: ReactNode;
  /** The MCP tab (M9). */
  mcp?: ReactNode;
  /** The Skills, Plugin and File hướng dẫn tabs (M11). */
  skills?: ReactNode;
  plugins?: ReactNode;
  instr?: ReactNode;
  /** The Bộ cấu hình tab (M15). */
  sets?: ReactNode;
  /** The Thống kê tab (M16). */
  stats?: ReactNode;
  /** The Sao lưu tab (M10). */
  backup?: ReactNode;
  /** The Chép sang Mac khác tab (M13). */
  copy?: ReactNode;
  /** The Lịch sử, Cảnh báo and Firstmate tabs (M6). */
  history?: ReactNode;
  alerts?: ReactNode;
  firstmate?: ReactNode;
  /** Alerts with buttons nobody pressed yet, on the sidebar. */
  unanswered?: number;
  /** Under the machine name in the sidebar: whether the other Macs have the rules. */
  syncNote?: string | null;
  /** The accounts the app logs in, merged into the Antigravity table. */
  agyLogins?: AgyLogins;
  /** The Claude accounts Quota Station keeps, in the Claude tab. */
  claude?: ClaudeManage;
  /** The app's version, under its name in the sidebar. */
  version?: string | null;
  /** A newer release on GitHub, as Tổng quan's banner. */
  release?: BannerView | null;
  /** Cài đặt › Nâng cao is on: its tabs and switches show. */
  advanced: boolean;
  /** The Firstmate switch inside Nâng cao is on too. */
  firstmateTab: boolean;
  /** This build logs Antigravity accounts in and switches them. */
  agyAccounts: boolean;
  /** The open dialog's body; null when none is open. */
  dialog?: ReactNode;
  /** Its width: the MCP preview and form need more room than a confirmation. */
  dialogSize?: 'wide' | 'lg' | 'xl';
  /** A new install's first run: the whole window, without the sidebar. */
  setup?: ReactNode;
}

export function MainWindow(props: MainWindowProps) {
  const tabProps: TabProps = {
    view: props.view,
    busy: props.busy,
    feedback: props.feedback,
    onAction: props.onAction,
    onSwitch: props.onSwitch,
    onSwitchAgy: props.onSwitchAgy ?? (() => {}),
    onToggle: props.onToggle,
    onPause: props.onPause,
    now: props.now,
    agyLogins: props.agyAccounts ? props.agyLogins : undefined,
    codexCard: props.codexCard,
    claude: props.claude,
  };
  const open = !!props.dialog;
  const hidden = hiddenTabs(props.advanced, props.firstmateTab);
  // A tab Nâng cao took away while it was open gives way to Tổng quan.
  const tab: Tab = hidden.has(props.tab as NavId) ? 'overview' : props.tab;
  return (
    <div className={cn('win-o material-window relative grid h-full w-full overflow-hidden', props.setup ? 'grid-cols-[minmax(0,1fr)]' : 'grid-cols-[var(--side-w)_minmax(0,1fr)]')}>
      {/* The window's top edge drags it, over the sidebar and the page alike, as a title bar would. */}
      <div className="dragbar absolute top-0 right-0 left-0 z-10 h-4" data-tauri-drag-region aria-hidden="true" />
      {props.setup ? (
        <main className="main material-main @container/main relative flex min-w-0 overflow-y-auto" inert={open}>
          {props.setup}
        </main>
      ) : (
        <div style={{ display: 'contents' }} inert={open}>
          <Sidebar
            tab={tab}
            machine={props.view.machine}
            version={props.version ?? null}
            note={props.advanced ? (props.syncNote ?? null) : null}
            alerts={props.unanswered ?? 0}
            hidden={hidden}
            advanced={props.advanced}
            onTab={props.onTab}
          />
          {/* The page is the container its layouts measure: the window less the sidebar. */}
          <main className="main material-main @container/main relative flex min-w-0 flex-col gap-3 overflow-y-auto pt-4 pr-[22px] pb-[22px] pl-3.5">
            {tab === 'overview' ? <Overview {...tabProps} advanced={props.advanced} agyAccounts={props.agyAccounts} release={props.release} onTab={props.onTab} /> : null}
            {tab === 'stats' ? props.stats : null}
            {tab === 'claude' ? <ClaudeTab {...tabProps} advanced={props.advanced} /> : null}
            {tab === 'agy' ? <AgyTab {...tabProps} agyAccounts={props.agyAccounts} /> : null}
            {tab === 'codex' ? props.codex : null}
            {tab === 'rules' ? props.rules : null}
            {tab === 'history' ? props.history : null}
            {tab === 'alerts' ? props.alerts : null}
            {tab === 'firstmate' ? props.firstmate : null}
            {tab === 'mcp' ? props.mcp : null}
            {tab === 'skills' ? props.skills : null}
            {tab === 'plugins' ? props.plugins : null}
            {tab === 'instr' ? props.instr : null}
            {tab === 'sets' ? props.sets : null}
            {tab === 'backup' ? props.backup : null}
            {tab === 'copy' ? props.copy : null}
            {tab === 'settings' ? props.settings : null}
          </main>
        </div>
      )}
      {/* The dialog layer covers the whole window, sidebar included; the
          dialog stands on the page beside the sidebar, near the top, as drawn.
          The one row fills the window below the top margin, so the dialog's
          height can stop 36 px short of the bottom (100% − 120 px of the window). */}
      <Dialog open={open}>
        {open ? (
          <DialogOverlay
            className={cn(
              'z-30 grid grid-rows-[minmax(0,1fr)] items-start justify-items-center rounded-[inherit] pt-[84px] [@media(max-height:640px)]:pt-6',
              // Beside the sidebar where the widest dialog fits there; over the whole window, as the old one stood, where it does not.
              !props.setup && 'min-[960px]:pl-(--side-w)',
            )}
          >
            <DialogContent size={props.dialogSize ?? 'wide'} className="max-h-[calc(100%-36px)]">{props.dialog}</DialogContent>
          </DialogOverlay>
        ) : null}
      </Dialog>
    </div>
  );
}
