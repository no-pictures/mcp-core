// SPDX-FileCopyrightText: 2025-2026 Stefan Grönke <stefan@gronke.net>
// SPDX-License-Identifier: AGPL-3.0-or-later

import { test, expect } from "@playwright/test";
import { openView } from "./helpers";

// The security-critical rendering path: markdown converts inside a sandboxed iframe (the demo
// enables the crate's `web-markdown` feature), and failures surface as the shell's standard
// error alert instead of a blank pane.
test.describe("Markdown rendering", () => {
  test("a markdown field renders inside a sandboxed iframe", async ({ page }) => {
    await openView(page, /^Record$/);
    await page.locator(".list-group-item").filter({ hasText: "Alpha record" }).click();

    // The summary (x-mcp-kind: markdown) becomes a script-free sandboxed iframe.
    const iframe = page.locator(".mcp-entity iframe.mcp-sandbox");
    await expect(iframe).toHaveAttribute("sandbox", "allow-same-origin");

    // The converted document carries the rendered structure, not raw markdown.
    const body = page.frameLocator(".mcp-entity iframe.mcp-sandbox").locator("body");
    await expect(body.getByRole("heading", { name: "Alpha" })).toBeVisible();
    await expect(body.locator("strong")).toHaveText("first");
    await expect(body.locator("li").first()).toHaveText("one");
  });
});

test.describe("Error states", () => {
  test("a failing list endpoint surfaces the error alert", async ({ page }) => {
    await page.route("**/api/catalog/items/record*", (route) =>
      route.fulfill({ status: 500, body: "boom" }),
    );
    await openView(page, /^Record$/);
    await expect(page.locator(".alert-warning")).toContainText("HTTP 500");
  });
});
