// The popover (F01, F03, F04, F07): every account, a Claude row opens the
// switch sheet, and the footer holds the automatic switching toggle with why
// Claude is on its account. Its header opens the main window and the settings
// tab. Antigravity rows switch in a later milestone.

import { useLayoutEffect, useRef, type ReactNode } from 'react';
import { Banner, Chip, Dot, Feedback, GearIcon, Meter, Toggle } from '../components/parts';
import { SHEET_TITLE_ID, useDialog } from '../components/Sheets';
import type { AutoView } from '../lib/switching';
import { bannerUnder, type Action, type FeedbackView, type PoolView, type PopoverView, type RowView, type SectionView } from '../lib/view';
import { WindowIcon } from '../window/icons';

function PoolTile({ pool }: { pool: PoolView }) {
  const cls = pool.tone === 'bad' ? 'bad' : pool.tone === 'muted' ? 'muted' : pool.kind;
  return (
    <div className="pool" title={pool.title || undefined}>
      <small>
        <span className={'idm ' + pool.kind} />
        {pool.label}
      </small>
      <b className="n">
        {pool.value}
        {pool.unit ? <i>{pool.unit}</i> : null}
      </b>
      <span className="meter" aria-hidden="true">
        <i className={cls} style={{ width: pool.percent + '%' }} />
      </span>
      <span className={'sub' + (pool.tone === 'bad' ? ' bad' : '')}>{pool.sub}</span>
    </div>
  );
}

function RowContent({ row }: { row: RowView }) {
  return (
    <>
      <span className={'av ' + row.kind} aria-hidden="true">
        {row.short}
      </span>
      <span className="rc" aria-hidden="true">
        <span className="nm">
          <b>{row.name}</b>
          {row.chips.map((c) => (
            <Chip key={c.t} chip={c} />
          ))}
        </span>
        <span className="wins">
          {row.lines.map((l) => (
            <span className="win" key={l.label} title={l.title || undefined}>
              <span className="wl">{l.label}</span>
              <Meter percent={l.percent} tone={row.dim ? 'muted' : l.tone} />
              <b className="wp">{l.text}</b>
              <span className="wr">{l.reset}</span>
            </span>
          ))}
        </span>
      </span>
    </>
  );
}

/**
 * A row the owner can press opens the switch sheet. The active row and the
 * rows that cannot switch stay buttons with `aria-disabled`, so they keep
 * focus and screen readers read them (G10). The one account a provider row
 * reads without a sidecar has nothing to switch to and is not a button.
 */
function AccountRow({ row, onSwitch }: { row: RowView; onSwitch(slot: number): void }) {
  const cls = 'row' + (row.active ? ' on' : '') + (row.dim ? ' dim' : '');
  if (row.slot === 0) {
    return (
      <div className={cls + ' ro'} role="listitem" aria-label={row.label} title={row.title || undefined}>
        <RowContent row={row} />
      </div>
    );
  }
  return (
    <div role="listitem">
      <button
        type="button"
        className={cls}
        aria-label={row.label}
        aria-disabled={row.switchable ? undefined : true}
        title={row.title || undefined}
        onClick={row.switchable ? () => onSwitch(row.slot) : undefined}
      >
        <RowContent row={row} />
      </button>
    </div>
  );
}

function SectionBlock(props: { id: 'c' | 'g'; title: string; section: SectionView; onSwitch(slot: number): void }) {
  const s = props.section;
  return (
    <>
      <div className="sec">
        <span className="lbl">
          <span className={'idm ' + props.id} />
          {props.title}
        </span>
        <span>{s.side}</span>
      </div>
      {s.rows.length ? (
        <div className="rows" role="list" aria-label={props.title}>
          {s.rows.map((r) => (
            <AccountRow key={r.key} row={r} onSwitch={props.onSwitch} />
          ))}
        </div>
      ) : null}
      {s.note ? <div className={('gnote ' + s.note.tone).trim()}>{s.note.text}</div> : null}
    </>
  );
}

