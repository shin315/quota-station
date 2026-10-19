// The Agent tabs' matrix (M9, reused by Skills in M11): a row per item, a
// column per agent, a macOS checkbox in every cell. The whole cell is what
// is pressed; a tick is not written yet (light fill, amber dot) until the
// pending bar's preview. Each column head is the agent's own icon and opens
// the column's menu: Bật tất cả, Tắt tất cả, Bỏ N thay đổi chưa ghi. A
// column that is read only has its icon dimmed and no menu.

import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { AGENT_MARK, AGENT_NAME, AGENTS, type ColumnMenu } from '../lib/mcp';
import type { Agent } from '../lib/model';
import { NAV_ICONS } from '../window/icons';
import { Button } from '../components/parts';

export function AgentLogo({ agent }: { agent: Agent }) {
  return (
    <span className={'agl ' + AGENT_MARK[agent]} aria-hidden="true">
      {NAV_ICONS[agent]}
    </span>
  );
}

export interface MatrixCell {
  /** `null`: nothing to tick here (`—`), or the file is unreadable (`?`). */
  checked: boolean | null;
  mark?: '—' | '?';
  pending: boolean;
  disabled: boolean;
  title: string;
}

export interface MatrixColumn {
  agent: Agent;
  /** The column has a menu: it can be written and has a box to tick. */
  open: boolean;
  /** Why the icon is dimmed. */
  note?: string;
}

/** The icon at a column's head, and its menu. */
function ColumnHead(props: {
  column: MatrixColumn;
  noun: string;
  menu: ColumnMenu;
  open: boolean;
  onOpen(open: boolean): void;
  onSet(on: boolean): void;
  onClear(): void;
}) {
  const { agent } = props.column;
  const name = AGENT_NAME[agent];
  const button = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const m = props.menu;
  const items = (): HTMLButtonElement[] => Array.from(list.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not([disabled])') ?? []);
  const onOpen = useRef(props.onOpen);
  onOpen.current = props.onOpen;

  // Open: focus on the first item; a press anywhere else closes it.
  useEffect(() => {
    if (!props.open) return;
    items()[0]?.focus();
    const away = (e: MouseEvent) => {
      const t = e.target as Node;
      if (!list.current?.contains(t) && !button.current?.contains(t)) onOpen.current(false);
    };
    document.addEventListener('mousedown', away);
    return () => document.removeEventListener('mousedown', away);
  }, [props.open]);

  const close = () => {
    props.onOpen(false);
    button.current?.focus();
  };
  const pick = (run: () => void) => {
    run();
    close();
  };
  const onKey = (e: KeyboardEvent) => {
    const all = items();
    const i = all.indexOf(document.activeElement as HTMLButtonElement);
    const go = (n: number) => {
      e.preventDefault();
      all[(n + all.length) % all.length]?.focus();
    };
    if (e.key === 'ArrowDown') go(i + 1);
    else if (e.key === 'ArrowUp') go(i < 0 ? all.length - 1 : i - 1);
    else if (e.key === 'Home') go(0);
    else if (e.key === 'End') go(all.length - 1);
    else if (e.key === 'Escape') {
      // The menu closes; the dialog or the window behind it stays.
      e.preventDefault();
      e.stopPropagation();
      close();
    } else if (e.key === 'Tab') props.onOpen(false);
  };

  if (!props.column.open)
    return (
      <span className="agb dis" title={name + ': ' + (props.column.note ?? 'chỉ xem')}>
        <AgentLogo agent={agent} />
        <span className="vh">{name}, chỉ xem</span>
      </span>
    );
  return (
    <>
      <button
        ref={button}
        type="button"
        className={'agb' + (props.open ? ' on' : '')}
        aria-haspopup="menu"
        aria-expanded={props.open}
        aria-label={name + ': tuỳ chọn cột'}
        title={name}
        onClick={() => props.onOpen(!props.open)}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown' && !props.open) {
            e.preventDefault();
            props.onOpen(true);
          }
        }}
      >
        <AgentLogo agent={agent} />
        <span className="cv" aria-hidden="true" />
      </button>
      {props.open ? (
        <div ref={list} className="cmenu" role="menu" aria-label={name} onKeyDown={onKey}>
          <div className="mh" aria-hidden="true">
            <b>{name}</b>
            <small>
              {m.ticked}/{m.total} {props.noun} đang tick
            </small>
          </div>
          <button type="button" role="menuitem" className="mi" tabIndex={-1} disabled={m.ticked === m.total} onClick={() => pick(() => props.onSet(true))}>
            Bật tất cả
          </button>
          <button type="button" role="menuitem" className="mi" tabIndex={-1} disabled={m.ticked === 0} onClick={() => pick(() => props.onSet(false))}>
            Tắt tất cả
          </button>
          {m.pending ? (
            <>
              <div className="sep" role="separator" />
              <button type="button" role="menuitem" className="mi" tabIndex={-1} onClick={() => pick(props.onClear)}>
                Bỏ {m.pending} thay đổi chưa ghi
              </button>
            </>
          ) : null}
        </div>
      ) : null}
    </>
  );
}

