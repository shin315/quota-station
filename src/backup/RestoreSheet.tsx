// The Sao lưu tab's dialogs (M10): "Xem trong gói", the Khôi phục sheet in
// four steps (pick, preview, keys, restore), the undo of a whole restore,
// and replacing or deleting a key the app keeps. As the approved prototype
// after round 2: plain words, no folder path or command; a key is a
// password field that starts empty and is never shown again.

import { useState } from 'react';
import { Button } from '../components/parts';
import { SHEET_TITLE_ID } from '../components/Sheets';
import {
  changedAgents,
  itemKey,
  itemLine,
  keySlot,
  keysFor,
  macOf,
  packLine,
  packNote,
  packWhen,
  packWhenLower,
  picked,
  resultLine,
  skippedLines,
  todoLines,
  undoLines,
  usable,
} from '../lib/backup';
import { AGENT_NAME, joinCommand } from '../lib/mcp';
import type { McpDef, Model, RestoreItem } from '../lib/model';
import { NAV_ICONS } from '../window/icons';
import type { Backup, BackupDialog, RestoreDialog } from './useBackup';

export function backupDialogSize(d: BackupDialog): 'lg' | 'xl' | 'wide' {
  return d.kind === 'restore' ? 'xl' : 'lg';
}

const STEPS = ['Chọn gói', 'Xem trước', 'Dán khoá', 'Khôi phục'];

function Steps({ at }: { at: number }) {
  return (
    <ol className="stp" aria-label="Các bước">
      {STEPS.map((t, i) => (
        <li key={t} className={i === at ? 'on' : i < at ? 'done' : undefined} aria-current={i === at ? 'step' : undefined}>
          <i>{i < at ? '✓' : i + 1}</i>
          {t}
        </li>
      ))}
    </ol>
  );
}

function Facts({ items }: { items: [string, string][] }) {
  return (
    <ul className="facts">
      {items.map(([k, t]) => (
        <li key={t} className={k || undefined}>
          <span>{t}</span>
        </li>
      ))}
    </ul>
  );
}

function Error({ d }: { d: { error: { title: string; body: string } | null } }) {
  if (!d.error) return null;
  return (
    <p className="fhint bad" role="alert">
      <b>{d.error.title}.</b> {d.error.body}
    </p>
  );
}

function ViewBody({ d, backup, now }: { d: Extract<BackupDialog, { kind: 'view' }>; backup: Backup; now: Date }) {
  const s = d.summary;
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{s ? 'Gói ' + packWhenLower(s.at, now) + ' · ' + s.mac : 'Gói sao lưu'}</h3>
      <Error d={d} />
      {s ? (
        <>
          <div className="sec-t">Trong gói</div>
          <ul className="facts">
            <li className="ok">
              <span>{s.servers.length} MCP, và agent nào đang bật</span>
            </li>
          </ul>
          <div className="sec-t">Khoá cần dán khi khôi phục</div>
          {s.keys.length ? (
            <Facts items={s.keys.map((k) => ['', k.mcp + (k.bitwarden ? ' · ' + k.bitwarden + ' (tên trong Bitwarden)' : '')])} />
          ) : (
            <Facts items={[['', 'Không có']]} />
          )}
          <div className="sec-t">Không có trong gói</div>
          <Facts
            items={[
              ['', 'Khoá, đăng nhập và tài khoản'],
              ['', 'Nhà cung cấp Codex'],
              ['', 'MCP có sẵn của agent, của plugin'],
            ]}
          />
        </>
      ) : !d.error ? (
        <p className="note">Đang đọc gói…</p>
      ) : null}
      <div className="btns">
        <Button large disabled={!s} onClick={() => backup.openRestore(d.id)}>
          Khôi phục gói này…
        </Button>
        <Button large primary onClick={backup.close}>
          Đóng
        </Button>
      </div>
    </>
  );
}

function defLine(def: McpDef): string {
  return def.kind === 'stdio' ? joinCommand(def.command, def.args) : def.url;
}

