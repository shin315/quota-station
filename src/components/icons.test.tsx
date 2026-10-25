// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Every icon the app draws is a Phosphor glyph from components/icons.tsx or a
// service's logo: no drawing of its own and no other icon set, in the source
// and on every page and the popover as they render.

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import tokens from '../styles/tokens.css?raw';
import { MainWindowApp, PopoverApp } from '../App';
import { fakeApi } from '../fixtures/fakeApi';
import { FIXTURE_NOW, fixtureModel } from '../fixtures/models';
import type { Api } from '../lib/api';
import type { Tab } from '../lib/model';
import { Glyph, GLYPHS, NOTE_GLYPHS, PLACE_GLYPHS, SECTION_GLYPHS, type GlyphName } from './icons';

/** The app's source, test files aside. */
const SOURCES = Object.entries(import.meta.glob<string>(['../**/*.tsx', '!../**/*.test.tsx'], { query: '?raw', import: 'default', eager: true }));

/** The glyphs' module, as the glob names it from here. */
const ICONS = /^\.\/icons\.tsx$/;

/** Where an <svg> of the app's own may be written: the services' logos, the glyphs' module, and Thống kê's chart. */
const OWN_SVG = [/^\.\/ui\/svgs\//, ICONS, /\/stats\/StatsTab\.tsx$/];

const TABS: Tab[] = ['overview', 'stats', 'claude', 'agy', 'codex', 'rules', 'history', 'alerts', 'firstmate', 'mcp', 'skills', 'plugins', 'instr', 'sets', 'backup', 'copy', 'settings'];

/** What an <svg> on screen is: a Phosphor glyph, a logo, or the chart; anything else is named by its markup. */
function kind(svg: SVGSVGElement): string {
  if (svg.classList.contains('glyph') && svg.getAttribute('viewBox') === '0 0 256 256') return 'glyph';
  if (svg.closest('.logo')) return 'logo';
  if (svg.getAttribute('role') === 'group' && svg.closest('section, [role=region], div')) return 'chart';
  return svg.outerHTML.slice(0, 160);
}

afterEach(cleanup);

describe('the icons', () => {
  it('are drawn nowhere but the glyphs, the logos and the chart', () => {
    expect(SOURCES.length).toBeGreaterThan(50);
    const drawn = SOURCES.filter(([path, code]) => /<svg[\s>]/.test(code) && !OWN_SVG.some((p) => p.test(path))).map(([path]) => path);
    expect(drawn).toEqual([]);
  });

  it('come from Phosphor and no other icon set', () => {
    const imports = SOURCES.flatMap(([path, code]) => [...code.matchAll(/from '([^']+)'/g)].map((m) => [path, m[1]]));
    const sets = imports.filter(([, from]) => /lucide|heroicons|tabler|react-icons|feather|remixicon|hugeicons|@radix-ui\/react-icons/.test(from));
    expect(sets).toEqual([]);
    const phosphor = imports.filter(([, from]) => from.startsWith('@phosphor-icons/'));
    expect(phosphor.length).toBeGreaterThan(0);
    expect(phosphor.filter(([path]) => !ICONS.test(path))).toEqual([]);
  });

  it('name each Phosphor glyph by its own name, and draw it in Duotone with its back layer', () => {
    for (const [name, Drawing] of Object.entries(GLYPHS)) {
      const pascal = name.replace(/(^|-)([a-z0-9])/g, (_, __, c: string) => c.toUpperCase());
      expect(Drawing.displayName).toBe(pascal + 'Icon');
      const { container } = render(<Glyph name={name as GlyphName} tint="blue" />);
      const svg = container.querySelector('svg')!;
      expect(svg.getAttribute('viewBox')).toBe('0 0 256 256');
      expect(svg.classList.contains('text-glyph-blue')).toBe(true);
      expect(svg.getAttribute('aria-hidden')).toBe('true');
      expect(svg.querySelector('[opacity]')).not.toBeNull();
      cleanup();
    }
  });

  it('take a tint the tokens define for the light look, the dark look and macOS’s dark', () => {
    const tints = new Set([...Object.values(PLACE_GLYPHS), ...Object.values(SECTION_GLYPHS), ...Object.values(NOTE_GLYPHS)].map((r) => r.tint));
    for (const tint of tints) expect(tokens.match(new RegExp('--glyph-' + tint + ': #[0-9a-f]{6};', 'g'))?.length, tint).toBe(3);
  });

  it('give each sidebar item its own glyph and tint', () => {
    const places = Object.values(PLACE_GLYPHS);
    expect(new Set(places.map((p) => p.glyph)).size).toBe(places.length);
    expect(new Set(places.map((p) => p.tint)).size).toBe(places.length);
  });

  it('on every page and in the popover, are glyphs, logos or the chart', async () => {
    const seen = new Set<string>();
    for (const tab of TABS) {
      const fake = fakeApi(fixtureModel('fresh'), tab);
      for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
      render(
        <div className="qs app-window">
          <MainWindowApp api={fake.api} clock={() => FIXTURE_NOW} />
        </div>,
      );
      await screen.findByRole('navigation', { name: 'Các tab' });
      const svgs = [...document.querySelectorAll('svg')];
      expect(svgs.length, tab).toBeGreaterThan(0);
      for (const svg of svgs) {
        const k = kind(svg);
        expect(['glyph', 'logo', 'chart'], tab + ': ' + k).toContain(k);
        seen.add(k);
      }
      cleanup();
    }
    const fake = fakeApi(fixtureModel('fresh'));
    for (const name of Object.keys(fake.api) as (keyof Api)[]) vi.spyOn(fake.api, name);
    render(
      <div className="qs app-popover">
        <PopoverApp api={fake.api} clock={() => FIXTURE_NOW} />
      </div>,
    );
    await screen.findByRole('dialog', { name: 'Quota Station' });
    for (const svg of document.querySelectorAll('svg')) expect(['glyph', 'logo'], 'popover').toContain(kind(svg));
    expect([...seen].sort()).toEqual(['chart', 'glyph', 'logo']);
  });
});
