"""Colour specification parsing and conversion (``matplotlib.colors``).

Area ``colors``. Every op mirrors one public ``matplotlib.colors`` function.

Argument conventions
--------------------
* A *colour spec* argument (``c``, ``c1``, ``c2`` and every element of a
  colour list) is passed to matplotlib with **JSON arrays converted to Python
  tuples, recursively**. So ``[1, 0, 0]`` is the RGB tuple ``(1, 0, 0)`` and
  ``["red", 0.5]`` is the ``(colour, alpha)`` pair ``("red", 0.5)``. JSON
  numbers stay numbers (a bare number is *not* a valid colour).
* A *colour list* argument (``to_rgba_array``'s ``c``) is a JSON array whose
  elements are colour specs; it is passed as a Python ``list`` unless
  ``tuple: true`` (then the top level is a tuple too, which enables the
  ``(colour, alpha)`` pair form) or ``ndarray: true`` (then the whole value is
  ``np.asarray(c, dtype=float)``, i.e. an ``(n, 3|4)`` float array, which takes
  matplotlib's vectorised fast path with range checking). A JSON string is
  passed as-is.
* Results: RGBA tuples become ``[r, g, b, a]``; arrays become nested lists.
  Exceptions are recorded as ``{"error": "<ExceptionType>"}``.

The reference configuration is matplotlib's defaults, so ``"C0"``..``"C9"``
resolve through the default ``axes.prop_cycle`` (the tab10 colours).

Ops
---
``colors.to_rgba``            {c, alpha?} -> [r, g, b, a]
``colors.to_rgba_array``      {c, alpha?, tuple?, ndarray?} -> [[r, g, b, a], ...]
``colors.to_rgb``             {c} -> [r, g, b]
``colors.to_hex``             {c, keep_alpha?} -> "#rrggbb" / "#rrggbbaa"
``colors.is_color_like``      {c} -> bool
``colors.same_color``         {c1, c2} -> bool (colour lists as for to_rgba_array)
``colors.rgb_to_hsv``         {arr} -> same shape; arr is ``np.asarray(arr, float)``
``colors.hsv_to_rgb``         {hsv} -> same shape; hsv is ``np.asarray(hsv, float)``
``colors.BASE_COLORS``        {} -> {name: [r, g, b]}
``colors.TABLEAU_COLORS``     {} -> {name: "#rrggbb"}
``colors.CSS4_COLORS``        {} -> {name: "#RRGGBB"}
``colors.XKCD_COLORS``        {} -> {"xkcd:<name>": "#rrggbb"}
``colors.get_named_colors_mapping`` {} -> the merged lookup table used by
                              ``to_rgba`` (name -> raw table value). It is
                              built as XKCD, then XKCD with 'grey'->'gray',
                              then CSS4, then TABLEAU, then TABLEAU with
                              'gray'->'grey', then BASE (later wins).
``colors.color_sequences``    {} -> ordered list of registered sequence names
``colors.color_sequence``     {name} -> [[r, g, b, a], ...] (``to_rgba`` of each entry)
"""

import math

import matplotlib as mpl
import matplotlib.colors as mcolors
import numpy as np
import pytest

from spec_support import ProbeSet

probes = ProbeSet("colors")


def _spec(value):
    """JSON value -> matplotlib colour spec (arrays become tuples)."""
    if isinstance(value, list):
        return tuple(_spec(v) for v in value)
    return value


def _spec_list(value, tuple_=False, ndarray=False):
    """JSON value -> matplotlib colour list (see module docstring)."""
    if ndarray:
        return np.asarray(value, dtype=float)
    if isinstance(value, list):
        items = [_spec(v) for v in value]
        return tuple(items) if tuple_ else items
    return value


def to_rgba(c, alpha=None):
    return mcolors.to_rgba(_spec(c), alpha)


def to_rgba_array(c, alpha=None, tuple=False, ndarray=False):  # noqa: A002 - JSON arg name
    return mcolors.to_rgba_array(_spec_list(c, tuple, ndarray), alpha)


def to_rgb(c):
    return mcolors.to_rgb(_spec(c))


def to_hex(c, keep_alpha=False):
    return mcolors.to_hex(_spec(c), keep_alpha=keep_alpha)


