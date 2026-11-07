"""
The program's replacements of matplotlib's text measurement and text
drawing, handed to pltrs (Rust).

matplotlib measures every line of text through the module function
``text._get_text_metrics_with_cache`` and draws every text (titles, tick
and axis labels, legend and table texts, annotations) through
``Text.draw``. Its tests replace both with ``monkeypatch.setattr``, so that
texts become placeholder boxes (the ``text_placeholders`` fixture). pltrs
(Rust) measures and draws texts itself, so it is given stand-ins calling
the replacements, with the attributes they stand for (`watch`), which it
reads each time it would use a stand-in:

* while ``_get_text_metrics_with_cache`` is replaced, pltrs (Rust)
  measures every line it lays out by calling it, and reads the font height
  metrics from the font's tables as matplotlib's ``Text._get_layout`` does
  (``FT2Font.get_sfnt_table``; without them, it measures ``"lp"``);
* while ``Text.draw`` is replaced, pltrs (Rust) draws every text by calling
  it with a stand-in for the text (a `.Text` of its own, in display
  coordinates where the text is drawn) and a `stand_in_renderer`; the
  artists drawn with that renderer are drawn in place of the text.

When the originals come back, pltrs (Rust) measures and draws texts itself
again, without calling Python. The module and the class stay plain:
nothing watches the assignments.
"""

import functools

from ._native import text_overrides as _native
from ._text_metrics import current_renderer
from .font_manager import FontProperties, fontManager, get_font

#: The tables ``Text._get_layout`` reads the font height metrics from:
#: (table, line gap, ascent, descent).
_HEIGHT_TABLES = (
    ('OS/2', 'sTypoLineGap', 'sTypoAscender', 'sTypoDescender'),
    ('hhea', 'lineGap', 'ascent', 'descent'),
)

#: The module whose ``_get_text_metrics_with_cache`` and the class whose
#: ``draw`` the program may replace.
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
    """Measure a line pltrs (Rust) lays out with the replaced function,
    handing it the renderer pltrs lays out with (a stand-in when no
    renderer is laying out)."""
    module = _watched["module"]
    renderer = current_renderer() or stand_in_renderer(dpi)
    return module._get_text_metrics_with_cache(
        renderer, s, FontProperties._from_native(props), ismath, dpi)


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
    text = text_class._from_native(stand_in, None)
    # Measured in its own place, where matplotlib's text is in its figure.
    text._stand_in = True
    text_class.draw(text, renderer)
    return renderer._drawn


def watch(module, text_class):
    """Have pltrs (Rust) measure and draw texts through the program's
    replacements of the ``_get_text_metrics_with_cache`` of *module*
    (`.text`) and of the ``draw`` of *text_class* (`.Text`) while there
    are any; their current ones are the originals."""
    _watched.update(module=module, **{"class": text_class})
    _native.set_measurer(_measure_line, _font_height,
                         (module, "_get_text_metrics_with_cache",
                          module._get_text_metrics_with_cache))
    _native.set_painter(_paint, (text_class, "draw", text_class.draw))
