// The MCP tab's dialogs (M9): the batch preview every write goes through,
// the add and edit form (typed, or pasted as JSON or TOML), the comparison
// of a server that differs between agents, delete, turning a stored server
// back on, pasting a key, how to install a missing command, the servers
// agents bring themselves, and the backups.
//
// A secret is never shown: the form's secret fields are password fields
// that start empty, and what the preview shows is masked by the Rust side.

import { useEffect, useRef, useState } from 'react';
import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { Input, Textarea } from '@/components/ui/input';
import { Segment, Segmented } from '@/components/ui/segmented';
import { cn } from '@/lib/utils';
import { CheckLine, CheckRow, Command, COMPARE, COMPARE_DIFF, COMPARE_HEAD, DiffFile, DiffLine, DiffView, KeyMasked, PACK_META, PACK_ROW, SECRET_ROW, Steps as StepList, TECH } from '../components/blocks';
import { LABEL, rowClass } from '../components/page';
import { Button, ChoiceGroup, CODE, Facts as FactList, FeedbackLine, FeedbackText, FieldHint, LinkButton } from '../components/parts';
import {
  AGENT_NAME,
  AGENTS,
  RESTART,
  isSecretPair,
  joinCommand,
  lineHasSecret,
  previewLead,
  refusalText,
  type FormPair,
  type McpForm,
} from '../lib/mcp';
import type { Agent, McpBlock, McpDef, McpPaste, McpRow, Model } from '../lib/model';
import { dayClock } from '../lib/switching';
import { LOCK_ICON } from '../window/icons';
import { AgentLogo, HEAD_OFF, MATRIX_CELL, MATRIX_HEAD, MATRIX_NAME, MATRIX_ROW, MATRIX_TITLE } from './CheckMatrix';
import { khoAgents } from './McpTab';
import type { BatchDialog, FormDialog, Mcp, McpDialog } from './useMcp';

/** The dialog's width: the preview and the form need room for lines of config. */
export function mcpDialogSize(d: McpDialog): 'lg' | 'xl' | 'wide' {
  if (d.kind === 'batch' || d.kind === 'form') return 'xl';
  if (d.kind === 'howCmd') return 'wide';
  return 'lg';
}

function Steps({ at }: { at: 0 | 1 }) {
  return <StepList names={['Điền', 'Xem trước']} at={at} />;
}

export function Facts({ items }: { items: [string, string][] }) {
  return <FactList items={items.map(([k, text]) => ({ k, text }))} />;
}

function blockLabel(agent: Agent, b: McpBlock): string {
  const what = agent === 'codex' ? '[mcp_servers.' + b.name + ']' : b.name;
  if (b.action === 'add') return 'thêm mục ' + what;
  if (b.action === 'remove') return 'bỏ mục ' + what;
  return 'mục ' + what;
}

function Diff({ file, agent, block }: { file: string; agent: Agent; block: McpBlock }) {
  return (
    <DiffView head={<DiffFile>{file}</DiffFile>} right={blockLabel(agent, block)}>
      {block.lines.map((l, i) => (
        <DiffLine key={i} op={l.op}>
          {l.op === 'gap' ? '…' : l.text || ' '}
          {l.secret ? <KeyMasked /> : null}
        </DiffLine>
      ))}
    </DiffView>
  );
}

function claudeNote(b: McpBlock): string {
  const notes = ['Claude Code tự ghi ~/.claude.json.'];
  if (b.kept && b.action === 'off') notes.push('App giữ cấu hình của ' + b.name + ' để bật lại.');
  else if (b.kept) notes.push('App giữ cấu hình trong kho, khoá trong Keychain.');
  if (b.secret && b.action !== 'off' && b.action !== 'remove') notes.push('Khoá đi vào lệnh, không vào log hay bản lưu.');
  return notes.join(' ');
}

export function BlockView({ agent, file, block }: { agent: Agent; file: string; block: McpBlock }) {
  const extra = [
    block.from ? 'Chép từ ' + AGENT_NAME[block.from] + '.' : '',
    block.dropped.length ? AGENT_NAME[agent] + ' không có ' + block.dropped.join(', ') + ': bỏ khi ghi.' : '',
    agent !== 'claude' && block.kept ? 'App giữ cấu hình trong kho, khoá trong Keychain.' : '',
  ]
    .filter(Boolean)
    .join(' ');
  if (block.commands.length)
    return (
      <>
        {block.commands.map((c) => (
          <Command key={c}>{c}</Command>
        ))}
        <p className={TECH}>{(claudeNote(block) + ' ' + extra).trim()}</p>
      </>
    );
  return (
    <>
      <Diff file={file} agent={agent} block={block} />
      {extra ? <p className={TECH}>{extra}</p> : null}
    </>
  );
}

