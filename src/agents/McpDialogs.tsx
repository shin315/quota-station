// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

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
import { ServiceIcon } from '../components/logos';
import { LABEL, rowClass } from '../components/page';
import { Button, ChoiceGroup, CODE, Facts as FactList, FeedbackLine, FeedbackText, FieldHint, LinkButton } from '../components/parts';
import {
  AGENT_NAME,
  AGENTS,
  isSecretPair,
  joinCommand,
  lineHasSecret,
  previewLead,
  refusalText,
  restartText,
  type FormPair,
  type McpForm,
} from '../lib/mcp';
import { serviceOf } from '../lib/brands';
import type { Agent, McpBlock, McpDef, McpPaste, McpRow, Model } from '../lib/model';
import { t } from '../lib/i18n';
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
  return <StepList names={[t('Điền', 'Fill in'), t('Xem trước', 'Preview')]} at={at} />;
}

export function Facts({ items }: { items: [string, string][] }) {
  return <FactList items={items.map(([k, text]) => ({ k, text }))} />;
}

function blockLabel(agent: Agent, b: McpBlock): string {
  const what = agent === 'codex' ? '[mcp_servers.' + b.name + ']' : b.name;
  if (b.action === 'add') return t('thêm mục ' + what, 'add entry ' + what);
  if (b.action === 'remove') return t('bỏ mục ' + what, 'remove entry ' + what);
  return t('mục ' + what, 'entry ' + what);
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
  const notes = [t('Claude Code tự ghi ~/.claude.json.', 'Claude Code writes ~/.claude.json itself.')];
  if (b.kept && b.action === 'off') notes.push(t('App giữ cấu hình của ' + b.name + ' để bật lại.', 'The app keeps ' + b.name + '’s config to turn it back on.'));
  else if (b.kept) notes.push(t('App giữ cấu hình trong kho, khoá trong Keychain.', 'The app keeps the config in its store and the keys in the Keychain.'));
  if (b.secret && b.action !== 'off' && b.action !== 'remove') notes.push(t('Khoá đi vào lệnh, không vào log hay bản lưu.', 'The key goes into the command, not into logs or backups.'));
  return notes.join(' ');
}

