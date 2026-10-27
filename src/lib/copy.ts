// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Chép sang Mac khác tab (M13, D), as plain functions: which items can go
// which way, which are ticked to begin with, what is sent, and every line the
// tab and its dialogs say. Plain words, as the approved prototype's `cp`
// screens: no path to a command, no hash.

import { AGENT_NAME } from './mcp';
import { fileLabel, names, partLines, plural, type PartLine } from './backup';
import { clock, pad } from './format';
import { t } from './i18n';
import type { Agent, CopyComparison, CopyItem, CopySelection, RestoreDone, RestoreItemResult, RestoreUndone } from './model';
import type { ChipView, FeedbackView } from './view';

export type CopyTicks = Record<string, boolean>;

/** The groups of the list, in order, with their titles in the language on screen. */
export const groups = (): { group: CopyItem['group']; title: string }[] => [
  { group: 'mcp', title: 'MCP' },
  { group: 'skill', title: 'Skills' },
  { group: 'plugin', title: t('Plugin', 'Plugins') },
  { group: 'file', title: t('File hướng dẫn', 'Instruction files') },
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
      return { t: t('chỉ có ở máy này', 'only on this Mac'), k: 'info' };
    case 'theirs':
      return { t: t('chỉ có ở ' + other, 'only on ' + other), k: '' };
    case 'diff':
      return { t: t('khác cấu hình', 'different config'), k: 'warn' };
    case 'ver':
      return { t: t('khác bản', 'different version'), k: 'warn' };
    case 'text':
      return { t: t('khác nội dung', 'different content'), k: 'warn' };
  }
}

/** "Xem": the two Macs side by side. */
export const hasView = (i: CopyItem): boolean => i.state === 'diff' || i.state === 'ver' || i.state === 'text';

/** What differs in a server's configuration, by field. */
const whatText = (): Record<string, string> => ({
  command: t('lệnh', 'command'),
  args: t('tham số', 'arguments'),
  env: t('biến môi trường', 'environment variables'),
  cwd: t('thư mục chạy', 'working folder'),
  url: t('địa chỉ', 'URL'),
  headers: t('header', 'headers'),
  agents: t('agent có nó', 'which agents have it'),
});

/** `2026-10-17T…` as `17/10`. */
export function day(iso: string | null): string {
  if (!iso) return '?';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? '?' : pad(d.getDate()) + '/' + pad(d.getMonth() + 1);
}

/** Why a skill copied by hand cannot go, by reason. */
const leftOutText = (): Record<string, string> => ({
  key: t('có dòng giống khoá', 'has a line that looks like a key'),
  link: t('có liên kết bên trong', 'has links inside'),
  large: t('lớn quá', 'too large'),
  unreadable: t('không đọc được', 'cannot be read'),
});

/** The line under an item's name. */
export function itemNote(i: CopyItem, push: boolean, here: string, other: string, keys: boolean): string {
  const dest = push ? other : here;
  if (wrongWay(i, push))
    return push
      ? t('Chọn chiều Lấy về để lấy mục này.', 'Switch the direction to bring this item here.')
      : t('Chọn chiều Từ máy này để gửi mục này.', 'Switch the direction to send this item from this Mac.');
  const why = blocked(i, push);
  if (i.group === 'mcp') {
    if (why === 'absent') return t(dest + ' chưa cài agent nào dùng MCP này.', dest + ' has no agent that uses this MCP.');
    if (i.state === 'diff') {
      const labels = whatText();
      const what = i.what.map((w) => labels[w] ?? w);
      return what.length ? t('Khác ' + what.join(', '), 'Different ' + what.join(', ')) : t('Khác cấu hình', 'Different config');
    }
    if (i.keys.length) return keys ? t('Gửi kèm khoá qua SSH', 'Key sent along over SSH') : t('Cần dán khoá trên ' + dest, 'Needs a key pasted on ' + dest);
    return t('Thêm cho các agent đang có nó', 'Added for the agents that have it');
  }
  if (i.group === 'skill') {
    if (why === 'leftOut') {
      const reason = leftOutText()[i.note ?? ''] ?? t('không đọc được', 'cannot be read');
      return t('Cài tay, ' + reason + ': không chép', 'Installed by hand, ' + reason + ': not copied');
    }
    if (why === 'source') return t('Hai máy cài từ nguồn khác nhau: không chép', 'Installed from different sources on the two Macs: not copied');
    if (why === 'hand' || i.state === 'text') return t('Hai bản cài tay khác nhau: không chép', 'Two different versions installed by hand: not copied');
    if (i.state === 'ver') {
      const line = t(other + ' có bản ' + day(i.theirsAt) + ', máy này ' + day(i.mineAt), other + ' has the ' + day(i.theirsAt) + ' version, this Mac ' + day(i.mineAt));
      return why === 'older' ? line + t('. Bản trên ' + dest + ' mới hơn.', '. The one on ' + dest + ' is newer.') : line;
    }
    return i.hand ? t('Cài tay: chép cả thư mục', 'Installed by hand: copies the whole folder') : t('Từ GitHub: cài bản mới nhất trên ' + dest, 'From GitHub: installs the latest version on ' + dest);
  }
  if (i.group === 'plugin')
    return t(
      'Chạy lệnh cài của ' + (i.agent ? AGENT_NAME[i.agent] : 'agent') + ' trên ' + dest,
      'Runs ' + (i.agent ? AGENT_NAME[i.agent] + '’s' : 'the agent’s') + ' install command on ' + dest,
    );
  if (why === 'key') return t('Có dòng giống khoá: không chép', 'Has a line that looks like a key: not copied');
  if (why === 'excluded') return t('Bạn để file này ngoài gói sao lưu: không chép', 'You left this file out of backups: not copied');
  if (why === 'unreadable') return t('Không đọc được file: không chép', 'Cannot read the file: not copied');
  if (i.state === 'text') return t('Khác ' + i.changed + ' dòng', plural(i.changed, 'line differs', 'lines differ'));
  const where = i.state === 'mine' ? other : here;
  return t('Chưa có trên ' + where, 'Not on ' + where + ' yet');
}

