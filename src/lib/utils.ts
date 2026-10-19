// shadcn/ui's `cn`: joins class names and lets a later Tailwind class win
// over an earlier one. tailwind-merge learns the theme's own names (theme.css),
// so `text-body` stays a size and `text-ink2` a colour.

import { clsx, type ClassValue } from 'clsx';
import { extendTailwindMerge } from 'tailwind-merge';

const twMerge = extendTailwindMerge({
  extend: {
    theme: {
      text: ['cap', 'xs', 's', 'sm', 'body', 'base', 'title', 'sheet', 'num', 'h2', 'big'],
      'font-weight': ['reg', 'med', 'semi', 'bold', 'heavy', 'black'],
      radius: ['pop', 'win', 'sheet', 'card', 'tile', 'row', 'ctl', 'chip'],
      shadow: ['pop', 'sheet', 'card', 'win'],
      color: [
        'ink',
        'ink2',
        'ink3',
        'fill',
        'fill2',
        'sel',
        'card',
        'card-line',
        'winbg',
        'scrim',
        'track',
        'glass',
        'glass-solid',
        'glass-line',
        'accent',
        'accent-ink',
        'on-accent',
        'info-ink',
        'ok',
        'ok-ink',
        'warn',
        'warn-ink',
        'bad',
        'bad-ink',
        'bad-fill',
        'id-claude',
        'id-claude-ink',
        'id-claude-tint',
        'id-agy',
        'id-agy-ink',
        'id-agy-tint',
        'id-codex',
        'id-codex-ink',
        'id-codex-tint',
      ],
    },
  },
});

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
