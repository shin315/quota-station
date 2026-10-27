// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The File hướng dẫn tab of the main window (M11, F): the agents'
// instruction files (CLAUDE.md, rules, AGENTS.md, GEMINI.md and any added by
// hand), when each changed and how it stands against the newest backup
// pack, and whether it goes into the pack. The app never edits one. A file
// with a line shaped like a key stays out of every pack; its value is
// masked wherever the window shows it.
// As the approved redraw (instr-A, 25/10/2026) lays it out: one table, its
// files by agent under their group heads, "Thêm file…" at the table's head
// and the two notes at its foot.

import { useState } from 'react';
import { Banner, Button, Chip, Dot, Facts as FactList, Feedback, Toggle, FieldHint, LinkButton } from '../components/parts';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { DiffFile, DiffLine, DiffView, KeyMasked, TECH } from '../components/blocks';
import { CARD_END, COUNT, Empty, GROUP_HEAD, TabHead, TabSub, TabTitle } from '../components/page';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import type { Api } from '../lib/api';
import { clock } from '../lib/format';
import { cloudDrive, mac, t } from '../lib/i18n';
import { AGENT_NAME, AGENTS } from '../lib/mcp';
import type { Agent, InstrFile, InstrShown, McpLine, Model } from '../lib/model';
import { dayClock } from '../lib/switching';
import type { Action, BannerView, ChipView, FeedbackView, PopoverView } from '../lib/view';
import { Facts } from './McpDialogs';

export type InstrDialog =
  | { kind: 'view'; path: string; shown: InstrShown | null; error: string | null }
  | { kind: 'diff'; path: string; lines: McpLine[] | null; error: string | null }
  | { kind: 'add'; path: string; error: string | null };

/** Why a file was not added, from the Rust side's code; null for a code it does not know. */
function addError(code: string): string | null {
  if (code === 'outside') return t('File phải nằm trong thư mục nhà, ngoài thư mục của app.', 'The file must be in your home folder, outside the app’s folder.');
  if (code === 'type') return t('Chỉ nhận file .md hoặc .txt.', 'Only .md or .txt files.');
  if (code === 'missing') return t('Không thấy file này.', 'This file was not found.');
  if (code === 'large') return t('File lớn hơn 1 MB.', 'The file is larger than 1 MB.');
  return null;
}

/** Why a file could not be shown or compared; null for a code it does not know. */
function showError(code: string): string | null {
  if (code === 'missing') return t('Không còn file này. Bấm Đọc lại.', 'This file is gone. Press Read again.');
  if (code === 'noPack') return t('Chưa có gói sao lưu nào để so.', 'There is no backup pack to compare with yet.');
  if (code === 'notInPack') return t('Gói mới nhất không có file này.', 'The latest pack does not have this file.');
  return null;
}

