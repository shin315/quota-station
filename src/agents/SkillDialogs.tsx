// The Skills tab's dialogs (M11): the batch preview every box goes through,
// installing from GitHub in three steps (the repository, the skills with
// their SKILL.md to read first, the preview), gathering the skills installed
// by hand, a newer version, removing, the skills agents and plugins bring,
// a link to nothing, and how to get `npx`. No dialog runs a skill.

import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { CheckLine, CheckRow, Command, DiffFile, DiffLine, DiffView, Steps as StepList, TECH, type LineOp } from '../components/blocks';
import { Banner, Button, Chip, ChoiceGroup, FieldHint, KVT, LinkButton } from '../components/parts';
import { AGENT_NAME, AGENTS, RESTART } from '../lib/mcp';
import type { Agent, Model } from '../lib/model';
import { groupFile, onAt, sizeText, skillRefusalText } from '../lib/skills';
import { AgentLogo } from './CheckMatrix';
import { Facts } from './McpDialogs';
import type { SkillBatch, SkillDialog, SkillGh, Skills } from './useSkills';

/** The dialog's width: the preview and the repository's list need room. */
export function skillDialogSize(d: SkillDialog): 'lg' | 'xl' | 'wide' {
  if (d.kind === 'batch' || (d.kind === 'gh' && d.step !== 'find')) return 'xl';
  return 'lg';
}

function Steps({ at }: { at: 0 | 1 | 2 }) {
  return <StepList names={['Repo', 'Chọn skill', 'Cài']} at={at} />;
}

export function Lines({ head, right, lines }: { head: string; right: string; lines: { op: LineOp; text: string }[] }) {
  return (
    <DiffView head={<DiffFile>{head}</DiffFile>} right={right}>
      {lines.map((l, i) => (
        <DiffLine key={i} op={l.op}>
          {l.op === 'gap' ? '…' : l.text || ' '}
        </DiffLine>
      ))}
    </DiffView>
  );
}

function Gone({ skills }: { skills: Skills }) {
  return (
    <>
      <DialogTitle>Không còn skill này</DialogTitle>
      <DialogLead>Danh sách vừa đổi. Bấm Đọc lại rồi thử lại.</DialogLead>
      <DialogButtons>
        <Button large primary onClick={skills.close}>
          Đóng
        </Button>
      </DialogButtons>
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
      <DialogTitle>{d.title}</DialogTitle>
      <DialogLead>Không chép, không sửa nội dung skill.</DialogLead>
      {p.groups.map((g) => (
        <section key={g.agent} aria-label={AGENT_NAME[g.agent]}>
          <DialogSection>
            <AgentLogo agent={g.agent} />
            {AGENT_NAME[g.agent]}
          </DialogSection>
          <Lines head={g.file} right={g.agent === 'codex' ? g.count + ' skill' : g.count + ' liên kết'} lines={g.lines} />
          {g.agent === 'codex' ? <p className={TECH}>Codex đọc thẳng thư mục chung, nên bật hay tắt cho Codex là một mục [[skills.config]] trong config.toml.</p> : null}
        </section>
      ))}
      <Facts items={facts} />
      <DialogButtons>
        <Button large disabled={skills.working} onClick={skills.close}>
          Quay lại
        </Button>
        <Button large primary disabled={skills.working} onClick={skills.write}>
          {skills.working ? 'Đang ghi…' : 'Ghi ' + n + ' thay đổi'}
        </Button>
      </DialogButtons>
    </>
  );
}

function AgentTicks(props: { model: Model; value: Record<Agent, boolean>; onChange(v: Record<Agent, boolean>): void }) {
  const cols = props.model.skills.table?.columns ?? [];
  return (
    <div role="group" aria-label="Bật cho" className="flex flex-wrap gap-x-3.5 gap-y-1">
      {AGENTS.map((a) => {
        const can = !!cols.find((c) => c.agent === a)?.writable;
        return (
          <CheckLine
            key={a}
            className="m-0"
            title={can ? undefined : AGENT_NAME[a] + ': chỉ xem trên máy này'}
            checked={can && props.value[a]}
            disabled={!can}
            onChange={(on) => props.onChange({ ...props.value, [a]: on })}
          >
            {AGENT_NAME[a]}
          </CheckLine>
        );
      })}
    </div>
  );
}

function GhFind({ d, skills }: { d: SkillGh; skills: Skills }) {
  const go = () => d.repo.trim() && skills.findRepo(d);
  return (
    <>
      <DialogTitle>Cài skill từ GitHub</DialogTitle>
      <Steps at={0} />
      <label className="text-s font-semi" htmlFor="gh-repo">
        Repo hoặc link GitHub
      </label>
      <Input
        id="gh-repo"
        mono
        invalid={!!d.error}
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
        <FieldHint tone="bad" id="gh-repo-e">
          {d.error}
        </FieldHint>
      ) : null}
      <div className="mt-2.5">
        <Facts items={[['w', 'Skill có thể chạy lệnh trên máy khi agent dùng nó. Chỉ cài từ nguồn bạn tin.']]} />
      </div>
      <DialogButtons>
        <Button large onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={!d.repo.trim() || skills.working} onClick={go}>
          {skills.working ? 'Đang tìm…' : 'Tìm skill'}
        </Button>
      </DialogButtons>
    </>
  );
}

