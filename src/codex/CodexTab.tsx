// The Codex tab of the main window (M8) and Codex's compact card on Tổng
// quan. The tab: the provider in use with a choice between the two sides
// (each opens the confirmation sheet), Codex on this Mac, the resumable
// sessions by provider, the recent switches, how many MCP servers Codex has
// (they are turned on and off in the MCP tab, M9), the guide to stop using CC
// Switch, and the file the app writes.

import type { ReactNode } from 'react';
import { Banner, Button, Chip, Dot, Feedback } from '../components/parts';
import {
  CHATGPT_ID,
  ccSwitchManages,
  codexBanner,
  codexLine,
  currentId,
  externalBanner,
  hasLogin,
  refusalText,
  shortName,
} from '../lib/codex';
import type { CodexView, Model } from '../lib/model';
import { dayClock } from '../lib/switching';
import type { Action, BannerView, ChipView, FeedbackView, PopoverView } from '../lib/view';

/** Days of running the switch here before the CC Switch guide's next steps open. */
export const STEADY_DAYS = 3;

function Card(props: { title: ReactNode; id?: string; x?: boolean; children: ReactNode }) {
  return (
    <div className="card" id={props.id}>
      <h4>
        {props.x ? <span className="idm x" /> : null}
        {props.title}
      </h4>
      {props.children}
    </div>
  );
}

