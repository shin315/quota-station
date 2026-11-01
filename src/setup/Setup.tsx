// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The first run (f-lang … f-done): a new install opens the main window on
// these steps once, on the commands the app already has. The language, what
// this Mac has (each service, or a read that failed), the Claude accounts
// Quota Station keeps (the one Claude Code is signed in to taken at once,
// nicknames, more with the browser or a setup-token), the Keychain, macOS
// notifications, opening at login and the menu bar, then where to go next.

import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { Glyph } from '../components/icons';
import { BannerBox, Button, Chip, ChoiceGroup, FieldHint, LinkButton, Rich, Spinner } from '../components/parts';
import { LABEL, rowClass } from '../components/page';
import { LOGIN_WAIT_MS, minutes } from '../claude/ClaudeDialogs';
import type { ClaudeAccountsState } from '../claude/useClaudeAccounts';
import { permissionChip } from '../lib/alerts';
import { accountName, localName } from '../lib/claudeAccounts';
import { appName, mac, onWindows, os, osName, resolveLang, secretStore, systemLanguages, systemSettings, t } from '../lib/i18n';
import type { ClaudeAccountRow, Language, Menubar, MenubarWindow, Model } from '../lib/model';
import type { Action } from '../lib/view';
import { LaunchAtLogin, MenubarRow, MenubarWindowRow } from '../settings/Settings';
import { EmptyBox } from '../window/MainWindow';

export type SetupStep = 'lang' | 'detect' | 'acc' | 'perm' | 'done';

export interface SetupProps {
  model: Model;
  claude: ClaudeAccountsState;
  busy: Action | null;
  /** `refresh` (Kiểm lại), `keychain` (Cấp quyền một lần), `notify:settings`. */
  onAction(act: Action): void;
  onLanguage(language: Language): void;
  /** Keeps the account Claude Code is signed in to. */
  onAddCurrent(): void;
  onSetAlias(slot: number, alias: string): void;
  onRemove(slot: number): void;
  onLaunchAtLogin(on: boolean): Promise<string | null>;
  onOpenLoginItems(): void;
  onMenubar(menubar: Menubar): void;
  onMenubarWindow(window: MenubarWindow): void;
  onFinish(open: 'window' | 'popover'): void;
  /** The line after an action that failed. */
  notice?: ReactNode;
}

/** What this Mac has, as the app read it. */
export function detected(model: Model) {
  const s = model.snapshot;
  return {
    claude: s.claude.status !== 'notSetUp',
    codex: model.codex.present,
    agy: s.agy.status !== 'notSetUp',
  };
}

function steps(model: Model): [SetupStep, string][] {
  const out: [SetupStep, string][] = [
    ['lang', t('Ngôn ngữ', 'Language')],
    ['detect', mac(t('Máy này có gì', 'This Mac'))],
  ];
  if (detected(model).claude) out.push(['acc', t('Tài khoản Claude', 'Claude accounts')]);
  out.push(['perm', os(t('Quyền và thanh menu', 'Permissions'), t('Quyền và khay hệ thống', 'Permissions'))], ['done', t('Xong', 'Done')]);
  return out;
}

function Steps({ list, at }: { list: [SetupStep, string][]; at: number }) {
  return (
    <ol className="m-0 mb-1.5 flex list-none flex-wrap gap-1.5 p-0" aria-label={t('Các bước', 'Steps')}>
      {list.map(([id, name], i) => (
        <li
          key={id}
          aria-current={i === at ? 'step' : undefined}
          className={cn(
            'rounded-chip px-2 py-0.5 text-xs font-med',
            i === at ? 'bg-accent text-on-accent' : 'bg-fill',
            i < at ? 'text-ok' : i > at ? 'text-ink2' : undefined,
          )}
        >
          {i + 1}. {name}
        </li>
      ))}
    </ol>
  );
}

function Title({ children, lead }: { children: ReactNode; lead?: string }) {
  return (
    <>
      <h1 className="m-0 text-h2 tracking-[-.01em]">{children}</h1>
      {lead ? <p className="m-0 text-sm text-ink2">{lead}</p> : null}
    </>
  );
}

