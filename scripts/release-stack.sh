#!/usr/bin/env bash
# release-stack.sh - coordinated stack release helper for OpenMassSpec.
#
# Reads pinned versions across the five-repo stack (this repo +
# OpenMassSpecCore, OpenTFRaw, OpenTimsTDF, OpenWRaw), emits a release-notes
# draft aggregated from each repo's CHANGELOG.md, and optionally creates
# and pushes an annotated umbrella SemVer tag on this repo.
#
# Usage:
#   scripts/release-stack.sh                       # dry-run, notes to stdout
#   scripts/release-stack.sh --name v0.1.0         # set umbrella tag name
#   scripts/release-stack.sh --tag --apply         # create tag locally
#   scripts/release-stack.sh --tag --push --apply  # also push to origin
#
# Flags:
#   --name <vX.Y.Z>     Umbrella tag name (default: read from
#                       crates/openmassspec-io-cli/Cargo.toml as v<version>).
#   --tag               Create an annotated tag on HEAD of this repo.
#   --push              Push the created tag to origin (implies --tag).
#   --apply             Required for any mutating action. Without it,
#                       the script is dry-run only.
#   --write-stack-md    Overwrite STACK.md with the current pin table.
#   --gate-speclance     Run scripts/truthtest-speclance.sh before tagging.
#                       Aborts tag creation on non-zero exit. With
#                       --apply this gate is enforced; without --apply
#                       only the gate command is reported.
#   --gate-with-corpus  Pass --with-corpus to the SpecLance gate. Implies
#                       --gate-speclance.
#   --skip-release-check  Bypass the CI/audit release-readiness check
#                       (scripts/check-release-ready.sh) that otherwise
#                       runs automatically before tagging under --apply.
#   --help              Show this help.
#
# Exit codes:
#   0 success / dry-run completed
#   1 unexpected error
#   2 dirty working tree in one of the repos
#   3 missing sibling repo
#   4 SpecLance truth-test gate failed (under --gate-speclance --apply)
#   5 release-readiness check failed (CI/audit not green; under --apply,
#     unless --skip-release-check)
set -euo pipefail

# Locations - this script lives in OpenMassSpec/scripts/.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UMBRELLA_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECTS_DIR="$(cd "$UMBRELLA_DIR/.." && pwd)"

REPOS=(
    "OpenMassSpec"
    "OpenMassSpecCore"
    "OpenTFRaw"
    "OpenTimsTDF"
    "OpenWRaw"
    "OpenARaw"
    "OpenSXRaw"
    "OpenSZRaw"
)

# Pretty names + version-file locations for each repo.
declare -A VERSION_FILE
VERSION_FILE[OpenMassSpec]="Cargo.toml"
VERSION_FILE[OpenMassSpecCore]="Cargo.toml"
VERSION_FILE[OpenTFRaw]="Cargo.toml"
VERSION_FILE[OpenTimsTDF]="Cargo.toml"
VERSION_FILE[OpenWRaw]="Cargo.toml"
VERSION_FILE[OpenARaw]="Cargo.toml"
VERSION_FILE[OpenSXRaw]="Cargo.toml"
VERSION_FILE[OpenSZRaw]="Cargo.toml"

declare -A DISPLAY_NAME
DISPLAY_NAME[OpenMassSpec]="OpenMassSpec (umbrella)"
DISPLAY_NAME[OpenMassSpecCore]="openmassspec-core"
DISPLAY_NAME[OpenTFRaw]="opentfraw"
DISPLAY_NAME[OpenTimsTDF]="opentimstdf"
DISPLAY_NAME[OpenWRaw]="openwraw"
DISPLAY_NAME[OpenARaw]="openaraw"
DISPLAY_NAME[OpenSXRaw]="opensxraw"
DISPLAY_NAME[OpenSZRaw]="openszraw"

# Defaults.
TAG_NAME=""
DO_TAG=0
DO_PUSH=0
DO_APPLY=0
WRITE_STACK_MD=0
DO_GATE=0
GATE_WITH_CORPUS=0
SKIP_RELEASE_CHECK=0

