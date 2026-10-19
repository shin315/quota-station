// The popover (F01, F03, F04, F07), at the approved "Gọn hơn" level: one
// header line with this Mac's name, Đang dùng on top (the account in use on
// each service, its share used and when it resets), then a card per
// account in the main window's order, the one in use framed. Each group (Claude, Antigravity) folds to
// its header alone, and stays folded the next time the popover opens. A card opens its switch sheet; the footer holds the
// automatic switching toggle. The header's two buttons switch between the
// light and dark look and open the main window. Under the groups, Codex is
// one row (M8): the provider in use and where a switch would go; pressing it
// opens the confirmation sheet, never a one-tap switch.

import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Dialog, DialogContent, DialogOverlay } from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { AccountName, IdMark, NowStrip, WindowRow } from '../components/accounts';
import { Banner, Chip, Dot, Feedback, LinkButton, Toggle } from '../components/parts';
import type { AutoView } from '../lib/switching';
import type { CodexRowView } from '../lib/codex';
import { bannerUnder, type Action, type CardView, type FeedbackView, type GroupView, type PopoverView } from '../lib/view';
import { ChevronIcon, MoonIcon, NAV_ICONS, OpenIcon, SunIcon } from '../window/icons';

/** Which groups are folded, kept in this window's storage so it outlives the app. */
export const FOLD_KEY = 'quota-station.popover.folded';

export type Folded = { c: boolean; g: boolean };

export function loadFolded(): Folded {
  try {
    const saved = JSON.parse(localStorage.getItem(FOLD_KEY) ?? '{}') as Partial<Folded>;
    return { c: saved.c === true, g: saved.g === true };
  } catch {
    return { c: false, g: false };
  }
}

function saveFolded(folded: Folded) {
  try {
    localStorage.setItem(FOLD_KEY, JSON.stringify(folded));
  } catch {
    // Storage off: the fold lasts while the app runs.
  }
}

/** A round icon button in the popover's header. */
const ICON_BUTTON = 'grid size-[26px] cursor-pointer place-items-center rounded-ctl border-0 bg-transparent text-ink2 hover:bg-fill [&_svg]:block';

/** A group's header line: Claude and Antigravity fold; Codex's does not. */
const GROUP_HEAD = 'mx-1.5 mt-1 mb-0.5 flex w-[calc(100%-12px)] items-center gap-1.5 rounded-row border-0 bg-transparent px-2 py-[5px] text-left text-xs text-ink2';

const NOTE_TONE: Record<string, string> = { warn: 'bg-warn/14 text-warn-ink', bad: 'bg-bad/12 text-bad-ink' };

/** A note under a group: what is the matter with its accounts. */
function GroupNote({ note }: { note: { tone: string; text: string } }) {
  return <div className={cn('mx-3.5 mt-0.5 mb-1 rounded-lg px-2 py-1.5 text-s', NOTE_TONE[note.tone] ?? 'bg-fill text-ink2')}>{note.text}</div>;
}

function CardBody({ card }: { card: CardView }) {
  return (
    <>
      {/* The card's frame says it is in use; the main window keeps the organization and the hours. */}
      <span className="flex min-w-0 items-baseline gap-1.5">
        <AccountName name={card.name} sub="" full={card.full} chips={card.chips} card />
      </span>
      {card.rows.map((c) => (
        <WindowRow key={c.label} cell={c} wide={card.kind === 'g'} dim={card.dim} />
      ))}
    </>
  );
}

/**
 * A card the owner can press opens the switch sheet. The active card and
 * the ones that cannot switch stay buttons with `aria-disabled`, so they
 * keep focus and screen readers read them (G10). The one account a provider
 * row reads without an account list has nothing to switch to and is not a button.
 */
function AccountCard({ card, onSwitch }: { card: CardView; onSwitch(slot: number): void }) {
  const cls = cn(
    'acard2 block w-full rounded-tile border-0 bg-fill px-[11px] pt-2 pb-[9px] text-left',
    card.active ? 'on cursor-default bg-sel shadow-[inset_0_0_0_1.5px_var(--accent)]' : 'cursor-pointer aria-disabled:cursor-default not-aria-disabled:hover:bg-sel',
  );
  if (card.slot === 0) {
    return (
      <div className={cn(cls, 'cursor-default')} role="listitem" aria-label={card.label} title={card.title || undefined}>
        <CardBody card={card} />
      </div>
    );
  }
  return (
    <div role="listitem">
      <button
        type="button"
        className={cls}
        aria-label={card.label}
        aria-disabled={card.switchable ? undefined : true}
        title={card.title || undefined}
        onClick={card.switchable ? () => onSwitch(card.slot) : undefined}
      >
        <CardBody card={card} />
      </button>
    </div>
  );
}

