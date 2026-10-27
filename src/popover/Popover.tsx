// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The popover (F01, F03, F04, F07), as the approved direction C, "Đang dùng
// trước": one header line with this Mac's name, then a section per service
// (Claude, Antigravity, Codex), each its account in use with every window's
// bar, the share used and when it resets, and its other accounts behind one
// "n tài khoản khác" right under it. A section folds to its header (which
// then says the account in use and its number), and the folds and the
// disclosures stay as they were the next time the popover opens. Another
// account opens its switch sheet; one whose login expired opens the sign-in
// again. Codex's ChatGPT accounts sit as Claude's do, and its custom API keeps
// its own entry; both open the confirmation sheet, never a one-tap switch.
// The popover writes no per-account notes (the rotation, a number's age):
// those stay in the main window, an old number is drawn dim with its age on
// hover, and only what the owner must do (sign in again) keeps a short line.
// The header's buttons switch between the light and dark look, open the
// notifications (a bell with the red count of those not read yet: the main
// window with its bell's panel open) and open the main window. The footer
// holds the automatic switching toggle while Cài đặt › Nâng cao is on, a
// quiet line when a newer release is out, and what the numbers mean behind
// Chi tiết.

import { useId, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Dialog, DialogContent, DialogOverlay } from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { IdMark, InUse } from '../components/accounts';
import { PlaceGlyph } from '../components/icons';
import { Banner, Button, Dot, Feedback, LinkButton, Meter, More, Skeleton, Spinner, Toggle } from '../components/parts';
import { isGemini } from '../lib/accounts';
import { mac, t } from '../lib/i18n';
import type { AutoView } from '../lib/switching';
import type { CodexPopView } from '../lib/codex';
import { bannerUnder, type Action, type CardView, type CellView, type ChipView, type FeedbackView, type GroupView, type NoteView, type NowRowView, type PopoverView, type UsedTone } from '../lib/view';
import { ChevronIcon, MoonIcon, OpenIcon, SunIcon } from '../window/icons';
import { bellCount } from '../alerts/Notifications';

type Service = 'c' | 'g' | 'x';

/** Which sections are folded, kept in this window's storage so it outlives the app. */
export const FOLD_KEY = 'quota-station.popover.folded';
/** Which sections show their other accounts, kept the same way. */
export const MORE_KEY = 'quota-station.popover.more';

export type Flags = Record<Service, boolean>;

function loadFlags(key: string): Flags {
  try {
    const saved = JSON.parse(localStorage.getItem(key) ?? '{}') as Partial<Flags>;
    return { c: saved.c === true, g: saved.g === true, x: saved.x === true };
  } catch {
    return { c: false, g: false, x: false };
  }
}

function saveFlags(key: string, flags: Flags) {
  try {
    localStorage.setItem(key, JSON.stringify(flags));
  } catch {
    // Storage off: the fold lasts while the app runs.
  }
}

/** A section's flags, flipped one at a time and kept. */
function useFlags(key: string): [Flags, (id: Service) => void] {
  const [flags, setFlags] = useState(() => loadFlags(key));
  const flip = (id: Service) =>
    setFlags((cur) => {
      const next = { ...cur, [id]: !cur[id] };
      saveFlags(key, next);
      return next;
    });
  return [flags, flip];
}

/** A number's colour when its quota runs short. */
function toneInk(tone: UsedTone | string): string | undefined {
  return tone === 'warn' ? 'text-warn-ink!' : tone === 'bad' ? 'text-bad-ink!' : undefined;
}

/**
 * An account's hover: its email and where its numbers come from, then the
 * notes the popover no longer writes (an old number's age, the rotation).
 */
function hoverOf(title: string, chips: ChipView[]): string | undefined {
  return [title, ...chips.map((c) => c.t)].filter(Boolean).join(' · ') || undefined;
}

/** The columns of a window's line: its name, a bar, the share used, when it resets. */
const ROW_COLS = {
  wide: 'grid-cols-[92px_minmax(0,1fr)_34px_60px]',
  base: 'grid-cols-[52px_minmax(0,1fr)_34px_60px]',
  half: 'grid-cols-[28px_minmax(0,1fr)_30px_42px] gap-[5px]',
};

