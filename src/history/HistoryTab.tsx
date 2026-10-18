// The Lịch sử tab (F08): every switch on this Mac, newest first, a day at a
// time, from the app's journal, the switcher's log and what the readings
// showed. The list is read from the Rust side when the tab opens and again
// after each read while it is open.

import { useEffect, useState } from 'react';
import { Chip, Dot } from '../components/parts';
import { HISTORY_FILTERS, historyDays, type HistoryFilter } from '../lib/history';
import type { HistoryEntry, Model } from '../lib/model';
import type { PopoverView } from '../lib/view';

export interface HistoryTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  now: Date;
  load(): Promise<HistoryEntry[]>;
}

export function HistoryTab({ model, fresh, now, load }: HistoryTabProps) {
  const [filter, setFilter] = useState<HistoryFilter>('all');
  const [entries, setEntries] = useState<HistoryEntry[] | 'failed' | null>(null);
  // A new read or a switch may have added a line.
  const reads = model.reads;
  const switching = model.claudeSwitch.busy != null || model.agySwitch.busy != null;
  useEffect(() => {
    if (switching) return;
    let live = true;
    load().then(
      (e) => live && setEntries(e),
      () => live && setEntries((cur) => (Array.isArray(cur) ? cur : 'failed')),
    );
    return () => {
      live = false;
    };
  }, [load, reads, switching]);

  const days = Array.isArray(entries) ? historyDays(entries, filter, model, now) : [];
  return (
    <>
      <div className="tb" data-tauri-drag-region>
        <h2>Lịch sử</h2>
        <small>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </small>
        <div className="seg inline" role="group" aria-label="Lọc lịch sử">
          {HISTORY_FILTERS.map((f) => (
            <button key={f.v} type="button" aria-pressed={filter === f.v} onClick={() => setFilter(f.v)}>
              {f.t}
            </button>
          ))}
        </div>
      </div>
      <div className="card">
        {entries === null ? null : entries === 'failed' ? (
          <div className="empty">
            <b>Chưa đọc được lịch sử</b>App thử lại sau lần đọc số tới.
          </div>
        ) : !days.length ? (
          <div className="empty">
            <b>Chưa có lần chuyển nào</b>Lịch sử của máy này bắt đầu từ khi cài app. Mỗi lần chuyển tay, switcher chuyển, hay app thấy tài khoản
            đổi sẽ hiện ở đây.
          </div>
        ) : (
          <>
            <p style={{ margin: 0 }}>
              Lịch sử của {model.machine}. Nguồn: app (bạn bấm), switcher (dòng log), quan sát (tài khoản đổi giữa hai lần đọc).
            </p>
            {days.map((d) => (
              <section key={d.label} aria-label={d.label}>
                <div className="day">{d.label}</div>
                {d.rows.map((r) => (
                  <div className="hrow" key={r.key} title={r.raw ?? undefined}>
                    <time>{r.time}</time>
                    <span>
                      <span className={('pill ' + r.pill.k).trim()}>{r.pill.t}</span>
                    </span>
                    <div>
                      <div className="what">
                        {r.from ? (
                          <>
                            <span className="from">{r.from}</span>
                            <span className="arrow" aria-label="sang">
                              →
                            </span>
                          </>
                        ) : null}
                        <span className="to">{r.to}</span>
                      </div>
                      <small>{r.why}</small>
                    </div>
                    <span>
                      <Chip chip={r.source} />
                    </span>
                  </div>
                ))}
              </section>
            ))}
          </>
        )}
      </div>
    </>
  );
}
