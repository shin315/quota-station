// The confirmation sheets (ConfirmSheet `.sheet` in the popover, LeaveDialog
// `.dlg` in the main window): switch Claude (F04), switch Antigravity (M4c),
// pause automatic switching (F07), the two switcher guides, and Codex (M8):
// its switch with the optional restart box, the restart of its background
// process, the file changed outside the app, and its guides and previews.
// Both hosts draw the same body.

import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { agySheet, agyWorking, type AppAnswer } from '../lib/agySwitch';
import { codexSheet, daemonFacts, externalSheet, longName, targetId } from '../lib/codex';
import { dayClock } from '../lib/switching';
import type { AgyHow, CodexTarget, Model, Sessions } from '../lib/model';
import {
  claudeSheet,
  guide,
  pauseOptions,
  pauseSheet,
  type HoldChoice,
  type PauseChoice,
} from '../lib/switching';
import { Button, ChoiceGroup, Rich } from './parts';

export type SheetState =
  | { kind: 'claude'; slot: number }
  | { kind: 'agy'; number: number }
  | { kind: 'pause'; preset: PauseChoice }
  | { kind: 'guide'; id: 'auto' | 'switcher' }
  | { kind: 'codex'; target: CodexTarget }
  /** Restart Codex's background process; `after` straight after a switch whose box was ticked. */
  | { kind: 'daemon'; after: boolean }
  | { kind: 'codexExternal' }
  | { kind: 'codexInfo'; id: 'login' | 'mcp' | 'backup' };

export const SHEET_TITLE_ID = 'qs-dlg-t';

const FOCUSABLE = 'button:not([disabled]):not([tabindex="-1"]), [href], input:not([disabled])';

/**
 * G10: the dialog takes focus when it opens, keeps Tab inside, and gives
 * focus back to where it was when it closes.
 */
export function useDialog(open: boolean) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const before = document.activeElement as HTMLElement | null;
    const el = ref.current;
    const first = el?.querySelector<HTMLElement>('[role="radio"][aria-checked="true"]') ?? el?.querySelector<HTMLElement>(FOCUSABLE);
    (first ?? el)?.focus();
    return () => before?.focus?.();
  }, [open]);
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== 'Tab' || !ref.current) return;
    const items = Array.from(ref.current.querySelectorAll<HTMLElement>(FOCUSABLE + ', [role="radio"][tabindex="0"]'));
    if (!items.length) return;
    const i = items.indexOf(document.activeElement as HTMLElement);
    const next = e.shiftKey ? (i <= 0 ? items.length - 1 : i - 1) : i === items.length - 1 ? 0 : i + 1;
    e.preventDefault();
    items[next].focus();
  };
  return { ref, onKeyDown };
}

export interface SheetBodyProps {
  sheet: SheetState;
  model: Model;
  now: Date;
  /** The clock at the moment the owner confirms, to catch a choice that has just lapsed. */
  clock: () => Date;
  sessions: Sessions | null | 'loading';
  /** What the Antigravity sheet learnt about Antigravity.app. */
  agyApp: AppAnswer;
  /** The switch is running: the sheet shows it until the result is in. */
  working: boolean;
  /** The button the running Antigravity switch was started with. */
  agyHow: AgyHow | null;
  onClose(): void;
  onSwitch(slot: number, holdUntil: Date | null): void;
  onSwitchAgy(number: number, how: AgyHow): void;
  onPause(until: Date | null): void;
  onSettings(): void;
  /** Codex (M8): switch, with the restart sheet after it when `restart`. */
  onCodexSwitch?(target: CodexTarget, restart: boolean): void;
  onCodexRestart?(after: boolean): void;
  /** "Không khởi động lại" after a switch. */
  onCodexSkipRestart?(): void;
  onCodexKeep?(): void;
  /** Another Codex sheet: "Về lại …", the guides. */
  onCodexOpen?(sheet: SheetState): void;
  onCodexExport?(): void;
  /** The guide to stop using CC Switch, in the Codex tab. */
  onCodexGuide?(): void;
}

