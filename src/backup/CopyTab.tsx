// The Chép sang Mac khác tab of the main window (M13, D), as the approved
// prototype's `cp` screens: pick one of Cài đặt › Các Mac khác and the way
// (from this Mac, or to it), "So sánh", then the items that differ by group,
// each with its checkbox. What only the sending Mac has is ticked to begin
// with; what both have but not alike, the owner ticks. "Gửi cả khoá qua SSH"
// starts off. Frosted panels on the window's glass, no panel inside one.

import { useRef, type KeyboardEvent } from 'react';
import { Banner, Button, Chip, Dot, Feedback, Toggle } from '../components/parts';
import { GROUPS, hasView, itemKey, itemName, itemNote, macChip, readAt, sameLine, selectable, stateChip, wrongWay } from '../lib/copy';
import type { CopyItem, Model } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';
import { NAV_ICONS, SYNC_BIG } from '../window/icons';
import type { Copy } from './useCopy';

function MacList({ copy }: { copy: Copy }) {
  const group = useRef<HTMLDivElement>(null);
  const move = (e: KeyboardEvent, i: number) => {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const n = copy.peers.length;
    const next = (i + step + n) % n;
    copy.choose(copy.peers[next].alias);
    group.current?.querySelectorAll<HTMLButtonElement>('[role="radio"]')[next]?.focus();
  };
  return (
    <div ref={group} className="opts" role="radiogroup" aria-label="Mac">
      {copy.peers.map((p, i) => {
        const on = p.alias === copy.mac;
        const chip = macChip(copy.answers[p.alias]);
        return (
          <button
            key={p.alias}
            type="button"
            role="radio"
            aria-checked={on}
            tabIndex={on ? 0 : -1}
            disabled={copy.working || copy.phase === 'reading'}
            onClick={() => copy.choose(p.alias)}
            onKeyDown={(e) => move(e, i)}
          >
            <b>
              {p.name || p.alias} {chip ? <Chip chip={chip} /> : null}
            </b>
            <small>tên kết nối {p.alias}</small>
          </button>
        );
      })}
    </div>
  );
}

function ItemRow({ i, copy }: { i: CopyItem; copy: Copy }) {
  const k = itemKey(i);
  const wrong = wrongWay(i, copy.push);
  const can = selectable(i, copy.push);
  const name = itemName(i);
  return (
    <label className={'ck' + (can ? '' : ' off')}>
      <input type="checkbox" checked={can && !!copy.ticks[k]} disabled={!can || copy.working} onChange={(e) => copy.tick(k, e.target.checked)} />
      <span>
        <b>{name}</b>
        <small>{itemNote(i, copy.push, copy.here, copy.other, copy.keys)}</small>
      </span>
      <span className="r">
        <Chip chip={stateChip(i, copy.other)} />
        {hasView(i) && !wrong ? (
          <button
            type="button"
            className="lnk sm"
            aria-label={'Xem ' + name}
            onClick={(e) => {
              e.preventDefault();
              copy.openItem(k);
            }}
          >
            Xem
          </button>
        ) : null}
      </span>
    </label>
  );
}

function DiffList({ copy }: { copy: Copy }) {
  const c = copy.cmp;
  if (!c) return null;
  const arrow = copy.push ? ' → ' : ' ← ';
  return (
    <div className="card">
      <h4>
        Khác nhau · {copy.here}
        {arrow}
        {copy.other} <span className="cnt">{readAt(c)}</span>
      </h4>
      {c.diff.items.length ? (
        GROUPS.map(({ group, title }) => {
          const list = c.diff.items.filter((i) => i.group === group);
          if (!list.length) return null;
          return (
            <div key={group} className="cpg" role="group" aria-label={title}>
              <div className="sec-t">
                {NAV_ICONS[group === 'skill' ? 'skills' : group === 'plugin' ? 'plugins' : group === 'file' ? 'instr' : 'mcp']}
                {title}
              </div>
              {list.map((i) => (
                <ItemRow key={itemKey(i)} i={i} copy={copy} />
              ))}
            </div>
          );
        })
      ) : (
        <p className="note">Hai máy giống nhau: không có gì để chép.</p>
      )}
      <p className="note">
        {sameLine(c)}{' '}
        {c.diff.same.length ? (
          <button type="button" className="lnk sm" aria-expanded={copy.showSame} onClick={() => copy.setShowSame(!copy.showSame)}>
            {copy.showSame ? 'Ẩn' : 'Hiện'}
          </button>
        ) : null}
      </p>
      {copy.showSame ? <p className="note">{c.diff.same.join(', ')}</p> : null}
    </div>
  );
}

export interface CopyTabProps {
  model: Model;
  feedback: FeedbackView | null;
  copy: Copy;
  onAction(act: Action): void;
}

