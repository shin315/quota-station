// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Sao lưu tab of the main window (M10), as the approved prototype's
// `bk` screen after round 2: the newest pack, every Mac's packs in iCloud
// Drive, the packs made on their own, and the keys the app keeps (name and
// Bitwarden item only: replace or delete, never show). No folder path, no
// "Keychain", nothing technical.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { PACK_META, PACK_ROW, ROW_ACTIONS, SETTING_ROW } from '../components/blocks';
import { COUNT, Empty, EmptyIcon, GRID, STACK, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, Dot, Feedback, LinkButton, Toggle } from '../components/parts';
import { cn } from '@/lib/utils';
import { clock } from '../lib/format';
import { keysLine, macOf, packLine, packNote, packWhen, packWhenLower, autoErrorText, usable } from '../lib/backup';
import { t } from '../lib/i18n';
import type { Model, PackInfo } from '../lib/model';
import type { Action, BannerView, FeedbackView, PopoverView } from '../lib/view';
import { ARCHIVE_BIG } from '../window/icons';
import type { Backup } from './useBackup';

function banners(model: Model, now: Date): BannerView[] {
  const v = model.backup;
  const out: BannerView[] = [];
  if (!v.icloud)
    out.push({
      tone: 'bad',
      title: t('iCloud Drive đang tắt trên máy này', 'iCloud Drive is off on this Mac'),
      body: t('Bật iCloud Drive để sao lưu, và để lấy gói của Mac khác.', 'Turn on iCloud Drive to back up, and to get packs from other Macs.'),
      actions: [{ label: t('Kiểm lại', 'Check again'), act: 'backup:reread' }],
    });
  const cloud = (v.packs ?? []).filter((p) => p.state === 'cloud');
  if (cloud.length)
    out.push({
      tone: 'warn',
      title:
        cloud.length === 1
          ? t('Gói ' + packWhenLower(cloud[0].at, now) + ' chưa tải về máy', 'Pack from ' + packWhenLower(cloud[0].at, now) + ' not downloaded yet')
          : t(cloud.length + ' gói chưa tải về máy', cloud.length + ' packs not downloaded yet'),
      body: t('iCloud Drive mới có tên file. Mở thư mục trong Finder để tải, hoặc chờ.', 'iCloud Drive only has the file name so far. Open the folder in Finder to download it, or wait.'),
      actions: [
        { label: t('Mở trong Finder', 'Open in Finder'), act: 'backup:open' },
        { label: t('Kiểm lại', 'Check again'), act: 'backup:reread' },
      ],
    });
  const auto = autoErrorText(v);
  if (auto) out.push({ tone: 'warn', title: auto, body: t('Bấm Sao lưu bây giờ để thử lại.', 'Press Back up now to try again.'), actions: [] });
  if (v.restore)
    out.push({
      tone: 'info',
      title: t('Đã khôi phục lúc ', 'Restored at ') + clock(new Date(v.restore.at * 1000).toISOString()),
      body: t('Từ gói của ' + v.restore.mac + '. Hoàn tác được cả lần khôi phục.', 'From the pack of ' + v.restore.mac + '. The whole restore can be undone.'),
      actions: [{ label: t('Hoàn tác…', 'Undo…'), act: 'backup:undo' }],
    });
  return out;
}

function PackRow({ p, model, now, backup }: { p: PackInfo; model: Model; now: Date; backup: Backup }) {
  const note = packNote(p);
  const open = usable(p);
  const when = packWhen(p.at, now);
  return (
    <div className={PACK_ROW}>
      <span>
        {when}
        <small className={PACK_META}>{macOf(p, model.machine)}</small>
      </span>
      <span className={PACK_META}>
        {p.state === 'ok' ? packLine(p) : null}
        {note ? <Chip chip={{ t: note, k: p.state === 'broken' ? 'bad' : 'warn' }} /> : null}
      </span>
      <span className="flex gap-2.5">
        <LinkButton small aria-label={t('Xem gói ', 'View pack ') + when} disabled={!open} onClick={() => backup.openView(p.id)}>
          {t('Xem', 'View')}
        </LinkButton>
        <LinkButton small aria-label={t('Khôi phục gói ', 'Restore pack ') + when} disabled={!open || backup.working} onClick={() => backup.openRestore(p.id)}>
          {t('Khôi phục…', 'Restore…')}
        </LinkButton>
      </span>
    </div>
  );
}

export interface BackupTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  backup: Backup;
  onAction(act: Action): void;
}

