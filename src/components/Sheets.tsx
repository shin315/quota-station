// The confirmation sheets (ConfirmSheet `.sheet` in the popover, LeaveDialog
// `.dlg` in the main window): switch Claude (F04), switch Antigravity (M4c),
// pause automatic switching (F07), the two switcher guides, and Codex (M8):
// its switch with the optional restart box, the restart of its background
// process, the file changed outside the app, and its guides and previews;
// "Dán khoá" for the custom provider and "Áp dụng ngay" for the plugins box.
// Both hosts draw the same body.

import { useState } from 'react';
import { Checkbox } from '@/components/ui/checkbox';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { agySheet, agyWorking, type AppAnswer } from '../lib/agySwitch';
import {
  codexSheet,
  daemonFacts,
  externalSheet,
  keyFacts,
  keyMode,
  keyRefusalText,
  longName,
  PLUGINS_HINT,
  PLUGINS_LABEL,
  pluginsSheet,
  targetId,
} from '../lib/codex';
import { dayClock } from '../lib/switching';
import type { AgyHow, CodexKeyPreview, CodexTarget, CodexView, Model, Sessions } from '../lib/model';
import {
  claudeSheet,
  guide,
  pauseOptions,
  pauseSheet,
  type HoldChoice,
  type PauseChoice,
} from '../lib/switching';
import { CheckLine, Command, DiffFile, DiffLine, DiffView, KeyMasked } from './blocks';
import { Button, ChoiceGroup, CODE, Facts as FactList, FeedbackLine, FeedbackText, FieldHint, Hint, KVT, LinkButton, Rich, Spinner } from './parts';

/** A numbered how-to list. */
const HOWTO = 'm-0 mb-3 grid gap-1.5 pl-5 text-sm';

export type SheetState =
  | { kind: 'claude'; slot: number }
  | { kind: 'agy'; number: number }
  | { kind: 'pause'; preset: PauseChoice }
  | { kind: 'guide'; id: 'auto' | 'switcher' }
  | { kind: 'codex'; target: CodexTarget }
  /** Restart Codex's background process; `after` straight after a switch whose box was ticked. */
  | { kind: 'daemon'; after: boolean }
  | { kind: 'codexExternal' }
  | { kind: 'codexInfo'; id: 'login' | 'backup' }
  /** "Dán khoá…" for the custom provider. */
  | { kind: 'codexKey' }
  /** The plugins box changed while Codex is on the custom provider: apply it now? */
  | { kind: 'codexPlugins'; on: boolean };

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
  /** The guide to stop using CC Switch, in the Codex tab. */
  onCodexGuide?(): void;
  /** "Dùng plugin khi chạy Custom API": kept for the next switch. */
  onCodexPlugins?(on: boolean): void;
  /** "Dán khoá": the preview; the key comes back only as its hint. */
  onCodexKeyPreview?(key: string, baseUrl: string | null): Promise<CodexKeyPreview>;
  /** Writes the key the preview showed, with what its undo puts back. */
  onCodexSetKey?(key: string, baseUrl: string | null, base: string, undo: NonNullable<CodexView['keyUndo']> | null): void;
}

function ClaudeBody(props: SheetBodyProps & { slot: number }) {
  const [choice, setChoice] = useState<HoldChoice>('60');
  const v = claudeSheet(props.model, props.slot, choice, props.sessions, props.now);
  if (!v) {
    return (
      <>
        <DialogTitle>Không còn tài khoản này</DialogTitle>
        <DialogLead>claude-swap không còn liệt kê tài khoản {props.slot}. Bấm Làm mới rồi thử lại.</DialogLead>
        <DialogButtons>
          <Button large primary onClick={props.onClose}>
            Đóng
          </Button>
        </DialogButtons>
      </>
    );
  }
  if (props.working) {
    return (
      <>
        <DialogTitle>{v.confirm.replace('Chuyển sang', 'Đang chuyển sang')}…</DialogTitle>
        <FeedbackLine tone="info" style={{ margin: '8px 0 0' }}>
          <Spinner />
          <div>
            <FeedbackText title={'cswap switch ' + props.slot + ' đang chuyển tài khoản'} body="Sau đó app đợi 5 giây rồi đọc lại tài khoản đang dùng để kiểm." />
          </div>
        </FeedbackLine>
      </>
    );
  }
  // The hold ends at the time its choice shows.
  const confirm = () => props.onSwitch(props.slot, v.options.find((o) => o.v === choice)?.until ?? null);
  return (
    <>
      <DialogTitle>{v.title}</DialogTitle>
      <Facts facts={v.facts} />
      <ChoiceGroup<HoldChoice> id="qs-hold" label="Giữ tài khoản này" options={v.options} value={choice} onChange={setChoice} />
      <Hint tone={v.hint.tone} aria-live="polite">
        {v.hint.text}
      </Hint>
      <DialogButtons>
        {v.blocked ? (
          <LinkButton style={{ marginRight: 'auto' }} onClick={props.onSettings}>
            Mở cài đặt
          </LinkButton>
        ) : null}
        <Button large onClick={props.onClose}>
          Thôi
        </Button>
        <Button large primary disabled={v.blocked} onClick={confirm}>
          {v.confirm}
        </Button>
      </DialogButtons>
    </>
  );
}

