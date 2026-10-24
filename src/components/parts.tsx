// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

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
import { Icon, type IconName } from './icons';

/** A bar's fill by tone (D01): grey under 70 %, amber from 70, red from 90; the service's colour for `c` and `g`. */
const METER_FILL: Record<string, string> = { warn: 'bg-warn', bad: 'bg-bad', muted: 'bg-dot-off', c: 'bg-id-claude', g: 'bg-id-agy' };

export function Meter({ percent, tone, className }: { percent: number | null; tone: string; className?: string }) {
  const width = percent == null ? 0 : Math.max(percent, 2);
  // The number always sits next to the bar, so screen readers skip the bar.
  return (
    <span className={cn('relative block h-(--bar-h) min-w-10 overflow-hidden rounded-chip bg-bar-track', className)} aria-hidden="true">
      <i className={cn('absolute inset-y-0 left-0 block rounded-[inherit]', METER_FILL[tone] ?? 'bg-bar-fill')} style={{ width: width + '%' }} />
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
  /** The first action of a neutral banner: blue-tinted (D07). */
  tint?: boolean;
  /** Only text until hovered. */
  quiet?: boolean;
  large?: boolean;
  small?: boolean;
  /** Only an icon: give it a `label`. */
  icon?: boolean;
  /** An icon before the words. */
  lead?: IconName;
  disabled?: boolean;
  label?: string;
  title?: string;
  className?: string;
  onClick?: () => void;
}) {
  return (
    <UiButton
      variant={props.primary ? 'primary' : props.danger ? 'danger' : props.tint ? 'tint' : props.quiet ? 'quiet' : 'default'}
      size={props.icon ? 'icon' : props.large ? 'lg' : props.small ? 'sm' : 'default'}
      className={props.className}
      disabled={props.disabled}
      aria-label={props.label}
      title={props.title ?? (props.icon ? props.label : undefined)}
      onClick={props.onClick}
    >
      {props.lead ? <Icon name={props.lead} /> : null}
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

/**
 * A banner's icon (D07, D08): red for an error or a quota nearly gone; any
 * other note is neutral, its icon saying what it is about (old numbers, a
 * permission, anything else). Amber is only for the quota's bars and numbers.
 */
const BANNER_ICON: Record<string, IconName> = { bad: 'circle-alert', stale: 'clock', perm: 'key-round', note: 'info' };

/**
 * A banner: an icon, its title and its line, then `children` (its buttons).
 * In the main window the buttons sit to the right; in the popover and in
 * a narrow card they go under the text.
 */
export function BannerBox(props: {
  tone: string;
  icon?: BannerView['icon'];
  title: ReactNode;
  body?: ReactNode;
  style?: CSSProperties;
  className?: string;
  children?: ReactNode;
}) {
  const bad = props.tone === 'bad';
  return (
    <div
      className={cn(
        'banner grid grid-cols-[auto_minmax(0,1fr)_auto] items-start gap-x-2.5 gap-y-1 rounded-card border px-3 py-2.5',
        'in-data-[slot=popover]:grid-cols-[auto_minmax(0,1fr)] in-data-[narrow]:grid-cols-[auto_minmax(0,1fr)]',
        bad ? 'border-bn-bad-line bg-bn-bad' : 'border-line-2 bg-card-2',
        props.className,
      )}
      data-tone={props.tone || undefined}
      role={bad ? 'alert' : 'status'}
      style={props.style}
    >
      <Icon name={BANNER_ICON[bad ? 'bad' : (props.icon ?? 'note')]} className={cn('mt-px size-[18px]', bad ? 'text-bad' : 'text-ink2')} />
      <div className="grid min-w-0 gap-0.5">
        <b className="text-sm font-semi text-heading">{props.title}</b>
        {props.body ? <span className="text-[12.5px] text-ink2">{props.body}</span> : null}
      </div>
      {props.children}
    </div>
  );
}

/** A banner's buttons: to its right, or under its text where it is narrow. */
export function BannerButtons({ children }: { children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-end gap-2 self-center in-data-[slot=popover]:col-start-2 in-data-[slot=popover]:justify-start in-data-[narrow]:col-start-2 in-data-[narrow]:justify-start">
      {children}
    </div>
  );
}

export function Banner(props: { banner: BannerView; onAction: (act: Action) => void; busy?: Action | null }) {
  const b = props.banner;
  const neutral = b.tone !== 'bad';
  return (
    <BannerBox tone={b.tone} icon={b.icon} title={b.title} body={b.body ? <Rich text={b.body} /> : null}>
      {b.actions.length ? (
        <BannerButtons>
          {b.actions.map((a, i) => (
            <Button key={a.act} small tint={neutral && i === 0} disabled={props.busy === a.act} onClick={() => props.onAction(a.act)}>
              {a.label}
            </Button>
          ))}
        </BannerButtons>
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
      className="toast material-dlg absolute bottom-4 left-1/2 z-40 flex -translate-x-1/2 items-center gap-2 whitespace-nowrap rounded-chip px-3.5 py-[7px] text-sm font-med text-ink shadow-dlg in-data-[slot=popover]:bottom-16"
      role="status"
      title={fb.body || undefined}
    >
      <Icon name="circle-check" className="text-ok" />
      {fb.title.replace(/\.$/, '')}
    </div>
  );
}

/** A feedback line's icon (D08): a check when it went well, a circle with `!` when it did not, `i` for a note. */
const FEEDBACK_ICON: Record<string, [IconName, string]> = {
  '': ['circle-check', 'text-ok'],
  bad: ['circle-alert', 'text-bad'],
  warn: ['triangle-alert', 'text-ink2'],
  info: ['info', 'text-accent-ink'],
};

/** A line after an action: an icon, the text, its buttons. */
export function FeedbackLine({ tone, className, style, children }: { tone: string; className?: string; style?: CSSProperties; children: ReactNode }) {
  const [icon, ink] = FEEDBACK_ICON[tone] ?? FEEDBACK_ICON[''];
  return (
    <div
      style={style}
      data-tone={tone || undefined}
      className={cn(
        'flex items-start gap-2.5 rounded-card border px-3 py-2.5 text-sm',
        tone === 'bad' ? 'border-bn-bad-line bg-bn-bad' : 'border-line bg-card',
        className,
      )}
      role={tone === 'bad' ? 'alert' : 'status'}
    >
      <Icon name={icon} className={cn('mt-px size-5', ink)} />
      {children}
    </div>
  );
}

/** A line's title and the smaller line under it. */
export function FeedbackText({ title, body }: { title: ReactNode; body: ReactNode }) {
  return (
    <>
      <b className="block font-semi text-heading">{title}</b>
      <small className="block text-[12.5px] text-ink2">{body}</small>
    </>
  );
}

/** The spinner while something runs. */
export function Spinner({ className }: { className?: string }) {
  return (
    <span
      className={cn('mt-px size-[18px] flex-[0_0_auto] animate-qs-spin rounded-full border-2 border-line-2 border-t-accent motion-reduce:animate-none', className)}
      aria-hidden="true"
    />
  );
}

/** A grey bar that shimmers where a number will come (the first read). */
export function Skeleton({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        'inline-block h-2.5 animate-qs-shimmer rounded bg-[linear-gradient(90deg,var(--bar-track),color-mix(in_srgb,var(--bar-track)_55%,var(--text-3)),var(--bar-track))] bg-size-[200%_100%] motion-reduce:animate-none',
        className,
      )}
      aria-hidden="true"
    />
  );
}

/**
 * `inline`: a line inside a card, which never becomes a toast. Its buttons
 * sit to the right in the main window and under the text in the popover.
 */
export function Feedback(props: { fb: FeedbackView; inline?: boolean; onAction: (act: Action) => void; className?: string; style?: CSSProperties }) {
  const { fb, inline, onAction } = props;
  if (isQuiet(fb) && !inline) return <Toast fb={fb} />;
  return (
    <FeedbackLine tone={fb.tone} className={props.className} style={props.style}>
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-2 in-data-[slot=popover]:block">
        <div className="min-w-0 flex-1">
          <FeedbackText title={fb.title} body={fb.body} />
        </div>
        {fb.actions?.length ? (
          <div className="flex flex-wrap gap-2 in-data-[slot=popover]:mt-2">
            {fb.actions.map((a) => (
              <Button key={a.act} small onClick={() => onAction(a.act)}>
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
  /** `s`: the second line, with ``code`` or **bold** runs drawn (see Rich). */
  options: (Omit<ChoiceView<V>, 'until'> & { until?: Date | null })[];
  value: V;
  /** Two columns by default; the pause's choices stack in one. */
  columns?: 1 | 3;
  onChange(v: V): void;
}) {
  return (
    <>
      <ChoiceLabel id={props.id + '-l'} className={props.labelClassName}>
        {props.label}
      </ChoiceLabel>
      <RadioGroup
        className={props.columns === 3 ? 'grid-cols-[repeat(3,minmax(0,1fr))]' : props.columns === 1 ? 'grid-cols-1' : undefined}
        aria-labelledby={props.id + '-l'}
        value={props.value}
        onValueChange={(v) => props.onChange(v as V)}
      >
        {props.options.map((o) => (
          <RadioGroupItem key={o.v} value={o.v}>
            <b className="font-semi">{o.t}</b>
            <small className="col-start-2 text-xs text-ink2 tabular-nums">
              <Rich text={o.s} />
            </small>
          </RadioGroupItem>
        ))}
      </RadioGroup>
    </>
  );
}

/** Code in a line of text: monospaced on a light fill. */
export const CODE = 'rounded-[5px] border border-line bg-card-2 px-[5px] py-px font-mono text-[.92em] text-ink [overflow-wrap:anywhere]';

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

/** A fact's icon: a warning, a refusal, a check, or a note. */
const FACT_ICON: Record<string, [IconName, string]> = {
  w: ['triangle-alert', 'text-ink2'],
  x: ['circle-alert', 'text-bad'],
  ok: ['circle-check', 'text-ok'],
};

/** The facts on a sheet: one line each, an icon before it. */
export function Facts({ items, className }: { items: { k: string; text: ReactNode; key?: string }[]; className?: string }) {
  return (
    <ul className={cn('m-0 grid list-none gap-2 p-0 text-sm text-ink [&_b]:font-semi', className)}>
      {items.map((f, i) => {
        const [icon, ink] = FACT_ICON[f.k] ?? ['info', 'text-ink3'];
        return (
          <li key={f.key ?? (typeof f.text === 'string' ? f.text : i)} className="grid grid-cols-[16px_minmax(0,1fr)] items-start gap-2">
            <Icon name={icon} className={cn('mt-0.5', ink)} />
            <span>{f.text}</span>
          </li>
        );
      })}
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

/** A dot by tone: green by default; grey for an account not in use. */
const DOT_FILL: Record<string, string> = { warn: 'bg-warn', bad: 'bg-bad', muted: 'bg-dot-off', off: 'bg-dot-off', info: 'bg-accent', ok: 'bg-ok' };

export function Dot({ tone, className }: { tone: string; className?: string }) {
  return <span data-tone={tone} className={cn('inline-block size-2 flex-[0_0_auto] rounded-full', DOT_FILL[tone] ?? 'bg-ok', className)} aria-hidden="true" />;
}
