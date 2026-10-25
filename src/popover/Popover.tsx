// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The popover (F01, F03, F04, F07), at the approved "Gọn hơn" level: one
// header line with this Mac's name, Đang dùng on top (the account in use on
// each service, its share used and when it resets), then a card per
// account in the main window's order, the one in use framed. Each group
// (Claude, Antigravity) folds to its header alone, and stays folded the next
// time the popover opens. A card opens its switch sheet; one whose login
// expired opens the sign-in again. The footer holds the automatic switching
// toggle while Cài đặt › Nâng cao is on, and a quiet line when a newer
// release is out. The header's two buttons switch between the light and
// dark look and open the main window. Under the groups, Codex is one row
// (M8): the provider in use and where a switch would go; pressing it opens
// the confirmation sheet, never a one-tap switch.

import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Dialog, DialogContent, DialogOverlay } from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { AccountName, IdMark } from '../components/accounts';
import { AgentIcon } from '../components/logos';
import { Banner, Button, Chip, Dot, Feedback, LinkButton, Meter, Skeleton, Spinner, Toggle } from '../components/parts';
import { t } from '../lib/i18n';
import type { AutoView } from '../lib/switching';
import type { CodexRowView } from '../lib/codex';
import { bannerUnder, type Action, type CardView, type CellView, type FeedbackView, type GroupView, type NowView, type PopoverView, type UsedTone } from '../lib/view';
import { ChevronIcon, MoonIcon, OpenIcon, SunIcon } from '../window/icons';

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

/** A number's colour when its quota runs short. */
function toneInk(tone: UsedTone | string): string | undefined {
  return tone === 'warn' ? 'text-warn-ink!' : tone === 'bad' ? 'text-bad-ink!' : undefined;
}

/** The flex name in Đang dùng, as the prototype drew it. */
const NOW_NAME = 'flex min-w-0 flex-wrap items-center gap-[5px] overflow-hidden text-ellipsis whitespace-nowrap font-semi';

/**
 * Đang dùng: one line per service, the account in use, its share used and
 * when that resets. The window shows on hover.
 */
function NowStrip({ now, loading }: { now: NowView; loading: boolean }) {
  if (!now.rows.length) return null;
  return (
    <div className="mx-2.5 mt-0.5 mb-1.5 grid gap-[3px] rounded-tile bg-fill px-2.5 pt-[5px] pb-1.5" role="list" aria-label={t('Đang dùng', 'In use')}>
      {now.rows.map((r) => (
        <div
          className="grid min-h-[19px] grid-cols-[62px_minmax(0,1fr)_34px_60px] items-center gap-[7px] text-xs text-ink2 tabular-nums"
          role="listitem"
          key={r.id}
          title={r.title}
          aria-label={r.label}
        >
          <span className="inline-flex items-center gap-[5px] text-ink2" aria-hidden="true">
            <IdMark id={r.id} className="m-0" />
            {r.svc}
          </span>
          <span className="flex min-w-0 items-baseline gap-1.5" aria-hidden="true">
            <b className={cn(NOW_NAME, 'text-sm text-ink')}>{r.name}</b>
            {r.extra ? <b className={cn('flex-none whitespace-nowrap font-semi', toneInk(r.extra.tone))}>C/GPT {r.extra.text}</b> : null}
          </span>
          {loading ? (
            <Skeleton className="w-full" />
          ) : (
            <b className={cn('text-right', r.used == null ? 'font-reg text-ink3' : 'font-semi text-ink', toneInk(r.tone))} aria-hidden="true">
              {r.text}
            </b>
          )}
          <span className="overflow-hidden text-ellipsis whitespace-nowrap text-left" aria-hidden="true">
            {loading ? null : r.reset}
          </span>
        </div>
      ))}
    </div>
  );
}

