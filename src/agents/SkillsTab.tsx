// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Skills tab of the main window (M11, B and G): every skill of Claude
// Code, Codex and Antigravity in the MCP tab's matrix, a checkbox per agent,
// "Cài từ GitHub" at the table's head (skills-R2, 25/10/2026). Ticks wait on
// the pending bar until one preview writes them all. Above the table, one
// card for the newer versions found on GitHub (the app never updates on its
// own): all of them, or a repository's set, at once through one preview;
// under it, on the window's glass, one line each for the skills installed
// by hand to gather, the links to nothing, and the skills agents and
// plugins bring.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Segment, Segmented } from '@/components/ui/segmented';
import { CARD_END, COUNT, Empty, EmptyIcon, InfoLine, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, Dot, Feedback, LinkButton, More } from '../components/parts';
import { mac, t } from '../lib/i18n';
import { AGENT_NAME, columnMenu, pendingList, shown } from '../lib/mcp';
import type { Agent, Model, SkillRow, SkillTable } from '../lib/model';
import { checkedLine, FILTERS, filterCounts, filterWord, filterRows, pendingSkillText, skillBanners, skillCellTitle, skillChips, sourceLine, updateGroups, type SkillFilter } from '../lib/skills';
import type { Action, FeedbackView, PopoverView } from '../lib/view';
import { NoteGlyph } from '../components/icons';
import { SPARK_BIG } from '../window/icons';
import { CheckMatrix, MATRIX_TITLE, PendingBar, type MatrixCell, type MatrixColumn } from './CheckMatrix';
import type { Skills } from './useSkills';

function columns(table: SkillTable): MatrixColumn[] {
  return table.columns.map((c) => ({
    agent: c.agent,
    open: c.writable && table.rows.some((r) => !r.cells[c.agent].lock),
    note: !c.present
      ? mac(t('chưa cài trên máy này', 'not installed on this Mac'))
      : c.state === 'invalid' || c.state === 'unreadable'
        ? t('file lỗi, chỉ xem', 'file has errors, view only')
        : undefined,
  }));
}

