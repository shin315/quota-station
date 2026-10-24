// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The tools made for the author's own Macs (the `author-tools` feature,
// `capabilities.authorTools`): Cài đặt › Nâng cao and what it holds. A
// release build leaves them out, and its model has none of their parts.

import type { AppView, FirstmateView, Model, RotationView, SourceView } from './model';

/** A model from a build with the author's tools: the rules editor, Firstmate and the update from source come with it. */
export type AuthorModel = Model & { rotation: RotationView; firstmate: FirstmateView; app: AppView & SourceView };

/** `model` as a build with the author's tools sends it; null from a build without them. */
export function authorModel(model: Model): AuthorModel | null {
  const { rotation, firstmate, app } = model;
  return model.capabilities.authorTools && rotation && firstmate && app.sourceStatus !== undefined ? (model as AuthorModel) : null;
}
