// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// The Antigravity rotation's switch, under the Antigravity tab's account
// table when the app logs two accounts or more in: on, its LaunchAgent runs
// `quota-station antigravity rotate` every 3 minutes, whether the app is
// open or not, as the Claude rotation's does. Off for a new owner. It asks
// Rust for its state when it shows, and again after each change; an api
// without the rotation (the fixtures') shows nothing.

import { useEffect, useState } from 'react';
import { LIST_ROW } from '../components/page';
import { Toggle } from '../components/parts';
import type { AgyRotation as RotationView, Api } from '../lib/api';
import { t } from '../lib/i18n';

/** What the rotation does, in one line under its name. */
export function rotationLine(view: RotationView): string {
  if (!view.supported) return t('Chỉ có trên macOS.', 'On macOS only.');
  return t(
    `Mỗi 3 phút: khi tài khoản đang dùng còn ${view.threshold}% Gemini trở xuống, chuyển sang tài khoản còn nhiều nhất (hơn ít nhất ${view.minLead} điểm), tối đa một lần mỗi ${view.cooldownMinutes} phút. Antigravity.app đang chạy được thoát rồi mở lại; các phiên agy đang chạy cần mở lại.`,
    `Every 3 minutes: when the account in use has ${view.threshold}% of Gemini left or less, switch to the account with the most (at least ${view.minLead} points more), at most once every ${view.cooldownMinutes} minutes. A running Antigravity.app is quit and opened again; running agy sessions need a restart.`,
  );
}

export function AgyRotation({ api }: { api: Api }) {
  const [view, setView] = useState<RotationView | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    let live = true;
    api.agyRotation?.().then(
      (v) => live && setView(v),
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [api]);
  if (!view || !api.agySetRotation) return null;
  const set = async (on: boolean) => {
    setBusy(true);
    setError('');
    try {
      setView(await api.agySetRotation!(on));
    } catch (e) {
      setError(t('Không đổi được: ', 'Could not change it: ') + String(e));
    } finally {
      setBusy(false);
    }
  };
  const name = t('Tự chuyển khi gần hết Gemini', 'Switch automatically when Gemini runs low');
  return (
    <div className={LIST_ROW} role="group" aria-label={name}>
      <div>
        <b>{name}</b>
        <small className="block text-xs text-ink2">{rotationLine(view)}</small>
        {error ? (
          <small className="block text-xs text-bad-ink" role="alert">
            {error}
          </small>
        ) : null}
      </div>
      <span />
      <Toggle on={view.on === true} label={name} disabled={busy || !view.supported} onClick={() => void set(view.on !== true)} />
    </div>
  );
}