/** Quay lại on the left, then the step's own buttons and the one that goes on. */
function Nav({ back, children }: { back?: () => void; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-end gap-2">
      {back ? (
        <Button className="mr-auto" onClick={back}>
          {t('Quay lại', 'Back')}
        </Button>
      ) : null}
      {children}
    </div>
  );
}

// ===== Bước 1: ngôn ngữ =====

function LangStep({ model, onLanguage }: Pick<SetupProps, 'model' | 'onLanguage'>) {
  const system = resolveLang('auto', systemLanguages());
  const systemName = system === 'vi' ? 'Tiếng Việt' : 'English';
  return (
    <>
      <Title lead={t('Đổi lại được trong Cài đặt.', 'You can change it later in Settings.')}>{t('Chào bạn. Chọn ngôn ngữ', 'Welcome. Pick a language')}</Title>
      <ChoiceGroup<Language>
        id="qs-su-lang"
        label={t('Ngôn ngữ', 'Language')}
        labelClassName="sr-only"
        columns={1}
        value={model.language}
        onChange={onLanguage}
        options={[
          {
            v: 'auto',
            t: t('Theo ' + osName() + ' (' + systemName + ')', 'Match ' + osName() + ' (' + systemName + ')'),
            s:
              system === 'vi'
                ? mac(t('Máy đặt tiếng Việt nên app dùng tiếng Việt.', 'This Mac is set to Vietnamese, so the app uses Vietnamese.'))
                : mac(t('Máy đặt tiếng Anh nên app dùng tiếng Anh.', 'This Mac is set to English, so the app uses English.')),
          },
          { v: 'vi', t: 'Tiếng Việt', s: '' },
          { v: 'en', t: 'English', s: '' },
        ]}
      />
    </>
  );
}

// ===== Bước 2: máy này có gì =====

type Mark = 'ok' | 'none' | 'note';

function Found({ mark, title, line, children }: { mark: Mark; title: string; line: string; children?: ReactNode }) {
  return (
    <li className="grid grid-cols-[16px_minmax(0,1fr)] items-start gap-x-2.5 gap-y-1 border-t-[.5px] border-card-line py-2 first:border-t-0">
      <span className={cn('mt-px grid size-4 place-items-center', mark === 'ok' ? 'text-ok' : 'text-ink2')}>
        <span className="sr-only">{mark === 'ok' ? t('có', 'found') : mark === 'note' ? t('lưu ý', 'note') : t('không có', 'not found')}</span>
        {mark === 'none' ? <i className="h-0.5 w-2.5 rounded-[1px] bg-ink3" aria-hidden="true" /> : <Glyph name={mark === 'ok' ? 'check-circle' : 'warning-circle'} className="size-3.5" />}
      </span>
      <div className="grid gap-0.5">
        <b className="text-body font-semi">{title}</b>
        <small className="text-xs text-ink2">
          <Rich text={line} />
        </small>
        {children}
      </div>
    </li>
  );
}

/** A section the app could not read: its reason, as the groups' notes say it. */
function readNote(reason: string | null): string {
  return t('Chưa đọc được hạn mức', 'Could not read the quota') + (reason ? ': ' + reason : '') + '.';
}

