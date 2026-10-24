// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the Luật xoay vòng tab shows (M5, F06): the changes in the editor,
// the line for each Mac, the banners about the file on disk and the other
// Macs, the feedback after each action, and the table that compares two
// copies. Pure, like `view.ts`. Copy follows the approved prototype
// (`tabRules`, `compareTable`, `leaveDialog`, `fbLine` rules-*).

import { claudeAliases, claudeNames } from './accounts';
import { clock, pad } from './format';
import { t } from './i18n';
import type { AuthorModel } from './author';
import type { Model, PeerView, Rules, RulesError, Stamp } from './model';
import { dayClock, type NameOf } from './switching';
import type { BannerView, FeedbackView } from './view';

export function cloneRules(r: Rules): Rules {
  return { ...r, team: [...r.team], expire: r.expire.map((e) => ({ ...e })) };
}

function normal(r: Rules): string {
  const expire = [...r.expire].sort((a, b) => a.slot - b.slot);
  return JSON.stringify([r.team, r.personal, r.personalFrom, r.personalTo, r.full, r.fableFull, expire.map((e) => [e.slot, e.at])]);
}

export function sameRules(a: Rules, b: Rules): boolean {
  return normal(a) === normal(b);
}

/** `cả ba Mac`: this Mac and the others. */
export function macsWord(n: number): string {
  const vi = ({ 1: 'máy này', 2: 'cả hai Mac', 3: 'cả ba Mac', 4: 'cả bốn Mac' } as Record<number, string>)[n] ?? n + ' Mac';
  const en = ({ 1: 'this Mac', 2: 'both Macs', 3: 'all three Macs', 4: 'all four Macs' } as Record<number, string>)[n] ?? 'all ' + n + ' Macs';
  return t(vi, en);
}

/** `08:00–24:00`: the file's `8-23` takes hour 23 in, so it ends at 24:00. */
export function hoursText(from: number, to: number): string {
  return pad(from) + ':00–' + pad(to + 1) + ':00';
}

/** `rotation_bad_value:personal_hours` → `khoá personal_hours sai`. */
export function keyWords(reason: string | null): string {
  if (!reason) return t('file không đọc được', 'the file cannot be read');
  const [what, key] = reason.split(':');
  if (what === 'rotation_bad_value') return t('khoá ' + key + ' sai', 'key ' + key + ' has a bad value');
  if (what === 'rotation_missing_key') return t('thiếu khoá ' + key, 'key ' + key + ' is missing');
  if (what === 'rotation_malformed_line') return t('có dòng không đọc được', 'a line cannot be read');
  if (what === 'rotation_personal_in_team') return t('tài khoản cá nhân cũng nằm trong vòng Team', 'the personal account is also in the Team rotation');
  return reason;
}

function cap(s: string): string {
  return s ? s.charAt(0).toUpperCase() + s.slice(1) : s;
}

/** An end time as written (`2026-01-16T17:45`, local) in the window's words. */
export function endText(at: string, now: Date): string {
  return Number.isNaN(new Date(at).getTime()) ? at : dayClock(at, now);
}

/** Every slot the editor offers: claude-swap's accounts and any slot the rules name. */
export function slotsOf(model: Model, ...rules: (Rules | null)[]): number[] {
  const out = new Set<number>(model.snapshot.claude.accounts.map((a) => a.number).filter((n) => n > 0));
  for (const r of rules) {
    if (!r) continue;
    r.team.forEach((n) => out.add(n));
    if (r.personal) out.add(r.personal);
  }
  return [...out].sort((a, b) => a - b);
}

/**
 * The editor's names: the window's (`lan.acme1`, or `Team A` while names
 * are hidden), `Tài khoản 5` for a slot it does not list.
 */
export function ruleNames(model: Model): NameOf {
  const names = claudeNames(model.snapshot.claude.accounts, model.snapshot.claude.personalHours, model.advanced.on, claudeAliases(model));
  return (n) => names.get(n)?.name ?? t('Tài khoản ', 'Account ') + n;
}

