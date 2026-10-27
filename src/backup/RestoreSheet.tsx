// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

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
  plural,
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
import { cloudDrive, mac, t } from '../lib/i18n';
import { AGENT_NAME, joinCommand } from '../lib/mcp';
import type { McpDef, Model, RestoreFileItem, RestoreItem, RestorePlan } from '../lib/model';
import { NAV_ICONS } from '../window/icons';
import { partKey, type Backup, type BackupDialog, type RestoreDialog } from './useBackup';

export function backupDialogSize(d: BackupDialog): 'lg' | 'xl' | 'wide' {
  return d.kind === 'restore' ? 'xl' : 'lg';
}

const steps = () => [t('Chọn gói', 'Choose pack'), t('Xem trước', 'Preview'), t('Dán khoá', 'Paste keys'), t('Khôi phục', 'Restore')];

function Steps({ at }: { at: number }) {
  return <StepList names={steps()} at={at} />;
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
      <DialogTitle>{s ? t('Gói ', 'Pack ') + packWhenLower(s.at, now) + ' · ' + s.mac : t('Gói sao lưu', 'Backup pack')}</DialogTitle>
      <Error d={d} />
      {s ? (
        <>
          <DialogSection>{t('Trong gói', 'In the pack')}</DialogSection>
          <FactList
            items={[
              { k: 'ok', key: 'mcp', text: t(s.servers.length + ' MCP, và agent nào đang bật', s.servers.length + ' MCP, and which agents have them on') },
              ...(s.skills?.length
                ? [{ k: 'ok', key: 'skills', text: <span title={s.skills.join(', ')}>
                        {t(
                          s.skills.length + ' skill: skill từ GitHub ghi nguồn, skill cài tay ghi tên',
                          plural(s.skills.length, 'skill', 'skills') + ': from GitHub by source, installed by hand by name',
                        )}
                      </span> }]
                : []),
              ...(s.plugins?.length ? [{ k: 'ok', key: 'plugins', text: <span title={s.plugins.join(', ')}>{t(s.plugins.length + ' plugin, và bật hay tắt', plural(s.plugins.length, 'plugin', 'plugins') + ', and whether each is on')}</span> }] : []),
              ...(s.instructions?.length
                ? [{ k: 'ok', key: 'instr', text: <span title={s.instructions.join(', ')}>{t(s.instructions.length + ' file hướng dẫn', plural(s.instructions.length, 'instruction file', 'instruction files'))}</span> }]
                : []),
            ]}
          />
          <DialogSection>{t('Khoá cần dán khi khôi phục', 'Keys to paste on restore')}</DialogSection>
          {s.keys.length ? (
            <Facts items={s.keys.map((k) => ['', k.mcp + (k.bitwarden ? t(' · lưu ở ', ' · stored in ') + k.bitwarden : '')])} />
          ) : (
            <Facts items={[['', t('Không có', 'None')]]} />
          )}
          <DialogSection>{t('Không có trong gói', 'Not in the pack')}</DialogSection>
          <Facts
            items={[
              ['', t('Khoá, đăng nhập và tài khoản', 'Keys, sign-ins and accounts')],
              ['', t('Nhà cung cấp Codex', 'Codex provider')],
              ['', t('MCP và skill có sẵn của agent, của plugin', 'MCP and skills built into agents or plugins')],
              ['', t('File hướng dẫn có dòng dạng khoá', 'Instruction files with lines that look like keys')],
            ]}
          />
        </>
      ) : !d.error ? (
        <p>{t('Đang đọc gói…', 'Reading the pack…')}</p>
      ) : null}
      <DialogButtons>
        <Button large disabled={!s} onClick={() => backup.openRestore(d.id)}>
          {t('Khôi phục gói này…', 'Restore this pack…')}
        </Button>
        <Button large primary onClick={backup.close}>
          {t('Đóng', 'Close')}
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
            <small>{mac(t('Trên máy', 'On this Mac'))}</small>
            <code>{defLine(c.here.def)}</code>
            {pairs(c.here.def) ? <code>{pairs(c.here.def)}</code> : null}
          </span>
          <span>
            <small>{t('Trong gói', 'In the pack')}</small>
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
      <DialogTitle>{t('Khôi phục', 'Restore')}</DialogTitle>
      <Steps at={0} />
      <DialogLead>{t('Các gói trong ' + cloudDrive() + '.', 'Packs in ' + cloudDrive() + '.')}</DialogLead>
      <RadioGroup
        className="mb-2 grid-cols-[minmax(0,1fr)]"
        aria-label={t('Gói để khôi phục', 'Pack to restore')}
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
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={!d.pack || backup.working} onClick={() => backup.next(d)}>
          {t('Tiếp', 'Next')}
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
      aria-label={t('So sánh ', 'Compare ') + label}
      aria-expanded={d.compare === k}
      onClick={(e) => {
        e.preventDefault();
        backup.update((x) => ({ ...x, compare: x.compare === k ? null : k }));
      }}
    >
      {t('So sánh', 'Compare')}
    </LinkButton>
  );
}

