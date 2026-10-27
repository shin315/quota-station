// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Skills tab's dialogs (M11): the batch preview every box goes through,
// installing from GitHub in three steps (the repository, the skills with
// their SKILL.md to read first, the preview), gathering the skills installed
// by hand, a newer version (one skill, or several in one preview), removing,
// the skills agents and plugins bring, a link to nothing, and how to get
// `npx`. No dialog runs a skill.

import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { CheckLine, CheckRow, Command, DiffFile, DiffLine, DiffView, Steps as StepList, TECH, type LineOp } from '../components/blocks';
import { Banner, Button, Chip, ChoiceGroup, FieldHint, KVT, LinkButton } from '../components/parts';
import { mac, os, t } from '../lib/i18n';
import { AGENT_NAME, AGENTS, restartText } from '../lib/mcp';
import type { Agent, Model } from '../lib/model';
import { groupFile, onAt, sizeText, skillRefusalText } from '../lib/skills';
import { AgentLogo } from './CheckMatrix';
import { Facts } from './McpDialogs';
import type { SkillBatch, SkillDialog, SkillGh, Skills, SkillUpdateMany } from './useSkills';

/** The dialog's width: the preview and the repository's list need room. */
export function skillDialogSize(d: SkillDialog): 'lg' | 'xl' | 'wide' {
  if (d.kind === 'batch' || (d.kind === 'gh' && d.step !== 'find')) return 'xl';
  return 'lg';
}

/** "3 skills", in English: the Vietnamese has no plural. */
const skillsEn = (n: number): string => n + (n === 1 ? ' skill' : ' skills');

function Steps({ at }: { at: 0 | 1 | 2 }) {
  return <StepList names={['Repo', t('Chọn skill', 'Pick skills'), t('Cài', 'Install')]} at={at} />;
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
      <DialogTitle>{t('Không còn skill này', 'This skill is gone')}</DialogTitle>
      <DialogLead>{t('Danh sách vừa đổi. Bấm Đọc lại rồi thử lại.', 'The list just changed. Press Read again, then try again.')}</DialogLead>
      <DialogButtons>
        <Button large primary onClick={skills.close}>
          {t('Đóng', 'Close')}
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
    ['ok', t('Sao lưu ' + files + ' trước khi ghi. Hoàn tác được.', 'Backs up ' + files + ' before writing. Can be undone.')],
    ['ok', t('Tắt là bỏ liên kết hay thêm một mục trong config.toml: thư mục skill giữ nguyên.', 'Turning off removes a link or adds an entry to config.toml: the skill folder stays as it is.')],
    ...p.refused.map((r): [string, string] => [
      'x',
      t('Bỏ qua ' + r.name + ' cho ' + AGENT_NAME[r.agent] + ': ' + skillRefusalText(r.reason) + '.', 'Skipping ' + r.name + ' for ' + AGENT_NAME[r.agent] + ': ' + skillRefusalText(r.reason) + '.'),
    ]),
    ...p.groups.map((g): [string, string] => ['w', restartText(g.agent)]),
  ];
  return (
    <>
      <DialogTitle>{d.title}</DialogTitle>
      <DialogLead>{t('Không chép, không sửa nội dung skill.', 'Does not copy or edit what is in a skill.')}</DialogLead>
      {p.groups.map((g) => (
        <section key={g.agent} aria-label={AGENT_NAME[g.agent]}>
          <DialogSection>
            <AgentLogo agent={g.agent} />
            {AGENT_NAME[g.agent]}
          </DialogSection>
          <Lines head={g.file} right={g.agent === 'codex' ? t(g.count + ' skill', skillsEn(g.count)) : t(g.count + ' liên kết', g.count + (g.count === 1 ? ' link' : ' links'))} lines={g.lines} />
          {g.agent === 'codex' ? <p className={TECH}>
              {t(
                'Codex đọc thẳng thư mục chung, nên bật hay tắt cho Codex là một mục [[skills.config]] trong config.toml.',
                'Codex reads the shared folder directly, so turning a skill on or off for Codex is a [[skills.config]] entry in config.toml.',
              )}
            </p> : null}
        </section>
      ))}
      <Facts items={facts} />
      <DialogButtons>
        <Button large disabled={skills.working} onClick={skills.close}>
          {t('Quay lại', 'Back')}
        </Button>
        <Button large primary disabled={skills.working} onClick={skills.write}>
          {skills.working ? t('Đang ghi…', 'Writing…') : t('Ghi ' + n + ' thay đổi', n === 1 ? 'Write 1 change' : 'Write ' + n + ' changes')}
        </Button>
      </DialogButtons>
    </>
  );
}

