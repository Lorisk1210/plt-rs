"""
What ultraplot's documents and the last tests of the libraries' suites
need of matplotlib, beyond `test_eco9`: the child Axes of an Axes whose
class places its titles itself placed by their locators of the
program's before the titles are lifted (ultraplot's inset colorbars
under its super title), and a legend made for an Axes and added to them
drawn once (ultraplot's centred legend rows), the Axes' own legend kept
as an artist twice, and a program's own mathtext font class asked for
every glyph (ultraplot's routing of symbols to Computer Modern): the
glyphs matplotlib's own classes give through it are pltrs' own, string
by string. Each test does what a library does, without the library. The expected values were recorded from matplotlib 3.11.2
running the same code.
"""

import ast
import io
import pathlib

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs.axes import Axes
from pltrs.font_manager import FontProperties
from pltrs.legend import Legend
from pltrs import _mathtext
from pltrs.mathtext import MathTextParser
from pltrs.patches import Patch, Rectangle
from pltrs.transforms import Bbox
from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


class _TitledAxes(Axes):
    """An Axes class placing its titles its own way, after matplotlib's
    (ultraplot's `_update_title_position`)."""

    name = "eco10-titled"

    def _update_title_position(self, renderer):
        super()._update_title_position(renderer)


def test_titles_placed_by_a_class_follow_the_children_its_locators_place():
    fig = plt.figure(figsize=(5, 5))
    ax = fig.add_subplot(axes_class=_TitledAxes)
    child = ax.inset_axes([0.2, 0.8, 0.3, 0.05])
    calls = []
    place = {"y": 0.9}

    def locate(asked, renderer):
        calls.append("parent" if asked is ax else "child" if asked is child else "other")
        return Bbox.from_bounds(0.2, place["y"], 0.3, 0.05)

    child.set_axes_locator(locate)
    fig.canvas.draw()
    # Without a title matplotlib lifts nothing, and places no child for it
    # (ultraplot's geographic insets keep their own aspect so).
    calls.clear()
    ax._update_title_position(fig.canvas.get_renderer())
    assert calls == []
    ax.set_title("title")
    fig.canvas.draw()
    assert ax.title.get_position()[1] == pytest.approx(1.0909, abs=5e-5)
    calls.clear()
    place["y"] = 0.5
    ax._update_title_position(fig.canvas.get_renderer())
    # matplotlib places the child first, calling its locator with the
    # parent, and the title comes back down to the Axes' top.
    assert calls == ["parent"]
    assert ax.title.get_position()[1] == 1.0
    assert child.get_position().bounds == pytest.approx((0.2, 0.5, 0.3, 0.05))


def _swatch(fig, legend):
    """The colour drawn at the middle of the legend's first handle."""
    fig.canvas.draw()
    box = legend.legend_handles[0].get_window_extent(fig.canvas.get_renderer())
    x, y = int((box.x0 + box.x1) / 2), int((box.y0 + box.y1) / 2)
    buffer = io.BytesIO()
    fig.savefig(buffer, format="rgba")
    width, height = fig.canvas.get_width_height()
    pixels = np.frombuffer(buffer.getvalue(), np.uint8).reshape(height, width, 4)
    return tuple(int(v) for v in pixels[height - 1 - y, x, :3])


_GRAY = (0.286, 0.314, 0.341)


def test_a_legend_made_for_an_axes_and_added_to_them_is_drawn_once():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=100)
    legend = Legend(ax, [Patch(facecolor=_GRAY, alpha=0.7)], ["q"], loc="center",
                    frameon=False)
    ax.add_artist(legend)
    assert ax.legend_ is None
    assert sum(child is legend for child in ax.get_children()) == 1
    assert _swatch(fig, legend) == (127, 132, 137)


@pytest.mark.parametrize("removed", [False, True])
def test_the_axes_legend_added_as_an_artist_is_drawn_twice_unless_removed(removed):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=100)
    ax.add_patch(Rectangle((2, 2), 1, 1, facecolor=_GRAY, alpha=0.7, label="q"))
    legend = ax.legend(loc="center", frameon=False)
    if removed:
        legend.remove()
    ax.add_artist(legend)
    assert (ax.legend_ is None) == removed
    assert sum(child is legend for child in ax.get_children()) == (1 if removed else 2)
    assert _swatch(fig, legend) == ((127, 132, 137) if removed else (89, 95, 101))


#: matplotlib's mathtext test strings the spec draws (``math_tests``,
#: ``font_tests`` and ``svgastext_math_tests`` of ``test_mathtext.py``).
_CASES = pathlib.Path(__file__).parents[2] / "spec/reference/figures/cases_mpl_text.py"


