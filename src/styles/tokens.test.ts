// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The theme packs' promise (docs/themes.md), checked on tokens.css itself:
// in every pack and both modes, every text colour reaches WCAG AA (4.5:1) on
// every surface it is drawn on, and the quota bars (not short, short, out)
// stay apart from each other and from their track, for normal sight and for
// the two common red-green deficiencies.
//
// Glass is composited over two grounds per mode, the bounds of what shows
// through macOS's material behind the windows: light over white and over a
// mid grey a dark wallpaper bleeds through; dark over black and over a grey
// a bright wallpaper bleeds through.

import { describe, expect, it } from 'vitest';
import source from './tokens.css?raw';
import { THEME_PACKS, type ThemePack } from '../lib/model';

type Tokens = Record<string, string>;
type Rgb = [number, number, number];
type Rgba = [number, number, number, number];

const css = source.replace(/\/\*[\s\S]*?\*\//g, '');

/** The declarations of the first top-level rule whose selector list is exactly `selector`. */
function block(selector: string): Tokens {
  const norm = (s: string) => s.replace(/\s+/g, ' ').trim();
  const re = /([^{}]+)\{([^{}]*)\}/g;
  for (let m = re.exec(css); m; m = re.exec(css)) {
    if (norm(m[1]) !== norm(selector)) continue;
    const out: Tokens = {};
    for (const d of m[2].split(';')) {
      const i = d.indexOf(':');
      if (i > 0 && d.trim().startsWith('--')) out[d.slice(0, i).trim().slice(2)] = d.slice(i + 1).trim();
    }
    return out;
  }
  throw new Error('no rule ' + selector);
}

const BASE_LIGHT = block(':root, [data-theme="light"], [data-theme="auto"], [data-pack="default"]');
const BASE_DARK = block('[data-theme="dark"]');
const ALIASES = block(':root, [data-theme], [data-material], [data-pack]');

function tokens(pack: ThemePack, dark: boolean): Tokens {
  const t = { ...BASE_LIGHT, ...ALIASES, ...(dark ? BASE_DARK : {}) };
  if (pack !== 'default') Object.assign(t, block(`[data-pack="${pack}"]`), dark ? block(`[data-pack="${pack}"][data-theme="dark"]`) : {});
  return t;
}

function color(t: Tokens, name: string, depth = 0): Rgba {
  const v = t[name];
  if (v == null || depth > 8) throw new Error('no token --' + name);
  const ref = /^var\(--([\w-]+)\)$/.exec(v);
  if (ref) return color(t, ref[1], depth + 1);
  if (v === 'transparent') return [0, 0, 0, 0];
  const hex = /^#([0-9a-f]{6})$/i.exec(v);
  if (hex) return [0, 2, 4].map((i) => parseInt(hex[1].slice(i, i + 2), 16)).concat(1) as Rgba;
  const rgba = /^rgba?\(([^)]+)\)$/.exec(v);
  if (rgba) {
    const [r, g, b, a = '1'] = rgba[1].split(',').map((x) => x.trim());
    return [Number(r), Number(g), Number(b), Number(a)];
  }
  throw new Error(`--${name} is not a colour: ${v}`);
}

const over = (top: Rgba, under: Rgb): Rgb => [0, 1, 2].map((i) => top[i] * top[3] + under[i] * (1 - top[3])) as Rgb;
const lin = (c: number) => ((c /= 255) <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const lum = (c: Rgb) => 0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2]);
function ratio(a: Rgb, b: Rgb) {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** OKLab, for how far apart two colours look. */
function oklab(c: Rgb): Rgb {
  const [r, g, b] = c.map(lin);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s, 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s, 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s];
}
const deltaE = (a: Rgb, b: Rgb) => Math.hypot(...oklab(a).map((v, i) => v - oklab(b)[i]));

