// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Cài đặt › Hiển thị › Chủ đề: how a theme pack is seen and picked. Each
// pack is shown as Quota Station's window in small, drawn in the pack's own
// tokens (MiniWindow): its desktop, its glass over two shapes (clear glass
// lets them through, dense glass hides them), its sidebar and the row in
// use, a title in its type, a group with two quota bars, and its accent.
// The six sit three to a row, each its name under it, in the look the
// windows are in (Giao diện); while they follow macOS, light and dark split
// across the diagonal. Picking one redraws every window at once.

import { RadioGroup as RadioGroupPrimitive } from 'radix-ui';
import { t } from '../lib/i18n';
import { THEME_PACKS, type Theme, type ThemePack } from '../lib/model';
import { cn } from '@/lib/utils';

/** The theme packs by the names the owner gave their styles; the same in both languages. */
export const PACK_NAMES: Record<ThemePack, string> = {
  default: 'Mặc định',
  kinhNhip: 'Kính Nhịp',
  daiSangTinh: 'Dải sáng tĩnh',
  motNhipNhin: 'Một nhịp nhìn',
  banDieuKhien: 'Bàn điều khiển',
  chieuLang: 'Chiều lặng',
};

/** A row of the Hiển thị card that spans its width: the name, then the picker under it. */
const BLOCK_ROW = 'srow grid gap-y-2 border-t-[.5px] border-card-line py-2.5 first:border-t-0 first:pt-0.5 last:pb-0.5';

/**
 * Quota Station's window in small, in `pack`'s tokens and one mode (`auto`
 * follows macOS, as the windows do). Its box sets its size; everything in it
 * is in container units, so it reads the same at any size.
 */
export function MiniWindow({ pack, mode, className }: { pack: ThemePack; mode: Theme; className?: string }) {
  return (
    <span
      data-pack={pack}
      data-theme={mode}
      aria-hidden="true"
      className={cn('@container relative block overflow-hidden text-left font-sans [font-feature-settings:var(--font-features)] [background:var(--desk)]', className)}
    >
      {/* Behind the window: what clear glass lets through. */}
      <span className="absolute top-[4%] left-[50%] size-[46cqw] rounded-full bg-[#ff9a62]" />
      <span className="absolute top-[46%] left-[80%] size-[30cqw] rounded-full bg-[#4f7dff]" />
      <span className="absolute top-[12%] -right-[6%] -bottom-[12%] left-[7%] overflow-hidden rounded-[calc(var(--r-win)*.6)] [background:var(--win-mist),var(--glass)] shadow-[0_4px_14px_rgba(0,0,0,.22),inset_0_0_0_.5px_var(--glass-line)] backdrop-blur-[calc(var(--bl-pop)*.3)] backdrop-saturate-[var(--sat)] in-data-[material=solid]:[background:var(--win-mist),var(--glass-solid)] in-data-[material=solid]:backdrop-blur-none">
        <span className="absolute inset-y-0 left-0 grid w-[27cqw] content-start gap-[1.2cqw] px-[2.6cqw] pt-[13cqw] [background:var(--side-glass)]">
          {[0.55, 0.7, 0.45, 0.6].map((w, i) => (
            <span key={i} className={cn('flex h-[6cqw] items-center rounded-[calc(var(--r-row)*.5)] px-[2cqw]', i === 1 && '[background:var(--sel)]')}>
              <span className={cn('h-[1.6cqw] rounded-full bg-ink', i === 1 ? 'opacity-80' : 'opacity-30')} style={{ width: w * 100 + '%' }} />
            </span>
          ))}
        </span>
        <span className="absolute inset-y-0 right-0 left-[27cqw] grid content-start gap-[2.4cqw] pt-[11cqw] pr-[10cqw] pl-[5cqw]">
          <span className="block text-[6.4cqw]/none font-heading text-ink">Claude</span>
          <span className="grid gap-[2.6cqw] rounded-[calc(var(--r-card)*.55)] px-[3.6cqw] py-[3cqw] [background:var(--panel)] shadow-[inset_0_0_0_.5px_var(--panel-line)]">
            <MiniBar label={t('5 giờ', '5-hour')} used={62} tone="ok" />
            <MiniBar label={t('Tuần', 'Week')} used={88} tone="warn" />
            <span className="flex justify-end">
              <span className="rounded-[calc(var(--r-ctl)*.6)] bg-accent px-[2.6cqw] py-[.8cqw] text-[3.6cqw]/none font-semi text-on-accent">{t('Chuyển', 'Switch')}</span>
            </span>
          </span>
        </span>
        {/* Over the sidebar's tint, as macOS draws them. */}
        <span className="absolute top-[5cqw] left-[4cqw] flex gap-[1.3cqw]">
          <span className="size-[2.2cqw] rounded-full bg-[#ff5f57]" />
          <span className="size-[2.2cqw] rounded-full bg-[#febc2e]" />
          <span className="size-[2.2cqw] rounded-full bg-[#28c840]" />
        </span>
      </span>
    </span>
  );
}