/** One window: its name, a bar, the share used and when it resets. An old number is drawn dim. */
function WindowRow({ cell, label, cols = 'base', dim, loading }: { cell: CellView; label?: string; cols?: keyof typeof ROW_COLS; dim?: boolean; loading?: boolean }) {
  const tone = dim ? 'muted' : cell.tone;
  return (
    <span className={cn('mt-1 grid items-center gap-[7px] text-xs text-ink2 tabular-nums', ROW_COLS[cols])} title={cell.title}>
      <span className="truncate">{label ?? cell.label}</span>
      {loading ? (
        <>
          <Skeleton className="h-(--meter-h-s) w-full" />
          <span />
          <span />
        </>
      ) : (
        <>
          <Meter percent={cell.used} tone={tone} className="m-0 h-(--meter-h-s)" />
          <b className={cn('text-right', cell.used == null ? 'font-reg text-ink3' : 'font-semi text-ink', toneInk(tone), dim && 'text-ink3')}>{cell.text}</b>
          <span className={cn('truncate', (cell.used == null || dim) && 'text-ink3')}>{cell.used == null ? '' : cell.reset}</span>
        </>
      )}
    </span>
  );
}

/** Antigravity's four windows in two columns, Gemini and Claude/GPT, each with its five hours and its week. */
function AgyColumns({ cells, dim, loading }: { cells: CellView[]; dim: boolean; loading: boolean }) {
  const cols = [
    { name: 'Gemini', cells: cells.filter(isGemini) },
    { name: 'Claude/GPT', cells: cells.filter((c) => !isGemini(c)) },
  ];
  return (
    <span className="mt-1 grid grid-cols-2 gap-x-3">
      {cols.map((col, i) => (
        <span key={col.name} className={cn('min-w-0', i > 0 && 'border-l-[.5px] border-glass-line pl-3')}>
          <small className="block text-xs text-ink2">{col.name}</small>
          {col.cells.map((c) => (
            <WindowRow key={c.kind} cell={c} label={c.kind.endsWith('Week') ? t('Tuần', 'Week') : t('5 giờ', '5-hour')} cols="half" dim={dim} loading={loading} />
          ))}
        </span>
      ))}
    </span>
  );
}

/** Antigravity's four windows are there to draw in two columns (an account with no reading has two numbers). */
const fourWindows = (card: CardView) => card.kind === 'g' && card.cells.length === 4 && card.cells.some(isGemini);

/** What the owner must do: sign in again. One short line, no chip. */
function ReloginLine() {
  return <small className="block text-xs text-warn-ink">{t('Cần đăng nhập lại', 'Needs to sign in again')}</small>;
}

/** The tile the account in use sits in. */
const TILE = 'mx-2.5 block rounded-tile bg-fill px-[11px] pt-2 pb-[9px]';

/** The account in use: its name, "Đang dùng", and every window. A status, not a button. */
function InUseCard({ card, loading, onRelogin }: { card: CardView; loading: boolean; onRelogin(slot: number): void }) {
  return (
    <div className={TILE} role="group" aria-label={card.label} title={hoverOf(card.title, card.chips)} data-active="true" data-stale={card.dim || undefined}>
      <span className="flex min-w-0 items-center gap-2">
        <b className="min-w-0 truncate text-body font-semi">{card.name}</b>
        <InUse />
      </span>
      {card.relogin ? (
        <span className="flex items-center gap-2">
          <ReloginLine />
          <LinkButton small onClick={() => onRelogin(card.slot)}>
            {t('Đăng nhập lại…', 'Sign in again…')}
          </LinkButton>
        </span>
      ) : null}
      {fourWindows(card) ? (
        <AgyColumns cells={card.cells} dim={card.dim} loading={loading} />
      ) : (
        card.rows.map((c) => <WindowRow key={c.kind} cell={c} cols={card.kind === 'g' ? 'wide' : 'base'} dim={card.dim} loading={loading} />)
      )}
    </div>
  );
}

/** A row of another account's quiet list: its name, then its windows' short lines. */
function OtherBody({ name, relogin, children }: { name: string; relogin?: boolean; children: ReactNode }) {
  return (
    <>
      <b className="block truncate text-body font-semi">{name}</b>
      {relogin ? <ReloginLine /> : null}
      {children}
    </>
  );
}