function BatchBody({ d, mcp, rows }: { d: BatchDialog; mcp: Mcp; rows: McpRow[] }) {
  const p = d.preview;
  const n = p.changes.length;
  const files = p.groups.map((g) => (g.agent === 'claude' ? 'danh sách MCP của Claude Code' : g.file)).join(', ');
  const facts: [string, string][] = [
    ['ok', 'Sao lưu ' + files + ' trước khi ghi. Bản lưu không chứa khoá.'],
    ['ok', 'Chỉ sửa đúng mục trong xem trước; phần còn lại của file giữ nguyên. Mỗi file ghi một lần.'],
    ...p.refused.map((r): [string, string] => ['x', 'Bỏ qua ' + r.name + ' cho ' + AGENT_NAME[r.agent] + ': ' + refusalText(r.reason) + '.']),
    ...p.groups.map((g): [string, string] => ['w', RESTART[g.agent]]),
  ];
  return (
    <>
      <DialogTitle>{d.title}</DialogTitle>
      {d.steps ? <Steps at={1} /> : null}
      <DialogLead>{previewLead(p, rows)}</DialogLead>
      {p.groups.map((g) => (
        <section key={g.agent} aria-label={AGENT_NAME[g.agent]}>
          <DialogSection>
            <AgentLogo agent={g.agent} />
            {AGENT_NAME[g.agent]}
          </DialogSection>
          {g.blocks.map((b) => (
            <BlockView key={b.name + b.action} agent={g.agent} file={g.file} block={b} />
          ))}
        </section>
      ))}
      <Facts items={facts} />
      <DialogButtons>
        <Button large disabled={mcp.working} onClick={mcp.back}>
          Quay lại
        </Button>
        <Button large primary disabled={mcp.working} onClick={mcp.write}>
          {mcp.working ? 'Đang ghi…' : 'Ghi ' + n + ' thay đổi'}
        </Button>
      </DialogButtons>
    </>
  );
}

// ===== The add and edit form =====

function PairRow(props: { pair: FormPair; i: number; http: boolean; onChange(p: FormPair): void; onRemove(): void }) {
  const p = props.pair;
  const label = props.http ? 'Header' : 'Biến';
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_auto] items-center gap-1.5">
      <Input
        mono
        aria-label={label + ' ' + (props.i + 1) + ': tên'}
        value={p.name}
        placeholder={props.http ? 'Authorization' : 'API_KEY'}
        onChange={(e) => props.onChange({ ...p, name: e.target.value, secret: p.kept || isSecretPair(e.target.value, p.value) })}
      />
      <span className="flex min-w-0 items-center gap-1.5">
        <Input
          type={p.secret ? 'password' : 'text'}
          autoComplete="off"
          spellCheck={false}
          aria-label={label + ' ' + (props.i + 1) + ': giá trị'}
          value={p.value}
          placeholder={p.kept ? 'giữ khoá đang có' : 'giá trị'}
          onChange={(e) => props.onChange({ ...p, value: e.target.value, secret: p.kept || isSecretPair(p.name, e.target.value) })}
        />
        {p.secret ? (
          <span className="inline-flex items-center gap-1 whitespace-nowrap text-xs text-info-ink [&_svg]:flex-none" title="Giá trị vào Keychain, không hiện lại">
            {LOCK_ICON}
            Keychain
          </span>
        ) : null}
      </span>
      <LinkButton small aria-label={'Bỏ dòng ' + (p.name || props.i + 1)} onClick={props.onRemove}>
        Bỏ
      </LinkButton>
    </div>
  );
}

function AgentTicks(props: { model: Model; value: Record<Agent, boolean>; label: string; onChange(v: Record<Agent, boolean>): void }) {
  const cols = props.model.mcp.table?.columns ?? [];
  return (
    <div role="group" aria-label={props.label} className="flex flex-wrap gap-x-3.5 gap-y-1">
      {AGENTS.map((a) => {
        const can = !!cols.find((c) => c.agent === a)?.writable;
        return (
          <CheckLine
            key={a}
            className="m-0"
            title={can ? undefined : AGENT_NAME[a] + ': chỉ xem trên máy này'}
            checked={can && props.value[a]}
            disabled={!can}
            onChange={(on) => props.onChange({ ...props.value, [a]: on })}
          >
            {AGENT_NAME[a]}
          </CheckLine>
        );
      })}
    </div>
  );
}

