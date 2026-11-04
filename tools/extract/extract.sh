#!/usr/bin/env bash
# Re-extract Dwarven Realms game data (DataTables, string tables, blueprint bytecode)
# from the installed game into data/extracted/. Run again after every game patch.
#
# Needs: cargo (for repak), docker (for the .NET SDK used by CUE4Parse), python3,
# and a .usmap matching the game build in data/mappings/ (see README.md to regenerate).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
GAME="${GAME:-$HOME/Games/steamapps/common/Dwarven Realms}"
PAK="$GAME/ProjectAlpha/Content/Paks/ProjectAlpha-Windows.pak"
WORK="${WORK:-$HOME/.cache/dr-planner}"
USMAP="${USMAP:-$(ls -t "$ROOT"/data/mappings/*.usmap | head -1)}"
BIN="$ROOT/tools/.bin"
DOTNET_IMAGE="mcr.microsoft.com/dotnet/sdk:10.0"

# Blueprints whose bytecode holds the stat formulas.
BLUEPRINTS="/BP_AttributesComponent.,/BP_Character_Player.,/BP_Character_Base.,/BP_PlayerManagerComponent.,/BP_GameLibrary.,/BP_ItemsLibrary.,/UI_Character.,/BP_PlayerStanceSkillManagerComponent.,/UI_ItemToolTip.,/BP_StatusEffect_Base.,/BP_AbilitySystemComponent.,/BP_RaceManager.,/BP_PetManager.,/BP_ContainerComponent.,/BP_EquipmentComponent.,/BP_HotbarComponent.,/BP_PlayerController.,/BP_PotionManager.,/UI_CharInfo_Cards_Grid.,/BP_RuptureOrbsComponent."

mkdir -p "$WORK" "$BIN"

if [ ! -x "$BIN/bin/repak" ]; then
    echo "== installing repak"
    cargo install --root "$BIN" --git https://github.com/trumank/repak repak_cli
fi

echo "== unpacking pak ($PAK)"
rm -rf "$WORK/raw"
"$BIN/bin/repak" unpack -q -f -o "$WORK/raw" \
    -i ProjectAlpha/Content/EasySurvivalRPG/Blueprints \
    -i ProjectAlpha/Content/EasySurvivalRPG/Pets \
    -i ProjectAlpha/Content/EasySurvivalRPG/Skins \
    -i ProjectAlpha/Content/StylizedTigerMount/Blueprints \
    -i ProjectAlpha/Content/Blueprints \
    -i ProjectAlpha/Content/ElementalInfusion \
    -i ProjectAlpha/Content/LocalizationSource \
    -i ProjectAlpha/Content/Localization \
    "$PAK"

echo "== building CUE4Parse exporter"
mkdir -p "$WORK/nuget" "$WORK/exporter"
docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -e DOTNET_CLI_HOME=/tmp \
    -v "$ROOT/tools/extract:/src:ro" -v "$WORK:/w" "$DOTNET_IMAGE" \
    sh -c "cp -r /src /tmp/x && cd /tmp/x/cue4parse && NUGET_PACKAGES=/w/nuget dotnet publish -c Release -o /w/exporter -v q"

run_export() { # <outDir> <filters>
    rm -rf "$WORK/$1"
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -e DOTNET_CLI_HOME=/tmp \
        -e USMAP=/map.usmap -v "$USMAP:/map.usmap:ro" -v "$WORK:/w" "$DOTNET_IMAGE" \
        dotnet /w/exporter/dr-extract.dll /w/raw/ProjectAlpha "/w/$1" "$2"
}

echo "== exporting tables"
run_export tables "/DT_,LocalizationSource/,/Game.locres,/STR_,/E_"
echo "== exporting blueprints"
run_export blueprints "$BLUEPRINTS"

rm -rf "$ROOT/data/extracted"
mkdir -p "$ROOT/data/extracted"
cp -r "$WORK/tables/ProjectAlpha/Content" "$ROOT/data/extracted/tables"
cp -r "$WORK/blueprints/ProjectAlpha/Content" "$ROOT/data/extracted/blueprints"

echo "== attack animation timings"
python3 -I - "$ROOT/data/extracted/tables" > "$WORK/montages.txt" <<'PY'
import json, sys
rows = json.load(open(sys.argv[1] + "/EasySurvivalRPG/Blueprints/Misc/WeaponAttacksSystem/DT_WeaponAttacks.json"))[0]["Rows"]
paths = set()
for row in rows.values():
    for k, v in row.items():
        if k.startswith("Animation"):
            for a, b in v.items():
                if a.startswith("Anim") and isinstance(b, dict) and b.get("ObjectPath"):
                    paths.add(b["ObjectPath"].split(".")[0])
print("\n".join(sorted(paths)))
PY
ANIMS=()
while read -r m; do
    b="ProjectAlpha/Content/${m#/Game/}"
    for e in uasset uexp ubulk; do grep -qx "$b.$e" "$WORK/paklist.txt" && ANIMS+=(-i "$b.$e"); done
