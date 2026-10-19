// The Skills tab's dialogs (M11): the batch preview every box goes through,
// installing from GitHub in three steps (the repository, the skills with
// their SKILL.md to read first, the preview), gathering the skills installed
// by hand, a newer version, removing, the skills agents and plugins bring,
// a link to nothing, and how to get `npx`. No dialog runs a skill.

import { Banner, Button, ChoiceGroup } from '../components/parts';
import { SHEET_TITLE_ID } from '../components/Sheets';
import { AGENT_NAME, AGENTS, RESTART } from '../lib/mcp';
import type { Agent, Model } from '../lib/model';
import { groupFile, onAt, sizeText, skillRefusalText } from '../lib/skills';
import { AgentLogo } from './CheckMatrix';
import { Facts, LINE_CLASS } from './McpDialogs';
import type { SkillBatch, SkillDialog, SkillGh, Skills } from './useSkills';

/** The dialog's width: the preview and the repository's list need room. */
export function skillDialogSize(d: SkillDialog): 'lg' | 'xl' | 'wide' {
  if (d.kind === 'batch' || (d.kind === 'gh' && d.step !== 'find')) return 'xl';
  return 'lg';
}

function Steps({ at }: { at: 0 | 1 | 2 }) {
  return (
    <ol className="stp" aria-label="Các bước">
      {['Repo', 'Chọn skill', 'Cài'].map((t, i) => (
        <li key={t} className={i === at ? 'on' : i < at ? 'done' : undefined} aria-current={i === at ? 'step' : undefined}>
          <i>{i < at ? '✓' : i + 1}</i>
          {t}
        </li>
      ))}
    </ol>
  );
}

function Lines({ head, right, lines }: { head: string; right: string; lines: { op: keyof typeof LINE_CLASS; text: string }[] }) {
  return (
    <div className="diff">
      <div className="fh">
        <span className="mono">{head}</span>
        <span className="r">{right}</span>
      </div>
      <pre>
        {lines.map((l, i) => (
          <span key={i} className={LINE_CLASS[l.op]}>
            {l.op === 'gap' ? '…' : l.text || ' '}
          </span>
        ))}
      </pre>
    </div>
  );
}

function Gone({ skills }: { skills: Skills }) {
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Không còn skill này</h3>
      <p className="lead">Danh sách vừa đổi. Bấm Đọc lại rồi thử lại.</p>
      <div className="btns">
        <Button large primary onClick={skills.close}>
          Đóng
        </Button>
      </div>
    </>
  );
}

function BatchBody({ d, skills }: { d: SkillBatch; skills: Skills }) {
  const p = d.preview;
  const n = p.changes.length;
  const files = p.groups.map((g) => groupFile(g.agent)).join(', ');
  const facts: [string, string][] = [
    ['ok', 'Sao lưu ' + files + ' trước khi ghi. Hoàn tác được.'],
    ['ok', 'Tắt là bỏ liên kết hay thêm một mục trong config.toml: thư mục skill giữ nguyên.'],
    ...p.refused.map((r): [string, string] => ['x', 'Bỏ qua ' + r.name + ' cho ' + AGENT_NAME[r.agent] + ': ' + skillRefusalText(r.reason) + '.']),
    ...p.groups.map((g): [string, string] => ['w', RESTART[g.agent]]),
  ];
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{d.title}</h3>
      <p className="lead">Không chép, không sửa nội dung skill.</p>
      {p.groups.map((g) => (
        <section key={g.agent} aria-label={AGENT_NAME[g.agent]}>
          <div className="sec-t">
            <AgentLogo agent={g.agent} />
            {AGENT_NAME[g.agent]}
          </div>
          <Lines head={g.file} right={g.agent === 'codex' ? g.count + ' skill' : g.count + ' liên kết'} lines={g.lines} />
          {g.agent === 'codex' ? <p className="tech">Codex đọc thẳng thư mục chung, nên bật hay tắt cho Codex là một mục [[skills.config]] trong config.toml.</p> : null}
        </section>
      ))}
      <Facts items={facts} />
      <div className="btns">
        <Button large disabled={skills.working} onClick={skills.close}>
          Quay lại
        </Button>
        <Button large primary disabled={skills.working} onClick={skills.write}>
          {skills.working ? 'Đang ghi…' : 'Ghi ' + n + ' thay đổi'}
        </Button>
      </div>
    </>
  );
}