function pasteSummary(p: McpPaste | null, saved: boolean): { facts: [string, string][]; error: string | null } {
  if (!p) return { facts: [], error: null };
  const errors: Record<NonNullable<McpPaste['error']>, string> = {
    empty: '',
    tooLong: 'Đoạn dán quá dài cho một danh sách MCP.',
    parse: 'Không đọc được: ' + (p.detail ?? 'sai cú pháp') + '.',
    noName: 'Thiếu tên MCP. Dán dạng {"tên": {"command": …}} hoặc cả khối "mcpServers".',
    none: 'Không thấy MCP nào trong đoạn này.',
    keychain: 'Chưa lưu được khoá vào Keychain. Chưa ghi gì.',
  };
  if (p.error) return { facts: [], error: errors[p.error] || null };
  const facts: [string, string][] = [];
  const names = p.servers.map((s) => s.name);
  if (names.length) facts.push(['ok', 'Tìm thấy ' + names.length + ' MCP: ' + names.join(', ') + '.']);
  const secrets = p.servers.flatMap((s) => s.slots.map((x) => x.label));
  if (secrets.length)
    facts.push(['ok', secrets.length + ' giá trị có dạng khoá: ' + secrets.join(', ') + '. ' + (saved ? 'Đã lưu vào Keychain.' : 'App che giá trị và lưu vào Keychain khi bấm Tiếp.')]);
  if (p.skipped.length) facts.push(['w', 'Bỏ qua ' + p.skipped.join(', ') + ': tên có ký tự lạ, hoặc không có lệnh hay địa chỉ.']);
  return { facts, error: null };
}

