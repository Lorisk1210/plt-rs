"""
Default legend handlers (matplotlib's ``legend_handler``).

A handler says how a legend draws the entry of a kind of handle: a line, a
patch, a collection, a container or a tuple of handles. The handler map of
a legend (`.Legend.get_default_handler_map`, the *handler_map* keyword)
maps handle types, or handles themselves, to handlers.

The classes and their parameters are matplotlib 3.11.2's, under the
Matplotlib License. pltrs (Rust) draws the entries (``pltrs::legend``): a
legend passes it the parameters of the handlers it uses, as far as pltrs
(Rust)'s handlers take them (`HandlerTuple`'s layout).
"""


def update_from_first_child(tgt, src):
    """Update *tgt* from the first child of the container *src*."""
    first_child = next(iter(src.get_children()), None)
    if first_child is not None:
        tgt.update_from(first_child)


class HandlerBase:
    """
    A base class for default legend handlers.

    The derived classes are meant to override *create_artists* method, which
    has the following signature::

      def create_artists(self, legend, orig_handle,
                         xdescent, ydescent, width, height, fontsize,
                         trans):

    The overridden method needs to create artists of the given
    transform that fits in the given dimension (xdescent, ydescent,
    width, height) that are scaled by fontsize if necessary.
    """

    def __init__(self, xpad=0., ypad=0., update_func=None):
        """
        Parameters
        ----------
        xpad : float, optional
            Padding in x-direction.
        ypad : float, optional
            Padding in y-direction.
        update_func : callable, optional
            Function for updating the legend handler properties from another
            legend handler, used by `~HandlerBase.update_prop`.
        """
        self._xpad, self._ypad = xpad, ypad
        self._update_prop_func = update_func


class HandlerNpoints(HandlerBase):
    """
    A legend handler that shows *numpoints* points in the legend entry.
    """

    def __init__(self, marker_pad=0.3, numpoints=None, **kwargs):
        """
        Parameters
        ----------
        marker_pad : float
            Padding between points in legend entry.
        numpoints : int
            Number of points to show in legend entry.
        **kwargs
            Keyword arguments forwarded to `.HandlerBase`.
        """
        super().__init__(**kwargs)

        self._numpoints = numpoints
        self._marker_pad = marker_pad

    def get_numpoints(self, legend):
        """The number of points the entry shows: the handler's, else the
        legend's."""
        if self._numpoints is None:
            return legend.numpoints
        else:
            return self._numpoints


class HandlerNpointsYoffsets(HandlerNpoints):
    """
    A legend handler that shows *numpoints* in the legend, and allows them to
    be individually offset in the y-direction.
    """

    def __init__(self, numpoints=None, yoffsets=None, **kwargs):
        """
        Parameters
        ----------
        numpoints : int
            Number of points to show in legend entry.
        yoffsets : array of floats
            Length *numpoints* list of y offsets for each point in
            legend entry.
        **kwargs
            Keyword arguments forwarded to `.HandlerNpoints`.
        """
        super().__init__(numpoints=numpoints, **kwargs)
        self._yoffsets = yoffsets


class HandlerLine2DCompound(HandlerNpoints):
    """
    Original handler for `.Line2D` instances, that relies on combining
    a line-only with a marker-only artist.
    """


class HandlerLine2D(HandlerNpoints):
    """
    Handler for `.Line2D` instances.
    """


class HandlerPatch(HandlerBase):
    """
    Handler for `.Patch` instances.
    """

    def __init__(self, patch_func=None, **kwargs):
        """
        Parameters
        ----------
        patch_func : callable, optional
            The function that creates the legend key artist.
        **kwargs
            Keyword arguments forwarded to `.HandlerBase`.
        """
        super().__init__(**kwargs)
        self._patch_func = patch_func


class HandlerStepPatch(HandlerBase):
    """
    Handler for `~.matplotlib.patches.StepPatch` instances.
    """


class HandlerLineCollection(HandlerLine2D):
    """
    Handler for `.LineCollection` instances.
    """


class HandlerRegularPolyCollection(HandlerNpointsYoffsets):
    r"""Handler for `.RegularPolyCollection`\s."""

    def __init__(self, yoffsets=None, sizes=None, **kwargs):
        super().__init__(yoffsets=yoffsets, **kwargs)

        self._sizes = sizes


class HandlerPathCollection(HandlerRegularPolyCollection):
    r"""Handler for `.PathCollection`\s, which are used by `~.Axes.scatter`."""


class HandlerCircleCollection(HandlerRegularPolyCollection):
    r"""Handler for `.CircleCollection`\s."""


class HandlerErrorbar(HandlerLine2D):
    """Handler for Errorbars."""

    def __init__(self, xerr_size=0.5, yerr_size=None,
                 marker_pad=0.3, numpoints=None, **kwargs):

        self._xerr_size = xerr_size
        self._yerr_size = yerr_size

        super().__init__(marker_pad=marker_pad, numpoints=numpoints, **kwargs)


class HandlerStem(HandlerNpointsYoffsets):
    """
    Handler for plots produced by `~.Axes.stem`.
    """

    def __init__(self, marker_pad=0.3, numpoints=None,
                 bottom=None, yoffsets=None, **kwargs):
        """
        Parameters
        ----------
        marker_pad : float, default: 0.3
            Padding between points in legend entry.
        numpoints : int, optional
            Number of points to show in legend entry.
        bottom : float, optional

        yoffsets : array of floats, optional
            Length *numpoints* list of y offsets for each point in
            legend entry.
        **kwargs
            Keyword arguments forwarded to `.HandlerNpointsYoffsets`.
        """
        super().__init__(marker_pad=marker_pad, numpoints=numpoints,
                         yoffsets=yoffsets, **kwargs)
        self._bottom = bottom


class HandlerTuple(HandlerBase):
    """
    Handler for Tuple.
    """

    def __init__(self, ndivide=1, pad=None, **kwargs):
        """
        Parameters
        ----------
        ndivide : int or None, default: 1
            The number of sections to divide the legend area into.  If None,
            use the length of the input tuple.
        pad : float, default: :rc:`legend.borderpad`
            Padding in units of fraction of font size.
        **kwargs
            Keyword arguments forwarded to `.HandlerBase`.
        """
        self._ndivide = ndivide
        self._pad = pad
        super().__init__(**kwargs)


class HandlerPolyCollection(HandlerBase):
    """
    Handler for `.PolyCollection` used in `~.Axes.fill_between` and
    `~.Axes.stackplot`.
    """
