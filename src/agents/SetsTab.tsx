// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Bộ cấu hình tab of the main window (M15, N), as the approved
// prototype's `set` screens: the set in use and how far the Mac has drifted
// from it, the sets with Áp…, Sửa and Xoá, and their dialogs. A set says,
// for each MCP server and skill, Bật, Tắt or Không đổi, and for Codex
// Giữ nguyên or a side of the Codex switch; it is applied for the whole Mac
// after one preview by agent, and Codex changes only through the Codex
// switch of version 1, opened as step 2. Frosted panels on the window's
// glass, no panel inside one. As the approved redraw (sets-A, 25/10/2026)
// lays it out: the set in use as a strip on top, then the sets as cards of
// one size, each with its actions on its bottom line, and the two notes on
// the glass under them.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Segment, Segmented } from '@/components/ui/segmented';
import { ROW_ACTIONS } from '../components/blocks';
import { COUNT, Empty, EmptyIcon, TabHead, TabSub, TabTitle } from '../components/page';
import { Button, ChoiceGroup, Chip, Dot, Facts as FactList, Feedback, LinkButton } from '../components/parts';
import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { t } from '../lib/i18n';
import { AGENT_NAME } from '../lib/mcp';
import type { CodexTarget, ConfigSet, Model, SetPreview, SetWant } from '../lib/model';
import {
  appliedState,
  byAgent,
  changeCount,
  codexStep,
  driftChip,
  refusedLines,
  sessionsLine,
  setSummary,
  sideName,
  skippedLine,
} from '../lib/sets';
import { dayClock } from '../lib/switching';
import type { Action, FeedbackView, PopoverView } from '../lib/view';
import { NAV_ICONS, STACK_BIG } from '../window/icons';
import { AgentLogo } from './CheckMatrix';
import { BlockView, Facts } from './McpDialogs';
import { Lines } from './SkillDialogs';
import type { SetDialog, Sets } from './useSets';

export function setDialogSize(d: SetDialog): 'lg' | 'xl' {
  return d.kind === 'edit' ? 'xl' : 'lg';
}

function InUse({ model, sets, now }: { model: Model; sets: Sets; now: Date }) {
  const v = model.sets;
  const a = v.applied;
  const st = appliedState(v, model.codex);
  if (!a || !st)
    return (
      <Panel>
        <CardTitle>{t('Đang dùng', 'In use')}</CardTitle>
        <CardText className="m-0">{t('Chưa áp bộ nào.', 'No set applied yet.')}</CardText>
      </Panel>
    );
  const chip = st.lines ? driftChip(st.lines.length) : null;
  const since = st.lines?.length ? st.lines.join(' · ') : '';
  return (
    <Panel>
      <CardTitle>{t('Đang dùng', 'In use')}</CardTitle>
      <div className="flex items-center justify-between gap-2.5">
        <div>
          <b className="text-title">{a.name}</b>
          <small className="block text-xs text-ink2 [overflow-wrap:anywhere]">
            {since
              ? t('áp lúc ' + dayClock(a.at, now) + ' · sau đó ' + since, 'applied at ' + dayClock(a.at, now) + ' · since then ' + since)
              : t('áp lúc ' + dayClock(a.at, now), 'applied at ' + dayClock(a.at, now))}
          </small>
        </div>
        <span className={ROW_ACTIONS}>
          {chip ? <Chip chip={chip} /> : null}
          {a.canUndo ? (
            <LinkButton small disabled={sets.working} onClick={sets.undo}>
              {t('Hoàn tác bộ', 'Undo the set')}
            </LinkButton>
          ) : null}
        </span>
      </div>
    </Panel>
  );
}

/** The sets: three cards a row, two where the page has under 640 px, one under 420. */
const SET_GRID =
  'grid grid-cols-[repeat(3,minmax(0,1fr))] items-stretch gap-3 @max-[640px]/main:grid-cols-[repeat(2,minmax(0,1fr))] @max-[420px]/main:grid-cols-[minmax(0,1fr)]';

