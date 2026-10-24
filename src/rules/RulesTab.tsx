// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Luật xoay vòng tab (F06): the prototype's rules editor (mockup B's
// chips and sliders), the preview from the switcher's dry run in one
// sentence, the save bar with one line per Mac, and the two dialogs: compare
// two copies, and unsaved changes. Rows carry short names and no line of
// explanation; accounts go by the window's names. The state is in `useRules`.

import { buttonVariants } from '@/components/ui/button';
import { Card as Panel, CardText } from '@/components/ui/card';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { inputVariants, Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { Empty, LABEL, rowClass, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, Dot, Facts, Feedback, FieldHint, KVT, LinkButton, Spinner } from '../components/parts';
import { clock, pad } from '../lib/format';
import type { Model, Rules } from '../lib/model';
import { compareRows, localLine, macsWord, markLeft, peerLine, rotationBanner, ruleEmails, rulesProblem, slotsOf } from '../lib/rules';
import type { Action, FeedbackView, PopoverView } from '../lib/view';
import type { RulesState } from './useRules';

export interface RulesTabProps {
  model: Model;
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
  return on ? <span className="font-semi text-accent-ink"> · đã đổi</span> : null;
}

/** An account to pick: a chip with its place in the round, blue once picked. */
const PICK = 'group/pk inline-flex cursor-pointer items-center gap-1.5 rounded-chip border-0 bg-fill py-[3px] pr-2.5 pl-1.5 text-sm aria-pressed:bg-accent/15 aria-pressed:text-info-ink';
const PICK_MARK =
  'grid size-4 place-items-center rounded-full bg-fill2 text-[10px] font-heavy text-ink2 group-aria-pressed/pk:bg-accent group-aria-pressed/pk:text-on-accent';

/** The marks under a threshold slider. */
const MARKS = [80, 85, 90, 95, 98];

/** A threshold slider; each mark sits where its value is on the track. */
export function Slider(props: { id: string; value: number; label: string; disabled: boolean; onChange(v: number): void }) {
  const min = Math.min(80, Math.floor(props.value));
  const max = Math.max(98, Math.ceil(props.value));
  return (
    <div className="slider relative max-w-[420px] pt-[18px]">
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

function Editor({ model, rules: st, now }: { model: Model; rules: RulesState; now: Date }) {
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
          Team trong vòng
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
                    title={emailOf(n) || 'tài khoản ' + n}
                    disabled={disabled}
                    onClick={() => st.edit((d) => (d.team = i >= 0 ? d.team.filter((x) => x !== n) : [...d.team, n]))}
                  >
                    <span className={PICK_MARK} aria-hidden="true">
                      {i >= 0 ? i + 1 : ''}
                    </span>
                    {nameOf(n)}
                    {i >= 0 ? <span className="sr-only">, thứ tự {i + 1}</span> : null}
                  </button>
                );
              })}
          </div>
          {!r.team.length ? <FieldHint tone="bad">Chọn ít nhất một tài khoản Team.</FieldHint> : null}
        </div>
      </div>

      <div className={rowClass(persChanged)}>
        <div className={LABEL} id="qs-r-pers">
          Cá nhân
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
                    title={emailOf(n) || 'tài khoản ' + n}
                    disabled={disabled}
                    onClick={() => st.edit((d) => (d.personal = on ? null : n))}
                  >
                    <span className={PICK_MARK} aria-hidden="true">
                      {on ? '✓' : ''}
                    </span>
                    {nameOf(n)}
                    {on ? <span className="sr-only">, trong vòng</span> : null}
                  </button>
                );
              })}
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-2 text-sm">
            <span>Từ</span>
            <HourSelect
              value={r.personalFrom}
              from={0}
              to={23}
              label="Giờ bắt đầu dùng tài khoản cá nhân"
              disabled={disabled}
              onChange={(v) => st.edit((d) => (d.personalFrom = v))}
            />
            <span>đến</span>
            <HourSelect
              value={endHour}
              from={1}
              to={24}
              label="Giờ kết thúc dùng tài khoản cá nhân"
              disabled={disabled}
              onChange={(v) => st.edit((d) => (d.personalTo = v - 1))}
            />
            <span className="text-ink2">mỗi ngày</span>
          </div>
          <div className="relative mt-2 h-2 rounded-[4px] bg-track" aria-hidden="true">
            <i className="absolute inset-y-0 rounded-[4px] bg-accent" style={{ left: ((r.personalFrom / 24) * 100).toFixed(1) + '%', width: ((Math.max(endHour - r.personalFrom, 0) / 24) * 100).toFixed(1) + '%' }} />
            <em className="absolute -top-0.5 h-3 w-0.5 rounded-[1px] bg-ink" style={{ left: ((nowMin / 1440) * 100).toFixed(1) + '%' }} />
          </div>
          <div className="mt-[3px] flex justify-between text-cap text-ink3 tabular-nums" aria-hidden="true">
            {[0, 6, 12, 18, 24].map((t) => (
              <span key={t}>{t}</span>
            ))}
          </div>
          {hoursBad ? <FieldHint tone="bad">Giờ kết thúc phải sau giờ bắt đầu.</FieldHint> : null}
          {!r.personal ? <FieldHint>Không có tài khoản cá nhân: switcher chỉ dùng các tài khoản Team.</FieldHint> : null}
        </div>
      </div>

      <div className={rowClass(r.full !== sv.full)}>
        <div className={LABEL}>
          Chuyển khi 5 giờ đã dùng
          <Changed on={r.full !== sv.full} />
        </div>
        <Slider id="full" disabled={disabled} value={r.full} label="Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng" onChange={(v) => st.edit((d) => (d.full = v))} />
      </div>

      <div className={rowClass(r.fableFull !== sv.fableFull)}>
        <div className={LABEL}>
          Bỏ qua khi Fable tuần đã dùng
          <Changed on={r.fableFull !== sv.fableFull} />
        </div>
        <Slider id="fable" disabled={disabled} value={r.fableFull} label="Ngưỡng Fable tuần, phần trăm đã dùng" onChange={(v) => st.edit((d) => (d.fableFull = v))} />
      </div>

      <div className={rowClass(expChanged)}>
        <div className={LABEL}>
          Giờ rời vòng
          <Changed on={expChanged} />
        </div>
        <div>
          <div className={cn(KVT, 'max-w-[480px]')}>
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
                        aria-label={'Giờ rời vòng của ' + nameOf(n)}
                        disabled={disabled}
                        onChange={(ev) => {
                          const v = ev.target.value;
                          if (v) st.edit((d) => (d.expire = d.expire.map((x) => (x.slot === n ? { slot: n, at: v } : x))));
                        }}
                      />{' '}
                      <LinkButton disabled={disabled} onClick={() => st.edit((d) => (d.expire = d.expire.filter((x) => x.slot !== n)))}>
                        Bỏ
                      </LinkButton>
                    </>
                  ) : (
                    <>
                      <span className="text-ink2">Chưa đặt giờ rời vòng</span>{' '}
                      <LinkButton
                        aria-label={'Đặt giờ rời vòng cho ' + nameOf(n)}
                        disabled={disabled}
                        onClick={() => st.edit((d) => (d.expire = [...d.expire, { slot: n, at: defaultEnd(now) }].sort((a, b) => a.slot - b.slot)))}
                      >
                        Đặt giờ
                      </LinkButton>
                    </>
                  )}
                </span>,
              ];
            })}
          </div>
        </div>
      </div>

      <div className={rowClass(false)}>
        <div className={LABEL}>Xem trước</div>
        <div className="preview rounded-[8px] bg-fill px-2.5 py-2 text-sm [&_b]:font-semi" aria-live="polite" title={st.preview?.kind === 'ok' ? st.preview.raw : undefined}>
          {st.preview?.kind === 'ok' ? (
            <b>{st.preview.text}</b>
          ) : st.preview?.kind === 'error' ? (
            <span>{st.preview.text}</span>
          ) : st.preview?.kind === 'loading' ? (
            <span>Đang hỏi switcher…</span>
          ) : (
            <span>{rulesProblem(r) ? 'Sửa luật cho hợp lệ để xem trước.' : '—'}</span>
          )}
        </div>
      </div>
      {model.rotation.ignoredKeys.length ? (
        <CardText className="mt-2 mb-0">Giữ nguyên các khoá khác trong file: {model.rotation.ignoredKeys.join(', ')}.</CardText>
      ) : null}
    </div>
  );
}

