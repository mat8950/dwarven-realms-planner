import json, sys, collections
live = json.load(open(sys.argv[1]))["components"][0]
eng = json.load(open(sys.argv[2]))
def summed(arr):
    d = collections.OrderedDict()
    for e in arr:
        d[e["tag"]] = d.get(e["tag"], 0.0) + e["value"]
    return d
for k in ["BaseAttributes", "PlayerAttributes", "EquipmentAttributes", "EffectAttributes", "SelectedItemAttributes", "AttributesNamed"]:
    a, b = summed(live[k]), summed(eng[k])
    bad = [(t, a.get(t), b.get(t)) for t in sorted(set(a) | set(b))
           if abs((a.get(t) or 0) - (b.get(t) or 0)) > 1e-4 * max(1, abs(a.get(t) or 0))]
    print(f"== {k}: game {len(live[k])} entries / engine {len(eng[k])}; {len(bad)} differ")
    for t, x, y in bad[:int(sys.argv[3]) if len(sys.argv) > 3 else 40]:
        print(f"   {t.replace('EasyRPG.',''):70s} game={x} engine={y}")
print("stance mult game", live["StanceDamageMultiplier"], live["StanceCritMult"], "engine", eng["StanceDamageMultiplier"], eng["StanceCritMult"])
