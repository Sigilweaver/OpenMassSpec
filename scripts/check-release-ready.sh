#!/usr/bin/env bash
# check-release-ready.sh - refuse to call a commit release-ready unless
# CI and the security audit have both completed successfully for it.
#
# publish.yml triggers directly on `push: tags: ["v*"]`. GitHub Actions
# has no way for one workflow file to `needs:` a job defined in another
# workflow file, so publish.yml itself cannot be made to wait on ci.yml
# or audit.yml. The gate has to live here, before the tag exists.
#
# scripts/release-stack.sh --apply runs this automatically before it
# creates/pushes the umbrella tag. Run it by hand first when tagging
# manually (see RELEASING.md step 7).
#
# Usage:
#   scripts/check-release-ready.sh [ref]   # default ref: HEAD
#
# Exit codes:
#   0 CI and audit both completed successfully for the resolved SHA
#   1 CI or audit missing, not completed, or not successful for the SHA
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

REF="${1:-HEAD}"
# `^{commit}` peels annotated tags to the commit they point at; git rev-parse
# on an annotated tag alone returns the tag object's own SHA, which never
# matches a workflow run's head SHA.
SHA="$(git -C "$REPO_DIR" rev-parse "${REF}^{commit}")"

# Query the most recent run of a workflow for $SHA and judge it. Prints
# a one-line status to stderr; returns non-zero unless the run exists,
# is completed, and succeeded.
check_workflow() {
    local workflow="$1"
    local run_json status conclusion url

    run_json="$(cd "$REPO_DIR" && gh run list -w "$workflow" -c "$SHA" \
        --json status,conclusion,url -L 1)"

    if [ "$run_json" = "[]" ]; then
        echo "[error] no run of $workflow found for $SHA" >&2
        return 1
    fi

    status="$(printf '%s' "$run_json" | jq -r '.[0].status')"
    conclusion="$(printf '%s' "$run_json" | jq -r '.[0].conclusion')"
    url="$(printf '%s' "$run_json" | jq -r '.[0].url')"

    if [ "$status" != "completed" ]; then
        echo "[error] $workflow for $SHA is not completed (status: $status) - $url" >&2
        return 1
    fi
    if [ "$conclusion" != "success" ]; then
        echo "[error] $workflow for $SHA did not succeed (conclusion: $conclusion) - $url" >&2
        return 1
    fi

    echo "[ok] $workflow green for $SHA - $url" >&2
    return 0
}

ready=1
check_workflow ci.yml || ready=0
check_workflow audit.yml || ready=0

if [ "$ready" -ne 1 ]; then
    echo "[error] $SHA is not release-ready" >&2
    exit 1
fi

echo "[ok] $SHA is release-ready: CI and audit are both green" >&2
exit 0