function FormBody({ d, mcp, model }: { d: FormDialog; mcp: Mcp; model: Model }) {
  const f = d.form;
  const set = (patch: Partial<McpForm>) => mcp.setDialog({ ...d, form: { ...f, ...patch }, error: null });
  const http = f.kind === 'http';
  const secrets = f.pairs.some((p) => p.secret);
  const lineSecret = !http && lineHasSecret(f.line);
  // The paste is read as it is typed, masked by the Rust side.
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const latest = useRef(d);
  latest.current = d;
  useEffect(() => () => clearTimeout(timer.current), []);
  const onPaste = (text: string) => {
    mcp.setDialog({ ...d, text, saved: null, error: null });
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      void mcp.paste(text).then(
        (found) => latest.current.text === text && mcp.setDialog({ ...latest.current, found }),
        () => {},
      );
    }, 250);
  };
  const summary = pasteSummary(d.found, !!d.saved);
  const pasteSecrets = !!d.found?.servers.some((s) => s.slots.length);
  return (
    <>
      <DialogTitle>{f.edit ? 'Sửa ' + f.name : 'Thêm MCP'}</DialogTitle>
      {f.edit ? null : <Steps at={0} />}
      {f.edit ? null : (
        <Segmented variant="inline" className="mt-0 mb-2.5" aria-label="Cách thêm">
          <Segment pressed={d.mode === 'form'} onClick={() => mcp.setDialog({ ...d, mode: 'form', error: null })}>
            Điền tay
          </Segment>
          <Segment pressed={d.mode === 'paste'} onClick={() => mcp.setDialog({ ...d, mode: 'paste', error: null })}>
            Dán JSON hoặc TOML
          </Segment>
        </Segmented>
      )}
      {d.mode === 'paste' ? (
        <>
          {d.saved ? (
            <DialogLead>Khoá đã vào Keychain. Bấm Tiếp để xem lại phần sẽ ghi, hoặc dán đoạn khác.</DialogLead>
          ) : null}
          <Textarea
            mono
            rows={9}
            aria-label="JSON hoặc TOML"
            spellCheck={false}
            autoComplete="off"
            style={{ resize: 'vertical' }}
            placeholder={'{\n  "mcpServers": {\n    "tên": { "command": "npx", "args": ["-y", "gói-mcp"] }\n  }\n}'}
            value={d.text}
            onChange={(e) => onPaste(e.target.value)}
          />
          {summary.facts.length || summary.error ? (
            <div className="mt-2 rounded-tile bg-fill px-2.5 py-2 text-sm">
              {summary.error ? <p className="m-0 text-xs text-bad-ink">{summary.error}</p> : <Facts items={summary.facts} />}
            </div>
          ) : null}
          <div className="mt-2 grid">
            {pasteSecrets ? (
              <div className={rowClass()}>
                <label className={LABEL} htmlFor="mcp-bw">Tên trong Bitwarden</label>
                <div>
                  <Input id="mcp-bw" value={f.bitwarden} onChange={(e) => set({ bitwarden: e.target.value })} />
                  <FieldHint>Chỉ là gợi ý để biết khoá nằm ở mục nào khi phải dán lại.</FieldHint>
                </div>
              </div>
            ) : null}
            <div className={rowClass()}>
              <span className={LABEL}>Bật cho</span>
              <div>
                <AgentTicks model={model} label="Bật cho" value={f.agents} onChange={(agents) => set({ agents })} />
              </div>
            </div>
          </div>
        </>
      ) : (
        <div className="grid">
          <div className={rowClass()}>
            <label className={LABEL} htmlFor="mcp-name">Tên</label>
            <div>
              <Input
                id="mcp-name"
                mono
                value={f.name}
                readOnly={f.edit}
                autoComplete="off"
                spellCheck={false}
                onChange={(e) => set({ name: e.target.value.trim() })}
              />
            </div>
          </div>
          <div className={rowClass()}>
            <span className={LABEL}>Chạy bằng</span>
            <div>
              <Segmented variant="inline" aria-label="Chạy bằng">
                <Segment pressed={!http} disabled={f.edit} onClick={() => set({ kind: 'stdio', line: '', pairs: [] })}>
                  Lệnh trên máy
                </Segment>
                <Segment pressed={http} disabled={f.edit} onClick={() => set({ kind: 'http', line: '', pairs: [] })}>
                  Địa chỉ http
                </Segment>
              </Segmented>
            </div>
          </div>
          <div className={rowClass()}>
            <label className={LABEL} htmlFor="mcp-line">{http ? 'Địa chỉ' : 'Lệnh và tham số'}</label>
            <div>
              <Input
                id="mcp-line"
                mono
                value={f.line}
                autoComplete="off"
                spellCheck={false}
                placeholder={http ? 'https://mcp.example.com/mcp' : 'npx -y gói-mcp'}
                onChange={(e) => set({ line: e.target.value })}
              />
              {lineSecret ? (
                <FieldHint>Lệnh có giá trị giống khoá: nên để khoá ở biến môi trường. Xem trước và bản lưu vẫn che nó.</FieldHint>
              ) : !http ? (
                <FieldHint>Tên lệnh như npx hay uvx: agent tìm trong PATH của nó.</FieldHint>
              ) : null}
            </div>
          </div>
          <div className={rowClass()}>
            <span className={LABEL}>{http ? 'Header' : 'Biến môi trường'}</span>
            <div className="grid gap-1.5">
              {f.pairs.map((p, i) => (
                <PairRow
                  key={i}
                  pair={p}
                  i={i}
                  http={http}
                  onChange={(next) => set({ pairs: f.pairs.map((x, j) => (j === i ? next : x)) })}
                  onRemove={() => set({ pairs: f.pairs.filter((_, j) => j !== i) })}
                />
              ))}
              <div>
                <LinkButton small onClick={() => set({ pairs: [...f.pairs, { name: '', value: '', secret: false, kept: false }] })}>
                  + Thêm {http ? 'header' : 'biến'}
                </LinkButton>
              </div>
              {secrets ? (
                <>
                  <FieldHint>Tên có dạng khoá: giá trị vào Keychain, không hiện lại, không vào bản lưu.</FieldHint>
                  <div className="flex items-center gap-2">
                    <label htmlFor="mcp-bw" className="m-0 whitespace-nowrap text-xs text-ink2">
                      Tên trong Bitwarden
                    </label>
                    <Input id="mcp-bw" value={f.bitwarden} onChange={(e) => set({ bitwarden: e.target.value })} />
                  </div>
                </>
              ) : null}
            </div>
          </div>
          <div className={rowClass()}>
            <span className={LABEL}>{f.edit ? 'Ghi cho' : 'Bật cho'}</span>
            <div>
              <AgentTicks model={model} label={f.edit ? 'Ghi cho' : 'Bật cho'} value={f.agents} onChange={(agents) => set({ agents })} />
              {f.edit ? <FieldHint>Agent chưa có {f.name} mà được tick thì thêm mới.</FieldHint> : null}
            </div>
          </div>
        </div>
      )}
      {d.error ? (
        <p className="mt-1 text-xs text-bad-ink" role="alert">
          {d.error}
        </p>
      ) : null}
      <DialogButtons className="mt-3">
        <Button large disabled={mcp.working} onClick={mcp.close}>
          Huỷ
        </Button>
        <Button large primary disabled={mcp.working || (d.mode === 'paste' && !d.saved && !d.found?.servers.length)} onClick={() => mcp.submitForm(d)}>
          {mcp.working ? 'Đang đọc…' : 'Tiếp: xem trước'}
        </Button>
      </DialogButtons>
    </>
  );
}

// ===== Compare =====

function cellText(def: McpDef, what: 'run' | 'args'): string {
  if (def.kind === 'http') return what === 'run' ? def.url : '';
  return what === 'run' ? def.command : def.args.length ? joinCommand(def.args[0], def.args.slice(1)) : '(không có)';
}

