"""
The setters matplotlib's `.ContourSet` constructor calls, for a contour set
of a class of the program's that overrides some.

pltrs (Rust) makes a contour set with its final look at once. A class of
the program's that overrides a setter sees the calls matplotlib's
constructor makes (`._program_classes.replay_setters`): those of
``Collection.__init__``, its alpha and transform, the colormap and norm,
the levels' values, the line look of a set of lines (the edges of a filled
one), and those of the Axes adding the set. Each call gets the value
matplotlib's constructor gives, as the set ends up with it.

Not replayed:
- the calls matplotlib makes again through the norm's callbacks
  (``changed``, which sets the array again and updates the colours as
  each of the norm's limits is set), and the private setters the public
  ones call: they depend on how many callbacks the norm has, not on the
  set;
- ``set_transform(None)``, which matplotlib's set resolves to its Axes'
  data transform where it is asked (``get_transform``), and pltrs' setter
  takes for no transform;
- ``set_array`` with the values of the levels, which leaves the colours of
  a set of given colours otherwise than matplotlib's;
- ``set_hatch(None)`` for a set of hatched levels, whose hatches pltrs'
  setter would clear (matplotlib's set draws each level's own).
"""

import math

from . import rcParams
from ._program_classes import replay_setters

#: Where the keywords a set of a class of the program's was made with wait
#: for its look to be set (`given`, `replay_making`).
_GIVEN = "_given_to_contour_constructor"


def given(contours, kwargs, filled):
    """Keep the keywords *kwargs* the contour set *contours* (filled or
    not) is made with, if its class overrides a setter, for
    `replay_making`."""
    if type(contours)._program_setters:
        contours.__dict__[_GIVEN] = (dict(kwargs), bool(filled))


def replay_making(contours):
    """Call the setters a class of the program's overrides as matplotlib's
    constructor calls them before the keywords it was given are set."""
    kept = contours.__dict__.pop(_GIVEN, None)
    if kept is not None:
        replay_setters(contours, list(_making(contours, *kept)))
        contours.__dict__["_contour_axes_add"] = True


def replay_adding(contours):
    """Call the setters a class of the program's overrides as matplotlib's
    Axes calls them when the constructor adds the set: a filled set's
    label (``_child<n>``), its figure, then its clip path, the Axes'
    patch."""
    if contours.__dict__.pop("_contour_axes_add", False):
        axes = contours.axes
        calls = []
        if contours.filled:
            # A filled set has no label: the Axes names it as its child.
            calls.append(("set_label", (contours.get_label(),)))
        calls += [("set_figure", (axes.get_figure(root=False),)),
                  ("set_clip_path", (axes.patch,))]
        replay_setters(contours, calls)


def _making(contours, kwargs, filled):
    """The setter calls of matplotlib's ``ContourSet.__init__`` up to the
    keywords it sets last, ``(name, arguments)`` in its order."""
    antialiased = kwargs.get("antialiased")
    if antialiased is None and filled:
        antialiased = False
    # Collection.__init__, then its own keywords.
    yield from [("set_facecolor", (None,)), ("set_edgecolor", (None,)),
                ("set_linewidth", (None,)), ("set_linestyle", ("solid",)),
                ("set_antialiased", (antialiased,)), ("set_pickradius", (5.0,)),
                ("set_urls", (None,))]
    if all(hatch is None for hatch in contours.hatches):
        yield "set_hatch", (None,)
    yield from [("set_hatchcolor", (None,)), ("set_zorder", (1,)),
                ("set_alpha", (kwargs.get("alpha"),))]
    if kwargs.get("transform") is not None:
        yield "set_transform", (kwargs["transform"],)
    colorizer = kwargs.get("colorizer")
    colors = kwargs.get("colors")
    cmap = colorizer.cmap if colorizer is not None else kwargs.get("cmap")
    norm = colorizer.norm if colorizer is not None else kwargs.get("norm")
    if colors is not None:
        # The colormap made of the colours.
        cmap = contours.cmap
    if norm is None and getattr(contours, "logscale", False):
        norm = contours.norm
    # Setting the colormap runs the set's ``changed``, which processes the
    # colours (indexing given colours through a `.NoNorm`); the norm given
    # comes after.
    yield "set_cmap", (cmap,)
    if colors is not None:
        yield "set_norm", (contours.norm,)
    if norm is not None:
        yield "set_norm", (norm,)
    if filled:
        yield "set_edgecolor", ("none",)
    else:
        yield from [("set_facecolor", ("none",)),
                    ("set_linewidth", (_linewidths(contours, kwargs.get("linewidths")),)),
                    ("set_linestyle", (_linestyles(contours, kwargs.get("linestyles")),)),
                    ("set_label", ("_nolegend_",)), ("set_zorder", (2,))]


def _linewidths(contours, linewidths):
    """matplotlib's ``ContourSet._process_linewidths``: a width per level."""
    levels = len(contours.levels)
    if linewidths is None:
        width = rcParams['contour.linewidth']
        return [rcParams['lines.linewidth'] if width is None else width] * levels
    if not hasattr(linewidths, "__iter__"):
        return [linewidths] * levels
    linewidths = list(linewidths)
    return (linewidths * math.ceil(levels / len(linewidths)))[:levels]


def _linestyles(contours, linestyles):
    """matplotlib's ``ContourSet._process_linestyles``: a style per level,
    the negative levels of a monochrome set dashed as it says."""
    levels = list(contours.levels)
    if linestyles is None:
        styles = ['solid'] * len(levels)
        if contours.monochrome:
            eps = -(contours.zmax - contours.zmin) * 1e-15
            styles = [contours.negative_linestyles if level < eps else style
                      for level, style in zip(levels, styles)]
        return styles
    if isinstance(linestyles, str):
        return [linestyles] * len(levels)
    styles = list(linestyles)
    return (styles * math.ceil(len(levels) / len(styles)))[:len(levels)]
