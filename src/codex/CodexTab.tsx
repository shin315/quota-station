// The Codex tab of the main window (M8) and Codex's compact card on Tổng
// quan. The tab reads like the Claude and Antigravity tabs: one table with
// the two sides (each "Chuyển" opens the confirmation sheet), "Dán khoá…"
// for the custom provider and the plugins box under it, the technical
// detail behind "Chi tiết"; then Codex on this Mac and the folded guide to
// stop using CC Switch.

import type { ReactNode } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { AccountName, IdMark, InUse } from '../components/accounts';
import { CheckLine } from '../components/blocks';
import { Empty, GRID2, TABLE_ROW, TABLE_ROW_ON, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, BannerBox, Button, Chip, CODE, Dot, Feedback, KVT, LinkButton } from '../components/parts';
import {
  CHATGPT_ID,
  ccSwitchManages,
  codexBanner,
  codexLine,
  currentId,
  externalBanner,
  hasLogin,
  keyMode,
  PLUGINS_HINT,
  pluginsNeedApply,
  PLUGINS_LABEL,
  recentText,
  refusalText,
  shortName,
  undoLabel,
} from '../lib/codex';
import type { CodexView, Model } from '../lib/model';
import { dayClock } from '../lib/switching';
import type { Action, BannerView, ChipView, FeedbackView, PopoverView } from '../lib/view';

/** Days of running the switch here before the CC Switch guide's next steps open. */
export const STEADY_DAYS = 3;

function Card(props: { title: ReactNode; id?: string; x?: boolean; children: ReactNode }) {
  return (
    <Panel id={props.id}>
      <CardTitle>
        {props.x ? <IdMark id="x" /> : null}
        {props.title}
      </CardTitle>
      {props.children}
    </Panel>
  );
}

/** A line with its button at the far end. */
const ROW2 = 'flex items-center justify-between gap-2.5';
const DETAILS = 'mt-2 text-sm text-ink2';
const SUMMARY = 'cursor-pointer list-none text-accent-ink [&::-webkit-details-marker]:hidden';

/** A note at the end of a card; red or amber when it warns. */
function Note({ tone, children }: { tone: string; children: ReactNode }) {
  return <CardText className={cn('mt-2 mb-0', tone === 'warn' ? 'text-warn-ink' : tone === 'bad' ? 'text-bad-ink' : undefined)}>{children}</CardText>;
}

function Rows({ rows }: { rows: { key: ReactNode; value: ReactNode; id: string }[] }) {
  return <div className={KVT}>{rows.flatMap((r) => [<span key={r.id + ':k'}>{r.key}</span>, <span key={r.id + ':v'}>{r.value}</span>])}</div>;
}

function loginChip(v: CodexView): ChipView {
  switch (v.login) {
    case 'chatGpt':
      return { t: 'còn trong máy', k: 'ok' };
    case 'keychain':
      return { t: 'trong Keychain', k: 'info', title: 'Codex giữ đăng nhập trong Keychain; app không mở Keychain để kiểm.' };
    case 'other':
      return { t: 'đăng nhập khác', k: 'warn' };
    case 'apiKey':
      return { t: 'chỉ có khoá API', k: 'warn' };
    case 'unreadable':
      return { t: 'không đọc được', k: 'bad' };
    case 'ephemeral':
      return { t: 'không lưu', k: 'warn' };
    default:
      return { t: 'chưa có', k: 'warn' };
  }
}

function keyChip(v: CodexView): ChipView {
  const c = v.custom;
  if (!c) return { t: 'không có bảng', k: 'bad' };
  switch (c.key) {
    case 'bearer':
      return { t: 'có · experimental_bearer_token', k: 'ok' };
    case 'envKey':
      return { t: 'có · biến ' + (c.envKey ?? ''), k: 'ok' };
    case 'headers':
      return { t: 'có · header', k: 'ok' };
    case 'placeholder':
      return { t: 'chỉ khoá giữ chỗ của CC Switch', k: 'bad' };
    default:
      return { t: 'thiếu', k: 'bad' };
  }
}

