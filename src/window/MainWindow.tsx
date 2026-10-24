// The main window: the prototype's window with the tabs Tổng quan, Claude,
// Antigravity, Luật xoay vòng and Cài đặt. Claude accounts switch from the
// tables (F04), Tổng quan and Claude show automatic switching and why Claude
// is on its account (F07), Luật xoay vòng edits the rotation for every Mac
// (F06), and Antigravity also shows the accounts the app logs in (M4a) and
// switches from its tables once quota-axi lists them (M4c). Lịch sử, Cảnh
// báo and Firstmate come from M6, and Cảnh báo counts the alerts nobody
// answered yet. Codex (M8) has its own tab and a compact card on Tổng quan.
// The Agent group starts with MCP (M9): every agent's MCP servers in one table,
// then Skills, Plugin and File hướng dẫn (M11).
// The Sao lưu group has the backup packs and the restore (M10), and the copy
// to or from another Mac over SSH (M13).
// The Claude tab has a red banner while a variable overrides Claude Code's login (M14).
// Bộ cấu hình (M15) closes the Agent group: named sets applied for the whole Mac.
// Thống kê (M16) follows Tổng quan: the tokens counted from this Mac's session logs.

import { Fragment, type ReactNode } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Dialog, DialogContent, DialogOverlay } from '@/components/ui/dialog';
import { Segment, Segmented } from '@/components/ui/segmented';
import { cn } from '@/lib/utils';
import { AddAccount, LoginActions, LoginNotes, LogoutConfirm, loginChips, type AgyLogins } from '../antigravity/AgyAccounts';
import { AccountName, IdMark, InUse, NowCard, WindowCell } from '../components/accounts';
import { Empty, GRID2, STACK, TABLE_ROW, TABLE_ROW_ON, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, CODE, Dot, Feedback, KVT, Toggle } from '../components/parts';
import type { Tab } from '../lib/model';
import { pauseOptions, type AutoView, type PauseChoice } from '../lib/switching';
import { bannerUnder, type Action, type CardView, type ChipView, type FeedbackView, type NoteView } from '../lib/view';
import type { KeyValue, WindowView } from '../lib/window';
import { NAV_ICONS } from './icons';

type NavId = keyof typeof NAV_ICONS;

/** The prototype's sidebar. */
const NAVS: (['-', string] | [NavId, string])[] = [
  ['overview', 'Tổng quan'],
  ['stats', 'Thống kê'],
  ['-', 'Tài khoản'],
  ['claude', 'Claude'],
  ['agy', 'Antigravity'],
  ['codex', 'Codex'],
  ['-', 'Tự động'],
  ['rules', 'Luật xoay vòng'],
  ['history', 'Lịch sử'],
  ['alerts', 'Cảnh báo'],
  ['firstmate', 'Firstmate'],
  ['-', 'Agent'],
  ['mcp', 'MCP'],
  ['skills', 'Skills'],
  ['plugins', 'Plugin'],
  ['instr', 'File hướng dẫn'],
  ['sets', 'Bộ cấu hình'],
  ['-', 'Sao lưu'],
  ['backup', 'Sao lưu'],
  ['copy', 'Chép sang Mac khác'],
];

function NavButton(props: { id: NavId; label: string; tab: Tab; badge?: number; onTab(tab: Tab): void }) {
  const on = props.id === props.tab;
  return (
    <button
      type="button"
      className={cn(
        'flex w-full cursor-pointer items-center gap-[9px] rounded-lg border-0 px-2 py-[5px] text-left text-base [&>svg]:flex-[0_0_auto] [&>svg]:text-accent-ink',
        on ? 'bg-fill2 font-semi' : 'bg-transparent hover:bg-fill',
      )}
      aria-current={on ? 'page' : undefined}
      onClick={() => props.onTab(props.id as Tab)}
    >
      {NAV_ICONS[props.id]}
      {props.label}
      {props.badge ? (
        <>
          <span className="ml-auto rounded-full bg-bad-fill px-1.5 text-cap leading-4 font-heavy text-white" aria-hidden="true">
            {props.badge}
          </span>
          <span className="sr-only">, {props.badge} cảnh báo mới</span>
        </>
      ) : null}
    </button>
  );
}

