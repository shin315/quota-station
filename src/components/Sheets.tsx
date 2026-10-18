// The confirmation sheets (ConfirmSheet `.sheet` in the popover, LeaveDialog
// `.dlg` in the main window): switch Claude (F04), pause automatic switching
// (F07), and the two switcher guides. Both hosts draw the same body.

import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import type { Model, Sessions } from '../lib/model';
import {
  claudeSheet,
  guide,
  holdOptions,
  pauseOptions,
  pauseSheet,
  type HoldChoice,
  type PauseChoice,
} from '../lib/switching';
import { Button, ChoiceGroup, Rich } from './parts';

export type SheetState =
  | { kind: 'claude'; slot: number }
  | { kind: 'pause'; preset: PauseChoice }
  | { kind: 'guide'; id: 'auto' | 'switcher' };

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
  /** A fresh clock for the moment the owner confirms: a hold runs from then. */
  clock: () => Date;
  sessions: Sessions | null | 'loading';
  /** The switch is running: the sheet shows it until the result is in. */
  working: boolean;
  onClose(): void;
  onSwitch(slot: number, holdUntil: Date | null): void;
  onPause(until: Date | null): void;
  onSettings(): void;
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
  const confirm = () => {
    const target = props.model.snapshot.claude.accounts.find((a) => a.number === props.slot);
    const until = target ? (holdOptions(target, props.clock()).find((o) => o.v === choice)?.until ?? null) : null;
    props.onSwitch(props.slot, until);
  };
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

function PauseBody(props: SheetBodyProps & { preset: PauseChoice }) {
  const [choice, setChoice] = useState<PauseChoice>(props.preset);
  // The clock as of the last confirm, when it ran ahead of the window's.
  const [late, setLate] = useState<Date | null>(null);
  const now = late && late > props.now ? late : props.now;
  const v = pauseSheet(props.model, choice, now);
  const value = v.options.some((o) => o.v === choice) ? choice : v.options[0].v;
  const confirm = () => {
    const at = props.clock();
    const option = pauseOptions(at).find((o) => o.v === value);
    // "Đến 18:00" is gone once 18:00 has passed: show the choices again
    // rather than pause with no end.
    if (option) props.onPause(option.until);
    else setLate(at);
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

export function SheetBody(props: SheetBodyProps) {
  const s = props.sheet;
  if (s.kind === 'claude') return <ClaudeBody key={'c' + s.slot} {...props} slot={s.slot} />;
  if (s.kind === 'pause') return <PauseBody key={'p' + s.preset} {...props} preset={s.preset} />;
  return <GuideBody {...props} id={s.id} />;
}
