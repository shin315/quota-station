// What the Luật xoay vòng tab shows (M5, F06): the changes in the editor,
// the line for each Mac, the banners about the file on disk and the other
// Macs, the feedback after each action, and the table that compares two
// copies. Pure, like `view.ts`. Copy follows the approved prototype
// (`tabRules`, `compareTable`, `leaveDialog`, `fbLine` rules-*).

import { clock, pad } from './format';
import type { Model, PeerView, Rules, RulesError, Stamp } from './model';
import { dayClock, namesOf, type NameOf } from './switching';
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
  return ({ 1: 'máy này', 2: 'cả hai Mac', 3: 'cả ba Mac', 4: 'cả bốn Mac' } as Record<number, string>)[n] ?? n + ' Mac';
}

/** `08:00–24:00`: the file's `8-23` takes hour 23 in, so it ends at 24:00. */
export function hoursText(from: number, to: number): string {
  return pad(from) + ':00–' + pad(to + 1) + ':00';
}

/** `rotation_bad_value:personal_hours` → `khoá personal_hours sai`. */
export function keyWords(reason: string | null): string {
  if (!reason) return 'file không đọc được';
  const [what, key] = reason.split(':');
  if (what === 'rotation_bad_value') return 'khoá ' + key + ' sai';
  if (what === 'rotation_missing_key') return 'thiếu khoá ' + key;
  if (what === 'rotation_malformed_line') return 'có dòng không đọc được';
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

/** The editor's names: the window's (`Team A`), `Tài khoản 5` for a slot it does not list. */
export function ruleNames(model: Model): NameOf {
  const of = namesOf(model);
  return (n) => cap(of(n));
}

/** What changed from `a` (saved) to `b` (the editor), one line each. */
export function ruleChanges(a: Rules, b: Rules, nameOf: NameOf, now: Date): string[] {
  const out: string[] = [];
  const list = (r: Rules) => r.team.map(nameOf).join(', ') || 'không có';
  if (a.team.join() !== b.team.join()) out.push('Tài khoản Team được xoay vòng: ' + list(a) + ' → ' + list(b));
  const pers = (r: Rules) => (r.personal ? nameOf(r.personal) : 'không có');
  if (a.personal !== b.personal) out.push('Tài khoản cá nhân: ' + pers(a) + ' → ' + pers(b));
  if (a.personalFrom !== b.personalFrom || a.personalTo !== b.personalTo)
    out.push('Giờ dùng tài khoản cá nhân: ' + hoursText(a.personalFrom, a.personalTo) + ' → ' + hoursText(b.personalFrom, b.personalTo));
  if (a.full !== b.full) out.push('Ngưỡng tự chuyển: ' + a.full + '% → ' + b.full + '%');
  if (a.fableFull !== b.fableFull) out.push('Ngưỡng Fable tuần: ' + a.fableFull + '% → ' + b.fableFull + '%');
  const slots = [...new Set([...a.expire, ...b.expire].map((e) => e.slot))].sort((x, y) => x - y);
  for (const s of slots) {
    const x = a.expire.find((e) => e.slot === s)?.at;
    const y = b.expire.find((e) => e.slot === s)?.at;
    if (x !== y) out.push('Giờ rời vòng của ' + nameOf(s) + ': ' + (x ? endText(x, now) : 'chưa đặt') + ' → ' + (y ? endText(y, now) : 'chưa đặt'));
  }
  return out;
}

/** Why the rules in the editor cannot be saved yet; null when they can. */
export function rulesProblem(r: Rules): string | null {
  if (!r.team.length) return 'Chọn ít nhất một tài khoản Team.';
  if (r.personalTo < r.personalFrom) return 'Giờ kết thúc phải sau giờ bắt đầu.';
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
  if (p.pushing) return line('muted', 'đang gửi…', 'đang gửi…');
  if (!s) return line('muted', 'chưa gửi lần nào', 'chưa gửi lần nào');
  switch (s.kind) {
    case 'synced':
      return p.current ? line('ok', 'đã nhận ' + clock(s.at), clock(s.at)) : line('warn', 'chưa nhận bản mới', 'chưa nhận bản mới, sẽ gửi');
    case 'unreachable':
      return line('warn', 'chưa tới được · thử lại ' + clock(s.retryAt), 'chưa tới được, sẽ thử lại', s.error);
    case 'mismatch':
      return line('warn', 'đọc lại không khớp · thử lại ' + clock(s.retryAt), 'đọc lại không khớp, sẽ thử lại');
    case 'failed':
      return line('bad', 'không ghi được · thử lại ' + clock(s.retryAt), 'không ghi được, sẽ thử lại', s.error);
    case 'newer':
      return line('info', 'có bản mới hơn ' + clock(s.their.at), 'có bản mới hơn');
    case 'edited':
      return line('info', 'bản đã bị sửa tay', 'bản đã bị sửa tay');
  }
}

/** This Mac in the save bar. */
export function localLine(model: Model, dirty: boolean, restored: boolean): MacLine {
  const r = model.rotation;
  const name = model.machine + ' (máy này)';
  const line = (tone: MacTone, text: string): MacLine => ({ name, tone, text, short: model.machine + ': ' + text, title: '' });
  if (restored) return line('bad', r.stamp ? 'đã khôi phục bản ' + clock(r.stamp.at) : 'đã khôi phục bản trước');
  if (dirty) return line('warn', 'chưa lưu');
  if (r.status === 'missing') return line('muted', 'chưa có file');
  if (r.status !== 'ok') return line('bad', 'file không hợp lệ');
  return line('ok', r.stamp ? 'lưu ' + clock(r.stamp.at) : 'chưa lưu bằng app');
}

/** `Mac B: 14:30 · Mac C: chưa tới được, sẽ thử lại`, with the worst Mac's tone; null with no other Mac. */
export function syncSummary(model: Model): { tone: MacTone; text: string } | null {
  const lines = model.rotation.peers.map(peerLine);
  if (!lines.length) return null;
  const tone = (['bad', 'warn', 'info', 'muted'] as const).find((t) => lines.some((l) => l.tone === t)) ?? 'ok';
  return { tone, text: lines.map((l) => l.short).join(' · ') };
}

/** The sidebar's line under the machine name. */
export function sidebarLine(model: Model): string | null {
  const peers = model.rotation.peers;
  if (!peers.length || !model.rotation.stamp) return null;
  const behind = peers.filter((p) => !(p.current && p.state?.kind === 'synced'));
  if (!behind.length) return 'luật đã gửi tới ' + peers.length + ' Mac khác lúc ' + clock(model.rotation.stamp.at);
  return behind.length + ' Mac chưa nhận luật mới';
}

// ===== Banners about the file and the other Macs =====

/** The banner the file on disk or another Mac calls for; null when all is in step. */
export function rotationBanner(model: Model): BannerView | null {
  const r = model.rotation;
  if (r.status === 'missing')
    return {
      tone: 'info',
      title: 'Chưa có file luật trên máy này',
      body: 'Switcher đang chạy theo mặc định của nó. Tạo file từ mặc định để sửa được.',
      actions: [{ label: 'Tạo từ mặc định', act: 'rules:create' }],
    };
  if (r.status === 'unreadable')
    return { tone: 'bad', title: 'Không đọc được file luật trên máy này', body: r.reason ?? '', actions: [] };
  if (r.status === 'invalid')
    return {
      tone: 'bad',
      title: 'quota-axi không đọc được file luật trên máy này',
      body:
        cap(keyWords(r.reason)) +
        '. quota-axi đang tính theo tài khoản đang dùng. Bộ sửa hiện mặc định của switcher: lưu để ghi đè, bản cũ được sao lưu.',
      actions: [],
    };
  const at = r.stamp ? clock(r.stamp.at) : '';
  const other = r.peers.find(conflict);
  if (other && other.state && (other.state.kind === 'newer' || other.state.kind === 'edited')) {
    const newer = other.state.kind === 'newer';
    return {
      tone: 'info',
      title: newer ? 'Bản trên ' + other.name + ' mới hơn (' + clock(other.state.their.at) + ')' : 'Bản trên ' + other.name + ' đã bị sửa tay',
      body:
        (newer
          ? 'Có người sửa luật trên ' + other.name + ' sau bản ' + at + ' của máy này.'
          : 'File luật trên ' + other.name + ' khác bản ' + at + ' của máy này dù cùng dấu thời gian.') + ' App không tự gộp hai bản. Tải bản đó về?',
      actions: [
        { label: 'Dùng bản trên ' + other.name, act: ('rules:adopt:' + other.alias) as `rules:${string}` },
        { label: 'So sánh', act: ('rules:compare:' + other.alias) as `rules:${string}` },
      ],
    };
  }
  const behind = r.peers.filter((p) => p.current && failing(p) && !p.pushing);
  if (behind.length && r.stamp) {
    const fine = r.peers.filter((p) => p.current && p.state?.kind === 'synced');
    return {
      tone: 'warn',
      title: 'Chưa gửi được luật tới ' + behind.map((p) => p.name).join(', '),
      body: 'Đã lưu trên máy này' + (fine.length ? ' và ' + fine.map((p) => p.name).join(', ') : '') + ' lúc ' + at + '. App thử lại mỗi 5 phút.',
      actions: [{ label: 'Thử lại', act: 'rules:retry' }],
    };
  }
  return null;
}

/** The file changed on disk while the editor held changes (two Macs editing at once). */
export function changedBanner(model: Model): BannerView {
  const s = model.rotation.stamp;
  const by = s && s.machine !== model.machine ? s.machine : null;
  return {
    tone: 'info',
    title: by ? 'Bản trên ' + by + ' mới hơn, tải lại?' : 'File luật vừa đổi trên máy này',
    body:
      'Trong lúc bạn sửa, file luật được ghi lại' +
      (s ? ' lúc ' + clock(s.at) : '') +
      (by ? ' bằng bản của ' + by : '') +
      '. App không tự gộp hai bản: tải lại để xem bản đó, hoặc giữ bản đang sửa.',
    actions: [
      { label: 'Tải lại', act: 'rules:reload' },
      { label: 'Giữ bản đang sửa', act: 'rules:overwrite' },
    ],
  };
}

/** What a failed save, adopt or keep says. `before` is the stamp of the file before. */
export function rulesErrorBanner(e: RulesError, before: Stamp | null, peer?: string): BannerView {
  const back = e.restored ? 'Đã khôi phục bản ' + (before ? clock(before.at) : 'trước') + ' trên máy này.' : 'Chưa khôi phục được bản trước, xem file trên máy này.';
  const dismiss = [{ label: 'Sửa lại', act: 'rules:dismiss' as const }];
  switch (e.kind) {
    case 'refused':
      return { tone: 'bad', title: 'quota-axi từ chối luật vừa lưu', body: cap(keyWords(e.detail)) + '. ' + back + ' Chưa gửi sang Mac khác.', actions: dismiss };
    case 'unchecked':
      return {
        tone: 'bad',
        title: 'Chưa kiểm được luật vừa lưu',
        body: 'quota-axi không chạy được' + (e.detail ? ' (' + e.detail + ')' : '') + '. ' + back + ' Chưa gửi sang Mac khác.',
        actions: dismiss,
      };
    case 'invalid':
      return { tone: 'bad', title: 'Luật chưa hợp lệ', body: cap(keyWords(e.detail)) + '. Chưa ghi gì.', actions: dismiss };
    case 'unreachable':
    case 'noFile':
    case 'unknownMac':
      return {
        tone: 'warn',
        title: 'Chưa tải được bản trên ' + (peer ?? 'Mac kia'),
        body: e.kind === 'noFile' ? 'Máy đó không có file luật.' : 'SSH không tới được' + (e.detail ? ': ' + e.detail : '') + '. Máy này giữ nguyên.',
        actions: [],
      };
    default:
      return { tone: 'bad', title: 'Không ghi được file luật', body: (e.detail ?? e.kind) + '. Máy này giữ nguyên.', actions: dismiss };
  }
}

// ===== Feedback after an action =====

export function saveFeedback(model: Model): FeedbackView {
  const r = model.rotation;
  const at = r.stamp ? clock(r.stamp.at) : '';
  const peers = r.peers;
  const behind = peers.filter((p) => !(p.current && p.state?.kind === 'synced'));
  if (!behind.length)
    return {
      tone: '',
      title: 'Đã lưu luật cho ' + macsWord(1 + peers.length) + ' lúc ' + at,
      body: 'quota-axi đọc lại: đã kiểm tra đủ 6 mục của luật ✓',
    };
  return {
    tone: 'warn',
    title: 'Đã lưu luật trên máy này lúc ' + at,
    body: 'quota-axi đã kiểm tra đủ 6 mục ✓. ' + behind.map((p) => peerLine(p).short).join(' · ') + '.',
  };
}

export function adoptFeedback(peer: string, their: Stamp | null, now: Date): FeedbackView {
  return {
    tone: '',
    title: 'Đã dùng bản luật của ' + peer + (their ? ' (' + clock(their.at) + ')' : ''),
    body: 'Đã ghi lên máy này và gửi tới các Mac còn lại lúc ' + clock(now.toISOString()) + '.',
  };
}

export function keepFeedback(model: Model, peer: string): FeedbackView {
  const at = model.rotation.stamp ? clock(model.rotation.stamp.at) : '';
  return {
    tone: '',
    title: 'Đã giữ bản của máy này',
    body: 'Đã gửi bản ' + at + ' tới ' + peer + '. Bản của ' + peer + ' được sao lưu trên máy đó.',
  };
}

export function retryFeedback(model: Model, now: Date): FeedbackView {
  const behind = model.rotation.peers.filter((p) => !(p.current && p.state?.kind === 'synced'));
  if (!behind.length)
    return {
      tone: '',
      title: 'Đã gửi luật tới các Mac khác lúc ' + clock(now.toISOString()),
      body: macsWord(1 + model.rotation.peers.length) + ' giờ dùng cùng một bản luật.',
    };
  return { tone: 'warn', title: 'Vẫn chưa gửi được', body: behind.map((p) => peerLine(p).short).join(' · ') + '. App thử lại mỗi 5 phút.' };
}

// ===== Two copies side by side =====

export interface CompareRow {
  label: string;
  mine: string;
  theirs: string;
  diff: boolean;
}

export function compareRows(mine: Rules, theirs: Rules, nameOf: NameOf, now: Date): CompareRow[] {
  const team = (r: Rules) => r.team.map(nameOf).join(', ') || 'không có';
  const pers = (r: Rules) => (r.personal ? nameOf(r.personal) : 'không có');
  const ends = (r: Rules) =>
    [...r.expire]
      .sort((a, b) => a.slot - b.slot)
      .map((e) => nameOf(e.slot) + ' ' + endText(e.at, now))
      .join(', ') || 'chưa đặt';
  const rows: [string, (r: Rules) => string][] = [
    ['Tài khoản Team được xoay vòng', team],
    ['Tài khoản cá nhân', pers],
    ['Giờ dùng tài khoản cá nhân', (r) => hoursText(r.personalFrom, r.personalTo)],
    ['Ngưỡng tự chuyển', (r) => r.full + '%'],
    ['Ngưỡng Fable tuần', (r) => r.fableFull + '%'],
    ['Giờ rời vòng', ends],
  ];
  return rows.map(([label, f]) => ({ label, mine: f(mine), theirs: f(theirs), diff: f(mine) !== f(theirs) }));
}
