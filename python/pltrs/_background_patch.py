"""
The backgrounds of axes and figures (matplotlib's ``Axes.patch`` and
``Figure.patch``, each a `.Rectangle`): pltrs (Rust) draws them as part of
their axes or figure, so their objects read and set the background
properties pltrs (Rust) keeps there. `BackgroundPatch` holds what both
have in common; `FigurePatch` is a figure's (an axes' is
``axes._decorations.AxesPatch``).
"""

from numbers import Real

from . import _live_transforms, cbook
from ._artist_children import ChildrenSearchMixin
from ._native import figure as _figure


class BackgroundPatch(ChildrenSearchMixin):
    """
    The `.Rectangle` API of a background whose face colour, edge colour,
    line width and visibility pltrs (Rust) keeps: subclasses read them
    (``get_facecolor``, ...) and set them as given (``_set_facecolor``,
    ``_set_edgecolor``), with the patch's alpha already applied.
    """

    _ALIASES = {"facecolor": ["fc"], "edgecolor": ["ec"], "linewidth": ["lw"]}

    def __init__(self):
        self._animated = False
        self._alpha = None
        self._picker = None
        # The colours as given, which the alpha overrides when set
        # (matplotlib's ``_original_facecolor``); None for those never
        # given through the patch.
        self._original = {"face": None, "edge": None}

    def get_animated(self):
        """Return whether the background is animated."""
        return self._animated

    def set_animated(self, b):
        """
        Set whether the background is animated.

        As in matplotlib, the axes or figure draws its background whatever
        this flag says; it only tells blitting code (the widgets) to draw
        the background itself, as it draws every animated child.
        """
        self._animated = bool(b)

    def get_picker(self):
        """Return the picking behavior of the background (see
        `.Artist.set_picker`)."""
        return self._picker

    def set_picker(self, picker):
        """Define the picking behavior of the background (see
        `.Artist.set_picker`)."""
        self._picker = picker

    def pickable(self):
        """Return whether the background is pickable."""
        return self.get_figure(root=False) is not None and self._picker is not None

    def pick(self, mouseevent):
        """
        Process a pick event: the background fires one when it has a picker
        and *mouseevent* is over it (it has no children to pass it on to).
        """
        if not self.pickable():
            return
        from .backend_bases import PickEvent
        if callable(self._picker):
            inside, prop = self._picker(self, mouseevent)
        else:
            inside, prop = self.contains(mouseevent)
        if inside:
            PickEvent("pick_event", self.get_figure(root=True).canvas,
                      mouseevent, self, **prop)._process()

    def get_alpha(self):
        """Return the alpha overriding the colours' own, or None."""
        return self._alpha

    def set_alpha(self, alpha):
        """
        Set the alpha value used for blending: it overrides the alpha of the
        face and edge colours, as a `.Patch`'s does.

        Parameters
        ----------
        alpha : float or None
            *alpha* must be within the 0-1 range, inclusive.
        """
        if alpha is not None and not isinstance(alpha, Real):
            raise TypeError(
                f'alpha must be numeric or None, not {type(alpha)}')
        if alpha is not None and not (0 <= alpha <= 1):
            raise ValueError(f'alpha ({alpha}) is outside 0-1 range')
        face = self._original["face"]
        edge = self._original["edge"]
        self._alpha = alpha
        self.set_facecolor(self.get_facecolor() if face is None else face)
        self.set_edgecolor(self.get_edgecolor() if edge is None else edge)

    def _with_alpha(self, color):
        """*color* with the patch's alpha, if it has one."""
        from .colors import to_rgba
        if self._alpha is None or cbook._str_lower_equal(color, "none"):
            return color
        return to_rgba(color, self._alpha)

    def set_facecolor(self, color):
        """Set the face color of the background."""
        self._original["face"] = color
        self._set_facecolor(self._with_alpha(color))

    def set_edgecolor(self, color):
        """Set the edge color of the background."""
        color = "none" if color is None else color
        self._original["edge"] = color
        self._set_edgecolor(self._with_alpha(color))

    def set_color(self, c):
        """Set both the edgecolor and the facecolor of the background."""
        self.set_facecolor(c)
        self.set_edgecolor(c)

    def get_fc(self):
        """Alias for `get_facecolor`."""
        return self.get_facecolor()

    def set_fc(self, color):
        """Alias for `set_facecolor`."""
        self.set_facecolor(color)

    def get_ec(self):
        """Alias for `get_edgecolor`."""
        return self.get_edgecolor()

    def set_ec(self, color):
        """Alias for `set_edgecolor`."""
        self.set_edgecolor(color)

    def get_lw(self):
        """Alias for `get_linewidth`."""
        return self.get_linewidth()

    def set_lw(self, w):
        """Alias for `set_linewidth`."""
        self.set_linewidth(w)

    def set(self, **kwargs):
        """Set several properties at once (``set_<name>(value)`` each)."""
        for name, value in cbook.normalize_kwargs(kwargs, self._ALIASES).items():
            setter = getattr(self, f"set_{name}", None)
            if setter is None:
                raise AttributeError(
                    f"Rectangle.set() got an unexpected keyword argument {name!r}")
            setter(value)

    def remove(self):
        """The background is part of its axes or figure, which cannot lose
        it."""
        raise NotImplementedError('cannot remove artist')


class FigurePatch(BackgroundPatch):
    """
    The background of a figure or subfigure (matplotlib's ``Figure.patch``,
    a unit `.Rectangle` in subfigure coordinates): pltrs (Rust) draws it as
    part of the figure, so this object reads and sets the figure's
    background properties.
    """

    def __init__(self, figure):
        super().__init__()
        self._parent = figure

    def get_figure(self, root=False):
        """Return the (sub)figure the background is the background of."""
        return self._parent.get_figure(root=True) if root else self._parent

    def get_transform(self):
        """Return the transform of the unit square: the figure's
        ``transSubfigure``."""
        return self._parent.transSubfigure

    def get_visible(self):
        """Return whether the background is drawn."""
        return _figure.get_frameon(self._parent._native)

    def set_visible(self, b):
        """Set whether the background is drawn."""
        _figure.set_frameon(self._parent._native, bool(b))

    def get_facecolor(self):
        """Return the face color of the background."""
        from .colors import to_rgba
        return to_rgba(_figure.get_facecolor(self._parent._native))

    def _set_facecolor(self, color):
        _figure.set_facecolor(self._parent._native, color)

    def get_edgecolor(self):
        """Return the edge color of the background."""
        from .colors import to_rgba
        return to_rgba(_figure.get_edgecolor(self._parent._native))

    def _set_edgecolor(self, color):
        _figure.set_edgecolor(self._parent._native, color)

    def contains(self, mouseevent):
        """Return whether the mouse event occurred in the background."""
        return self._parent.contains(mouseevent)

    def _clip_spec(self):
        """The native clip path of artists clipped to the background: the
        box of the (sub)figure, as matplotlib clips to a `.Rectangle`."""
        from ._artist_clipping import _rectangle
        from .transforms import Bbox
        return ("bbox", _rectangle(Bbox.unit()),
                _live_transforms.to_native(self._parent.transSubfigure))

    def get_linewidth(self):
        """Return the line width of the background's edge in points."""
        return _figure.get_linewidth(self._parent._native)

    def set_linewidth(self, w):
        """Set the line width of the background's edge in points."""
        _figure.set_linewidth(self._parent._native, float(w))
