"""Map matplotlib's public API (mpl-api.jsonl) to pltrs (rust-api.jsonl).

Writes match.jsonl: each matplotlib record with an automatic verdict:
  status: covered | differently | n/a | missing | review
  rust: the pltrs item(s) that cover it, note: why.
Manual rules (rules.py) are applied on top, by key "module:Owner.name".
"""
import collections
import json
import re
import sys

from classmap import CLASS_MAP, ARTIST_TRAITS  # noqa: E402
from rules import RULES, KWARG_RULES, OWNER_RULES  # noqa: E402

from paths import DATA as D
mpl = [json.loads(line) for line in open(f"{D}/mpl-api.jsonl")]
rust = [json.loads(line) for line in open(f"{D}/rust-api.jsonl")]
porting = [json.loads(line) for line in open(f"{D}/porting.jsonl")]

TYPE_KINDS = {"struct", "enum", "trait", "type_alias"}
MEMBER_KINDS = {"method", "field", "variant", "trait_function", "assoc_const", "constant"}
STD = {"Clone", "Debug", "PartialEq", "Eq", "Any", "Borrow", "BorrowMut", "CloneToUninit",
       "From", "Into", "TryFrom", "TryInto", "ToOwned", "StructuralPartialEq", "Default",
       "Send", "Sync", "Unpin", "Freeze", "RefUnwindSafe", "UnwindSafe", "Copy", "Hash"}

types = collections.defaultdict(list)          # name -> [path]
members = collections.defaultdict(dict)        # owner -> {name: record}
std_impls = collections.defaultdict(set)       # owner -> {trait}
functions = collections.defaultdict(list)      # name -> [path]
mentions = collections.defaultdict(set)        # token -> {"Owner::name"}
traits_of = collections.defaultdict(set)       # owner -> local traits it implements
inputs = {}                                    # (owner, name) -> inputs
fields_of = collections.defaultdict(set)       # type -> field and method names

for r in rust:
    kind, owner, name = r["kind"], r["owner"], r["name"]
    label = f"{owner}::{name}" if owner else name
    if kind in TYPE_KINDS and not owner:
        types[name].append(r["path"])
    elif kind == "std_impl":
        std_impls[owner].add(name)
    elif kind == "function" and not owner:
        last = r["path"].split("::")[-1]
        if last[:1].isupper():
            members[last].setdefault(name, r)
            fields_of[last].add(name)
            inputs[(last, name)] = r.get("inputs", [])
            label = f"{last}::{name}"
        else:
            functions[name].append(r["path"])
            if "pyplot" in r["path"]:
                members["pyplot"].setdefault(name, r)
                fields_of["pyplot"].add(name)
                inputs[("pyplot", name)] = r.get("inputs", [])
    elif kind in MEMBER_KINDS or kind.startswith("trait_"):
        members[owner].setdefault(name, r)
        fields_of[owner].add(name)
        if r.get("via") and r["via"] not in STD:
            traits_of[owner].add(r["via"])
        if r.get("inputs") is not None:
            inputs.setdefault((owner, name), r.get("inputs", []))
    for token in r.get("mentions", []):
        mentions[token].add(label)

porting_items = collections.defaultdict(set)
porting_rows = collections.defaultdict(list)
for p in porting:
    for item in p["items"]:
        porting_items[p["token"]].add(item)
    porting_rows[p["token"]].append(p["row"])


def owners_for(module, owner):
    key = f"{module}:{owner}"
    mapped = CLASS_MAP.get(key, CLASS_MAP.get(owner))
    if isinstance(mapped, tuple):
        return mapped
    if mapped is None:
        mapped = [owner] if owner in types else []
    expanded = list(mapped)
    for o in mapped:
        expanded.extend(sorted(traits_of.get(o, ())))
    return expanded


AXIS_LETTER = re.compile(r"^(set_|get_)?([xyz])(.+)$")


def snake_case(name):
    """`embedTTF` -> `embed_ttf`, `newPage` -> `new_page`."""
    name = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name).lower()


def name_candidates(name):
    c = [name]
    if name != name.lower() and not name.isupper():
        c.append(snake_case(name))
    if name.startswith("get_"):
        base = name[4:]
        c += [base, "current_" + base, "shown_" + base, base + "s", "is_" + base]
    if name.startswith("is_"):
        c += [name[3:]]
    if name.startswith("set_"):
        c += [name + "_with"]
    m = AXIS_LETTER.match(name)
    if m and len(m.group(3)) > 2:
        prefix = m.group(1) or ""
        rest = m.group(3).lstrip("_")
        if prefix == "get_":
            c += [rest, "axis_" + rest]
        else:
            c += [prefix + rest, prefix + "axis_" + rest]
    if name.endswith("_on") and name.startswith("get_"):
        c += [name[4:]]
    c += [name + "_with", name.replace("get_", "").replace("_", "")]
    out = []
    for x in c:
        if x not in out:
            out.append(x)
    return out


