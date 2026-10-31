// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the switching parts show (M3): the Claude switch sheet (F04) and
// the feedback after each action. Pure, like `view.ts`: the same model and
// clock give the same words. Copy follows the approved prototype
// (`sheetClaude`, `fbLine`, wording review W01–W11, W43).

import { clock, pad } from './format';
import { mac, os, t } from './i18n';
import type { Model, Outcome, Sessions } from './model';
import { BAD_USED, claudeAliases, claudeCells, tightest } from './accounts';
// @private:begin
import { rolesOf } from './accounts';
// @private:end
import { claudeNames, type Action, type FeedbackView } from './view';
// @private:begin
import { autoFacts, heldFeedback, holdPart, holdTail, switchedTail, type HoldChoice } from '@/private/switching';
// @private:end

/** One of a sheet's choices. */
export interface ChoiceView<V extends string> {
  v: V;
  t: string;
  s: string;
  /** When what it chooses ends; null for none or no end. */
  until: Date | null;
}

/** `Team B`, or `tài khoản 5` for a slot the snapshot does not list. */
export type NameOf = (slot: number) => string;

export function namesOf(model: Model): NameOf {
  const names = claudeNames(
    model.snapshot.claude.accounts,
    claudeAliases(model),
  // @private:begin
    rolesOf(model),
  // @private:end
  );
  return (n) => names.get(n)?.name ?? t('tài khoản ' + n, 'account ' + n);
}

/** `17:29` today, `ngày mai 03:20` tomorrow, `22/01 03:20` later or earlier. */
export function dayClock(iso: string, now: Date): string {
  const d = new Date(iso);
  const day = (x: Date) => x.toDateString();
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (day(d) === day(now)) return clock(iso);
  if (day(d) === day(tomorrow)) return t('ngày mai ', 'tomorrow ') + clock(iso);
  if (day(d) === day(yesterday)) return t('hôm qua ', 'yesterday ') + clock(iso);
  return pad(d.getDate()) + '/' + pad(d.getMonth() + 1) + ' ' + clock(iso);
}

// ===== The Claude switch sheet (F04) =====

export interface FactView {
  /** `.facts li` class: '' bullet, `w` warning, `x` blocker, `ok` a promise kept. */
  k: '' | 'w' | 'x' | 'ok';
  /** Text with **bold** runs. */
  text: string;
}

export interface ClaudeSheetView {
  /** The account's name, for the title while the switch runs. */
  name: string;
  title: string;
  facts: FactView[];
  // @private:begin
  options: ChoiceView<HoldChoice>[];
  hint: { tone: '' | 'warn'; text: string };
  // @private:end
  confirm: string;
  /** Nothing can switch now (no accounts kept yet, or the setup is out of date): the confirm button is off. */
  blocked: boolean;
}

function sessionsFact(sessions: Sessions | null | 'loading'): FactView {
  if (sessions === 'loading') return { k: '', text: mac(t('Đang đếm các phiên Claude đang chạy trên máy này…', 'Counting the Claude sessions running on this Mac…')) };
  if (!sessions)
    return {
      k: 'w',
      text: t(
        'Các phiên Claude đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo. Việc này dùng thêm hạn mức.',
        'Running Claude sessions rebuild their cache on their next message, which uses extra quota.',
      ),
    };
  if (sessions.live === 0 && sessions.unreadable === 0)
    return {
      k: '',
      text: mac(
        t(
          'Không có phiên Claude nào đang chạy trên máy này, nên không phiên nào phải tạo lại bộ nhớ đệm.',
          'No Claude sessions are running on this Mac, so none has to rebuild its cache.',
        ),
      ),
    };
  if (sessions.live === 0)
    return {
      k: 'w',
      text: mac(
        t(
          'Không đọc được ' + sessions.unreadable + ' bản ghi phiên Claude trên máy này, nên có thể có phiên đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo.',
          'Could not read ' + sessions.unreadable + ' Claude session ' + (sessions.unreadable === 1 ? 'record' : 'records') +
            ' on this Mac, so a running session may rebuild its cache on its next message.',
        ),
      ),
    };
  const one = sessions.live === 1;
  const n = t(
    (sessions.unreadable ? 'Ít nhất ' : '') + sessions.live + ' phiên Claude',
    (sessions.unreadable ? 'At least ' : '') + sessions.live + (one ? ' Claude session' : ' Claude sessions'),
  );
  return {
    k: 'w',
    text: t(
      '**' + n + '** đang chạy sẽ tạo lại bộ nhớ đệm ở tin nhắn tiếp theo. Việc này dùng thêm hạn mức.',
      '**' + n + '** ' + (one ? 'is' : 'are') + ' running. Each rebuilds its cache on its next message, which uses extra quota.',
    ),
  };
}

