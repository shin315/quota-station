// Small components named after the design system: Meter, Chip, Button,
// Banner, Feedback, Toast, Toggle and ChoiceGroup, drawn with Tailwind on the
// tokens and with shadcn/ui's Button, Badge, Switch and RadioGroup.

import { Fragment, type ComponentProps, type CSSProperties, type ReactNode } from 'react';
import { Badge, badgeTone } from '@/components/ui/badge';
import { Button as UiButton } from '@/components/ui/button';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/utils';
import type { ChoiceView } from '../lib/switching';
import { isQuiet, type Action, type BannerView, type ChipView, type FeedbackView } from '../lib/view';

/** A bar's fill by tone: green by default, the service's colour for `c` and `g`. */
const METER_FILL: Record<string, string> = { warn: 'bg-warn', bad: 'bg-bad', muted: 'bg-ink3', c: 'bg-id-claude', g: 'bg-id-agy' };

export function Meter({ percent, tone, className }: { percent: number | null; tone: string; className?: string }) {
  const width = percent == null ? 0 : Math.max(percent, 1);
  // The number always sits next to the bar, so screen readers skip the bar.
  return (
    <span className={cn('mt-[5px] mb-1 block h-(--meter-h) overflow-hidden rounded-[2px] bg-track', className)} aria-hidden="true">
      <i className={cn('block h-full rounded-[2px]', METER_FILL[tone] ?? 'bg-ok')} style={{ width: width + '%' }} />
    </span>
  );
}

export function Chip({ chip, className }: { chip: ChipView; className?: string }) {
  return (
    <Badge tone={badgeTone(chip.k)} className={className} title={chip.title}>
      {chip.t}
    </Badge>
  );
}

export function Button(props: {
  children: ReactNode;
  primary?: boolean;
  /** Removes or puts back something: red. */
  danger?: boolean;
  large?: boolean;
  small?: boolean;
  disabled?: boolean;
  label?: string;
  title?: string;
  className?: string;
  onClick?: () => void;
}) {
  return (
    <UiButton
      variant={props.primary ? 'primary' : props.danger ? 'danger' : 'default'}
      size={props.large ? 'lg' : props.small ? 'sm' : 'default'}
      className={props.className}
      disabled={props.disabled}
      aria-label={props.label}
      title={props.title}
      onClick={props.onClick}
    >
      {props.children}
    </UiButton>
  );
}

/** A text button in the accent colour (`bad`: red), as macOS draws links. */
export function LinkButton({
  small,
  bad,
  className,
  ...props
}: Omit<ComponentProps<'button'>, 'type'> & { small?: boolean; bad?: boolean }) {
  return <UiButton variant={bad ? 'linkBad' : 'link'} size={small ? 'linkSm' : 'link'} className={className} {...props} />;
}

const BANNER_FILL: Record<string, string> = { warn: 'bg-warn/16', bad: 'bg-bad/14', info: 'bg-accent/13' };
const BANNER_INK: Record<string, string> = { warn: 'text-warn-ink', bad: 'text-bad-ink', info: 'text-info-ink' };

/** A banner's tinted box, its title and its line; `children` go under them. */
export function BannerBox(props: { tone: string; title: ReactNode; body?: ReactNode; style?: CSSProperties; children?: ReactNode }) {
  return (
    <div
      className={cn('banner mx-2.5 mb-2 rounded-tile px-[11px] py-[9px]', BANNER_FILL[props.tone] ?? 'bg-fill')}
      data-tone={props.tone || undefined}
      role="status"
      style={props.style}
    >
      <b className={cn('block text-body font-bold', BANNER_INK[props.tone])}>{props.title}</b>
      {props.body ? <p className="mt-0.5 mb-[7px] text-sm text-ink2">{props.body}</p> : null}
      {props.children}
    </div>
  );
}

export function Banner(props: { banner: BannerView; onAction: (act: Action) => void; busy?: Action | null }) {
  const b = props.banner;
  return (
    <BannerBox tone={b.tone} title={b.title} body={b.body}>
      {b.actions.length ? (
        <div className="flex flex-wrap gap-1.5">
          {b.actions.map((a, i) => (
            <Button key={a.act} primary={i === 0} disabled={props.busy === a.act} onClick={() => props.onAction(a.act)}>
              {a.label}
            </Button>
          ))}
        </div>
      ) : null}
    </BannerBox>
  );
}