export function useInstr(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const [dialog, setDialog] = useState<InstrDialog | null>(null);
  const [working, setWorking] = useState(false);
  const files = model?.instructions.view?.files ?? [];

  const show = async (path: string) => {
    setDialog({ kind: 'view', path, shown: null, error: null });
    try {
      const shown = await api.instrShow(path);
      setDialog((d) => (d?.kind === 'view' && d.path === path ? { ...d, shown } : d));
    } catch (e) {
      setDialog((d) => (d?.kind === 'view' && d.path === path ? { ...d, error: showError(String(e)) ?? t('Không đọc được file: ', 'Could not read the file: ') + String(e) } : d));
    }
  };

  const compare = async (path: string) => {
    setDialog({ kind: 'diff', path, lines: null, error: null });
    try {
      const lines = await api.instrCompare(path);
      setDialog((d) => (d?.kind === 'diff' && d.path === path ? { ...d, lines } : d));
    } catch (e) {
      setDialog((d) => (d?.kind === 'diff' && d.path === path ? { ...d, error: showError(String(e)) ?? t('Chưa so được: ', 'Could not compare: ') + String(e) } : d));
    }
  };

  const reread = () => void api.instrRead().catch(() => {});

  const notKey = async (path: string, line: number) => {
    setWorking(true);
    try {
      await api.instrNotKey(path, line);
      showFeedback({
        tone: '',
        title: t('Đã đánh dấu dòng ' + line + ' không phải khoá', 'Marked line ' + line + ' as not a key'),
        body: t('Chỉ dòng này, đúng nội dung này. Nếu sửa dòng này, ứng dụng sẽ kiểm tra lại.', 'Only this line, with exactly this content. If the line changes, the app checks it again.'),
      });
      await show(path);
    } catch {
      showFeedback({ tone: 'bad', title: t('Chưa đánh dấu được', 'Could not mark it'), body: t('File vừa đổi. Bấm Đọc lại rồi thử lại.', 'The file just changed. Press Read again, then try again.') });
      setDialog(null);
    } finally {
      setWorking(false);
    }
  };

  const add = async (d: Extract<InstrDialog, { kind: 'add' }>) => {
    setWorking(true);
    try {
      const shown = await api.instrAdd(d.path);
      setDialog(null);
      showFeedback({
        tone: '',
        title: t('Đã thêm ' + shown, 'Added ' + shown),
        body: t('File vào gói sao lưu ở lần sao lưu tới, nếu không có dòng dạng khoá.', 'The file goes into the backup pack at the next backup, if no line looks like a key.'),
      });
    } catch (e) {
      setDialog({ ...d, error: addError(String(e)) ?? String(e) });
    } finally {
      setWorking(false);
    }
  };

  const forget = async (path: string) => {
    await api.instrForget(path).catch(() => {});
    setDialog(null);
  };

  const backupNow = async () => {
    setWorking(true);
    try {
      await api.backupNow();
      setDialog(null);
      showFeedback({ tone: 'info', title: t('Đã sao lưu', 'Backed up'), body: mac(t('Gói mới có bản file hướng dẫn trên máy lúc này.', 'The new pack has the instruction files as they are on this Mac now.')) });
      reread();
    } catch (e) {
      showFeedback({
        tone: 'bad',
        title: t('Chưa sao lưu được', 'Could not back up'),
        body: String(e) === 'noICloud' ? mac(t(cloudDrive() + ' chưa bật trên máy này.', cloudDrive() + ' is not turned on on this Mac.')) : t('Xem tab Sao lưu để biết thêm.', 'See the Backup tab for more.'),
      });
      setDialog(null);
    } finally {
      setWorking(false);
    }
  };

  /** The tab's actions on banners. Returns false for others. */
  const act = (a: Action): boolean => {
    if (a === 'instr:reread') reread();
    else if (a.startsWith('instr:key:')) void show(a.slice('instr:key:'.length));
    else return false;
    return true;
  };

  return {
    files,
    dialog,
    working,
    setDialog,
    close: () => !working && setDialog(null),
    act,
    reread,
    show: (path: string) => void show(path),
    compare: (path: string) => void compare(path),
    include: (path: string, on: boolean) =>
      void api
        .instrInclude(path, on)
        .catch(() => showFeedback({ tone: 'bad', title: t('Chưa lưu được', 'Could not save'), body: t('App không ghi được lựa chọn này. Thử lại.', 'The app could not save this choice. Try again.') })),
    notKey: (path: string, line: number) => void notKey(path, line),
    add: (d: Extract<InstrDialog, { kind: 'add' }>) => void add(d),
    forget: (path: string) => void forget(path),
    open: (path: string) => void api.instrOpen(path).catch(() => {}),
    backupNow: () => void backupNow(),
  };
}

export type Instr = ReturnType<typeof useInstr>;

const file = (path: string) => path.split('/').pop() ?? path;

function statusChip(f: InstrFile, packAt: number | null): ChipView {
  const at = packAt ? clock(new Date(packAt * 1000).toISOString()) : '';
  if (f.keyLines.length)
    return {
      t: t('có dạng khoá · dòng ' + f.keyLines.join(', '), 'looks like a key · ' + (f.keyLines.length === 1 ? 'line ' : 'lines ') + f.keyLines.join(', ')),
      k: 'bad',
      title: t('Gói không bao giờ chứa khoá.', 'A pack never holds keys.'),
    };
  if (f.error) return { t: t('không đọc được', 'unreadable'), k: 'warn', title: f.error };
  if (f.vsPack === 'same') return { t: t('giống gói ' + at, 'same as pack ' + at), k: 'ok' };
  if (f.vsPack === 'changed') return { t: t('đã đổi sau gói ' + at, 'changed after pack ' + at), k: 'info' };
  if (f.vsPack === 'new') return { t: t('chưa có trong gói', 'not in the pack yet'), k: '' };
  return { t: t('chưa có gói', 'no pack yet'), k: '' };
}

