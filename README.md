# omnisearch

**Search MCP** for **parallel** multi-engine web **search** — so your AI finds what a single search (or the model alone) usually misses.

One [MCP](https://modelcontextprotocol.io) server for Claude, Cursor, and any MCP client. Drop in the API keys you already have.

**Why people use it**

- **Search + MCP** — web search as a first-class tool in your agent setup
- **Parallel multi-search** — several engines at once, not one at a time
- **Better coverage** — merges and dedupes so you see the overlap *and* the unique hits
- **More than “just Google”** — web, news, code, papers, and social in one place
- **Research mode** — search, then pull the best pages when you need depth
- **No tiny result ceiling** — optional limits for cost/time; not a hard “top 5” product cap

Works with Tavily, Exa, Brave, Firecrawl, Linkup, Perplexity, and more. Missing keys are simply skipped.

License: Apache-2.0 · Copyright 2026 Ravi Tharuma

## Quick start

```bash
git clone https://github.com/RaviTharuma/omnisearch.git
cd omnisearch
cargo install --path .
```

Copy `.env.example` to `.env` and add any keys you have.

```bash
omnisearch          # MCP over stdio (default)
omnisearch http     # optional HTTP transport
```

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

Then connect to `http://127.0.0.1:48731/mcp` with `Authorization: Bearer replace-me`.

## What you can do

| Tool | What it does |
| --- | --- |
| `search` | Parallel multi-engine search (default). Auto mode picks engines from the query. |
| `ai_search` | Answer-oriented search |
| `research` | Search, then extract top pages |
| `extract` / `web_extract` | Pull clean content from URLs |
| `tavily_search` `exa_search` `linkup_search` `brave_search` `kagi_search` | Single-engine search |
| `x_search` `reddit_search` `youtube_search` `instagram_search` `facebook_search` | Social |
| `github_search` | Repos or code |
| `firecrawl_scrape` `firecrawl_crawl` `firecrawl_map` | Site scrape / crawl / map |
| `get_provider_info` | Which engines are configured (no secrets) |
| `search_health` | Live cooldown, latency, recent errors |
| `quality_report` | Coverage / diversity diagnostics |
| `provider_bench` | Quick latency check across engines |

Example `search` call:

```json
{
  "query": "Swiss AI regulation 2026",
  "providers": ["tavily", "exa", "brave"],
  "search_type": "news",
  "freshness": "week",
  "country": "CH",
  "language": "de"
}
```

Each hit includes title, URL, snippet, and which engines found it (`sources[]`). If some engines fail, you still get results from the ones that worked.

## Engines

Add the keys you have — everything else is ignored. Wikipedia, Semantic Scholar, Bluesky, and Reddit public search work with no key.

| Engine | Env | Search | Extract |
| --- | --- | --- | --- |
| Tavily | `TAVILY_API_KEY` | yes | yes |
| Exa | `EXA_API_KEY` | yes | yes |
| Firecrawl | `FIRECRAWL_API_KEY` | yes | yes |
| Linkup | `LINKUP_API_KEY` | yes | — |
| Brave | `BRAVE_API_KEY` | yes | — |
| Kagi | `KAGI_API_KEY` | yes | — |
| Perplexity | `PERPLEXITY_API_KEY` | yes | — |
| You.com | `YOU_API_KEY` or `YDC_API_KEY` | yes | yes |
| Parallel.ai | `PARALLEL_API_KEY` | yes | yes |
| Querit | `QUERIT_API_KEY` | yes | yes |
| TinyFish | `TINYFISH_API_KEY` | yes | — |
| Keenable | `KEENABLE_API_KEY` or `KEENABLE_PUBLIC=true` | yes | keyed |
| GitHub | `GITHUB_TOKEN` | yes | — |
| X | `X_BEARER_TOKEN` or `XAI_API_KEY` | yes | — |
| Reddit | none, or OAuth pair | yes | — |
| YouTube | `YOUTUBE_API_KEY` or `GOOGLE_API_KEY` | yes | — |
| Instagram | Graph token + business account | hashtag | — |
| Facebook | `FACEBOOK_ACCESS_TOKEN` | pages | — |
| Wikipedia | none | yes | — |
| Scholar | none | yes | — |
| Mastodon | `MASTODON_INSTANCE` | yes | — |
| Bluesky | none | yes | — |
| Extra MCP backends | `OMNISEARCH_MCP_BACKENDS` | yes | — |

`OMNISEARCH_MCP_BACKENDS=official` can attach remote MCP endpoints for Tavily, Exa, Firecrawl, Linkup, Kagi, and Perplexity when keys are present. See `.env.example`.

### Social API limits

- **Instagram**: official hashtag search only (Meta rate limits apply).
- **Facebook**: official Pages Search only.
- **X**: prefer `X_BEARER_TOKEN`; `XAI_API_KEY` uses xAI’s `x_search` path.

## How it works (optional detail)

- Runs configured engines **in parallel**, then merges and dedupes by URL/title
- Keeps provenance (`sources[]`) so you know who found each hit
- Retries / cools down after rate limits and outages; auto mode skips unhealthy engines
- Optional budgets for time, width, and estimated cost — not a tiny fixed result cap
- Optional grounded snippets, free-first ladder routing, dual-key failover, and cost metadata
- Freshness filters and locale defaults (`OMNISEARCH_COUNTRY`, `OMNISEARCH_LANGUAGE`)
- Safe extract path (blocks private/local addresses)

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## Version

v0.1.0
