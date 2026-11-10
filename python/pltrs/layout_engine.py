"""
Classes to layout elements in a `.Figure`.

Figures have a ``layout_engine`` property that holds a subclass of
`~.LayoutEngine` defined here (or *None* for no layout).  At draw time
``figure.get_layout_engine().execute()`` is called, the goal of which is
usually to rearrange Axes on the figure to produce a pleasing layout. This is
like a ``draw`` callback but with two differences.  First, when printing we
disable the layout engine for the final draw. Second, it is useful to know the
layout engine while the figure is being created.  In particular, colorbars are
made differently with different layout engines (for historical reasons).

Matplotlib has two built-in layout engines:

- `.TightLayoutEngine` was the first layout engine added to Matplotlib.
  See also :ref:`tight_layout_guide`.
- `.ConstrainedLayoutEngine` is more modern and generally gives better results.
  See also :ref:`constrainedlayout_guide`.

The classes and their API are matplotlib 3.11.2's, under the Matplotlib
License; the engines themselves are pltrs (Rust)'s (``pltrs::layout``). A
figure gives pltrs (Rust) the description of its engine (`_native_spec`)
whenever the engine or its parameters change.
"""

import weakref

from . import _lazy_imports, _layoutgrid, rcParams

_layout_checks = _lazy_imports.module(globals(), "._layout_checks")


class LayoutEngine:
    """
    Base class for Matplotlib layout engines.

    A layout engine can be passed to a figure at instantiation or at any time
    with `~.figure.Figure.set_layout_engine`.  Once attached to a figure, the
    layout engine ``execute`` function is called at draw time by
    `~.figure.Figure.draw`, providing a special draw-time hook.

    .. note::

       However, note that layout engines affect the creation of colorbars, so
       `~.figure.Figure.set_layout_engine` should be called before any
       colorbars are created.

    Currently, there are two properties of `LayoutEngine` classes that are
    consulted while manipulating the figure:

    - ``engine.colorbar_gridspec`` tells `.Figure.colorbar` whether to make the
       axes using the gridspec method (see `.colorbar.make_axes_gridspec`) or
       not (see `.colorbar.make_axes`);
    - ``engine.adjust_compatible`` stops `.Figure.subplots_adjust` from being
        run if it is not compatible with the layout engine.

    To implement a custom `LayoutEngine`:

    1. override ``_adjust_compatible`` and ``_colorbar_gridspec``
    2. override `LayoutEngine.set` to update *self._params*
    3. override `LayoutEngine.execute` with your implementation

    """

    _adjust_compatible = None
    _colorbar_gridspec = None

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._params = {}
        self._figures = weakref.WeakSet()

    def set(self, **kwargs):
        """
        Set the parameters for the layout engine.
        """
        raise NotImplementedError

    @property
    def colorbar_gridspec(self):
        """
        Return a boolean if the layout engine creates colorbars using a
        gridspec.
        """
        if self._colorbar_gridspec is None:
            raise NotImplementedError
        return self._colorbar_gridspec

    @property
    def adjust_compatible(self):
        """
        Return a boolean if the layout engine is compatible with
        `~.Figure.subplots_adjust`.
        """
        if self._adjust_compatible is None:
            raise NotImplementedError
        return self._adjust_compatible

    def get(self):
        """
        Return copy of the parameters for the layout engine.
        """
        return dict(self._params)

    def __getstate__(self):
        # The figures using the engine are not part of it: a figure read
        # back from a pickle attaches itself again (`Figure.__setstate__`).
        return {**self.__dict__, "_figures": None}

    def __setstate__(self, state):
        self.__dict__.update(state)
        self._figures = weakref.WeakSet()

    def _attach(self, fig):
        """
        Become the engine of *fig*: give pltrs (Rust) its description. An
        engine whose class of the program's lays figures out itself (its
        own ``execute``, as plotnine's) stands as a placeholder there, and
        the figure runs its ``execute`` before every layout of a draw, as
        matplotlib's draw runs it (see `.axes._program_locators`).
        """
        from ._native import figure as _figure
        in_python = self._executes_in_python() or self._lays_out_kept_grids(fig)
        _figure.set_layout_engine(
            fig._native, self._placeholder_spec() if in_python else self._native_spec())
        # An engine of the program's may not have run this class' __init__.
        self.__dict__.setdefault("_figures", weakref.WeakSet()).add(fig)
        if in_python:
            from .axes import _program_locators
            _program_locators.follow_engine(fig)

    def _executes_in_python(self):
        """
        Whether pltrs (Rust) leaves the layout to the engine's ``execute``,
        which the figure runs before every layout of a draw: an engine of
        a class of the program's with its own, or one holding parameters
        pltrs (Rust) cannot hold, which matplotlib fails on as it lays the
        figure out (see `._layout_checks`).
        """
        from ._program_classes import takes_from_program
        return takes_from_program(type(self), "execute") or not self._held_natively()

    def _held_natively(self):
        """Whether pltrs (Rust) holds the engine's parameters (the base
        class has none)."""
        return True

    def _lays_out_kept_grids(self, fig):
        """Whether the engine lays out grids of *fig* holding spacings or
        ratios pltrs (Rust) cannot hold, which matplotlib's layout computes
        with (the base class lays out none)."""
        return False

    def _placeholder_spec(self):
        """A placeholder of the engine as pltrs (Rust) takes it: no layout
        of its own, the engine's compatibility with `.Figure.subplots_adjust`
        and its way of making colorbars."""
        return ("placeholder", bool(self.adjust_compatible),
                bool(self.colorbar_gridspec))

    def _changed(self):
        """Give the figures using this engine its new parameters."""
        for fig in list(self._figures):
            if fig.get_layout_engine() is self:
                self._attach(fig)

    def execute(self, fig):
        """
        Execute the layout on the figure given by *fig*.
        """
        # subclasses must implement this.
        raise NotImplementedError

    def _execute_native(self, fig):
        """
        Have pltrs (Rust) lay *fig* out with the engine: constrained layout
        returns its layout grids, a private debugging structure (see
        `._layoutgrid`), or None without grids.
        """
        return _layoutgrid.execute(fig, self._native_spec())


