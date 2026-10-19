// The MCP tab of the main window (M9): every MCP server of Claude Code,
// Codex and Antigravity in one matrix, a checkbox per agent. Ticks wait on
// the pending bar until one preview writes them all; a server for a known
// service shows that service's logo before its name; under the table, one
// short line each for the servers kept in the app's store, the ones agents
// and plugins bring themselves (read only), and the backups.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { COUNT, Empty, EmptyIcon, InfoLine, TabHead, TabSub, TabTitle } from '../components/page';
import { ServiceIcon } from '../components/logos';
import { Banner, Button, Chip, Dot, Feedback, LinkButton } from '../components/parts';
import { serviceOf } from '../lib/brands';
import { clock } from '../lib/format';
import {
  AGENT_NAME,
  cellTitle,
  columnMenu,
  pendingList,
  pendingText,
  rowCanAll,
  rowChips,
  shown,
  whereLine,
} from '../lib/mcp';
import type { Agent, McpRow, McpTable, Model } from '../lib/model';
import { dayClock } from '../lib/switching';
import type { Action, BannerView, FeedbackView, PopoverView } from '../lib/view';
import { ARCHIVE_ICON, LOCK_ICON, NAV_ICONS, PLUG_BIG, PUZZLE_ICON } from '../window/icons';
import { CheckMatrix, LOCK, MATRIX_HOVER, MATRIX_TITLE, PendingBar, type MatrixCell, type MatrixColumn } from './CheckMatrix';
import { editable, type Mcp } from './useMcp';

/** What is the matter with a column, said once above the table. */
function columnBanners(table: McpTable): BannerView[] {
  const out: BannerView[] = [];
  for (const c of table.columns) {
    const name = AGENT_NAME[c.agent];
    if (c.state === 'invalid' || c.state === 'unreadable')
      out.push({
        tone: 'bad',
        title: 'Không đọc được ' + c.file.split('/').pop() + ' của ' + name,
        body: (c.reason ?? 'Không rõ lý do') + '. Cột ' + name + ' chỉ xem cho đến khi file đọc được; app không ghi vào file lỗi.',
        actions: [{ label: 'Đọc lại', act: 'mcp:reread' }],
      });
    else if (c.state === 'absent')
      out.push({ tone: 'info', title: name + ' chưa cài trên máy này', body: 'Cột ' + name + ' để trống.', actions: [] });
    else if (c.state === 'noCli')
      out.push({
        tone: 'warn',
        title: 'Không tìm thấy lệnh claude',
        body: 'Claude Code tự ghi danh sách MCP qua lệnh claude mcp, nên cột Claude Code chỉ xem. Cài Claude Code rồi bấm Đọc lại.',
        actions: [{ label: 'Đọc lại', act: 'mcp:reread' }],
      });
  }
  return out;
}

/** A server short of a key, or whose command is not on this Mac. */
function rowBanners(rows: McpRow[], later: string[]): BannerView[] {
  const out: BannerView[] = [];
  for (const r of rows) {
    if (later.includes(r.name)) continue;
    const missing = r.secrets.filter((s) => !s.have);
    if (missing.length)
      out.push({
        tone: 'warn',
        title: r.name + ' thiếu khoá trên máy này',
        body: 'Ô tick bị khoá ở agent chưa có ' + r.name + ', đến khi có ' + missing.map((s) => s.label).join(', ') + '.',
        actions: [
          { label: 'Dán khoá', act: `mcp:key:${r.name}` },
          { label: 'Để sau', act: `mcp:later:${r.name}` },
        ],
      });
    else if (r.missingCommand?.path)
      out.push({
        tone: 'warn',
        title: 'Lệnh của ' + r.name + ' chưa có trên máy này',
        body: r.missingCommand.command + ' không có, nên ' + r.name + ' chỉ tắt được, chưa bật được. Cài lệnh rồi bấm Đọc lại.',
        actions: [
          { label: 'Cách cài', act: `mcp:howcmd:${r.name}` },
          { label: 'Đọc lại', act: 'mcp:reread' },
        ],
      });
  }
  return out;
}

