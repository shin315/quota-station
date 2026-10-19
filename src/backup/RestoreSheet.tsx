// The Sao lưu tab's dialogs (M10, M12): "Xem trong gói", the Khôi phục
// sheet in four steps (pick, preview, keys, restore), the undo of a whole
// restore, and replacing or deleting a key the app keeps. The preview has
// the servers, then the skills, plugins and instruction files by group; the
// result has "Thử lại" on a part that failed. As the approved prototype
// after round 2: plain words, no folder path or command; a key is a
// password field that starts empty and is never shown again.

import { useState, type ReactNode } from 'react';
import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { CheckRow, DiffLine, DiffView, GROUP_PANEL, KeyMasked, Progress as ProgressList, SECRET_ROW, Steps as StepList } from '../components/blocks';
import { Button, Chip, Facts as FactList, LinkButton } from '../components/parts';
import {
  anyTicked,
  changedLines,
  fileKey,
  fileLabel,
  fileLine,
  itemKey,
  itemLine,
  keySlot,
  keysFor,
  macOf,
  packLine,
  packNote,
  packWhen,
  packWhenLower,
  partLines,
  partNotes,
  picked,
  PLUGINS,
  pluginGroups,
  pluginKey,
  pluginLine,
  resultLine,
  SKILL_TITLE,
  skillGroups,
  skillLine,
  SKILLS_GITHUB,
  SKILLS_HAND,
  skippedLines,
  todoLines,
  undoLines,
  usable,
  type Ticks,
} from '../lib/backup';
import { AGENT_NAME, joinCommand } from '../lib/mcp';
import type { McpDef, Model, RestoreFileItem, RestoreItem, RestorePlan } from '../lib/model';
import { NAV_ICONS } from '../window/icons';
import { partKey, type Backup, type BackupDialog, type RestoreDialog } from './useBackup';

export function backupDialogSize(d: BackupDialog): 'lg' | 'xl' | 'wide' {
  return d.kind === 'restore' ? 'xl' : 'lg';
}

const STEPS = ['Chọn gói', 'Xem trước', 'Dán khoá', 'Khôi phục'];

function Steps({ at }: { at: number }) {
  return <StepList names={STEPS} at={at} />;
}

export function Facts({ items }: { items: [string, string][] }) {
  return <FactList items={items.map(([k, text]) => ({ k, text }))} />;
}

function Error({ d }: { d: { error: { title: string; body: string } | null } }) {
  if (!d.error) return null;
  return (
    <p className="mt-1 text-xs text-bad-ink" role="alert">
      <b>{d.error.title}.</b> {d.error.body}
    </p>
  );
}

