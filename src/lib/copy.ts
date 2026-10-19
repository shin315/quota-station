// The Chép sang Mac khác tab (M13, D), as plain functions: which items can go
// which way, which are ticked to begin with, what is sent, and every line the
// tab and its dialogs say. Plain words, as the approved prototype's `cp`
// screens: no path to a command, no hash.

import { AGENT_NAME } from './mcp';
import { fileLabel, names, partLines, type PartLine } from './backup';
import { clock, pad } from './format';
import type { Agent, CopyComparison, CopyItem, CopySelection, RestoreDone, RestoreItemResult, RestoreUndone } from './model';
import type { ChipView, FeedbackView } from './view';

export type CopyTicks = Record<string, boolean>;

export const GROUPS: { group: CopyItem['group']; title: string }[] = [
  { group: 'mcp', title: 'MCP' },
  { group: 'skill', title: 'Skills' },
  { group: 'plugin', title: 'Plugin' },
  { group: 'file', title: 'File hướng dẫn' },
];

export const itemKey = (i: Pick<CopyItem, 'group' | 'id' | 'agent'>): string => i.group + '|' + (i.agent ?? '') + '|' + i.id;

/** The item's name as the list shows it. */
export function itemName(i: CopyItem): string {
  if (i.group === 'file') return fileLabel(i.id, i.agent);
  if (i.group === 'plugin') return i.id.split('@')[0] + (i.agent ? ' · ' + AGENT_NAME[i.agent] : '');
  return i.id;
}

/** Only on the Mac it would go to: it goes the other way. */
export const wrongWay = (i: CopyItem, push: boolean): boolean => (push ? i.state === 'theirs' : i.state === 'mine');

export const blocked = (i: CopyItem, push: boolean): CopyItem['push'] => (push ? i.push : i.pull);

export const selectable = (i: CopyItem, push: boolean): boolean => !wrongWay(i, push) && !blocked(i, push);

/** Ticked to begin with: what only the sending Mac has. What both have, the owner ticks. */
export function initialTicks(c: CopyComparison, push: boolean): CopyTicks {
  const out: CopyTicks = {};
  for (const i of c.diff.items) if (i.state === (push ? 'mine' : 'theirs') && selectable(i, push)) out[itemKey(i)] = true;
  return out;
}

export const picked = (c: CopyComparison, ticks: CopyTicks, push: boolean): CopyItem[] =>
  c.diff.items.filter((i) => ticks[itemKey(i)] && selectable(i, push));

/** What is sent for the items ticked. A skill of another version is brought to its newest. */
export function selectionOf(items: CopyItem[]): CopySelection {
  const s: CopySelection = { mcp: [], skills: [], updates: [], plugins: [], files: [] };
  for (const i of items) {
    if (i.group === 'mcp') s.mcp.push(i.id);
    else if (i.group === 'skill') (i.state === 'ver' ? s.updates : s.skills).push(i.id);
    else if (i.group === 'plugin' && i.agent) s.plugins.push({ agent: i.agent, id: i.id });
    else if (i.group === 'file') s.files.push(i.id);
  }
  return s;
}

/** The chip beside an item. */
export function stateChip(i: CopyItem, other: string): ChipView {
  switch (i.state) {
    case 'mine':
      return { t: 'chỉ có ở máy này', k: 'info' };
    case 'theirs':
      return { t: 'chỉ có ở ' + other, k: '' };
    case 'diff':
      return { t: 'khác cấu hình', k: 'warn' };
    case 'ver':
      return { t: 'khác bản', k: 'warn' };
    case 'text':
      return { t: 'khác nội dung', k: 'warn' };
  }
}

/** "Xem": the two Macs side by side. */
export const hasView = (i: CopyItem): boolean => i.state === 'diff' || i.state === 'ver' || i.state === 'text';

