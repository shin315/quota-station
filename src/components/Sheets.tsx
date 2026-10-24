// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The confirmation sheets (ConfirmSheet `.sheet` in the popover, LeaveDialog
// `.dlg` in the main window): switch Claude (F04), switch Antigravity (M4c),
// pause automatic switching (F07), the two rotation guides, and Codex (M8):
// its switch with the optional restart box, the restart of its background
// process, the file changed outside the app, and its guides and previews;
// "Dán khoá" for the custom provider and "Áp dụng ngay" for the plugins box.
// Both hosts draw the same body.

import { useState } from 'react';
import { Checkbox } from '@/components/ui/checkbox';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { agySheet, agyWorking, type AppAnswer } from '@agy/agySwitch';
import {
  codexSheet,
  daemonFacts,
  externalSheet,
  keyFacts,
  keyMode,
  keyRefusalText,
  longName,
  pluginsHint,
  pluginsLabel,
  pluginsSheet,
  targetId,
} from '../lib/codex';
import { dayClock } from '../lib/switching';
import { t } from '../lib/i18n';
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

/** What a write of config.toml goes through. */
const WRITE_STEPS = () =>
  t('Sao lưu bản cũ → ghi file tạm → kiểm mã băm → thay file → đọc lại.', 'Back up the old file → write a temporary file → check its hash → replace the file → read it again.');

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
        <DialogTitle>{t('Không còn tài khoản này', 'This account is gone')}</DialogTitle>
        <DialogLead>
          {t(
            'claude-swap không còn liệt kê tài khoản ' + props.slot + '. Bấm Làm mới rồi thử lại.',
            'claude-swap no longer lists account ' + props.slot + '. Press Refresh and try again.',
          )}
        </DialogLead>
        <DialogButtons>
          <Button large primary onClick={props.onClose}>
            {t('Đóng', 'Close')}
          </Button>
        </DialogButtons>
      </>
    );
  }
  if (props.working) {
    return (
      <>
        <DialogTitle>{t('Đang chuyển sang ' + v.name + '…', 'Switching to ' + v.name + '…')}</DialogTitle>
        <FeedbackLine tone="info" style={{ margin: '8px 0 0' }}>
          <Spinner />
          <div>
            <FeedbackText
              title={t('Đang đổi đăng nhập Claude Code sang ' + v.name, 'Switching the Claude Code sign-in to ' + v.name)}
              body={t('Sau đó app đợi 5 giây rồi đọc lại tài khoản đang dùng để kiểm.', 'Then the app waits 5 seconds and reads the account in use again to check.')}
            />
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
      {v.options.length ? (
        <>
          <ChoiceGroup<HoldChoice> id="qs-hold" label={t('Giữ tài khoản này', 'Keep this account')} options={v.options} value={choice} onChange={setChoice} />
          <Hint tone={v.hint.tone} aria-live="polite">
            {v.hint.text}
          </Hint>
        </>
      ) : null}
      <DialogButtons>
        {v.blocked ? (
          <LinkButton style={{ marginRight: 'auto' }} onClick={props.onSettings}>
            {t('Mở cài đặt', 'Open Settings')}
          </LinkButton>
        ) : null}
        <Button large onClick={props.onClose}>
          {t('Thôi', 'Cancel')}
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
        <DialogTitle>{t('Không còn tài khoản này', 'This account is gone')}</DialogTitle>
        <DialogLead>
          {t(
            'Tài khoản Antigravity ' + props.number + ' không còn trong danh sách đăng nhập của app. Bấm Làm mới rồi thử lại.',
            'Antigravity account ' + props.number + ' is no longer in the app’s sign-in list. Press Refresh and try again.',
          )}
        </DialogLead>
        <DialogButtons>
          <Button large primary onClick={props.onClose}>
            {t('Đóng', 'Close')}
          </Button>
        </DialogButtons>
      </>
    );
  }
  if (props.working) {
    return (
      <>
        <DialogTitle>{v.working}</DialogTitle>
        <FeedbackLine tone="info" style={{ margin: '8px 0 0' }}>
          <Spinner />
          <div>
            <FeedbackText title={t('Đang chuyển tài khoản Antigravity', 'Switching the Antigravity account')} body={agyWorking(props.agyHow)} />
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
          {t('Thôi', 'Cancel')}
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
      <ChoiceGroup<PauseChoice> id="qs-pause" label={t('Trong bao lâu', 'For how long')} options={v.options} value={value} columns={1} onChange={setChoice} />
      <Hint aria-live="polite">{v.hint}</Hint>
      <DialogButtons>
        <Button large onClick={props.onClose}>
          {t('Thôi', 'Cancel')}
        </Button>
        <Button large primary onClick={confirm}>
          {t('Tạm dừng', 'Pause')}
        </Button>
      </DialogButtons>
    </>
  );
}