/** A set: its name, what it does, and its actions on the card's bottom line. */
function SetCard({ set, model, sets }: { set: ConfigSet; model: Model; sets: Sets }) {
  const inUse = model.sets.applied?.id === set.id;
  return (
    <Panel className="flex flex-col" role="listitem" aria-label={set.name}>
      <b className="flex flex-wrap items-center gap-1.5 text-body font-semi">
        {set.name} {inUse ? <Chip chip={{ t: t('đang dùng', 'in use'), k: 'info' }} /> : null}
      </b>
      <CardText className="mt-1 mb-2.5">{setSummary(set, model.codex)}</CardText>
      <div className="mt-auto flex items-center gap-2.5 border-t-[.5px] border-card-line pt-2.5">
        <Button primary={!inUse} disabled={sets.working} label={t('Áp bộ ' + set.name, 'Apply the set ' + set.name)} onClick={() => sets.openApply(set)}>
          {t('Áp dụng…', 'Apply…')}
        </Button>
        <span className={'ml-auto ' + ROW_ACTIONS}>
          <LinkButton small aria-label={t('Sửa bộ ' + set.name, 'Edit the set ' + set.name)} disabled={sets.working} onClick={() => sets.openEdit(set)}>
            {t('Sửa', 'Edit')}
          </LinkButton>
          <LinkButton small aria-label={t('Xoá bộ ' + set.name, 'Delete the set ' + set.name)} disabled={sets.working} onClick={() => sets.remove(set)}>
            {t('Xoá', 'Delete')}
          </LinkButton>
        </span>
      </div>
    </Panel>
  );
}

export interface SetsTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  sets: Sets;
  onAction(act: Action): void;
}

export function SetsTab({ model, fresh, feedback, now, sets, onAction }: SetsTabProps) {
  const list = model.sets.sets;
  return (
    <>
      <TabHead>
        <TabTitle>{t('Bộ cấu hình', 'Config sets')}</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine + t(' · áp cho cả máy', ' · applies to the whole Mac')}
        </TabSub>
      </TabHead>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {!list.length ? (
        <Panel>
          <Empty>
            <EmptyIcon>{STACK_BIG}</EmptyIcon>
            <b>{t('Chưa có bộ nào', 'No sets yet')}</b>
            <CardText className="mt-2 mb-0">
              {t(
                'Một bộ ghi MCP và skills nào bật, nào tắt, để đổi cả loạt khi chuyển sang dự án khác.',
                'A set records which MCP and skills are on or off, so you can change them all at once for another project.',
              )}
            </CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary disabled={sets.working} onClick={() => sets.openNew(true)}>
                {t('Tạo bộ từ cấu hình hiện tại', 'Create a set from the current setup')}
              </Button>
            </div>
          </Empty>
        </Panel>
      ) : (
        <>
          <InUse model={model} sets={sets} now={now} />
          <div className="flex items-center gap-2.5">
            <CardTitle bare className="m-0">
              {t('Các bộ', 'Sets')} <span className={COUNT}>{list.length}</span>
            </CardTitle>
            <Button primary className="ml-auto" disabled={sets.working} onClick={() => sets.openNew(false)}>
              {t('Tạo bộ', 'New set')}
            </Button>
          </div>
          <div className={SET_GRID} role="list" aria-label={t('Các bộ', 'Sets')}>
            {list.map((s) => (
              <SetCard key={s.id} set={s} model={model} sets={sets} />
            ))}
          </div>
          <div className="px-3.5">
            <FactList
              className="mb-0"
              items={[
                {
                  k: '',
                  text: t(
                    'Áp cho cả máy: mọi dự án, mọi phiên mở sau đó. Phiên đang chạy giữ cấu hình cũ.',
                    'Applies to the whole Mac: every project, every session opened afterwards. Running sessions keep the old setup.',
                  ),
                },
                {
                  k: '',
                  text: t(
                    'Bộ chỉ bật, tắt MCP và skills đã có. Codex đổi qua tấm chuyển Codex như bình thường.',
                    'A set only turns MCP and skills that are already here on or off. Codex changes through the Codex switch sheet as usual.',
                  ),
                },
              ]}
            />
          </div>
        </>
      )}
    </>
  );
}

