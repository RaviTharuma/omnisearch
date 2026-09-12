# Contributing to omnisearch

Thanks for helping improve omnisearch.

**By participating (opening issues with patches, submitting pull requests, or proposing workflows), you agree to:**

1. The [Code of Conduct](CODE_OF_CONDUCT.md)
2. The full [Disclaimer / Assumption of Risk](DISCLAIMER.md) — including **§7 Contributors** (warranties + indemnity)
3. The [Apache License 2.0](LICENSE)

If you do not agree — especially the no-liability, supply-chain, spend, and malicious-contribution terms — **do not contribute**.

## Hard rules for every contribution

- **No malware, backdoors, miners, secret exfiltration, or hidden API-spend amplifiers.**
- **No drive-by dependency upgrades** that expand supply-chain surface without clear justification and lockfile review.
- **Never commit secrets** (`.env`, tokens, keys). Never log secrets.
- **Do not weaken SSRF / private-IP / metadata protections** without an explicit security design reviewed in the PR description.
- **You are solely responsible** for the contents of your PR. Maintainer merge does **not** transfer that responsibility or create liability for Project Parties (see [DISCLAIMER.md](DISCLAIMER.md)).

## Before you start

1. Search [existing issues](https://github.com/RaviTharuma/omnisearch/issues) and pull requests for duplicates.
2. For security issues, **do not** open a public issue — follow [SECURITY.md](SECURITY.md).
3. Prefer a short issue before large architectural PRs (new providers, auth redesigns, protocol changes).

## Ways to contribute

| Kind | Prefer |
| --- | --- |
| Bug fix | Repro steps + failing test when practical |
| Docs | Clarity in README, `docs/`, or community files |
| Provider adapter | Extend existing patterns; keys via env / named accounts only |
| Feature | Issue first if it changes MCP tool surface or config schema |
| CI / release | Keep workflows locked and smoke tests green; treat workflow changes as high risk |

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

**Warning:** live-key benches can burn paid credits. You pay those bills — not the Project Parties.

## Coding guidelines

- **Rust style:** `cargo fmt` is mandatory; clippy with `-D warnings` must pass.
- **Secrets:** never log, return, or snapshot API keys, tokens, or account secrets. Health/report tools must stay sanitized.
- **Providers:** follow existing adapter + `OMNISEARCH_ACCOUNTS` / dual-key failover patterns. Prefer extending shared infrastructure over one-off auth.
- **Fail closed:** invalid account pins, bad gateway config, and unsafe extract targets should error clearly without leaking secrets.
- **SSRF / fetch safety:** do not weaken loopback, private-IP, or metadata-host blocks without an explicit, reviewed design.
- **MCP surface:** new tools or breaking argument changes need docs (`README.md`, changelog) in the same PR.
- **Dependencies:** pin via `Cargo.lock`; justify every new crate; prefer well-known maintained crates; avoid unexplained binary blobs.
- **Comments:** English only; no emoji in comments.

## Tests

- Add or update unit/integration tests for behavior you change.
- Provider HTTP: mock responses when possible.
- If a live-key test is unavoidable, gate it and document the env vars; default CI must not require secrets.

## Pull requests

1. Branch from the latest `main`.
2. Keep PRs focused — one concern per PR when practical.
3. Fill out the pull request template, including the **contributor warranty** checkboxes.
4. Ensure CI is green: fmt, clippy, tests, and any package-version checks. Green CI is **not** a safety certification.
5. Update `CHANGELOG.md` for user-visible changes.
6. Do not force-push shared branches you do not own; rebase only if maintainers ask.

Maintainers may squash-merge, rewrite, or reject any contribution for any reason or no reason. Merge **does not** mean the change was audited for security, supply-chain risk, or spend impact.

You retain copyright on your contributions; by submitting a PR you license your contribution under the same [Apache License 2.0](LICENSE) as the project (including the patent grant), and you accept [DISCLAIMER.md](DISCLAIMER.md) §7.

### Developer Certificate of Origin (DCO)

Each contribution must be certifiable under the DCO 1.1. By opening a PR you certify:

```
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.

Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

Optional: add `Signed-off-by: Your Name <email>` in commits. Opening the PR with the template checkboxes checked is the minimum acceptance for this repository.

## Issue reporting

Use the GitHub issue forms when available:

- **Bug report** — unexpected behavior, crashes, incorrect merge/dedupe, SSRF/fetch regressions
- **Feature request** — new capability or provider with clear use case
- **Provider / integration** — specific API adapter or named-account behavior

See [Reporting issues](docs/reporting-issues.md) for what to include and what to redact.

## Review expectations

- Small, well-tested PRs review faster.
- Maintainers may request changes, split PRs, or decline scope that conflicts with project priorities.
- Review is **best-effort and non-exhaustive**. Maintainers explicitly disclaim liability for accepting a bad or malicious PR (see [DISCLAIMER.md](DISCLAIMER.md) §3.2 and §5).
- “LGTM” is not a security audit.

## Questions

Use GitHub Issues for actionable work. For configuration help, check [README.md](README.md), [docs/accounts-and-gateways.md](docs/accounts-and-gateways.md), and [SUPPORT.md](SUPPORT.md) first.