/** One window on a card: its name, a bar, the share used and when it resets. */
function WindowRow({ cell, wide, dim, loading }: { cell: CellView; wide?: boolean; dim?: boolean; loading?: boolean }) {
  const tone = dim ? 'muted' : cell.tone;
  return (
    <span
      className={cn(
        'mt-1 grid items-center gap-[7px] text-xs text-ink2 tabular-nums',
        wide ? 'grid-cols-[92px_minmax(0,1fr)_34px_60px]' : 'grid-cols-[52px_minmax(0,1fr)_34px_60px]',
      )}
      title={cell.title}
    >
      <span>{cell.label}</span>
      {loading ? (
        <>
          <Skeleton className="h-(--meter-h-s) w-full" />
          <span />
          <span />
        </>
      ) : (
        <>
          <Meter percent={cell.used} tone={tone} className="m-0 h-(--meter-h-s)" />
          <b className={cn('text-right', cell.used == null ? 'font-reg text-ink3' : 'font-semi text-ink', toneInk(tone))}>{cell.text}</b>
          <span className={cn('whitespace-nowrap', cell.used == null && 'font-reg text-ink3')}>{cell.used == null ? '' : cell.reset}</span>
        </>
      )}
    </span>
  );
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

function CardBody({ card, loading }: { card: CardView; loading: boolean }) {
  return (
    <>
      {/* The card's frame says it is in use; the main window keeps the organization and the hours. */}
      <span className="flex min-w-0 items-baseline gap-1.5">
        <AccountName name={card.name} sub="" full={card.full} chips={card.chips} card />
      </span>
      {card.rows.map((c) => (
        <WindowRow key={c.kind} cell={c} wide={card.kind === 'g'} dim={card.dim} loading={loading} />
      ))}
    </>
  );
}

/**
 * A card the owner can press opens the switch sheet (or the sign-in
 * again). The active card and the ones that cannot switch stay buttons with
 * `aria-disabled`, so they keep focus and screen readers read them (G10).
 * The one account a provider row reads without an account list has nothing
 * to switch to and is not a button. An account out of the rotation has a
 * dashed edge.
 */
function AccountCard(props: { card: CardView; loading: boolean; onSwitch(slot: number): void; onRelogin(slot: number): void }) {
  const { card } = props;
  const can = !props.loading && (card.relogin || card.switchable);
  const cls = cn(
    'acard2 block w-full rounded-tile border-0 bg-fill px-[11px] pt-2 pb-[9px] text-left',
    card.active ? 'on cursor-default bg-sel shadow-[inset_0_0_0_1.5px_var(--accent)]' : 'cursor-pointer aria-disabled:cursor-default not-aria-disabled:hover:bg-sel',
    card.resting && !card.active && 'bg-transparent outline-1 -outline-offset-1 outline-glass-line outline-dashed',
  );
  if (card.slot === 0) {
    return (
      <div className={cn(cls, 'cursor-default')} role="listitem" aria-label={card.label} title={card.title || undefined} data-active={card.active || undefined}>
        <CardBody card={card} loading={props.loading} />
      </div>
    );
  }
  return (
    <div role="listitem">
      <button
        type="button"
        className={cls}
        aria-label={card.relogin ? card.name + t(', cần đăng nhập lại. Bấm để đăng nhập lại', ', needs to sign in again. Press to sign in again') : card.label}
        aria-disabled={can ? undefined : true}
        title={card.title || undefined}
        data-active={card.active || undefined}
        onClick={can ? () => (card.relogin ? props.onRelogin(card.slot) : props.onSwitch(card.slot)) : undefined}
      >
        <CardBody card={card} loading={props.loading} />
      </button>
    </div>
  );
}

function Group(props: {
  group: GroupView;
  loading: boolean;
  folded: boolean;
  onFold(): void;
  onSwitch(slot: number): void;
  onRelogin(slot: number): void;
  onAction(act: Action): void;
  /** Claude Code is here and keeps no account yet: an empty card that adds one. */
  empty?: boolean;
}) {
  const g = props.group;
  const single = g.cards.length === 1;
  return (
    <section aria-label={g.title}>
      {/* Đang dùng already says whose account and how much: the header is the group's name alone. */}
      <button type="button" className={cn(GROUP_HEAD, 'cursor-pointer hover:bg-fill')} aria-expanded={!props.folded} onClick={props.onFold}>
        <span className={cn('inline-grid place-items-center transition-transform duration-(--dur-1) ease-qs', props.folded ? '' : 'rotate-90')}>
          <ChevronIcon />
        </span>
        <IdMark id={g.id} />
        <span className="font-bold tracking-[.02em] uppercase">{g.title}</span>
        {g.readOnly ? <Chip chip={{ t: t('chỉ đọc', 'read-only'), k: 'tag' }} /> : null}
      </button>
      {props.folded ? null : (
        <>
          {g.readOnly && g.note ? <p className="mx-3.5 mt-0 mb-1 text-xs text-ink2">{g.note.text}</p> : null}
          {props.empty ? (
            <EmptyCard
              title={t('Chưa có tài khoản Claude nào', 'No Claude accounts yet')}
              body={t('Đăng nhập một tài khoản để xem hạn mức và chuyển nhanh ở đây.', 'Sign in to an account to see its quota and switch here.')}
            >
              <Button small primary onClick={() => props.onAction('claude:add')}>
                {t('Thêm tài khoản…', 'Add account…')}
              </Button>
            </EmptyCard>
          ) : null}
          {g.cards.length ? (
            <div className="grid gap-1.5 px-2.5 pb-1" role="list" aria-label={g.title}>
              {g.cards.map((c) => (
                <AccountCard key={c.key} card={c} loading={props.loading} onSwitch={props.onSwitch} onRelogin={props.onRelogin} />
              ))}
            </div>
          ) : null}
          {g.id === 'c' && single && g.cards[0].slot > 0 ? (
            <p className="mx-3.5 mt-0 mb-1 text-xs text-ink2">
              {t('Chỉ có một tài khoản. ', 'Only one account. ')}
              <LinkButton small onClick={() => props.onAction('claude:add')}>
                {t('Thêm tài khoản', 'Add an account')}
              </LinkButton>
              {t(' để chuyển nhanh ở đây.', ' to switch here.')}
            </p>
          ) : null}
          {g.note && !g.readOnly ? <GroupNote note={g.note} /> : null}
        </>
      )}
    </section>
  );
}

/** An empty card: what is missing and how to get it. */
function EmptyCard(props: { title: string; body: string; children?: ReactNode }) {
  return (
    <div className="mx-2.5 mb-1.5 grid justify-items-center gap-1 rounded-tile bg-fill px-3 py-3 text-center">
      <b className="text-body font-semi">{props.title}</b>
      <small className="text-xs text-ink2">{props.body}</small>
      {props.children ? <div className="mt-1 flex flex-wrap justify-center gap-1.5">{props.children}</div> : null}
    </div>
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
          <span className="grid size-(--av) place-items-center" aria-hidden="true">
            <AgentIcon agent="codex" />
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
            <small className="block text-xs text-ink2">
              {t('Luật xoay vòng · ', 'Rotation rules · ')}
              {sync.text}
            </small>
          </div>
        </div>
      ) : null}
    </>
  );
}

