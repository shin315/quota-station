// The Bộ cấu hình tab of the main window (M15, N), as the approved
// prototype's `set` screens: the set in use and how far the Mac has drifted
// from it, the sets with Áp…, Sửa and Xoá, and their dialogs. A set says,
// for each MCP server and skill, Bật, Tắt or Không đổi, and for Codex
// Giữ nguyên or a side of the Codex switch; it is applied for the whole Mac
// after one preview by agent, and Codex changes only through the Codex
// switch of version 1, opened as step 2. Frosted panels on the window's
// glass, no panel inside one.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Segment, Segmented } from '@/components/ui/segmented';
import { ROW_ACTIONS, SETTING_ROW } from '../components/blocks';
import { COUNT, Empty, EmptyIcon, TabHead, TabSub, TabTitle } from '../components/page';
import { Button, ChoiceGroup, Chip, Dot, Feedback, LinkButton } from '../components/parts';
import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
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
        <CardTitle>Đang dùng</CardTitle>
        <CardText className="m-0">Chưa áp bộ nào.</CardText>
      </Panel>
    );
  const chip = st.lines ? driftChip(st.lines.length) : null;
  return (
    <Panel>
      <CardTitle>Đang dùng</CardTitle>
      <div className="flex items-center justify-between gap-2.5">
        <div>
          <b className="text-title">{a.name}</b>
          <small className="block text-xs text-ink2 [overflow-wrap:anywhere]">
            áp lúc {dayClock(a.at, now)}
            {st.lines?.length ? ' · sau đó ' + st.lines.join(' · ') : ''}
          </small>
        </div>
        <span className={ROW_ACTIONS}>
          {chip ? <Chip chip={chip} /> : null}
          {a.canUndo ? (
            <LinkButton small disabled={sets.working} onClick={sets.undo}>
              Hoàn tác bộ
            </LinkButton>
          ) : null}
        </span>
      </div>
    </Panel>
  );
}

function SetRow({ set, model, sets }: { set: ConfigSet; model: Model; sets: Sets }) {
  const inUse = model.sets.applied?.id === set.id;
  return (
    <div className={SETTING_ROW}>
      <div>
        <b>
          {set.name} {inUse ? <Chip chip={{ t: 'đang dùng', k: 'info' }} /> : null}
        </b>
        <small>{setSummary(set, model.codex)}</small>
      </div>
      <div className={ROW_ACTIONS}>
        <Button primary={!inUse} disabled={sets.working} label={'Áp bộ ' + set.name} onClick={() => sets.openApply(set)}>
          Áp…
        </Button>
        <LinkButton small aria-label={'Sửa bộ ' + set.name} disabled={sets.working} onClick={() => sets.openEdit(set)}>
          Sửa
        </LinkButton>
        <LinkButton small aria-label={'Xoá bộ ' + set.name} disabled={sets.working} onClick={() => sets.remove(set)}>
          Xoá
        </LinkButton>
      </div>
    </div>
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
        <TabTitle>Bộ cấu hình</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · áp cho cả máy
        </TabSub>
        <Button primary disabled={sets.working} onClick={() => sets.openNew(!list.length)}>
          Tạo bộ
        </Button>
      </TabHead>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {!list.length ? (
        <Panel>
          <Empty>
            <EmptyIcon>{STACK_BIG}</EmptyIcon>
            <b>Chưa có bộ nào</b>
            <CardText className="mt-2 mb-0">Một bộ ghi MCP và skills nào bật, nào tắt, để đổi cả loạt khi chuyển sang dự án khác.</CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary disabled={sets.working} onClick={() => sets.openNew(true)}>
                Tạo bộ từ cấu hình hiện tại
              </Button>
            </div>
          </Empty>
        </Panel>
      ) : (
        <>
          <InUse model={model} sets={sets} now={now} />
          <Panel>
            <CardTitle>
              Các bộ <span className={COUNT}>{list.length}</span>
            </CardTitle>
            {list.map((s) => (
              <SetRow key={s.id} set={s} model={model} sets={sets} />
            ))}
          </Panel>
          <Panel>
            <Facts
              items={[
                ['', 'Áp cho cả máy: mọi dự án, mọi phiên mở sau đó. Phiên đang chạy giữ cấu hình cũ.'],
                ['', 'Bộ chỉ bật, tắt MCP và skills đã có. Codex đổi qua tấm chuyển Codex như bình thường.'],
              ]}
            />
          </Panel>
        </>
      )}
    </>
  );
}

// ===== The dialogs =====

const WANTS: { v: SetWant | 'keep'; t: string }[] = [
  { v: 'on', t: 'Bật' },
  { v: 'off', t: 'Tắt' },
  { v: 'keep', t: 'Không đổi' },
];

