# Disclaimer

**omnisearch** is an independent open-source project. It is provided for convenience so AI clients and developers can query multiple search, social, and extract APIs through one MCP (or optional HTTP) interface.

## No affiliation

omnisearch is **not** affiliated with, endorsed by, or sponsored by any third-party search, social, extract, or AI vendor whose APIs or public endpoints it may call. Product and company names are used only to identify interoperable services. All trademarks remain the property of their respective owners.

## Your keys, your accounts, your compliance

You supply API keys, tokens, and account credentials. You are solely responsible for:

- Complying with each provider’s terms of service, acceptable use policy, rate limits, and regional restrictions
- Paying any usage charges those providers bill to your accounts
- Keeping secrets out of source control, logs, screenshots, and public issues
- Obtaining any rights or consents required for the queries you run and the content you retrieve

The maintainers do not operate or control third-party APIs. Provider outages, pricing changes, API breaks, or account suspensions are outside the scope of this project.

## No warranty; results are not advice

THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, AND NONINFRINGEMENT. See the [Apache License 2.0](LICENSE) for the full legal terms.

Search and extract results may be incomplete, outdated, biased, duplicated, or wrong. omnisearch does **not** guarantee ranking quality, coverage, freshness, or factual accuracy. Do not treat output as legal, medical, financial, or other professional advice. Verify important claims against primary sources.

## Security and privacy

- Queries and URLs you send through omnisearch are forwarded to the providers you have configured (and, when research/extract is used, to destination sites). Assume third parties can see those requests.
- Do not put secrets, personal data you are not allowed to process, or regulated data into queries unless you have a lawful basis and the provider allows it.
- Optional HTTP mode is intended for trusted local or private networks. You must set strong `AUTH_TOKENS` (or equivalent) and not expose an unauthenticated listener to the public internet.
- Built-in SSRF protections reduce risk when fetching pages; they are not a substitute for deploying behind your own network controls.

## Cost and rate limits

Parallel fan-out can call many paid APIs at once. Misconfiguration, high `limit` / `unlimited`, or research/extract loops can incur material cost. Budget env vars and tool arguments help but do not eliminate risk. Monitor provider dashboards.

## Liability

TO THE MAXIMUM EXTENT PERMITTED BY LAW, THE AUTHORS AND COPYRIGHT HOLDERS ARE NOT LIABLE FOR ANY CLAIM, DAMAGES, OR OTHER LIABILITY ARISING FROM USE OF THE SOFTWARE, INCLUDING LOST DATA, LOST PROFITS, API BILLS, ACCOUNT BANS, OR RELIANCE ON SEARCH RESULTS. See [LICENSE](LICENSE).

## Reporting concerns

- Security vulnerabilities: see [SECURITY.md](SECURITY.md)
- General questions and bugs: see [SUPPORT.md](SUPPORT.md) and [CONTRIBUTING.md](CONTRIBUTING.md)
