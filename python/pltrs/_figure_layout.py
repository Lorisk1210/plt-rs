"""
The layout methods of figures and subfigures (matplotlib's
``FigureBase.suptitle``, ``supxlabel``, ``supylabel``, the ``align_*``
methods, ``get_tightbbox`` and ``Figure.tight_layout``), as mixins the
classes include. pltrs (Rust) lays the figures out (``pltrs::layout``).
"""

import numpy as np

from . import _api, _handles
from ._native import figure as _figure, figure_layout as _figure_layout
from .text import Text, _layout_renderer, _measuring_with
from .transforms import Bbox, TransformedBbox

#: The native function making each figure-wide text, and whether its
#: position across its edge is its y (else its x).
_SUPER_TEXTS = {
    "title": (_figure.suptitle, True),
    "xlabel": (_figure.supxlabel, True),
    "ylabel": (_figure.supylabel, False),
}


def _axes_natives(axs):
    """The native handles of *axs*, or None for all the figure's axes."""
    return None if axs is None else [ax._native for ax in axs]


class FigureBaseLayout:
    """The figure-wide texts and the alignment of a figure or subfigure."""

    def _suplabels(self, t, kind, kwargs):
        """
        Add or update the figure-wide text *kind* (``"title"``,
        ``"xlabel"`` or ``"ylabel"``) showing *t*, as matplotlib's
        ``_suplabels`` does: its defaults come back, *x* and *y* place it,
        and the layout engine places it across its edge unless that
        coordinate is given.
        """
        x = kwargs.pop('x', None)
        y = kwargs.pop('y', None)
        make, across_is_y = _SUPER_TEXTS[kind]
        text = _handles.wrap(make(self._native, str(t)), Text, self.figure)
        across, along = (y, x) if across_is_y else (x, y)
        if across is not None:
            _figure_layout.place_super_text(self._native, kind, float(across))
        if along is not None:
            (text.set_x if across_is_y else text.set_y)(along)
        text._internal_update(kwargs)
        self.stale = True
        return text

    def suptitle(self, t, **kwargs):
        """
        Add a centered super title to the figure (top center by default;
        *x*, *y* place it, in figure coordinates; the other keywords are
        `.Text` properties).
        """
        return self._suplabels(t, "title", kwargs)

    def supxlabel(self, t, **kwargs):
        """Add a centered super xlabel to the figure (see `suptitle`)."""
        return self._suplabels(t, "xlabel", kwargs)

    def supylabel(self, t, **kwargs):
        """Add a centered super ylabel to the figure (see `suptitle`)."""
        return self._suplabels(t, "ylabel", kwargs)

    def _super_text(self, index):
        """The figure-wide text *index* (title, xlabel, ylabel) or None."""
        native = _figure_layout.super_texts(self._native)[index]
        return None if native is None else _handles.wrap(native, Text,
                                                         self.figure)

    _suptitle = property(lambda self: self._super_text(0),
                         doc="The figure title, or None.")
    _supxlabel = property(lambda self: self._super_text(1),
                          doc="The figure-wide x label, or None.")
    _supylabel = property(lambda self: self._super_text(2),
                          doc="The figure-wide y label, or None.")

    def get_suptitle(self):
        """Return the suptitle as string or an empty string if not set."""
        return _figure.super_texts(self._native)[0]

    def get_supxlabel(self):
        """Return the supxlabel as string or an empty string if not set."""
        return _figure.super_texts(self._native)[1]

    def get_supylabel(self):
        """Return the supylabel as string or an empty string if not set."""
        return _figure.super_texts(self._native)[2]

    def align_xlabels(self, axs=None):
        """
        Align the xlabels of subplots in the same subplot row if label
        alignment is being done automatically (i.e. the label position is
        not manually set).

        Parameters
        ----------
        axs : list of `~matplotlib.axes.Axes`
            Optional list of (or `~numpy.ndarray`) `~matplotlib.axes.Axes`
            to align the xlabels.
            Default is to align all Axes on the figure.
        """
        _figure_layout.align(self._native, _axes_natives(axs), "x")

    def align_ylabels(self, axs=None):
        """
        Align the ylabels of subplots in the same subplot column if label
        alignment is being done automatically (i.e. the label position is
        not manually set).

        Parameters
        ----------
        axs : list of `~matplotlib.axes.Axes`
            Optional list (or `~numpy.ndarray`) of `~matplotlib.axes.Axes`
            to align the ylabels.
            Default is to align all Axes on the figure.
        """
        _figure_layout.align(self._native, _axes_natives(axs), "y")

    def align_titles(self, axs=None):
        """
        Align the titles of subplots in the same subplot row if title
        alignment is being done automatically (i.e. the title position is
        not manually set).

        Parameters
        ----------
        axs : list of `~matplotlib.axes.Axes`
            Optional list of (or ndarray) `~matplotlib.axes.Axes`
            to align the titles.
            Default is to align all Axes on the figure.
        """
        _figure_layout.align(self._native, _axes_natives(axs), "titles")

    def align_labels(self, axs=None):
        """
        Align the xlabels and ylabels of subplots with the same subplots
        row or column (respectively) if label alignment is being
        done automatically (i.e. the label position is not manually set).

        Parameters
        ----------
        axs : list of `~matplotlib.axes.Axes`
            Optional list (or `~numpy.ndarray`) of `~matplotlib.axes.Axes`
            to align the labels.
            Default is to align all Axes on the figure.
        """
        self.align_xlabels(axs=axs)
        self.align_ylabels(axs=axs)

    def autofmt_xdate(
            self, bottom=0.2, rotation=30, ha='right', which='major'):
        """
        Date ticklabels often overlap, so it is useful to rotate them
        and right align them.  Also, a common use case is a number of
        subplots with shared x-axis where the x-axis is date data.  The
        ticklabels are often long, and it helps to rotate them on the
        bottom subplot and turn them off on other subplots, as well as
        turn off xlabels.

        Parameters
        ----------
        bottom : float, default: 0.2
            The bottom of the subplots for `subplots_adjust`.
        rotation : float, default: 30 degrees
            The rotation angle of the xtick labels in degrees.
        ha : {'left', 'center', 'right'}, default: 'right'
            The horizontal alignment of the xticklabels.
        which : {'major', 'minor', 'both'}, default: 'major'
            Selects which ticklabels to rotate.
        """
        _api.check_in_list(['major', 'minor', 'both'], which=which)
        _api.check_in_list(['left', 'center', 'right'], ha=ha)
        _figure_layout.autofmt_xdate(self._native, float(bottom),
                                     float(rotation), ha, which)
        self.stale = True

    def get_default_bbox_extra_artists(self):
        """
        Return a list of Artists typically used in `.Figure.get_tightbbox`.
        """
        bbox_artists = [artist for artist in self.get_children()
                        if (artist.get_visible() and artist.get_in_layout())]
        for ax in self.axes:
            if ax.get_visible():
                bbox_artists.extend(ax.get_default_bbox_extra_artists())
        return bbox_artists

    def get_tightbbox(self, renderer=None, *, bbox_extra_artists=None):
        """
        Return a (tight) bounding box of the figure *in inches*.

        Note that `.FigureBase` differs from all other artists, which return
        their `.Bbox` in pixels.

        Artists that have ``artist.set_in_layout(False)`` are not included
        in the bbox.

        Parameters
        ----------
        renderer : `.RendererBase` subclass
            The renderer (pltrs (Rust) measures the figure itself).
        bbox_extra_artists : list of `.Artist` or ``None``
            List of artists to include in the tight bounding box.  If
            ``None`` (default), then all artist children of each Axes are
            included in the tight bounding box.

        Returns
        -------
        `.BboxBase`
            containing the bounding box (in figure inches).
        """
        if bbox_extra_artists is None:
            x0, y0, x1, y1 = _figure_layout.figure_tightbbox(self._native)
            return Bbox([[x0, y0], [x1, y1]])
        boxes = [artist.get_tightbbox(renderer) for artist in bbox_extra_artists]
        for ax in self.axes:
            if ax.get_visible():
                # Some Axes don't take the bbox_extra_artists keyword.
                try:
                    boxes.append(ax.get_tightbbox(
                        renderer, bbox_extra_artists=bbox_extra_artists))
                except TypeError:
                    boxes.append(ax.get_tightbbox(renderer))
        boxes = [box for box in boxes
                 if box is not None and np.isfinite(box.width)
                 and np.isfinite(box.height)
                 and (box.width != 0 or box.height != 0)]
        isfigure = hasattr(self, 'bbox_inches')
        if not boxes:
            if isfigure:
                return self.bbox_inches
            # Subfigures do not have bbox_inches, but do have a bbox.
            boxes = [self.bbox]
        bbox = Bbox.union(boxes)
        if isfigure:
            # Transform from pixels to inches.
            bbox = TransformedBbox(bbox, self.dpi_scale_trans.inverted())
        return bbox