function Group(props: { group: GroupView; folded: boolean; onFold(): void; onSwitch(slot: number): void }) {
  const g = props.group;
  return (
    <section aria-label={g.title}>
      {/* Đang dùng already says whose account and how much: the header is the group's name alone. */}
      <button type="button" className={cn(GROUP_HEAD, 'cursor-pointer hover:bg-fill')} aria-expanded={!props.folded} onClick={props.onFold}>
        <span className={cn('inline-grid place-items-center transition-transform duration-(--dur-1) ease-qs', props.folded ? '' : 'rotate-90')}>
          <ChevronIcon />
        </span>
        <IdMark id={g.id} />
        <span className="font-bold tracking-[.02em]">{g.title}</span>
      </button>
      {props.folded ? null : (
        <>
          {g.cards.length ? (
            <div className="grid gap-1.5 px-2.5 pb-1" role="list" aria-label={g.title}>
              {g.cards.map((c) => (
                <AccountCard key={c.key} card={c} onSwitch={props.onSwitch} />
              ))}
            </div>
          ) : null}
          {g.note ? <GroupNote note={g.note} /> : null}
        </>
      )}
    </section>
  );
}

/**
 * CodexRow = `.row.cx`, one line: the provider in use, any state chip, and
 * `→` where a switch goes. The sub-line and Codex App's state stay in the
 * Codex tab and the row's spoken label.
 */
function CodexSection({ row, onOpen, onAction }: { row: CodexRowView; onOpen(): void; onAction(act: Action): void }) {
  return (
    <section aria-label="Codex">
      <div className={cn(GROUP_HEAD, 'cursor-default')}>
        <IdMark id="x" />
        <span className="font-bold tracking-[.02em]">CODEX</span>
      </div>
      <div className="px-1.5">
        <button
          type="button"
          className="group grid w-full cursor-pointer grid-cols-[var(--av)_minmax(0,1fr)_auto] items-center gap-[9px] rounded-row border-0 bg-transparent px-2 py-[5px] text-left focus-visible:-outline-offset-2 aria-disabled:cursor-default not-aria-disabled:hover:bg-fill"
          aria-disabled={row.off ? true : undefined}
          aria-label={row.label + '. ' + row.sub + (row.head ? '. ' + row.head : '')}
          onClick={row.off ? undefined : onOpen}
        >
          <span
            className="grid size-(--av) place-items-center rounded-full bg-id-codex-tint text-[10.5px] font-heavy text-id-codex-ink [&_svg]:size-[13px]"
            aria-hidden="true"
          >
            {NAV_ICONS.codex}
          </span>
          <span className="min-w-0">
            <span className="flex min-w-0 flex-wrap items-center gap-[5px]">
              <b className="text-body font-semi">Codex</b>
              {row.chips.map((c) => (
                <Chip key={c.t} chip={c} className="font-med" />
              ))}
            </span>
          </span>
          <span className="whitespace-nowrap text-sm font-med text-accent-ink group-aria-disabled:text-ink2 group-aria-disabled:line-through" aria-hidden="true">
            {row.go}
          </span>
        </button>
      </div>
      {row.note ? <GroupNote note={row.note} /> : null}
      {row.fix ? (
        <div className="mx-3.5 mb-1.5">
          <LinkButton small onClick={() => onAction(row.fix!.act)}>
            {row.fix.label}
          </LinkButton>
        </div>
      ) : null}
    </section>
  );
}

/**
 * G17: outside the scrolling body, so the toggle is always in view. The why
 * line shows only when there is something to act on; the usual reason stays
 * on the main window's Claude tab. The other Macs' line likewise shows only
 * when a copy is behind or failed.
 */
function Footer({ auto, sync, onToggle }: { auto: AutoView; sync: SyncLine | null; onToggle(): void }) {
  return (
    <>
      <div className="li flex items-center gap-2 text-sm">
        <div className="min-w-0 flex-1">
          <b>{auto.title}</b>
          {auto.popLine ? <small className="block text-xs text-ink2">{auto.popLine}</small> : null}
        </div>
        <Toggle on={auto.toggle.on} label={auto.toggle.label} disabled={auto.toggle.disabled} onClick={onToggle} />
      </div>
      {sync && sync.tone !== 'ok' ? (
        <div className="li flex items-center gap-2 text-sm">
          <Dot tone={sync.tone} />
          <div className="min-w-0 flex-1">
            <small className="block text-xs text-ink2">Luật xoay vòng · {sync.text}</small>
          </div>
        </div>
      ) : null}
    </>
  );
}

/** The other Macs and the rotation file, in the footer (I09). */
export interface SyncLine {
  tone: 'ok' | 'warn' | 'bad' | 'info' | 'muted';
  text: string;
}