/** The whole email of each slot, for hover; '' while names are hidden. */
export function ruleEmails(model: Model): (slot: number) => string {
  const names = claudeNames(model.snapshot.claude.accounts);
  return (n) => names.get(n)?.full ?? '';
}

/** Where a mark sits on a slider from `min` to `max`, in percent of its width. */
export function markLeft(value: number, min: number, max: number): number {
  return max > min ? Math.round(((value - min) / (max - min)) * 10000) / 100 : 0;
}

/** What changed from `a` (saved) to `b` (the editor), one line each. */
export function ruleChanges(a: Rules, b: Rules, nameOf: NameOf, now: Date): string[] {
  const out: string[] = [];
  const list = (r: Rules) => r.team.map(nameOf).join(', ') || t('không có', 'none');
  if (a.team.join() !== b.team.join()) out.push(t('Tài khoản Team được xoay vòng: ', 'Team accounts in rotation: ') + list(a) + ' → ' + list(b));
  const pers = (r: Rules) => (r.personal ? nameOf(r.personal) : t('không có', 'none'));
  if (a.personal !== b.personal) out.push(t('Tài khoản cá nhân: ', 'Personal account: ') + pers(a) + ' → ' + pers(b));
  if (a.personalFrom !== b.personalFrom || a.personalTo !== b.personalTo)
    out.push(t('Giờ dùng tài khoản cá nhân: ', 'Personal account hours: ') + hoursText(a.personalFrom, a.personalTo) + ' → ' + hoursText(b.personalFrom, b.personalTo));
  if (a.full !== b.full) out.push(t('Ngưỡng tự chuyển: ', 'Auto-switch threshold: ') + a.full + '% → ' + b.full + '%');
  if (a.fableFull !== b.fableFull) out.push(t('Ngưỡng Fable tuần: ', 'Fable week threshold: ') + a.fableFull + '% → ' + b.fableFull + '%');
  const slots = [...new Set([...a.expire, ...b.expire].map((e) => e.slot))].sort((x, y) => x - y);
  for (const s of slots) {
    const x = a.expire.find((e) => e.slot === s)?.at;
    const y = b.expire.find((e) => e.slot === s)?.at;
    const notSet = t('chưa đặt', 'not set');
    if (x !== y)
      out.push(t('Giờ rời vòng của ' + nameOf(s) + ': ', nameOf(s) + ' leaves rotation at: ') + (x ? endText(x, now) : notSet) + ' → ' + (y ? endText(y, now) : notSet));
  }
  return out;
}

/** Why the rules in the editor cannot be saved yet; null when they can. */
export function rulesProblem(r: Rules): string | null {
  if (!r.team.length) return t('Chọn ít nhất một tài khoản Team.', 'Pick at least one Team account.');
  if (r.personalTo < r.personalFrom) return t('Giờ kết thúc phải sau giờ bắt đầu.', 'The end hour must come after the start hour.');
  if (r.personal !== null && r.team.includes(r.personal)) return t('Tài khoản cá nhân không được nằm trong vòng Team.', 'The personal account cannot be in the Team rotation.');
  return null;
}

// ===== One line per Mac =====

export type MacTone = 'ok' | 'warn' | 'bad' | 'info' | 'muted';

export interface MacLine {
  name: string;
  tone: MacTone;
  /** After the name in the save bar: `đã nhận 14:30`. */
  text: string;
  /** The status line: `Mac B: 14:30`, `Mac C: chưa tới được, sẽ thử lại`. */
  short: string;
  /** Hover text: the error SSH gave. */
  title: string;
}

/** Waits for the owner rather than for the next try. */
function conflict(p: PeerView): boolean {
  return p.current && (p.state?.kind === 'newer' || p.state?.kind === 'edited');
}

/** Tried and not there yet; the app tries again. */
function failing(p: PeerView): boolean {
  const k = p.state?.kind;
  return k === 'unreachable' || k === 'mismatch' || k === 'failed';
}

