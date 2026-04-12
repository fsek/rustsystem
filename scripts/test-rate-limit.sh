#!/usr/bin/env bash
#
# Test rate limiting on a running rustsystem instance.
#
# Fires TOTAL_REQUESTS concurrent requests at every rate-limited endpoint and
# checks that at least one 429 is returned. All endpoints share the same
# per-IP bucket (burst 30, 10/s refill), so a short pause between tests lets
# the bucket partially refill.
#
# Usage:
#   ./scripts/test-rate-limit.sh <server-url> [trustauth-url]
#
#   e.g. ./scripts/test-rate-limit.sh https://rosta.fsektionen.se https://auth.fsektionen.se

set -euo pipefail

BURST_SIZE=30
TOTAL_REQUESTS=60
REFILL_PAUSE=4   # seconds to wait between endpoint tests

# ── Endpoint list ─────────────────────────────────────────────────────────────
# Format: "<METHOD> <prefix> <path>"
# prefix is substituted with the appropriate base URL at runtime.
SERVER_ENDPOINTS=(
    "POST   SERVER /api/login"
    "POST   SERVER /api/create-meeting"
    "GET    SERVER /api/session-ids"
    "DELETE SERVER /api/host/close-meeting"
    "POST   SERVER /api/voter/submit"
    "GET    SERVER /api/common/vote-active"
    "GET    SERVER /api/common/vote-progress"
)

TRUSTAUTH_ENDPOINTS=(
    "POST   TRUSTAUTH /api/login"
    "POST   TRUSTAUTH /api/register"
    "GET    TRUSTAUTH /api/vote-data"
    "GET    TRUSTAUTH /api/is-registered"
)

# ── Args ──────────────────────────────────────────────────────────────────────
if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "Usage: $0 <server-url> [trustauth-url]" >&2
    exit 1
fi

SERVER_URL="${1%/}"
TRUSTAUTH_URL="${2:-}"

ENDPOINTS=("${SERVER_ENDPOINTS[@]}")
if [[ -n "$TRUSTAUTH_URL" ]]; then
    ENDPOINTS+=("${TRUSTAUTH_ENDPOINTS[@]}")
else
    echo "Note: no trustauth URL provided — skipping trustauth endpoints."
    echo
fi

# ── Probe function ────────────────────────────────────────────────────────────
probe_endpoint() {
    local method="$1"
    local url="$2"
    local tmpdir
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' RETURN

    for i in $(seq 1 $TOTAL_REQUESTS); do
        curl -sk -X "$method" -o /dev/null -w "%{http_code}\n" "$url" > "$tmpdir/$i" &
    done
    wait

    local ok=0 r429=0 other=0
    for f in "$tmpdir"/*; do
        local status
        status=$(cat "$f")
        if [[ $status == 429 ]]; then
            (( r429++ )) || true
        elif [[ $status -ge 100 ]]; then
            (( ok++ )) || true
        else
            (( other++ )) || true
        fi
    done

    echo "$ok $r429 $other"
}

# ── Main loop ─────────────────────────────────────────────────────────────────
PASS=0
FAIL=0
INCONCLUSIVE=0
FIRST=1

for entry in "${ENDPOINTS[@]}"; do
    read -r method prefix path <<< "$entry"

    if [[ $prefix == "SERVER" ]]; then
        base="$SERVER_URL"
    else
        base="$TRUSTAUTH_URL"
    fi
    url="${base}${path}"

    if [[ $FIRST -eq 0 ]]; then
        sleep $REFILL_PAUSE
    fi
    FIRST=0

    printf "%-8s %-50s " "$method" "$url"

    read -r ok r429 other <<< "$(probe_endpoint "$method" "$url")"

    if [[ $r429 -gt 0 ]]; then
        printf "PASS  (ok=%-2s 429=%-2s)\n" "$ok" "$r429"
        (( PASS++ )) || true
    elif [[ $ok -eq $TOTAL_REQUESTS ]]; then
        printf "FAIL  all %s requests succeeded — rate limiting may be inactive\n" "$TOTAL_REQUESTS"
        (( FAIL++ )) || true
    else
        printf "INCONCLUSIVE  (ok=%-2s 429=0 other=%-2s)\n" "$ok" "$other"
        (( INCONCLUSIVE++ )) || true
    fi
done

# ── Summary ───────────────────────────────────────────────────────────────────
total=$(( PASS + FAIL + INCONCLUSIVE ))
echo
echo "────────────────────────────────────────"
echo "Results: $PASS/$total passed, $FAIL failed, $INCONCLUSIVE inconclusive"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
