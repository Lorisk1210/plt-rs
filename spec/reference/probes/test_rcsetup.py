"""Validating rcParams from rc-file text (``matplotlib.rcsetup``), as style
sheets and ``matplotlibrc`` files set them.

Area ``rcsetup``. Every parameter is tried with candidate texts chosen by
its validator: typical values, other spellings, edge cases and invalid
input.

Ops
---
``rc.validate``  {key, value} -> canonical value, or an error when rejected
``style.sheet``  {name} -> {key: canonical value} for every parameter the
                 public style sheet ``name`` sets, keys sorted

Canonical values compare meaning rather than Python representation:

* ``None``, booleans, numbers and strings as they are; lists and tuples as
  lists.
* Colours (any parameter validated as a colour) as ``[r, g, b, a]``, except
  ``"Cn"`` cycle references and keywords such as ``"auto"``, kept as text.
* Line styles as ``"solid"``, ``"dashed"``, ``"dashdot"``, ``"dotted"`` or
  ``"none"``, or ``[offset, [on, off, ...]]``.
* Cap and join styles by name; cyclers as ``{"cycler": [[property,
  [values...]], ...]}``, properties sorted (a product's key order is a
  Python set's), each value canonical for its property.
* ``legend.labelcolor`` keeps ``None`` (the text colour) as ``None``, and
  ``*.minor.ndivs`` its ``auto`` in lower case.

Left out where pltrs is stricter on purpose: marker texts that name no
marker and font weights or stretches outside 0-1000, which matplotlib only
rejects when a figure uses them, and eight-character colour names for
``legend.labelcolor``, which an operator-precedence slip in
``_validate_color_or_linecolor`` rejects.
* Markers as matplotlib keeps them, an int code or the marker text, with
  every spelling of "no marker" as ``"None"``.
"""

import re

import matplotlib as mpl
import matplotlib.style
from cycler import Cycler
from matplotlib import colors as mcolors
from matplotlib import rcsetup

from spec_support import ProbeSet

probes = ProbeSet("rcsetup")

# Parameters left out: the backend is pyplot's business, and path.effects
# only holds Python objects (any rc-file text becomes a list of strings).
SKIP = {"backend", "path.effects"}

COLORS = ["red", "#ff0000", "ff0000", "FF000080", "0.5", "1.5", "C3", "C12", "none", "None",
          "(1, 0, 0)", "(1, 0, 0, 0.5)", "(1, 0)", "tab:blue", "xkcd:sky blue", "abc", "#abc",
          "#abcd", "auto", "inherit", "", "Red", "k",
          # From matplotlib's test_rcparams.py.
          "AABBCC00", "tab:veryblue", "(0, 1, 0, 1, 0)", "(0, 1, none)", '(0, 1, "0.5")',
          "1, 0, 0"]
LINESTYLES = ["-", "--", "-.", ":", "solid", "dashed", "dashdot", "dotted", "Dashed", "None",
              "none", " ", "", "(0, (4, 2))", "(1.5, (1, 2, 3, 4))", "(0, [4, 2])", "(0, ())",
              "(0, None)", "bogus", "[4, 2]", "4, 2", "DoTtEd", "1, 3", "1, 2, 3",
              '("a", [1, 2])', "(1, [1, 2, 3])", "([1, 2], 1)", "(-1, [1, 2])"]
FONTSIZES = ["12", "12.5", "large", "x-small", "xx-large", "smaller", "larger", "medium",
             "huge", "-3", "Large", "0"]

