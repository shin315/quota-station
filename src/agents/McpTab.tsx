// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The MCP tab of the main window (M9): every MCP server of Claude Code,
// Codex and Antigravity in one matrix, a checkbox per agent, "Thêm MCP" at
// the table's head (mcp-R2, 25/10/2026). Ticks wait on the pending bar until
// one preview writes them all; a server for a known service shows that
// service's logo before its name; under the table, on the window's glass,
// one short line each for the servers kept in the app's store, the ones
// agents and plugins bring themselves (read only), and the backups.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { CARD_END, COUNT, Empty, EmptyIcon, InfoLine, TabHead, TabSub, TabTitle } from '../components/page';
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
import { mac, t } from '../lib/i18n';
import { dayClock } from '../lib/switching';
import type { Action, BannerView, FeedbackView, PopoverView } from '../lib/view';
import { NoteGlyph } from '../components/icons';
import { LOCK_ICON, PLUG_BIG } from '../window/icons';
import { CheckMatrix, LOCK, MATRIX_HOVER, MATRIX_TITLE, PendingBar, type MatrixCell, type MatrixColumn } from './CheckMatrix';
import { editable, type Mcp } from './useMcp';

/** What is the matter with a column, said once above the table. */
function columnBanners(table: McpTable): BannerView[] {
  const out: BannerView[] = [];
  for (const c of table.columns) {
    const name = AGENT_NAME[c.agent];
    if (c.state === 'invalid' || c.state === 'unreadable') {
      const file = c.file.split('/').pop();
      const reason = c.reason ?? t('Không rõ lý do', 'Unknown reason');
      out.push({
        tone: 'bad',
        title: t('Không đọc được ' + file + ' của ' + name, 'Cannot read ' + name + '’s ' + file),
        body: t(
          reason + '. Cột ' + name + ' chỉ xem cho đến khi file đọc được; app không ghi vào file lỗi.',
          reason + '. The ' + name + ' column is view only until the file can be read; the app does not write to a broken file.',
        ),
        actions: [{ label: t('Đọc lại', 'Read again'), act: 'mcp:reread' }],
      });
    } else if (c.state === 'absent')
      out.push({
        tone: 'info',
        title: mac(t(name + ' chưa cài trên máy này', name + ' is not installed on this Mac')),
        body: t('Cột ' + name + ' để trống.', 'The ' + name + ' column stays empty.'),
        actions: [],
      });
    else if (c.state === 'noCli')
      out.push({
        tone: 'warn',
        title: t('Không tìm thấy lệnh claude', 'The claude command was not found'),
        body: t(
          'Claude Code tự ghi danh sách MCP qua lệnh claude mcp, nên cột Claude Code chỉ xem. Cài Claude Code rồi bấm Đọc lại.',
          'Claude Code writes its MCP list itself through claude mcp, so the Claude Code column is view only. Install Claude Code, then press Read again.',
        ),
        actions: [{ label: t('Đọc lại', 'Read again'), act: 'mcp:reread' }],
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
        title: mac(t(r.name + ' thiếu khoá trên máy này', r.name + ' is missing a key on this Mac')),
        body: t(
          'Thiếu ' + missing.map((s) => s.label).join(', ') + ': chưa bật được ' + r.name + ' ở agent chưa có nó.',
          'Missing ' + missing.map((s) => s.label).join(', ') + ': ' + r.name + ' cannot be turned on for agents that do not have it yet.',
        ),
        actions: [
          { label: t('Dán khoá', 'Paste key'), act: `mcp:key:${r.name}` },
          { label: t('Để sau', 'Later'), act: `mcp:later:${r.name}` },
        ],
      });
    else if (r.missingCommand?.path)
      out.push({
        tone: 'warn',
        title: mac(t('Lệnh của ' + r.name + ' chưa có trên máy này', r.name + '’s command is not on this Mac')),
        body: t(
          r.missingCommand.command + ' không có, nên ' + r.name + ' chỉ tắt được, chưa bật được. Cài lệnh rồi bấm Đọc lại.',
          r.missingCommand.command + ' is missing, so ' + r.name + ' can only be turned off, not on. Install the command, then press Read again.',
        ),
        actions: [
          { label: t('Cách cài', 'How to install'), act: `mcp:howcmd:${r.name}` },
          { label: t('Đọc lại', 'Read again'), act: 'mcp:reread' },
        ],
      });
  }
  return out;
}

function columns(table: McpTable): MatrixColumn[] {
  return table.columns.map((c) => ({
    agent: c.agent,
    open: c.writable && table.rows.some((r) => !r.cells[c.agent].lock),
    note:
      c.state === 'absent'
        ? mac(t('chưa cài trên máy này', 'not installed on this Mac'))
        : c.state === 'invalid' || c.state === 'unreadable'
          ? t('file lỗi, chỉ xem', 'broken file, view only')
          : c.state === 'noCli'
            ? t('không có lệnh claude', 'no claude command')
            : undefined,
  }));
}

