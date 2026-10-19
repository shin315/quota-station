// The Bộ cấu hình tab of the main window (M15, N), as the approved
// prototype's `set` screens: the set in use and how far the Mac has drifted
// from it, the sets with Áp…, Sửa and Xoá, and their dialogs. A set says,
// for each MCP server and skill, Bật, Tắt or Không đổi, and for Codex
// Giữ nguyên or a side of the Codex switch; it is applied for the whole Mac
// after one preview by agent, and Codex changes only through the Codex
// switch of version 1, opened as step 2. Frosted panels on the window's
// glass, no panel inside one.

import { Button, ChoiceGroup, Chip, Dot, Feedback } from '../components/parts';
import { SHEET_TITLE_ID } from '../components/Sheets';
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
      <div className="card">
        <h4>Đang dùng</h4>
        <p style={{ margin: 0 }}>Chưa áp bộ nào.</p>
      </div>
    );
  const chip = st.lines ? driftChip(st.lines.length) : null;
  return (
    <div className="card">
      <h4>Đang dùng</h4>
      <div className="row2">
        <div>
          <b style={{ fontSize: 'var(--fs-title)' }}>{a.name}</b>
          <small className="sline">
            áp lúc {dayClock(a.at, now)}
            {st.lines?.length ? ' · sau đó ' + st.lines.join(' · ') : ''}
          </small>
        </div>
        <span className="a">
          {chip ? <Chip chip={chip} /> : null}
          {a.canUndo ? (
            <button type="button" className="lnk sm" disabled={sets.working} onClick={sets.undo}>
              Hoàn tác bộ
            </button>
          ) : null}
        </span>
      </div>
    </div>
  );
}

