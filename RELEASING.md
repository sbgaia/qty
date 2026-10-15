# Releasing

Releases are published to crates.io by the `Release` workflow when a version
tag is pushed.

## Once: set up publishing

1. On crates.io, create an API token (Account Settings → API Tokens) with the
   `publish-new` and `publish-update` scopes.
2. In the GitHub repository, add it as the secret `CARGO_REGISTRY_TOKEN`
   (Settings → Secrets and variables → Actions).

After the first release you can switch to crates.io's trusted publishing
instead of a long-lived token.

## Each release

1. Choose the version. Before 1.0, a breaking change bumps the minor version
   (0.1 → 0.2) and anything else the patch version. The `Semver` CI job
   compares the code with the latest published version and fails on a
   breaking change whose version was not bumped.
2. In a pull request:
   - set `version` in `Cargo.toml`;
   - in `CHANGELOG.md`, rename `## [Unreleased]` to
     `## [X.Y.Z] - YYYY-MM-DD` and add a new, empty `## [Unreleased]` above it.
3. Merge it once CI passes, then tag the merge commit on `master`:

   ```sh
   git checkout master && git pull
   ./scripts/release-check.sh vX.Y.Z   # checks the tag, version and changelog
   git tag vX.Y.Z && git push origin vX.Y.Z
   ```

The workflow checks the tag again, runs `cargo publish` (which builds the
packaged crate on its own) and creates a GitHub release whose notes are the
changelog section.

## Updating UnitsNet

Every Monday the `UnitsNet update` workflow checks for new UnitsNet commits
and, if there are any, opens a pull request that moves the pin in
`scripts/download.sh` and regenerates the code, with a summary of the test
results. You can also run it by hand from the Actions tab, or locally with
`./scripts/update-unitsnet.sh`.
