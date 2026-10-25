// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The interface's icons: Phosphor's glyphs (@phosphor-icons/react, MIT,
// THIRD_PARTY.md), in their Duotone weight and each tinted its own colour as
// Clean Station tints its SF Symbols; a control's plus, dots, caret or arrow
// takes their Bold, in the control's colour. Only the glyphs named here are
// bundled, so nothing is fetched. Every glyph the app draws, every place's
// and section's glyph and tint, are here, for the window, its dialogs and
// the popover alike. The services' own logos are svgl's (`logos.tsx`), not
// these.

import type { Icon as PhosphorIcon } from '@phosphor-icons/react';
import { AnchorIcon } from '@phosphor-icons/react/dist/csr/Anchor';
import { ArchiveIcon } from '@phosphor-icons/react/dist/csr/Archive';
import { ArrowCircleUpIcon } from '@phosphor-icons/react/dist/csr/ArrowCircleUp';
import { ArrowRightIcon } from '@phosphor-icons/react/dist/csr/ArrowRight';
import { ArrowSquareOutIcon } from '@phosphor-icons/react/dist/csr/ArrowSquareOut';
import { ArrowsClockwiseIcon } from '@phosphor-icons/react/dist/csr/ArrowsClockwise';
import { BellIcon } from '@phosphor-icons/react/dist/csr/Bell';
import { CaretRightIcon } from '@phosphor-icons/react/dist/csr/CaretRight';
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
import { cn } from '@/lib/utils';

/** The Phosphor glyphs the app draws, by Phosphor's own names. */
export const GLYPHS = {
  'anchor': AnchorIcon,
  'archive': ArchiveIcon,
  'arrow-circle-up': ArrowCircleUpIcon,
  'arrow-right': ArrowRightIcon,
  'arrow-square-out': ArrowSquareOutIcon,
  'arrows-clockwise': ArrowsClockwiseIcon,
  'bell': BellIcon,
  'caret-right': CaretRightIcon,
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
} satisfies Record<string, PhosphorIcon>;

export type GlyphName = keyof typeof GLYPHS;

/** The tints a glyph takes (tokens.css `--glyph-*`), each readable on the sidebar and the cards in both looks. */
const TINTS = {
  blue: 'text-glyph-blue',
  green: 'text-glyph-green',
  orange: 'text-glyph-orange',
  indigo: 'text-glyph-indigo',
  red: 'text-glyph-red',
  teal: 'text-glyph-teal',
  purple: 'text-glyph-purple',
  yellow: 'text-glyph-yellow',
  mint: 'text-glyph-mint',
  slate: 'text-glyph-slate',
  pink: 'text-glyph-pink',
  brown: 'text-glyph-brown',
  cyan: 'text-glyph-cyan',
  gray: 'text-glyph-gray',
};

export type GlyphTint = keyof typeof TINTS;

/**
 * A Phosphor glyph in its Duotone weight, 16 px by default: the outline in
 * `tint` (or the text's colour) and its back layer in the same colour at
 * `--glyph-back`. `bold`: a control's glyph (a button's plus, a menu's
 * dots), in Phosphor's Bold with no back layer, as the stroke it replaced.
 * `className` sizes it.
 */
export function Glyph({ name, tint, bold, className }: { name: GlyphName; tint?: GlyphTint; bold?: boolean; className?: string }) {
  const Drawing = GLYPHS[name];
  return (
    <Drawing
      weight={bold ? 'bold' : 'duotone'}
      className={cn('glyph size-4 flex-none', tint && TINTS[tint], className)}
      aria-hidden="true"
      focusable="false"
    />
  );
}

/**
 * Each place of the app (a sidebar item, and the popover that names the same
 * places) with its glyph and its own tint, as Clean Station's sidebar does:
 * neighbours in the sidebar's order take different hues. Claude, Antigravity
 * and Codex show their logos.
 */
export const PLACE_GLYPHS = {
  overview: { glyph: 'squares-four', tint: 'blue' },
  stats: { glyph: 'chart-bar', tint: 'green' },
  rules: { glyph: 'repeat', tint: 'orange' },
  history: { glyph: 'clock-counter-clockwise', tint: 'indigo' },
  alerts: { glyph: 'bell', tint: 'red' },
  firstmate: { glyph: 'anchor', tint: 'teal' },
  mcp: { glyph: 'plug', tint: 'purple' },
  skills: { glyph: 'lightning', tint: 'yellow' },
  plugins: { glyph: 'puzzle-piece', tint: 'mint' },
  instr: { glyph: 'file-text', tint: 'slate' },
  sets: { glyph: 'stack', tint: 'pink' },
  backup: { glyph: 'archive', tint: 'brown' },
  copy: { glyph: 'swap', tint: 'cyan' },
  settings: { glyph: 'gear-six', tint: 'gray' },
} satisfies Record<string, { glyph: GlyphName; tint: GlyphTint }>;

export type Place = keyof typeof PLACE_GLYPHS;

/**
 * The cards of Cài đặt (and the first run's variables card), each with its
 * glyph and tint; the ones that are a place's own take that place's.
 */
export const SECTION_GLYPHS = {
  advanced: { glyph: 'sliders-horizontal', tint: 'orange' },
  general: PLACE_GLYPHS.settings,
  display: { glyph: 'monitor', tint: 'blue' },
  notify: PLACE_GLYPHS.alerts,
  updates: { glyph: 'arrows-clockwise', tint: 'green' },
  services: { glyph: 'database', tint: 'purple' },
  env: { glyph: 'terminal-window', tint: 'teal' },
  backup: PLACE_GLYPHS.backup,
  diag: { glyph: 'wrench', tint: 'slate' },
} satisfies Record<string, { glyph: GlyphName; tint: GlyphTint }>;

export type Section = keyof typeof SECTION_GLYPHS;

/** The lines under a table (InfoLine): what each says, with its glyph and tint. */
export const NOTE_GLYPHS = {
  /** An MCP server put away for every agent. */
  putAway: PLACE_GLYPHS.backup,
  /** What an agent or a plugin brings itself. */
  builtIn: PLACE_GLYPHS.plugins,
  /** The backups kept before each write. */
  backups: PLACE_GLYPHS.history,
  /** A newer version. */
  newer: { glyph: 'arrow-circle-up', tint: 'green' },
  /** Skills installed by hand, to gather into one place. */
  gather: { glyph: 'folders', tint: 'blue' },
  /** A link to a folder that is gone. */
  deadLink: { glyph: 'link-break', tint: 'orange' },
} satisfies Record<string, { glyph: GlyphName; tint: GlyphTint }>;

export type Note = keyof typeof NOTE_GLYPHS;

/** A place's glyph in its tint; `className` sizes it. */
export function PlaceGlyph({ place, className }: { place: Place; className?: string }) {
  const { glyph, tint } = PLACE_GLYPHS[place];
  return <Glyph name={glyph} tint={tint} className={className} />;
}

/** A section's glyph in its tint. */
export function SectionGlyph({ section, className }: { section: Section; className?: string }) {
  const { glyph, tint } = SECTION_GLYPHS[section];
  return <Glyph name={glyph} tint={tint} className={className} />;
}

/** A note's glyph in its tint, 15 px as the line draws it. */
export function NoteGlyph({ note }: { note: Note }) {
  const { glyph, tint } = NOTE_GLYPHS[note];
  return <Glyph name={glyph} tint={tint} className="size-[15px]" />;
}