function Sidebar(props: { tab: Tab; machine: string; note: string | null; alerts: number; onTab(tab: Tab): void }) {
  const { tab, machine, note, onTab } = props;
  return (
    <nav className="m-0 flex min-h-0 flex-col gap-px rounded-none bg-transparent px-2.5 pt-5 pb-2.5" aria-label="Mục" data-tauri-drag-region>
      {/* macOS draws the window's traffic lights here; the browser preview draws the prototype's. */}
      <div className="flex min-h-3.5 items-center gap-[7px] px-3 pb-3.5" data-tauri-drag-region aria-hidden="true">
        <i className={LIGHT + ' bg-[#ff5f57]'} />
        <i className={LIGHT + ' bg-[#febc2e]'} />
        <i className={LIGHT + ' bg-[#28c840]'} />
      </div>
      {NAVS.map((n) =>
        n[0] === '-' ? (
          <div className="px-2 pt-2 pb-0.5 text-cap font-semi tracking-[.02em] text-ink2" key={n[1]}>
            {n[1]}
          </div>
        ) : (
          <NavButton
            key={n[0]}
            id={n[0]}
            label={n[1]}
            badge={n[0] === 'alerts' ? props.alerts : undefined}
            tab={tab}
            onTab={onTab}
          />
        ),
      )}
      <div className="flex-1" data-tauri-drag-region />
      <NavButton id="settings" label="Cài đặt" tab={tab} onTab={onTab} />
      <div className="px-2 py-1.5 text-s text-ink2">
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
        Làm mới
      </Button>
    </TabHead>
  );
}

/** The traffic lights the browser preview draws; macOS draws its own in the app. */
const LIGHT = 'size-3 rounded-full [body:not(.preview)_&]:invisible';

function Card(props: { title: ReactNode; kind?: 'c' | 'g'; children: ReactNode }) {
  return (
    <Panel>
      <CardTitle>
        {props.kind ? <IdMark id={props.kind} /> : null}
        {props.title}
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
    <Button primary disabled={!card.switchable} label={'Chuyển sang ' + card.name} title={card.off || undefined} onClick={() => onSwitch(card.slot)}>
      Chuyển
    </Button>
  );
}

/** A row of a window table (see TABLE_ROW). */
const ROW = TABLE_ROW;
/** The table's head rows: the windows' names. */
const HEAD = ROW + ' h pt-0 pb-1 text-xs text-ink3';

/** One account in a window table: its name, a fixed column per window, and its buttons. */
function TableRow(props: { card: CardView; extra?: ReactNode; chips?: ChipView[]; tight?: boolean; onSwitch(slot: number): void }) {
  const c = props.card;
  return (
    <div
      className={cn(ROW, c.active && TABLE_ROW_ON)}
      role="listitem"
      aria-label={c.label}
      title={c.title || undefined}
    >
      <AccountName name={c.name} sub={c.sub} full={c.full} chips={[...c.chips, ...(props.chips ?? [])]} />
      {c.cells.map((cell) => (
        <WindowCell key={cell.label} cell={cell} dim={c.dim} tight={props.tight} />
      ))}
      <span className="flex items-center justify-end gap-1.5">
        <SwitchCell card={c} onSwitch={props.onSwitch} />
        {props.extra}
      </span>
    </div>
  );
}

/** Claude: 5 giờ · Tuần · Fable tuần, the same columns for every account. `tight`: on Tổng quan. */
function ClaudeTable(props: { cards: CardView[]; tight?: boolean; onSwitch(slot: number): void }) {
  if (!props.cards.length) return null;
  return (
    <div className="grid [--wt-cols:minmax(96px,1.25fr)_repeat(3,minmax(0,1fr))_76px]" role="list" aria-label="Claude Code">
      <div className={HEAD} aria-hidden="true">
        <span>đã dùng</span>
        <span>5 giờ</span>
        <span>Tuần</span>
        <span>Fable tuần</span>
        <span />
      </div>
      {props.cards.map((c) => (
        <TableRow key={c.key} card={c} tight={props.tight} onSwitch={props.onSwitch} />
      ))}
    </div>
  );
}

/**
 * Antigravity: Gemini and Claude/GPT, five hours and a week each. An
 * account only Cockpit Tools reads has one number per group instead.
 */