/** What the copy does with an item, for the preview. */
export function planLine(i: CopyItem, keys: boolean, dest: string): string {
  const name = itemName(i);
  if (i.group === 'mcp') {
    const what = i.state === 'diff' ? t('thay cấu hình bằng bản của máy gửi', 'replace the config with the sending Mac’s') : t('thêm cho các agent đang có nó', 'add for the agents that have it');
    return name + ': ' + what + (i.keys.length && !keys ? t(' · tắt đến khi dán khoá', ' · off until a key is pasted') : '');
  }
  if (i.group === 'skill') return name + ': ' + (i.state === 'ver' ? t('cập nhật lên bản mới nhất từ GitHub', 'update to the latest version from GitHub') : t('cài trên ' + dest, 'install on ' + dest));
  if (i.group === 'plugin')
    return t(name + ': cài bằng lệnh của ' + (i.agent ? AGENT_NAME[i.agent] : 'agent'), name + ': install with ' + (i.agent ? AGENT_NAME[i.agent] + '’s' : 'the agent’s') + ' command');
  return name + ': ' + (i.state === 'text' ? t('thay bằng bản của máy gửi', 'replace with the sending Mac’s version') : t('thêm file', 'add the file'));
}

/** "Giống nhau: 11 mục." */
export const sameLine = (c: CopyComparison): string => t('Giống nhau: ' + c.diff.same.length + ' mục.', 'Same on both: ' + plural(c.diff.same.length, 'item', 'items') + '.');

/** When the comparison read the other Mac: "cập nhật lúc 14:35". */
export const readAt = (c: CopyComparison): string => t('cập nhật lúc ', 'updated at ') + clock(new Date(c.at * 1000).toISOString());

/** A server's line of the result, as the copy says it. */
function serverLine(r: RestoreItemResult, dest: string): [string, string, string] {
  const n = r.done.length + r.kept.length;
  const notWritten = t('chưa ghi được', 'not written');
  if (r.kind === 'replace') {
    if (r.done.length) return ['', r.name + t(' · thay cấu hình', ' · config replaced'), ''];
    if (r.skipped)
      return [
        'w',
        t(r.name + ' · giữ bản trên ' + dest, r.name + ' · kept the version on ' + dest),
        r.skipped === 'noKey' ? t('chưa có khoá', 'no key yet') : t('máy đó chưa cài lệnh của nó', 'that Mac does not have its command'),
      ];
    return ['x', r.name, notWritten];
  }
  if (r.failed.length && !n) return ['x', r.name, notWritten];
  if (r.kept.length && !r.done.length)
    return [
      'w',
      r.name + t(' · thêm nhưng tắt', ' · added but off'),
      r.why === 'noCommand'
        ? t('máy đó chưa cài ' + (r.command ?? 'lệnh của nó'), 'that Mac does not have ' + (r.command ?? 'its command'))
        : r.why === 'noKey'
          ? t(dest + ' sẽ hỏi dán khoá', dest + ' will ask for the key')
          : t('tắt như trên máy gửi', 'off as on the sending Mac'),
    ];
  const failed = r.failed.map((a) => AGENT_NAME[a]).join(', ');
  return [r.failed.length ? 'w' : '', r.name, r.failed.length ? t('chưa ghi được ở ' + failed, 'not written in ' + failed) : ''];
}

