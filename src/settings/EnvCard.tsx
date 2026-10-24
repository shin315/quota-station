// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cài đặt's card "Biến môi trường đè đăng nhập" (M14, I): each variable set
// in a shell file or launchd, the agent it overrides and how, with "Mở file"
// at its line and "Bỏ qua"; a green line when there is none. "Xem danh
// sách" says what the app looks at. The app never reads a value, never
// edits a shell file and never unsets anything.

import { useRef, useState } from 'react';
import { Facts } from '../agents/McpDialogs';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { DialogButtons, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { Icon } from '../components/icons';
import { Button, Chip, LinkButton } from '../components/parts';
import type { Api } from '../lib/api';
import {
  agentChip,
  checkedFeedback,
  checkedLine,
  envFailedFeedback,
  ignoredFeedback,
  openedFeedback,
  openFailedFeedback,
  unreadNote,
  whereLine,
} from '../lib/env';
import { t } from '../lib/i18n';
import type { Model } from '../lib/model';
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
  /** Its own actions; false for the others. `env:settings` is the window's. */
  const act = (a: Action): boolean => {
    if (a === 'env:check') void check();
    else if (a === 'env:list') setList(true);
    else if (a.startsWith('env:open:')) void open(a.slice('env:open:'.length));
    else if (a.startsWith('env:ignore:')) void ignore(a.slice('env:ignore:'.length));
    else if (a.startsWith('env:back:')) void back(a.slice('env:back:'.length));
    else return false;
    return true;
  };
  return { list, act, close: () => setList(false) };
}

export type EnvState = ReturnType<typeof useEnv>;

export function EnvCard({ model, onAction }: { model: Model; onAction(act: Action): void }) {
  const env = model.env;
  return (
    <Panel className="env-card">
      <CardTitle>
        <Icon name="terminal" />
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
                {f.place !== 'launchctl' ? (
                  <Button label={t('Mở file: ', 'Open file: ') + whereLine(f)} onClick={() => onAction(`env:open:${f.id}`)}>
                    {t('Mở file', 'Open file')}
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
          {t('Kiểm lại', 'Check again')}
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
      <DialogTitle>{t('App kiểm những gì', 'What the app checks')}</DialogTitle>
      <DialogSection>{t('Biến', 'Variables')}</DialogSection>
      <p className="mt-0 mb-1.5 block rounded-sm bg-fill px-1 font-mono text-xs leading-[1.9] text-ink [overflow-wrap:anywhere]">{model.env.vars.join(' · ')}</p>
      <DialogSection>{t('Chỗ', 'Places')}</DialogSection>
      <Facts items={model.env.places.map((p) => ['', p === 'launchctl' ? t('launchctl (app mở từ Dock)', 'launchctl (apps opened from the Dock)') : p])} />
      <p className="mt-1.5 mb-0 text-xs text-ink2">
        {t(
          'App tìm tên biến ở đầu dòng trong từng file và hỏi launchctl biến có đặt hay không. App không đọc giá trị, không chạy file shell, không sửa file và không tự bỏ biến.',
          'The app looks for each name at the start of a line in each file, and asks launchctl whether it is set. It never reads a value, runs a shell file, edits a file or unsets a variable.',
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