function GuideBody(props: SheetBodyProps & { id: 'auto' | 'switcher' }) {
  const g = guide(props.id, props.model.capabilities.authorTools);
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
          {t('Đóng', 'Close')}
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
        title={t('Đang chuyển Codex sang ' + to + '…', 'Switching Codex to ' + to + '…')}
        line={t('Đang ghi config.toml', 'Writing config.toml')}
        detail={WRITE_STEPS()}
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
            {pluginsLabel()}
            <small>{pluginsHint()}</small>
          </span>
        </CheckLine>
      ) : null}
      <label className="mb-2.5 flex cursor-pointer items-start gap-2 text-sm">
        <Checkbox checked={restart} disabled={v.blocked} onCheckedChange={(on) => setRestart(on === true)} />
        <span>
          {t('Khởi động lại tiến trình nền Codex sau khi chuyển', 'Restart the Codex background process after switching')}
          <small className="block text-xs text-ink2">
            {t(
              'Việc đang chạy trong Terminal sẽ bị ngắt. App hỏi lại một lần trước khi làm. Codex App vẫn phải tự thoát và mở lại.',
              'Work running in Terminal will stop. The app asks once more first. Codex App still has to quit and reopen on its own.',
            )}
          </small>
        </span>
      </label>
      <DialogButtons>
        {v.ccSwitch && props.onCodexGuide ? (
          <LinkButton style={{ marginRight: 'auto' }} onClick={props.onCodexGuide}>
            {t('Hướng dẫn ngừng dùng CC Switch', 'How to stop using CC Switch')}
          </LinkButton>
        ) : null}
        <Button large onClick={props.onClose}>
          {t('Thôi', 'Cancel')}
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
    return (
      <Working
        title={t('Đang khởi động lại tiến trình nền Codex…', 'Restarting the Codex background process…')}
        line="codex app-server daemon restart"
        detail={t('Tiến trình nền làm xong việc đang chạy (tối đa 60 giây) rồi chạy lại.', 'The background process finishes running work (up to 60 seconds), then starts again.')}
      />
    );
  const name = longName(v, v.current === 'custom' && v.custom ? v.custom.id : 'openai');
  return (
    <>
      <DialogTitle>{t('Khởi động lại tiến trình nền Codex?', 'Restart the Codex background process?')}</DialogTitle>
      {props.after ? (
        <DialogLead>
          {t(
            'Đã chuyển Codex sang ' + name + '. Bạn đã chọn khởi động lại tiến trình nền sau khi chuyển; app hỏi lại vì việc này ngắt việc đang chạy.',
            'Switched Codex to ' + name + '. You chose to restart the background process after switching; the app asks again because this stops running work.',
          )}
        </DialogLead>
      ) : null}
      {v.procs && !v.procs.daemon ? <DialogLead>{t('Tiến trình nền Codex đang không chạy: không có gì để khởi động lại.', 'The Codex background process is not running: there is nothing to restart.')}</DialogLead> : null}
      <Facts facts={daemonFacts()} />
      <DialogButtons>
        {props.after ? (
          <Button large onClick={props.onCodexSkipRestart}>
            {t('Không khởi động lại', 'Do not restart')}
          </Button>
        ) : (
          <Button large onClick={props.onClose}>
            {t('Thôi', 'Cancel')}
          </Button>
        )}
        <Button large primary disabled={v.busy != null} onClick={() => props.onCodexRestart?.(props.after)}>
          {t('Khởi động lại', 'Restart')}
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
        <DialogTitle>{t('Cấu hình Codex không còn khác', 'The Codex settings no longer differ')}</DialogTitle>
        <DialogLead>{t('config.toml đang là bản app ghi lần cuối, hoặc bạn đã giữ bản này.', 'config.toml is the copy the app last wrote, or you kept this one.')}</DialogLead>
        <DialogButtons>
          <Button large primary onClick={props.onClose}>
            {t('Đóng', 'Close')}
          </Button>
        </DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>{t('Cấu hình Codex đã đổi ngoài app', 'The Codex settings changed outside the app')}</DialogTitle>
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
      <Facts facts={[{ k: '', text: t('Bản trước khi đổi vẫn còn trong thư mục sao lưu của app.', 'The copy from before the change is still in the app’s backup folder.') }]} />
      <DialogButtons>
        {v.back ? (
          <Button large onClick={() => props.onCodexOpen?.({ kind: 'codex', target: v.back!.target })}>
            {v.back.label}
          </Button>
        ) : null}
        <Button large primary onClick={props.onCodexKeep}>
          {t('Giữ như bây giờ', 'Keep it as it is')}
        </Button>
      </DialogButtons>
    </>
  );
}