def find_member(owners, name):
    for cand in name_candidates(name):
        for o in owners:
            if cand in members.get(o, {}):
                return f"{o}::{cand}"
    return None


def evidence(name, owners):
    """Doc mentions and porting rows naming `name`."""
    found = []
    for item in sorted(porting_items.get(name, ())):
        owner = item.split("::")[0]
        if owner in owners:
            found.append(item)
    for item in sorted(mentions.get(name, ())):
        owner = item.split("::")[0]
        if owner in owners:
            found.append(item)
    return found


def any_evidence(name):
    return sorted(set(porting_items.get(name, ())) | mentions.get(name, set()))


def option_names(owner, method):
    """Parameter names, option fields (and the names their docs give in
    backticks) and builders of the Rust methods that implement `method` on
    `owner`, following option structs two levels deep."""
    names = set()
    rust_methods = [n for n in members.get(owner, {}) if n == method or n.startswith(method + "_")]
    seen = set()

    def add_type(t, depth):
        if t in seen or depth > 3 or t not in fields_of:
            return
        seen.add(t)
        for f in fields_of[t]:
            names.add(f)
            rec = members[t].get(f)
            if rec and rec["kind"] == "field":
                names.update(rec.get("mentions", []))
                for nested in rec.get("ftypes", []):
                    add_type(nested, depth + 1)

    for m in rust_methods:
        for pname, tnames in inputs.get((owner, m), []):
            names.add(pname)
            for t in tnames:
                add_type(t, 0)
    return names, rust_methods


PROPS_TYPES = {"LineProps", "PatchProps", "CollectionProps", "TextProps", "BaseProps"}


def kwarg_verdict(rec, owners):
    method, param = re.match(r"(.+)\((.+)\)", rec["name"]).groups()
    if rec.get("param_kind") == "*args":
        return "differently", "", "Python varargs: pltrs takes typed positional arguments"
    owner = owners[0] if owners and not isinstance(owners, tuple) else ""
    names, rust_methods = set(), []
    if rec["module"] == "matplotlib.pyplot":
        # pyplot wrappers take the same options as the methods they wrap:
        # reuse the verdict on the Axes method's parameter.
        axes_verdict = axes_kwarg_status.get(f"{method}({param})")
        if axes_verdict and axes_verdict[0] != "review":
            return axes_verdict
        for o in ("pyplot", "Axes", "Figure"):
            if method in members.get(o, {}) or any(n.startswith(method + "_") for n in members.get(o, {})):
                n, m = option_names(o, method)
                names |= n
                rust_methods += [f"{o}::{x}" for x in m]
        owner = ""
    else:
        names, rust_methods = option_names(owner, method) if owner else (set(), [])
    if not rust_methods and rec["module"] == "matplotlib.pyplot":
        rust_methods = [n for n in functions if n == method or n.startswith(method + "_")]
    if rec.get("param_kind") == "**kwargs":
        return "differently", "", "artist properties: a props struct (LineProps/PatchProps/CollectionProps/TextProps) or setters on the returned handle"
    snake = re.sub(r"(?<=[a-z])([A-Z])", r"_\1", param).lower()
    for cand in (param, param.rstrip("s"), param + "s", param.replace("_", ""), snake, param.lower()):
        if cand in names:
            return "covered", f"{label(owner, rust_methods)}({cand})", ""
    return "review", label(owner, rust_methods) if rust_methods else "", "parameter not found in the method's options"


def label(owner, rust_methods):
    joined = "/".join(rust_methods[:3])
    return f"{owner}::{joined}" if owner else joined