export function BlockView({ agent, file, block }: { agent: Agent; file: string; block: McpBlock }) {
  const extra = [
    block.from ? t('Chép từ ' + AGENT_NAME[block.from] + '.', 'Copied from ' + AGENT_NAME[block.from] + '.') : '',
    block.dropped.length
      ? t(
          AGENT_NAME[agent] + ' không có ' + block.dropped.join(', ') + ': bỏ khi ghi.',
          AGENT_NAME[agent] + ' has no ' + block.dropped.join(', ') + ': left out when writing.',
        )
      : '',
    agent !== 'claude' && block.kept ? t('App giữ cấu hình trong kho, khoá trong Keychain.', 'The app keeps the config in its store and the keys in the Keychain.') : '',
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
  const files = p.groups.map((g) => (g.agent === 'claude' ? t('danh sách MCP của Claude Code', 'Claude Code’s MCP list') : g.file)).join(', ');
  const facts: [string, string][] = [
    ['ok', t('Sao lưu ' + files + ' trước khi ghi. Bản lưu không chứa khoá.', 'Backs up ' + files + ' before writing. Backups hold no keys.')],
    [
      'ok',
      t(
        'Chỉ sửa đúng mục trong xem trước; phần còn lại của file giữ nguyên. Mỗi file ghi một lần.',
        'Only edits the entries in the preview; the rest of each file stays as it is. Each file is written once.',
      ),
    ],
    ...p.refused.map((r): [string, string] => [
      'x',
      t(
        'Bỏ qua ' + r.name + ' cho ' + AGENT_NAME[r.agent] + ': ' + refusalText(r.reason) + '.',
        'Skipping ' + r.name + ' for ' + AGENT_NAME[r.agent] + ': ' + refusalText(r.reason) + '.',
      ),
    ]),
    ...p.groups.map((g): [string, string] => ['w', restartText(g.agent)]),
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
          {t('Quay lại', 'Back')}
        </Button>
        <Button large primary disabled={mcp.working} onClick={mcp.write}>
          {mcp.working ? t('Đang ghi…', 'Writing…') : t('Ghi ' + n + ' thay đổi', n === 1 ? 'Write 1 change' : 'Write ' + n + ' changes')}
        </Button>
      </DialogButtons>
    </>
  );
}

// ===== The add and edit form =====

function PairRow(props: { pair: FormPair; i: number; http: boolean; onChange(p: FormPair): void; onRemove(): void }) {
  const p = props.pair;
  const label = props.http ? t('Header', 'Header') : t('Biến', 'Variable');
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_auto] items-center gap-1.5">
      <Input
        mono
        aria-label={t(label + ' ' + (props.i + 1) + ': tên', label + ' ' + (props.i + 1) + ': name')}
        value={p.name}
        placeholder={props.http ? 'Authorization' : 'API_KEY'}
        onChange={(e) => props.onChange({ ...p, name: e.target.value, secret: p.kept || isSecretPair(e.target.value, p.value) })}
      />
      <span className="flex min-w-0 items-center gap-1.5">
        <Input
          type={p.secret ? 'password' : 'text'}
          autoComplete="off"
          spellCheck={false}
          aria-label={t(label + ' ' + (props.i + 1) + ': giá trị', label + ' ' + (props.i + 1) + ': value')}
          value={p.value}
          placeholder={p.kept ? t('giữ khoá đang có', 'keep the current key') : t('giá trị', 'value')}
          onChange={(e) => props.onChange({ ...p, value: e.target.value, secret: p.kept || isSecretPair(p.name, e.target.value) })}
        />
        {p.secret ? (
          <span className="inline-flex items-center gap-1 whitespace-nowrap text-xs text-info-ink [&_svg]:flex-none" title={t('Giá trị vào Keychain, không hiện lại', 'The value goes to the Keychain and is not shown again')}>
            {LOCK_ICON}
            Keychain
          </span>
        ) : null}
      </span>
      <LinkButton small aria-label={t('Bỏ dòng ' + (p.name || props.i + 1), 'Remove row ' + (p.name || props.i + 1))} onClick={props.onRemove}>
        {t('Bỏ', 'Remove')}
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
            title={can ? undefined : t(AGENT_NAME[a] + ': chỉ xem trên máy này', AGENT_NAME[a] + ': view only on this Mac')}
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
    tooLong: t('Đoạn dán quá dài cho một danh sách MCP.', 'The pasted text is too long for an MCP list.'),
    parse: t('Không đọc được: ' + (p.detail ?? 'sai cú pháp') + '.', 'Cannot read it: ' + (p.detail ?? 'bad syntax') + '.'),
    noName: t(
      'Thiếu tên MCP. Dán dạng {"tên": {"command": …}} hoặc cả khối "mcpServers".',
      'The MCP name is missing. Paste {"name": {"command": …}} or the whole "mcpServers" block.',
    ),
    none: t('Không thấy MCP nào trong đoạn này.', 'No MCP found in this text.'),
    keychain: t('Chưa lưu được khoá vào Keychain. Chưa ghi gì.', 'Could not save the key to the Keychain. Nothing was written.'),
  };
  if (p.error) return { facts: [], error: errors[p.error] || null };
  const facts: [string, string][] = [];
  const names = p.servers.map((s) => s.name);
  if (names.length) facts.push(['ok', t('Tìm thấy ' + names.length + ' MCP: ' + names.join(', ') + '.', 'Found ' + names.length + ' MCP: ' + names.join(', ') + '.')]);
  const secrets = p.servers.flatMap((s) => s.slots.map((x) => x.label));
  if (secrets.length)
    facts.push([
      'ok',
      t(
        secrets.length + ' giá trị có dạng khoá: ' + secrets.join(', ') + '. ' + (saved ? 'Đã lưu vào Keychain.' : 'App che giá trị và lưu vào Keychain khi bấm Tiếp.'),
        (secrets.length === 1 ? '1 value looks like a key: ' : secrets.length + ' values look like keys: ') +
          secrets.join(', ') +
          '. ' +
          (saved ? 'Saved to the Keychain.' : 'The app masks them and saves them to the Keychain when you press Next.'),
      ),
    ]);
  if (p.skipped.length)
    facts.push([
      'w',
      t(
        'Bỏ qua ' + p.skipped.join(', ') + ': tên có ký tự lạ, hoặc không có lệnh hay địa chỉ.',
        'Skipping ' + p.skipped.join(', ') + ': the name has odd characters, or there is no command or address.',
      ),
    ]);
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
      <DialogTitle>{f.edit ? t('Sửa ' + f.name, 'Edit ' + f.name) : t('Thêm MCP', 'Add MCP')}</DialogTitle>
      {f.edit ? null : <Steps at={0} />}
      {f.edit ? null : (
        <Segmented variant="inline" className="mt-0 mb-2.5" aria-label={t('Cách thêm', 'How to add')}>
          <Segment pressed={d.mode === 'form'} onClick={() => mcp.setDialog({ ...d, mode: 'form', error: null })}>
            {t('Điền tay', 'Fill in by hand')}
          </Segment>
          <Segment pressed={d.mode === 'paste'} onClick={() => mcp.setDialog({ ...d, mode: 'paste', error: null })}>
            {t('Dán JSON hoặc TOML', 'Paste JSON or TOML')}
          </Segment>
        </Segmented>
      )}
      {d.mode === 'paste' ? (
        <>
          {d.saved ? (
            <DialogLead>
              {t(
                'Khoá đã vào Keychain. Bấm Tiếp để xem lại phần sẽ ghi, hoặc dán đoạn khác.',
                'The keys are in the Keychain. Press Next to review what will be written, or paste something else.',
              )}
            </DialogLead>
          ) : null}
          <Textarea
            mono
            rows={9}
            aria-label={t('JSON hoặc TOML', 'JSON or TOML')}
            spellCheck={false}
            autoComplete="off"
            style={{ resize: 'vertical' }}
            placeholder={t(
              '{\n  "mcpServers": {\n    "tên": { "command": "npx", "args": ["-y", "gói-mcp"] }\n  }\n}',
              '{\n  "mcpServers": {\n    "name": { "command": "npx", "args": ["-y", "some-mcp"] }\n  }\n}',
            )}
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
                <label className={LABEL} htmlFor="mcp-bw">
                  {t('Lưu ở đâu (gợi ý)', 'Where it is stored (hint)')}
                </label>
                <div>
                  <Input id="mcp-bw" value={f.bitwarden} onChange={(e) => set({ bitwarden: e.target.value })} />
                  <FieldHint>
                    {t('Chỉ là gợi ý để biết khoá nằm ở mục nào khi phải dán lại.', 'Only a hint, so you know where to find the key if you have to paste it again.')}
                  </FieldHint>
                </div>
              </div>
            ) : null}
            <div className={rowClass()}>
              <span className={LABEL}>{t('Bật cho', 'Turn on for')}</span>
              <div>
                <AgentTicks model={model} label={t('Bật cho', 'Turn on for')} value={f.agents} onChange={(agents) => set({ agents })} />
              </div>
            </div>
          </div>
        </>
      ) : (
        <div className="grid">
          <div className={rowClass()}>
            <label className={LABEL} htmlFor="mcp-name">
              {t('Tên', 'Name')}
            </label>
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
            <span className={LABEL}>{t('Chạy bằng', 'Runs as')}</span>
            <div>
              <Segmented variant="inline" aria-label={t('Chạy bằng', 'Runs as')}>
                <Segment pressed={!http} disabled={f.edit} onClick={() => set({ kind: 'stdio', line: '', pairs: [] })}>
                  {t('Lệnh trên máy', 'Local command')}
                </Segment>
                <Segment pressed={http} disabled={f.edit} onClick={() => set({ kind: 'http', line: '', pairs: [] })}>
                  {t('Địa chỉ http', 'http address')}
                </Segment>
              </Segmented>
            </div>
          </div>
          <div className={rowClass()}>
            <label className={LABEL} htmlFor="mcp-line">
              {http ? t('Địa chỉ', 'Address') : t('Lệnh và tham số', 'Command and arguments')}
            </label>
            <div>
              <Input
                id="mcp-line"
                mono
                value={f.line}
                autoComplete="off"
                spellCheck={false}
                placeholder={http ? 'https://mcp.example.com/mcp' : t('npx -y gói-mcp', 'npx -y some-mcp')}
                onChange={(e) => set({ line: e.target.value })}
              />
              {lineSecret ? (
                <FieldHint>
                  {t(
                    'Lệnh có giá trị giống khoá: nên để khoá ở biến môi trường. Xem trước và bản lưu vẫn che nó.',
                    'The command holds something that looks like a key: better put keys in environment variables. The preview and backups still mask it.',
                  )}
                </FieldHint>
              ) : !http ? (
                <FieldHint>{t('Tên lệnh như npx hay uvx: agent tìm trong PATH của nó.', 'A command name such as npx or uvx: the agent finds it in its PATH.')}</FieldHint>
              ) : null}
            </div>
          </div>
          <div className={rowClass()}>
            <span className={LABEL}>{http ? t('Header', 'Headers') : t('Biến môi trường', 'Environment variables')}</span>
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
                  {http ? t('+ Thêm header', '+ Add header') : t('+ Thêm biến', '+ Add variable')}
                </LinkButton>
              </div>
              {secrets ? (
                <>
                  <FieldHint>
                    {t(
                      'Tên có dạng khoá: giá trị vào Keychain, không hiện lại, không vào bản lưu.',
                      'The name looks like a key: the value goes to the Keychain, is not shown again and stays out of backups.',
                    )}
                  </FieldHint>
                  <div className="flex items-center gap-2">
                    <label htmlFor="mcp-bw" className="m-0 whitespace-nowrap text-xs text-ink2">
                      {t('Lưu ở đâu (gợi ý)', 'Where it is stored (hint)')}
                    </label>
                    <Input id="mcp-bw" value={f.bitwarden} onChange={(e) => set({ bitwarden: e.target.value })} />
                  </div>
                </>
              ) : null}
            </div>
          </div>
          <div className={rowClass()}>
            <span className={LABEL}>{f.edit ? t('Ghi cho', 'Write for') : t('Bật cho', 'Turn on for')}</span>
            <div>
              <AgentTicks model={model} label={f.edit ? t('Ghi cho', 'Write for') : t('Bật cho', 'Turn on for')} value={f.agents} onChange={(agents) => set({ agents })} />
              {f.edit ? (
                <FieldHint>{t('Agent chưa có ' + f.name + ' mà được tick thì thêm mới.', 'A ticked agent that does not have ' + f.name + ' yet gets it added.')}</FieldHint>
              ) : null}
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
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={mcp.working || (d.mode === 'paste' && !d.saved && !d.found?.servers.length)} onClick={() => mcp.submitForm(d)}>
          {mcp.working ? t('Đang đọc…', 'Reading…') : t('Tiếp: xem trước', 'Next: preview')}
        </Button>
      </DialogButtons>
    </>
  );
}

