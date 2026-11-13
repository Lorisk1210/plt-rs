"""
Provides classes of simple units that will be used with `.AxesDivider`
class (or others) to determine the size of each Axes. The unit
classes define `!get_size` method that returns a tuple of two floats,
meaning relative and absolute sizes, respectively.

Note that this class is nothing more than a simple tuple of two
floats. Take a look at the Divider class to see how these two
values are used.

Once created, the unit classes can be modified by simple arithmetic
operations: addition /subtraction with another unit type or a real number and scaling
(multiplication or division) by a real number.

matplotlib's ``mpl_toolkits.axes_grid1.axes_size`` (from matplotlib 3.11.2,
under the Matplotlib License, kept as it is). Each class also describes
itself to pltrs (Rust) (``_to_native``), whose dividers evaluate the sizes
at every layout pass; matplotlib keeps what a size is made of unchecked
and fails on what it cannot compute with only as a divider divides, so a
size made of what pltrs (Rust) cannot take is described as `.Unusable`
(see ``axes_divider.Divider._locate``).
"""

from numbers import Real

import contextlib
import threading

from pltrs import _api, _named
from pltrs.artist import Artist
from pltrs.axes import Axes
from pltrs._offsetbox_trees import Unusable, number
from pltrs._program_classes import takes_from_program


class _Describing(threading.local):
    """The figure whose divider's sizes are being described
    (`describing_for`)."""

    figure = None


_describing = _Describing()


@contextlib.contextmanager
def describing_for(figure):
    """While the sizes of a divider of *figure* are described: a size of
    the program's is asked with the renderer of *figure* (`_ProgramSize`)."""
    saved, _describing.figure = _describing.figure, figure
    try:
        yield
    finally:
        _describing.figure = saved


class _ProgramSize:
    """A size of the program's as pltrs (Rust) asks it: its ``get_size``
    called with the renderer matplotlib's divider hands it (that of the
    print running, else the one the canvas of *figure* drew with last)."""

    def __init__(self, size, figure):
        self._size = size
        self._figure = figure

    def get_size(self, _renderer):
        from pltrs._python_drawing import made_renderer
        renderer = made_renderer()
        if renderer is None and self._figure is not None:
            # The canvas' renderer as it stands: the figure is being laid
            # out, and cannot be asked for one.
            renderer = vars(self._figure.canvas).get("renderer")
        return self._size.get_size(renderer)


def _described(size):
    """The native description of *size*; `.Unusable` for no size, or one
    made of what pltrs (Rust) cannot take. A size whose ``get_size`` the
    program defines is evaluated through it, whatever class it derives
    from (mplhep's ``RemainderFixed`` derives from `Scaled`)."""
    if takes_from_program(type(size), "get_size"):
        return ("python", _ProgramSize(size, _describing.figure))
    describe = getattr(type(size), "_to_native", None)
    if describe is None:
        raise Unusable
    return describe(size)


def _natives(artists, cls):
    """The natives of *artists*, a list of instances of *cls*;
    `.Unusable` for anything else."""
    try:
        artists = list(artists)
    except TypeError:
        raise Unusable from None
    if not all(isinstance(artist, cls) for artist in artists):
        raise Unusable
    return [artist._native for artist in artists]


class _Base:
    def __rmul__(self, other):
        return self * other

    def __mul__(self, other):
        if not isinstance(other, Real):
            return NotImplemented
        return Fraction(other, self)

    def __div__(self, other):
        return (1 / other) * self

    def __add__(self, other):
        if isinstance(other, _Base):
            return Add(self, other)
        else:
            return Add(self, Fixed(other))

    def __neg__(self):
        return -1 * self

    def __radd__(self, other):
        # other cannot be a _Base instance, because A + B would trigger
        # A.__add__(B) first.
        return Add(self, Fixed(other))

    def __sub__(self, other):
        return self + (-other)

    def get_size(self, renderer):
        """
        Return two-float tuple with relative and absolute sizes.
        """
        raise NotImplementedError("Subclasses must implement")

    def _to_native(self):
        """
        The description of the size pltrs (Rust) evaluates: for a size of
        the program's, the size itself, whose ``get_size`` pltrs (Rust)
        calls at every layout, with no renderer (None), since none is at
        hand while it lays the figure out.
        """
        return ("python", self)


class Add(_Base):
    """
    Sum of two sizes.
    """

    def __init__(self, a, b):
        self._a = a
        self._b = b

    def get_size(self, renderer):
        a_rel_size, a_abs_size = self._a.get_size(renderer)
        b_rel_size, b_abs_size = self._b.get_size(renderer)
        return a_rel_size + b_rel_size, a_abs_size + b_abs_size

    def _to_native(self):
        return ("sum", [_described(self._a), _described(self._b)])


class Fixed(_Base):
    """
    Simple fixed size with absolute part = *fixed_size* and relative part = 0.
    """

    def __init__(self, fixed_size):
        _api.check_isinstance(Real, fixed_size=fixed_size)
        self.fixed_size = fixed_size

    def get_size(self, renderer):
        rel_size = 0.
        abs_size = self.fixed_size
        return rel_size, abs_size

    def _to_native(self):
        return ("fixed", number(self.fixed_size))


class Scaled(_Base):
    """
    Simple scaled(?) size with absolute part = 0 and
    relative part = *scalable_size*.
    """

    def __init__(self, scalable_size):
        self._scalable_size = scalable_size

    def get_size(self, renderer):
        rel_size = self._scalable_size
        abs_size = 0.
        return rel_size, abs_size

    def _to_native(self):
        return ("scaled", number(self._scalable_size))


Scalable = Scaled


