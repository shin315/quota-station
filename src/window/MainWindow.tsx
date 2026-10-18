// The main window (M2, read only): the prototype's window with the tabs
// Tổng quan, Claude, Antigravity and Cài đặt. The sidebar keeps the tabs of
// later milestones in place, disabled. No row or button switches an account.

import type { ReactNode } from 'react';
import { Banner, Button, Chip, Dot, Feedback, Meter } from '../components/parts';
import type { Tab } from '../lib/model';
import type { Action, FeedbackView, NoteView, RowView } from '../lib/view';
import type { BigPoolView, WindowView } from '../lib/window';
import { NAV_ICONS } from './icons';

type NavId = keyof typeof NAV_ICONS;

/** The prototype's sidebar; `later` tabs arrive in later milestones. */
const NAVS: (['-', string] | [NavId, string, 'later'?])[] = [
  ['overview', 'Tổng quan'],
  ['-', 'Tài khoản'],
  ['claude', 'Claude'],
  ['agy', 'Antigravity'],
  ['codex', 'Codex', 'later'],
  ['-', 'Tự động'],
  ['rules', 'Luật xoay vòng', 'later'],
  ['history', 'Lịch sử', 'later'],
  ['alerts', 'Cảnh báo', 'later'],
  ['firstmate', 'Firstmate', 'later'],
];

function NavButton(props: { id: NavId; label: string; tab: Tab; later?: boolean; onTab(tab: Tab): void }) {
  const on = props.id === props.tab;
  return (
    <button
      type="button"
      className={'nav' + (on ? ' on' : '')}
      aria-current={on ? 'page' : undefined}
      disabled={props.later}
      title={props.later ? 'Có ở bản sau' : undefined}
      onClick={props.later ? undefined : () => props.onTab(props.id as Tab)}
    >
      {NAV_ICONS[props.id]}
      {props.label}
    </button>
  );
}

function Sidebar({ tab, machine, onTab }: { tab: Tab; machine: string; onTab(tab: Tab): void }) {
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
          <NavButton key={n[0]} id={n[0]} label={n[1]} later={n[2] === 'later'} tab={tab} onTab={onTab} />
        ),
      )}
      <div className="grow" data-tauri-drag-region />
      <NavButton id="settings" label="Cài đặt" tab={tab} onTab={onTab} />
      <div className="machine">{machine}</div>
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

function BigPool({ pool }: { pool: BigPoolView }) {
  return (
    <div className="card" title={pool.hover || undefined}>
      <h4>
        <span className={'idm ' + pool.kind} />
        {pool.title}
      </h4>
      <div className="big">
        {pool.value}
        {pool.unit ? <i>{pool.unit}</i> : null}
      </div>
      <span className="meter" aria-hidden="true">
        <i className={pool.fill} style={{ width: pool.percent + '%' }} />
      </span>
      <small className="pline">{pool.line}</small>
    </div>
  );
}

/** One account in a window table: the prototype's `trRow`, without the switch button. */
function TableRow({ row, freshness }: { row: RowView; freshness: boolean }) {
  const chips = freshness ? row.tags : row.chips;
  return (
    <div className={'tr' + (row.active ? ' on' : '')} role="listitem" aria-label={row.label} title={row.title || undefined}>
      <span className={'av ' + row.kind} aria-hidden="true">
        {row.short}
      </span>
      <div className="nm" aria-hidden="true">
        <b>{row.name}</b>
        {chips.map((c) => (
          <Chip key={c.t} chip={c} />
        ))}
      </div>
      {row.lines.map((l) => (
        <div className="cell" key={l.label} title={l.title || undefined} aria-hidden="true">
          <small>
            <span>{l.label}</span>
            <b>{l.text}</b>
          </small>
          <Meter percent={l.percent} tone={row.dim ? 'muted' : l.tone} />
          <em>{l.long}</em>
        </div>
      ))}
      {freshness ? (
        <span className="fresh" aria-hidden="true">
          {row.freshness ? <Chip chip={row.freshness} /> : '—'}
        </span>
      ) : null}
    </div>
  );
}