export function BackupTab({ model, feedback, now, backup, onAction }: BackupTabProps) {
  const v = model.backup;
  const busy = backup.working;
  const packs = v.packs;
  const top = (
    <TabHead data-tauri-drag-region>
      <TabTitle>{t('Sao lưu', 'Backup')}</TabTitle>
      <TabSub>
        <Dot tone={v.icloud ? 'ok' : 'stale'} />
        {model.machine} · iCloud Drive
      </TabSub>
      <Button primary disabled={busy || !v.icloud} onClick={backup.backupNow}>
        {v.busy === 'packing' ? t('Đang sao lưu…', 'Backing up…') : t('Sao lưu bây giờ', 'Back up now')}
      </Button>
      <Button disabled={busy || !packs?.some(usable)} onClick={() => backup.openRestore()}>
        {t('Khôi phục…', 'Restore…')}
      </Button>
    </TabHead>
  );
  if (!packs)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <b>{t('Đang tìm các gói sao lưu…', 'Looking for backup packs…')}</b>
            {t('Trong iCloud Drive.', 'In iCloud Drive.')}
          </Empty>
        </Panel>
      </>
    );
  const list = banners(model, now);
  const last = packs.find((p) => p.mine && p.state === 'ok') ?? packs.find((p) => p.state === 'ok') ?? null;
  const keys = v.keys ?? [];
  return (
    <>
      {top}
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {list.map((b) => (
        <Banner key={b.title} banner={b} onAction={onAction} />
      ))}
      {!packs.length ? (
        <Panel>
          <Empty>
            <EmptyIcon>{ARCHIVE_BIG}</EmptyIcon>
            <b>{t('Chưa có gói sao lưu nào', 'No backup packs yet')}</b>
            <CardText className="mt-2 mb-0">
              {t('Gói gồm các MCP của bạn, không có khoá. Lưu trong iCloud Drive để máy khác lấy được.', 'A pack holds your MCP servers, without keys. It is kept in iCloud Drive so other Macs can get it.')}
            </CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary disabled={busy || !v.icloud} onClick={backup.backupNow}>
                {t('Sao lưu lần đầu', 'Make the first backup')}
              </Button>
            </div>
          </Empty>
        </Panel>
      ) : (
        <div className={cn(GRID, 'max-[860px]:grid-cols-[minmax(0,1fr)]')}>
          <div className={STACK}>
            {last ? (
              <Panel>
                <CardTitle>{t('Gói gần nhất', 'Latest pack')}</CardTitle>
                <div className="text-h2 font-bold tracking-[-.02em] tabular-nums">{packWhen(last.at, now)}</div>
                <p className="mt-0.5 mb-1 text-sm text-ink2">{macOf(last, model.machine) + ' · ' + packLine(last)}</p>
                <CardText className="m-0">
                  {keysLine(last.keys)}{' '}
                  <LinkButton small onClick={() => backup.openView(last.id)}>
                    {t('Xem trong gói', 'See inside')}
                  </LinkButton>
                </CardText>
              </Panel>
            ) : null}
            <Panel>
              <CardTitle>
                {t('Các gói', 'Packs')} <span className={COUNT}>{packs.length}</span>
              </CardTitle>
              {packs.map((p) => (
                <PackRow key={p.id} p={p} model={model} now={now} backup={backup} />
              ))}
            </Panel>
          </div>
          <div className={STACK}>
            <Panel>
              <CardTitle>{t('Tự sao lưu', 'Automatic backup')}</CardTitle>
              <div className={SETTING_ROW}>
                <span>{t('Sau mỗi thay đổi trong app', 'After every change in the app')}</span>
                <Toggle on={v.auto.afterChange} label={t('Sau mỗi thay đổi trong app', 'After every change in the app')} onClick={() => backup.setAuto(!v.auto.afterChange, v.auto.daily)} />
              </div>
              <div className={SETTING_ROW}>
                <span>{t('Mỗi ngày, nếu agent có gì đổi', 'Daily, if an agent changed')}</span>
                <Toggle on={v.auto.daily} label={t('Mỗi ngày, nếu agent có gì đổi', 'Daily, if an agent changed')} onClick={() => backup.setAuto(v.auto.afterChange, !v.auto.daily)} />
              </div>
              <CardText className="mt-2 mb-0">{t('Giữ 10 gói gần nhất của mỗi Mac.', 'Keeps the 10 latest packs per Mac.')}</CardText>
            </Panel>
            <Panel>
              <CardTitle>
                {t('Khoá đã lưu', 'Saved keys')} <span className={COUNT}>{keys.length}</span>
              </CardTitle>
              {keys.map((k) => (
                <div key={k.mcp + '|' + k.slot} className={SETTING_ROW}>
                  <div>
                    <b>{k.mcp}</b>
                    <small>{k.bitwarden ? t('Lưu ở: ', 'Stored in: ') + k.bitwarden : k.label}</small>
                  </div>
                  <div className={ROW_ACTIONS}>
                    <LinkButton small aria-label={t('Thay khoá ', 'Replace key ') + k.mcp} disabled={busy} onClick={() => backup.setDialog({ kind: 'key', key: k })}>
                      {t('Thay', 'Replace')}
                    </LinkButton>
                    <LinkButton small aria-label={t('Xoá khoá ', 'Delete key ') + k.mcp} disabled={busy} onClick={() => backup.setDialog({ kind: 'delKey', key: k })}>
                      {t('Xoá', 'Delete')}
                    </LinkButton>
                  </div>
                </div>
              ))}
              {!keys.length ? <CardText className="mt-2 mb-0">{t('Chưa có khoá nào app giữ trên máy này.', 'The app keeps no keys on this Mac yet.')}</CardText> : null}
              <CardText className="mt-2 mb-0">{t('Khoá chỉ nằm trên máy này, không vào gói.', 'Keys stay on this Mac and never go into a pack.')}</CardText>
            </Panel>
          </div>
        </div>
      )}
    </>
  );
}
