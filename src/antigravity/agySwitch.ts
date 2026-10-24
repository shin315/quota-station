// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// What switching Antigravity shows (M4c): the confirmation sheet with the
// plan's three buttons, the feedback after a switch, and the banner when a
// running Antigravity.app wrote its old login back within six hours. Pure,
// like `switching.ts`: the same model and answers give the same words.

import { clock } from '../lib/format';
import { t } from '../lib/i18n';
import type { AgyApp, AgyHow, AgyLogin, AgyOutcome, Model } from '../lib/model';
import type { FactView, NameOf } from '../lib/switching';
import { agyCells, agyGroups } from '../lib/accounts';
import { agyName, agyNames, type Action, type BannerView, type FeedbackView } from '../lib/view';

/** `Ultra 2`, by the number the row and the app's list share. */
export const agyNameOf: NameOf = (n) => agyName({ number: n }).name;

/** Names by the model's accounts: their email while names show, else Ultra N. */
export function agyNamesOf(model: Model | null | undefined): NameOf {
  if (!model) return agyNameOf;
  const names = agyNames(model.snapshot.agy.accounts, model.advanced.on);
  return (n) => names.get(n)?.name ?? agyNameOf(n);
}

/** The account logged in here with that number, when the row lists the app's own accounts. */
export function agyLoginOf(model: Model, number: number): AgyLogin | null {
  if (!model.agySwitch?.listed) return null;
  return model.antigravity?.accounts.find((a) => a.number === number) ?? null;
}

/** What the sheet knows about Antigravity.app: still asking, could not ask, or the answer. */
export type AppAnswer = AgyApp | null | 'loading';

export interface AgyButton {
  how: AgyHow;
  label: string;
  primary: boolean;
}

export interface AgySheetView {
  title: string;
  /** The title while the switch runs. */
  working: string;
  facts: FactView[];
  /** The switch buttons, left to right after "Thôi"; empty when the switch cannot run. */
  buttons: AgyButton[];
  /** Waiting for the answer about Antigravity.app: the buttons show, switched off. */
  waiting: boolean;
}

/** The sheet for switching to account `number`; null when it is not one of the app's accounts. */
export function agySheet(model: Model, number: number, app: AppAnswer): AgySheetView | null {
  const login = agyLoginOf(model, number);
  const target = model.snapshot.agy.accounts.find((a) => a.number === number);
  if (!login || !target) return null;
  const name = agyNamesOf(model)(number);
  const facts: FactView[] = [];
  // Each model group's tighter window, as the share used. No reset is read
  // here, so the clock does not matter.
  const parts = agyGroups(agyCells(target, new Date(0)))
    .filter((g) => g.cell)
    .map((g) => '**' + g.cell!.text + '** ' + g.label + ' ' + g.cell!.unit);
  if (parts.length) facts.push({ k: '', text: t(name + ' đã dùng ' + parts.join(', ') + '.', name + ' has used ' + parts.join(', ') + '.') });

  let blocked = false;
  if (login.status === 'needsLogin') {
    blocked = true;
    facts.push({
      k: 'x',
      text: t(name + ' cần đăng nhập lại trong tab Antigravity trước khi chuyển.', name + ' needs to sign in again in the Antigravity tab before switching.'),
    });
  }
  if (app === 'loading') facts.push({ k: '', text: t('Đang xem Antigravity.app có đang chạy không…', 'Checking whether Antigravity.app is running…') });
  else if (app !== null && app.installed === 'tooOld') {
    blocked = true;
    facts.push({
      k: 'x',
      text: t(
        'Antigravity.app bản ' + (app.version ?? '?') + ' quá cũ: app chỉ chuyển được Antigravity.app 2.0 trở lên. Cập nhật Antigravity.app rồi thử lại.',
        'Antigravity.app ' + (app.version ?? '?') + ' is too old: the app can only switch Antigravity.app 2.0 or later. Update Antigravity.app, then try again.',
      ),
    });
  } else if (app !== null && app.installed === 'unreadable') {
    blocked = true;
    facts.push({
      k: 'x',
      text: t(
        'Không đọc được phiên bản Antigravity.app, nên app không biết bản này giữ đăng nhập ở đâu. Mở lại bảng này sau, hoặc cài lại Antigravity.app.',
        'Could not read the Antigravity.app version, so the app does not know where this version keeps its sign-in. Open this sheet again later, or reinstall Antigravity.app.',
      ),
    });
  } else if (app === null || (app.installed === 'supported' && app.running === null))
    facts.push({ k: 'w', text: t('Không biết Antigravity.app có đang chạy không.', 'Could not tell whether Antigravity.app is running.') });
  else if (app.installed === 'missing')
    facts.push({ k: '', text: t('Máy này không có Antigravity.app: chỉ đổi tài khoản của agy.', 'This Mac has no Antigravity.app: only agy’s account changes.') });
  else if (app.running)
    facts.push({
      k: 'w',
      text: t(
        '**Antigravity.app đang chạy.** App đang chạy có thể ghi lại tài khoản cũ, nên thoát app rồi chuyển là chắc nhất.',
        '**Antigravity.app is running.** A running app can write the old account back, so quitting it first is safest.',
      ),
    });
  else facts.push({ k: '', text: t('Antigravity.app không chạy.', 'Antigravity.app is not running.') });
  facts.push({ k: 'w', text: t('Phiên agy đang chạy giữ tài khoản cũ cho tới khi khởi động lại.', 'Running agy sessions keep the old account until they restart.') });

  // The plan's three buttons while the app runs (or may run, or the answer
  // is not in yet); only "Chuyển" when it does not.
  const mayRun = app === null || app === 'loading' || (app.installed === 'supported' && app.running !== false);
  const buttons: AgyButton[] = blocked
    ? []
    : mayRun
      ? [
          { how: 'keepRunning', label: t('Chuyển, để app chạy', 'Switch, keep the app open'), primary: false },
          { how: 'quitFirst', label: t('Thoát app rồi chuyển', 'Quit the app, then switch'), primary: true },
        ]
      : [{ how: 'plain', label: t('Chuyển', 'Switch'), primary: true }];
  return {
    title: t('Chuyển Antigravity sang ' + name + '?', 'Switch Antigravity to ' + name + '?'),
    working: t('Đang chuyển Antigravity sang ' + name + '…', 'Switching Antigravity to ' + name + '…'),
    facts,
    buttons,
    waiting: app === 'loading',
  };
}