/** A newer release (u-pop): a quiet line at the foot, opening Tổng quan where its banner is. */
function ReleaseLine({ line, onAction }: { line: { title: string; body: string }; onAction(act: Action): void }) {
  return (
    <div className="li flex items-center gap-2 text-sm" role="status">
      {/* One quiet line: no red dot, no macOS notification. */}
      <small className="min-w-0 flex-1 text-xs text-ink2">
        {line.title} · {line.body}
      </small>
      <LinkButton small onClick={() => onAction('release:open')}>
        {t('Xem', 'View')}
      </LinkButton>
    </div>
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
  /** Automatic switching, in the footer; null while Cài đặt › Nâng cao is off. */
  auto: AutoView | null;
  /** The other Macs' copies of the rotation file; null with no other Mac. */
  sync?: SyncLine | null;
  /** A newer release on GitHub, at the foot; null when there is none to tell. */
  release?: { title: string; body: string } | null;
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

export function Popover({ view, codex = null, auto, sync = null, release = null, feedback, busy, dark = false, onAction, onSwitch, onSwitchAgy, onToggle, sheet, onCloseSheet, onHeight }: PopoverProps) {
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
  const theme = dark ? t('Chuyển sang giao diện sáng', 'Switch to the light look') : t('Chuyển sang giao diện tối', 'Switch to the dark look');
  const relogin = (slot: number) => onAction(`claude:relogin:${slot}`);

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
        <button type="button" className={ICON_BUTTON} aria-label={t('Mở Quota Station', 'Open Quota Station')} title={t('Mở Quota Station', 'Open Quota Station')} onClick={() => onAction('window')}>
          <OpenIcon />
        </button>
      </div>
      <div className="body min-h-0 flex-[1_1_auto] overflow-y-auto overscroll-contain pb-1" inert={open}>
        <div ref={content}>
          {view.loading ? (
            <p className="mx-3.5 mt-0 mb-1.5 flex items-center gap-2 text-xs text-ink2">
              <Spinner className="size-3 border-[1.5px]" />
              {t('Đang đọc hạn mức lần đầu…', 'Reading quota for the first time…')}
            </p>
          ) : null}
          {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
          {banner ? <Banner banner={banner} onAction={onAction} busy={busy} /> : null}
          {view.none ? (
            <EmptyCard
              title={t('Chưa thấy Claude Code, Codex hay Antigravity trên máy này', 'No Claude Code, Codex or Antigravity on this Mac yet')}
              body={t('Quota Station hiện hạn mức của các agent này. Cài một agent rồi bấm Kiểm lại.', 'Quota Station shows the quota of these agents. Install one, then press Check again.')}
            >
              <Button small onClick={() => onAction('refresh')} disabled={busy === 'refresh'}>
                {t('Kiểm lại', 'Check again')}
              </Button>
            </EmptyCard>
          ) : (
            <>
              <NowStrip now={view.now} loading={view.loading} />
              {view.claude.shown ? (
                <Group
                  group={view.claude}
                  loading={view.loading}
                  empty={view.claude.empty}
                  folded={folded.c}
                  onFold={() => fold('c')}
                  onSwitch={onSwitch}
                  onRelogin={relogin}
                  onAction={onAction}
                />
              ) : null}
              {view.agy.shown ? (
                <Group
                  group={view.agy}
                  loading={view.loading}
                  folded={folded.g}
                  onFold={() => fold('g')}
                  onSwitch={onSwitchAgy ?? (() => {})}
                  onRelogin={() => onAction('window')}
                  onAction={onAction}
                />
              ) : null}
              {codex ? <CodexSection row={codex} onOpen={() => onAction('codex:open')} onAction={onAction} /> : null}
            </>
          )}
        </div>
      </div>
      {auto || release ? (
        <div className="grid flex-none gap-[7px] border-t-[.5px] border-glass-line px-3.5 pt-2 pb-2.5" ref={foot} inert={open}>
          {release ? <ReleaseLine line={release} onAction={onAction} /> : null}
          {auto ? <Footer auto={auto} sync={sync} onToggle={onToggle} /> : null}
        </div>
      ) : null}
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
