// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cài đặt's card "Biến môi trường đè đăng nhập" (M14, I): each variable set
// in a shell file, Claude Code's settings or launchd, the agent it overrides
// and how, with "Mở file" at its line, "Gỡ" and "Bỏ qua"; a green line when
// there is none. "Xem danh sách" says what the app looks at. "Gỡ" shows the
// one line or key that goes, its value masked, before it takes it out, and
// its "Hoàn tác" puts it back. The app never reads a value and never unsets
// anything.

import { useRef, useState } from 'react';
import { Facts } from '../agents/McpDialogs';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { DiffFile, DiffLine, DiffView } from '../components/blocks';
import { SectionGlyph } from '../components/icons';
import { Button, Chip, LinkButton } from '../components/parts';
import type { Api } from '../lib/api';
import {
  agentChip,
  checkedFeedback,
  checkedLine,
  envFailedFeedback,
  ignoredFeedback,
  isJson,
  onlyWhen,
  openedFeedback,
  openFailedFeedback,
  isFile,
  placeLabel,
  refusalText,
  removedFeedback,
  removeFailedFeedback,
  restoredFeedback,
  restoreFailedFeedback,
  unreadNote,
  whereLine,
} from '../lib/env';
import { os, t } from '../lib/i18n';
import type { EnvPreview, Model } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';

/** The list dialog, and what the card's, banner's and feedback's buttons do. */
export function useEnv(api: Api, model: Model | null, showFeedback: (fb: FeedbackView | null) => void) {
  const [list, setList] = useState(false);
  const latest = useRef(model);
  latest.current = model;
  const found = (id: string) => latest.current?.env.found.find((f) => f.id === id);
  const check = async () => {
    try {
      await api.envCheck();
      if (latest.current) showFeedback(checkedFeedback(latest.current.env));
    } catch {
      showFeedback(envFailedFeedback());
    }
  };
  const open = async (id: string) => {
    const f = found(id);
    try {
      const editor = await api.envOpen(id);
      showFeedback(f ? openedFeedback(f, editor) : null);
    } catch {
      showFeedback(openFailedFeedback(f));
    }
  };
  const ignore = async (id: string) => {
    const f = found(id);
    try {
      await api.envIgnore(id);
      if (f) showFeedback(ignoredFeedback(f));
    } catch {
      showFeedback(envFailedFeedback());
    }
  };
  const back = async (id: string) => {
    showFeedback(null);
    await api.envUnignore(id).catch(() => showFeedback(envFailedFeedback()));
  };
  /** "Gỡ"'s preview while it is open, and whether the removal is under way. */
  const [preview, setPreview] = useState<EnvPreview | null>(null);
  const [working, setWorking] = useState(false);
  /** What each "Gỡ" took out, for its "Hoàn tác"'s words. */
  const removed = useRef(new Map<string, EnvPreview>());
  const askRemove = async (id: string) => {
    try {
      setPreview(await api.envRemovePreview(id));
    } catch (e) {
      showFeedback(removeFailedFeedback(undefined, String(e)));
    }
  };
  const remove = async () => {
    const p = preview;
    if (!p || p.refusal || working) return;
    setWorking(true);
    try {
      await api.envRemove(p.id);
      removed.current.set(p.id, p);
      showFeedback(removedFeedback(p));
    } catch (e) {
      showFeedback(removeFailedFeedback(p, String(e)));
    } finally {
      setWorking(false);
      setPreview(null);
    }
  };
  const restore = async (id: string) => {
    const p = removed.current.get(id);
    showFeedback(null);
    try {
      await api.envRestore(id);
      removed.current.delete(id);
      if (p) showFeedback(restoredFeedback(p));
    } catch (e) {
      showFeedback(restoreFailedFeedback(String(e)));
    }
  };
  /** Its own actions; false for the others. `env:settings` is the window's. */
  const act = (a: Action): boolean => {
    if (a === 'env:check') void check();
    else if (a === 'env:list') setList(true);
    else if (a.startsWith('env:open:')) void open(a.slice('env:open:'.length));
    else if (a.startsWith('env:ignore:')) void ignore(a.slice('env:ignore:'.length));
    else if (a.startsWith('env:back:')) void back(a.slice('env:back:'.length));
    else if (a.startsWith('env:remove:')) void askRemove(a.slice('env:remove:'.length));
    else if (a.startsWith('env:restore:')) void restore(a.slice('env:restore:'.length));
    else return false;
    return true;
  };
  return {
    list,
    preview,
    working,
    act,
    remove,
    close: () => {
      setList(false);
      if (!working) setPreview(null);
    },
  };
}