def is_color_like(c):
    return bool(mcolors.is_color_like(_spec(c)))


def same_color(c1, c2):
    return bool(mcolors.same_color(_spec_list(c1), _spec_list(c2)))


# --------------------------------------------------------------------------
# Spec corpora
# --------------------------------------------------------------------------

XKCD_SPECIAL = [
    "xkcd:sky blue",
    "xkcd:robin's egg blue",
    "xkcd:grey",
    "xkcd:gray",
    "xkcd:greyish",
    "xkcd:grayish",
    "xkcd:blue/green",
    "xkcd:light grey green",
    "xkcd:light gray green",
    "xkcd:cloudy blue",
    "xkcd:purple",
    "xkcd:green",
    "xkcd:white",
    "xkcd:black",
]

CASE_VARIANTS = [
    "Red",
    "RED",
    "rEd",
    "DarkSlateGray",
    "DARKSLATEGREY",
    "AliceBlue",
    "TAB:BLUE",
    "Tab:Orange",
    "XKCD:SKY BLUE",
    "xkcd:Sky Blue",
    "B",
    "R",
    "K",
    "W",
]

CYCLE_SPECS = [f"C{i}" for i in range(10)] + [
    "C10",
    "C11",
    "C19",
    "C100",
    "C01",
    "C007",
    "C",
    "c0",
    "C-1",
    "C1.5",
    "C 1",
    " C1",
]

GREY_STRINGS = [
    "0",
    "1",
    "0.0",
    "1.0",
    "0.5",
    "0.25",
    ".5",
    "0.75",
    "1e-1",
    "5E-1",
    " 0.5",
    "0.5 ",
    "0.5_0",
    "0.333",
    "-0",
    "-0.1",
    "1.1",
    "2",
    "nan",
    "inf",
    "-inf",
]

HEX_STRINGS = [
    "#ff0000",
    "#FF0000",
    "#00ff00",
    "#0000ff",
    "#123456",
    "#abcdef",
    "#ABCDEF",
    "#aBcDeF",
    "#000000",
    "#ffffff",
    "#808080",
    "#f00",
    "#F00",
    "#0f0",
    "#00f",
    "#123",
    "#fff",
    "#000",
    "#ff000080",
    "#FF000080",
    "#00ff0000",
    "#0000ffff",
    "#12345678",
    "#f008",
    "#0f0f",
    "#00f0",
    "#ff00",
    "#1234",
    "#ff000",
    "#ff00000",
    "#ff0000ff0",
    "#ff0000ff00",
    "#",
    "#ff",
    "#gggggg",
    "#ff00zz",
    "ff0000",
    "#ff 000",
    "##ff0000",
]

TUPLE_SPECS = [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
    [0.5, 0.5, 0.5],
    [0.1, 0.2, 0.3],
    [1, 0, 0, 0.5],
    [0.1, 0.2, 0.3, 0.4],
    [0, 0, 0, 0],
    [1, 1, 1, 1],
    [1.0, 0.0, 0.0],
    [float("nan"), 0, 0],
    [0, 0, 0, float("nan")],
    [1.5, 0, 0],
    [-0.1, 0, 0],
    [0, 0, 0, 1.01],
    [0, 0, 0, -0.01],
    [float("inf"), 0, 0],
    [1, 0],
    [1, 0, 0, 0, 0],
    [],
    ["1", "0", "0"],
    ["red", 0.5],
    ["red", 0],
    ["red", 1],
    ["#ff000080", 0.5],
    ["#ff000080", None],
    ["red", None],
    [[1, 0, 0], 0.5],
    [[1, 0, 0, 0.3], 0.5],
    [[1, 0, 0, 0.3], None],
    ["C1", 0.25],
    ["none", 0.5],
    ["red", 1.5],
    ["red", -0.5],
    [["red", 0.3], 0.5],
    ["notacolor", 0.5],
    [[0, 0, 0], [0, 0, 0]],
]

SPECIAL_STRINGS = [
    "none",
    "None",
    "NONE",
    "nOnE",
    "",
    " ",
    "notacolor",
    "transparent",
    "sky blue",
    "redd",
    " red",
    "red ",
    "tab:",
    "xkcd:",
    "tab:notacolor",
    "xkcd:notacolor",
    "css4:red",
]