function AccountTable(props: { label: string; rows: RowView[]; header?: boolean; freshness?: boolean }) {
  if (!props.rows.length) return null;
  const fresh = !!props.freshness;
  return (
    <div className={'tbl ro' + (fresh ? ' withfresh' : '')} role="list" aria-label={props.label}>
      {props.header ? (
        <div className="tr h" aria-hidden="true">
          <span />
          <span>Tài khoản</span>
          <span>Hạn mức 5 giờ</span>
          <span>Hạn mức tuần / Fable</span>
          {fresh ? <span>Cập nhật</span> : null}
        </div>
      ) : null}
      {props.rows.map((r) => (
        <TableRow key={r.key} row={r} freshness={fresh} />
      ))}
    </div>
  );
}

function Note({ note }: { note: NoteView | null }) {
  return note ? <p className={('note ' + note.tone).trim()}>{note.text}</p> : null;
}

function KeyValues({ rows }: { rows: WindowView['agy']['cockpit'] }) {
  return (
    <div className="kvt">
      {rows.map((r) => [
        <span key={r.key + ':k'}>{r.key}</span>,
        <span key={r.key + ':v'}>{r.chip ? <Chip chip={r.chip} /> : r.value}</span>,
      ])}
    </div>
  );
}

// ===== Tabs =====

interface TabProps {
  view: WindowView;
  busy: Action | null;
  feedback: FeedbackView | null;
  onAction(act: Action): void;
}

function Top(props: TabProps & { title: string; banner?: boolean }) {
  return (
    <>
      <TitleBar title={props.title} view={props.view} busy={props.busy === 'refresh'} onRefresh={() => props.onAction('refresh')} />
      {props.feedback ? <Feedback fb={props.feedback} /> : null}
      {props.banner !== false && props.view.banner ? (
        <Banner banner={props.view.banner} onAction={props.onAction} busy={props.busy} />
      ) : null}
    </>
  );
}

function Overview(props: TabProps) {
  const v = props.view;
  return (
    <>
      <Top {...props} title="Tổng quan" />
      <div className="grid">
        <div className="stack">
          <div className="grid2">
            <BigPool pool={v.pools.claude} />
            <BigPool pool={v.pools.agy} />
          </div>
          <Card title={v.claude.title} kind="c">
            <AccountTable label="Claude Code" rows={v.claude.rows} header />
            <Note note={v.claude.note} />
          </Card>
          {v.agy.present ? (
            <Card title={v.agy.title} kind="g">
              <AccountTable label="Antigravity" rows={v.agy.rows} />
              <Note note={v.agy.note} />
            </Card>
          ) : null}
        </div>
        <div className="stack">
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
  return (
    <>
      <Top {...props} title="Claude" />
      <Card title={v.claude.tabTitle} kind="c">
        <AccountTable label="Claude Code" rows={v.claude.rows} header freshness />
        <Note note={v.claude.note} />
      </Card>
    </>
  );
}

function AgyTab(props: TabProps) {
  const a = props.view.agy;
  return (
    <>
      <Top {...props} title="Antigravity" banner={false} />
      {a.banner ? <Banner banner={a.banner} onAction={props.onAction} busy={props.busy} /> : null}
      {a.present ? (
        <>
          <Card title="Tài khoản" kind="g">
            <AccountTable label="Antigravity" rows={a.rows} />
            {a.others ? <p className="note">{a.others}</p> : null}
            <Note note={a.tabNote} />
          </Card>
          <div className="grid2">
            <Card title="Cockpit">
              <KeyValues rows={a.cockpit} />
            </Card>
          </div>
        </>
      ) : (
        <div className="card">
          <div className="empty">
            <b>Máy này chưa có Antigravity</b>quota-axi không thấy Antigravity trên máy này.
          </div>
          <Note note={a.tabNote} />
        </div>
      )}
    </>
  );
}

export interface MainWindowProps {
  tab: Tab;
  view: WindowView;
  busy: Action | null;
  feedback: FeedbackView | null;
  onTab(tab: Tab): void;
  onAction(act: Action): void;
  /** The Cài đặt tab, drawn by the settings component. */
  settings: ReactNode;
}

export function MainWindow(props: MainWindowProps) {
  const tabProps: TabProps = { view: props.view, busy: props.busy, feedback: props.feedback, onAction: props.onAction };
  return (
    <div className="win-o">
      <Sidebar tab={props.tab} machine={props.view.machine} onTab={props.onTab} />
      <main className="main">
        {props.tab === 'overview' ? <Overview {...tabProps} /> : null}
        {props.tab === 'claude' ? <ClaudeTab {...tabProps} /> : null}
        {props.tab === 'agy' ? <AgyTab {...tabProps} /> : null}
        {props.tab === 'settings' ? props.settings : null}
      </main>
    </div>
  );
}