function AgyBody(props: SheetBodyProps & { number: number }) {
  const v = agySheet(props.model, props.number, props.agyApp);
  if (!v) {
    return (
      <>
        <DialogTitle>Không còn tài khoản này</DialogTitle>
        <DialogLead>Tài khoản Antigravity {props.number} không còn trong danh sách đăng nhập của app. Bấm Làm mới rồi thử lại.</DialogLead>
        <DialogButtons>
          <Button large primary onClick={props.onClose}>
            Đóng
          </Button>
        </DialogButtons>
      </>
    );
  }
  if (props.working) {
    return (
      <>
        <DialogTitle>{v.title.replace('Chuyển', 'Đang chuyển').replace('?', '…')}</DialogTitle>
        <FeedbackLine tone="info" style={{ margin: '8px 0 0' }}>
          <Spinner />
          <div>
            <FeedbackText title="Đang chuyển tài khoản Antigravity" body={agyWorking(props.agyHow)} />
          </div>
        </FeedbackLine>
      </>
    );
  }
  return (
    <>
      <DialogTitle>{v.title}</DialogTitle>
      <Facts facts={v.facts} />
      <DialogButtons>
        <Button large onClick={props.onClose}>
          Thôi
        </Button>
        {v.buttons.map((b) => (
          <Button key={b.how} large primary={b.primary} disabled={v.waiting} onClick={() => props.onSwitchAgy(props.number, b.how)}>
            {b.label}
          </Button>
        ))}
      </DialogButtons>
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
      <DialogTitle>{v.title}</DialogTitle>
      <DialogLead>{v.lead}</DialogLead>
      <ChoiceGroup<PauseChoice> id="qs-pause" label="Trong bao lâu" options={v.options} value={value} columns={3} onChange={setChoice} />
      <Hint aria-live="polite">{v.hint}</Hint>
      <DialogButtons>
        <Button large onClick={props.onClose}>
          Thôi
        </Button>
        <Button large primary onClick={confirm}>
          Tạm dừng
        </Button>
      </DialogButtons>
    </>
  );
}

function GuideBody(props: SheetBodyProps & { id: 'auto' | 'switcher' }) {
  const g = guide(props.id);
  return (
    <>
      <DialogTitle>{g.title}</DialogTitle>
      <DialogLead>{g.lead}</DialogLead>
      <ol className={HOWTO}>
        {g.steps.map((s) => (
          <li key={s}>
            <Rich text={s} />
          </li>
        ))}
      </ol>
      <DialogButtons>
        <Button large primary onClick={props.onClose}>
          Đóng
        </Button>
      </DialogButtons>
    </>
  );
}

function Facts({ facts }: { facts: { k: string; text: string }[] }) {
  return (
    <FactList items={facts.map((f) => ({ k: f.k, key: f.text, text: <Rich text={f.text} /> }))} />
  );
}

