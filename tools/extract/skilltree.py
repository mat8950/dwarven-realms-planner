"""Extracts the main skill tree layout from the UI_SkillTree_S4realzies widget export.

usage: skilltree.py <UI_SkillTree_S4realzies.json> <out.json>

Each node widget sits in a CanvasPanelSlot (top-left offset, auto size = NodeSize).
Base nodes are bought as the DT_GENERATED_SkillTree_Main row of the same name; dropdown
nodes offer the rows "<name>_0", "<name>_1"... (one of them can be owned).
"""
import json
import sys

DEFAULT_SIZE = 30.0  # NodeSize of the UI_SkillTreeNode_Base / _DropDown class defaults

src, dst = sys.argv[1], sys.argv[2]
objs = json.load(open(src))
# Slot names repeat across canvases: find each node's slot by its content instead.
slots = {}
for o in objs:
    if o.get("Type") == "CanvasPanelSlot":
        content = o.get("Properties", {}).get("Content", {}).get("ObjectName", "")
        slots[content.split(".")[-1].rstrip("'")] = o.get("Properties", {})

nodes = []
for o in objs:
    kind = {"UI_SkillTreeNode_Base_C": "base", "UI_SkillTreeNode_DropDown_C": "dropdown"}.get(o.get("Type"))
    if not kind:
        continue
    p = o.get("Properties", {})
    slot = slots.get(o["Name"], {})
    off = slot.get("LayoutData", {}).get("Offsets", {})
    size = p.get("NodeSize") or {}
    nodes.append({
        "id": o["Name"],
        "kind": kind,
        "x": off.get("Left", 0.0),
        "y": off.get("Top", 0.0),
        "w": size.get("X", DEFAULT_SIZE),
        "h": size.get("Y", DEFAULT_SIZE),
        "root": bool(p.get("IsRootNode")),
        "links": p.get("ConnectedNodes", []),
        "category": (p.get("Category") or {}).get("RowName"),
    })

json.dump({"nodes": nodes}, open(dst, "w"), indent=0)
print(f"{len(nodes)} skill tree nodes", file=sys.stderr)
