# Gradience SDKs

Official and community SDKs for the Gradience Wallet API.

## Available SDKs

| SDK | Language | Status | Path |
|-----|----------|--------|------|
| `gradience-sdk` | Python | ✅ v0.1 shipped | [`python/`](./python/) |
| `@gradience/sdk` | TypeScript | ✅ v0.1 shipped | [`typescript/`](./typescript/) |
| `gradience` (Go) | Go | ✅ v0.1 skeleton | [`go/`](./go/) |
| `io.gradience:sdk` | Java | ✅ v0.1 skeleton | [`java/`](./java/) |
| `gradience` (Ruby) | Ruby | ✅ v0.1 skeleton | [`ruby/`](./ruby/) |

## Quick Links

- [SDK Development Guide & Roadmap](../docs/06-sdk-guide.md)
- [Gradience API Reference](../crates/gradience-api)
- [Python SDK README](./python/README.md)
- [TypeScript SDK README](./typescript/README.md)
- [Go SDK README](./go/README.md)
- [Java SDK README](./java/README.md)
- [Ruby SDK README](./ruby/README.md)

## Adding a New SDK

1. Create a new folder under `sdk/<language>/`.
2. Implement a thin HTTP wrapper around the [Gradience REST API](../crates/gradience-api).
3. Keep cryptography out of the SDK — delegate signing to the local API/MCP.
4. Add `README.md`, tests, and a `package.json`/`pyproject.toml`/`go.mod`/`build.gradle`/`gemspec` equivalent.
5. Update this file and [`docs/06-sdk-guide.md`](../docs/06-sdk-guide.md).