/** G17: outside the scrolling body, so the toggle is always in view. */
function Footer({ auto, onToggle }: { auto: AutoView; onToggle(): void }) {
  return (
    <div className="li">
      <div className="t">
        <b>{auto.title}</b>
        <small>{auto.line}</small>
        {auto.log && !auto.paused ? <small>{auto.log}</small> : null}
      </div>
      <Toggle on={auto.toggle.on} label={auto.toggle.label} disabled={auto.toggle.disabled} onClick={onToggle} />
    </div>
  );
}

export interface PopoverProps {
  view: PopoverView;
  auto: AutoView;
  feedback?: FeedbackView | null;
  busy?: Action | null;
  onAction(act: Action): void;
  onSwitch(slot: number): void;
  onToggle(): void;
  /** The open sheet's body; null when none is open. */
  sheet?: ReactNode;
  onCloseSheet?(): void;
  /** The popover's natural height, so the window can fit it. */
  onHeight?(height: number): void;
}

export function Popover({ view, auto, feedback, busy, onAction, onSwitch, onToggle, sheet, onCloseSheet, onHeight }: PopoverProps) {
  const head = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const foot = useRef<HTMLDivElement>(null);
  const open = !!sheet;
  const dialog = useDialog(open);
  const banner = bannerUnder(view.banner, feedback);

  useLayoutEffect(() => {
    if (!onHeight || !head.current || !content.current) return;
    const measure = () => {
      const natural = (head.current?.offsetHeight ?? 0) + (content.current?.offsetHeight ?? 0) + (foot.current?.offsetHeight ?? 0) + 4;
      // An open sheet needs room for all of itself above the popover's bottom edge.
      const sheetHeight = dialog.ref.current ? dialog.ref.current.scrollHeight + 16 : 0;
      onHeight(Math.max(natural, sheetHeight));
    };
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    for (const el of [head.current, content.current, foot.current, dialog.ref.current]) if (el) ro.observe(el);
    return () => ro.disconnect();
  }, [onHeight, open, dialog.ref]);

  return (
    <div className={'pop' + (open ? ' open' : '')} role="dialog" aria-label="Quota Station">
      <div className="hd" ref={head} inert={open}>
        <div className="tt">
          <b>Quota Station</b>
          <small>
            <Dot tone={view.fresh.tone} />
            {view.machine} · {view.fresh.text}
          </small>
        </div>
        <button type="button" className="ib" aria-label="Cài đặt" title="Cài đặt" onClick={() => onAction('settings')}>
          <GearIcon />
        </button>
        <button type="button" className="ib" aria-label="Mở cửa sổ chính" title="Mở cửa sổ chính" onClick={() => onAction('window')}>
          <WindowIcon />
        </button>
      </div>
      <div className="body" inert={open}>
        <div ref={content}>
          <div className="sum">
            {view.summary.map((x, i) => (
              <span key={x.t}>
                {i ? <span> · </span> : null}
                <b style={{ fontWeight: i === 0 ? 600 : 500, color: x.bad ? 'var(--bad-ink)' : undefined }}>{x.t}</b>
              </span>
            ))}
          </div>
          {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
          {banner ? <Banner banner={banner} onAction={onAction} busy={busy} /> : null}
          <div className="pools">
            <PoolTile pool={view.pools.claude} />
            <PoolTile pool={view.pools.agy} />
          </div>
          <SectionBlock id="c" title="CLAUDE CODE" section={view.claude} onSwitch={onSwitch} />
          <SectionBlock id="g" title="ANTIGRAVITY" section={view.agy} onSwitch={onSwitch} />
          <div className="legend" aria-hidden="true">
            ↻ đặt lại sau · n = ngày · g = giờ
          </div>
        </div>
      </div>
      <div className="ft" ref={foot} inert={open}>
        <Footer auto={auto} onToggle={onToggle} />
      </div>
      <div className="scrim" aria-hidden="true" onClick={onCloseSheet} />
      {sheet ? (
        <div
          className="sheet"
          role="dialog"
          aria-modal="true"
          aria-labelledby={SHEET_TITLE_ID}
          tabIndex={-1}
          ref={dialog.ref}
          onKeyDown={dialog.onKeyDown}
        >
          {sheet}
        </div>
      ) : null}
    </div>
  );
}
