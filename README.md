# mcp-core

Shared infrastructure for MCP and web servers in Rust:
HTTP transports for MCP (Streamable HTTP and legacy SSE), token/session authentication, a hardened axum web harness, and an embedded, schema-driven web UI for inspecting a server's data and operating its tools.

## Feature flags

Every capability is opt-in, so a consumer only compiles the dependency graph it uses.

| Feature | What it enables |
|---|---|
| `auth` | `TokenAuthLayer`: Bearer/Basic token middleware with constant-time comparison |
| `config` | `BaseConfig` (env-derived), CSPRNG token generation, `safe_resolve` path containment |
| `bootstrap` | `init_tracing` (tracing-subscriber setup) |
| `sse` | MCP over legacy HTTP+SSE: `GET /sse` + `POST /message` (`AuthSseServer`) |
| `streamable-http` | MCP over Streamable HTTP: a single `/mcp` endpoint (`streamable_http_router`) |
| `stdio` | rmcp's stdio transport for MCP-over-stdin/stdout consumers, without any HTTP stack |
| `web` | Web harness: `serve` on multiple addresses, security headers + CSP, gzip, `protect` |
| `web-ui` | The embedded Lit shell at `/ui/`, `app_router`, typed `DataCatalog` + search routers |
| `web-markdown` | Markdown rendering in the shell (sandboxed iframe); vendors `marked` |
| `web-ui-dev` | Live-reload shell development from `web/src`; never enable in a release build |
| `cli` | `ServerArgs` (clap flags: `--stdio`/`--sse`/`--mcp`/`--web`, listen, auth) |
| `server` | Everything a full server binary needs (`web` + transports + `cli` + `bootstrap`) |
| `test-harness` | `testing::connect_stdio`/`connect_http`: drive your server binary with a real MCP client |
| `ts-export` | ts-rs export of the web wire types (CI keeps `web/src/shell/generated` in sync) |
| `full` *(default)* | `auth` + `config` + `sse` + `streamable-http` + `stdio` + `bootstrap` |

## Usage

The crate is consumed as a git dependency; pin a tag (or a rev) rather than tracking `main`:

```toml
[dependencies]
mcp-core = { git = "https://github.com/MCPeas/mcp-core", tag = "v0.1.0" }

# Or with specific features only
mcp-core = { git = "https://github.com/MCPeas/mcp-core", tag = "v0.1.0", default-features = false, features = ["streamable-http", "web-ui"] }
```

The minimum supported Rust version is 1.95.

### Quickstart: Streamable HTTP + web UI

`app_router` assembles the whole app under one auth policy: a public landing/login page at `/`, the embedded shell at `/ui/`, your REST API, and your MCP endpoint.

```rust
use mcp_core::{app_router, data_catalog_router, streamable_http_router, web::{serve, Landing}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_core::init_tracing("my_server=debug,mcp_core=info");

    let mcp = streamable_http_router(MyServer::new, "/mcp");
    let api = data_catalog_router(std::sync::Arc::new(MyCatalog));
    let auth_token = std::env::var("AUTH_TOKEN").ok();

    let app = app_router(
        api,
        "/api",
        mcp,
        auth_token.as_deref(),
        Landing::new("my-server").mcp(true),
    );
    serve(app, vec!["127.0.0.1".parse()?], 8080).await?;
    Ok(())
}
```

With `AUTH_TOKEN` set, one check guards `/ui`, the API and `/mcp` together:
browsers sign in at `/` (a `SameSite=Strict` session cookie), MCP clients send `Authorization: Bearer <token>`.
Unset, everything is open — intended for loopback-only development.

The shell explores a typed `DataCatalog` (entity lists, facets, relations, per-item actions calling MCP tools) and offers an operations console over the server's own `/mcp` endpoint — no per-server frontend code required.

### Legacy HTTP+SSE transport

