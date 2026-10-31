// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The interface's icons: Phosphor's glyphs (@phosphor-icons/react, MIT,
// THIRD_PARTY.md), in their Regular weight and one colour, the text's around
// them; a control's plus, dots, caret or arrow takes their Bold. Only the
// glyphs named here are bundled, so nothing is fetched. Every glyph the app
// draws, and every place's and section's glyph, are here, for the window,
// its dialogs and the popover alike. The services' own logos are svgl's (`logos.tsx`), not
// these.

import type { Icon as PhosphorIcon } from '@phosphor-icons/react';
import { AnchorIcon } from '@phosphor-icons/react/dist/csr/Anchor';
import { AppWindowIcon } from '@phosphor-icons/react/dist/csr/AppWindow';
import { ArchiveIcon } from '@phosphor-icons/react/dist/csr/Archive';
import { ArrowCircleUpIcon } from '@phosphor-icons/react/dist/csr/ArrowCircleUp';
import { ArrowRightIcon } from '@phosphor-icons/react/dist/csr/ArrowRight';
import { ArrowsClockwiseIcon } from '@phosphor-icons/react/dist/csr/ArrowsClockwise';
import { BellIcon } from '@phosphor-icons/react/dist/csr/Bell';
import { CaretDownIcon } from '@phosphor-icons/react/dist/csr/CaretDown';
import { CaretRightIcon } from '@phosphor-icons/react/dist/csr/CaretRight';
import { CaretUpIcon } from '@phosphor-icons/react/dist/csr/CaretUp';
import { ChartBarIcon } from '@phosphor-icons/react/dist/csr/ChartBar';
import { CheckIcon } from '@phosphor-icons/react/dist/csr/Check';
import { CheckCircleIcon } from '@phosphor-icons/react/dist/csr/CheckCircle';
import { ClockCounterClockwiseIcon } from '@phosphor-icons/react/dist/csr/ClockCounterClockwise';
import { CopyIcon } from '@phosphor-icons/react/dist/csr/Copy';
import { DatabaseIcon } from '@phosphor-icons/react/dist/csr/Database';
import { DotsThreeIcon } from '@phosphor-icons/react/dist/csr/DotsThree';
import { FileTextIcon } from '@phosphor-icons/react/dist/csr/FileText';
import { FoldersIcon } from '@phosphor-icons/react/dist/csr/Folders';
import { GearSixIcon } from '@phosphor-icons/react/dist/csr/GearSix';
import { LightningIcon } from '@phosphor-icons/react/dist/csr/Lightning';
import { LinkBreakIcon } from '@phosphor-icons/react/dist/csr/LinkBreak';
import { LockIcon } from '@phosphor-icons/react/dist/csr/Lock';
import { MonitorIcon } from '@phosphor-icons/react/dist/csr/Monitor';
import { MoonIcon } from '@phosphor-icons/react/dist/csr/Moon';
import { PlugIcon } from '@phosphor-icons/react/dist/csr/Plug';
import { PlusIcon } from '@phosphor-icons/react/dist/csr/Plus';
import { PuzzlePieceIcon } from '@phosphor-icons/react/dist/csr/PuzzlePiece';
import { RepeatIcon } from '@phosphor-icons/react/dist/csr/Repeat';
import { SlidersHorizontalIcon } from '@phosphor-icons/react/dist/csr/SlidersHorizontal';
import { SquaresFourIcon } from '@phosphor-icons/react/dist/csr/SquaresFour';
import { StackIcon } from '@phosphor-icons/react/dist/csr/Stack';
import { SunIcon } from '@phosphor-icons/react/dist/csr/Sun';
import { SwapIcon } from '@phosphor-icons/react/dist/csr/Swap';
import { TerminalWindowIcon } from '@phosphor-icons/react/dist/csr/TerminalWindow';
import { WarningCircleIcon } from '@phosphor-icons/react/dist/csr/WarningCircle';
import { WrenchIcon } from '@phosphor-icons/react/dist/csr/Wrench';
import { XIcon } from '@phosphor-icons/react/dist/csr/X';
import { cn } from '@/lib/utils';

