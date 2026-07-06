// SPDX-FileCopyrightText: 2025-2026 Stefan Grönke <stefan@gronke.net>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dogfoods the consumer test harness against the demo server: the spawn/connect paths
//! (`connect_stdio`, `connect_http`) drive the same binary the browser e2e suite uses, so the
//! "an MCP client can connect and call tools" contract is verified over both transports.

#![cfg(feature = "test-harness")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use mcp_core::testing::{connect_http, connect_stdio, Result};

/// Builds the demo once per test run and returns its binary path.
fn demo_bin() -> &'static str {
    static BIN: OnceLock<String> = OnceLock::new();
    BIN.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let status = Command::new(cargo)
            .args(["build", "-p", "mcp-ui-demo"])
            .status()
            .expect("spawn cargo build");
        assert!(status.success(), "cargo build -p mcp-ui-demo failed");

        let target = std::env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"));
        target
            .join("debug")
            .join(format!("mcp-ui-demo{}", std::env::consts::EXE_SUFFIX))
            .into_os_string()
            .into_string()
            .expect("utf-8 binary path")
    })
}

#[tokio::test]
async fn stdio_transport_serves_the_demo_tools() -> Result<()> {
    let mcp = connect_stdio(demo_bin(), &[]).await?;

    let tools = mcp.tools().await?;
    assert!(tools.contains(&"echo".to_string()), "tools: {tools:?}");
    assert!(tools.contains(&"add".to_string()), "tools: {tools:?}");

    let outcome = mcp
        .call_tool("add", serde_json::json!({ "a": 2, "b": 40 }))
        .await?;
    assert_eq!(outcome.text(), "42");
    Ok(())
}

#[tokio::test]
async fn streamable_http_transport_serves_the_demo_tools() -> Result<()> {
    let mcp = connect_http(demo_bin(), &[]).await?;

    let tools = mcp.tools().await?;
    assert!(tools.contains(&"echo".to_string()), "tools: {tools:?}");

    let outcome = mcp
        .call_tool("echo", serde_json::json!({ "message": "hello" }))
        .await?;
    assert_eq!(outcome.text(), "hello");
    Ok(())
}
