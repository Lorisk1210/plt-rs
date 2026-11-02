"""Extract the porting guide's equivalence tables: matplotlib tokens to the
pltrs items of the same row, as JSON lines {token, items, row, file}."""
import glob
import json
import re

SPAN = re.compile(r"`([^`]+)`")
LINK = re.compile(r"\[`([^`]+)`\]")
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
from paths import PORTING as ROOT

for path in sorted(glob.glob(f"{ROOT}/*.rs")):
    for line in open(path):
        if not line.startswith("//! |") or "---" in line:
            continue
        cells = [c.strip() for c in line[4:].strip().strip("|").split("|")]
        if len(cells) < 2 or cells[0] == "matplotlib":
            continue
        mpl, rust = cells[0], " | ".join(cells[1:])
        items = LINK.findall(rust)
        items += [s for s in SPAN.findall(rust) if s not in items]
        tokens = set()
        for span in SPAN.findall(mpl):
            tokens.update(WORD.findall(span))
        for token in sorted(tokens):
            print(json.dumps({"token": token, "items": items, "row": line[4:].strip(),
                              "file": path.rsplit("/", 1)[1]}))
