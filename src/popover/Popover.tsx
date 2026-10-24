// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The popover, rebuilt from the community edition's approved prototype
// (24/10/2026; RP-A, D02, D03, D07, D08): the app's name with this Mac and
// how fresh the numbers are, then Đang dùng (the account in use on each
// service, its share used and when it resets), then a group per service:
// Claude's and Antigravity's accounts as cards in the main window's order,
// the one in use with a green dot and a passive "đang dùng" chip, and
// Codex as one switch button. Each group folds to its header and stays
// folded the next time the popover opens. A card opens its switch sheet;
// one whose login expired opens the sign-in again. The footer holds the
// automatic switching toggle while Cài đặt › Nâng cao is on. The header's
// two buttons switch the light and dark look and open the main window.

import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Dialog, DialogContent, DialogOverlay } from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { Icon } from '../components/icons';
import { AgentIcon } from '../components/logos';
import { Banner, Button, Chip, Dot, Feedback, LinkButton, Meter, Skeleton, Spinner, Toggle } from '../components/parts';
import { t } from '../lib/i18n';
import type { Agent } from '../lib/model';
import type { AutoView } from '../lib/switching';
import type { CodexRowView } from '../lib/codex';
import { bannerUnder, type Action, type CardView, type CellView, type FeedbackView, type GroupView, type NowView, type PopoverView, type UsedTone } from '../lib/view';

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
function toneInk(tone: UsedTone | string): string {
  return tone === 'warn' ? 'text-warn-ink' : tone === 'bad' ? 'text-bad-ink' : '';
}

const SVC_AGENT: Record<string, Agent> = { c: 'claude', g: 'agy', x: 'codex' };

/**
 * Đang dùng: a column per service, two lines each (the service and its
 * share used; the account and when it resets), since four blocks per
 * service do not fit 360 points with 13-point text.
 */
