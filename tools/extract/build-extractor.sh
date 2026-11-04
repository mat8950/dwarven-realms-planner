#!/usr/bin/env bash
# Builds the dr-extract sidecar (tools/extract/cue4parse) for one platform and puts it where Tauri
# expects it: app/src-tauri/binaries/dr-extract-<rust target triple>[.exe].
#
# usage: tools/extract/build-extractor.sh [<dotnet runtime id>]    (default: this machine)
# Runtime ids: linux-x64, win-x64, osx-arm64. Uses `dotnet` when it has an SDK, else Docker.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RID="${1:-}"
if [ -z "$RID" ]; then
    case "$(uname -s)-$(uname -m)" in
        Linux-x86_64) RID=linux-x64 ;;
        Linux-aarch64) RID=linux-arm64 ;;
        Darwin-arm64) RID=osx-arm64 ;;
        Darwin-x86_64) RID=osx-x64 ;;
        MINGW*|MSYS*|CYGWIN*) RID=win-x64 ;;
        *) echo "unknown platform, pass a runtime id" >&2; exit 1 ;;
    esac
fi
case "$RID" in
    linux-x64) TRIPLE=x86_64-unknown-linux-gnu ;;
    linux-arm64) TRIPLE=aarch64-unknown-linux-gnu ;;
    win-x64) TRIPLE=x86_64-pc-windows-msvc ;;
    osx-arm64) TRIPLE=aarch64-apple-darwin ;;
    osx-x64) TRIPLE=x86_64-apple-darwin ;;
    *) echo "unsupported runtime id $RID" >&2; exit 1 ;;
esac
EXT=""; [ "$RID" = win-x64 ] && EXT=.exe

OUT="$ROOT/target/dr-extract/$RID"
mkdir -p "$OUT" "$ROOT/app/src-tauri/binaries"
if dotnet --list-sdks 2>/dev/null | grep -q .; then
    dotnet publish "$ROOT/tools/extract/cue4parse/app.csproj" -c Release -r "$RID" -o "$OUT"
else
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -e DOTNET_CLI_HOME=/tmp \
        -v "$ROOT/tools/extract:/src:ro" -v "$OUT:/out" -v "${NUGET_CACHE:-$HOME/.cache/dr-planner/nuget}:/nuget" \
        mcr.microsoft.com/dotnet/sdk:10.0 \
        sh -c "cp -r /src /tmp/x && cd /tmp/x/cue4parse && NUGET_PACKAGES=/nuget dotnet publish -c Release -r $RID -o /out"
fi
cp "$OUT/dr-extract$EXT" "$ROOT/app/src-tauri/binaries/dr-extract-$TRIPLE$EXT"
echo "built app/src-tauri/binaries/dr-extract-$TRIPLE$EXT"