const OTHER_ROW = 'block w-full border-0 bg-transparent px-[11px] pt-[7px] pb-2 text-left';
const OTHER_PRESS = 'cursor-pointer aria-disabled:cursor-default not-aria-disabled:hover:bg-sel';

/**
 * Another account: pressing it opens the switch sheet (or the sign-in
 * again). One that cannot switch stays a button with `aria-disabled`, so it
 * keeps focus and screen readers read it (G10). The one account a provider
 * row reads without an account list is not a button.
 */
function OtherCard(props: { card: CardView; loading: boolean; onSwitch(slot: number): void; onRelogin(slot: number): void }) {
  const { card } = props;
  const can = !props.loading && (card.relogin || card.switchable);
  const body = (
    <OtherBody name={card.name} relogin={card.relogin}>
      {card.rows.map((c) => (
        <WindowRow key={c.kind} cell={c} cols={card.kind === 'g' ? 'wide' : 'base'} dim={card.dim} loading={props.loading} />
      ))}
    </OtherBody>
  );
  const hover = hoverOf(card.title, card.chips);
  if (card.slot === 0) {
    return (
      <div className={OTHER_ROW} role="listitem" aria-label={card.label} title={hover} data-stale={card.dim || undefined}>
        {body}
      </div>
    );
  }
  return (
    <div role="listitem">
      <button
        type="button"
        className={cn(OTHER_ROW, OTHER_PRESS)}
        aria-label={card.relogin ? card.name + t(', cần đăng nhập lại. Bấm để đăng nhập lại', ', needs to sign in again. Press to sign in again') : card.label}
        aria-disabled={can ? undefined : true}
        title={hover}
        data-stale={card.dim || undefined}
        onClick={can ? () => (card.relogin ? props.onRelogin(card.slot) : props.onSwitch(card.slot)) : undefined}
      >
        {body}
      </button>
    </div>
  );
}

/** A disclosure's caret, turned down while it is open. */
function Caret({ open }: { open: boolean }) {
  return (
    <span className={cn('inline-grid place-items-center transition-transform duration-(--dur-1) ease-qs', open && 'rotate-90')} aria-hidden="true">
      <ChevronIcon />
    </span>
  );
}

function othersText(n: number, all: boolean): string {
  if (all) return t(n + ' tài khoản', n + (n === 1 ? ' account' : ' accounts'));
  return t(n + ' tài khoản khác', n + (n === 1 ? ' other account' : ' other accounts'));
}

/**
 * The other accounts of a section, behind one line right under the account
 * in use. Closed, the line still says how many must sign in again.
 */
function Others(props: { label: string; count: number; all: boolean; relogin: number; open: boolean; onOpen(): void; children: ReactNode }) {
  const list = useId();
  return (
    <>
      <button
        type="button"
        className="mx-2.5 mt-1 flex w-[calc(100%-20px)] cursor-pointer items-center gap-2 rounded-row border-0 bg-transparent px-[11px] py-[5px] text-left text-sm text-ink2 hover:bg-fill"
        aria-expanded={props.open}
        aria-controls={props.open ? list : undefined}
        onClick={props.onOpen}
      >
        <Caret open={props.open} />
        <span className="font-med">{othersText(props.count, props.all)}</span>
        {props.relogin && !props.open ? (
          <span className="truncate text-xs text-warn-ink">· {t(props.relogin + ' cần đăng nhập lại', props.relogin + ' to sign in again')}</span>
        ) : null}
      </button>
      {props.open ? (
        <div id={list} className="mx-2.5 mt-0.5 grid divide-y-[.5px] divide-glass-line overflow-hidden rounded-tile bg-fill" role="list" aria-label={props.label}>
          {props.children}
        </div>
      ) : null}
    </>
  );
}

/** A round icon button in the popover's header. */
const ICON_BUTTON = 'grid size-[26px] cursor-pointer place-items-center rounded-ctl border-0 bg-transparent text-ink2 hover:bg-fill [&_svg]:block';

/** A section's header line, which folds it. */
const SECTION_HEAD = 'mx-1.5 mt-1 mb-1 flex w-[calc(100%-12px)] cursor-pointer items-center gap-1.5 rounded-row border-0 bg-transparent px-2 py-[5px] text-left text-xs text-ink2 hover:bg-fill';