/** The Phosphor glyphs the app draws, by Phosphor's own names. */
export const GLYPHS = {
  'anchor': AnchorIcon,
  'app-window': AppWindowIcon,
  'archive': ArchiveIcon,
  'arrow-circle-up': ArrowCircleUpIcon,
  'arrow-right': ArrowRightIcon,
  'arrows-clockwise': ArrowsClockwiseIcon,
  'bell': BellIcon,
  'caret-down': CaretDownIcon,
  'caret-right': CaretRightIcon,
  'caret-up': CaretUpIcon,
  'chart-bar': ChartBarIcon,
  'check': CheckIcon,
  'check-circle': CheckCircleIcon,
  'clock-counter-clockwise': ClockCounterClockwiseIcon,
  'copy': CopyIcon,
  'database': DatabaseIcon,
  'dots-three': DotsThreeIcon,
  'file-text': FileTextIcon,
  'folders': FoldersIcon,
  'gear-six': GearSixIcon,
  'lightning': LightningIcon,
  'link-break': LinkBreakIcon,
  'lock': LockIcon,
  'monitor': MonitorIcon,
  'moon': MoonIcon,
  'plug': PlugIcon,
  'plus': PlusIcon,
  'puzzle-piece': PuzzlePieceIcon,
  'repeat': RepeatIcon,
  'sliders-horizontal': SlidersHorizontalIcon,
  'squares-four': SquaresFourIcon,
  'stack': StackIcon,
  'sun': SunIcon,
  'swap': SwapIcon,
  'terminal-window': TerminalWindowIcon,
  'warning-circle': WarningCircleIcon,
  'wrench': WrenchIcon,
  'x': XIcon,
} satisfies Record<string, PhosphorIcon>;

export type GlyphName = keyof typeof GLYPHS;

/**
 * A Phosphor glyph in its Regular weight and the text's colour, 16 px by
 * default. `bold`: a control's glyph (a button's plus, a menu's dots), in
 * Phosphor's Bold, as the stroke it replaced. `className` sizes it.
 */
export function Glyph({ name, bold, className }: { name: GlyphName; bold?: boolean; className?: string }) {
  const Drawing = GLYPHS[name];
  return (
    <Drawing
      weight={bold ? 'bold' : 'regular'}
      className={cn('glyph size-4 flex-none', className)}
      aria-hidden="true"
      focusable="false"
    />
  );
}

/**
 * Each place of the app (a sidebar item, Thông báo's bell, and the popover
 * that names the same places) with its glyph. Claude, Antigravity and Codex
 * show their logos.
 */
export const PLACE_GLYPHS = {
  overview: 'squares-four',
  stats: 'chart-bar',
  // @private:begin
  rules: 'repeat',
  // @private:end
  history: 'clock-counter-clockwise',
  alerts: 'bell',
  // @private:begin
  firstmate: 'anchor',
  // @private:end
  mcp: 'plug',
  skills: 'lightning',
  plugins: 'puzzle-piece',
  instr: 'file-text',
  sets: 'stack',
  cliTools: 'arrow-circle-up',
  backup: 'archive',
  // @private:begin
  copy: 'swap',
  // @private:end
  settings: 'gear-six',
} satisfies Record<string, GlyphName>;

export type Place = keyof typeof PLACE_GLYPHS;

/**
 * The cards of Cài đặt (and the first run's variables card), each with its
 * glyph; the ones that are a place's own take that place's.
 */
export const SECTION_GLYPHS = {
  // @private:begin
  advanced: 'sliders-horizontal',
  // @private:end
  general: PLACE_GLYPHS.settings,
  display: 'monitor',
  tray: 'app-window',
  notify: PLACE_GLYPHS.alerts,
  updates: 'arrows-clockwise',
  services: 'database',
  env: 'terminal-window',
  cli: 'terminal-window',
  backup: PLACE_GLYPHS.backup,
  diag: 'wrench',
} satisfies Record<string, GlyphName>;

export type Section = keyof typeof SECTION_GLYPHS;

/** The lines under a table (InfoLine): what each says, with its glyph. */
export const NOTE_GLYPHS = {
  /** An MCP server put away for every agent. */
  putAway: PLACE_GLYPHS.backup,
  /** What an agent or a plugin brings itself. */
  builtIn: PLACE_GLYPHS.plugins,
  /** The backups kept before each write. */
  backups: PLACE_GLYPHS.history,
  /** A newer version. */
  newer: 'arrow-circle-up',
  /** Skills installed by hand, to gather into one place. */
  gather: 'folders',
  /** A link to a folder that is gone. */
  deadLink: 'link-break',
} satisfies Record<string, GlyphName>;

export type Note = keyof typeof NOTE_GLYPHS;

/** A place's glyph; `className` sizes it. */
export function PlaceGlyph({ place, className }: { place: Place; className?: string }) {
  return <Glyph name={PLACE_GLYPHS[place]} className={className} />;
}

/** A section's glyph. */
export function SectionGlyph({ section, className }: { section: Section; className?: string }) {
  return <Glyph name={SECTION_GLYPHS[section]} className={className} />;
}

/** A note's glyph, 15 px as the line draws it. */
export function NoteGlyph({ note }: { note: Note }) {
  return <Glyph name={NOTE_GLYPHS[note]} className="size-[15px]" />;
}