function columns(table: McpTable): MatrixColumn[] {
  return table.columns.map((c) => ({
    agent: c.agent,
    open: c.writable && table.rows.some((r) => !r.cells[c.agent].lock),
    note: c.state === 'absent' ? 'chưa cài trên máy này' : c.state === 'invalid' || c.state === 'unreadable' ? 'file lỗi, chỉ xem' : c.state === 'noCli' ? 'không có lệnh claude' : undefined,
  }));
}

function cellOf(table: McpTable, mcp: Mcp, row: McpRow, agent: Agent): MatrixCell {
  const col = table.columns.find((c) => c.agent === agent);
  if (col?.state === 'absent') return { checked: null, mark: '—', pending: false, disabled: true, title: AGENT_NAME[agent] + ' chưa cài trên máy này' };
  if (col?.state === 'invalid' || col?.state === 'unreadable')
    return { checked: null, mark: '?', pending: false, disabled: true, title: 'Không đọc được ' + col.file.split('/').pop() };
  const cell = row.cells[agent];
  const p = mcp.pending[row.name + '|' + agent];
  return { checked: shown(mcp.pending, row, agent), pending: p !== undefined, disabled: cell.lock != null, title: cellTitle(row, agent) };
}

function Name({ row }: { row: McpRow }) {
  const locked = row.secrets.length > 0 && row.secrets.every((s) => s.have);
  return (
    <>
      <div className={MATRIX_TITLE}>
        <ServiceIcon service={serviceOf(row.name, row.program, row.host)} />
        <b>{row.name}</b>
        {locked ? (
          <span className={LOCK} role="img" aria-label="có khoá, giữ ngoài giao diện" title="Có khoá. App không hiện, không ghi vào bản lưu hay log.">
            {LOCK_ICON}
          </span>
        ) : null}
        {rowChips(row).map((c) => (
          <Chip key={c.t} chip={c} />
        ))}
      </div>
      <small>{whereLine(row)}</small>
    </>
  );
}

function owners(table: McpTable): string {
  const names = [...new Set(table.owned.map((o) => o.owner))];
  return names.length ? names.join(', ') : 'agent';
}

export interface McpTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  mcp: Mcp;
  onAction(act: Action): void;
}