NON_STRING_SPECS = [0, 1, 0.5, None, True]


def _xkcd_sample():
    names = list(mcolors.XKCD_COLORS)
    return names[::10] + XKCD_SPECIAL


def _all_named_specs():
    tableau_greys = [k.replace("gray", "grey") for k in mcolors.TABLEAU_COLORS if "gray" in k]
    return (
        list(mcolors.BASE_COLORS)
        + list(mcolors.TABLEAU_COLORS)
        + tableau_greys
        + list(mcolors.CSS4_COLORS)
        + _xkcd_sample()
    )


def _mixed_corpus():
    """A representative cross-section of every kind of spec (for derived ops)."""
    return (
        ["red", "r", "k", "w", "tab:blue", "tab:grey", "xkcd:sky blue", "rebeccapurple", "Navy"]
        + CYCLE_SPECS
        + GREY_STRINGS
        + HEX_STRINGS
        + TUPLE_SPECS
        + SPECIAL_STRINGS
        + NON_STRING_SPECS
    )


# --------------------------------------------------------------------------
# to_rgba
# --------------------------------------------------------------------------


@pytest.mark.parametrize(
    "spec, expected",
    [
        ("red", (1.0, 0.0, 0.0, 1.0)),
        ("#00ff00", (0.0, 1.0, 0.0, 1.0)),
        ("#00f8", (0.0, 0.0, 1.0, 0x88 / 255)),
        ("0.25", (0.25, 0.25, 0.25, 1.0)),
        ("none", (0.0, 0.0, 0.0, 0.0)),
        ("g", (0.0, 0.5, 0.0, 1.0)),
        ("green", (0.0, 128 / 255, 0.0, 1.0)),
        ("tab:blue", (0x1F / 255, 0x77 / 255, 0xB4 / 255, 1.0)),
        ("C0", (0x1F / 255, 0x77 / 255, 0xB4 / 255, 1.0)),
        ("C10", (0x1F / 255, 0x77 / 255, 0xB4 / 255, 1.0)),
        ("RED", (1.0, 0.0, 0.0, 1.0)),
        ("xkcd:sky blue", (0x75 / 255, 0xBB / 255, 0xFD / 255, 1.0)),
    ],
)
def test_to_rgba_known_values(spec, expected):
    assert probes.probe("colors.to_rgba", to_rgba, c=spec) == pytest.approx(expected)


def test_to_rgba_rejects_garbage():
    for spec in ["notacolor", "R", "c0", "sky blue", "#ff000", "1.5", "-0.1", "nan", ""]:
        assert isinstance(probes.probe("colors.to_rgba", to_rgba, c=spec), ValueError), spec


def test_to_rgba_named_tables():
    for name in _all_named_specs():
        probes.probe("colors.to_rgba", to_rgba, c=name)
    for name in CASE_VARIANTS:
        probes.probe("colors.to_rgba", to_rgba, c=name)


def test_to_rgba_cycle_grey_hex_tuple_special():
    for spec in CYCLE_SPECS + GREY_STRINGS + HEX_STRINGS + TUPLE_SPECS + SPECIAL_STRINGS + NON_STRING_SPECS:
        probes.probe("colors.to_rgba", to_rgba, c=spec)


def test_to_rgba_semantics():
    # 'grey'/'gray' spellings are interchangeable in xkcd and tableau names.
    assert to_rgba("xkcd:grey") == to_rgba("xkcd:gray")
    assert to_rgba("tab:grey") == to_rgba("tab:gray")
    # Single-letter names are case sensitive; longer names are not.
    assert isinstance(probes.probe("colors.to_rgba", to_rgba, c="R"), ValueError)
    assert to_rgba("ReD") == (1.0, 0.0, 0.0, 1.0)
    # A pair's alpha overrides the colour's own alpha.
    assert to_rgba(["#ff000080", 0.25])[3] == 0.25
    # "none" ignores alpha.
    assert to_rgba("none", 0.5) == (0.0, 0.0, 0.0, 0.0)
    # NaN components pass the range check.
    assert math.isnan(to_rgba([float("nan"), 0, 0])[0])