function AgentTicks(props: { model: Model; value: Record<Agent, boolean>; onChange(v: Record<Agent, boolean>): void }) {
  const cols = props.model.skills.table?.columns ?? [];
  return (
    <div role="group" aria-label="Bật cho" style={{ display: 'flex', flexWrap: 'wrap', gap: '4px 14px' }}>
      {AGENTS.map((a) => {
        const can = !!cols.find((c) => c.agent === a)?.writable;
        return (
          <label key={a} className="check" style={{ margin: 0 }} title={can ? undefined : AGENT_NAME[a] + ': chỉ xem trên máy này'}>
            <input type="checkbox" checked={can && props.value[a]} disabled={!can} onChange={(e) => props.onChange({ ...props.value, [a]: e.target.checked })} />
            {AGENT_NAME[a]}
          </label>
        );
      })}
    </div>
  );
}

function GhFind({ d, skills }: { d: SkillGh; skills: Skills }) {
  const go = () => d.repo.trim() && skills.findRepo(d);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Cài skill từ GitHub</h3>
      <Steps at={0} />
      <label className="seglbl" htmlFor="gh-repo">
        Repo hoặc link GitHub
      </label>
      <input
        id="gh-repo"
        className={'inp mono' + (d.error ? ' bad' : '')}
        value={d.repo}
        placeholder="chủ/tên-repo"
        autoFocus
        aria-invalid={!!d.error}
        aria-describedby={d.error ? 'gh-repo-e' : undefined}
        onChange={(e) => skills.setDialog({ ...d, repo: e.target.value, error: null })}
        onKeyDown={(e) => {
          if (e.key === 'Enter') go();
        }}
      />
      {d.error ? (
        <div className="fhint bad" id="gh-repo-e">
          {d.error}
        </div>
      ) : null}
      <div style={{ marginTop: 10 }}>
        <Facts items={[['w', 'Skill có thể chạy lệnh trên máy khi agent dùng nó. Chỉ cài từ nguồn bạn tin.']]} />
      </div>
      <div className="btns">
        <Button large onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={!d.repo.trim() || skills.working} onClick={go}>
          {skills.working ? 'Đang tìm…' : 'Tìm skill'}
        </Button>
      </div>
    </>
  );
}

function GhPick({ d, skills, model }: { d: SkillGh; skills: Skills; model: Model }) {
  const list = d.list?.skills ?? [];
  const pick = (name: string, on: boolean) => skills.setDialog({ ...d, picked: on ? [...d.picked, name] : d.picked.filter((n) => n !== name) });
  if (d.reading)
    return (
      <>
        <h3 id={SHEET_TITLE_ID}>SKILL.md của {d.reading.name}</h3>
        <p className="lead">Chỉ đọc. Cài xong, agent làm theo hướng dẫn này khi dùng skill.</p>
        {d.reading.error ? (
          <p className="note bad">{d.reading.error}</p>
        ) : d.reading.text == null ? (
          <p className="note">Đang đọc từ {d.repo}…</p>
        ) : (
          <div className="diff">
            <pre>
              {d.reading.text.split('\n').map((l, i) => (
                <span key={i} className="c">
                  {l || ' '}
                </span>
              ))}
            </pre>
          </div>
        )}
        <div className="btns">
          <Button large primary onClick={() => skills.setDialog({ ...d, reading: null })}>
            Quay lại danh sách
          </Button>
        </div>
      </>
    );
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Chọn skill trong {d.repo}</h3>
      <Steps at={1} />
      <div role="group" aria-label="Skill trong repo">
        {list.map((s) => (
          <label key={s.name} className={'ck' + (s.installed ? ' dis' : '')}>
            <input type="checkbox" checked={!s.installed && d.picked.includes(s.name)} disabled={s.installed} onChange={(e) => pick(s.name, e.target.checked)} />
            <span>
              <b>{s.name}</b>
              <small>{s.description}</small>
            </span>
            {s.installed ? (
              <span className="chip ok">đã cài</span>
            ) : (
              <button type="button" className="lnk sm" aria-label={'Đọc SKILL.md của ' + s.name} onClick={(e) => (e.preventDefault(), skills.readMd(d, s.name))}>
                Đọc SKILL.md
              </button>
            )}
          </label>
        ))}
      </div>
      <div className="sec-t">Bật cho</div>
      <AgentTicks model={model} value={d.agents} onChange={(agents) => skills.setDialog({ ...d, agents })} />
      <div className="btns">
        <Button large onClick={() => skills.setDialog({ ...d, step: 'find' })}>
          Quay lại
        </Button>
        <span className="grow" />
        <Button large onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={!d.picked.length} onClick={() => skills.setDialog({ ...d, step: 'preview' })}>
          Tiếp: xem trước
        </Button>
      </div>
    </>
  );
}

