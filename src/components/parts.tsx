// Small components named after the design system (components.css header):
// Meter = .meter, Chip = .chip, Button = .btn, Banner = .banner, Feedback = .fb.

import type { ReactNode } from 'react';
import type { Action, BannerView, ChipView, FeedbackView } from '../lib/view';

export function Meter({ percent, tone }: { percent: number | null; tone: string }) {
  const width = percent == null ? 0 : Math.max(percent, 1);
  // The number always sits next to the bar, so screen readers skip the bar.
  return (
    <span className="meter" aria-hidden="true">
      <i className={tone} style={{ width: width + '%' }} />
    </span>
  );
}

export function Chip({ chip }: { chip: ChipView }) {
  return <span className={('chip ' + chip.k).trim()}>{chip.t}</span>;
}

export function Button(props: {
  children: ReactNode;
  primary?: boolean;
  disabled?: boolean;
  onClick?: () => void;
}) {
  return (
    <button type="button" className={'btn' + (props.primary ? ' pri' : '')} disabled={props.disabled} onClick={props.onClick}>
      {props.children}
    </button>
  );
}

export function Banner(props: { banner: BannerView; onAction: (act: Action) => void; busy?: Action | null }) {
  const b = props.banner;
  return (
    <div className={'banner ' + b.tone} role="status">
      <b>{b.title}</b>
      {b.body ? <p>{b.body}</p> : null}
      {b.actions.length ? (
        <div className="acts">
          {b.actions.map((a, i) => (
            <Button key={a.act} primary={i === 0} disabled={props.busy === a.act} onClick={() => props.onAction(a.act)}>
              {a.label}
            </Button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function Feedback({ fb }: { fb: FeedbackView }) {
  return (
    <div className={('fb ' + fb.tone).trim()} role="status">
      <div style={{ minWidth: 0, flex: 1 }}>
        <b>{fb.title}</b>
        <small>{fb.body}</small>
      </div>
    </div>
  );
}

export function Dot({ tone }: { tone: string }) {
  return <span className={'dot ' + tone} aria-hidden="true" />;
}

/** SF Symbols-like gear, drawn as in the prototype sketch. */
export function GearIcon() {
  return (
    <svg width="15" height="15" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="8" r="2.3" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <path
        d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
      />
    </svg>
  );
}