/** The custom provider's row chips: only what needs acting on. */
function customChips(v: CodexView): ChipView[] {
  const c = v.custom;
  if (!c) return [];
  const chips: ChipView[] = [];
  if (c.key === 'placeholder') chips.push({ t: 'chỉ khoá giữ chỗ', k: 'bad' });
  if (c.key === 'none') chips.push({ t: 'chưa có khoá', k: 'bad' });
  if (c.proxy) chips.push({ t: 'qua proxy CC Switch', k: 'warn' });
  if (v.reach.state === 'down') chips.push({ t: 'không trả lời', k: 'bad' });
  return chips;
}

function host(url: string | null): string {
  if (!url) return 'không có base_url';
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

function reachText(v: CodexView, now: Date): string {
  if (v.reach.state === 'up') return 'trả lời ' + v.reach.ms + ' ms';
  if (v.reach.state === 'down') return 'không trả lời từ ' + (v.reach.since ? dayClock(v.reach.since, now) : '?');
  return v.custom?.baseUrl ? 'đang kiểm…' : '';
}

/** "Chuyển" is a button to press; the side in use gets a status in its place. */
function SideCell({ on, off, label, title, act, onAction }: { on: boolean; off: boolean; label: string; title?: string; act: Action; onAction(a: Action): void }) {
  if (on) return <InUse />;
  return (
    <Button primary disabled={off} label={label} title={title} onClick={() => onAction(act)}>
      Chuyển
    </Button>
  );
}

function Providers({ v, now, onAction, onPlugins }: { v: CodexView; now: Date; onAction(a: Action): void; onPlugins(on: boolean): void }) {
  const c = v.custom;
  const busy = v.busy != null;
  const refusal = v.current === 'custom' ? v.backRefusal : v.refusal;
  const mode = keyMode(v);
  const loginSub = hasLogin(v) ? 'đăng nhập còn trong máy' : '';
  return (
    <Card title="Nhà cung cấp" x>
      <div className="grid [--wt-cols:minmax(0,1fr)_auto]" role="list" aria-label="Nhà cung cấp Codex">
        <div className={cn(TABLE_ROW, v.current === 'chatgpt' && TABLE_ROW_ON)} role="listitem" aria-label="Tài khoản ChatGPT">
          <AccountName name="Tài khoản ChatGPT" sub={loginSub} full="" chips={hasLogin(v) ? [] : [{ t: 'chưa đăng nhập', k: 'warn' }]} />
          <span className="flex items-center justify-end gap-2.5">
            <SideCell
              on={v.current === 'chatgpt'}
              off={!!v.backRefusal || busy}
              label="Chuyển sang tài khoản ChatGPT"
              title={v.backRefusal ? refusalText(v, v.backRefusal) : undefined}
              act="codex:switch:chatGpt"
              onAction={onAction}
            />
          </span>
        </div>
        {c ? (
          <div className={cn(TABLE_ROW, v.current === 'custom' && TABLE_ROW_ON)} role="listitem" aria-label={c.name}>
            <AccountName name={c.name} sub={[host(c.baseUrl), reachText(v, now)].filter(Boolean).join(' · ')} full={c.baseUrl ?? ''} chips={customChips(v)} />
            <span className="flex items-center justify-end gap-2.5">
              {mode ? (
                <LinkButton disabled={busy} onClick={() => onAction('codex:key')}>
                  {mode === 'replace' ? 'Đổi khoá…' : mode === 'env' ? 'Cách đặt khoá…' : 'Dán khoá…'}
                </LinkButton>
              ) : null}
              <SideCell
                on={v.current === 'custom'}
                off={!!v.refusal || busy}
                label={'Chuyển sang ' + c.name}
                title={v.refusal ? refusalText(v, v.refusal) : undefined}
                act="codex:switch:custom"
                onAction={onAction}
              />
            </span>
          </div>
        ) : null}
      </div>
      {v.current === 'other' ? <Note tone="warn">Codex đang dùng nhà cung cấp "{v.other}". Chọn một bên để chuyển.</Note> : null}
      {refusal ? <Note tone={refusal.kind === 'noCustom' ? '' : 'bad'}>{refusalText(v, refusal)}</Note> : null}
      {v.openaiBaseUrl ? <Note tone="warn">config.toml có openai_base_url: tài khoản ChatGPT đang được gửi tới địa chỉ khác.</Note> : null}
      {c ? (
        <CheckLine className="mt-2.5 mb-0" checked={v.plugins} disabled={busy} onChange={onPlugins}>
          <span>
            {PLUGINS_LABEL}
            <small>{PLUGINS_HINT}</small>
          </span>
        </CheckLine>
      ) : null}
      {pluginsNeedApply(v, v.plugins) ? (
        <div className={cn(ROW2, 'mt-1.5 text-sm text-warn-ink')}>
          <span>config.toml chưa theo lựa chọn này.</span>
          <LinkButton disabled={busy} onClick={() => onAction('codex:plugins:apply')}>
            Áp dụng ngay…
          </LinkButton>
        </div>
      ) : null}
      {v.keyUndo ? (
        <div className={cn(ROW2, 'mt-2 text-sm text-ink2')}>
          <span>Đã dán khoá{v.keyUndo.at ? ' lúc ' + dayClock(v.keyUndo.at, now) : ''}.</span>
          <LinkButton disabled={busy} onClick={() => onAction('codex:undoKey')}>
            {undoLabel(v.keyUndo)}
          </LinkButton>
        </div>
      ) : null}
      <Details v={v} now={now} onAction={onAction} />
    </Card>
  );
}

/** What the tab used to spell out, behind one "Chi tiết". */
function Details({ v, now, onAction }: { v: CodexView; now: Date; onAction(a: Action): void }) {
  const c = v.custom;
  const w = v.lastWrite;
  const requires = c?.requiresOpenaiAuth == null ? 'chưa có' : String(c.requiresOpenaiAuth);
  const wants = v.wantsRequires != null && v.wantsRequires !== c?.requiresOpenaiAuth ? ' → ' + v.wantsRequires + ' khi chuyển' : '';
  const r = v.resume;
  return (
    <details className={DETAILS}>
      <summary className={SUMMARY}>Chi tiết</summary>
      <div className="mt-2 grid gap-3">
        <Rows
          rows={[
            { id: 'login', key: 'Đăng nhập ChatGPT (auth.json)', value: <Chip chip={loginChip(v)} /> },
            ...(c
              ? [
                  { id: 'url', key: 'Địa chỉ ' + c.name, value: c.baseUrl ? <span className={CODE}>{c.baseUrl}</span> : 'không có' },
                  { id: 'key', key: 'Khoá API riêng', value: <Chip chip={keyChip(v)} /> },
                  { id: 'req', key: 'requires_openai_auth', value: <span className={CODE}>{requires + wants}</span> },
                ]
              : []),
            { id: 'w', key: 'File app ghi', value: <span className={CODE}>{v.home}/config.toml</span> },
            { id: 'n', key: 'Không ghi', value: <span className={CODE}>auth.json · [mcp_servers]</span> },
            {
              id: 'b',
              key: 'Sao lưu',
              value: (
                <>
                  {v.backups.length} bản · không chứa khoá <LinkButton onClick={() => onAction('codex:backup')}>Xem</LinkButton>
                </>
              ),
            },
            {
              id: 'l',
              key: 'App ghi lần cuối',
              value: w ? (
                <>
                  {(w.at ? dayClock(w.at, now) + ' · ' : '') + shortName(v, w.provider) + ' '}
                  <Chip chip={w.matches ? { t: 'mã băm khớp', k: 'ok' } : { t: 'file đã khác', k: 'warn' }} />
                </>
              ) : (
                'chưa ghi lần nào'
              ),
            },
            {
              id: 'r',
              key: 'Phiên tiếp tục được',
              value: r ? 'ChatGPT ' + (r[CHATGPT_ID] ?? 0) + (c ? ' · ' + c.name + ' ' + (r[c.id] ?? 0) : '') + ' (codex resume tách theo nhà cung cấp)' : '…',
            },
          ]}
        />
        <div>
          <b className="mb-1 block text-xs font-semi text-ink2">Lần chuyển gần đây</b>
          {v.recent.length ? (
            <div className="grid gap-1">
              {v.recent.map((e) => (
                <div key={e.at + e.to + e.result} className="grid grid-cols-[64px_minmax(0,1fr)] gap-2">
                  <time className="tabular-nums">{dayClock(e.at, now)}</time>
                  <span className="text-ink">{recentText(v, e)}</span>
                </div>
              ))}
            </div>
          ) : (
            'Chưa có lần chuyển nào trên máy này.'
          )}
        </div>
      </div>
    </details>
  );
}

function OnThisMac({ v, onAction }: { v: CodexView; onAction(a: Action): void }) {
  const p = v.procs;
  const on = v.mcp.filter((m) => m.enabled).length;
  const line = [
    'CLI ' + (v.version ?? 'không tìm thấy'),
    p ? p.sessions + ' phiên Terminal' : null,
    p ? 'Codex App ' + (p.app ? 'đang mở' : 'không mở') : null,
  ]
    .filter(Boolean)
    .join(' · ');
  return (
    <Card title="Codex trên máy này">
      <div className="text-sm text-ink2">
        <Dot tone={p?.daemon ? 'ok' : 'muted'} className="mr-1.5" />
        {line}
      </div>
      <div className={cn(ROW2, 'mt-1.5 text-sm text-ink2')}>
        <span>{v.mcp.length ? v.mcp.length + ' MCP · ' + on + ' đang bật' : 'Chưa có MCP'}</span>
        <LinkButton onClick={() => onAction('mcp:tab')}>Mở tab MCP</LinkButton>
      </div>
      <div className={cn(ROW2, 'mt-2.5')}>
        <small className="text-xs text-ink2">Đổi xong: ⌘Q Codex App rồi mở lại.</small>
        <Button disabled={v.busy != null} onClick={() => onAction('codex:daemon')}>
          Khởi động lại tiến trình nền…
        </Button>
      </div>
    </Card>
  );
}

/** Whole days since `iso`. */
function daysSince(iso: string | null, now: Date): number {
  return iso ? Math.max(0, Math.floor((now.getTime() - new Date(iso).getTime()) / 86400000)) : 0;
}

function CcSwitchGuide({ v, macs, now }: { v: CodexView; macs: number; now: Date }) {
  const cc = v.ccSwitch;
  const days = daysSince(v.firstSwitchAt, now);
  const steady = days >= STEADY_DAYS;
  const where = macs > 1 ? 'cả ' + macs + ' Mac' : 'máy này';
  const chip: ChipView = cc.running ? { t: 'đang chạy', k: 'warn' } : cc.installed ? { t: 'đã cài · đang tắt', k: '' } : { t: 'đã gỡ', k: 'ok' };
  const step = (done: boolean, open: boolean) => (done ? 'done' : open ? '' : 'lock');
  const items: { text: string; small?: string; cls: string }[] = [
    {
      text: 'Chuyển Codex bằng Quota Station ổn vài ngày trên ' + where,
      small:
        'Máy này: ' +
        Math.min(days, STEADY_DAYS) +
        '/' +
        STEADY_DAYS +
        ' ngày' +
        (v.firstSwitchAt ? ' từ lần chuyển đầu (' + dayClock(v.firstSwitchAt, now) + ')' : ', chưa chuyển lần nào') +
        '.' +
        (macs > 1 ? ' Các Mac khác: xem trên từng máy.' : ''),
      cls: step(steady, true),
    },
    { text: 'Xuất bản sao lưu SQL của CC Switch', small: 'CC Switch › Cài đặt › Import/Export › Export SQL Backup', cls: step(false, steady) },
    { text: 'So danh sách ở tab MCP với MCP trong CC Switch', small: 'MCP đang tắt chỉ có trong dữ liệu CC Switch: thêm lại ở tab MCP', cls: step(false, steady) },
    { text: 'Chuyển Codex một lần bằng Quota Station', cls: step(!!v.firstSwitchAt, steady) },
    {
      text: 'Dọn dấu vết CC Switch trong config.toml',
      small: cc.traces.length ? 'Còn: ' + cc.traces.join(' · ') + '. Sửa tay; app không sửa các dòng này.' : 'Không còn dấu vết nào.',
      cls: step(!cc.traces.length, steady),
    },
    { text: 'Tắt CC Switch khi đăng nhập, rồi thoát CC Switch', cls: step(!cc.running, steady) },
    {
      text: 'Kiểm skill của Codex trong ~/.codex/skills',
      small: 'Skill nào là liên kết vào thư mục skills của CC Switch thì thay bằng bản sao thật hoặc cài lại skill, trước khi xoá dữ liệu CC Switch',
      cls: step(false, steady),
    },
    { text: 'Sau một ngày ổn: bỏ CC Switch.app vào Thùng rác', small: 'Giữ thư mục ~/.cc-switch thêm 2 tuần', cls: step(!cc.installed, steady) },
  ];
  const done = items.filter((i) => i.cls === 'done').length;
  return (
    <Card
      id="cc-card"
      title={
        <>
          {'Ngừng dùng CC Switch · ' + done + '/' + items.length}
          <span className="ml-auto font-reg">
            <Chip chip={chip} />
          </span>
        </>
      }
    >
      <CardText className="mb-0">Giữ CC Switch tới khi Quota Station chuyển Codex ổn vài ngày.</CardText>
      <details className={DETAILS}>
        <summary className={SUMMARY}>Xem các bước</summary>
        <ol className="m-0 mt-2 grid list-none gap-[7px] p-0 text-sm text-ink [counter-reset:st]">
          {items.map((i) => (
            <li
              key={i.text}
              className={cn(
                'grid grid-cols-[20px_minmax(0,1fr)] items-start gap-2 before:grid before:size-[18px] before:place-items-center before:rounded-full before:text-[10px] before:font-heavy before:[counter-increment:st]',
                i.cls === 'done'
                  ? "before:bg-ok/20 before:text-ok-ink before:content-['✓']"
                  : 'before:bg-fill2 before:text-ink2 before:content-[counter(st)]',
                i.cls === 'lock' &&
                  "grid-cols-[20px_minmax(0,1fr)_auto] text-ink2 after:whitespace-nowrap after:rounded-chip after:bg-fill2 after:px-1.5 after:text-cap after:leading-4 after:text-ink2 after:content-['chờ_bước_trước']",
              )}
            >
              <span>
                {i.text}
                {i.small ? <small className="block text-xs text-ink2">{i.small}</small> : null}
              </span>
            </li>
          ))}
        </ol>
      </details>
    </Card>
  );
}

export interface CodexTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  onAction(act: Action): void;
  /** "Dùng plugin khi chạy Custom API". */
  onPlugins(on: boolean): void;
}