function DetectStep({ model }: { model: Model }) {
  const has = detected(model);
  const s = model.snapshot;
  const signedIn = s.claude.accounts.find((a) => a.active);
  const codex = model.codex;
  const none = !has.claude && !has.codex && !has.agy;
  return (
    <>
      <Title lead={mac(t('Quota Station chỉ đọc những gì đã có trên máy. Không gửi gì đi đâu.', 'Quota Station only reads what is already on this Mac. Nothing is sent anywhere.'))}>
        {mac(t('Máy này có gì', 'What this Mac has'))}
      </Title>
      {none ? (
        <BannerBox
          tone=""
          title={t('Chưa thấy agent nào', 'No agents found')}
          body={t(
            'Quota Station hiện hạn mức của Claude Code, Codex và Antigravity. Cài ít nhất một cái rồi bấm Kiểm tra lại; app vẫn mở được để bạn xem trước.',
            'Quota Station shows the quota of Claude Code, Codex and Antigravity. Install at least one and press Check again; the app still opens so you can look around.',
          )}
        />
      ) : null}
      <ul className="m-0 grid list-none rounded-tile bg-fill px-3 py-1" aria-label={mac(t('Máy này có gì', 'What this Mac has'))}>
        {has.claude ? (
          s.claude.status === 'unavailable' ? (
            <Found mark="note" title="Claude Code" line={'`~/.claude` · ' + readNote(s.claude.reason)} />
          ) : (
            <Found
              mark="ok"
              title="Claude Code"
              line={
                signedIn
                  ? signedIn.email
                    ? t('`~/.claude` · đang đăng nhập ' + signedIn.email, '`~/.claude` · signed in as ' + signedIn.email)
                    : t('`~/.claude` · đang đăng nhập', '`~/.claude` · signed in')
                  : t('`~/.claude` · đã cài, chưa đăng nhập tài khoản nào', '`~/.claude` · installed, not signed in')
              }
            />
          )
        ) : (
          <Found mark="none" title={t('Không thấy Claude Code', 'No Claude Code')} line={t('Cài Claude Code nếu bạn dùng nó; bỏ qua cũng được.', 'Install Claude Code if you use it; skipping is fine.')} />
        )}
        {has.codex ? (
          <Found
            mark={codex.config === 'ok' || codex.config === 'missing' ? 'ok' : 'note'}
            title="Codex"
            line={
              '`' +
              codex.home +
              '` · ' +
              (codex.config === 'invalid' || codex.config === 'unreadable'
                ? t('chưa đọc được config.toml', 'could not read config.toml')
                : codex.custom
                  ? t('ChatGPT, có 1 Custom API (' + codex.custom.name + ')', 'ChatGPT, 1 Custom API (' + codex.custom.name + ')')
                  : 'ChatGPT')
            }
          />
        ) : (
          <Found mark="none" title={t('Không thấy Codex', 'No Codex')} line={mac(t('Không sao: Quota Station chỉ hiện dịch vụ máy có.', 'That is fine: Quota Station only shows what this Mac has.'))} />
        )}
        {has.agy ? (
          s.agy.status === 'unavailable' ? (
            <Found mark="note" title="Antigravity" line={readNote(s.agy.reason)} />
          ) : (
            <Found mark="ok" title="Antigravity" line={t(appName('Antigravity') + ' · đọc tài khoản đang đăng nhập', appName('Antigravity') + ' · reads the signed-in account')} />
          )
        ) : (
          <Found mark="none" title={t('Không thấy Antigravity', 'No Antigravity')} line={t('Không sao.', 'That is fine.')} />
        )}
      </ul>
    </>
  );
}

// ===== Bước 3: tài khoản Claude =====

/** A nickname, saved when the field is left. */
function Nickname({ row, onSave }: { row: ClaudeAccountRow; onSave(alias: string): void }) {
  const [value, setValue] = useState(row.alias ?? '');
  const saved = useRef(row.alias ?? '');
  useEffect(() => {
    saved.current = row.alias ?? '';
    setValue(row.alias ?? '');
  }, [row.alias]);
  const who = row.email ?? accountName(row);
  return (
    <Input
      className="w-[150px]"
      value={value}
      maxLength={24}
      placeholder={localName(row)}
      aria-label={t('Tên gọi của ', 'Nickname for ') + who}
      onChange={(e) => setValue(e.target.value)}
      onBlur={() => {
        if (value.trim() !== saved.current) {
          saved.current = value.trim();
          onSave(value.trim());
        }
      }}
    />
  );
}

function WaitLine({ started, onCancel }: { started: number; onCancel(): void }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  const left = minutes(LOGIN_WAIT_MS - (now - started));
  return (
    <>
      <div className="flex items-start gap-2.5 rounded-tile bg-fill px-[11px] py-2 text-sm" role="status">
        <Spinner />
        <div className="grid gap-0.5">
          <b className="font-semi text-heading">{t('Đang chờ trình duyệt…', 'Waiting for the browser…')}</b>
          <small className="text-xs text-ink2 tabular-nums">
            {t('Đăng nhập Claude trong tab vừa mở. Chờ tối đa 10 phút · còn ' + left, 'Sign in to Claude in the tab that opened. Waits up to 10 minutes · ' + left + ' left')}
          </small>
        </div>
      </div>
      <Nav>
        <Button onClick={onCancel}>{t('Thôi', 'Cancel')}</Button>
      </Nav>
    </>
  );
}

