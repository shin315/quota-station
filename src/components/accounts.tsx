// How one account and its quota windows are drawn, in the popover and in
// the main window's tables (classes from the redesign: AccountName = .an,
// WindowCell = .wc, WindowRow = .wrow, RoomStrip = .room, InUse = .inuse).
// Every number is the share used: the bars fill as the quota goes.

import type { CellView, ChipView, RoomView } from '../lib/view';
import { Chip, Meter } from './parts';

/** Where an account's name is shown: its name, a sub-line, and chips. */
export function AccountName(props: { name: string; sub: string; full: string; chips?: ChipView[] }) {
  return (
    <span className="an">
      <b title={props.full || undefined}>{props.name}</b>
      {props.sub || props.chips?.length ? (
        <small>
          {props.sub ? <span>{props.sub}</span> : null}
          {props.chips?.map((c) => (
            <Chip key={c.t} chip={c} />
          ))}
        </small>
      ) : null}
    </span>
  );
}

/** One window in a table cell: the share used, a bar, and when it resets. */
export function WindowCell({ cell, long, dim }: { cell: CellView; long?: boolean; dim?: boolean }) {
  if (cell.none) {
    return (
      <span className="wc none" title={cell.title}>
        <b>—</b>
      </span>
    );
  }
  const tone = dim ? 'muted' : cell.tone;
  return (
    <span className="wc" title={cell.title}>
      <b className={tone === 'ok' || tone === 'muted' ? undefined : tone}>{cell.text}</b>
      <Meter percent={cell.used} tone={tone} />
      <small>{long ? cell.long : cell.reset}</small>
    </span>
  );
}

/** One window on a popover card: its name, a bar, the share used and when it resets. */
export function WindowRow({ cell, wide, dim }: { cell: CellView; wide?: boolean; dim?: boolean }) {
  const tone = dim ? 'muted' : cell.tone;
  return (
    <span className={'wrow' + (wide ? ' g' : '') + (cell.none ? ' none' : '')} title={cell.title}>
      <span>{cell.label}</span>
      <Meter percent={cell.used} tone={tone} />
      <b className={tone === 'warn' || tone === 'bad' ? 'tn-' + tone : undefined}>{cell.text}</b>
      <span className="r">{cell.reset}</span>
    </span>
  );
}

/** Room before the switch threshold, with each member's tightest window as a bar. */
export function RoomStrip({ room, big }: { room: RoomView; big?: boolean }) {
  return (
    <div className={'room' + (big ? ' card' : '')} title={room.title}>
      <span className="v">
        {room.value}
        <i>{room.unit}</i>
      </span>
      <span>
        <span className="segs" aria-hidden="true">
          {room.segs.map((s) => (
            <span className="seg1" key={s.key}>
              <i className={s.tone} style={{ width: (s.used ?? 0) + '%' }} />
              {room.mark != null ? <em style={{ left: room.mark + '%' }} /> : null}
            </span>
          ))}
        </span>
        {big ? (
          <span className="names">
            {room.segs.map((s) => (
              <span key={s.key}>
                {s.name} {s.used == null ? '—' : s.used + '%'}
              </span>
            ))}
          </span>
        ) : null}
      </span>
      <span className={'sub' + (room.tone === 'bad' ? ' tn-bad' : '')}>{room.sub}</span>
    </div>
  );
}

/** "Đang dùng": a status, not a button. A dot and words, no fill, no border, nothing to press. */
export function InUse() {
  return (
    <span className="inuse">
      <span className="dot ok" aria-hidden="true" />
      Đang dùng
    </span>
  );
}
