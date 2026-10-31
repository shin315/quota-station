// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Chép sang Mac khác tab's dialogs (M13), as the prototype's `cp-item`,
// `cp-go` and `cp-done`: one item on both Macs side by side (a command's
// path is not shown: another Mac keeps it elsewhere), the copy's preview,
// the copy running, its result with "Hoàn tác trên …", and the undo, which
// runs on the Mac the copy was written to.

import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { COMPARE, COMPARE_DIFF, COMPARE_HEAD, DiffLine, DiffView, Progress, TECH } from '@/components/blocks';
import { Button } from '@/components/parts';
import { changedLines, plural } from '@/lib/backup';
import { day, doneLines, itemKey, itemName, onText, planLine, stateChip } from './copy';
import { t } from '@/lib/i18n';
import { AGENT_NAME, joinCommand } from '@/lib/mcp';
import type { CopyItem, McpDef } from '@/lib/model';
import { Facts } from '@/backup/RestoreSheet';
import type { Copy, CopyDialog } from './useCopy';

export function copyDialogSize(d: CopyDialog, copy: Copy): 'lg' | 'xl' {
  if (d.kind === 'item') return copy.cmp?.diff.items.find((i) => itemKey(i) === d.key)?.group === 'file' ? 'xl' : 'lg';
  return 'lg';
}

/** A definition as the comparison shows it: the command by its name only. */
function command(def: McpDef | null): string {
  if (!def) return t('không có', 'none');
  if (def.kind === 'http') return def.url;
  return joinCommand(def.command.split('/').pop() ?? def.command, def.args);
}

function pairs(def: McpDef | null): string {
  if (!def) return '';
  const list = def.kind === 'stdio' ? def.env : def.headers;
  return list.map((p) => p.name + ' = ' + p.value).join(', ');
}

function Cells({ k, a, b }: { k: string; a: string; b: string }) {
  const x = a !== b ? COMPARE_DIFF : undefined;
  return (
    <>
      <span className="text-ink2">{k}</span>
      <span className={x}>{a || '–'}</span>
      <span className={x}>{b || '–'}</span>
    </>
  );
}