function keyBanner(f: InstrFile): BannerView {
  const n = f.keyLines[0];
  return {
    tone: 'bad',
    title: t(file(f.path) + ' chưa vào gói sao lưu', file(f.path) + ' is not in the backup pack'),
    body: t(
      'Dòng ' + f.keyLines.join(', ') + ' có dạng khoá. Gói không bao giờ chứa khoá.',
      (f.keyLines.length === 1 ? 'Line ' + n + ' looks' : 'Lines ' + f.keyLines.join(', ') + ' look') + ' like a key. A pack never holds keys.',
    ),
    actions: [
      { label: t('Xem dòng ' + n, 'View line ' + n), act: `instr:key:${f.path}` },
      { label: t('Đọc lại', 'Read again'), act: 'instr:reread' },
    ],
  };
}

export interface InstrTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  instr: Instr;
  onAction(act: Action): void;
}

/**
 * A file's row: path, when it changed, against the pack, in the pack, its
 * link; its agent is the group head above it. Where the page has under
 * 720 px: the path on a line of its own, the rest on the line under it.
 */
const FILE_ROW =
  'grid grid-cols-[minmax(0,1.7fr)_104px_minmax(0,1.1fr)_62px_64px] items-center gap-2.5 border-t-[.5px] border-card-line px-1.5 py-[7px] text-sm @max-[720px]/main:grid-cols-[auto_minmax(0,1fr)_auto_auto] @max-[720px]/main:gap-y-1 @max-[720px]/main:[&>:nth-child(1)]:col-span-full';

/** The files by agent, in the agents' order; those added by hand last. */
function byAgent(files: InstrFile[]): { agent: Agent | null; files: InstrFile[] }[] {
  const groups = [...AGENTS, null].map((agent) => ({ agent, files: files.filter((f) => f.agent === agent) }));
  return groups.filter((g) => g.files.length);
}

function FileRow({ f, packAt, now, instr }: { f: InstrFile; packAt: number | null; now: Date; instr: Instr }) {
  const key = f.keyLines.length > 0;
  const diff = f.vsPack === 'changed' && !key;
  return (
    <div className={FILE_ROW} role="listitem" aria-label={f.path}>
      <span className="[font:11.5px_var(--mono)] [overflow-wrap:anywhere]">{f.path}</span>
      <span className="text-ink2 tabular-nums">{f.modifiedAt ? dayClock(f.modifiedAt, now) : '—'}</span>
      <span>
        <Chip chip={statusChip(f, packAt)} />
      </span>
      <span>
        <Toggle on={f.include && !key && !f.error} disabled={key || !!f.error} label={t('Đưa ' + f.path + ' vào gói', 'Put ' + f.path + ' in the pack')} onClick={() => instr.include(f.path, !f.include)} />
      </span>
      <span className="text-right">
        <LinkButton
          small
          aria-label={diff ? t('So sánh ' + f.path, 'Compare ' + f.path) : t('Xem ' + f.path, 'View ' + f.path)}
          onClick={() => (diff ? instr.compare(f.path) : instr.show(f.path))}
        >
          {diff ? t('So sánh', 'Compare') : t('Xem', 'View')}
        </LinkButton>
      </span>
    </div>
  );
}

