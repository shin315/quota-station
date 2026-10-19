// The Agent tabs' matrix (M9, reused by Skills in M11): a row per item, a
// column per agent, a macOS checkbox in every cell. The whole cell is what
// is pressed; a tick is not written yet (light fill, amber dot) until the
// pending bar's preview. Each column head is the agent's own icon and opens
// the column's menu: Bật tất cả, Tắt tất cả, Bỏ N thay đổi chưa ghi. A
// column that is read only has its icon dimmed and no menu.

import { useRef, useState, type ReactNode } from 'react';
import { Checkbox as CheckboxPrimitive } from 'radix-ui';
import { CheckboxBox } from '@/components/ui/checkbox';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';
import { AGENT_MARK, AGENT_NAME, AGENTS, type ColumnMenu } from '../lib/mcp';
import type { Agent } from '../lib/model';
import { NAV_ICONS } from '../window/icons';
import { Button } from '../components/parts';

/** Each agent's icon in its own colour. */
const LOGO_INK: Record<'c' | 'x' | 'g', string> = { c: 'text-id-claude-ink', x: 'text-id-codex-ink', g: 'text-id-agy-ink' };

export function AgentLogo({ agent }: { agent: Agent }) {
  return (
    <span className={cn('agl inline-flex size-4 items-center justify-center [&_svg]:size-4', LOGO_INK[AGENT_MARK[agent]])} aria-hidden="true">
      {NAV_ICONS[agent]}
    </span>
  );
}

/**
 * A row of the matrix: the name, a column per agent (`--mx-cell` wide), the
 * row's links. In a dialog the columns are narrower.
 */
export const MATRIX_ROW =
  'grid grid-cols-[minmax(0,1.7fr)_repeat(3,var(--mx-cell,96px))_132px] items-center gap-2 border-t-[.5px] border-card-line px-1.5 py-2 text-sm first:border-t-0 in-data-[kind=dialog]:grid-cols-[minmax(0,1fr)_repeat(3,64px)]';
/** The head row: the column icons, small and grey. */
export const MATRIX_HEAD = 'pt-0 pb-1 text-xs text-ink3 [&>span]:flex [&>span]:items-center [&>span]:gap-[5px]';
/** The name cell: the name in bold, the line under it cut short. */
export const MATRIX_NAME =
  'block min-w-0 [&_b]:text-body [&_b]:font-semi [&_b]:[overflow-wrap:anywhere] [&_small]:block [&_small]:overflow-hidden [&_small]:text-ellipsis [&_small]:whitespace-nowrap [&_small]:text-xs [&_small]:text-ink2';
/** The name and its chips on one line. */
export const MATRIX_TITLE = 'flex flex-wrap items-center gap-1.5';
/** A row's link that shows while the row is pointed at or holds the focus. */
export const MATRIX_HOVER = 'opacity-0 group-focus-within/row:opacity-100 group-hover/row:opacity-100';
/** A lock next to a name: the row has a key, kept out of sight. */
export const LOCK = 'inline-flex text-ink3';
/** A cell: its box in the middle. */
export const MATRIX_CELL = 'flex min-h-[22px] items-center justify-center';

export interface MatrixCell {
  /** `null`: nothing to tick here (`—`), the file is unreadable (`?`), or a skill's link points to nothing (`!`). */
  checked: boolean | null;
  mark?: '—' | '?' | '!';
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

const HEAD_BUTTON = 'agb inline-flex h-7 items-center gap-[3px] rounded-[8px] border-0 bg-transparent pr-[7px] pl-2 text-ink2';
/** A column head with no menu: the icon dimmed. */
export const HEAD_OFF = cn(HEAD_BUTTON, 'cursor-default pr-2 opacity-50');

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
  const list = useRef<HTMLDivElement>(null);
  const m = props.menu;

