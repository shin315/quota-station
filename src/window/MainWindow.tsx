// The main window: the prototype's window with the tabs Tổng quan, Claude,
// Antigravity, Luật xoay vòng and Cài đặt. Claude accounts switch from the
// tables (F04), Tổng quan and Claude show automatic switching and why Claude
// is on its account (F07), Luật xoay vòng edits the rotation for every Mac
// (F06), and Antigravity also shows the accounts the app logs in (M4a) and
// switches from its tables once quota-axi lists them (M4c). Lịch sử, Cảnh
// báo and Firstmate come from M6, and Cảnh báo counts the alerts nobody
// answered yet. Codex (M8) has its own tab and a compact card on Tổng quan.

import { Fragment, type ReactNode } from 'react';
import { AddAccount, LoginActions, LoginNotes, LogoutConfirm, loginChips, type AgyLogins } from '../antigravity/AgyAccounts';
import { AccountName, InUse, NowCard, WindowCell } from '../components/accounts';
import { Banner, Button, Chip, Dot, Feedback, Toggle } from '../components/parts';
import { SHEET_TITLE_ID, useDialog } from '../components/Sheets';
import type { Tab } from '../lib/model';
import { pauseOptions, type AutoView, type PauseChoice } from '../lib/switching';
import { bannerUnder, type Action, type CardView, type ChipView, type FeedbackView, type NoteView } from '../lib/view';
import type { KeyValue, WindowView } from '../lib/window';
import { NAV_ICONS } from './icons';

type NavId = keyof typeof NAV_ICONS;

/** The prototype's sidebar. */
const NAVS: (['-', string] | [NavId, string])[] = [
  ['overview', 'Tổng quan'],
  ['-', 'Tài khoản'],
  ['claude', 'Claude'],
  ['agy', 'Antigravity'],
  ['codex', 'Codex'],
  ['-', 'Tự động'],
  ['rules', 'Luật xoay vòng'],
  ['history', 'Lịch sử'],
  ['alerts', 'Cảnh báo'],
  ['firstmate', 'Firstmate'],
];

function NavButton(props: { id: NavId; label: string; tab: Tab; badge?: number; onTab(tab: Tab): void }) {
  const on = props.id === props.tab;
  return (
    <button
      type="button"
      className={'nav' + (on ? ' on' : '')}
      aria-current={on ? 'page' : undefined}
      onClick={() => props.onTab(props.id as Tab)}
    >
      {NAV_ICONS[props.id]}
      {props.label}
      {props.badge ? (
        <>
          <span className="badge" aria-hidden="true">
            {props.badge}
          </span>
          <span className="vh">, {props.badge} cảnh báo mới</span>
        </>
      ) : null}
    </button>
  );
}

