// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cài đặt › Chung › Cập nhật Agent: Claude Code, the Codex CLI and the
// Antigravity CLI, a row each with its version, the newest, how it was
// installed and its button; then "Kiểm bản mới" and the daily check. The
// rows are laid out as the rest of Cài đặt (SetRow).

import { useState } from 'react';
import { Command } from '../components/blocks';
import { Glyph } from '../components/icons';
import { AgentIcon } from '../components/logos';
import { Button, Chip, CODE, FieldHint, More, Rich, Spinner, Toggle } from '../components/parts';
import { CLI_NAME, cliRow } from '../lib/cliTools';
import { clock } from '../lib/format';
import { t } from '../lib/i18n';
import type { CliTool, CliToolsView, CliToolView } from '../lib/model';
import { SetRow } from './SetRow';

export interface CliToolsProps {
  view: CliToolsView;
  onCheck(): void;
  onUpdate(tool: CliTool): void;
  onDaily(on: boolean): void;
}

/** A command and "Chép lệnh", which says "Đã chép" once it has. */
function CopyCommand({ command }: { command: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="flex items-start gap-2 [&>div]:mb-0 [&>div]:flex-1">
      <Command>{command}</Command>
      <Button
        small
        onClick={() =>
          navigator.clipboard.writeText(command).then(
            () => setCopied(true),
            () => setCopied(false),
          )
        }
      >
        <Glyph name={copied ? 'check' : 'copy'} bold />
        {copied ? t('Đã chép', 'Copied') : t('Chép lệnh', 'Copy command')}
      </Button>
    </div>
  );
}

function ToolRow({ v, onUpdate, onCheck, checking }: { v: CliToolView; onUpdate(tool: CliTool): void; onCheck(): void; checking: boolean }) {
  const row = cliRow(v);
  const name = CLI_NAME[v.tool];
  const a = row.action;
  return (
    <SetRow
      label={
        <span className="inline-flex items-center gap-1.5">
          <AgentIcon agent={v.tool} />
          <span>{name}</span>
        </span>
      }
      inline
      hint={
        <>
          <span className="tabular-nums">
            {row.status}{' '}
            {v.path ? (
              <More small about={name}>
                <p>
                  <span className={CODE}>{v.path}</span>
                </p>
                {v.command && !v.handsOff ? (
                  <p>
                    {t('Cập nhật chạy ', 'Update runs ')}
                    <span className={CODE}>{v.command}</span>
                    {v.safeWhileRunning
                      ? t(', an toàn khi đang có phiên: phiên cũ chạy tiếp bản cũ tới khi mở lại.', ', safe while sessions run: they keep the old version until opened again.')
                      : t('. Chỉ chạy khi không có phiên nào của công cụ đang mở.', '. It runs only when none of the tool’s sessions is open.')}
                  </p>
                ) : null}
                {v.last && !v.last.ok && v.last.log ? (
                  <p>
                    {t('Log: ', 'Log: ')}
                    <span className={CODE}>{v.last.log}</span>
                  </p>
                ) : null}
              </More>
            ) : null}
          </span>
          {row.note ? (
            <FieldHint tone={row.note.tone}>
              <Rich text={row.note.text} />
            </FieldHint>
          ) : null}
          {a.kind === 'copy' || a.kind === 'install' ? <CopyCommand command={a.command} /> : null}
        </>
      }
    >
      {a.kind === 'update' ? (
        <Button small primary onClick={() => onUpdate(v.tool)}>
          {t('Cập nhật', 'Update')}
        </Button>
      ) : a.kind === 'updating' ? (
        <Button small disabled>
          <Spinner />
          {t('Đang cập nhật…', 'Updating…')}
        </Button>
      ) : a.kind === 'recheck' ? (
        <Button small disabled={checking} onClick={onCheck}>
          {t('Kiểm lại', 'Check again')}
        </Button>
      ) : a.kind === 'latest' ? (
        <Chip chip={{ t: t('mới nhất', 'newest'), k: 'ok' }} />
      ) : a.kind === 'install' ? (
        <Chip chip={{ t: t('chưa cài', 'not installed'), k: '' }} />
      ) : a.kind === 'copy' ? (
        <Chip chip={{ t: t('chạy trong Terminal', 'run in Terminal'), k: 'warn' }} />
      ) : v.handsOff === 'bundled' ? (
        <Chip chip={{ t: t('do app khác cập nhật', 'updated by its app'), k: '' }} />
      ) : null}
    </SetRow>
  );
}

/** "hôm nay 14:20", or a date further back. */
function checkedText(at: number | null): string {
  if (!at) return t('Chưa kiểm lần nào.', 'Not checked yet.');
  return t('Kiểm lúc ' + clock(new Date(at * 1000).toISOString()) + '.', 'Checked at ' + clock(new Date(at * 1000).toISOString()) + '.');
}

export function CliToolsRows({ view, onCheck, onUpdate, onDaily }: CliToolsProps) {
  const updating = view.tools.some((v) => v.updating);
  const daily = t('Tự kiểm mỗi ngày', 'Check daily');
  const label = t('Kiểm bản mới', 'New versions');
  return (
    <>
      {view.tools.map((v) => (
        <ToolRow key={v.tool} v={v} onUpdate={onUpdate} onCheck={onCheck} checking={view.checking} />
      ))}
      <SetRow
        label={label}
        inline
        hint={
          <>
            <span>
              {checkedText(view.checkedAt)}{' '}
              <More small about={label}>
                <p>
                  {t(
                    'Hỏi nơi phát hành của từng cách cài: Anthropic, Homebrew, npm, GitHub, Google. Không gửi gì về máy này hay tài khoản.',
                    'Asks where each install method publishes: Anthropic, Homebrew, npm, GitHub, Google. Nothing about this Mac or the accounts is sent.',
                  )}
                </p>
              </More>
            </span>
            {view.offline ? <FieldHint tone="bad">{t('Lần kiểm trước không tới được máy chủ nào.', 'The last check reached no server.')}</FieldHint> : null}
          </>
        }
      >
        <Button small disabled={view.checking || updating} onClick={onCheck}>
          {view.checking ? (
            <>
              <Spinner />
              {t('Đang kiểm…', 'Checking…')}
            </>
          ) : (
            t('Kiểm ngay', 'Check now')
          )}
        </Button>
      </SetRow>
      <SetRow label={daily} inline hint={t('Chỉ kiểm và báo ở chuông, không tự cập nhật.', 'Only checks and tells you under the bell; never updates.')}>
        <Toggle on={view.daily} label={daily} onClick={() => onDaily(!view.daily)} />
      </SetRow>
    </>
  );
}
