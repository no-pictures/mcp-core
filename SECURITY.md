# Security policy

## Supported versions

The latest tagged release and the `main` branch receive security fixes.

## Reporting a vulnerability

Report vulnerabilities privately to Stefan Grönke <stefan@gronke.net>.
Include a description, reproduction steps, and the affected component (auth, transports, web harness, or the embedded UI).

## Scope notes

- Servers built on this crate bind loopback by default; reports about intentionally exposed development setups (no `AUTH_TOKEN`, non-loopback binds) should state why the default posture is at fault.
- `streamable_http_router` accepts any `Host` header to work behind reverse proxies; DNS-rebinding hardening for directly exposed ports is documented in the README.