function NowStrip({ now, loading }: { now: NowView; loading: boolean }) {
  if (!now.rows.length) return null;
  const cols = now.rows.length === 1 ? 'grid-cols-1' : now.rows.length === 2 ? 'grid-cols-2' : 'grid-cols-3';
  return (
    <div className="grid gap-2 rounded-card border border-line bg-pop-now px-3 pt-2.5 pb-3 shadow-[inset_0_1px_0_var(--hi)]">
      <span className="text-xs font-semi text-ink2">{t('Đang dùng', 'In use')}</span>
      <div className={cn('grid', cols)} role="list" aria-label={t('Đang dùng', 'In use')}>
        {now.rows.map((r) => (
          <div
            className="grid grid-cols-[16px_minmax(0,1fr)] items-center gap-x-1.5 border-l border-line px-[7px] first:border-l-0 first:pl-0 last:pr-0"
            role="listitem"
            key={r.id}
            title={r.title}
            aria-label={r.label}
          >
            <AgentIcon agent={SVC_AGENT[r.id]} className="size-4" />
            <span className="grid min-w-0 leading-[1.35]" aria-hidden="true">
              <span className="flex min-w-0 items-baseline justify-between gap-1">
                <span className="text-xs font-semi text-heading">{r.svc}</span>
                {loading ? (
                  <Skeleton className="w-8" />
                ) : (
                  <span className={cn('text-xs font-semi tabular-nums', r.used == null ? 'text-ink3' : toneInk(r.tone))}>{r.text}</span>
                )}
              </span>
              <span className="flex min-w-0 flex-wrap items-baseline justify-between gap-x-1">
                <span className="min-w-0 flex-[0_1_auto] overflow-hidden text-ellipsis whitespace-nowrap text-2xs text-ink2">{r.name}</span>
                {r.extra ? (
                  <span className={cn('whitespace-nowrap text-2xs font-semi', toneInk(r.extra.tone))}>C/GPT {r.extra.text}</span>
                ) : r.reset ? (
                  <span className="whitespace-nowrap text-2xs text-ink2 tabular-nums">{r.reset}</span>
                ) : null}
              </span>
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

/** One window on a card: its name, a 7-point bar, the share used and when it resets. */
function WindowRow({ cell, dim, loading }: { cell: CellView; dim?: boolean; loading?: boolean }) {
  const tone = dim ? 'muted' : cell.tone;
  if (loading) {
    return (
      <>
        <span className="whitespace-nowrap">{cell.label}</span>
        <Skeleton className="h-(--pop-bar-h) w-full rounded-chip" />
        <span />
        <span />
      </>
    );
  }
  return (
    <>
      <span className="whitespace-nowrap" title={cell.title}>
        {cell.label}
      </span>
      <Meter percent={cell.used} tone={tone} className="h-(--pop-bar-h)" />
      <span className={cn('text-right font-med tabular-nums', cell.used == null ? 'text-ink3' : toneInk(tone))}>{cell.text}</span>
      <span className="text-right text-[12.5px] whitespace-nowrap text-ink2 tabular-nums">{cell.used == null ? '' : cell.reset}</span>
    </>
  );
}

/** A card's head: a dot (green for the account in use), its name, its chips, and › when pressing it does something. */
function CardHead({ card, go, single }: { card: CardView; go: boolean; single: boolean }) {
  return (
    <span className="flex min-w-0 items-center gap-2">
      <Dot tone={card.active ? 'ok' : 'off'} className={card.resting ? 'bg-transparent shadow-[inset_0_0_0_1.5px_var(--dot-off)]' : undefined} />
      <span className="max-w-[60%] shrink-0 overflow-hidden text-[14px] font-heading text-ellipsis whitespace-nowrap text-heading" title={card.full || undefined}>
        {card.name}
      </span>
      {card.chips.map((c) => (
        <Chip key={c.t} chip={c} className="ml-0.5 min-w-0 shrink overflow-hidden text-ellipsis whitespace-nowrap" />
      ))}
      {card.active && !single ? <Chip chip={{ t: t('đang dùng', 'in use'), k: 'on' }} className="ml-auto shrink-0" /> : null}
      {go ? <Icon name="chevron-right" className={cn('size-[15px] text-ink3', card.active && !single ? 'ml-1' : 'ml-auto')} /> : null}
    </span>
  );
}

const CARD =
  'grid w-full gap-[7px] rounded-card border border-line bg-pop-card px-3 pt-2.5 pb-[11px] text-left text-body text-ink shadow-[inset_0_1px_0_var(--hi),var(--sh-card)]';

/**
 * A card the owner can press opens the switch sheet (or the sign-in
 * again). The card in use and the ones that cannot switch stay buttons with
 * `aria-disabled`, so they keep focus and screen readers read them (G10).
 * The one account a row reads without a list has nothing to switch to and
 * is not a button.
 */
function AccountCard(props: { card: CardView; single: boolean; loading: boolean; onSwitch(slot: number): void; onRelogin(slot: number): void }) {
  const { card } = props;
  const can = !props.loading && (card.relogin || card.switchable);
  const cls = cn(CARD, card.resting && 'border-dashed bg-transparent shadow-none');
  const rows = (
    <span className="grid grid-cols-[minmax(48px,max-content)_minmax(0,1fr)_36px_minmax(50px,max-content)] items-center gap-x-2.5 gap-y-1">
      {card.rows.map((c) => (
        <WindowRow key={c.kind} cell={c} dim={card.dim} loading={props.loading} />
      ))}
    </span>
  );
  if (card.slot === 0) {
    return (
      <div className={cls} role="listitem" aria-label={card.label} title={card.title || undefined} data-active={card.active || undefined}>
        <CardHead card={card} go={false} single={props.single} />
        {rows}
      </div>
    );
  }
  return (
    <div role="listitem">
      <button
        type="button"
        className={cn(cls, can ? 'cursor-pointer transition-colors hover:border-line-2' : 'cursor-default')}
        aria-label={card.relogin ? card.name + t(', cần đăng nhập lại. Bấm để đăng nhập lại', ', needs to sign in again. Press to sign in again') : card.label}
        aria-disabled={can ? undefined : true}
        title={card.title || undefined}
        data-active={card.active || undefined}
        onClick={can ? () => (card.relogin ? props.onRelogin(card.slot) : props.onSwitch(card.slot)) : undefined}
      >
        <CardHead card={card} go={can} single={props.single} />
        {rows}
      </button>
    </div>
  );
}

/** A group's header: the service's logo and name, its account count or a read-only tag, and the fold arrow. */
function GroupHead(props: { agent: Agent; title: string; tail: ReactNode; folded: boolean; onFold(): void }) {
  return (
    <button
      type="button"
      className="flex w-full cursor-pointer items-center gap-2 rounded-md border-0 bg-transparent px-0.5 py-0.5 text-left text-heading hover:[&>svg:last-child]:text-ink2"
      aria-expanded={!props.folded}
      onClick={props.onFold}
    >
      <AgentIcon agent={props.agent} className="size-5" />
      <span className="text-pop-h font-heading tracking-head">{props.title}</span>
      {props.tail}
      <Icon name={props.folded ? 'chevron-right' : 'chevron-down'} className="ml-auto size-3.5 text-ink3" />
    </button>
  );
}

function Group(props: {
  group: GroupView;
  agent: Agent;
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
  const tail = g.readOnly ? (
    <Chip chip={{ t: t('chỉ đọc', 'read-only'), k: 'tag' }} />
  ) : g.cards.length ? (
    <span className="text-xs text-ink2 tabular-nums">{g.cards.length}</span>
  ) : null;
  const single = g.cards.length === 1;
  return (
    <section className="grid gap-2" aria-label={g.title}>
      <GroupHead agent={props.agent} title={g.title} tail={tail} folded={props.folded} onFold={props.onFold} />
      {props.folded ? null : (
        <>
          {g.readOnly && g.note ? <p className="-mt-1 pl-[30px] text-xs text-ink2">{g.note.text}</p> : null}
          {props.empty ? (
            <EmptyCard
              agent={props.agent}
              title={t('Chưa có tài khoản Claude nào', 'No Claude accounts yet')}
              body={t('Đăng nhập một tài khoản để xem hạn mức và chuyển nhanh ở đây.', 'Sign in to an account to see its quota and switch here.')}
            >
              <Button small primary onClick={() => props.onAction('claude:add')}>
                {t('Thêm tài khoản…', 'Add account…')}
              </Button>
            </EmptyCard>
          ) : null}
          {g.cards.length ? (
            <div className="grid gap-2" role="list" aria-label={g.title}>
              {g.cards.map((c) => (
                <AccountCard key={c.key} card={c} single={single} loading={props.loading} onSwitch={props.onSwitch} onRelogin={props.onRelogin} />
              ))}
            </div>
          ) : null}
          {props.agent === 'claude' && single && g.cards[0].slot > 0 ? (
            <p className="text-xs text-ink2">
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

/** A note under a group: what is the matter with its accounts. */
function GroupNote({ note }: { note: { tone: string; text: string } }) {
  return <p className={cn('text-xs', note.tone === 'bad' ? 'text-bad-ink' : 'text-ink2')}>{note.text}</p>;
}

/** An empty card with a dashed edge: what is missing and how to get it. */
function EmptyCard(props: { agent?: Agent; title: string; body: string; children?: ReactNode }) {
  return (
    <div className="grid justify-items-center gap-1.5 rounded-card border border-dashed border-line-2 bg-pop-card px-4 py-[22px] text-center">
      {props.agent ? <AgentIcon agent={props.agent} className="size-[30px] text-ink2" /> : <Icon name="info" className="size-[26px] text-ink3" />}
      <b className="text-body font-semi text-heading">{props.title}</b>
      <small className="max-w-[420px] text-sm text-ink2">{props.body}</small>
      {props.children ? <div className="mt-1.5 flex flex-wrap justify-center gap-2">{props.children}</div> : null}
    </div>
  );
}

/**
 * Codex: no account cards, one switch button (M8). The header names the
 * provider in use; the button names where a switch goes and opens the
 * confirmation sheet, never a one-tap switch.
 */
function CodexSection({ row, onOpen, onAction }: { row: CodexRowView; onOpen(): void; onAction(act: Action): void }) {
  const [current, ...states] = row.chips;
  return (
    <section className="grid gap-2" aria-label="Codex">
      <div className="flex items-center gap-2 px-0.5 py-0.5 text-heading">
        <AgentIcon agent="codex" className="size-5" />
        <span className="text-pop-h font-heading tracking-head">Codex</span>
        {current ? <Chip chip={{ ...current, k: 'info' }} /> : null}
        {states.map((c) => (
          <Chip key={c.t} chip={c} />
        ))}
      </div>
      <button
        type="button"
        className="flex min-h-[34px] w-full cursor-pointer items-center justify-center gap-2 rounded-ctl border border-tint-ink/22 bg-tint px-3 text-[14px] font-semi text-tint-ink hover:brightness-[.97] aria-disabled:cursor-default aria-disabled:opacity-55"
        aria-disabled={row.off ? true : undefined}
        aria-label={row.label + '. ' + row.sub + (row.head ? '. ' + row.head : '')}
        onClick={row.off ? undefined : onOpen}
      >
        <Icon name="arrow-right" className="size-[17px]" />
        <span aria-hidden="true">{row.go.replace(/^→\s*/, '')}</span>
      </button>
      {row.note ? <GroupNote note={row.note} /> : null}
      {row.fix ? (
        <div>
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
      <div className="flex items-center gap-2.5">
        <Icon name={auto.toggle.on ? 'shuffle' : 'pause'} className="size-[17px] text-ink2" />
        <div className="grid min-w-0 flex-1">
          <b className="text-body font-semi text-heading">{auto.title}</b>
          {auto.popLine ? <small className="text-2xs text-ink2">{auto.popLine}</small> : null}
        </div>
        <Toggle on={auto.toggle.on} label={auto.toggle.label} disabled={auto.toggle.disabled} onClick={onToggle} />
      </div>
      {sync && sync.tone !== 'ok' ? (
        <div className="flex items-center gap-2 text-xs text-ink2">
          <Dot tone={sync.tone} />
          <small className="min-w-0 flex-1 text-xs">
            {t('Luật xoay vòng · ', 'Rotation rules · ')}
            {sync.text}
          </small>
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
  /** Automatic switching, in the footer; null while Cài đặt › Nâng cao is off. */
  auto: AutoView | null;
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
  const theme = dark ? t('Chuyển sang giao diện sáng', 'Switch to the light look') : t('Chuyển sang giao diện tối', 'Switch to the dark look');
  const fresh = view.fresh.tone === 'ok';
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
      className="material-pop relative flex max-h-screen w-full max-w-full flex-col overflow-hidden rounded-pop in-[.preview]:max-h-(--pop-max-h)"
      role="dialog"
      aria-label="Quota Station"
    >
      {/* The app's name; under it this Mac, and how old the numbers are when they are not fresh. */}
      <div className="flex flex-none items-center gap-0.5 pt-3.5 pr-2.5 pb-2.5 pl-4" ref={head} inert={open}>
        <div className="mr-auto grid min-w-0 gap-px">
          <b className="text-pop-title leading-tight font-heavy tracking-head text-heading">Quota Station</b>
          <span className="flex min-w-0 items-center gap-1.5 whitespace-nowrap text-xs text-ink2">
            <Dot tone={fresh ? 'ok' : view.fresh.tone === 'bad' ? 'bad' : 'off'} />
            {view.machine}
            {fresh ? null : <small className="overflow-hidden text-xs text-ellipsis">· {view.fresh.text}</small>}
          </span>
        </div>
        <Button quiet icon label={theme} onClick={() => onAction('theme')}>
          <Icon name={dark ? 'moon' : 'sun'} />
        </Button>
        <Button quiet icon label={t('Mở Quota Station', 'Open Quota Station')} onClick={() => onAction('window')}>
          <Icon name="square-arrow-out-up-right" />
        </Button>
      </div>
      <div className="body min-h-0 flex-[1_1_auto] overflow-y-auto overscroll-contain" inert={open}>
        <div className="grid content-start gap-3 px-3 pb-3" ref={content}>
          {view.loading ? (
            <p className="flex items-center gap-2 text-xs text-ink2">
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
                  agent="claude"
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
                  agent="agy"
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
      {auto ? (
        <div className="grid flex-none gap-[7px] border-t border-divider px-3.5 py-[9px]" ref={foot} inert={open}>
          <Footer auto={auto} sync={sync} onToggle={onToggle} />
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
