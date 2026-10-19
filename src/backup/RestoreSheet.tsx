// The Sao lưu tab's dialogs (M10, M12): "Xem trong gói", the Khôi phục
// sheet in four steps (pick, preview, keys, restore), the undo of a whole
// restore, and replacing or deleting a key the app keeps. The preview has
// the servers, then the skills, plugins and instruction files by group; the
// result has "Thử lại" on a part that failed. As the approved prototype
// after round 2: plain words, no folder path or command; a key is a
// password field that starts empty and is never shown again.

import { useState, type ReactNode } from 'react';
import { LINE_CLASS } from '../agents/McpDialogs';
import { Button } from '../components/parts';
import { SHEET_TITLE_ID } from '../components/Sheets';
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
  return (
    <ol className="stp" aria-label="Các bước">
      {STEPS.map((t, i) => (
        <li key={t} className={i === at ? 'on' : i < at ? 'done' : undefined} aria-current={i === at ? 'step' : undefined}>
          <i>{i < at ? '✓' : i + 1}</i>
          {t}
        </li>
      ))}
    </ol>
  );
}

function Facts({ items }: { items: [string, string][] }) {
  return (
    <ul className="facts">
      {items.map(([k, t]) => (
        <li key={t} className={k || undefined}>
          <span>{t}</span>
        </li>
      ))}
    </ul>
  );
}

function Error({ d }: { d: { error: { title: string; body: string } | null } }) {
  if (!d.error) return null;
  return (
    <p className="fhint bad" role="alert">
      <b>{d.error.title}.</b> {d.error.body}
    </p>
  );
}

