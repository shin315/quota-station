// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// What switching Antigravity shows (M4c): the confirmation sheet with the
// plan's three buttons, the feedback after a switch, and the banner when a
// running Antigravity.app wrote its old login back within six hours. Pure,
// like `switching.ts`: the same model and answers give the same words.

import { clock } from '../lib/format';
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
  if (parts.length) facts.push({ k: '', text: name + ' đã dùng ' + parts.join(', ') + '.' });

  let blocked = false;
  if (login.status === 'needsLogin') {
    blocked = true;
    facts.push({ k: 'x', text: name + ' cần đăng nhập lại trong tab Antigravity trước khi chuyển.' });
  }
  if (app === 'loading') facts.push({ k: '', text: 'Đang xem Antigravity.app có đang chạy không…' });
  else if (app !== null && app.installed === 'tooOld') {
    blocked = true;
    facts.push({
      k: 'x',
      text:
        'Antigravity.app bản ' + (app.version ?? '?') + ' quá cũ: app chỉ chuyển được Antigravity.app 2.0 trở lên. Cập nhật Antigravity.app rồi thử lại.',
    });
  } else if (app !== null && app.installed === 'unreadable') {
    blocked = true;
    facts.push({
      k: 'x',
      text: 'Không đọc được phiên bản Antigravity.app, nên app không biết bản này giữ đăng nhập ở đâu. Mở lại bảng này sau, hoặc cài lại Antigravity.app.',
    });
  } else if (app === null || (app.installed === 'supported' && app.running === null))
    facts.push({ k: 'w', text: 'Không biết Antigravity.app có đang chạy không.' });
  else if (app.installed === 'missing') facts.push({ k: '', text: 'Máy này không có Antigravity.app: chỉ đổi tài khoản của agy.' });
  else if (app.running)
    facts.push({
      k: 'w',
      text: '**Antigravity.app đang chạy.** App đang chạy có thể ghi lại tài khoản cũ, nên thoát app rồi chuyển là chắc nhất.',
    });
  else facts.push({ k: '', text: 'Antigravity.app không chạy.' });
  facts.push({ k: 'w', text: 'Phiên agy đang chạy giữ tài khoản cũ cho tới khi khởi động lại.' });

  // The plan's three buttons while the app runs (or may run, or the answer
  // is not in yet); only "Chuyển" when it does not.
  const mayRun = app === null || app === 'loading' || (app.installed === 'supported' && app.running !== false);
  const buttons: AgyButton[] = blocked
    ? []
    : mayRun
      ? [
          { how: 'keepRunning', label: 'Chuyển, để app chạy', primary: false },
          { how: 'quitFirst', label: 'Thoát app rồi chuyển', primary: true },
        ]
      : [{ how: 'plain', label: 'Chuyển', primary: true }];
  return { title: 'Chuyển Antigravity sang ' + name + '?', facts, buttons, waiting: app === 'loading' };
}

/** The line under the working sheet's title; `null` for a switch started in the other window. */
export function agyWorking(how: AgyHow | null): string {
  const write = 'ghi đăng nhập dùng chung của Antigravity.app và agy';
  const check = 'rồi đợi 5 giây để đọc lại tài khoản agy đang dùng.';
  if (how === 'quitFirst') return 'Thoát Antigravity.app, ' + write + ', mở lại app, ' + check;
  // Not known which button: only what every switch does.
  if (how === null) return 'Ghi ' + write.slice('ghi '.length) + ', ' + check;
  return 'Làm mới đăng nhập, ' + write + ', ' + check;
}

// ===== Feedback after a switch =====

const CLOSE = { label: 'Đóng', act: 'closeFeedback' as Action };
const REFRESH = { label: 'Làm mới', act: 'refresh' as Action };

function tails(o: { how: AgyHow; reopened: boolean | null; saved: boolean }): { text: string; warn: boolean } {
  let text = '';
  let warn = false;
  if (o.how === 'keepRunning')
    text += ' Antigravity.app vẫn chạy và có thể ghi lại tài khoản cũ; app sẽ báo nếu bị ghi đè trong 6 giờ tới.';
  if (o.reopened === false) {
    text += ' Chưa mở lại được Antigravity.app, hãy mở bằng tay.';
    warn = true;
  }
  if (!o.saved) {
    text += ' Chưa ghi được danh sách tài khoản của app, nên app có thể chưa nhận ra tài khoản mới.';
    warn = true;
  }
  return { text, warn };
}