function cellOf(table: McpTable, mcp: Mcp, row: McpRow, agent: Agent): MatrixCell {
  const col = table.columns.find((c) => c.agent === agent);
  if (col?.state === 'absent') return { checked: null, mark: '—', pending: false, disabled: true, title: mac(t(AGENT_NAME[agent] + ' chưa cài trên máy này', AGENT_NAME[agent] + ' is not installed on this Mac')) };
  if (col?.state === 'invalid' || col?.state === 'unreadable')
    return { checked: null, mark: '?', pending: false, disabled: true, title: t('Không đọc được ' + col.file.split('/').pop(), 'Cannot read ' + col.file.split('/').pop()) };
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
          <span
            className={LOCK}
            role="img"
            aria-label={t('có khoá, giữ ngoài giao diện', 'has a key, kept out of the window')}
            title={t('Có khoá. App không hiện, không ghi vào bản lưu hay log.', 'Has a key. The app does not show it or write it to backups or logs.')}
          >
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
  return names.length ? names.join(', ') : t('agent', 'the agents');
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
  const sub = v.busy === 'reading' ? t('đang đọc…', 'reading…') : v.readAt ? t('đọc lúc ' + clock(v.readAt), 'read at ' + clock(v.readAt)) : fresh.text;
  const top = (
    <TabHead>
      <TabTitle>MCP</TabTitle>
      <TabSub>
        <Dot tone={v.busy === 'reading' ? 'stale' : 'ok'} />
        {model.machine} · {sub}
      </TabSub>
      <Button disabled={busy} onClick={mcp.reread}>
        {t('Đọc lại', 'Read again')}
      </Button>
    </TabHead>
  );
  if (!table)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <b>{t('Đang đọc danh sách MCP…', 'Reading the MCP lists…')}</b>
            {mac(t('Claude Code, Codex và Antigravity, trên máy này.', 'Claude Code, Codex and Antigravity, on this Mac.'))}
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
            {t('MCP của bạn', 'Your MCP')} <span className={COUNT}>{table.rows.length}</span>
            <span className={CARD_END}>
              <Button primary small disabled={busy} onClick={mcp.add}>
                {t('Thêm MCP', 'Add MCP')}
              </Button>
            </span>
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
                  <LinkButton small className={MATRIX_HOVER} aria-label={t('Bật ' + r.name + ' cho cả ba agent', 'Turn on ' + r.name + ' for all three agents')} onClick={() => mcp.rowAll(r)}>
                    {t('Bật cả ba', 'All three')}
                  </LinkButton>
                ) : null}
                <LinkButton small aria-label={t('Sửa ' + r.name, 'Edit ' + r.name)} disabled={!editable(r).length || busy} onClick={() => mcp.edit(r)}>
                  {t('Sửa', 'Edit')}
                </LinkButton>
                <LinkButton
                  small
                  aria-label={t('Xoá ' + r.name, 'Delete ' + r.name)}
                  disabled={busy || !Object.values(r.cells).some((c) => c.state !== 'absent' && c.lock !== 'readOnly' && c.lock !== 'shape')}
                  onClick={() => mcp.setDialog({ kind: 'del', name: r.name, mode: 'kho', keys: true })}
                >
                  {t('Xoá', 'Delete')}
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
            <b>{mac(t('Chưa có MCP nào trên máy này', 'No MCP on this Mac yet'))}</b>
            <CardText className="mt-2 mb-0">{t('Thêm một MCP một lần cho cả ba agent.', 'Add an MCP once for all three agents.')}</CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary disabled={busy} onClick={mcp.add}>
                {t('Thêm MCP', 'Add MCP')}
              </Button>
            </div>
          </Empty>
        </Panel>
      )}
      {/* On the window's glass, under the table: no card of their own. */}
      <div className="grid px-3.5">
        {table.stored.map((k) => (
          <InfoLine
            key={k.name}
            icon={<NoteGlyph note="putAway" />}
            actions={
              <>
                <Button disabled={busy} onClick={() => mcp.setDialog({ kind: 'kho', name: k.name, agents: khoAgents(table, k.agents) })}>
                  {t('Bật lại…', 'Turn back on…')}
                </Button>
                <LinkButton small disabled={busy} onClick={() => mcp.setDialog({ kind: 'forget', name: k.name, keys: true })}>
                  {t('Xoá hẳn', 'Delete for good')}
                </LinkButton>
              </>
            }
          >
            <b>{k.name}</b>
            {t(
              ' đang cất: tắt ở mọi agent từ ' + dayClock(new Date(k.at * 1000).toISOString(), now) + '.',
              ' is put away: off for every agent since ' + dayClock(new Date(k.at * 1000).toISOString(), now) + '.',
            )}
          </InfoLine>
        ))}
        {table.owned.length ? (
          <InfoLine
            icon={<NoteGlyph note="builtIn" />}
            actions={
              <LinkButton small onClick={() => mcp.setDialog({ kind: 'owned' })}>
                {t('Xem', 'View')}
              </LinkButton>
            }
          >
            {t(
              table.owned.length + ' MCP có sẵn trong ' + owners(table) + '. App để yên, không sửa.',
              table.owned.length + ' MCP built into ' + owners(table) + '. The app leaves them alone.',
            )}
          </InfoLine>
        ) : null}
        <InfoLine
          icon={<NoteGlyph note="backups" />}
          actions={
            <LinkButton small onClick={() => mcp.setDialog({ kind: 'backups' })}>
              {t('Bản đã lưu', 'Backups')}
            </LinkButton>
          }
        >
          {t(
            'Trước mỗi lần ghi, app lưu lại bản cũ để lấy lại được. Bản lưu không chứa khoá.',
            'Before each write, the app keeps the old version so it can be restored. Backups hold no keys.',
          )}
        </InfoLine>
      </div>
    </>
  );
}

/** The agents a stored server goes back to: those it was in that can be written. */
export function khoAgents(table: McpTable, was: Agent[]): Record<Agent, boolean> {
  const can = (a: Agent) => !!table.columns.find((c) => c.agent === a)?.writable;
  return { claude: was.includes('claude') && can('claude'), codex: was.includes('codex') && can('codex'), agy: was.includes('agy') && can('agy') };
}