function CompareBody({ d, mcp }: { d: Extract<McpDialog, { kind: 'compare' }>; mcp: Mcp }) {
  const row = mcp.table?.rows.find((r) => r.name === d.name);
  if (!row) return <Gone mcp={mcp} />;
  const agents = AGENTS.filter((a) => row.defs[a]);
  const defs = agents.map((a) => row.defs[a]!.def);
  const http = defs[0]?.kind === 'http';
  const pairNames = [...new Set(defs.flatMap((x) => (x.kind === 'http' ? x.headers : x.env).map((p) => p.name)))];
  const lines: { k: string; cells: string[]; secret?: boolean }[] = [
    { k: http ? 'Địa chỉ' : 'Lệnh', cells: defs.map((x) => cellText(x, 'run')) },
    ...(http ? [] : [{ k: 'Tham số', cells: defs.map((x) => cellText(x, 'args')) }]),
    ...pairNames.map((n) => {
      const slot = (http ? 'header:' : 'env:') + n;
      const secret = agents.some((a) => row.defs[a]!.slots.includes(slot));
      const same = row.sameSecrets[slot];
      return {
        k: n,
        secret,
        cells: defs.map((x) => {
          const p = (x.kind === 'http' ? x.headers : x.env).find((q) => q.name === n);
          if (!p) return '(không có)';
          return secret ? (same === undefined ? 'có khoá' : same ? 'giống nhau' : 'khác nhau') : p.value;
        }),
      };
    }),
  ];
  const options = [
    ...agents.map((a) => ({ v: a as string, t: 'Bản của ' + AGENT_NAME[a], s: 'Ghi bản này cho ' + agents.filter((x) => x !== a).map((x) => AGENT_NAME[x]).join(', '), until: null })),
    { v: 'keep', t: 'Để khác nhau', s: 'Không ghi gì', until: null },
    { v: 'form', t: 'Sửa tay', s: 'Mở biểu mẫu', until: null },
  ];
  const next = () => {
    if (d.pick === 'keep') mcp.close();
    else if (d.pick === 'form') mcp.editForm(row);
    else mcp.useOne(row.name, d.pick as Agent);
  };
  return (
    <>
      <DialogTitle>{row.name} khác nhau giữa các agent</DialogTitle>
      <div
        className={COMPARE}
        style={{ gridTemplateColumns: '140px repeat(' + agents.length + ', minmax(0, 1fr))' }}
        role="table"
        aria-label={'So ' + row.name}
      >
        <span className={COMPARE_HEAD} role="columnheader" />
        {agents.map((a) => (
          <span key={a} className={COMPARE_HEAD} role="columnheader">
            <AgentLogo agent={a} />
            {AGENT_NAME[a]}
          </span>
        ))}
        {lines.map((l) => {
          const differs = new Set(l.cells).size > 1 || l.cells.includes('khác nhau');
          return [
            <span key={l.k + ':k'} className="text-ink2" role="rowheader">
              {l.k}
            </span>,
            ...l.cells.map((c, i) => (
              <span key={l.k + ':' + i} className={cn(!l.secret && !c.startsWith('(') && CODE, differs && COMPARE_DIFF)} role="cell">
                {l.secret ? (
                  <>
                    {LOCK_ICON} {c}
                  </>
                ) : (
                  c
                )}
              </span>
            )),
          ];
        })}
      </div>
      <p className={TECH}>
        Khoá chỉ so trong máy, không hiện.
      </p>
      <div className="mt-2.5">
        <ChoiceGroup id="mcp-cmp" label="Dùng bản nào" options={options} value={d.pick} onChange={(pick) => mcp.setDialog({ ...d, pick })} />
      </div>
      <DialogButtons className="mt-3">
        <Button large onClick={mcp.close}>
          Huỷ
        </Button>
        <Button large primary disabled={mcp.working} onClick={next}>
          {d.pick === 'keep' ? 'Xong' : d.pick === 'form' ? 'Mở biểu mẫu' : 'Tiếp: xem trước'}
        </Button>
      </DialogButtons>
    </>
  );
}

// ===== Delete, the store, keys =====

function Gone({ mcp }: { mcp: Mcp }) {
  return (
    <>
      <DialogTitle>Không còn MCP này</DialogTitle>
      <DialogLead>Danh sách vừa đổi. Bấm Đọc lại rồi thử lại.</DialogLead>
      <DialogButtons>
        <Button large primary onClick={mcp.close}>
          Đóng
        </Button>
      </DialogButtons>
    </>
  );
}

