// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Plugin tab of the main window (M11, E): the plugins Claude Code and
// Codex list themselves (a plugin for a known service with its logo), what
// each brings, and their marketplaces. Read
// only: the app never installs, removes, turns on or off a plugin. "Lệnh cài
// lại" shows the agents' own commands a restore would run.

import { useState } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { Command } from '../components/blocks';
import { ServiceIcon } from '../components/logos';
import { COUNT, Empty, GRID2, TabHead, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, CODE, Dot, KVT } from '../components/parts';
import { pluginName, serviceOf } from '../lib/brands';
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

/** A plugin: its id, what it brings, on or off. */
const PLUGIN_ROW = 'grid grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_70px] items-center gap-2.5 border-t-[.5px] border-card-line py-[7px] text-sm';

function Rows({ side }: { side: PluginSide }) {
  if (side.state !== 'ok') return <CardText className="mt-2 mb-0">Chưa đọc được.</CardText>;
  if (!side.plugins.length) return <CardText className="mt-2 mb-0">Chưa cài plugin nào.</CardText>;
  return (
    <div role="list" aria-label={'Plugin của ' + AGENT_NAME[side.agent]}>
      <div className={cn(PLUGIN_ROW, 'border-t-0 pt-0 text-xs text-ink3')} aria-hidden="true">
        <span>Plugin</span>
        <span>Mang theo</span>
        <span>Trạng thái</span>
      </div>
      {side.plugins.map((p) => (
        <div key={p.id} className={PLUGIN_ROW} role="listitem" aria-label={p.id + (p.enabled ? ', bật' : ', tắt')}>
          <span className="flex min-w-0 items-center gap-1.5">
            <ServiceIcon service={serviceOf(pluginName(p.id))} />
            <span className="min-w-0 [font:11.5px_var(--mono)] [overflow-wrap:anywhere]" title={p.version ? 'bản ' + p.version : undefined}>
              {p.id}
            </span>
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
      <TabHead>
        <TabTitle>Plugin</TabTitle>
        <TabSub>
          <Dot tone={v.busy ? 'stale' : 'ok'} />
          {model.machine} · {sub}
        </TabSub>
        <Button disabled={!sides} onClick={plugins.show}>
          Lệnh cài lại
        </Button>
        <Button disabled={v.busy} onClick={() => onAction('plugins:reread')}>
          Đọc lại
        </Button>
      </TabHead>
      {(sides ?? []).map((s) => {
        const b = sideBanner(s);
        return b ? <Banner key={s.agent} banner={b} onAction={onAction} /> : null;
      })}
      <p className="m-0">
        Chỉ xem. App không cài, gỡ, bật hay tắt plugin ở đây; khi khôi phục, app chạy lệnh cài của chính agent.
      </p>
      {!sides ? (
        <Panel>
          <Empty>
            <b>Đang đọc danh sách plugin…</b>
            Claude Code và Codex, trên máy này.
          </Empty>
        </Panel>
      ) : (
        <>
          <div className={GRID2}>
            {sides.map((s) => (
              <Panel key={s.agent}>
                <CardTitle>
                  <AgentLogo agent={s.agent} /> {AGENT_NAME[s.agent]} <span className={COUNT}>{s.state === 'ok' ? s.plugins.length : '?'}</span>
                </CardTitle>
                <Rows side={s} />
              </Panel>
            ))}
          </div>
          <Panel>
            <CardTitle>
              Marketplace <span className={COUNT}>{markets.length}</span>
            </CardTitle>
            {markets.length ? (
              <div className={KVT}>
                {markets.flatMap((m) => [
                  <span key={m.agent + m.name + ':k'}>
                    <span className={CODE}>{m.name}</span> · {marketSource(m)}
                  </span>,
                  <span key={m.agent + m.name + ':v'}>{AGENT_NAME[m.agent]}</span>,
                ])}
              </div>
            ) : (
              <CardText className="mt-2 mb-0">Chưa thêm marketplace nào.</CardText>
            )}
          </Panel>
          <Panel>
            <CardTitle>Trong gói sao lưu</CardTitle>
            <div className={KVT}>
              <span>Ghi</span>
              <span>tên@marketplace, nguồn marketplace, bật hay tắt</span>
              <span>Không ghi</span>
              <span>nội dung plugin, MCP và skill plugin mang theo</span>
            </div>
          </Panel>
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
      <DialogTitle>Lệnh app chạy khi khôi phục</DialogTitle>
      <DialogLead>Đúng lệnh của từng agent, chạy lần lượt, có xem trước. Plugin có sẵn của agent không cần cài lại.</DialogLead>
      {agents.length ? (
        agents.map((a) => (
          <section key={a} aria-label={AGENT_NAME[a]}>
            <DialogSection>
              <AgentLogo agent={a} />
              {AGENT_NAME[a]}
            </DialogSection>
            {r[a]!.map((c) => (
              <Command key={c}>{c}</Command>
            ))}
          </section>
        ))
      ) : (
        <p>Không có plugin nào cần cài lại.</p>
      )}
      <Facts items={[['', 'Chỉ để xem: khôi phục plugin làm ở phần khôi phục gói sao lưu.']]} />
      <DialogButtons>
        <Button large primary onClick={plugins.close}>
          Đóng
        </Button>
      </DialogButtons>
    </>
  );
}