/** Machado et al. (2009) at full severity, on linear light. */
const CVD: Record<string, number[][]> = {
  normal: [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
  ],
  protan: [
    [0.152286, 1.052583, -0.204868],
    [0.114503, 0.786281, 0.099216],
    [-0.003882, -0.048116, 1.051998],
  ],
  deutan: [
    [0.367322, 0.860646, -0.227968],
    [0.280085, 0.672501, 0.047413],
    [-0.01182, 0.04294, 0.968881],
  ],
};
function seen(c: Rgb, m: number[][]): Rgb {
  const l = c.map(lin);
  const unlin = (v: number) => 255 * (v <= 0.0031308 ? 12.92 * v : 1.055 * Math.max(v, 0) ** (1 / 2.4) - 0.055);
  return m.map((row) => unlin(Math.min(1, row[0] * l[0] + row[1] * l[1] + row[2] * l[2]))) as Rgb;
}

const GROUNDS: Record<'light' | 'dark', Rgb[]> = {
  light: [
    [255, 255, 255],
    [184, 184, 190],
  ],
  dark: [
    [0, 0, 0],
    [106, 106, 112],
  ],
};

/** Every surface text sits on, composited, with whether it is glass (the stronger --ink3 and --accent-ink apply). */
function surfaces(t: Tokens, dark: boolean): { name: string; rgb: Rgb; glass: boolean }[] {
  const out: { name: string; rgb: Rgb; glass: boolean }[] = [];
  const solid: Rgb = over(color(t, 'glass-solid'), [255, 255, 255]);
  for (const [i, ground] of GROUNDS[dark ? 'dark' : 'light'].entries()) {
    const glass = over(color(t, 'glass'), ground);
    out.push({ name: `glass/${i}`, rgb: glass, glass: true });
    out.push({ name: `panel/${i}`, rgb: over(color(t, 'panel'), glass), glass: true });
    out.push({ name: `fill2 on panel/${i}`, rgb: over(color(t, 'fill2'), over(color(t, 'panel'), glass)), glass: true });
    out.push({ name: `sel/${i}`, rgb: over(color(t, 'sel'), glass), glass: true });
    out.push({ name: `side/${i}`, rgb: over(color(t, 'side-glass'), glass), glass: true });
  }
  out.push({ name: 'solid', rgb: solid, glass: false });
  out.push({ name: 'panel on solid', rgb: over(color(t, 'panel'), solid), glass: false });
  out.push({ name: 'sel on solid', rgb: over(color(t, 'sel'), solid), glass: false });
  out.push({ name: 'card', rgb: over(color(t, 'card'), solid), glass: false });
  return out;
}

const TEXT = ['ink', 'ink2', 'ink3', 'accent-ink', 'info-ink', 'ok-ink', 'warn-ink', 'bad-ink'];

/**
 * Mặc định is the look the app wore before the packs, kept unchanged: where
 * the worst ground shows through its glass, a few of its smaller texts fall
 * short. They are listed here so none is added; the packs have none.
 */
const DEFAULT_SHORT: Record<'light' | 'dark', string[]> = {
  light: [
    '--ink2 on fill2 on panel/1: 4.24',
    '--ink3-glass on fill2 on panel/1: 4.05',
    '--accent-ink-glass on fill2 on panel/1: 4.00',
    '--ink3 on panel on solid: 4.48',
    '--ink3 on sel on solid: 4.31',
  ],
  dark: [
    '--ink3-glass on panel/1: 4.21',
    '--accent-ink-glass on panel/1: 4.21',
    '--ink2 on fill2 on panel/1: 3.52',
    '--ink3-glass on fill2 on panel/1: 3.14',
    '--accent-ink-glass on fill2 on panel/1: 3.14',
    '--info-ink on fill2 on panel/1: 3.49',
    '--ok-ink on fill2 on panel/1: 4.25',
    '--bad-ink on fill2 on panel/1: 3.56',
    '--ink2 on sel/1: 4.42',
    '--ink3-glass on sel/1: 3.94',
    '--accent-ink-glass on sel/1: 3.94',
    '--info-ink on sel/1: 4.38',
    '--bad-ink on sel/1: 4.47',
    '--ink3 on sel on solid: 4.16',
    '--accent-ink on sel on solid: 4.38',
  ],
};