// ===== The dialogs =====

const wants = (): { v: SetWant | 'keep'; t: string }[] => [
  { v: 'on', t: t('Bật', 'On') },
  { v: 'off', t: t('Tắt', 'Off') },
  { v: 'keep', t: t('Không đổi', 'No change') },
];

function WantSeg({ name, value, onChange }: { name: string; value: SetWant | undefined; onChange(v: SetWant | undefined): void }) {
  const cur = value ?? 'keep';
  return (
    <Segmented variant="inline" className="w-[232px]" aria-label={name}>
      {wants().map((w) => (
        <Segment key={w.v} className="px-1.5 py-[3px]" pressed={cur === w.v} onClick={() => onChange(w.v === 'keep' ? undefined : w.v)}>
          {w.t}
        </Segment>
      ))}
    </Segmented>
  );
}

function Entries(props: { names: string[]; here: Set<string>; entries: Record<string, SetWant>; onChange(e: Record<string, SetWant>): void }) {
  return (
    <div className="mt-0 mb-1 grid">
      {props.names.map((n) => (
        <div key={n} className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3 gap-y-1 border-t-[.5px] border-card-line py-1 text-sm first:border-t-0">
          <span className="[overflow-wrap:anywhere]">
            {n}
            {props.here.has(n) ? null : <small className="block text-xs text-ink3">{t('máy này không có', 'not on this Mac')}</small>}
          </span>
          <WantSeg
            name={n}
            value={props.entries[n]}
            onChange={(w) => {
              const next = { ...props.entries };
              if (w) next[n] = w;
              else delete next[n];
              props.onChange(next);
            }}
          />
        </div>
      ))}
    </div>
  );
}

function names(table: string[], kept: Record<string, SetWant>): { names: string[]; here: Set<string> } {
  const here = new Set(table);
  const extra = Object.keys(kept).filter((n) => !here.has(n));
  return { names: [...table, ...extra.sort()], here };
}