usage() {
    sed -n '2,36p' "$0" | sed 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
    case "$1" in
        --name) TAG_NAME="$2"; shift 2 ;;
        --tag) DO_TAG=1; shift ;;
        --push) DO_PUSH=1; DO_TAG=1; shift ;;
        --apply) DO_APPLY=1; shift ;;
        --write-stack-md) WRITE_STACK_MD=1; shift ;;
        --gate-speclance) DO_GATE=1; shift ;;
        --gate-with-corpus) DO_GATE=1; GATE_WITH_CORPUS=1; shift ;;
        --skip-release-check) SKIP_RELEASE_CHECK=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) echo "Unknown flag: $1" >&2; usage >&2; exit 1 ;;
    esac
done

read_version() {
    # First `version = "X.Y.Z"` line in a Cargo.toml.
    local file="$1"
    grep -m1 -E '^version\s*=\s*"' "$file" \
        | sed -E 's/^version\s*=\s*"([^"]+)".*/\1/'
}

repo_short_sha() {
    git -C "$1" rev-parse --short HEAD
}

repo_is_clean() {
    local dir="$1"
    [ -z "$(git -C "$dir" status --porcelain --untracked-files=no)" ]
}

# Collect pins.
declare -A VERSIONS
declare -A SHAS
DIRTY_REPOS=()

for repo in "${REPOS[@]}"; do
    dir="$PROJECTS_DIR/$repo"
    if [ ! -d "$dir/.git" ]; then
        echo "Missing repo: $dir" >&2
        exit 3
    fi
    VERSIONS[$repo]="$(read_version "$dir/${VERSION_FILE[$repo]}")"
    SHAS[$repo]="$(repo_short_sha "$dir")"
    if ! repo_is_clean "$dir"; then
        DIRTY_REPOS+=("$repo")
    fi
done

# Resolve umbrella tag name.
if [ -z "$TAG_NAME" ]; then
    TAG_NAME="v${VERSIONS[OpenMassSpec]}"
fi

# Emit pin table to stdout (and optionally STACK.md).
emit_pin_table() {
    printf '## Pinned versions\n\n'
    printf '| Component | Version | SHA |\n'
    printf '|-----------|---------|-----|\n'
    for repo in "${REPOS[@]}"; do
        printf '| %s | %s | `%s` |\n' \
            "${DISPLAY_NAME[$repo]}" \
            "${VERSIONS[$repo]}" \
            "${SHAS[$repo]}"
    done
}

# Pull the most-recent versioned section from a CHANGELOG.md. If the
# file has an `[Unreleased]` section with content, prefer that; else
# fall back to the first versioned section.
extract_changelog() {
    local file="$1"
    [ -f "$file" ] || { echo "(no CHANGELOG.md)"; return; }
    awk '
        BEGIN { inblk=0; printed=0 }
        /^## / {
            if (inblk && printed) exit
            if ($0 ~ /^## \[/) { inblk=1; print; next }
            inblk=0
        }
        inblk { print; if (NF>0) printed=1 }
    ' "$file"
}

emit_release_notes() {
    printf '# OpenMassSpec stack release %s\n\n' "$TAG_NAME"
    printf 'Coordinated snapshot of the OpenMassSpec stack.\n\n'
    emit_pin_table
    printf '\n## Per-repo changes\n\n'
    for repo in "${REPOS[@]}"; do
        printf '### %s %s\n\n' \
            "${DISPLAY_NAME[$repo]}" "${VERSIONS[$repo]}"
        extract_changelog "$PROJECTS_DIR/$repo/CHANGELOG.md"
        printf '\n'
    done
}

# Print the draft to stdout so the operator can review and pipe.
NOTES="$(emit_release_notes)"
printf '%s\n' "$NOTES"