function DelBody({ d, mcp }: { d: Extract<McpDialog, { kind: 'del' }>; mcp: Mcp }) {
  const row = mcp.table?.rows.find((r) => r.name === d.name);
  if (!row) return <Gone mcp={mcp} />;
  const where = AGENTS.filter((a) => row.cells[a].state !== 'absent').map((a) => AGENT_NAME[a] + (row.cells[a].parked ? ' (app đang giữ)' : ''));
  const labels = row.secrets.map((s) => s.label).join(', ');
  return (
    <>
      <DialogTitle>Xoá {row.name}?</DialogTitle>
      <DialogLead>Đang có ở {where.join(', ')}.</DialogLead>
      <ChoiceGroup
        id="mcp-del"
        label="Cách xoá"
        value={d.mode}
        options={[
          { v: 'kho', t: 'Tắt ở mọi agent, giữ trong kho', s: 'Bật lại được bất cứ lúc nào', until: null },
          { v: 'hard', t: 'Xoá hẳn', s: 'Bỏ khỏi file của agent và khỏi app', until: null },
        ]}
        onChange={(mode) => mcp.setDialog({ ...d, mode })}
      />
      {d.mode === 'hard' && row.secrets.length ? (
        <CheckLine className="mt-2.5" checked={d.keys} onChange={(keys) => mcp.setDialog({ ...d, keys })}>
          <span>
            Xoá cả khoá {labels} trong Keychain
            <small>Bản lưu trước khi ghi không có khoá; thêm lại sẽ hỏi dán lại.</small>
          </span>
        </CheckLine>
      ) : null}
      <div className="mt-2.5">
        <Facts items={[['ok', 'Xem trước từng file rồi mới ghi. Sao lưu file của ' + where.join(', ') + ' trước khi ghi.']]} />
      </div>
      <DialogButtons>
        <Button large onClick={mcp.close}>
          Huỷ
        </Button>
        <Button large primary disabled={mcp.working} onClick={() => mcp.remove(d)}>
          {d.mode === 'hard' ? 'Tiếp: xem trước khi xoá hẳn' : 'Tiếp: xem trước'}
        </Button>
      </DialogButtons>
    </>
  );
}

function KhoBody({ d, mcp }: { d: Extract<McpDialog, { kind: 'kho' }>; mcp: Mcp }) {
  const k = mcp.table?.stored.find((s) => s.name === d.name);
  if (!k || !mcp.table) return <Gone mcp={mcp} />;
  const files: Record<Agent, string> = Object.fromEntries(mcp.table.columns.map((c) => [c.agent, c.file])) as Record<Agent, string>;
  const can = khoAgents(mcp.table, AGENTS);
  const sse = k.kind === 'sse';
  return (
    <>
      <DialogTitle>Bật lại {k.name}</DialogTitle>
      <DialogLead>Chọn agent. App ghi lại cấu hình đã giữ trong kho, khoá lấy từ Keychain.</DialogLead>
      {k.missing.length ? (
        <FeedbackLine tone="warn" style={{ margin: '0 0 8px' }}>
          <div>
            <FeedbackText title={'Keychain không còn khoá ' + k.missing.join(', ')} body="Dán khoá trước, rồi bật lại." />
          </div>
        </FeedbackLine>
      ) : null}
      <div>
        {AGENTS.map((a) => {
          const off = !can[a] || (a === 'codex' && sse);
          return (
            <CheckRow key={a} off={off} checked={!off && d.agents[a]} disabled={off} onChange={(on) => mcp.setDialog({ ...d, agents: { ...d.agents, [a]: on } })}>
              <span>
                <b>{AGENT_NAME[a]}</b>
                <small>{a === 'claude' ? 'claude mcp add-json --scope user' : files[a]}</small>
              </span>
              <span>{off ? <small>{a === 'codex' && sse ? 'không chạy SSE' : 'chỉ xem'}</small> : null}</span>
            </CheckRow>
          );
        })}
      </div>
      <DialogButtons className="mt-3">
        <Button large onClick={mcp.close}>
          Huỷ
        </Button>
        {k.missing.length ? (
          <Button large primary onClick={() => mcp.setDialog({ kind: 'key', name: k.name })}>
            Dán khoá…
          </Button>
        ) : (
          <Button large primary disabled={mcp.working || !AGENTS.some((a) => d.agents[a])} onClick={() => mcp.unstore(k.name, d.agents)}>
            Tiếp: xem trước
          </Button>
        )}
      </DialogButtons>
    </>
  );
}

