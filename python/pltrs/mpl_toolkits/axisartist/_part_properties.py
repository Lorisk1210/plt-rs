"""
The `.Text` properties of the texts of axis artists and the `.Line2D`
properties of their ticks: matplotlib's `LabelBase` is a `.Text` and its
`Ticks` a `.Line2D`, where pltrs' are artists over the parts pltrs (Rust)
draws.

Each keeps a text, or a line, of its own holding those properties, which
checks and normalizes them as a text or a line does. pltrs (Rust) draws
the texts with their font, the alignment and spacing of their lines and
their rotation mode (handed it as their *look* whenever one changes), and
keeps the other properties as matplotlib keeps them on texts it places
itself. matplotlib's `Ticks.draw` reads none of the line properties but
the marks' colour and width, which `Ticks` has of its own.
"""

import functools

from pltrs import _api, lines as mlines, text as mtext
from pltrs.font_manager import FontManager, FontProperties
from pltrs._native import text as _text


def _on_text(name, look=False):
    """The `.Text` method *name* run on the texts' own text, handing pltrs
    (Rust) their look afterwards when it is part of it (*look*)."""
    method = getattr(mtext.Text, name)

    @functools.wraps(method)
    def on_text(self, *args, **kwargs):
        result = method(self._props_text(), *args, **kwargs)
        if look:
            self._set_look(self._look_of(self._props_text()))
        return result

    return on_text


@_api.define_aliases({
    "fontfamily": ["family"], "fontname": ["name"], "fontstyle": ["style"],
    "fontvariant": ["variant"], "fontweight": ["weight"],
    "fontstretch": ["stretch"], "multialignment": ["ma"],
    "fontproperties": ["font", "font_properties"],
})
class TextProperties:
    """
    The `.Text` properties of the texts of an axis artist that their
    string, size, colour, angle and alignment leave.

    A subclass gives the texts' look as pltrs (Rust) has it
    (``_native_look``) and hands it a new one (``_set_look``).
    """

    def _props_text(self):
        """The text of their own holding the texts' properties, made with
        the look pltrs (Rust) has."""
        text = self.__dict__.get("_props_text_object")
        if text is None:
            text = self._props_text_object = mtext.Text()
            font, multialignment, linespacing, rotation_mode = self._native_look()
            text.set_fontproperties(FontProperties._from_native(font))
            if multialignment is not None:
                text.set_multialignment(multialignment)
            text.set_linespacing("normal" if linespacing is None else linespacing)
            text.set_rotation_mode(rotation_mode)
        return text

    @staticmethod
    def _look_of(text):
        """The look of *text* as pltrs (Rust) takes it: its font
        properties, the alignment of its lines (None: as the text), its
        line spacing (None: normal) and its rotation mode."""
        return (FontManager._props(text.get_fontproperties()),
                _text.get_multialignment(text._native),
                _text.get_linespacing(text._native),
                text.get_rotation_mode())

    def get_fontproperties(self):
        """Return the `.font_manager.FontProperties` of the texts, at
        their size."""
        font = self._props_text().get_fontproperties().copy()
        font.set_size(self.get_size())
        return font

    def set_fontproperties(self, fp):
        """
        Set the font properties of the texts, their size included.

        Parameters
        ----------
        fp : `.font_manager.FontProperties` or `str` or `pathlib.Path`
        """
        font = FontProperties._from_any(fp)
        self._props_text().set_fontproperties(font)
        self._set_look(self._look_of(self._props_text()))
        self.set_size(font.get_size_in_points())

    set_fontfamily = _on_text("set_fontfamily", look=True)
    set_fontname = _on_text("set_fontname", look=True)
    set_fontstyle = _on_text("set_fontstyle", look=True)
    set_fontvariant = _on_text("set_fontvariant", look=True)
    set_fontweight = _on_text("set_fontweight", look=True)
    set_fontstretch = _on_text("set_fontstretch", look=True)
    set_math_fontfamily = _on_text("set_math_fontfamily", look=True)
    set_linespacing = _on_text("set_linespacing", look=True)
    set_multialignment = _on_text("set_multialignment", look=True)
    set_rotation_mode = _on_text("set_rotation_mode", look=True)

    get_fontfamily = _on_text("get_fontfamily")
    get_fontname = _on_text("get_fontname")
    get_fontstyle = _on_text("get_fontstyle")
    get_fontvariant = _on_text("get_fontvariant")
    get_fontweight = _on_text("get_fontweight")
    get_stretch = _on_text("get_stretch")
    get_math_fontfamily = _on_text("get_math_fontfamily")
    get_linespacing = _on_text("get_linespacing")
    get_rotation_mode = _on_text("get_rotation_mode")

    set_antialiased = _on_text("set_antialiased")
    get_antialiased = _on_text("get_antialiased")
    set_backgroundcolor = _on_text("set_backgroundcolor")
    set_bbox = _on_text("set_bbox")
    get_bbox_patch = _on_text("get_bbox_patch")
    set_fontfeatures = _on_text("set_fontfeatures")
    get_fontfeatures = _on_text("get_fontfeatures")
    set_language = _on_text("set_language")
    get_language = _on_text("get_language")
    set_parse_math = _on_text("set_parse_math")
    get_parse_math = _on_text("get_parse_math")
    set_transform_rotates_text = _on_text("set_transform_rotates_text")
    get_transform_rotates_text = _on_text("get_transform_rotates_text")
    set_usetex = _on_text("set_usetex")
    get_usetex = _on_text("get_usetex")
    set_wrap = _on_text("set_wrap")
    get_wrap = _on_text("get_wrap")