# Warn about dirty trees but only fail when applying.
if [ "${#DIRTY_REPOS[@]}" -gt 0 ]; then
    {
        printf '\n[warn] dirty working tree in:'
        for r in "${DIRTY_REPOS[@]}"; do printf ' %s' "$r"; done
        printf '\n'
    } >&2
    if [ "$DO_APPLY" -eq 1 ]; then
        echo "[error] refusing to mutate with dirty trees" >&2
        exit 2
    fi
fi

# Optionally rewrite STACK.md.
if [ "$WRITE_STACK_MD" -eq 1 ]; then
    target="$UMBRELLA_DIR/STACK.md"
    if [ "$DO_APPLY" -eq 1 ]; then
        {
            printf '# OpenMassSpec stack snapshot\n\n'
            printf 'Current pinned versions across the OpenMassSpec stack.\n'
            printf 'Regenerate with `scripts/release-stack.sh '
            printf -- '--write-stack-md --apply`.\n\n'
            emit_pin_table
        } > "$target"
        echo "[ok] wrote $target" >&2
    else
        echo "[dry-run] would overwrite $target" >&2
    fi
fi

# Optionally run the SpecLance truth-test gate before tagging.
if [ "$DO_GATE" -eq 1 ]; then
    gate_cmd=("$SCRIPT_DIR/truthtest-speclance.sh")
    if [ "$GATE_WITH_CORPUS" -eq 1 ]; then
        gate_cmd+=("--with-corpus")
    fi
    if [ "$DO_APPLY" -eq 1 ]; then
        echo "[gate] running: ${gate_cmd[*]}" >&2
        if ! "${gate_cmd[@]}" >&2; then
            echo "[error] SpecLance truth-test gate failed; refusing to tag" >&2
            exit 4
        fi
        echo "[ok] SpecLance truth-test gate green" >&2
    else
        echo "[dry-run] would run gate: ${gate_cmd[*]}" >&2
    fi
fi

# Refuse to tag a commit unless CI and audit are both green for it.
# publish.yml cannot `needs:` a job in a separate workflow file, so this
# is enforced here, before the tag exists, rather than inside
# publish.yml. Runs automatically under --apply; --skip-release-check
# bypasses it for exceptional cases.
if [ "$DO_TAG" -eq 1 ]; then
    if [ "$SKIP_RELEASE_CHECK" -eq 1 ]; then
        echo "[warn] skipping release-readiness check (--skip-release-check)" >&2
    elif [ "$DO_APPLY" -eq 1 ]; then
        echo "[check] running: $SCRIPT_DIR/check-release-ready.sh" >&2
        if ! "$SCRIPT_DIR/check-release-ready.sh" >&2; then
            echo "[error] release-readiness check failed; refusing to tag" >&2
            exit 5
        fi
        echo "[ok] release-readiness check passed" >&2
    else
        echo "[dry-run] would run: $SCRIPT_DIR/check-release-ready.sh" >&2
    fi
fi

# Optionally create / push the umbrella tag.
if [ "$DO_TAG" -eq 1 ]; then
    if [ "$DO_APPLY" -ne 1 ]; then
        echo "[dry-run] would create annotated tag $TAG_NAME on OpenMassSpec" >&2
        if [ "$DO_PUSH" -eq 1 ]; then
            echo "[dry-run] would push $TAG_NAME to origin" >&2
        fi
        exit 0
    fi
    if git -C "$UMBRELLA_DIR" rev-parse "$TAG_NAME" >/dev/null 2>&1; then
        echo "[error] tag $TAG_NAME already exists" >&2
        exit 1
    fi
    tmpf="$(mktemp)"
    printf '%s\n' "$NOTES" > "$tmpf"
    git -C "$UMBRELLA_DIR" tag -a "$TAG_NAME" -F "$tmpf"
    rm -f "$tmpf"
    echo "[ok] created tag $TAG_NAME" >&2
    if [ "$DO_PUSH" -eq 1 ]; then
        git -C "$UMBRELLA_DIR" push origin "$TAG_NAME"
        echo "[ok] pushed $TAG_NAME to origin" >&2
    fi
fi