/** Every line of the result: the servers, then the skills, plugins and files. */
export function doneLines(done: RestoreDone, dest: string): [string, string, string][] {
  const parts = done.parts ?? [];
  const updated = parts.filter((r) => r.ok && r.part === 'skill' && r.kind === 'update');
  const rest: PartLine[] = partLines(
    parts.filter((r) => !(r.ok && r.kind === 'update')),
    'sender',
  );
  const out: [string, string, string][] = done.items.map((r) => serverLine(r, dest));
  if (updated.length)
    out.push(['', t(updated.length + ' skill lên bản mới nhất', plural(updated.length, 'skill', 'skills') + ' updated to the latest version'), names(updated.map((r) => r.id))]);
  for (const l of rest) out.push([l.tone, l.title, l.sub]);
  if (done.failed) out.push(['x', 'MCP', t('một phần chưa ghi được', 'part was not written')]);
  return out;
}

/** Why a step did not run, in words. */
export function copyError(e: unknown, other: string): FeedbackView {
  const s = String(e);
  if (s === 'timeout' || s.startsWith('unreachable'))
    return {
      tone: 'bad',
      title: t(other + ' không trả lời qua SSH', other + ' is not answering over SSH'),
      body: t(
        'Chờ 20 giây không thấy trả lời. Kiểm tra xem máy đã bật và tên kết nối trong Cài đặt › Các Mac khác.',
        'No answer after 20 seconds. Check that the Mac is on, and its SSH name in Settings › Other Macs.',
      ),
    };
  if (s === 'old')
    return {
      tone: 'warn',
      title: t('Quota Station trên ' + other + ' chưa có phần MCP và skills', 'Quota Station on ' + other + ' does not handle MCP and skills yet'),
      body: t(
        'Máy đó chạy bản 1. Cập nhật app trên máy đó (Cài đặt › Cập nhật từ mã nguồn) rồi so sánh lại.',
        'That Mac runs version 1. Update the app there (Settings › Update from source), then compare again.',
      ),
    };
  if (s === 'noApp')
    return {
      tone: 'warn',
      title: t('Không thấy Quota Station trên ' + other, 'Quota Station not found on ' + other),
      body: t('Cài app vào thư mục Applications của máy đó rồi so sánh lại.', 'Install the app in that Mac’s Applications folder, then compare again.'),
    };
  if (s === 'noSsh')
    return { tone: 'bad', title: t('Không thấy ssh trên máy này', 'ssh not found on this Mac'), body: t('App dùng ssh của macOS để nói chuyện với Mac khác.', 'The app uses the ssh in macOS to talk to other Macs.') };
  if (s === 'unknownMac')
    return { tone: 'warn', title: t('Mac này không còn trong danh sách', 'This Mac is no longer in the list'), body: t('Kiểm tra lại trong Cài đặt › Các Mac khác.', 'Check Settings › Other Macs.') };
  if (s === 'busy' || s === 'refused:busy')
    return { tone: 'warn', title: t('Đang có một lần ghi khác', 'Another write is running'), body: t('Đợi xong rồi thử lại.', 'Wait for it to finish, then try again.') };
  if (s === 'stale')
    return { tone: 'warn', title: t('Cần so sánh lại', 'Compare again'), body: t('Bấm So sánh lại để đọc hai máy rồi chọn mục.', 'Press Compare again to read both Macs, then pick the items.') };
  if (s.startsWith('changed:'))
    return {
      tone: 'warn',
      title: t('Chưa chép: file vừa đổi', 'Not copied: a file just changed'),
      body: t(
        'File của agent trên máy nhận vừa đổi sau lần so sánh. Chưa ghi gì. Bấm So sánh lại.',
        'An agent file on the receiving Mac changed after the comparison. Nothing was written. Press Compare again.',
      ),
    };
  if (s === 'keychain')
    return {
      tone: 'bad',
      title: t('Chưa lưu được khoá', 'Could not save the key'),
      body: t(
        'Keychain trên máy nhận không nhận khoá lúc này. Chưa ghi gì. Thử lại, hoặc tắt Gửi cả khoá.',
        'Keychain on the receiving Mac is not accepting keys right now. Nothing was written. Try again, or turn off Also send keys.',
      ),
    };
  if (s === 'skill')
    return {
      tone: 'bad',
      title: t('Chưa chép', 'Not copied'),
      body: t(
        'Một skill cài tay vừa đổi và có chỗ trông giống khoá. Không có gì rời khỏi máy.',
        'A skill installed by hand just changed and has something that looks like a key. Nothing left this Mac.',
      ),
    };
  if (s === 'waiting')
    return {
      tone: 'warn',
      title: t(other + ' vẫn đang ghi', other + ' is still writing'),
      body: t('Mở tab Sao lưu trên máy đó để xem kết quả, hoặc so sánh lại sau.', 'Open the Backup tab on that Mac to see the result, or compare again later.'),
    };
  if (s === 'lost')
    return {
      tone: 'bad',
      title: t('Lần ghi trên máy nhận dừng giữa chừng', 'The write on the receiving Mac stopped partway'),
      body: t('Mở tab Sao lưu trên máy đó: nếu có lần khôi phục mới thì hoàn tác được ở đó.', 'Open the Backup tab on that Mac: if it shows a new restore, it can be undone there.'),
    };
  if (s === 'gone')
    return {
      tone: 'warn',
      title: t('Không còn gì để hoàn tác', 'Nothing left to undo'),
      body: t('Máy nhận đã hoàn tác, hoặc đã khôi phục một lần khác sau đó.', 'The receiving Mac already undid it, or restored something else since.'),
    };
  if (s === 'protocol' || s === 'refused:protocol')
    return { tone: 'warn', title: t('Hai máy chạy hai bản app khác nhau', 'The two Macs run different app versions'), body: t('Cập nhật Quota Station trên cả hai máy rồi thử lại.', 'Update Quota Station on both Macs, then try again.') };
  return { tone: 'bad', title: t('Chưa chạy được', 'Could not run'), body: t('App không làm được bước này. Không có gì được ghi.', 'The app could not do this step. Nothing was written.') };
}

