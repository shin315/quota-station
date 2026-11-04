// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { describe, expect, it } from 'vitest';
import { communityModel, fixtureModel, releaseModel } from '@/fixtures/models';
import { appFixture } from '@/fixtures/models';
import { authorModel, macTools, sourceView } from './advanced';

describe('the author\'s tools', () => {
  it('come with a build that has them, and not with one that does not', () => {
    const full = fixtureModel('fresh');
    expect(authorModel(full)).toBe(full);
    expect(authorModel(communityModel(full))).toBeNull();
  });

  it('are in the downloaded app (`build=release`), which has no Antigravity accounts', () => {
    const release = releaseModel(fixtureModel('fresh'));
    expect(release.capabilities.authorTools).toBe(true);
    expect(release.capabilities.agyAccounts).toBe(false);
    expect(authorModel(release)).toBe(release);
    expect(release.advanced.on).toBe(true);
  });

  it('need every part of theirs in the model, not only the capability', () => {
    const full = fixtureModel('fresh');
    expect(authorModel({ ...full, rotation: undefined })).toBeNull();
    expect(authorModel({ ...full, capabilities: { ...full.capabilities, authorTools: false } })).toBeNull();
  });

  it('come on Windows without what only a Mac does', () => {
    const full = fixtureModel('fresh');
    const { version, build, commit, launchAtLogin } = appFixture();
    const win = { ...full, capabilities: { ...full.capabilities, platform: 'windows' as const }, app: { version, build, commit, launchAtLogin } };
    expect(authorModel(win)).toBe(win);
    expect(macTools(win)).toBe(false);
    expect(sourceView(win.app)).toBeNull();
    expect(macTools(full)).toBe(true);
    expect(sourceView(full.app)).toBe(full.app);
  });
});
