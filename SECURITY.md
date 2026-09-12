# Security Policy

## Supported versions

Security fixes are applied to the latest release on `main` and, when practical, to the most recent tagged release. Older tags are generally unsupported unless a critical issue warrants a backport.

| Version | Supported |
| --- | --- |
| Latest release / `main` | Yes |
| Older tags | Best effort only |

## What to report

Please report vulnerabilities that could affect users of omnisearch, including but not limited to:

- Credential or token leakage in logs, MCP responses, health tools, or error messages
- Authentication bypass on the optional HTTP transport (`AUTH_TOKENS`, bind address, CORS)
- SSRF or unsafe URL fetch / extract that reaches private, link-local, or metadata endpoints
- Path traversal or arbitrary file write via spill/temp paths
- Dependency or build-pipeline issues that ship compromised binaries
- Denial of service that is cheap to trigger from a remote MCP/HTTP client beyond normal rate limits

## What not to report here

- Bugs that do not have a security impact — use a normal [bug report](https://github.com/RaviTharuma/omnisearch/issues/new/choose)
- Provider outages, rate limits, or billing disputes with third-party APIs
- “Please add CAPTCHA bypass / scrape behind login” style requests — out of scope and may violate third-party terms (see [DISCLAIMER.md](DISCLAIMER.md))

## How to report

**Preferred:** use [GitHub Security Advisories](https://github.com/RaviTharuma/omnisearch/security/advisories/new) (private vulnerability reporting) for this repository.

If that is unavailable, open a **minimal** public issue titled `Security: contact maintainers` **without** exploit details, and wait for a maintainer to follow up privately.

Include:

1. Affected version / commit
2. Impact (confidentiality, integrity, availability)
3. Reproduction steps or PoC (kept private)
4. Any known mitigations

## Response process

1. Maintainers acknowledge the report when able.
2. We validate, assess severity, and prepare a fix.
3. We coordinate disclosure: prefer a fixed release (and advisory) before full public detail.
4. Credit reporters who wish to be named, unless they prefer anonymity.

We do not pay a bug bounty at this time.

## Safe harbor

Good-faith research that avoids privacy violations, destruction of data, and disruption of production services you do not operate is welcome. Do not use findings to access accounts or data that are not yours.

## Hardening tips for operators

- Prefer stdio MCP over exposing HTTP on public interfaces
- Use strong, rotated `AUTH_TOKENS` if you enable HTTP; bind to localhost unless you terminate TLS and auth elsewhere
- Never commit `.env` or paste keys into issues
- Monitor provider spend; parallel search can amplify cost
- Keep omnisearch and its dependencies updated

See also [DISCLAIMER.md](DISCLAIMER.md).
