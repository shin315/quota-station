// How one account and its quota windows are drawn, in the popover and in
// the main window's tables (classes from the redesign: AccountName = .an,
// WindowCell = .wc, WindowRow = .wrow, NowStrip = .now, NowCard = .nowc,
// InUse = .inuse).
// Every number is the share used: the bars fill as the quota goes.

import type { CellView, ChipView, NowView, UsedTone } from '../lib/view';
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
export function WindowCell({ cell, dim }: { cell: CellView; dim?: boolean }) {
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
      <small>{cell.reset}</small>
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

function toneClass(tone: UsedTone): string | undefined {
  return tone === 'warn' || tone === 'bad' ? 'tn-' + tone : undefined;
}

/**
 * Đang dùng in the popover (`.now.c`): one line per service, the account in
 * use, its share used and when that resets. The window shows on hover.
 */
export function NowStrip({ now }: { now: NowView }) {
  if (!now.rows.length) return null;
  return (
    <div className="now c" role="list" aria-label="Đang dùng">
      {now.rows.map((r) => (
        <div className="now-r" role="listitem" key={r.id} title={r.title} aria-label={r.label}>
          <span className="svc" aria-hidden="true">
            <span className={'idm ' + r.id} />
            {r.svc}
          </span>
          <span className="nmw" aria-hidden="true">
            <b className="nm">{r.name}</b>
            {r.extra ? <b className={'xt ' + (toneClass(r.extra.tone) ?? '')}>C/GPT {r.extra.text}</b> : null}
          </span>
          <b className={'pc' + (r.used == null ? ' none' : '') + (toneClass(r.tone) ? ' ' + toneClass(r.tone) : '')} aria-hidden="true">
            {r.text}
          </b>
          <span className="r" aria-hidden="true">
            {r.reset}
          </span>
        </div>
      ))}
    </div>
  );
}

/** Đang dùng on Tổng quan (`.nowc`): a tile per service with the number large. */
export function NowCard({ now }: { now: NowView }) {
  if (!now.rows.length) return null;
  return (
    <div className="card nowc">
      <h4>Đang dùng</h4>
      <div className="nowg" role="list" aria-label="Đang dùng">
        {now.rows.map((r) => (
          <div className="nt" role="listitem" key={r.id} title={r.title} aria-label={r.label}>
            <small className="svc" aria-hidden="true">
              <span className={'idm ' + r.id} />
              {r.svc}
            </small>
            <b className="nm" aria-hidden="true">
              {r.name}
            </b>
            <small className="sub" aria-hidden="true">
              {r.sub || '\u00a0'}
            </small>
            <b className={'bigp' + (r.used == null ? ' none' : '') + (toneClass(r.tone) ? ' ' + toneClass(r.tone) : '')} aria-hidden="true">
              {r.text}
            </b>
            <Meter percent={r.used} tone={r.tone} />
            <small aria-hidden="true">
              {r.used == null ? r.note : 'đã dùng ' + r.unit + (r.reset ? ' · ' + r.reset : '')}
              {r.extra ? (
                <>
                  {' · '}
                  <b className={toneClass(r.extra.tone)}>C/GPT {r.extra.text}</b>
                </>
              ) : null}
            </small>
          </div>
        ))}
      </div>
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