function ForgetBody({ d, mcp }: { d: Extract<McpDialog, { kind: 'forget' }>; mcp: Mcp }) {
  const k = mcp.table?.stored.find((s) => s.name === d.name);
  if (!k) return <Gone mcp={mcp} />;
  return (
    <>
      <DialogTitle>Xoá hẳn {k.name}?</DialogTitle>
      <DialogLead>Bỏ khỏi kho của app. Không agent nào đang có {k.name}, nên không file nào của agent đổi.</DialogLead>
      {k.def.slots.length ? (
        <CheckLine checked={d.keys} onChange={(keys) => mcp.setDialog({ ...d, keys })}>
          <span>
            Xoá cả khoá trong Keychain
            <small>Thêm lại sau sẽ phải dán lại khoá.</small>
          </span>
        </CheckLine>
      ) : null}
      <DialogButtons>
        <Button large onClick={mcp.close}>
          Huỷ
        </Button>
        <Button large danger disabled={mcp.working} onClick={() => mcp.forget(k.name, d.keys)}>
          Xoá hẳn
        </Button>
      </DialogButtons>
    </>
  );
}

function KeyBody({ d, mcp }: { d: Extract<McpDialog, { kind: 'key' }>; mcp: Mcp }) {
  const row = mcp.table?.rows.find((r) => r.name === d.name);
  const kept = mcp.table?.stored.find((s) => s.name === d.name);
  const secrets = row ? row.secrets.filter((s) => !s.have) : (kept?.missing ?? []).map((id) => ({ id, label: id.split(':').pop() ?? id, bitwarden: null }));
  const [values, setValues] = useState<Record<string, string>>({});
  const [bitwarden, setBitwarden] = useState('');
  if (!secrets.length)
    return (
      <>
        <DialogTitle>{d.name} đã có đủ khoá</DialogTitle>
        <DialogButtons>
          <Button large primary onClick={mcp.close}>
            Đóng
          </Button>
        </DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>Dán khoá cho {d.name}</DialogTitle>
      {secrets.map((s) => (
        <div key={s.id} className={SECRET_ROW}>
          <span>
            <b>{s.label}</b>
            <small>{s.bitwarden ? 'Bitwarden: ' + s.bitwarden : 'Khoá này chưa có trên máy này'}</small>
          </span>
          <span className="flex items-center gap-1.5">
            <Input
              className="font-mono tracking-[.12em]"
              type="password"
              autoComplete="off"
              aria-label={s.label}
              placeholder="dán vào đây"
              value={values[s.id] ?? ''}
              onChange={(e) => setValues({ ...values, [s.id]: e.target.value })}
            />
          </span>
        </div>
      ))}
      <div className="mt-1 mb-2 grid">
        <div className={rowClass()}>
          <label className={LABEL} htmlFor="mcp-key-bw">Tên trong Bitwarden</label>
          <div>
            <Input id="mcp-key-bw" value={bitwarden} placeholder={secrets[0]?.bitwarden ?? ''} onChange={(e) => setBitwarden(e.target.value)} />
          </div>
        </div>
      </div>
      <Facts items={[['ok', 'Lưu vào Keychain của Quota Station. App không hiện lại giá trị.'], ['ok', 'Sau đó tick ' + d.name + ' cho agent cần dùng.']]} />
      <DialogButtons>
        <Button large onClick={mcp.close}>
          Huỷ
        </Button>
        <Button large primary disabled={mcp.working || !Object.values(values).some(Boolean)} onClick={() => mcp.saveKeys(d.name, values, bitwarden)}>
          Lưu vào Keychain
        </Button>
      </DialogButtons>
    </>
  );
}

function HowCmdBody({ d, mcp }: { d: Extract<McpDialog, { kind: 'howCmd' }>; mcp: Mcp }) {
  const row = mcp.table?.rows.find((r) => r.name === d.name);
  const command = row?.missingCommand?.command ?? '';
  return (
    <>
      <DialogTitle>Cài lệnh của {d.name}</DialogTitle>
      <DialogLead>{d.name} chạy lệnh này, mà máy này chưa có:</DialogLead>
      <Command>{command}</Command>
      <p className={TECH}>App không tự cài lệnh. Cài theo hướng dẫn của MCP đó (thường là npm, brew hay uv), rồi bấm Đọc lại.</p>
      <DialogButtons>
        <Button large onClick={mcp.close}>
          Đóng
        </Button>
        <Button
          large
          primary
          onClick={() => {
            mcp.close();
            mcp.reread();
          }}
        >
          Đọc lại
        </Button>
      </DialogButtons>
    </>
  );
}