class PlaceHolderLayoutEngine(LayoutEngine):
    """
    This layout engine does not adjust the figure layout at all.

    The purpose of this `.LayoutEngine` is to act as a placeholder when the user removes
    a layout engine to ensure an incompatible `.LayoutEngine` cannot be set later.

    Parameters
    ----------
    adjust_compatible, colorbar_gridspec : bool
        Allow the PlaceHolderLayoutEngine to mirror the behavior of whatever
        layout engine it is replacing.

    """

    def __init__(self, adjust_compatible, colorbar_gridspec, **kwargs):
        self._adjust_compatible = adjust_compatible
        self._colorbar_gridspec = colorbar_gridspec
        super().__init__(**kwargs)

    def _native_spec(self):
        """The engine as pltrs (Rust) takes it."""
        return self._placeholder_spec()

    def execute(self, fig):
        """
        Do nothing.
        """
        return


class TightLayoutEngine(LayoutEngine):
    """
    Implements the ``tight_layout`` geometry management.  See
    :ref:`tight_layout_guide` for details.
    """
    _adjust_compatible = True
    _colorbar_gridspec = True

    def __init__(self, *, pad=1.08, h_pad=None, w_pad=None,
                 rect=(0, 0, 1, 1), **kwargs):
        """
        Initialize tight_layout engine.

        Parameters
        ----------
        pad : float, default: 1.08
            Padding between the figure edge and the edges of subplots, as a
            fraction of the font size.
        h_pad, w_pad : float
            Padding (height/width) between edges of adjacent subplots.
            Defaults to *pad*.
        rect : tuple (left, bottom, right, top), default: (0, 0, 1, 1).
            rectangle in normalized figure coordinates that the subplots
            (including labels) will fit into.
        """
        super().__init__(**kwargs)
        for td in ['pad', 'h_pad', 'w_pad', 'rect']:
            # initialize these in case None is passed in above:
            self._params[td] = None
        self.set(pad=pad, h_pad=h_pad, w_pad=w_pad, rect=rect)

    def _lays_out_kept_grids(self, fig):
        return _layout_checks.grids_kept(fig)

    def execute(self, fig):
        """
        Execute tight_layout.

        This decides the subplot parameters given the padding that
        will allow the Axes labels to not be covered by other labels
        and Axes.

        Parameters
        ----------
        fig : `.Figure` to perform layout on.

        See Also
        --------
        .figure.Figure.tight_layout
        .pyplot.tight_layout
        """
        # What matplotlib's computes with first (pltrs (Rust) measures with
        # the figure's renderer itself).
        fig._get_renderer()
        if _layout_checks.check_tight(fig, self._params):
            return self._execute_native(fig)

    def _held_natively(self):
        return _layout_checks.holds_tight(self._params)

    def _native_spec(self):
        """The engine as pltrs (Rust) takes it."""
        info = self._params
        return ("tight", float(info['pad']),
                None if info['h_pad'] is None else float(info['h_pad']),
                None if info['w_pad'] is None else float(info['w_pad']),
                tuple(map(float, info['rect'])))

    def set(self, *, pad=None, w_pad=None, h_pad=None, rect=None):
        """
        Set the pads for tight_layout.

        Parameters
        ----------
        pad : float
            Padding between the figure edge and the edges of subplots, as a
            fraction of the font size.
        w_pad, h_pad : float
            Padding (width/height) between edges of adjacent subplots.
            Defaults to *pad*.
        rect : tuple (left, bottom, right, top)
            rectangle in normalized figure coordinates that the subplots
            (including labels) will fit into.
        """
        for td in self.set.__kwdefaults__:
            if locals()[td] is not None:
                self._params[td] = locals()[td]
        self._changed()


