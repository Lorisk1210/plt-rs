"""
How a row describes a value, and how rows are recorded.

A description names the number type (Python ``int``/``float``/``bool``,
``np.<type>`` for a numpy scalar, ``ndarray[<dtype>,<shape>]``,
``MaskedArray[...]``, ``list[...]``/``tuple[...]`` with the descriptions of
their first items) and the value rounded, so that two runs compare row by
row. `types_of` strips the values, leaving what this sweep is about.
"""

import json
import re
import warnings

import numpy as np


def describe(value, depth=0):
    """A comparable description of *value*'s number type and value."""
    if isinstance(value, np.ma.MaskedArray):
        head = ""
        if value.dtype.kind in "biuf":
            filled = np.ma.filled(value.astype(float), np.nan).ravel()[:3]
            head = f"={filled.round(4).tolist()}"
        return f"MaskedArray[{value.dtype},{value.shape}]{head}"
    if isinstance(value, np.ndarray):
        head = value.ravel()[:3]
        shown = (head.round(4).tolist() if value.dtype.kind in "biuf"
                 else [str(item)[:20] for item in head])
        return f"ndarray[{value.dtype},{value.shape}]={shown}"
    if isinstance(value, np.generic):
        shown = round(float(value), 4) if value.dtype.kind in "biuf" else str(value)
        return f"np.{type(value).__name__}={shown}"
    if isinstance(value, bool):
        return f"bool={value}"
    if isinstance(value, int):
        return f"int={value}"
    if isinstance(value, float):
        return f"float={round(value, 4)}"
    if isinstance(value, (list, tuple)):
        if depth > 1:
            return f"{type(value).__name__}[{len(value)}]"
        items = ", ".join(describe(item, depth + 1) for item in list(value)[:4])
        return f"{type(value).__name__}[{len(value)}]({items})"
    if isinstance(value, dict):
        keys = sorted(value, key=str)[:3]
        items = ", ".join(f"{describe(key, depth + 1)}: {describe(value[key], depth + 1)}"
                          for key in keys)
        return "dict{" + items + "}"
    if value is None:
        return "None"
    if isinstance(value, str):
        return f"str={value[:30]!r}"
    name = type(value).__name__
    if name in ("Bbox", "TransformedBbox"):
        return f"{name}({describe(value.get_points(), depth + 1)})"
    return name


def layout(value):
    """How an array a callback is handed lies in memory: C- or
    Fortran-contiguous or neither, and writeable or not."""
    flags = value.flags
    order = "C" if flags.c_contiguous else "F" if flags.f_contiguous else "-"
    return f"{order}{'w' if flags.writeable else 'r'}"


#: The tokens of a description that name a type.
TYPE_TOKENS = re.compile(
    r"MaskedArray\[\w+|ndarray\[\w+|np\.\w+|\bint\b|\bfloat\b|\bbool\b|\blist\b|\btuple\b"
    r"|\bdict\b|\bstr\b|\bNone\b|\braises \w+|_StickyEdgeList")


def types_of(description):
    """What *description* says of types and shapes, without the values."""
    without_texts = re.sub(r"=('[^']*'|\"[^\"]*\")", "", description)
    return re.sub(r"=(\[[^\]]*\]|[^,)\]\"]*)", "", without_texts)


def kind_of(theirs, ours):
    """A name for the kind of a difference between matplotlib's
    description *theirs* and pltrs' *ours*: the type tokens that differ
    (matplotlib's -> pltrs'), ``structure`` where their numbers differ,
    ``shape`` or ``layout`` where only shapes or memory layouts do, and
    ``value`` where the types are the same."""
    if ours is None:
        return "row missing"
    if theirs.startswith("raises") or ours.startswith("raises"):
        return "raises"
    found_theirs, found_ours = TYPE_TOKENS.findall(theirs), TYPE_TOKENS.findall(ours)
    if found_theirs == found_ours:
        if types_of(theirs) != types_of(ours):
            return "layout" if "layout" in theirs else "shape"
        return "value"
    pairs = sorted({f"{a} -> {b}" for a, b in zip(found_theirs, found_ours) if a != b})
    if len(found_theirs) != len(found_ours):
        pairs.append("structure")
    return "; ".join(pairs).replace("[", " ")


class Rows:
    """The rows of one run: ``{"<surface> | <what>": description}``."""

    def __init__(self):
        self.found = {}

    def row(self, surface, what, read):
        """Record what *read()* returns (or the type of what it raises) as
        the row *surface | what*."""
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                value = describe(read())
        except Exception as error:  # noqa: BLE001 - a refusal is a row's value
            value = f"raises {type(error).__name__}"
        self.found[f"{surface} | {what}"] = value

    def seen(self, surface):
        """A `Seen` whose calls become rows of *surface* once it is
        `Seen.close`d."""
        return Seen(self, surface)


class Seen:
    """The descriptions of the arguments a program's callbacks get: per
    callback, the set of argument types over all its calls and its first
    call (how many calls is not this sweep's business), and for arrays how
    they lie in memory."""

    def __init__(self, rows, surface):
        self.rows = rows
        self.surface = surface
        self.calls = {}

    def note(self, name, *args):
        """Note a call of the callback *name* with *args*; returns None, so
        that a lambda can note and go on (``note(...) or result``)."""
        described = tuple(describe(arg) for arg in args)
        layouts = tuple(layout(arg) for arg in args if isinstance(arg, np.ndarray))
        self.calls.setdefault(name, []).append((described, layouts))

    def close(self):
        """Turn the calls noted into rows."""
        for name, calls in sorted(self.calls.items()):
            kinds = sorted({tuple(types_of(item) for item in described)
                            for described, _ in calls})
            self.rows.found[f"{self.surface} | {name} argument types"] = json.dumps(kinds)
            self.rows.found[f"{self.surface} | {name} first call"] = json.dumps(calls[0][0])
            layouts = sorted({layouts for _, layouts in calls if layouts})
            if layouts:
                self.rows.found[f"{self.surface} | {name} array layout"] = (
                    "layout " + json.dumps(layouts))
        self.calls.clear()