export function McpTab({ model, fresh, feedback, now, mcp, onAction }: McpTabProps) {
  const v = model.mcp;
  const table = v.table;
  const busy = v.busy != null || mcp.working;
  const sub = v.busy === 'reading' ? 'đang đọc…' : v.readAt ? 'đọc lúc ' + clock(v.readAt) : fresh.text;
  const top = (
    <TabHead>
      <TabTitle>MCP</TabTitle>
      <TabSub>
        <Dot tone={v.busy === 'reading' ? 'stale' : 'ok'} />
        {model.machine} · {sub}
      </TabSub>
      <Button primary disabled={!table || busy} onClick={mcp.add}>
        Thêm MCP
      </Button>
      <Button disabled={busy} onClick={mcp.reread}>
        Đọc lại
      </Button>
    </TabHead>
  );
  if (!table)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <b>Đang đọc danh sách MCP…</b>
            Claude Code, Codex và Antigravity, trên máy này.
          </Empty>
        </Panel>
      </>
    );
  const banners = [...columnBanners(table), ...rowBanners(table.rows, mcp.later)];
  const list = pendingList(mcp.pending, table.rows);
  return (
    <>
      {top}
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {banners.map((b) => (
        <Banner key={b.title} banner={b} onAction={onAction} />
      ))}
      {table.rows.length ? (
        <Panel>
          <CardTitle>
            MCP của bạn <span className={COUNT}>{table.rows.length}</span>
          </CardTitle>
          <CheckMatrix
            noun="MCP"
            rows={table.rows}
            columns={columns(table)}
            rowKey={(r) => r.name}
            name={(r) => <Name row={r} />}
            actions={(r) => (
              <>
                {rowCanAll(mcp.pending, r) ? (
                  <LinkButton small className={MATRIX_HOVER} aria-label={'Bật ' + r.name + ' cho cả ba agent'} onClick={() => mcp.rowAll(r)}>
                    Bật cả ba
                  </LinkButton>
                ) : null}
                <LinkButton small aria-label={'Sửa ' + r.name} disabled={!editable(r).length || busy} onClick={() => mcp.edit(r)}>
                  Sửa
                </LinkButton>
                <LinkButton
                  small
                  aria-label={'Xoá ' + r.name}
                  disabled={busy || !Object.values(r.cells).some((c) => c.state !== 'absent' && c.lock !== 'readOnly' && c.lock !== 'shape')}
                  onClick={() => mcp.setDialog({ kind: 'del', name: r.name, mode: 'kho', keys: true })}
                >
                  Xoá
                </LinkButton>
              </>
            )}
            cell={(r, a) => cellOf(table, mcp, r, a)}
            onToggle={mcp.toggle}
            menu={(a) => columnMenu(mcp.pending, table.rows, a)}
            onColumn={mcp.setColumn}
            onClearColumn={mcp.clearColumn}
            flash={mcp.flash}
          />
          <PendingBar lines={list.map(pendingText)} busy={busy} onClear={mcp.clearPending} onPreview={mcp.previewPending} />
        </Panel>
      ) : (
        <Panel>
          <Empty>
            <EmptyIcon>{PLUG_BIG}</EmptyIcon>
            <b>Chưa có MCP nào trên máy này</b>
            <CardText className="mt-2 mb-0">Thêm một MCP một lần cho cả ba agent.</CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary disabled={busy} onClick={mcp.add}>
                Thêm MCP
              </Button>
            </div>
          </Empty>
        </Panel>
      )}
      <Panel>
        <div className="grid">
          {table.stored.map((k) => (
            <InfoLine
              key={k.name}
              icon={ARCHIVE_ICON}
              actions={
                <>
                  <Button disabled={busy} onClick={() => mcp.setDialog({ kind: 'kho', name: k.name, agents: khoAgents(table, k.agents) })}>
                    Bật lại…
                  </Button>
                  <LinkButton small disabled={busy} onClick={() => mcp.setDialog({ kind: 'forget', name: k.name, keys: true })}>
                    Xoá hẳn
                  </LinkButton>
                </>
              }
            >
              <b>{k.name}</b> đang cất: tắt ở mọi agent từ {dayClock(new Date(k.at * 1000).toISOString(), now)}.
            </InfoLine>
          ))}
          {table.owned.length ? (
            <InfoLine
              icon={PUZZLE_ICON}
              actions={
                <LinkButton small onClick={() => mcp.setDialog({ kind: 'owned' })}>
                  Xem
                </LinkButton>
              }
            >
              {table.owned.length} MCP có sẵn trong {owners(table)}. App để yên, không sửa.
            </InfoLine>
          ) : null}
          <InfoLine
            icon={NAV_ICONS.history}
            actions={
              <LinkButton small onClick={() => mcp.setDialog({ kind: 'backups' })}>
                Bản đã lưu
              </LinkButton>
            }
          >
            Trước mỗi lần ghi, app lưu lại bản cũ để lấy lại được. Bản lưu không chứa khoá.
          </InfoLine>
        </div>
      </Panel>
    </>
  );
}

/** The agents a stored server goes back to: those it was in that can be written. */
export function khoAgents(table: McpTable, was: Agent[]): Record<Agent, boolean> {
  const can = (a: Agent) => !!table.columns.find((c) => c.agent === a)?.writable;
  return { claude: was.includes('claude') && can('claude'), codex: was.includes('codex') && can('codex'), agy: was.includes('agy') && can('agy') };
}