export type EnvState = ReturnType<typeof useEnv>;

/** `id`: in Cài đặt, the card is a region named by its title (in the Dịch vụ tab). */
export function EnvCard({ model, onAction, id }: { model: Model; onAction(act: Action): void; id?: string }) {
  const env = model.env;
  return (
    <Panel className="env-card" id={id} role={id ? 'region' : undefined} aria-labelledby={id ? id + '-t' : undefined}>
      <CardTitle id={id ? id + '-t' : undefined}>
        <SectionGlyph section="env" />
        {t('Biến môi trường đè đăng nhập', 'Environment variables that override sign-in')}
      </CardTitle>
      {env.found.length ? (
        <div role="list" aria-label={t('Biến đè đăng nhập', 'Variables that override sign-in')}>
          {env.found.map((f) => (
            <div
              key={f.id}
              className="grid grid-cols-[minmax(0,1fr)_auto] gap-x-3 gap-y-1 border-t-[.5px] border-card-line py-[9px] text-sm first:border-t-0 first:pt-0.5"
              role="listitem"
              aria-label={f.name + ', ' + whereLine(f)}
            >
              <span>
                <b className="[font:var(--fw-semi)_12px_var(--mono)] [overflow-wrap:anywhere]">{f.name}</b> <Chip chip={agentChip(f)} />
                <small className="block text-xs text-ink2">{whereLine(f)}</small>
                <small className="block text-xs text-ink2">{f.effect}</small>
              </span>
              <span className="flex items-start gap-1.5">
                {isFile(f.place) ? (
                  <Button label={t('Mở file: ', 'Open file: ') + whereLine(f)} onClick={() => onAction(`env:open:${f.id}`)}>
                    {t('Mở file', 'Open file')}
                  </Button>
                ) : null}
                {isFile(f.place) ? (
                  <Button label={t('Gỡ ' + f.name + ' ở ' + whereLine(f), 'Remove ' + f.name + ' from ' + whereLine(f))} onClick={() => onAction(`env:remove:${f.id}`)}>
                    {t('Gỡ', 'Remove')}
                  </Button>
                ) : null}
                <Button label={t('Bỏ qua ' + f.name + ' ở ' + whereLine(f), 'Ignore ' + f.name + ' in ' + whereLine(f))} onClick={() => onAction(`env:ignore:${f.id}`)}>
                  {t('Bỏ qua', 'Ignore')}
                </Button>
              </span>
            </div>
          ))}
        </div>
      ) : env.checkedAt ? (
        <CardText className="mt-0.5 flex items-center gap-1.5 text-ink">
          <Chip chip={{ t: t('không có', 'none'), k: 'ok' }} /> {t('Không có biến nào đè đăng nhập.', 'No variable overrides a sign-in.')}
        </CardText>
      ) : null}
      {unreadNote(env) ? <CardText className="mt-1 mb-0 text-warn-ink">{unreadNote(env)}</CardText> : null}
      <CardText className="mt-1.5 mb-0">
        {checkedLine(env)}{' '}
        <LinkButton small disabled={env.checking} onClick={() => onAction('env:check')}>
          {t('Kiểm tra lại', 'Check again')}
        </LinkButton>
        {' · '}
        <LinkButton small onClick={() => onAction('env:list')}>
          {t('Xem danh sách', 'See the list')}
        </LinkButton>
      </CardText>
    </Panel>
  );
}

