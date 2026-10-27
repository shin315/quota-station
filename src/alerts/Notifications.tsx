// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Thông báo (F09): the bell at the right of every page's title row, with a
// red count of the notifications not read yet, and the panel it opens: the
// recent ones a day at a time, one line each with the buttons of its macOS
// card, and at its foot the way to Cài đặt › Thông báo, where the kinds are
// switched and tested. Opening the panel reads them. A button pressed here
// or on the macOS card counts once; the line then says what was chosen and
// how it went.

import { Fragment, useEffect, useRef, useState } from 'react';
import { Button as UiButton } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { cn } from '@/lib/utils';
import { PlaceGlyph } from '../components/icons';
import { GROUP_HEAD } from '../components/page';
import { Banner, Button, Feedback, FeedbackLine, FeedbackText, LinkButton, Spinner } from '../components/parts';
import { alertCard, permissionBanner, type AlertCardView } from '../lib/alerts';
import { osName, t } from '../lib/i18n';
import type { Model } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';

/** The bell's count: nothing at zero, "9+" past nine. */
export function bellCount(unread: number): string {
  return unread <= 0 ? '' : unread > 9 ? '9+' : String(unread);
}

/** A notification's mark by tone: amber unless it says otherwise. */
const MARK: Record<string, string> = {
  bad: 'bg-bad/18 text-bad-ink',
  ok: 'bg-ok/20 text-ok-ink',
  info: 'bg-accent/16 text-info-ink',
};

/**
 * One notification: its mark, its title, its line and when it came, the
 * buttons of its macOS card under them; a blue dot while it is new. Once
 * answered it says what was chosen and how it went.
 */