const WHAT: Record<string, string> = {
  command: 'lệnh',
  args: 'tham số',
  env: 'biến môi trường',
  cwd: 'thư mục chạy',
  url: 'địa chỉ',
  headers: 'header',
  agents: 'agent có nó',
};

/** `2026-10-17T…` as `17/10`. */
export function day(iso: string | null): string {
  if (!iso) return '?';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? '?' : pad(d.getDate()) + '/' + pad(d.getMonth() + 1);
}

const LEFT_OUT: Record<string, string> = {
  key: 'có dòng giống khoá',
  link: 'có liên kết bên trong',
  large: 'lớn quá',
  unreadable: 'không đọc được',
};

/** The line under an item's name. */
export function itemNote(i: CopyItem, push: boolean, here: string, other: string, keys: boolean): string {
  const dest = push ? other : here;
  if (wrongWay(i, push)) return push ? 'Chọn chiều Lấy về để lấy mục này.' : 'Chọn chiều Từ máy này để gửi mục này.';
  const why = blocked(i, push);
  if (i.group === 'mcp') {
    if (why === 'absent') return dest + ' chưa cài agent nào dùng MCP này.';
    if (i.state === 'diff') {
      const what = i.what.map((w) => WHAT[w] ?? w);
      return what.length ? 'Khác ' + what.join(', ') : 'Khác cấu hình';
    }
    if (i.keys.length) return keys ? 'Gửi kèm khoá qua SSH' : 'Cần dán khoá trên ' + dest;
    return 'Thêm cho các agent đang có nó';
  }
  if (i.group === 'skill') {
    if (why === 'leftOut') return 'Cài tay, ' + (LEFT_OUT[i.note ?? ''] ?? 'không đọc được') + ': không chép';
    if (why === 'source') return 'Hai máy cài từ nguồn khác nhau: không chép';
    if (why === 'hand' || i.state === 'text') return 'Hai bản cài tay khác nhau: không chép';
    if (i.state === 'ver') {
      const line = other + ' có bản ' + day(i.theirsAt) + ', máy này ' + day(i.mineAt);
      return why === 'older' ? line + '. Bản trên ' + dest + ' mới hơn.' : line;
    }
    return i.hand ? 'Cài tay: chép cả thư mục' : 'Từ GitHub: cài bản mới nhất trên ' + dest;
  }
  if (i.group === 'plugin') return 'Chạy lệnh cài của ' + (i.agent ? AGENT_NAME[i.agent] : 'agent') + ' trên ' + dest;
  if (why === 'key') return 'Có dòng giống khoá: không chép';
  if (why === 'excluded') return 'Bạn để file này ngoài gói sao lưu: không chép';
  if (why === 'unreadable') return 'Không đọc được file: không chép';
  if (i.state === 'text') return 'Khác ' + i.changed + ' dòng';
  return 'Chưa có trên ' + (i.state === 'mine' ? other : here);
}

/** What the copy does with an item, for the preview. */
export function planLine(i: CopyItem, keys: boolean, dest: string): string {
  const name = itemName(i);
  if (i.group === 'mcp') {
    const what = i.state === 'diff' ? 'thay cấu hình bằng bản của máy gửi' : 'thêm cho các agent đang có nó';
    return name + ': ' + what + (i.keys.length && !keys ? ' · tắt đến khi dán khoá' : '');
  }
  if (i.group === 'skill') return name + ': ' + (i.state === 'ver' ? 'cập nhật lên bản mới nhất từ GitHub' : 'cài trên ' + dest);
  if (i.group === 'plugin') return name + ': cài bằng lệnh của ' + (i.agent ? AGENT_NAME[i.agent] : 'agent');
  return name + ': ' + (i.state === 'text' ? 'thay bằng bản của máy gửi' : 'thêm file');
}

/** "Giống nhau: 11 mục." */
export const sameLine = (c: CopyComparison): string => 'Giống nhau: ' + c.diff.same.length + ' mục.';