export function peerLine(p: PeerView): MacLine {
  const line = (tone: MacTone, text: string, short: string, title = ''): MacLine => ({ name: p.name, tone, text, short: p.name + ': ' + short, title });
  const s = p.state;
  if (p.pushing) return line('muted', t('đang gửi…', 'sending…'), t('đang gửi…', 'sending…'));
  if (!s) return line('muted', t('chưa gửi lần nào', 'never sent'), t('chưa gửi lần nào', 'never sent'));
  switch (s.kind) {
    case 'synced':
      return p.current
        ? line('ok', t('đã nhận ' + clock(s.at), 'received ' + clock(s.at)), clock(s.at))
        : line('warn', t('chưa nhận bản mới', 'new rules not received yet'), t('chưa nhận bản mới, sẽ gửi', 'new rules not received yet, will send'));
    case 'unreachable':
      return line('warn', t('chưa tới được · thử lại ' + clock(s.retryAt), 'unreachable · retrying at ' + clock(s.retryAt)), t('chưa tới được, sẽ thử lại', 'unreachable, will retry'), s.error);
    case 'mismatch':
      return line(
        'warn',
        t('đọc lại không khớp · thử lại ' + clock(s.retryAt), 'read-back did not match · retrying at ' + clock(s.retryAt)),
        t('đọc lại không khớp, sẽ thử lại', 'read-back did not match, will retry'),
      );
    case 'failed':
      return line('bad', t('không ghi được · thử lại ' + clock(s.retryAt), 'could not write · retrying at ' + clock(s.retryAt)), t('không ghi được, sẽ thử lại', 'could not write, will retry'), s.error);
    case 'newer':
      return line('info', t('có bản mới hơn ' + clock(s.their.at), 'has a newer copy from ' + clock(s.their.at)), t('có bản mới hơn', 'has a newer copy'));
    case 'edited':
      return line('info', t('bản đã bị sửa tay', 'copy edited by hand'), t('bản đã bị sửa tay', 'copy edited by hand'));
  }
}

/** This Mac in the save bar. */
export function localLine(model: AuthorModel, dirty: boolean, restored: boolean): MacLine {
  const r = model.rotation;
  const name = model.machine + t(' (máy này)', ' (this Mac)');
  const line = (tone: MacTone, text: string): MacLine => ({ name, tone, text, short: model.machine + ': ' + text, title: '' });
  if (restored) return line('bad', r.stamp ? t('đã khôi phục bản ' + clock(r.stamp.at), 'restored the ' + clock(r.stamp.at) + ' copy') : t('đã khôi phục bản trước', 'restored the previous copy'));
  if (dirty) return line('warn', t('chưa lưu', 'not saved'));
  if (r.status === 'missing') return line('muted', t('chưa có file', 'no file yet'));
  if (r.status === 'unreadable') return line('bad', t('không đọc được file', 'cannot read the file'));
  if (r.status !== 'ok') return line('bad', t('file không hợp lệ', 'invalid file'));
  return line('ok', r.stamp ? t('lưu ' + clock(r.stamp.at), 'saved ' + clock(r.stamp.at)) : t('chưa lưu bằng app', 'not saved by the app yet'));
}

/** `Mac B: 14:30 · Mac C: chưa tới được, sẽ thử lại`, with the worst Mac's tone; null with no other Mac. */
export function syncSummary(model: AuthorModel): { tone: MacTone; text: string } | null {
  const lines = model.rotation.peers.map(peerLine);
  if (!lines.length) return null;
  const tone = (['bad', 'warn', 'info', 'muted'] as const).find((t) => lines.some((l) => l.tone === t)) ?? 'ok';
  return { tone, text: lines.map((l) => l.short).join(' · ') };
}

/** The sidebar's line under the machine name. */
export function sidebarLine(model: AuthorModel): string | null {
  const peers = model.rotation.peers;
  if (!peers.length || !model.rotation.stamp) return null;
  const behind = peers.filter((p) => !(p.current && p.state?.kind === 'synced'));
  if (!behind.length)
    return t(
      'luật đã gửi tới ' + peers.length + ' Mac khác lúc ' + clock(model.rotation.stamp.at),
      'rules sent to ' + peers.length + (peers.length === 1 ? ' other Mac' : ' other Macs') + ' at ' + clock(model.rotation.stamp.at),
    );
  return t(
    behind.length + ' Mac chưa nhận luật mới',
    behind.length + (behind.length === 1 ? ' Mac has' : ' Macs have') + ' not received the new rules',
  );
}

