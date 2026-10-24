// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Cảnh báo tab (F09): the recent alerts as the prototype's `.acard`,
// each with the buttons of its macOS card, then the five kinds with "Gửi
// thử" and a switch each. A button pressed here or on the macOS card counts
// once; the card then says what was chosen and how it went.

import { useState } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { Empty, GRID2, LIST_ROW, STACK, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Dot, Feedback, FeedbackLine, FeedbackText, Spinner, Toggle } from '../components/parts';
import { alertCard, alertKinds, permissionBanner, type AlertCardView } from '../lib/alerts';
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
          <Button disabled={props.testOff} label={'Gửi thử: ' + k.title} onClick={() => props.onTest(k.kind)}>
            Gửi thử
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

function AlertCard({ card, pressing, onPress }: { card: AlertCardView; pressing: boolean; onPress(action: string): void }) {
  const r = card.result;
  return (
    <div
      className={cn(
        'grid grid-cols-[30px_minmax(0,1fr)] gap-2.5 rounded-sheet bg-card px-3.5 py-3 shadow-[inset_0_0_0_.5px_var(--card-line),0_2px_8px_rgba(0,0,0,.06)] [&_b]:block [&_b]:text-base',
        'in-[.app-window]:bg-fill in-[.app-window]:shadow-none',
      )}
      role="group"
      aria-label={'Cảnh báo: ' + card.title}
    >
      <span className={cn('grid size-[30px] place-items-center rounded-[8px] font-black', MARK[card.icon.k] ?? 'bg-warn/22 text-warn-ink')} aria-hidden="true">
        {card.icon.mark}
      </span>
      <div>
        <b>{card.title}</b>
        <p className="mt-0.5 mb-2 text-sm text-ink2">{card.body}</p>
        {card.actions.length ? (
          <div className="flex flex-wrap gap-1.5">
            {card.actions.map((a, i) => (
              <Button key={a.id} primary={i === 0} disabled={pressing} onClick={() => onPress(a.id)}>
                {a.label}
              </Button>
            ))}
          </div>
        ) : null}
        {card.chose ? (
          <div className="mt-0.5 text-sm text-ink2">
            {card.chose}
            {r && 'working' in r ? (
              <FeedbackLine tone="info" style={{ margin: '6px 0 0' }}>
                <Spinner />
                <div>
                  <FeedbackText title={r.working} body="Chờ tối đa 60 giây." />
                </div>
              </FeedbackLine>
            ) : r ? (
              <Feedback fb={r} inline onAction={() => {}} style={{ margin: '6px 0 0' }} />
            ) : null}
          </div>
        ) : null}
        <div className="mt-2 text-xs text-ink3 tabular-nums">{card.meta}</div>
      </div>
    </div>
  );
}

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
  const banner = permissionBanner(a.permission);
  const press = async (id: string, action: string) => {
    setPressing(id);
    try {
      await props.onPress(id, action);
    } finally {
      setPressing(null);
    }
  };
  return (
    <>
      <TabHead>
        <TabTitle>Cảnh báo</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
      </TabHead>
      {props.feedback ? <Feedback fb={props.feedback} onAction={props.onAction} /> : null}
      {banner ? <Banner banner={banner} onAction={props.onAction} /> : null}
      <div className={GRID2}>
        <div className={STACK}>
          <Panel bare={a.recent.length > 0}>
            <CardTitle bare={a.recent.length > 0}>Gần đây</CardTitle>
            {a.recent.length ? (
              <div className={STACK}>
                {a.recent.map((r) => (
                  <AlertCard key={r.id} card={alertCard(r, now)} pressing={pressing === r.id} onPress={(act) => void press(r.id, act)} />
                ))}
              </div>
            ) : (
              <Empty>
                <b>Chưa có cảnh báo</b>Khi có, thẻ hiện ở đây với cùng nút như thông báo macOS.
              </Empty>
            )}
          </Panel>
        </div>
        <Panel>
          <CardTitle>Loại cảnh báo</CardTitle>
          <AlertKinds model={model} switches={a.switches} testOff={a.permission === 'denied'} onTest={props.onTest} onSwitch={props.onSwitch} />
          <CardText className="mt-2.5 mb-0">Không lặp lại cảnh báo cho cùng một sự kiện. Thông báo không có email, không có nút chỉ để mở app.</CardText>
        </Panel>
      </div>
    </>
  );
}