def test_to_rgba_alpha_override():
    specs = ["red", "#ff000080", "#f008", "0.5", "C3", "tab:green", "xkcd:sky blue", "none", "None",
             [1, 0, 0], [1, 0, 0, 0.2], ["red", 0.3], ["#ff000080", None]]
    for spec in specs:
        for alpha in [0.0, 0.25, 0.5, 1.0, 1.5, -0.25, float("nan")]:
            probes.probe("colors.to_rgba", to_rgba, c=spec, alpha=alpha)
        probes.probe("colors.to_rgba", to_rgba, c=spec, alpha=None)


# --------------------------------------------------------------------------
# to_rgb / to_hex / is_color_like
# --------------------------------------------------------------------------


def test_to_rgb():
    for spec in _mixed_corpus():
        probes.probe("colors.to_rgb", to_rgb, c=spec)
    assert to_rgb("#ff000080") == (1.0, 0.0, 0.0)


def test_to_hex():
    for spec in _mixed_corpus():
        probes.probe("colors.to_hex", to_hex, c=spec)
        probes.probe("colors.to_hex", to_hex, c=spec, keep_alpha=True)
    # Python's round() (half-to-even) is applied to value * 255.
    halves = [[k / 510, (k + 2) / 510, (k + 4) / 510] for k in range(1, 510, 6)]
    for rgb in halves:
        probes.probe("colors.to_hex", to_hex, c=rgb)
    for value in [0.5, 0.001, 0.002, 0.998, 0.999, 1 / 3, 2 / 3, 0.1, 0.9]:
        probes.probe("colors.to_hex", to_hex, c=[value, value, value, value], keep_alpha=True)
    assert to_hex([0.5, 0.5, 0.5]) == "#808080"
    assert to_hex("red", keep_alpha=True) == "#ff0000ff"
    assert to_hex(["red", 0.5], keep_alpha=True) == "#ff000080"


def test_is_color_like():
    for spec in _mixed_corpus() + CASE_VARIANTS + XKCD_SPECIAL:
        probes.probe("colors.is_color_like", is_color_like, c=spec)
    assert is_color_like("C99")
    assert not is_color_like("c1")
    assert not is_color_like(0.5)


# --------------------------------------------------------------------------
# to_rgba_array / same_color
# --------------------------------------------------------------------------