CANDIDATES = {
    "validate_bool": ["True", "false", "yes", "NO", "on", "1", "0", "t", "F", "maybe", "",
                      "2"],
    "validate_float": ["1", "-2.5", "1e3", "inf", "-inf", "nan", "0x10", "abc", "", "1,2",
                       " 3 ", "1_000"],
    "_validate_greaterthan_minushalf": ["0", "-0.5", "-0.4", "2", "x"],
    "_validate_greaterequal0_lessequal1": ["0", "1", "0.5", "-0.1", "1.1", "nan"],
    "validate_float_or_None": ["None", "none", "0.5", "abc", ""],
    "validate_int": ["3", "-4", "2.5", "1e3", "abc", " 7 ", "1_000", ""],
    "validate_int_or_None": ["None", "5", "x"],
    "validate_str": ["hello", "None", "", "a, b"],
    "validate_str_or_None": ["None", "x", "none"],
    "validate_strlist": ["a, b", "a,,b", "", "single", " x , y ", "[a, b]"],
    "validate_floatlist": ["1, 2", "1", "[1, 2]", "(1, 2)", "1, x", "", "1, 2, 3", "1,,2"],
    "validate_intlist": ["-5, 6", "1", "1, 2, 3", "1.5, 2", "[-5, 6]"],
    "validate_color": COLORS,
    "validate_color_or_auto": COLORS + ["AUTO"],
    "validate_color_or_inherit": COLORS + ["INHERIT"],
    "_validate_color_or_None": COLORS,
    "_validate_color_or_edge": COLORS + ["edge", "Edge"],
    "_validate_color_or_linecolor": [c for c in COLORS if len(c) != 8 or c.startswith("FF")]
    + ["linecolor", "markerfacecolor", "mfc", "markeredgecolor", "mec", "LineColor", "line",
       "marker"],
    "_validate_linestyle": LINESTYLES,
    "_validate_linestyle_or_None": LINESTYLES,
    "validate_fontsize": FONTSIZES,
    "validate_fontsize_None": FONTSIZES + ["None"],
    "validate_fontweight": ["bold", "normal", "100", "950", "1000", "heavy", "ultralight",
                            "Bold", "bolder", "bogus", "400.5", "extra bold"],
    "validate_fontstretch": ["condensed", "normal", "500", "ultra-expanded", "bogus",
                             "Condensed"],
    "validate_font_properties": ["DejaVu Sans:bold", "serif-12", "Arial", "a:b=c"],
    "_validate_marker": ["o", "s", "$x$", "None", "none", "", "4", "8", "11", "0", "D"],
    "JoinStyle": ["miter", "round", "bevel", "Round", "mitre", "bogus"],
    "CapStyle": ["butt", "round", "projecting", "Butt", "square", "bogus"],
    "validate_fonttype": ["3", "42", "1", "type3", "truetype", "Type3", "bogus"],
    "_validate_pathlike": ["~/x", "a/b", ""],
    "_validate_minor_tick_ndivs": ["auto", "4", "0", "-1", "2.5", "bogus", "Auto"],
    "validate_cycler": [
        "cycler('color', ['r', 'g'])",
        "cycler(color='rgb')",
        "cycler(c='rgb')",
        "cycler('color', 'rgb') + cycler('linestyle', ['-', '--', ':'])",
        "cycler('lw', [1, 2])",
        "cycler('color', ['r']) * cycler('marker', ['o', 's'])",
        "3 * cycler('color', ['r'])",
        "cycler('color', 'rg') * 2",
        "2.0 * cycler('color', 'rg')",
        "cycler('color', 'rgb')[1:]",
        "cycler('color', 'rgb')[::-1]",
        "concat(cycler('color', 'r'), cycler('color', 'g'))",
        "cycler('linestyle', ['-', (0, (1, 1))])",
        "cycler('marker', ['o', 4, '$x$'])",
        "cycler('color', ['r']) + cycler('color', ['g'])",
        "cycler('c', 'r') + cycler('color', 'g')",
        "cycler('bogus', [1])",
        "cycler('color', ['notacolor'])",
        "cycler('lw', ['a'])",
        "cycler('dashes', [[1, 2], [3, 4]])",
        "cycler(markevery=[None, 8, (30, 8), [16, 24, 30], 0.1, (0.45, 0.1)])",
        "cycler('markevery', [(1, 0.5)])",
        "cycler('facecolor', ['r', 'none'])",
        "__import__('os')",
        "cycler('color', 'rg') + 2",
        "",
        # From matplotlib's test_rcparams.py (test_validator_valid/invalid).
        'cycler("color", "Dark2")',
        'cycler("color", ["r", "g", "b"]) +\n     cycler("mew", [2, 3, 5])',
        '(cycler("color", ["r", "g", "b"]) +\n     cycler("mew", [2, 3, 5]))',
        "cycler('color', 'rgb')\n",
        "cycler('hatch', ['\\x2f', 'x\\\\\\\\'])",
        "cycler(c='rgb', lw=[1, 2, 3])",
        "cycler('c', 'rgb') * cycler('linestyle', ['-', '--'])",
        "2 * cycler('color', 'r' + 'gb')",
        "cycler(c='r' + 'gb', lw=[1, 2, 3])",
        "concat(cycler('color', 'rgb'), cycler('color', 'cmk'))",
        "cycler('color', 'rgbcmk')[:3]",
        'cycler("bleh, [])',
        "cycler('color', 'rgb') * * cycler('color', 'rgb')",
        'Cycler("linewidth", [1, 2, 3])',
        "cycler('c', [j.__class__(j) for j in ['r', 'b']])",
        "cycler('color', [x for x in ['r', 'g', 'b']])",
        "1 + 2",
        'os.system("echo Gotcha")',
        "import os",
        'cycler("waka", [1, 2, 3])',
        "cycler(c=[1, 2, 3])",
        "cycler(lw=['a', 'b', 'c'])",
        "cycler('color', ['C1', 'r', 'g'])",
        "cycler('hatch', ['--|', '\\\\oO', '/+*/.x', ''])",
        "cycler('hatch', ['--_'])",
    ],
    "validate_hist_bins": ["auto", "10", "1, 2, 3", "sturges", "fd", "bogus", "2.5", "0"],
    "validate_whiskers": ["1.5", "5, 95", "5", "a", "5, 95, 99", "(5, 95)"],
    "validate_aspect": ["auto", "equal", "2", "-1", "0", "bogus", "Equal"],
    "validate_axisbelow": ["line", "True", "false", "bogus", "LINE", "yes"],
    "validate_dpi": ["figure", "100", "72.5", "-1", "bogus", "Figure"],
    "validate_bbox": ["tight", "standard", "Tight", "None", "none", "loose"],
    "validate_sketch": ["None", "none", "(1, 100, 2)", "1, 100, 2", "[1, 100, 2]", "1, 2",
                        "(None)"],
    "validate_ps_distiller": ["None", "ghostscript", "xpdf", "false", "bogus", "Ghostscript",
                              "none"],
    "_validate_mathtext_fallback": ["cm", "stix", "stixsans", "None", "bogus", "STIX"],
    "_validate_cmap": ["viridis", "bogus", "Viridis"],
    "_validate_date": ["2000-01-01", "1970-01-01T00:00", "bogus"],
    "_validate_legend_loc": ["best", "upper right", "1", "10", "11", "(0.5, 0.5)", "center",
                             "bogus", "0.5, 0.5", "Best", "(0.5, 0.5, 1)"],
    "_validate_toolbar": ["None", "toolbar2", "toolmanager", "bogus", "TOOLBAR2"],
}