export function InstrTab({ model, fresh, feedback, now, instr, onAction }: InstrTabProps) {
  const v = model.instructions;
  const view = v.view;
  const sub = v.readAt ? t('cập nhật lúc ' + clock(v.readAt), 'updated at ' + clock(v.readAt)) : fresh.text;
  return (
    <>
      <TabHead>
        <TabTitle>{t('File hướng dẫn', 'Instruction files')}</TabTitle>
        <TabSub>
          <Dot tone="ok" />
          {model.machine} · {sub}
        </TabSub>
        <Button onClick={instr.reread}>{t('Làm mới', 'Refresh')}</Button>
      </TabHead>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {(view?.files ?? [])
        .filter((f) => f.keyLines.length)
        .map((f) => (
          <Banner key={f.path} banner={keyBanner(f)} onAction={onAction} />
        ))}
      <Panel>
        <CardTitle>
          {mac(t('Trong máy này', 'On this Mac'))} {view ? <span className={COUNT}>{view.files.length}</span> : null}
          <span className={CARD_END}>
            <Button small onClick={() => instr.setDialog({ kind: 'add', path: '', error: null })}>
              {t('Thêm file…', 'Add file…')}
            </Button>
          </span>
        </CardTitle>
        {!view ? (
          <Empty>
            <b>{t('Đang đọc file hướng dẫn…', 'Reading the instruction files…')}</b>
            {mac(t('CLAUDE.md, AGENTS.md, GEMINI.md và các file luật, trên máy này.', 'CLAUDE.md, AGENTS.md, GEMINI.md and the rule files on this Mac.'))}
          </Empty>
        ) : view.files.length ? (
          <div role="group" aria-label={t('File hướng dẫn', 'Instruction files')}>
            <div className={cn(FILE_ROW, 'border-t-0 pt-0 text-xs text-ink3 @max-[720px]/main:hidden')} aria-hidden="true">
              <span>File</span>
              <span>{t('Sửa lúc', 'Modified')}</span>
              <span>{t('So với gói', 'Versus the pack')}</span>
              <span>{t('Trong gói', 'In the pack')}</span>
              <span />
            </div>
            {byAgent(view.files).map((g) => {
              const name = g.agent ? AGENT_NAME[g.agent] : t('Thêm tay', 'Added by hand');
              return (
                <section key={g.agent ?? 'hand'} aria-label={name}>
                  {/* Every group head has its hairline, the first one too: it parts the head row from the files. */}
                  <h5 className={cn(GROUP_HEAD, 'm-0 px-1.5 first:border-t-[.5px] first:pt-2.5')}>
                    {name} <span className={COUNT}>· {g.files.length}</span>
                  </h5>
                  <div role="list" aria-label={t('File của ' + name, name + ' files')}>
                    {g.files.map((f) => (
                      <FileRow key={f.path} f={f} packAt={view.packAt} now={now} instr={instr} />
                    ))}
                  </div>
                </section>
              );
            })}
          </div>
        ) : (
          <CardText className="mt-2 mb-0">{t('Không thấy file hướng dẫn nào. Thêm file bằng nút Thêm file….', 'No instruction files found. Add one with the Add file… button.')}</CardText>
        )}
        <div className="mt-1 border-t-[.5px] border-card-line px-1.5 pt-2.5">
          <FactList
            className="mb-0"
            items={[
              { k: '', text: t('App không sửa các file này. Khôi phục chỉ ghi file bạn chọn, sau khi xem so sánh từng dòng.', 'The app does not edit these files. A restore only writes the files you pick, after a line-by-line comparison.') },
              { k: '', text: mac(t('Bản trên máy khác gói thì mặc định giữ bản trên máy.', 'When the copy on this Mac differs from the pack, the one on this Mac is kept by default.')) },
            ]}
          />
        </div>
      </Panel>
    </>
  );
}

function Numbered({ shown }: { shown: InstrShown }) {
  return (
    <DiffView>
      {shown.lines.map((l, i) => (
        <DiffLine key={i} n={i + 1} op={l.key ? undefined : 'ctx'} lit={l.key}>
          {l.text || ' '}
          {l.key ? <KeyMasked>{t('dạng khoá, đã che', 'looks like a key, masked')}</KeyMasked> : null}
        </DiffLine>
      ))}
    </DiffView>
  );
}

function ViewBody({ d, instr, now }: { d: Extract<InstrDialog, { kind: 'view' }>; instr: Instr; now: Date }) {
  const f = instr.files.find((x) => x.path === d.path);
  const keys = d.shown ? d.shown.lines.flatMap((l, i) => (l.key ? [i + 1] : [])) : [];
  const meta = f ? (f.size < 1024 ? f.size + ' B' : Math.round(f.size / 1024) + ' KB') + (f.modifiedAt ? t(' · sửa ' + dayClock(f.modifiedAt, now), ' · modified ' + dayClock(f.modifiedAt, now)) : '') : '';
  return (
    <>
      <DialogTitle>{d.path}</DialogTitle>
      <DialogLead>
        {keys.length
          ? t(
              'Dòng ' + keys.join(', ') + ' có dạng khoá. Giá trị đã được che.',
              (keys.length === 1 ? 'Line ' + keys[0] + ' looks like a key. Its value is' : 'Lines ' + keys.join(', ') + ' look like keys. Their values are') + ' masked.',
            )
          : meta + t(' · chỉ xem', ' · view only')}
      </DialogLead>
      {d.error ? <p>{d.error}</p> : d.shown ? <Numbered shown={d.shown} /> : <p>{t('Đang đọc…', 'Reading…')}</p>}
      {keys.length ? (
        <Facts
          items={[
            ['x', t('Chưa vào gói sao lưu.', 'Not in the backup pack.')],
            [
              '',
              t(
                'Bỏ khoá khỏi file (cất ở trình quản lý mật khẩu) rồi Đọc lại. Hoặc đánh dấu dòng này không phải khoá.',
                'Take the key out of the file (keep it in a password manager), then Read again. Or mark this line as not a key.',
              ),
            ],
          ]}
        />
      ) : null}
      <DialogButtons>
        {keys.length ? (
          <Button large disabled={instr.working} onClick={() => instr.notKey(d.path, keys[0])}>
            {t('Dòng ' + keys[0] + ' không phải khoá', 'Line ' + keys[0] + ' is not a key')}
          </Button>
        ) : null}
        {f?.extra ? (
          <Button large onClick={() => instr.forget(d.path)}>
            {t('Bỏ khỏi danh sách', 'Remove from the list')}
          </Button>
        ) : null}
        <span className="flex-1" />
        <Button large onClick={() => instr.open(d.path)}>
          {t('Mở trong trình soạn thảo', 'Open in editor')}
        </Button>
        {keys.length ? (
          <Button
            large
            primary
            onClick={() => {
              instr.reread();
              instr.setDialog(null);
            }}
          >
            {t('Đọc lại', 'Read again')}
          </Button>
        ) : (
          <Button large primary onClick={instr.close}>
            {t('Đóng', 'Close')}
          </Button>
        )}
      </DialogButtons>
    </>
  );
}

