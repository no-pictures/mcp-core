# Changelog

Notable changes to mcp-core.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); releases are git tags.

## [Unreleased]

### Added
- CI gates: per-feature builds (cargo-hack), dependency and license audit (cargo-deny), REUSE lint.
- `rust-version` (MSRV) 1.95, Dependabot updates, and a security policy (`SECURITY.md`).

### Fixed
- The landing page escapes the reflected `Host`/`X-Forwarded-Proto` origin, and the plain-`web` CSP gains `script-src 'self'`.
- The token-auth middleware builds its 401 challenge without panicking on a header-invalid realm.
- The typed catalog clamps the client-supplied list `limit` to 500.
- The shell redirects to sign-in when the session cookie expires, surfaces renderer failures instead of a blank pane, and no longer shows German strings.

## [0.1.0] - 2026-07-06

Initial release: token/session authentication, environment configuration with safe path resolution, MCP transports (Streamable HTTP `/mcp`, legacy HTTP+SSE, stdio), the hardened web harness, the embedded schema-driven web UI (typed catalog, search, operations console), shared server CLI flags, and the consumer test harness.