function CodexInfoBody(props: SheetBodyProps & { id: 'login' | 'backup' }) {
  const v = props.model.codex;
  const close = (
    <Button large primary onClick={props.onClose}>
      {t('Đóng', 'Close')}
    </Button>
  );
  if (props.id === 'login')
    return (
      <>
        <DialogTitle>{t('Đăng nhập ChatGPT cho Codex', 'Sign in to ChatGPT for Codex')}</DialogTitle>
        <DialogLead>{t('Quota Station không đăng nhập hộ và không đọc nội dung đăng nhập.', 'Quota Station does not sign in for you and does not read the sign-in.')}</DialogLead>
        <ol className={HOWTO}>
          <li>{t('Mở Codex App.', 'Open Codex App.')}</li>
          <li>{t('Chọn đăng nhập bằng ChatGPT và làm theo trang đăng nhập.', 'Choose to sign in with ChatGPT and follow the sign-in page.')}</li>
          <li>{t('Quay lại tab Codex, bấm Kiểm lại.', 'Come back to the Codex tab and press Check again.')}</li>
        </ol>
        <DialogButtons>{close}</DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>{t('Thư mục sao lưu Codex', 'Codex backup folder')}</DialogTitle>
      <DialogLead>
        <span className={CODE}>{v.backupDir}</span>
      </DialogLead>
      <div className={KVT} style={{ marginBottom: 10 }}>
        {v.backups.length ? (
          v.backups.map((b, i) => [
            <span key={b + ':k'}>config.toml · {dayClock(b, props.now)}</span>,
            <span key={b + ':v'}>{i === 0 ? t('mới nhất', 'newest') : ''}</span>,
          ])
        ) : (
          <span>{t('Chưa có bản sao lưu nào: app chưa ghi config.toml lần nào.', 'No backups yet: the app has not written config.toml.')}</span>
        )}
      </div>
      <DialogLead>
        {t('Giữ 10 bản gần nhất. Bản lưu không chứa khoá: app thay giá trị của khoá bằng ', 'Keeps the 10 newest. Backups hold no key: the app replaces the key’s value with ')}
        <span className={CODE}>&lt;redacted&gt;</span>
        {t('. Lần ghi lỗi không đổi file đang dùng, nên chưa cần khôi phục.', '. A failed write does not change the file in use, so there is nothing to restore.')}
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
  const name = v.custom?.name ?? t('nhà cung cấp riêng', 'the custom provider');
  const close = (label = t('Thôi', 'Cancel')) => (
    <Button large onClick={props.onClose}>
      {label}
    </Button>
  );
  if (props.working)
    return (
      <Working
        title={t('Đang ghi khoá cho ' + name + '…', 'Writing the key for ' + name + '…')}
        line={t('Đang ghi config.toml', 'Writing config.toml')}
        detail={t(
          'Sao lưu bản cũ (không chứa khoá) → ghi file tạm → kiểm mã băm → thay file → đọc lại.',
          'Back up the old file (without the key) → write a temporary file → check its hash → replace the file → read it again.',
        )}
      />
    );
  if (mode === 'env')
    return (
      <>
        <DialogTitle>{t(name + ' đọc khoá từ biến ' + v.custom?.envKey, name + ' reads its key from the variable ' + v.custom?.envKey)}</DialogTitle>
        <DialogLead>{t('App không ghi gì: đặt biến này rồi mở lại Codex.', 'The app writes nothing: set this variable, then reopen Codex.')}</DialogLead>
        <ol className={HOWTO}>
          <li>
            {t('Thêm vào ', 'Add to ')}
            <span className={CODE}>~/.zshrc</span>:<Command>export {v.custom?.envKey}=&lt;{t('khoá của bạn', 'your key')}&gt;</Command>
          </li>
          <li>{t('Mở Terminal mới rồi chạy codex.', 'Open a new Terminal and run codex.')}</li>
          <li>
            {t(
              'Codex App mở từ Dock có thể không thấy biến trong ~/.zshrc. Khi đó dùng codex trong Terminal.',
              'Codex App opened from the Dock may not see the variable in ~/.zshrc. Then use codex in Terminal.',
            )}
          </li>
        </ol>
        <DialogButtons>{close(t('Đóng', 'Close'))}</DialogButtons>
      </>
    );
  if (!mode)
    return (
      <>
        <DialogTitle>{t('Chưa ghi khoá được', 'Could not write the key')}</DialogTitle>
        <DialogLead>{v.custom?.key === 'headers' ? keyRefusalText(v, { kind: 'headers' }) : keyRefusalText(v, { kind: v.custom ? 'shape' : 'noCustom' })}</DialogLead>
        <DialogButtons>{close(t('Đóng', 'Close'))}</DialogButtons>
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
      else if (p.detail || !p.hash) setError(t('Không đọc được config.toml (' + (p.detail ?? 'không rõ lý do') + ').', 'Could not read config.toml (' + (p.detail ?? 'unknown reason') + ').'));
      else setPreview(p);
    } catch {
      setError(t('Không gọi được lệnh xem trước. Chưa đổi gì.', 'Could not run the preview. Nothing changed.'));
    } finally {
      setChecking(false);
    }
  };
  if (preview)
    return (
      <>
        <DialogTitle>{t('Ghi khoá cho ' + name + '?', 'Write the key for ' + name + '?')}</DialogTitle>
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
            {t('Sửa lại', 'Edit')}
          </Button>
          <Button
            large
            primary
            disabled={v.busy != null}
            onClick={() =>
              props.onCodexSetKey?.(key, url.trim() || null, preview.hash!, preview.undo ? { at: null, token: preview.undo, baseUrl: !!url.trim() } : null)
            }
          >
            {t('Ghi khoá', 'Write key')}
          </Button>
        </DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>{mode === 'replace' ? t('Đổi khoá API của ' + name, 'Change ' + name + '’s API key') : t('Dán khoá API cho ' + name, 'Paste the API key for ' + name)}</DialogTitle>
      <DialogLead>
        {t(
          'Khoá ghi vào bảng ' + name + ' trong config.toml, như CC Switch. App không hiện lại khoá, không ghi nó vào log hay bản sao lưu.',
          'The key goes into the ' + name + ' table in config.toml, as CC Switch does. The app never shows the key again or writes it to a log or backup.',
        )}
      </DialogLead>
      <label className="mb-2 block text-sm">
        <span className="mb-1 block font-semi">{t('Khoá API', 'API key')}</span>
        <Input
          type="password"
          mono
          autoComplete="off"
          spellCheck={false}
          autoFocus
          aria-label={t('Khoá API của ' + name, name + '’s API key')}
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
          <span className="mb-1 block font-semi">{t('Địa chỉ thật của ' + name, name + '’s real address')}</span>
          <Input mono spellCheck={false} aria-label={t('Địa chỉ thật của ' + name, name + '’s real address')} placeholder="https://…/v1" value={url} onChange={(e) => setUrl(e.target.value)} />
          <FieldHint>
            {t(
              'Đang trỏ tới proxy của CC Switch (127.0.0.1:15721), chỉ chạy khi CC Switch mở. Để trống thì giữ nguyên.',
              'Points to CC Switch’s proxy (127.0.0.1:15721), which only runs while CC Switch is open. Leave it empty to keep it.',
            )}
          </FieldHint>
        </label>
      ) : null}
      {catalog ? <FieldHint className="mb-2">
          {t(
            'config.toml còn model_catalog_json của CC Switch: giữ file danh mục đó, đừng xoá. App không sửa dòng này.',
            'config.toml still has CC Switch’s model_catalog_json: keep that catalog file, do not delete it. The app does not change this line.',
          )}
        </FieldHint> : null}
      {error ? (
        <Hint tone="bad" aria-live="polite">
          {error}
        </Hint>
      ) : null}
      <DialogButtons>
        {close()}
        <Button large primary disabled={!key.trim() || checking} onClick={() => void look()}>
          {t('Xem trước', 'Preview')}
        </Button>
      </DialogButtons>
    </>
  );
}

function CodexPluginsBody(props: SheetBodyProps & { on: boolean }) {
  const v = pluginsSheet(props.model, props.on);
  if (props.working) return <Working title={t('Đang ghi config.toml…', 'Writing config.toml…')} line={t('Đổi requires_openai_auth', 'Changing requires_openai_auth')} detail={WRITE_STEPS()} />;
  return (
    <>
      <DialogTitle>{v.title}</DialogTitle>
      <DialogLead>
        {t(
          'Đã lưu lựa chọn cho lần chuyển sau. Codex đang dùng ' + props.model.codex.custom?.name + ': áp dụng ngay thì app sửa config.toml.',
          'Saved for the next switch. Codex uses ' + props.model.codex.custom?.name + ': applying it now makes the app edit config.toml.',
        )}
      </DialogLead>
      <Facts facts={v.facts} />
      <DialogButtons>
        <Button large onClick={props.onClose}>
          {t('Để lần chuyển sau', 'At the next switch')}
        </Button>
        <Button large primary disabled={props.model.codex.busy != null} onClick={() => props.onCodexSwitch?.('custom', false)}>
          {t('Áp dụng ngay', 'Apply now')}
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