function pairs(def: McpDef): string {
  const list = def.kind === 'stdio' ? def.env : def.headers;
  return list.map((p) => p.name + ' = ' + p.value).join(', ');
}

function Compare({ item }: { item: RestoreItem }) {
  return (
    <div className="rcmp">
      {item.compare.map((c) => (
        <div key={c.agent} className="rc">
          <b>{AGENT_NAME[c.agent]}</b>
          <span>
            <small>Trên máy</small>
            <code>{defLine(c.here.def)}</code>
            {pairs(c.here.def) ? <code>{pairs(c.here.def)}</code> : null}
          </span>
          <span>
            <small>Trong gói</small>
            <code>{defLine(c.pack.def)}</code>
            {pairs(c.pack.def) ? <code>{pairs(c.pack.def)}</code> : null}
          </span>
        </div>
      ))}
    </div>
  );
}

function PickStep({ d, backup, model, now }: { d: RestoreDialog; backup: Backup; model: Model; now: Date }) {
  const packs = model.backup.packs ?? [];
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Khôi phục</h3>
      <Steps at={0} />
      <p className="lead">Các gói trong iCloud Drive.</p>
      <div className="opts one" role="radiogroup" aria-label="Gói để khôi phục">
        {packs.map((p) => {
          const note = packNote(p);
          const on = d.pack === p.id;
          return (
            <button
              key={p.id}
              type="button"
              role="radio"
              aria-checked={on}
              disabled={!usable(p)}
              onClick={() => backup.update((x) => ({ ...x, pack: p.id, error: null }))}
            >
              <b>{packWhen(p.at, now) + ' · ' + macOf(p, model.machine)}</b>
              <small>{note ?? packLine(p)}</small>
            </button>
          );
        })}
      </div>
      <Error d={d} />
      <div className="btns">
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={!d.pack || backup.working} onClick={() => backup.next(d)}>
          Tiếp
        </Button>
      </div>
    </>
  );
}

function PreviewStep({ d, backup, now }: { d: RestoreDialog; backup: Backup; now: Date }) {
  const plan = d.plan;
  if (!plan) return null;
  const skipped = skippedLines(plan);
  const keys = keysFor(plan, d.ticks);
  const any = picked(plan, d.ticks).length > 0;
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Xem trước: gói {packWhenLower(plan.at, now)}</h3>
      <Steps at={1} />
      <div className="sec-t">
        {NAV_ICONS.mcp}
        MCP
      </div>
      <div className="grpx">
        {plan.items.map((i) => {
          const k = itemKey(i);
          return (
            <div key={k}>
              <label className="ck">
                <input type="checkbox" checked={!!d.ticks[k]} onChange={(e) => backup.update((x) => ({ ...x, ticks: { ...x.ticks, [k]: e.target.checked } }))} />
                <span>
                  <b>{i.name}</b>
                  <small>{itemLine(i)}</small>
                </span>
                <span className="r">
                  {i.missingCommand ? <span className="chip warn">chưa cài</span> : null}
                  {i.kind === 'replace' ? (
                    <button
                      type="button"
                      className="lnk sm"
                      aria-expanded={d.compare === k}
                      onClick={(e) => {
                        e.preventDefault();
                        backup.update((x) => ({ ...x, compare: x.compare === k ? null : k }));
                      }}
                    >
                      So sánh
                    </button>
                  ) : null}
                </span>
              </label>
              {d.compare === k ? <Compare item={i} /> : null}
            </div>
          );
        })}
        {!plan.items.length ? <p className="note" style={{ margin: '6px 0' }}>Máy này đã có đủ các MCP trong gói.</p> : null}
        {plan.same.length ? <p className="note" style={{ margin: '4px 0 2px' }}>Giống trên máy: {plan.same.join(', ')}.</p> : null}
      </div>
      {skipped.length ? <Facts items={skipped.map((t) => ['w', t])} /> : null}
      <Facts
        items={[
          ['ok', 'Không đổi đăng nhập, tài khoản hay nhà cung cấp Codex.'],
          ['ok', 'Lưu lại bản cũ trước khi ghi. Hoàn tác được cả lần khôi phục.'],
        ]}
      />
      <Error d={d} />
      <div className="btns">
        <Button large onClick={() => backup.back(d)}>
          Quay lại
        </Button>
        <span className="grow" />
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        {d.error ? (
          <Button large onClick={() => backup.toPreview(d)}>
            Đọc lại
          </Button>
        ) : null}
        <Button large primary disabled={!any || backup.working} onClick={() => backup.next(d)}>
          {keys.length ? 'Tiếp: dán khoá' : 'Khôi phục'}
        </Button>
      </div>
    </>
  );
}