function GhPreview({ d, skills }: { d: SkillGh; skills: Skills }) {
  const names = d.picked;
  const links = AGENTS.filter((a) => a !== 'codex' && d.agents[a]);
  const lines = names.flatMap((n) => [
    ...links.map((a) => ({ op: 'add' as const, text: (a === 'claude' ? '~/.claude/skills/' : '~/.gemini/config/skills/') + n + ' → ~/.agents/skills/' + n })),
    { op: 'ctx' as const, text: d.agents.codex ? 'Codex đọc thẳng thư mục chung' : 'Codex: thêm mục [[skills.config]] name = "' + n + '", enabled = false' },
  ]);
  const title = names.length === 1 ? 'Cài ' + names[0] + '?' : 'Cài ' + names.length + ' skill?';
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>{skills.working ? 'Đang cài ' + names.join(', ') : title}</h3>
      <Steps at={2} />
      <div className="cmd">
        <span className="p" aria-hidden="true">
          $
        </span>
        <code>{'npx skills add ' + d.repo + names.map((n) => ' --skill ' + n).join('') + ' -g -y -a claude-code'}</code>
      </div>
      <p className="tech">Chính lệnh npx skills cài vào ~/.agents/skills và ghi nguồn vào file khoá của nó. App chạy lệnh trong một thư mục trống của app.</p>
      <Lines head="liên kết" right="sau khi cài" lines={lines} />
      <Facts
        items={[
          ['ok', 'Không chạy script nào của skill lúc cài.'],
          ['ok', 'Gói sao lưu ghi nguồn GitHub, không chép nội dung.'],
          ['ok', 'Hoàn tác được: gỡ skill vừa cài, có bản lưu trước khi gỡ.'],
        ]}
      />
      <div className="btns">
        <Button large disabled={skills.working} onClick={() => skills.setDialog({ ...d, step: 'pick' })}>
          Quay lại
        </Button>
        <span className="grow" />
        <Button large disabled={skills.working} onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.install(d)}>
          {skills.working ? 'Đang cài…' : 'Cài'}
        </Button>
      </div>
    </>
  );
}

function ImportBody({ d, skills }: { d: Extract<SkillDialog, { kind: 'import' }>; skills: Skills }) {
  const rows = (skills.table?.rows ?? []).filter((r) => r.importable);
  const pick = (name: string, on: boolean) => skills.setDialog({ ...d, picked: on ? [...d.picked, name] : d.picked.filter((n) => n !== name) });
  const n = d.picked.length;
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Gom {rows.length} skill cài tay về một chỗ</h3>
      <p className="lead">App chuyển các skill này vào thư mục chung của ba agent. Agent nào đang bật vẫn bật như cũ.</p>
      <div role="group" aria-label="Skill cài tay">
        {rows.map((r) => (
          <label key={r.name} className="ck">
            <input type="checkbox" checked={d.picked.includes(r.name)} onChange={(e) => pick(r.name, e.target.checked)} />
            <span>
              <b>{r.name}</b>
              <small>
                đang ở thư mục của {r.home ? AGENT_NAME[r.home] : 'agent'} · {sizeText(r.size)}
              </small>
            </span>
            <small>{onAt(r)}</small>
          </label>
        ))}
      </div>
      <div style={{ marginTop: 10 }}>
        <Facts
          items={[
            ['ok', 'Lưu lại bản cũ trước khi chuyển. Hoàn tác được.'],
            ['ok', 'Không sửa nội dung skill, không chạy file nào trong đó.'],
          ]}
        />
      </div>
      <div className="btns">
        <Button large onClick={skills.close}>
          Để sau
        </Button>
        <Button large primary disabled={!n || skills.working} onClick={() => skills.gather(d.picked)}>
          {skills.working ? 'Đang gom…' : 'Gom ' + n + ' skill'}
        </Button>
      </div>
    </>
  );
}