function WantSeg({ name, value, onChange }: { name: string; value: SetWant | undefined; onChange(v: SetWant | undefined): void }) {
  const cur = value ?? 'keep';
  return (
    <Segmented variant="inline" className="w-[232px]" aria-label={name}>
      {WANTS.map((w) => (
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
            {props.here.has(n) ? null : <small className="block text-xs text-ink3">máy này không có</small>}
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
    { v: 'keep', t: 'Giữ nguyên', s: 'không đổi nhà cung cấp', until: null },
    { v: 'chatGpt', t: 'Tài khoản ChatGPT', s: 'qua tấm chuyển Codex', until: null },
    ...(cx.custom ? [{ v: 'custom' as const, t: cx.custom.name, s: 'qua tấm chuyển Codex', until: null }] : []),
  ];
  return (
    <>
      <DialogTitle>{isNew ? 'Tạo bộ' : 'Sửa bộ ' + s.name}</DialogTitle>
      <div className="mb-2.5 flex items-center justify-between gap-2.5">
        <Input
          className="max-w-[260px]"
          aria-label="Tên bộ"
          placeholder="Tên dự án, ví dụ Web app"
          value={s.name}
          maxLength={60}
          onChange={(e) => sets.setDraft({ ...s, name: e.target.value })}
        />
        <LinkButton small disabled={!model.mcp.table && !model.skills.table} onClick={sets.takeCurrent}>
          Lấy từ cấu hình hiện tại
        </LinkButton>
      </div>
      <DialogSection>
        {NAV_ICONS.mcp}MCP · áp cho mọi agent đang có MCP đó
      </DialogSection>
      {mcp.names.length ? (
        <Entries names={mcp.names} here={mcp.here} entries={s.mcp} onChange={(e) => sets.setDraft({ ...s, mcp: e })} />
      ) : (
        <DialogLead>{model.mcp.table ? 'Máy này chưa có MCP nào.' : 'Đang đọc danh sách MCP…'}</DialogLead>
      )}
      <DialogSection>{NAV_ICONS.skills}Skills · áp cho mọi agent bật được</DialogSection>
      {sk.names.length ? (
        <Entries names={sk.names} here={sk.here} entries={s.skills} onChange={(e) => sets.setDraft({ ...s, skills: e })} />
      ) : (
        <DialogLead>{model.skills.table ? 'Máy này chưa có skill nào.' : 'Đang đọc danh sách skills…'}</DialogLead>
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
          Huỷ
        </Button>
        <Button large primary disabled={sets.working} onClick={sets.save}>
          {sets.working ? 'Đang lưu…' : 'Lưu bộ'}
        </Button>
      </DialogButtons>
    </>
  );
}

function Details({ p }: { p: SetPreview }) {
  if (!p.mcp.groups.length && !p.skills.groups.length) return null;
  return (
    <details className="mt-2 text-sm text-ink2">
      <summary className="cursor-pointer list-none text-accent-ink [&::-webkit-details-marker]:hidden">Xem từng dòng sẽ ghi</summary>
      {p.mcp.groups.map((g) => (
        <section key={'m' + g.agent} aria-label={'MCP của ' + AGENT_NAME[g.agent]}>
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
        <section key={'s' + g.agent} aria-label={'Skills của ' + AGENT_NAME[g.agent]}>
          <DialogSection>
            <AgentLogo agent={g.agent} />
            Skills · {AGENT_NAME[g.agent]}
          </DialogSection>
          <Lines head={g.file} right={g.agent === 'codex' ? g.count + ' skill' : g.count + ' liên kết'} lines={g.lines} />
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
      <DialogTitle>Áp bộ {d.set.name}?</DialogTitle>
      {!p && !d.error ? <DialogLead>Đang đọc file của các agent…</DialogLead> : null}
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
                Nhà cung cấp
              </span>
              <span>Bước 2: mở tấm chuyển Codex về {sideName(step, model.codex)}</span>
            </div>
          ) : null}
          {!n && !step ? <p className="m-0">Máy đang đúng như bộ này. Không có gì để ghi.</p> : null}
        </div>
      ) : null}
      {p ? <Details p={p} /> : null}
      {p ? (
        <div className="mt-2.5">
          <Facts
            items={[
              ...skipped.map((t): [string, string] => ['x', t]),
              ['w', sessionsLine(d.sessions, model.codex)],
              ['w', 'Áp cho cả máy, mọi dự án.'],
              ...(n ? [['ok', 'Sao lưu file của agent trước khi ghi. Chỉ sửa đúng các mục của bộ. Hoàn tác được cả bộ.'] as [string, string]] : []),
              ...(step ? [['', 'Codex chỉ đổi nhà cung cấp qua tấm chuyển Codex, ở bước 2.'] as [string, string]] : []),
            ]}
          />
        </div>
      ) : null}
      {d.error ? <p>{d.error}</p> : null}
      <DialogButtons>
        <Button large disabled={sets.working && !!p} onClick={sets.close}>
          Huỷ
        </Button>
        <Button large primary disabled={sets.working || !p || (!n && !step)} onClick={sets.apply}>
          {sets.working && p ? 'Đang ghi…' : !n && step ? 'Tiếp: chuyển Codex' : 'Áp bộ'}
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