/** A small line under a field: grey, red when refused, green when fine, faint when off. */
export function FieldHint({ tone, className, ...props }: ComponentProps<'div'> & { tone?: string }) {
  return (
    <div
      data-tone={tone || undefined}
      className={cn('mt-1 text-xs', tone === 'bad' ? 'text-bad-ink' : tone === 'ok' ? 'text-ok-ink' : tone === 'off' ? 'text-ink3' : 'text-ink2', className)}
      {...props}
    />
  );
}

/**
 * A finished action, said once at the bottom of its tab or of the popover
 * (over its footer); it goes by itself. The app shows and hides it, so it
 * stays on the tab of the action it reports.
 */
export function Toast({ fb }: { fb: FeedbackView }) {
  return (
    <div
      className="toast absolute bottom-4 left-1/2 z-40 flex -translate-x-1/2 items-center gap-2 whitespace-nowrap rounded-full bg-glass-solid px-3.5 py-[7px] text-sm font-med text-ink shadow-[var(--sh-sheet),inset_0_0_0_.5px_var(--glass-line)] in-data-[slot=popover]:bottom-16"
      role="status"
      title={fb.body || undefined}
    >
      <span className="font-black text-ok-ink" aria-hidden="true">
        ✓
      </span>
      {fb.title.replace(/\.$/, '')}
    </div>
  );
}

const FEEDBACK_FILL: Record<string, string> = { info: 'bg-accent/12', warn: 'bg-warn/15', bad: 'bg-bad/13' };

/** A line after an action: tinted by its tone, its buttons under it. */
export function FeedbackLine({ tone, className, style, children }: { tone: string; className?: string; style?: CSSProperties; children: ReactNode }) {
  return (
    <div
      style={style}
      data-tone={tone || undefined}
      className={cn('mx-2.5 mb-2 flex items-start gap-2 rounded-tile px-[11px] py-2 text-sm in-[.app-window]:m-0', FEEDBACK_FILL[tone] ?? 'bg-ok/14', className)}
      role="status"
    >
      {children}
    </div>
  );
}

/** A line's title and the smaller line under it. */
export function FeedbackText({ title, body }: { title: ReactNode; body: ReactNode }) {
  return (
    <>
      <b className="block font-semi">{title}</b>
      <small className="block text-xs text-ink2">{body}</small>
    </>
  );
}

/** The working sheet's spinner. */
export function Spinner({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        'mt-px size-3.5 flex-[0_0_auto] animate-qs-spin rounded-full border-2 border-fill2 border-t-accent',
        className,
      )}
      aria-hidden="true"
    />
  );
}

/** `inline`: a line inside a card, which never becomes a toast. */
export function Feedback(props: { fb: FeedbackView; inline?: boolean; onAction: (act: Action) => void; className?: string; style?: CSSProperties }) {
  const { fb, inline, onAction } = props;
  if (isQuiet(fb) && !inline) return <Toast fb={fb} />;
  return (
    <FeedbackLine tone={fb.tone} className={props.className} style={props.style}>
      <div style={{ minWidth: 0, flex: 1 }}>
        <FeedbackText title={fb.title} body={fb.body} />
        {fb.actions?.length ? (
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {fb.actions.map((a, i) => (
              <Button key={a.act} primary={i === 0 && fb.tone !== ''} onClick={() => onAction(a.act)}>
                {a.label}
              </Button>
            ))}
          </div>
        ) : null}
      </div>
    </FeedbackLine>
  );
}

/** The design system's switch: on means automatic switching may run. */
export function Toggle(props: { on: boolean; label: string; disabled?: boolean; onClick(): void }) {
  return <Switch checked={props.on} aria-label={props.label} disabled={props.disabled} onCheckedChange={() => props.onClick()} />;
}

/** The small bold label over a choice or a segmented control. */
export function ChoiceLabel({ id, className, children }: { id?: string; className?: string; children: ReactNode }) {
  return (
    <div className={cn('mb-1 text-s font-semi', className)} id={id}>
      {children}
    </div>
  );
}

