// SPDX-FileCopyrightText: 2025-2026 Stefan Grönke <stefan@gronke.net>
// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it } from "vitest";
import { fieldKind, fieldLayout } from "../../src/shell/catalog-fields.js";

describe("fieldKind", () => {
  it("prefers the explicit x-mcp-kind", () => {
    expect(fieldKind({ type: "string", "x-mcp-kind": "markdown" })).toBe("markdown");
  });

  it("maps enum, format, and type to built-in kinds", () => {
    expect(fieldKind({ type: "string", enum: ["a", "b"] })).toBe("enum");
    expect(fieldKind({ type: "string", format: "date" })).toBe("date");
    expect(fieldKind({ type: "string", format: "date-time" })).toBe("date");
    expect(fieldKind({ type: "integer" })).toBe("number");
    expect(fieldKind({ type: "boolean" })).toBe("bool");
    expect(fieldKind({ type: "array" })).toBe("list");
    expect(fieldKind({ type: "object" })).toBe("json");
    expect(fieldKind({ type: "string" })).toBe("text");
    expect(fieldKind({})).toBe("text");
  });
});

describe("fieldLayout", () => {
  it("routes kinds to their entity-view placement", () => {
    expect(fieldLayout("markdown")).toBe("body");
    expect(fieldLayout("table")).toBe("section");
    expect(fieldLayout("json")).toBe("section");
    expect(fieldLayout("text")).toBe("metadata");
    expect(fieldLayout("date")).toBe("metadata");
  });
});