function ClaudeBody(props: SheetBodyProps & { slot: number }) {
  const [choice, setChoice] = useState<HoldChoice>('60');
  const v = claudeSheet(props.model, props.slot, choice, props.sessions, props.now);
  if (!v) {
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>Không còn tài khoản này</h3>
        <p className="lead">claude-swap không còn liệt kê tài khoản {props.slot}. Bấm Làm mới rồi thử lại.</p>
        <div className="btns">
          <Button large primary onClick={props.onClose}>
            Đóng
          </Button>
        </div>
      </>
    );
  }
  if (props.working) {
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>{v.confirm.replace('Chuyển sang', 'Đang chuyển sang')}…</h3>
        <div className="fb info" role="status" style={{ margin: '8px 0 0' }}>
          <span className="spin" aria-hidden="true" />
          <div>
            <b>cswap switch {props.slot} đang chuyển tài khoản</b>
            <small>Sau đó app đợi 5 giây rồi đọc lại tài khoản đang dùng để kiểm.</small>
          </div>
        </div>
      </>
    );
  }
  // The hold ends at the time its choice shows.
  const confirm = () => props.onSwitch(props.slot, v.options.find((o) => o.v === choice)?.until ?? null);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{v.title}</h3>
      <ul className="facts">
        {v.facts.map((f) => (
          <li key={f.text} className={f.k}>
            <span>
              <Rich text={f.text} />
            </span>
          </li>
        ))}
      </ul>
      <ChoiceGroup<HoldChoice> id="qs-hold" label="Giữ tài khoản này" options={v.options} value={choice} onChange={setChoice} />
      <div className={('hint ' + v.hint.tone).trim()} aria-live="polite">
        {v.hint.text}
      </div>
      <div className="btns">
        {v.blocked ? (
          <button type="button" className="lnk" style={{ marginRight: 'auto' }} onClick={props.onSettings}>
            Mở cài đặt
          </button>
        ) : null}
        <Button large onClick={props.onClose}>
          Thôi
        </Button>
        <Button large primary disabled={v.blocked} onClick={confirm}>
          {v.confirm}
        </Button>
      </div>
    </>
  );
}

function AgyBody(props: SheetBodyProps & { number: number }) {
  const v = agySheet(props.model, props.number, props.agyApp);
  if (!v) {
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>Không còn tài khoản này</h3>
        <p className="lead">Tài khoản Antigravity {props.number} không còn trong danh sách đăng nhập của app. Bấm Làm mới rồi thử lại.</p>
        <div className="btns">
          <Button large primary onClick={props.onClose}>
            Đóng
          </Button>
        </div>
      </>
    );
  }
  if (props.working) {
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>{v.title.replace('Chuyển', 'Đang chuyển').replace('?', '…')}</h3>
        <div className="fb info" role="status" style={{ margin: '8px 0 0' }}>
          <span className="spin" aria-hidden="true" />
          <div>
            <b>Đang chuyển tài khoản Antigravity</b>
            <small>{agyWorking(props.agyHow)}</small>
          </div>
        </div>
      </>
    );
  }
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{v.title}</h3>
      <ul className="facts">
        {v.facts.map((f) => (
          <li key={f.text} className={f.k}>
            <span>
              <Rich text={f.text} />
            </span>
          </li>
        ))}
      </ul>
      <div className="btns">
        <Button large onClick={props.onClose}>
          Thôi
        </Button>
        {v.buttons.map((b) => (
          <Button key={b.how} large primary={b.primary} disabled={v.waiting} onClick={() => props.onSwitchAgy(props.number, b.how)}>
            {b.label}
          </Button>
        ))}
      </div>
    </>
  );
}

function PauseBody(props: SheetBodyProps & { preset: PauseChoice }) {
  const [choice, setChoice] = useState<PauseChoice>(props.preset);
  // The clock as of the last confirm, when it ran ahead of the window's.
  const [late, setLate] = useState<Date | null>(null);
  const now = late && late > props.now ? late : props.now;
  const v = pauseSheet(props.model, choice, now);
  const value = v.options.some((o) => o.v === choice) ? choice : v.options[0].v;
  const confirm = () => {
    const at = props.clock();
    // "Đến 18:00" is gone once 18:00 has passed: show the choices again
    // rather than pause with no end.
    if (!pauseOptions(at).some((o) => o.v === value)) return setLate(at);
    // The pause ends at the time its choice shows.
    const option = v.options.find((o) => o.v === value);
    if (option) props.onPause(option.until);
  };
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{v.title}</h3>
      <p className="lead">{v.lead}</p>
      <ChoiceGroup<PauseChoice> id="qs-pause" label="Trong bao lâu" options={v.options} value={value} columns={3} onChange={setChoice} />
      <div className="hint" aria-live="polite">
        {v.hint}
      </div>
      <div className="btns">
        <Button large onClick={props.onClose}>
          Thôi
        </Button>
        <Button large primary onClick={confirm}>
          Tạm dừng
        </Button>
      </div>
    </>
  );
}

