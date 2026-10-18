// The Luật xoay vòng tab (F06): the prototype's rules editor (mockup B's
// chips and sliders), the preview from the switcher's dry run, the save bar
// with one line per Mac, and the two dialogs: compare two copies, and
// unsaved changes. The state is in `useRules`.

import { Banner, Button, Chip, Dot, Feedback } from '../components/parts';
import { SHEET_TITLE_ID } from '../components/Sheets';
import { clock, pad } from '../lib/format';
import type { Model, Rules } from '../lib/model';
import { compareRows, localLine, macsWord, peerLine, rotationBanner, rulesProblem, slotsOf } from '../lib/rules';
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
  return on ? <span className="chg"> · đã đổi</span> : null;
}

function Slider(props: { id: string; value: number; label: string; disabled: boolean; onChange(v: number): void }) {
  const min = Math.min(80, Math.floor(props.value));
  const max = Math.max(98, Math.ceil(props.value));
  return (
    <div className="slider">
      <span className="val">{props.value}%</span>
      <input
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
      <div className="marks" aria-hidden="true">
        {[80, 85, 90, 95, 98].map((m) => (
          <span key={m}>{m}</span>
        ))}
      </div>
    </div>
  );
}

function HourSelect(props: { value: number; from: number; to: number; label: string; disabled: boolean; onChange(v: number): void }) {
  const hours: number[] = [];
  for (let h = props.from; h <= props.to; h++) hours.push(h);
  return (
    <select className="inp tsel" value={props.value} aria-label={props.label} disabled={props.disabled} onChange={(e) => props.onChange(Number(e.target.value))}>
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
    <div className="form">
      <div className={'fr' + (teamChanged ? ' changed' : '')}>
        <div className="fl" id="qs-r-team">
          Tài khoản Team được xoay vòng
          <Changed on={teamChanged} />
          <small>Bấm để thêm hoặc bỏ. Số là thứ tự ưu tiên. Khoá team</small>
        </div>
        <div>
          <div className="pick" role="group" aria-labelledby="qs-r-team">
            {slots
              .filter((n) => n !== r.personal || r.team.includes(n))
              .map((n) => {
                const i = r.team.indexOf(n);
                return (
                  <button
                    key={n}
                    type="button"
                    aria-pressed={i >= 0}
                    title={'tài khoản ' + n}
                    disabled={disabled}
                    onClick={() => st.edit((d) => (d.team = i >= 0 ? d.team.filter((x) => x !== n) : [...d.team, n]))}
                  >
                    <span className="o" aria-hidden="true">
                      {i >= 0 ? i + 1 : ''}
                    </span>
                    {nameOf(n)}
                    {i >= 0 ? <span className="vh">, thứ tự {i + 1}</span> : null}
                  </button>
                );
              })}
          </div>
          {!r.team.length ? <div className="fhint bad">Chọn ít nhất một tài khoản Team.</div> : null}
        </div>
      </div>

      <div className={'fr' + (persChanged ? ' changed' : '')}>
        <div className="fl" id="qs-r-pers">
          Tài khoản cá nhân
          <Changed on={persChanged} />
          <small>Chỉ dùng trong giờ. Khoá personal, personal_hours</small>
        </div>
        <div>
          <div className="pick" role="group" aria-labelledby="qs-r-pers">
            {slots
              // The chosen one stays shown even inside Team, so the overlap can be undone here.
              .filter((n) => !r.team.includes(n) || r.personal === n)
              .map((n) => {
                const on = r.personal === n;
                return (
                  <button
                    key={n}
                    type="button"
                    aria-pressed={on}
                    title={'tài khoản ' + n}
                    disabled={disabled}
                    onClick={() => st.edit((d) => (d.personal = on ? null : n))}
                  >
                    <span className="o" aria-hidden="true">
                      {on ? '✓' : ''}
                    </span>
                    {nameOf(n)}
                    {on ? <span className="vh">, trong vòng</span> : null}
                  </button>
                );
              })}
          </div>
          <div className="trange" style={{ marginTop: 8 }}>
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
            <span style={{ color: 'var(--ink2)' }}>mỗi ngày</span>
          </div>
          <div className="tbar24" aria-hidden="true">
            <i style={{ left: ((r.personalFrom / 24) * 100).toFixed(1) + '%', width: ((Math.max(endHour - r.personalFrom, 0) / 24) * 100).toFixed(1) + '%' }} />
            <em style={{ left: ((nowMin / 1440) * 100).toFixed(1) + '%' }} />
          </div>
          <div className="tticks" aria-hidden="true">
            {[0, 6, 12, 18, 24].map((t) => (
              <span key={t}>{t}</span>
            ))}
          </div>
          {hoursBad ? <div className="fhint bad">Giờ kết thúc phải sau giờ bắt đầu.</div> : null}
          {!r.personal ? <div className="fhint">Không có tài khoản cá nhân: switcher chỉ dùng các tài khoản Team.</div> : null}
        </div>
      </div>

      <div className={'fr' + (r.full !== sv.full ? ' changed' : '')}>
        <div className="fl">
          Tự chuyển khi đã dùng {r.full}% hạn mức 5 giờ
          <Changed on={r.full !== sv.full} />
          <small>Ngưỡng tự chuyển. Khoá full</small>
        </div>
        <Slider id="full" disabled={disabled} value={r.full} label="Ngưỡng tự chuyển, phần trăm hạn mức 5 giờ đã dùng" onChange={(v) => st.edit((d) => (d.full = v))} />
      </div>

      <div className={'fr' + (r.fableFull !== sv.fableFull ? ' changed' : '')}>
        <div className="fl">
          Bỏ qua tài khoản khi đã dùng {r.fableFull}% hạn mức Fable trong tuần
          <Changed on={r.fableFull !== sv.fableFull} />
          <small>Khoá fable_full</small>
        </div>
        <Slider id="fable" disabled={disabled} value={r.fableFull} label="Ngưỡng Fable tuần, phần trăm đã dùng" onChange={(v) => st.edit((d) => (d.fableFull = v))} />
      </div>

      <div className={'fr' + (expChanged ? ' changed' : '')}>
        <div className="fl">
          Giờ rời vòng
          <Changed on={expChanged} />
          <small>Sau giờ này, tài khoản không còn trong vòng. Khoá expire</small>
        </div>
        <div>
          <div className="kvt" style={{ maxWidth: 480 }}>
            {expireSlots.map((n) => {
              const e = r.expire.find((x) => x.slot === n);
              return [
                <span key={n + ':k'}>{nameOf(n)}</span>,
                <span key={n + ':v'}>
                  {e ? (
                    <>
                      <input
                        type="datetime-local"
                        className="inp s"
                        style={{ width: 196 }}
                        value={localInput(e.at)}
                        aria-label={'Giờ rời vòng của ' + nameOf(n)}
                        disabled={disabled}
                        onChange={(ev) => {
                          const v = ev.target.value;
                          if (v) st.edit((d) => (d.expire = d.expire.map((x) => (x.slot === n ? { slot: n, at: v } : x))));
                        }}
                      />{' '}
                      <button type="button" className="lnk" disabled={disabled} onClick={() => st.edit((d) => (d.expire = d.expire.filter((x) => x.slot !== n)))}>
                        Bỏ
                      </button>
                    </>
                  ) : (
                    <>
                      <span style={{ color: 'var(--ink2)' }}>Chưa đặt giờ rời vòng</span>{' '}
                      <button
                        type="button"
                        className="lnk"
                        aria-label={'Đặt giờ rời vòng cho ' + nameOf(n)}
                        disabled={disabled}
                        onClick={() => st.edit((d) => (d.expire = [...d.expire, { slot: n, at: defaultEnd(now) }].sort((a, b) => a.slot - b.slot)))}
                      >
                        Đặt giờ
                      </button>
                    </>
                  )}
                </span>,
              ];
            })}
          </div>
        </div>
      </div>

      <div className="fr">
        <div className="fl">
          Xem trước<small>Từ switcher --dry-run với luật đang sửa, lúc {clock(now.toISOString())}</small>
        </div>
        <div className="preview" aria-live="polite">
          {st.preview?.kind === 'ok' ? (
            <>
              <b>{st.preview.text}</b>
              <br />
              <span className="mono" style={{ fontSize: 'var(--fs-sm)' }}>
                {st.preview.raw}
              </span>
            </>
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
        <p className="note">Giữ nguyên các khoá khác trong file: {model.rotation.ignoredKeys.join(', ')}.</p>
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
    <div className="savebar">
      <div className="macs" aria-label="Các Mac">
        {macs.map((m) => (
          <div className="mac" key={m.name} title={m.title || undefined}>
            <Dot tone={m.tone} />
            {m.name} <small>· {m.text}</small>
          </div>
        ))}
      </div>
      <span className="grow" aria-live="polite">
        {st.dirty ? 'Có ' + st.changes.length + ' thay đổi chưa lưu.' : ''}
      </span>
      {st.dirty ? (
        <Button large disabled={st.working != null} onClick={st.revert}>
          Huỷ thay đổi
        </Button>
      ) : null}
      {st.working === 'save' ? (
        <span className="btn lg pri" style={{ display: 'inline-flex', gap: 6, alignItems: 'center' }} role="status">
          <span className="spin" style={{ borderTopColor: '#fff' }} aria-hidden="true" />
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
      <div className="tb" data-tauri-drag-region>
        <h2>Luật xoay vòng</h2>
        <small>
          <Dot tone={fresh.tone} />
          {model.machine} · {fresh.text}
        </small>
        <span className="mono">{model.rotation.path}</span>
      </div>
      {feedback ? <Feedback fb={feedback} onAction={onAction} /> : null}
      {banner ? <Banner banner={banner} onAction={onAction} busy={null} /> : null}
      <div className="card">
        {st.current ? (
          <Editor model={model} rules={st} now={now} />
        ) : (
          <div className="empty">
            <b>Chưa có luật để sửa</b>Tạo file từ mặc định của switcher rồi sửa và lưu cho các Mac.
          </div>
        )}
        <SaveBar model={model} rules={st} />
      </div>
    </>
  );
}

/** "So sánh hai bản luật": this Mac's copy beside another Mac's. */
export function CompareDialog({ model, rules: st, now }: { model: Model; rules: RulesState; now: Date }) {
  const peer = model.rotation.peers.find((p) => p.alias === st.compare);
  const mine = model.rotation.rules;
  const conflict = !!peer?.state && (peer.state.kind === 'newer' || peer.state.kind === 'edited');
  if (!peer || !mine || !peer.theirRules) {
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>So sánh hai bản luật</h3>
        <p className="lead">
          {peer && mine && conflict
            ? 'App không đọc được bản trên ' + peer.name + ' nên không so sánh được. Giữ bản máy này để ghi đè nó (bản kia được sao lưu trên ' + peer.name + ').'
            : 'Chưa có bản của Mac kia để so sánh. Bấm Thử lại rồi mở lại.'}
        </p>
        <div className="btns">
          {peer && mine && conflict ? (
            <Button large onClick={() => void st.keep(peer.alias)}>
              Giữ bản máy này
            </Button>
          ) : null}
          <Button large primary onClick={st.closeCompare}>
            Đóng
          </Button>
        </div>
      </>
    );
  }
  const their = peer.state && (peer.state.kind === 'newer' || peer.state.kind === 'edited') ? peer.state.their : null;
  const rows = compareRows(mine, peer.theirRules, st.nameOf, now);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>So sánh hai bản luật</h3>
      <p className="lead">
        Máy này lưu lúc {model.rotation.stamp ? clock(model.rotation.stamp.at) : '—'}. {their ? peer.name + ' lưu lúc ' + clock(their.at) + '.' : 'Bản trên ' + peer.name + ' không có dòng updated của app.'} App không tự gộp hai bản.
      </p>
      <table className="cmp">
        <thead>
          <tr>
            <th>Mục</th>
            <th>Máy này</th>
            <th>{peer.name}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.label} className={row.diff ? 'diff' : undefined}>
              <td>
                {row.label}
                {row.diff ? (
                  <>
                    {' '}
                    <Chip chip={{ t: 'khác', k: 'warn' }} />
                  </>
                ) : null}
              </td>
              <td>{row.mine}</td>
              <td>{row.theirs}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="btns">
        <Button large onClick={() => void st.keep(peer.alias)}>
          Giữ bản máy này
        </Button>
        <Button large primary onClick={() => void st.adopt(peer.alias)}>
          {'Dùng bản trên ' + peer.name}
        </Button>
      </div>
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
      <h3 id={SHEET_TITLE_ID}>{LEAVE_TITLES[st.leave?.kind ?? 'tab']}</h3>
      <ul className="facts">
        {st.changes.map((c) => (
          <li key={c}>
            <span>{c}</span>
          </li>
        ))}
      </ul>
      <p style={{ fontSize: 'var(--fs-sm)', color: 'var(--ink2)', margin: '0 0 12px' }}>Chưa lưu trên máy nào.</p>
      <div className="btns">
        <Button large onClick={() => onAnswer('discard')}>
          Bỏ thay đổi
        </Button>
        <Button large onClick={() => onAnswer('cancel')}>
          Huỷ
        </Button>
        <Button large primary disabled={!!(st.current && rulesProblem(st.current))} onClick={() => onAnswer('save')}>
          Lưu
        </Button>
      </div>
    </>
  );
}
