// The popover (F01, F03, F04, F07), at the approved "Gọn hơn" level: one
// header line with this Mac's name, Đang dùng on top (the account in use on
// each service, its share used and when it resets), then a card per
// account, the one in use first. Each group (Claude, Antigravity) folds to
// its header alone, and stays folded the next time the popover opens. A card opens its switch sheet; the footer holds the
// automatic switching toggle. The header's two buttons switch between the
// light and dark look and open the main window. Under the groups, Codex is
// one row (M8): the provider in use and where a switch would go; pressing it
// opens the confirmation sheet, never a one-tap switch.

import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { AccountName, NowStrip, WindowRow } from '../components/accounts';
import { Banner, Chip, Dot, Feedback, Toggle } from '../components/parts';
import { SHEET_TITLE_ID, useDialog } from '../components/Sheets';
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

function CardBody({ card }: { card: CardView }) {
  return (
    <>
      {/* The card's frame says it is in use; the main window keeps the organization and the hours. */}
      <span className="h">
        <AccountName name={card.name} sub="" full={card.full} chips={card.chips} />
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
  const cls = 'acard2' + (card.active ? ' on' : '') + (card.dim ? ' dim' : '');
  if (card.slot === 0) {
    return (
      <div className={cls + ' ro'} role="listitem" aria-label={card.label} title={card.title || undefined}>
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
      <button type="button" className="ghd" aria-expanded={!props.folded} onClick={props.onFold}>
        <span className={'chev' + (props.folded ? '' : ' open')}>
          <ChevronIcon />
        </span>
        <span className={'idm ' + g.id} />
        <span className="lbl">{g.title}</span>
      </button>
      {props.folded ? null : (
        <>
          {g.cards.length ? (
            <div className="acards" role="list" aria-label={g.title}>
              {g.cards.map((c) => (
                <AccountCard key={c.key} card={c} onSwitch={props.onSwitch} />
              ))}
            </div>
          ) : null}
          {g.note ? <div className={('gnote ' + g.note.tone).trim()}>{g.note.text}</div> : null}
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
function CodexSection({ row, onOpen }: { row: CodexRowView; onOpen(): void }) {
  return (
    <section aria-label="Codex">
      <div className="ghd st">
        <span className="idm x" />
        <span className="lbl">CODEX</span>
      </div>
      <div className="rows">
        <button
          type="button"
          className="row cx one"
          aria-disabled={row.off ? true : undefined}
          aria-label={row.label + '. ' + row.sub + (row.head ? '. ' + row.head : '')}
          onClick={row.off ? undefined : onOpen}
        >
          <span className="av x" aria-hidden="true">
            {NAV_ICONS.codex}
          </span>
          <span className="rc">
            <span className="nm">
              <b>Codex</b>
              {row.chips.map((c) => (
                <Chip key={c.t} chip={c} />
              ))}
            </span>
          </span>
          <span className="cxgo" aria-hidden="true">
            {row.go}
          </span>
        </button>
      </div>
      {row.note ? <div className={('gnote ' + row.note.tone).trim()}>{row.note.text}</div> : null}
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
      <div className="li">
        <div className="t">
          <b>{auto.title}</b>
          {auto.popLine ? <small>{auto.popLine}</small> : null}
        </div>
        <Toggle on={auto.toggle.on} label={auto.toggle.label} disabled={auto.toggle.disabled} onClick={onToggle} />
      </div>
      {sync && sync.tone !== 'ok' ? (
        <div className="li">
          <span className={'dot ' + sync.tone} aria-hidden="true" />
          <div className="t">
            <small>Luật xoay vòng · {sync.text}</small>
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
  const dialog = useDialog(open);
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
      const sheetHeight = dialog.ref.current ? dialog.ref.current.scrollHeight + 16 : 0;
      onHeight(Math.max(natural, sheetHeight));
    };
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    for (const el of [head.current, content.current, foot.current, dialog.ref.current]) if (el) ro.observe(el);
    return () => ro.disconnect();
  }, [onHeight, open, dialog.ref, folded]);

  return (
    <div className={'pop' + (open ? ' open' : '')} role="dialog" aria-label="Quota Station">
      <div className="hd one" ref={head} inert={open}>
        {/* One line: this Mac, and how old the numbers are only when they are not fresh. */}
        <div className="tt">
          <b>
            <Dot tone={view.fresh.tone} />
            {view.machine}
            {view.fresh.tone === 'ok' ? null : <small>· {view.fresh.text}</small>}
          </b>
        </div>
        <button type="button" className="ib" aria-label={theme} title={theme} onClick={() => onAction('theme')}>
          {dark ? <SunIcon /> : <MoonIcon />}
        </button>
        <button type="button" className="ib" aria-label="Mở Quota Station" title="Mở Quota Station" onClick={() => onAction('window')}>
          <OpenIcon />
        </button>
      </div>
      <div className="body" inert={open}>
        <div ref={content}>
          {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
          {banner ? <Banner banner={banner} onAction={onAction} busy={busy} /> : null}
          <NowStrip now={view.now} />
          <Group group={view.claude} folded={folded.c} onFold={() => fold('c')} onSwitch={onSwitch} />
          <Group group={view.agy} folded={folded.g} onFold={() => fold('g')} onSwitch={onSwitchAgy ?? (() => {})} />
          {codex ? <CodexSection row={codex} onOpen={() => onAction('codex:open')} /> : null}
        </div>
      </div>
      <div className="ft" ref={foot} inert={open}>
        <Footer auto={auto} sync={sync} onToggle={onToggle} />
      </div>
      <div className="scrim" aria-hidden="true" onClick={onCloseSheet} />
      {sheet ? (
        <div
          className="sheet"
          role="dialog"
          aria-modal="true"
          aria-labelledby={SHEET_TITLE_ID}
          tabIndex={-1}
          ref={dialog.ref}
          onKeyDown={dialog.onKeyDown}
        >
          {sheet}
        </div>
      ) : null}
    </div>
  );
}
