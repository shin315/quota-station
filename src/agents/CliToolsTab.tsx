// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Agent › Cập nhật, the Cập nhật Agent page: Claude Code, the Codex CLI and
// the Antigravity CLI. One line at the top says whether they are the newest,
// with "Kiểm tra ngay"; then a row per tool, its name and one line (its
// version and the newest, a new version, checking, or could not check) with
// its button, the rest behind "Chi tiết"; then "Tự động kiểm tra". The rows
// are laid out as Cài đặt's (SetRow). It was Cài đặt › Chung's Cập nhật
// Agent card until 27/10/2026; since then each row is a name and at most
// one short line (the captain's rule of 27/10/2026).

import { useState } from 'react';
import { Card as Panel, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { Command } from '../components/blocks';
import { Glyph, type GlyphName, SectionGlyph } from '../components/icons';
import { AgentIcon } from '../components/logos';
import { TabHead, TabNote, TabSub, TabTitle } from '../components/page';
import { Button, CODE, Dot, More, Rich, Spinner, Toggle } from '../components/parts';
import { CLI_NAME, cliRow, cliSummary, type CliRow, type CliSummary } from '../lib/cliTools';
import { t } from '../lib/i18n';
import type { CliTool, CliToolsView, CliToolView, Model } from '../lib/model';
import { SetRow } from '../settings/SetRow';

export interface CliToolsTabProps {
  model: Model;
  now: Date;
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

/** A row's one line in the colour of what it says. */
const LINE_INK: Record<CliRow['line']['tone'], string> = { ok: 'text-ok-ink', info: 'text-info-ink', warn: 'text-warn-ink', bad: 'text-bad-ink', off: 'text-ink3', '': '' };

function ToolRow({ v, view, now, onUpdate, onCheck }: { v: CliToolView; view: CliToolsView; now: Date; onUpdate(tool: CliTool): void; onCheck(): void }) {
  const row = cliRow(v, view, now);
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
        <span className="tabular-nums">
          <span className={LINE_INK[row.line.tone]} data-tone={row.line.tone || undefined} data-check={row.check ?? undefined}>
            {row.line.text}
          </span>{' '}
          <More small about={name}>
            {v.path ? (
              <p>
                <span className={CODE}>{v.path}</span>
              </p>
            ) : null}
            {row.more.map((line) => (
              <p key={line}>
                <Rich text={line} />
              </p>
            ))}
            {v.command && !v.handsOff && v.version ? (
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
            {row.command ? <CopyCommand command={row.command} /> : null}
          </More>
        </span>
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
        <Button small disabled={view.checking} onClick={onCheck}>
          {t('Kiểm tra lại', 'Check again')}
        </Button>
      ) : null}
    </SetRow>
  );
}

const SUMMARY_GLYPH: Record<CliSummary['tone'], GlyphName> = {
  ok: 'check-circle',
  info: 'arrow-circle-up',
  warn: 'warning-circle',
  bad: 'warning-circle',
  stale: 'arrows-clockwise',
  off: 'clock-counter-clockwise',
};

const SUMMARY_INK: Record<CliSummary['tone'], string> = {
  ok: 'text-ok-ink',
  info: 'text-info-ink',
  warn: 'text-warn-ink',
  bad: 'text-bad-ink',
  stale: 'text-ink2',
  off: 'text-ink2',
};

/** The line at the top: whether every tool is the newest, when that was checked, and "Kiểm tra ngay". */
function Summary({ view, summary, onCheck }: { view: CliToolsView; summary: CliSummary; onCheck(): void }) {
  const updating = view.tools.some((v) => v.updating);
  return (
    <Panel>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <div role="status" aria-label={t('Tóm tắt', 'Summary')} data-tone={summary.tone} className="flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-0.5 text-base">
          {summary.tone === 'stale' ? <Spinner /> : <Glyph name={SUMMARY_GLYPH[summary.tone]} bold className={cn('size-[18px]', SUMMARY_INK[summary.tone])} />}
          <b className="font-semi">{summary.text}</b>
          {summary.when ? <span className="text-sm text-ink2 tabular-nums">· {summary.when}</span> : null}
        </div>
        <Button small disabled={view.checking || updating} onClick={onCheck}>
          {view.checking ? (
            <>
              <Spinner />
              {t('Đang kiểm tra…', 'Checking…')}
            </>
          ) : (
            t('Kiểm tra ngay', 'Check now')
          )}
        </Button>
      </div>
    </Panel>
  );
}

/** "Tự động kiểm tra": once a day, where the check asks behind "Chi tiết". */
function DailyRow({ view, onDaily }: { view: CliToolsView; onDaily(on: boolean): void }) {
  const daily = t('Tự động kiểm tra', 'Check automatically');
  return (
    <SetRow
      label={daily}
      inline
      hint={
        <span>
          {view.daily ? t('Mỗi ngày, chỉ báo ở chuông.', 'Once a day; only tells you under the bell.') : t('Tắt: chỉ kiểm tra khi bạn bấm.', 'Off: checks only when you press.')}{' '}
          <More small about={daily}>
            <p>
              {t(
                'Mỗi ngày một lần, và khi mở trang này nếu lần kiểm tra trước đã quá một ngày. Không bao giờ tự cập nhật.',
                'Once a day, and when this page opens if the last check is over a day old. It never updates anything itself.',
              )}
            </p>
            <p>
              {t(
                'Hỏi nơi phát hành của từng cách cài: Anthropic, Homebrew, npm, GitHub, Google. Không gửi gì về máy này hay tài khoản.',
                'Asks where each install method publishes: Anthropic, Homebrew, npm, GitHub, Google. Nothing about this Mac or the accounts is sent.',
              )}
            </p>
          </More>
        </span>
      }
    >
      <Toggle on={view.daily} label={daily} onClick={() => onDaily(!view.daily)} />
    </SetRow>
  );
}

export function CliToolsTab({ model, now, onCheck, onUpdate, onDaily }: CliToolsTabProps) {
  const view = model.cliTools;
  const summary = view ? cliSummary(view, now) : null;
  const title = t('Cập nhật Agent', 'Agent updates');
  return (
    <>
      <TabHead>
        <TabTitle>{title}</TabTitle>
        <TabSub>
          <Dot tone={summary?.tone === 'ok' ? 'ok' : summary?.tone === 'info' ? 'info' : summary?.tone === 'warn' || summary?.tone === 'bad' ? 'warn' : 'off'} />
          {model.machine}
        </TabSub>
      </TabHead>
      <TabNote>
        {t('Chỉ kiểm tra và báo ở chuông · Cập nhật khi bạn bấm. ', 'Only checks and tells you under the bell · updates when you press. ')}
        <More about={title}>
          <p>
            {t(
              'Claude Code, Codex CLI và Antigravity CLI trên máy này. App đọc phiên bản bằng --version và hỏi nơi phát hành của từng cách cài.',
              'Claude Code, the Codex CLI and the Antigravity CLI on this Mac. The app reads each version with --version and asks where each install method publishes.',
            )}
          </p>
          <p>
            {t(
              'Cập nhật chạy lệnh của chính công cụ. Claude Code và Antigravity CLI cập nhật được khi đang có phiên; Codex CLI chờ tới khi các phiên của nó đóng.',
              'An update runs the tool’s own command. Claude Code and the Antigravity CLI update while sessions run; the Codex CLI waits until its sessions are closed.',
            )}
          </p>
        </More>
      </TabNote>
      {view && summary ? (
        <>
          <Summary view={view} summary={summary} onCheck={onCheck} />
          <Panel role="region" aria-labelledby="qs-cli-tools-t">
            <CardTitle id="qs-cli-tools-t">
              <SectionGlyph section="cli" />
              {t('Công cụ', 'Tools')}
            </CardTitle>
            <div className="grid">
              {view.tools.map((v) => (
                <ToolRow key={v.tool} v={v} view={view} now={now} onUpdate={onUpdate} onCheck={onCheck} />
              ))}
            </div>
          </Panel>
          <Panel role="region" aria-labelledby="qs-cli-check-t">
            <CardTitle id="qs-cli-check-t">
              <SectionGlyph section="updates" />
              {t('Kiểm tra bản mới', 'Check for new versions')}
            </CardTitle>
            <div className="grid">
              <DailyRow view={view} onDaily={onDaily} />
            </div>
          </Panel>
        </>
      ) : null}
    </>
  );
}
