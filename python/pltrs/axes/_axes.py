"""
The `Axes` class: plotting methods on a pltrs (Rust) axes (matplotlib's
``axes._axes`` and ``axes._base``).

Each plotting method turns matplotlib's signature (positional forms,
aliases, defaults) into the explicit arguments of a native function of
``_native.axes``, and wraps the artists it returns (see `.._handles`).
"""

from .. import _handles, cbook
from .._native import axes as _axes
from ._artist_list import ArtistList
from ._base import _AxesBase
from ._clear import ClearMixin
from ..axis import XAxis, YAxis
from ..legend import Legend
from ..lines import Line2D
from ._imaging import ImagingMixin
from ._insets import InsetsMixin
from ._plotting import PlottingMixin
from ._titles import TitlesMixin
from .._text_methods import AxesText


class Axes(PlottingMixin, ImagingMixin, InsetsMixin, AxesText, TitlesMixin,
           ClearMixin, _AxesBase):
    """
    An Axes object encapsulates all the elements of an individual (sub-)plot
    in a figure.

    See matplotlib's documentation of `matplotlib.axes.Axes`.
    """

    #: The name of the projection, under which `.projections` registers
    #: the class.
    name = "rectilinear"

    @classmethod
    def _from_native(cls, native, figure):
        ax = super()._from_native(native, figure)
        ax._init_axes_state()
        return ax

    def _init_axes_state(self):
        super()._init_axes_state()
        self.xaxis = XAxis(self)
        self.yaxis = YAxis(self)

    def set_xlabel(self, xlabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the x-axis (see `.Axis.set_label_text`)."""
        return self.xaxis._set_axes_label(xlabel, fontdict, labelpad, loc,
                                          kwargs)

    def get_xlabel(self):
        """Get the xlabel text string."""
        return _axes.get_xlabel(self._native)

    def set_ylabel(self, ylabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the y-axis (see `.Axis.set_label_text`)."""
        return self.yaxis._set_axes_label(ylabel, fontdict, labelpad, loc,
                                          kwargs)

    def get_ylabel(self):
        """Get the ylabel text string."""
        return _axes.get_ylabel(self._native)

    def _display_to_data(self, xy):
        """The data coordinates of the display point *xy*
        (``self.transData.inverted().transform(xy)``)."""
        x, y = xy
        return _axes.display_to_data(self._native, x, y)

    def get_xlim(self):
        """Return the x-axis view limits."""
        return _axes.get_xlim(self._native)

    def get_ylim(self):
        """Return the y-axis view limits."""
        return _axes.get_ylim(self._native)

    @property
    def lines(self):
        """The lines of the Axes."""
        return ArtistList("lines", lambda: _handles.wrap_all(
            _axes.get_lines(self._native), Line2D, self.figure))

    def get_lines(self):
        """Return a list of lines contained by the Axes."""
        return cbook.silent_list('Line2D', self.lines)

    def get_legend_handles_labels(self, legend_handler_map=None):
        """
        Return handles and labels for legend

        ``ax.legend()`` is equivalent to ::

          h, l = ax.get_legend_handles_labels()
          ax.legend(h, l)
        """
        from .. import legend as mlegend
        return mlegend._get_legend_handles_labels([self], legend_handler_map)

    def legend(self, *args, **kwargs):
        """
        Place a legend on the Axes.

        Call signatures::

            legend()
            legend(handles, labels)
            legend(handles=handles)
            legend(labels)

        See matplotlib's documentation of `matplotlib.axes.Axes.legend` for
        the keyword arguments, which are those of `.Legend`.

        Returns
        -------
        `~matplotlib.legend.Legend`
        """
        from .. import legend as mlegend
        handles, labels, kwargs = mlegend._parse_legend_args([self], *args, **kwargs)
        return mlegend.Legend(self, handles, labels, **kwargs)

    def get_legend(self):
        """Return the `.Legend` instance, or None if no legend is defined."""
        from .._native import legend as _legend
        return _handles.wrap(_legend.axes_legend(self._native), Legend, self.figure)

    @property
    def legend_(self):
        """The `.Legend` of the Axes, or None."""
        return self.get_legend()

    @legend_.setter
    def legend_(self, legend):
        # matplotlib's attribute: None takes the legend off the Axes.
        current = self.get_legend()
        if legend is current:
            return
        if legend is not None:
            raise NotImplementedError(
                "pltrs: legend_ takes None or the Axes' own legend")
        current.remove()
