# Security Policy

## Supported versions

Security fixes are applied to the latest release on `main` and, when practical, to the most recent tagged release. Older tags are generally unsupported unless a critical issue warrants a backport.

| Version | Supported |
| --- | --- |
| Latest release / `main` | Best effort only |
| Older tags | Best effort only |

**No SLA.** Acknowledgement, triage, fix, and disclosure are voluntary. Reporting a vulnerability does **not** create liability, a warranty, or a service obligation for Project Parties. See [DISCLAIMER.md](DISCLAIMER.md).

## What to report

Please report vulnerabilities that could affect users of omnisearch, including but not limited to:

- Credential or token leakage in logs, MCP responses, health tools, or error messages
- Authentication bypass on the optional HTTP transport (`AUTH_TOKENS`, bind address, CORS)
- SSRF or unsafe URL fetch / extract that reaches private, link-local, or metadata endpoints
- Path traversal or arbitrary file write via spill/temp paths
- Dependency or build-pipeline issues that ship compromised binaries (**supply chain**)
- Hidden spend amplifiers or credential exfiltration in contributions or dependencies
- Denial of service that is cheap to trigger from a remote MCP/HTTP client beyond normal rate limits

## What not to report here

- Bugs that do not have a security impact — use a normal [bug report](https://github.com/RaviTharuma/omnisearch/issues/new/choose)
- Provider outages, rate limits, billing disputes, or **your own API credit overusage** — you alone control keys and spend ([DISCLAIMER.md](DISCLAIMER.md) §3.3)
- “Please add CAPTCHA bypass / scrape behind login” style requests — out of scope and may violate third-party terms

## How to report

**Preferred:** use [GitHub Security Advisories](https://github.com/RaviTharuma/omnisearch/security/advisories/new) (private vulnerability reporting) for this repository.

If that is unavailable, open a **minimal** public issue titled `Security: contact maintainers` **without** exploit details, and wait for a maintainer to follow up privately.

Include:

1. Affected version / commit
2. Impact (confidentiality, integrity, availability, **or unbounded spend**)
3. Reproduction steps or PoC (kept private)
4. Any known mitigations

## Response process

1. Maintainers acknowledge the report when able (no timeline promised).
2. We may validate, assess severity, and prepare a fix — or decline / defer.
3. We prefer coordinated disclosure when a fix ships; public detail may wait.
4. Credit reporters who wish to be named, unless they prefer anonymity.

We do not pay a bug bounty at this time.

**Missed vulns, delayed fixes, and compromised releases remain risks you assume** ([DISCLAIMER.md](DISCLAIMER.md) §3 and §5).

## Safe harbor

Good-faith research that avoids privacy violations, destruction of data, and disruption of production services you do not operate is welcome. Do not use findings to access accounts or data that are not yours.

## Hardening tips for operators

- Prefer stdio MCP over exposing HTTP on public interfaces
- Use strong, rotated `AUTH_TOKENS` if you enable HTTP; bind to localhost unless you terminate TLS and auth elsewhere
- Never commit `.env` or paste keys into issues
- **Cap spend at the vendor** (billing alerts, hard quotas). Do not rely on omnisearch budget flags alone
- Pin commits / verify release checksums; review `Cargo.lock` and workflow changes yourself
- Keep omnisearch and its dependencies updated at your own risk/cadence
- Treat every merge — including maintainer merges — as untrusted until you review it

See also [DISCLAIMER.md](DISCLAIMER.md).
