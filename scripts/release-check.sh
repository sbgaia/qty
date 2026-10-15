#!/usr/bin/env bash
# Checks that a release tag matches the crate version and that CHANGELOG.md
# has a dated section for it; prints that section (the release notes).
#
#   ./scripts/release-check.sh v0.1.0
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${1:?usage: $0 vX.Y.Z}"
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
if [ "$tag" != "v$version" ]; then
  echo "error: tag $tag does not match the version in Cargo.toml ($version)" >&2
  exit 1
fi

# The section `## [X.Y.Z] - YYYY-MM-DD`, up to the next `## [` heading.
notes=$(awk -v v="$version" '
  index($0, "## [" v "] - ") == 1 { found = 1; next }
  found && /^## \[/ { exit }
  found { print }
' CHANGELOG.md)
if ! grep -qE "^## \[$version\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$" CHANGELOG.md || [ -z "${notes//[[:space:]]/}" ]; then
  echo "error: CHANGELOG.md has no dated, non-empty section '## [$version] - YYYY-MM-DD'" >&2
  exit 1
fi
printf '%s\n' "$notes"