def test_to_rgba_array():
    cases = [
        dict(c="red"),
        dict(c="none"),
        dict(c="None"),
        dict(c="notacolor"),
        dict(c="C2"),
        dict(c="0.5"),
        dict(c="#ff000080"),
        dict(c="#ff000080", alpha=0.5),
        dict(c="red", alpha=0.5),
        dict(c="red", alpha=[0.1, 0.5, 1.0]),
        dict(c="red", alpha=[]),
        dict(c="red", alpha=1.5),
        dict(c="none", alpha=0.5),
        dict(c="none", alpha=[0.1, 0.2]),
        dict(c=[]),
        dict(c=["red"]),
        dict(c=["red", "green", "blue"]),
        dict(c=["red", "none", "blue"]),
        dict(c=["red", "none", "blue"], alpha=0.5),
        dict(c=["red", "none", "blue"], alpha=[0.1, 0.2, 0.3]),
        dict(c=["red", "green"], alpha=[0.1, 0.2, 0.3]),
        dict(c=["red", "green"], alpha=[0.1, 0.2, 0.3], tuple=True),
        dict(c=["C0", "C1", "C2", "C10"]),
        dict(c=["tab:blue", "xkcd:sky blue", "rebeccapurple", "0.25", "#abc", "#abcd"]),
        dict(c=["red", "notacolor"]),
        dict(c=["red", [0, 1, 0]]),
        dict(c=["red", [0, 1, 0, 0.5], ["blue", 0.25]]),
        dict(c=[[1, 0, 0], [0, 1, 0]]),
        dict(c=[[1, 0, 0], [0, 1, 0]], alpha=0.5),
        dict(c=[[1, 0, 0, 0.5], [0, 1, 0, 1]]),
        dict(c=[[1, 0, 0, 0.5], [0, 1, 0, 1]], alpha=[0.1, 0.9]),
        dict(c=[[1, 0, 0], [0, 1, 0, 0.5]]),
        dict(c=[[2, 0, 0, 1], [0, 1, 0, 1]]),
        dict(c=[[2, 0, 0], [0, 1, 0]]),
        dict(c=[[2, 0, 0], [0, 1, 0, 0.5]]),
        dict(c=[[1, 0], [0, 1]]),
        dict(c=[[1, 0, 0]], ndarray=True),
        dict(c=[[1, 0, 0], [0, 1, 0]], ndarray=True),
        dict(c=[[1, 0, 0], [0, 1, 0]], alpha=0.5, ndarray=True),
        dict(c=[[1, 0, 0], [0, 1, 0]], alpha=[0.25, 0.75], ndarray=True),
        dict(c=[[1, 0, 0]], alpha=[0.25, 0.75], ndarray=True),
        dict(c=[[1, 0, 0], [0, 1, 0]], alpha=[0.25, 0.5, 0.75], ndarray=True),
        dict(c=[[1, 0, 0, 0.5], [0, 1, 0, 1]], ndarray=True),
        dict(c=[[1, 0, 0, 0.5], [0, 1, 0, 1]], alpha=0.1, ndarray=True),
        dict(c=[[2, 0, 0, 1], [0, 1, 0, 1]], ndarray=True),
        dict(c=[[-1, 0, 0]], ndarray=True),
        dict(c=[[float("nan"), 0, 0]], ndarray=True),
        dict(c=[1, 0, 0]),
        dict(c=[1, 0, 0], tuple=True),
        dict(c=[1, 0, 0, 0.5], tuple=True),
        dict(c=["red", 0.5], tuple=True),
        dict(c=["red", 0.5]),
        dict(c=[[1, 0, 0], 0.5], tuple=True),
        dict(c=["red", 0.5], alpha=0.9, tuple=True),
        dict(c=["red", "blue"], tuple=True),
        dict(c=[0.5, 0.5], tuple=True),
        dict(c=[0.5, 0.5]),
        dict(c=["0.5", "0.75"]),
    ]
    for case in cases:
        probes.probe("colors.to_rgba_array", to_rgba_array, **case)
    assert to_rgba_array("none").shape == (0, 4)
    # Nested 4-lists bypass the range check (only the ndarray path checks).
    assert to_rgba_array([[2, 0, 0, 1], [0, 1, 0, 1]])[0][0] == 2
    assert isinstance(probes.probe("colors.to_rgba_array", to_rgba_array,
                                   c=[[2, 0, 0, 1], [0, 1, 0, 1]], ndarray=True), ValueError)


def test_same_color():
    cases = [
        ("red", "r"),
        ("red", "#ff0000"),
        ("red", "#ff0000ff"),
        ("red", "#ff0000fe"),
        ("red", [1, 0, 0]),
        ("red", [1, 0, 0, 1]),
        ("C0", "tab:blue"),
        ("C0", "#1f77b4"),
        ("grey", "gray"),
        ("xkcd:grey", "xkcd:gray"),
        ("green", "g"),
        ("none", "none"),
        ("none", "red"),
        ("none", [0, 0, 0, 0]),
        ("none", "None"),
        (["red", "blue"], ["r", "b"]),
        (["red", "blue"], ["b", "r"]),
        (["red", "blue"], ["red"]),
        (["red", "blue"], "red"),
        ("0.5", [0.5, 0.5, 0.5]),
        ("notacolor", "red"),
    ]
    for c1, c2 in cases:
        probes.probe("colors.same_color", same_color, c1=c1, c2=c2)
    assert same_color("C0", "tab:blue")
    assert not same_color("red", "#ff0000fe")


# --------------------------------------------------------------------------
# HSV conversions
# --------------------------------------------------------------------------