function NoteRow({ card, fresh, pressing, onPress }: { card: AlertCardView; fresh: boolean; pressing: boolean; onPress(action: string): void }) {
  const r = card.result;
  return (
    <div
      className="arec relative grid grid-cols-[26px_minmax(0,1fr)] items-start gap-x-2.5 gap-y-1.5 border-t-[.5px] border-card-line py-2.5"
      role="group"
      aria-label={t('Thông báo: ', 'Notification: ') + card.title}
    >
      <span className={cn('grid size-[26px] place-items-center rounded-[7px] text-sm font-black', MARK[card.icon.k] ?? 'bg-warn/22 text-warn-ink')} aria-hidden="true">
        {card.icon.mark}
      </span>
      <div className="min-w-0">
        <b className="block text-sm font-semi">
          {card.title}
          {fresh ? (
            <>
              <i className="ml-1.5 inline-block size-[7px] rounded-full bg-accent align-middle" aria-hidden="true" />
              <span className="sr-only">{t(', mới', ', new')}</span>
            </>
          ) : null}
        </b>
        <p className="mt-0.5 mb-0 text-sm text-ink2">{card.body}</p>
        {card.chose ? (
          <div className="mt-1 text-sm text-ink2">
            {card.chose}
            {r && 'working' in r ? (
              <FeedbackLine tone="info" style={{ margin: '6px 0 0' }}>
                <Spinner />
                <div>
                  <FeedbackText title={r.working} body={t('Chờ tối đa 60 giây.', 'Waits up to 60 seconds.')} />
                </div>
              </FeedbackLine>
            ) : r ? (
              <Feedback fb={r} inline onAction={() => {}} style={{ margin: '6px 0 0' }} />
            ) : null}
          </div>
        ) : null}
        <div className="mt-1 text-xs text-ink3 tabular-nums">{card.meta}</div>
      </div>
      {card.actions.length ? (
        <div className="col-start-2 flex flex-wrap gap-1.5">
          {card.actions.map((a, i) => (
            <Button key={a.id} small primary={i === 0} disabled={pressing} onClick={() => onPress(a.id)}>
              {a.label}
            </Button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

/** The panel shows this many at first; "Xem tất cả" the rest. */
export const PANEL_FIRST = 5;

export interface NotificationBellProps {
  model: Model;
  now: Date;
  open: boolean;
  onOpenChange(open: boolean): void;
  /** The panel is open with notifications not read yet: they are now. */
  onRead(): void;
  /** Resolves once the press is noted (a switch back runs on after it), to what went wrong if anything did. */
  onPress(id: string, action: string): Promise<FeedbackView | null>;
  /** The permission banner's buttons. */
  onAction(act: Action): void;
  /** "Cài đặt thông báo": Cài đặt at its Thông báo group. */
  onSettings(): void;
}

export function NotificationBell(props: NotificationBellProps) {
  const { model, now, open } = props;
  const a = model.alerts;
  const count = bellCount(a.unread);
  const [pressing, setPressing] = useState<string | null>(null);
  const [all, setAll] = useState(false);
  const [feedback, setFeedback] = useState<FeedbackView | null>(null);
  // The ones that were new when the panel opened keep their dot while it stays open.
  const [fresh, setFresh] = useState<string[]>([]);
  const unreadIds = a.recent
    .filter((r) => !r.read)
    .map((r) => r.id)
    .join(' ');
  const onRead = useRef(props.onRead);
  onRead.current = props.onRead;
  useEffect(() => {
    if (!open) {
      setFresh([]);
      setAll(false);
      setFeedback(null);
      return;
    }
    if (!unreadIds) return;
    setFresh((cur) => [...cur, ...unreadIds.split(' ')]);
    onRead.current();
  }, [open, unreadIds]);

  const press = async (id: string, action: string) => {
    setPressing(id);
    setFeedback(null);
    try {
      setFeedback(await props.onPress(id, action));
    } finally {
      setPressing(null);
    }
  };
  const banner = permissionBanner(a.permission);
  const shown = all ? a.recent : a.recent.slice(0, PANEL_FIRST);
  const days: { label: string; cards: { id: string; card: AlertCardView }[] }[] = [];
  for (const r of shown) {
    const card = alertCard(r, now);
    const last = days[days.length - 1];
    if (last && last.label === card.day) last.cards.push({ id: r.id, card });
    else days.push({ label: card.day, cards: [{ id: r.id, card }] });
  }
  const label = count ? t('Thông báo, ' + a.unread + ' chưa đọc', 'Notifications, ' + a.unread + ' unread') : t('Thông báo', 'Notifications');
  return (
    <Popover open={open} onOpenChange={props.onOpenChange}>
      <PopoverTrigger asChild>
        <UiButton variant="quiet" size="icon" className="relative flex-none" aria-label={label} title={t('Thông báo', 'Notifications')}>
          <PlaceGlyph place="alerts" />
          {count ? (
            <span
              className="absolute -top-0.5 -right-1 min-w-4 rounded-full bg-bad-fill px-1 text-center text-cap leading-4 font-heavy text-white tabular-nums"
              aria-hidden="true"
              data-testid="bell-count"
            >
              {count}
            </span>
          ) : null}
        </UiButton>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        aria-label={t('Thông báo', 'Notifications')}
        tabIndex={-1}
        // The panel itself takes the focus, not its first button: Enter must not answer a card. Tab goes on to the buttons.
        onOpenAutoFocus={(e) => {
          e.preventDefault();
          (e.currentTarget as HTMLElement | null)?.focus();
        }}
        className="flex max-h-[min(560px,var(--radix-popover-content-available-height,560px))] w-[min(400px,calc(100vw-24px))] max-w-none flex-col p-0 text-ink"
      >
        <div className="flex flex-none items-center gap-2 px-3.5 pt-3 pb-1.5">
          <b className="flex-1 text-body font-semi">{t('Thông báo', 'Notifications')}</b>
          {a.recent.length ? <small className="text-xs text-ink3 tabular-nums">{t(a.recent.length + ' gần đây', a.recent.length + ' recent')}</small> : null}
        </div>
        <div className="min-h-0 flex-[1_1_auto] overflow-y-auto overscroll-contain px-3.5 pb-1" role="region" aria-label={t('Gần đây', 'Recent')}>
          {banner ? <Banner banner={banner} onAction={props.onAction} /> : null}
          {feedback ? <Feedback fb={feedback} inline onAction={() => setFeedback(null)} style={{ margin: '4px 0 8px' }} /> : null}
          {days.length ? (
            days.map((d) => (
              <Fragment key={d.label}>
                <div className={cn(GROUP_HEAD, 'border-t-0 pt-1.5 text-xs text-ink2 [&:not(:first-child)]:pt-3')}>{d.label}</div>
                {d.cards.map((c) => (
                  <NoteRow key={c.id} card={c.card} fresh={fresh.includes(c.id)} pressing={pressing === c.id} onPress={(act) => void press(c.id, act)} />
                ))}
              </Fragment>
            ))
          ) : (
            <div className="px-2 py-8 text-center text-sm text-ink2">
              <b className="mb-1 block text-body font-semi text-ink">{t('Chưa có thông báo', 'No notifications yet')}</b>
              {t('Khi có, thông báo sẽ hiện ở đây với cùng các nút bấm như thông báo ' + osName() + '.', 'When one comes, the notification shows here with the same buttons as the ' + osName() + ' notification.')}
            </div>
          )}
          {!all && a.recent.length > PANEL_FIRST ? (
            <LinkButton className="mt-1 mb-2 block w-full" onClick={() => setAll(true)}>
              {t('Xem tất cả (' + a.recent.length + ')', 'See all (' + a.recent.length + ')')}
            </LinkButton>
          ) : null}
        </div>
        <div className="flex flex-none items-center border-t-[.5px] border-card-line px-3.5 py-2">
          <LinkButton onClick={props.onSettings}>{t('Cài đặt thông báo', 'Notification settings')}</LinkButton>
        </div>
      </PopoverContent>
    </Popover>
  );
}
