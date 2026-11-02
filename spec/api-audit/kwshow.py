"""For review kwargs of OWNER in MODULE, print each method's Rust options.

Usage: kwshow.py MODULE OWNER [METHOD...]"""
import collections
import json
import sys

sys.argv += [] if len(sys.argv) > 2 else [""]
from paths import DATA as D
module, owner = sys.argv[1], sys.argv[2]
only = set(sys.argv[3:])
rust = [json.loads(line) for line in open(f"{D}/rust-api.jsonl")]
fields = collections.defaultdict(list)
methods = collections.defaultdict(dict)
for r in rust:
    if r["kind"] == "field":
        fields[r["owner"]].append(r["name"])
    if r["kind"] in ("method", "trait_function") and r.get("inputs") is not None:
        methods[r["owner"]].setdefault(r["name"], r["inputs"])
review = collections.OrderedDict()
for line in open(f"{D}/match.jsonl"):
    r = json.loads(line)
    if r["status"] == "review" and r["kind"] == "kwarg" and r["module"] == module and r["owner"] == owner:
        m, p = r["name"].rstrip(")").split("(")
        if only and m not in only:
            continue
        review.setdefault(m, []).append(p)
rust_owner = {"matplotlib.axes": "Axes", "mpl_toolkits.mplot3d": "Axes3D"}.get(module, owner)
for m, params in review.items():
    print(f"== {m}: {' '.join(params)}")
    for name, ins in methods[rust_owner].items():
        if name == m or name.startswith(m + "_"):
            sig = ", ".join(f"{n}: {'/'.join(t)}" for n, t in ins)
            print(f"   {name}({sig})")
            for _, types in ins:
                for t in types:
                    if t in fields:
                        print(f"      {t}: {' '.join(fields[t])}")