RGB_SAMPLES = [
    [0, 0, 0],
    [1, 1, 1],
    [0.5, 0.5, 0.5],
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 1, 0],
    [0, 1, 1],
    [1, 0, 1],
    [1, 0.5, 0],
    [1, 0, 0.5],
    [0.5, 1, 0],
    [0, 1, 0.5],
    [0.5, 0, 1],
    [0, 0.5, 1],
    [0.2, 0.4, 0.6],
    [0.6, 0.4, 0.2],
    [0.3, 0.3, 0.7],
    [0.9, 0.1, 0.1],
    [0.1, 0.9, 0.9],
    [0.25, 0.75, 0.75],
    [0.123, 0.456, 0.789],
    [1e-9, 0, 0],
    [1, 1, 0.999999],
]


def _rgb_to_hsv(arr):
    return mcolors.rgb_to_hsv(np.asarray(arr, dtype=float))


def _hsv_to_rgb(hsv):
    return mcolors.hsv_to_rgb(np.asarray(hsv, dtype=float))


def test_rgb_to_hsv():
    for rgb in RGB_SAMPLES:
        probes.probe("colors.rgb_to_hsv", _rgb_to_hsv, arr=rgb)
    probes.probe("colors.rgb_to_hsv", _rgb_to_hsv, arr=RGB_SAMPLES)
    probes.probe("colors.rgb_to_hsv", _rgb_to_hsv, arr=[RGB_SAMPLES[:4], RGB_SAMPLES[4:8]])
    for bad in [[1.5, 0, 0], [-0.5, 0, 0], [1, 0], [1, 0, 0, 1]]:
        assert isinstance(probes.probe("colors.rgb_to_hsv", _rgb_to_hsv, arr=bad), ValueError)
    assert list(_rgb_to_hsv([1, 0, 0])) == [0.0, 1.0, 1.0]


def test_hsv_to_rgb():
    hsv_samples = [
        [h / 12, s, v]
        for h in range(13)
        for s, v in [(1, 1), (0.5, 1), (1, 0.5), (0.25, 0.75)]
    ] + [
        [0, 0, 0],
        [0.3, 0, 0.7],
        [0.999999, 1, 1],
        [1 / 6, 1, 1],
        [0.5, 1, 1],
        [0.123, 0.456, 0.789],
        [0.7, 0.2, 0.0],
    ]
    for hsv in hsv_samples:
        probes.probe("colors.hsv_to_rgb", _hsv_to_rgb, hsv=hsv)
    probes.probe("colors.hsv_to_rgb", _hsv_to_rgb, hsv=hsv_samples)
    probes.probe("colors.hsv_to_rgb", _hsv_to_rgb, hsv=[hsv_samples[:3], hsv_samples[3:6]])
    for bad in [[1, 0], [0, 0, 0, 0]]:
        assert isinstance(probes.probe("colors.hsv_to_rgb", _hsv_to_rgb, hsv=bad), ValueError)
    # Round trip through the HSV conversion of every RGB sample.
    for rgb in RGB_SAMPLES:
        hsv = _rgb_to_hsv(rgb).tolist()
        probes.probe("colors.hsv_to_rgb", _hsv_to_rgb, hsv=hsv)


# --------------------------------------------------------------------------
# Named-colour tables and colour sequences
# --------------------------------------------------------------------------


def test_named_color_tables():
    tables = {
        "colors.BASE_COLORS": mcolors.BASE_COLORS,
        "colors.TABLEAU_COLORS": mcolors.TABLEAU_COLORS,
        "colors.CSS4_COLORS": mcolors.CSS4_COLORS,
        "colors.XKCD_COLORS": mcolors.XKCD_COLORS,
    }
    for op, table in tables.items():
        probes.probe(op, lambda table=table: dict(table))
    full = probes.probe("colors.get_named_colors_mapping", lambda: dict(mcolors.get_named_colors_mapping()))
    assert len(mcolors.CSS4_COLORS) == 148
    assert full["xkcd:grey"] == full["xkcd:gray"]
    assert full["grey"] == mcolors.CSS4_COLORS["grey"]
    assert full["g"] == (0, 0.5, 0)


def _color_sequence(name):
    return [mcolors.to_rgba(c) for c in mpl.color_sequences[name]]


def test_color_sequences():
    names = probes.probe("colors.color_sequences", lambda: list(mpl.color_sequences))
    for name in names:
        probes.probe("colors.color_sequence", _color_sequence, name=name)
    assert isinstance(probes.probe("colors.color_sequence", _color_sequence, name="nope"), KeyError)
