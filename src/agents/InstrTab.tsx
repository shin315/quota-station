// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The File hướng dẫn tab of the main window (M11, F): the agents'
// instruction files (CLAUDE.md, rules, AGENTS.md, GEMINI.md and any added by
// hand), when each changed and how it stands against the newest backup
// pack, and whether it goes into the pack. The app never edits one. A file
// with a line shaped like a key stays out of every pack; its value is
// masked wherever the window shows it.

import { useState } from 'react';
import { Banner, Button, Chip, Dot, Feedback, Toggle, FieldHint, LinkButton } from '../components/parts';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { DiffFile, DiffLine, DiffView, KeyMasked, TECH } from '../components/blocks';
import { COUNT, Empty, TabHead, TabSub, TabTitle } from '../components/page';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import type { Api } from '../lib/api';
import { clock } from '../lib/format';
import { AGENT_NAME } from '../lib/mcp';
import type { InstrFile, InstrShown, McpLine, Model } from '../lib/model';
import { dayClock } from '../lib/switching';
import type { Action, BannerView, ChipView, FeedbackView, PopoverView } from '../lib/view';
import { Facts } from './McpDialogs';

export type InstrDialog =
  | { kind: 'view'; path: string; shown: InstrShown | null; error: string | null }
  | { kind: 'diff'; path: string; lines: McpLine[] | null; error: string | null }
  | { kind: 'add'; path: string; error: string | null };

const ADD_ERROR: Record<string, string> = {
  outside: 'File phải nằm trong thư mục nhà, ngoài thư mục của app.',
  type: 'Chỉ nhận file .md hoặc .txt.',
  missing: 'Không thấy file này.',
  large: 'File lớn hơn 1 MB.',
};