/** One box. `aria-disabled` keeps a locked box reachable, so its reason can be read. */
function CheckCell({ cell, label, onToggle }: { cell: MatrixCell; label: string; onToggle(): void }) {
  if (cell.checked == null)
    return (
      <span className={cell.mark === '?' ? 'q' : 'na'} title={cell.title || undefined} role="img" aria-label={label + ': ' + (cell.title || 'không có')}>
        {cell.mark ?? '—'}
      </span>
    );
  return (
    <button
      type="button"
      className={'cbx' + (cell.pending ? ' pend' : '')}
      role="checkbox"
      aria-checked={cell.checked}
      aria-disabled={cell.disabled || undefined}
      aria-label={label + (cell.pending ? ', chưa ghi' : '')}
      title={cell.title || undefined}
      onClick={() => {
        if (!cell.disabled) onToggle();
      }}
    >
      <span className="bx" aria-hidden="true" />
    </button>
  );
}

export interface CheckMatrixProps<R> {
  /** What a row is, for the menus: `MCP`, `skill`. */
  noun: string;
  rows: R[];
  columns: MatrixColumn[];
  rowKey(row: R): string;
  /** The name and the line under it. */
  name(row: R): ReactNode;
  /** The row's links at the end: Bật cả ba, Sửa, Xoá. */
  actions?(row: R): ReactNode;
  cell(row: R, agent: Agent): MatrixCell;
  onToggle(row: R, agent: Agent): void;
  menu(agent: Agent): ColumnMenu;
  onColumn(agent: Agent, on: boolean): void;
  onClearColumn(agent: Agent): void;
  /** A row just written or added, lit for a moment. */
  flash?: string | null;
}

export function CheckMatrix<R>(props: CheckMatrixProps<R>) {
  const [menu, setMenu] = useState<Agent | null>(null);
  const columns = AGENTS.map((a) => props.columns.find((c) => c.agent === a) ?? { agent: a, open: false });
  return (
    <div className="mx">
      <div className="mx-r h">
        <span>Tên</span>
        {columns.map((c) => (
          <span key={c.agent} className="ac">
            <ColumnHead
              column={c}
              noun={props.noun}
              menu={props.menu(c.agent)}
              open={menu === c.agent}
              onOpen={(open) => setMenu(open ? c.agent : null)}
              onSet={(on) => props.onColumn(c.agent, on)}
              onClear={() => props.onClearColumn(c.agent)}
            />
          </span>
        ))}
        <span />
      </div>
      {props.rows.map((r) => {
        const key = props.rowKey(r);
        return (
          <div key={key} className={'mx-r' + (props.flash === key ? ' flash' : '')} role="group" aria-label={key}>
            <div className="nm">{props.name(r)}</div>
            {AGENTS.map((a) => (
              <span key={a} className="ce">
                <CheckCell cell={props.cell(r, a)} label={key + ' cho ' + AGENT_NAME[a]} onToggle={() => props.onToggle(r, a)} />
              </span>
            ))}
            <span className="act">{props.actions?.(r)}</span>
          </div>
        );
      })}
    </div>
  );
}

/** "N thay đổi chưa ghi" under the matrix, with Bỏ and Xem trước và ghi…. */
export function PendingBar(props: { lines: string[]; busy?: boolean; onClear(): void; onPreview(): void }) {
  const n = props.lines.length;
  if (!n) return null;
  return (
    <div className="pendbar" role="status">
      <span className="pd" aria-hidden="true" />
      <span className="t">
        <b>{n} thay đổi chưa ghi</b>
        <small title={props.lines.join(' · ')}>{props.lines.slice(0, 3).join(' · ') + (n > 3 ? ' · …' : '')}</small>
      </span>
      <Button onClick={props.onClear}>Bỏ</Button>
      <Button primary disabled={props.busy} onClick={props.onPreview}>
        Xem trước và ghi…
      </Button>
    </div>
  );
}
