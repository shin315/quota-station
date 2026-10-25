// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Firstmate tab (F10): whether Firstmate holds new Claude work, from the
// line its quota-pool watch writes (`quota-pool.last`, at the path set in
// Cài đặt), next to the pool the app counts. Read only. The status on top,
// then three cards side by side, one question each: what Firstmate sees,
// each account's share used, and what its quota-axi can read; the note on
// when each side read under them.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { GRID3, TabHead, TabSub, TabTitle } from '../components/page';
import { Button, Chip, Dot } from '../components/parts';
import { firstmateTab, type FirstmateRow } from '../lib/alerts';
import type { AuthorModel } from '../lib/author';
import { t } from '../lib/i18n';
import type { Action, PopoverView } from '../lib/view';

export interface FirstmateTabProps {
  model: AuthorModel;
  fresh: PopoverView['fresh'];
  now: Date;
  onAction(act: Action): void;
}

/** A card's lines: the name on the left, the value on the right, a hairline between. */
function Rows({ rows, label }: { rows: FirstmateRow[]; label: string }) {
  return (
    <div className="fmrows grid" role="list" aria-label={label}>
      {rows.map((r) => (
        <div
          key={r.key}
          role="listitem"
          className="flex min-h-[29px] items-center justify-between gap-3 border-t-[.5px] border-card-line py-[5px] text-sm first:border-t-0"
        >
          <span className="min-w-0 text-ink2">{r.key}</span>
          <span className={cn('text-right tabular-nums', r.tone === 'bad' ? 'font-semi text-bad-ink' : undefined)}>{r.chip ? <Chip chip={r.chip} /> : r.value}</span>
        </div>
      ))}
    </div>
  );
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
          {v.setUp ? <Button onClick={() => onAction('settings')}>{t('Mở Cài đặt', 'Open Settings')}</Button> : null}
        </div>
      </Panel>
      {v.seen.length ? (
        <>
          <div className={GRID3}>
            <Panel>
              <CardTitle>{t('Firstmate đang thấy', 'What Firstmate sees')}</CardTitle>
              <Rows rows={v.seen} label={t('Firstmate đang thấy', 'What Firstmate sees')} />
            </Panel>
            <Panel>
              <CardTitle>{t('Đã dùng từng tài khoản', 'Used per account')}</CardTitle>
              <Rows rows={v.used.length ? v.used : [{ key: t('Chưa có tài khoản', 'No accounts yet'), value: '—' }]} label={t('Đã dùng từng tài khoản', 'Used per account')} />
            </Panel>
            <Panel>
              <CardTitle>{t('Kết nối', 'Connection')}</CardTitle>
              <Rows rows={v.links} label={t('Kết nối', 'Connection')} />
            </Panel>
          </div>
          {v.note ? <CardText className="mx-0.5 mt-0 mb-0">{v.note}</CardText> : null}
        </>
      ) : null}
    </>
  );
}
