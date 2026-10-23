"""Fontconfig pattern strings, ported from matplotlib's own
``tests/test_fontconfig_pattern.py``: ``FontProperties`` written as a
pattern and read back, patterns parsed into properties, and malformed
patterns.

Area ``mpl_fontconfig_pattern``. Every op runs under the reference
rcParams (``matplotlib.testing.setup()``: ``font.family = ['DejaVu
Sans']``, ``font.size = 10``).

Ops
---
``font_manager.FontProperties``  {kw} -> ``FontProperties(**kw)`` read
    back: ``{family, style, variant, weight, stretch, size,
    math_fontfamily, pattern}`` (``size`` in points, ``pattern`` the
    fontconfig pattern, which ``str()`` returns). A ``family`` given alone
    is parsed as a fontconfig pattern; malformed patterns raise
    ``ValueError``.
``font_manager.FontProperties.round_trip``  {kw} -> the same record of
    ``FontProperties(str(FontProperties(**kw)))``.
``_fontconfig_pattern.parse_fontconfig_pattern``  {pattern} -> the parsed
    properties (lists of strings by key); ``ValueError`` for syntax errors.

Cases
-----
``test_fontconfig_pattern`` round-trips its three property sets;
``test_fontconfig_str`` reads its two patterns and builds the properties
they are compared with; ``test_fontconfig_unknown_constant`` reads
``":unknown"``. Further patterns exercise escapes, several families and
values, constants and sizes.
"""

from matplotlib.font_manager import FontProperties
from matplotlib._fontconfig_pattern import parse_fontconfig_pattern

from spec_support import ProbeSet

probes = ProbeSet("mpl_fontconfig_pattern")


def _record(fp):
    return {
        "family": fp.get_family(),
        "style": fp.get_style(),
        "variant": fp.get_variant(),
        "weight": fp.get_weight(),
        "stretch": fp.get_stretch(),
        "size": fp.get_size_in_points(),
        "math_fontfamily": fp.get_math_fontfamily(),
        "pattern": fp.get_fontconfig_pattern(),
    }


def properties(**kw):
    probes.probe("font_manager.FontProperties", lambda kw: _record(FontProperties(**kw)), kw=kw)


def round_trip(**kw):
    probes.probe("font_manager.FontProperties.round_trip",
                 lambda kw: _record(FontProperties(str(FontProperties(**kw)))), kw=kw)


def test_fontconfig_pattern():
    round_trip()
    round_trip(family="serif", size=20, style="italic")
    round_trip(family="sans-serif", size=24, weight="bold", style="oblique",
               variant="small-caps", stretch="expanded")
    # Escapes, several families and numeric values.
    round_trip(family=["Foo-Bar", "Baz:Qux", "a,b"], size=7.5, weight=600, stretch=300)
    round_trip(family="DejaVu Sans", size="x-large", weight="ultralight")


def test_fontconfig_str():
    properties(family="sans\\-serif:style=normal:variant=normal:weight=normal"
                      ":stretch=normal:size=12.0")
    properties(family="sans-serif", style="normal", variant="normal", weight="normal",
               stretch="normal", size=12.0)
    properties(family="serif-24:style=oblique:variant=small-caps:weight=bold"
                      ":stretch=expanded")
    properties(family="serif", size=24, weight="bold", style="oblique", variant="small-caps",
               stretch="expanded")


def test_fontconfig_unknown_constant():
    properties(family=":unknown")


def test_more_patterns():
    for pattern in ["Foo,Bar-12:bold:slant=italic", "Foo\\,Bar\\-Baz-8.5", ":condensed",
                    ":weight=thin", ":roman:medium", "Foo-12,14", "-12",
                    ":size=12:size=14", ":family=A,B:style=oblique", "Foo:", "Foo-",
                    ":style=oblique:style=", "Foo-1e3"]:
        probes.probe("_fontconfig_pattern.parse_fontconfig_pattern", parse_fontconfig_pattern,
                     pattern=pattern)
        properties(family=pattern)
