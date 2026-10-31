// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Luật xoay vòng tab (F06): the prototype's rules editor (mockup B's
// chips and sliders) as one form, and at the card's foot one row with the
// preview from the switcher's dry run in one sentence, a line per Mac and
// the save button; then the two dialogs: compare two copies, and unsaved
// changes. Rows carry short names; a slider and the hours have one line of
// meaning under them, the rest behind "Chi tiết". The hours' rail and the
// sliders share one width. Accounts go by the window's names. The state is
// in `useRules`.

import { buttonVariants } from '@/components/ui/button';
import { Card as Panel, CardText } from '@/components/ui/card';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { inputVariants, Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { Empty, LABEL, rowClass, TabHead, TabSub, TabTitle } from '@/components/page';
import { Banner, Button, Chip, Dot, Facts, Feedback, FieldHint, KVT, LinkButton, More, Spinner } from '@/components/parts';
import { clock, pad } from '@/lib/format';
import { t } from '@/lib/i18n';
import type { AuthorModel } from '@/private/author';
import type { Rules } from '@/lib/model';
import { compareRows, localLine, macsWord, markLeft, peerLine, rotationBanner, ruleEmails, rulesProblem, slotsOf } from './rules';
import type { Action, FeedbackView, PopoverView } from '@/lib/view';
import type { RulesState } from './useRules';

export interface RulesTabProps {
  model: AuthorModel;
  fresh: PopoverView['fresh'];
  rules: RulesState;
  feedback: FeedbackView | null;
  now: Date;
  onAction(act: Action): void;
}

/** `2026-01-16T17:45` for a `datetime-local` field, from any form the file holds. */
function localInput(at: string): string {
  if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/.test(at)) return at;
  const d = new Date(at);
  if (Number.isNaN(d.getTime())) return at.slice(0, 16);
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + 'T' + pad(d.getHours()) + ':' + pad(d.getMinutes());
}

/** "Đặt giờ": 18:00 today, or tomorrow once that has passed. */
function defaultEnd(now: Date): string {
  const d = new Date(now);
  d.setHours(18, 0, 0, 0);
  if (d.getTime() <= now.getTime()) d.setDate(d.getDate() + 1);
  return localInput(d.toISOString());
}

function Changed({ on }: { on: boolean }) {
  return on ? <span className="font-semi text-accent-ink">{t(' · đã đổi', ' · changed')}</span> : null;
}

/** An account to pick: a chip with its place in the round, blue once picked. */
const PICK = 'group/pk inline-flex cursor-pointer items-center gap-1.5 rounded-chip border-0 bg-fill py-[3px] pr-2.5 pl-1.5 text-sm aria-pressed:bg-accent/15 aria-pressed:text-info-ink';
const PICK_MARK =
  'grid size-4 place-items-center rounded-full bg-fill2 text-[10px] font-heavy text-ink2 group-aria-pressed/pk:bg-accent group-aria-pressed/pk:text-on-accent';

/** The marks under a threshold slider. */
const MARKS = [80, 85, 90, 95, 98];

/** The width the hours' rail and the two sliders share, so they start and end on the same lines. */
const RAIL = 'w-full max-w-[560px]';

/** A threshold slider; each mark sits where its value is on the track. */
export function Slider(props: { id: string; value: number; label: string; disabled: boolean; onChange(v: number): void }) {
  const min = Math.min(80, Math.floor(props.value));
  const max = Math.max(98, Math.ceil(props.value));
  return (
    <div className="slider relative pt-[18px]">
      <span className="absolute top-0 right-0 text-sm font-semi tabular-nums">{props.value}%</span>
      <input
        className="mx-0 mt-1 mb-0.5 w-full accent-accent"
        type="range"
        min={min}
        max={max}
        step={1}
        value={props.value}
        aria-label={props.label}
        aria-valuetext={props.value + '%'}
        disabled={props.disabled}
        onChange={(e) => props.onChange(Number(e.target.value))}
      />
      <div className="marks relative block h-3.5 text-cap text-ink3 tabular-nums" aria-hidden="true">
        {MARKS.filter((m) => m >= min && m <= max).map((m) => (
          <span
            key={m}
            className="absolute top-0 -translate-x-1/2 before:absolute before:top-[-5px] before:left-1/2 before:h-1 before:w-px before:bg-ink3 before:content-[''] first:translate-x-0 first:before:left-0 last:-translate-x-full last:before:right-0 last:before:left-auto"
            style={{ left: markLeft(m, min, max) + '%' }}>
            {m}
          </span>
        ))}
      </div>
    </div>
  );
}

