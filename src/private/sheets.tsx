// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The rotation's sheets (F07): pause automatic switching, and the two
// rotation guides. `components/Sheets.tsx` draws them in both hosts.

import { useState } from 'react';
import { DialogButtons, DialogLead, DialogTitle } from '@/components/ui/dialog';
import { Button, ChoiceGroup, Hint, Rich } from '@/components/parts';
import { HOWTO, type SheetBodyProps } from '@/components/Sheets';
import { t } from '@/lib/i18n';
import { guide, pauseOptions, pauseSheet, type PauseChoice } from './switching';

export function PauseBody(props: SheetBodyProps & { preset: PauseChoice }) {
  const [choice, setChoice] = useState<PauseChoice>(props.preset);
  // The clock as of the last confirm, when it ran ahead of the window's.
  const [late, setLate] = useState<Date | null>(null);
  const now = late && late > props.now ? late : props.now;
  const v = pauseSheet(props.model, choice, now);
  const value = v.options.some((o) => o.v === choice) ? choice : v.options[0].v;
  const confirm = () => {
    const at = props.clock();
    // "Đến 18:00" is gone once 18:00 has passed: show the choices again
    // rather than pause with no end.
    if (!pauseOptions(at).some((o) => o.v === value)) return setLate(at);
    // The pause ends at the time its choice shows.
    const option = v.options.find((o) => o.v === value);
    if (option) props.onPause(option.until);
  };
  return (
    <>
      <DialogTitle>{v.title}</DialogTitle>
      <DialogLead>{v.lead}</DialogLead>
      <ChoiceGroup<PauseChoice> id="qs-pause" label={t('Trong bao lâu', 'For how long')} options={v.options} value={value} columns={1} onChange={setChoice} />
      <Hint aria-live="polite">{v.hint}</Hint>
      <DialogButtons>
        <Button large onClick={props.onClose}>
          {t('Thôi', 'Cancel')}
        </Button>
        <Button large primary onClick={confirm}>
          {t('Tạm dừng', 'Pause')}
        </Button>
      </DialogButtons>
    </>
  );
}

export function GuideBody(props: SheetBodyProps & { id: 'auto' | 'switcher' }) {
  const g = guide(props.id, props.model.capabilities.authorTools);
  return (
    <>
      <DialogTitle>{g.title}</DialogTitle>
      <DialogLead>{g.lead}</DialogLead>
      <ol className={HOWTO}>
        {g.steps.map((s) => (
          <li key={s}>
            <Rich text={s} />
          </li>
        ))}
      </ol>
      <DialogButtons>
        <Button large primary onClick={props.onClose}>
          {t('Đóng', 'Close')}
        </Button>
      </DialogButtons>
    </>
  );
}
