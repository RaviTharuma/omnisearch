# Reporting issues

Use this checklist so maintainers can reproduce and fix problems quickly.

## Choose the right template

| Situation | Template |
| --- | --- |
| Crash, wrong results, MCP/HTTP bug | Bug report |
| New tool, flag, or behavior | Feature request |
| One provider, named account, or gateway | Provider / integration |
| Credential leak, auth bypass, SSRF | **Do not file publicly** — [SECURITY.md](../SECURITY.md) |

## Always include

1. **omnisearch version** — `Cargo.toml` version, git tag, or commit SHA
2. **OS and arch** — e.g. Linux x86_64, macOS arm64
3. **How you run it** — Claude Desktop, Cursor `mcp.json`, `omnisearch http`, etc.
4. **Minimal config** — which providers/env vars (names only, not values)
5. **Expected vs actual** — one short paragraph each
6. **Repro steps** — numbered; smallest query/tool call that fails

## Redact secrets

Never paste:

- API keys, bearer tokens, OAuth client secrets
- Full `.env` files
- Authorization headers
- Account passwords or session cookies

Replace secrets with placeholders (`BRAVE_API_KEY=***`). Sanitize logs the same way.

## Provider-specific reports

Also include:

- Provider name and whether you use legacy env keys or `OMNISEARCH_ACCOUNTS`
- HTTP status and **non-secret** error body snippets from the provider (if any)
- Whether dual-key (`_2` / `_3`) or named-account rotation was involved
- Approximate time (UTC) and whether the vendor dashboard showed errors/quota

## Feature requests

Describe the user problem first, then a proposed shape (tool name, args, config). Call out breaking changes. Large provider additions should note official API docs links (no scraped-login designs).

## After you file

- Reply to maintainer questions on the same issue
- Open a PR only if you can follow [CONTRIBUTING.md](../CONTRIBUTING.md)
- Close or update the issue if you find it was a third-party outage or misconfiguration

See [SUPPORT.md](../SUPPORT.md) for channel guidance.