// ===== Banners about the file and the other Macs =====

/** The banner the file on disk or another Mac calls for; null when all is in step. */
export function rotationBanner(model: AuthorModel): BannerView | null {
  const r = model.rotation;
  if (r.status === 'missing')
    return {
      tone: 'info',
      title: t('Chưa có file luật trên máy này', 'No rules file on this Mac yet'),
      body: t(
        'Switcher đang chạy theo mặc định của nó. Tạo file từ mặc định để sửa được.',
        'The switcher runs on its own defaults. Create the file from those defaults to edit it.',
      ),
      actions: [{ label: t('Tạo từ mặc định', 'Create from defaults'), act: 'rules:create' }],
    };
  if (r.status === 'unreadable')
    return { tone: 'bad', title: t('Không đọc được file luật trên máy này', 'Cannot read the rules file on this Mac'), body: r.reason ?? '', actions: [] };
  if (r.status === 'invalid')
    return {
      tone: 'bad',
      title: t('File luật trên máy này không hợp lệ', 'The rules file on this Mac is not valid'),
      body:
        cap(keyWords(r.reason)) +
        t(
          '. App đang tính theo tài khoản đang dùng. Bộ sửa hiện mặc định của switcher: lưu để ghi đè, bản cũ được sao lưu.',
          '. The app goes by the account in use. The editor shows the switcher’s defaults: save to overwrite; the old copy is backed up.',
        ),
      actions: [],
    };
  const at = r.stamp ? clock(r.stamp.at) : '';
  const other = r.peers.find(conflict);
  if (other && other.state && (other.state.kind === 'newer' || other.state.kind === 'edited')) {
    const newer = other.state.kind === 'newer' ? other.state.their : null;
    return {
      tone: 'info',
      title: newer
        ? t('Bản trên ' + other.name + ' mới hơn (' + clock(newer.at) + ')', 'The copy on ' + other.name + ' is newer (' + clock(newer.at) + ')')
        : t('Bản trên ' + other.name + ' đã bị sửa tay', 'The copy on ' + other.name + ' was edited by hand'),
      body:
        (newer
          ? t('Có người sửa luật trên ' + other.name + ' sau bản ' + at + ' của máy này.', 'Someone edited the rules on ' + other.name + ' after this Mac’s ' + at + ' copy.')
          : other.state.their
            ? t(
                'File luật trên ' + other.name + ' khác bản ' + at + ' của máy này dù cùng dấu thời gian.',
                'The rules file on ' + other.name + ' differs from this Mac’s ' + at + ' copy, though both have the same timestamp.',
              )
            : t(
                'File luật trên ' + other.name + ' không có dòng updated của app, hoặc app không đọc được nó; có thể đã được sửa tay. App không ghi đè.',
                'The rules file on ' + other.name + ' has no updated line from the app, or the app cannot read it; it may have been edited by hand. The app does not overwrite it.',
              )) + t(' App không tự gộp hai bản. Tải bản đó về?', ' The app does not merge the two copies. Fetch that copy?'),
      actions: [
        // A copy the app cannot read cannot be taken; the compare sheet still offers keeping this one.
        ...(other.theirRules ? [{ label: t('Dùng bản trên ' + other.name, 'Use the copy on ' + other.name), act: `rules:adopt:${other.alias}` as const }] : []),
        { label: t('So sánh', 'Compare'), act: `rules:compare:${other.alias}` },
      ],
    };
  }
  const behind = r.peers.filter((p) => p.current && failing(p) && !p.pushing);
  if (behind.length && r.stamp) {
    const fine = r.peers.filter((p) => p.current && p.state?.kind === 'synced');
    return {
      tone: 'warn',
      title: t('Chưa gửi được luật tới ', 'Could not send the rules to ') + behind.map((p) => p.name).join(', '),
      body: t(
        'Đã lưu trên máy này' + (fine.length ? ' và ' + fine.map((p) => p.name).join(', ') : '') + ' lúc ' + at + '. App thử lại mỗi 5 phút.',
        'Saved on this Mac' + (fine.length ? ' and ' + fine.map((p) => p.name).join(', ') : '') + ' at ' + at + '. The app tries again every 5 minutes.',
      ),
      actions: [{ label: t('Thử lại', 'Try again'), act: 'rules:retry' }],
    };
  }
  return null;
}