function Working({ title, line, detail }: { title: string; line: string; detail: string }) {
  return (
    <>
      <DialogTitle>{title}</DialogTitle>
      <FeedbackLine tone="info" style={{ margin: '8px 0 0' }}>
        <Spinner />
        <div>
          <FeedbackText title={line} body={detail} />
        </div>
      </FeedbackLine>
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
      <DialogTitle>{v.title}</DialogTitle>
      <DialogLead>{v.lead}</DialogLead>
      <Facts facts={v.facts} />
      {props.target === 'custom' && props.onCodexPlugins ? (
        <CheckLine checked={props.model.codex.plugins} disabled={v.blocked} onChange={props.onCodexPlugins}>
          <span>
            {PLUGINS_LABEL}
            <small>{PLUGINS_HINT}</small>
          </span>
        </CheckLine>
      ) : null}
      <label className="mb-2.5 flex cursor-pointer items-start gap-2 text-sm">
        <Checkbox checked={restart} disabled={v.blocked} onCheckedChange={(on) => setRestart(on === true)} />
        <span>
          Khởi động lại tiến trình nền Codex sau khi chuyển
          <small className="block text-xs text-ink2">
            Việc đang chạy trong Terminal sẽ bị ngắt. App hỏi lại một lần trước khi làm. Codex App vẫn phải tự thoát và mở lại.
          </small>
        </span>
      </label>
      <DialogButtons>
        {v.ccSwitch && props.onCodexGuide ? (
          <LinkButton style={{ marginRight: 'auto' }} onClick={props.onCodexGuide}>
            Hướng dẫn ngừng dùng CC Switch
          </LinkButton>
        ) : null}
        <Button large onClick={props.onClose}>
          Thôi
        </Button>
        <Button large primary disabled={v.blocked} onClick={() => props.onCodexSwitch?.(props.target, restart)}>
          {v.confirm}
        </Button>
      </DialogButtons>
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
      <DialogTitle>Khởi động lại tiến trình nền Codex?</DialogTitle>
      {props.after ? (
        <DialogLead>
          Đã chuyển Codex sang {name}. Bạn đã chọn khởi động lại tiến trình nền sau khi chuyển; app hỏi lại vì việc này ngắt việc đang chạy.
        </DialogLead>
      ) : null}
      {v.procs && !v.procs.daemon ? <DialogLead>Tiến trình nền Codex đang không chạy: không có gì để khởi động lại.</DialogLead> : null}
      <Facts facts={daemonFacts()} />
      <DialogButtons>
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
      </DialogButtons>
    </>
  );
}

function ExternalBody(props: SheetBodyProps) {
  const v = externalSheet(props.model);
  if (!v)
    return (
      <>
        <DialogTitle>Cấu hình Codex không còn khác</DialogTitle>
        <DialogLead>config.toml đang là bản app ghi lần cuối, hoặc bạn đã giữ bản này.</DialogLead>
        <DialogButtons>
          <Button large primary onClick={props.onClose}>
            Đóng
          </Button>
        </DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>Cấu hình Codex đã đổi ngoài app</DialogTitle>
      <DialogLead>{v.lead}</DialogLead>
      <dl className="m-0 mb-2.5 grid grid-cols-[auto_1fr] gap-x-2.5 gap-y-[3px] text-sm">
        {v.rows.map((r) => [
          <dt key={r.key + ':k'} className="text-ink2">
            {r.key}
          </dt>,
          <dd key={r.key + ':v'} className="m-0 tabular-nums">
            {r.value}
          </dd>,
        ])}
      </dl>
      <Facts facts={[{ k: '', text: 'Bản trước khi đổi vẫn còn trong thư mục sao lưu của app.' }]} />
      <DialogButtons>
        {v.back ? (
          <Button large onClick={() => props.onCodexOpen?.({ kind: 'codex', target: v.back!.target })}>
            {v.back.label}
          </Button>
        ) : null}
        <Button large primary onClick={props.onCodexKeep}>
          Giữ như bây giờ
        </Button>
      </DialogButtons>
    </>
  );
}