/** "So sánh" of an instruction file: this Mac's lines against the pack's, keys masked. */
function FileCompare({ f }: { f: RestoreFileItem }) {
  return (
    <DiffView flat head={<span>{mac(t('Trên máy → trong gói', 'On this Mac → in the pack'))}</span>} right={t(f.changed + ' dòng khác', plural(f.changed, 'line differs', 'lines differ'))}>
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
            {!sk.github.length && !sk.hand.length ? <Note>{t('Không có skill nào để khôi phục.', 'No skills to restore.')}</Note> : null}
            {skills.same.length ? <Note>{mac(t('Giống trên máy: ' + skills.same.join(', ') + '.', 'Same as on this Mac: ' + skills.same.join(', ') + '.'))}</Note> : null}
          </div>
        </>
      ) : null}
      {plugins && (plugins.items.length || plugins.same.length || plugins.skipped.length) ? (
        <>
          <DialogSection>
            {NAV_ICONS.plugins}
            {t('Plugin', 'Plugins')}
          </DialogSection>
          <div className={GROUP_PANEL}>
            {pl.main.length ? <Tick k={PLUGINS} d={d} backup={backup} title={t(pl.main.length + ' plugin', plural(pl.main.length, 'plugin', 'plugins'))} line={pluginLine(pl.main)} /> : null}
            {pl.off.map((p) => (
              <Tick key={pluginKey(p)} k={pluginKey(p)} d={d} backup={backup} title={'Plugin ' + p.id.split('@')[0]} line={t('Codex · tắt trong gói. Cài lại thì sẽ bật.', 'Codex · off in the pack. Reinstalling turns it on.')} />
            ))}
            {!pl.main.length && !pl.off.length ? <Note>{t('Không có plugin nào để cài.', 'No plugins to install.')}</Note> : null}
            {plugins.same.length ? <Note>{mac(t('Giống trên máy: ' + plugins.same.length + ' plugin.', 'Same as on this Mac: ' + plural(plugins.same.length, 'plugin', 'plugins') + '.'))}</Note> : null}
          </div>
        </>
      ) : null}
      {files && (files.items.length || files.same.length || files.kept.length) ? (
        <>
          <DialogSection>
            {NAV_ICONS.instr}
            {t('File hướng dẫn', 'Instruction files')}
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
            {!files.items.length ? <Note>{t('Không có file nào để ghi.', 'No files to write.')}</Note> : null}
            {files.same.length ? <Note>{mac(t('Giống trên máy: ' + files.same.length + ' file.', 'Same as on this Mac: ' + plural(files.same.length, 'file', 'files') + '.'))}</Note> : null}
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
      <DialogTitle>{t('Xem trước: gói ', 'Preview: pack from ') + packWhenLower(plan.at, now)}</DialogTitle>
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
                    {i.missingCommand ? <Chip chip={{ t: t('chưa cài', 'not installed'), k: 'warn' }} /> : null}
                    {i.kind === 'replace' ? <CompareButton k={k} d={d} backup={backup} label={i.name} /> : null}
                  </>
                }
              />
              {d.compare === k ? <Compare item={i} /> : null}
            </div>
          );
        })}
        {!plan.items.length ? <Note>{mac(t('Máy này đã có đủ các MCP trong gói.', 'This Mac already has every MCP in the pack.'))}</Note> : null}
        {plan.same.length ? <Note>{mac(t('Giống trên máy: ' + plan.same.join(', ') + '.', 'Same as on this Mac: ' + plan.same.join(', ') + '.'))}</Note> : null}
      </div>
      <MoreGroups d={d} backup={backup} plan={plan} />
      {skipped.length ? <Facts items={skipped.map((line) => ['w', line])} /> : null}
      <Facts
        items={[
          ['ok', t('Không đổi đăng nhập, tài khoản hay nhà cung cấp Codex.', 'Does not change sign-ins, accounts or the Codex provider.')],
          ['ok', t('Lưu lại bản cũ trước khi ghi. Hoàn tác được cả lần khôi phục.', 'Saves the old versions before writing. The whole restore can be undone.')],
        ]}
      />
      <Error d={d} />
      <DialogButtons>
        <Button large onClick={() => backup.back(d)}>
          {t('Quay lại', 'Back')}
        </Button>
        <span className="flex-1" />
        <Button large onClick={backup.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        {d.error ? (
          <Button large onClick={() => backup.toPreview(d)}>
            {t('Làm mới', 'Refresh')}
          </Button>
        ) : null}
        <Button large primary disabled={!any || backup.working} onClick={() => backup.next(d)}>
          {keys.length ? t('Tiếp: dán khoá', 'Next: paste keys') : t('Khôi phục', 'Restore')}
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
      <DialogTitle>{t('Dán khoá', 'Paste keys')}</DialogTitle>
      <Steps at={2} />
      <DialogLead>
        {mac(
          t(
            keys.length + ' khoá, lấy từ nơi bạn cất chúng. App giữ trên máy này và không hiện lại.',
            plural(keys.length, 'key', 'keys') + ', from wherever you keep them. The app keeps them on this Mac and never shows them again.',
          ),
        )}
      </DialogLead>
      {keys.map((k) => {
        const id = keySlot(k);
        return (
          <div key={id} className={SECRET_ROW}>
            <span>
              <b>{k.mcp}</b>
              <small>{k.bitwarden ? t('Lưu ở: ', 'Stored in: ') + k.bitwarden : k.label}</small>
            </span>
            <span className="flex items-center gap-1.5">
              <Input
                className="font-mono tracking-[.12em]"
                type="password"
                autoComplete="off"
                aria-label={t('Khoá của ', 'Key for ') + k.mcp}
                placeholder={t('dán vào đây', 'paste here')}
                value={d.keys[id] ?? ''}
                onChange={(e) => backup.update((x) => ({ ...x, keys: { ...x.keys, [id]: e.target.value } }))}
              />
            </span>
          </div>
        );
      })}
      <Facts items={[['', t('Để trống cũng được: MCP vẫn được thêm nhưng tắt, dán khoá sau ở tab MCP.', 'You can leave it empty: the MCP is still added, but off. Paste the key later in the MCP tab.')]]} />
      <DialogButtons>
        <Button large onClick={() => backup.back(d)}>
          {t('Quay lại', 'Back')}
        </Button>
        <span className="flex-1" />
        <Button large onClick={backup.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={backup.working} onClick={() => backup.next(d)}>
          {t('Khôi phục', 'Restore')}
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
  if (plugins) out.push(t(plugins + ' plugin', plural(plugins, 'plugin', 'plugins')));
  for (const f of plan.files?.items ?? []) if (d.ticks[fileKey(f.path)]) out.push(fileLabel(f.path, f.agent));
  return out;
}

function RunStep({ d, backup }: { d: RestoreDialog; backup: Backup }) {
  if (d.step === 'run' || !d.done) {
    return (
      <>
        <DialogTitle>{t('Đang khôi phục…', 'Restoring…')}</DialogTitle>
        <Steps at={3} />
        <ProgressList busy lines={runningLines(d).map((line): [string, string, string] => ['wait', line, ''])} />
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
          aria-label={t('Thử lại ', 'Try again: ') + l.title}
          disabled={backup.working}
          onClick={() => l.retry && backup.retry(d, l.retry)}
        >
          {d.retrying === partKey(l.retry) ? t('Đang thử…', 'Trying…') : t('Thử lại', 'Try again')}
        </LinkButton>
      ) : undefined,
    ]),
  ];
  return (
    <>
      <DialogTitle>{partly ? t('Đã khôi phục một phần', 'Partly restored') : t('Đã khôi phục', 'Restored')}</DialogTitle>
      <Steps at={4} />
      <ProgressList lines={lines} />
      {todo.length ? (
        <>
          <DialogSection>{t('Còn lại bạn làm', 'Left for you to do')}</DialogSection>
          <Facts items={todo.map((line) => ['', line])} />
        </>
      ) : null}
      <DialogButtons>
        {done.canUndo ? (
          <Button large onClick={() => backup.setDialog({ kind: 'undo', changed: null, from: d })}>
            {t('Hoàn tác lần khôi phục…', 'Undo the restore…')}
          </Button>
        ) : null}
        <span className="flex-1" />
        <Button large onClick={backup.toMcp}>
          {t('Xem tab MCP', 'See the MCP tab')}
        </Button>
        <Button large primary onClick={backup.close}>
          {t('Xong', 'Done')}
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
      <DialogTitle>{at ? t('Hoàn tác lần khôi phục ' + packWhenLower(r?.at ?? null, now) + '?', 'Undo the restore from ' + packWhenLower(r?.at ?? null, now) + '?') : t('Hoàn tác lần khôi phục?', 'Undo the restore?')}</DialogTitle>
      {changed ? (
        <Facts
          items={[
            ...changed.map((line): [string, string] => ['w', line]),
            ['', t('Hoàn tác vẫn chỉ bỏ những gì lần khôi phục đã thêm hay thay; phần bạn sửa thêm vẫn giữ.', 'Undo still removes only what the restore added or replaced; your later edits stay.')],
          ]}
        />
      ) : (
        <Facts items={undoLines(d.from?.done)} />
      )}
      <DialogButtons>
        <Button large onClick={() => (d.from ? backup.setDialog(d.from) : backup.close())}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large danger disabled={backup.working} onClick={() => backup.undo(!!changed, d.from)}>
          {changed ? t('Vẫn hoàn tác', 'Undo anyway') : t('Hoàn tác', 'Undo')}
        </Button>
      </DialogButtons>
    </>
  );
}

