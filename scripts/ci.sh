#!/bin/sh
# The gate, in one place.
#
# CI calls this on every OS, and a person can run the same file before a merge, so the steps and
# the workflow cannot drift.  The matrix supplies the OS; this supplies the commands.
#
# `--hygiene` adds the checks that only make sense on a clean checkout: the tree has nothing
# uncommitted, and no tracked file names a machine, a secret, a large artifact, or a regenerable
# matrix row.  Local runs skip them because the tree is being edited by definition.
#
# The commit-message check stays in the workflow: it needs the range a push brought, which a
# script cannot know.

set -eu

cd "$(git rev-parse --show-toplevel)"

hygiene=0
case "${1:-}" in
    --hygiene) hygiene=1 ;;
    "") ;;
    *)
        echo "usage: sh scripts/ci.sh [--hygiene]" >&2
        exit 2
        ;;
esac

echo "== fmt"
cargo fmt --all --check

echo "== clippy"
cargo clippy --all-targets -- -D warnings

echo "== test"
cargo test --release

echo "== gate selftest"
if command -v node >/dev/null 2>&1; then
    node dsh/tsare-gate/selftest.mjs
else
    echo "node is not on PATH; the gate selftest is skipped"
fi

echo "== documents"
cargo run --release --quiet -- check SELF.tsare
cargo run --release --quiet -- check docs/REPO.tsare
cargo run --release --quiet -- check prelude/KERNEL.tsare
cargo run --release --quiet -- check prelude/WORLDVIEW.tsare
cargo run --release --quiet -- check prelude/PROCESS.tsare

# The package is built from the `include` list in Cargo.toml.  A file that a test reads but the
# list omits passes locally and fails from a published crate; this is the check that catches it.
# `--allow-dirty` because the check is the list, not the git state.
echo "== package"
cargo package --allow-dirty

if [ "$hygiene" -eq 1 ]; then
    echo "== the tree is tracked"
    if [ -n "$(git status --porcelain)" ]; then
        git status --porcelain
        echo "the checkout has changes that are not committed"
        exit 1
    fi

    # `scripts/ci.sh` is excluded because it contains the pattern itself, as the workflow is
    # excluded for the same reason.
    echo "== no machine paths"
    if git grep -n -I -E '(^|[^[:alnum:]_])([A-Za-z]:[\\/]|/Users/|/home/[a-z]|/root/|\\\\[A-Za-z0-9_.-]+\\|(127\.0\.0\.1|192\.168\.[0-9]+|10\.[0-9]+\.[0-9]+\.[0-9]+|172\.(1[6-9]|2[0-9]|3[01])\.[0-9]+\.[0-9]+|localhost:[0-9]+))' -- . ':!*.lock' ':!.github/**' ':!scripts/ci.sh'; then
        echo "a tracked file names a machine path; use a path relative to the repository root"
        exit 1
    fi

    echo "== no secrets"
    if git grep -n -I -E '(ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----|xox[baprs]-[A-Za-z0-9-]{10,}|AIza[0-9A-Za-z_-]{35})' -- . ':!*.lock'; then
        echo "a tracked file contains something shaped like a secret"
        exit 1
    fi

    echo "== no large tracked files"
    bad=""
    for file in $(git ls-files); do
        size=$(wc -c < "$file")
        if [ "$size" -gt 262144 ]; then
            bad="$bad$file ($size bytes)\n"
        fi
    done
    if [ -n "$bad" ]; then
        printf '%b' "$bad"
        echo "a tracked file over 256 KiB is an artifact; generate it, link it, or keep it out"
        exit 1
    fi

    echo "== no regenerable matrix rows"
    if git ls-files 'tests/fixtures/harden/artifacts/*.jsonl' | grep -q .; then
        git ls-files 'tests/fixtures/harden/artifacts/*.jsonl'
        echo "matrix-*.jsonl is regenerable and full of local paths; keep it untracked"
        exit 1
    fi
fi

echo "ci: ok"