function CodexInfoBody(props: SheetBodyProps & { id: 'login' | 'backup' }) {
  const v = props.model.codex;
  const close = (
    <Button large primary onClick={props.onClose}>
      Đóng
    </Button>
  );
  if (props.id === 'login')
    return (
      <>
        <DialogTitle>Đăng nhập ChatGPT cho Codex</DialogTitle>
        <DialogLead>Quota Station không đăng nhập hộ và không đọc nội dung đăng nhập.</DialogLead>
        <ol className={HOWTO}>
          <li>Mở Codex App.</li>
          <li>Chọn đăng nhập bằng ChatGPT và làm theo trang đăng nhập.</li>
          <li>Quay lại tab Codex, bấm Kiểm lại.</li>
        </ol>
        <DialogButtons>{close}</DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>Thư mục sao lưu Codex</DialogTitle>
      <DialogLead>
        <span className={CODE}>{v.backupDir}</span>
      </DialogLead>
      <div className={KVT} style={{ marginBottom: 10 }}>
        {v.backups.length ? (
          v.backups.map((b, i) => [
            <span key={b + ':k'}>config.toml · {dayClock(b, props.now)}</span>,
            <span key={b + ':v'}>{i === 0 ? 'mới nhất' : ''}</span>,
          ])
        ) : (
          <span>Chưa có bản sao lưu nào: app chưa ghi config.toml lần nào.</span>
        )}
      </div>
      <DialogLead>
        Giữ 10 bản gần nhất. Bản lưu không chứa khoá: app thay giá trị của khoá bằng <span className={CODE}>&lt;redacted&gt;</span>. Lần ghi lỗi
        không đổi file đang dùng, nên chưa cần khôi phục.
      </DialogLead>
      <DialogButtons>{close}</DialogButtons>
    </>
  );
}

function CodexKeyBody(props: SheetBodyProps) {
  const v = props.model.codex;
  const mode = keyMode(v);
  // The key lives only in this sheet: gone when it closes.
  const [key, setKey] = useState('');
  const [url, setUrl] = useState('');
  const [preview, setPreview] = useState<CodexKeyPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const name = v.custom?.name ?? 'nhà cung cấp riêng';
  const close = (label = 'Thôi') => (
    <Button large onClick={props.onClose}>
      {label}
    </Button>
  );
  if (props.working)
    return <Working title={'Đang ghi khoá cho ' + name + '…'} line="Đang ghi config.toml" detail="Sao lưu bản cũ (không chứa khoá) → ghi file tạm → kiểm mã băm → thay file → đọc lại." />;
  if (mode === 'env')
    return (
      <>
        <DialogTitle>{name} đọc khoá từ biến {v.custom?.envKey}</DialogTitle>
        <DialogLead>App không ghi gì: đặt biến này rồi mở lại Codex.</DialogLead>
        <ol className={HOWTO}>
          <li>
            Thêm vào <span className={CODE}>~/.zshrc</span>:
            <Command>export {v.custom?.envKey}=&lt;khoá của bạn&gt;</Command>
          </li>
          <li>Mở Terminal mới rồi chạy codex.</li>
          <li>Codex App mở từ Dock có thể không thấy biến trong ~/.zshrc. Khi đó dùng codex trong Terminal.</li>
        </ol>
        <DialogButtons>{close('Đóng')}</DialogButtons>
      </>
    );
  if (!mode)
    return (
      <>
        <DialogTitle>Chưa ghi khoá được</DialogTitle>
        <DialogLead>{v.custom?.key === 'headers' ? keyRefusalText(v, { kind: 'headers' }) : keyRefusalText(v, { kind: v.custom ? 'shape' : 'noCustom' })}</DialogLead>
        <DialogButtons>{close('Đóng')}</DialogButtons>
      </>
    );
  const catalog = v.ccSwitch.traces.some((t) => t.startsWith('model_catalog_json'));
  const look = async () => {
    if (!props.onCodexKeyPreview) return;
    setChecking(true);
    setError(null);
    try {
      const p = await props.onCodexKeyPreview(key, url.trim() || null);
      if (p.refusal) setError(keyRefusalText(v, p.refusal));
      else if (p.detail || !p.hash) setError('Không đọc được config.toml (' + (p.detail ?? 'không rõ lý do') + ').');
      else setPreview(p);
    } catch {
      setError('Không gọi được lệnh xem trước. Chưa đổi gì.');
    } finally {
      setChecking(false);
    }
  };
  if (preview)
    return (
      <>
        <DialogTitle>Ghi khoá cho {name}?</DialogTitle>
        <DiffView head={<DiffFile>{v.home}/config.toml</DiffFile>}>
          {preview.lines.map((l, i) => (
            <DiffLine key={i} op={l.op}>
              {l.text}
              {l.secret ? <KeyMasked /> : null}
            </DiffLine>
          ))}
        </DiffView>
        <Facts facts={keyFacts(v, preview)} />
        <DialogButtons>
          <Button large onClick={() => setPreview(null)}>
            Sửa lại
          </Button>
          <Button
            large
            primary
            disabled={v.busy != null}
            onClick={() =>
              props.onCodexSetKey?.(key, url.trim() || null, preview.hash!, preview.undo ? { at: null, token: preview.undo, baseUrl: !!url.trim() } : null)
            }
          >
            Ghi khoá
          </Button>
        </DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>{mode === 'replace' ? 'Đổi khoá API của ' + name : 'Dán khoá API cho ' + name}</DialogTitle>
      <DialogLead>Khoá ghi vào bảng {name} trong config.toml, như CC Switch. App không hiện lại khoá, không ghi nó vào log hay bản sao lưu.</DialogLead>
      <label className="mb-2 block text-sm">
        <span className="mb-1 block font-semi">Khoá API</span>
        <Input
          type="password"
          mono
          autoComplete="off"
          spellCheck={false}
          autoFocus
          aria-label={'Khoá API của ' + name}
          placeholder="sk-…"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && key.trim()) void look();
          }}
        />
      </label>
      {v.custom?.proxy ? (
        <label className="mb-2 block text-sm">
          <span className="mb-1 block font-semi">Địa chỉ thật của {name}</span>
          <Input mono spellCheck={false} aria-label={'Địa chỉ thật của ' + name} placeholder="https://…/v1" value={url} onChange={(e) => setUrl(e.target.value)} />
          <FieldHint>Đang trỏ tới proxy của CC Switch (127.0.0.1:15721), chỉ chạy khi CC Switch mở. Để trống thì giữ nguyên.</FieldHint>
        </label>
      ) : null}
      {catalog ? <FieldHint className="mb-2">config.toml còn model_catalog_json của CC Switch: giữ file danh mục đó, đừng xoá. App không sửa dòng này.</FieldHint> : null}
      {error ? (
        <Hint tone="bad" aria-live="polite">
          {error}
        </Hint>
      ) : null}
      <DialogButtons>
        {close()}
        <Button large primary disabled={!key.trim() || checking} onClick={() => void look()}>
          Xem trước
        </Button>
      </DialogButtons>
    </>
  );
}