def _math_strings():
    found = {}
    for node in ast.parse(_CASES.read_text(encoding="utf-8")).body:
        if (isinstance(node, ast.Assign) and len(node.targets) == 1
                and getattr(node.targets[0], "id", None) in ("MATH_TESTS", "FONT_TESTS",
                                                             "SVGASTEXT")):
            # The spec's own lists (one made by a comprehension), evaluated
            # without any name of the module's.
            expression = ast.Expression(node.value)
            found[node.targets[0].id] = eval(  # noqa: S307 (the repository's file)
                compile(expression, str(_CASES), "eval"), {"__builtins__": {}})
    return [s for name in ("MATH_TESTS", "FONT_TESTS", "SVGASTEXT") for s in found[name]]


def _glyphs(parse):
    return ([(font.fname, font.face_index, size, code, index, round(ox, 9), round(oy, 9))
             for font, size, code, index, ox, oy in parse.glyphs],
            [tuple(round(v, 9) for v in rect) for rect in parse.rects],
            (round(parse.width, 9), round(parse.height, 9), round(parse.depth, 9)))


@pytest.mark.parametrize("fontset", ["cm", "stix", "stixsans", "dejavusans",
                                     "dejavuserif", "custom"])
def test_matplotlibs_font_classes_give_the_glyphs_pltrs_typesets(fontset, monkeypatch):
    # A class of the program's that changes nothing has pltrs (Rust) ask
    # the Python classes for every glyph: they must answer as it does.
    mapping = MathTextParser._font_type_mapping
    own = mapping[fontset]
    prop = FontProperties(math_fontfamily=fontset)
    strings = _math_strings()
    native = [_glyphs(MathTextParser("path").parse(s, dpi=100, prop=prop)) for s in strings]
    monkeypatch.setitem(mapping, fontset, type("Program" + own.__name__, (own,), {}))
    asked = [_glyphs(MathTextParser("path").parse(s, dpi=100, prop=prop)) for s in strings]
    differ = [s for s, a, b in zip(strings, native, asked) if a != b]
    assert differ == []


class _Routing(_mathtext.DejaVuSansFonts):
    """Routes ``\\sum`` to Computer Modern while the program says so,
    through ``super()`` otherwise (ultraplot's routing of big operators)."""

    route = False

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # Read once per instance: matplotlib makes one per parse.
        self._route = _Routing.route
        self._cm = _mathtext.BakomaFonts(*args, **kwargs)

    def _get_glyph(self, fontname, font_class, sym):
        if self._route and sym == r"\sum":
            return self._cm._get_glyph(fontname, font_class, sym)
        return super()._get_glyph(fontname, font_class, sym)

    def get_sized_alternatives_for_symbol(self, fontname, sym):
        if self._route and sym == r"\sum":
            return self._cm.get_sized_alternatives_for_symbol(fontname, sym)
        return super().get_sized_alternatives_for_symbol(fontname, sym)


def test_a_font_class_of_the_programs_is_asked_for_every_glyph_until_the_cache_clears(
        monkeypatch):
    monkeypatch.setitem(MathTextParser._font_type_mapping, "dejavusans", _Routing)
    monkeypatch.setattr(_Routing, "route", False)
    parser = MathTextParser("path")
    expression = r"$\sum x \left(\frac{a}{b}\right)$"

    def first_two():
        return [glyph[0].postscript_name for glyph in parser.parse(expression).glyphs][:2]

    assert first_two() == ["DejaVuSansDisplay", "DejaVuSans-Oblique"]
    _Routing.route = True
    # The parse cache keeps the parse until the program clears it.
    assert first_two() == ["DejaVuSansDisplay", "DejaVuSans-Oblique"]
    MathTextParser._parse_cached.cache_clear()
    assert first_two() == ["Cmex10", "DejaVuSans-Oblique"]
    _Routing.route = False
    MathTextParser._parse_cached.cache_clear()
    assert first_two() == ["DejaVuSansDisplay", "DejaVuSans-Oblique"]


class _Placeholder(_mathtext.UnicodeFonts):
    """Takes the family ``nosuchfamily`` :rc:`mathtext.rm` names as sans
    (ultraplot's ``'regular'`` placeholder), as it is made."""

    def __init__(self, *args, **kwargs):
        with plt.rc_context({"mathtext.rm": "sans"}):
            super().__init__(*args, **kwargs)


def test_the_fonts_of_a_fontset_a_class_of_the_programs_serves_are_its_own_to_look_up(
        monkeypatch, caplog):
    # matplotlib looks the fontset's fonts up through the class alone, so
    # the placeholder it replaces is never looked up.
    monkeypatch.setitem(MathTextParser._font_type_mapping, "custom", _Placeholder)
    with plt.rc_context({"mathtext.fontset": "custom", "mathtext.rm": "nosuchfamily"}):
        fig = plt.figure()
        fig.text(0.1, 0.5, r"$\mathrm{abc} + x$")
        fig.canvas.draw()
    assert not [record for record in caplog.records if "nosuchfamily" in record.getMessage()]
