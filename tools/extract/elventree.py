"""Lists the elven tree's skills from the UI_CraftingSkillTree_S24 widget export.

usage: elventree.py <UI_CraftingSkillTree_S24.json> <out.json>

Each UI_SkillSlot names its DT_Skills_Crafting_S24 row (SkillHandle); its CanvasPanelSlot, found by content
(slot names repeat), gives its position. The table has rows the tree does not show. Same output as
`dr-extract game` (GameExport.ElvenTree).
"""
import json
import sys

src, dst = sys.argv[1], sys.argv[2]
objs = json.load(open(src))
slots = {}
for o in objs:
    if o.get("Type") == "CanvasPanelSlot":
        content = ((o.get("Properties") or {}).get("Content") or {}).get("ObjectName", "")
        slots[content.split(".")[-1].rstrip("'")] = o.get("Properties") or {}

out = []
for o in objs:
    if o.get("Type") != "UI_SkillSlot_C":
        continue
    row = (((o.get("Properties") or {}).get("SkillHandle") or {}).get("RowName"))
    if not row or row == "None":
        continue
    off = (slots.get(o["Name"], {}).get("LayoutData") or {}).get("Offsets") or {}
    out.append({"id": o["Name"], "row": row, "x": off.get("Left", 0.0), "y": off.get("Top", 0.0)})

if not out:
    sys.exit("no skill slot found in UI_CraftingSkillTree_S24 (is UI_SkillSlot unpacked?)")
json.dump({"slots": out}, open(dst, "w"), indent=1)
print(f"{len(out)} elven tree slots", file=sys.stderr)