function AgyTable(props: { cards: CardView[]; logins?: AgyLogins; tight?: boolean; onSwitch(slot: number): void }) {
  if (!props.cards.length) return null;
  const four = props.cards.every((c) => c.cells.length === 4);
  const logins = props.logins;
  const cols = props.tight
    ? four
      ? '[--wt-cols:minmax(96px,1fr)_repeat(4,minmax(0,.82fr))_70px]'
      : '[--wt-cols:minmax(96px,1fr)_repeat(2,minmax(0,1fr))_70px]'
    : four
      ? logins
        ? '[--wt-cols:minmax(0,1.1fr)_repeat(4,minmax(0,.8fr))_180px]'
        : '[--wt-cols:minmax(104px,1.25fr)_repeat(4,minmax(0,.8fr))_76px]'
      : logins
        ? '[--wt-cols:minmax(0,1.2fr)_repeat(2,minmax(0,1fr))_180px]'
        : '[--wt-cols:minmax(104px,1.25fr)_repeat(2,minmax(0,1fr))_76px]';
  return (
    <div className={cn('grid', cols)} role="list" aria-label="Antigravity">
      {four ? (
        <>
          <div className={HEAD} aria-hidden="true">
            <span />
            <span className={GROUP}>Gemini</span>
            <span className={GROUP}>Claude/GPT</span>
            <span />
          </div>
          <div className={cn(HEAD, 'border-t-0')} aria-hidden="true">
            <span>đã dùng</span>
            <span>5 giờ</span>
            <span>Tuần</span>
            <span>5 giờ</span>
            <span>Tuần</span>
            <span />
          </div>
        </>
      ) : (
        <div className={HEAD} aria-hidden="true">
          <span>đã dùng</span>
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

/** Automatic switching on one line: what it does now, and its switch. */
function AutoCard({ auto, onToggle, children }: { auto: AutoView; onToggle(): void; children?: ReactNode }) {
  return (
    <Panel>
      <div className="flex items-center justify-between gap-2.5">
        <div className="min-w-0">
          <b className="block text-body font-semi">{auto.title}</b>
          <small className="block text-xs text-ink2">{auto.line}</small>
        </div>
        <Toggle on={auto.toggle.on} label={auto.toggle.label} disabled={auto.toggle.disabled} onClick={onToggle} />
      </div>
      {children}
    </Panel>
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
  claudeAccounts?: ReactNode;
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

function Overview(props: TabProps) {
  const v = props.view;
  return (
    <>
      <Top {...props} title="Tổng quan" />
      <div className="grid grid-cols-[minmax(0,2.6fr)_minmax(0,1fr)] items-start gap-3 [@media(max-width:1000px)]:grid-cols-[minmax(0,1fr)]">
        <div className={STACK}>
          <NowCard now={v.now} />
          <Card title="Claude" kind="c">
            <ClaudeTable cards={v.claude.cards} tight onSwitch={props.onSwitch} />
            <Note note={v.claude.note} />
          </Card>
          {v.agy.present ? (
            <Card title="Antigravity" kind="g">
              <AgyTable cards={v.agy.cards} tight onSwitch={props.onSwitchAgy} />
              <Note note={v.agy.note} />
            </Card>
          ) : null}
          {props.codexCard}
        </div>
        <div className={STACK}>
          <AutoCard auto={v.auto} onToggle={props.onToggle} />
          <Card title="Sắp tới">
            {v.upcoming.length ? (
              <div className="grid gap-2 text-sm">
                {v.upcoming.map((u) => (
                  <div key={u.key} className="grid grid-cols-[54px_minmax(0,1fr)] gap-2">
                    <time className="text-ink2 tabular-nums">{u.at}</time>
                    <span>
                      {u.tone ? <Dot tone={u.tone} className="mr-1.5" /> : null}
                      {u.what}
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <CardText>Không có mốc đặt lại nào trong 24 giờ tới.</CardText>
            )}
          </Card>
        </div>
      </div>
    </>
  );
}

function ClaudeTab(props: TabProps) {
  const v = props.view;
  const a = v.auto;
  const sw = v.switcher;
  return (
    <>
      <Top {...props} title="Claude" />
      {v.envBanner ? <Banner banner={v.envBanner} onAction={props.onAction} busy={props.busy} /> : null}
      {v.claudeBanner ? <Banner banner={v.claudeBanner} onAction={props.onAction} busy={props.busy} /> : null}
      <Card title={<>Tài khoản{v.claude.count ? <span className="ml-auto font-reg">{v.claude.count}</span> : null}</>} kind="c">
        <ClaudeTable cards={v.claude.cards} onSwitch={props.onSwitch} />
        <Note note={v.claude.note} />
      </Card>
      {props.claudeAccounts}
      <div className={GRID2}>
        <AutoCard auto={a} onToggle={props.onToggle}>
          <CardText style={{ margin: '12px 0 4px' }} id="qs-qp">
            Tạm dừng
          </CardText>
          <Segmented aria-labelledby="qs-qp">
            {pauseOptions(props.now).map((o) => (
              <Segment key={o.v} disabled={a.toggle.disabled} onClick={() => props.onPause(o.v)}>
                {o.t}
              </Segment>
            ))}
          </Segmented>
        </AutoCard>
        <Card title="Switcher">
          <div className="text-sm text-ink2">
            <Dot tone={sw.tone} className="mr-1.5" />
            {sw.line}
          </div>
          <details className="mt-2 text-sm text-ink2">
            <summary className="cursor-pointer list-none text-accent-ink [&::-webkit-details-marker]:hidden">Chi tiết</summary>
            <div className="mt-2">
              <KeyValues rows={sw.details} />
            </div>
          </details>
        </Card>
      </div>
    </>
  );
}

function AgyTab(props: TabProps) {
  const a = props.view.agy;
  const logins = props.agyLogins;
  return (
    <>
      <Top {...props} title="Antigravity" banner={false} />
      {a.banner ? <Banner banner={a.banner} onAction={props.onAction} busy={props.busy} /> : null}
      <Card title={<>Tài khoản{logins ? <span className="ml-auto font-reg">{<AddAccount logins={logins} />}</span> : null}</>} kind="g">
        {a.present ? <AgyTable cards={a.cards} logins={a.own ? logins : undefined} onSwitch={props.onSwitchAgy} /> : null}
        {!a.present ? (
          <Empty>
            <b>Máy này chưa có Antigravity</b>
          </Empty>
        ) : null}
        <Note note={a.tabNote} />
        {logins ? <LoginNotes logins={logins} listed={a.own ? a.cards.map((c) => c.slot) : []} /> : null}
      </Card>
      {a.cockpit ? (
        <div className={GRID2}>
          <Card title="Cockpit Tools">
            <KeyValues rows={a.cockpit} />
          </Card>
        </div>
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
  claudeAccounts?: ReactNode;
  /** The open dialog's body; null when none is open. */
  dialog?: ReactNode;
  /** Its width: the MCP preview and form need more room than a confirmation. */
  dialogSize?: 'wide' | 'lg' | 'xl';
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
    agyLogins: props.agyLogins,
    codexCard: props.codexCard,
    claudeAccounts: props.claudeAccounts,
  };
  const open = !!props.dialog;
  return (
    <div className="win-o material-window relative grid h-full w-full grid-cols-[var(--side-w)_minmax(0,1fr)] overflow-hidden">
      {/* The window's top edge drags it, over the sidebar and the page alike, as a title bar would. */}
      <div className="dragbar absolute top-0 right-0 left-0 z-10 h-4" data-tauri-drag-region aria-hidden="true" />
      <div style={{ display: 'contents' }} inert={open}>
        <Sidebar
          tab={props.tab}
          machine={props.view.machine}
          note={props.syncNote ?? null}
          alerts={props.unanswered ?? 0}
          onTab={props.onTab}
        />
        <main className="main relative flex min-w-0 flex-col gap-3 overflow-y-auto pt-4 pr-[22px] pb-[22px] pl-3.5">
          {props.tab === 'overview' ? <Overview {...tabProps} /> : null}
          {props.tab === 'stats' ? props.stats : null}
          {props.tab === 'claude' ? <ClaudeTab {...tabProps} /> : null}
          {props.tab === 'agy' ? <AgyTab {...tabProps} /> : null}
          {props.tab === 'codex' ? props.codex : null}
          {props.tab === 'rules' ? props.rules : null}
          {props.tab === 'history' ? props.history : null}
          {props.tab === 'alerts' ? props.alerts : null}
          {props.tab === 'firstmate' ? props.firstmate : null}
          {props.tab === 'mcp' ? props.mcp : null}
          {props.tab === 'skills' ? props.skills : null}
          {props.tab === 'plugins' ? props.plugins : null}
          {props.tab === 'instr' ? props.instr : null}
          {props.tab === 'sets' ? props.sets : null}
          {props.tab === 'backup' ? props.backup : null}
          {props.tab === 'copy' ? props.copy : null}
          {props.tab === 'settings' ? props.settings : null}
        </main>
      </div>
      {/* The dialog layer covers the whole window, sidebar included. */}
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
