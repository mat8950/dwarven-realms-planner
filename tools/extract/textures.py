"""List the pak paths of the textures and fonts the planner UI uses.

usage: python3 textures.py <data/extracted/tables> <paklist.txt>  > texture-paths.txt
Prints one pak path per line (.uasset, plus .uexp/.ubulk when present).
"""
import json
import sys
from pathlib import Path

tables = Path(sys.argv[1])
pak = set(Path(sys.argv[2]).read_text().split("\n"))

# Whole folders: UI chrome (slot silhouettes, rarity backgrounds, frames) and fonts.
FOLDERS = [
    "ProjectAlpha/Content/EasySurvivalRPG/Textures/UI/",
    "ProjectAlpha/Content/EasySurvivalRPG/Textures/Fonts/",
    "ProjectAlpha/Content/EasySurvivalRPG/Blueprints/Equipment_Modifiers/",
]


def icons(o, acc):
    if isinstance(o, dict):
        for k, v in o.items():
            if k.startswith("Icon") and isinstance(v, dict) and v.get("ObjectPath"):
                acc.add(v["ObjectPath"])
            icons(v, acc)
    elif isinstance(o, list):
        for v in o:
            icons(v, acc)


def rows(rel):
    return json.load(open(tables / rel))[0]["Rows"]


wanted = {"/Game/EasySurvivalRPG/Textures/New_Icons/dwarven-heart"}
icons(rows("EasySurvivalRPG/Blueprints/DataTables/GeneratedItems/DT_Base_Item_Attributes.json"), wanted)
icons(rows("EasySurvivalRPG/Blueprints/Abilities/SkillAbilities/Localized/DT_PlayerAbilities.json"), wanted)
icons(rows("EasySurvivalRPG/Blueprints/Misc/WeaponAttacksSystem/DT_WeaponAttacks.json"), wanted)
icons(rows("EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/SkillDataTables/DT_GENERATED_SkillTree_Main.json"), wanted)
for f in sorted((tables / "EasySurvivalRPG/Blueprints/DataTables/StanceSkills").glob("DT_*Skills*.json")):
    icons(rows(f.relative_to(tables)), wanted)
icons(rows("EasySurvivalRPG/Blueprints/DataTables/StanceSkills/Localized/DT_Skills_Crafting_S24.json"), wanted)
icons(rows("EasySurvivalRPG/Blueprints/DataTables/LocalizedDataTables/DT_StatusEffects.json"), wanted)
for name, row in rows("EasySurvivalRPG/Blueprints/DataTables/DT_Items.json").items():
    tags = json.dumps({k: v for k, v in row.items() if k.startswith("ItemTags")})
    if "Items.Equipment" in tags or "Items.Weapon" in tags:
        icons(row, wanted)

out = set()
for p in wanted:
    base = "ProjectAlpha/Content/" + p.split(".")[0].removeprefix("/Game/")
    for ext in (".uasset", ".uexp", ".ubulk"):
        if base + ext in pak:
            out.add(base + ext)
for f in pak:
    if any(f.startswith(d) for d in FOLDERS) and not Path(f).name.startswith(("BP_", "UI_", "DT_", "STR_", "E_")):
        out.add(f)
print("\n".join(sorted(out)))
