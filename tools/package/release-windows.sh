#!/usr/bin/env bash
# Builds the Windows installer (setup.exe, the only Windows download) on this Linux PC.
#   tools/package/release-windows.sh            build only, into target/windows/
#   tools/package/release-windows.sh --release  also attach both to the GitHub release v<version>
#                                               (created as a draft when missing; publish it by hand)
# The Linux and macOS installers come from GitHub Actions, into the same release:
#   gh workflow run release.yml -f platform=linux -f tag=v<version>
#   gh workflow run release.yml -f platform=macos -f tag=v<version>
# Then, once every system is in the release: tools/package/update-manifest.sh v<version> (the updater's latest.json).
# Needs: rustup target x86_64-pc-windows-msvc, cargo-xwin, and Docker (or dotnet for the extractor,
# makensis for the installer). The setup is signed for the updater with the project's key: ~/.tauri/dr-planner.key
# and its password in ~/.tauri/dr-planner.password (or TAURI_SIGNING_PRIVATE_KEY[_PASSWORD] already set).
set -euo pipefail
cd "$(dirname "$0")/../.."

release=0
[ "${1:-}" = --release ] && release=1
version=$(python3 -c 'import json; print(json.load(open("app/src-tauri/tauri.conf.json"))["version"])')
tag="v$version"

if [ $release = 1 ]; then
  # The release says which commit it was built from: that commit must be on GitHub.
  [ -z "$(git status --porcelain)" ] || { echo "Commit the changes first: the release is built from HEAD." >&2; exit 1; }
  git fetch -q origin
  [ "$(git rev-parse HEAD)" = "$(git rev-parse '@{u}')" ] || { echo "Push HEAD first." >&2; exit 1; }
fi

command -v makensis >/dev/null || PATH="$PWD/tools/package/bin:$PATH"

# The updater's signature of the setup (bundle.createUpdaterArtifacts).
if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  [ -f ~/.tauri/dr-planner.key ] || { echo "No updater signing key (~/.tauri/dr-planner.key)." >&2; exit 1; }
  export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/dr-planner.key)"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(cat ~/.tauri/dr-planner.password 2>/dev/null || true)"
fi

tools/extract/build-extractor.sh win-x64
(cd app && npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis)

out=target/windows
mkdir -p "$out"
setup="$out/Dwarven-Realms-Planner-$version-win64-setup.exe"
nsis="target/x86_64-pc-windows-msvc/release/bundle/nsis/Dwarven Realms Planner_${version}_x64-setup.exe"
cp "$nsis" "$setup"
cp "$nsis.sig" "$setup.sig"
ls -la "$setup"

[ $release = 1 ] || exit 0

if gh release view "$tag" >/dev/null 2>&1; then
  gh release upload "$tag" "$setup" "$setup.sig" --clobber
else
  gh release create "$tag" "$setup" "$setup.sig" --draft --target "$(git rev-parse HEAD)" \
    --title "Dwarven Realms Planner $tag" --notes "$(cat tools/package/release-notes.md)"
fi
gh release view "$tag" --json url,isDraft,assets -q '.url + (if .isDraft then " (draft)" else "" end) + "\n" + ([.assets[].name] | join("\n"))'