/** "Xem danh sách": the names and the places, and what the app does with them. */
export function EnvListBody({ model, env }: { model: Model; env: EnvState }) {
  return (
    <>
      <DialogTitle>{t('Ứng dụng kiểm tra những gì', 'What the app checks')}</DialogTitle>
      <DialogSection>{t('Biến', 'Variables')}</DialogSection>
      <p className="mt-0 mb-1.5 block rounded-sm bg-fill px-1 font-mono text-xs leading-[1.9] text-ink [overflow-wrap:anywhere]">{model.env.vars.join(' · ')}</p>
      <DialogSection>{t('Chỗ', 'Places')}</DialogSection>
      <Facts items={model.env.places.map((p) => ['', p === 'launchctl' ? t('launchctl (app mở từ Dock)', 'launchctl (apps opened from the Dock)') : p])} />
      <p className="mt-1.5 mb-0 text-xs text-ink2">
        {os(
          t(
            'App tìm tên biến ở đầu dòng trong từng file shell, trong mục env của settings của Claude Code, và hỏi launchctl biến có đặt hay không. App không đọc giá trị, không chạy file shell và không tự bỏ biến; app chỉ sửa file khi bạn bấm Gỡ ở một dòng.',
            'The app looks for each name at the start of a line in each shell file and in the env of Claude Code’s settings, and asks launchctl whether it is set. It never reads a value, runs a shell file or unsets a variable by itself; it edits a file only when you press Remove on a row.',
          ),
          t(
            'App tìm tên biến ở đầu dòng trong $PROFILE của PowerShell (kể cả dòng trong một hàm, chỉ có tác dụng khi gọi hàm đó), trong mục env của settings của Claude Code, và hỏi Windows biến có đặt trong biến người dùng hay biến hệ thống không. App không đọc giá trị, không chạy file shell và không tự bỏ biến; app chỉ sửa file khi bạn bấm Gỡ ở một dòng.',
            'The app looks for each name at the start of a line in PowerShell’s $PROFILE (a line inside a function counts only when that function is called) and in the env of Claude Code’s settings, and asks Windows whether it is set as a user or system variable. It never reads a value, runs a shell file or unsets a variable by itself; it edits a file only when you press Remove on a row.',
          ),
        )}
      </p>
      <DialogButtons>
        <Button large primary onClick={env.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </>
  );
}

/** "Gỡ": the one line or key that goes, its value masked, and what stays; or why the app will not. */
export function EnvRemoveBody({ env, onAction }: { env: EnvState; onAction(act: Action): void }) {
  const p = env.preview;
  if (!p) return null;
  const json = isJson(p.place);
  const only = onlyWhen(p);
  return (
    <>
      <DialogTitle>{t('Gỡ ' + p.name + '?', 'Remove ' + p.name + '?')}</DialogTitle>
      <DialogLead>
        {p.refusal
          ? t('App không gỡ được dòng này. ', 'The app will not remove this line. ') + refusalText(p.refusal) + t(' Mở file để sửa bằng tay.', ' Open the file to edit it yourself.')
          : json
            ? t(
                'App chỉ xoá đúng khoá này trong mục env; file vẫn là JSON hợp lệ, phần còn lại (MCP, hooks, quyền) giữ nguyên.',
                'The app removes just this key from env; the file stays valid JSON and the rest (MCP, hooks, permissions) stays as it is.',
              )
            : t(
                'App chỉ xoá đúng dòng này; mọi dòng khác, mã hoá và kiểu xuống dòng của file giữ nguyên.',
                'The app removes just this line; every other line, the encoding and the line endings stay as they are.',
              )}
      </DialogLead>
      <DialogSection>{json ? t('Khoá sẽ xoá', 'The key that goes') : t('Dòng sẽ xoá', 'The line that goes')}</DialogSection>
      <DiffView head={<DiffFile>{placeLabel(p.place)}</DiffFile>} right={t('dòng ', 'line ') + p.line}>
        <DiffLine op="del" n={p.line}>
          {p.shown}
        </DiffLine>
      </DiffView>
      {only ? (
        <p className="mt-0 mb-1.5 text-xs text-ink2">
          {t('Dòng này nằm trong một hàm PowerShell nên ', 'This line is inside a PowerShell function, so it applies ') + only + '.'}
        </p>
      ) : null}
      {p.refusal ? null : (
        <p className="mt-0 mb-1.5 text-xs text-ink2">
          {t(
            'Giá trị đã che. Phần bị xoá được giữ trong thư mục riêng của app (chỉ tài khoản của bạn mở được) để Hoàn tác; không có bản sao nào cạnh file.',
            'The value is hidden. What goes is kept in the app’s own folder (only your account can open it) for Undo; no copy is left beside the file.',
          )}
        </p>
      )}
      <DialogButtons>
        {p.refusal ? (
          <>
            <Button large onClick={env.close}>
              {t('Đóng', 'Close')}
            </Button>
            <Button
              large
              primary
              onClick={() => {
                env.close();
                onAction(`env:open:${p.id}`);
              }}
            >
              {t('Mở file', 'Open file')}
            </Button>
          </>
        ) : (
          <>
            <Button large disabled={env.working} onClick={env.close}>
              {t('Huỷ', 'Cancel')}
            </Button>
            <Button large danger disabled={env.working} onClick={() => void env.remove()}>
              {t('Gỡ', 'Remove')}
            </Button>
          </>
        )}
      </DialogButtons>
    </>
  );
}