/** The line under the working sheet's title; `null` for a switch started in the other window. */
export function agyWorking(how: AgyHow | null): string {
  const write = 'đăng nhập dùng chung của Antigravity.app và agy';
  const check = 'rồi đợi 5 giây để đọc lại tài khoản agy đang dùng.';
  const writeEn = 'the shared sign-in of Antigravity.app and agy';
  const checkEn = 'then waits 5 seconds to read back the account agy uses.';
  if (how === 'quitFirst')
    return t('Thoát Antigravity.app, ghi ' + write + ', mở lại app, ' + check, 'Quits Antigravity.app, writes ' + writeEn + ', reopens the app, ' + checkEn);
  // Not known which button: only what every switch does.
  if (how === null) return t('Ghi ' + write + ', ' + check, 'Writes ' + writeEn + ', ' + checkEn);
  return t('Làm mới đăng nhập, ghi ' + write + ', ' + check, 'Refreshes the sign-in, writes ' + writeEn + ', ' + checkEn);
}

// ===== Feedback after a switch =====

const close = () => ({ label: t('Đóng', 'Close'), act: 'closeFeedback' as Action });
const refresh = () => ({ label: t('Làm mới', 'Refresh'), act: 'refresh' as Action });

function tails(o: { how: AgyHow; reopened: boolean | null; saved: boolean }): { text: string; warn: boolean } {
  let text = '';
  let warn = false;
  if (o.how === 'keepRunning')
    text += t(
      ' Antigravity.app vẫn chạy và có thể ghi lại tài khoản cũ; app sẽ báo nếu bị ghi đè trong 6 giờ tới.',
      ' Antigravity.app is still running and may write the old account back; the app tells you if that happens in the next 6 hours.',
    );
  if (o.reopened === false) {
    text += t(' Chưa mở lại được Antigravity.app, hãy mở bằng tay.', ' Could not reopen Antigravity.app; open it by hand.');
    warn = true;
  }
  if (!o.saved) {
    text += t(
      ' Chưa ghi được danh sách tài khoản của app, nên app có thể chưa nhận ra tài khoản mới.',
      ' Could not write the app’s account list, so the app may not pick up the new account yet.',
    );
    warn = true;
  }
  return { text, warn };
}

