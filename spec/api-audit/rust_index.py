"""Index pltrs' public API from rustdoc JSON, as JSON lines.

Each record: crate, path (module path of the item or its owner), owner
(type or trait name, "" for free items), name, kind, via (trait name for
methods that come from a trait impl), doc (first line).
"""
import json
import sys

from paths import DOC
CRATES = ["pltrs", "pltrs_core", "pltrs_text", "pltrs_render", "pltrs_font", "pltrs_raster", "pltrs_codec"]


import re

SPAN = re.compile(r"`([^`]+)`")
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def mentions(docs, always=False):
    """The identifiers in backticks of docs that talk about matplotlib."""
    if not docs:
        return []
    if not always and "matplotlib" not in docs and "pyplot" not in docs and "mpl" not in docs:
        return []
    found = []
    for span in SPAN.findall(docs):
        if "::" in span or span.startswith("["):
            continue
        found.extend(WORD.findall(span))
    return sorted(set(found))


def first_line(docs):
    return (docs or "").strip().split("\n")[0][:160]


def inner_kind(item):
    return next(iter(item["inner"]))


def type_id(ty):
    if isinstance(ty, dict) and "resolved_path" in ty:
        return ty["resolved_path"]["id"]
    return None


def type_names(ty, acc):
    """Collects the names of the paths a type mentions."""
    if isinstance(ty, dict):
        if "resolved_path" in ty:
            acc.append(ty["resolved_path"]["path"].split("::")[-1])
        for value in ty.values():
            type_names(value, acc)
    elif isinstance(ty, list):
        for value in ty:
            type_names(value, acc)
    return acc


def inputs_of(item):
    inner = item["inner"].get("function")
    if not inner:
        return []
    return [[name, type_names(ty, [])] for name, ty in inner["sig"]["inputs"] if name != "self"]


def index_crate(crate):
    data = json.load(open(f"{DOC}/{crate}.json"))
    idx = data["index"]
    paths = data["paths"]
    out = []

    def path_of(item_id):
        p = paths.get(str(item_id))
        return "::".join(p["path"][:-1]) if p else ""

    def emit(**rec):
        out.append(dict(crate=crate, **rec))

    for key, item in idx.items():
        if item["crate_id"] != 0 or item.get("visibility") != "public":
            continue
        kind = inner_kind(item)
        name = item["name"]
        if kind in ("struct", "enum", "trait", "function", "constant", "static",
                    "type_alias", "macro", "union"):
            p = paths.get(key)
            if p is None:
                # Not reachable by a public path (e.g. a method's function
                # item is handled with its impl below).
                continue
            emit(path="::".join(p["path"][:-1]), owner="", name=name, kind=kind,
                 via="", doc=first_line(item["docs"]), inputs=inputs_of(item),
                 mentions=mentions(item["docs"]))
            if kind == "struct":
                fields = item["inner"]["struct"]["kind"]
                if isinstance(fields, dict) and "plain" in fields:
                    for fid in fields["plain"]["fields"]:
                        f = idx.get(str(fid))
                        if f and f.get("visibility") == "public":
                            emit(path="::".join(p["path"]), owner=name, name=f["name"],
                                 kind="field", via="", doc=first_line(f["docs"]),
                                 mentions=mentions(f["docs"], always=True),
                                 ftypes=type_names(f["inner"]["struct_field"], []))
            elif kind == "enum":
                for vid in item["inner"]["enum"]["variants"]:
                    v = idx.get(str(vid))
                    if v:
                        emit(path="::".join(p["path"]), owner=name, name=v["name"],
                             kind="variant", via="", doc=first_line(v["docs"]),
                             mentions=mentions(v["docs"]))
                        vk = v["inner"]["variant"]["kind"]
                        if isinstance(vk, dict) and "struct" in vk:
                            for fid in vk["struct"]["fields"]:
                                f = idx.get(str(fid))
                                if f:
                                    emit(path="::".join(p["path"]), owner=f"{name}::{v['name']}",
                                         name=f["name"], kind="field", via="",
                                         doc=first_line(f["docs"]))
            elif kind == "trait":
                for mid in item["inner"]["trait"]["items"]:
                    m = idx.get(str(mid))
                    if m:
                        emit(path="::".join(p["path"]), owner=name, name=m["name"],
                             kind="trait_" + inner_kind(m), via="", doc=first_line(m["docs"]),
                             inputs=inputs_of(m), mentions=mentions(m["docs"]))

    # Methods from impl blocks on local types.
    for key, item in idx.items():
        if item["crate_id"] != 0 or inner_kind(item) != "impl":
            continue
        imp = item["inner"]["impl"]
        if imp["is_synthetic"]:
            continue
        tid = type_id(imp["for"])
        if tid is None or str(tid) not in paths:
            continue
        p = paths[str(tid)]
        if p["crate_id"] != 0:
            continue
        owner = p["path"][-1]
        trait = imp["trait"]
        via = ""
        if trait is not None:
            tpath = paths.get(str(trait["id"]))
            via = trait["path"].split("::")[-1]
            if tpath is None or tpath["crate_id"] != 0:
                # Std traits: record the interesting ones (conversions,
                # parsing, operators) by trait name only.
                emit(path="::".join(p["path"][:-1]), owner=owner, name=via, kind="std_impl",
                     via=via, doc="")
                continue
        for mid in imp["items"]:
            m = idx.get(str(mid))
            if not m or m["name"] is None:
                continue
            if trait is None and m.get("visibility") != "public":
                continue
            emit(path="::".join(p["path"][:-1]), owner=owner, name=m["name"],
                 kind="method" if inner_kind(m) == "function" else inner_kind(m),
                 via=via, doc=first_line(m["docs"]), inputs=inputs_of(m),
                 mentions=mentions(m["docs"]))
        for name in imp.get("provided_trait_methods", []):
            emit(path="::".join(p["path"][:-1]), owner=owner, name=name, kind="method",
                 via=via, doc="(provided)")
    return out


records = []
for crate in CRATES:
    records.extend(index_crate(crate))
seen = set()
for r in records:
    key = (r["crate"], r["path"], r["owner"], r["name"], r["kind"], r["via"])
    if key in seen:
        continue
    seen.add(key)
    print(json.dumps(r))
print(f"{len(seen)} records", file=sys.stderr)
