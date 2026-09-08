# Named accounts and OmniRoute gateways

## Provider accounts

`OMNISEARCH_ACCOUNTS` is a JSON array. Names are unique per provider.

```sh
OMNISEARCH_ACCOUNTS='[{"name":"work","provider":"tavily","credentials":{"api_key":"replace-work"}},{"name":"personal","provider":"tavily","credentials":{"api_key":"replace-personal"}}]'
```

Use any existing provider ID. Each account creates an independent adapter with its own key ring; accounts rotate round-robin and failures cool down only that account. Named accounts **replace** the legacy credentials for that provider; providers absent from the array retain their existing environment/key-list behavior. Individual attempts share the provider timeout budget (one extra share is reserved for scheduling). Pagination stays bound to the originating account. `account_health` exposes names, counters, latency and remaining cooldown without credentials; existing `search_health` is unchanged.

Each account contains `provider`, `name`, and a `credentials` object. Credential shapes: `{ "api_key": "..." }` for key-based adapters; `{ "bearer_token": "..." }` or `{ "xai_api_key": "..." }` for X; `{ "client_id": "...", "client_secret": "..." }` for Reddit; `{ "token": "...", "user_id": "..." }` for Instagram; `{ "token": "..." }` for Facebook; `{ "instance": "https://mastodon.example", "token": "..." }` for Mastodon; `{ "url": "https://mcp.example/mcp", "token": "..." }` for MCP backends. Public Reddit/Wikipedia/DuckDuckGo/Bluesky use `{}`. Unsupported combinations are rejected rather than silently ignored. Endpoints remain provider-level configuration. Provider aliases and legacy key names remain unchanged.

Supported existing authentication only: Reddit client ID + client secret uses its existing OAuth client-credentials flow; Reddit bearer-token injection is not implemented. X uses bearer tokens or existing xAI API keys. Mastodon uses instance URL + optional bearer token; Bluesky currently uses public search; unused legacy login variables are not promoted to a working authentication flow. Other adapters use their existing API-key/token mechanism. No new OAuth browser-login or refresh-token implementation is claimed. Never put real credentials in source control.

## OmniRoute

Flow: MCP client → OmniSearch → OmniRoute `/v1/search` → provider credentials held by OmniRoute. No upstream provider keys are needed in OmniSearch.

```sh
OMNISEARCH_OMNIROUTE_GATEWAYS='[{"name":"primary","base_url":"http://127.0.0.1:20128","api_key":"replace-gateway-key","provider":"brave-search","estimated_search_usd":0.01},{"name":"backup","base_url":"https://route.example/v1","api_key":"replace-backup-key"}]'
```

The base URL may be a server root or end in `/v1`. OmniSearch sends `Authorization: Bearer <gateway key>` to POST `/v1/search`, with `query`, `max_results` (1–20), `search_type` (`web` by default, optionally `news`), and optional OmniRoute provider ID. Omit `provider` to let OmniRoute route. OmniRoute provider identifiers are **not** OmniSearch identifiers (for example `brave-search`, not `brave`). The normalized response's `results` is required; title, URL, optional snippet and published date become MCP search hits. Provider-native passthrough endpoints, internal admin endpoints and chat-completions are not used.

Use `providers: ["omniroute"]` with `search` to restrict calls to gateways; it also participates in normal configured-provider selection and auto intent routing. Multiple connections rotate and fail over independently. Gateway cost defaults to a conservative **estimate** of USD 0.01/request, not free; override `estimated_search_usd` for your route. It is not an actual billing report. Requests are limited to 500 characters; this normalized endpoint has no cursor pagination or extraction. Search type is connection-configured because OmniSearch verticals and OmniRoute `web/news` are different contracts.

Local/private gateway addresses are deliberately supported as trusted operator configuration, not user-supplied extraction URLs. URL credentials, fragments and query strings are rejected. Use HTTPS for remote gateways. Errors never echo gateway response bodies or credentials. Existing HTTP client follows no redirects. Invalid config fails startup before requests.

Contract source: `diegosouzapw/OmniRoute`, `src/app/api/v1/search/route.ts`, `src/shared/validation/schemas/apiV1.ts`, `open-sse/handlers/search.ts`. Gateway authentication is OmniRoute API-key authentication; upstream OAuth, where applicable, remains OmniRoute's responsibility.
