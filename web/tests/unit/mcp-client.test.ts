// SPDX-FileCopyrightText: 2025-2026 Stefan Grönke <stefan@gronke.net>
// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it } from "vitest";
import { eventStreamData } from "../../src/shell/mcp-client.js";

describe("eventStreamData", () => {
  it("extracts one payload per data line", () => {
    const body = 'event: message\ndata: {"a":1}\n\nevent: message\ndata: {"b":2}\n\n';
    expect(eventStreamData(body)).toEqual(['{"a":1}', '{"b":2}']);
  });

  it("tolerates a missing space after the colon", () => {
    expect(eventStreamData('data:{"a":1}\n\n')).toEqual(['{"a":1}']);
  });

  it("skips empty data lines, comments, and other fields", () => {
    const body = ": keep-alive\nid: 7\nretry: 100\ndata:\n\ndata: x\n\n";
    expect(eventStreamData(body)).toEqual(["x"]);
  });

  it("returns nothing for an empty body", () => {
    expect(eventStreamData("")).toEqual([]);
  });
});
