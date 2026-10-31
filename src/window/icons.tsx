// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The icons the window and the popover share: Phosphor glyphs, their names
// from components/icons.tsx, at the sizes the prototype's drawings had. The three agents show their own logos (components/logos.tsx).

import { Glyph, PlaceGlyph, type Place } from '../components/icons';
import { AgentIcon } from '../components/logos';

/** The popover's theme button in a light look: switch to dark. */
export function MoonIcon() {
  return <Glyph name="moon" className="size-[15px]" />;
}

/** The same in a dark look: switch to light. */
export function SunIcon() {
  return <Glyph name="sun" className="size-[15px]" />;
}

/** Opens the main window: the app's window. */
export function OpenIcon() {
  return <Glyph name="app-window" className="size-[15px]" />;
}

/** A group's fold: a caret, turned down while the group is open. */
export function ChevronIcon() {
  return <Glyph name="caret-right" bold className="size-[11px]" />;
}

/** A sidebar icon: its place's Phosphor glyph (components/icons.tsx), 15 px as the drawings it replaced. */
function nav(place: Place) {
  return <PlaceGlyph place={place} className="size-[15px]" />;
}

/** The sidebar's icons; the three agents show their own logos. */
export const NAV_ICONS = {
  overview: nav('overview'),
  stats: nav('stats'),
  claude: <AgentIcon agent="claude" className="size-[15px]" />,
  agy: <AgentIcon agent="agy" className="size-[15px]" />,
  codex: <AgentIcon agent="codex" className="size-[15px]" />,
  // @private:begin
  rules: nav('rules'),
  // @private:end
  history: nav('history'),
  // @private:begin
  firstmate: nav('firstmate'),
  // @private:end
  mcp: nav('mcp'),
  skills: nav('skills'),
  plugins: nav('plugins'),
  instr: nav('instr'),
  sets: nav('sets'),
  cliTools: nav('cliTools'),
  backup: nav('backup'),
  // @private:begin
  copy: nav('copy'),
  // @private:end
  settings: nav('settings'),
};

/** The small lock beside a server that has a secret, in the line's colour. */
export const LOCK_ICON = <Glyph name="lock" className="size-[11px]" />;

/** An empty table's large glyph: its place's. */
function big(place: Place) {
  return <PlaceGlyph place={place} className="size-[22px]" />;
}

/** The big plug of the empty MCP table. */
export const PLUG_BIG = big('mcp');

// @private:begin
/** The two Macs of the empty Chép sang Mac khác tab. */
export const SYNC_BIG = big('copy');
// @private:end

/** The big box of the empty Sao lưu tab. */
export const ARCHIVE_BIG = big('backup');

/** The big spark of the empty Skills table. */
export const SPARK_BIG = big('skills');

/** The big stack of the empty Bộ cấu hình tab. */
export const STACK_BIG = big('sets');