/** When the comparison read the other Mac: "đọc lúc 14:35". */
export const readAt = (c: CopyComparison): string => 'đọc lúc ' + clock(new Date(c.at * 1000).toISOString());

/** A server's line of the result, as the copy says it. */
function serverLine(r: RestoreItemResult, dest: string): [string, string, string] {
  const n = r.done.length + r.kept.length;
  if (r.kind === 'replace') {
    if (r.done.length) return ['', r.name + ' · thay cấu hình', ''];
    if (r.skipped) return ['w', r.name + ' · giữ bản trên ' + dest, r.skipped === 'noKey' ? 'chưa có khoá' : 'máy đó chưa cài lệnh của nó'];
    return ['x', r.name, 'chưa ghi được'];
  }
  if (r.failed.length && !n) return ['x', r.name, 'chưa ghi được'];
  if (r.kept.length && !r.done.length)
    return ['w', r.name + ' · thêm nhưng tắt', r.why === 'noCommand' ? 'máy đó chưa cài ' + (r.command ?? 'lệnh của nó') : r.why === 'noKey' ? dest + ' sẽ hỏi dán khoá' : 'tắt như trên máy gửi'];
  return [r.failed.length ? 'w' : '', r.name, r.failed.length ? 'chưa ghi được ở ' + r.failed.map((a) => AGENT_NAME[a]).join(', ') : ''];
}

/** Every line of the result: the servers, then the skills, plugins and files. */
export function doneLines(done: RestoreDone, dest: string): [string, string, string][] {
  const parts = done.parts ?? [];
  const updated = parts.filter((r) => r.ok && r.part === 'skill' && r.kind === 'update');
  const rest: PartLine[] = partLines(parts.filter((r) => !(r.ok && r.kind === 'update')));
  const out: [string, string, string][] = done.items.map((r) => serverLine(r, dest));
  if (updated.length) out.push(['', updated.length + ' skill lên bản mới nhất', names(updated.map((r) => r.id))]);
  for (const l of rest) out.push([l.tone, l.title.replace('thay bằng bản trong gói', 'thay bằng bản của máy gửi'), l.sub]);
  if (done.failed) out.push(['x', 'MCP', 'một phần chưa ghi được']);
  return out;
}

/** Why a step did not run, in words. */
export function copyError(e: unknown, other: string): FeedbackView {
  const s = String(e);
  if (s === 'timeout' || s.startsWith('unreachable'))
    return { tone: 'bad', title: other + ' không trả lời qua SSH', body: 'Chờ 20 giây không thấy trả lời. Kiểm máy đã bật và tên kết nối trong Cài đặt › Các Mac khác.' };
  if (s === 'old') return { tone: 'warn', title: 'Quota Station trên ' + other + ' chưa có phần MCP và skills', body: 'Máy đó chạy bản 1. Cập nhật app trên máy đó (Cài đặt › Cập nhật từ mã nguồn) rồi so sánh lại.' };
  if (s === 'noApp') return { tone: 'warn', title: 'Không thấy Quota Station trên ' + other, body: 'Cài app vào thư mục Applications của máy đó rồi so sánh lại.' };
  if (s === 'noSsh') return { tone: 'bad', title: 'Không thấy ssh trên máy này', body: 'App dùng ssh của macOS để nói chuyện với Mac khác.' };
  if (s === 'unknownMac') return { tone: 'warn', title: 'Mac này không còn trong danh sách', body: 'Kiểm lại Cài đặt › Các Mac khác.' };
  if (s === 'busy' || s === 'refused:busy') return { tone: 'warn', title: 'Đang có một lần ghi khác', body: 'Đợi xong rồi thử lại.' };
  if (s === 'stale') return { tone: 'warn', title: 'Cần so sánh lại', body: 'Bấm So sánh lại để đọc hai máy rồi chọn mục.' };
  if (s.startsWith('changed:'))
    return { tone: 'warn', title: 'Chưa chép: file vừa đổi', body: 'File của agent trên máy nhận vừa đổi sau lần so sánh. Chưa ghi gì. Bấm So sánh lại.' };
  if (s === 'keychain') return { tone: 'bad', title: 'Chưa lưu được khoá', body: 'Keychain trên máy nhận không nhận khoá lúc này. Chưa ghi gì. Thử lại, hoặc tắt Gửi cả khoá.' };
  if (s === 'skill') return { tone: 'bad', title: 'Chưa chép', body: 'Một skill cài tay vừa đổi và có chỗ trông giống khoá. Không có gì rời khỏi máy.' };
  if (s === 'waiting') return { tone: 'warn', title: other + ' vẫn đang ghi', body: 'Mở tab Sao lưu trên máy đó để xem kết quả, hoặc so sánh lại sau.' };
  if (s === 'lost') return { tone: 'bad', title: 'Lần ghi trên máy nhận dừng giữa chừng', body: 'Mở tab Sao lưu trên máy đó: nếu có lần khôi phục mới thì hoàn tác được ở đó.' };
  if (s === 'gone') return { tone: 'warn', title: 'Không còn gì để hoàn tác', body: 'Máy nhận đã hoàn tác, hoặc đã khôi phục một lần khác sau đó.' };
  if (s === 'protocol' || s === 'refused:protocol') return { tone: 'warn', title: 'Hai máy chạy hai bản app khác nhau', body: 'Cập nhật Quota Station trên cả hai máy rồi thử lại.' };
  return { tone: 'bad', title: 'Chưa chạy được', body: 'App không làm được bước này. Không có gì được ghi.' };
}