done < "$WORK/montages.txt"
# Weapon skills (from DT_WeaponAttacks) plus the basic-attack combos (whole folders).
"$BIN/bin/repak" unpack -q -f -o "$WORK/raw" "${ANIMS[@]}" \
    -i ProjectAlpha/Content/Combat_Animations/WeaponBasicAttacks -i ProjectAlpha/Content/UnarmedAutoAttacks "$PAK"
{ sed 's|^/Game/|ProjectAlpha/Content/|; s|$|.uasset|' "$WORK/montages.txt"
  grep -E '^ProjectAlpha/Content/(Combat_Animations/WeaponBasicAttacks|UnarmedAutoAttacks)/.*\.uasset$' "$WORK/paklist.txt"; } > "$WORK/montage-filters.txt"
rm -rf "$WORK/montjson"
docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -e DOTNET_CLI_HOME=/tmp \
    -e USMAP=/map.usmap -v "$USMAP:/map.usmap:ro" -v "$WORK:/w" "$DOTNET_IMAGE" \
    dotnet /w/exporter/dr-extract.dll /w/raw/ProjectAlpha /w/montjson @/w/montage-filters.txt json
python3 -I "$ROOT/tools/extract/anims.py" "$WORK/montjson" "$ROOT/data/extracted/attack_anims.json"

echo "== skill tree layout and elven tree"
run_export ui "/UI_SkillTree_S4realzies.,/UI_CraftingSkillTree_S24."
python3 -I "$ROOT/tools/extract/skilltree.py" \
    "$WORK/ui/ProjectAlpha/Content/EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/UI_SkillTree_S4realzies.json" \
    "$ROOT/data/extracted/skill_tree.json"
python3 -I "$ROOT/tools/extract/elventree.py" \
    "$WORK/ui/ProjectAlpha/Content/EasySurvivalRPG/Blueprints/DataTables/StanceSkills/UI_CraftingSkillTree_S24.json" \
    "$ROOT/data/extracted/elven_tree.json"

echo "== decompiling blueprints"
mkdir -p "$ROOT/research/pseudo"
find "$ROOT/data/extracted/blueprints" -name '*.json' | while read -r f; do
    python3 -I "$ROOT/tools/extract/kismet.py" "$f" \
        | sed 's/CallFunc_//g; s/_ReturnValue//g; s/Default__KismetArrayLibrary\.//g; s/Default__BlueprintMapLibrary\.//g' \
        > "$ROOT/research/pseudo/$(basename "${f%.json}").txt"
done

echo "== textures and fonts"
"$BIN/bin/repak" list "$PAK" > "$WORK/paklist.txt"
python3 -I "$ROOT/tools/extract/textures.py" "$ROOT/data/extracted/tables" "$WORK/paklist.txt" > "$WORK/texture-paths.txt"
mapfile -t TEX < "$WORK/texture-paths.txt"
INCLUDES=()
for f in "${TEX[@]}"; do INCLUDES+=(-i "$f"); done
"$BIN/bin/repak" unpack -q -f -o "$WORK/raw" "${INCLUDES[@]}" "$PAK"
grep '\.uasset$' "$WORK/texture-paths.txt" > "$WORK/texture-filters.txt"
rm -rf "$WORK/assets"
docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -e DOTNET_CLI_HOME=/tmp \
    -e USMAP=/map.usmap -v "$USMAP:/map.usmap:ro" -v "$WORK:/w" "$DOTNET_IMAGE" \
    dotnet /w/exporter/dr-extract.dll /w/raw/ProjectAlpha /w/assets @/w/texture-filters.txt assets
# Game paths (/Game/X/Y) map to data/assets/X/Y.webp.
rm -rf "$ROOT/data/assets"
(cd "$WORK/assets/ProjectAlpha/Content" && find . -name '*.png') | while read -r f; do
    mkdir -p "$ROOT/data/assets/$(dirname "$f")"
    magick "$WORK/assets/ProjectAlpha/Content/$f" -quality 90 "$ROOT/data/assets/${f%.png}.webp"
done
mkdir -p "$ROOT/data/assets/fonts"
for f in CormorantUnicase-Bold MasonSanAltBol Petrona-VariableFont_wght AlegreyaSans-Bold; do
    "$BIN/bin/repak" get "$PAK" "ProjectAlpha/Content/EasySurvivalRPG/Textures/Fonts/$f.ufont" > "$ROOT/data/assets/fonts/$f.ttf"
done

echo 2 > "$ROOT/data/extracted/FORMAT"   # GameExport.Format / dr_core::setup::DATA_FORMAT
grep -E '"buildid"' "$GAME/../../appmanifest_2015240.acf" | tr -d '\t"' | sed 's/buildid/build /' > "$ROOT/data/extracted/BUILD"
echo "== done: $(cat "$ROOT/data/extracted/BUILD")"
