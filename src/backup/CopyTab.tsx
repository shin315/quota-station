// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Chép sang Mac khác tab of the main window (M13, D), as the approved
// prototype's `cp` screens: pick one of Cài đặt › Các Mac khác and the way
// (from this Mac, or to it), "So sánh", then the items that differ by group,
// each with its checkbox. What only the sending Mac has is ticked to begin
// with; what both have but not alike, the owner ticks. "Gửi cả khoá qua SSH"
// starts off. Frosted panels on the window's glass, no panel inside one.

import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Segment, Segmented } from '@/components/ui/segmented';
import { CheckRow, Progress, SETTING_ROW } from '../components/blocks';
import { COUNT, Empty, EmptyIcon, GRID, STACK, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, Dot, Feedback, LinkButton, Toggle } from '../components/parts';
import { groups, hasView, itemKey, itemName, itemNote, macChip, readAt, sameLine, selectable, stateChip, wrongWay } from '../lib/copy';
import type { AuthorModel } from '../lib/author';
import { t } from '../lib/i18n';
import { plural } from '../lib/backup';
import type { CopyItem } from '../lib/model';
import type { Action, FeedbackView } from '../lib/view';
import { NAV_ICONS, SYNC_BIG } from '../window/icons';
import type { Copy } from './useCopy';

/** The Macs to copy with: arrow keys move the choice, Tab reaches the chosen one. */
function MacList({ copy }: { copy: Copy }) {
  return (
    <RadioGroup aria-label="Mac" value={copy.mac ?? ''} onValueChange={copy.choose}>
      {copy.peers.map((p) => {
        const chip = macChip(copy.answers[p.alias]);
        return (
          <RadioGroupItem key={p.alias} value={p.alias} disabled={copy.working || copy.phase === 'reading'}>
            <b className="font-semi">
              {p.name || p.alias} {chip ? <Chip chip={chip} /> : null}
            </b>
            <small className="col-start-2 text-xs text-ink2 tabular-nums">{t('tên kết nối ', 'SSH name ') + p.alias}</small>
          </RadioGroupItem>
        );
      })}
    </RadioGroup>
  );
}

function ItemRow({ i, copy }: { i: CopyItem; copy: Copy }) {
  const k = itemKey(i);
  const wrong = wrongWay(i, copy.push);
  const can = selectable(i, copy.push);
  const name = itemName(i);
  return (
    <CheckRow className={can ? undefined : 'cursor-default [&_b]:text-ink2'} checked={can && !!copy.ticks[k]} disabled={!can || copy.working} onChange={(on) => copy.tick(k, on)}>
      <span>
        <b>{name}</b>
        <small>{itemNote(i, copy.push, copy.here, copy.other, copy.keys)}</small>
      </span>
      <span className="flex items-center gap-1.5">
        <Chip chip={stateChip(i, copy.other)} />
        {hasView(i) && !wrong ? (
          <LinkButton
            small
            aria-label={t('Xem ', 'View ') + name}
            onClick={(e) => {
              e.preventDefault();
              copy.openItem(k);
            }}
          >
            {t('Xem', 'View')}
          </LinkButton>
        ) : null}
      </span>
    </CheckRow>
  );
}

