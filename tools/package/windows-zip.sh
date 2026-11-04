#!/usr/bin/env bash
# Zips a Windows build (no installer) with its first-use instructions, for testers.
# Build first, from Linux:
#   tools/extract/build-extractor.sh win-x64
#   (cd app && npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle)
# Output: target/windows/Dwarven-Realms-Planner-<version>-win64.zip
set -euo pipefail
cd "$(dirname "$0")/../.."

bin=target/x86_64-pc-windows-msvc/release
version=$(python3 -c 'import json; print(json.load(open("app/src-tauri/tauri.conf.json"))["version"])')
name="Dwarven Realms Planner"
out=target/windows
dir="$out/pkg/$name"

rm -rf "$out/pkg"
mkdir -p "$dir"
cp "$bin/dr-app.exe" "$dir/$name.exe"
cp "$bin/dr-extract.exe" "$dir/"
for f in LICENSE-MIT LICENSE-APACHE; do sed 's/$/\r/' "$f" > "$dir/$f.txt"; done
sed -e "s/{version}/$version/" -e 's/$/\r/' tools/package/README-windows.txt > "$dir/README.txt"

zip="$out/Dwarven-Realms-Planner-$version-win64.zip"
rm -f "$zip"
python3 - "$out/pkg" "$name" "$zip" <<'EOF'
import os, sys, zipfile
root, name, dest = sys.argv[1:]
with zipfile.ZipFile(dest, 'w', zipfile.ZIP_DEFLATED) as z:
    for d, _, files in os.walk(os.path.join(root, name)):
        for f in sorted(files):
            p = os.path.join(d, f)
            z.write(p, os.path.relpath(p, root))
EOF
echo "$zip"