function MiniBar({ label, used, tone }: { label: string; used: number; tone: 'ok' | 'warn' }) {
  return (
    <span className="grid gap-[1.2cqw]">
      <span className="flex items-baseline justify-between">
        <span className="text-[3.6cqw]/none text-ink2">{label}</span>
        <span className="text-[4.8cqw]/none font-semi text-ink">{used}%</span>
      </span>
      <span className="block h-[1.6cqw] overflow-hidden rounded-full bg-track">
        <span className={cn('block h-full rounded-full', tone === 'ok' ? 'bg-meter-ok' : 'bg-warn')} style={{ width: used + '%' }} />
      </span>
    </span>
  );
}

/**
 * A pack as the windows will draw it: in the mode picked, or, while they
 * follow macOS, light and dark split across the diagonal, as macOS draws
 * its own Tự động.
 */
function Specimen({ pack, theme, className }: { pack: ThemePack; theme: Theme; className?: string }) {
  if (theme !== 'auto') return <MiniWindow pack={pack} mode={theme} className={className} />;
  return (
    <span className={cn('relative block overflow-hidden', className)}>
      <MiniWindow pack={pack} mode="light" className="absolute inset-0" />
      <MiniWindow pack={pack} mode="dark" className="absolute inset-0 [clip-path:polygon(100%_0,100%_100%,0_100%)]" />
    </span>
  );
}

/** The ring round the pack chosen, in the accent of the windows as they are now. */
const RING =
  "relative block before:pointer-events-none before:absolute before:-inset-[3px] before:z-[1] before:rounded-[11px] before:border-2 before:border-transparent before:content-[''] group-data-[state=checked]/pack:before:border-accent";

/** Chủ đề: the six windows in small, three to a row (two in a narrow window), each its name under it. */
export function ThemePicker({ value, theme, onPick }: { value: ThemePack; theme: Theme; onPick(pack: ThemePack): void }) {
  return (
    <div className={BLOCK_ROW}>
      <div id="qs-pack-l" className="text-sm font-semi">
        {t('Chủ đề', 'Theme')}
      </div>
      <RadioGroupPrimitive.Root
        className="grid grid-cols-3 gap-x-3.5 gap-y-3 pt-1 @max-[600px]/main:grid-cols-2"
        aria-labelledby="qs-pack-l"
        value={value}
        loop
        onValueChange={(v) => onPick(v as ThemePack)}
      >
        {THEME_PACKS.map((pack) => (
          <RadioGroupPrimitive.Item
            key={pack}
            value={pack}
            className="group/pack grid min-w-0 cursor-pointer gap-1.5 rounded-[10px] border-0 bg-transparent p-0 text-center text-xs text-ink2 focus-visible:outline-offset-4 data-[state=checked]:font-semi data-[state=checked]:text-ink"
          >
            <span className={RING}>
              <Specimen pack={pack} theme={theme} className="aspect-[16/9] rounded-[8px] shadow-[0_0_0_.5px_var(--glass-line)]" />
            </span>
            <span className="truncate">{PACK_NAMES[pack]}</span>
          </RadioGroupPrimitive.Item>
        ))}
      </RadioGroupPrimitive.Root>
    </div>
  );
}
