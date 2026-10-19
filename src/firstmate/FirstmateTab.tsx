// The Firstmate tab (F10): whether Firstmate holds new Claude work, from the
// line its quota-pool watch writes (`quota-pool.last`, at the path set in
// Cài đặt), next to the pool quota-axi reports. Read only.

import { Button, Chip, Dot } from '../components/parts';
import { firstmateTab } from '../lib/alerts';
import type { Model } from '../lib/model';
import type { Action, PopoverView } from '../lib/view';

export interface FirstmateTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  now: Date;
  onAction(act: Action): void;
}

export function FirstmateTab({ model, fresh, now, onAction }: FirstmateTabProps) {
  const v = firstmateTab(model, now);
  return (
    <>
      <div className="tb" data-tauri-drag-region="deep">
        <h2>Firstmate</h2>
        <small>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </small>
      </div>
      <div className="card">
        <div className="row2">
          <div role="status">
            <b style={{ fontSize: 'var(--fs-sheet)' }}>
              <span className={'dot ' + v.tone} aria-hidden="true" style={{ marginRight: 8 }} />
              {v.title}
            </b>
            <p style={{ margin: '4px 0 0' }}>{v.body}</p>
          </div>
          {v.setUp ? <Button onClick={() => onAction('settings')}>Mở Cài đặt</Button> : null}
        </div>
      </div>
      {v.rows.length ? (
        <div className="card">
          <h4>Firstmate đang thấy</h4>
          <div className="kvt">
            {v.rows.map((r) => [
              <span key={r.key + ':k'}>{r.key}</span>,
              <span key={r.key + ':v'}>{r.chip ? <Chip chip={r.chip} /> : r.value}</span>,
            ])}
          </div>
          {v.note ? <p style={{ margin: '10px 0 0' }}>{v.note}</p> : null}
        </div>
      ) : null}
    </>
  );
}