/** "Đã hoàn tác trên MacBook Air." */
export function undoneFeedback(u: RestoreUndone, dest: string): FeedbackView {
  const n = u.outcome ? u.outcome.done.reduce((sum, d) => sum + d.changes, 0) : 0;
  const said = [
    n ? t('trả ' + n + ' mục về như trước', 'put back ' + plural(n, 'item', 'items')) : '',
    u.kept ? t('bỏ ' + u.kept + ' MCP đang giữ tắt', 'removed ' + u.kept + ' MCP kept off') : '',
    u.keys ? t('xoá ' + u.keys + ' khoá vừa gửi', 'deleted ' + plural(u.keys, 'key', 'keys') + ' just sent') : '',
    u.skills ? t('trả ' + u.skills + ' skill về như trước', 'put back ' + plural(u.skills, 'skill', 'skills')) : '',
    u.plugins ? t('gỡ ' + u.plugins + ' plugin', 'uninstalled ' + plural(u.plugins, 'plugin', 'plugins')) : '',
    u.files ? t('trả ' + u.files + ' file hướng dẫn về như trước', 'put back ' + plural(u.files, 'instruction file', 'instruction files')) : '',
  ]
    .filter(Boolean)
    .join(', ');
  const left = u.left ? t(' Giữ nguyên ' + u.left + ' mục đã sửa sau lần chép.', ' Kept ' + plural(u.left, 'item', 'items') + ' edited after the copy.') : '';
  return {
    tone: 'info',
    title: t('Đã hoàn tác trên ' + dest + '.', 'Undone on ' + dest + '.'),
    body: (said ? said.charAt(0).toUpperCase() + said.slice(1) + '.' : t('Không còn gì để trả về.', 'Nothing left to put back.')) + left,
  };
}

/** A Mac in the list: its name, and how it answered last. */
export function macChip(state: 'ok' | 'unreach' | 'old' | 'noApp' | undefined): ChipView | null {
  if (state === 'ok') return { t: t('SSH ổn', 'SSH OK'), k: 'ok' };
  if (state === 'unreach') return { t: t('không trả lời', 'no answer'), k: 'bad' };
  if (state === 'old') return { t: t('bản cũ', 'old version'), k: 'warn' };
  if (state === 'noApp') return { t: t('chưa có app', 'no app'), k: 'warn' };
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
export const onText = (agents: Agent[]): string => (agents.length ? agents.map((a) => AGENT_NAME[a]).join(', ') : t('không agent nào', 'no agent'));