def _on_line(name):
    """The `.Line2D` method *name* run on the ticks' own line."""
    method = getattr(mlines.Line2D, name)

    @functools.wraps(method)
    def on_line(self, *args, **kwargs):
        return method(self._props_line(), *args, **kwargs)

    return on_line


@_api.define_aliases({
    "antialiased": ["aa"], "drawstyle": ["ds"], "linestyle": ["ls"],
    "linewidth": ["lw"], "markerfacecolor": ["mfc"],
    "markerfacecoloralt": ["mfcalt"], "markersize": ["ms"],
})
class LineProperties:
    """The `.Line2D` properties of ticks that their drawing does not read
    (matplotlib's `Ticks.draw` strokes their marks in their own colour and
    width), kept on a line of their own as a `.Line2D` keeps them."""

    def _props_line(self):
        """The line of their own holding the ticks' line properties
        (matplotlib's ticks start as the line through ``[0], [0]``)."""
        line = self.__dict__.get("_props_line_object")
        if line is None:
            line = self._props_line_object = mlines.Line2D([0.], [0.])
        return line

    set_antialiased = _on_line("set_antialiased")
    get_antialiased = _on_line("get_antialiased")
    set_dash_capstyle = _on_line("set_dash_capstyle")
    get_dash_capstyle = _on_line("get_dash_capstyle")
    set_dash_joinstyle = _on_line("set_dash_joinstyle")
    get_dash_joinstyle = _on_line("get_dash_joinstyle")
    set_solid_capstyle = _on_line("set_solid_capstyle")
    get_solid_capstyle = _on_line("get_solid_capstyle")
    set_solid_joinstyle = _on_line("set_solid_joinstyle")
    get_solid_joinstyle = _on_line("get_solid_joinstyle")
    set_dashes = _on_line("set_dashes")
    set_data = _on_line("set_data")
    get_data = _on_line("get_data")
    set_xdata = _on_line("set_xdata")
    get_xdata = _on_line("get_xdata")
    set_ydata = _on_line("set_ydata")
    get_ydata = _on_line("get_ydata")
    set_drawstyle = _on_line("set_drawstyle")
    get_drawstyle = _on_line("get_drawstyle")
    set_fillstyle = _on_line("set_fillstyle")
    get_fillstyle = _on_line("get_fillstyle")
    set_gapcolor = _on_line("set_gapcolor")
    get_gapcolor = _on_line("get_gapcolor")
    set_linestyle = _on_line("set_linestyle")
    get_linestyle = _on_line("get_linestyle")
    set_linewidth = _on_line("set_linewidth")
    get_linewidth = _on_line("get_linewidth")
    set_marker = _on_line("set_marker")
    get_marker = _on_line("get_marker")
    set_markerfacecolor = _on_line("set_markerfacecolor")
    get_markerfacecolor = _on_line("get_markerfacecolor")
    set_markerfacecoloralt = _on_line("set_markerfacecoloralt")
    get_markerfacecoloralt = _on_line("get_markerfacecoloralt")
    set_markersize = _on_line("set_markersize")
    get_markersize = _on_line("get_markersize")
    set_markevery = _on_line("set_markevery")
    get_markevery = _on_line("get_markevery")
    set_pickradius = _on_line("set_pickradius")
    get_pickradius = _on_line("get_pickradius")
