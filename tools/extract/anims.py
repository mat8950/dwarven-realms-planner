"""Timing of attack animations, for attack rate / DPS.

usage: python3 anims.py <exported anim json dir> <out.json>
Writes {"/Game/.../CA_MagicalLance": {...}} with, per animation:
  length    SequenceLength (s)
  rate      RateScale
  finish    BP_Notify_FinishAbility time (weapon skills: when the next attack can start)
  continue  start of the BP_NotifyState_ContinueAttack window (basic combos: next hit)
  next      animation played by that ContinueAttack, with its play rate settings
"""
import json
import os
import sys


def game_path(p):
    p = p.split(".")[0]
    return "/Game/" + p.split("Content/", 1)[-1] if "Content/" in p else p


src, out = sys.argv[1], sys.argv[2]
res = {}
for d, _, files in os.walk(src):
    for f in files:
        if not f.endswith(".json"):
            continue
        exports = json.load(open(os.path.join(d, f)))
        seq = next((e for e in exports if e.get("Type") == "AnimSequence"), None)
        if not seq:
            continue
        by_name = {e.get("Name"): e for e in exports}
        p = seq.get("Properties") or {}
        entry = {"length": p.get("SequenceLength", 0.0), "rate": p.get("RateScale", 1.0)}
        for n in p.get("Notifies", []):
            t = n.get("LinkValue")
            if not isinstance(t, (int, float)):
                continue
            ref = n.get("Notify") or n.get("NotifyStateClass") or {}
            name = ref.get("ObjectName", "") if isinstance(ref, dict) else ""
            if "FinishAbility" in name:
                entry["finish"] = min(entry.get("finish", t), t)
            if "NotifyState_ContinueAttack" in name:
                obj = by_name.get(name.split(":")[-1].rstrip("'"), {})
                nxt = (obj.get("Properties") or {}).get("NextAnim") or {}
                anim = next((v for k, v in nxt.items() if k.startswith("Anim")), None)
                if isinstance(anim, dict) and anim.get("ObjectPath") and ("continue" not in entry or t < entry["continue"]):
                    entry["continue"] = t
                    entry["next"] = game_path(anim["ObjectPath"])
                    entry["next_play_rate"] = next((v for k, v in nxt.items() if k.startswith("InPlayRate")), 1.0)
                    entry["next_attack_speed"] = next((v for k, v in nxt.items() if k.startswith("AffectedByAttackSpeed")), True)
        entry.setdefault("finish", entry["length"])
        rel = os.path.relpath(os.path.join(d, f), src)[: -len(".json")]
        res["/Game/" + rel.split("Content/", 1)[-1]] = entry
json.dump(res, open(out, "w"), indent=1, sort_keys=True)
print(len(res), "animations")