function ViewBody({ d, backup, now }: { d: Extract<BackupDialog, { kind: 'view' }>; backup: Backup; now: Date }) {
  const s = d.summary;
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{s ? 'Gói ' + packWhenLower(s.at, now) + ' · ' + s.mac : 'Gói sao lưu'}</h3>
      <Error d={d} />
      {s ? (
        <>
          <div className="sec-t">Trong gói</div>
          <ul className="facts">
            <li className="ok">
              <span>{s.servers.length} MCP, và agent nào đang bật</span>
            </li>
            {s.skills?.length ? (
              <li className="ok">
                <span title={s.skills.join(', ')}>{s.skills.length} skill: skill từ GitHub ghi nguồn, skill cài tay ghi tên</span>
              </li>
            ) : null}
            {s.plugins?.length ? (
              <li className="ok">
                <span title={s.plugins.join(', ')}>{s.plugins.length} plugin, và bật hay tắt</span>
              </li>
            ) : null}
            {s.instructions?.length ? (
              <li className="ok">
                <span title={s.instructions.join(', ')}>{s.instructions.length} file hướng dẫn</span>
              </li>
            ) : null}
          </ul>
          <div className="sec-t">Khoá cần dán khi khôi phục</div>
          {s.keys.length ? (
            <Facts items={s.keys.map((k) => ['', k.mcp + (k.bitwarden ? ' · ' + k.bitwarden + ' (tên trong Bitwarden)' : '')])} />
          ) : (
            <Facts items={[['', 'Không có']]} />
          )}
          <div className="sec-t">Không có trong gói</div>
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
        <p className="note">Đang đọc gói…</p>
      ) : null}
      <div className="btns">
        <Button large disabled={!s} onClick={() => backup.openRestore(d.id)}>
          Khôi phục gói này…
        </Button>
        <Button large primary onClick={backup.close}>
          Đóng
        </Button>
      </div>
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
    <div className="rcmp">
      {item.compare.map((c) => (
        <div key={c.agent} className="rc">
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
      <h3 id={SHEET_TITLE_ID}>Khôi phục</h3>
      <Steps at={0} />
      <p className="lead">Các gói trong iCloud Drive.</p>
      <div className="opts one" role="radiogroup" aria-label="Gói để khôi phục">
        {packs.map((p) => {
          const note = packNote(p);
          const on = d.pack === p.id;
          return (
            <button
              key={p.id}
              type="button"
              role="radio"
              aria-checked={on}
              disabled={!usable(p)}
              onClick={() => backup.update((x) => ({ ...x, pack: p.id, error: null }))}
            >
              <b>{packWhen(p.at, now) + ' · ' + macOf(p, model.machine)}</b>
              <small>{note ?? packLine(p)}</small>
            </button>
          );
        })}
      </div>
      <Error d={d} />
      <div className="btns">
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={!d.pack || backup.working} onClick={() => backup.next(d)}>
          Tiếp
        </Button>
      </div>
    </>
  );
}

/** One line of the preview with its checkbox, as the servers' lines are. */
function Tick({ k, d, backup, title, line, disabled, right }: { k: string; d: RestoreDialog; backup: Backup; title: string; line: string; disabled?: boolean; right?: ReactNode }) {
  return (
    <label className="ck">
      <input
        type="checkbox"
        checked={!!d.ticks[k]}
        disabled={disabled}
        onChange={(e) => backup.update((x) => ({ ...x, ticks: { ...x.ticks, [k]: e.target.checked } }))}
      />
      <span>
        <b>{title}</b>
        <small>{line}</small>
      </span>
      {right ? <span className="r">{right}</span> : null}
    </label>
  );
}

function CompareButton({ k, d, backup, label }: { k: string; d: RestoreDialog; backup: Backup; label: string }) {
  return (
    <button
      type="button"
      className="lnk sm"
      aria-label={'So sánh ' + label}
      aria-expanded={d.compare === k}
      onClick={(e) => {
        e.preventDefault();
        backup.update((x) => ({ ...x, compare: x.compare === k ? null : k }));
      }}
    >
      So sánh
    </button>
  );
}

/** "So sánh" of an instruction file: this Mac's lines against the pack's, keys masked. */
function FileCompare({ f }: { f: RestoreFileItem }) {
  return (
    <div className="diff">
      <div className="fh">
        <span>Trên máy → trong gói</span>
        <span className="r">{f.changed} dòng khác</span>
      </div>
      <pre>
        {f.diff.map((l, i) => (
          <span key={i} className={LINE_CLASS[l.op]}>
            {l.op === 'gap' ? '…' : l.text || ' '}
            {l.secret ? <span className="kc">khoá, đã che</span> : null}
          </span>
        ))}
      </pre>
    </div>
  );
}

const Note = ({ children }: { children: ReactNode }) => (
  <p className="note" style={{ margin: '4px 0 2px' }}>
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
          <div className="sec-t">
            {NAV_ICONS.skills}
            Skills
          </div>
          <div className="grpx">
            {sk.github.length ? <Tick k={SKILLS_GITHUB} d={d} backup={backup} title={SKILL_TITLE.github(sk.github.length)} line={skillLine('github', sk.github)} /> : null}
            {sk.hand.length ? <Tick k={SKILLS_HAND} d={d} backup={backup} title={SKILL_TITLE.hand(sk.hand.length)} line={skillLine('hand', sk.hand)} /> : null}
            {!sk.github.length && !sk.hand.length ? <Note>Không có skill nào để khôi phục.</Note> : null}
            {skills.same.length ? <Note>Giống trên máy: {skills.same.join(', ')}.</Note> : null}
          </div>
        </>
      ) : null}
      {plugins && (plugins.items.length || plugins.same.length || plugins.skipped.length) ? (
        <>
          <div className="sec-t">
            {NAV_ICONS.plugins}
            Plugin
          </div>
          <div className="grpx">
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
          <div className="sec-t">
            {NAV_ICONS.instr}
            File hướng dẫn
          </div>
          <div className="grpx">
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
      <h3 id={SHEET_TITLE_ID}>Xem trước: gói {packWhenLower(plan.at, now)}</h3>
      <Steps at={1} />
      <div className="sec-t">
        {NAV_ICONS.mcp}
        MCP
      </div>
      <div className="grpx">
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
                    {i.missingCommand ? <span className="chip warn">chưa cài</span> : null}
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
      <div className="btns">
        <Button large onClick={() => backup.back(d)}>
          Quay lại
        </Button>
        <span className="grow" />
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
      </div>
    </>
  );
}

function KeysStep({ d, backup }: { d: RestoreDialog; backup: Backup }) {
  const plan = d.plan;
  if (!plan) return null;
  const keys = keysFor(plan, d.ticks);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Dán khoá từ Bitwarden</h3>
      <Steps at={2} />
      <p className="lead">{keys.length} khoá, lấy từ Bitwarden. App giữ trên máy này và không hiện lại.</p>
      {keys.map((k) => {
        const id = keySlot(k);
        return (
          <div key={id} className="sec">
            <span>
              <b>{k.mcp}</b>
              <small>{k.bitwarden ? 'Trong Bitwarden: ' + k.bitwarden : k.label}</small>
            </span>
            <span className="in">
              <input
                className="inp"
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
      <div className="btns">
        <Button large onClick={() => backup.back(d)}>
          Quay lại
        </Button>
        <span className="grow" />
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={backup.working} onClick={() => backup.next(d)}>
          Khôi phục
        </Button>
      </div>
    </>
  );
}

function Progress({ lines }: { lines: [string, string, string, ReactNode?][] }) {
  return (
    <ul className="prog">
      {lines.map(([tone, title, sub, action]) => (
        <li key={title}>
          <span className={'ic' + (tone ? ' ' + tone : '')} aria-hidden="true">
            {tone === 'x' ? '×' : tone === 'w' ? '!' : tone === 'run' ? '' : '✓'}
          </span>
          <span>
            {title}
            {sub ? <small>{sub}</small> : null}
          </span>
          {action ? <span className="r">{action}</span> : null}
        </li>
      ))}
    </ul>
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
        <h3 id={SHEET_TITLE_ID}>Đang khôi phục…</h3>
        <Steps at={3} />
        <ul className="prog" aria-busy="true">
          {runningLines(d).map((t) => (
            <li key={t} className="dim">
              <span className="ic wait" aria-hidden="true" />
              <span>{t}</span>
            </li>
          ))}
        </ul>
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
        <button
          type="button"
          className="lnk sm"
          aria-label={'Thử lại ' + l.title}
          disabled={backup.working}
          onClick={() => l.retry && backup.retry(d, l.retry)}
        >
          {d.retrying === partKey(l.retry) ? 'Đang thử…' : 'Thử lại'}
        </button>
      ) : undefined,
    ]),
  ];
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{partly ? 'Đã khôi phục một phần' : 'Đã khôi phục'}</h3>
      <Steps at={4} />
      <Progress lines={lines} />
      {todo.length ? (
        <>
          <div className="sec-t">Còn lại bạn làm</div>
          <Facts items={todo.map((t) => ['', t])} />
        </>
      ) : null}
      <div className="btns">
        {done.canUndo ? (
          <Button large onClick={() => backup.setDialog({ kind: 'undo', changed: null, from: d })}>
            Hoàn tác lần khôi phục…
          </Button>
        ) : null}
        <span className="grow" />
        <Button large onClick={backup.toMcp}>
          Xem tab MCP
        </Button>
        <Button large primary onClick={backup.close}>
          Xong
        </Button>
      </div>
    </>
  );
}