```rust
use mcp_core::{AuthSseServer, TokenAuthLayer};

let (mut sse_server, sse_router) = AuthSseServer::new();
let protected = sse_router.layer(TokenAuthLayer::new(std::env::var("AUTH_TOKEN")?));

while let Some(transport) = sse_server.next_transport().await {
    // Serve one MCP session on this transport.
}
```

### Authentication middleware

```rust
use mcp_core::TokenAuthLayer;
use axum::{Router, routing::get};

let app = Router::new()
    .route("/api", get(handler))
    .layer(TokenAuthLayer::new(token));
```

`TokenAuthLayer` accepts `Authorization: Bearer <token>` or Basic auth with the token as password, compares in constant time, and answers everything else with a 401 challenge.

### Configuration

```rust
use mcp_core::BaseConfig;

let config = BaseConfig::from_env();
let (token, was_generated) = config.get_or_generate_token();
```

Environment variables read by `BaseConfig`:

- `HOST` — server bind address (default `127.0.0.1`)
- `PORT` — server port (default `3000`)
- `DATA_PATH` — base path for data files (default `./data`)
- `AUTH_TOKEN` — authentication token; generated (CSPRNG) when unset

Servers using the `cli` feature take the same decisions from clap flags instead: `ServerArgs` provides `--stdio`, `--sse`, `--mcp`, `--web`, `--http-port`, `--http-listen` and `--auth-token`/`AUTH_TOKEN`.

### Testing a consumer server

The `test-harness` feature (a dev-dependency for consumers) spawns your compiled server binary and drives it with a real rmcp client over stdio (`connect_stdio`) or Streamable HTTP (`connect_http`), plus JSON helpers to assert on tool results.
It expects the binary to understand the `ServerArgs` flags.

## Demo

`examples/mcp-ui-demo` is the runnable reference: a Streamable HTTP MCP server with a typed catalog, search, markdown rendering and per-item actions, served with the embedded UI.

```sh
cargo run -p mcp-ui-demo
# landing on http://127.0.0.1:8080/   UI on /ui/   MCP on /mcp
```

Set `AUTH_TOKEN` to try the sign-in flow; the Playwright e2e suite (`web/e2e`) runs against this demo.

## Security notes

- Servers bind loopback (IPv4 + IPv6) by default; exposing other addresses is an explicit opt-in.
- `streamable_http_router` disables rmcp's allowed-hosts check (DNS-rebinding protection) to accept reverse-proxied requests; re-enable it if you expose the port directly to a hostile network.
- Every response carries baseline security headers: a CSP (with `script-src` pinned to `'self'` plus the hash of the shell's inlined import map), `X-Frame-Options: DENY`, `nosniff`, `no-referrer`, and a restrictive Permissions-Policy.
- Untrusted rendered content (markdown, HTML tool output) is contained in script-free sandboxed iframes.
- Session cookies are signed, `HttpOnly`, `Secure`, `SameSite=Strict`.

## Development

```sh
cargo test --all-features        # Rust tests (also regenerates the ts-rs bindings)
cargo hack check --each-feature --no-dev-deps   # every feature compiles in isolation
cargo deny check                 # advisories, licenses, sources
cd web && npm ci && npm run check   # prettier + eslint + tsc
cd web && npm run test:e2e       # Playwright against the built demo
```

CI runs all of the above plus a REUSE lint; `web/src/shell/generated` must stay in sync with the Rust wire types.

## Versioning

Releases are git tags (`v0.1.0`, ...) with notable changes recorded in [CHANGELOG.md](CHANGELOG.md).
The crate is not published to crates.io; consumers pin a tag as shown above.

## License

Copyright (C) 2025-2026 Stefan Grönke <stefan@gronke.net>.

mcp-core is free software: you can redistribute it and/or modify it under the
terms of the GNU Affero General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later version
(`AGPL-3.0-or-later`). See [LICENSE](LICENSE) for the full text.

Source: <https://github.com/MCPeas/mcp-core>