/** Choose-one group (G10): radios with a second line, arrow keys move the choice. */
export function ChoiceGroup<V extends string>(props: {
  id: string;
  label: string;
  labelClassName?: string;
  options: ChoiceView<V>[];
  value: V;
  columns?: 3;
  onChange(v: V): void;
}) {
  return (
    <>
      <ChoiceLabel id={props.id + '-l'} className={props.labelClassName}>
        {props.label}
      </ChoiceLabel>
      <RadioGroup
        className={props.columns ? 'grid-cols-[repeat(3,minmax(0,1fr))]' : undefined}
        aria-labelledby={props.id + '-l'}
        value={props.value}
        onValueChange={(v) => props.onChange(v as V)}
      >
        {props.options.map((o) => (
          <RadioGroupItem key={o.v} value={o.v}>
            <b className="font-semi">{o.t}</b>
            <small className="col-start-2 text-xs text-ink2 tabular-nums">{o.s}</small>
          </RadioGroupItem>
        ))}
      </RadioGroup>
    </>
  );
}

/** Code in a line of text: monospaced on a light fill. */
export const CODE = 'rounded-sm bg-fill px-1 [font:11px_var(--mono)] text-ink [overflow-wrap:anywhere]';

/** Key and value pairs: the key grey, the value to the right. */
export const KVT =
  'kvt grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3 gap-y-1.5 text-sm [&>span:nth-child(even)]:text-right [&>span:nth-child(even)]:tabular-nums [&>span:nth-child(odd)]:text-ink2';

/** Text with `**bold**` and `` `code` `` runs, as the sheets write their facts. */
export function Rich({ text }: { text: string }) {
  const parts = text.split(/(\*\*[^*]+\*\*|`[^`]+`)/);
  return (
    <>
      {parts.map((p, i) =>
        p.startsWith('**') ? (
          <b key={i} className="tabular-nums">
            {p.slice(2, -2)}
          </b>
        ) : p.startsWith('`') ? (
          <code key={i} className={CODE}>
            {p.slice(1, -1)}
          </code>
        ) : (
          <Fragment key={i}>{p}</Fragment>
        ),
      )}
    </>
  );
}

/** A fact's mark: a blue dot, or `!`, `×`, `✓` for a warning, a refusal, a check. */
const FACT_MARK: Record<string, string> = {
  w: "before:content-['!'] before:font-black before:text-warn-ink",
  x: "before:content-['×'] before:font-black before:text-bad-ink",
  ok: "before:content-['✓'] before:font-black before:text-ok-ink",
};

/** The facts on a sheet: one line each, a mark before it. */
export function Facts({ items, className }: { items: { k: string; text: ReactNode; key?: string }[]; className?: string }) {
  return (
    <ul className={cn('m-0 mb-2.5 grid list-none gap-1.5 p-0 text-sm text-ink2 [&_b]:font-semi [&_b]:text-ink', className)}>
      {items.map((f, i) => (
        <li
          key={f.key ?? (typeof f.text === 'string' ? f.text : i)}
          className={cn(
            'grid grid-cols-[16px_minmax(0,1fr)] gap-1.5 before:text-center',
            FACT_MARK[f.k] ?? "before:content-['•'] before:text-accent-ink",
          )}
        >
          <span>{f.text}</span>
        </li>
      ))}
    </ul>
  );
}

/** The line under a choice: what it means. */
export function Hint({ tone, className, children, ...props }: ComponentProps<'div'> & { tone?: string }) {
  return (
    <div data-tone={tone || undefined} className={cn('mt-[5px] mb-3 text-xs', tone === 'warn' ? 'text-warn-ink' : 'text-ink2', className)} {...props}>
      {children}
    </div>
  );
}

/** A dot by tone: green by default. */
const DOT_FILL: Record<string, string> = { warn: 'bg-warn', bad: 'bg-bad', muted: 'bg-ink3', info: 'bg-accent', ok: 'bg-ok' };

export function Dot({ tone, className }: { tone: string; className?: string }) {
  return <span data-tone={tone} className={cn('inline-block size-[7px] flex-[0_0_auto] rounded-full', DOT_FILL[tone] ?? 'bg-ok', className)} aria-hidden="true" />;
}