function HourSelect(props: { value: number; from: number; to: number; label: string; disabled: boolean; onChange(v: number): void }) {
  const hours: number[] = [];
  for (let h = props.from; h <= props.to; h++) hours.push(h);
  return (
    <select className={cn(inputVariants(), 'w-auto')} value={props.value} aria-label={props.label} disabled={props.disabled} onChange={(e) => props.onChange(Number(e.target.value))}>
      {hours.map((h) => (
        <option key={h} value={h}>
          {pad(h)}:00
        </option>
      ))}
    </select>
  );
}

function Editor({ model, rules: st, now }: { model: AuthorModel; rules: RulesState; now: Date }) {
  const r = st.current as Rules;
  const sv = st.saved ?? model.rotation.defaults;
  const nameOf = st.nameOf;
  const emailOf = ruleEmails(model);
  const slots = slotsOf(model, r, st.saved);
  const teamChanged = r.team.join() !== sv.team.join();
  const persChanged = r.personal !== sv.personal || r.personalFrom !== sv.personalFrom || r.personalTo !== sv.personalTo;
  const expChanged = JSON.stringify(r.expire) !== JSON.stringify(sv.expire);
  const nowMin = now.getHours() * 60 + now.getMinutes();
  // The file's last hour is included: 8-23 is 08:00 to 24:00.
  const endHour = r.personalTo + 1;
  const hoursBad = r.personalTo < r.personalFrom;
  const expireSlots = [...new Set([...r.team, ...r.expire.map((e) => e.slot)])];
  const disabled = st.working != null;

  return (
    <div className="form grid">
      <div className={rowClass(teamChanged)}>
        <div className={LABEL} id="qs-r-team">
          {t('Team trong vòng', 'Team in rotation')}
          <Changed on={teamChanged} />
        </div>
        <div>
          <div className="flex flex-wrap gap-1.5" role="group" aria-labelledby="qs-r-team">
            {slots
              .filter((n) => n !== r.personal || r.team.includes(n))
              .map((n) => {
                const i = r.team.indexOf(n);
                return (
                  <button
                    key={n}
                    type="button"
                    className={PICK}
                    aria-pressed={i >= 0}
                    title={emailOf(n) || t('tài khoản ', 'account ') + n}
                    disabled={disabled}
                    onClick={() => st.edit((d) => (d.team = i >= 0 ? d.team.filter((x) => x !== n) : [...d.team, n]))}
                  >
                    <span className={PICK_MARK} aria-hidden="true">
                      {i >= 0 ? i + 1 : ''}
                    </span>
                    {nameOf(n)}
                    {i >= 0 ? <span className="sr-only">{t(', thứ tự ', ', priority ') + (i + 1)}</span> : null}
                  </button>
                );
              })}
          </div>
          {!r.team.length ? <FieldHint tone="bad">{t('Chọn ít nhất một tài khoản Team.', 'Pick at least one Team account.')}</FieldHint> : null}
        </div>
      </div>

      <div className={rowClass(persChanged)}>
        <div className={LABEL} id="qs-r-pers">
          {t('Cá nhân', 'Personal')}
          <Changed on={persChanged} />
        </div>
        <div>
          <div className="flex flex-wrap gap-1.5" role="group" aria-labelledby="qs-r-pers">
            {slots
              // The chosen one stays shown even inside Team, so the overlap can be undone here.
              .filter((n) => !r.team.includes(n) || r.personal === n)
              .map((n) => {
                const on = r.personal === n;
                return (
                  <button
                    key={n}
                    type="button"
                    className={PICK}
                    aria-pressed={on}
                    title={emailOf(n) || t('tài khoản ', 'account ') + n}
                    disabled={disabled}
                    onClick={() => st.edit((d) => (d.personal = on ? null : n))}
                  >
                    <span className={PICK_MARK} aria-hidden="true">
                      {on ? '✓' : ''}
                    </span>
                    {nameOf(n)}
                    {on ? <span className="sr-only">{t(', trong vòng', ', in rotation')}</span> : null}
                  </button>
                );
              })}
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-2 text-sm">
            <span>{t('Từ', 'From')}</span>
            <HourSelect
              value={r.personalFrom}
              from={0}
              to={23}
              label={t('Giờ bắt đầu dùng tài khoản cá nhân', 'Personal account start hour')}
              disabled={disabled}
              onChange={(v) => st.edit((d) => (d.personalFrom = v))}
            />
            <span>{t('đến', 'to')}</span>
            <HourSelect
              value={endHour}
              from={1}
              to={24}
              label={t('Giờ kết thúc dùng tài khoản cá nhân', 'Personal account end hour')}
              disabled={disabled}
              onChange={(v) => st.edit((d) => (d.personalTo = v - 1))}
            />
            <span className="text-ink2">{t('mỗi ngày', 'every day')}</span>
          </div>
          <div className={RAIL}>
            <div className="relative mt-2.5 h-2 rounded-[4px] bg-track" aria-hidden="true">
              <i className="absolute inset-y-0 rounded-[4px] bg-accent" style={{ left: ((r.personalFrom / 24) * 100).toFixed(1) + '%', width: ((Math.max(endHour - r.personalFrom, 0) / 24) * 100).toFixed(1) + '%' }} />
              <em className="absolute -top-0.5 h-3 w-0.5 rounded-[1px] bg-ink" style={{ left: ((nowMin / 1440) * 100).toFixed(1) + '%' }} />
            </div>
            <div className="mt-[3px] flex justify-between text-cap text-ink3 tabular-nums" aria-hidden="true">
              {[0, 6, 12, 18, 24].map((h) => (
                <span key={h}>{h}</span>
              ))}
            </div>
            {!hoursBad ? (
              <FieldHint className="mt-1">
                {t('Giờ trên Mac này · kết thúc lúc ' + pad(endHour) + ':00', 'This Mac’s clock · ends at ' + pad(endHour) + ':00')}{' '}
                <More small about={t('giờ dùng tài khoản cá nhân', 'personal account hours')}>
                  <p>
                    {t(
                      'Tài khoản cá nhân chỉ được dùng trong giờ này, khi không tài khoản Team nào còn hạn mức.',
                      'The personal account is used only in these hours, when no Team account has quota left.',
                    )}
                  </p>
                  <p>
                    {t(
                      'File luật ghi ' + r.personalFrom + '–' + r.personalTo + ': giờ ' + r.personalTo + ' vẫn tính, nên kết thúc lúc ' + pad(endHour) + ':00.',
                      'The rules file says ' + r.personalFrom + '–' + r.personalTo + ': hour ' + r.personalTo + ' still counts, so it ends at ' + pad(endHour) + ':00.',
                    )}
                  </p>
                </More>
              </FieldHint>
            ) : null}
          </div>
          {hoursBad ? <FieldHint tone="bad">{t('Giờ kết thúc phải sau giờ bắt đầu.', 'The end hour must come after the start hour.')}</FieldHint> : null}
          {!r.personal ? <FieldHint>{t('Không có tài khoản cá nhân: switcher chỉ dùng các tài khoản Team.', 'No personal account: the switcher uses only Team accounts.')}</FieldHint> : null}
        </div>
      </div>

      <div className={rowClass(r.full !== sv.full)}>
        <div className={LABEL}>
          {t('Chuyển khi 5 giờ đã dùng', 'Switch when 5-hour use reaches')}
          <Changed on={r.full !== sv.full} />
        </div>
        <div className={RAIL}>
          <Slider id="full" disabled={disabled} value={r.full} label={t('Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng', 'Auto-switch threshold, percent of the 5-hour quota used')} onChange={(v) => st.edit((d) => (d.full = v))} />
          <FieldHint className="mt-1">
            {t(
              'Tài khoản đang dùng đạt ' + r.full + '% thì xét chuyển sang tài khoản còn dưới ' + r.full + '%.',
              'At ' + r.full + '% used, a switch to an account still under ' + r.full + '% is considered.',
            )}{' '}
            <More small about={t('ngưỡng tự chuyển', 'the auto-switch threshold')}>
              <p>{t('Số được xét là số cao hơn giữa hạn mức 5 giờ và tuần của tài khoản.', 'The number checked is the higher of the account’s 5-hour and week windows.')}</p>
              <p>
                {t(
                  'Khi chuyển, switcher ưu tiên tài khoản sắp rời vòng, rồi tài khoản Team có cửa sổ 5 giờ kết thúc sớm nhất; bằng nhau thì theo thứ tự ở Team trong vòng.',
                  'When it switches, the switcher goes first to an account about to leave the rotation, then to the Team account whose 5-hour window ends first; on a tie, the order in Team in rotation.',
                )}
              </p>
              <p>
                {t(
                  'Việc tự chuyển xét lại mỗi 3 phút, nên lần chuyển có thể đến vài phút sau khi chạm ngưỡng. Lưu luật chỉ ghi file luật, không bật hay tắt việc tự chuyển.',
                  'Automatic switching checks again every 3 minutes, so a switch can come a few minutes after the threshold. Saving only writes the rules file; it does not turn automatic switching on or off.',
                )}
              </p>
            </More>
          </FieldHint>
        </div>
      </div>

      <div className={rowClass(r.fableFull !== sv.fableFull)}>
        <div className={LABEL}>
          {t('Bỏ qua khi Fable tuần đã dùng', 'Skip when Fable week use reaches')}
          <Changed on={r.fableFull !== sv.fableFull} />
        </div>
        <div className={RAIL}>
          <Slider id="fable" disabled={disabled} value={r.fableFull} label={t('Ngưỡng Fable tuần, phần trăm đã dùng', 'Fable week threshold, percent used')} onChange={(v) => st.edit((d) => (d.fableFull = v))} />
          <FieldHint className="mt-1">
            {t(
              'Tài khoản có Fable tuần từ ' + r.fableFull + '% được xếp sau cùng khi chọn tài khoản để chuyển.',
              'Accounts at ' + r.fableFull + '% of the Fable week or more go last when picking an account to switch to.',
            )}{' '}
            <More small about={t('ngưỡng Fable tuần', 'the Fable week threshold')}>
              <p>
                {t(
                  'Tài khoản đó vẫn được chọn khi không còn tài khoản nào khác còn hạn mức. Chỉ tính khi số Fable tuần của nó đọc được và còn mới.',
                  'Such an account is still picked when no other account has quota left. It only counts while its Fable week number was read and is fresh.',
                )}
              </p>
            </More>
          </FieldHint>
        </div>
      </div>

      <div className={rowClass(expChanged)}>
        <div className={LABEL}>
          {t('Giờ rời vòng', 'Leaves rotation at')}
          <Changed on={expChanged} />
        </div>
        <div>
          <div className={cn(KVT, RAIL)}>
            {expireSlots.map((n) => {
              const e = r.expire.find((x) => x.slot === n);
              return [
                <span key={n + ':k'}>{nameOf(n)}</span>,
                <span key={n + ':v'}>
                  {e ? (
                    <>
                      <Input
                        type="datetime-local"
                        className="w-[196px]"
                        value={localInput(e.at)}
                        aria-label={t('Giờ rời vòng của ' + nameOf(n), 'When ' + nameOf(n) + ' leaves rotation')}
                        disabled={disabled}
                        onChange={(ev) => {
                          const v = ev.target.value;
                          if (v) st.edit((d) => (d.expire = d.expire.map((x) => (x.slot === n ? { slot: n, at: v } : x))));
                        }}
                      />{' '}
                      <LinkButton disabled={disabled} onClick={() => st.edit((d) => (d.expire = d.expire.filter((x) => x.slot !== n)))}>
                        {t('Bỏ', 'Clear')}
                      </LinkButton>
                    </>
                  ) : (
                    <>
                      <span className="text-ink2">{t('Chưa đặt giờ rời vòng', 'No leave time set')}</span>{' '}
                      <LinkButton
                        aria-label={t('Đặt giờ rời vòng cho ' + nameOf(n), 'Set when ' + nameOf(n) + ' leaves rotation')}
                        disabled={disabled}
                        onClick={() => st.edit((d) => (d.expire = [...d.expire, { slot: n, at: defaultEnd(now) }].sort((a, b) => a.slot - b.slot)))}
                      >
                        {t('Đặt giờ', 'Set time')}
                      </LinkButton>
                    </>
                  )}
                </span>,
              ];
            })}
          </div>
        </div>
      </div>

      {model.rotation.ignoredKeys.length ? (
        <CardText className="mt-2 mb-0">
          {t('Giữ nguyên các khoá khác trong file: ', 'Other keys in the file stay as they are: ') + model.rotation.ignoredKeys.join(', ') + '.'}
        </CardText>
      ) : null}
    </div>
  );
}

