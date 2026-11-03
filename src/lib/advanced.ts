// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The advanced part (`capabilities.authorTools`): Cài đặt › Nâng cao and what
// it holds, the rotation with its rules, its pause and its switcher, the other
// Macs, Chép sang Mac khác and the update from source. A build without it
// sends none of their parts in the model, and the windows show none of them.
// Windows has them too but for what only a Mac does (`macTools`).

import type { AppView, Model, RotationView, SourceView } from '@/lib/model';

/** A model from a build with the advanced part: the rules editor comes with it. */
export type AuthorModel = Model & { rotation: RotationView };

/** `model` as a build with the advanced part sends it; null from a build without it. */
export function authorModel(model: Model): AuthorModel | null {
  return model.capabilities.authorTools && model.rotation ? (model as AuthorModel) : null;
}

/**
 * What only a Mac has of the advanced part: the other Macs (the rules'
 * copy to them, Chép sang Mac khác) and the update from source. A Windows
 * build has none of their commands, so the windows offer none of them.
 */
export function macTools(model: Pick<Model, 'capabilities'>): boolean {
  return model.capabilities.platform !== 'windows';
}

/** The update from source in the app's row; null where the build has none (Windows). */
export function sourceView(app: AppView): (AppView & SourceView) | null {
  return app.sourceStatus !== undefined ? (app as AppView & SourceView) : null;
}

/**
 * Cài đặt › Nâng cao is on. A build without the advanced part sends no
 * `advanced` in its model: Nâng cao is never on there.
 */
export function advancedOn(model: Pick<Model, 'advanced'>): boolean {
  return model.advanced?.on === true;
}
