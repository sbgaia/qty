#!/usr/bin/env bash
# Download unit definition JSON files from the UnitsNet repository.
#
# Usage:
#   ./scripts/download.sh              # download the default set
#   ./scripts/download.sh --all        # download every quantity definition
#   ./scripts/download.sh --current    # re-download the quantities in src/units/
#   ./scripts/download.sh Length Mass  # download named quantities
#
# Quantity definitions are saved to data/quantities/ and the curated
# cross-type relations file is saved to data/relations/, both relative
# to the repo root.
# Existing files are overwritten so the directories always reflect the
# latest upstream definitions.

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────────────

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QUANTITIES_DIR="${REPO_ROOT}/data/quantities"
RELATIONS_DIR="${REPO_ROOT}/data/relations"
# The UnitsNet commit the generated code is built from. Pinned so that
# regenerating is reproducible (and CI can check that the committed code is
# up to date). To upgrade: set a new commit here, run
# `./scripts/download.sh --current && cargo run -p codegen`, and review the diff.
# Override for one run with `UNITSNET_REF=<commit-or-branch> ./scripts/download.sh`.
UNITSNET_REF="${UNITSNET_REF:-3bf1ef4b3c4d82c05c7a2667b25e0867358f9a62}"

BASE_URL="https://raw.githubusercontent.com/angularsen/UnitsNet/${UNITSNET_REF}/Common/UnitDefinitions"
API_URL="https://api.github.com/repos/angularsen/UnitsNet/contents/Common/UnitDefinitions?ref=${UNITSNET_REF}"
RELATIONS_URL="https://raw.githubusercontent.com/angularsen/UnitsNet/${UNITSNET_REF}/Common/UnitRelations.json"

# The subset used by this project (kept in sync with codegen).
DEFAULT_QUANTITIES=(
    Duration
    Energy
    Length
    Mass
    Speed
    Temperature
)

# ── Argument parsing ───────────────────────────────────────────────────────────

ALL=false
CURRENT=false
EXPLICIT=()

for arg in "$@"; do
    case "$arg" in
        --all) ALL=true ;;
        --current) CURRENT=true ;;
        --help|-h)
            sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) EXPLICIT+=("$arg") ;;
    esac
done

# ── Resolve the list of quantities to download ────────────────────────────────

if $ALL; then
    echo "Fetching full quantity list from GitHub API…"
    if ! command -v curl &>/dev/null; then
        echo "error: curl is required" >&2; exit 1
    fi
    QUANTITIES=()
    while IFS= read -r name; do
        QUANTITIES+=("$name")
    done < <(
        curl -fsSL "$API_URL" \
          -H "Accept: application/vnd.github+json" \
          | grep -oP '"name":\s*"\K[^"]+(?=\.json")' \
          | sort
    )
    echo "Found ${#QUANTITIES[@]} quantities."
elif $CURRENT; then
    # The generated files record their source definition; no API call needed
    # (the GitHub API is rate-limited for unauthenticated requests).
    QUANTITIES=()
    while IFS= read -r name; do
        QUANTITIES+=("$name")
    done < <(
        grep -h '^// Source: data/quantities/' "${REPO_ROOT}"/src/units/*.rs \
          | sed 's#.*/##; s#\.json$##' \
          | sort
    )
    echo "Found ${#QUANTITIES[@]} quantities in src/units/."
elif [[ ${#EXPLICIT[@]} -gt 0 ]]; then
    QUANTITIES=("${EXPLICIT[@]}")
else
    QUANTITIES=("${DEFAULT_QUANTITIES[@]}")
fi

# ── Download ──────────────────────────────────────────────────────────────────

mkdir -p "$QUANTITIES_DIR" "$RELATIONS_DIR"

ok=0
fail=0

# Download the curated cross-type relations file into its own directory.
printf "  %-40s" "relations/UnitRelations.json"
if curl -fsSL "$RELATIONS_URL" -o "${RELATIONS_DIR}/UnitRelations.json"; then
    echo "ok"
    (( ok++ )) || true
else
    echo "FAILED"
    (( fail++ )) || true
fi

for qty in "${QUANTITIES[@]}"; do
    url="${BASE_URL}/${qty}.json"
    dest="${QUANTITIES_DIR}/${qty}.json"
    printf "  %-40s" "quantities/${qty}.json"
    if curl -fsSL "$url" -o "$dest"; then
        echo "ok"
        (( ok++ )) || true
    else
        echo "FAILED (HTTP error or not found)"
        (( fail++ )) || true
    fi
done

echo ""
echo "UnitsNet ref: ${UNITSNET_REF}"
echo "Downloaded: ${ok}  Failed: ${fail}"

if [[ $fail -gt 0 ]]; then
    exit 1
fi