/** The switcher's answer for the draft, in one sentence; its own line on hover. */
function Preview({ rules: st }: { rules: RulesState }) {
  const r = st.current;
  return (
    <div
      className="preview min-w-0 flex-1 rounded-[8px] bg-fill px-2.5 py-[5px] text-sm [&_b]:font-semi"
      aria-live="polite"
      aria-labelledby="qs-r-prev"
      title={st.preview?.kind === 'ok' ? st.preview.raw : undefined}
    >
      {st.preview?.kind === 'ok' ? (
        <b>{st.preview.text}</b>
      ) : st.preview?.kind === 'error' ? (
        <span>{st.preview.text}</span>
      ) : st.preview?.kind === 'loading' ? (
        <span>{t('Đang kiểm tra kết quả xem trước…', 'Checking preview…')}</span>
      ) : (
        <span>{r && rulesProblem(r) ? t('Sửa luật cho hợp lệ để xem trước.', 'Fix the rules to see a preview.') : '—'}</span>
      )}
    </div>
  );
}

/**
 * The card's foot, one row: the preview, a line per Mac and the save
 * button, with what is unsaved and "Huỷ thay đổi" before it. It wraps where
 * the page is narrow.
 */
function SaveBar({ model, rules: st }: { model: AuthorModel; rules: RulesState }) {
  const peers = model.rotation.peers;
  const macs = [localLine(model, st.dirty, !!st.error?.restored), ...peers.map(peerLine)];
  const problem = st.current ? rulesProblem(st.current) : 'no rules';
  const canSave = !!st.current && !problem && (st.dirty || model.rotation.status === 'invalid') && !st.stale;
  return (
    <div className="mt-auto flex flex-wrap items-center gap-x-4 gap-y-2.5 border-t-[.5px] border-card-line pt-3">
      {st.current ? (
        <div className="flex min-w-[260px] flex-[1_1_300px] items-center gap-3">
          <span className={cn(LABEL, 'pt-0')} id="qs-r-prev">
            {t('Xem trước', 'Preview')}
          </span>
          <Preview rules={st} />
        </div>
      ) : null}
      <div className="flex flex-wrap items-center gap-x-3.5 gap-y-1" role="group" aria-label={t('Các Mac', 'Macs')}>
        {macs.map((m) => (
          <div className="flex items-center gap-1.5 text-sm whitespace-nowrap" key={m.name} title={m.title || undefined}>
            <Dot tone={m.tone} />
            {m.name} <small className="text-xs text-ink2">· {m.text}</small>
          </div>
        ))}
      </div>
      <div className="ml-auto flex flex-wrap items-center justify-end gap-2.5">
        {st.dirty ? (
          <>
            <span className="text-sm text-ink2" aria-live="polite">
              {t('Có ' + st.changes.length + ' thay đổi chưa lưu.', st.changes.length === 1 ? '1 unsaved change.' : st.changes.length + ' unsaved changes.')}
            </span>
            <Button large disabled={st.working != null} onClick={st.revert}>
              {t('Bỏ thay đổi', 'Discard changes')}
            </Button>
          </>
        ) : null}
        {st.working === 'save' ? (
          <span className={cn(buttonVariants({ variant: 'primary', size: 'lg' }), 'inline-flex items-center gap-1.5')} role="status">
            <Spinner className="border-t-on-accent" />
            {t('Đang lưu…', 'Saving…')}
          </span>
        ) : (
          <Button large primary disabled={!canSave || st.working != null} onClick={st.save}>
            {t('Lưu cho ' + macsWord(1 + peers.length), 'Save for ' + macsWord(1 + peers.length))}
          </Button>
        )}
      </div>
    </div>
  );
}

