// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { describe, expect, it } from 'vitest';
import { communityModel, fixtureModel } from '../fixtures/models';
import { authorModel } from './author';

describe('the author\'s tools', () => {
  it('come with a build that has them, and not with one that does not', () => {
    const full = fixtureModel('fresh');
    expect(authorModel(full)).toBe(full);
    expect(authorModel(communityModel(full))).toBeNull();
  });

  it('need every part of theirs in the model, not only the capability', () => {
    const full = fixtureModel('fresh');
    expect(authorModel({ ...full, rotation: undefined })).toBeNull();
    expect(authorModel({ ...full, firstmate: undefined })).toBeNull();
    expect(authorModel({ ...full, capabilities: { ...full.capabilities, authorTools: false } })).toBeNull();
  });
});