function OwnedBody({ mcp }: { mcp: Mcp }) {
  const owned = mcp.table?.owned ?? [];
  return (
    <>
      <DialogTitle>MCP có sẵn trong agent và plugin</DialogTitle>
      <div className="grid">
        <div className={cn(MATRIX_ROW, MATRIX_HEAD)}>
          <span>Tên</span>
          {AGENTS.map((a) => (
            <span key={a} className="relative justify-center">
              <span className={HEAD_OFF} title={AGENT_NAME[a]}>
                <AgentLogo agent={a} />
                <span className="sr-only">{AGENT_NAME[a]}</span>
              </span>
            </span>
          ))}
        </div>
        {owned.map((o) => (
          <div key={o.name} className={MATRIX_ROW} role="group" aria-label={o.name}>
            <div className={cn(MATRIX_NAME, '[&_b]:font-med [&_b]:text-ink2')}>
              <div className={MATRIX_TITLE}>
                <b>{o.name}</b>
              </div>
              <small>của {o.owner}</small>
            </div>
            {AGENTS.map((a) => (
              <span key={a} className={MATRIX_CELL}>
                {o.agents.includes(a) ? (
                  <span className="text-sm text-ink2" role="img" aria-label={'có ở ' + AGENT_NAME[a]}>
                    ✓
                  </span>
                ) : (
                  <span className="text-ink3" role="img" aria-label={'không có ở ' + AGENT_NAME[a]}>
                    —
                  </span>
                )}
              </span>
            ))}
          </div>
        ))}
      </div>
      <div className="mt-2.5">
        <Facts
          items={[
            ['', 'Đến từ chính agent hoặc từ plugin, nên app chỉ xem: không bật tắt, không sao lưu.'],
            ['', 'Cài lại agent hay plugin là chúng tự có lại.'],
          ]}
        />
      </div>
      <DialogButtons>
        <Button large primary onClick={mcp.close}>
          Đóng
        </Button>
      </DialogButtons>
    </>
  );
}

const ACTION_WORD: Record<string, string> = { on: 'bật', off: 'tắt', add: 'thêm', remove: 'bỏ', replace: 'sửa' };

function backupNote(agent: Agent, changes: string[]): string {
  if (!changes.length) return agent === 'codex' ? 'trước khi chuyển Codex hoặc trước một lần ghi khác' : 'trước khi ghi';
  return (
    'trước khi ' +
    changes
      .map((c) => {
        const at = c.lastIndexOf(':');
        return (ACTION_WORD[c.slice(at + 1)] ?? 'ghi') + ' ' + c.slice(0, at);
      })
      .join(', ')
  );
}

function BackupsBody({ mcp, model, now }: { mcp: Mcp; model: Model; now: Date }) {
  const list = model.mcp.backups;
  const agents = AGENTS.filter((a) => list.some((b) => b.agent === a));
  return (
    <>
      <DialogTitle>Bản đã lưu trước khi ghi</DialogTitle>
      <DialogLead>Giữ 10 bản gần nhất cho mỗi agent, trên máy này. Bản lưu không chứa khoá: giá trị của khoá được thay bằng &lt;redacted&gt;.</DialogLead>
      {list.length ? (
        <div>
          {list.slice(0, 30).map((b) => (
            <div key={b.agent + b.file} className={PACK_ROW}>
              <span className={PACK_META}>{dayClock(b.at, now)}</span>
              <span>
                {AGENT_NAME[b.agent]}
                <small className="block text-xs text-ink2">{backupNote(b.agent, b.changes)}</small>
              </span>
              <span className={cn(PACK_META, 'rounded-sm bg-fill px-1 font-mono leading-[normal] [overflow-wrap:anywhere]')}>{b.file}</span>
            </div>
          ))}
        </div>
      ) : (
        <DialogLead>Chưa có bản lưu nào: app chưa ghi danh sách MCP lần nào.</DialogLead>
      )}
      <DialogButtons className="mt-3">
        {agents.map((a) => (
          <Button key={a} large onClick={() => mcp.openBackups(a)}>
            Mở thư mục {AGENT_NAME[a]}
          </Button>
        ))}
        <Button large primary onClick={mcp.close}>
          Đóng
        </Button>
      </DialogButtons>
    </>
  );
}

export function McpDialogBody({ mcp, model, now }: { mcp: Mcp; model: Model; now: Date }) {
  const d = mcp.dialog;
  if (!d) return null;
  const rows = mcp.table?.rows ?? [];
  switch (d.kind) {
    case 'batch':
      return <BatchBody d={d} mcp={mcp} rows={rows} />;
    case 'form':
      return <FormBody d={d} mcp={mcp} model={model} />;
    case 'compare':
      return <CompareBody d={d} mcp={mcp} />;
    case 'del':
      return <DelBody d={d} mcp={mcp} />;
    case 'kho':
      return <KhoBody d={d} mcp={mcp} />;
    case 'forget':
      return <ForgetBody d={d} mcp={mcp} />;
    case 'key':
      return <KeyBody key={d.name} d={d} mcp={mcp} />;
    case 'howCmd':
      return <HowCmdBody d={d} mcp={mcp} />;
    case 'owned':
      return <OwnedBody mcp={mcp} />;
    case 'backups':
      return <BackupsBody mcp={mcp} model={model} now={now} />;
  }
}
