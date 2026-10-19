// The Chép sang Mac khác tab's dialogs (M13), as the prototype's `cp-item`,
// `cp-go` and `cp-done`: one item on both Macs side by side (a command's
// path is not shown: another Mac keeps it elsewhere), the copy's preview,
// the copy running, its result with "Hoàn tác trên …", and the undo, which
// runs on the Mac the copy was written to.

import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { COMPARE, COMPARE_DIFF, COMPARE_HEAD, DiffLine, DiffView, Progress, TECH } from '../components/blocks';
import { Button } from '../components/parts';
import { changedLines } from '../lib/backup';
import { day, doneLines, itemKey, itemName, onText, planLine, stateChip } from '../lib/copy';
import { AGENT_NAME, joinCommand } from '../lib/mcp';
import type { CopyItem, McpDef } from '../lib/model';
import { Facts } from './RestoreSheet';
import type { Copy, CopyDialog } from './useCopy';

export function copyDialogSize(d: CopyDialog, copy: Copy): 'lg' | 'xl' {
  if (d.kind === 'item') return copy.cmp?.diff.items.find((i) => itemKey(i) === d.key)?.group === 'file' ? 'xl' : 'lg';
  return 'lg';
}

/** A definition as the comparison shows it: the command by its name only. */
function command(def: McpDef | null): string {
  if (!def) return 'không có';
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
            <Cells k="Biến, header" a={pairs(i.compare.find((c) => c.mine)?.mine ?? null)} b={pairs(i.compare.find((c) => c.theirs)?.theirs ?? null)} />
          ) : null}
          <Cells k="Agent bật" a={onText(i.mineOn)} b={onText(i.theirsOn)} />
        </div>
        <p className={TECH}>Đường dẫn lệnh khác nhau giữa hai máy là bình thường, không tính là khác. Khoá không đọc, không so.</p>
      </>
    );
  } else if (i.group === 'skill') {
    body = (
      <>
        <div className={COMPARE}>
          {head}
          <Cells k="Bản" a={i.mineAt ? day(i.mineAt) : 'cài tay'} b={i.theirsAt ? day(i.theirsAt) : 'cài tay'} />
        </div>
        <p className={TECH}>
          {i.state === 'ver'
            ? 'Chép là cập nhật skill lên bản mới nhất từ GitHub trên máy có bản cũ hơn. App lưu bản đang có trước khi cập nhật.'
            : 'Hai bản cài tay khác nhau: sửa trên một máy cho giống ý bạn, app không tự chọn bản nào.'}
        </p>
      </>
    );
  } else {
    // The lines run from this Mac's file to the other's: shown as the file
    // that would be written to now, then after the copy.
    const dest = copy.push ? copy.other : copy.here;
    const lines = copy.push ? i.lines.map((l) => ({ ...l, op: l.op === 'add' ? ('del' as const) : l.op === 'del' ? ('add' as const) : l.op })) : i.lines;
    body = (
      <DiffView head={<span>Trên {dest} bây giờ → sau khi chép</span>} right={i.changed + ' dòng khác'}>
        {lines.length ? (
          lines.map((l, n) => (
            <DiffLine key={n} op={l.op}>
              {l.op === 'gap' ? '…' : l.text || ' '}
            </DiffLine>
          ))
        ) : (
          <DiffLine op="ctx">Có dòng giống khoá ở một máy: app không hiện, không chép.</DiffLine>
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
          Đóng
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
          Chép {items.length} mục {copy.push ? 'sang ' + copy.other : 'về máy này'}?
        </DialogTitle>
        <DialogLead>Quota Station trên {dest} tự ghi, sao lưu trước khi ghi.</DialogLead>
        <Facts items={items.map((i) => ['', planLine(i, copy.keys, dest)])} />
        <Facts
          items={[
            ['ok', 'Không đổi nhà cung cấp Codex, đăng nhập hay tài khoản.'],
            copy.keys && keys ? ['w', 'Gửi ' + keys + ' khoá qua SSH vào Keychain của ' + dest + '.'] : ['ok', 'Không gửi khoá.'],
            ['ok', 'Hoàn tác được cả lần chép, trên ' + dest + '.'],
          ]}
        />
        {d.error ? (
          <p className="mt-1 text-xs text-bad-ink" role="alert">
            <b>{d.error.title}.</b> {d.error.body}
          </p>
        ) : null}
        <DialogButtons>
          <Button large onClick={copy.close}>
            Huỷ
          </Button>
          <Button large primary disabled={!items.length || copy.working} onClick={copy.run}>
            Chép
          </Button>
        </DialogButtons>
      </>
    );
  if (d.step === 'run' || d.step === 'undoing' || !d.done)
    return (
      <>
        <DialogTitle>{d.step === 'undoing' ? 'Đang hoàn tác trên ' + dest + '…' : 'Đang chép…'}</DialogTitle>
        <Progress busy lines={items.map((i): [string, string, string] => ['wait', itemName(i), ''])} />
        <p>{copy.push ? copy.other + ' tự ghi bằng Quota Station của nó. App hỏi lại mỗi giây, mỗi lần chờ tối đa 20 giây.' : 'Đang ghi trên máy này.'}</p>
      </>
    );
  const done = d.done;
  if (d.step === 'undo') {
    const changed = d.changed ? changedLines('changed:' + d.changed).map((t) => t.replace('lần khôi phục', 'lần chép')) : null;
    return (
      <>
        <DialogTitle>Hoàn tác lần chép trên {dest}?</DialogTitle>
        {changed ? (
          <Facts items={[...changed.map((t): [string, string] => ['w', t]), ['', 'Hoàn tác vẫn chỉ bỏ những gì lần chép đã thêm hay thay; phần sửa thêm vẫn giữ.']]} />
        ) : (
          <Facts
            items={[
              ['', 'Bỏ các mục vừa thêm và trả các mục vừa thay về như trước khi chép.'],
              ['', 'Xoá các khoá vừa gửi, gỡ skill và plugin vừa cài, trả skill vừa cập nhật về bản cũ.'],
              ['ok', 'Chạy trên ' + dest + ', bằng bản lưu Quota Station ở đó giữ trước khi ghi.'],
            ]}
          />
        )}
        <DialogButtons>
          <Button large onClick={copy.backToDone}>
            Huỷ
          </Button>
          <Button large danger disabled={copy.working} onClick={() => copy.undo(!!changed)}>
            {changed ? 'Vẫn hoàn tác' : 'Hoàn tác'}
          </Button>
        </DialogButtons>
      </>
    );
  }
  const lines = doneLines(done, dest);
  const partly = lines.some(([t]) => t === 'x');
  return (
    <>
      <DialogTitle>{partly ? 'Đã chép một phần' : 'Đã chép ' + items.length + ' mục'}</DialogTitle>
      <Progress lines={lines} />
      {d.error ? (
        <p className="mt-1 text-xs text-bad-ink" role="alert">
          <b>{d.error.title}.</b> {d.error.body}
        </p>
      ) : null}
      <DialogButtons>
        {done.canUndo ? (
          <Button large disabled={copy.working} onClick={copy.askUndo}>
            Hoàn tác trên {dest}
          </Button>
        ) : null}
        <span className="flex-1" />
        <Button large primary onClick={copy.finish}>
          Xong
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
