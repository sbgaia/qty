#!/usr/bin/env bash
# Moves the pinned UnitsNet commit (scripts/download.sh) to the latest
# upstream master, downloads the definitions, regenerates the code and runs
# the tests. Prints a Markdown summary on stdout, for a pull request.
#
# In GitHub Actions it also sets the step outputs `changed` (true/false) and
# `short` (the new commit, abbreviated).
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

output() { [ -n "${GITHUB_OUTPUT:-}" ] && echo "$1=$2" >> "$GITHUB_OUTPUT"; return 0; }

# The pattern matches the literal `${UNITSNET_REF:-<sha>}` in the script.
# shellcheck disable=SC2016
current=$(sed -n 's/^UNITSNET_REF="\${UNITSNET_REF:-\([0-9a-f]\{40\}\)}"$/\1/p' scripts/download.sh)
if [ -z "$current" ]; then
  echo "error: no pinned commit found in scripts/download.sh" >&2
  exit 1
fi
latest=$(git ls-remote https://github.com/angularsen/UnitsNet refs/heads/master | cut -f1)
if [ -z "$latest" ]; then
  echo "error: could not read UnitsNet's latest commit" >&2
  exit 1
fi
if [ "$current" = "$latest" ]; then
  echo "UnitsNet is up to date (${current:0:12})."
  output changed false
  exit 0
fi
output changed true
output short "${latest:0:12}"

sed -i "s/$current/$latest/" scripts/download.sh
log=$(mktemp)

echo "Updates the UnitsNet definitions from \`${current:0:12}\` to \`${latest:0:12}\`"
echo "([upstream changes](https://github.com/angularsen/UnitsNet/compare/$current...$latest))."
echo

if ! ./scripts/download.sh --current > "$log" 2>&1; then
  echo "**Downloading the definitions failed:**"
  echo; echo '```'; tail -n 20 "$log"; echo '```'
  exit 0
fi

if ! cargo run -q -p codegen > /dev/null 2> "$log"; then
  echo "**The code generator failed.** The new definitions need a change to the generator:"
  echo; echo '```'; tail -n 30 "$log"; echo '```'
  exit 0
fi
notes=$(grep '^codegen: note' "$log" | sed 's/^codegen: note: /- /')
if [ -n "$notes" ]; then
  echo "Generator notes (new quantities fall into the \`misc\` feature group until"
  echo "they are added to a group in \`codegen/src/generator.rs\`):"
  echo
  echo "$notes"
  echo
fi

echo "Changed files: $(git status --porcelain -- src tests | wc -l)."
echo
if cargo test -q -p qty --all-features > "$log" 2>&1; then
  echo "\`cargo test --all-features\` passes."
else
  echo "**\`cargo test --all-features\` fails:**"
  echo; echo '```'; grep -E "^(error|test result)|FAILED|panicked" -A 4 "$log" | head -n 60; echo '```'
fi