function AccStep(props: SetupProps & { back(): void; next(): void }) {
  const { model, claude } = props;
  const v = model.claudeAccounts;
  const list = v.accounts;
  const busy = v.working != null;
  const signedIn = model.snapshot.claude.accounts.some((a) => a.active);
  // The account Claude Code is signed in to is taken at once, as drawn (f-acc-1).
  const took = useRef(false);
  useEffect(() => {
    if (!took.current && !list.length && signedIn && !busy) {
      took.current = true;
      props.onAddCurrent();
    }
  });
  const wait = claude.dialog?.kind === 'wait' && claude.dialog.slot == null ? claude.dialog : null;
  const title = <Title>{t('Tài khoản Claude', 'Claude accounts')}</Title>;
  if (wait)
    return (
      <>
        {title}
        <WaitLine started={wait.started} onCancel={claude.cancelLogin} />
      </>
    );
  const browser = () => void claude.login(null);
  const token = () => claude.open({ kind: 'token', error: null });
  if (!list.length)
    return (
      <>
        <Title
          lead={mac(
            t(
              'Quota Station giữ đăng nhập của từng tài khoản trong ' + secretStore() + ' của máy này, để chuyển Claude Code giữa chúng. Một tài khoản cũng được.',
              'Quota Station keeps each account’s sign-in in this Mac’s ' + secretStore() + ' so it can switch Claude Code between them. One account is fine.',
            ),
          )}
        >
          {t('Tài khoản Claude', 'Claude accounts')}
        </Title>
        {busy && v.working === 'current' ? (
          <p className="m-0 flex items-center gap-2 text-sm text-ink2" role="status">
            <Spinner />
            {t('Đang lấy tài khoản Claude Code đang đăng nhập…', 'Taking the account Claude Code is signed in to…')}
          </p>
        ) : (
          <EmptyBox title={t('Claude Code chưa đăng nhập tài khoản nào', 'Claude Code is not signed in to any account')} body={t('Đăng nhập tài khoản đầu tiên ngay đây.', 'Sign in to your first account right here.')}>
            <Button small primary disabled={busy || !v.claude} onClick={browser}>
              {t('Đăng nhập trong trình duyệt', 'Sign in with the browser')}
            </Button>
            <Button small disabled={busy} onClick={token}>
              {t('Dán setup-token', 'Paste a setup-token')}
            </Button>
          </EmptyBox>
        )}
        <Nav back={props.back}>
          <Button primary onClick={props.next}>
            {t('Bỏ qua, thêm sau', 'Skip, add later')}
          </Button>
        </Nav>
      </>
    );
  return (
    <>
      <Title
        lead={
          list.length === 1
            ? t(
                'Đã thấy tài khoản Claude Code đang đăng nhập. Thêm tài khoản khác nếu bạn có nhiều tài khoản; một tài khoản cũng được.',
                'Found the account Claude Code is signed in to. Add others if you have several; one is fine.',
              )
            : t(
                'Mỗi tài khoản có tên gọi riêng (để trống thì dùng phần trước @). Thứ tự là thứ tự hiện trong popover.',
                'Each account can have a nickname (empty uses the part before @). The order is the order in the popover.',
              )
        }
      >
        {t('Tài khoản Claude', 'Claude accounts')}
      </Title>
      {/* One grid for the head and the rows: "Tên gọi" stands over the nicknames, and every
          nickname lines up whether or not its row has "Bỏ". */}
      <section
        className="material-card grid grid-cols-[minmax(0,1fr)_150px_auto] gap-x-3 rounded-card px-3.5 py-3"
        aria-label={t('Tài khoản Claude', 'Claude accounts')}
      >
        <div className="col-span-full grid grid-cols-subgrid items-baseline pb-2">
          <b className="text-sm font-bold text-ink2">{t(list.length + ' tài khoản', list.length + (list.length === 1 ? ' account' : ' accounts'))}</b>
          <span className="text-xs text-ink3">{t('Tên gọi', 'Nickname')}</span>
        </div>
        <ul className="col-span-full m-0 grid list-none grid-cols-subgrid gap-y-2 p-0">
          {list.map((row) => (
            <li key={row.slot} className="col-span-full grid grid-cols-subgrid items-center">
              <div className="grid min-w-0 flex-1 gap-0.5">
                <b className="truncate text-body font-semi">{row.email ?? accountName(row)}</b>
                <small className="text-xs text-ink2">
                  {row.kind === 'setupToken' ? t('Setup-token', 'Setup token') : t('Đăng nhập bằng trình duyệt', 'Browser sign-in')}
                  {row.active ? t(' · Claude Code đang đăng nhập', ' · Claude Code is signed in to it') : ''}
                </small>
              </div>
              <Nickname row={row} onSave={(alias) => props.onSetAlias(row.slot, alias)} />
              {/* The account Claude Code is signed in to stays: switching needs it. */}
              {row.active ? <span /> : (
                <LinkButton small disabled={busy} onClick={() => props.onRemove(row.slot)} aria-label={t('Bỏ ', 'Remove ') + (row.email ?? accountName(row))}>
                  {t('Bỏ', 'Remove')}
                </LinkButton>
              )}
            </li>
          ))}
        </ul>
      </section>
      <div className="flex flex-wrap gap-2">
        <Button small lead="plus" disabled={busy || !v.claude} onClick={browser}>
          {t('Thêm tài khoản khác…', 'Add another account…')}
        </Button>
        <Button small disabled={busy} onClick={token}>
          {t('Dán setup-token', 'Paste a setup-token')}
        </Button>
      </div>
      <Nav back={props.back}>
        <Button primary onClick={props.next}>
          {t('Tiếp tục', 'Continue')}
        </Button>
      </Nav>
    </>
  );
}