export function claudeSheet(
  model: Model,
  slot: number,
  // @private:begin
  choice: HoldChoice,
  // @private:end
  sessions: Sessions | null | 'loading',
  now: Date,
): ClaudeSheetView | null {
  const c = model.snapshot.claude;
  const target = c.accounts.find((a) => a.number === slot);
  if (!target) return null;
  const nameOf = namesOf(model);
  const name = nameOf(slot);
  const facts: FactView[] = [];
  // Every window, as the share used.
  const cells = claudeCells(target, now).filter((c) => !c.none);
  const parts = cells.map((c) => (c.used == null ? t('chưa có số', 'no numbers yet') : '**' + c.text + '**') + ' ' + c.unit);
  if (parts.length) facts.push({ k: '', text: t(name + ' đã dùng ' + parts.join(', ') + '.', name + ' has used ' + parts.join(', ') + '.') });
  facts.push(sessionsFact(sessions));

  // @private:begin
  // The rotation and its hold are Nâng cao's: without it the sheet only switches.
  const auto = model.advanced.on;
  const rotation = auto ? autoFacts(model, target, name, now) : { facts: [], personalOut: false };
  facts.push(...rotation.facts);
  // @private:end
  const tight = tightest(cells);
  if (tight && tight.used! >= BAD_USED)
    facts.push({ k: 'w', text: t(name + ' đã dùng ' + tight.used + '% hạn mức ' + tight.unit + '.', name + ' has used ' + tight.used + '% of its ' + tight.unit + ' quota.') });
  let blocked = !model.claudeSwitch.ready;
  if (blocked)
    facts.push({
      k: 'x',
      text: mac(
        t(
          'Quota Station chưa giữ tài khoản Claude nào trên máy này. Nhập từ claude-swap hoặc thêm tài khoản ở tab Claude.',
          'Quota Station keeps no Claude accounts on this Mac yet. Import from claude-swap or add an account in the Claude tab.',
        ),
      ),
    });
  // @private:begin
  const legacy = model.switcher.legacy;
  if (!blocked && legacy) {
    blocked = true;
    facts.push({
      k: 'x',
      text: t(
        'Bộ tự chuyển cũ `' + legacy + '` còn chạy. Tắt nó trước (hướng dẫn ở tab Claude), để hai bên không chuyển chồng nhau.',
        'The old auto-switcher `' + legacy + '` is still running. Turn it off first (the steps are in the Claude tab) so the two do not switch over each other.',
      ),
    });
  }
  // @private:end

  const title = t('Chuyển Claude sang ' + name + '?', 'Switch Claude to ' + name + '?');
  const confirm = t('Chuyển sang ' + name, 'Switch to ' + name);
  // @private:begin
  if (auto) return { name, title, facts, confirm, blocked, ...holdPart(model, target, choice, rotation.personalOut, now) };
  // @private:end
  return {
    name,
    title,
    facts,
    // @private:begin
    options: [],
    hint: { tone: '', text: '' },
    // @private:end
    confirm,
    blocked,
  };
}

// ===== Feedback after an action =====

export const close = () => ({ label: t('Đóng', 'Close'), act: 'closeFeedback' as Action });

export function switchFeedback(
  outcome: Outcome,
  model: Model,
  // @private:begin
  now: Date,
  // @private:end
): FeedbackView {
  const nameOf = namesOf(model);
  const n = nameOf(outcome.slot);
  const retry = { label: t('Thử lại', 'Try again'), act: ('switch:' + outcome.slot) as Action };
  switch (outcome.kind) {
    case 'switched': {
      let tone: FeedbackView['tone'] = '';
      let tail = '';
      // @private:begin
      const held = heldFeedback(outcome.hold, n, now);
      if (held) return held;
      [tone, tail] = switchedTail(outcome.hold, model, now);
      // @private:end
      return {
        tone,
        title: t('Đã chuyển sang ' + n + '.', 'Switched to ' + n + '.'),
        body: t('Đọc lại sau 5 giây: ' + n + ' đang dùng ✓.', 'Read again after 5 seconds: ' + n + ' is in use ✓.') + tail,
        actions: [close()],
      };
    }
    case 'raced': {
      const a = nameOf(outcome.active);
      let title = t('Một lần chuyển khác vừa chuyển sang ' + a, 'Another switch just moved to ' + a);
      let tail = '';
      const actions: { label: string; act: Action }[] = [{ label: t('Chuyển lại sang ' + n, 'Switch back to ' + n), act: ('switch:' + outcome.slot) as Action }];
      // @private:begin
      const h = outcome.hold;
      if (outcome.bySwitcherAt)
        title = t('Switcher vừa chuyển sang ' + a + ' lúc ' + clock(outcome.bySwitcherAt), 'The switcher just switched to ' + a + ' at ' + clock(outcome.bySwitcherAt));
      tail = holdTail(h, now);
      if (h && !h.failed) actions.push({ label: t('Cho phép tự chuyển lại', 'Allow auto-switch again'), act: 'resume' });
      // @private:end
      actions.push({ label: t('Để vậy', 'Leave it'), act: 'closeFeedback' });
      return {
        tone: 'warn',
        title,
        body: t('Hai lần ghi gần nhau, lần sau thắng. Đang dùng ' + a + '.', 'Two writes came close together and the later one won. ' + a + ' is in use.') + tail,
        actions,
      };
    }
    case 'unconfirmed': {
      let tail = '';
      // @private:begin
      tail = holdTail(outcome.hold, now);
      // @private:end
      return {
        tone: 'warn',
        title: t('Đã gửi lệnh chuyển sang ' + n, 'Sent the switch to ' + n),
        body:
          t(
            'Đã ghi đăng nhập mới, nhưng Claude Code giờ đang ở một tài khoản Quota Station không giữ.',
            'The new sign-in is written, but Claude Code is now on an account Quota Station does not keep.',
          ) + tail,
        actions: [{ label: t('Làm mới', 'Refresh'), act: 'refresh' }, close()],
      };
    }
    case 'failed': {
      const body = failedBody(outcome, nameOf);
      const stays =
        outcome.error === 'unknownSlot' ||
        outcome.error === 'unmanaged' ||
        // @private:begin
        outcome.error === 'legacy' ||
        // @private:end
        false;
      return {
        tone: 'bad',
        title: t('Chưa chuyển được sang ' + n, 'Could not switch to ' + n),
        body,
        actions: stays ? [close()] : [retry, close()],
      };
    }
  }
}