function KeysStep({ d, backup }: { d: RestoreDialog; backup: Backup }) {
  const plan = d.plan;
  if (!plan) return null;
  const keys = keysFor(plan, d.ticks);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Dán khoá từ Bitwarden</h3>
      <Steps at={2} />
      <p className="lead">{keys.length} khoá, lấy từ Bitwarden. App giữ trên máy này và không hiện lại.</p>
      {keys.map((k) => {
        const id = keySlot(k);
        return (
          <div key={id} className="sec">
            <span>
              <b>{k.mcp}</b>
              <small>{k.bitwarden ? 'Trong Bitwarden: ' + k.bitwarden : k.label}</small>
            </span>
            <span className="in">
              <input
                className="inp"
                type="password"
                autoComplete="off"
                aria-label={'Khoá của ' + k.mcp}
                placeholder="dán vào đây"
                value={d.keys[id] ?? ''}
                onChange={(e) => backup.update((x) => ({ ...x, keys: { ...x.keys, [id]: e.target.value } }))}
              />
            </span>
          </div>
        );
      })}
      <Facts items={[['', 'Để trống cũng được: MCP vẫn được thêm nhưng tắt, dán khoá sau ở tab MCP.']]} />
      <div className="btns">
        <Button large onClick={() => backup.back(d)}>
          Quay lại
        </Button>
        <span className="grow" />
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={backup.working} onClick={() => backup.next(d)}>
          Khôi phục
        </Button>
      </div>
    </>
  );
}

function Progress({ lines }: { lines: [string, string, string][] }) {
  return (
    <ul className="prog">
      {lines.map(([tone, title, sub]) => (
        <li key={title}>
          <span className={'ic' + (tone ? ' ' + tone : '')} aria-hidden="true">
            {tone === 'x' ? '×' : tone === 'w' ? '!' : tone === 'run' ? '' : '✓'}
          </span>
          <span>
            {title}
            {sub ? <small>{sub}</small> : null}
          </span>
        </li>
      ))}
    </ul>
  );
}

function RunStep({ d, backup }: { d: RestoreDialog; backup: Backup }) {
  if (d.step === 'run' || !d.done) {
    const items = d.plan ? picked(d.plan, d.ticks) : [];
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>Đang khôi phục…</h3>
        <Steps at={3} />
        <ul className="prog" aria-busy="true">
          {items.map((i) => (
            <li key={itemKey(i)} className="dim">
              <span className="ic wait" aria-hidden="true" />
              <span>MCP {i.name}</span>
            </li>
          ))}
        </ul>
      </>
    );
  }
  const done = d.done;
  const todo = todoLines(done);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{done.failed ? 'Đã khôi phục một phần' : 'Đã khôi phục'}</h3>
      <Steps at={4} />
      <Progress lines={done.items.map(resultLine)} />
      {todo.length ? (
        <>
          <div className="sec-t">Còn lại bạn làm</div>
          <Facts items={todo.map((t) => ['', t])} />
        </>
      ) : null}
      <div className="btns">
        {done.canUndo ? (
          <Button large onClick={() => backup.setDialog({ kind: 'undo', changed: null, from: d })}>
            Hoàn tác lần khôi phục…
          </Button>
        ) : null}
        <span className="grow" />
        <Button large onClick={backup.toMcp}>
          Xem tab MCP
        </Button>
        <Button large primary onClick={backup.close}>
          Xong
        </Button>
      </div>
    </>
  );
}