// ===== Bước 4: quyền và thanh menu =====

function Row({ title, line, children }: { title: string; line?: string; children: ReactNode }) {
  return (
    <div className={rowClass()}>
      <div className={LABEL}>{title}</div>
      <div>
        <div className="flex items-center gap-2">{children}</div>
        {line ? <FieldHint>{line}</FieldHint> : null}
      </div>
    </div>
  );
}

/** The step's name: the menu bar, or Windows' tray. */
const permTitle = (): string => os(t('Quyền và thanh menu', 'Permissions and menu bar'), t('Quyền và khay hệ thống', 'Permissions and system tray'));

function PermStep(props: SetupProps) {
  const { model, busy, onAction } = props;
  const keychain = model.snapshot.keychain;
  const notify = model.alerts.permission;
  return (
    <>
      <Title>{permTitle()}</Title>
      <section className="material-card grid rounded-card px-3.5 py-3" aria-label={permTitle()}>
        {/* Windows asks nothing before the app reads Claude Code's sign-in. */}
        {onWindows() ? null : (
          <Row title={t('Quyền Keychain', 'Keychain access')} line={t('Để đọc hạn mức của từng tài khoản Claude. macOS hỏi một lần.', 'To read each Claude account’s quota. macOS asks once.')}>
            {keychain === 'granted' ? (
              <Chip chip={{ t: t('đã cấp', 'granted'), k: 'ok' }} />
            ) : keychain === 'required' ? (
              <Button small disabled={busy === 'keychain'} onClick={() => onAction('keychain')}>
                {t('Cấp quyền một lần', 'Grant once')}
              </Button>
            ) : (
              <Chip chip={{ t: t('chưa rõ', 'unknown'), k: '' }} />
            )}
          </Row>
        )}
        <Row title={t('Thông báo', 'Notifications')} line={t('Báo khi hạn mức sắp cạn hoặc một tài khoản cần đăng nhập lại.', 'Tells you when quota is running out or an account needs to sign in again.')}>
          {notify === 'allowed' || notify === 'unbundled' ? (
            <Chip chip={permissionChip(notify)} />
          ) : (
            <Button small onClick={() => onAction('notify:settings')}>
              {notify === 'denied' ? t('Mở ', 'Open ') + systemSettings() : t('Cho phép', 'Allow')}
            </Button>
          )}
        </Row>
        {/* Every row as Cài đặt draws it: its name on the left, its control on the right. */}
        <LaunchAtLogin app={model.app} onSet={props.onLaunchAtLogin} onOpen={props.onOpenLoginItems} />
        <MenubarRow model={model} onPick={props.onMenubar} />
        <MenubarWindowRow model={model} onPick={props.onMenubarWindow} />
      </section>
    </>
  );
}

