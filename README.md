# omnisearch

Give your AI parallel web search across many engines so it finds sources a single search usually misses.

- **Parallel by default.** Every engine you configure is queried at once, not one at a time.
- **Merges and dedupes.** One ranked list. Duplicate URLs and titles collapse. Each hit records which engines found it.
- **Web, news, code, and social.** Brave and GitHub join automatically when keyed. News, X, Reddit, YouTube, and more when you add their keys.
- **Research mode.** Search first, then read the top pages so answers can cite real sources.
- **Drop-in for Claude Desktop and Cursor.** Point the client at the `omnisearch` binary and start searching.

Apache-2.0. Copyright 2026 Ravi Tharuma.

## Get started

Clone the repo, add the keys you have, run the binary:

```bash
git clone https://github.com/RaviTharuma/omnisearch.git
cd omnisearch
cp .env.example .env
omnisearch
```

`omnisearch` talks to Claude Desktop, Cursor, and any other MCP client over stdio. Use `omnisearch http` if you want a local HTTP endpoint instead.

If the binary is not on your PATH yet, install it from this checkout (`cargo install --path .`) or drop a release build on your PATH. Toolchain notes are under [Development](#development).

### Cursor

`.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "omnisearch": {
      "command": "omnisearch",
      "args": ["stdio"],
      "env": {
        "BRAVE_API_KEY": "...",
        "GITHUB_TOKEN": "...",
        "TAVILY_API_KEY": "tvly-...",
        "EXA_API_KEY": "..."
      }
    }
  }
}
```

### Claude Desktop

Same `command` and `env`, no `args`:

```json
{
  "omnisearch": {
    "command": "omnisearch"
  }
}
```

Any other MCP client can use `command: omnisearch` with `args: ["stdio"]`.

### HTTP (optional)

```bash
AUTH_TOKENS=replace-me OMNISEARCH_HTTP_BIND=127.0.0.1:48731 omnisearch http
```

`http://127.0.0.1:48731/mcp` with `Authorization: Bearer replace-me`. `OMNISEARCH_HTTP_RPM` rate-limits per token.

## Tools

| Tool | Purpose |
| --- | --- |
| `search` | Query every configured engine in parallel and return one merged list. Default `mode=all`. `auto` picks engines from the query and recent health. `ladder` tries free engines first. |
| `ai_search` | Answer-oriented subset (Tavily / Kagi / You.com / Exa / Perplexity when configured). |
| `research` | Search, then extract top URLs under a time budget. |
| `extract` / `web_extract` | Pull page text from extract vendors, then a safe direct fetch. |
| `brave_search` `tavily_search` `exa_search` `linkup_search` `kagi_search` | Single-engine web search. |
| `github_search` | Repos (`kind=repo`), code (`kind=code`), users (`kind=users`). |
| `x_search` `reddit_search` `youtube_search` `instagram_search` `facebook_search` | Official social APIs. |
| `firecrawl_scrape` `firecrawl_crawl` `firecrawl_map` | Site scrape / crawl / map. |
| `get_provider_info` | Configured, cost, requires_key, notes. No secrets. |
| `search_health` | Cooldown, latency, recent errors. |
| `quality_report` | Domain diversity and coverage. |
| `provider_bench` | Latency / error bench. |

`search` arguments:

```json
{
  "query": "Swiss AI regulation 2026",
  "providers": ["brave", "github", "tavily", "exa"],
  "limit": 20,
  "unlimited": false,
  "mode": "all",
  "search_type": "news",
  "freshness": "week",
  "country": "CH",
  "language": "de",
  "max_providers": 8,
  "timeout_seconds": 20,
  "budget_usd": 0.05,
  "no_cache": false,
  "cache_partial": false,
  "ground_top": 3,
  "evidence_min": 8,
  "quality_report": true
}
```

Each hit: `title`, `url`, `snippet`, `provider`, `score?`, `confidence?`, `published_at?`, `sources[]`, `snippet_grounded`.

Run meta: `selected`, `successful`, `failed`, `timed_out`, `skipped`, `cost_usd`, `provider_used`, `stop_reason`. Partial runs are not cached unless `cache_partial` is set.

`limit` is a per-engine hint. Omit it or set `unlimited: true` to keep paging until engines are exhausted or 10,000 unique results. Budgets cap spend, how many engines run, and time — not a tiny result ceiling.

## Providers

Unconfigured engines are skipped. Wikipedia, Semantic Scholar, Bluesky, and Reddit public JSON work without keys. Brave (`BRAVE_API_KEY`) and GitHub (`GITHUB_TOKEN` / `GITHUB_API_KEY`) join the default run when set.

| Provider | Env | Search | Extract | Notes |
| --- | --- |
| Brave | `BRAVE_API_KEY` | yes | — | Web + news. On by default when set. |
| GitHub | `GITHUB_TOKEN` or `GITHUB_API_KEY` | yes | — | Repos, code, users. On by default when set. |
| Tavily | `TAVILY_API_KEY` | yes | yes | News topic + time_range |
| Exa | `EXA_API_KEY` | yes | yes | Neural / keyword + `/contents` |
| Firecrawl | `FIRECRAWL_API_KEY` | yes | yes | scrape / crawl / map |
| Linkup | `LINKUP_API_KEY` | yes | — | `outputType=searchResults` |
| Kagi | `KAGI_API_KEY` | yes | — | `Authorization: Bot …` |
| Perplexity | `PERPLEXITY_API_KEY` | yes | — | POST `/search` |
| You.com | `YOU_API_KEY` or `YDC_API_KEY` | yes | yes | `ydc-index.io/v1/search` |
| Parallel.ai | `PARALLEL_API_KEY` | yes | yes | `/v1/search`, `/v1/extract` |
| Querit | `QUERIT_API_KEY` | yes | yes | Multilingual filters |
| TinyFish | `TINYFISH_API_KEY` | yes | — | Source-only |
| Keenable | `KEENABLE_API_KEY` or `KEENABLE_PUBLIC=true` | yes | keyed | Public tier is keyless |
| X | `X_BEARER_TOKEN` or `XAI_API_KEY` | yes | — | API v2 recent search, else xAI `x_search` |
| Reddit | none, or OAuth pair | yes | — | `search.json`; OAuth when set |
| YouTube | `YOUTUBE_API_KEY` or `GOOGLE_API_KEY` | yes | — | Data API v3 |
| Instagram | Graph token + business account | hashtag | — | Official hashtag API only |
| Facebook | `FACEBOOK_ACCESS_TOKEN` | pages | — | Official `/pages/search` only |
| Wikipedia | none | yes | — | MediaWiki |
| Scholar | none | yes | — | Semantic Scholar |
| Mastodon | `MASTODON_INSTANCE` | yes | — | `/api/v2/search` |
| Bluesky | none | yes | — | `app.bsky.feed.searchPosts` |
| MCP backends | `OMNISEARCH_MCP_BACKENDS` | yes | — | `name\|url\|token,...` or `official` |

`OMNISEARCH_MCP_BACKENDS=official` attaches remotes that have keys: Tavily, Exa, Firecrawl, Linkup, Kagi, Perplexity.

Social API limits: Instagram is hashtag-only (Meta caps unique hashtags 30 / 7 days). Facebook is Pages Search only. X prefers `X_BEARER_TOKEN` on `https://api.x.com/2/tweets/search/recent`.

## How it works

- Every configured engine runs in parallel. `mode=ladder` runs free engines first and can stop once enough evidence is in (`stop_reason=evidence`).
- Results merge with reciprocal rank fusion: `score += 1 / (k + rank)` (`OMNISEARCH_RRF_K`, default 60). Confidence blends that score with recency and domain trust.
- Tracking parameters are stripped. `sources[]` lists every engine that returned the URL.
- Dual-key failover: `NAME_2` / `NAME_3` after 429 / 5xx / timeout.
- Health cooldown: 429 / timeouts cool an engine; `mode=auto` skips it.
- `ground_top` fetches the top N URLs (private and loopback hosts blocked) and reframes the snippet.
- `mode=auto` ranks engines by recent success and query type.
- `OMNISEARCH_AUTO_ALLOW_USD` skips expensive engines unless you list them in `providers[]`.
- In-process cache with a TTL. Partial runs are not cached unless asked. `no_cache: true` bypasses.
- Known shorteners are dropped; hits per domain are capped, then overflow is appended.
- Freshness: `day` / `week` / `month` / `year`. Defaults `OMNISEARCH_COUNTRY=CH`, `OMNISEARCH_LANGUAGE=de`.
- Extract and direct-fetch block loopback, link-local, private IPs, `file:`, and metadata hosts.
- Payloads over `OMNISEARCH_INLINE_MAX_BYTES` spill to a temp file.

## Development

Needs a Rust 1.88+ toolchain. Binary and crate name: `omnisearch`. v0.1.0.

```bash
cargo install --path .
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
omnisearch bench --query "Swiss AI regulation 2026"
```
