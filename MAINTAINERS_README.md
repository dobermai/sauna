# Maintainers Guide

## Release Process

### Prerequisites

1. Install cargo-release:
   ```bash
   cargo install cargo-release
   ```

2. Ensure `CARGO_REGISTRY_TOKEN` secret is configured in GitHub repository settings:
   - Go to https://github.com/dobermai/sauna/settings/secrets/actions
   - Add secret with your [crates.io API token](https://crates.io/settings/tokens)

### Creating a Release

Run one of the following commands depending on the type of release:

```bash
cargo release patch  # 0.1.0 -> 0.1.1 (bug fixes)
cargo release minor  # 0.1.0 -> 0.2.0 (new features)
cargo release major  # 0.1.0 -> 1.0.0 (breaking changes)
```

This automatically:
1. Bumps the version in `Cargo.toml`
2. Creates a commit "Release vX.Y.Z"
3. Creates a git tag `vX.Y.Z`
4. Pushes commit and tag to GitHub

GitHub Actions then:
1. Verifies the tag matches `Cargo.toml` version
2. Publishes `klafs-api` to crates.io

### Dry Run

To preview what will happen without making changes:

```bash
cargo release patch --dry-run
```

### What Gets Published

Only the `klafs-api` library crate is published to crates.io. The `sauna` CLI is not published.