// ===== Bước 5: xong =====

function DoneStep({ model }: { model: Model }) {
  const has = detected(model);
  const n = model.claudeAccounts.accounts.length;
  const items = [
    has.claude
      ? 'Claude: ' + (n ? t(n + ' tài khoản', n + (n === 1 ? ' account' : ' accounts')) : t('chưa có tài khoản (thêm ở tab Claude)', 'no account yet (add one in the Claude tab)'))
      : '',
    has.codex ? 'Codex: ChatGPT' + (model.codex.custom ? ' · ' + model.codex.custom.name : '') : '',
    has.agy ? t('Antigravity: tài khoản đang đăng nhập (chỉ đọc)', 'Antigravity: the signed-in account (read-only)') : '',
  ].filter(Boolean);
  return (
    <>
      <Title>{t('Xong', 'All set')}</Title>
      <p className="m-0 text-sm text-ink">{os(
          t('Quota Station nằm trên thanh menu. Bấm vào để xem hạn mức và chuyển tài khoản.', 'Quota Station lives in the menu bar. Click it to see quota and switch accounts.'),
          t('Quota Station nằm ở khay hệ thống, góc phải thanh tác vụ. Bấm vào để xem hạn mức và chuyển tài khoản.', 'Quota Station lives in the system tray, at the right of the taskbar. Click it to see quota and switch accounts.'),
        )}</p>
      {items.length ? (
        <ul className="m-0 grid list-disc gap-2 pl-3.5 text-sm text-ink marker:text-ink3">
          {items.map((x) => (
            <li key={x}>{x}</li>
          ))}
        </ul>
      ) : null}
    </>
  );
}

export function Setup(props: SetupProps) {
  const { model, claude } = props;
  const list = steps(model);
  const [step, setStep] = useState<SetupStep>('lang');
  // A step that went away (no Claude Code any more) gives way to the next.
  const at = Math.max(
    0,
    list.findIndex(([id]) => id === step),
  );
  const current = list[at][0];
  const go = (i: number) => setStep(list[Math.min(Math.max(i, 0), list.length - 1)][0]);
  const back = at > 0 ? () => go(at - 1) : undefined;
  const next = () => go(at + 1);
  // An account added here shows in the list with its nickname field, not in a dialog.
  useEffect(() => {
    if (claude.dialog?.kind === 'done' || claude.dialog?.kind === 'dup') claude.close();
  }, [claude]);
  const has = detected(model);
  return (
    <div className="mx-auto grid w-full max-w-[680px] content-start gap-3 px-6 pt-11 pb-6">
      <Steps list={list} at={at} />
      {props.notice}
      {current === 'lang' ? (
        <>
          <LangStep model={model} onLanguage={props.onLanguage} />
          <Nav>
            <Button primary onClick={next}>
              {t('Tiếp tục', 'Continue')}
            </Button>
          </Nav>
        </>
      ) : null}
      {current === 'detect' ? (
        <>
          <DetectStep model={model} />
          <Nav back={back}>
            <Button disabled={props.busy === 'refresh'} onClick={() => props.onAction('refresh')}>
              {t('Kiểm tra lại', 'Check again')}
            </Button>
            <Button primary onClick={next}>
              {has.claude || has.codex || has.agy ? t('Tiếp tục', 'Continue') : t('Vẫn tiếp tục', 'Continue anyway')}
            </Button>
          </Nav>
        </>
      ) : null}
      {current === 'acc' ? <AccStep {...props} back={() => go(at - 1)} next={next} /> : null}
      {current === 'perm' ? (
        <>
          <PermStep {...props} />
          <Nav back={back}>
            <Button primary onClick={next}>
              {t('Tiếp tục', 'Continue')}
            </Button>
          </Nav>
        </>
      ) : null}
      {current === 'done' ? (
        <>
          <DoneStep model={model} />
          <Nav back={back}>
            <Button onClick={() => props.onFinish('window')}>{t('Mở cửa sổ chính', 'Open the main window')}</Button>
            <Button primary onClick={() => props.onFinish('popover')}>
              {t('Mở popover', 'Open the popover')}
            </Button>
          </Nav>
        </>
      ) : null}
    </div>
  );
}