COLOR_VALIDATORS = {"validate_color", "validate_color_or_auto", "validate_color_or_inherit",
                    "_validate_color_or_None", "_validate_color_or_edge",
                    "_validate_color_or_linecolor"}
LINESTYLE_VALIDATORS = {"_validate_linestyle", "_validate_linestyle_or_None"}
NO_MARKER = {"None", "none", " ", ""}
CYCLE_REF = re.compile(r"^C\d+$")
COLOR_KEYWORDS = {"auto", "inherit", "edge", "linecolor", "markerfacecolor", "mfc",
                  "markeredgecolor", "mec"}
COLOR_PROPERTIES = {"color", "facecolor", "edgecolor", "markerfacecolor", "markeredgecolor"}


def _name(validator):
    return getattr(validator, "__name__", None) or type(validator).__name__


def _plain(value):
    if isinstance(value, (list, tuple)):
        return [_plain(v) for v in value]
    if hasattr(value, "name") and not isinstance(value, str):  # Cap/JoinStyle
        return value.name
    return value


def _color(value):
    if value is None:
        return None
    if isinstance(value, str) and (CYCLE_REF.match(value) or value in COLOR_KEYWORDS):
        return value
    return [float(c) for c in mcolors.to_rgba(value)]


def _linestyle(value):
    if value is None:
        return None
    if isinstance(value, str):
        long = {"-": "solid", "--": "dashed", "-.": "dashdot", ":": "dotted"}
        if value in long:
            return long[value]
        return "none" if value.lower() in {"none", " ", ""} else value.lower()
    offset, dashes = value
    return [float(offset), None if dashes is None else [float(d) for d in dashes]]


def _marker(value):
    return "None" if isinstance(value, str) and value in NO_MARKER else value


def _cycle_value(prop, value):
    if prop in COLOR_PROPERTIES:
        return _color(value)
    if prop == "linestyle":
        return _linestyle(value)
    if prop == "marker":
        return _marker(value)
    return _plain(value)


def _canon(name, value):
    if name == "_validate_color_or_linecolor" and isinstance(value, str) \
            and value.lower() == "none":
        return None
    if name == "_validate_minor_tick_ndivs" and isinstance(value, str):
        return value.lower()
    if name in COLOR_VALIDATORS:
        return _color(value)
    if name in LINESTYLE_VALIDATORS:
        return _linestyle(value)
    if name == "_validate_marker":
        return _marker(value)
    if isinstance(value, Cycler):
        return {"cycler": [[prop, [_cycle_value(prop, v) for v in values]]
                           for prop, values in sorted(value.by_key().items())]}
    return _plain(value)


def _validate(key, value):
    validator = mpl.rcParams.validate[key]
    return _canon(_name(validator), validator(value))


def _candidates(key, validator):
    name = _name(validator)
    if name == "ValidateInStrings":
        valid = sorted(set(validator.valid.values()) if isinstance(validator.valid, dict)
                       else set(validator.valid))
        return valid + [valid[0].upper(), "bogus", ""]
    return CANDIDATES[name]


def test_validate():
    for key, validator in rcsetup._validators.items():
        if key in SKIP:
            continue
        for value in _candidates(key, validator):
            probes.probe("rc.validate", _validate, key=key, value=value)


def _sheet(name):
    sheet = matplotlib.style.library[name]
    return {key: _canon(_name(mpl.rcParams.validate[key]), value)
            for key, value in sorted(sheet.items()) if key not in SKIP}


def test_style_sheets():
    for name in matplotlib.style.available:
        probes.probe("style.sheet", _sheet, name=name)
