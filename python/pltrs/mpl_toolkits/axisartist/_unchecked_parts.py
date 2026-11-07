"""
What the program gives the parts of an axis artist (its ticks, tick labels
and label) that matplotlib keeps unchecked and fails on only as it draws
them: a padding or a tick size that is no number, a transform that is no
transform, a filter that is no function, path effects that are none.

pltrs (Rust) draws the parts with their axis artist and cannot draw them
with such values: the part keeps each instead (see ``pltrs._unusable``),
with what matplotlib's draw of the part computes first with it, and
pltrs (Rust) draws with what it had. A part drawn on its own then fails
through its own ``draw``, as matplotlib's does; one an axis artist draws
makes the axis artist fail where matplotlib's draws its parts
(`note_kept`).
"""

from numbers import Real

import numpy as np

from pltrs import _python_drawing, _unusable
from pltrs.transforms import Affine2D, Transform


def points(part, name, value, use):
    """
    *value*, the length in points of *part*'s setting *name* (a label's
    pad, the size of ticks), as the float pltrs (Rust) takes: itself while
    it is a number; for one that is none, the part keeps it, failing as
    *use* (what matplotlib's draw computes first with it) does, and pltrs
    (Rust) keeps the last number.
    """
    numbers = part.__dict__.setdefault("_usable_points", {})
    if isinstance(value, Real):
        _unusable.drop(part, name)
        numbers[name] = float(value)
    else:
        _unusable.keep(part, name, value, use)
    return numbers.get(name, 0.)


def padded(figure_of):
    """What matplotlib's draw of a text of an axis artist computes first
    with its pad, with the renderer of the figure of *figure_of*: the
    external pad (pixels) and the pad in pixels."""
    return lambda pad: 0. + _python_drawing.drawing_renderer(figure_of).points_to_pixels(pad)


def sized(figure_of):
    """What matplotlib's draw of an axis artist computes first with the
    size of its ticks, with the renderer of the figure of *figure_of*: the
    size in pixels."""
    return lambda size: _python_drawing.drawing_renderer(figure_of).points_to_pixels(size) * 1


def usable_transform(part, given, use):
    """Whether pltrs (Rust) can place *part* with the transform *given* to
    it (a transform, or None for the identity); for one that is no
    transform, the part keeps it, failing as *use* (what matplotlib's draw
    computes first with it) does."""
    if given is None or isinstance(given, Transform):
        _unusable.drop(part, "transform")
        return True
    _unusable.keep(part, "transform", given, use)
    return False


def shifted(transform):
    """What matplotlib's draw of a text of an axis artist computes first
    with its *transform*: the transform shifted by its offset."""
    return transform + Affine2D()


def placed(transform):
    """What matplotlib's draw of ticks computes first with their
    *transform*: their locations through it."""
    return transform.transform_non_affine(np.zeros((1, 2)))


def check_text_drawing(text):
    """Keep what matplotlib's ``Text.draw`` of *text*, a text of an axis
    artist, fails on once it draws a line (``text._draws_text()``): path
    effects that are not effects (it draws with each), a filter that is no
    function (it calls it with what it drew)."""
    effects = text.get_path_effects()
    if effects and not all(hasattr(effect, "draw_path") for effect in _listed(effects)):
        _unusable.keep(text, "path_effects", effects,
                       lambda kept: text._draws_text() and _draw_effects(kept))
    else:
        _unusable.drop(text, "path_effects")
    agg_filter = text.get_agg_filter()
    if agg_filter is None or callable(agg_filter):
        _unusable.drop(text, "agg_filter")
    else:
        _unusable.keep(text, "agg_filter", agg_filter,
                       lambda kept: text._draws_text() and _filter(kept))


def _listed(effects):
    """The path *effects* as matplotlib iterates them (a string: its
    characters)."""
    try:
        return list(effects)
    except TypeError:
        return [effects]


def _draw_effects(effects):
    """Reach each of the path *effects*' ``draw_path``, as matplotlib's
    path effect renderer draws a text with them."""
    for effect in effects:
        getattr(effect, "draw_path")


def _filter(agg_filter):
    """Call *agg_filter*, as matplotlib's renderer calls the filter of what
    it drew."""
    agg_filter(np.zeros((1, 1, 4)), 72.)


def note_kept(part, drawn):
    """
    Have the axis artist drawing *part* (``drawn.axis_artist``, None for a
    part drawn on its own) fail where matplotlib's draws its parts while
    one of them keeps a value matplotlib fails on (see the module
    documentation); draw as before once none does.
    """
    axis_artist = drawn.axis_artist
    if axis_artist is None:
        return
    if any(_unusable.keeps(other) for other in axis_artist._parts()):
        if not _unusable.kept(axis_artist, "parts", False):
            _unusable.keep(axis_artist, "parts", True, axis_artist._check_parts)
    else:
        _unusable.drop(axis_artist, "parts")