function GuideBody(props: SheetBodyProps & { id: 'auto' | 'switcher' }) {
  const g = guide(props.id);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{g.title}</h3>
      <p className="lead">{g.lead}</p>
      <ol className="howto">
        {g.steps.map((s) => (
          <li key={s}>
            <Rich text={s} />
          </li>
        ))}
      </ol>
      <div className="btns">
        <Button large primary onClick={props.onClose}>
          Đóng
        </Button>
      </div>
    </>
  );
}

function Facts({ facts }: { facts: { k: string; text: string }[] }) {
  return (
    <ul className="facts">
      {facts.map((f) => (
        <li key={f.text} className={f.k}>
          <span>
            <Rich text={f.text} />
          </span>
        </li>
      ))}
    </ul>
  );
}

function Working({ title, line, detail }: { title: string; line: string; detail: string }) {
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{title}</h3>
      <div className="fb info" role="status" style={{ margin: '8px 0 0' }}>
        <span className="spin" aria-hidden="true" />
        <div>
          <b>{line}</b>
          <small>{detail}</small>
        </div>
      </div>
    </>
  );
}

function CodexBody(props: SheetBodyProps & { target: CodexTarget }) {
  // G14: off by default; ticked, the restart sheet asks once more after the switch.
  const [restart, setRestart] = useState(false);
  const v = codexSheet(props.model, props.target);
  const to = longName(props.model.codex, targetId(props.model.codex, props.target));
  if (props.working)
    return (
      <Working
        title={'Đang chuyển Codex sang ' + to + '…'}
        line="Đang ghi config.toml"
        detail="Sao lưu bản cũ → ghi file tạm → kiểm mã băm → thay file → đọc lại."
      />
    );
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{v.title}</h3>
      <p className="lead">{v.lead}</p>
      <Facts facts={v.facts} />
      <label className="check">
        <input type="checkbox" checked={restart} disabled={v.blocked} onChange={(e) => setRestart(e.target.checked)} />
        <span>
          Khởi động lại tiến trình nền Codex sau khi chuyển
          <small>Việc đang chạy trong Terminal sẽ bị ngắt. App hỏi lại một lần trước khi làm. Codex App vẫn phải tự thoát và mở lại.</small>
        </span>
      </label>
      <div className="btns">
        {v.ccSwitch && props.onCodexGuide ? (
          <button type="button" className="lnk" style={{ marginRight: 'auto' }} onClick={props.onCodexGuide}>
            Hướng dẫn ngừng dùng CC Switch
          </button>
        ) : null}
        <Button large onClick={props.onClose}>
          Thôi
        </Button>
        <Button large primary disabled={v.blocked} onClick={() => props.onCodexSwitch?.(props.target, restart)}>
          {v.confirm}
        </Button>
      </div>
    </>
  );
}

function DaemonBody(props: SheetBodyProps & { after: boolean }) {
  const v = props.model.codex;
  if (props.working)
    return <Working title="Đang khởi động lại tiến trình nền Codex…" line="codex app-server daemon restart" detail="Tiến trình nền làm xong việc đang chạy (tối đa 60 giây) rồi chạy lại." />;
  const name = longName(v, v.current === 'custom' && v.custom ? v.custom.id : 'openai');
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Khởi động lại tiến trình nền Codex?</h3>
      {props.after ? (
        <p className="lead">
          Đã chuyển Codex sang {name}. Bạn đã chọn khởi động lại tiến trình nền sau khi chuyển; app hỏi lại vì việc này ngắt việc đang chạy.
        </p>
      ) : null}
      {v.procs && !v.procs.daemon ? <p className="lead">Tiến trình nền Codex đang không chạy: không có gì để khởi động lại.</p> : null}
      <Facts facts={daemonFacts()} />
      <div className="btns">
        {props.after ? (
          <Button large onClick={props.onCodexSkipRestart}>
            Không khởi động lại
          </Button>
        ) : (
          <Button large onClick={props.onClose}>
            Thôi
          </Button>
        )}
        <Button large primary disabled={v.busy != null} onClick={() => props.onCodexRestart?.(props.after)}>
          Khởi động lại
        </Button>
      </div>
    </>
  );
}

