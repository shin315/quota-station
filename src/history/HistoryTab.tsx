// The Lịch sử tab (F08): every switch on this Mac, newest first, a day at a
// time, from the app's journal, the switcher's log and what the readings
// showed. The list is read from the Rust side when the tab opens and again
// after each read while it is open. Each row: time, tag, the accounts and a
// short reason, and where it came from; the switcher's own line on hover.

import { useEffect, useState } from 'react';
import { Badge, badgeTone } from '@/components/ui/badge';
import { Card as Panel } from '@/components/ui/card';
import { Segment, Segmented } from '@/components/ui/segmented';
import { Empty, TabHead, TabSub, TabTitle } from '../components/page';
import { Dot } from '../components/parts';
import { HISTORY_FILTERS, historyDays, type HistoryFilter } from '../lib/history';
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

export function HistoryTab({ model, fresh, now, load }: HistoryTabProps) {
  const [filter, setFilter] = useState<HistoryFilter>('all');
  const [entries, setEntries] = useState<HistoryEntry[] | 'failed' | null>(null);
  // A new read or a switch may have added a line.
  const reads = model.reads;
  const switching = model.claudeSwitch.busy != null || model.agySwitch.busy != null || model.codex.busy != null;
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
  return (
    <>
      <TabHead>
        <TabTitle>Lịch sử</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
        <Segmented variant="inline" aria-label="Lọc lịch sử">
          {HISTORY_FILTERS.map((f) => (
            <Segment key={f.v} pressed={filter === f.v} onClick={() => setFilter(f.v)}>
              {f.t}
            </Segment>
          ))}
        </Segmented>
      </TabHead>
      <Panel>
        {entries === null ? null : entries === 'failed' ? (
          <Empty>
            <b>Chưa đọc được lịch sử</b>App thử lại sau lần đọc số tới.
          </Empty>
        ) : !entries.length ? (
          <Empty>
            <b>Chưa có lần chuyển nào</b>Lịch sử của máy này bắt đầu từ khi cài app. Mỗi lần chuyển tay, switcher chuyển, hay app thấy tài khoản
            đổi sẽ hiện ở đây.
          </Empty>
        ) : !days.length ? (
          <Empty>
            <b>Chưa có lần chuyển {HISTORY_FILTERS.find((f) => f.v === filter)?.t} nào</b>Chọn Tất cả để xem các lần chuyển khác.
          </Empty>
        ) : (
          <>
            {days.map((d) => (
              <section key={d.label} aria-label={d.label}>
                <div className="pt-2.5 pb-1 text-xs font-bold text-ink2">{d.label}</div>
                {d.rows.map((r) => (
                  <div
                    className="hrow grid grid-cols-[48px_92px_minmax(0,1fr)_auto] items-start gap-2.5 border-t-[.5px] border-card-line py-2 text-sm"
                    key={r.key}
                    title={r.raw ?? undefined}
                  >
                    <time className="pt-px text-ink2 tabular-nums">{r.time}</time>
                    <span>
                      <Badge shape="pill" tone={PILL_TONE[r.pill.k] ?? badgeTone(r.pill.k)}>
                        {r.pill.t}
                      </Badge>
                    </span>
                    <div>
                      <div className="flex flex-wrap items-center gap-1.5">
                        {r.from ? (
                          <>
                            <span className="rounded-md bg-fill px-1.5 py-px text-s">{r.from}</span>
                            <span className="text-ink3" role="img" aria-label="sang">
                              →
                            </span>
                          </>
                        ) : null}
                        <span className="rounded-md bg-ok/15 px-1.5 py-px text-s font-semi text-ok-ink">{r.to}</span>
                      </div>
                      <small className="mt-0.5 block text-ink2">{r.why}</small>
                    </div>
                    <span className="whitespace-nowrap pt-0.5 text-cap text-ink3">{r.source.t}</span>
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