function Rows({ rows }: { rows: { key: ReactNode; value: ReactNode; id: string }[] }) {
  return <div className="kvt">{rows.flatMap((r) => [<span key={r.id + ':k'}>{r.key}</span>, <span key={r.id + ':v'}>{r.value}</span>])}</div>;
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

function reachChip(v: CodexView, now: Date): ChipView {
  if (v.reach.state === 'up') return { t: 'trả lời ' + v.reach.ms + ' ms', k: 'ok' };
  if (v.reach.state === 'down') return { t: 'không trả lời từ ' + (v.reach.since ? dayClock(v.reach.since, now) : '?'), k: 'bad' };
  return { t: v.custom?.baseUrl ? 'đang kiểm…' : 'không có base_url', k: '' };
}

function Provider({ v, now, onAction }: { v: CodexView; now: Date; onAction(a: Action): void }) {
  const c = v.custom;
  const name = c?.name ?? 'Nhà cung cấp riêng';
  const sides: { t: string; on: boolean; act: Action; off: boolean }[] = [
    { t: 'Tài khoản ChatGPT', on: v.current === 'chatgpt', act: 'codex:switch:chatGpt', off: !!v.backRefusal },
    { t: name, on: v.current === 'custom', act: 'codex:switch:custom', off: !!v.refusal },
  ];
  const requires = c?.requiresOpenaiAuth == null ? 'chưa có' : String(c.requiresOpenaiAuth);
  const wants = v.wantsRequires != null && v.wantsRequires !== c?.requiresOpenaiAuth ? ' → ' + v.wantsRequires + ' khi chuyển' : '';
  const refusal = v.current === 'custom' ? v.backRefusal : v.refusal;
  return (
    <Card title="Nhà cung cấp đang dùng" x>
      <div className="seg inline" role="group" aria-label="Nhà cung cấp Codex">
        {sides.map((s) => (
          <button
            key={s.act}
            type="button"
            aria-pressed={s.on}
            disabled={!s.on && (s.off || v.busy != null)}
            onClick={s.on ? undefined : () => onAction(s.act)}
          >
            {s.t}
          </button>
        ))}
      </div>
      {v.current === 'other' ? <p className="note warn">Codex đang dùng nhà cung cấp "{v.other}". Chọn một bên để chuyển.</p> : null}
      <div style={{ marginTop: 12 }}>
        <Rows
          rows={[
            { id: 'login', key: 'Đăng nhập ChatGPT (auth.json)', value: <Chip chip={loginChip(v)} /> },
            ...(c
              ? [
                  {
                    id: 'url',
                    key: (
                      <>
                        {c.name}
                        {c.baseUrl ? <> · <span className="mono">{c.baseUrl}</span></> : null}
                      </>
                    ),
                    value: <Chip chip={reachChip(v, now)} />,
                  },
                  { id: 'key', key: 'Cấu hình ' + c.name + ' có khoá API riêng', value: <Chip chip={keyChip(v)} /> },
                  { id: 'req', key: 'requires_openai_auth', value: <span className="mono">{requires + wants}</span> },
                ]
              : []),
          ]}
        />
      </div>
      {refusal ? <p className={'note ' + (refusal.kind === 'noCustom' ? '' : 'bad')}>{refusalText(v, refusal)}</p> : null}
      {v.openaiBaseUrl ? <p className="note warn">config.toml có openai_base_url: tài khoản ChatGPT đang được gửi tới địa chỉ khác.</p> : null}
      <p style={{ margin: '10px 0 0' }}>Bấm vào bên chưa chọn để mở tấm xác nhận. App chỉ đổi dòng chọn nhà cung cấp trong config.toml.</p>
    </Card>
  );
}

function OnThisMac({ v, onAction }: { v: CodexView; onAction(a: Action): void }) {
  const p = v.procs;
  return (
    <Card title="Codex trên máy này">
      <Rows
        rows={[
          { id: 'cli', key: 'Codex CLI', value: (v.version ?? 'không tìm thấy') + ' · tiến trình nền ' + (p ? (p.daemon ? 'đang chạy' : 'không chạy') : 'không rõ') },
          { id: 'tty', key: 'Phiên codex trong Terminal', value: p ? String(p.sessions) : 'không rõ' },
          { id: 'app', key: 'Codex App', value: p ? (p.app ? 'đang mở' : 'không mở') : 'không rõ' },
        ]}
      />
      <div className="row2" style={{ marginTop: 10 }}>
        <small style={{ color: 'var(--ink2)' }}>Đổi nhà cung cấp xong thì Codex App phải thoát hẳn (⌘Q) và mở lại.</small>
        <Button disabled={v.busy != null} onClick={() => onAction('codex:daemon')}>
          Khởi động lại tiến trình nền…
        </Button>
      </div>
    </Card>
  );
}

function Resumable({ v }: { v: CodexView }) {
  const r = v.resume;
  const chatgpt = r?.[CHATGPT_ID] ?? 0;
  const custom = v.custom ? (r?.[v.custom.id] ?? 0) : 0;
  const other = r ? Object.entries(r).reduce((n, [id, c]) => (id === CHATGPT_ID || id === v.custom?.id ? n : n + c), 0) : 0;
  const tile = (label: string, n: number | null) => (
    <div className="pool">
      <small>{label}</small>
      <b className="n">
        {n == null ? '…' : n}
        <i>phiên</i>
      </b>
    </div>
  );
  return (
    <Card title="Các phiên có thể tiếp tục">
      <div className="grid2">
        {tile('Tài khoản ChatGPT', r ? chatgpt : null)}
        {v.custom ? tile(v.custom.name, r ? custom : null) : null}
      </div>
      <p style={{ margin: '10px 0 0' }}>
        Codex ghi nhà cung cấp vào từng phiên. <span className="mono">codex resume</span> chỉ hiện nhóm của bên đang dùng; Codex App hiện cả
        hai. App không gộp hay chuyển lịch sử. Tiếp tục phiên ở nhà cung cấp khác có thể gặp lỗi: một số dữ liệu mã hoá chỉ nhà cung cấp cũ
        đọc được.{other ? ' ' + other + ' phiên khác thuộc nhà cung cấp khác.' : ''}
      </p>
    </Card>
  );
}

function Recent({ v, now }: { v: CodexView; now: Date }) {
  return (
    <Card title="Lần chuyển gần đây">
      {v.recent.length ? (
        <div className="list">
          {v.recent.map((r) => (
            <div key={r.at + r.to}>
              <time>{dayClock(r.at, now)}</time>
              <span>
                <b>
                  Codex · {shortName(v, r.from)} → {shortName(v, r.to)}
                </b>
                <small style={{ display: 'block', color: 'var(--ink2)' }}>
                  {r.result === 'observed'
                    ? 'config.toml đổi ngoài app (có thể do CC Switch hoặc sửa tay)'
                    : 'Bạn chuyển trong app · giữ đăng nhập ChatGPT' + (r.restarted ? ' · đã khởi động lại tiến trình nền' : '')}
                </small>
              </span>
            </div>
          ))}
        </div>
      ) : (
        <p>Chưa có lần chuyển nào trên máy này.</p>
      )}
    </Card>
  );
}

/** Codex's MCP servers in one line: the MCP tab turns them on and off (M9). */
function McpLine({ v, onAction }: { v: CodexView; onAction(a: Action): void }) {
  const on = v.mcp.filter((m) => m.enabled).length;
  return (
    <Card title={<>MCP của Codex<span className="r">đọc từ config.toml</span></>}>
      <div className="row2">
        <span>{v.mcp.length ? v.mcp.length + ' MCP · ' + on + ' đang bật' : 'config.toml không có máy chủ MCP nào.'}</span>
        <Button onClick={() => onAction('mcp:tab')}>Mở tab MCP</Button>
      </div>
      <p style={{ margin: '8px 0 0' }}>Bật, tắt, thêm hay xoá MCP cho cả ba agent ở tab MCP. Chuyển nhà cung cấp không đụng tới phần này.</p>
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
  return (
    <Card id="cc-card" title={<>Hướng dẫn ngừng dùng CC Switch<span className="r"><Chip chip={chip} /></span></>}>
      <p>Giữ CC Switch tới khi Quota Station chuyển Codex ổn vài ngày trên {where}.</p>
      <ol className="clist">
        {items.map((i) => (
          <li key={i.text} className={i.cls || undefined}>
            <span>
              {i.text}
              {i.small ? <small>{i.small}</small> : null}
            </span>
          </li>
        ))}
      </ol>
    </Card>
  );
}

function Files({ v, now, onAction }: { v: CodexView; now: Date; onAction(a: Action): void }) {
  const w = v.lastWrite;
  return (
    <Card title="File app ghi">
      <Rows
        rows={[
          { id: 'w', key: 'Ghi', value: <span className="mono">{v.home}/config.toml</span> },
          { id: 'n', key: 'Không ghi', value: <span className="mono">auth.json</span> },
          { id: 'm', key: 'MCP', value: <><span className="mono">[mcp_servers]</span> chỉ đổi từ tab MCP</> },
          {
            id: 'b',
            key: 'Sao lưu',
            value: (
              <>
                {v.backups.length} bản · giữ 10 bản gần nhất · không chứa khoá{' '}
                <button type="button" className="lnk" onClick={() => onAction('codex:backup')}>
                  Xem
                </button>
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
        ]}
      />
    </Card>
  );
}

export interface CodexTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  feedback: FeedbackView | null;
  now: Date;
  onAction(act: Action): void;
}

export function CodexTab({ model, fresh, feedback, now, onAction }: CodexTabProps) {
  const v = model.codex;
  const top = (
    <div className="tb" data-tauri-drag-region="deep">
      <h2>Codex</h2>
      <small>
        <Dot tone={fresh.tone} />
        {model.machine} · {fresh.text}
      </small>
      <Button disabled={v.busy != null} onClick={() => onAction('codex:check')}>
        Kiểm lại
      </Button>
    </div>
  );
  if (!v.present)
    return (
      <>
        {top}
        <div className="card">
          <div className="empty">
            <b>Máy này chưa có Codex</b>
            Không thấy thư mục {v.home}. Nếu Codex dùng thư mục khác, đặt nó ở Cài đặt › Thư mục Codex.
          </div>
        </div>
      </>
    );
  const banners: BannerView[] = [];
  const main = codexBanner(model);
  if (main) banners.push(main);
  if (v.external && main?.title !== externalBanner(v)?.title) banners.push(externalBanner(v)!);
  if (ccSwitchManages(v))
    banners.push({
      tone: 'warn',
      title: 'CC Switch đang chạy và quản lý Codex',
      body: 'Khi CC Switch chuyển nhà cung cấp, nó ghi đè lựa chọn nhà cung cấp trong config.toml. Trong lúc thử, chỉ chuyển Codex ở một nơi.',
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
        <div className="banner bad" role="status">
          <b>Không đọc được config.toml của Codex</b>
          <p>{v.reason ?? 'Không rõ lý do'}. App không sửa file này cho tới khi Codex đọc được nó.</p>
        </div>
      ) : null}
      <div className="grid">
        <div className="stack">
          <Provider v={v} now={now} onAction={onAction} />
          <OnThisMac v={v} onAction={onAction} />
          <Resumable v={v} />
          <Recent v={v} now={now} />
        </div>
        <div className="stack">
          <McpLine v={v} onAction={onAction} />
          <CcSwitchGuide v={v} macs={1 + model.rotation.peers.length} now={now} />
          <Files v={v} now={now} onAction={onAction} />
        </div>
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
    <div className="card">
      <h4>
        <span className="idm x" />
        Codex
        <span className="r">{v.procs?.app ? 'Codex App đang mở' : ''}</span>
      </h4>
      <div className="row2">
        <div>
          <b className={line.tone === 'bad' ? 'tn-bad' : undefined}>{v.current === 'custom' ? 'Qua ' + v.custom!.name : v.current === 'other' ? 'Nhà cung cấp ' + cur : 'Tài khoản ChatGPT'}</b>
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
    </div>
  );
}