function DiffList({ copy }: { copy: Copy }) {
  const c = copy.cmp;
  if (!c) return null;
  const arrow = copy.push ? ' → ' : ' ← ';
  return (
    <Panel>
      <CardTitle>
        {t('Khác nhau · ', 'Differences · ')}
        {copy.here}
        {arrow}
        {copy.other} <span className={COUNT}>{readAt(c)}</span>
      </CardTitle>
      {c.diff.items.length ? (
        groups().map(({ group, title }) => {
          const list = c.diff.items.filter((i) => i.group === group);
          if (!list.length) return null;
          return (
            <div key={group} className="[&+&]:mt-1.5 [&>div+label]:border-t-0" role="group" aria-label={title}>
              <div className="mt-1.5 mb-0.5 flex items-center gap-1.5 text-xs font-bold text-ink2 [&_svg]:size-[13px]">
                {NAV_ICONS[group === 'skill' ? 'skills' : group === 'plugin' ? 'plugins' : group === 'file' ? 'instr' : 'mcp']}
                {title}
              </div>
              {list.map((i) => (
                <ItemRow key={itemKey(i)} i={i} copy={copy} />
              ))}
            </div>
          );
        })
      ) : (
        <CardText className="mt-2 mb-0">{t('Hai máy giống nhau: không có gì để chép.', 'The two Macs match: nothing to copy.')}</CardText>
      )}
      <CardText className="mt-2 mb-0">
        {sameLine(c)}{' '}
        {c.diff.same.length ? (
          <LinkButton small aria-expanded={copy.showSame} onClick={() => copy.setShowSame(!copy.showSame)}>
            {copy.showSame ? t('Ẩn', 'Hide') : t('Hiện', 'Show')}
          </LinkButton>
        ) : null}
      </CardText>
      {copy.showSame ? <CardText className="mt-2 mb-0">{c.diff.same.join(', ')}</CardText> : null}
    </Panel>
  );
}

export interface CopyTabProps {
  model: AuthorModel;
  feedback: FeedbackView | null;
  copy: Copy;
  onAction(act: Action): void;
}