// ===== Compare =====

/** A cell of the comparison: `none` when the agent has nothing there, `differs` when only the hidden keys say so. */
interface CompareCell {
  text: string;
  none?: boolean;
  differs?: boolean;
}

const noneCell = (): CompareCell => ({ text: t('(không có)', '(none)'), none: true });

function cellText(def: McpDef, what: 'run' | 'args'): CompareCell {
  if (def.kind === 'http') return { text: what === 'run' ? def.url : '' };
  return what === 'run' ? { text: def.command } : def.args.length ? { text: joinCommand(def.args[0], def.args.slice(1)) } : noneCell();
}

function CompareBody({ d, mcp }: { d: Extract<McpDialog, { kind: 'compare' }>; mcp: Mcp }) {
  const row = mcp.table?.rows.find((r) => r.name === d.name);
  if (!row) return <Gone mcp={mcp} />;
  const agents = AGENTS.filter((a) => row.defs[a]);
  const defs = agents.map((a) => row.defs[a]!.def);
  const http = defs[0]?.kind === 'http';
  const pairNames = [...new Set(defs.flatMap((x) => (x.kind === 'http' ? x.headers : x.env).map((p) => p.name)))];
  const lines: { k: string; cells: CompareCell[]; secret?: boolean }[] = [
    { k: http ? t('Địa chỉ', 'Address') : t('Lệnh', 'Command'), cells: defs.map((x) => cellText(x, 'run')) },
    ...(http ? [] : [{ k: t('Tham số', 'Arguments'), cells: defs.map((x) => cellText(x, 'args')) }]),
    ...pairNames.map((n) => {
      const slot = (http ? 'header:' : 'env:') + n;
      const secret = agents.some((a) => row.defs[a]!.slots.includes(slot));
      const same = row.sameSecrets[slot];
      return {
        k: n,
        secret,
        cells: defs.map((x): CompareCell => {
          const p = (x.kind === 'http' ? x.headers : x.env).find((q) => q.name === n);
          if (!p) return noneCell();
          if (!secret) return { text: p.value };
          if (same === undefined) return { text: t('có khoá', 'has a key') };
          return same ? { text: t('giống nhau', 'the same') } : { text: t('khác nhau', 'different'), differs: true };
        }),
      };
    }),
  ];
  const options = [
    ...agents.map((a) => {
      const others = agents
        .filter((x) => x !== a)
        .map((x) => AGENT_NAME[x])
        .join(', ');
      return {
        v: a as string,
        t: t('Bản của ' + AGENT_NAME[a], AGENT_NAME[a] + '’s version'),
        s: t('Ghi bản này cho ' + others, 'Write this version for ' + others),
        until: null,
      };
    }),
    { v: 'keep', t: t('Để khác nhau', 'Leave them different'), s: t('Không ghi gì', 'Writes nothing'), until: null },
    { v: 'form', t: t('Sửa tay', 'Edit by hand'), s: t('Mở biểu mẫu', 'Opens the form'), until: null },
  ];
  const next = () => {
    if (d.pick === 'keep') mcp.close();
    else if (d.pick === 'form') mcp.editForm(row);
    else mcp.useOne(row.name, d.pick as Agent);
  };
  return (
    <>
      <DialogTitle>{t(row.name + ' khác nhau giữa các agent', row.name + ' differs between agents')}</DialogTitle>
      <div
        className={COMPARE}
        style={{ gridTemplateColumns: '140px repeat(' + agents.length + ', minmax(0, 1fr))' }}
        role="table"
        aria-label={t('So ' + row.name, 'Compare ' + row.name)}
      >
        <span className={COMPARE_HEAD} role="columnheader" />
        {agents.map((a) => (
          <span key={a} className={COMPARE_HEAD} role="columnheader">
            <AgentLogo agent={a} />
            {AGENT_NAME[a]}
          </span>
        ))}
        {lines.map((l) => {
          const differs = new Set(l.cells.map((c) => c.text)).size > 1 || l.cells.some((c) => c.differs);
          return [
            <span key={l.k + ':k'} className="text-ink2" role="rowheader">
              {l.k}
            </span>,
            ...l.cells.map((c, i) => (
              <span key={l.k + ':' + i} className={cn(!l.secret && !c.none && CODE, differs && COMPARE_DIFF)} role="cell">
                {l.secret ? (
                  <>
                    {LOCK_ICON} {c.text}
                  </>
                ) : (
                  c.text
                )}
              </span>
            )),
          ];
        })}
      </div>
      <p className={TECH}>
        {t('Khoá chỉ so trong máy, không hiện.', 'Keys are only compared on this Mac, never shown.')}
      </p>
      <div className="mt-2.5">
        <ChoiceGroup id="mcp-cmp" label={t('Dùng bản nào', 'Which version to use')} options={options} value={d.pick} onChange={(pick) => mcp.setDialog({ ...d, pick })} />
      </div>
      <DialogButtons className="mt-3">
        <Button large onClick={mcp.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={mcp.working} onClick={next}>
          {d.pick === 'keep' ? t('Xong', 'Done') : d.pick === 'form' ? t('Mở biểu mẫu', 'Open the form') : t('Tiếp: xem trước', 'Next: preview')}
        </Button>
      </DialogButtons>
    </>
  );
}

// ===== Delete, the store, keys =====

function Gone({ mcp }: { mcp: Mcp }) {
  return (
    <>
      <DialogTitle>{t('Không còn MCP này', 'This MCP is gone')}</DialogTitle>
      <DialogLead>{t('Danh sách vừa đổi. Bấm Đọc lại rồi thử lại.', 'The list just changed. Press Read again, then try again.')}</DialogLead>
      <DialogButtons>
        <Button large primary onClick={mcp.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </>
  );
}

function DelBody({ d, mcp }: { d: Extract<McpDialog, { kind: 'del' }>; mcp: Mcp }) {
  const row = mcp.table?.rows.find((r) => r.name === d.name);
  if (!row) return <Gone mcp={mcp} />;
  const where = AGENTS.filter((a) => row.cells[a].state !== 'absent').map((a) => AGENT_NAME[a] + (row.cells[a].parked ? t(' (app đang giữ)', ' (kept by the app)') : ''));
  const labels = row.secrets.map((s) => s.label).join(', ');
  return (
    <>
      <DialogTitle>{t('Xoá ' + row.name + '?', 'Delete ' + row.name + '?')}</DialogTitle>
      <DialogLead>{t('Đang có ở ' + where.join(', ') + '.', 'Now in ' + where.join(', ') + '.')}</DialogLead>
      <ChoiceGroup
        id="mcp-del"
        label={t('Cách xoá', 'How to delete')}
        value={d.mode}
        options={[
          { v: 'kho', t: t('Tắt ở mọi agent, giữ trong kho', 'Turn off for every agent, keep in the store'), s: t('Bật lại được bất cứ lúc nào', 'Can be turned back on at any time'), until: null },
          { v: 'hard', t: t('Xoá hẳn', 'Delete for good'), s: t('Bỏ khỏi file của agent và khỏi app', 'Removes it from the agents’ files and from the app'), until: null },
        ]}
        onChange={(mode) => mcp.setDialog({ ...d, mode })}
      />
      {d.mode === 'hard' && row.secrets.length ? (
        <CheckLine className="mt-2.5" checked={d.keys} onChange={(keys) => mcp.setDialog({ ...d, keys })}>
          <span>
            {t('Xoá cả khoá ' + labels + ' trong Keychain', 'Also delete the ' + labels + ' key from the Keychain')}
            <small>{t('Bản lưu trước khi ghi không có khoá; thêm lại sẽ hỏi dán lại.', 'Backups hold no keys; adding it again will ask you to paste them.')}</small>
          </span>
        </CheckLine>
      ) : null}
      <div className="mt-2.5">
        <Facts
          items={[
            [
              'ok',
              t(
                'Xem trước từng file rồi mới ghi. Sao lưu file của ' + where.join(', ') + ' trước khi ghi.',
                'Previews each file before writing. Backs up the files of ' + where.join(', ') + ' before writing.',
              ),
            ],
          ]}
        />
      </div>
      <DialogButtons>
        <Button large onClick={mcp.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={mcp.working} onClick={() => mcp.remove(d)}>
          {d.mode === 'hard' ? t('Tiếp: xem trước khi xoá hẳn', 'Next: preview before deleting') : t('Tiếp: xem trước', 'Next: preview')}
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
      <DialogTitle>{t('Bật lại ' + k.name, 'Turn ' + k.name + ' back on')}</DialogTitle>
      <DialogLead>
        {t(
          'Chọn agent. App ghi lại cấu hình đã giữ trong kho, khoá lấy từ Keychain.',
          'Choose the agents. The app writes back the config kept in its store, with the keys from the Keychain.',
        )}
      </DialogLead>
      {k.missing.length ? (
        <FeedbackLine tone="warn" style={{ margin: '0 0 8px' }}>
          <div>
            <FeedbackText
              title={t('Keychain không còn khoá ' + k.missing.join(', '), 'The Keychain no longer has the key ' + k.missing.join(', '))}
              body={t('Dán khoá trước, rồi bật lại.', 'Paste the key first, then turn it back on.')}
            />
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
              <span>{off ? <small>{a === 'codex' && sse ? t('không chạy SSE', 'does not run SSE') : t('chỉ xem', 'view only')}</small> : null}</span>
            </CheckRow>
          );
        })}
      </div>
      <DialogButtons className="mt-3">
        <Button large onClick={mcp.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        {k.missing.length ? (
          <Button large primary onClick={() => mcp.setDialog({ kind: 'key', name: k.name })}>
            {t('Dán khoá…', 'Paste key…')}
          </Button>
        ) : (
          <Button large primary disabled={mcp.working || !AGENTS.some((a) => d.agents[a])} onClick={() => mcp.unstore(k.name, d.agents)}>
            {t('Tiếp: xem trước', 'Next: preview')}
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
      <DialogTitle>{t('Xoá hẳn ' + k.name + '?', 'Delete ' + k.name + ' for good?')}</DialogTitle>
      <DialogLead>
        {t(
          'Bỏ khỏi kho của app. Không agent nào đang có ' + k.name + ', nên không file nào của agent đổi.',
          'Removes it from the app’s store. No agent has ' + k.name + ' now, so no agent file changes.',
        )}
      </DialogLead>
      {k.def.slots.length ? (
        <CheckLine checked={d.keys} onChange={(keys) => mcp.setDialog({ ...d, keys })}>
          <span>
            {t('Xoá cả khoá trong Keychain', 'Also delete its keys from the Keychain')}
            <small>{t('Thêm lại sau sẽ phải dán lại khoá.', 'Adding it again later means pasting the keys again.')}</small>
          </span>
        </CheckLine>
      ) : null}
      <DialogButtons>
        <Button large onClick={mcp.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large danger disabled={mcp.working} onClick={() => mcp.forget(k.name, d.keys)}>
          {t('Xoá hẳn', 'Delete for good')}
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
        <DialogTitle>{t(d.name + ' đã có đủ khoá', d.name + ' has all its keys')}</DialogTitle>
        <DialogButtons>
          <Button large primary onClick={mcp.close}>
            {t('Đóng', 'Close')}
          </Button>
        </DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>{t('Dán khoá cho ' + d.name, 'Paste keys for ' + d.name)}</DialogTitle>
      {secrets.map((s) => (
        <div key={s.id} className={SECRET_ROW}>
          <span>
            <b>{s.label}</b>
            <small>{s.bitwarden ? t('Lưu ở: ' + s.bitwarden, 'Stored in: ' + s.bitwarden) : t('Khoá này chưa có trên máy này', 'This key is not on this Mac yet')}</small>
          </span>
          <span className="flex items-center gap-1.5">
            <Input
              className="font-mono tracking-[.12em]"
              type="password"
              autoComplete="off"
              aria-label={s.label}
              placeholder={t('dán vào đây', 'paste here')}
              value={values[s.id] ?? ''}
              onChange={(e) => setValues({ ...values, [s.id]: e.target.value })}
            />
          </span>
        </div>
      ))}
      <div className="mt-1 mb-2 grid">
        <div className={rowClass()}>
          <label className={LABEL} htmlFor="mcp-key-bw">
            {t('Lưu ở đâu (gợi ý)', 'Where it is stored (hint)')}
          </label>
          <div>
            <Input id="mcp-key-bw" value={bitwarden} placeholder={secrets[0]?.bitwarden ?? ''} onChange={(e) => setBitwarden(e.target.value)} />
          </div>
        </div>
      </div>
      <Facts
        items={[
          ['ok', t('Lưu vào Keychain của Quota Station. App không hiện lại giá trị.', 'Saved to Quota Station’s Keychain. The app does not show the value again.')],
          ['ok', t('Sau đó tick ' + d.name + ' cho agent cần dùng.', 'Then tick ' + d.name + ' for the agents that need it.')],
        ]}
      />
      <DialogButtons>
        <Button large onClick={mcp.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={mcp.working || !Object.values(values).some(Boolean)} onClick={() => mcp.saveKeys(d.name, values, bitwarden)}>
          {t('Lưu vào Keychain', 'Save to Keychain')}
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
      <DialogTitle>{t('Cài lệnh của ' + d.name, 'Install ' + d.name + '’s command')}</DialogTitle>
      <DialogLead>{t(d.name + ' chạy lệnh này, mà máy này chưa có:', d.name + ' runs this command, which this Mac does not have:')}</DialogLead>
      <Command>{command}</Command>
      <p className={TECH}>
        {t(
          'App không tự cài lệnh. Cài theo hướng dẫn của MCP đó (thường là npm, brew hay uv), rồi bấm Đọc lại.',
          'The app does not install commands itself. Install it as that MCP’s guide says (usually npm, brew or uv), then press Read again.',
        )}
      </p>
      <DialogButtons>
        <Button large onClick={mcp.close}>
          {t('Đóng', 'Close')}
        </Button>
        <Button
          large
          primary
          onClick={() => {
            mcp.close();
            mcp.reread();
          }}
        >
          {t('Đọc lại', 'Read again')}
        </Button>
      </DialogButtons>
    </>
  );
}

function OwnedBody({ mcp }: { mcp: Mcp }) {
  const owned = mcp.table?.owned ?? [];
  return (
    <>
      <DialogTitle>{t('MCP có sẵn trong agent và plugin', 'MCP built into agents and plugins')}</DialogTitle>
      <div className="grid">
        <div className={cn(MATRIX_ROW, MATRIX_HEAD)}>
          <span>{t('Tên', 'Name')}</span>
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
                <ServiceIcon service={serviceOf(o.name)} />
                <b>{o.name}</b>
              </div>
              <small>{t('của ' + o.owner, 'from ' + o.owner)}</small>
            </div>
            {AGENTS.map((a) => (
              <span key={a} className={MATRIX_CELL}>
                {o.agents.includes(a) ? (
                  <span className="text-sm text-ink2" role="img" aria-label={t('có ở ' + AGENT_NAME[a], 'in ' + AGENT_NAME[a])}>
                    ✓
                  </span>
                ) : (
                  <span className="text-ink3" role="img" aria-label={t('không có ở ' + AGENT_NAME[a], 'not in ' + AGENT_NAME[a])}>
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
            ['', t('Đến từ chính agent hoặc từ plugin, nên app chỉ xem: không bật tắt, không sao lưu.', 'They come from the agent itself or a plugin, so the app only shows them: no turning on or off, no backups.')],
            ['', t('Cài lại agent hay plugin là chúng tự có lại.', 'Reinstalling the agent or plugin brings them back.')],
          ]}
        />
      </div>
      <DialogButtons>
        <Button large primary onClick={mcp.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </>
  );
}

function actionWord(action: string): string {
  switch (action) {
    case 'on':
      return t('bật', 'turning on');
    case 'off':
      return t('tắt', 'turning off');
    case 'add':
      return t('thêm', 'adding');
    case 'remove':
      return t('bỏ', 'removing');
    case 'replace':
      return t('sửa', 'editing');
    default:
      return t('ghi', 'writing');
  }
}

function backupNote(agent: Agent, changes: string[]): string {
  if (!changes.length)
    return agent === 'codex' ? t('trước khi chuyển Codex hoặc trước một lần ghi khác', 'before a Codex switch or another write') : t('trước khi ghi', 'before writing');
  const what = changes
    .map((c) => {
      const at = c.lastIndexOf(':');
      return actionWord(c.slice(at + 1)) + ' ' + c.slice(0, at);
    })
    .join(', ');
  return t('trước khi ' + what, 'before ' + what);
}

function BackupsBody({ mcp, model, now }: { mcp: Mcp; model: Model; now: Date }) {
  const list = model.mcp.backups;
  const agents = AGENTS.filter((a) => list.some((b) => b.agent === a));
  return (
    <>
      <DialogTitle>{t('Bản đã lưu trước khi ghi', 'Backups made before writing')}</DialogTitle>
      <DialogLead>
        {t(
          'Giữ 10 bản gần nhất cho mỗi agent, trên máy này. Bản lưu không chứa khoá: giá trị của khoá được thay bằng <redacted>.',
          'Keeps the 10 latest for each agent, on this Mac. Backups hold no keys: each key’s value is replaced with <redacted>.',
        )}
      </DialogLead>
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
        <DialogLead>{t('Chưa có bản lưu nào: app chưa ghi danh sách MCP lần nào.', 'No backups yet: the app has not written an MCP list.')}</DialogLead>
      )}
      <DialogButtons className="mt-3">
        {agents.map((a) => (
          <Button key={a} large onClick={() => mcp.openBackups(a)}>
            {t('Mở thư mục ' + AGENT_NAME[a], 'Open the ' + AGENT_NAME[a] + ' folder')}
          </Button>
        ))}
        <Button large primary onClick={mcp.close}>
          {t('Đóng', 'Close')}
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