export function RulesTab({ model, fresh, rules: st, feedback, now, onAction }: RulesTabProps) {
  const banner = st.banner ?? st.error?.banner ?? rotationBanner(model);
  return (
    <>
      <TabHead>
        <TabTitle>{t('Luật xoay vòng', 'Rotation rules')}</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
      </TabHead>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {banner ? <Banner banner={banner} onAction={onAction} busy={null} /> : null}
      {/* With rules to edit the card takes the rest of the page, so its foot stays at the bottom. */}
      <Panel className={cn('flex flex-col gap-3', st.current && 'flex-1')}>
        {st.current ? (
          <Editor model={model} rules={st} now={now} />
        ) : (
          <Empty>
            <b>{t('Chưa có luật để sửa', 'No rules to edit yet')}</b>
            {t('Tạo file từ mặc định của switcher rồi sửa và lưu cho các Mac.', 'Create the file from the switcher’s defaults, then edit it and save it for your Macs.')}
          </Empty>
        )}
        <SaveBar model={model} rules={st} />
      </Panel>
    </>
  );
}

const TH = 'px-1.5 py-1 text-left text-xs font-semi text-ink2';
const TD = 'border-t-[.5px] border-card-line px-1.5 py-[5px] align-top tabular-nums';

/** "So sánh hai bản luật": this Mac's copy beside another Mac's. */
export function CompareDialog({ model, rules: st, now }: { model: AuthorModel; rules: RulesState; now: Date }) {
  const peer = model.rotation.peers.find((p) => p.alias === st.compare);
  const mine = model.rotation.rules;
  // As the banner sees it: a conflict about this Mac's file as it is now.
  const conflict = !!peer?.current && !!peer.state && (peer.state.kind === 'newer' || peer.state.kind === 'edited');
  if (!peer || !mine || !peer.theirRules) {
    return (
      <>
        <DialogTitle>{t('So sánh hai bản luật', 'Compare the two copies of the rules')}</DialogTitle>
        <DialogLead>
          {peer && mine && conflict
            ? t(
                'App không đọc được bản trên ' + peer.name + ' nên không so sánh được. Giữ bản máy này để ghi đè nó (bản kia được sao lưu trên ' + peer.name + ').',
                'The app cannot read the copy on ' + peer.name + ', so it cannot compare them. Keep this Mac’s copy to overwrite it (that copy is backed up on ' + peer.name + ').',
              )
            : t('Chưa có bản của Mac kia để so sánh. Bấm Thử lại rồi mở lại.', 'There is no copy from the other Mac to compare yet. Press Try again, then open this again.')}
        </DialogLead>
        <DialogButtons>
          {peer && mine && conflict ? (
            <Button large onClick={() => void st.keep(peer.alias)}>
              {t('Giữ bản máy này', 'Keep this Mac’s copy')}
            </Button>
          ) : null}
          <Button large primary onClick={st.closeCompare}>
            {t('Đóng', 'Close')}
          </Button>
        </DialogButtons>
      </>
    );
  }
  const their = peer.state && (peer.state.kind === 'newer' || peer.state.kind === 'edited') ? peer.state.their : null;
  const rows = compareRows(mine, peer.theirRules, st.nameOf, now);
  const savedAt = model.rotation.stamp ? clock(model.rotation.stamp.at) : '—';
  return (
    <>
      <DialogTitle>{t('So sánh hai bản luật', 'Compare the two copies of the rules')}</DialogTitle>
      <DialogLead>
        {t(
          'Máy này lưu lúc ' + savedAt + '. ' + (their ? peer.name + ' lưu lúc ' + clock(their.at) + '.' : 'Bản trên ' + peer.name + ' không có dòng updated của app.') + ' App không tự gộp hai bản.',
          'This Mac saved at ' + savedAt + '. ' + (their ? peer.name + ' saved at ' + clock(their.at) + '.' : 'The copy on ' + peer.name + ' has no updated line from the app.') + ' The app does not merge the two copies.',
        )}
      </DialogLead>
      <table className="mt-0 mb-3 w-full border-collapse text-sm">
        <thead>
          <tr>
            <th className={TH}>{t('Mục', 'Item')}</th>
            <th className={TH}>{t('Máy này', 'This Mac')}</th>
            <th className={TH}>{peer.name}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.label} className={row.diff ? '[&>td]:bg-warn/10' : undefined}>
              <td className={TD}>
                {row.label}
                {row.diff ? (
                  <>
                    {' '}
                    <Chip chip={{ t: t('khác', 'differs'), k: 'warn' }} />
                  </>
                ) : null}
              </td>
              <td className={TD}>{row.mine}</td>
              <td className={TD}>{row.theirs}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <DialogButtons>
        <Button large onClick={() => void st.keep(peer.alias)}>
          {t('Giữ bản máy này', 'Keep this Mac’s copy')}
        </Button>
        <Button large primary onClick={() => void st.adopt(peer.alias)}>
          {t('Dùng bản trên ' + peer.name, 'Use the copy on ' + peer.name)}
        </Button>
      </DialogButtons>
    </>
  );
}

