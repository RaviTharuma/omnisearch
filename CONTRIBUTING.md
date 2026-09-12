# Contributing to omnisearch

Thanks for helping improve omnisearch. This document covers how to propose changes, report issues, and keep the project maintainable.

By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md) and acknowledge the [Disclaimer](DISCLAIMER.md) and [Apache License 2.0](LICENSE).

## Before you start

1. Search [existing issues](https://github.com/RaviTharuma/omnisearch/issues) and pull requests for duplicates.
2. For security issues, **do not** open a public issue — follow [SECURITY.md](SECURITY.md).
3. Prefer a short issue or discussion before large architectural PRs (new providers, auth redesigns, protocol changes).

## Ways to contribute

| Kind | Prefer |
| --- | --- |
| Bug fix | Repro steps + failing test when practical |
| Docs | Clarity in README, `docs/`, or community files |
| Provider adapter | Extend existing patterns; keys via env / named accounts only |
| Feature | Issue first if it changes MCP tool surface or config schema |
| CI / release | Keep workflows locked and smoke tests green |

## Development setup

Requires Rust **1.88+** (see `rust-toolchain.toml`).

```bash
git clone https://github.com/RaviTharuma/omnisearch.git
cd omnisearch
cp .env.example .env   # add only keys you need; never commit .env
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Optional: `omnisearch bench --query "…"` against live keys. Prefer `wiremock` or local fixtures in automated tests so CI stays keyless.

## Coding guidelines

- **Rust style:** `cargo fmt` is mandatory; clippy with `-D warnings` must pass.
- **Secrets:** never log, return, or snapshot API keys, tokens, or account secrets. Health/report tools must stay sanitized.
- **Providers:** follow existing adapter + `OMNISEARCH_ACCOUNTS` / dual-key failover patterns. Prefer extending shared infrastructure over one-off auth.
- **Fail closed:** invalid account pins, bad gateway config, and unsafe extract targets should error clearly without leaking secrets.
- **SSRF / fetch safety:** do not weaken loopback, private-IP, or metadata-host blocks without an explicit, reviewed design.
- **MCP surface:** new tools or breaking argument changes need docs (`README.md`, changelog) in the same PR.
- **Dependencies:** pin via `Cargo.lock`; avoid drive-by version bumps unrelated to the change.
- **Comments:** English only; no emoji in comments.

## Tests

- Add or update unit/integration tests for behavior you change.
- Provider HTTP: mock responses when possible.
- If a live-key test is unavoidable, gate it and document the env vars; default CI must not require secrets.

## Pull requests

1. Branch from the latest `main`.
2. Keep PRs focused — one concern per PR when practical.
3. Fill out the pull request template.
4. Ensure CI is green: fmt, clippy, tests, and any package-version checks.
5. Update `CHANGELOG.md` for user-visible changes.
6. Do not force-push shared branches you do not own; rebase only if maintainers ask.

Maintainers may squash-merge. You retain copyright on your contributions; by submitting a PR you license your contribution under the same [Apache License 2.0](LICENSE) as the project (including the patent grant).

## Issue reporting

Use the GitHub issue forms when available:

- **Bug report** — unexpected behavior, crashes, incorrect merge/dedupe, SSRF/fetch regressions
- **Feature request** — new capability or provider with clear use case
- **Provider / integration** — specific API adapter or named-account behavior

See [Reporting issues](docs/reporting-issues.md) for what to include and what to redact.

## Review expectations

- Small, well-tested PRs review faster.
- Maintainers may request changes, split PRs, or decline scope that conflicts with project priorities (see project docs for provider priority).
- “LGTM” is not automatic merge; CI and docs still need to land.

## Questions

Use GitHub Issues for actionable work. For “how do I configure X?”, check [README.md](README.md), [docs/accounts-and-gateways.md](docs/accounts-and-gateways.md), and [SUPPORT.md](SUPPORT.md) first.