/** "Đã hoàn tác trên MacBook Air." */
export function undoneFeedback(u: RestoreUndone, dest: string): FeedbackView {
  const n = u.outcome ? u.outcome.done.reduce((t, d) => t + d.changes, 0) : 0;
  const said = [
    n ? 'trả ' + n + ' mục về như trước' : '',
    u.kept ? 'bỏ ' + u.kept + ' MCP đang giữ tắt' : '',
    u.keys ? 'xoá ' + u.keys + ' khoá vừa gửi' : '',
    u.skills ? 'trả ' + u.skills + ' skill về như trước' : '',
    u.plugins ? 'gỡ ' + u.plugins + ' plugin' : '',
    u.files ? 'trả ' + u.files + ' file hướng dẫn về như trước' : '',
  ]
    .filter(Boolean)
    .join(', ');
  const left = u.left ? ' Giữ nguyên ' + u.left + ' mục đã sửa sau lần chép.' : '';
  return { tone: 'info', title: 'Đã hoàn tác trên ' + dest + '.', body: (said ? said.charAt(0).toUpperCase() + said.slice(1) + '.' : 'Không còn gì để trả về.') + left };
}

/** A Mac in the list: its name, and how it answered last. */
export function macChip(state: 'ok' | 'unreach' | 'old' | 'noApp' | undefined): ChipView | null {
  if (state === 'ok') return { t: 'SSH ổn', k: 'ok' };
  if (state === 'unreach') return { t: 'không trả lời', k: 'bad' };
  if (state === 'old') return { t: 'bản cũ', k: 'warn' };
  if (state === 'noApp') return { t: 'chưa có app', k: 'warn' };
  return null;
}

/** How a Mac answered, from what a call rejected with. */
export function macState(e: unknown): 'unreach' | 'old' | 'noApp' | undefined {
  const s = String(e);
  if (s === 'timeout' || s.startsWith('unreachable')) return 'unreach';
  if (s === 'old') return 'old';
  if (s === 'noApp') return 'noApp';
  return undefined;
}

/** The agents a server is on, for the comparison. */
export const onText = (agents: Agent[]): string => (agents.length ? agents.map((a) => AGENT_NAME[a]).join(', ') : 'không agent nào');