/**
 * A section's header: its fold, the service's logo and name. Folded, it
 * says whose account is in use and its number, so folding hides nothing
 * that matters.
 */
function SectionHead({ id, title, folded, now, onFold }: { id: Service; title: string; folded: boolean; now?: NowRowView; onFold(): void }) {
  return (
    <button type="button" className={SECTION_HEAD} aria-expanded={!folded} onClick={onFold}>
      <Caret open={!folded} />
      <IdMark id={id} className="size-3.5" />
      <span className="font-bold tracking-[.02em] uppercase">{title}</span>
      {folded && now ? (
        <span className="ml-auto flex min-w-0 items-baseline gap-1.5 text-xs tabular-nums" title={now.title}>
          <span className="truncate text-ink2">{now.name}</span>
          <b className={cn('font-semi', now.used == null ? 'text-ink3' : 'text-ink', toneInk(now.tone))}>{now.text}</b>
          {now.extra ? <b className={cn('whitespace-nowrap font-semi', toneInk(now.extra.tone))}>C/GPT {now.extra.text}</b> : null}
        </span>
      ) : null}
    </button>
  );
}

const NOTE_TONE: Record<string, string> = { warn: 'bg-warn/14 text-warn-ink', bad: 'bg-bad/12 text-bad-ink' };

/** A note under a section: what is the matter with its accounts. */
function SectionNote({ note }: { note: NoteView }) {
  return <div className={cn('mx-2.5 mt-1 rounded-lg px-2.5 py-1.5 text-s', NOTE_TONE[note.tone] ?? 'bg-fill text-ink2')}>{note.text}</div>;
}