function ItemBody({ i, copy }: { i: CopyItem; copy: Copy }) {
  const name = itemName(i);
  const head = (
    <>
      <span className={COMPARE_HEAD} />
      <span className={COMPARE_HEAD}>{copy.here}</span>
      <span className={COMPARE_HEAD}>{copy.other}</span>
    </>
  );
  let body;
  if (i.group === 'mcp') {
    body = (
      <>
        <div className={COMPARE}>
          {head}
          {i.compare.map((c) => (
            <Cells key={c.agent} k={AGENT_NAME[c.agent]} a={command(c.mine)} b={command(c.theirs)} />
          ))}
          {i.compare.some((c) => pairs(c.mine) || pairs(c.theirs)) ? (
            <Cells k={t('Biến, header', 'Variables, headers')} a={pairs(i.compare.find((c) => c.mine)?.mine ?? null)} b={pairs(i.compare.find((c) => c.theirs)?.theirs ?? null)} />
          ) : null}
          <Cells k={t('Agent bật', 'Agents on')} a={onText(i.mineOn)} b={onText(i.theirsOn)} />
        </div>
        <p className={TECH}>
          {t(
            'Đường dẫn lệnh khác nhau giữa hai máy là bình thường, không tính là khác. Khoá không đọc, không so.',
            'Command paths that differ between the two Macs are normal and do not count as a difference. Keys are not read or compared.',
          )}
        </p>
      </>
    );
  } else if (i.group === 'skill') {
    body = (
      <>
        <div className={COMPARE}>
          {head}
          <Cells k={t('Bản', 'Version')} a={i.mineAt ? day(i.mineAt) : t('cài tay', 'by hand')} b={i.theirsAt ? day(i.theirsAt) : t('cài tay', 'by hand')} />
        </div>
        <p className={TECH}>
          {i.state === 'ver'
            ? t(
                'Chép là cập nhật skill lên bản mới nhất từ GitHub trên máy có bản cũ hơn. App lưu bản đang có trước khi cập nhật.',
                'Copying updates the skill to the latest version from GitHub on the Mac with the older one. The app saves the current version before updating.',
              )
            : t(
                'Hai bản cài tay khác nhau: sửa trên một máy cho giống ý bạn, app không tự chọn bản nào.',
                'Two different versions installed by hand: edit one Mac to match what you want; the app does not pick one.',
              )}
        </p>
      </>
    );
  } else {
    // The lines run from this Mac's file to the other's: shown as the file
    // that would be written to now, then after the copy.
    const dest = copy.push ? copy.other : copy.here;
    const lines = copy.push ? i.lines.map((l) => ({ ...l, op: l.op === 'add' ? ('del' as const) : l.op === 'del' ? ('add' as const) : l.op })) : i.lines;
    body = (
      <DiffView head={<span>{t('Trên ' + dest + ' bây giờ → sau khi chép', 'On ' + dest + ' now → after the copy')}</span>} right={t(i.changed + ' dòng khác', plural(i.changed, 'line differs', 'lines differ'))}>
        {lines.length ? (
          lines.map((l, n) => (
            <DiffLine key={n} op={l.op}>
              {l.op === 'gap' ? '…' : l.text || ' '}
            </DiffLine>
          ))
        ) : (
          <DiffLine op="ctx">{t('Có dòng giống khoá ở một máy: app không hiện, không chép.', 'One Mac has a line that looks like a key: the app does not show or copy it.')}</DiffLine>
        )}
      </DiffView>
    );
  }
  return (
    <>
      <DialogTitle>
        {name} · {stateChip(i, copy.other).t}
      </DialogTitle>
      {body}
      <DialogButtons>
        <Button large primary onClick={copy.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </>
  );
}

function GoBody({ d, copy }: { d: Extract<CopyDialog, { kind: 'go' }>; copy: Copy }) {
  const items = copy.items;
  const dest = copy.push ? copy.other : copy.here;
  const keys = items.filter((i) => i.group === 'mcp').reduce((n, i) => n + i.keys.length, 0);
  if (d.step === 'prev')
    return (
      <>
        <DialogTitle>
          {t(
            'Chép ' + items.length + ' mục ' + (copy.push ? 'sang ' + copy.other : 'về máy này') + '?',
            'Copy ' + plural(items.length, 'item', 'items') + ' ' + (copy.push ? 'to ' + copy.other : 'to this Mac') + '?',
          )}
        </DialogTitle>
        <DialogLead>{t('Quota Station trên ' + dest + ' tự ghi, sao lưu trước khi ghi.', 'Quota Station on ' + dest + ' writes it itself, after a backup.')}</DialogLead>
        <Facts items={items.map((i) => ['', planLine(i, copy.keys, dest)])} />
        <Facts
          items={[
            ['ok', t('Không đổi nhà cung cấp Codex, đăng nhập hay tài khoản.', 'Does not change the Codex provider, sign-ins or accounts.')],
            copy.keys && keys
              ? ['w', t('Gửi ' + keys + ' khoá qua SSH vào Keychain của ' + dest + '.', 'Sends ' + plural(keys, 'key', 'keys') + ' over SSH into the Keychain on ' + dest + '.')]
              : ['ok', t('Không gửi khoá.', 'Sends no keys.')],
            ['ok', t('Hoàn tác được cả lần chép, trên ' + dest + '.', 'The whole copy can be undone, on ' + dest + '.')],
          ]}
        />
        {d.error ? (
          <p className="mt-1 text-xs text-bad-ink" role="alert">
            <b>{d.error.title}.</b> {d.error.body}
          </p>
        ) : null}
        <DialogButtons>
          <Button large onClick={copy.close}>
            {t('Huỷ', 'Cancel')}
          </Button>
          <Button large primary disabled={!items.length || copy.working} onClick={copy.run}>
            {t('Chép', 'Copy')}
          </Button>
        </DialogButtons>
      </>
    );
  if (d.step === 'run' || d.step === 'undoing' || !d.done)
    return (
      <>
        <DialogTitle>{d.step === 'undoing' ? t('Đang hoàn tác trên ' + dest + '…', 'Undoing on ' + dest + '…') : t('Đang chép…', 'Copying…')}</DialogTitle>
        <Progress busy lines={items.map((i): [string, string, string] => ['wait', itemName(i), ''])} />
        <p>
          {copy.push
            ? t(
                copy.other + ' tự ghi bằng Quota Station của nó. App hỏi lại mỗi giây, mỗi lần chờ tối đa 20 giây.',
                copy.other + ' writes it with its own Quota Station. The app checks every second, waiting up to 20 seconds each time.',
              )
            : t('Đang ghi trên máy này.', 'Writing on this Mac.')}
        </p>
      </>
    );
  const done = d.done;
  if (d.step === 'undo') {
    const changed = d.changed ? changedLines('changed:' + d.changed, 'copy') : null;
    return (
      <>
        <DialogTitle>{t('Hoàn tác lần chép trên ' + dest + '?', 'Undo the copy on ' + dest + '?')}</DialogTitle>
        {changed ? (
          <Facts
            items={[
              ...changed.map((line): [string, string] => ['w', line]),
              ['', t('Hoàn tác vẫn chỉ bỏ những gì lần chép đã thêm hay thay; phần sửa thêm vẫn giữ.', 'Undo still removes only what the copy added or replaced; later edits stay.')],
            ]}
          />
        ) : (
          <Facts
            items={[
              ['', t('Bỏ các mục vừa thêm và trả các mục vừa thay về như trước khi chép.', 'Removes the items just added and puts back what was replaced, as before the copy.')],
              [
                '',
                t(
                  'Xoá các khoá vừa gửi, gỡ skill và plugin vừa cài, trả skill vừa cập nhật về bản cũ.',
                  'Deletes the keys just sent, uninstalls the skills and plugins just installed, and returns updated skills to their old version.',
                ),
              ],
              ['ok', t('Chạy trên ' + dest + ', bằng bản lưu Quota Station ở đó giữ trước khi ghi.', 'Runs on ' + dest + ', from the copy Quota Station saved there before writing.')],
            ]}
          />
        )}
        <DialogButtons>
          <Button large onClick={copy.backToDone}>
            {t('Huỷ', 'Cancel')}
          </Button>
          <Button large danger disabled={copy.working} onClick={() => copy.undo(!!changed)}>
            {changed ? t('Vẫn hoàn tác', 'Undo anyway') : t('Hoàn tác', 'Undo')}
          </Button>
        </DialogButtons>
      </>
    );
  }
  const lines = doneLines(done, dest);
  const partly = lines.some(([tone]) => tone === 'x');
  return (
    <>
      <DialogTitle>{partly ? t('Đã chép một phần', 'Partly copied') : t('Đã chép ' + items.length + ' mục', 'Copied ' + plural(items.length, 'item', 'items'))}</DialogTitle>
      <Progress lines={lines} />
      {d.error ? (
        <p className="mt-1 text-xs text-bad-ink" role="alert">
          <b>{d.error.title}.</b> {d.error.body}
        </p>
      ) : null}
      <DialogButtons>
        {done.canUndo ? (
          <Button large disabled={copy.working} onClick={copy.askUndo}>
            {t('Hoàn tác trên ' + dest, 'Undo on ' + dest)}
          </Button>
        ) : null}
        <span className="flex-1" />
        <Button large primary onClick={copy.finish}>
          {t('Xong', 'Done')}
        </Button>
      </DialogButtons>
    </>
  );
}

export function CopyDialogBody({ copy }: { copy: Copy }) {
  const d = copy.dialog;
  if (!d) return null;
  if (d.kind === 'item') {
    const i = copy.cmp?.diff.items.find((x) => itemKey(x) === d.key);
    return i ? <ItemBody i={i} copy={copy} /> : null;
  }
  return <GoBody d={d} copy={copy} />;
}