def _get_axes_aspect(ax):
    aspect = ax.get_aspect()
    if aspect == "auto":
        aspect = 1.
    return aspect


class _AxesExtent(_Base):
    """The data width or height of an Axes, as a relative size."""

    _native_name = None

    def __init__(self, axes, aspect=1., ref_ax=None):
        self._axes = axes
        self._aspect = aspect
        if aspect == "axes" and ref_ax is None:
            raise ValueError("ref_ax must be set when aspect='axes'")
        self._ref_ax = ref_ax

    def _to_native(self):
        aspect = (("axes", _natives([self._ref_ax], Axes)[0]) if self._aspect == "axes"
                  else number(self._aspect))
        return (self._native_name, _natives([self._axes], Axes)[0], aspect)


class AxesX(_AxesExtent):
    """
    Scaled size whose relative part corresponds to the data width
    of the *axes* multiplied by the *aspect*.
    """

    _native_name = "axes_x"
    __init__ = _named.named("AxesX.__init__", _AxesExtent.__init__)

    def get_size(self, renderer):
        l1, l2 = self._axes.get_xlim()
        if self._aspect == "axes":
            ref_aspect = _get_axes_aspect(self._ref_ax)
            aspect = ref_aspect / _get_axes_aspect(self._axes)
        else:
            aspect = self._aspect

        rel_size = abs(l2-l1)*aspect
        abs_size = 0.
        return rel_size, abs_size


class AxesY(_AxesExtent):
    """
    Scaled size whose relative part corresponds to the data height
    of the *axes* multiplied by the *aspect*.
    """

    _native_name = "axes_y"
    __init__ = _named.named("AxesY.__init__", _AxesExtent.__init__)

    def get_size(self, renderer):
        l1, l2 = self._axes.get_ylim()

        if self._aspect == "axes":
            aspect = _get_axes_aspect(self._axes)
        else:
            aspect = self._aspect

        rel_size = abs(l2-l1)*aspect
        abs_size = 0.
        return rel_size, abs_size


class MaxExtent(_Base):
    """
    Size whose absolute part is either the largest width or the largest height
    of the given *artist_list*.
    """

    def __init__(self, artist_list, w_or_h):
        self._artist_list = artist_list
        _api.check_in_list(["width", "height"], w_or_h=w_or_h)
        self._w_or_h = w_or_h

    def add_artist(self, a):
        self._artist_list.append(a)

    def get_size(self, renderer):
        rel_size = 0.
        extent_list = [
            getattr(a.get_window_extent(renderer), self._w_or_h) / a.figure.dpi
            for a in self._artist_list]
        abs_size = max(extent_list, default=0)
        return rel_size, abs_size

    def _to_native(self):
        return (f"max_{self._w_or_h}", _natives(self._artist_list, Artist))


class MaxWidth(MaxExtent):
    """
    Size whose absolute part is the largest width of the given *artist_list*.
    """

    def __init__(self, artist_list):
        super().__init__(artist_list, "width")


class MaxHeight(MaxExtent):
    """
    Size whose absolute part is the largest height of the given *artist_list*.
    """

    def __init__(self, artist_list):
        super().__init__(artist_list, "height")


class Fraction(_Base):
    """
    An instance whose size is a *fraction* of the *ref_size*.

    >>> s = Fraction(0.3, AxesX(ax))
    """

    def __init__(self, fraction, ref_size):
        _api.check_isinstance(Real, fraction=fraction)
        self._fraction_ref = ref_size
        self._fraction = fraction

    def get_size(self, renderer):
        if self._fraction_ref is None:
            return self._fraction, 0.
        else:
            r, a = self._fraction_ref.get_size(renderer)
            rel_size = r*self._fraction
            abs_size = a*self._fraction
            return rel_size, abs_size

    def _to_native(self):
        return ("fraction", number(self._fraction),
                None if self._fraction_ref is None
                else _described(self._fraction_ref))


def from_any(size, fraction_ref=None):
    """
    Create a Fixed unit when the first argument is a float, or a
    Fraction unit if that is a string that ends with %. The second
    argument is only meaningful when Fraction unit is created.

    >>> from mpl_toolkits.axes_grid1.axes_size import from_any
    >>> a = from_any(1.2) # => Fixed(1.2)
    >>> from_any("50%", a) # => Fraction(0.5, a)
    """
    if isinstance(size, Real):
        return Fixed(size)
    elif isinstance(size, str):
        if size[-1] == "%":
            return Fraction(float(size[:-1]) / 100, fraction_ref)
    raise ValueError("Unknown format")


class _AxesDecorationsSize(_Base):
    """
    Fixed size, corresponding to the size of decorations on a given Axes side.
    """

    _get_size_map = {
        "left":   lambda tight_bb, axes_bb: axes_bb.xmin - tight_bb.xmin,
        "right":  lambda tight_bb, axes_bb: tight_bb.xmax - axes_bb.xmax,
        "bottom": lambda tight_bb, axes_bb: axes_bb.ymin - tight_bb.ymin,
        "top":    lambda tight_bb, axes_bb: tight_bb.ymax - axes_bb.ymax,
    }

    def __init__(self, ax, direction):
        _api.check_in_list(self._get_size_map, direction=direction)
        self._direction = direction
        self._ax_list = [ax] if isinstance(ax, Axes) else ax

    def get_size(self, renderer):
        sz = max([
            self._get_size_map[self._direction](
                ax.get_tightbbox(renderer, call_axes_locator=False), ax.bbox)
            for ax in self._ax_list])
        dpi = renderer.points_to_pixels(72)
        abs_size = sz / dpi
        rel_size = 0
        return rel_size, abs_size

    def _to_native(self):
        return ("decorations", _natives(self._ax_list, Axes), self._direction)