/** Why a switch did not land, in words. */
function failedBody(outcome: Extract<Outcome, { kind: 'failed' }>, nameOf: NameOf): string {
  const detail = outcome.detail ? ' (' + outcome.detail + ')' : '';
  switch (outcome.error) {
    // @private:begin
    case 'legacy':
      return t(
        'Bộ tự chuyển cũ ' + outcome.detail + ' còn chạy. Tắt nó trước theo hướng dẫn ở tab Claude. Chưa đổi gì.',
        'The old auto-switcher ' + outcome.detail + ' is still running. Turn it off first with the steps in the Claude tab. Nothing changed.',
      );
    // @private:end
    case 'busy':
      return t(
        'Claude Code đang giữ khoá đăng nhập (đang làm mới hoặc ghi đăng nhập). Thử lại sau vài giây. Chưa đổi gì.',
        'Claude Code holds the sign-in lock (refreshing or writing its sign-in). Try again in a few seconds. Nothing changed.',
      );
    case 'noCredential':
      return t(
        'Quota Station không có đăng nhập dùng được của tài khoản này. Đăng nhập lại nó ở tab Claude. Chưa đổi gì.',
        'Quota Station has no usable sign-in for this account. Sign it in again in the Claude tab. Nothing changed.',
      );
    case 'unmanaged':
      return t(
        'Claude Code đang đăng nhập một tài khoản Quota Station chưa giữ. Thêm tài khoản đang đăng nhập ở tab Claude trước, để không mất đăng nhập đó. Chưa đổi gì.',
        'Claude Code is signed in to an account Quota Station does not keep yet. Add the signed-in account in the Claude tab first so that sign-in is not lost. Nothing changed.',
      );
    case 'unconfirmed':
      return outcome.active != null
        ? t(
            'Ghi xong nhưng đọc lại không khớp, và không đặt lại được như cũ. Đang dùng ' + nameOf(outcome.active) + '.',
            'The write finished but reading it back did not match, and it could not be put back. ' + nameOf(outcome.active) + ' is in use.',
          )
        : t(
            'Ghi xong nhưng đọc lại không khớp, nên app đã đặt lại như cũ. Chưa đổi gì.',
            'The write finished but reading it back did not match, so the app put it back. Nothing changed.',
          );
    case 'unknownSlot':
      return t('Quota Station không giữ tài khoản này. Chưa đổi gì.', 'Quota Station does not keep this account. Nothing changed.');
    case 'keychain':
      return os(
        t('Keychain không cho đọc hoặc ghi đăng nhập' + detail + '. Chưa đổi gì.', 'The Keychain did not allow reading or writing the sign-in' + detail + '. Nothing changed.'),
        t('Windows không cho đọc hoặc ghi đăng nhập' + detail + '. Chưa đổi gì.', 'Windows did not allow reading or writing the sign-in' + detail + '. Nothing changed.'),
      );
    default:
      return t('Chuyển không được' + detail + '. Chưa đổi gì.', 'The switch failed' + detail + '. Nothing changed.');
  }
}

/** The switch call itself was refused (`busy`, `unknownSlot`) or broke. */
export function switchErrorFeedback(slot: number, error: unknown, model: Model): FeedbackView {
  const n = namesOf(model)(slot);
  const why = String(error);
  return {
    tone: 'bad',
    title: t('Chưa chuyển được sang ' + n, 'Could not switch to ' + n),
    body:
      why === 'busy'
        ? t('Một lần chuyển khác đang chạy. Chờ nó xong rồi thử lại.', 'Another switch is running. Wait for it to finish, then try again.')
        : why === 'unknownSlot'
          ? t('Quota Station không còn giữ tài khoản này. Bấm Làm mới rồi thử lại.', 'Quota Station no longer keeps this account. Press Refresh, then try again.')
          : t('Không gọi được lệnh chuyển. Chưa đổi gì.', 'Could not run the switch. Nothing changed.'),
    actions: [close()],
  };
}