function leaveTitle(kind: 'tab' | 'close' | 'quit'): string {
  if (kind === 'close') return t('Lưu thay đổi luật trước khi đóng cửa sổ?', 'Save the rule changes before closing the window?');
  if (kind === 'quit') return t('Lưu thay đổi luật trước khi thoát Quota Station?', 'Save the rule changes before quitting Quota Station?');
  return t('Lưu thay đổi luật trước khi rời trang này?', 'Save the rule changes before leaving this page?');
}

/** Unsaved changes when leaving the tab, closing the window or quitting. */
export function LeaveDialog({ rules: st, onAnswer }: { rules: RulesState; onAnswer(a: 'discard' | 'cancel' | 'save'): void }) {
  return (
    <>
      <DialogTitle>{leaveTitle(st.leave?.kind ?? 'tab')}</DialogTitle>
      <Facts items={st.changes.map((c) => ({ k: '', text: c }))} />
      <p className="mt-0 mb-3 text-sm text-ink2">{t('Chưa lưu trên máy nào.', 'Not saved on any Mac yet.')}</p>
      <DialogButtons>
        <Button large onClick={() => onAnswer('discard')}>
          {t('Bỏ thay đổi', 'Discard changes')}
        </Button>
        <Button large onClick={() => onAnswer('cancel')}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={!!(st.current && rulesProblem(st.current))} onClick={() => onAnswer('save')}>
          {t('Lưu', 'Save')}
        </Button>
      </DialogButtons>
    </>
  );
}
