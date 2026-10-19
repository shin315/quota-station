// The Cảnh báo tab (F09): the recent alerts as the prototype's `.acard`,
// each with the buttons of its macOS card, then the five kinds with "Gửi
// thử" and a switch each. A button pressed here or on the macOS card counts
// once; the card then says what was chosen and how it went.

import { useState } from 'react';
import { Banner, Button, Dot, Feedback, Toggle } from '../components/parts';
import { ALERT_KINDS, alertCard, permissionBanner, type AlertCardView } from '../lib/alerts';
import type { AlertKind, AlertSwitches, Model } from '../lib/model';
import type { Action, FeedbackView, PopoverView } from '../lib/view';

/** The five kinds: what each is, "Gửi thử" and its switch. Also in Cài đặt. */
export function AlertKinds(props: {
  switches: AlertSwitches;
  /** macOS shows no card: nothing to send a test to. */
  testOff: boolean;
  onTest(kind: AlertKind): void;
  onSwitch(kind: AlertKind, on: boolean): void;
}) {
  return (
    <div>
      {ALERT_KINDS.map((k) => (
        <div className="arow" key={k.kind}>
          <div>
            <b style={{ fontWeight: 'var(--fw-semi)' }}>{k.title}</b>
            <small>{k.text}</small>
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

function AlertCard({ card, pressing, onPress }: { card: AlertCardView; pressing: boolean; onPress(action: string): void }) {
  const r = card.result;
  return (
    <div className="acard" role="group" aria-label={'Cảnh báo: ' + card.title}>
      <span className={('ai ' + card.icon.k).trim()} aria-hidden="true">
        {card.icon.mark}
      </span>
      <div>
        <b>{card.title}</b>
        <p>{card.body}</p>
        {card.actions.length ? (
          <div className="acts">
            {card.actions.map((a, i) => (
              <Button key={a.id} primary={i === 0} disabled={pressing} onClick={() => onPress(a.id)}>
                {a.label}
              </Button>
            ))}
          </div>
        ) : null}
        {card.chose ? (
          <div className="ares">
            {card.chose}
            {r && 'working' in r ? (
              <div className="fb info" role="status">
                <span className="spin" aria-hidden="true" />
                <div>
                  <b>{r.working}</b>
                  <small>Chờ tối đa 60 giây.</small>
                </div>
              </div>
            ) : r ? (
              <Feedback fb={r} inline onAction={() => {}} />
            ) : null}
          </div>
        ) : null}
        <div className="meta">{card.meta}</div>
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
      <div className="tb" data-tauri-drag-region="deep">
        <h2>Cảnh báo</h2>
        <small>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </small>
      </div>
      {props.feedback ? <Feedback fb={props.feedback} onAction={props.onAction} /> : null}
      {banner ? <Banner banner={banner} onAction={props.onAction} /> : null}
      <div className="grid2">
        <div className="stack">
          <div className="card">
            <h4>Gần đây</h4>
            {a.recent.length ? (
              <div className="stack">
                {a.recent.map((r) => (
                  <AlertCard key={r.id} card={alertCard(r, now)} pressing={pressing === r.id} onPress={(act) => void press(r.id, act)} />
                ))}
              </div>
            ) : (
              <div className="empty">
                <b>Chưa có cảnh báo</b>Khi có, thẻ hiện ở đây với cùng nút như thông báo macOS.
              </div>
            )}
          </div>
        </div>
        <div className="card">
          <h4>Loại cảnh báo</h4>
          <AlertKinds switches={a.switches} testOff={a.permission === 'denied'} onTest={props.onTest} onSwitch={props.onSwitch} />
          <p style={{ margin: '10px 0 0' }}>Không lặp lại cảnh báo cho cùng một sự kiện. Thông báo không có email, không có nút chỉ để mở app.</p>
        </div>
      </div>
    </>
  );
}
