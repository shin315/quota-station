// Small components named after the design system (components.css header):
// Meter = .meter, Chip = .chip, Button = .btn, Banner = .banner, Feedback = .fb,
// Toggle = .tog, Segmented = .seg, ChoiceGroup = .opts.

import { Fragment, useRef, type KeyboardEvent, type ReactNode } from 'react';
import type { ChoiceView } from '../lib/switching';
import { isQuiet, type Action, type BannerView, type ChipView, type FeedbackView } from '../lib/view';

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
  return (
    <span className={('chip ' + chip.k).trim()} title={chip.title}>
      {chip.t}
    </span>
  );
}

export function Button(props: {
  children: ReactNode;
  primary?: boolean;
  /** Removes or puts back something: red. */
  danger?: boolean;
  large?: boolean;
  disabled?: boolean;
  label?: string;
  title?: string;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      className={'btn' + (props.primary ? ' pri' : '') + (props.danger ? ' danger' : '') + (props.large ? ' lg' : '')}
      disabled={props.disabled}
      aria-label={props.label}
      title={props.title}
      onClick={props.onClick}
    >
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

/** A finished action, said once at the bottom; it goes by itself. */
export function Toast({ fb }: { fb: FeedbackView }) {
  return (
    <div className="toast" role="status" title={fb.body || undefined}>
      <span className="ok" aria-hidden="true">
        ✓
      </span>
      {fb.title.replace(/\.$/, '')}
    </div>
  );
}

/** `inline`: a line inside a card, which never becomes a toast. */
export function Feedback({ fb, inline, onAction }: { fb: FeedbackView; inline?: boolean; onAction: (act: Action) => void }) {
  if (isQuiet(fb) && !inline) return <Toast fb={fb} />;
  return (
    <div className={('fb ' + fb.tone).trim()} role="status">
      <div style={{ minWidth: 0, flex: 1 }}>
        <b>{fb.title}</b>
        <small>{fb.body}</small>
        {fb.actions?.length ? (
          <div className="acts">
            {fb.actions.map((a, i) => (
              <Button key={a.act} primary={i === 0 && fb.tone !== ''} onClick={() => onAction(a.act)}>
                {a.label}
              </Button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}

/** The design system's switch: on means automatic switching may run. */
export function Toggle(props: { on: boolean; label: string; disabled?: boolean; onClick(): void }) {
  return (
    <button
      type="button"
      className="tog"
      role="switch"
      aria-checked={props.on}
      aria-label={props.label}
      disabled={props.disabled}
      onClick={props.onClick}
    />
  );
}

/** Choose-one group (`.opts`, G10): radios with a second line, arrow keys move the choice. */
export function ChoiceGroup<V extends string>(props: {
  id: string;
  label: string;
  options: ChoiceView<V>[];
  value: V;
  columns?: 3;
  onChange(v: V): void;
}) {
  const group = useRef<HTMLDivElement>(null);
  const move = (e: KeyboardEvent, i: number) => {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const n = props.options.length;
    const next = props.options[(i + step + n) % n];
    props.onChange(next.v);
    group.current?.querySelectorAll<HTMLButtonElement>('[role="radio"]')[(i + step + n) % n]?.focus();
  };
  return (
    <>
      <div className="seglbl" id={props.id + '-l'}>
        {props.label}
      </div>
      <div ref={group} className={'opts' + (props.columns ? ' c3' : '')} role="radiogroup" aria-labelledby={props.id + '-l'}>
        {props.options.map((o, i) => {
          const on = o.v === props.value;
          return (
            <button
              key={o.v}
              type="button"
              role="radio"
              aria-checked={on}
              tabIndex={on ? 0 : -1}
              onClick={() => props.onChange(o.v)}
              onKeyDown={(e) => move(e, i)}
            >
              <b>{o.t}</b>
              <small>{o.s}</small>
            </button>
          );
        })}
      </div>
    </>
  );
}

/** Text with `**bold**` and `` `code` `` runs, as the sheets write their facts. */
export function Rich({ text }: { text: string }) {
  const parts = text.split(/(\*\*[^*]+\*\*|`[^`]+`)/);
  return (
    <>
      {parts.map((p, i) =>
        p.startsWith('**') ? (
          <b key={i} className="n">
            {p.slice(2, -2)}
          </b>
        ) : p.startsWith('`') ? (
          <code key={i}>{p.slice(1, -1)}</code>
        ) : (
          <Fragment key={i}>{p}</Fragment>
        ),
      )}
    </>
  );
}

export function Dot({ tone }: { tone: string }) {
  return <span className={'dot ' + tone} aria-hidden="true" />;
}

/** SF Symbols-like gear, drawn as in the prototype sketch. */
