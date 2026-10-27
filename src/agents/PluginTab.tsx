// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Plugin tab of the main window (M11, E): the plugins Claude Code and
// Codex list themselves (a plugin for a known service with its logo), what
// each brings, and their marketplaces. Read
// only: the app never installs, removes, turns on or off a plugin. "Lệnh cài
// lại" shows the agents' own commands a restore would run.
// As the approved redraw (plugins-R2, 25/10/2026) lays it out: one line
// under the title with the rest behind "Chi tiết", one table of every
// agent's plugins under one head row, then Marketplace and what the pack
// keeps side by side, ending on one line.

import { useState } from 'react';
import { Card as Panel, CardText, CardTitle } from '@/components/ui/card';
import { DialogButtons, DialogLead, DialogSection, DialogTitle } from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { Command } from '../components/blocks';
import { ServiceIcon } from '../components/logos';
import { COUNT, Empty, GRID2_EVEN, GROUP_HEAD, TabHead, TabNote, TabSub, TabTitle } from '../components/page';
import { Banner, Button, Chip, CODE, Dot, More } from '../components/parts';
import { pluginName, serviceOf } from '../lib/brands';
import { clock } from '../lib/format';
import { mac, t } from '../lib/i18n';
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
      title: t('Không đọc được plugin của ' + name, 'Could not read ' + name + '’s plugins'),
      body: t(
        'Không tìm thấy lệnh ' + CLI[s.agent] + '. Gói sao lưu giữ danh sách cũ của ' + name + ' đến khi đọc lại được.',
        'The ' + CLI[s.agent] + ' command was not found. The backup pack keeps ' + name + '’s old list until it can be read again.',
      ),
      actions: [{ label: t('Đọc lại', 'Read again'), act: 'plugins:reread' }],
    };
  if (s.state === 'failed')
    return {
      tone: 'warn',
      title: t('Không đọc được plugin của ' + name, 'Could not read ' + name + '’s plugins'),
      body:
        (s.detail ? t(CLI[s.agent] + ' plugin list báo: ' + s.detail + '. ', CLI[s.agent] + ' plugin list said: ' + s.detail + '. ') : '') +
        t('Gói sao lưu giữ danh sách cũ đến khi đọc lại được.', 'The backup pack keeps the old list until it can be read again.'),
      actions: [{ label: t('Đọc lại', 'Read again'), act: 'plugins:reread' }],
    };
  return null;
}

/** A plugin: its id, what it brings, on or off at the row's end; the first under its band has no hairline. */
const PLUGIN_ROW =
  'grid grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_70px] items-center gap-2.5 border-t-[.5px] border-card-line px-1.5 py-[7px] text-sm first:border-t-0 [&>:last-child]:justify-self-end';

/** One agent's plugins under its group head: its logo, its name and how many. */
function Group({ side }: { side: PluginSide }) {
  const name = AGENT_NAME[side.agent];
  return (
    <section aria-label={name} className="mt-1.5 first-of-type:mt-0">
      {/* A band across the table, so each agent's plugins read as one block. */}
      <h5 className={cn(GROUP_HEAD, 'm-0 rounded-lg border-t-0 bg-fill px-1.5 py-1 first:border-t-0 first:pt-1')}>
        <AgentLogo agent={side.agent} />
        {name}
        <span className={COUNT}>· {side.state === 'ok' ? side.plugins.length : '?'}</span>
      </h5>
      <Rows side={side} />
    </section>
  );
}

function Rows({ side }: { side: PluginSide }) {
  if (side.state !== 'ok') return <CardText className="mb-1 px-1.5">{t('Chưa đọc được.', 'Not read yet.')}</CardText>;
  if (!side.plugins.length) return <CardText className="mb-1 px-1.5">{t('Chưa cài plugin nào.', 'No plugins installed.')}</CardText>;
  return (
    <div role="list" aria-label={t('Plugin của ' + AGENT_NAME[side.agent], AGENT_NAME[side.agent] + ' plugins')}>
      {side.plugins.map((p) => (
        <div key={p.id} className={PLUGIN_ROW} role="listitem" aria-label={p.id + (p.enabled ? t(', bật', ', on') : t(', tắt', ', off'))}>
          <span className="flex min-w-0 items-center gap-1.5">
            <ServiceIcon service={serviceOf(pluginName(p.id))} />
            <span className="min-w-0 [font:11.5px_var(--mono)] [overflow-wrap:anywhere]" title={p.version ? t('bản ' + p.version, 'version ' + p.version) : undefined}>
              {p.id}
            </span>
          </span>
          <span title={p.brings.skills.length ? t('skill: ', 'skills: ') + p.brings.skills.join(', ') : undefined}>{brings(p)}</span>
          <span>
            <Chip chip={p.enabled ? { t: t('bật', 'on'), k: 'ok' } : { t: t('tắt', 'off'), k: '' }} />
          </span>
        </div>
      ))}
    </div>
  );
}

/** A row of the two cards under the table: its words on the left, its value at the right. */
const SIDE_ROW = 'grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3 border-t-[.5px] border-card-line py-[7px] text-sm first:border-t-0';

