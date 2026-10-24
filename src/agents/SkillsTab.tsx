// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Skills tab of the main window (M11, B and G): every skill of Claude
// Code, Codex and Antigravity in the MCP tab's matrix, a checkbox per agent.
// Ticks wait on the pending bar until one preview writes them all. Above
// the table, the newer versions found on GitHub (the app never updates on
// its own), all of them or a repository's set at once through one preview;
// under it, one line each for the skills installed by hand to
// gather, the links to nothing, and the skills agents and plugins bring.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Segment, Segmented } from '@/components/ui/segmented';
import { COUNT, Empty, EmptyIcon, InfoLine, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, Dot, Feedback, LinkButton } from '../components/parts';
import { t } from '../lib/i18n';
import { AGENT_NAME, columnMenu, pendingList, shown } from '../lib/mcp';
import type { Agent, Model, SkillRow, SkillTable } from '../lib/model';
import { checkedLine, FILTERS, filterCounts, filterWord, filterRows, pendingSkillText, skillBanners, skillCellTitle, skillChips, sourceLine, updateGroups, type SkillFilter } from '../lib/skills';
import type { Action, FeedbackView, PopoverView } from '../lib/view';
import { PUZZLE_ICON, SPARK_BIG, STACK_ICON } from '../window/icons';
import { CheckMatrix, MATRIX_TITLE, PendingBar, type MatrixCell, type MatrixColumn } from './CheckMatrix';
import type { Skills } from './useSkills';

function columns(table: SkillTable): MatrixColumn[] {
  return table.columns.map((c) => ({
    agent: c.agent,
    open: c.writable && table.rows.some((r) => !r.cells[c.agent].lock),
    note: !c.present
      ? t('chưa cài trên máy này', 'not installed on this Mac')
      : c.state === 'invalid' || c.state === 'unreadable'
        ? t('file lỗi, chỉ xem', 'file has errors, view only')
        : undefined,
  }));
}

function cellOf(table: SkillTable, skills: Skills, row: SkillRow, agent: Agent): MatrixCell {
  const col = table.columns.find((c) => c.agent === agent);
  if (col && !col.present) return { checked: null, mark: '—', pending: false, disabled: true, title: t(AGENT_NAME[agent] + ' chưa cài trên máy này', AGENT_NAME[agent] + ' is not installed on this Mac') };
  if (col?.state === 'invalid' || col?.state === 'unreadable') return { checked: null, mark: '?', pending: false, disabled: true, title: t('Không đọc được ' + col.file, 'Could not read ' + col.file) };
  const cell = row.cells[agent];
  if (cell.lock === 'broken') return { checked: null, mark: '!', pending: false, disabled: true, title: t('Liên kết hỏng', 'Broken link') };
  if (cell.state === 'absent') return { checked: null, mark: '—', pending: false, disabled: true, title: skillCellTitle(row, agent) };
  const p = skills.pending[row.name + '|' + agent];
  return { checked: shown(skills.pending, row, agent), pending: p !== undefined, disabled: cell.lock != null, title: skillCellTitle(row, agent) };
}

function Name({ row }: { row: SkillRow }) {
  return (
    <>
      <div className={MATRIX_TITLE}>
        <b>{row.name}</b>
        {skillChips(row).map((c) => (
          <Chip key={c.t} chip={c} />
        ))}
      </div>
      <small>{sourceLine(row)}</small>
    </>
  );
}

function Filters({ rows, value, onChange }: { rows: SkillRow[]; value: SkillFilter; onChange(f: SkillFilter): void }) {
  const n = filterCounts(rows);
  return (
    <div className="mt-0 mb-1.5 flex flex-wrap items-center gap-2">
      <Segmented variant="inline" aria-label={t('Lọc', 'Filter')}>
        {FILTERS.map((f) => (
          <Segment key={f} pressed={value === f} onClick={() => onChange(f)}>
            {filterWord(f)} {n[f]}
          </Segment>
        ))}
      </Segmented>
    </div>
  );
}

export interface SkillsTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  skills: Skills;
  onAction(act: Action): void;
}

