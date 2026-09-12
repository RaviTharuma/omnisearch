# Governance

omnisearch is maintained by **Ravi Tharuma** and volunteer contributors.

## Roles

| Role | Responsibilities |
| --- | --- |
| Maintainer | Merge rights, releases, security response, roadmap priorities — all **best-effort**, no SLA |
| Contributor | Issues, PRs, docs, and reviews under [CONTRIBUTING.md](CONTRIBUTING.md) and [DISCLAIMER.md](DISCLAIMER.md) §7 |

**Important:** Maintainership does **not** create a duty to audit every line, dependency, or workflow; does **not** create liability for merged PRs; does **not** create liability for user API spend or supply-chain events; and does **not** turn free GitHub hosting of this repo into a consumer product or paid support offering. Publishing code is a courtesy under Apache-2.0. See [DISCLAIMER.md](DISCLAIMER.md) §§0–5 and §8A.

## Decision making

- Day-to-day: maintainers may merge, reject, revert, or ignore changes when CI is green and the change fits project goals — or for any other reason.
- Breaking MCP/config changes: prefer discussion in an issue before merge.
- Security: follow [SECURITY.md](SECURITY.md); security fixes may land ahead of public detail. Response is best-effort only.
- Provider priority and product direction are set by maintainers.
- **Reverts are normal.** A merge is not a permanence guarantee or a safety certification.

## Releases

- Version is the `Cargo.toml` / `Cargo.lock` package version.
- Git tags must match that version (`vX.Y.Z`) for the release workflow.
- User-visible changes belong in [CHANGELOG.md](CHANGELOG.md).
- Release artifacts and Actions are provided **without warranty**; consumers must verify checksums and review changes themselves ([DISCLAIMER.md](DISCLAIMER.md) §3.1).

## License and contribution terms

Contributions are accepted under [Apache License 2.0](LICENSE) plus the contributor warranties/indemnity in [DISCLAIMER.md](DISCLAIMER.md) §7 and the DCO terms in [CONTRIBUTING.md](CONTRIBUTING.md). Submitting a contribution constitutes agreement. There is no separate paid CLA.

## Changes to this document

Governance updates land via PR like any other docs change.