function DiffBody({ d, instr, model }: { d: Extract<InstrDialog, { kind: 'diff' }>; instr: Instr; model: Model }) {
  const at = model.instructions.view?.packAt;
  const changed = d.lines?.filter((l) => l.op === 'add' || l.op === 'del').length ?? 0;
  return (
    <>
      <DialogTitle>
        {t(d.path + ' · so với gói ' + (at ? clock(new Date(at * 1000).toISOString()) : ''), d.path + ' · versus pack ' + (at ? clock(new Date(at * 1000).toISOString()) : ''))}
      </DialogTitle>
      {d.error ? (
        <p>{d.error}</p>
      ) : d.lines ? (
        <DiffView head={<DiffFile>{file(d.path)}</DiffFile>} right={t(changed + ' dòng khác', changed + (changed === 1 ? ' line differs' : ' lines differ'))}>
          {d.lines.map((l, i) => (
            <DiffLine key={i} op={l.op}>
              {l.op === 'gap' ? '…' : l.text || ' '}
              {l.secret ? <KeyMasked /> : null}
            </DiffLine>
          ))}
        </DiffView>
      ) : (
        <p>{t('Đang so…', 'Comparing…')}</p>
      )}
      <p className={TECH}>{mac(t('Gói tự sao lưu sẽ lấy bản trên máy ở lần tới.', 'The next automatic backup takes the copy on this Mac.'))}</p>
      <DialogButtons>
        <Button large disabled={instr.working} onClick={instr.backupNow}>
          {instr.working ? t('Đang sao lưu…', 'Backing up…') : t('Sao lưu bây giờ', 'Back up now')}
        </Button>
        <Button large primary onClick={instr.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </>
  );
}

function AddBody({ d, instr }: { d: Extract<InstrDialog, { kind: 'add' }>; instr: Instr }) {
  const go = () => d.path.trim() && instr.add(d);
  return (
    <>
      <DialogTitle>{t('Thêm file hướng dẫn', 'Add an instruction file')}</DialogTitle>
      <label className="text-s font-semi" htmlFor="in-add">
        {t('Đường dẫn file .md hoặc .txt trong thư mục nhà', 'Path to a .md or .txt file in your home folder')}
      </label>
      <Input
        id="in-add"
        mono
        invalid={!!d.error}
        value={d.path}
        placeholder="~/notes/team.md"
        autoFocus
        aria-invalid={!!d.error}
        onChange={(e) => instr.setDialog({ ...d, path: e.target.value, error: null })}
        onKeyDown={(e) => {
          if (e.key === 'Enter') go();
        }}
      />
      {d.error ? <FieldHint tone="bad">{d.error}</FieldHint> : null}
      <div className="mt-2.5">
        <Facts items={[['ok', t('App chỉ đọc file này để đưa vào gói sao lưu, không sửa.', 'The app only reads this file to put it in the backup pack; it never edits it.')]]} />
      </div>
      <DialogButtons>
        <Button large onClick={instr.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={!d.path.trim() || instr.working} onClick={go}>
          {t('Thêm', 'Add')}
        </Button>
      </DialogButtons>
    </>
  );
}

export function InstrDialogBody({ instr, model, now }: { instr: Instr; model: Model; now: Date }) {
  const d = instr.dialog;
  if (!d) return null;
  if (d.kind === 'view') return <ViewBody d={d} instr={instr} now={now} />;
  if (d.kind === 'diff') return <DiffBody d={d} instr={instr} model={model} />;
  return <AddBody d={d} instr={instr} />;
}

export const instrDialogSize = (d: InstrDialog): 'lg' | 'xl' => (d.kind === 'add' ? 'lg' : 'xl');
