// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, in the Claude tab's table: each
// account's ⋯ menu (details and nickname, holding it out of the rotation,
// signing in again, removing it), "Thêm tài khoản" and its dialogs
// (ClaudeDialogs), claude-swap's import while its folder is here, and the
// rotation's LaunchAgent with the old switcher's warning.

import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { CardText } from '@/components/ui/card';
import { BannerBox, BannerButtons, Button, LinkButton, Rich, Toggle } from '../components/parts';
import { Icon } from '../components/icons';
import { t } from '../lib/i18n';
import { rotationMatters, rotationRow, setupText } from '../lib/claudeAccounts';
import type { ClaudeAccountsView } from '../lib/model';
import type { Action, CardView } from '../lib/view';
import type { ClaudeManage } from '../window/MainWindow';
import type { ClaudeAccountsState } from './useClaudeAccounts';

interface Props {
  view: ClaudeAccountsView;
  claude: ClaudeAccountsState;
  advanced: boolean;
  onAction(act: Action): void;
  /** A build with the author's tools: the line on what Firstmate runs shows. */
  authorTools?: boolean;
}

const ITEM = 'px-2.5 py-[7px] data-[highlighted]:bg-ctl-hover data-[highlighted]:text-ink';

function AccountMenu({ card, view, claude, advanced }: { card: CardView } & Pick<Props, 'view' | 'claude' | 'advanced'>) {
  const row = view.accounts.find((a) => a.slot === card.slot);
  if (!row) return null;
  const busy = view.working != null;
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="grid size-7 cursor-pointer place-items-center rounded-ctl border border-line-2 bg-ctl text-ink hover:bg-ctl-hover data-[state=open]:bg-ctl-hover [&>svg]:size-4"
          aria-label={t('Thêm thao tác cho ', 'More actions for ') + card.name}
          title={t('Thêm thao tác cho ', 'More actions for ') + card.name}
        >
          <Icon name="ellipsis" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem className={ITEM} onSelect={() => claude.open({ kind: 'detail', slot: row.slot })}>
          {t('Chi tiết và tên gọi…', 'Details and nickname…')}
        </DropdownMenuItem>
        {rotationMatters(view, advanced) ? (
          row.enabled ? (
            <DropdownMenuItem className={ITEM} disabled={busy} onSelect={() => claude.open({ kind: 'rest', slot: row.slot })}>
              {t('Cho nghỉ khỏi tự chuyển…', 'Leave out of auto-switch…')}
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem className={ITEM} disabled={busy} onSelect={() => claude.setEnabled(row.slot, true)}>
              {t('Cho vào lại tự chuyển', 'Put back into auto-switch')}
            </DropdownMenuItem>
          )
        ) : null}
        <DropdownMenuItem className={ITEM} disabled={busy || !view.claude} onSelect={() => claude.open({ kind: 'relogin', slot: row.slot })}>
          {t('Đăng nhập lại…', 'Sign in again…')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem className={ITEM + ' font-med text-bad-ink'} onSelect={() => claude.open({ kind: 'remove', slot: row.slot })}>
          {t('Xoá khỏi máy này…', 'Remove from this Mac…')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The rotation's LaunchAgent: its switch, and the warning while the old switcher is loaded. */
function Rotation({ view, claude, onAction, authorTools }: Props) {
  const rotation = rotationRow(view);
  return (
    <div className="mt-3 grid gap-2 border-t border-divider pt-3 first:mt-0 first:border-t-0 first:pt-0">
      <div className="flex items-center gap-3">
        <div className="min-w-0 flex-1">
          <b className="block text-body font-med text-heading">{t('Tự chuyển bằng Quota Station', 'Auto-switch with Quota Station')}</b>
          <small className="block text-[13px] text-ink2">{rotation.line}</small>
        </div>
        <Toggle
          on={rotation.on}
          label={t('Tự chuyển bằng Quota Station', 'Auto-switch with Quota Station')}
          disabled={rotation.disabled}
          onClick={() => claude.setRotation(!rotation.on)}
        />
      </div>
      {rotation.legacy ? (
        <BannerBox tone="warn" title={t('Bộ tự chuyển cũ còn chạy', 'The old auto-switcher is still running')} body={rotation.legacy}>
          <LinkButton onClick={() => onAction('guide:auto')}>{t('Hướng dẫn tắt', 'How to turn it off')}</LinkButton>
        </BannerBox>
      ) : null}
      {authorTools ? (
        <CardText className="mb-0 text-xs text-ink2">
          {/* The commands as code, not between backticks. */}
          <Rich
            text={t(
              'Firstmate đọc `quota-station claude status --json` và chạy `quota-station claude run N -- …`.',
              'Firstmate reads `quota-station claude status --json` and runs `quota-station claude run N -- …`.',
            )}
          />
        </CardText>
      ) : null}
    </div>
  );
}

/** What the Claude tab's table and cards take from the accounts Quota Station keeps. */
export function claudeManage(props: Props): ClaudeManage {
  const { view, claude, advanced, onAction } = props;
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
    menu: (card) => <AccountMenu card={card} view={view} claude={claude} advanced={advanced} />,
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
    rotation: <Rotation {...props} />,
    rotationAlone: view.rotation === true || !!view.legacy,
  };
}