const MODES = [false, true];
const cases = THEME_PACKS.flatMap((pack) => MODES.map((dark) => [pack, dark] as const));

describe('theme packs', () => {
  it.each(cases)('%s (dark %s): every text colour reaches 4.5:1 on every surface', (pack, dark) => {
    const t = tokens(pack, dark);
    const low: string[] = [];
    for (const s of surfaces(t, dark)) {
      for (const name of TEXT) {
        const key = s.glass && (name === 'ink3' || name === 'accent-ink') ? name + '-glass' : name;
        const r = ratio(over(color(t, key), s.rgb), s.rgb);
        if (r < 4.5) low.push(`--${key} on ${s.name}: ${r.toFixed(2)}`);
      }
    }
    const solid = over(color(t, 'glass-solid'), [255, 255, 255]);
    const accent = over(color(t, 'accent'), solid);
    const onAccent = ratio(over(color(t, 'on-accent'), accent), accent);
    if (onAccent < 4.5) low.push(`--on-accent on --accent: ${onAccent.toFixed(2)}`);
    const badFill = over(color(t, 'bad-fill'), solid);
    const badge = ratio([255, 255, 255], badFill);
    if (badge < 4.5) low.push(`white on --bad-fill: ${badge.toFixed(2)}`);
    expect(low, `${pack} ${dark ? 'dark' : 'light'}`).toEqual(pack === 'default' ? DEFAULT_SHORT[dark ? 'dark' : 'light'] : []);
  });

  it.each(cases)('%s (dark %s): the quota bars stay apart, for normal sight and red-green deficiencies', (pack, dark) => {
    const t = tokens(pack, dark);
    const ground = over(color(t, 'panel'), over(color(t, 'glass-solid'), [255, 255, 255]));
    const track = over(color(t, 'track'), ground);
    const bars: Record<string, Rgb> = { ok: over(color(t, 'meter-ok'), track), warn: over(color(t, 'warn'), track), bad: over(color(t, 'bad'), track) };
    const near: string[] = [];
    for (const [vision, m] of Object.entries(CVD)) {
      const names = Object.keys(bars);
      for (const [i, a] of names.entries()) {
        if (deltaE(seen(bars[a], m), seen(track, m)) < 0.12) near.push(`${vision}: ${a} on its track`);
        for (const b of names.slice(i + 1)) {
          // Short and out also differ by their number's colour and the words beside them; the bar adds a cue.
          if (deltaE(seen(bars[a], m), seen(bars[b], m)) < 0.05) near.push(`${vision}: ${a} / ${b} ${deltaE(seen(bars[a], m), seen(bars[b], m)).toFixed(3)}`);
        }
      }
    }
    expect(near, `${pack} ${dark ? 'dark' : 'light'}`).toEqual([]);
  });

  it('each pack sets the same names in both modes, so a swatch never shows another pack’s value', () => {
    const light = Object.keys(block('[data-pack="kinhNhip"]')).sort();
    for (const pack of THEME_PACKS.filter((p) => p !== 'default')) {
      expect(Object.keys(block(`[data-pack="${pack}"]`)).sort()).toEqual(light);
      const dark = Object.keys(block(`[data-pack="${pack}"][data-theme="dark"]`)).sort();
      expect(dark).toEqual(Object.keys(block(`[data-pack="${pack}"][data-theme="auto"]`)).sort());
      expect(light.filter((k) => !dark.includes(k)).sort()).toEqual(['font', 'font-features', 'r-card', 'r-ctl', 'r-pop', 'r-row', 'r-sheet', 'r-tile', 'r-win']);
    }
  });
});