export function CopyTab({ model, feedback, copy, onAction }: CopyTabProps) {
  const ssh = model.rotation.ssh;
  const act = (a: Action) => (a === 'copy:compare' ? copy.compare() : onAction(a));
  const top = (
    <TabHead data-tauri-drag-region>
      <TabTitle>{t('Chép sang Mac khác', 'Copy to another Mac')}</TabTitle>
      <TabSub>
        <Dot tone={ssh ? 'ok' : 'stale'} />
        {model.machine + t(' · qua SSH', ' · over SSH')}
      </TabSub>
    </TabHead>
  );
  if (!copy.peers.length)
    return (
      <>
        {top}
        <Panel>
          <Empty>
            <EmptyIcon>{SYNC_BIG}</EmptyIcon>
            <b>{t('Chưa có Mac nào để chép', 'No Macs to copy with yet')}</b>
            <CardText className="mt-2 mb-0">{t('Thêm Mac trong Cài đặt › Các Mac khác, bằng tên kết nối SSH của nó.', 'Add a Mac in Settings › Other Macs, by its SSH name.')}</CardText>
            <div className="mt-2.5 flex flex-wrap justify-center gap-2">
              <Button primary onClick={() => onAction('settings')}>
                {t('Mở Cài đặt', 'Open Settings')}
              </Button>
            </div>
          </Empty>
        </Panel>
      </>
    );
  const reading = copy.phase === 'reading';
  const n = copy.items.length;
  const err = copy.error;
  const unreach = copy.errorKind === 'unreach';
  return (
    <>
      {top}
      {!ssh ? (
        <Banner
          banner={{
            tone: 'bad',
            title: t('Không thấy ssh trên máy này', 'ssh not found on this Mac'),
            body: t('App dùng ssh của macOS để nói chuyện với Mac khác.', 'The app uses the ssh in macOS to talk to other Macs.'),
            actions: [],
          }}
          onAction={act}
        />
      ) : null}
      <Panel>
        <CardTitle>Mac</CardTitle>
        <MacList copy={copy} />
        <div className="mt-2.5 flex flex-wrap items-center gap-2.5">
          <Segmented variant="inline" aria-label={t('Chiều chép', 'Copy direction')}>
            <Segment className="px-3" pressed={copy.push} disabled={copy.working || reading} onClick={() => copy.way(true)}>
              {t('Từ máy này sang ' + copy.other, 'From this Mac to ' + copy.other)}
            </Segment>
            <Segment className="px-3" pressed={!copy.push} disabled={copy.working || reading} onClick={() => copy.way(false)}>
              {t('Lấy từ ' + copy.other + ' về', 'Bring from ' + copy.other)}
            </Segment>
          </Segmented>
          <Button primary className="ml-auto" disabled={!ssh || reading || copy.working || !copy.mac} onClick={copy.compare}>
            {copy.phase === 'idle' ? t('So sánh', 'Compare') : t('So sánh lại', 'Compare again')}
          </Button>
        </div>
      </Panel>
      {feedback ? <Feedback fb={feedback} onAction={act} /> : null}
      {copy.phase === 'error' && err ? (
        <Banner
          banner={{
            tone: err.tone === 'bad' ? 'bad' : err.tone === 'warn' ? 'warn' : 'info',
            title: err.title,
            body: err.body,
            actions: unreach
              ? [
                  { label: t('Thử lại', 'Try again'), act: 'copy:compare' },
                  { label: t('Mở Cài đặt', 'Open Settings'), act: 'settings' },
                ]
              : [{ label: t('So sánh lại', 'Compare again'), act: 'copy:compare' }],
          }}
          onAction={act}
        />
      ) : null}
      {reading ? (
        <Panel>
          <Progress
            busy
            lines={[
              ['', t('Kết nối ', 'Connecting to ') + copy.mac, ''],
              [
                'run',
                t('Đọc MCP, skills, plugin và file hướng dẫn trên ' + copy.other, 'Reading MCP, skills, plugins and instruction files on ' + copy.other),
                t('chỉ đọc tên, cấu hình và mã băm; không đọc khoá', 'names, configs and hashes only; no keys'),
              ],
              ['wait', t('So với máy này', 'Comparing with this Mac'), ''],
            ]}
          />
        </Panel>
      ) : null}
      {copy.phase === 'idle' ? (
        <Panel>
          <CardText className="m-0">{t('Chọn Mac và bấm So sánh. App chỉ đọc, chưa ghi gì.', 'Pick a Mac and press Compare. The app only reads; nothing is written yet.')}</CardText>
        </Panel>
      ) : null}
      {copy.phase === 'diff' && copy.cmp ? (
        <div className={GRID}>
          <div className={STACK}>
            <DiffList copy={copy} />
          </div>
          <div className={STACK}>
            <Panel>
              <CardTitle>{t('Khoá', 'Keys')}</CardTitle>
              <div className={SETTING_ROW}>
                <span>{t('Gửi cả khoá qua SSH', 'Also send keys over SSH')}</span>
                <Toggle on={copy.keys} label={t('Gửi cả khoá qua SSH', 'Also send keys over SSH')} disabled={copy.working} onClick={() => copy.setKeys(!copy.keys)} />
              </div>
              <CardText className="mt-2 mb-0">
                {copy.keys
                  ? t(
                      'Khoá đi trong kết nối SSH và vào thẳng Keychain của ' + (copy.push ? copy.other : 'máy này') + '; không nằm trên đĩa ở chỗ nào khác.',
                      'Keys travel inside the SSH connection straight into the Keychain on ' + (copy.push ? copy.other : 'this Mac') + '; they are not stored on disk anywhere else.',
                    )
                  : t((copy.push ? copy.other : 'Máy này') + ' sẽ hỏi dán khoá khi mở Quota Station.', (copy.push ? copy.other : 'This Mac') + ' will ask for the keys when Quota Station opens.')}
              </CardText>
            </Panel>
            <Panel>
              <CardText className="mt-0 mb-2">{t('Chỉ chép mục đã chọn. Không ghi đè cả gói, không bên nào tự thắng.', 'Copies only the items you picked. Nothing is overwritten wholesale, and neither side wins on its own.')}</CardText>
              <Button primary large disabled={!n || copy.working} onClick={copy.openGo}>
                {n
                  ? t('Chép ' + n + ' mục' + (copy.push ? ' sang ' + copy.other : ' về máy này') + '…', 'Copy ' + plural(n, 'item', 'items') + (copy.push ? ' to ' + copy.other : ' to this Mac') + '…')
                  : t('Chưa chọn mục nào', 'No items picked')}
              </Button>
            </Panel>
          </div>
        </div>
      ) : null}
    </>
  );
}