export interface PopoverProps {
  view: PopoverView;
  /** The Codex row (M8); null on a Mac without Codex. */
  codex?: CodexRowView | null;
  auto: AutoView;
  /** The other Macs' copies of the rotation file; null with no other Mac. */
  sync?: SyncLine | null;
  feedback?: FeedbackView | null;
  busy?: Action | null;
  /** The look on screen now is dark: the theme button offers light. */
  dark?: boolean;
  onAction(act: Action): void;
  onSwitch(slot: number): void;
  /** An Antigravity card was pressed (M4c). */
  onSwitchAgy?(number: number): void;
  onToggle(): void;
  /** The open sheet's body; null when none is open. */
  sheet?: ReactNode;
  onCloseSheet?(): void;
  /** The popover's natural height, so the window can fit it. */
  onHeight?(height: number): void;
}

export function Popover({ view, codex = null, auto, sync = null, feedback, busy, dark = false, onAction, onSwitch, onSwitchAgy, onToggle, sheet, onCloseSheet, onHeight }: PopoverProps) {
  const head = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const foot = useRef<HTMLDivElement>(null);
  const open = !!sheet;
  const sheetRef = useRef<HTMLDivElement>(null);
  const banner = bannerUnder(view.banner, feedback);
  const [folded, setFolded] = useState(loadFolded);
  const fold = (id: 'c' | 'g') =>
    setFolded((cur) => {
      const next = { ...cur, [id]: !cur[id] };
      saveFolded(next);
      return next;
    });
  const theme = dark ? 'Chuyển sang giao diện sáng' : 'Chuyển sang giao diện tối';

  useLayoutEffect(() => {
    if (!onHeight || !head.current || !content.current) return;
    const measure = () => {
      const natural = (head.current?.offsetHeight ?? 0) + (content.current?.offsetHeight ?? 0) + (foot.current?.offsetHeight ?? 0) + 4;
      // An open sheet needs room for all of itself above the popover's bottom edge.
      const sheetHeight = sheetRef.current ? sheetRef.current.scrollHeight + 16 : 0;
      onHeight(Math.max(natural, sheetHeight));
    };
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    for (const el of [head.current, content.current, foot.current, sheetRef.current]) if (el) ro.observe(el);
    return () => ro.disconnect();
  }, [onHeight, open, folded]);

  return (
    <div
      data-slot="popover"
      className="material-pop relative flex max-h-screen w-full max-w-full flex-col overflow-hidden rounded-pop shadow-[var(--sh-pop),inset_0_0_0_.5px_var(--glass-line)] in-[.preview]:max-h-(--pop-max)"
      role="dialog"
      aria-label="Quota Station"
    >
      <div className="flex flex-none items-center gap-1 pt-2.5 pr-2.5 pb-1.5 pl-3.5" ref={head} inert={open}>
        {/* One line: this Mac, and how old the numbers are only when they are not fresh. */}
        <div className="min-w-0 flex-1">
          <b className="flex min-w-0 items-center gap-1.5 whitespace-nowrap text-body font-semi">
            <Dot tone={view.fresh.tone} />
            {view.machine}
            {view.fresh.tone === 'ok' ? null : <small className="overflow-hidden text-ellipsis text-s font-reg text-ink2">· {view.fresh.text}</small>}
          </b>
        </div>
        <button type="button" className={ICON_BUTTON} aria-label={theme} title={theme} onClick={() => onAction('theme')}>
          {dark ? <SunIcon /> : <MoonIcon />}
        </button>
        <button type="button" className={ICON_BUTTON} aria-label="Mở Quota Station" title="Mở Quota Station" onClick={() => onAction('window')}>
          <OpenIcon />
        </button>
      </div>
      <div className="body min-h-0 flex-[1_1_auto] overflow-y-auto overscroll-contain pb-1" inert={open}>
        <div ref={content}>
          {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
          {banner ? <Banner banner={banner} onAction={onAction} busy={busy} /> : null}
          <NowStrip now={view.now} />
          <Group group={view.claude} folded={folded.c} onFold={() => fold('c')} onSwitch={onSwitch} />
          <Group group={view.agy} folded={folded.g} onFold={() => fold('g')} onSwitch={onSwitchAgy ?? (() => {})} />
          {codex ? <CodexSection row={codex} onOpen={() => onAction('codex:open')} onAction={onAction} /> : null}
        </div>
      </div>
      <div className="grid flex-none gap-[7px] border-t-[.5px] border-glass-line px-3.5 pt-2 pb-2.5" ref={foot} inert={open}>
        <Footer auto={auto} sync={sync} onToggle={onToggle} />
      </div>
      <DialogOverlay
        className={cn('z-20 transition-opacity duration-(--dur-2) ease-qs', open ? 'opacity-100' : 'pointer-events-none opacity-0')}
        aria-hidden="true"
        onClick={onCloseSheet}
      />
      <Dialog open={open}>
        {sheet ? (
          <DialogContent kind="sheet" ref={sheetRef}>
            {sheet}
          </DialogContent>
        ) : null}
      </Dialog>
    </div>
  );
}
