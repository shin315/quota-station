// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The main window: the prototype's window with the tabs Tổng quan, Claude,
// Antigravity, Luật xoay vòng and Cài đặt. Claude accounts switch from the
// tables (F04), Tổng quan and Claude show automatic switching and why Claude
// is on its account (F07), Luật xoay vòng edits the rotation for every Mac
// (F06), and Antigravity also shows the accounts the app logs in (M4a) and
// switches from its tables once its row lists them (M4c). Lịch sử and
// Firstmate come from M6, and Thông báo, the bell at the right of every
// page's title row, with its red count of the notifications not read yet.
// Codex (M8) has its own tab and a compact card on Tổng quan.
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
import { AccountName, IdMark, InUse, WindowCell } from '../components/accounts';
import { Empty, HeadEnd, OV_COLS, TABLE, TABLE_BOX, TABLE_CELLS, TABLE_END, TABLE_HEAD, TABLE_ROW, TABLE_ROW_ON, TabHead, TabNote, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, CODE, Dot, Feedback, KVT, More, Toggle } from '../components/parts';
import type { Tab } from '../lib/model';
import { pauseOptions, type AutoView, type PauseChoice } from '../lib/switching';
import { bannerUnder, type Action, type BannerView, type CardView, type ChipView, type FeedbackView, type NoteView } from '../lib/view';
import type { KeyValue, WindowView } from '../lib/window';
import { mac, onWindows, os, t } from '../lib/i18n';
import { Glyph, type GlyphName } from '../components/icons';
import { NAV_ICONS } from './icons';

type NavId = keyof typeof NAV_ICONS;

/** The sidebar, by group. */
function navs(advanced: boolean): (['-', string] | [NavId, string])[] {
  return [
    ['overview', t('Tổng quan', 'Overview')],
    ['stats', t('Thống kê', 'Stats')],
    ['-', t('Tài khoản', 'Accounts')],
    ['claude', 'Claude'],
    ['agy', 'Antigravity'],
    ['codex', 'Codex'],
    // Without automatic switching the group only watches.
    ['-', advanced ? t('Tự động', 'Automation') : t('Theo dõi', 'Activity')],
    ['rules', t('Luật xoay vòng', 'Rotation rules')],
    ['history', t('Lịch sử', 'History')],
    ['firstmate', 'Firstmate'],
    ['-', 'Agent'],
    ['mcp', 'MCP'],
    ['skills', 'Skills'],
    ['plugins', t('Plugin', 'Plugins')],
    ['instr', t('File hướng dẫn', 'Instruction files')],
    ['sets', t('Bộ cấu hình', 'Config sets')],
    ['-', t('Sao lưu', 'Backup')],
    ['backup', t('Sao lưu', 'Backup')],
    ['copy', t('Chép sang Mac khác', 'Copy to another Mac')],
  ];
}

/** The small tag on what Nâng cao holds. */
export function AdvancedTag({ className, hidden }: { className?: string; hidden?: boolean }) {
  return (
    <span aria-hidden={hidden || undefined} className={cn('rounded-chip bg-accent/14 px-1.5 text-cap leading-4 font-semi whitespace-nowrap text-info-ink', className)}>
      {t('Nâng cao', 'Advanced')}
    </span>
  );
}

