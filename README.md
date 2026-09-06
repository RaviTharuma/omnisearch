# omnisearch

Unified [Model Context Protocol](https://modelcontextprotocol.io) server that searches and extracts across many providers from one tool surface.

Default behavior: **fan out in parallel** to every configured provider, **RRF-merge** results, and **dedupe by URL/title** while keeping `sources[]` provenance. There is **no small product cap** on result count. `limit` is an optional per-provider fetch hint. Omit it (or set `unlimited: true`) to page until providers exhaust or the **10,000 unique-result safety bound** (`OMNISEARCH_SAFETY_BOUND`).

Budgets (`max_providers`, `timeout_seconds`, `budget_usd`) cap **spend, fan-out width, and time** — not a tiny result ceiling.

License: Apache-2.0. Copyright 2026 Ravi Tharuma.

## Install

```bash
git clone https://github.com/RaviTharuma/omnisearch.git
cd omnisearch
cargo install --path .
```

Binary name: `omnisearch`.

```bash
# MCP over stdio (default)
omnisearch

# Streamable HTTP
omnisearch http

# Provider latency bench
omnisearch bench --query "rust async"
```

Copy `.env.example` to `.env` and set the keys you have. Missing keys are skipped, not fatal.

## MCP client wiring

### Claude Desktop

`claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "omnisearch": {
      "command": "omnisearch",
      "env": {
        "TAVILY_API_KEY": "tvly-...",
        "EXA_API_KEY": "..."
      }
    }
  }
}
```

### Cursor

`.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "omnisearch": {
      "command": "omnisearch",
      "args": ["stdio"],
      "env": {
        "TAVILY_API_KEY": "tvly-...",
        "EXA_API_KEY": "..."
      }
    }
  }
}
```

### mcpick / generic

```json
{
  "omnisearch": {
    "command": "omnisearch",
    "args": ["stdio"]
  }
}
```

HTTP transport (optional):

```bash
AUTH_TOKENS=replace-me OMNISEARCH_HTTP_BIND=127.0.0.1:48731 omnisearch http
```

Clients that speak streamable HTTP can use `http://127.0.0.1:48731/mcp` with `Authorization: Bearer replace-me`. `OMNISEARCH_HTTP_RPM` rate-limits per token.

## Tools

| Tool | Purpose |
| --- | --- |
| `search` | Parallel fan-out + RRF merge. Default `mode=all`. `mode=auto` uses query intent + health. `mode=ladder` is free-first then paid. |
| `ai_search` | Answer-oriented subset (Tavily/Kagi/You.com/Exa when configured). |
| `research` | Search, then extract top URLs under a time budget. |
| `extract` / `web_extract` | Vendor extract cascade, then SSRF-safe direct fetch. |
| `tavily_search` `exa_search` `linkup_search` `brave_search` `kagi_search` | Single-provider web search. |
| `x_search` `reddit_search` `youtube_search` `instagram_search` `facebook_search` | Social. |
| `github_search` | Repositories (`kind=repo`) or code (`kind=code`). |
| `firecrawl_scrape` `firecrawl_crawl` `firecrawl_map` | Firecrawl site tools. |
| `get_provider_info` | Non-secret metadata (configured, cost, requires_key, notes). |
| `search_health` | Live cooldown, latency, recent errors, requires_key. |
| `quality_report` | Domain diversity / coverage diagnostics. |
| `provider_bench` | Latency/error bench for configured providers. |

`search` arguments (stable JSON):

```json
{
  "query": "Swiss AI regulation 2026",
  "providers": ["tavily", "exa", "brave"],
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

Unified hit shape: `title`, `url`, `snippet`, `provider`, `score?`, `confidence?`, `published_at?`, `sources[]`, `snippet_grounded`.

Responses include partial-success metadata: `selected`, `successful`, `failed`, `timed_out`, `skipped`, `cost_usd`, `provider_used`, `stop_reason`. Partial fan-outs are **not cached** unless `cache_partial` / `OMNISEARCH_CACHE_PARTIAL` is set.

## Providers

Providers are env-gated. Unconfigured ones are skipped. Wikipedia, Semantic Scholar, Bluesky public search, and Reddit public JSON work without keys.

| Provider | Env | Search | Extract | Notes |
| --- | --- | --- | --- | --- |
| Tavily | `TAVILY_API_KEY` | yes | yes | News topic + time_range |
| Exa | `EXA_API_KEY` | yes | yes | Neural/keyword + `/contents` |
| Firecrawl | `FIRECRAWL_API_KEY` | yes | yes | scrape / crawl / map |
| Linkup | `LINKUP_API_KEY` | yes | — | `outputType=searchResults` |
| Brave | `BRAVE_API_KEY` | yes | — | Web + news, locale, freshness |
| Kagi | `KAGI_API_KEY` | yes | — | `Authorization: Bot …` |
| Perplexity | `PERPLEXITY_API_KEY` | yes | — | POST `/search`. Official MCP: `https://mcp.perplexity.ai/mcp` |
| You.com | `YOU_API_KEY` or `YDC_API_KEY` | yes | yes | `ydc-index.io/v1/search` |
| Parallel.ai | `PARALLEL_API_KEY` | yes | yes | `/v1/search`, `/v1/extract` |
| Querit | `QUERIT_API_KEY` | yes | yes | Multilingual filters |
| TinyFish | `TINYFISH_API_KEY` | yes | — | Source-only |
| Keenable | `KEENABLE_API_KEY` or `KEENABLE_PUBLIC=true` | yes | yes (keyed) | Public tier is keyless + rate limited |
| GitHub | `GITHUB_TOKEN` | yes | — | Repos / code |
| X | `X_BEARER_TOKEN` or `XAI_API_KEY` | yes | — | X API v2 recent search, else xAI `x_search` |
| Reddit | none, or `REDDIT_CLIENT_ID` + `REDDIT_CLIENT_SECRET` | yes | — | `search.json` + User-Agent; OAuth when set |
| YouTube | `YOUTUBE_API_KEY` or `GOOGLE_API_KEY` | yes | — | Data API v3 `search.list` |
| Instagram | `INSTAGRAM_ACCESS_TOKEN` + `INSTAGRAM_BUSINESS_ACCOUNT_ID` | hashtag | — | Official Graph hashtag API only. No free-text post search. |
| Facebook | `FACEBOOK_ACCESS_TOKEN` or `META_ACCESS_TOKEN` | pages | — | Official `/pages/search` only. Public post keyword search is not available. |
| Wikipedia | none | yes | — | MediaWiki search |
| Scholar | none | yes | — | Semantic Scholar paper search |
| Mastodon | `MASTODON_INSTANCE` (+ optional token) | yes | — | `/api/v2/search` |
| Bluesky | none | yes | — | `app.bsky.feed.searchPosts` |
| MCP backends | `OMNISEARCH_MCP_BACKENDS` | yes | — | `name|url|token,...` or `official` |

`OMNISEARCH_MCP_BACKENDS=official` attaches remotes that have keys: Tavily `https://mcp.tavily.com/mcp`, Exa `https://mcp.exa.ai/mcp`, Firecrawl `https://mcp.firecrawl.dev/mcp`, Linkup `https://mcp.linkup.so/mcp`, Kagi `https://kagi.com/api/mcp`, Perplexity `https://mcp.perplexity.ai/mcp`.

### Social API limits

- **Instagram**: official hashtag search (`/ig_hashtag_search` + `/recent_media`). Queries are reduced to a hashtag. Meta caps unique hashtags (30 / 7 days).
- **Facebook**: official Pages Search. Graph no longer offers public post keyword search.
- **X**: prefer `X_BEARER_TOKEN` against `https://api.x.com/2/tweets/search/recent`. `XAI_API_KEY` uses `POST https://api.x.ai/v1/responses` with `{ "type": "x_search" }`.

## Orchestration

- **Parallel default**: all configured providers, `tokio` join. `mode=ladder` / `OMNISEARCH_ROUTE=ladder` runs free providers first and may stop with `stop_reason=evidence`.
- **RRF + blend**: `score += 1 / (k + rank)` with `OMNISEARCH_RRF_K` (default 60), then `confidence = 0.50·RRF + 0.25·recency + 0.25·trust`.
- **Dedupe**: normalized URL (tracking params stripped) and near-duplicate titles. `sources[]` lists every contributing provider.
- **Dual-key failover**: `PROVIDER_API_KEY_2` (and `_3`) is tried after 429 / 5xx / timeout on the same provider.
- **Failover**: 429 / timeouts start a cooldown (`OMNISEARCH_COOLDOWN_SECS`). Auto routes skip cooling providers.
- **Grounded snippets**: `ground_top` / `OMNISEARCH_GROUND_TOP` fetches the top N URLs (SSRF-safe) and reframes the snippet; engine text is the fallback.
- **Adaptive routing**: `mode=auto` ranks by recent success and query intent (news, code, social, scholarly, video).
- **Cost gates**: `OMNISEARCH_AUTO_ALLOW_USD` skips expensive providers unless they are explicitly listed.
- **Cache**: in-process TTL (`OMNISEARCH_CACHE_TTL_SECS`). Pass `no_cache: true` to bypass.
- **Spam / diversity**: drops known shortener hosts; prefers domain diversity (`OMNISEARCH_MAX_PER_DOMAIN`) then appends overflow.
- **Freshness**: `day` / `week` / `month` / `year` forwarded to vendors and applied when `published_at` is parseable.
- **Locale**: `OMNISEARCH_COUNTRY` default `CH`, `OMNISEARCH_LANGUAGE` default `de`.
- **SSRF**: extract/direct-fetch blocks loopback, link-local, private IPs, `file:`, and metadata hosts.
- **Large payloads**: if JSON exceeds `OMNISEARCH_INLINE_MAX_BYTES`, results spill to a temp file and `delivery.mode` becomes `file`.

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Requires Rust 1.88+.

## Version

v0.1.0
