// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, in the Claude tab's table: each
// account's ⋯ menu (details and nickname, signing in again, removing it),
// "Thêm tài khoản" and its dialogs (ClaudeDialogs), and claude-swap's
// import while its folder is here.

import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { BannerBox, BannerButtons, Button } from '../components/parts';
import { Glyph } from '../components/icons';
import { mac, t } from '../lib/i18n';
import { setupText } from '../lib/claudeAccounts';
import type { ClaudeAccountsView } from '../lib/model';
import type { Action, CardView } from '../lib/view';
import type { ClaudeManage } from '../window/MainWindow';
import type { ClaudeAccountsState } from './useClaudeAccounts';

interface Props {
  view: ClaudeAccountsView;
  claude: ClaudeAccountsState;
  onAction(act: Action): void;
}

const ITEM = 'px-2.5 py-[7px] data-[highlighted]:bg-ctl-hover data-[highlighted]:text-ink';

function AccountMenu({
  card,
  view,
  claude,
}: { card: CardView } & Pick<
  Props,
  | 'view'
  | 'claude'
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
        </BannerButtons>
      </BannerBox>
    ) : null,
    menu: (card) => (
      <AccountMenu
        card={card}
        view={view}
        claude={claude}
      />
    ),
    onAdd: () => claude.open({ kind: 'add', how: view.claude ? 'browser' : 'current', error: null }),
    emptyExtra: view.importable ? (
      <>
        <Button small disabled={busy} onClick={() => onAction('claude:import')}>
          {t('Nhập từ claude-swap', 'Import from claude-swap')}
        </Button>
      </>
    ) : null,
  };
}