/** `12/09`: the day a version came, as the prototype writes it. */
function day(iso: string): string {
  const d = new Date(iso);
  return String(d.getDate()).padStart(2, '0') + '/' + String(d.getMonth() + 1).padStart(2, '0');
}

function UpdateBody({ d, skills }: { d: Extract<SkillDialog, { kind: 'update' }>; skills: Skills }) {
  const r = skills.table?.rows.find((x) => x.name === d.name);
  const u = r?.update;
  if (!r || !u) return <Gone skills={skills} />;
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Cập nhật {r.name}?</h3>
      <div className="kvt" style={{ marginBottom: 10 }}>
        <span>Bản trên máy</span>
        <span>{u.fromAt ? day(u.fromAt) : 'không rõ ngày cài'}</span>
        <span>Bản mới</span>
        <span>
          {day(new Date(u.seenAt * 1000).toISOString())} · {r.repo}
        </span>
        <span>Thay đổi</span>
        <span>{u.plain}</span>
      </div>
      {r.edited ? (
        <Banner
          banner={{ tone: 'warn', title: 'Bạn đã sửa skill này trên máy', body: 'Cập nhật sẽ thay phần bạn sửa bằng bản mới. App lưu lại bản đang có để bạn lấy lại.', actions: [] }}
          onAction={() => {}}
        />
      ) : null}
      <Facts
        items={[
          ['ok', 'Agent nào đang bật skill này vẫn bật.'],
          ['ok', 'Lưu lại bản đang có trước khi cập nhật.'],
        ]}
      />
      <p className="tech">Cập nhật bằng npx skills update {r.name}. Không chạy script nào của skill.</p>
      <button type="button" className="lnk" onClick={() => skills.openLink(u.link)}>
        Xem thay đổi trên GitHub
      </button>
      <div className="btns">
        <Button large disabled={skills.working} onClick={() => skills.skip(r.name)}>
          Bỏ qua bản này
        </Button>
        <span className="grow" />
        <Button large disabled={skills.working} onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.update(r.name)}>
          {skills.working ? 'Đang cập nhật…' : 'Cập nhật'}
        </Button>
      </div>
    </>
  );
}

function DelBody({ d, skills }: { d: Extract<SkillDialog, { kind: 'del' }>; skills: Skills }) {
  const r = skills.table?.rows.find((x) => x.name === d.name);
  if (!r) return <Gone skills={skills} />;
  const fromLock = r.source !== 'hand';
  const mode = fromLock ? d.mode : 'off';
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Gỡ {r.name}?</h3>
      <p className="lead">Đang bật ở {onAt(r)}.</p>
      <ChoiceGroup<RowMode>
        id="sk-del"
        label="Cách gỡ"
        value={mode}
        options={[
          { v: 'off', t: 'Tắt ở mọi agent', s: 'Giữ thư mục, bật lại được', until: null },
          { v: 'hard', t: 'Gỡ hẳn', s: fromLock ? 'npx skills remove ' + r.name : 'Skill cài tay: app không xoá thư mục', until: null },
        ]}
        onChange={(m) => skills.setDialog({ ...d, mode: fromLock ? m : 'off' })}
      />
      <div style={{ marginTop: 10 }}>
        <Facts
          items={
            mode === 'hard'
              ? [['ok', 'Lưu thư mục skill trước khi gỡ. Lấy lại được trong 10 bản lưu gần nhất.']]
              : [['ok', 'Xem trước rồi mới ghi. Thư mục skill giữ nguyên.']]
          }
        />
      </div>
      <div className="btns">
        <Button large onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary={mode !== 'hard'} danger={mode === 'hard'} disabled={skills.working} onClick={() => skills.remove(r, mode)}>
          {mode === 'hard' ? (skills.working ? 'Đang gỡ…' : 'Gỡ hẳn') : 'Tiếp: xem trước'}
        </Button>
      </div>
    </>
  );
}

