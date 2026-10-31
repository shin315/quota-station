// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The tools made for the author's own Macs (the `author-tools` feature,
// `capabilities.authorTools`): Cài đặt › Nâng cao and what it holds. A
// release build leaves them out, and its model has none of their parts.
// Windows has them too but for what only a Mac does (`macTools`).

import type { AppView, FirstmateView, Model, RotationView, SourceView } from '@/lib/model';

/** A model from a build with the author's tools: the rules editor and Firstmate come with it. */
export type AuthorModel = Model & { rotation: RotationView; firstmate: FirstmateView };

/** `model` as a build with the author's tools sends it; null from a build without them. */
export function authorModel(model: Model): AuthorModel | null {
  const { rotation, firstmate } = model;
  return model.capabilities.authorTools && rotation && firstmate ? (model as AuthorModel) : null;
}

/**
 * What only a Mac has of the author's tools: the other Macs (the rules'
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