function marketSource(m: Marketplace): string {
  if (m.kind === 'builtin') return t('có sẵn', 'built in');
  if (m.kind === 'local') return mac(t('thư mục trên máy', 'folder on this Mac'));
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
  const sub = v.busy ? t('đang đọc…', 'reading…') : v.readAt ? t('đọc lúc ' + clock(v.readAt), 'read at ' + clock(v.readAt)) : fresh.text;
  const markets = (sides ?? []).flatMap((s) => s.marketplaces.map((m) => ({ ...m, agent: s.agent })));
  return (
    <>
      <TabHead>
        <TabTitle>{t('Plugin', 'Plugins')}</TabTitle>
        <TabSub>
          <Dot tone={v.busy ? 'stale' : 'ok'} />
          {model.machine} · {sub}
        </TabSub>
        <Button disabled={!sides} onClick={plugins.show}>
          {t('Lệnh cài lại', 'Reinstall commands')}
        </Button>
        <Button disabled={v.busy} onClick={() => onAction('plugins:reread')}>
          {t('Đọc lại', 'Read again')}
        </Button>
      </TabHead>
      <TabNote>
        {t('Chỉ xem plugin · Khôi phục qua lệnh của agent. ', 'Plugins are view only · restored through the agent’s own command. ')}
        <More about={t('Plugin', 'Plugins')}>
          <p>
            {t(
              'App không cài, gỡ, bật hay tắt plugin ở đây: agent tự quản plugin của nó.',
              'The app does not install, remove, enable or disable plugins here: each agent manages its own.',
            )}
          </p>
          <p>
            {t(
              'Gói sao lưu chỉ ghi tên@marketplace, nguồn marketplace và bật hay tắt; không chứa nội dung plugin, cũng không chứa MCP hay skill plugin mang theo.',
              'A backup pack only records name@marketplace, the marketplace’s source and on or off; it holds neither the plugin’s contents nor the MCP and skills it brings.',
            )}
          </p>
          <p>
            {t(
              'Khi khôi phục, app chạy lệnh cài của chính agent sau khi bạn xem trước (nút Lệnh cài lại), nên agent phải cài lại được plugin từ marketplace. Plugin có sẵn của agent không cần cài lại.',
              'On restore, the app runs the agent’s own install command after you preview it (the Reinstall commands button), so the agent has to be able to install the plugin from its marketplace again. An agent’s built-in plugins need no reinstall.',
            )}
          </p>
        </More>
      </TabNote>
      {(sides ?? []).map((s) => {
        const b = sideBanner(s);
        return b ? <Banner key={s.agent} banner={b} onAction={onAction} /> : null;
      })}
      {!sides ? (
        <Panel>
          <Empty>
            <b>{t('Đang đọc danh sách plugin…', 'Reading the plugin list…')}</b>
            {mac(t('Claude Code và Codex, trên máy này.', 'Claude Code and Codex, on this Mac.'))}
          </Empty>
        </Panel>
      ) : (
        <>
          <Panel>
            <CardTitle>{t('Plugin đã cài', 'Installed plugins')}</CardTitle>
            <div className={cn(PLUGIN_ROW, 'border-t-0 pt-0 text-xs text-ink3')} aria-hidden="true">
              <span>Plugin</span>
              <span>{t('Mang theo', 'Brings')}</span>
              <span>{t('Trạng thái', 'State')}</span>
            </div>
            {sides.map((s) => (
              <Group key={s.agent} side={s} />
            ))}
          </Panel>
          <div className={GRID2_EVEN}>
            <Panel>
              <CardTitle>
                Marketplace <span className={COUNT}>{markets.length}</span>
              </CardTitle>
              {markets.length ? (
                <div role="list" aria-label="Marketplace">
                  {markets.map((m) => (
                    <div key={m.agent + m.name} className={SIDE_ROW} role="listitem">
                      <span className="min-w-0 text-ink2 [overflow-wrap:anywhere]">
                        <span className={CODE}>{m.name}</span> · {marketSource(m)}
                      </span>
                      <span>{AGENT_NAME[m.agent]}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <CardText className="mb-0">{t('Chưa thêm marketplace nào.', 'No marketplaces added.')}</CardText>
              )}
            </Panel>
            <Panel>
              <CardTitle>{t('Trong gói sao lưu', 'In the backup pack')}</CardTitle>
              <div className={SIDE_ROW}>
                <span className="text-ink2">{t('Ghi', 'Kept')}</span>
                <span className="text-right">{t('tên@marketplace, nguồn marketplace, bật hay tắt', 'name@marketplace, marketplace source, on or off')}</span>
              </div>
              <div className={SIDE_ROW}>
                <span className="text-ink2">{t('Không ghi', 'Not kept')}</span>
                <span className="text-right">{t('nội dung plugin, MCP và skill plugin mang theo', 'the plugin’s contents, and the MCP and skills it brings')}</span>
              </div>
            </Panel>
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
      <DialogTitle>{t('Lệnh app chạy khi khôi phục', 'Commands the app runs on restore')}</DialogTitle>
      <DialogLead>
        {t(
          'Đúng lệnh của từng agent, chạy lần lượt, có xem trước. Plugin có sẵn của agent không cần cài lại.',
          'Each agent’s own commands, run one by one, with a preview. An agent’s built-in plugins need no reinstall.',
        )}
      </DialogLead>
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
        <p>{t('Không có plugin nào cần cài lại.', 'No plugins need reinstalling.')}</p>
      )}
      <Facts items={[['', t('Chỉ để xem: khôi phục plugin làm ở phần khôi phục gói sao lưu.', 'For viewing only: plugins are restored when you restore a backup pack.')]]} />
      <DialogButtons>
        <Button large primary onClick={plugins.close}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </>
  );
}