function cellOf(table: SkillTable, skills: Skills, row: SkillRow, agent: Agent): MatrixCell {
  const col = table.columns.find((c) => c.agent === agent);
  if (col && !col.present) return { checked: null, mark: '—', pending: false, disabled: true, title: mac(t(AGENT_NAME[agent] + ' chưa cài trên máy này', AGENT_NAME[agent] + ' is not installed on this Mac')) };
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

/**
 * The newer versions found on GitHub, one card: how many, "Cập nhật tất cả",
 * then a line per repository with its skills and "Cập nhật cả bộ". Nothing
 * updates before its preview.
 */
function UpdateCard({ rows, skills, progress }: { rows: SkillRow[]; skills: Skills; progress: Model['skills']['progress'] }) {
  const upd = rows.filter((r) => r.update);
  if (!upd.length) return null;
  const busy = skills.working;
  const sets = updateGroups(rows);
  // A skill whose repository is not known has no set: it is only in "Cập nhật tất cả".
  const loose = upd.filter((r) => !r.repo);
  const view = (name: string) => (
    <LinkButton key={name} small disabled={busy} onClick={() => skills.setDialog({ kind: 'update', name })}>
      {t('Xem ' + name, 'View ' + name)}
    </LinkButton>
  );
  return (
    <section className="rounded-card bg-accent/12 px-3.5 py-3" aria-labelledby="qs-skill-updates">
      <div className="flex flex-wrap items-start gap-x-3 gap-y-2">
        <div className="min-w-0 flex-1">
          <b id="qs-skill-updates" className="block text-body font-bold text-info-ink">
            {t(upd.length + ' skill có bản mới', upd.length === 1 ? '1 skill has an update' : upd.length + ' skills have updates')}
          </b>
          {/* A div, not a p: the popover's paragraphs open inside it. */}
          <div className="mt-0.5 text-sm text-ink2">
            {t('Chỉ cập nhật sau khi bạn xem trước. ', 'Updates only after you preview them. ')}
            <More about={t('cập nhật skill', 'skill updates')}>
              <p>
                {t(
                  'App không tự cập nhật. Mỗi lần cập nhật mở bản xem trước; chỉ khi bạn bấm Cập nhật, app mới chạy npx skills update.',
                  'The app never updates on its own. Each update opens a preview first; only when you press Update does the app run npx skills update.',
                )}
              </p>
              <p>
                {t(
                  'Cập nhật cả bộ gồm mọi skill có bản mới của repo đó, trong một bản xem trước; bỏ tick skill nào muốn để sau.',
                  'Update the set takes every skill with an update from that repository in one preview; untick any you want to leave for later.',
                )}
              </p>
              <p>
                {mac(
                  t(
                    'Skill bạn đã sửa trên máy để chưa tick, để giữ phần bạn sửa. Tick thì bản mới thay phần sửa; app lưu lại bản đang có để lấy lại.',
                    'A skill you edited on this Mac starts unticked, to keep your edits. Ticked, the new version replaces them; the app keeps a copy of the current one to get back.',
                  ),
                )}
              </p>
              <p>{t('Agent nào đang bật skill vẫn bật. Không chạy script nào của skill.', 'Agents that have a skill on keep it on. None of the skills’ scripts run.')}</p>
            </More>
          </div>
        </div>
        <Button primary disabled={busy} onClick={() => skills.openUpdateMany(null)}>
          {progress
            ? t('Đang cập nhật ' + (progress.done + 1) + '/' + progress.total + '…', 'Updating ' + (progress.done + 1) + '/' + progress.total + '…')
            : t('Cập nhật tất cả (' + upd.length + ')', 'Update all (' + upd.length + ')')}
        </Button>
      </div>
      <div className="mt-2 grid border-t-[.5px] border-card-line [&>*:first-child]:border-t-0">
        {sets.map((g) => (
          <InfoLine
            key={g.repo}
            icon={<NoteGlyph note="newer" />}
            actions={
              <>
                {g.names.slice(0, 2).map(view)}
                <LinkButton small aria-label={t('Cập nhật cả bộ ' + g.repo, 'Update the whole ' + g.repo + ' set')} disabled={busy} onClick={() => skills.openUpdateMany(g.repo)}>
                  {t('Cập nhật cả bộ', 'Update the set')}
                </LinkButton>
              </>
            }
          >
            <b>{g.repo}</b> · {t(g.names.length + ' skill có bản mới: ', g.names.length + (g.names.length === 1 ? ' skill has an update: ' : ' skills have updates: '))}
            {g.names.join(', ')}
          </InfoLine>
        ))}
        {loose.map((r) => (
          <InfoLine key={r.name} icon={<NoteGlyph note="newer" />} actions={view(r.name)}>
            <b>{r.name}</b> · {t('có bản mới, không rõ repo', 'has an update, repository unknown')}
          </InfoLine>
        ))}
      </div>
    </section>
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
  const top = (
    <TabHead>
      <TabTitle>Skills</TabTitle>
      <TabSub>
        <Dot tone={v.busy === 'reading' || v.updates.checking ? 'stale' : 'ok'} />
        {model.machine} · {sub}
      </TabSub>
      <Button disabled={v.updates.checking} onClick={skills.check}>
        {v.updates.checking ? t('Đang kiểm tra…', 'Checking…') : t('Kiểm tra bản mới', 'Check updates')}
      </Button>
    </TabHead>
  );
  if (!table)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <b>{t('Đang đọc danh sách skill…', 'Reading the skill list…')}</b>
            {mac(t('Claude Code, Codex và Antigravity, trên máy này.', 'Claude Code, Codex and Antigravity, on this Mac.'))}
          </Empty>
        </Panel>
      </>
    );
  const list = filterRows(table.rows, skills.filter);
  const pend = pendingList(skills.pending, table.rows);
  const hand = table.rows.filter((r) => r.importable);
  const owned = table.owned.reduce((n, o) => n + o.names.length, 0);
  const showHand = hand.length > 0 && (skills.filter === 'all' || skills.filter === 'hand');
  const install = (
    <Button primary small disabled={busy || !v.npx} title={v.npx ? undefined : t('Không tìm thấy npx', 'npx not found')} onClick={skills.openGh}>
      {t('Cài từ GitHub', 'Install from GitHub')}
    </Button>
  );
  return (
    <>
      {top}
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {skillBanners(v, table).map((b) => (
        <Banner key={b.title} banner={b} onAction={onAction} />
      ))}
      {v.npx ? <UpdateCard rows={table.rows} skills={skills} progress={v.progress} /> : null}
      {table.rows.length ? (
        <Panel>
          <CardTitle>
            {t('Skills của bạn', 'Your skills')} <span className={COUNT}>{table.rows.length}</span>
            <span className={CARD_END}>{install}</span>
          </CardTitle>
          <Filters rows={table.rows} value={skills.filter} onChange={skills.setFilter} />
          {list.length ? (
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
      {/* On the window's glass, under the table: no card of their own. */}
      {showHand || table.broken.length || owned ? (
        <div className="grid px-3.5">
          {showHand ? (
            <InfoLine
              icon={<NoteGlyph note="gather" />}
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
              icon={<NoteGlyph note="deadLink" />}
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
              icon={<NoteGlyph note="builtIn" />}
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
      ) : null}
    </>
  );
}
