// SPDX-FileCopyrightText: 2025-2026 Stefan Grönke <stefan@gronke.net>
// SPDX-License-Identifier: AGPL-3.0-or-later

// Unit tests only: tests/unit/** runs under Vitest (node + lit's SSR DOM shims); e2e/** stays
// Playwright's. Unit tests live outside src/ because build.rs bakes all of src/ into the
// served dist.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["tests/unit/**/*.test.ts"],
  },
});