type RowMode = 'off' | 'hard';

function OwnedBody({ skills }: { skills: Skills }) {
  const owned = skills.table?.owned ?? [];
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Skill có sẵn trong agent và plugin</h3>
      <div className="kvt">
        {owned.flatMap((o) => [
          <span key={o.what + ':k'}>{o.what}</span>,
          <span key={o.what + ':v'}>
            {o.names.length} skill · {AGENT_NAME[o.agent]}
            <small style={{ display: 'block', color: 'var(--ink2)' }}>{o.names.join(', ')}</small>
          </span>,
        ])}
      </div>
      <div style={{ marginTop: 10 }}>
        <Facts
          items={[
            ['', 'App chỉ xem các skill này: không bật tắt, không sao lưu.'],
            ['', 'Đăng nhập lại agent hoặc cài lại plugin là chúng tự có lại.'],
          ]}
        />
      </div>
      <div className="btns">
        <Button large primary onClick={skills.close}>
          Đóng
        </Button>
      </div>
    </>
  );
}

function BrokenBody({ d, skills }: { d: Extract<SkillDialog, { kind: 'broken' }>; skills: Skills }) {
  const b = skills.table?.broken.find((x) => x.agent === d.agent && x.name === d.name);
  if (!b) return <Gone skills={skills} />;
  const dir = skills.table?.columns.find((c) => c.agent === b.agent)?.file ?? '';
  const inShared = skills.table?.rows.some((r) => r.name === b.name && r.home == null);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Liên kết hỏng: {b.name}</h3>
      <p className="lead">
        {dir}
        {b.name} trỏ tới một thư mục không còn.
      </p>
      <Lines head={dir} right="bỏ liên kết" lines={[{ op: 'del', text: b.name + ' → ' + b.target + ' (không còn)' }]} />
      <Facts
        items={[
          ['ok', 'Chỉ bỏ liên kết, không xoá thư mục nào.'],
          inShared ? ['', b.name + ' có ở thư mục chung: tick lại ô của ' + AGENT_NAME[b.agent] + ' để bật.'] : ['w', 'Không thấy ' + b.name + ' ở thư mục chung. Muốn dùng lại thì cài lại từ GitHub.'],
        ]}
      />
      <div className="btns">
        <Button large onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.dropLink(b.agent, b.name)}>
          Bỏ liên kết
        </Button>
      </div>
    </>
  );
}

function NpxBody({ skills }: { skills: Skills }) {
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Cài Node.js để có npx</h3>
      <p className="lead">App dùng npx skills để cài và cập nhật skill từ GitHub, nên cần Node.js trên máy này.</p>
      <Facts
        items={[
          ['', 'Cài Node.js bản LTS từ nodejs.org, hoặc bằng Homebrew: brew install node.'],
          ['', 'App tìm npx ở /opt/homebrew/bin, /usr/local/bin và ~/.nvm.'],
          ['ok', 'Bật, tắt và gom skill không cần npx.'],
        ]}
      />
      <div className="btns">
        <Button large onClick={skills.close}>
          Đóng
        </Button>
        <Button
          large
          primary
          onClick={() => {
            skills.close();
            skills.reread();
          }}
        >
          Đọc lại
        </Button>
      </div>
    </>
  );
}

export function SkillDialogBody({ skills, model }: { skills: Skills; model: Model }) {
  const d = skills.dialog;
  if (!d) return null;
  switch (d.kind) {
    case 'batch':
      return <BatchBody d={d} skills={skills} />;
    case 'gh':
      return d.step === 'find' ? <GhFind d={d} skills={skills} /> : d.step === 'pick' ? <GhPick d={d} skills={skills} model={model} /> : <GhPreview d={d} skills={skills} />;
    case 'import':
      return <ImportBody d={d} skills={skills} />;
    case 'update':
      return <UpdateBody d={d} skills={skills} />;
    case 'del':
      return <DelBody d={d} skills={skills} />;
    case 'owned':
      return <OwnedBody skills={skills} />;
    case 'broken':
      return <BrokenBody d={d} skills={skills} />;
    case 'npx':
      return <NpxBody skills={skills} />;
  }
}