function ViewBody({ d, backup, now }: { d: Extract<BackupDialog, { kind: 'view' }>; backup: Backup; now: Date }) {
  const s = d.summary;
  return (
    <>
      <DialogTitle>{s ? 'Gói ' + packWhenLower(s.at, now) + ' · ' + s.mac : 'Gói sao lưu'}</DialogTitle>
      <Error d={d} />
      {s ? (
        <>
          <DialogSection>Trong gói</DialogSection>
          <FactList
            items={[
              { k: 'ok', key: 'mcp', text: s.servers.length + ' MCP, và agent nào đang bật' },
              ...(s.skills?.length
                ? [{ k: 'ok', key: 'skills', text: <span title={s.skills.join(', ')}>{s.skills.length} skill: skill từ GitHub ghi nguồn, skill cài tay ghi tên</span> }]
                : []),
              ...(s.plugins?.length ? [{ k: 'ok', key: 'plugins', text: <span title={s.plugins.join(', ')}>{s.plugins.length} plugin, và bật hay tắt</span> }] : []),
              ...(s.instructions?.length
                ? [{ k: 'ok', key: 'instr', text: <span title={s.instructions.join(', ')}>{s.instructions.length} file hướng dẫn</span> }]
                : []),
            ]}
          />
          <DialogSection>Khoá cần dán khi khôi phục</DialogSection>
          {s.keys.length ? (
            <Facts items={s.keys.map((k) => ['', k.mcp + (k.bitwarden ? ' · ' + k.bitwarden + ' (tên trong Bitwarden)' : '')])} />
          ) : (
            <Facts items={[['', 'Không có']]} />
          )}
          <DialogSection>Không có trong gói</DialogSection>
          <Facts
            items={[
              ['', 'Khoá, đăng nhập và tài khoản'],
              ['', 'Nhà cung cấp Codex'],
              ['', 'MCP và skill có sẵn của agent, của plugin'],
              ['', 'File hướng dẫn có dòng dạng khoá'],
            ]}
          />
        </>
      ) : !d.error ? (
        <p>Đang đọc gói…</p>
      ) : null}
      <DialogButtons>
        <Button large disabled={!s} onClick={() => backup.openRestore(d.id)}>
          Khôi phục gói này…
        </Button>
        <Button large primary onClick={backup.close}>
          Đóng
        </Button>
      </DialogButtons>
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
    <div className="mt-0 mr-0 mb-2 ml-[26px] grid gap-1.5">
      {item.compare.map((c) => (
        <div key={c.agent} className="grid grid-cols-[90px_minmax(0,1fr)_minmax(0,1fr)] gap-2 text-xs [&_code]:block [&_code]:[font:11px_var(--mono)] [&_code]:text-ink [&_code]:[overflow-wrap:anywhere] [&_small]:block [&_small]:text-ink3">
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
      <DialogTitle>Khôi phục</DialogTitle>
      <Steps at={0} />
      <DialogLead>Các gói trong iCloud Drive.</DialogLead>
      <RadioGroup
        className="mb-2 grid-cols-[minmax(0,1fr)]"
        aria-label="Gói để khôi phục"
        value={d.pack ?? ''}
        onValueChange={(pack) => backup.update((x) => ({ ...x, pack, error: null }))}
      >
        {packs.map((p) => {
          const note = packNote(p);
          return (
            <RadioGroupItem key={p.id} value={p.id} className="disabled:opacity-50" disabled={!usable(p)}>
              <b className="font-semi">{packWhen(p.at, now) + ' · ' + macOf(p, model.machine)}</b>
              <small className="col-start-2 text-xs text-ink2 tabular-nums">{note ?? packLine(p)}</small>
            </RadioGroupItem>
          );
        })}
      </RadioGroup>
      <Error d={d} />
      <DialogButtons>
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={!d.pack || backup.working} onClick={() => backup.next(d)}>
          Tiếp
        </Button>
      </DialogButtons>
    </>
  );
}

/** One line of the preview with its checkbox, as the servers' lines are. */
function Tick({ k, d, backup, title, line, disabled, right }: { k: string; d: RestoreDialog; backup: Backup; title: string; line: string; disabled?: boolean; right?: ReactNode }) {
  return (
    <CheckRow checked={!!d.ticks[k]} disabled={disabled} onChange={(on) => backup.update((x) => ({ ...x, ticks: { ...x.ticks, [k]: on } }))}>
      <span>
        <b>{title}</b>
        <small>{line}</small>
      </span>
      {right ? <span className="flex items-center gap-1.5">{right}</span> : null}
    </CheckRow>
  );
}

function CompareButton({ k, d, backup, label }: { k: string; d: RestoreDialog; backup: Backup; label: string }) {
  return (
    <LinkButton
      small
      aria-label={'So sánh ' + label}
      aria-expanded={d.compare === k}
      onClick={(e) => {
        e.preventDefault();
        backup.update((x) => ({ ...x, compare: x.compare === k ? null : k }));
      }}
    >
      So sánh
    </LinkButton>
  );
}

/** "So sánh" of an instruction file: this Mac's lines against the pack's, keys masked. */
function FileCompare({ f }: { f: RestoreFileItem }) {
  return (
    <DiffView flat head={<span>Trên máy → trong gói</span>} right={f.changed + ' dòng khác'}>
      {f.diff.map((l, i) => (
        <DiffLine key={i} op={l.op}>
          {l.op === 'gap' ? '…' : l.text || ' '}
          {l.secret ? <KeyMasked /> : null}
        </DiffLine>
      ))}
    </DiffView>
  );
}

const Note = ({ children }: { children: ReactNode }) => (
  <p className="mt-1 mb-0.5 text-xs text-ink2">
    {children}
  </p>
);

/** The skills, plugins and instruction files of the preview (M12), each group as the prototype has it. */
function MoreGroups({ d, backup, plan }: { d: RestoreDialog; backup: Backup; plan: RestorePlan }) {
  const sk = skillGroups(plan);
  const pl = pluginGroups(plan);
  const files = plan.files;
  const skills = plan.skills;
  const plugins = plan.plugins;
  return (
    <>
      {skills && (skills.items.length || skills.same.length || skills.leftOut.length) ? (
        <>
          <DialogSection>
            {NAV_ICONS.skills}
            Skills
          </DialogSection>
          <div className={GROUP_PANEL}>
            {sk.github.length ? <Tick k={SKILLS_GITHUB} d={d} backup={backup} title={SKILL_TITLE.github(sk.github.length)} line={skillLine('github', sk.github)} /> : null}
            {sk.hand.length ? <Tick k={SKILLS_HAND} d={d} backup={backup} title={SKILL_TITLE.hand(sk.hand.length)} line={skillLine('hand', sk.hand)} /> : null}
            {!sk.github.length && !sk.hand.length ? <Note>Không có skill nào để khôi phục.</Note> : null}
            {skills.same.length ? <Note>Giống trên máy: {skills.same.join(', ')}.</Note> : null}
          </div>
        </>
      ) : null}
      {plugins && (plugins.items.length || plugins.same.length || plugins.skipped.length) ? (
        <>
          <DialogSection>
            {NAV_ICONS.plugins}
            Plugin
          </DialogSection>
          <div className={GROUP_PANEL}>
            {pl.main.length ? <Tick k={PLUGINS} d={d} backup={backup} title={pl.main.length + ' plugin'} line={pluginLine(pl.main)} /> : null}
            {pl.off.map((p) => (
              <Tick key={pluginKey(p)} k={pluginKey(p)} d={d} backup={backup} title={'Plugin ' + p.id.split('@')[0]} line="Codex · tắt trong gói. Cài lại thì sẽ bật." />
            ))}
            {!pl.main.length && !pl.off.length ? <Note>Không có plugin nào để cài.</Note> : null}
            {plugins.same.length ? <Note>Giống trên máy: {plugins.same.length} plugin.</Note> : null}
          </div>
        </>
      ) : null}
      {files && (files.items.length || files.same.length || files.kept.length) ? (
        <>
          <DialogSection>
            {NAV_ICONS.instr}
            File hướng dẫn
          </DialogSection>
          <div className={GROUP_PANEL}>
            {files.items.map((f) => {
              const k = fileKey(f.path);
              return (
                <div key={k}>
                  <Tick k={k} d={d} backup={backup} title={fileLabel(f.path, f.agent)} line={fileLine(f)} right={f.kind === 'replace' ? <CompareButton k={k} d={d} backup={backup} label={fileLabel(f.path, f.agent)} /> : null} />
                  {d.compare === k ? <FileCompare f={f} /> : null}
                </div>
              );
            })}
            {!files.items.length ? <Note>Không có file nào để ghi.</Note> : null}
            {files.same.length ? <Note>Giống trên máy: {files.same.length} file.</Note> : null}
          </div>
        </>
      ) : null}
    </>
  );
}

function PreviewStep({ d, backup, now }: { d: RestoreDialog; backup: Backup; now: Date }) {
  const plan = d.plan;
  if (!plan) return null;
  const skipped = [...skippedLines(plan), ...partNotes(plan)];
  const keys = keysFor(plan, d.ticks);
  const any = anyTicked(plan, d.ticks as Ticks);
  return (
    <>
      <DialogTitle>Xem trước: gói {packWhenLower(plan.at, now)}</DialogTitle>
      <Steps at={1} />
      <DialogSection>
        {NAV_ICONS.mcp}
        MCP
      </DialogSection>
      <div className={GROUP_PANEL}>
        {plan.items.map((i) => {
          const k = itemKey(i);
          return (
            <div key={k}>
              <Tick
                k={k}
                d={d}
                backup={backup}
                title={i.name}
                line={itemLine(i)}
                right={
                  <>
                    {i.missingCommand ? <Chip chip={{ t: 'chưa cài', k: 'warn' }} /> : null}
                    {i.kind === 'replace' ? <CompareButton k={k} d={d} backup={backup} label={i.name} /> : null}
                  </>
                }
              />
              {d.compare === k ? <Compare item={i} /> : null}
            </div>
          );
        })}
        {!plan.items.length ? <Note>Máy này đã có đủ các MCP trong gói.</Note> : null}
        {plan.same.length ? <Note>Giống trên máy: {plan.same.join(', ')}.</Note> : null}
      </div>
      <MoreGroups d={d} backup={backup} plan={plan} />
      {skipped.length ? <Facts items={skipped.map((t) => ['w', t])} /> : null}
      <Facts
        items={[
          ['ok', 'Không đổi đăng nhập, tài khoản hay nhà cung cấp Codex.'],
          ['ok', 'Lưu lại bản cũ trước khi ghi. Hoàn tác được cả lần khôi phục.'],
        ]}
      />
      <Error d={d} />
      <DialogButtons>
        <Button large onClick={() => backup.back(d)}>
          Quay lại
        </Button>
        <span className="flex-1" />
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
      </DialogButtons>
    </>
  );
}

function KeysStep({ d, backup }: { d: RestoreDialog; backup: Backup }) {
  const plan = d.plan;
  if (!plan) return null;
  const keys = keysFor(plan, d.ticks);
  return (
    <>
      <DialogTitle>Dán khoá từ Bitwarden</DialogTitle>
      <Steps at={2} />
      <DialogLead>{keys.length} khoá, lấy từ Bitwarden. App giữ trên máy này và không hiện lại.</DialogLead>
      {keys.map((k) => {
        const id = keySlot(k);
        return (
          <div key={id} className={SECRET_ROW}>
            <span>
              <b>{k.mcp}</b>
              <small>{k.bitwarden ? 'Trong Bitwarden: ' + k.bitwarden : k.label}</small>
            </span>
            <span className="flex items-center gap-1.5">
              <Input
                className="font-mono tracking-[.12em]"
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
      <DialogButtons>
        <Button large onClick={() => backup.back(d)}>
          Quay lại
        </Button>
        <span className="flex-1" />
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={backup.working} onClick={() => backup.next(d)}>
          Khôi phục
        </Button>
      </DialogButtons>
    </>
  );
}

/** What the restore runs, one line each, while it runs. */
function runningLines(d: RestoreDialog): string[] {
  const plan = d.plan;
  if (!plan) return [];
  const out = picked(plan, d.ticks).map((i) => 'MCP ' + i.name);
  const sk = skillGroups(plan);
  if (d.ticks[SKILLS_GITHUB] && sk.github.length) out.push(SKILL_TITLE.github(sk.github.length));
  if (d.ticks[SKILLS_HAND] && sk.hand.length) out.push(SKILL_TITLE.hand(sk.hand.length));
  const pl = pluginGroups(plan);
  const plugins = (d.ticks[PLUGINS] ? pl.main.length : 0) + pl.off.filter((p) => d.ticks[pluginKey(p)]).length;
  if (plugins) out.push(plugins + ' plugin');
  for (const f of plan.files?.items ?? []) if (d.ticks[fileKey(f.path)]) out.push(fileLabel(f.path, f.agent));
  return out;
}

function RunStep({ d, backup }: { d: RestoreDialog; backup: Backup }) {
  if (d.step === 'run' || !d.done) {
    return (
      <>
        <DialogTitle>Đang khôi phục…</DialogTitle>
        <Steps at={3} />
        <ProgressList busy lines={runningLines(d).map((t): [string, string, string] => ['wait', t, ''])} />
      </>
    );
  }
  const done = d.done;
  const todo = todoLines(done);
  const parts = partLines(done.parts ?? []);
  const partly = !!done.failed || (done.parts ?? []).some((r) => !r.ok);
  const lines: [string, string, string, ReactNode?][] = [
    ...done.items.map(resultLine),
    ...parts.map((l): [string, string, string, ReactNode?] => [
      l.tone,
      l.title,
      l.sub,
      l.retry ? (
        <LinkButton
          small
          aria-label={'Thử lại ' + l.title}
          disabled={backup.working}
          onClick={() => l.retry && backup.retry(d, l.retry)}
        >
          {d.retrying === partKey(l.retry) ? 'Đang thử…' : 'Thử lại'}
        </LinkButton>
      ) : undefined,
    ]),
  ];
  return (
    <>
      <DialogTitle>{partly ? 'Đã khôi phục một phần' : 'Đã khôi phục'}</DialogTitle>
      <Steps at={4} />
      <ProgressList lines={lines} />
      {todo.length ? (
        <>
          <DialogSection>Còn lại bạn làm</DialogSection>
          <Facts items={todo.map((t) => ['', t])} />
        </>
      ) : null}
      <DialogButtons>
        {done.canUndo ? (
          <Button large onClick={() => backup.setDialog({ kind: 'undo', changed: null, from: d })}>
            Hoàn tác lần khôi phục…
          </Button>
        ) : null}
        <span className="flex-1" />
        <Button large onClick={backup.toMcp}>
          Xem tab MCP
        </Button>
        <Button large primary onClick={backup.close}>
          Xong
        </Button>
      </DialogButtons>
    </>
  );
}

function UndoBody({ d, backup, model, now }: { d: Extract<BackupDialog, { kind: 'undo' }>; backup: Backup; model: Model; now: Date }) {
  const r = model.backup.restore;
  const at = r ? new Date(r.at * 1000).toISOString() : null;
  const changed = d.changed ? changedLines('changed:' + d.changed) : null;
  return (
    <>
      <DialogTitle>{at ? 'Hoàn tác lần khôi phục ' + packWhenLower(r?.at ?? null, now) + '?' : 'Hoàn tác lần khôi phục?'}</DialogTitle>
      {changed ? (
        <Facts
          items={[
            ...changed.map((t): [string, string] => ['w', t]),
            ['', 'Hoàn tác vẫn chỉ bỏ những gì lần khôi phục đã thêm hay thay; phần bạn sửa thêm vẫn giữ.'],
          ]}
        />
      ) : (
        <Facts items={undoLines(d.from?.done)} />
      )}
      <DialogButtons>
        <Button large onClick={() => (d.from ? backup.setDialog(d.from) : backup.close())}>
          Huỷ
        </Button>
        <Button large danger disabled={backup.working} onClick={() => backup.undo(!!changed, d.from)}>
          {changed ? 'Vẫn hoàn tác' : 'Hoàn tác'}
        </Button>
      </DialogButtons>
    </>
  );
}

function KeyBody({ d, backup }: { d: Extract<BackupDialog, { kind: 'key' }>; backup: Backup }) {
  const [value, setValue] = useState('');
  return (
    <>
      <DialogTitle>Thay khoá</DialogTitle>
      <div className={SECRET_ROW}>
        <span>
          <b>{d.key.mcp}</b>
          <small>{d.key.bitwarden ? 'Trong Bitwarden: ' + d.key.bitwarden : d.key.label}</small>
        </span>
        <span className="flex items-center gap-1.5">
          <Input className="font-mono tracking-[.12em]" type="password" autoComplete="off" aria-label="Khoá mới" placeholder="dán khoá mới" value={value} onChange={(e) => setValue(e.target.value)} />
        </span>
      </div>
      <Facts items={[['ok', 'App đưa khoá mới vào các agent đang dùng MCP này, có xem trước và lưu bản cũ.']]} />
      <DialogButtons>
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={backup.working || !value.trim()} onClick={() => backup.replaceKey(d.key, value)}>
          Lưu
        </Button>
      </DialogButtons>
    </>
  );
}

function DelKeyBody({ d, backup }: { d: Extract<BackupDialog, { kind: 'delKey' }>; backup: Backup }) {
  return (
    <>
      <DialogTitle>Xoá khoá {d.key.mcp}?</DialogTitle>
      <Facts
        items={[
          ['', 'Khoá chỉ bị xoá khỏi máy này. Agent đang dùng ' + d.key.mcp + ' vẫn giữ bản của nó.'],
          ['', 'Lần khôi phục sau trên máy này, app sẽ hỏi lại khoá này.'],
        ]}
      />
      <DialogButtons>
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large danger disabled={backup.working} onClick={() => backup.deleteKey(d.key)}>
          Xoá khoá
        </Button>
      </DialogButtons>
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