export function CodexTab({ model, fresh, feedback, now, onAction, onPlugins }: CodexTabProps) {
  const v = model.codex;
  const top = (
    <TabHead>
      <TabTitle>Codex</TabTitle>
      <TabSub>
        <Dot tone={fresh.tone} />
        {model.machine} · {fresh.text}
      </TabSub>
      <Button disabled={v.busy != null} onClick={() => onAction('codex:check')}>
        Kiểm lại
      </Button>
    </TabHead>
  );
  if (!v.present)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <b>Máy này chưa có Codex</b>
            Không thấy thư mục {v.home}. Nếu Codex dùng thư mục khác, đặt nó ở Cài đặt › Thư mục Codex.
          </Empty>
        </Panel>
      </>
    );
  const banners: BannerView[] = [];
  const main = codexBanner(model);
  if (main) banners.push(main);
  if (v.external && main?.title !== externalBanner(v)?.title) banners.push(externalBanner(v)!);
  if (ccSwitchManages(v))
    banners.push({
      tone: 'warn',
      title: 'CC Switch đang chạy và cũng đổi Codex',
      body: 'Chỉ chuyển Codex ở một nơi: lần chuyển sau của CC Switch ghi đè lựa chọn ở đây.',
      actions: [],
    });
  return (
    <>
      {top}
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {banners.map((b) => (
        <Banner key={b.title} banner={b} onAction={onAction} />
      ))}
      {v.config === 'invalid' || v.config === 'unreadable' ? (
        <BannerBox
          tone="bad"
          title="Không đọc được config.toml của Codex"
          body={(v.reason ?? 'Không rõ lý do') + '. App không sửa file này cho tới khi Codex đọc được nó.'}
        />
      ) : null}
      <Providers v={v} now={now} onAction={onAction} onPlugins={onPlugins} />
      <div className={GRID2}>
        <OnThisMac v={v} onAction={onAction} />
        <CcSwitchGuide v={v} macs={1 + model.rotation.peers.length} now={now} />
      </div>
    </>
  );
}

