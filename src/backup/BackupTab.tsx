// The Sao lưu tab of the main window (M10), as the approved prototype's
// `bk` screen after round 2: the newest pack, every Mac's packs in iCloud
// Drive, the packs made on their own, and the keys the app keeps (name and
// Bitwarden item only: replace or delete, never show). No folder path, no
// "Keychain", nothing technical.

import { Banner, Button, Chip, Dot, Feedback, Toggle } from '../components/parts';
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
    <div className="pk">
      <span>
        {when}
        <small className="m">{macOf(p, model.machine)}</small>
      </span>
      <span className="m">
        {p.state === 'ok' ? packLine(p) : null}
        {note ? <Chip chip={{ t: note, k: p.state === 'broken' ? 'bad' : 'warn' }} /> : null}
      </span>
      <span className="a">
        <button type="button" className="lnk sm" aria-label={'Xem gói ' + when} disabled={!open} onClick={() => backup.openView(p.id)}>
          Xem
        </button>
        <button type="button" className="lnk sm" aria-label={'Khôi phục gói ' + when} disabled={!open || backup.working} onClick={() => backup.openRestore(p.id)}>
          Khôi phục…
        </button>
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
    <div className="tb" data-tauri-drag-region>
      <h2>Sao lưu</h2>
      <small>
        <Dot tone={v.icloud ? 'ok' : 'stale'} />
        {model.machine} · iCloud Drive
      </small>
      <Button primary disabled={busy || !v.icloud} onClick={backup.backupNow}>
        {v.busy === 'packing' ? 'Đang sao lưu…' : 'Sao lưu bây giờ'}
      </Button>
      <Button disabled={busy || !packs?.some(usable)} onClick={() => backup.openRestore()}>
        Khôi phục…
      </Button>
    </div>
  );
  if (!packs)
    return (
      <>
        {top}
        <div className="card">
          <div className="empty">
            <b>Đang tìm các gói sao lưu…</b>
            Trong iCloud Drive.
          </div>
        </div>
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
        <div className="card">
          <div className="empty">
            <span className="ei" aria-hidden="true">
              {ARCHIVE_BIG}
            </span>
            <b>Chưa có gói sao lưu nào</b>
            <p className="note">Gói gồm các MCP của bạn, không có khoá. Lưu trong iCloud Drive để máy khác lấy được.</p>
            <div className="btns">
              <Button primary disabled={busy || !v.icloud} onClick={backup.backupNow}>
                Sao lưu lần đầu
              </Button>
            </div>
          </div>
        </div>
      ) : (
        <div className="bkg">
          <div className="stack">
            {last ? (
              <div className="card">
                <h4>Gói gần nhất</h4>
                <div className="bigw">{packWhen(last.at, now)}</div>
                <p className="ln">{macOf(last, model.machine) + ' · ' + packLine(last)}</p>
                <p className="note" style={{ margin: 0 }}>
                  {keysLine(last.keys)}{' '}
                  <button type="button" className="lnk sm" onClick={() => backup.openView(last.id)}>
                    Xem trong gói
                  </button>
                </p>
              </div>
            ) : null}
            <div className="card">
              <h4>
                Các gói <span className="cnt">{packs.length}</span>
              </h4>
              {packs.map((p) => (
                <PackRow key={p.id} p={p} model={model} now={now} backup={backup} />
              ))}
            </div>
          </div>
          <div className="stack">
            <div className="card">
              <h4>Tự sao lưu</h4>
              <div className="set">
                <span>Sau mỗi thay đổi trong app</span>
                <Toggle on={v.auto.afterChange} label="Sau mỗi thay đổi trong app" onClick={() => backup.setAuto(!v.auto.afterChange, v.auto.daily)} />
              </div>
              <div className="set">
                <span>Mỗi ngày, nếu agent có gì đổi</span>
                <Toggle on={v.auto.daily} label="Mỗi ngày, nếu agent có gì đổi" onClick={() => backup.setAuto(v.auto.afterChange, !v.auto.daily)} />
              </div>
              <p className="note">Giữ 10 gói gần nhất của mỗi Mac.</p>
            </div>
            <div className="card">
              <h4>
                Khoá đã lưu <span className="cnt">{keys.length}</span>
              </h4>
              {keys.map((k) => (
                <div key={k.mcp + '|' + k.slot} className="set">
                  <div>
                    <b>{k.mcp}</b>
                    <small>{k.bitwarden ? 'Trong Bitwarden: ' + k.bitwarden : k.label}</small>
                  </div>
                  <div className="a">
                    <button type="button" className="lnk sm" aria-label={'Thay khoá ' + k.mcp} disabled={busy} onClick={() => backup.setDialog({ kind: 'key', key: k })}>
                      Thay
                    </button>
                    <button type="button" className="lnk sm" aria-label={'Xoá khoá ' + k.mcp} disabled={busy} onClick={() => backup.setDialog({ kind: 'delKey', key: k })}>
                      Xoá
                    </button>
                  </div>
                </div>
              ))}
              {!keys.length ? <p className="note">Chưa có khoá nào app giữ trên máy này.</p> : null}
              <p className="note">Khoá chỉ nằm trên máy này, không vào gói.</p>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
