// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Firstmate tab (F10): whether Firstmate holds new Claude work, from the
// line its quota-pool watch writes (`quota-pool.last`, at the path set in
// Cài đặt), next to the pool the app counts. Read only.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { TabHead, TabSub, TabTitle } from '../components/page';
import { Button, Chip, Dot, KVT } from '../components/parts';
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
      <TabHead>
        <TabTitle>Firstmate</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
      </TabHead>
      <Panel>
        <div className="flex items-center justify-between gap-2.5">
          <div role="status">
            <b className="text-sheet">
              <Dot tone={v.tone} className="mr-2" />
              {v.title}
            </b>
            <CardText className="mt-1 mb-0">{v.body}</CardText>
          </div>
          {v.setUp ? <Button onClick={() => onAction('settings')}>Mở Cài đặt</Button> : null}
        </div>
      </Panel>
      {v.rows.length ? (
        <Panel>
          <CardTitle>Firstmate đang thấy</CardTitle>
          <div className={KVT}>
            {v.rows.map((r) => [
              <span key={r.key + ':k'}>{r.key}</span>,
              <span key={r.key + ':v'}>{r.chip ? <Chip chip={r.chip} /> : r.value}</span>,
            ])}
          </div>
          {v.note ? <CardText className="mt-2.5 mb-0">{v.note}</CardText> : null}
        </Panel>
      ) : null}
    </>
  );
}