function failedBody(error: string, detail: string, reopened: boolean | null): string {
  const nothing = t(' Chưa đổi gì.', ' Nothing changed.');
  const d = detail ? ' (' + detail + ')' : '';
  switch (error) {
    case 'busy':
      return t('Một lần chuyển khác đang chạy. Chờ nó xong rồi thử lại.', 'Another switch is running. Wait for it to finish, then try again.');
    case 'unknownAccount':
      return t('Tài khoản này không còn trong danh sách của app. Bấm Làm mới rồi thử lại.', 'This account is no longer in the app’s list. Press Refresh, then try again.');
    case 'index':
      return t('Không đọc được danh sách tài khoản của app.', 'Could not read the app’s account list.') + nothing;
    case 'needsLogin':
      return (
        t(
          'Google không nhận đăng nhập của tài khoản này nữa. Đăng nhập lại trong tab Antigravity rồi thử lại.',
          'Google no longer accepts this account’s sign-in. Sign in again in the Antigravity tab, then try again.',
        ) + nothing
      );
    case 'oldApp':
      return (
        t(
          'Antigravity.app bản ' + (detail || '?') + ' quá cũ: app chỉ chuyển được bản 2.0 trở lên.',
          'Antigravity.app ' + (detail || '?') + ' is too old: the app can only switch version 2.0 or later.',
        ) + nothing
      );
    case 'appVersion':
      return t('Không đọc được phiên bản Antigravity.app' + d + ', nên app không chuyển.', 'Could not read the Antigravity.app version' + d + ', so the app did not switch.') + nothing;
    case 'loggingOut':
      return t('Đang đăng xuất một tài khoản Antigravity. Chờ xong rồi thử lại.', 'An Antigravity account is signing out. Wait for it to finish, then try again.');
    case 'loggingIn':
      return t('Đang có một lần đăng nhập Antigravity. Chờ xong rồi thử lại.', 'An Antigravity sign-in is in progress. Wait for it to finish, then try again.');
    case 'prepareTimeout':
      return t('Google không trả lời kịp trong 60 giây.', 'Google did not answer within 60 seconds.') + nothing;
    case 'prepare':
      return t('Bước chuẩn bị đăng nhập dừng giữa chừng.', 'Getting the sign-in ready stopped partway.') + nothing;
    case 'refresh':
      return t('Không làm mới được đăng nhập của tài khoản này' + d + '.', 'Could not refresh this account’s sign-in' + d + '.') + nothing;
    case 'client':
      return t('Thiếu hoặc sai oauth-client.json trên máy này.', 'oauth-client.json is missing or wrong on this Mac.') + nothing;
    case 'keychain':
      return t('Không đọc được đăng nhập của tài khoản này trong Keychain của app.', 'Could not read this account’s sign-in from the app’s Keychain.') + nothing;
    case 'notQuit':
      return t(
        'Antigravity.app không thoát trong 20 giây, có thể đang hỏi lưu việc. App không ép thoát và chưa đổi đăng nhập nào. Đóng Antigravity.app rồi thử lại, hoặc chọn "Chuyển, để app chạy".',
        'Antigravity.app did not quit within 20 seconds; it may be asking to save work. The app does not force it to quit and has not changed any sign-in. Close Antigravity.app and try again, or choose “Switch, keep the app open”.',
      );
    case 'appRunning':
      return (
        t('Antigravity.app vừa mở lại sau khi bảng chuyển kiểm tra.', 'Antigravity.app opened again after the switch sheet checked it.') +
        nothing +
        t(' Bấm Thử lại để chọn thoát app rồi chuyển, hoặc để app chạy.', ' Press Try again to quit the app and then switch, or to keep it open.')
      );
    case 'appState':
      return t('Không kiểm được Antigravity.app có đang chạy không.', 'Could not check whether Antigravity.app is running.') + nothing;
    case 'write':
    case 'writeTimeout': {
      const why = error === 'writeTimeout' ? t('security không trả lời trong 15 giây', 'security did not answer within 15 seconds') : detail;
      const back =
        reopened === true
          ? t(' Antigravity.app đã mở lại.', ' Antigravity.app has reopened.')
          : reopened === false
            ? t(' Chưa mở lại được Antigravity.app, hãy mở bằng tay.', ' Could not reopen Antigravity.app; open it by hand.')
            : '';
      const w = why ? ' (' + why + ')' : '';
      return (
        t(
          'Không ghi được đăng nhập dùng chung vào Keychain' + w + '. Đăng nhập cũ vẫn giữ nguyên.',
          'Could not write the shared sign-in to the Keychain' + w + '. The old sign-in is unchanged.',
        ) + back
      );
    }
    default:
      return t('Không gọi được lệnh chuyển.', 'Could not run the switch.') + nothing;
  }
}

