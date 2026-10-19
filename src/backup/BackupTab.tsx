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
      title: 'iCloud Drive đang tắt trên máy này',
      body: 'Bật iCloud Drive để sao lưu, và để lấy gói của Mac khác.',
      actions: [{ label: 'Kiểm lại', act: 'backup:reread' }],
    });
  const cloud = (v.packs ?? []).filter((p) => p.state === 'cloud');
  if (cloud.length)
    out.push({
      tone: 'warn',
      title: cloud.length === 1 ? 'Gói ' + packWhenLower(cloud[0].at, now) + ' chưa tải về máy' : cloud.length + ' gói chưa tải về máy',
      body: 'iCloud Drive mới có tên file. Mở thư mục trong Finder để tải, hoặc chờ.',
      actions: [
        { label: 'Mở trong Finder', act: 'backup:open' },
        { label: 'Kiểm lại', act: 'backup:reread' },
      ],
    });
  const auto = autoErrorText(v);
  if (auto) out.push({ tone: 'warn', title: auto, body: 'Bấm Sao lưu bây giờ để thử lại.', actions: [] });
  if (v.restore)
    out.push({
      tone: 'info',
      title: 'Đã khôi phục lúc ' + clock(new Date(v.restore.at * 1000).toISOString()),
      body: 'Từ gói của ' + v.restore.mac + '. Hoàn tác được cả lần khôi phục.',
      actions: [{ label: 'Hoàn tác…', act: 'backup:undo' }],
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
        <LinkButton small aria-label={'Xem gói ' + when} disabled={!open} onClick={() => backup.openView(p.id)}>
          Xem
        </LinkButton>
        <LinkButton small aria-label={'Khôi phục gói ' + when} disabled={!open || backup.working} onClick={() => backup.openRestore(p.id)}>
          Khôi phục…
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
      <TabTitle>Sao lưu</TabTitle>
      <TabSub>
        <Dot tone={v.icloud ? 'ok' : 'stale'} />
        {model.machine} · iCloud Drive
      </TabSub>
      <Button primary disabled={busy || !v.icloud} onClick={backup.backupNow}>
        {v.busy === 'packing' ? 'Đang sao lưu…' : 'Sao lưu bây giờ'}
      </Button>
      <Button disabled={busy || !packs?.some(usable)} onClick={() => backup.openRestore()}>
        Khôi phục…
      </Button>
    </TabHead>
  );
  if (!packs)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <b>Đang tìm các gói sao lưu…</b>
            Trong iCloud Drive.
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
            <b>Chưa có gói sao lưu nào</b>
            <CardText className="mt-2 mb-0">Gói gồm các MCP của bạn, không có khoá. Lưu trong iCloud Drive để máy khác lấy được.</CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary disabled={busy || !v.icloud} onClick={backup.backupNow}>
                Sao lưu lần đầu
              </Button>
            </div>
          </Empty>
        </Panel>
      ) : (
        <div className={cn(GRID, 'max-[860px]:grid-cols-[minmax(0,1fr)]')}>
          <div className={STACK}>
            {last ? (
              <Panel>
                <CardTitle>Gói gần nhất</CardTitle>
                <div className="text-h2 font-bold tracking-[-.02em] tabular-nums">{packWhen(last.at, now)}</div>
                <p className="mt-0.5 mb-1 text-sm text-ink2">{macOf(last, model.machine) + ' · ' + packLine(last)}</p>
                <CardText className="m-0">
                  {keysLine(last.keys)}{' '}
                  <LinkButton small onClick={() => backup.openView(last.id)}>
                    Xem trong gói
                  </LinkButton>
                </CardText>
              </Panel>
            ) : null}
            <Panel>
              <CardTitle>
                Các gói <span className={COUNT}>{packs.length}</span>
              </CardTitle>
              {packs.map((p) => (
                <PackRow key={p.id} p={p} model={model} now={now} backup={backup} />
              ))}
            </Panel>
          </div>
          <div className={STACK}>
            <Panel>
              <CardTitle>Tự sao lưu</CardTitle>
              <div className={SETTING_ROW}>
                <span>Sau mỗi thay đổi trong app</span>
                <Toggle on={v.auto.afterChange} label="Sau mỗi thay đổi trong app" onClick={() => backup.setAuto(!v.auto.afterChange, v.auto.daily)} />
              </div>
              <div className={SETTING_ROW}>
                <span>Mỗi ngày, nếu agent có gì đổi</span>
                <Toggle on={v.auto.daily} label="Mỗi ngày, nếu agent có gì đổi" onClick={() => backup.setAuto(v.auto.afterChange, !v.auto.daily)} />
              </div>
              <CardText className="mt-2 mb-0">Giữ 10 gói gần nhất của mỗi Mac.</CardText>
            </Panel>
            <Panel>
              <CardTitle>
                Khoá đã lưu <span className={COUNT}>{keys.length}</span>
              </CardTitle>
              {keys.map((k) => (
                <div key={k.mcp + '|' + k.slot} className={SETTING_ROW}>
                  <div>
                    <b>{k.mcp}</b>
                    <small>{k.bitwarden ? 'Trong Bitwarden: ' + k.bitwarden : k.label}</small>
                  </div>
                  <div className={ROW_ACTIONS}>
                    <LinkButton small aria-label={'Thay khoá ' + k.mcp} disabled={busy} onClick={() => backup.setDialog({ kind: 'key', key: k })}>
                      Thay
                    </LinkButton>
                    <LinkButton small aria-label={'Xoá khoá ' + k.mcp} disabled={busy} onClick={() => backup.setDialog({ kind: 'delKey', key: k })}>
                      Xoá
                    </LinkButton>
                  </div>
                </div>
              ))}
              {!keys.length ? <CardText className="mt-2 mb-0">Chưa có khoá nào app giữ trên máy này.</CardText> : null}
              <CardText className="mt-2 mb-0">Khoá chỉ nằm trên máy này, không vào gói.</CardText>
            </Panel>
          </div>
        </div>
      )}
    </>
  );
}
