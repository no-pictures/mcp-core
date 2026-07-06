// SPDX-FileCopyrightText: 2025-2026 Stefan Grönke <stefan@gronke.net>
// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it } from "vitest";
import { defaults, toInputValue } from "../../src/shell/schema-form.js";

describe("toInputValue", () => {
  it("passes primitives through as strings", () => {
    expect(toInputValue("x")).toBe("x");
    expect(toInputValue(42)).toBe("42");
    expect(toInputValue(false)).toBe("false");
  });

  it("never renders objects or nullish values as text", () => {
    expect(toInputValue({ a: 1 })).toBe("");
    expect(toInputValue([1])).toBe("");
    expect(toInputValue(null)).toBe("");
    expect(toInputValue(undefined)).toBe("");
  });
});

describe("defaults", () => {
  it("collects the declared property defaults", () => {
    const schema = {
      type: "object",
      properties: {
        q: { type: "string", default: "hello" },
        limit: { type: "integer", default: 10 },
        flag: { type: "boolean", default: false },
        free: { type: "string" },
      },
    };
    expect(defaults(schema)).toEqual({ q: "hello", limit: 10, flag: false });
  });

  it("is empty for a schema without properties or for null", () => {
    expect(defaults({})).toEqual({});
    expect(defaults(null)).toEqual({});
  });
});