function AgentTicks(props: { model: Model; value: Record<Agent, boolean>; onChange(v: Record<Agent, boolean>): void }) {
  const cols = props.model.skills.table?.columns ?? [];
  return (
    <div role="group" aria-label={t('Bật cho', 'Turn on for')} className="flex flex-wrap gap-x-3.5 gap-y-1">
      {AGENTS.map((a) => {
        const can = !!cols.find((c) => c.agent === a)?.writable;
        return (
          <CheckLine
            key={a}
            className="m-0"
            title={can ? undefined : mac(t(AGENT_NAME[a] + ': chỉ xem trên máy này', AGENT_NAME[a] + ': view only on this Mac'))}
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
      <DialogTitle>{t('Cài skill từ GitHub', 'Install skills from GitHub')}</DialogTitle>
      <Steps at={0} />
      <label className="text-s font-semi" htmlFor="gh-repo">
        {t('Repo hoặc link GitHub', 'Repo or GitHub link')}
      </label>
      <Input
        id="gh-repo"
        mono
        invalid={!!d.error}
        value={d.repo}
        placeholder={t('chủ/tên-repo', 'owner/repo-name')}
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
        <Facts items={[['w', mac(t('Skill có thể chạy lệnh trên máy khi agent dùng nó. Chỉ cài từ nguồn bạn tin.', 'A skill can run commands on this Mac when an agent uses it. Only install from sources you trust.'))]]} />
      </div>
      <DialogButtons>
        <Button large onClick={skills.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={!d.repo.trim() || skills.working} onClick={go}>
          {skills.working ? t('Đang tìm…', 'Searching…') : t('Tìm skill', 'Find skills')}
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
        <DialogTitle>{t('SKILL.md của ' + d.reading.name, 'SKILL.md of ' + d.reading.name)}</DialogTitle>
        <DialogLead>{t('Chỉ đọc. Cài xong, agent làm theo hướng dẫn này khi dùng skill.', 'Read only. Once installed, the agent follows these instructions when it uses the skill.')}</DialogLead>
        {d.reading.error ? (
          <p>{d.reading.error}</p>
        ) : d.reading.text == null ? (
          <p>{t('Đang đọc từ ' + d.repo + '…', 'Reading from ' + d.repo + '…')}</p>
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
            {t('Quay lại danh sách', 'Back to the list')}
          </Button>
        </DialogButtons>
      </>
    );
  return (
    <>
      <DialogTitle>{t('Chọn skill trong ' + d.repo, 'Pick skills in ' + d.repo)}</DialogTitle>
      <Steps at={1} />
      <div role="group" aria-label={t('Skill trong repo', 'Skills in the repo')}>
        {list.map((s) => (
          <CheckRow key={s.name} off={s.installed} checked={!s.installed && d.picked.includes(s.name)} disabled={s.installed} onChange={(on) => pick(s.name, on)}>
            <span>
              <b>{s.name}</b>
              <small>{s.description}</small>
            </span>
            {s.installed ? (
              <Chip chip={{ t: t('đã cài', 'installed'), k: 'ok' }} />
            ) : (
              <LinkButton small aria-label={t('Đọc SKILL.md của ' + s.name, 'Read SKILL.md of ' + s.name)} onClick={(e) => (e.preventDefault(), skills.readMd(d, s.name))}>
                {t('Đọc SKILL.md', 'Read SKILL.md')}
              </LinkButton>
            )}
          </CheckRow>
        ))}
      </div>
      <DialogSection>{t('Bật cho', 'Turn on for')}</DialogSection>
      <AgentTicks model={model} value={d.agents} onChange={(agents) => skills.setDialog({ ...d, agents })} />
      <DialogButtons>
        <Button large onClick={() => skills.setDialog({ ...d, step: 'find' })}>
          {t('Quay lại', 'Back')}
        </Button>
        <span className="flex-1" />
        <Button large onClick={skills.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={!d.picked.length} onClick={() => skills.setDialog({ ...d, step: 'preview' })}>
          {t('Tiếp: xem trước', 'Next: preview')}
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
    { op: 'ctx' as const, text: d.agents.codex
        ? t('Codex đọc thẳng thư mục chung', 'Codex reads the shared folder directly')
        : t('Codex: thêm mục [[skills.config]] name = "' + n + '", enabled = false', 'Codex: adds the entry [[skills.config]] name = "' + n + '", enabled = false'),
    },
  ]);
  const title = names.length === 1 ? t('Cài ' + names[0] + '?', 'Install ' + names[0] + '?') : t('Cài ' + names.length + ' skill?', 'Install ' + names.length + ' skills?');
  return (
    <>
      <DialogTitle>{skills.working ? t('Đang cài ' + names.join(', '), 'Installing ' + names.join(', ')) : title}</DialogTitle>
      <Steps at={2} />
      <Command>{'npx skills add ' + d.repo + names.map((n) => ' --skill ' + n).join('') + ' -g -y -a claude-code'}</Command>
      <p className={TECH}>
        {t(
          'Chính lệnh npx skills cài vào ~/.agents/skills và ghi nguồn vào file khoá của nó. App chạy lệnh trong một thư mục trống của app.',
          'npx skills itself installs into ~/.agents/skills and records the source in its lock file. The app runs the command in an empty folder of its own.',
        )}
      </p>
      <Lines head={t('liên kết', 'links')} right={t('sau khi cài', 'after install')} lines={lines} />
      <Facts
        items={[
          ['ok', t('Không chạy script nào của skill lúc cài.', 'Runs none of the skill’s scripts while installing.')],
          ['ok', t('Gói sao lưu ghi nguồn GitHub, không chép nội dung.', 'The backup pack records the GitHub source, not the contents.')],
          ['ok', t('Hoàn tác được: gỡ skill vừa cài, có bản lưu trước khi gỡ.', 'Can be undone: removes the skill just installed, with a backup before removing.')],
        ]}
      />
      <DialogButtons>
        <Button large disabled={skills.working} onClick={() => skills.setDialog({ ...d, step: 'pick' })}>
          {t('Quay lại', 'Back')}
        </Button>
        <span className="flex-1" />
        <Button large disabled={skills.working} onClick={skills.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.install(d)}>
          {skills.working ? t('Đang cài…', 'Installing…') : t('Cài', 'Install')}
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
      <DialogTitle>{t('Gom ' + rows.length + ' skill cài tay về một chỗ', 'Gather ' + rows.length + (rows.length === 1 ? ' hand-installed skill' : ' hand-installed skills') + ' in one place')}</DialogTitle>
      <DialogLead>
        {t(
          'App chuyển các skill này vào thư mục chung của ba agent. Agent nào đang bật vẫn bật như cũ.',
          'The app moves these skills into the folder the three agents share. Agents that have them on keep them on.',
        )}
      </DialogLead>
      <div role="group" aria-label={t('Skill cài tay', 'Skills installed by hand')}>
        {rows.map((r) => (
          <CheckRow key={r.name} checked={d.picked.includes(r.name)} onChange={(on) => pick(r.name, on)}>
            <span>
              <b>{r.name}</b>
              <small>
                {t('đang ở thư mục của ' + (r.home ? AGENT_NAME[r.home] : 'agent'), 'in ' + (r.home ? AGENT_NAME[r.home] + '’s' : 'the agent’s') + ' folder')} · {sizeText(r.size)}
              </small>
            </span>
            <small>{onAt(r)}</small>
          </CheckRow>
        ))}
      </div>
      <div className="mt-2.5">
        <Facts
          items={[
            ['ok', t('Lưu lại bản cũ trước khi chuyển. Hoàn tác được.', 'Keeps a copy of the old version before moving. Can be undone.')],
            ['ok', t('Không sửa nội dung skill, không chạy file nào trong đó.', 'Does not edit the skill or run any file in it.')],
          ]}
        />
      </div>
      <DialogButtons>
        <Button large onClick={skills.close}>
          {t('Để sau', 'Later')}
        </Button>
        <Button large primary disabled={!n || skills.working} onClick={() => skills.gather(d.picked)}>
          {skills.working ? t('Đang gom…', 'Gathering…') : t('Gom ' + n + ' skill', 'Gather ' + skillsEn(n))}
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
      <DialogTitle>{t('Cập nhật ' + r.name + '?', 'Update ' + r.name + '?')}</DialogTitle>
      <div className={cn(KVT, 'mb-2.5')}>
        <span>{mac(t('Bản trên máy', 'Version on this Mac'))}</span>
        <span>{u.fromAt ? day(u.fromAt) : t('không rõ ngày cài', 'install date unknown')}</span>
        <span>{t('Bản mới', 'New version')}</span>
        <span>
          {day(new Date(u.seenAt * 1000).toISOString())} · {r.repo}
        </span>
        <span>{t('Thay đổi', 'Changes')}</span>
        <span>{u.plain}</span>
      </div>
      {r.edited ? (
        <Banner
          banner={{
            tone: 'warn',
            title: mac(t('Bạn đã sửa skill này trên máy', 'You edited this skill on this Mac')),
            body: t(
              'Cập nhật sẽ thay phần bạn sửa bằng bản mới. App lưu lại bản đang có để bạn lấy lại.',
              'Updating replaces your edits with the new version. The app keeps a copy of the current one so you can get it back.',
            ),
            actions: [],
          }}
          onAction={() => {}}
        />
      ) : null}
      <Facts
        items={[
          ['ok', t('Agent nào đang bật skill này vẫn bật.', 'Agents that have this skill on keep it on.')],
          ['ok', t('Lưu lại bản đang có trước khi cập nhật.', 'Keeps a copy of the current version before updating.')],
        ]}
      />
      <p className={TECH}>
        {t(
          'Cập nhật bằng npx skills update ' + r.name + '. Không chạy script nào của skill.',
          'Updates with npx skills update ' + r.name + '. Runs none of the skill’s scripts.',
        )}
      </p>
      <LinkButton onClick={() => skills.openLink(u.link)}>{t('Xem thay đổi trên GitHub', 'See the changes on GitHub')}</LinkButton>
      <DialogButtons>
        <Button large disabled={skills.working} onClick={() => skills.skip(r.name)}>
          {t('Bỏ qua bản này', 'Skip this version')}
        </Button>
        <span className="flex-1" />
        <Button large disabled={skills.working} onClick={skills.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.update(r.name)}>
          {skills.working ? t('Đang cập nhật…', 'Updating…') : t('Cập nhật', 'Update')}
        </Button>
      </DialogButtons>
    </>
  );
}

function UpdateManyBody({ d, skills, model }: { d: SkillUpdateMany; skills: Skills; model: Model }) {
  const rows = (skills.table?.rows ?? []).filter((r) => r.update && d.names.includes(r.name));
  if (!rows.length) return <Gone skills={skills} />;
  const picked = rows.map((r) => r.name).filter((n) => d.picked.includes(n));
  const pick = (name: string, on: boolean) => skills.setDialog({ ...d, picked: on ? [...d.picked, name] : d.picked.filter((n) => n !== name) });
  const edited = rows.filter((r) => r.edited);
  const p = skills.working ? model.skills.progress : null;
  const n = picked.length;
  return (
    <>
      <DialogTitle>
        {p
          ? t('Đang cập nhật ' + (p.done + 1) + '/' + p.total + ': ' + p.name, 'Updating ' + (p.done + 1) + '/' + p.total + ': ' + p.name)
          : d.repo
            ? t('Cập nhật cả bộ ' + d.repo + '?', 'Update the whole ' + d.repo + ' set?')
            : t('Cập nhật tất cả skill có bản mới?', 'Update every skill that has an update?')}
      </DialogTitle>
      <DialogLead>
        {t(
          rows.length + ' skill có bản mới. Bỏ tick skill nào muốn để sau.',
          rows.length + (rows.length === 1 ? ' skill has an update.' : ' skills have updates.') + ' Untick any you want to leave for later.',
        )}
      </DialogLead>
      {edited.length ? (
        <Banner
          banner={{
            tone: 'warn',
            title: mac(t('Bạn đã sửa skill này trên máy', 'You edited this skill on this Mac')),
            body: t(
              edited.map((r) => r.name).join(', ') + ' chưa tick, để giữ phần bạn sửa. Tick thì bản mới thay phần bạn sửa; app lưu lại bản đang có để bạn lấy lại.',
              edited.map((r) => r.name).join(', ') +
                ' left unticked to keep your edits. Ticked, the new version replaces your edits; the app keeps a copy of the current one so you can get it back.',
            ),
            actions: [],
          }}
          onAction={() => {}}
        />
      ) : null}
      <div role="group" aria-label={t('Skill sẽ cập nhật', 'Skills to update')}>
        {rows.map((r) => (
          <CheckRow key={r.name} checked={picked.includes(r.name)} disabled={skills.working} onChange={(on) => pick(r.name, on)}>
            <span>
              <b>{r.name}</b>
              <small>
                {(r.update!.fromAt ? day(r.update!.fromAt) : t('không rõ ngày cài', 'install date unknown')) + ' → ' + day(new Date(r.update!.seenAt * 1000).toISOString()) + ' · ' + r.update!.plain}
                {d.repo ? null : ' · ' + (r.repo ?? t('không rõ nguồn', 'source unknown'))}
              </small>
            </span>
            {r.edited ? <Chip chip={{ t: t('bạn đã sửa', 'you edited it'), k: 'warn' }} /> : null}
          </CheckRow>
        ))}
      </div>
      <div className="mt-2.5">
        <Facts
          items={[
            ['ok', t('Agent nào đang bật skill vẫn bật.', 'Agents that have a skill on keep it on.')],
            ['ok', t('Lưu lại bản đang có của từng skill trước khi cập nhật.', 'Keeps a copy of each skill’s current version before updating.')],
          ]}
        />
      </div>
      <p className={TECH}>
        {t(
          'Cập nhật ' + n + ' skill, lần lượt bằng npx skills update <tên> -g -y. Không chạy script nào của skill.',
          'Updates ' + skillsEn(n) + ', one at a time, with npx skills update <name> -g -y. Runs none of the skills’ scripts.',
        )}
      </p>
      <DialogButtons>
        <Button large disabled={skills.working} onClick={skills.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={!n || skills.working} onClick={() => skills.updateMany(picked)}>
          {skills.working ? t('Đang cập nhật…', 'Updating…') : t('Cập nhật ' + n + ' skill', 'Update ' + skillsEn(n))}
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
      <DialogTitle>{t('Gỡ ' + r.name + '?', 'Remove ' + r.name + '?')}</DialogTitle>
      <DialogLead>{t('Đang bật ở ' + onAt(r) + '.', 'On in ' + onAt(r) + '.')}</DialogLead>
      <ChoiceGroup<RowMode>
        id="sk-del"
        label={t('Cách gỡ', 'How to remove')}
        value={mode}
        options={[
          { v: 'off', t: t('Tắt ở mọi agent', 'Turn off for every agent'), s: t('Giữ thư mục, bật lại được', 'Keeps the folder; can be turned back on'), until: null },
          {
            v: 'hard',
            t: t('Gỡ hẳn', 'Remove for good'),
            s: fromLock ? 'npx skills remove ' + r.name : t('Skill cài tay: app không xoá thư mục', 'Installed by hand: the app does not delete the folder'),
            until: null,
          },
        ]}
        onChange={(m) => skills.setDialog({ ...d, mode: fromLock ? m : 'off' })}
      />
      <div className="mt-2.5">
        <Facts
          items={
            mode === 'hard'
              ? [['ok', t('Lưu thư mục skill trước khi gỡ. Lấy lại được trong 10 bản lưu gần nhất.', 'Backs up the skill folder before removing. It can be recovered from the 10 latest backups.')]]
              : [['ok', t('Xem trước rồi mới ghi. Thư mục skill giữ nguyên.', 'Previews before writing. The skill folder stays as it is.')]]
          }
        />
      </div>
      <DialogButtons>
        <Button large onClick={skills.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary={mode !== 'hard'} danger={mode === 'hard'} disabled={skills.working} onClick={() => skills.remove(r, mode)}>
          {mode === 'hard' ? (skills.working ? t('Đang gỡ…', 'Removing…') : t('Gỡ hẳn', 'Remove for good')) : t('Tiếp: xem trước', 'Next: preview')}
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
      <DialogTitle>{t('Skill có sẵn trong agent và plugin', 'Skills built into agents and plugins')}</DialogTitle>
      <div className={KVT}>
        {owned.flatMap((o) => [
          <span key={o.what + ':k'}>{o.what}</span>,
          <span key={o.what + ':v'}>
            {t(o.names.length + ' skill', skillsEn(o.names.length))} · {AGENT_NAME[o.agent]}
            <small className="block text-ink2">{o.names.join(', ')}</small>
          </span>,
        ])}
      </div>
      <div className="mt-2.5">
        <Facts
          items={[
            ['', t('App chỉ xem các skill này: không bật tắt, không sao lưu.', 'The app only shows these skills: it does not turn them on or off or back them up.')],
            ['', t('Đăng nhập lại agent hoặc cài lại plugin là chúng tự có lại.', 'Signing in to the agent again or reinstalling the plugin brings them back.')],
          ]}
        />
      </div>
      <DialogButtons>
        <Button large primary onClick={skills.close}>
          {t('Đóng', 'Close')}
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
      <DialogTitle>{t('Liên kết hỏng: ' + b.name, 'Broken link: ' + b.name)}</DialogTitle>
      <DialogLead>
        {dir}
        {b.name}
        {t(' trỏ tới một thư mục không còn.', ' points to a folder that is gone.')}
      </DialogLead>
      <Lines head={dir} right={t('bỏ liên kết', 'remove link')} lines={[{ op: 'del', text: b.name + ' → ' + b.target + t(' (không còn)', ' (gone)') }]} />
      <Facts
        items={[
          ['ok', t('Chỉ bỏ liên kết, không xoá thư mục nào.', 'Only removes the link; deletes no folder.')],
          inShared
            ? ['', t(b.name + ' có ở thư mục chung: tick lại ô của ' + AGENT_NAME[b.agent] + ' để bật.', b.name + ' is in the shared folder: tick ' + AGENT_NAME[b.agent] + '’s box again to turn it on.')]
            : ['w', t('Không thấy ' + b.name + ' ở thư mục chung. Muốn dùng lại thì cài lại từ GitHub.', b.name + ' is not in the shared folder. To use it again, reinstall it from GitHub.')],
        ]}
      />
      <DialogButtons>
        <Button large onClick={skills.close}>
          {t('Huỷ', 'Cancel')}
        </Button>
        <Button large primary disabled={skills.working} onClick={() => skills.dropLink(b.agent, b.name)}>
          {t('Bỏ liên kết', 'Remove link')}
        </Button>
      </DialogButtons>
    </>
  );
}

function NpxBody({ skills }: { skills: Skills }) {
  return (
    <>
      <DialogTitle>{t('Cài Node.js để có npx', 'Install Node.js to get npx')}</DialogTitle>
      <DialogLead>
        {mac(
          t(
            'App dùng npx skills để cài và cập nhật skill từ GitHub, nên cần Node.js trên máy này.',
            'The app uses npx skills to install and update skills from GitHub, so it needs Node.js on this Mac.',
          ),
        )}
      </DialogLead>
      <Facts
        items={[
          [
            '',
            os(
              t('Cài Node.js bản LTS từ nodejs.org, hoặc bằng Homebrew: brew install node.', 'Install the LTS version of Node.js from nodejs.org, or with Homebrew: brew install node.'),
              t('Cài Node.js bản LTS từ nodejs.org, hoặc bằng winget: winget install OpenJS.NodeJS.LTS.', 'Install the LTS version of Node.js from nodejs.org, or with winget: winget install OpenJS.NodeJS.LTS.'),
            ),
          ],
          [
            '',
            os(
              t('App tìm npx ở /opt/homebrew/bin, /usr/local/bin và ~/.nvm.', 'The app looks for npx in /opt/homebrew/bin, /usr/local/bin and ~/.nvm.'),
              t('App tìm npx ở %ProgramFiles%\\nodejs, %APPDATA%\\npm và nvm-windows.', 'The app looks for npx in %ProgramFiles%\\nodejs, %APPDATA%\\npm and nvm-windows.'),
            ),
          ],
          ['ok', t('Bật, tắt và gom skill không cần npx.', 'Turning skills on or off and gathering them does not need npx.')],
        ]}
      />
      <DialogButtons>
        <Button large onClick={skills.close}>
          {t('Đóng', 'Close')}
        </Button>
        <Button
          large
          primary
          onClick={() => {
            skills.close();
            skills.reread();
          }}
        >
          {t('Đọc lại', 'Read again')}
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
    case 'updateMany':
      return <UpdateManyBody d={d} skills={skills} model={model} />;
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