function UndoBody({ d, backup, model, now }: { d: Extract<BackupDialog, { kind: 'undo' }>; backup: Backup; model: Model; now: Date }) {
  const r = model.backup.restore;
  const at = r ? new Date(r.at * 1000).toISOString() : null;
  const changed = d.changed ? changedLines('changed:' + d.changed) : null;
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{at ? 'Hoàn tác lần khôi phục ' + packWhenLower(r?.at ?? null, now) + '?' : 'Hoàn tác lần khôi phục?'}</h3>
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
      <div className="btns">
        <Button large onClick={() => (d.from ? backup.setDialog(d.from) : backup.close())}>
          Huỷ
        </Button>
        <Button large danger disabled={backup.working} onClick={() => backup.undo(!!changed, d.from)}>
          {changed ? 'Vẫn hoàn tác' : 'Hoàn tác'}
        </Button>
      </div>
    </>
  );
}

function KeyBody({ d, backup }: { d: Extract<BackupDialog, { kind: 'key' }>; backup: Backup }) {
  const [value, setValue] = useState('');
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Thay khoá</h3>
      <div className="sec">
        <span>
          <b>{d.key.mcp}</b>
          <small>{d.key.bitwarden ? 'Trong Bitwarden: ' + d.key.bitwarden : d.key.label}</small>
        </span>
        <span className="in">
          <input className="inp" type="password" autoComplete="off" aria-label="Khoá mới" placeholder="dán khoá mới" value={value} onChange={(e) => setValue(e.target.value)} />
        </span>
      </div>
      <Facts items={[['ok', 'App đưa khoá mới vào các agent đang dùng MCP này, có xem trước và lưu bản cũ.']]} />
      <div className="btns">
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large primary disabled={backup.working || !value.trim()} onClick={() => backup.replaceKey(d.key, value)}>
          Lưu
        </Button>
      </div>
    </>
  );
}

function DelKeyBody({ d, backup }: { d: Extract<BackupDialog, { kind: 'delKey' }>; backup: Backup }) {
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Xoá khoá {d.key.mcp}?</h3>
      <Facts
        items={[
          ['', 'Khoá chỉ bị xoá khỏi máy này. Agent đang dùng ' + d.key.mcp + ' vẫn giữ bản của nó.'],
          ['', 'Lần khôi phục sau trên máy này, app sẽ hỏi lại khoá này.'],
        ]}
      />
      <div className="btns">
        <Button large onClick={backup.close}>
          Huỷ
        </Button>
        <Button large danger disabled={backup.working} onClick={() => backup.deleteKey(d.key)}>
          Xoá khoá
        </Button>
      </div>
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

