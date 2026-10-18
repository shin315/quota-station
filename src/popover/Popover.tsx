// The read-only popover (F01, F03): the prototype's popover without the
// switching parts, which arrive in later milestones. Its header opens the
// main window and the settings tab.

import { useLayoutEffect, useRef } from 'react';
import { Banner, Chip, Dot, Feedback, GearIcon, Meter } from '../components/parts';
import type { Action, FeedbackView, PoolView, PopoverView, RowView, SectionView } from '../lib/view';
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

function AccountRow({ row }: { row: RowView }) {
  return (
    <div
      className={'row ro' + (row.active ? ' on' : '') + (row.dim ? ' dim' : '')}
      role="listitem"
      aria-label={row.label}
      title={row.title || undefined}
    >
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
    </div>
  );
}

function SectionBlock(props: { id: 'c' | 'g'; title: string; section: SectionView }) {
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
            <AccountRow key={r.key} row={r} />
          ))}
        </div>
      ) : null}
      {s.note ? <div className={('gnote ' + s.note.tone).trim()}>{s.note.text}</div> : null}
    </>
  );
}

export interface PopoverProps {
  view: PopoverView;
  feedback?: FeedbackView | null;
  busy?: Action | null;
  onAction(act: Action): void;
  /** The popover's natural height, so the window can fit it. */
  onHeight?(height: number): void;
}

export function Popover({ view, feedback, busy, onAction, onHeight }: PopoverProps) {
  const head = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (!onHeight || !head.current || !content.current) return;
    const measure = () => onHeight((head.current?.offsetHeight ?? 0) + (content.current?.offsetHeight ?? 0) + 4);
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    ro.observe(head.current);
    ro.observe(content.current);
    return () => ro.disconnect();
  }, [onHeight]);

  return (
    <div className="pop" role="dialog" aria-label="Quota Station">
      <div className="hd" ref={head}>
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
      <div className="body">
        <div ref={content}>
          <div className="sum">
            {view.summary.map((x, i) => (
              <span key={x.t}>
                {i ? <span> · </span> : null}
                <b style={{ fontWeight: i === 0 ? 600 : 500, color: x.bad ? 'var(--bad-ink)' : undefined }}>{x.t}</b>
              </span>
            ))}
          </div>
          {feedback ? <Feedback fb={feedback} /> : null}
          {view.banner ? <Banner banner={view.banner} onAction={onAction} busy={busy} /> : null}
          <div className="pools">
            <PoolTile pool={view.pools.claude} />
            <PoolTile pool={view.pools.agy} />
          </div>
          <SectionBlock id="c" title="CLAUDE CODE" section={view.claude} />
          <SectionBlock id="g" title="ANTIGRAVITY" section={view.agy} />
          <div className="legend" aria-hidden="true">
            ↻ đặt lại sau · n = ngày · g = giờ
          </div>
        </div>
      </div>
    </div>
  );
}
