"""Print match.jsonl records of a status, grouped by module and owner.

Usage: show.py STATUS [MODULE_PREFIX] [--kinds k1,k2] [--notes]"""
import collections
import json
import sys

from paths import DATA as D
status = sys.argv[1]
prefix = sys.argv[2] if len(sys.argv) > 2 and not sys.argv[2].startswith("--") else ""
kinds = None
if "--kinds" in sys.argv:
    kinds = set(sys.argv[sys.argv.index("--kinds") + 1].split(","))
notes = "--notes" in sys.argv
groups = collections.OrderedDict()
for line in open(f"{D}/match.jsonl"):
    r = json.loads(line)
    if r["status"] != status or not r["module"].startswith(prefix):
        continue
    if kinds and r["kind"] not in kinds:
        continue
    groups.setdefault((r["module"], r["owner"]), []).append(r)
total = 0
for (module, owner), rs in groups.items():
    total += len(rs)
    if notes:
        print(f"## {module}:{owner}")
        for r in rs:
            print(f"   {r['name']} [{r['kind']}] -> {r['rust']} | {r['note']}")
    else:
        names = " ".join(r["name"] + ("" if not r["rust"] else "{" + r["rust"][:40] + "}") for r in rs)
        print(f"## {module}:{owner} ({len(rs)}): {names}")
print(total, file=sys.stderr)
