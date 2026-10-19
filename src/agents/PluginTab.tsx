// The Plugin tab of the main window (M11, E): the plugins Claude Code and
// Codex list themselves, what each brings, and their marketplaces. Read
// only: the app never installs, removes, turns on or off a plugin. "Lệnh cài
// lại" shows the agents' own commands a restore would run.

import { useState } from 'react';
import { Banner, Button, Chip, Dot } from '../components/parts';
import { SHEET_TITLE_ID } from '../components/Sheets';
import { clock } from '../lib/format';
import { AGENT_NAME, AGENTS } from '../lib/mcp';
import type { Agent, Marketplace, Model, PluginSide } from '../lib/model';
import { brings } from '../lib/skills';
import type { Action, BannerView, PopoverView } from '../lib/view';
import { AgentLogo } from './CheckMatrix';
import { Facts } from './McpDialogs';

const CLI: Record<Agent, string> = { claude: 'claude', codex: 'codex', agy: 'agy' };

function sideBanner(s: PluginSide): BannerView | null {
  const name = AGENT_NAME[s.agent];
  if (s.state === 'noCli')
    return {
      tone: 'warn',
      title: 'Không đọc được plugin của ' + name,
      body: 'Không tìm thấy lệnh ' + CLI[s.agent] + '. Gói sao lưu giữ danh sách cũ của ' + name + ' đến khi đọc lại được.',
      actions: [{ label: 'Đọc lại', act: 'plugins:reread' }],
    };
  if (s.state === 'failed')
    return {
      tone: 'warn',
      title: 'Không đọc được plugin của ' + name,
      body: (s.detail ? CLI[s.agent] + ' plugin list báo: ' + s.detail + '. ' : '') + 'Gói sao lưu giữ danh sách cũ đến khi đọc lại được.',
      actions: [{ label: 'Đọc lại', act: 'plugins:reread' }],
    };
  return null;
}

function Rows({ side }: { side: PluginSide }) {
  if (side.state !== 'ok') return <p className="note">Chưa đọc được.</p>;
  if (!side.plugins.length) return <p className="note">Chưa cài plugin nào.</p>;
  return (
    <div role="list" aria-label={'Plugin của ' + AGENT_NAME[side.agent]}>
      <div className="pr h" aria-hidden="true">
        <span>Plugin</span>
        <span>Mang theo</span>
        <span>Trạng thái</span>
      </div>
      {side.plugins.map((p) => (
        <div key={p.id} className="pr" role="listitem" aria-label={p.id + (p.enabled ? ', bật' : ', tắt')}>
          <span className="id" title={p.version ? 'bản ' + p.version : undefined}>
            {p.id}
          </span>
          <span title={p.brings.skills.length ? 'skill: ' + p.brings.skills.join(', ') : undefined}>{brings(p)}</span>
          <span>
            <Chip chip={p.enabled ? { t: 'bật', k: 'ok' } : { t: 'tắt', k: '' }} />
          </span>
        </div>
      ))}
    </div>
  );
}

function marketSource(m: Marketplace): string {
  if (m.kind === 'builtin') return 'có sẵn';
  if (m.kind === 'local') return 'thư mục trên máy';
  return m.source ?? m.kind;
}

export function usePlugins() {
  const [open, setOpen] = useState(false);
  return { open, show: () => setOpen(true), close: () => setOpen(false) };
}

export type Plugins = ReturnType<typeof usePlugins>;

export interface PluginTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  plugins: Plugins;
  onAction(act: Action): void;
}

export function PluginTab({ model, fresh, plugins, onAction }: PluginTabProps) {
  const v = model.plugins;
  const sides = v.sides;
  const sub = v.busy ? 'đang đọc…' : v.readAt ? 'đọc lúc ' + clock(v.readAt) : fresh.text;
  const markets = (sides ?? []).flatMap((s) => s.marketplaces.map((m) => ({ ...m, agent: s.agent })));
  return (
    <>
      <div className="tb" data-tauri-drag-region="deep">
        <h2>Plugin</h2>
        <small>
          <Dot tone={v.busy ? 'stale' : 'ok'} />
          {model.machine} · {sub}
        </small>
        <Button disabled={!sides} onClick={plugins.show}>
          Lệnh cài lại
        </Button>
        <Button disabled={v.busy} onClick={() => onAction('plugins:reread')}>
          Đọc lại
        </Button>
      </div>
      {(sides ?? []).map((s) => {
        const b = sideBanner(s);
        return b ? <Banner key={s.agent} banner={b} onAction={onAction} /> : null;
      })}
      <p className="note" style={{ margin: 0 }}>
        Chỉ xem. App không cài, gỡ, bật hay tắt plugin ở đây; khi khôi phục, app chạy lệnh cài của chính agent.
      </p>
      {!sides ? (
        <div className="card">
          <div className="empty">
            <b>Đang đọc danh sách plugin…</b>
            Claude Code và Codex, trên máy này.
          </div>
        </div>
      ) : (
        <>
          <div className="grid2">
            {sides.map((s) => (
              <div key={s.agent} className="card">
                <h4>
                  <AgentLogo agent={s.agent} /> {AGENT_NAME[s.agent]} <span className="cnt">{s.state === 'ok' ? s.plugins.length : '?'}</span>
                </h4>
                <Rows side={s} />
              </div>
            ))}
          </div>
          <div className="card">
            <h4>
              Marketplace <span className="cnt">{markets.length}</span>
            </h4>
            {markets.length ? (
              <div className="kvt">
                {markets.flatMap((m) => [
                  <span key={m.agent + m.name + ':k'}>
                    <span className="mono">{m.name}</span> · {marketSource(m)}
                  </span>,
                  <span key={m.agent + m.name + ':v'}>{AGENT_NAME[m.agent]}</span>,
                ])}
              </div>
            ) : (
              <p className="note">Chưa thêm marketplace nào.</p>
            )}
          </div>
          <div className="card">
            <h4>Trong gói sao lưu</h4>
            <div className="kvt">
              <span>Ghi</span>
              <span>tên@marketplace, nguồn marketplace, bật hay tắt</span>
              <span>Không ghi</span>
              <span>nội dung plugin, MCP và skill plugin mang theo</span>
            </div>
          </div>
        </>
      )}
    </>
  );
}

export function PluginDialogBody({ model, plugins }: { model: Model; plugins: Plugins }) {
  const r = model.plugins.reinstall;
  const agents = AGENTS.filter((a) => r[a]?.length);
  return (
    <>
      <h3 id={SHEET_TITLE_ID}>Lệnh app chạy khi khôi phục</h3>
      <p className="lead">Đúng lệnh của từng agent, chạy lần lượt, có xem trước. Plugin có sẵn của agent không cần cài lại.</p>
      {agents.length ? (
        agents.map((a) => (
          <section key={a} aria-label={AGENT_NAME[a]}>
            <div className="sec-t">
              <AgentLogo agent={a} />
              {AGENT_NAME[a]}
            </div>
            {r[a]!.map((c) => (
              <div key={c} className="cmd">
                <span className="p" aria-hidden="true">
                  $
                </span>
                <code>{c}</code>
              </div>
            ))}
          </section>
        ))
      ) : (
        <p className="note">Không có plugin nào cần cài lại.</p>
      )}
      <Facts items={[['', 'Chỉ để xem: khôi phục plugin làm ở phần khôi phục gói sao lưu.']]} />
      <div className="btns">
        <Button large primary onClick={plugins.close}>
          Đóng
        </Button>
      </div>
    </>
  );
}
