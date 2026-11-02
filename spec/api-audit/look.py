"""Look names up in the Rust index: look.py [-o OWNER] NAME|/REGEX/ ..."""
import json
import re
import sys

from paths import DATA as D
args = sys.argv[1:]
owner = None
if args[:1] == ["-o"]:
    owner, args = args[1], args[2:]
rust = [json.loads(line) for line in open(f"{D}/rust-api.jsonl")]
for name in args:
    pat = re.compile(name[1:-1]) if name.startswith("/") else None
    hits = []
    for r in rust:
        if r["kind"] == "std_impl":
            continue
        if owner and r["owner"] != owner:
            continue
        ok = pat.search(r["name"]) if pat else r["name"] == name
        mentioned = not pat and name in r.get("mentions", [])
        if ok or mentioned:
            tag = "" if ok else " (mentions)"
            hits.append(f"  {r['owner'] + '::' if r['owner'] else r['path'] + '::'}{r['name']} [{r['kind']}{'/' + r['via'] if r['via'] else ''}]{tag} {r['doc'][:90]}")
    print(f"{name}: {len(hits)}")
    print("\n".join(sorted(set(hits))[:int(__import__("os").environ.get("LIMIT", "40"))]))