  if (!props.column.open)
    return (
      <span className={HEAD_OFF} title={name + ': ' + (props.column.note ?? 'chỉ xem')}>
        <AgentLogo agent={agent} />
        <span className="sr-only">{name}, chỉ xem</span>
      </span>
    );
  return (
    <DropdownMenu open={props.open} onOpenChange={props.onOpen} modal={false}>
      <DropdownMenuTrigger
        className={cn(HEAD_BUTTON, 'cursor-pointer hover:bg-fill focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent data-[state=open]:bg-fill')}
        aria-label={name + ': tuỳ chọn cột'}
        title={name}
      >
        <AgentLogo agent={agent} />
        <span className="size-0 border-x-[3.5px] border-t-4 border-x-transparent border-t-ink3" aria-hidden="true" />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        ref={list}
        aria-label={name}
        aria-labelledby={undefined}
        loop
        // Open: focus on the first item, however it was opened (Radix focuses
        // the menu itself when a pointer opens it).
        onOpenAutoFocus={(e) => {
          e.preventDefault();
          list.current?.querySelector<HTMLElement>('[role="menuitem"]:not([disabled])')?.focus();
        }}
        // The menu closes; the dialog or the window behind it stays.
        onEscapeKeyDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === 'Tab') props.onOpen(false);
        }}
      >
        <DropdownMenuLabel aria-hidden="true">
          <b className="block text-sm font-semi text-ink">{name}</b>
          <small className="text-xs text-ink2">
            {m.ticked}/{m.total} {props.noun} đang tick
          </small>
        </DropdownMenuLabel>
        <DropdownMenuItem asChild disabled={m.ticked === m.total} onSelect={() => props.onSet(true)}>
          <button type="button" disabled={m.ticked === m.total}>
            Bật tất cả
          </button>
        </DropdownMenuItem>
        <DropdownMenuItem asChild disabled={m.ticked === 0} onSelect={() => props.onSet(false)}>
          <button type="button" disabled={m.ticked === 0}>
            Tắt tất cả
          </button>
        </DropdownMenuItem>
        {m.pending ? (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuItem asChild onSelect={props.onClear}>
              <button type="button">Bỏ {m.pending} thay đổi chưa ghi</button>
            </DropdownMenuItem>
          </>
        ) : null}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/**
 * One box. `aria-disabled` keeps a locked box reachable, so its reason can be
 * read. Enter ticks it too, as the button it was did.
 */
function CheckCell({ cell, label, onToggle }: { cell: MatrixCell; label: string; onToggle(): void }) {
  if (cell.checked == null)
    return (
      <span
        className={cell.mark === '?' || cell.mark === '!' ? 'font-bold text-warn-ink' : 'text-ink3'}
        title={cell.title || undefined}
        role="img"
        aria-label={label + ': ' + (cell.title || 'không có')}
      >
        {cell.mark ?? '—'}
      </span>
    );
  const toggle = () => {
    if (!cell.disabled) onToggle();
  };
  return (
    <CheckboxPrimitive.Root
      data-slot="checkbox"
      className={cn(
        'group/cb relative flex min-h-[34px] w-full cursor-pointer items-center justify-center gap-1.5 rounded-[8px] border-0 bg-transparent px-1 hover:bg-fill',
        'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent',
        cell.pending && "bg-accent/10 hover:bg-accent/10 after:absolute after:top-[5px] after:right-[7px] after:size-1.5 after:rounded-full after:bg-warn after:content-['']",
        'aria-disabled:cursor-default aria-disabled:opacity-40 aria-disabled:hover:bg-transparent',
      )}
      checked={cell.checked}
      aria-disabled={cell.disabled || undefined}
      aria-label={label + (cell.pending ? ', chưa ghi' : '')}
      title={cell.title || undefined}
      onCheckedChange={toggle}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          toggle();
        }
      }}
    >
      <CheckboxBox />
    </CheckboxPrimitive.Root>
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
    <div className="grid">
      <div className={cn(MATRIX_ROW, MATRIX_HEAD)}>
        <span>Tên</span>
        {columns.map((c) => (
          <span key={c.agent} className="relative justify-center">
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
          <div
            key={key}
            className={cn(
              MATRIX_ROW,
              'group/row',
              props.flash === key && 'flash rounded-[8px] border-t-transparent bg-accent/10',
              '[.flash+&]:border-t-transparent',
            )}
            role="group"
            aria-label={key}
          >
            <div className={MATRIX_NAME}>{props.name(r)}</div>
            {AGENTS.map((a) => (
              <span key={a} className={MATRIX_CELL}>
                <CheckCell cell={props.cell(r, a)} label={key + ' cho ' + AGENT_NAME[a]} onToggle={() => props.onToggle(r, a)} />
              </span>
            ))}
            <span className="flex items-center justify-end gap-2.5">{props.actions?.(r)}</span>
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
    <div className="pendbar mt-2 flex items-center gap-2.5 rounded-[10px] bg-[color-mix(in_srgb,var(--warn)_10%,var(--fill))] px-2.5 py-2" role="status">
      <span className="size-1.5 flex-[0_0_6px] rounded-full bg-warn" aria-hidden="true" />
      <span className="min-w-0 flex-auto text-sm">
        <b>{n} thay đổi chưa ghi</b>
        <small className="block overflow-hidden text-ellipsis whitespace-nowrap text-xs text-ink2" title={props.lines.join(' · ')}>{props.lines.slice(0, 3).join(' · ') + (n > 3 ? ' · …' : '')}</small>
      </span>
      <Button onClick={props.onClear}>Bỏ</Button>
      <Button primary disabled={props.busy} onClick={props.onPreview}>
        Xem trước và ghi…
      </Button>
    </div>
  );
}
