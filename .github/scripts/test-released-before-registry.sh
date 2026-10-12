#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/test-lib.sh"

REPO_ROOT="$(cd "$SRC_SCRIPTS/../.." && pwd)"
CRATE=br-llm-messages
ALIAS_TAG="$CRATE/v0.2.0"
LEGACY_TAG="v0.2.0"

install_stubs
init_repo

printf '[package]\nname = "%s"\nversion = "0.2.0"\nedition = "2024"\n' "$CRATE" >"$DEV/Cargo.toml"
mkdir -p "$DEV/src"
echo "pub fn x() {}" >"$DEV/src/lib.rs"
printf '# Changelog\n\n## Unreleased\n\n## 0.2.0 - 2026-10-08\n' >"$DEV/CHANGELOG.md"
printf 'target\n' >"$DEV/.gitignore"
g add -A && g commit -q -m "single-package layout, 0.2.0 released" && g push -q origin main
LEGACY_COMMIT="$(g rev-parse HEAD)"
g tag -a "$LEGACY_TAG" -m "released before the registry chain" && g push -q origin "$LEGACY_TAG"

g checkout -q -b feat/workspace
mkdir -p "$DEV/crates/$CRATE"
g mv src "crates/$CRATE/src"
g mv Cargo.toml "crates/$CRATE/Cargo.toml"
g mv CHANGELOG.md "crates/$CRATE/CHANGELOG.md"
printf '[workspace]\nresolver = "3"\nmembers = ["crates/br-llm-messages"]\n' >"$DEV/Cargo.toml"
cp "$REPO_ROOT/crates/$CRATE/registry.toml" "$DEV/crates/$CRATE/registry.toml"
copy_scripts
(cd "$DEV" && cargo generate-lockfile --offline -q)
MIGRATION_COMMIT="$(commit_all "workspace layout, release chain, version still 0.2.0")"

requests() { ls "$STUB_DIR/requests" | wc -l | tr -d ' '; }
detect() {
    in_dev env GITHUB_REF=refs/heads/main EVENT_NAME=push GITHUB_SHA="$(g rev-parse HEAD)" bash .github/scripts/release-detect.sh
}
gate() { in_dev env BR_REGISTRY_KEY=k GITHUB_OUTPUT="$WORK/gate.out" bash .github/scripts/registry-gate.sh "$@"; }

echo "== the version lookup reads a revision from before the workspace layout"
expect_equal "$(in_dev bash .github/scripts/crate-version.sh "$CRATE" "$LEGACY_COMMIT")" 0.2.0 "single-package revision: 0.2.0"
expect_equal "$(in_dev bash .github/scripts/crate-version.sh "$CRATE" "$MIGRATION_COMMIT")" 0.2.0 "workspace revision: 0.2.0"
expect 1 'is missing at' "a single-package revision of another package: refuses" in_dev bash .github/scripts/crate-version.sh br-other "$LEGACY_COMMIT"

echo "== the pull request that moves the layout is not a version bump"
stub_reset
stub_error librariesValidateReleaseTarget PATCH_IMPLEMENTED "implemented"
expect 0 'nothing to gate' "0.2.0 on both sides: the registry is not asked" in_dev env BASE_REF=main BR_REGISTRY_KEY=k bash .github/scripts/pr-registry-gate.sh
expect_equal "$(requests)" 0 "no registry request was sent"

echo "== first push to main after the move, the release tag in the new format does not exist yet"
g checkout -q main
g merge -q --ff-only feat/workspace
g push -q origin main
expect 0 'no prior br-llm-messages/v* tag' "semver baseline: no tag of the new format, skipped" in_dev env SEMVER_STUB=1 bash .github/scripts/semver-check.sh "$CRATE"
run detect
if [[ "$RC" -eq 0 ]] && grep -q '"crate":"br-llm-messages","version":"0.2.0"' <<<"$OUT"; then
    verdict yes "detect proposes 0.2.0, since no tag of the new format exists"
else
    verdict no "detect proposes 0.2.0, since no tag of the new format exists"
