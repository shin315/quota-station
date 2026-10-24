// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

// A text field: a light fill with a hairline, the text selectable.
const inputVariants = cva(
  'w-full min-w-0 select-text rounded-ctl border-0 bg-fill px-2 py-[5px] font-sans text-sm leading-[inherit] text-ink [font-variant-numeric:inherit] shadow-[inset_0_0_0_.5px_var(--card-line)]',
  {
    variants: {
      mono: { true: 'font-mono text-[11.5px] leading-[inherit]' },
      invalid: { true: 'shadow-[inset_0_0_0_1px_var(--bad)]' },
    },
  },
);

function Input({ className, mono, invalid, ...props }: React.ComponentProps<'input'> & VariantProps<typeof inputVariants>) {
  return <input data-slot="input" data-invalid={invalid || undefined} className={cn(inputVariants({ mono, invalid }), className)} {...props} />;
}

function Textarea({ className, mono, invalid, ...props }: React.ComponentProps<'textarea'> & VariantProps<typeof inputVariants>) {
  return <textarea data-slot="textarea" data-invalid={invalid || undefined} className={cn(inputVariants({ mono, invalid }), className)} {...props} />;
}

export { Input, Textarea, inputVariants };
