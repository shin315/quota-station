// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Sao lưu tab of the main window (M10), laid out as the redraw the
// owner approved on 25/10/2026 (backup-A): the newest pack, the packs made on
// their own and the keys the app keeps (name and Bitwarden item only: replace
// or delete, never show) as three cards side by side, then every Mac's packs
// in iCloud Drive as one table, a day at a time. No folder path, no
// "Keychain", nothing technical.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { ROW_ACTIONS, SETTING_ROW } from '../components/blocks';
import { COUNT, Empty, EmptyIcon, GRID3, GROUP_HEAD, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, Dot, Feedback, LinkButton, Toggle } from '../components/parts';
import { clock } from '../lib/format';
import { keysLine, macOf, packClock, packDay, packLine, packNote, packWhen, packWhenLower, autoErrorText, usable } from '../lib/backup';
import { cloudDrive, fileManager, mac, t } from '../lib/i18n';
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
      title: mac(t(cloudDrive() + ' đang tắt trên máy này', cloudDrive() + ' is off on this Mac')),
      body: mac(t('Bật ' + cloudDrive() + ' để sao lưu, và để lấy gói của Mac khác.', 'Turn on ' + cloudDrive() + ' to back up, and to get packs from other Macs.')),
      actions: [{ label: t('Kiểm tra lại', 'Check again'), act: 'backup:reread' }],
    });
  const cloud = (v.packs ?? []).filter((p) => p.state === 'cloud');
  if (cloud.length)
    out.push({
      tone: 'warn',
      title:
        cloud.length === 1
          ? t('Gói ' + packWhenLower(cloud[0].at, now) + ' chưa tải về máy', 'Pack from ' + packWhenLower(cloud[0].at, now) + ' not downloaded yet')
          : t(cloud.length + ' gói chưa tải về máy', cloud.length + ' packs not downloaded yet'),
      body: t(cloudDrive() + ' mới có tên file. Mở thư mục trong ' + fileManager() + ' để tải, hoặc chờ.', cloudDrive() + ' only has the file name so far. Open the folder in ' + fileManager() + ' to download it, or wait.'),
      actions: [
        { label: t('Mở trong ' + fileManager(), 'Open in ' + fileManager()), act: 'backup:open' },
        { label: t('Kiểm tra lại', 'Check again'), act: 'backup:reread' },
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

/**
 * The packs' table: Giờ, Mac, Nội dung, Trạng thái and the buttons, columns
 * every row shares (subgrid), so the buttons' column is as wide as the
 * widest row's. A day's packs under its name.
 */
const PACKS = 'grid min-w-0 grid-cols-[minmax(44px,auto)_minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.25fr)_auto] gap-x-3 text-sm';
const PACK_TR =
  'col-span-full grid grid-cols-subgrid items-center border-t-[.5px] border-card-line px-1.5 py-[7px] tabular-nums [[data-day]+&]:border-t-0';
const PACK_TH = 'col-span-full grid grid-cols-subgrid px-1.5 pb-1 text-xs whitespace-nowrap text-ink3';

function PackRow({ p, model, now, backup }: { p: PackInfo; model: Model; now: Date; backup: Backup }) {
  const note = packNote(p);
  const open = usable(p);
  const when = packWhen(p.at, now);
  return (
    <div className={PACK_TR} role="listitem" aria-label={when + ' · ' + macOf(p, model.machine)}>
      <span>{packClock(p.at) || '—'}</span>
      <span className="truncate">{macOf(p, model.machine)}</span>
      <span className={p.state === 'ok' ? undefined : 'text-ink3'}>{p.state === 'ok' ? packLine(p) : '—'}</span>
      <span className="min-w-0">{note ? <Chip chip={{ t: note, k: p.state === 'broken' ? 'bad' : 'warn' }} /> : null}</span>
      <span className="flex justify-end gap-2.5">
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

/** Every Mac's packs, newest first, a day at a time. */
function PackTable({ packs, model, now, backup }: { packs: PackInfo[]; model: Model; now: Date; backup: Backup }) {
  const days: { day: string; packs: PackInfo[] }[] = [];
  for (const p of packs) {
    const day = packDay(p.at, now);
    const last = days[days.length - 1];
    if (last && last.day === day) last.packs.push(p);
    else days.push({ day, packs: [p] });
  }
  return (
    <div className="overflow-x-auto overscroll-x-contain">
      <div className={PACKS}>
        <div className={PACK_TH} aria-hidden="true">
          <span>{t('Giờ', 'Time')}</span>
          <span>{mac('Mac')}</span>
          <span>{t('Nội dung', 'Contents')}</span>
          <span>{t('Trạng thái', 'Status')}</span>
          <span />
        </div>
        {days.map((d) => (
          <div key={d.day} className="contents" role="list" aria-label={d.day}>
            <div className={cn(GROUP_HEAD, 'px-1.5')} data-day aria-hidden="true">
              {d.day}
            </div>
            {d.packs.map((p) => (
              <PackRow key={p.id} p={p} model={model} now={now} backup={backup} />
            ))}
          </div>
        ))}
      </div>
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
        {model.machine} · {cloudDrive()}
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
            {t('Trong ' + cloudDrive() + '.', 'In ' + cloudDrive() + '.')}
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
              {mac(t('Gói gồm các MCP của bạn, không có khoá. Lưu trong ' + cloudDrive() + ' để máy khác lấy được.', 'A pack holds your MCP servers, without keys. It is kept in ' + cloudDrive() + ' so other Macs can get it.'))}
            </CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary disabled={busy || !v.icloud} onClick={backup.backupNow}>
                {t('Sao lưu lần đầu', 'Make the first backup')}
              </Button>
            </div>
          </Empty>
        </Panel>
      ) : (
        <>
          {/* What the next backup does, side by side: the newest pack, the packs made on their own, the keys. */}
          <div className={GRID3}>
            <Panel>
              <CardTitle>{t('Gói gần nhất', 'Latest pack')}</CardTitle>
              {last ? (
                <>
                  <div className="text-h2 font-bold tracking-[-.02em] tabular-nums">{packWhen(last.at, now)}</div>
                  <p className="mt-0.5 mb-1 text-sm text-ink2">{macOf(last, model.machine) + ' · ' + packLine(last)}</p>
                  <CardText className="m-0">{keysLine(last.keys)}</CardText>
                  <div className="mt-auto pt-2.5">
                    <LinkButton small onClick={() => backup.openView(last.id)}>
                      {t('Xem trong gói', 'See inside')}
                    </LinkButton>
                  </div>
                </>
              ) : (
                <CardText className="m-0">{mac(t('Chưa có gói nào mở được trên máy này.', 'No pack can be opened on this Mac yet.'))}</CardText>
              )}
            </Panel>
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
              <CardText className="mt-auto mb-0 pt-2">{mac(t('Giữ 10 gói gần nhất của mỗi Mac.', 'Keeps the 10 latest packs per Mac.'))}</CardText>
            </Panel>
            <Panel>
              <CardTitle>
                {t('Khoá đã lưu', 'Saved keys')} <span className={COUNT}>{keys.length}</span>
              </CardTitle>
              {keys.map((k) => (
                <div key={k.mcp + '|' + k.slot} className={SETTING_ROW}>
                  <div className="min-w-0">
                    <b className="block truncate">{k.mcp}</b>
                    <small className="truncate">{k.bitwarden ? t('Lưu ở: ', 'Stored in: ') + k.bitwarden : k.label}</small>
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
              {!keys.length ? <CardText className="mt-0 mb-0">{mac(t('Chưa có khoá nào app giữ trên máy này.', 'The app keeps no keys on this Mac yet.'))}</CardText> : null}
              <CardText className="mt-auto mb-0 pt-2">{mac(t('Khoá chỉ nằm trên máy này, không vào gói.', 'Keys stay on this Mac and never go into a pack.'))}</CardText>
            </Panel>
          </div>
          <Panel>
            <CardTitle>
              {t('Các gói', 'Packs')} <span className={COUNT}>{packs.length}</span>
            </CardTitle>
            <PackTable packs={packs} model={model} now={now} backup={backup} />
          </Panel>
        </>
      )}
    </>
  );
}