function Group(props: {
  group: GroupView;
  now?: NowRowView;
  loading: boolean;
  folded: boolean;
  more: boolean;
  onFold(): void;
  onMore(): void;
  onSwitch(slot: number): void;
  onRelogin(slot: number): void;
  onAction(act: Action): void;
  /** Claude Code is here and keeps no account yet: an empty card that adds one. */
  empty?: boolean;
}) {
  const g = props.group;
  const active = g.cards.find((c) => c.active);
  const others = g.cards.filter((c) => c !== active);
  return (
    <section className="pb-2" aria-label={g.title}>
      <SectionHead id={g.id} title={g.title} folded={props.folded} now={props.now} onFold={props.onFold} />
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
          {active ? <InUseCard card={active} loading={props.loading} onRelogin={props.onRelogin} /> : null}
          {others.length ? (
            <Others label={g.title} count={others.length} all={!active} relogin={others.filter((c) => c.relogin).length} open={props.more} onOpen={props.onMore}>
              {others.map((c) => (
                <OtherCard key={c.key} card={c} loading={props.loading} onSwitch={props.onSwitch} onRelogin={props.onRelogin} />
              ))}
            </Others>
          ) : null}
          {g.id === 'c' && g.cards.length === 1 && g.cards[0].slot > 0 ? (
            <p className="mx-3.5 mt-1 mb-0 text-xs text-ink2">
              {t('Chỉ có một tài khoản. ', 'Only one account. ')}
              <LinkButton small onClick={() => props.onAction('claude:add')}>
                {t('Thêm tài khoản', 'Add an account')}
              </LinkButton>
              {t(' để chuyển nhanh ở đây.', ' to switch here.')}
            </p>
          ) : null}
          {g.note && !g.readOnly ? <SectionNote note={g.note} /> : null}
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

/** A provider's line in place of windows: it reports no quota. */
function ProviderLine({ sub }: { sub: string }) {
  return <small className="block truncate text-xs text-ink2">{sub}</small>;
}

/**
 * Codex, a section like Claude's: the account in use (a ChatGPT sign-in with
 * its windows, or the provider Codex goes through), then every other
 * account, ChatGPT and custom alike, behind one line. Pressing one opens its
 * switch sheet; one that must sign in again opens the Codex tab.
 */
function CodexSection(props: { pop: CodexPopView; now?: NowRowView; folded: boolean; more: boolean; onFold(): void; onMore(): void; onOpen(): void; onAction(act: Action): void }) {
  const { pop } = props;
  const active = pop.rows.find((r) => r.active);
  const others = pop.rows.filter((r) => r !== active);
  return (
    <section className="pb-2" aria-label="Codex">
      <SectionHead id="x" title="Codex" folded={props.folded} now={props.now} onFold={props.onFold} />
      {props.folded ? null : (
        <>
          {active ? (
            <div className={TILE} role="group" aria-label={active.label + (pop.row.head ? '. ' + pop.row.head : '')} title={hoverOf([active.full, pop.notes].filter(Boolean).join(' · '), active.chips)} data-active="true" data-stale={active.dim || undefined}>
              <span className="flex min-w-0 items-center gap-2">
                <b className="min-w-0 truncate text-body font-semi">{active.name}</b>
                <InUse />
              </span>
              {active.cells ? active.cells.map((c) => <WindowRow key={c.kind} cell={c} dim={active.dim} />) : <ProviderLine sub={t('Custom API · không báo hạn mức', 'Custom API · reports no quota')} />}
            </div>
          ) : null}
          {others.length ? (
            <Others label="Codex" count={others.length} all={!active} relogin={others.filter((r) => r.relogin).length} open={props.more} onOpen={props.onMore}>
              {others.map((r) => {
                const can = r.relogin || !r.off;
                const act: Action = r.relogin ? 'codex:tab' : `codex:use:${r.id}`;
                return (
                  <div role="listitem" key={r.key}>
                    <button
                      type="button"
                      className={cn(OTHER_ROW, OTHER_PRESS)}
                      aria-label={r.label}
                      aria-disabled={can ? undefined : true}
                      title={hoverOf(r.full, r.chips)}
                      data-stale={r.dim || undefined}
                      onClick={can ? () => props.onAction(act) : undefined}
                    >
                      <OtherBody name={r.name} relogin={r.relogin}>
                        {r.relogin ? null : r.cells ? r.cells.map((c) => <WindowRow key={c.kind} cell={c} dim={r.dim} />) : <ProviderLine sub={r.sub} />}
                      </OtherBody>
                    </button>
                  </div>
                );
              })}
            </Others>
          ) : null}
          {pop.note ? <SectionNote note={pop.note} /> : null}
          {pop.row.note ? <SectionNote note={pop.row.note} /> : null}
          {pop.row.fix ? (
            <div className="mx-3.5 mt-1">
              <LinkButton small onClick={() => props.onAction(pop.row.fix!.act)}>
                {pop.row.fix.label}
              </LinkButton>
            </div>
          ) : null}
        </>
      )}
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
        <PlaceGlyph place="rules" className="size-[15px]" />
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

/** What the numbers mean, in one line; the rest behind Chi tiết. */
function Legend() {
  const line = t('% đã dùng · thời gian đặt lại', '% used · time until reset');
  return (
    <div className="flex items-center gap-2 text-xs text-ink2">
      <span className="min-w-0 flex-1 truncate">{line}</span>
      <More small about={line}>
        <p>{t('Số % là phần đã dùng của từng cửa sổ (5 giờ, tuần, Fable); thanh đầy dần khi dùng. Các dịch vụ không cộng chung.', 'The % is the share used of each window (5-hour, week, Fable); the bar fills as it is used. Services are never added up.')}</p>
        <p>{t('Thời gian bên cạnh là còn bao lâu nữa cửa sổ đó đặt lại.', 'The time beside it is how long until that window resets.')}</p>
        <p>{t('“—” là chưa có số, hoặc tài khoản không có cửa sổ đó; không phải 0%.', '“—” means no number yet, or no such window for the account; not 0%.')}</p>
        <p>{t('Số mờ là số cũ: rê chuột lên tài khoản để xem số đọc từ bao giờ. Ghi chú về vòng xoay nằm ở cửa sổ chính.', 'A dim number is an old one: hover the account to see how old. Notes about the rotation are in the main window.')}</p>
        <p>{t('Bấm một tài khoản khác để mở tấm xác nhận chuyển; không chuyển ngay.', 'Press another account to open the switch confirmation; nothing switches at once.')}</p>
      </More>
    </div>
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
  /** Codex (M8): its ChatGPT accounts and custom API; null on a Mac without Codex. */
  codex?: CodexPopView | null;
  /** Automatic switching, in the footer; null while Cài đặt › Nâng cao is off. */
  auto: AutoView | null;
  /** The other Macs' copies of the rotation file; null with no other Mac. */
  sync?: SyncLine | null;
  /** A newer release on GitHub, at the foot; null when there is none to tell. */
  release?: { title: string; body: string } | null;
  /** Notifications not read yet: the bell's count. */
  unread?: number;
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

export function Popover({ view, codex = null, auto, sync = null, release = null, unread = 0, feedback, busy, dark = false, onAction, onSwitch, onSwitchAgy, onToggle, sheet, onCloseSheet, onHeight }: PopoverProps) {
  const head = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const foot = useRef<HTMLDivElement>(null);
  const open = !!sheet;
  const sheetRef = useRef<HTMLDivElement>(null);
  const banner = bannerUnder(view.banner, feedback);
  const [folded, fold] = useFlags(FOLD_KEY);
  const [more, showMore] = useFlags(MORE_KEY);
  const theme = dark ? t('Chuyển sang giao diện sáng', 'Switch to the light look') : t('Chuyển sang giao diện tối', 'Switch to the dark look');
  const count = bellCount(unread);
  const bell = count ? t('Thông báo, ' + unread + ' chưa đọc', 'Notifications, ' + unread + ' unread') : t('Thông báo', 'Notifications');
  const relogin = (slot: number) => onAction(`claude:relogin:${slot}`);
  const nowOf = (id: Service) => view.now.rows.find((r) => r.id === id);

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
  }, [onHeight, open, folded, more]);

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
        <button type="button" className={cn(ICON_BUTTON, 'relative')} aria-label={bell} title={t('Thông báo', 'Notifications')} onClick={() => onAction('notifications')}>
          <PlaceGlyph place="alerts" className="size-[15px]" />
          {count ? (
            <span className="absolute -top-0.5 -right-0.5 min-w-3.5 rounded-full bg-bad-fill px-[3px] text-center text-[9px] leading-3.5 font-heavy text-white tabular-nums" aria-hidden="true">
              {count}
            </span>
          ) : null}
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
              title={mac(t('Chưa thấy Claude Code, Codex hay Antigravity trên máy này', 'No Claude Code, Codex or Antigravity on this Mac yet'))}
              body={t('Quota Station hiện hạn mức của các agent này. Cài một agent rồi bấm Kiểm lại.', 'Quota Station shows the quota of these agents. Install one, then press Check again.')}
            >
              <Button small onClick={() => onAction('refresh')} disabled={busy === 'refresh'}>
                {t('Kiểm lại', 'Check again')}
              </Button>
            </EmptyCard>
          ) : (
            <>
              {view.claude.shown ? (
                <Group
                  group={view.claude}
                  now={nowOf('c')}
                  loading={view.loading}
                  empty={view.claude.empty}
                  folded={folded.c}
                  more={more.c}
                  onFold={() => fold('c')}
                  onMore={() => showMore('c')}
                  onSwitch={onSwitch}
                  onRelogin={relogin}
                  onAction={onAction}
                />
              ) : null}
              {view.agy.shown ? (
                <Group
                  group={view.agy}
                  now={nowOf('g')}
                  loading={view.loading}
                  folded={folded.g}
                  more={more.g}
                  onFold={() => fold('g')}
                  onMore={() => showMore('g')}
                  onSwitch={onSwitchAgy ?? (() => {})}
                  onRelogin={() => onAction('window')}
                  onAction={onAction}
                />
              ) : null}
              {codex ? (
                <CodexSection
                  pop={codex}
                  now={nowOf('x')}
                  folded={folded.x}
                  more={more.x}
                  onFold={() => fold('x')}
                  onMore={() => showMore('x')}
                  onOpen={() => onAction('codex:open')}
                  onAction={onAction}
                />
              ) : null}
            </>
          )}
        </div>
      </div>
      {auto || release || !view.none ? (
        <div className="grid flex-none gap-[7px] border-t-[.5px] border-glass-line px-3.5 pt-2 pb-2.5" ref={foot} inert={open}>
          {release ? <ReleaseLine line={release} onAction={onAction} /> : null}
          {auto ? <Footer auto={auto} sync={sync} onToggle={onToggle} /> : null}
          {view.none ? null : <Legend />}
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
