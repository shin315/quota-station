// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The empty registry (`@qs-ext` without `QS_EXT_DIR`, vite.config.ts): the
// windows as the shared code draws them, nothing added.

import type { Extension } from './types';

const ext: Extension = { tabs: [] };

export default ext;