function SetRow({ set, model, sets }: { set: ConfigSet; model: Model; sets: Sets }) {
  const inUse = model.sets.applied?.id === set.id;
  return (
    <div className="set">
      <div>
        <b>
          {set.name} {inUse ? <Chip chip={{ t: 'đang dùng', k: 'info' }} /> : null}
        </b>
        <small>{setSummary(set, model.codex)}</small>
      </div>
      <div className="a">
        <Button primary={!inUse} disabled={sets.working} label={'Áp bộ ' + set.name} onClick={() => sets.openApply(set)}>
          Áp…
        </Button>
        <button type="button" className="lnk sm" aria-label={'Sửa bộ ' + set.name} disabled={sets.working} onClick={() => sets.openEdit(set)}>
          Sửa
        </button>
        <button type="button" className="lnk sm" aria-label={'Xoá bộ ' + set.name} disabled={sets.working} onClick={() => sets.remove(set)}>
          Xoá
        </button>
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
      <div className="tb" data-tauri-drag-region="deep">
        <h2>Bộ cấu hình</h2>
        <small>
          <Dot tone={fresh.tone} />
          {model.machine} · áp cho cả máy
        </small>
        <Button primary disabled={sets.working} onClick={() => sets.openNew(!list.length)}>
          Tạo bộ
        </Button>
      </div>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {!list.length ? (
        <div className="card">
          <div className="empty">
            <span className="ei" aria-hidden="true">
              {STACK_BIG}
            </span>
            <b>Chưa có bộ nào</b>
            <p className="note">Một bộ ghi MCP và skills nào bật, nào tắt, để đổi cả loạt khi chuyển sang dự án khác.</p>
            <div className="btns">
              <Button primary disabled={sets.working} onClick={() => sets.openNew(true)}>
                Tạo bộ từ cấu hình hiện tại
              </Button>
            </div>
          </div>
        </div>
      ) : (
        <>
          <InUse model={model} sets={sets} now={now} />
          <div className="card">
            <h4>
              Các bộ <span className="cnt">{list.length}</span>
            </h4>
            {list.map((s) => (
              <SetRow key={s.id} set={s} model={model} sets={sets} />
            ))}
          </div>
          <div className="card">
            <Facts
              items={[
                ['', 'Áp cho cả máy: mọi dự án, mọi phiên mở sau đó. Phiên đang chạy giữ cấu hình cũ.'],
                ['', 'Bộ chỉ bật, tắt MCP và skills đã có. Codex đổi qua tấm chuyển Codex như bình thường.'],
              ]}
            />
          </div>
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
    <div className="seg inline" role="group" aria-label={name}>
      {WANTS.map((w) => (
        <button key={w.v} type="button" aria-pressed={cur === w.v} onClick={() => onChange(w.v === 'keep' ? undefined : w.v)}>
          {w.t}
        </button>
      ))}
    </div>
  );
}

function Entries(props: { names: string[]; here: Set<string>; entries: Record<string, SetWant>; onChange(e: Record<string, SetWant>): void }) {
  return (
    <div className="sedit">
      {props.names.map((n) => (
        <div key={n} className="se">
          <span>
            {n}
            {props.here.has(n) ? null : <small>máy này không có</small>}
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
      <h3 id={SHEET_TITLE_ID}>{isNew ? 'Tạo bộ' : 'Sửa bộ ' + s.name}</h3>
      <div className="row2" style={{ marginBottom: 10 }}>
        <input
          className="inp"
          aria-label="Tên bộ"
          placeholder="Tên dự án, ví dụ Web app"
          value={s.name}
          maxLength={60}
          style={{ maxWidth: 260 }}
          onChange={(e) => sets.setDraft({ ...s, name: e.target.value })}
        />
        <button type="button" className="lnk sm" disabled={!model.mcp.table && !model.skills.table} onClick={sets.takeCurrent}>
          Lấy từ cấu hình hiện tại
        </button>
      </div>
      <div className="sec-t">
        {NAV_ICONS.mcp}MCP · áp cho mọi agent đang có MCP đó
      </div>
      {mcp.names.length ? (
        <Entries names={mcp.names} here={mcp.here} entries={s.mcp} onChange={(e) => sets.setDraft({ ...s, mcp: e })} />
      ) : (
        <p className="lead">{model.mcp.table ? 'Máy này chưa có MCP nào.' : 'Đang đọc danh sách MCP…'}</p>
      )}
      <div className="sec-t">{NAV_ICONS.skills}Skills · áp cho mọi agent bật được</div>
      {sk.names.length ? (
        <Entries names={sk.names} here={sk.here} entries={s.skills} onChange={(e) => sets.setDraft({ ...s, skills: e })} />
      ) : (
        <p className="lead">{model.skills.table ? 'Máy này chưa có skill nào.' : 'Đang đọc danh sách skills…'}</p>
      )}
      {cx.present ? (
        <div className="setcx">
          <ChoiceGroup id="set-cx" label="Codex" options={options} value={s.codex ?? 'keep'} onChange={(v) => sets.setDraft({ ...s, codex: v === 'keep' ? null : v })} />
        </div>
      ) : null}
      {d.error ? <p className="note bad">{d.error}</p> : null}
      <div className="btns">
        <Button large disabled={sets.working} onClick={sets.close}>
          Huỷ
        </Button>
        <Button large primary disabled={sets.working} onClick={sets.save}>
          {sets.working ? 'Đang lưu…' : 'Lưu bộ'}
        </Button>
      </div>
    </>
  );
}

function Details({ p }: { p: SetPreview }) {
  if (!p.mcp.groups.length && !p.skills.groups.length) return null;
  return (
    <details className="more">
      <summary>Xem từng dòng sẽ ghi</summary>
      {p.mcp.groups.map((g) => (
        <section key={'m' + g.agent} aria-label={'MCP của ' + AGENT_NAME[g.agent]}>
          <div className="sec-t">
            <AgentLogo agent={g.agent} />
            MCP · {AGENT_NAME[g.agent]}
          </div>
          {g.blocks.map((b) => (
            <BlockView key={b.name + b.action} agent={g.agent} file={g.file} block={b} />
          ))}
        </section>
      ))}
      {p.skills.groups.map((g) => (
        <section key={'s' + g.agent} aria-label={'Skills của ' + AGENT_NAME[g.agent]}>
          <div className="sec-t">
            <AgentLogo agent={g.agent} />
            Skills · {AGENT_NAME[g.agent]}
          </div>
          <Lines head={g.file} right={g.agent === 'codex' ? g.count + ' skill' : g.count + ' liên kết'} lines={g.lines} />
        </section>
      ))}
    </details>
  );
}

function ApplyBody({ d, sets, model }: { d: Extract<SetDialog, { kind: 'apply' }>; sets: Sets; model: Model }) {
  const p = d.preview;
  const step = codexStep(d.set, model.codex);
  const n = p ? changeCount(p) : 0;
  const skipped = p ? [...p.skipped.map(skippedLine), ...refusedLines(p)] : [];
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Áp bộ {d.set.name}?</h3>
      {!p && !d.error ? <p className="lead">Đang đọc file của các agent…</p> : null}
      {p ? (
        <div className="sapply">
          {byAgent(p).map((r) => (
            <div key={r.agent} className="sa">
              <span>
                <AgentLogo agent={r.agent} />
                {AGENT_NAME[r.agent]}
              </span>
              <span>
                {r.mcp.length ? <span className="l">MCP: {r.mcp.join(', ')}</span> : null}
                {r.skills.length ? <span className="l">Skills: {r.skills.join(', ')}</span> : null}
              </span>
            </div>
          ))}
          {step ? (
            <div className="sa">
              <span>
                <AgentLogo agent="codex" />
                Nhà cung cấp
              </span>
              <span>Bước 2: mở tấm chuyển Codex về {sideName(step, model.codex)}</span>
            </div>
          ) : null}
          {!n && !step ? <p style={{ margin: 0 }}>Máy đang đúng như bộ này. Không có gì để ghi.</p> : null}
        </div>
      ) : null}
      {p ? <Details p={p} /> : null}
      {p ? (
        <div style={{ marginTop: 10 }}>
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
      {d.error ? <p className="note bad">{d.error}</p> : null}
      <div className="btns">
        <Button large disabled={sets.working && !!p} onClick={sets.close}>
          Huỷ
        </Button>
        <Button large primary disabled={sets.working || !p || (!n && !step)} onClick={sets.apply}>
          {sets.working && p ? 'Đang ghi…' : !n && step ? 'Tiếp: chuyển Codex' : 'Áp bộ'}
        </Button>
      </div>
    </>
  );
}

export function SetDialogBody({ sets, model }: { sets: Sets; model: Model }) {
  const d = sets.dialog;
  if (!d) return null;
  if (d.kind === 'edit') return <EditBody d={d} sets={sets} model={model} />;
  return <ApplyBody d={d} sets={sets} model={model} />;
}
