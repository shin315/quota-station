// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The agents' and the services' logos. They come from svgl
// (https://svgl.app, see THIRD_PARTY.md) through its shadcn registry into
// `ui/svgs/`, so nothing is fetched while the app runs. Each logo is its
// owner's trademark and only names that service here.

import type { ComponentType, SVGProps } from 'react';
import { cn } from '@/lib/utils';
import type { Service } from '../lib/brands';
import type { Agent } from '../lib/model';
import { Antigravity } from './ui/svgs/antigravity';
import { AsanaLogo } from './ui/svgs/asanaLogo';
import { Atlassian } from './ui/svgs/atlassian';
import { Canva } from './ui/svgs/canva';
import { ClaudeAiIcon } from './ui/svgs/claudeAiIcon';
import { Cloudflare } from './ui/svgs/cloudflare';
import { CodexDark } from './ui/svgs/codexDark';
import { CodexLight } from './ui/svgs/codexLight';
import { Drive } from './ui/svgs/drive';
import { Figma } from './ui/svgs/figma';
import { GithubDark } from './ui/svgs/githubDark';
import { GithubLight } from './ui/svgs/githubLight';
import { Gmail } from './ui/svgs/gmail';
import { GoogleCalendar } from './ui/svgs/googleCalendar';
import { Linear } from './ui/svgs/linear';
import { Notion } from './ui/svgs/notion';
import { Playwright } from './ui/svgs/playwright';
import { Postgresql } from './ui/svgs/postgresql';
import { Slack } from './ui/svgs/slack';
import { Stripe } from './ui/svgs/stripe';
import { Supabase } from './ui/svgs/supabase';
import { Vercel } from './ui/svgs/vercel';
import { VercelDark } from './ui/svgs/vercelDark';

type Svg = ComponentType<SVGProps<SVGSVGElement>>;

/** One drawing for both looks, or one for the light look and one for the dark (logos.css shows the one in use). */
export type Logo = Svg | { light: Svg; dark: Svg };

export const AGENT_LOGO: Record<Agent, Logo> = {
  claude: ClaudeAiIcon,
  codex: { light: CodexLight, dark: CodexDark },
  agy: Antigravity,
};

export const SERVICE_LOGO: Record<Service, Logo> = {
  github: { light: GithubLight, dark: GithubDark },
  notion: Notion,
  figma: Figma,
  slack: Slack,
  linear: Linear,
  supabase: Supabase,
  vercel: { light: Vercel, dark: VercelDark },
  cloudflare: Cloudflare,
  gmail: Gmail,
  'google-drive': Drive,
  'google-calendar': GoogleCalendar,
  stripe: Stripe,
  atlassian: Atlassian,
  asana: AsanaLogo,
  canva: Canva,
  playwright: Playwright,
  postgresql: Postgresql,
};

/**
 * A logo in a square box `className` sizes (16 px by default), centred as the
 * glyphs it replaces were. Hidden from screen readers: the name beside it says it.
 */
export function LogoMark({ logo, className }: { logo: Logo; className?: string }) {
  let drawing;
  if ('light' in logo) {
    const { light: Light, dark: Dark } = logo;
    drawing = (
      <>
        <Light className="logo-light" />
        <Dark className="logo-dark" />
      </>
    );
  } else {
    const One = logo;
    drawing = <One />;
  }
  return (
    <span className={cn('logo inline-flex size-4 shrink-0 items-center justify-center [&_svg]:size-full', className)} aria-hidden="true">
      {drawing}
    </span>
  );
}

/** An agent's logo: Claude, Codex or Antigravity. */
export function AgentIcon({ agent, className }: { agent: Agent; className?: string }) {
  return <LogoMark logo={AGENT_LOGO[agent]} className={className} />;
}

/** A service's logo before a row's name, or nothing when the row names no service the app knows. */
export function ServiceIcon({ service, className }: { service: Service | null; className?: string }) {
  return service ? <LogoMark logo={SERVICE_LOGO[service]} className={cn('size-3.5', className)} /> : null;
}