function SaveBar({ model, rules: st }: { model: Model; rules: RulesState }) {
  const peers = model.rotation.peers;
  const macs = [localLine(model, st.dirty, !!st.error?.restored), ...peers.map(peerLine)];
  const problem = st.current ? rulesProblem(st.current) : 'no rules';
  const canSave = !!st.current && !problem && (st.dirty || model.rotation.status === 'invalid') && !st.stale;
  return (
    <div className="flex flex-wrap items-center gap-2.5 border-t-[.5px] border-card-line pt-2.5">
      <div className="grid gap-1.5" aria-label="Các Mac">
        {macs.map((m) => (
          <div className="flex items-center gap-2 text-sm" key={m.name} title={m.title || undefined}>
            <Dot tone={m.tone} />
            {m.name} <small className="text-ink2">· {m.text}</small>
          </div>
        ))}
      </div>
      <span className="min-w-[160px] flex-1 text-sm text-ink2" aria-live="polite">
        {st.dirty ? 'Có ' + st.changes.length + ' thay đổi chưa lưu.' : ''}
      </span>
      {st.dirty ? (
        <Button large disabled={st.working != null} onClick={st.revert}>
          Huỷ thay đổi
        </Button>
      ) : null}
      {st.working === 'save' ? (
        <span className={cn(buttonVariants({ variant: 'primary', size: 'lg' }), 'inline-flex items-center gap-1.5')} role="status">
          <Spinner className="border-t-white" />
          Đang lưu…
        </span>
      ) : (
        <Button large primary disabled={!canSave || st.working != null} onClick={st.save}>
          {'Lưu cho ' + macsWord(1 + peers.length)}
        </Button>
      )}
    </div>
  );
}