/** Errors worth a second try as they are. */
const RETRY = new Set(['prepareTimeout', 'prepare', 'refresh', 'notQuit', 'appState', 'appRunning', 'appVersion', 'write', 'writeTimeout']);

export function agySwitchFeedback(outcome: AgyOutcome, model?: Model | null): FeedbackView {
  const nameOf = agyNamesOf(model);
  const n = nameOf(outcome.number);
  switch (outcome.kind) {
    case 'switched': {
      const tail = tails(outcome);
      const by =
        outcome.basis === 'binding'
          ? t(' (theo tài khoản app vừa ghi; agy chưa có số để đối chiếu)', ' (going by the account the app just wrote; agy has no numbers to compare yet)')
          : '';
      return {
        // Antigravity.app kept running may write the old account back: say so, not as a passing toast.
        tone: tail.warn ? 'warn' : outcome.how === 'keepRunning' ? 'info' : '',
        title: t('Đã chuyển Antigravity sang ' + n + '.', 'Switched Antigravity to ' + n + '.'),
        body:
          t('Đọc lại sau 5 giây: agy đang dùng ' + n + by + ' ✓.', 'Read again after 5 seconds: agy uses ' + n + by + ' ✓.') +
          tail.text +
          t(' Phiên agy đang chạy vẫn dùng tài khoản cũ cho tới khi khởi động lại.', ' Running agy sessions keep the old account until they restart.'),
        actions: [],
      };
    }
    case 'unconfirmed': {
      const tail = tails(outcome);
      return {
        tone: 'warn',
        title: t('Đã ghi tài khoản ' + n + ' cho Antigravity', 'Wrote ' + n + ' for Antigravity'),
        body:
          (outcome.active != null
            ? t(
                'Nhưng đọc lại sau 5 giây, agy vẫn đang dùng ' + nameOf(outcome.active) + '. Antigravity.app hoặc phiên agy đang chạy có thể vẫn giữ tài khoản cũ.',
                'But reading again after 5 seconds, agy still uses ' + nameOf(outcome.active) + '. Antigravity.app or a running agy session may still hold the old account.',
              )
            : t('Nhưng app chưa đọc lại được tài khoản agy đang dùng để kiểm.', 'But the app could not read back which account agy uses to check.')) + tail.text,
        actions: [refresh(), close()],
      };
    }
    case 'failed':
      return {
        tone: 'bad',
        title: t('Chưa chuyển được Antigravity sang ' + n, 'Could not switch Antigravity to ' + n),
        body: failedBody(outcome.error, outcome.detail, outcome.reopened),
        actions: RETRY.has(outcome.error) ? [{ label: t('Thử lại', 'Try again'), act: ('agy:switch:' + outcome.number) as Action }, close()] : [close()],
      };
  }
}

/** The switch call itself was refused (`busy`, `loggingOut`, `loggingIn`, `unknownAccount`, `index`) or broke. */
export function agySwitchErrorFeedback(number: number, error: unknown, model?: Model | null): FeedbackView {
  const n = agyNamesOf(model)(number);
  return {
    tone: 'bad',
    title: t('Chưa chuyển được Antigravity sang ' + n, 'Could not switch Antigravity to ' + n),
    body: failedBody(String(error), '', null),
    actions: [close()],
  };
}

// ===== The overwrite banner =====

/** "Chuyển Antigravity bị ghi đè": the old account came back within six hours of a switch. */
export function overwriteBanner(model: Model): BannerView | null {
  const o = model.agySwitch?.overwritten;
  if (!o) return null;
  const nameOf = agyNamesOf(model);
  const to = nameOf(o.to);
  const from = nameOf(o.from);
  const again = o.overwrites > 1 ? t(' Đây là lần thứ ' + o.overwrites + '.', ' This has happened ' + o.overwrites + ' times.') : '';
  return {
    tone: 'warn',
    title: t('Chuyển Antigravity bị ghi đè', 'Antigravity switch overwritten'),
    body:
      t(
        'Lúc ' + clock(o.at) + ' agy lại dùng ' + from + ' sau khi chuyển sang ' + to + '. Antigravity.app đang chạy có thể đã ghi lại đăng nhập cũ.',
        'At ' + clock(o.at) + ' agy was using ' + from + ' again after the switch to ' + to + '. A running Antigravity.app may have written the old sign-in back.',
      ) + again,
    actions: [
      { label: t('Chuyển lại', 'Switch again'), act: ('agy:switch:' + o.to) as Action },
      { label: t('Thôi', 'Cancel'), act: 'agy:keep' },
    ],
  };
}