function NavButton(props: { id: NavId; label: string; tab: Tab; onTab(tab: Tab): void }) {
  const on = props.id === props.tab;
  return (
    <button
      type="button"
      className={cn(
        'flex w-full cursor-pointer items-center gap-[9px] rounded-lg border-0 px-2 py-[5px] text-left text-base whitespace-nowrap [&>svg]:flex-[0_0_auto]',
        on ? 'bg-fill2 font-semi' : 'bg-transparent hover:bg-fill',
      )}
      aria-current={on ? 'page' : undefined}
      onClick={() => props.onTab(props.id as Tab)}
    >
      {NAV_ICONS[props.id]}
      <span className="min-w-0 truncate">{props.label}</span>
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

function Sidebar(props: { tab: Tab; machine: string; note: string | null; hidden: Set<NavId>; advanced: boolean; onTab(tab: Tab): void }) {
  const { tab, machine, note, onTab } = props;
  const items = navs(props.advanced).filter((n) => n[0] === '-' || !props.hidden.has(n[0]));
  return (
    <nav className="material-side m-0 flex min-h-0 flex-col gap-px overflow-y-auto rounded-none px-2.5 pt-5 pb-2.5 win:pt-2.5" aria-label={t('Các tab', 'Tabs')} data-tauri-drag-region>
      {/* macOS draws the window's traffic lights here; the browser preview draws the prototype's. Windows draws its title bar above the page. */}
      {onWindows() ? null : (
        <div className="flex min-h-3.5 flex-none items-center gap-[7px] px-3 pb-3.5" data-tauri-drag-region aria-hidden="true">
          <i className={LIGHT + ' bg-[#ff5f57]'} />
          <i className={LIGHT + ' bg-[#febc2e]'} />
          <i className={LIGHT + ' bg-[#28c840]'} />
        </div>
      )}
      {items.map((n) =>
        n[0] === '-' ? (
          <div className="px-2 pt-2 pb-0.5 text-cap font-semi tracking-[.02em] text-ink2" key={n[1]}>
            {n[1]}
          </div>
        ) : (
          <NavButton key={n[0]} id={n[0]} label={n[1]} tab={tab} onTab={onTab} />
        ),
      )}
      <div className="flex-1" data-tauri-drag-region />
      <NavButton id="settings" label={t('Cài đặt', 'Settings')} tab={tab} onTab={onTab} />
      {/* One line; the whole of it on hover. */}
      <div className="truncate px-2 py-1.5 text-s text-ink2" title={note ? machine + ' · ' + note : machine}>
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
      <Button disabled={props.busy} onClick={props.onRefresh}>
        {t('Làm mới', 'Refresh')}
      </Button>
    </TabHead>
  );
}

/** The traffic lights the browser preview draws; macOS draws its own in the app. */
const LIGHT = 'size-3 rounded-full [body:not(.preview)_&]:invisible';

/**
 * A card: a service's (`kind`) has its mark; `under`: a line under the name;
 * `end`: at the title's right; `head`: a wider part of the head (Tổng quan's
 * automatic switching), which goes under the title where the page is narrow.
 */
function Card(props: { title: ReactNode; kind?: 'c' | 'g'; icon?: GlyphName; tag?: ReactNode; under?: ReactNode; end?: ReactNode; head?: ReactNode; children: ReactNode }) {
  return (
    <Panel>
      <CardTitle className={props.head ? 'flex-wrap gap-y-1' : undefined}>
        {props.kind ? <IdMark id={props.kind} /> : props.icon ? <Glyph name={props.icon} /> : null}
        {props.title}
        {props.tag}
        {props.under ? <span className="font-reg text-ink3">{props.under}</span> : null}
        {props.end ? <span className="ml-auto flex items-center gap-2 font-reg">{props.end}</span> : null}
        {props.head}
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
    <Button primary disabled={!card.switchable} label={t('Chuyển sang ' + card.name, 'Switch to ' + card.name)} title={card.off || undefined} onClick={() => onSwitch(card.slot)}>
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
    <div className={cn(ROW, c.active && TABLE_ROW_ON, c.resting && 'rounded-lg bg-fill')} role="listitem" aria-label={c.label} title={c.title || undefined}>
      <AccountName name={c.name} sub={c.sub} full={c.full} chips={[...c.chips, ...(props.chips ?? [])]} />
      <span className={TABLE_CELLS}>
        {c.cells.map((cell) => (
          <WindowCell key={cell.label} cell={cell} dim={c.dim} tight={props.tight} />
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

/**
 * Claude: 5 giờ · Tuần · Fable tuần, the same columns for every account.
 * `tight`: Tổng quan's table, whose name and buttons columns the other
 * sections share (OV_COLS).
 */
function ClaudeTable(props: { cards: CardView[]; tight?: boolean; menu?(card: CardView): ReactNode; onSwitch(slot: number): void }) {
  if (!props.cards.length) return null;
  // Whole class names, so Tailwind finds them; the Claude tab adds the ⋯ column.
  // The buttons' column is as wide as the widest row's buttons.
  const cols = props.tight
    ? OV_COLS.claude
    : props.menu
      ? '[--wt-cols:minmax(96px,1.25fr)_repeat(3,minmax(64px,1fr))_minmax(66px,auto)_28px]'
      : '[--wt-cols:minmax(96px,1.25fr)_repeat(3,minmax(64px,1fr))_minmax(66px,auto)]';
  return (
    <div className={TABLE_BOX}>
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
  const cols = props.tight
    ? four
      ? OV_COLS.agy4
      : OV_COLS.agy2
    : four
      ? '[--wt-cols:minmax(96px,1.25fr)_repeat(4,minmax(56px,.82fr))_minmax(66px,auto)]'
      : '[--wt-cols:minmax(96px,1.25fr)_repeat(2,minmax(64px,1fr))_minmax(66px,auto)]';
  return (
    <div className={TABLE_BOX}>
      <div className={cn(TABLE, cols)} role="list" aria-label="Antigravity">
        {four ? (
          <>
            <div className={HEAD} aria-hidden="true">
              <span />
              <span className={GROUP}>Gemini</span>
              <span className={GROUP}>Claude/GPT</span>
              <span />
            </div>
            <div className={cn(HEAD, 'border-t-0')} aria-hidden="true">
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
const GROUP = 'col-span-2 border-b-[.5px] border-card-line pb-0.5';

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

/** A card with nothing in it yet: a dashed box, a bold line, a word, and its button. */
export function EmptyBox(props: { icon?: ReactNode; title: string; body?: string; children?: ReactNode }) {
  return (
    <div className="grid justify-items-center gap-1 rounded-tile border border-dashed border-glass-line px-4 py-4 text-center">
      {props.icon}
      <b className="text-body font-semi">{props.title}</b>
      {props.body ? <small className="max-w-[420px] text-xs text-ink2">{props.body}</small> : null}
      {props.children ? <div className="mt-1.5 flex flex-wrap justify-center gap-1.5">{props.children}</div> : null}
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
  /** The rotation's LaunchAgent, in the Claude tab's "Chạy nền": its switch, its line, the old switcher's warning, what Firstmate runs. */
  rotation: { toggle: ReactNode; line: string; warning: ReactNode; firstmate: ReactNode };
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

/** Automatic switching in the Claude section's head on Tổng quan: its switch and why Claude is on its account. */
function AutoHead({ auto, onToggle }: { auto: AutoView; onToggle(): void }) {
  return (
    <span className="ml-auto flex min-w-0 items-center gap-2 font-reg">
      <b className="font-semi whitespace-nowrap text-ink">{auto.title}</b>
      <AdvancedTag />
      <Toggle on={auto.toggle.on} label={auto.toggle.label} disabled={auto.toggle.disabled} onClick={onToggle} />
      <small className="min-w-0 truncate text-xs text-ink2" title={auto.line}>
        {auto.line}
      </small>
    </span>
  );
}

/** What the numbers on Tổng quan are, said once for the whole page. */
function OverviewNote() {
  return (
    <TabNote>
      {t('% đã dùng của từng hạn mức', 'Share used of each quota')}
      {' · '}
      <More about={t('% đã dùng của từng hạn mức', 'Share used of each quota')}>
        <p>
          {t(
            'Mỗi số là phần đã dùng của một hạn mức, theo mẫu số riêng của dịch vụ và cửa sổ đó, nên không cộng được giữa các cột hay các dịch vụ.',
            'Each number is the share used of one quota, out of that service’s and window’s own total, so the columns and services do not add up.',
          )}
        </p>
        <p>
          {t(
            'Dòng dưới thanh là thời gian còn tới lúc hạn mức đặt lại. Dấu — là chưa có số, không phải 0. Thanh vàng từ 70%, đỏ từ 90%.',
            'The line under a bar is the time left until the quota resets. A dash means no number yet, not 0. Bars turn amber from 70 % and red from 90 %.',
          )}
        </p>
        <p>
          {t(
            'Chip “số … trước” cho biết số của tài khoản đó đọc cách đây bao lâu.',
            'A “numbers … old” chip says how long ago that account was read.',
          )}
        </p>
      </More>
    </TabNote>
  );
}

/**
 * Tổng quan: one section per service, each the whole width (Claude,
 * Antigravity, Codex), every account with its windows, the names and the
 * buttons on the same columns in all three. Automatic switching sits in
 * the Claude section's head.
 */
function Overview(props: TabProps & { advanced: boolean; agyAccounts: boolean; release?: BannerView | null; onTab(tab: Tab): void }) {
  const v = props.view;
  return (
    <>
      <Top {...props} title={t('Tổng quan', 'Overview')} />
      <OverviewNote />
      {v.claude.present ? (
        <Card title="Claude" kind="c" head={props.advanced && v.claude.cards.length > 1 ? <AutoHead auto={v.auto} onToggle={props.onToggle} /> : null}>
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

/** A half of the Claude tab's automatic switching card. */
const HALF = 'min-w-0 px-3.5 py-3';

/**
 * What runs the rotation: the LaunchAgent's switch, how it last ran (with
 * Nâng cao, from the switcher), and the rest behind "Chi tiết".
 */
function RunHalf(props: { title: string; tag?: ReactNode; rotation?: ClaudeManage['rotation']; switcher?: WindowView['switcher'] }) {
  const { rotation, switcher } = props;
  return (
    <>
      <div className="flex items-center justify-between gap-2.5">
        <b className="flex items-center gap-1.5 text-body font-semi" id="qs-run-h">
          {props.title}
          {props.tag}
        </b>
        {rotation?.toggle}
      </div>
      {switcher ? (
        <div className="mt-0.5 text-sm text-ink2">
          <Dot tone={switcher.tone} className="mr-1.5" />
          {switcher.line}
        </div>
      ) : null}
      {rotation ? <small className="mt-0.5 block text-xs text-ink2">{rotation.line}</small> : null}
      {rotation?.warning}
      {switcher || rotation?.firstmate ? (
        <div className="mt-2 text-xs text-ink2">
          {rotation?.firstmate ? t('Firstmate dùng cùng bộ luật. ', 'Firstmate uses the same rules. ') : null}
          <More small about={props.title}>
            <p>
              {t(
                'LaunchAgent com.shin315.quota-station.rotation chạy luật xoay vòng mỗi 3 phút, kể cả khi đã đóng app. Khi bộ tự chuyển cũ (com.cswap.prefer-team hoặc com.cswap.auto) còn được nạp, nó không chuyển gì.',
                'The LaunchAgent com.shin315.quota-station.rotation runs the rotation rules every 3 minutes, even with the app closed. While the old auto-switcher (com.cswap.prefer-team or com.cswap.auto) is loaded, it switches nothing.',
              )}
            </p>
            {rotation?.firstmate ? (
              <>
                {rotation.firstmate}
                <p>
                  {t(
                    'quota-axi của Firstmate, khi được trỏ vào app, đọc cùng file luật claude/rotation; giờ nó đọc có thể khác giờ app hiện.',
                    'Firstmate’s quota-axi, once pointed at the app, reads the same rules file, claude/rotation; it may read it at another time than the app shows.',
                  )}
                </p>
              </>
            ) : null}
            {switcher ? <KeyValues rows={switcher.details} /> : null}
          </More>
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
        <EmptyBox icon={<IdMark id="c" className="size-6" />} title={mac(t('Máy này chưa có Claude Code', 'This Mac has no Claude Code'))} body={t('Cài Claude Code rồi mở lại tab này. Quota Station đọc ~/.claude.', 'Install Claude Code, then open this tab again. Quota Station reads ~/.claude.')} />
      </>
    );
  return (
    <>
      <Top {...props} title="Claude" />
      {v.envBanner ? <Banner banner={v.envBanner} onAction={props.onAction} busy={props.busy} /> : null}
      {v.claudeBanner ? <Banner banner={v.claudeBanner} onAction={props.onAction} busy={props.busy} /> : null}
      <Card
        title={t('Tài khoản', 'Accounts')}
        kind="c"
        under={v.claude.count || null}
        end={
          m ? (
            <Button small lead="plus" onClick={m.onAdd}>
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
            icon={<IdMark id="c" className="size-6" />}
            title={t('Chưa có tài khoản Claude nào', 'No Claude accounts yet')}
            body={os(
              t(
                'Quota Station giữ đăng nhập của từng tài khoản trong Keychain của máy này, để bạn chuyển Claude Code giữa các tài khoản.',
                'Quota Station keeps each account’s sign-in in this Mac’s Keychain so you can switch Claude Code between them.',
              ),
              t(
                'Quota Station giữ đăng nhập của từng tài khoản trên máy này, được Windows mã hoá cho tài khoản của bạn, để bạn chuyển Claude Code giữa các tài khoản.',
                'Quota Station keeps each account’s sign-in on this PC, encrypted by Windows for your account, so you can switch Claude Code between them.',
              ),
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
        // One card in two halves that end on one line: switching itself, and what runs it.
        <Panel className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] p-0 @max-[680px]/main:grid-cols-[minmax(0,1fr)]">
          <section className={HALF} aria-labelledby="qs-auto-h">
            <div className="flex items-center justify-between gap-2.5">
              <b className="flex items-center gap-1.5 text-body font-semi" id="qs-auto-h">
                {a.title}
                <AdvancedTag />
              </b>
              <Toggle on={a.toggle.on} label={a.toggle.label} disabled={a.toggle.disabled} onClick={props.onToggle} />
            </div>
            <small className="block text-xs text-ink2">{a.line}</small>
            <CardText className="mt-3 mb-1" id="qs-qp">
              {t('Tạm dừng', 'Pause')}
            </CardText>
            <Segmented aria-labelledby="qs-qp">
              {pauseOptions(props.now).map((o) => (
                <Segment key={o.v} disabled={a.toggle.disabled} onClick={() => props.onPause(o.v)}>
                  {o.t}
                </Segment>
              ))}
            </Segmented>
          </section>
          <section className={cn(HALF, 'border-l-[.5px] border-card-line @max-[680px]/main:border-t-[.5px] @max-[680px]/main:border-l-0')} aria-labelledby="qs-run-h">
            <RunHalf title={t('Chạy nền', 'Background run')} tag={<AdvancedTag />} rotation={m?.rotation} switcher={sw} />
          </section>
        </Panel>
      ) : m?.rotationAlone ? (
        <Panel aria-labelledby="qs-run-h">
          <RunHalf title={t('Tự chuyển bằng Quota Station', 'Auto-switch with Quota Station')} rotation={m.rotation} />
        </Panel>
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
            <b>{mac(t('Máy này chưa có Antigravity', 'This Mac has no Antigravity'))}</b>
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
  /** The Lịch sử and Firstmate tabs (M6). */
  history?: ReactNode;
  firstmate?: ReactNode;
  /** The bell (Thông báo), at the right of every page's title row. */
  bell?: ReactNode;
  /** Under the machine name in the sidebar: whether the other Macs have the rules. */
  syncNote?: string | null;
  /** The accounts the app logs in, merged into the Antigravity table. */
  agyLogins?: AgyLogins;
  /** The Claude accounts Quota Station keeps, in the Claude tab. */
  claude?: ClaudeManage;
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
  // A tab Nâng cao took away while it was open gives way to Tổng quan, and
  // so does `alerts`, which asks for the bell's panel, not a page.
  const tab: Tab = hidden.has(props.tab as NavId) || props.tab === 'alerts' ? 'overview' : props.tab;
  return (
    <div className={cn('win-o material-window relative grid h-full w-full overflow-hidden', props.setup ? 'grid-cols-[minmax(0,1fr)]' : 'grid-cols-[var(--side-w)_minmax(0,1fr)]')}>
      {/* The window's top edge drags it, over the sidebar and the page alike, as a title bar would. Windows has its own title bar. */}
      {onWindows() ? null : <div className="dragbar absolute top-0 right-0 left-0 z-10 h-4" data-tauri-drag-region aria-hidden="true" />}
      {props.setup ? (
        <main className="main material-main @container/main relative flex min-w-0 overflow-y-auto" inert={open}>
          {props.setup}
        </main>
      ) : (
        <div style={{ display: 'contents' }} inert={open}>
          <Sidebar
            tab={tab}
            machine={props.view.machine}
            note={props.advanced ? (props.syncNote ?? null) : null}
            hidden={hidden}
            advanced={props.advanced}
            onTab={props.onTab}
          />
          {/* The page is the container its layouts measure: the window less the sidebar. */}
          <main className="main material-main @container/main relative flex min-w-0 flex-col gap-3 overflow-y-auto pt-4 pr-[22px] pb-[22px] pl-3.5">
            <HeadEnd.Provider value={props.bell ?? null}>
              {tab === 'overview' ? <Overview {...tabProps} advanced={props.advanced} agyAccounts={props.agyAccounts} release={props.release} onTab={props.onTab} /> : null}
              {tab === 'stats' ? props.stats : null}
              {tab === 'claude' ? <ClaudeTab {...tabProps} advanced={props.advanced} /> : null}
              {tab === 'agy' ? <AgyTab {...tabProps} agyAccounts={props.agyAccounts} /> : null}
              {tab === 'codex' ? props.codex : null}
              {tab === 'rules' ? props.rules : null}
              {tab === 'history' ? props.history : null}
              {tab === 'firstmate' ? props.firstmate : null}
              {tab === 'mcp' ? props.mcp : null}
              {tab === 'skills' ? props.skills : null}
              {tab === 'plugins' ? props.plugins : null}
              {tab === 'instr' ? props.instr : null}
              {tab === 'sets' ? props.sets : null}
              {tab === 'backup' ? props.backup : null}
              {tab === 'copy' ? props.copy : null}
              {tab === 'settings' ? props.settings : null}
            </HeadEnd.Provider>
          </main>
        </div>
      )}
      {/* The dialog layer covers the whole window, sidebar included, and the dialog stands in its middle. */}
      <Dialog open={open}>
        {open ? (
          <DialogOverlay className="z-30 grid place-items-center rounded-[inherit]">
            <DialogContent size={props.dialogSize ?? 'wide'}>{props.dialog}</DialogContent>
          </DialogOverlay>
        ) : null}
      </Dialog>
    </div>
  );
}