/** The file changed on disk while the editor held changes (two Macs editing at once). */
export function changedBanner(model: AuthorModel): BannerView {
  const s = model.rotation.stamp;
  const by = s && s.machine !== model.machine ? s.machine : null;
  return {
    tone: 'info',
    title: by ? t('Bản trên ' + by + ' mới hơn, tải lại?', 'The copy on ' + by + ' is newer. Reload?') : t('File luật vừa đổi trên máy này', 'The rules file just changed on this Mac'),
    body: t(
      'Trong lúc bạn sửa, file luật được ghi lại' +
        (s ? ' lúc ' + clock(s.at) : '') +
        (by ? ' bằng bản của ' + by : '') +
        '. App không tự gộp hai bản: tải lại để xem bản đó, hoặc giữ bản đang sửa.',
      'While you were editing, the rules file was written again' +
        (s ? ' at ' + clock(s.at) : '') +
        (by ? ' with the copy from ' + by : '') +
        '. The app does not merge the two copies: reload to see that copy, or keep the one you are editing.',
    ),
    actions: [
      { label: t('Tải lại', 'Reload'), act: 'rules:reload' },
      { label: t('Giữ bản đang sửa', 'Keep my edits'), act: 'rules:overwrite' },
    ],
  };
}

/** What a failed save, adopt or keep says. */
export function rulesErrorBanner(e: RulesError, peer?: string): BannerView {
  const dismiss = [{ label: t('Sửa lại', 'Edit'), act: 'rules:dismiss' as const }];
  const unchanged = t('. Máy này giữ nguyên.', '. This Mac is unchanged.');
  switch (e.kind) {
    case 'invalid':
      return { tone: 'bad', title: t('Luật chưa hợp lệ', 'The rules are not valid'), body: cap(keyWords(e.detail)) + t('. Chưa ghi gì.', '. Nothing was written.'), actions: dismiss };
    case 'unreachable':
    case 'noFile':
    case 'unknownMac':
      return {
        tone: 'warn',
        title: t('Chưa tải được bản trên ', 'Could not fetch the copy on ') + (peer ?? t('Mac kia', 'the other Mac')),
        body:
          e.kind === 'noFile'
            ? t('Máy đó không có file luật.', 'That Mac has no rules file.')
            : e.kind === 'unknownMac'
              ? t(
                  'Mac này không còn trong Cài đặt › Các Mac khác. Thêm lại rồi thử. Máy này giữ nguyên.',
                  'That Mac is no longer in Settings › Other Macs. Add it back, then try again. This Mac is unchanged.',
                )
              : t('SSH không tới được', 'SSH could not reach it') + (e.detail ? ': ' + e.detail : '') + unchanged,
        actions: [],
      };
    default:
      return { tone: 'bad', title: t('Không ghi được file luật', 'Could not write the rules file'), body: (e.detail ?? e.kind) + unchanged, actions: dismiss };
  }
}

// ===== Feedback after an action =====