function KeyBody({ d, backup }: { d: Extract<BackupDialog, { kind: 'key' }>; backup: Backup }) {
  const [value, setValue] = useState('');
  return (
    <>
      <DialogTitle>{t('Thay khoá', 'Replace key')}</DialogTitle>
      <div className={SECRET_ROW}>
        <span>
          <b>{d.key.mcp}</b>
          <small>{d.key.bitwarden ? t('Lưu ở: ', 'Stored in: ') + d.key.bitwarden : d.key.label}</small>
        </span>
        <span className="flex items-center gap-1.5">
          <Input className="font-mono tracking-[.12em]" type="password" autoComplete="off" aria-label={t('Khoá mới', 'New key')} placeholder={t('dán khoá mới', 'paste the new key')} value={value} onChange={(e) => setValue(e.target.value)} />
        </span>
      </div>
      <Facts items={[['ok', t('App đưa khoá mới vào các agent đang dùng MCP này, có xem trước và lưu bản cũ.', 'The app puts the new key into the agents using this MCP, with a preview, and saves the old one.')]]} />
      <DialogButtons>
        <Button large onClick={backup.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={backup.working || !value.trim()} onClick={() => backup.replaceKey(d.key, value)}>
          {t('Lưu', 'Save')}
        </Button>
      </DialogButtons>
    </>
  );
}

function DelKeyBody({ d, backup }: { d: Extract<BackupDialog, { kind: 'delKey' }>; backup: Backup }) {
  return (
    <>
      <DialogTitle>{t('Xoá khoá ' + d.key.mcp + '?', 'Delete the key for ' + d.key.mcp + '?')}</DialogTitle>
      <Facts
        items={[
          ['', mac(t('Khoá chỉ bị xoá khỏi máy này. Agent đang dùng ' + d.key.mcp + ' vẫn giữ bản của nó.', 'The key is deleted from this Mac only. Agents using ' + d.key.mcp + ' keep their own copy.'))],
          ['', mac(t('Lần khôi phục sau trên máy này, app sẽ hỏi lại khoá này.', 'On the next restore on this Mac, the app asks for this key again.'))],
        ]}
      />
      <DialogButtons>
        <Button large onClick={backup.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large danger disabled={backup.working} onClick={() => backup.deleteKey(d.key)}>
          {t('Xoá khoá', 'Delete key')}
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