def verdict(rec):
    module, owner, name, kind = rec["module"], rec["owner"], rec["name"], rec["kind"]
    key = f"{module}:{owner + '.' if owner else ''}{name}"
    if rec["deprecated"]:
        return "deprecated", "", "deprecated in matplotlib"
    for pattern, result in RULES:
        if re.fullmatch(pattern, key):
            return result
    if owner:
        okey = f"{module}:{owner}"
        for pattern, result in OWNER_RULES:
            if re.fullmatch(pattern, okey):
                return result
    if kind == "kwarg":
        for pattern, result in KWARG_RULES:
            if re.fullmatch(pattern, key) or re.fullmatch(pattern, name.split("(")[1].rstrip(")")):
                return result
        owners = owners_for(module, owner) if owner else []
        return kwarg_verdict(rec, owners)
    if kind == "class" and owner and not isinstance(owners_for(module, owner), tuple):
        hit = find_member(owners_for(module, owner), name)
        if hit:
            return "covered", hit, ""
    if kind in ("class", "private_base"):
        mapped = owners_for(module, f"{owner}.{name}" if owner else name)
        if isinstance(mapped, tuple):
            return mapped[0], "", mapped[1]
        if mapped:
            return "covered", ", ".join(mapped[:3]), ""
        ev = any_evidence(name)
        if ev:
            return "review", ", ".join(ev[:4]), "mentioned in docs/porting guide"
        return "review", "", "no pltrs type of this name"
    if name.startswith("__") and name.endswith("__"):
        mapped = owners_for(module, owner) if owner else []
        target = ", ".join(mapped[:2]) if isinstance(mapped, list) else ""
        if name == "__call__":
            return "differently", target, "calling the object is a named method (normalize, map, locate, format_ticks, ...)"
        if name in ("__getitem__", "__setitem__", "__delitem__", "__iter__", "__len__", "__contains__"):
            return "differently", target, "indexing and iteration are named methods (get, set, iter, len, contains, ...)"
        if name in ("__add__", "__radd__", "__sub__", "__mul__", "__rmul__", "__div__", "__neg__"):
            return "differently", target, "arithmetic on transforms and sizes is a named method (Transform::then, Size::Sum, Size::times)"
        if name == "__array__":
            return "differently", target, "the values are read with methods (Affine2D::matrix, Bbox::p0 / p1)"
        return "n/a", "", "Python protocol or dataclass machinery"
    if owner:
        owners = owners_for(module, owner)
        if isinstance(owners, tuple):
            return owners[0], "", f"class: {owners[1]}"
        hit = find_member(owners, name)
        if hit:
            return "covered", hit, ""
        ev = evidence(name, owners)
        if ev:
            return "covered", ", ".join(ev[:3]), "named in pltrs docs/porting guide"
        ev = any_evidence(name)
        if ev:
            return "review", ", ".join(ev[:4]), "mentioned elsewhere"
        return "review", "", f"not on {', '.join(owners[:4]) or 'any mapped type'}"
    # Module-level function or attribute.
    if module == "matplotlib.pyplot":
        if name in functions:
            return "covered", f"pyplot::{name}" if any("pyplot" in p for p in functions[name]) else f"{functions[name][0]}::{name}", ""
        for cand in name_candidates(name):
            if cand in functions:
                return "covered", f"{functions[cand][0]}::{cand}", ""
        for o in ("Axes", "Figure", "Axis"):
            hit = find_member([o], name)
            if hit:
                return "differently", hit, "no pyplot wrapper: call it on gca()/gcf()"
    for cand in name_candidates(name):
        if cand in functions:
            return "covered", f"{functions[cand][0]}::{cand}", ""
        if cand in types:
            return "covered", f"{types[cand][0]}::{cand}", ""
    ev = any_evidence(name)
    if ev:
        return "covered", ", ".join(ev[:3]), "named in pltrs docs/porting guide"
    return "review", "", "no pltrs function of this name"


out = open(f"{D}/match.jsonl", "w")
counts = collections.Counter()
axes_kwarg_status = {}
for rec in mpl:
    if rec["kind"] == "kwarg" and rec["module"] == "matplotlib.axes" and rec["owner"] == "Axes":
        axes_kwarg_status[rec["name"]] = verdict(rec)
method_status = {}
for rec in mpl:
    if rec["kind"] != "kwarg":
        method_status[(rec["module"], rec["owner"], rec["name"])] = verdict(rec)
for rec in mpl:
    status, item, note = verdict(rec)
    if rec["kind"] == "kwarg" and status == "review":
        method = rec["name"].split("(")[0]
        parent = method_status.get((rec["module"], rec["owner"], method))
        if parent and parent[0] in ("n/a", "missing", "deprecated", "differently"):
            status, item, note = parent[0], parent[1], f"parameter of `{method}`: {parent[2]}"
    rec.update(status=status, rust=item, note=note)
    counts[status] += 1
    out.write(json.dumps(rec) + "\n")
print(dict(counts))

# Check that the pltrs items the rules name exist.
known_types = set(types) | set(members)
bad = set()
for pattern, (status, item, note) in RULES + KWARG_RULES + OWNER_RULES:
    for owner, name in re.findall(r"\b([A-Z][A-Za-z0-9]+)::([a-z_][a-z0-9_]*)\b", item):
        if name not in members.get(owner, {}):
            bad.add(f"{owner}::{name}  ({pattern})")
    for mod, fn in re.findall(r"(?<![:\w])([a-z_]+)::([a-z_][a-z0-9_]*)\b", item):
        if fn not in functions and mod not in ("pltrs_core", "pltrs_text", "pltrs_render", "pltrs", "gui", "pltrs_font"):
            bad.add(f"{mod}::{fn}  ({pattern})")
    for typ in re.findall(r"(?<![:\w])([A-Z][A-Za-z0-9]+)(?![\w])", item):
        if typ not in known_types and typ not in ("None", "Some", "Vec", "Option"):
            bad.add(f"type {typ}  ({pattern})")
for b in sorted(bad):
    print("UNKNOWN", b)
