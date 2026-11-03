"""Summarises the DRDamageLog mod's CSV: hits per damage causer class, and for each class
the instances (beams, projectiles...), hits per instance and procs (instances spawned
within 50 ms of each other).

usage: python3 damagelog.py <DRDamageLog.csv>
"""
import csv
import sys
from collections import defaultdict

# Each game session appends a header line: keep the last session.
lines = open(sys.argv[1]).read().splitlines()
last = max(i for i, l in enumerate(lines) if l.startswith("time,"))
rows = []
for r in csv.DictReader(lines[last:]):
    try:
        rows.append((float(r["time"]), r["causer"], r["class"], float(r["damage"]), r["type"], r["crit"] == "true"))
    except ValueError:
        pass
if not rows:
    sys.exit("no hits in the log")
rows.sort()
t0, t1 = rows[0][0], rows[-1][0]
print(f"{len(rows)} hits over {t1 - t0:.2f} s")

by_class = defaultdict(list)
for r in rows:
    by_class[r[2]].append(r)
for cls, hits in sorted(by_class.items(), key=lambda kv: -sum(h[3] for h in kv[1])):
    total = sum(h[3] for h in hits)
    first = defaultdict(lambda: 1e18)
    count = defaultdict(int)
    for h in hits:
        first[h[1]] = min(first[h[1]], h[0])
        count[h[1]] += 1
    spawns = sorted(first.values())
    procs = 1 + sum(1 for a, b in zip(spawns, spawns[1:]) if b - a > 0.05)
    crit = sum(1 for h in hits if h[5]) / len(hits)
    print(f"\n== {cls} ({hits[0][4]})")
    print(f"  hits {len(hits)}, total {total:.4g}, mean hit {total / len(hits):.4g}, crit {crit:.0%}")
    print(f"  instances {len(first)}, hits per instance {len(hits) / len(first):.2f}, procs {procs}, "
          f"instances per proc {len(first) / procs:.2f}, hits per proc {len(hits) / procs:.2f}")
    if procs > 1:
        print(f"  mean time between procs {(spawns[-1] - spawns[0]) / (procs - 1):.3f} s")
