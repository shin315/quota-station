// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Lịch sử tab (F08): every switch on this Mac, newest first, a day at a
// time, from the app's journal, the switcher's log and what the readings
// showed. The list is read from the Rust side when the tab opens and again
// after each read while it is open. One card: the app filter at its top,
// then a line per switch in fixed columns: time, tag, the accounts, a short
// reason on one line (all of it on hover, and on focus once it is cut), and
// where it came from; the switcher's own line on hover.

import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { Badge, badgeTone } from '@/components/ui/badge';
import { Card as Panel } from '@/components/ui/card';
import { Segment, Segmented } from '@/components/ui/segmented';
import { cn } from '@/lib/utils';
import { Empty, GROUP_HEAD, TabHead, TabSub, TabTitle } from '../components/page';
import { Dot } from '../components/parts';
import { historyDays, historyFilters, type HistoryFilter } from '../lib/history';
import { mac, t } from '../lib/i18n';
import type { HistoryEntry, Model } from '../lib/model';
import type { PopoverView } from '../lib/view';

export interface HistoryTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  now: Date;
  load(): Promise<HistoryEntry[]>;
}

/** The history's tags by kind: a switch is blue, a warning amber, a service its colour. */
const PILL_TONE: Record<string, 'info' | 'warn'> = { sw: 'info', w: 'warn' };

/**
 * A line of a day: time, tag, accounts, reason, source, in the columns every
 * line shares. Where the page is narrow the reason goes under the rest.
 */
const ROW =
  'hrow grid grid-cols-[44px_96px_26%_minmax(0,1fr)_72px] items-center gap-3 border-t-[.5px] border-card-line py-[7px] text-sm @max-[640px]/main:grid-cols-[44px_auto_minmax(0,1fr)_auto] @max-[640px]/main:gap-x-2.5 @max-[640px]/main:gap-y-0.5';
const NARROW_WHY = '@max-[640px]/main:col-span-3 @max-[640px]/main:col-start-2 @max-[640px]/main:row-start-2';
const NARROW_SOURCE = '@max-[640px]/main:col-start-4 @max-[640px]/main:row-start-1';

/**
 * The reason, on one line: the whole of it on hover, and when the line cuts
 * it, the keyboard can reach it and it opens in full while focused.
 */
function Reason({ text, className }: { text: string; className?: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  const [cut, setCut] = useState(false);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const check = () => setCut(el.scrollWidth > el.clientWidth + 1);
    check();
    const ro = new ResizeObserver(check);
    ro.observe(el);
    return () => ro.disconnect();
  }, [text]);
  return (
    <span
      ref={ref}
      className={cn(
        'min-w-0 truncate rounded-sm text-ink2 focus-visible:whitespace-normal focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent-ink @max-[640px]/main:whitespace-normal',
        className,
      )}
      title={text}
      tabIndex={cut ? 0 : undefined}
    >
      {text}
    </span>
  );
}

export function HistoryTab({ model, fresh, now, load }: HistoryTabProps) {
  const [filter, setFilter] = useState<HistoryFilter>('all');
  const [entries, setEntries] = useState<HistoryEntry[] | 'failed' | null>(null);
  // A new read or a switch may have added a line.
  const reads = model.reads;
  const switching = model.claudeSwitch.busy != null || model.agySwitch?.busy != null || model.codex.busy != null;
  const loaded = entries !== null;
  useEffect(() => {
    // Read once on opening; after that, not while a switch is half done.
    if (switching && loaded) return;
    let live = true;
    load().then(
      (e) => live && setEntries(e),
      () => live && setEntries((cur) => (Array.isArray(cur) ? cur : 'failed')),
    );
    return () => {
      live = false;
    };
  }, [load, reads, switching, loaded]);

  const days = Array.isArray(entries) ? historyDays(entries, filter, model, now) : [];
  const filters = historyFilters();
  const filterName = filters.find((f) => f.v === filter)?.t;
  return (
    <>
      <TabHead>
        <TabTitle>{t('Lịch sử', 'History')}</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
      </TabHead>
      <Panel>
        <Segmented variant="inline" className="mb-1.5" aria-label={t('Lọc lịch sử', 'Filter history')}>
          {filters.map((f) => (
            <Segment key={f.v} className="px-3" pressed={filter === f.v} onClick={() => setFilter(f.v)}>
              {f.t}
            </Segment>
          ))}
        </Segmented>
        {entries === null ? null : entries === 'failed' ? (
          <Empty>
            <b>{t('Chưa đọc được lịch sử', 'Could not read the history')}</b>
            {t('App thử lại sau lần đọc số tới.', 'The app tries again after its next read.')}
          </Empty>
        ) : !entries.length ? (
          <Empty>
            <b>{t('Chưa có lần chuyển nào', 'No switches yet')}</b>
            {mac(
              t(
                'Lịch sử của máy này bắt đầu từ khi cài app. Mỗi lần chuyển tay, switcher chuyển, hay app thấy tài khoản đổi sẽ hiện ở đây.',
                'This Mac’s history starts when the app is installed. Every manual switch, switcher switch, or account change the app notices shows up here.',
              ),
            )}
          </Empty>
        ) : !days.length ? (
          <Empty>
            <b>{t('Chưa có lần chuyển ' + filterName + ' nào', 'No ' + filterName + ' switches yet')}</b>
            {t('Chọn Tất cả để xem các lần chuyển khác.', 'Pick All to see the other switches.')}
          </Empty>
        ) : (
          <>
            {days.map((d) => (
              <section key={d.label} aria-label={d.label} className="mt-1.5">
                <div className={GROUP_HEAD}>{d.label}</div>
                {d.rows.map((r) => (
                  <div className={ROW} key={r.key} title={r.raw ?? undefined}>
                    <time className="text-ink2 tabular-nums">{r.time}</time>
                    <span>
                      <Badge shape="pill" tone={PILL_TONE[r.pill.k] ?? badgeTone(r.pill.k)}>
                        {r.pill.t}
                      </Badge>
                    </span>
                    <span className="flex min-w-0 items-center gap-1.5">
                      {r.from ? (
                        <>
                          <span className="min-w-0 truncate rounded-md bg-fill px-1.5 py-px text-s" title={r.from}>
                            {r.from}
                          </span>
                          <span className="flex-none text-ink3" role="img" aria-label={t('sang', 'to')}>
                            →
                          </span>
                        </>
                      ) : null}
                      <span className="min-w-0 truncate rounded-md bg-ok/15 px-1.5 py-px text-s font-semi text-ok-ink" title={r.to}>
                        {r.to}
                      </span>
                    </span>
                    <Reason text={r.why} className={NARROW_WHY} />
                    <span className={cn('justify-self-end whitespace-nowrap text-xs text-ink3', NARROW_SOURCE)}>{r.source.t}</span>
                  </div>
                ))}
              </section>
            ))}
          </>
        )}
      </Panel>
    </>
  );
}