export function SkillsTab({ model, fresh, feedback, skills, onAction }: SkillsTabProps) {
  const v = model.skills;
  const table = v.table;
  const busy = skills.working;
  const sub = v.busy === 'reading' ? t('đang đọc…', 'reading…') : v.readAt ? checkedLine(v) : fresh.text;
  const newer = table?.rows.filter((r) => r.update).length ?? 0;
  const p = v.progress;
  const top = (
    <TabHead>
      <TabTitle>Skills</TabTitle>
      <TabSub>
        <Dot tone={v.busy === 'reading' || v.updates.checking ? 'stale' : 'ok'} />
        {model.machine} · {sub}
      </TabSub>
      <Button primary disabled={!table || busy || !v.npx} title={v.npx ? undefined : t('Không tìm thấy npx', 'npx not found')} onClick={skills.openGh}>
        {t('Cài từ GitHub', 'Install from GitHub')}
      </Button>
      <Button disabled={v.updates.checking} onClick={skills.check}>
        {v.updates.checking ? t('Đang kiểm…', 'Checking…') : t('Kiểm bản mới', 'Check updates')}
      </Button>
      {newer && v.npx ? (
        <Button disabled={busy} onClick={() => skills.openUpdateMany(null)}>
          {p
            ? t('Đang cập nhật ' + (p.done + 1) + '/' + p.total + '…', 'Updating ' + (p.done + 1) + '/' + p.total + '…')
            : t('Cập nhật tất cả (' + newer + ')', 'Update all (' + newer + ')')}
        </Button>
      ) : null}
    </TabHead>
  );
  if (!table)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <b>{t('Đang đọc danh sách skill…', 'Reading the skill list…')}</b>
            {t('Claude Code, Codex và Antigravity, trên máy này.', 'Claude Code, Codex and Antigravity, on this Mac.')}
          </Empty>
        </Panel>
      </>
    );
  const list = filterRows(table.rows, skills.filter);
  const pend = pendingList(skills.pending, table.rows);
  const hand = table.rows.filter((r) => r.importable);
  const owned = table.owned.reduce((n, o) => n + o.names.length, 0);
  const showHand = hand.length > 0 && (skills.filter === 'all' || skills.filter === 'hand');
  const sets = v.npx ? updateGroups(table.rows) : [];
  return (
    <>
      {top}
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {skillBanners(v, table).map((b) => (
        <Banner key={b.title} banner={b} onAction={onAction} />
      ))}
      {sets.length ? (
        <Panel>
          <CardTitle>
            {t('Bản mới theo bộ', 'Updates by set')} <span className={COUNT}>{sets.length}</span>
          </CardTitle>
          <div className="grid">
            {sets.map((g) => (
              <InfoLine
                key={g.repo}
                icon={NEWER_ICON}
                actions={
                  <LinkButton small aria-label={t('Cập nhật cả bộ ' + g.repo, 'Update the whole ' + g.repo + ' set')} disabled={busy} onClick={() => skills.openUpdateMany(g.repo)}>
                    {t('Cập nhật cả bộ', 'Update the set')}
                  </LinkButton>
                }
              >
                <b>{g.repo}</b> · {t(g.names.length + ' skill có bản mới: ', g.names.length + (g.names.length === 1 ? ' skill has an update: ' : ' skills have updates: '))}
                {g.names.join(', ')}
              </InfoLine>
            ))}
          </div>
        </Panel>
      ) : null}
      {table.rows.length ? (
        <Panel>
          <CardTitle>
            {t('Skills của bạn', 'Your skills')} <span className={COUNT}>{table.rows.length}</span>
          </CardTitle>
          <Filters rows={table.rows} value={skills.filter} onChange={skills.setFilter} />
          {list.length ? (
            <div className="[--mx-cell:104px]">
              <CheckMatrix
                noun="skill"
                rows={list}
                columns={columns(table)}
                rowKey={(r) => r.name}
                name={(r) => <Name row={r} />}
                actions={(r) => (
                  <>
                    {r.update && v.npx ? (
                      <LinkButton small aria-label={t('Cập nhật ' + r.name, 'Update ' + r.name)} disabled={busy} onClick={() => skills.setDialog({ kind: 'update', name: r.name })}>
                        {t('Cập nhật', 'Update')}
                      </LinkButton>
                    ) : null}
                    <LinkButton
                      small
                      aria-label={t('Gỡ ' + r.name, 'Remove ' + r.name)}
                      disabled={busy || r.home != null}
                      title={r.home ? t('Gom về thư mục chung trước', 'Gather it into the shared folder first') : undefined}
                      onClick={() => skills.setDialog({ kind: 'del', name: r.name, mode: 'off' })}
                    >
                      {t('Gỡ', 'Remove')}
                    </LinkButton>
                  </>
                )}
                cell={(r, a) => cellOf(table, skills, r, a)}
                onToggle={skills.toggle}
                menu={(a) => columnMenu(skills.pending, list, a)}
                onColumn={(a, on) => skills.setColumn(a, on, list)}
                onClearColumn={skills.clearColumn}
                flash={skills.flash}
              />
            </div>
          ) : (
            <CardText className="mt-2 mb-0">{t('Không có skill nào ở mục này.', 'No skills here.')}</CardText>
          )}
          <PendingBar lines={pend.map(pendingSkillText)} busy={busy} onClear={skills.clearPending} onPreview={skills.previewPending} />
        </Panel>
      ) : (
        <Panel>
          <Empty>
            <EmptyIcon>{SPARK_BIG}</EmptyIcon>
            <b>{t('Chưa có skill nào', 'No skills yet')}</b>
            <CardText className="mt-2 mb-0">{t('Cài từ GitHub, hoặc khôi phục từ gói sao lưu.', 'Install from GitHub, or restore from a backup pack.')}</CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary disabled={busy || !v.npx} onClick={skills.openGh}>
                {t('Cài từ GitHub', 'Install from GitHub')}
              </Button>
            </div>
          </Empty>
        </Panel>
      )}
      {showHand || table.broken.length || owned ? (
        <Panel>
          <div className="grid">
            {showHand ? (
              <InfoLine
                icon={STACK_ICON}
                actions={
                  <Button disabled={busy} onClick={() => skills.setDialog({ kind: 'import', picked: hand.map((r) => r.name) })}>
                    {t('Gom về…', 'Gather…')}
                  </Button>
                }
              >
                {t(
                  hand.length + ' skill cài tay đang nằm rải ở thư mục riêng của từng agent. Gom về một chỗ cho gọn, ba agent dùng chung một bản.',
                  hand.length +
                    (hand.length === 1 ? ' skill installed by hand sits' : ' skills installed by hand sit') +
                    ' in each agent’s own folder. Gather them in one place so the three agents share one copy.',
                )}
              </InfoLine>
            ) : null}
            {table.broken.map((b) => (
              <InfoLine
                key={b.agent + b.name}
                icon={NAV_LINK}
                actions={
                  <LinkButton small disabled={busy} onClick={() => skills.setDialog({ kind: 'broken', agent: b.agent, name: b.name })}>
                    {t('Sửa liên kết', 'Fix link')}
                  </LinkButton>
                }
              >
                <b>{b.name}</b> {t('ở ' + AGENT_NAME[b.agent] + ' trỏ tới một thư mục không còn.', 'in ' + AGENT_NAME[b.agent] + ' points to a folder that is gone.')}
              </InfoLine>
            ))}
            {owned ? (
              <InfoLine
                icon={PUZZLE_ICON}
                actions={
                  <LinkButton small onClick={() => skills.setDialog({ kind: 'owned' })}>
                    {t('Xem', 'View')}
                  </LinkButton>
                }
              >
                {t(
                  owned + ' skill có sẵn trong agent và plugin. App để yên, không sửa.',
                  owned + (owned === 1 ? ' skill comes' : ' skills come') + ' built into agents and plugins. The app leaves them alone.',
                )}
              </InfoLine>
            ) : null}
          </div>
        </Panel>
      ) : null}
    </>
  );
}

/** A newer version: an arrow up. */
const NEWER_ICON = (
  <svg width="15" height="15" viewBox="0 0 16 16" aria-hidden="true">
    <path d="M8 12.5v-9M4.5 7L8 3.5 11.5 7" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

/** A link to nothing. */
const NAV_LINK = (
  <svg width="15" height="15" viewBox="0 0 16 16" aria-hidden="true">
    <path d="M6.5 9.5l3-3M7 4.5l1-1a2.5 2.5 0 013.5 3.5l-1 1M9 11.5l-1 1A2.5 2.5 0 014.5 9l1-1" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
  </svg>
);