function UndoBody({ d, backup, model, now }: { d: Extract<BackupDialog, { kind: 'undo' }>; backup: Backup; model: Model; now: Date }) {
  const r = model.backup.restore;
  const at = r ? new Date(r.at * 1000).toISOString() : null;
  const changed = d.changed ? changedAgents('changed:' + d.changed) : null;
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{at ? 'Hoàn tác lần khôi phục ' + packWhenLower(r?.at ?? null, now) + '?' : 'Hoàn tác lần khôi phục?'}</h3>
      {changed ? (
        <Facts
          items={[
            ['w', 'File của ' + changed + ' đã sửa sau lần khôi phục.'],
            ['', 'Hoàn tác vẫn chỉ bỏ các MCP lần khôi phục đã thêm hay thay; phần bạn sửa thêm vẫn giữ.'],
          ]}
        />
      ) : (
        <Facts items={undoLines()} />
      )}
      <div className="btns">
        <Button large onClick={() => (d.from ? backup.setDialog(d.from) : backup.close())}>
          Huỷ
        </Button>
        <Button large danger disabled={backup.working} onClick={() => backup.undo(!!changed, d.from)}>
          {changed ? 'Vẫn hoàn tác' : 'Hoàn tác'}
        </Button>
      </div>
    </>
  );
}

function KeyBody({ d, backup }: { d: Extract<BackupDialog, { kind: 'key' }>; backup: Backup }) {
  const [value, setValue] = useState('');
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Thay khoá</h3>
      <div className="sec">
        <span>
          <b>{d.key.mcp}</b>
          <small>{d.key.bitwarden ? 'Trong Bitwarden: ' + d.key.bitwarden : d.key.label}</small>
        </span>
        <span className="in">
          <input className="inp" type="password" autoComplete="off" aria-label="Khoá mới" placeholder="dán khoá mới" value={value} onChange={(e) => setValue(e.target.value)} />
        </span>
      </div>
      <Facts items={[['ok', 'App đưa khoá mới vào các agent đang dùng MCP này, có xem trước và lưu bản cũ.']]} />
      <div className="btns">
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={backup.working || !value.trim()} onClick={() => backup.replaceKey(d.key, value)}>
          Lưu
        </Button>
      </div>
    </>
  );
}

function DelKeyBody({ d, backup }: { d: Extract<BackupDialog, { kind: 'delKey' }>; backup: Backup }) {
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Xoá khoá {d.key.mcp}?</h3>
      <Facts
        items={[
          ['', 'Khoá chỉ bị xoá khỏi máy này. Agent đang dùng ' + d.key.mcp + ' vẫn giữ bản của nó.'],
          ['', 'Lần khôi phục sau trên máy này, app sẽ hỏi lại khoá này.'],
        ]}
      />
      <div className="btns">
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large danger disabled={backup.working} onClick={() => backup.deleteKey(d.key)}>
          Xoá khoá
        </Button>
      </div>
    </>
  );
}

export function BackupDialogBody({ backup, model, now }: { backup: Backup; model: Model; now: Date }) {
  const d = backup.dialog;
  if (!d) return null;
  switch (d.kind) {
    case 'view':
      return <ViewBody d={d} backup={backup} now={now} />;
    case 'undo':
      return <UndoBody d={d} backup={backup} model={model} now={now} />;
    case 'key':
      return <KeyBody key={d.key.mcp + d.key.slot} d={d} backup={backup} />;
    case 'delKey':
      return <DelKeyBody d={d} backup={backup} />;
    case 'restore':
      if (d.step === 'pick') return <PickStep d={d} backup={backup} model={model} now={now} />;
      if (d.step === 'prev') return <PreviewStep d={d} backup={backup} now={now} />;
      if (d.step === 'keys') return <KeysStep d={d} backup={backup} />;
      return <RunStep d={d} backup={backup} />;
  }
}

