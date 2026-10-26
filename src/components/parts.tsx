// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Small components named after the design system: Meter, Chip, Button,
// Banner, Feedback, Toast, Toggle and ChoiceGroup, drawn with Tailwind on the
// tokens and with shadcn/ui's Button, Badge, Switch and RadioGroup.

import { Fragment, type ComponentProps, type CSSProperties, type ReactNode } from 'react';
import { Badge, badgeTone } from '@/components/ui/badge';
import { Button as UiButton } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/utils';
import type { ChoiceView } from '../lib/switching';
import { isQuiet, type Action, type BannerView, type ChipView, type FeedbackView } from '../lib/view';
import { t } from '../lib/i18n';
import { Glyph, type GlyphName } from './icons';

/** A bar's fill by tone: green by default (a theme pack's own grey), the service's colour for `c` and `g`. */
const METER_FILL: Record<string, string> = { warn: 'bg-warn', bad: 'bg-bad', muted: 'bg-ink3', c: 'bg-id-claude', g: 'bg-id-agy' };

export function Meter({ percent, tone, className }: { percent: number | null; tone: string; className?: string }) {
  const width = percent == null ? 0 : Math.max(percent, 1);
  // The number always sits next to the bar, so screen readers skip the bar.
  return (
    <span className={cn('mt-[5px] mb-1 block h-(--meter-h) overflow-hidden rounded-[2px] bg-track', className)} aria-hidden="true">
      <i className={cn('block h-full rounded-[2px]', METER_FILL[tone] ?? 'bg-meter-ok')} style={{ width: width + '%' }} />
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
  /** An icon before the words, in the button's colour. */
  lead?: GlyphName;
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
      {props.lead ? <Glyph name={props.lead} bold /> : null}
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
 * "Chi tiết": the longer explanation behind a short line, in a popover beside
 * it, so the page keeps one line and nothing is lost. `about` names what it
 * explains for VoiceOver; `text` replaces the link's words.
 */
export function More({ children, about, text, small, className }: { children: ReactNode; about?: string; text?: string; small?: boolean; className?: string }) {
  return (
    <Popover>
      <PopoverTrigger asChild>
        <LinkButton small={small} className={className} aria-label={about ? t('Chi tiết: ', 'Details: ') + about : undefined}>
          {text ?? t('Chi tiết', 'Details')}
        </LinkButton>
      </PopoverTrigger>
      <PopoverContent className="grid gap-1.5 [&_b]:font-semi [&_b]:text-ink [&_p]:m-0">{children}</PopoverContent>
    </Popover>
  );
}

const BANNER_FILL: Record<string, string> = { warn: 'bg-warn/16', bad: 'bg-bad/14', info: 'bg-accent/13' };
const BANNER_INK: Record<string, string> = { warn: 'text-warn-ink', bad: 'text-bad-ink', info: 'text-info-ink' };

/**
 * A banner's tinted box, its title and its line; `children` go under them.
 * `icon` says what a note is about; the box's tint already says how much it matters.
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
  return (
    <div
      className={cn('banner mx-2.5 mb-2 rounded-tile px-[11px] py-[9px]', BANNER_FILL[props.tone] ?? 'bg-fill', props.className)}
      data-tone={props.tone || undefined}
      role={props.tone === 'bad' ? 'alert' : 'status'}
      style={props.style}
    >
      <b className={cn('block text-body font-bold', BANNER_INK[props.tone])}>{props.title}</b>
      {props.body ? <p className="mt-0.5 mb-[7px] text-sm text-ink2">{props.body}</p> : null}
      {props.children}
    </div>
  );
}

/** A banner's buttons, under its text. */
export function BannerButtons({ children }: { children: ReactNode }) {
  return <div className="flex flex-wrap gap-1.5">{children}</div>;
}

export function Banner(props: { banner: BannerView; onAction: (act: Action) => void; busy?: Action | null }) {
  const b = props.banner;
  return (
    <BannerBox tone={b.tone} icon={b.icon} title={b.title} body={b.body ? <Rich text={b.body} /> : null}>
      {b.actions.length ? (
        <BannerButtons>
          {b.actions.map((a, i) => (
            <Button key={a.act} primary={i === 0} disabled={props.busy === a.act} onClick={() => props.onAction(a.act)}>
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
      className="toast absolute bottom-4 left-1/2 z-40 flex -translate-x-1/2 items-center gap-2 whitespace-nowrap rounded-full bg-glass-solid px-3.5 py-[7px] text-sm font-med text-ink shadow-[var(--sh-sheet),inset_0_0_0_.5px_var(--glass-line)] in-data-[slot=popover]:bottom-16"
      role="status"
      title={fb.body || undefined}
    >
      {/* The check is drawn, so the toast's text is its words alone. */}
      <span className="font-black text-ok-ink before:content-['✓']" aria-hidden="true" />
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
      role={tone === 'bad' ? 'alert' : 'status'}
    >
      {children}
    </div>
  );
}

/** A feedback line's title and its line. */
export function FeedbackText({ title, body }: { title: ReactNode; body: ReactNode }) {
  return (
    <>
      <b className="block font-semi">{title}</b>
      <small className="block text-xs text-ink2">{body}</small>
    </>
  );
}

/** The spinner while something runs. */
export function Spinner({ className }: { className?: string }) {
  return (
    <span
      className={cn('mt-px size-3.5 flex-[0_0_auto] animate-qs-spin rounded-full border-2 border-fill2 border-t-accent motion-reduce:animate-none', className)}
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
export const CODE = 'rounded-sm bg-fill px-1 [font:11px_var(--mono)] text-ink [overflow-wrap:anywhere]';

/**
 * Key and value pairs: the key grey, the value to the right. The key takes
 * its line up to 45 % of the width before it wraps, the value the rest; where
 * the pairs have under 17rem, the key goes over its value.
 */
export const KVT =
  'kvt @container grid grid-cols-[fit-content(45%)_minmax(0,1fr)] items-center gap-x-3 gap-y-1.5 text-sm [&>span:nth-child(even)]:text-right [&>span:nth-child(even)]:tabular-nums [&>span:nth-child(odd)]:text-ink2 @max-[17rem]:[&>span]:col-span-2 @max-[17rem]:[&>span:nth-child(even)]:mb-1 @max-[17rem]:[&>span:nth-child(even)]:text-left';

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
          className={cn('grid grid-cols-[16px_minmax(0,1fr)] gap-1.5 before:text-center', FACT_MARK[f.k] ?? "before:content-['•'] before:text-accent-ink")}
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

/** A dot by tone: green by default; grey for an account not in use. */
const DOT_FILL: Record<string, string> = { warn: 'bg-warn', bad: 'bg-bad', muted: 'bg-ink3', off: 'bg-ink3', info: 'bg-accent', ok: 'bg-ok' };

export function Dot({ tone, className }: { tone: string; className?: string }) {
  return <span data-tone={tone} className={cn('inline-block size-[7px] flex-[0_0_auto] rounded-full', DOT_FILL[tone] ?? 'bg-ok', className)} aria-hidden="true" />;
}
