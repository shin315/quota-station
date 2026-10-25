// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Cảnh báo tab (F09), in two views the title row switches between:
// Gần đây, the recent alerts a day at a time, one line each with the
// buttons of its macOS card on the right, and Loại cảnh báo, the five kinds
// with "Gửi thử" and a switch each. A button pressed here or on the macOS
// card counts once; the line then says what was chosen and how it went.

import { Fragment, useState } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Segment, Segmented } from '@/components/ui/segmented';
import { cn } from '@/lib/utils';
import { Empty, GROUP_HEAD, LIST_ROW, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Dot, Feedback, FeedbackLine, FeedbackText, LinkButton, Spinner, Toggle } from '../components/parts';
import { alertCard, alertKinds, permissionBanner, type AlertCardView } from '../lib/alerts';
import { t } from '../lib/i18n';
import type { AlertKind, AlertSwitches, Model } from '../lib/model';
import type { Action, FeedbackView, PopoverView } from '../lib/view';

/** The five kinds: what each is, "Gửi thử" and its switch. Also in Cài đặt. */
export function AlertKinds(props: {
  model: Model;
  switches: AlertSwitches;
  /** macOS shows no card: nothing to send a test to. */
  testOff: boolean;
  onTest(kind: AlertKind): void;
  onSwitch(kind: AlertKind, on: boolean): void;
}) {
  return (
    <div>
      {alertKinds(props.model).map((k) => (
        <div className={LIST_ROW} key={k.kind}>
          <div>
            <b className="font-semi">{k.title}</b>
            <small className="block text-xs text-ink2">{k.text}</small>
          </div>
          <Button disabled={props.testOff} label={t('Gửi thử: ', 'Send a test: ') + k.title} onClick={() => props.onTest(k.kind)}>
            {t('Gửi thử', 'Send a test')}
          </Button>
          <Toggle on={props.switches[k.kind]} label={k.title} onClick={() => props.onSwitch(k.kind, !props.switches[k.kind])} />
        </div>
      ))}
    </div>
  );
}

/** An alert's mark by tone: amber unless it says otherwise. */
const MARK: Record<string, string> = {
  bad: 'bg-bad/18 text-bad-ink',
  ok: 'bg-ok/20 text-ok-ink',
  info: 'bg-accent/16 text-info-ink',
};

/**
 * One alert: its mark, its title, its line and when it came on the left,
 * the buttons of its macOS card on the right (under the words where the page
 * is narrow). Once answered it says what was chosen and how it went.
 */
function AlertRow({ card, pressing, onPress }: { card: AlertCardView; pressing: boolean; onPress(action: string): void }) {
  const r = card.result;
  return (
    <div
      className="arec grid grid-cols-[30px_minmax(0,1fr)_auto] items-start gap-x-3 gap-y-2 border-t-[.5px] border-card-line py-2.5 @max-[640px]/main:grid-cols-[30px_minmax(0,1fr)]"
      role="group"
      aria-label={t('Cảnh báo: ', 'Alert: ') + card.title}
    >
      <span className={cn('grid size-[30px] place-items-center rounded-[8px] font-black', MARK[card.icon.k] ?? 'bg-warn/22 text-warn-ink')} aria-hidden="true">
        {card.icon.mark}
      </span>
      <div className="min-w-0">
        <b className="block text-base font-semi">{card.title}</b>
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
        <div className="flex flex-wrap justify-end gap-1.5 @max-[640px]/main:col-start-2 @max-[640px]/main:justify-start">
          {card.actions.map((a, i) => (
            <Button key={a.id} primary={i === 0} disabled={pressing} onClick={() => onPress(a.id)}>
              {a.label}
            </Button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

/** The two views of the tab. */
type AlertsView = 'recent' | 'kinds';

export interface AlertsTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  onAction(act: Action): void;
  /** Resolves once the press is noted (a switch back runs on after it). */
  onPress(id: string, action: string): Promise<void>;
  onTest(kind: AlertKind): void;
  onSwitch(kind: AlertKind, on: boolean): void;
}

export function AlertsTab(props: AlertsTabProps) {
  const { model, fresh, now } = props;
  const a = model.alerts;
  const [pressing, setPressing] = useState<string | null>(null);
  const [view, setView] = useState<AlertsView>('recent');
  const banner = permissionBanner(a.permission);
  const press = async (id: string, action: string) => {
    setPressing(id);
    try {
      await props.onPress(id, action);
    } finally {
      setPressing(null);
    }
  };
  // A test lands in Gần đây with its buttons, so that is where it opens.
  const test = (kind: AlertKind) => {
    props.onTest(kind);
    setView('recent');
  };
  const cards = a.recent.map((r) => ({ id: r.id, card: alertCard(r, now) }));
  const days: { label: string; cards: typeof cards }[] = [];
  for (const c of cards) {
    const last = days[days.length - 1];
    if (last && last.label === c.card.day) last.cards.push(c);
    else days.push({ label: c.card.day, cards: [c] });
  }
  return (
    <>
      <TabHead>
        <TabTitle className="whitespace-nowrap">{t('Cảnh báo', 'Alerts')}</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
        <Segmented variant="inline" aria-label={t('Xem cảnh báo', 'Show alerts')}>
          <Segment className="px-3" pressed={view === 'recent'} onClick={() => setView('recent')}>
            {t('Gần đây', 'Recent')}
          </Segment>
          <Segment className="px-3" pressed={view === 'kinds'} onClick={() => setView('kinds')}>
            {t('Loại cảnh báo', 'Alert types')}
          </Segment>
        </Segmented>
      </TabHead>
      {props.feedback ? <Feedback fb={props.feedback} onAction={props.onAction} /> : null}
      {banner ? <Banner banner={banner} onAction={props.onAction} /> : null}
      {view === 'recent' ? (
        <Panel aria-label={t('Gần đây', 'Recent')} role="region">
          {days.length ? (
            days.map((d) => (
              <Fragment key={d.label}>
                <div className={cn(GROUP_HEAD, 'border-t-0 pt-1.5 [&:not(:first-child)]:pt-3')}>{d.label}</div>
                {d.cards.map((c) => (
                  <AlertRow key={c.id} card={c.card} pressing={pressing === c.id} onPress={(act) => void press(c.id, act)} />
                ))}
              </Fragment>
            ))
          ) : (
            <Empty>
              <b>{t('Chưa có cảnh báo', 'No alerts yet')}</b>
              {t('Khi có, thẻ hiện ở đây với cùng nút như thông báo macOS.', 'When one comes, its card shows here with the same buttons as the macOS notification.')}
              <LinkButton className="mt-2 block w-full" onClick={() => setView('kinds')}>
                {t('Xem các loại cảnh báo', 'See the alert types')}
              </LinkButton>
            </Empty>
          )}
        </Panel>
      ) : (
        <Panel>
          <CardTitle>{t('Loại cảnh báo', 'Alert types')}</CardTitle>
          <AlertKinds model={model} switches={a.switches} testOff={a.permission === 'denied'} onTest={test} onSwitch={props.onSwitch} />
          <CardText className="mt-2.5 mb-0">
            {t(
              'Không lặp lại cảnh báo cho cùng một sự kiện. Thông báo không có email, không có nút chỉ để mở app.',
              'The same event is never alerted twice. Notifications carry no email and no button that only opens the app.',
            )}
          </CardText>
        </Panel>
      )}
    </>
  );
}
