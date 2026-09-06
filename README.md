# omnisearch

The search MCP you actually want in an agent: **one tool call, every engine you have keys for, merged and cited.**

`omnisearch` is a Rust [Model Context Protocol](https://modelcontextprotocol.io) server. It fans out in **parallel** to every configured provider, **RRF-merges** the lists, **dedupes by URL and title**, and keeps `sources[]` so you can see which engines found each hit. There is no tiny “top 5” product cap. `limit` is an optional per-provider hint. Omit it (or set `unlimited: true`) and it pages until providers exhaust or the **10,000 unique-result safety bound**.

Budgets cap **spend, width, and time** — not result quality.

**First-class engines** (always in the default fan-out when keyed): **Brave Search** (`BRAVE_API_KEY`) and **GitHub Search** (`GITHUB_TOKEN` or `GITHUB_API_KEY`) for repositories, code, and users.

Apache-2.0. Copyright 2026 Ravi Tharuma.

## Why this exists

A single search API misses what another engine has. Models guess when they should look. This server makes **multi-engine search the default**, then adds the controls agents need: provenance, partial-success metadata, failover, research (search then extract), freshness, news, locale, cache, spam/diversity, cost gates, and a live health surface.

Set the keys you already have. Missing keys are skipped, not fatal.

## Install

```bash
git clone https://github.com/RaviTharuma/omnisearch.git
cd omnisearch
cargo install --path .
```

```bash
omnisearch                          # MCP over stdio
omnisearch http                     # streamable HTTP
omnisearch bench --query "rust async"
```

Copy `.env.example` to `.env`.

### Claude Desktop

```json
{
  "mcpServers": {
    "omnisearch": {
      "command": "omnisearch",
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

### Any MCP client

```json
{
  "omnisearch": {
    "command": "omnisearch",
    "args": ["stdio"]
  }
}
```

HTTP (optional):

```bash
AUTH_TOKENS=replace-me OMNISEARCH_HTTP_BIND=127.0.0.1:48731 omnisearch http
```

Connect to `http://127.0.0.1:48731/mcp` with `Authorization: Bearer replace-me`. `OMNISEARCH_HTTP_RPM` rate-limits per token.

## Tools

| Tool | Purpose |
| --- | --- |
| `search` | Parallel fan-out + RRF merge. Default `mode=all`. `auto` = intent + health. `ladder` = free-first then paid. |
| `ai_search` | Answer-oriented subset (Tavily / Kagi / You.com / Exa / Perplexity when configured). |
| `research` | Search, then extract top URLs under a time budget. |
| `extract` / `web_extract` | Vendor extract cascade, then SSRF-safe direct fetch. |
| `brave_search` `tavily_search` `exa_search` `linkup_search` `kagi_search` | Single-engine web search. |
| `github_search` | Repos (`kind=repo`), code (`kind=code`), users (`kind=users`). |
| `x_search` `reddit_search` `youtube_search` `instagram_search` `facebook_search` | Official social APIs. |
| `firecrawl_scrape` `firecrawl_crawl` `firecrawl_map` | Site scrape / crawl / map. |
| `get_provider_info` | Configured, cost, requires_key, notes. No secrets. |
| `search_health` | Cooldown, latency, recent errors, requires_key. |
| `quality_report` | Domain diversity and coverage diagnostics. |
| `provider_bench` | Latency / error bench across configured engines. |

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

Hit shape: `title`, `url`, `snippet`, `provider`, `score?`, `confidence?`, `published_at?`, `sources[]`, `snippet_grounded`.

Every run reports `selected`, `successful`, `failed`, `timed_out`, `skipped`, `cost_usd`, `provider_used`, `stop_reason`. Partial fan-outs are **not cached** unless `cache_partial` is set.

## Providers

Unconfigured engines are skipped. Wikipedia, Semantic Scholar, Bluesky, and Reddit public JSON work without keys.

| Provider | Env | Search | Extract | Notes |
| --- | --- | --- | --- | --- |
| **Brave (first-class)** | `BRAVE_API_KEY` | yes | — | Web + news. Default fan-out when set. |
| **GitHub (first-class)** | `GITHUB_TOKEN` or `GITHUB_API_KEY` | yes | — | Repos, code, users. Default fan-out when set. |
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

### Social API limits

- **Instagram**: official hashtag search only. Meta caps unique hashtags (30 / 7 days).
- **Facebook**: official Pages Search only. Graph has no public post keyword search.
- **X**: prefer `X_BEARER_TOKEN` on `https://api.x.com/2/tweets/search/recent`.

## What the orchestrator does

- **Parallel default** — `tokio` join across every configured engine. `mode=ladder` runs free engines first and can stop on `stop_reason=evidence`.
- **RRF + confidence** — `score += 1 / (k + rank)` (`OMNISEARCH_RRF_K`, default 60), then `confidence = 0.50·RRF + 0.25·recency + 0.25·trust`.
- **Provenance** — tracking params stripped; `sources[]` lists every contributing engine.
- **Dual-key failover** — `NAME_2` / `NAME_3` after 429 / 5xx / timeout.
- **Health cooldown** — 429 / timeouts cool an engine; `mode=auto` skips it.
- **Grounded snippets** — `ground_top` fetches the top N URLs (SSRF-safe) and reframes the snippet.
- **Intent routing** — `mode=auto` ranks by recent success and query type (news, code, users, social, scholarly, video).
- **Cost gates** — `OMNISEARCH_AUTO_ALLOW_USD` skips expensive engines unless listed in `providers[]`.
- **Cache** — in-process TTL. Partial fan-outs are not cached unless asked. `no_cache: true` bypasses.
- **Spam / diversity** — drops known shorteners; caps hits per domain, then appends overflow.
- **Freshness + news + locale** — `day` / `week` / `month` / `year`; defaults `OMNISEARCH_COUNTRY=CH`, `OMNISEARCH_LANGUAGE=de`.
- **SSRF** — extract/direct-fetch blocks loopback, link-local, private IPs, `file:`, metadata hosts.
- **Large payloads** — over `OMNISEARCH_INLINE_MAX_BYTES`, results spill to a temp file.

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Rust 1.88+. Binary and crate name: `omnisearch`. v0.1.0.