fi
stub_error librariesValidateReleaseTarget PATCH_IMPLEMENTED "implemented"
expect 1 'does not exist in this repository' "the gate refuses to tag another commit for a version the registry already holds" gate "$CRATE" 0.2.0 "$ALIAS_TAG"

echo "== the tag in the new format is on the released commit"
g tag -a "$ALIAS_TAG" -m "$CRATE 0.2.0" "$LEGACY_COMMIT" && g push -q origin "$ALIAS_TAG"
expect 0 'matrix=[]' "detect releases nothing" detect
expect 0 'already shipped' "detect says the version is already shipped" detect
: >"$WORK/gate.out"
expect 0 'idempotent re-run' "the gate, if it runs, reports implemented and exits 0" gate "$CRATE" 0.2.0 "$ALIAS_TAG"
expect_file_has "$WORK/gate.out" 'implemented=true' "implemented=true written"
expect_equal "$(g rev-list -n 1 "refs/tags/$ALIAS_TAG")" "$LEGACY_COMMIT" "the tag still points at the released commit"
expect_file_has "$REPO_ROOT/.github/workflows/release-tags.yml" "if: steps.gate.outputs.implemented != 'true'" "the workflow skips prepare, tag and record when implemented=true"
expect 0 '' "an unrelated commit" git -C "$DEV" commit -q --allow-empty -m unrelated
expect 0 'matrix=[]' "an unrelated push releases nothing" detect
: >"$STUB_DIR/cargo.log"
expect 0 '' "semver baseline: the tag of the new format" in_dev env SEMVER_STUB=1 bash .github/scripts/semver-check.sh "$CRATE"
expect_file_has "$STUB_DIR/cargo.log" "semver-checks -p $CRATE" "semver-checks runs against a baseline"
expect_file_has "$STUB_DIR/cargo.log" "doc -p $CRATE --lib --no-deps --all-features --locked" "the baseline rustdoc is built"
expect_file_has "$STUB_DIR/cargo.log" "at $ALIAS_TAG" "from the single-package tree at the tag"

echo "== the next release: 0.3.0, baseline named by the registry in the old tag format"
stub_reset
set_version "$CRATE" 0.3.0
NEXT_COMMIT="$(commit_all "0.3.0")"
run detect
if [[ "$RC" -eq 0 ]] && grep -q '"crate":"br-llm-messages","version":"0.3.0","line":"main"' <<<"$OUT"; then verdict yes "detect proposes 0.3.0 only"; else verdict no "detect proposes 0.3.0 only"; fi
eligible_target "\"$LEGACY_TAG\"" false 0.3.0
: >"$WORK/gate.out"
expect 0 'eligible for release' "the gate accepts 0.3.0" gate "$CRATE" 0.3.0
expect_file_has "$WORK/gate.out" "baseline_tag=$LEGACY_TAG" "the baseline tag is the one the registry names, v0.2.0"
expect 0 'registry prepare' "prepare with the old-format baseline" in_dev env BR_REGISTRY_KEY=k REGISTRY_WORK="$WORK/reg" bash .github/scripts/registry-prepare.sh "$CRATE" 0.3.0 "$NEXT_COMMIT" "$LEGACY_TAG"
expect_file_has "$STUB_DIR/cargo.log" "diff $LEGACY_TAG..$NEXT_COMMIT" "the API diff runs from v0.2.0 to the release commit"
expect 0 "creating tag $CRATE/v0.3.0" "the new release takes the new tag format" in_dev env GITHUB_REPOSITORY=o/r bash .github/scripts/release-crate.sh "$CRATE" 0.3.0 refs/heads/main "$NEXT_COMMIT"
: >"$STUB_DIR/cargo.log"
expect 0 'registry record' "record with the old-format baseline" in_dev env BR_REGISTRY_KEY=k REGISTRY_WORK="$WORK/reg" bash .github/scripts/registry-record.sh "$CRATE" 0.3.0 "$CRATE/v0.3.0" "$NEXT_COMMIT" "$LEGACY_TAG"

summary