function CodexPluginsBody(props: SheetBodyProps & { on: boolean }) {
  const v = pluginsSheet(props.model, props.on);
  if (props.working) return <Working title="Đang ghi config.toml…" line="Đổi requires_openai_auth" detail="Sao lưu bản cũ → ghi file tạm → kiểm mã băm → thay file → đọc lại." />;
  return (
    <>
      <DialogTitle>{v.title}</DialogTitle>
      <DialogLead>Đã lưu lựa chọn cho lần chuyển sau. Codex đang dùng {props.model.codex.custom?.name}: áp dụng ngay thì app sửa config.toml.</DialogLead>
      <Facts facts={v.facts} />
      <DialogButtons>
        <Button large onClick={props.onClose}>
          Để lần chuyển sau
        </Button>
        <Button large primary disabled={props.model.codex.busy != null} onClick={() => props.onCodexSwitch?.('custom', false)}>
          Áp dụng ngay
        </Button>
      </DialogButtons>
    </>
  );
}

export function SheetBody(props: SheetBodyProps) {
  const s = props.sheet;
  if (s.kind === 'codexKey') return <CodexKeyBody {...props} />;
  if (s.kind === 'codexPlugins') return <CodexPluginsBody key={'pl' + s.on} {...props} on={s.on} />;
  if (s.kind === 'codex') return <CodexBody key={'x' + s.target} {...props} target={s.target} />;
  if (s.kind === 'daemon') return <DaemonBody {...props} after={s.after} />;
  if (s.kind === 'codexExternal') return <ExternalBody {...props} />;
  if (s.kind === 'codexInfo') return <CodexInfoBody {...props} id={s.id} />;
  if (s.kind === 'claude') return <ClaudeBody key={'c' + s.slot} {...props} slot={s.slot} />;
  if (s.kind === 'agy') return <AgyBody key={'g' + s.number} {...props} number={s.number} />;
  if (s.kind === 'pause') return <PauseBody key={'p' + s.preset} {...props} preset={s.preset} />;
  return <GuideBody {...props} id={s.id} />;
}