function failedBody(error: string, detail: string, reopened: boolean | null): string {
  const nothing = ' Chưa đổi gì.';
  switch (error) {
    case 'busy':
      return 'Một lần chuyển khác đang chạy. Chờ nó xong rồi thử lại.';
    case 'unknownAccount':
      return 'Tài khoản này không còn trong danh sách của app. Bấm Làm mới rồi thử lại.';
    case 'index':
      return 'Không đọc được danh sách tài khoản của app.' + nothing;
    case 'needsLogin':
      return 'Google không nhận đăng nhập của tài khoản này nữa. Đăng nhập lại trong tab Antigravity rồi thử lại.' + nothing;
    case 'oldApp':
      return 'Antigravity.app bản ' + (detail || '?') + ' quá cũ: app chỉ chuyển được bản 2.0 trở lên.' + nothing;
    case 'appVersion':
      return 'Không đọc được phiên bản Antigravity.app' + (detail ? ' (' + detail + ')' : '') + ', nên app không chuyển.' + nothing;
    case 'loggingOut':
      return 'Đang đăng xuất một tài khoản Antigravity. Chờ xong rồi thử lại.';
    case 'loggingIn':
      return 'Đang có một lần đăng nhập Antigravity. Chờ xong rồi thử lại.';
    case 'prepareTimeout':
      return 'Google không trả lời kịp trong 60 giây.' + nothing;
    case 'prepare':
      return 'Bước chuẩn bị đăng nhập dừng giữa chừng.' + nothing;
    case 'refresh':
      return 'Không làm mới được đăng nhập của tài khoản này' + (detail ? ' (' + detail + ')' : '') + '.' + nothing;
    case 'client':
      return 'Thiếu hoặc sai oauth-client.json trên máy này.' + nothing;
    case 'keychain':
      return 'Không đọc được đăng nhập của tài khoản này trong Keychain của app.' + nothing;
    case 'notQuit':
      return 'Antigravity.app không thoát trong 20 giây, có thể đang hỏi lưu việc. App không ép thoát và chưa đổi đăng nhập nào. Đóng Antigravity.app rồi thử lại, hoặc chọn "Chuyển, để app chạy".';
    case 'appRunning':
      return 'Antigravity.app vừa mở lại sau khi bảng chuyển kiểm tra.' + nothing + ' Bấm Thử lại để chọn thoát app rồi chuyển, hoặc để app chạy.';
    case 'appState':
      return 'Không kiểm được Antigravity.app có đang chạy không.' + nothing;
    case 'write':
    case 'writeTimeout': {
      const why = error === 'writeTimeout' ? 'security không trả lời trong 15 giây' : detail;
      const back = reopened === true ? ' Antigravity.app đã mở lại.' : reopened === false ? ' Chưa mở lại được Antigravity.app, hãy mở bằng tay.' : '';
      return 'Không ghi được đăng nhập dùng chung vào Keychain' + (why ? ' (' + why + ')' : '') + '. Đăng nhập cũ vẫn giữ nguyên.' + back;
    }
    default:
      return 'Không gọi được lệnh chuyển.' + nothing;
  }
}

/** Errors worth a second try as they are. */
const RETRY = new Set(['prepareTimeout', 'prepare', 'refresh', 'notQuit', 'appState', 'appRunning', 'appVersion', 'write', 'writeTimeout']);

export function agySwitchFeedback(outcome: AgyOutcome, model?: Model | null): FeedbackView {
  const nameOf = agyNamesOf(model);
  const t = nameOf(outcome.number);
  switch (outcome.kind) {
    case 'switched': {
      const tail = tails(outcome);
      const by = outcome.basis === 'binding' ? ' (theo tài khoản app vừa ghi; agy chưa có số để đối chiếu)' : '';
      return {
        // Antigravity.app kept running may write the old account back: say so, not as a passing toast.
        tone: tail.warn ? 'warn' : outcome.how === 'keepRunning' ? 'info' : '',
        title: 'Đã chuyển Antigravity sang ' + t + '.',
        body: 'Đọc lại sau 5 giây: agy đang dùng ' + t + by + ' ✓.' + tail.text + ' Phiên agy đang chạy vẫn dùng tài khoản cũ cho tới khi khởi động lại.',
        actions: [],
      };
    }
    case 'unconfirmed': {
      const tail = tails(outcome);
      return {
        tone: 'warn',
        title: 'Đã ghi tài khoản ' + t + ' cho Antigravity',
        body:
          (outcome.active != null
            ? 'Nhưng đọc lại sau 5 giây, agy vẫn đang dùng ' + nameOf(outcome.active) + '. Antigravity.app hoặc phiên agy đang chạy có thể vẫn giữ tài khoản cũ.'
            : 'Nhưng app chưa đọc lại được tài khoản agy đang dùng để kiểm.') + tail.text,
        actions: [REFRESH, CLOSE],
      };
    }
    case 'failed':
      return {
        tone: 'bad',
        title: 'Chưa chuyển được Antigravity sang ' + t,
        body: failedBody(outcome.error, outcome.detail, outcome.reopened),
        actions: RETRY.has(outcome.error) ? [{ label: 'Thử lại', act: ('agy:switch:' + outcome.number) as Action }, CLOSE] : [CLOSE],
      };
  }
}

/** The switch call itself was refused (`busy`, `loggingOut`, `loggingIn`, `unknownAccount`, `index`) or broke. */
export function agySwitchErrorFeedback(number: number, error: unknown, model?: Model | null): FeedbackView {
  return {
    tone: 'bad',
    title: 'Chưa chuyển được Antigravity sang ' + agyNamesOf(model)(number),
    body: failedBody(String(error), '', null),
    actions: [CLOSE],
  };
}

// ===== The overwrite banner =====

/** "Chuyển Antigravity bị ghi đè": the old account came back within six hours of a switch. */
export function overwriteBanner(model: Model): BannerView | null {
  const o = model.agySwitch?.overwritten;
  if (!o) return null;
  const nameOf = agyNamesOf(model);
  const to = nameOf(o.to);
  const again = o.overwrites > 1 ? ' Đây là lần thứ ' + o.overwrites + '.' : '';
  return {
    tone: 'warn',
    title: 'Chuyển Antigravity bị ghi đè',
    body:
      'Lúc ' + clock(o.at) + ' agy lại dùng ' + nameOf(o.from) + ' sau khi chuyển sang ' + to +
      '. Antigravity.app đang chạy có thể đã ghi lại đăng nhập cũ.' + again,
    actions: [
      { label: 'Chuyển lại', act: ('agy:switch:' + o.to) as Action },
      { label: 'Thôi', act: 'agy:keep' },
    ],
  };
}