export function saveFeedback(model: AuthorModel): FeedbackView {
  const r = model.rotation;
  const at = r.stamp ? clock(r.stamp.at) : '';
  const peers = r.peers;
  const behind = peers.filter((p) => !(p.current && p.state?.kind === 'synced'));
  if (!behind.length)
    return {
      tone: '',
      title: t('Đã lưu luật cho ' + macsWord(1 + peers.length) + ' lúc ' + at, 'Saved the rules for ' + macsWord(1 + peers.length) + ' at ' + at),
      body: t('Đã kiểm tra đủ 6 mục của luật ✓', 'Checked all 6 rule items ✓'),
    };
  return {
    tone: 'warn',
    title: t('Đã lưu luật trên máy này lúc ', 'Saved the rules on this Mac at ') + at,
    body: t('Đã kiểm tra đủ 6 mục ✓. ', 'Checked all 6 items ✓. ') + behind.map((p) => peerLine(p).short).join(' · ') + '.',
  };
}

export function adoptFeedback(peer: string, their: Stamp | null, now: Date): FeedbackView {
  return {
    tone: '',
    title: t('Đã dùng bản luật của ', 'Now using the rules from ') + peer + (their ? ' (' + clock(their.at) + ')' : ''),
    body: t('Đã ghi lên máy này và gửi tới các Mac còn lại lúc ', 'Written to this Mac and sent to the other Macs at ') + clock(now.toISOString()) + '.',
  };
}

export function keepFeedback(model: AuthorModel, peer: string): FeedbackView {
  const at = model.rotation.stamp ? clock(model.rotation.stamp.at) : '';
  return {
    tone: '',
    title: t('Đã giữ bản của máy này', 'Kept this Mac’s copy'),
    body: t(
      'Đã gửi bản ' + at + ' tới ' + peer + '. Bản của ' + peer + ' được sao lưu trên máy đó.',
      'Sent the ' + at + ' copy to ' + peer + '. ' + peer + '’s copy is backed up on that Mac.',
    ),
  };
}

export function retryFeedback(model: AuthorModel, now: Date): FeedbackView {
  const behind = model.rotation.peers.filter((p) => !(p.current && p.state?.kind === 'synced'));
  if (!behind.length)
    return {
      tone: '',
      title: t('Đã gửi luật tới các Mac khác lúc ', 'Sent the rules to the other Macs at ') + clock(now.toISOString()),
      body: t(
        macsWord(1 + model.rotation.peers.length) + ' giờ dùng cùng một bản luật.',
        model.rotation.peers.length ? cap(macsWord(1 + model.rotation.peers.length)) + ' now use the same rules.' : 'This Mac now uses the same rules.',
      ),
    };
  return {
    tone: 'warn',
    title: t('Vẫn chưa gửi được', 'Still not sent'),
    body: behind.map((p) => peerLine(p).short).join(' · ') + t('. App thử lại mỗi 5 phút.', '. The app tries again every 5 minutes.'),
  };
}

// ===== Two copies side by side =====

export interface CompareRow {
  label: string;
  mine: string;
  theirs: string;
  diff: boolean;
}

export function compareRows(mine: Rules, theirs: Rules, nameOf: NameOf, now: Date): CompareRow[] {
  const team = (r: Rules) => r.team.map(nameOf).join(', ') || t('không có', 'none');
  const pers = (r: Rules) => (r.personal ? nameOf(r.personal) : t('không có', 'none'));
  const ends = (r: Rules) =>
    [...r.expire]
      .sort((a, b) => a.slot - b.slot)
      .map((e) => nameOf(e.slot) + ' ' + endText(e.at, now))
      .join(', ') || t('chưa đặt', 'not set');
  const rows: [string, (r: Rules) => string][] = [
    [t('Tài khoản Team được xoay vòng', 'Team accounts in rotation'), team],
    [t('Tài khoản cá nhân', 'Personal account'), pers],
    [t('Giờ dùng tài khoản cá nhân', 'Personal account hours'), (r) => hoursText(r.personalFrom, r.personalTo)],
    [t('Ngưỡng tự chuyển', 'Auto-switch threshold'), (r) => r.full + '%'],
    [t('Ngưỡng Fable tuần', 'Fable week threshold'), (r) => r.fableFull + '%'],
    [t('Giờ rời vòng', 'Leaves rotation at'), ends],
  ];
  return rows.map(([label, f]) => ({ label, mine: f(mine), theirs: f(theirs), diff: f(mine) !== f(theirs) }));
}