class ConstrainedLayoutEngine(LayoutEngine):
    """
    Implements the ``constrained_layout`` geometry management.  See
    :ref:`constrainedlayout_guide` for details.
    """

    _adjust_compatible = False
    _colorbar_gridspec = False

    def __init__(self, *, h_pad=None, w_pad=None,
                 hspace=None, wspace=None, rect=(0, 0, 1, 1),
                 compress=False, **kwargs):
        """
        Initialize ``constrained_layout`` settings.

        Parameters
        ----------
        h_pad, w_pad : float
            Padding around the Axes elements in inches.
            Default to :rc:`figure.constrained_layout.h_pad` and
            :rc:`figure.constrained_layout.w_pad`.
        hspace, wspace : float
            Fraction of the figure to dedicate to space between the
            axes.  These are evenly spread between the gaps between the Axes.
            A value of 0.2 for a three-column layout would have a space
            of 0.1 of the figure width between each column.
            If h/wspace < h/w_pad, then the pads are used instead.
            Default to :rc:`figure.constrained_layout.hspace` and
            :rc:`figure.constrained_layout.wspace`.
        rect : tuple of 4 floats
            Rectangle in figure coordinates to perform constrained layout in
            (left, bottom, width, height), each from 0-1.
        compress : bool
            Whether to shift Axes so that white space in between them is
            removed. This is useful for simple grids of fixed-aspect Axes (e.g.
            a grid of images).  See :ref:`compressed_layout`.
        """
        super().__init__(**kwargs)
        # set the defaults:
        self.set(w_pad=rcParams['figure.constrained_layout.w_pad'],
                 h_pad=rcParams['figure.constrained_layout.h_pad'],
                 wspace=rcParams['figure.constrained_layout.wspace'],
                 hspace=rcParams['figure.constrained_layout.hspace'],
                 rect=(0, 0, 1, 1))
        # set anything that was passed in (None will be ignored):
        self.set(w_pad=w_pad, h_pad=h_pad, wspace=wspace, hspace=hspace,
                 rect=rect)
        self._compress = compress

    def execute(self, fig):
        """
        Perform constrained_layout and move and resize Axes accordingly.

        Parameters
        ----------
        fig : `.Figure` to perform layout on.
        """
        _layout_checks.check_constrained(fig, self._params)
        return self._execute_native(fig)

    def _held_natively(self):
        return _layout_checks.holds_constrained(self._params)

    def _lays_out_kept_grids(self, fig):
        return _layout_checks.grids_kept(fig)

    def _native_spec(self):
        """The engine as pltrs (Rust) takes it."""
        info = self._params
        return ("constrained",
                (float(info['h_pad']), float(info['w_pad']),
                 float(info['hspace']), float(info['wspace'])),
                tuple(float(info['rect'][i]) for i in range(4)), bool(self._compress))

    def set(self, *, h_pad=None, w_pad=None,
            hspace=None, wspace=None, rect=None):
        """
        Set the pads for constrained_layout.

        Parameters
        ----------
        h_pad, w_pad : float
            Padding around the Axes elements in inches.
            Default to :rc:`figure.constrained_layout.h_pad` and
            :rc:`figure.constrained_layout.w_pad`.
        hspace, wspace : float
            Fraction of the figure to dedicate to space between the
            axes.  These are evenly spread between the gaps between the Axes.
            A value of 0.2 for a three-column layout would have a space
            of 0.1 of the figure width between each column.
            If h/wspace < h/w_pad, then the pads are used instead.
            Default to :rc:`figure.constrained_layout.hspace` and
            :rc:`figure.constrained_layout.wspace`.
        rect : tuple of 4 floats
            Rectangle in figure coordinates to perform constrained layout in
            (left, bottom, width, height), each from 0-1.
        """
        for td in self.set.__kwdefaults__:
            if locals()[td] is not None:
                self._params[td] = locals()[td]
        self._changed()


__all__ = ["LayoutEngine", "PlaceHolderLayoutEngine", "TightLayoutEngine",
           "ConstrainedLayoutEngine"]


# What matplotlib's module also has from its imports, which programs
# import from it.
from contextlib import nullcontext  # noqa: E402, F401
from ._constrained_layout import do_constrained_layout  # noqa: E402, F401
from ._tight_layout import get_subplotspec_list, get_tight_layout_figure  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