function Sidebar(props: { tab: Tab; machine: string; note: string | null; alerts: number; onTab(tab: Tab): void }) {
  const { tab, machine, note, onTab } = props;
  return (
    <nav className="side" aria-label="Mục">
      {/* macOS draws the window's traffic lights here; the browser preview draws the prototype's. */}
      <div className="lights" data-tauri-drag-region aria-hidden="true">
        <i />
        <i />
        <i />
      </div>
      {NAVS.map((n) =>
        n[0] === '-' ? (
          <div className="grp" key={n[1]}>
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
      <div className="grow" data-tauri-drag-region />
      <NavButton id="settings" label="Cài đặt" tab={tab} onTab={onTab} />
      <div className="machine">
        <span>{machine}</span>
        {note ? ' · ' + note : null}
      </div>
    </nav>
  );
}

function TitleBar(props: { title: string; view: WindowView; busy: boolean; onRefresh(): void }) {
  return (
    <div className="tb" data-tauri-drag-region>
      <h2>{props.title}</h2>
      <small>
        <Dot tone={props.view.fresh.tone} />
        {props.view.machine} · {props.view.fresh.text}
      </small>
      <Button disabled={props.busy} onClick={props.onRefresh}>
        Làm mới
      </Button>
    </div>
  );
}

function Card(props: { title: ReactNode; kind?: 'c' | 'g'; children: ReactNode }) {
  return (
    <div className="card">
      <h4>
        {props.kind ? <span className={'idm ' + props.kind} /> : null}
        {props.title}
      </h4>
      {props.children}
    </div>
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

/** One account in a window table: its name, a fixed column per window, and its buttons. */
function TableRow(props: { card: CardView; long: boolean; extra?: ReactNode; chips?: ChipView[]; onSwitch(slot: number): void }) {
  const c = props.card;
  return (
    <div className={'wt-r' + (c.active ? ' on' : '')} role="listitem" aria-label={c.label} title={c.title || undefined}>
      <AccountName name={c.name} sub={c.sub} full={c.full} chips={[...c.chips, ...(props.chips ?? [])]} />
      {c.cells.map((cell) => (
        <WindowCell key={cell.label} cell={cell} long={props.long} dim={c.dim} />
      ))}
      <span className="act">
        <SwitchCell card={c} onSwitch={props.onSwitch} />
        {props.extra}
      </span>
    </div>
  );
}

/** Claude: 5 giờ · Tuần · Fable tuần, the same columns for every account. */
function ClaudeTable(props: { cards: CardView[]; long?: boolean; onSwitch(slot: number): void }) {
  if (!props.cards.length) return null;
  return (
    <div className="wt c" role="list" aria-label="Claude Code">
      <div className="wt-r h" aria-hidden="true">
        <span>đã dùng</span>
        <span>5 giờ</span>
        <span>Tuần</span>
        <span>Fable tuần</span>
        <span />
      </div>
      {props.cards.map((c) => (
        <TableRow key={c.key} card={c} long={!!props.long} onSwitch={props.onSwitch} />
      ))}
    </div>
  );
}

/**
 * Antigravity: Gemini and Claude/GPT, five hours and a week each. An
 * account only Cockpit Tools reads has one number per group instead.
 */
function AgyTable(props: { cards: CardView[]; long?: boolean; logins?: AgyLogins; onSwitch(slot: number): void }) {
  if (!props.cards.length) return null;
  const four = props.cards.every((c) => c.cells.length === 4);
  const logins = props.logins;
  return (
    <div className={'wt g' + (four ? '' : ' two') + (logins ? ' acts' : '')} role="list" aria-label="Antigravity">
      {four ? (
        <>
          <div className="wt-r h" aria-hidden="true">
            <span />
            <span className="grp">Gemini</span>
            <span className="grp">Claude/GPT</span>
            <span />
          </div>
          <div className="wt-r h h2" aria-hidden="true">
            <span>đã dùng</span>
            <span>5 giờ</span>
            <span>Tuần</span>
            <span>5 giờ</span>
            <span>Tuần</span>
            <span />
          </div>
        </>
      ) : (
        <div className="wt-r h" aria-hidden="true">
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
              long={!!props.long}
              chips={loginChips(row)}
              extra={logins && row ? <LoginActions logins={logins} row={row} /> : null}
              onSwitch={props.onSwitch}
            />
            {logins && row && logins.confirm === row.id ? <LogoutConfirm logins={logins} row={row} /> : null}
          </Fragment>
        );
      })}
    </div>
  );
}

function Note({ note }: { note: NoteView | null }) {
  return note ? <p className={('note ' + note.tone).trim()}>{note.text}</p> : null;
}

function KeyValues({ rows }: { rows: KeyValue[] }) {
  return (
    <div className="kvt">
      {rows.map((r) => [
        <span key={r.key + ':k'}>{r.key}</span>,
        <span key={r.key + ':v'} className={r.mono ? 'mono' : undefined}>
          {r.chip ? <Chip chip={r.chip} /> : r.value}
        </span>,
      ])}
    </div>
  );
}

/** Automatic switching on one line: what it does now, and its switch. */
function AutoCard({ auto, onToggle, children }: { auto: AutoView; onToggle(): void; children?: ReactNode }) {
  return (
    <div className="card">
      <div className="sw-row">
        <div className="t">
          <b>{auto.title}</b>
          <small>{auto.line}</small>
        </div>
        <Toggle on={auto.toggle.on} label={auto.toggle.label} disabled={auto.toggle.disabled} onClick={onToggle} />
      </div>
      {children}
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
      <div className="grid ov">
        <div className="stack">
          <NowCard now={v.now} />
          <Card title="Claude" kind="c">
            <ClaudeTable cards={v.claude.cards} onSwitch={props.onSwitch} />
            <Note note={v.claude.note} />
          </Card>
          {v.agy.present ? (
            <Card title="Antigravity" kind="g">
              <AgyTable cards={v.agy.cards} onSwitch={props.onSwitchAgy} />
              <Note note={v.agy.note} />
            </Card>
          ) : null}
          {props.codexCard}
        </div>
        <div className="stack">
          <AutoCard auto={v.auto} onToggle={props.onToggle} />
          <Card title="Sắp tới">
            {v.upcoming.length ? (
              <div className="list">
                {v.upcoming.map((u) => (
                  <div key={u.key}>
                    <time>{u.at}</time>
                    <span>
                      {u.tone ? <span className={'dot ' + u.tone} aria-hidden="true" /> : null}
                      {u.what}
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <p>Không có mốc đặt lại nào trong 24 giờ tới.</p>
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
      {v.claudeBanner ? <Banner banner={v.claudeBanner} onAction={props.onAction} busy={props.busy} /> : null}
      <Card title={<>Tài khoản{v.claude.count ? <span className="r">{v.claude.count}</span> : null}</>} kind="c">
        <ClaudeTable cards={v.claude.cards} long onSwitch={props.onSwitch} />
        <Note note={v.claude.note} />
      </Card>
      <div className="grid2">
        <AutoCard auto={a} onToggle={props.onToggle}>
          <p style={{ margin: '12px 0 4px' }} id="qs-qp">
            Tạm dừng
          </p>
          <div className="seg" role="group" aria-labelledby="qs-qp">
            {pauseOptions(props.now).map((o) => (
              <button key={o.v} type="button" disabled={a.toggle.disabled} onClick={() => props.onPause(o.v)}>
                {o.t}
              </button>
            ))}
          </div>
        </AutoCard>
        <Card title="Switcher">
          <div className="line">
            <Dot tone={sw.tone} />
            {sw.line}
          </div>
          <details className="more">
            <summary>Chi tiết</summary>
            <KeyValues rows={sw.details} />
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
      <Card title={<>Tài khoản{logins ? <span className="r">{<AddAccount logins={logins} />}</span> : null}</>} kind="g">
        {a.present ? <AgyTable cards={a.cards} long logins={a.own ? logins : undefined} onSwitch={props.onSwitchAgy} /> : null}
        {!a.present ? (
          <div className="empty">
            <b>Máy này chưa có Antigravity</b>
          </div>
        ) : null}
        <Note note={a.tabNote} />
        {logins ? <LoginNotes logins={logins} listed={a.own ? a.cards.map((c) => c.slot) : []} /> : null}
      </Card>
      {a.cockpit ? (
        <div className="grid2">
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
  /** The open dialog's body; null when none is open. */
  dialog?: ReactNode;
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
  };
  const open = !!props.dialog;
  const dialog = useDialog(open);
  return (
    <div className="win-o">
      <div style={{ display: 'contents' }} inert={open}>
        <Sidebar
          tab={props.tab}
          machine={props.view.machine}
          note={props.syncNote ?? null}
          alerts={props.unanswered ?? 0}
          onTab={props.onTab}
        />
        <main className="main">
          {props.tab === 'overview' ? <Overview {...tabProps} /> : null}
          {props.tab === 'claude' ? <ClaudeTab {...tabProps} /> : null}
          {props.tab === 'agy' ? <AgyTab {...tabProps} /> : null}
          {props.tab === 'codex' ? props.codex : null}
          {props.tab === 'rules' ? props.rules : null}
          {props.tab === 'history' ? props.history : null}
          {props.tab === 'alerts' ? props.alerts : null}
          {props.tab === 'firstmate' ? props.firstmate : null}
          {props.tab === 'settings' ? props.settings : null}
        </main>
      </div>
      {open ? (
        <div className="dlg-wrap">
          <div
            className="dlg wide"
            role="dialog"
            aria-modal="true"
            aria-labelledby={SHEET_TITLE_ID}
            tabIndex={-1}
            ref={dialog.ref}
            onKeyDown={dialog.onKeyDown}
          >
            {props.dialog}
          </div>
        </div>
      ) : null}
    </div>
  );
}