export function CopyTab({ model, feedback, copy, onAction }: CopyTabProps) {
  const ssh = model.rotation.ssh;
  const act = (a: Action) => (a === 'copy:compare' ? copy.compare() : onAction(a));
  const top = (
    <div className="tb" data-tauri-drag-region>
      <h2>Chép sang Mac khác</h2>
      <small>
        <Dot tone={ssh ? 'ok' : 'stale'} />
        {model.machine} · qua SSH
      </small>
    </div>
  );
  if (!copy.peers.length)
    return (
      <>
        {top}
        <div className="card">
          <div className="empty">
            <span className="ei" aria-hidden="true">
              {SYNC_BIG}
            </span>
            <b>Chưa có Mac nào để chép</b>
            <p className="note">Thêm Mac trong Cài đặt › Các Mac khác, bằng tên kết nối SSH của nó.</p>
            <div className="btns">
              <Button primary onClick={() => onAction('settings')}>
                Mở Cài đặt
              </Button>
            </div>
          </div>
        </div>
      </>
    );
  const reading = copy.phase === 'reading';
  const n = copy.items.length;
  const err = copy.error;
  const unreach = copy.errorKind === 'unreach';
  return (
    <>
      {top}
      {!ssh ? <Banner banner={{ tone: 'bad', title: 'Không thấy ssh trên máy này', body: 'App dùng ssh của macOS để nói chuyện với Mac khác.', actions: [] }} onAction={act} /> : null}
      <div className="card">
        <h4>Mac</h4>
        <MacList copy={copy} />
        <div className="cprow">
          <div className="seg inline" role="group" aria-label="Chiều chép">
            <button type="button" aria-pressed={copy.push} disabled={copy.working || reading} onClick={() => copy.way(true)}>
              Từ máy này sang {copy.other}
            </button>
            <button type="button" aria-pressed={!copy.push} disabled={copy.working || reading} onClick={() => copy.way(false)}>
              Lấy từ {copy.other} về
            </button>
          </div>
          <Button primary disabled={!ssh || reading || copy.working || !copy.mac} onClick={copy.compare}>
            {copy.phase === 'idle' ? 'So sánh' : 'So sánh lại'}
          </Button>
        </div>
      </div>
      {feedback ? <Feedback fb={feedback} onAction={act} /> : null}
      {copy.phase === 'error' && err ? (
        <Banner
          banner={{
            tone: err.tone === 'bad' ? 'bad' : err.tone === 'warn' ? 'warn' : 'info',
            title: err.title,
            body: err.body,
            actions: unreach
              ? [
                  { label: 'Thử lại', act: 'copy:compare' },
                  { label: 'Mở Cài đặt', act: 'settings' },
                ]
              : [{ label: 'So sánh lại', act: 'copy:compare' }],
          }}
          onAction={act}
        />
      ) : null}
      {reading ? (
        <div className="card">
          <ul className="prog" aria-busy="true">
            <li>
              <span className="ic" aria-hidden="true">
                ✓
              </span>
              <span>Kết nối {copy.mac}</span>
            </li>
            <li>
              <span className="ic run" aria-hidden="true" />
              <span>
                Đọc MCP, skills, plugin và file hướng dẫn trên {copy.other}
                <small>chỉ đọc tên, cấu hình và mã băm; không đọc khoá</small>
              </span>
            </li>
            <li className="dim">
              <span className="ic wait" aria-hidden="true" />
              <span>So với máy này</span>
            </li>
          </ul>
        </div>
      ) : null}
      {copy.phase === 'idle' ? (
        <div className="card">
          <p className="note" style={{ margin: 0 }}>
            Chọn Mac và bấm So sánh. App chỉ đọc, chưa ghi gì.
          </p>
        </div>
      ) : null}
      {copy.phase === 'diff' && copy.cmp ? (
        <div className="bkg">
          <div className="stack">
            <DiffList copy={copy} />
          </div>
          <div className="stack">
            <div className="card">
              <h4>Khoá</h4>
              <div className="set">
                <span>Gửi cả khoá qua SSH</span>
                <Toggle on={copy.keys} label="Gửi cả khoá qua SSH" disabled={copy.working} onClick={() => copy.setKeys(!copy.keys)} />
              </div>
              <p className="note">
                {copy.keys
                  ? 'Khoá đi trong kết nối SSH và vào thẳng Keychain của ' + (copy.push ? copy.other : 'máy này') + '; không nằm trên đĩa ở chỗ nào khác.'
                  : (copy.push ? copy.other : 'Máy này') + ' sẽ hỏi dán khoá khi mở Quota Station.'}
              </p>
            </div>
            <div className="card">
              <p className="note" style={{ margin: '0 0 8px' }}>
                Chỉ chép mục đã chọn. Không ghi đè cả gói, không bên nào tự thắng.
              </p>
              <Button primary large disabled={!n || copy.working} onClick={copy.openGo}>
                {n ? 'Chép ' + n + ' mục' + (copy.push ? ' sang ' + copy.other : ' về máy này') + '…' : 'Chưa chọn mục nào'}
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