function ExternalBody(props: SheetBodyProps) {
  const v = externalSheet(props.model);
  if (!v)
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>Cấu hình Codex không còn khác</h3>
        <p className="lead">config.toml đang là bản app ghi lần cuối, hoặc bạn đã giữ bản này.</p>
        <div className="btns">
          <Button large primary onClick={props.onClose}>
            Đóng
          </Button>
        </div>
      </>
    );
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Cấu hình Codex đã đổi ngoài app</h3>
      <p className="lead">{v.lead}</p>
      <dl className="kv">
        {v.rows.map((r) => [<dt key={r.key + ':k'}>{r.key}</dt>, <dd key={r.key + ':v'}>{r.value}</dd>])}
      </dl>
      <Facts facts={[{ k: '', text: 'Bản trước khi đổi vẫn còn trong thư mục sao lưu của app.' }]} />
      <div className="btns">
        {v.back ? (
          <Button large onClick={() => props.onCodexOpen?.({ kind: 'codex', target: v.back!.target })}>
            {v.back.label}
          </Button>
        ) : null}
        <Button large primary onClick={props.onCodexKeep}>
          Giữ như bây giờ
        </Button>
      </div>
    </>
  );
}

function CodexInfoBody(props: SheetBodyProps & { id: 'login' | 'mcp' | 'backup' }) {
  const v = props.model.codex;
  const close = (
    <Button large primary onClick={props.onClose}>
      Đóng
    </Button>
  );
  if (props.id === 'login')
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>Đăng nhập ChatGPT cho Codex</h3>
        <p className="lead">Quota Station không đăng nhập hộ và không đọc nội dung đăng nhập.</p>
        <ol className="howto">
          <li>Mở Codex App.</li>
          <li>Chọn đăng nhập bằng ChatGPT và làm theo trang đăng nhập.</li>
          <li>Quay lại tab Codex, bấm Kiểm lại.</li>
        </ol>
        <div className="btns">{close}</div>
      </>
    );
  if (props.id === 'mcp')
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>Xuất danh sách MCP của Codex</h3>
        <p className="lead">
          Đọc từ <span className="mono">{v.home}/config.toml</span>. App không ghi phần <span className="mono">[mcp_servers]</span>.
        </p>
        <div className="kvt" style={{ marginBottom: 10 }}>
          {v.mcp.length ? (
            v.mcp.map((m) => [
              <span key={m.name + ':k'}>
                <span className="mono">{m.name}</span> · {m.kind}
              </span>,
              <span key={m.name + ':v'}>{m.enabled ? 'bật' : 'tắt'}</span>,
            ])
          ) : (
            <span>Không có máy chủ MCP nào</span>
          )}
        </div>
        <p className="lead">
          Lưu vào thư mục Tải về (tên, loại, bật hay tắt; không chép lệnh, địa chỉ hay biến môi trường). Dùng file này để so với danh sách MCP
          trong CC Switch trước khi ngừng dùng CC Switch.
        </p>
        <div className="btns">
          <Button large onClick={props.onClose}>
            Thôi
          </Button>
          <Button large primary onClick={props.onCodexExport}>
            Lưu file
          </Button>
        </div>
      </>
    );
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Thư mục sao lưu Codex</h3>
      <p className="lead">
        <span className="mono">{v.backupDir}</span>
      </p>
      <div className="kvt" style={{ marginBottom: 10 }}>
        {v.backups.length ? (
          v.backups.map((b, i) => [
            <span key={b + ':k'}>config.toml · {dayClock(b, props.now)}</span>,
            <span key={b + ':v'}>{i === 0 ? 'mới nhất' : ''}</span>,
          ])
        ) : (
          <span>Chưa có bản sao lưu nào: app chưa ghi config.toml lần nào.</span>
        )}
      </div>
      <p className="lead">Giữ 10 bản gần nhất. Lần ghi lỗi không đổi file đang dùng, nên chưa cần khôi phục.</p>
      <div className="btns">{close}</div>
    </>
  );
}

export function SheetBody(props: SheetBodyProps) {
  const s = props.sheet;
  if (s.kind === 'codex') return <CodexBody key={'x' + s.target} {...props} target={s.target} />;
  if (s.kind === 'daemon') return <DaemonBody {...props} after={s.after} />;
  if (s.kind === 'codexExternal') return <ExternalBody {...props} />;
  if (s.kind === 'codexInfo') return <CodexInfoBody {...props} id={s.id} />;
  if (s.kind === 'claude') return <ClaudeBody key={'c' + s.slot} {...props} slot={s.slot} />;
  if (s.kind === 'agy') return <AgyBody key={'g' + s.number} {...props} number={s.number} />;
  if (s.kind === 'pause') return <PauseBody key={'p' + s.preset} {...props} preset={s.preset} />;
  return <GuideBody {...props} id={s.id} />;
}