function GhPick({ d, skills, model }: { d: SkillGh; skills: Skills; model: Model }) {
  const list = d.list?.skills ?? [];
  const pick = (name: string, on: boolean) => skills.setDialog({ ...d, picked: on ? [...d.picked, name] : d.picked.filter((n) => n !== name) });
  if (d.reading)
    return (
      <>
        <DialogTitle>SKILL.md của {d.reading.name}</DialogTitle>
        <DialogLead>Chỉ đọc. Cài xong, agent làm theo hướng dẫn này khi dùng skill.</DialogLead>
        {d.reading.error ? (
          <p>{d.reading.error}</p>
        ) : d.reading.text == null ? (
          <p>Đang đọc từ {d.repo}…</p>
        ) : (
          <DiffView>
            {d.reading.text.split('\n').map((l, i) => (
              <DiffLine key={i} op="ctx">
                {l || ' '}
              </DiffLine>
            ))}
          </DiffView>
        )}
        <DialogButtons>
          <Button large primary onClick={() => skills.setDialog({ ...d, reading: null })}>
            Quay lại danh sách
          </Button>
        </DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>Chọn skill trong {d.repo}</DialogTitle>
      <Steps at={1} />
      <div role="group" aria-label="Skill trong repo">
        {list.map((s) => (
          <CheckRow key={s.name} off={s.installed} checked={!s.installed && d.picked.includes(s.name)} disabled={s.installed} onChange={(on) => pick(s.name, on)}>
            <span>
              <b>{s.name}</b>
              <small>{s.description}</small>
            </span>
            {s.installed ? (
              <Chip chip={{ t: 'đã cài', k: 'ok' }} />
            ) : (
              <LinkButton small aria-label={'Đọc SKILL.md của ' + s.name} onClick={(e) => (e.preventDefault(), skills.readMd(d, s.name))}>
                Đọc SKILL.md
              </LinkButton>
            )}
          </CheckRow>
        ))}
      </div>
      <DialogSection>Bật cho</DialogSection>
      <AgentTicks model={model} value={d.agents} onChange={(agents) => skills.setDialog({ ...d, agents })} />
      <DialogButtons>
        <Button large onClick={() => skills.setDialog({ ...d, step: 'find' })}>
          Quay lại
        </Button>
        <span className="flex-1" />
        <Button large onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={!d.picked.length} onClick={() => skills.setDialog({ ...d, step: 'preview' })}>
          Tiếp: xem trước
        </Button>
      </DialogButtons>
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
      <DialogTitle>{skills.working ? 'Đang cài ' + names.join(', ') : title}</DialogTitle>
      <Steps at={2} />
      <Command>{'npx skills add ' + d.repo + names.map((n) => ' --skill ' + n).join('') + ' -g -y -a claude-code'}</Command>
      <p className={TECH}>Chính lệnh npx skills cài vào ~/.agents/skills và ghi nguồn vào file khoá của nó. App chạy lệnh trong một thư mục trống của app.</p>
      <Lines head="liên kết" right="sau khi cài" lines={lines} />
      <Facts
        items={[
          ['ok', 'Không chạy script nào của skill lúc cài.'],
          ['ok', 'Gói sao lưu ghi nguồn GitHub, không chép nội dung.'],
          ['ok', 'Hoàn tác được: gỡ skill vừa cài, có bản lưu trước khi gỡ.'],
        ]}
      />
      <DialogButtons>
        <Button large disabled={skills.working} onClick={() => skills.setDialog({ ...d, step: 'pick' })}>
          Quay lại
        </Button>
        <span className="flex-1" />
        <Button large disabled={skills.working} onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.install(d)}>
          {skills.working ? 'Đang cài…' : 'Cài'}
        </Button>
      </DialogButtons>
    </>
  );
}

