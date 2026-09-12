# Changelog

## 0.2.2

- Discord guild message search (`discord_search`) with `DISCORD_BOT_TOKEN` + `DISCORD_GUILD_IDS` and named `OMNISEARCH_ACCOUNTS` (`token` + `guild_ids`, optional `bearer`).
- Reddit named `Public {}` accounts (docs/code aligned); X/Reddit/Discord multi-account tests and account-pin tool notes.
- Auto intent routes `twitter`, `x.com`, and `discord` into the social provider set.

## 0.2.1

- Firecrawl crawl/map route through named-account pools (accounts-only configs work).
- Linkup: `/v1/fetch` extract, configurable depth (`fast`/`flash`/`standard`/`deep`), `maxResults`, freshness→`fromDate`.
- Optional `account` pin on search/extract/Firecrawl tools; unknown names fail closed; omit keeps rotation.
- `OMNISEARCH_MCP_BACKENDS=official` expands all env keys and named api_key accounts (Brave stays native must-have).

## 0.2.0

- Named per-provider accounts with independent rotation, timeout budgets, cooldowns, sanitized health, and account-bound pagination.
- Backward-compatible legacy key lists when no named accounts are configured for a provider.
- Multiple OmniRoute search gateways using gateway credentials only and the normalized `/v1/search` endpoint.
- `account_health` MCP tool; gateway validation, cost estimates, routing and mock coverage.

## 0.1.0

- Initial multi-provider MCP search server.
