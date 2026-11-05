#!/usr/bin/env bash
# Writes the updater manifest (latest.json) of a GitHub release from the signed bundles it holds, and attaches it.
#   tools/package/update-manifest.sh v0.3.0
# Run it once every system's files are in the release (Windows from release-windows.sh, Linux and macOS from the
# release workflow): the installed apps read the latest release's latest.json to find updates.
# Platforms: windows-x86_64 (the NSIS setup), linux-x86_64 (the AppImage), darwin-aarch64 (the .app.tar.gz).
set -euo pipefail
cd "$(dirname "$0")/../.."

tag="${1:?usage: update-manifest.sh <tag>}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

repo=$(gh repo view --json nameWithOwner -q .nameWithOwner)
gh release view "$tag" --json body,assets > "$work/release.json"
gh release download "$tag" -p '*.sig' -D "$work/sig" --clobber

python3 - "$repo" "$tag" "$work/release.json" "$work/sig" "$work/latest.json" <<'EOF'
import datetime, json, os, sys
repo, tag, release, sigdir, out = sys.argv[1:]
r = json.load(open(release))
# The tag's download URL, not the asset's own: while the release is a draft the asset URL has an "untagged-..." part
# that stops working once it is published.
assets = {a['name']: f'https://github.com/{repo}/releases/download/{tag}/{a["name"]}' for a in r['assets']}
platforms = {}
def add(keys, suffix):
    for name, url in assets.items():
        sig = os.path.join(sigdir, name + '.sig')
        if name.endswith(suffix) and os.path.isfile(sig):
            for k in keys:
                platforms[k] = {'signature': open(sig).read().strip(), 'url': url}
            return name
    return None
found = [
    add(['windows-x86_64', 'windows-x86_64-nsis'], '-win64-setup.exe'),
    add(['linux-x86_64', 'linux-x86_64-appimage'], '.AppImage'),
    add(['darwin-aarch64', 'darwin-aarch64-app'], '.app.tar.gz'),
]
if not platforms:
    sys.exit('no signed bundle in the release')
manifest = {
    'version': tag.lstrip('v'),
    'notes': r.get('body') or '',
    'pub_date': datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
    'platforms': platforms,
}
json.dump(manifest, open(out, 'w'), indent=2)
print('platforms:', ', '.join(sorted(k for k in platforms if k.count('-') == 1)))
print('bundles:', ', '.join(f for f in found if f))
EOF
gh release upload "$tag" "$work/latest.json" --clobber
echo "latest.json attached to $tag"