function ImportBody({ d, skills }: { d: Extract<SkillDialog, { kind: 'import' }>; skills: Skills }) {
  const rows = (skills.table?.rows ?? []).filter((r) => r.importable);
  const pick = (name: string, on: boolean) => skills.setDialog({ ...d, picked: on ? [...d.picked, name] : d.picked.filter((n) => n !== name) });
  const n = d.picked.length;
  return (
    <>
      <DialogTitle>Gom {rows.length} skill cài tay về một chỗ</DialogTitle>
      <DialogLead>App chuyển các skill này vào thư mục chung của ba agent. Agent nào đang bật vẫn bật như cũ.</DialogLead>
      <div role="group" aria-label="Skill cài tay">
        {rows.map((r) => (
          <CheckRow key={r.name} checked={d.picked.includes(r.name)} onChange={(on) => pick(r.name, on)}>
            <span>
              <b>{r.name}</b>
              <small>
                đang ở thư mục của {r.home ? AGENT_NAME[r.home] : 'agent'} · {sizeText(r.size)}
              </small>
            </span>
            <small>{onAt(r)}</small>
          </CheckRow>
        ))}
      </div>
      <div className="mt-2.5">
        <Facts
          items={[
            ['ok', 'Lưu lại bản cũ trước khi chuyển. Hoàn tác được.'],
            ['ok', 'Không sửa nội dung skill, không chạy file nào trong đó.'],
          ]}
        />
      </div>
      <DialogButtons>
        <Button large onClick={skills.close}>
          Để sau
        </Button>
        <Button large primary disabled={!n || skills.working} onClick={() => skills.gather(d.picked)}>
          {skills.working ? 'Đang gom…' : 'Gom ' + n + ' skill'}
        </Button>
      </DialogButtons>
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
      <DialogTitle>Cập nhật {r.name}?</DialogTitle>
      <div className={cn(KVT, 'mb-2.5')}>
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
      <p className={TECH}>Cập nhật bằng npx skills update {r.name}. Không chạy script nào của skill.</p>
      <LinkButton onClick={() => skills.openLink(u.link)}>Xem thay đổi trên GitHub</LinkButton>
      <DialogButtons>
        <Button large disabled={skills.working} onClick={() => skills.skip(r.name)}>
          Bỏ qua bản này
        </Button>
        <span className="flex-1" />
        <Button large disabled={skills.working} onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.update(r.name)}>
          {skills.working ? 'Đang cập nhật…' : 'Cập nhật'}
        </Button>
      </DialogButtons>
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
      <DialogTitle>Gỡ {r.name}?</DialogTitle>
      <DialogLead>Đang bật ở {onAt(r)}.</DialogLead>
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
      <div className="mt-2.5">
        <Facts
          items={
            mode === 'hard'
              ? [['ok', 'Lưu thư mục skill trước khi gỡ. Lấy lại được trong 10 bản lưu gần nhất.']]
              : [['ok', 'Xem trước rồi mới ghi. Thư mục skill giữ nguyên.']]
          }
        />
      </div>
      <DialogButtons>
        <Button large onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary={mode !== 'hard'} danger={mode === 'hard'} disabled={skills.working} onClick={() => skills.remove(r, mode)}>
          {mode === 'hard' ? (skills.working ? 'Đang gỡ…' : 'Gỡ hẳn') : 'Tiếp: xem trước'}
        </Button>
      </DialogButtons>
    </>
  );
}

type RowMode = 'off' | 'hard';

function OwnedBody({ skills }: { skills: Skills }) {
  const owned = skills.table?.owned ?? [];
  return (
    <>
      <DialogTitle>Skill có sẵn trong agent và plugin</DialogTitle>
      <div className={KVT}>
        {owned.flatMap((o) => [
          <span key={o.what + ':k'}>{o.what}</span>,
          <span key={o.what + ':v'}>
            {o.names.length} skill · {AGENT_NAME[o.agent]}
            <small className="block text-ink2">{o.names.join(', ')}</small>
          </span>,
        ])}
      </div>
      <div className="mt-2.5">
        <Facts
          items={[
            ['', 'App chỉ xem các skill này: không bật tắt, không sao lưu.'],
            ['', 'Đăng nhập lại agent hoặc cài lại plugin là chúng tự có lại.'],
          ]}
        />
      </div>
      <DialogButtons>
        <Button large primary onClick={skills.close}>
          Đóng
        </Button>
      </DialogButtons>
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
      <DialogTitle>Liên kết hỏng: {b.name}</DialogTitle>
      <DialogLead>
        {dir}
        {b.name} trỏ tới một thư mục không còn.
      </DialogLead>
      <Lines head={dir} right="bỏ liên kết" lines={[{ op: 'del', text: b.name + ' → ' + b.target + ' (không còn)' }]} />
      <Facts
        items={[
          ['ok', 'Chỉ bỏ liên kết, không xoá thư mục nào.'],
          inShared ? ['', b.name + ' có ở thư mục chung: tick lại ô của ' + AGENT_NAME[b.agent] + ' để bật.'] : ['w', 'Không thấy ' + b.name + ' ở thư mục chung. Muốn dùng lại thì cài lại từ GitHub.'],
        ]}
      />
      <DialogButtons>
        <Button large onClick={skills.close}>
          Huỷ
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.dropLink(b.agent, b.name)}>
          Bỏ liên kết
        </Button>
      </DialogButtons>
    </>
  );
}

function NpxBody({ skills }: { skills: Skills }) {
  return (
    <>
      <DialogTitle>Cài Node.js để có npx</DialogTitle>
      <DialogLead>App dùng npx skills để cài và cập nhật skill từ GitHub, nên cần Node.js trên máy này.</DialogLead>
      <Facts
        items={[
          ['', 'Cài Node.js bản LTS từ nodejs.org, hoặc bằng Homebrew: brew install node.'],
          ['', 'App tìm npx ở /opt/homebrew/bin, /usr/local/bin và ~/.nvm.'],
          ['ok', 'Bật, tắt và gom skill không cần npx.'],
        ]}
      />
      <DialogButtons>
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
      </DialogButtons>
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

