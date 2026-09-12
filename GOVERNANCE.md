# Governance

omnisearch is maintained by **Ravi Tharuma** and volunteer contributors.

## Roles

| Role | Responsibilities |
| --- | --- |
| Maintainer | Merge rights, releases, security response, roadmap priorities |
| Contributor | Issues, PRs, docs, and reviews under [CONTRIBUTING.md](CONTRIBUTING.md) |

## Decision making

- Day-to-day: maintainers merge when CI is green and the change fits project goals.
- Breaking MCP/config changes: prefer discussion in an issue before merge.
- Security: follow [SECURITY.md](SECURITY.md); security fixes may land ahead of public detail.
- Provider priority and product direction are set by maintainers (see project context / README).

## Releases

- Version is the `Cargo.toml` / `Cargo.lock` package version.
- Git tags must match that version (`vX.Y.Z`) for the release workflow.
- User-visible changes belong in [CHANGELOG.md](CHANGELOG.md).

## License

Contributions are accepted under [Apache License 2.0](LICENSE). There is no separate CLA at this time; submitting a contribution constitutes agreement to that license terms for your submission.

## Changes to this document

Governance updates land via PR like any other docs change.