/** Codex on Tổng quan: the provider in use, one line, and the switch to the other side. */
export function CodexCard({ model, onAction }: { model: Model; onAction(act: Action): void }) {
  const v = model.codex;
  if (!v.present) return null;
  const cur = currentId(v);
  const line = codexLine(v);
  const toCustom = v.current !== 'custom';
  const refusal = toCustom ? v.refusal : v.backRefusal;
  const sub =
    (v.current === 'custom'
      ? v.reach.state === 'down'
        ? v.custom!.name + ' không trả lời'
        : v.reach.state === 'up'
          ? 'trả lời ' + v.reach.ms + ' ms'
          : v.custom!.name
      : v.current === 'other'
        ? 'nhà cung cấp ' + cur
        : 'đăng nhập chính thức') + (hasLogin(v) ? ' · giữ đăng nhập ChatGPT' : ' · chưa đăng nhập ChatGPT');
  return (
    <Panel>
      <CardTitle>
        <IdMark id="x" />
        Codex
        <span className="ml-auto font-reg">{v.procs?.app ? 'Codex App đang mở' : ''}</span>
      </CardTitle>
      <div className={ROW2}>
        <div>
          <b className={line.tone === 'bad' ? 'text-bad-ink!' : undefined}>{v.current === 'custom' ? 'Qua ' + v.custom!.name : v.current === 'other' ? 'Nhà cung cấp ' + cur : 'Tài khoản ChatGPT'}</b>
          <small style={{ display: 'block', color: 'var(--ink2)', fontSize: 'var(--fs-xs)' }}>{sub}</small>
        </div>
        {v.custom || v.current !== 'chatgpt' ? (
          <Button
            disabled={!!refusal || v.busy != null}
            title={refusal ? refusalText(v, refusal) : undefined}
            onClick={() => onAction(toCustom ? 'codex:switch:custom' : 'codex:switch:chatGpt')}
          >
            {toCustom ? 'Sang ' + (v.custom?.name ?? '…') : 'Về ChatGPT'}
          </Button>
        ) : null}
      </div>
    </Panel>
  );
}