class FigureLayout:
    """`tight_layout` of a figure."""

    def tight_layout(self, *, pad=1.08, h_pad=None, w_pad=None, rect=None):
        """
        Adjust the padding between and around subplots.

        To exclude an artist on the Axes from the bounding box calculation
        that determines the subplot parameters (i.e. legend, or annotation),
        set ``a.set_in_layout(False)`` for that artist.

        Parameters
        ----------
        pad : float, default: 1.08
            Padding between the figure edge and the edges of subplots,
            as a fraction of the font size.
        h_pad, w_pad : float, default: *pad*
            Padding (height/width) between edges of adjacent subplots,
            as a fraction of the font size.
        rect : tuple (left, bottom, right, top), default: (0, 0, 1, 1)
            A rectangle in normalized figure coordinates into which the whole
            subplots area (including labels) will fit.

        See Also
        --------
        .Figure.set_layout_engine
        .pyplot.tight_layout
        """
        from .layout_engine import PlaceHolderLayoutEngine, TightLayoutEngine
        previous_engine = self.get_layout_engine()
        try:
            with _measuring_with(_layout_renderer(self)):
                replaced, not_applied = _figure_layout.tight_layout(
                    self._native, float(pad), h_pad, w_pad,
                    None if rect is None else tuple(rect))
        except RuntimeError:
            # The colorbars refused tight layout; the engine is removed
            # all the same.
            self._layout_engine = PlaceHolderLayoutEngine(
                previous_engine.adjust_compatible,
                previous_engine.colorbar_gridspec)
            raise
        if not replaced:
            return  # pltrs' modern looks keep constrained layout.
        # The pass runs once: the figure is left without an engine.
        self._layout_engine = PlaceHolderLayoutEngine(True, True)
        if not_applied is not None:
            _api.warn_external(not_applied)
        if previous_engine is not None and not isinstance(
                previous_engine, (TightLayoutEngine, PlaceHolderLayoutEngine)):
            _api.warn_external('The figure layout has changed to tight')
