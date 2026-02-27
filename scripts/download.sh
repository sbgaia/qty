#!/usr/bin/env bash
# Download unit definition JSON files from the UnitsNet repository.
#
# Usage:
#   ./scripts/download.sh              # download the default set
#   ./scripts/download.sh --all        # download every quantity definition
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
BASE_URL="https://raw.githubusercontent.com/angularsen/UnitsNet/master/Common/UnitDefinitions"
API_URL="https://api.github.com/repos/angularsen/UnitsNet/contents/Common/UnitDefinitions"
RELATIONS_URL="https://raw.githubusercontent.com/angularsen/UnitsNet/master/Common/UnitRelations.json"

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
EXPLICIT=()

for arg in "$@"; do
    case "$arg" in
        --all) ALL=true ;;
        --help|-h)
            sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
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
echo "Downloaded: ${ok}  Failed: ${fail}"

if [[ $fail -gt 0 ]]; then
    exit 1
fi