const SHOW_ERROR: Record<string, string> = { missing: 'Không còn file này. Bấm Đọc lại.', noPack: 'Chưa có gói sao lưu nào để so.', notInPack: 'Gói mới nhất không có file này.' };

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
      setDialog((d) => (d?.kind === 'view' && d.path === path ? { ...d, error: SHOW_ERROR[String(e)] ?? 'Không đọc được file: ' + String(e) } : d));
    }
  };

  const compare = async (path: string) => {
    setDialog({ kind: 'diff', path, lines: null, error: null });
    try {
      const lines = await api.instrCompare(path);
      setDialog((d) => (d?.kind === 'diff' && d.path === path ? { ...d, lines } : d));
    } catch (e) {
      setDialog((d) => (d?.kind === 'diff' && d.path === path ? { ...d, error: SHOW_ERROR[String(e)] ?? 'Chưa so được: ' + String(e) } : d));
    }
  };

  const reread = () => void api.instrRead().catch(() => {});

  const notKey = async (path: string, line: number) => {
    setWorking(true);
    try {
      await api.instrNotKey(path, line);
      showFeedback({ tone: '', title: 'Đã đánh dấu dòng ' + line + ' không phải khoá', body: 'Chỉ dòng này, đúng nội dung này. Sửa dòng thì app kiểm lại.' });
      await show(path);
    } catch {
      showFeedback({ tone: 'bad', title: 'Chưa đánh dấu được', body: 'File vừa đổi. Bấm Đọc lại rồi thử lại.' });
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
      showFeedback({ tone: '', title: 'Đã thêm ' + shown, body: 'File vào gói sao lưu ở lần sao lưu tới, nếu không có dòng dạng khoá.' });
    } catch (e) {
      setDialog({ ...d, error: ADD_ERROR[String(e)] ?? String(e) });
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
      showFeedback({ tone: 'info', title: 'Đã sao lưu', body: 'Gói mới có bản file hướng dẫn trên máy lúc này.' });
      reread();
    } catch (e) {
      showFeedback({ tone: 'bad', title: 'Chưa sao lưu được', body: String(e) === 'noICloud' ? 'iCloud Drive chưa bật trên máy này.' : 'Xem tab Sao lưu để biết thêm.' });
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
    include: (path: string, on: boolean) => void api.instrInclude(path, on).catch(() => showFeedback({ tone: 'bad', title: 'Chưa lưu được', body: 'App không ghi được lựa chọn này. Thử lại.' })),
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
  if (f.keyLines.length) return { t: 'có dạng khoá · dòng ' + f.keyLines.join(', '), k: 'bad', title: 'Gói không bao giờ chứa khoá.' };
  if (f.error) return { t: 'không đọc được', k: 'warn', title: f.error };
  if (f.vsPack === 'same') return { t: 'giống gói ' + at, k: 'ok' };
  if (f.vsPack === 'changed') return { t: 'đã đổi sau gói ' + at, k: 'info' };
  if (f.vsPack === 'new') return { t: 'chưa có trong gói', k: '' };
  return { t: 'chưa có gói', k: '' };
}

function keyBanner(f: InstrFile): BannerView {
  const n = f.keyLines[0];
  return {
    tone: 'bad',
    title: file(f.path) + ' chưa vào gói sao lưu',
    body: 'Dòng ' + f.keyLines.join(', ') + ' có dạng khoá. Gói không bao giờ chứa khoá.',
    actions: [
      { label: 'Xem dòng ' + n, act: `instr:key:${f.path}` },
      { label: 'Đọc lại', act: 'instr:reread' },
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

/** An instruction file: its path, agent, when it changed, against the pack, in the pack, a link. */
const FILE_ROW = 'grid grid-cols-[minmax(0,1.6fr)_90px_104px_minmax(0,1.1fr)_62px_56px] items-center gap-2.5 border-t-[.5px] border-card-line px-1.5 py-[7px] text-sm';

export function InstrTab({ model, fresh, feedback, now, instr, onAction }: InstrTabProps) {
  const v = model.instructions;
  const view = v.view;
  const sub = v.readAt ? 'đọc lúc ' + clock(v.readAt) : fresh.text;
  return (
    <>
      <TabHead>
        <TabTitle>File hướng dẫn</TabTitle>
        <TabSub>
          <Dot tone="ok" />
          {model.machine} · {sub}
        </TabSub>
        <Button onClick={() => instr.setDialog({ kind: 'add', path: '', error: null })}>Thêm file…</Button>
        <Button onClick={instr.reread}>Đọc lại</Button>
      </TabHead>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {(view?.files ?? [])
        .filter((f) => f.keyLines.length)
        .map((f) => (
          <Banner key={f.path} banner={keyBanner(f)} onAction={onAction} />
        ))}
      {!view ? (
        <Panel>
          <Empty>
            <b>Đang đọc file hướng dẫn…</b>
            CLAUDE.md, AGENTS.md, GEMINI.md và các file luật, trên máy này.
          </Empty>
        </Panel>
      ) : (
        <Panel>
          <CardTitle>
            Trong máy này <span className={COUNT}>{view.files.length}</span>
          </CardTitle>
          {view.files.length ? (
            <div role="list" aria-label="File hướng dẫn">
              <div className={cn(FILE_ROW, 'border-t-0 pt-0 text-xs text-ink3')} aria-hidden="true">
                <span>File</span>
                <span>Agent</span>
                <span>Sửa lúc</span>
                <span>So với gói</span>
                <span>Trong gói</span>
                <span />
              </div>
              {view.files.map((f) => {
                const key = f.keyLines.length > 0;
                const diff = f.vsPack === 'changed' && !key;
                return (
                  <div key={f.path} className={FILE_ROW} role="listitem" aria-label={f.path}>
                    <span className="[font:11.5px_var(--mono)] [overflow-wrap:anywhere]">{f.path}</span>
                    <span>{f.agent ? AGENT_NAME[f.agent] : 'thêm tay'}</span>
                    <span className="text-ink2 tabular-nums">{f.modifiedAt ? dayClock(f.modifiedAt, now) : '—'}</span>
                    <span>
                      <Chip chip={statusChip(f, view.packAt)} />
                    </span>
                    <span>
                      <Toggle on={f.include && !key && !f.error} disabled={key || !!f.error} label={'Đưa ' + f.path + ' vào gói'} onClick={() => instr.include(f.path, !f.include)} />
                    </span>
                    <span>
                      <LinkButton small aria-label={(diff ? 'So sánh ' : 'Xem ') + f.path} onClick={() => (diff ? instr.compare(f.path) : instr.show(f.path))}>
                        {diff ? 'So sánh' : 'Xem'}
                      </LinkButton>
                    </span>
                  </div>
                );
              })}
            </div>
          ) : (
            <CardText className="mt-2 mb-0">Không thấy file hướng dẫn nào. Thêm file bằng nút Thêm file….</CardText>
          )}
        </Panel>
      )}
      <Panel>
        <Facts
          items={[
            ['', 'App không sửa các file này. Khôi phục chỉ ghi file bạn chọn, sau khi xem so sánh từng dòng.'],
            ['', 'Bản trên máy khác gói thì mặc định giữ bản trên máy.'],
          ]}
        />
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
          {l.key ? <KeyMasked>dạng khoá, đã che</KeyMasked> : null}
        </DiffLine>
      ))}
    </DiffView>
  );
}

function ViewBody({ d, instr, now }: { d: Extract<InstrDialog, { kind: 'view' }>; instr: Instr; now: Date }) {
  const f = instr.files.find((x) => x.path === d.path);
  const keys = d.shown ? d.shown.lines.flatMap((l, i) => (l.key ? [i + 1] : [])) : [];
  const meta = f ? (f.size < 1024 ? f.size + ' B' : Math.round(f.size / 1024) + ' KB') + (f.modifiedAt ? ' · sửa ' + dayClock(f.modifiedAt, now) : '') : '';
  return (
    <>
      <DialogTitle>{d.path}</DialogTitle>
      <DialogLead>{keys.length ? 'Dòng ' + keys.join(', ') + ' có dạng khoá. Giá trị đã được che.' : meta + ' · chỉ xem'}</DialogLead>
      {d.error ? <p>{d.error}</p> : d.shown ? <Numbered shown={d.shown} /> : <p>Đang đọc…</p>}
      {keys.length ? (
        <Facts
          items={[
            ['x', 'Chưa vào gói sao lưu.'],
            ['', 'Bỏ khoá khỏi file (cất ở trình quản lý mật khẩu) rồi Đọc lại. Hoặc đánh dấu dòng này không phải khoá.'],
          ]}
        />
      ) : null}
      <DialogButtons>
        {keys.length ? (
          <Button large disabled={instr.working} onClick={() => instr.notKey(d.path, keys[0])}>
            Dòng {keys[0]} không phải khoá
          </Button>
        ) : null}
        {f?.extra ? (
          <Button large onClick={() => instr.forget(d.path)}>
            Bỏ khỏi danh sách
          </Button>
        ) : null}
        <span className="flex-1" />
        <Button large onClick={() => instr.open(d.path)}>
          Mở trong trình soạn thảo
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
            Đọc lại
          </Button>
        ) : (
          <Button large primary onClick={instr.close}>
            Đóng
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
        {d.path} · so với gói {at ? clock(new Date(at * 1000).toISOString()) : ''}
      </DialogTitle>
      {d.error ? (
        <p>{d.error}</p>
      ) : d.lines ? (
        <DiffView head={<DiffFile>{file(d.path)}</DiffFile>} right={changed + ' dòng khác'}>
          {d.lines.map((l, i) => (
            <DiffLine key={i} op={l.op}>
              {l.op === 'gap' ? '…' : l.text || ' '}
              {l.secret ? <KeyMasked /> : null}
            </DiffLine>
          ))}
        </DiffView>
      ) : (
        <p>Đang so…</p>
      )}
      <p className={TECH}>Gói tự sao lưu sẽ lấy bản trên máy ở lần tới.</p>
      <DialogButtons>
        <Button large disabled={instr.working} onClick={instr.backupNow}>
          {instr.working ? 'Đang sao lưu…' : 'Sao lưu bây giờ'}
        </Button>
        <Button large primary onClick={instr.close}>
          Đóng
        </Button>
      </DialogButtons>
    </>
  );
}

function AddBody({ d, instr }: { d: Extract<InstrDialog, { kind: 'add' }>; instr: Instr }) {
  const go = () => d.path.trim() && instr.add(d);
  return (
    <>
      <DialogTitle>Thêm file hướng dẫn</DialogTitle>
      <label className="text-s font-semi" htmlFor="in-add">
        Đường dẫn file .md hoặc .txt trong thư mục nhà
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
        <Facts items={[['ok', 'App chỉ đọc file này để đưa vào gói sao lưu, không sửa.']]} />
      </div>
      <DialogButtons>
        <Button large onClick={instr.close}>
          Huỷ
        </Button>
        <Button large primary disabled={!d.path.trim() || instr.working} onClick={go}>
          Thêm
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
