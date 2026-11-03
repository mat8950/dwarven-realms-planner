#!/usr/bin/env bash
# Installs UE4SS with the planner's measuring mods into the game, or removes it.
#
#   install.sh            copy UE4SS (from the backup) + DRLiveDump + DRDamageLog into the game
#   install.sh --remove   save the logs to research/validation/live-<date>/, then remove UE4SS
#
# The game must run with the Steam launch option WINEDLLOVERRIDES="dwmapi=n,b" %command%.
# UE4SS comes from ~/.cache/dr-planner/ue4ss-install-backup (a previous install), or from an
# RE-UE4SS release zip given as UE4SS_ZIP=... (experimental-latest, UE4SS_v*.zip).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
GAME="${GAME:-$HOME/Games/steamapps/common/Dwarven Realms}"
BIN="$GAME/ProjectAlpha/Binaries/Win64"
BACKUP="${BACKUP:-$HOME/.cache/dr-planner/ue4ss-install-backup}"
MODS="$(cd "$(dirname "$0")" && pwd)"

if pgrep -f 'ProjectAlpha-Win64-Shipping' >/dev/null; then
    echo "The game is running: close it first." >&2
    exit 1
fi

if [ "${1:-}" = "--remove" ]; then
    OUT="$ROOT/research/validation/live-$(date +%Y-%m-%d)"
    mkdir -p "$OUT"
    for f in DRDamageLog.csv DRLiveDump.json; do
        [ -f "$BIN/ue4ss/$f" ] && cp "$BIN/ue4ss/$f" "$OUT/" && echo "saved $OUT/$f"
    done
    # Keep an up-to-date copy for the next install, then take UE4SS out of the game.
    mkdir -p "$BACKUP"
    [ -d "$BIN/ue4ss" ] && cp -r "$BIN/ue4ss" "$BACKUP/" && cp "$BIN/dwmapi.dll" "$BACKUP/"
    rm -f "$BIN/dwmapi.dll"
    rm -r "$BIN/ue4ss"
    echo "UE4SS removed from $BIN. You can now remove the Steam launch option."
    exit 0
fi

if [ -n "${UE4SS_ZIP:-}" ]; then
    unzip -o -q "$UE4SS_ZIP" -d "$BIN"
elif [ -d "$BACKUP/ue4ss" ]; then
    cp "$BACKUP/dwmapi.dll" "$BIN/"
    cp -r "$BACKUP/ue4ss" "$BIN/"
else
    echo "No UE4SS backup in $BACKUP: set UE4SS_ZIP=<UE4SS_v*.zip>." >&2
    exit 1
fi
rm -f "$BIN/ue4ss/DRDamageLog.csv"
for m in DRLiveDump DRDamageLog; do
    rm -rf "$BIN/ue4ss/Mods/$m"
    cp -r "$MODS/$m" "$BIN/ue4ss/Mods/"
done
printf 'Keybinds : 1\nDRDump : 0\nDRLiveDump : 1\nDRDamageLog : 1\n' > "$BIN/ue4ss/Mods/mods.txt"
echo "UE4SS installed in $BIN with DRLiveDump and DRDamageLog."
echo "Launch option needed: WINEDLLOVERRIDES=\"dwmapi=n,b\" %command%"
