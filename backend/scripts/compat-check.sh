#!/usr/bin/env bash
# Backward-compat container harness: pulls the currently-deployed server image,
# boots it against an already-migrated database, and polls liveness + readiness
# to prove the old binary can still speak the new schema during a deploy-window
# coexistence period.
#
# Called from:
#   - .github/workflows/server-ci.yml (PR-time, gated on migration file changes)
#   - .github/workflows/release.yml migration-branch-test (release-time, final gate)
#
# Local dry-run:
#   bash backend/scripts/compat-check.sh "$DATABASE_URL" "ghcr.io/scanopy/scanopy/server:v0.14.0"
#
# Exit codes:
#   0 — all checks passed, OR prior image unavailable (first-release fallback),
#       OR readiness returned 404 (prior image predates the readiness endpoint).
#   1 — liveness failed, readiness 5xx, container exited, or other hard failure.
#
# When the script prints a line beginning with COMPAT_STATUS=, the caller is
# expected to tee it into $GITHUB_STEP_SUMMARY so the outcome is visible in the
# GitHub Actions UI.

set -euo pipefail

DB_URL="${1:-}"
IMAGE_REF="${2:-}"

if [ -z "$DB_URL" ] || [ -z "$IMAGE_REF" ]; then
    echo "Usage: $0 <DATABASE_URL> <IMAGE_REF>" >&2
    exit 2
fi

CONTAINER_NAME="scanopy-compat-$$"
HOST_PORT="${COMPAT_HOST_PORT:-60072}"
LIVENESS_BUDGET="${COMPAT_LIVENESS_BUDGET:-30}"
READINESS_BUDGET="${COMPAT_READINESS_BUDGET:-30}"
POLL_INTERVAL="${COMPAT_POLL_INTERVAL:-2}"

cleanup() {
    local rc=$?
    echo
    echo "=== Cleanup ==="
    if docker inspect "$CONTAINER_NAME" >/dev/null 2>&1; then
        if [ "$rc" -ne 0 ]; then
            echo "--- Container logs (last 200 lines) ---"
            docker logs "$CONTAINER_NAME" 2>&1 | tail -200 || true
            echo "--- End container logs ---"
        fi
        docker stop "$CONTAINER_NAME" >/dev/null 2>&1 || true
        docker rm "$CONTAINER_NAME" >/dev/null 2>&1 || true
        echo "  container $CONTAINER_NAME stopped + removed"
    fi
    exit "$rc"
}
trap cleanup EXIT INT TERM

echo "=== Pulling previous release image: $IMAGE_REF ==="
if ! docker pull "$IMAGE_REF" 2>&1; then
    echo
    echo "COMPAT_STATUS=SKIPPED: prior image $IMAGE_REF not available in registry (first release, or retention-pruned). Backward-compat check skipped."
    # Disarm the trap — we have no container to clean up and don't want to
    # re-enter cleanup with a zero rc that might surprise callers.
    trap - EXIT
    exit 0
fi

echo
echo "=== Connectivity probe: can a bridge-network container reach the DB? (diagnostic) ==="
# The host runner already reached the DB (the warm-compute SELECT 1 in
# release.yml succeeded). This probes whether a *container* on the default
# bridge network can too — if not, that's why the prior-release server
# container's pool acquire times out. Non-fatal: prints a signal, never fails.
set +e
timeout 25 docker run --rm postgres:16-alpine \
    psql "$DB_URL" -c 'SELECT 1' >/dev/null 2>&1
probe_rc=$?
set -e
if [ "$probe_rc" -eq 0 ]; then
    echo "  bridge-network container reached the DB OK"
else
    echo "  bridge-network container FAILED to reach the DB (rc=$probe_rc) — points to container egress/DNS; server now runs with --network host below"
fi

echo
echo "=== Starting container against migrated database ==="
# Share the runner's network stack (host networking) instead of the default
# bridge: the runner can reach the DB (warmup succeeded) but a bridge-network
# container may not, which surfaces as a pool acquire timeout at startup.
# Under --network host the -p mapping is ignored, so the server binds 60072
# directly on the host and the localhost polls below still work.
docker run -d \
    --name "$CONTAINER_NAME" \
    --network host \
    -e DATABASE_URL="$DB_URL" \
    "$IMAGE_REF" >/dev/null

echo "  container: $CONTAINER_NAME"
echo "  port: $HOST_PORT"

poll_status() {
    # Prints the HTTP status code, or "000" on connection failure.
    curl --silent --output /dev/null --write-out '%{http_code}' \
        --max-time 3 \
        "http://localhost:${HOST_PORT}$1" 2>/dev/null || echo "000"
}

container_running() {
    [ "$(docker inspect --format='{{.State.Running}}' "$CONTAINER_NAME" 2>/dev/null || echo false)" = "true" ]
}

echo
echo "=== Polling /api/health (liveness, budget ${LIVENESS_BUDGET}s) ==="
elapsed=0
while [ "$elapsed" -lt "$LIVENESS_BUDGET" ]; do
    if ! container_running; then
        echo "COMPAT_STATUS=FAIL: container exited during startup — see logs above."
        exit 1
    fi
    status=$(poll_status /api/health)
    if [ "$status" = "200" ]; then
        echo "  ✓ /api/health 200 after ${elapsed}s"
        break
    fi
    sleep "$POLL_INTERVAL"
    elapsed=$((elapsed + POLL_INTERVAL))
done
if [ "$status" != "200" ]; then
    echo "COMPAT_STATUS=FAIL: /api/health did not return 200 within ${LIVENESS_BUDGET}s (last status: $status)."
    exit 1
fi

echo
echo "=== Polling /api/health/ready (readiness, budget ${READINESS_BUDGET}s) ==="
elapsed=0
while [ "$elapsed" -lt "$READINESS_BUDGET" ]; do
    if ! container_running; then
        echo "COMPAT_STATUS=FAIL: container exited during readiness polling — see logs above."
        exit 1
    fi
    status=$(poll_status /api/health/ready)
    case "$status" in
        200)
            echo "  ✓ /api/health/ready 200 after ${elapsed}s"
            echo
            echo "COMPAT_STATUS=PASS: previous release binary can execute SQL against migrated schema."
            exit 0
            ;;
        404)
            echo "  ℹ /api/health/ready 404 — prior image predates the readiness endpoint."
            echo
            echo "COMPAT_STATUS=PARTIAL: prior image has no readiness endpoint (first release after backward-compat check landed). Downgraded to process-stayed-up check; /api/health succeeded. Subsequent releases will exercise the full DB probe."
            exit 0
            ;;
        5??|500|502|503|504)
            echo "COMPAT_STATUS=FAIL: /api/health/ready returned $status — previous binary cannot execute SQL against migrated schema. See logs above."
            exit 1
            ;;
    esac
    sleep "$POLL_INTERVAL"
    elapsed=$((elapsed + POLL_INTERVAL))
done

echo "COMPAT_STATUS=FAIL: /api/health/ready did not return a final status within ${READINESS_BUDGET}s (last status: $status)."
exit 1