export function RulesTab({ model, fresh, rules: st, feedback, now, onAction }: RulesTabProps) {
  const banner = st.banner ?? st.error?.banner ?? rotationBanner(model);
  return (
    <>
      <TabHead>
        <TabTitle>Luật xoay vòng</TabTitle>
        <TabSub>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </TabSub>
      </TabHead>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {banner ? <Banner banner={banner} onAction={onAction} busy={null} /> : null}
      <Panel>
        {st.current ? (
          <Editor model={model} rules={st} now={now} />
        ) : (
          <Empty>
            <b>Chưa có luật để sửa</b>Tạo file từ mặc định của switcher rồi sửa và lưu cho các Mac.
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
export function CompareDialog({ model, rules: st, now }: { model: Model; rules: RulesState; now: Date }) {
  const peer = model.rotation.peers.find((p) => p.alias === st.compare);
  const mine = model.rotation.rules;
  // As the banner sees it: a conflict about this Mac's file as it is now.
  const conflict = !!peer?.current && !!peer.state && (peer.state.kind === 'newer' || peer.state.kind === 'edited');
  if (!peer || !mine || !peer.theirRules) {
    return (
      <>
        <DialogTitle>So sánh hai bản luật</DialogTitle>
        <DialogLead>
          {peer && mine && conflict
            ? 'App không đọc được bản trên ' + peer.name + ' nên không so sánh được. Giữ bản máy này để ghi đè nó (bản kia được sao lưu trên ' + peer.name + ').'
            : 'Chưa có bản của Mac kia để so sánh. Bấm Thử lại rồi mở lại.'}
        </DialogLead>
        <DialogButtons>
          {peer && mine && conflict ? (
            <Button large onClick={() => void st.keep(peer.alias)}>
              Giữ bản máy này
            </Button>
          ) : null}
          <Button large primary onClick={st.closeCompare}>
            Đóng
          </Button>
        </DialogButtons>
      </>
    );
  }
  const their = peer.state && (peer.state.kind === 'newer' || peer.state.kind === 'edited') ? peer.state.their : null;
  const rows = compareRows(mine, peer.theirRules, st.nameOf, now);
  return (
    <>
      <DialogTitle>So sánh hai bản luật</DialogTitle>
      <DialogLead>
        Máy này lưu lúc {model.rotation.stamp ? clock(model.rotation.stamp.at) : '—'}. {their ? peer.name + ' lưu lúc ' + clock(their.at) + '.' : 'Bản trên ' + peer.name + ' không có dòng updated của app.'} App không tự gộp hai bản.
      </DialogLead>
      <table className="mt-0 mb-3 w-full border-collapse text-sm">
        <thead>
          <tr>
            <th className={TH}>Mục</th>
            <th className={TH}>Máy này</th>
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
                    <Chip chip={{ t: 'khác', k: 'warn' }} />
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
          Giữ bản máy này
        </Button>
        <Button large primary onClick={() => void st.adopt(peer.alias)}>
          {'Dùng bản trên ' + peer.name}
        </Button>
      </DialogButtons>
    </>
  );
}

const LEAVE_TITLES = {
  tab: 'Lưu thay đổi luật trước khi rời trang này?',
  close: 'Lưu thay đổi luật trước khi đóng cửa sổ?',
  quit: 'Lưu thay đổi luật trước khi thoát Quota Station?',
} as const;

/** Unsaved changes when leaving the tab, closing the window or quitting. */
export function LeaveDialog({ rules: st, onAnswer }: { rules: RulesState; onAnswer(a: 'discard' | 'cancel' | 'save'): void }) {
  return (
    <>
      <DialogTitle>{LEAVE_TITLES[st.leave?.kind ?? 'tab']}</DialogTitle>
      <Facts items={st.changes.map((c) => ({ k: '', text: c }))} />
      <p className="mt-0 mb-3 text-sm text-ink2">Chưa lưu trên máy nào.</p>
      <DialogButtons>
        <Button large onClick={() => onAnswer('discard')}>
          Bỏ thay đổi
        </Button>
        <Button large onClick={() => onAnswer('cancel')}>
          Huỷ
        </Button>
        <Button large primary disabled={!!(st.current && rulesProblem(st.current))} onClick={() => onAnswer('save')}>
          Lưu
        </Button>
      </DialogButtons>
    </>
  );
}
