"""
The program's replacements of matplotlib's text measurement and text
drawing, handed to pltrs (Rust).

matplotlib measures every line of text through the module function
``text._get_text_metrics_with_cache`` and draws every text (titles, tick
and axis labels, legend and table texts, annotations) through
``Text.draw``. Its tests replace both with ``monkeypatch.setattr``, so that
texts become placeholder boxes (the ``text_placeholders`` fixture). pltrs
(Rust) measures and draws texts itself, so the `.text` module and its
`.Text` class watch these two attributes (`watch`):

* while ``_get_text_metrics_with_cache`` is replaced, pltrs (Rust)
  measures every line it lays out by calling it, and reads the font height
  metrics from the font's tables as matplotlib's ``Text._get_layout`` does
  (``FT2Font.get_sfnt_table``; without them, it measures ``"lp"``);
* while ``Text.draw`` is replaced, pltrs (Rust) draws every text by calling
  it with a stand-in for the text (a `.Text` of its own, in display
  coordinates where the text is drawn) and a `stand_in_renderer`; the
  artists drawn with that renderer are drawn in place of the text.

When the originals come back, pltrs (Rust) measures and draws texts itself
again, without calling Python.
"""

import functools
import types

from ._native import text_overrides as _native
from .font_manager import FontProperties, fontManager, get_font

#: The tables ``Text._get_layout`` reads the font height metrics from:
#: (table, line gap, ascent, descent).
_HEIGHT_TABLES = (
    ('OS/2', 'sTypoLineGap', 'sTypoAscender', 'sTypoDescender'),
    ('hhea', 'lineGap', 'ascent', 'descent'),
)

#: The module watched for a replaced ``_get_text_metrics_with_cache``, the
#: class watched for a replaced ``draw``, and their originals.
_watched = {}


def stand_in_renderer(dpi):
    """
    The renderer pltrs (Rust) hands to the program's replacements of text
    measurement and drawing: a renderer at *dpi* measuring text as the Agg
    canvas does, with which artists are drawn where pltrs (Rust) is
    drawing the text (`.Artist.draw`).
    """
    return _stand_in_renderer_class()(dpi)


@functools.cache
def _stand_in_renderer_class():
    """The class of `stand_in_renderer` (made once the package is
    imported: `.backend_bases` imports the modules importing this one)."""
    from .backend_bases import RendererBase

    class StandInRenderer(RendererBase):
        __doc__ = stand_in_renderer.__doc__

        def __init__(self, dpi):
            super().__init__()
            self.dpi = dpi
            self._drawn = []

        def points_to_pixels(self, points):
            # docstring inherited
            return points * self.dpi / 72

        def _draw_artist(self, artist):
            """Draw *artist* where pltrs (Rust) draws the text."""
            self._drawn.append(artist._native)

    return StandInRenderer


def _measure_line(s, props, ismath, dpi):
    """Measure a line pltrs (Rust) lays out with the replaced function."""
    module = _watched["module"]
    return module._get_text_metrics_with_cache(
        stand_in_renderer(dpi), s, FontProperties._from_native(props), ismath,
        dpi)


def _font_height(props, dpi):
    """
    The minimum ascent, descent and line gap of lines in the font *props*
    describes, at *dpi*, from the font's tables as ``Text._get_layout``
    reads them; None when the font has none of them.
    """
    prop = FontProperties._from_native(props)
    font = get_font(fontManager._find_fonts_by_props(prop))
    for table_name, linegap_key, ascent_key, descent_key in _HEIGHT_TABLES:
        table = font.get_sfnt_table(table_name)
        if table is None:
            continue
        fontsize = prop.get_size_in_points()
        units_per_em = font.get_sfnt_table('head')['unitsPerEm']
        scale = 1 / units_per_em * fontsize * dpi / 72
        return (table[ascent_key] * scale, -table[descent_key] * scale,
                table[linegap_key] * scale)
    return None


def _paint(stand_in, dpi):
    """Draw a text pltrs (Rust) draws with the replaced ``Text.draw``: the
    native handles of the artists to draw in its place."""
    text_class = _watched["class"]
    renderer = stand_in_renderer(dpi)
    text_class.draw(text_class._from_native(stand_in, None), renderer)
    return renderer._drawn


def _sync():
    """Hand pltrs (Rust) the replacements in force, or none."""
    module = _watched["module"]
    measured = (module.__dict__.get("_get_text_metrics_with_cache")
                is not _watched["measure"])
    _native.set_measurer(*((_measure_line, _font_height) if measured
                           else (None, None)))
    painted = _watched["class"].draw is not _watched["draw"]
    _native.set_painter(_paint if painted else None)


class _WatchedModule(types.ModuleType):
    """The class of the `.text` module: it tells pltrs (Rust) when
    ``_get_text_metrics_with_cache`` is replaced or restored."""

    def __setattr__(self, name, value):
        super().__setattr__(name, value)
        if name == "_get_text_metrics_with_cache":
            _sync()

    def __delattr__(self, name):
        super().__delattr__(name)
        if name == "_get_text_metrics_with_cache":
            _sync()


class WatchesDraw(type):
    """The metaclass of `.Text`: it tells pltrs (Rust) when ``Text.draw``
    is replaced or restored."""

    def __setattr__(cls, name, value):
        super().__setattr__(name, value)
        if name == "draw" and _watched:
            _sync()

    def __delattr__(cls, name):
        super().__delattr__(name)
        if name == "draw" and _watched:
            _sync()


def watch(module, text_class):
    """Watch *module* (`.text`) and *text_class* (`.Text`), whose current
    ``_get_text_metrics_with_cache`` and ``draw`` are the originals."""
    _watched.update(module=module, measure=module._get_text_metrics_with_cache,
                    **{"class": text_class, "draw": text_class.draw})
    module.__class__ = _WatchedModule
