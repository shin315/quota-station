// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Claude accounts Quota Station keeps, in the Claude tab's table: each
// account's ⋯ menu (details and nickname, signing in again, removing it),
// "Thêm tài khoản" and its dialogs (ClaudeDialogs), and claude-swap's
// import while its folder is here.
// @private:begin
// The menu also holds an account out of the rotation, and the tab has the
// rotation's LaunchAgent with the old switcher's warning.
// @private:end

import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { BannerBox, BannerButtons, Button } from '../components/parts';
import { Glyph } from '../components/icons';
import { mac, t } from '../lib/i18n';
import { setupText } from '../lib/claudeAccounts';
// @private:begin
import { CardText } from '@/components/ui/card';
import { LinkButton, Rich, Toggle } from '../components/parts';
import { rotationMatters, rotationRow } from '../lib/claudeAccounts';
// @private:end
import type { ClaudeAccountsView } from '../lib/model';
import type { Action, CardView } from '../lib/view';
import type { ClaudeManage } from '../window/MainWindow';
import type { ClaudeAccountsState } from './useClaudeAccounts';

interface Props {
  view: ClaudeAccountsView;
  claude: ClaudeAccountsState;
  onAction(act: Action): void;
  // @private:begin
  advanced: boolean;
  /** A build with the author's tools: the line on what Firstmate runs shows. */
  authorTools?: boolean;
  // @private:end
}

const ITEM = 'px-2.5 py-[7px] data-[highlighted]:bg-ctl-hover data-[highlighted]:text-ink';

function AccountMenu({
  card,
  view,
  claude,
  // @private:begin
  advanced,
  // @private:end
}: { card: CardView } & Pick<
  Props,
  | 'view'
  | 'claude'
  // @private:begin
  | 'advanced'
  // @private:end
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
        {/* @private:begin */}
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
        {/* @private:end */}
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

// @private:begin
/**
 * The rotation's LaunchAgent, in parts the Claude tab lays out: its switch,
 * its line, the warning while the old switcher is loaded, and what Firstmate
 * runs (a build with the author's tools).
 */
function rotationParts({ view, claude, onAction, authorTools }: Props): ClaudeManage['rotation'] {
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
    firstmate: authorTools ? (
      <CardText className="mb-0 text-xs text-ink2">
        {/* The commands as code, not between backticks. */}
        <Rich
          text={t(
            'Firstmate đọc `quota-station claude status --json` và chạy `quota-station claude run N -- …`.',
            'Firstmate reads `quota-station claude status --json` and runs `quota-station claude run N -- …`.',
          )}
        />
      </CardText>
    ) : null,
  };
}
// @private:end

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
          {/* @private:begin */}
          <Button small onClick={() => onAction('guide:switcher')}>
            {t('Hướng dẫn', 'Guide')}
          </Button>
          {/* @private:end */}
        </BannerButtons>
      </BannerBox>
    ) : null,
    menu: (card) => (
      <AccountMenu
        card={card}
        view={view}
        claude={claude}
        // @private:begin
        advanced={props.advanced}
        // @private:end
      />
    ),
    onAdd: () => claude.open({ kind: 'add', how: view.claude ? 'browser' : 'current', error: null }),
    emptyExtra: view.importable ? (
      <>
        <Button small disabled={busy} onClick={() => onAction('claude:import')}>
          {t('Nhập từ claude-swap', 'Import from claude-swap')}
        </Button>
        {/* @private:begin */}
        <LinkButton small onClick={() => onAction('guide:switcher')}>
          {t('Hướng dẫn chuyển từ claude-swap', 'How to move from claude-swap')}
        </LinkButton>
        {/* @private:end */}
      </>
    ) : null,
    // @private:begin
    rotation: rotationParts(props),
    rotationAlone: view.rotation === true || !!view.legacy,
    // @private:end
  };
}
