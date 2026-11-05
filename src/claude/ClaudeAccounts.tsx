// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, in the Claude tab's table: each
// account's ⋯ menu (details and nickname, signing in again, removing it),
// "Thêm tài khoản" and its dialogs (ClaudeDialogs), and claude-swap's
// import while its folder is here. The menu also holds an account out of the
// rotation, and the tab has the rotation's LaunchAgent with the old
// switcher's warning.

import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { BannerBox, BannerButtons, Button, LinkButton, Toggle } from '../components/parts';
import { Glyph } from '../components/icons';
import { mac, t } from '../lib/i18n';
import { rotationMatters, rotationRow, setupText } from '../lib/claudeAccounts';
import ext from '@qs-ext';
import type { ClaudeAccountsView } from '../lib/model';
import type { Action, CardView } from '../lib/view';
import type { ClaudeManage } from '../window/MainWindow';
import type { ClaudeAccountsState } from './useClaudeAccounts';
import type { SimpleAutoView } from '../lib/autoSwitch';

interface Props {
  view: ClaudeAccountsView;
  claude: ClaudeAccountsState;
  onAction(act: Action): void;
  /** Cài đặt › Nâng cao is on: the rotation rules exist. */
  advanced: boolean;
  /** A build with the advanced part: an extension's note on the rotation shows (src/ext/types.ts). */
  authorTools?: boolean;
  /** "Tự chuyển khi hết quota" with Nâng cao off (`lib/autoSwitch`). */
  auto?: SimpleAutoView;
}

const ITEM = 'px-2.5 py-[7px] data-[highlighted]:bg-ctl-hover data-[highlighted]:text-ink';

function AccountMenu({
  card,
  view,
  claude,
  advanced,
}: { card: CardView } & Pick<
  Props,
  | 'view'
  | 'claude'
  | 'advanced'
>) {
  const row = view.accounts.find((a) => a.slot === card.slot);
  if (!row) return null;
  const busy = view.working != null;
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="grid size-6 cursor-pointer place-items-center rounded-ctl border-0 bg-fill2 text-ink hover:brightness-[.97] data-[state=open]:brightness-[.94] [&>svg]:size-3.5"
          aria-label={t('Thêm thao tác cho ', 'More actions for ') + card.name}
          title={t('Thêm thao tác cho ', 'More actions for ') + card.name}
        >
          <Glyph name="dots-three" bold />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem className={ITEM} onSelect={() => claude.open({ kind: 'detail', slot: row.slot })}>
          {t('Chi tiết và tên gọi…', 'Details and nickname…')}
        </DropdownMenuItem>
        {rotationMatters(view, advanced) ? (
          row.enabled ? (
            <DropdownMenuItem className={ITEM} disabled={busy} onSelect={() => claude.open({ kind: 'rest', slot: row.slot })}>
              {t('Tạm dừng tự chuyển…', 'Pause auto-switch…')}
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem className={ITEM} disabled={busy} onSelect={() => claude.setEnabled(row.slot, true)}>
              {t('Đưa lại vào tự chuyển', 'Resume auto-switch')}
            </DropdownMenuItem>
          )
        ) : null}
        <DropdownMenuItem className={ITEM} disabled={busy || !view.claude} onSelect={() => claude.open({ kind: 'relogin', slot: row.slot })}>
          {t('Đăng nhập lại…', 'Sign in again…')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem className={ITEM + ' font-med text-bad-ink'} onSelect={() => claude.open({ kind: 'remove', slot: row.slot })}>
          {mac(t('Xoá khỏi máy này…', 'Remove from this Mac…'))}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/**
 * The rotation's LaunchAgent, in parts the Claude tab lays out: its switch,
 * its line and the warning while the old switcher is loaded.
 */
/** And an extension's note (a build with the advanced part). */
function rotationParts(props: Props): ClaudeManage['rotation'] {
  const { view, claude, onAction } = props;
  const rotation = rotationRow(view);
  return {
    toggle: (
      <Toggle
        on={rotation.on}
        label={t('Tự chuyển bằng Quota Station', 'Auto-switch with Quota Station')}
        disabled={rotation.disabled}
        onClick={() => claude.setRotation(!rotation.on)}
      />
    ),
    line: rotation.line,
    warning: rotation.legacy ? (
      <BannerBox tone="warn" title={t('Bộ tự chuyển cũ còn chạy', 'The old auto-switcher is still running')} body={rotation.legacy} className="mx-0 mt-2 mb-0">
        <LinkButton onClick={() => onAction('guide:auto')}>{t('Hướng dẫn tắt', 'How to turn it off')}</LinkButton>
      </BannerBox>
    ) : null,
    note: props.authorTools && ext.rotationNote ? { lead: ext.rotationNote.lead(), details: <ext.rotationNote.Details /> } : null,
  };
}

/** What the Claude tab's table and cards take from the accounts Quota Station keeps. */
export function claudeManage(props: Props): ClaudeManage {
  const { view, claude, onAction } = props;
  const busy = view.working != null;
  const setup = view.importable ? setupText(view) : null;
  return {
    setup: setup ? (
      <BannerBox tone="info" title={setup.title} body={setup.body}>
        <BannerButtons>
          <Button small tint disabled={busy} onClick={() => onAction('claude:import')}>
            {t('Nhập từ claude-swap', 'Import from claude-swap')}
          </Button>
          <Button small onClick={() => onAction('guide:switcher')}>
            {t('Hướng dẫn', 'Guide')}
          </Button>
        </BannerButtons>
      </BannerBox>
    ) : null,
    menu: (card) => (
      <AccountMenu
        card={card}
        view={view}
        claude={claude}
        advanced={props.advanced}
      />
    ),
    onAdd: () => claude.open({ kind: 'add', how: view.claude ? 'browser' : 'current', error: null }),
    emptyExtra: view.importable ? (
      <>
        <Button small disabled={busy} onClick={() => onAction('claude:import')}>
          {t('Nhập từ claude-swap', 'Import from claude-swap')}
        </Button>
        <LinkButton small onClick={() => onAction('guide:switcher')}>
          {t('Hướng dẫn chuyển từ claude-swap', 'How to move from claude-swap')}
        </LinkButton>
      </>
    ) : null,
    rotation: rotationParts(props),
    rotationAlone: view.rotation === true || !!view.legacy,
    auto: props.auto
      ? {
          view: props.auto,
          // The switch is the rotation's: on runs it every 3 minutes, off stops it.
          onToggle: () => claude.setRotation(!props.auto!.on),
          onResume: () => onAction('resume'),
        }
      : undefined,
  };
}
