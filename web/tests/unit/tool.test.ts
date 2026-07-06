// SPDX-FileCopyrightText: 2025-2026 Stefan Grönke <stefan@gronke.net>
// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it } from "vitest";
import { tryParseJson } from "../../src/shell/tool.js";

describe("tryParseJson", () => {
  it("parses objects and arrays", () => {
    expect(tryParseJson('{"x":1}')).toEqual({ x: 1 });
    expect(tryParseJson(" [1,2] ")).toEqual([1, 2]);
  });

  it("leaves prose, numbers, and malformed JSON alone", () => {
    expect(tryParseJson("plain text")).toBeUndefined();
    expect(tryParseJson("42")).toBeUndefined();
    expect(tryParseJson('{"broken":')).toBeUndefined();
  });
});