function EditBody({ d, sets, model }: { d: Extract<SetDialog, { kind: 'edit' }>; sets: Sets; model: Model }) {
  const s = d.draft;
  const isNew = !s.id;
  const mcp = names(model.mcp.table?.rows.map((r) => r.name) ?? [], s.mcp);
  const sk = names(model.skills.table?.rows.map((r) => r.name) ?? [], s.skills);
  const cx = model.codex;
  const options: { v: CodexTarget | 'keep'; t: string; s: string; until: null }[] = [
    { v: 'keep', t: t('Giữ nguyên', 'Keep as is'), s: t('không đổi nhà cung cấp', 'provider unchanged'), until: null },
    { v: 'chatGpt', t: t('Tài khoản ChatGPT', 'ChatGPT account'), s: t('qua tấm chuyển Codex', 'through the Codex switch sheet'), until: null },
    ...(cx.custom ? [{ v: 'custom' as const, t: cx.custom.name, s: t('qua tấm chuyển Codex', 'through the Codex switch sheet'), until: null }] : []),
  ];
  return (
    <>
      <DialogTitle>{isNew ? t('Tạo bộ', 'New set') : t('Sửa bộ ' + s.name, 'Edit the set ' + s.name)}</DialogTitle>
      <div className="mb-2.5 flex items-center justify-between gap-2.5">
        <Input
          className="max-w-[260px]"
          aria-label={t('Tên bộ', 'Set name')}
          placeholder={t('Tên dự án, ví dụ Web app', 'Project name, e.g. Web app')}
          value={s.name}
          maxLength={60}
          onChange={(e) => sets.setDraft({ ...s, name: e.target.value })}
        />
        <LinkButton small disabled={!model.mcp.table && !model.skills.table} onClick={sets.takeCurrent}>
          {t('Lấy từ cấu hình hiện tại', 'Take from the current setup')}
        </LinkButton>
      </div>
      <DialogSection>
        {NAV_ICONS.mcp}
        {t('MCP · áp cho mọi agent đang có MCP đó', 'MCP · applies to every agent that has that MCP')}
      </DialogSection>
      {mcp.names.length ? (
        <Entries names={mcp.names} here={mcp.here} entries={s.mcp} onChange={(e) => sets.setDraft({ ...s, mcp: e })} />
      ) : (
        <DialogLead>{model.mcp.table ? t('Máy này chưa có MCP nào.', 'This Mac has no MCP yet.') : t('Đang đọc danh sách MCP…', 'Reading the MCP list…')}</DialogLead>
      )}
      <DialogSection>
        {NAV_ICONS.skills}
        {t('Skills · áp cho mọi agent bật được', 'Skills · applies to every agent that can turn them on')}
      </DialogSection>
      {sk.names.length ? (
        <Entries names={sk.names} here={sk.here} entries={s.skills} onChange={(e) => sets.setDraft({ ...s, skills: e })} />
      ) : (
        <DialogLead>{model.skills.table ? t('Máy này chưa có skill nào.', 'This Mac has no skills yet.') : t('Đang đọc danh sách skills…', 'Reading the skills list…')}</DialogLead>
      )}
      {cx.present ? (
        <div>
          <ChoiceGroup
            id="set-cx"
            labelClassName="mt-2.5 flex items-center gap-1.5 text-xs font-bold text-ink2"
            label="Codex" options={options} value={s.codex ?? 'keep'} onChange={(v) => sets.setDraft({ ...s, codex: v === 'keep' ? null : v })} />
        </div>
      ) : null}
      {d.error ? <p>{d.error}</p> : null}
      <DialogButtons>
        <Button large disabled={sets.working} onClick={sets.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={sets.working} onClick={sets.save}>
          {sets.working ? t('Đang lưu…', 'Saving…') : t('Lưu bộ', 'Save set')}
        </Button>
      </DialogButtons>
    </>
  );
}

function Details({ p }: { p: SetPreview }) {
  if (!p.mcp.groups.length && !p.skills.groups.length) return null;
  return (
    <details className="mt-2 text-sm text-ink2">
      <summary className="cursor-pointer list-none text-accent-ink [&::-webkit-details-marker]:hidden">{t('Xem từng dòng sẽ ghi', 'See each line to be written')}</summary>
      {p.mcp.groups.map((g) => (
        <section key={'m' + g.agent} aria-label={t('MCP của ' + AGENT_NAME[g.agent], AGENT_NAME[g.agent] + '’s MCP')}>
          <DialogSection>
            <AgentLogo agent={g.agent} />
            MCP · {AGENT_NAME[g.agent]}
          </DialogSection>
          {g.blocks.map((b) => (
            <BlockView key={b.name + b.action} agent={g.agent} file={g.file} block={b} />
          ))}
        </section>
      ))}
      {p.skills.groups.map((g) => (
        <section key={'s' + g.agent} aria-label={t('Skills của ' + AGENT_NAME[g.agent], AGENT_NAME[g.agent] + '’s skills')}>
          <DialogSection>
            <AgentLogo agent={g.agent} />
            Skills · {AGENT_NAME[g.agent]}
          </DialogSection>
          <Lines head={g.file} right={
              g.agent === 'codex'
                ? t(g.count + ' skill', g.count + (g.count === 1 ? ' skill' : ' skills'))
                : t(g.count + ' liên kết', g.count + (g.count === 1 ? ' link' : ' links'))
            } lines={g.lines} />
        </section>
      ))}
    </details>
  );
}

/** What a set changes for one agent: the agent, then the lines. */
const APPLY_ROW =
  'grid grid-cols-[130px_minmax(0,1fr)] gap-x-3 gap-y-1 border-t-[.5px] border-card-line py-2 text-sm first:border-t-0 first:pt-0.5 max-[640px]:grid-cols-[minmax(0,1fr)]';
const APPLY_AGENT = 'flex items-center gap-1.5 text-ink2';

function ApplyBody({ d, sets, model }: { d: Extract<SetDialog, { kind: 'apply' }>; sets: Sets; model: Model }) {
  const p = d.preview;
  const step = codexStep(d.set, model.codex);
  const n = p ? changeCount(p) : 0;
  const skipped = p ? [...p.skipped.map(skippedLine), ...refusedLines(p)] : [];
  return (
    <>
      <DialogTitle>{t('Áp bộ ' + d.set.name + '?', 'Apply the set ' + d.set.name + '?')}</DialogTitle>
      {!p && !d.error ? <DialogLead>{t('Đang đọc file của các agent…', 'Reading the agents’ files…')}</DialogLead> : null}
      {p ? (
        <div className="mt-0 mb-1 grid">
          {byAgent(p).map((r) => (
            <div key={r.agent} className={APPLY_ROW}>
              <span className={APPLY_AGENT}>
                <AgentLogo agent={r.agent} />
                {AGENT_NAME[r.agent]}
              </span>
              <span>
                {r.mcp.length ? <span className="block [overflow-wrap:anywhere]">MCP: {r.mcp.join(', ')}</span> : null}
                {r.skills.length ? <span className="block [overflow-wrap:anywhere]">Skills: {r.skills.join(', ')}</span> : null}
              </span>
            </div>
          ))}
          {step ? (
            <div className={APPLY_ROW}>
              <span className={APPLY_AGENT}>
                <AgentLogo agent="codex" />
                {t('Nhà cung cấp', 'Provider')}
              </span>
              <span>{t('Bước 2: mở tấm chuyển Codex về ' + sideName(step, model.codex), 'Step 2: open the Codex switch sheet for ' + sideName(step, model.codex))}</span>
            </div>
          ) : null}
          {!n && !step ? <p className="m-0">{t('Máy đang đúng như bộ này. Không có gì để ghi.', 'This Mac already matches this set. Nothing to write.')}</p> : null}
        </div>
      ) : null}
      {p ? <Details p={p} /> : null}
      {p ? (
        <div className="mt-2.5">
          <Facts
            items={[
              ...skipped.map((line): [string, string] => ['x', line]),
              ['w', sessionsLine(d.sessions, model.codex)],
              ['w', t('Áp cho cả máy, mọi dự án.', 'Applies to the whole Mac, every project.')],
              ...(n
                ? [
                    [
                      'ok',
                      t(
                        'Sao lưu file của agent trước khi ghi. Chỉ sửa đúng các mục của bộ. Hoàn tác được cả bộ.',
                        'Backs up the agents’ files before writing. Only edits the set’s own entries. The whole set can be undone.',
                      ),
                    ] as [string, string],
                  ]
                : []),
              ...(step ? [['', t('Codex chỉ đổi nhà cung cấp qua tấm chuyển Codex, ở bước 2.', 'Codex only changes provider through the Codex switch sheet, in step 2.')] as [string, string]] : []),
            ]}
          />
        </div>
      ) : null}
      {d.error ? <p>{d.error}</p> : null}
      <DialogButtons>
        <Button large disabled={sets.working && !!p} onClick={sets.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={sets.working || !p || (!n && !step)} onClick={sets.apply}>
          {sets.working && p ? t('Đang ghi…', 'Writing…') : !n && step ? t('Tiếp: chuyển Codex', 'Next: switch Codex') : t('Áp bộ', 'Apply set')}
        </Button>
      </DialogButtons>
    </>
  );
}

export function SetDialogBody({ sets, model }: { sets: Sets; model: Model }) {
  const d = sets.dialog;
  if (!d) return null;
  if (d.kind === 'edit') return <EditBody d={d} sets={sets} model={model} />;
  return <ApplyBody d={d} sets={sets} model={model} />;
}
