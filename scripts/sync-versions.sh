#!/usr/bin/env bash
# Sync the lockstep version of the JS package, the PyO3 wheel, the
# pure-Python mirror, the PHP package and the docs site from the Rust
# workspace version in the root Cargo.toml. Used by the release-pr
# workflow after release-plz bumps Cargo.toml; safe to run by hand too
# (no-op when everything already matches).
#
# NOTE: the version source is the root Cargo.toml [workspace.package]
# version (same as indycheck/dusrelimage). If it is missing the script
# fails loudly instead of guessing from another file — release-plz,
# cargo --workspace and this script all need it.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ ! -f Cargo.toml ]; then
  echo "error: no root Cargo.toml — cannot determine the workspace version" >&2
  exit 1
fi

version=$(grep -m1 '^version =' Cargo.toml | sed -E 's/version = "(.*)"/\1/')
echo "syncing lockstep version to $version"

# packages/js/package.json is pretty-printed ("version": "0.1.0", with a
# space after the colon); website/package.json and
# packages/php/composer.json are compact single-line JSON
# ("version":"0.1.0", no space) — the capture group keeps each file's
# own separator, so a no-op sync changes no bytes at all.
sed -i -E "s/(\"version\": *)\"[0-9]+\.[0-9]+\.[0-9]+\"/\1\"$version\"/" packages/js/package.json website/package.json packages/php/composer.json
sed -i -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$version\"/" crates/dusimagez-py/pyproject.toml packages/python/pyproject.toml
sed -i -E "s/softwareVersion: '[0-9]+\.[0-9]+\.[0-9]+'/softwareVersion: '$version'/" website/astro.config.mjs

# NOT synced (no version marker to sync):
# - packages/go/go.mod — the module carries no version; releases are
#   tagged vX.Y.Z on the repo.
# - packages/swift/Package.swift — SwiftPM packages carry no version;
#   releases are tagged vX.Y.Z on the repo.
# - packages/kotlin/library/build.gradle.kts — there is NO versionName /
#   versionCode in the library; the Rust core is the version of
#   record. If a versionName is ever added, add a sed for it here.

echo "packages/js/package.json: $(grep -o '\"version\": *\"[^\"]*\"' packages/js/package.json)"
echo "crates/dusimagez-py/pyproject.toml: $(grep -m1 '^version' crates/dusimagez-py/pyproject.toml)"
echo "packages/python/pyproject.toml: $(grep -m1 '^version' packages/python/pyproject.toml)"
echo "website/package.json: $(grep -o '\"version\":\"[^\"]*\"' website/package.json)"
echo "website/astro.config.mjs: $(grep -m1 'softwareVersion' website/astro.config.mjs)"
echo "packages/php/composer.json: $(grep -o '\"version\":\"[^\"]*\"' packages/php/composer.json)"
