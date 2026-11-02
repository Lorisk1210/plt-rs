"""Insert rows into the porting guide's tables and add their link targets.

Usage: guide_rows.py SPEC.json, where SPEC is a list of
{"file": ..., "after": <start of an existing row>, "rows": [[mpl, rust], ...],
 "links": {"Name::item": "crate::path::Name::item"}}.
"""
import json
import re
import sys

from paths import PORTING

ROOT = PORTING + "/"

for entry in json.load(open(sys.argv[1])):
    path = ROOT + entry["file"]
    lines = open(path).read().split("\n")
    index = next(i for i, line in enumerate(lines) if line.startswith("//! | " + entry["after"]))
    widths = [len(cell) for cell in lines[index][4:].strip().strip("|").split("|")]
    new = []
    for mpl, rust in entry["rows"]:
        mpl = (" " + mpl + " ").ljust(widths[0])
        rust = (" " + rust + " ").ljust(widths[1])
        new.append(f"//! |{mpl}|{rust}|")
    lines[index + 1:index + 1] = new
    text = "\n".join(lines)
    defined = set(re.findall(r"^//! \[`([^`]+)`\]:", text, re.M))
    missing = [name for name in entry.get("links", {}) if name not in defined]
    if missing:
        last = max(i for i, line in enumerate(text.split("\n")) if re.match(r"^//! \[`[^`]+`\]:", line))
        lines = text.split("\n")
        lines[last + 1:last + 1] = [f"//! [`{name}`]: {entry['links'][name]}" for name in missing]
        text = "\n".join(lines)
    open(path, "w").write(text)
    print(f"{entry['file']}: +{len(new)} rows, +{len(missing)} links")
