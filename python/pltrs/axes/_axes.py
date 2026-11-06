"""
The `Axes` class: plotting methods on a pltrs (Rust) axes (matplotlib's
``axes._axes`` and ``axes._base``).

Each plotting method turns matplotlib's signature (positional forms,
aliases, defaults) into the explicit arguments of a native function of
``_native.axes``, and wraps the artists it returns (see `.._handles`).
"""

from .. import _docstring
from .. import _handles, _values, cbook
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
from ._plotting.bars import _GroupedBarReturn  # noqa: F401  (matplotlib defines it here)
from ._titles import TitlesMixin
from .._text_methods import AxesText


@_docstring.interpd
class Axes(PlottingMixin, ImagingMixin, InsetsMixin, AxesText, TitlesMixin,
           ClearMixin, _AxesBase):
    """
    An Axes object encapsulates all the elements of an individual (sub-)plot in
    a figure.

    It contains most of the (sub-)plot elements: `~.axis.Axis`,
    `~.axis.Tick`, `~.lines.Line2D`, `~.text.Text`, `~.patches.Polygon`, etc.,
    and sets the coordinate system.

    Like all visible elements in a figure, Axes is an `.Artist` subclass.

    The `Axes` instance supports callbacks through a callbacks attribute which
    is a `~.cbook.CallbackRegistry` instance.  The events you can connect to
    are 'xlim_changed' and 'ylim_changed' and the callback will be called with
    func(*ax*) where *ax* is the `Axes` instance.

    .. note::

        As a user, you do not instantiate Axes directly, but use Axes creation
        methods instead; e.g. from `.pyplot` or `.Figure`:
        `~.pyplot.subplots`, `~.pyplot.subplot_mosaic` or `.Figure.add_axes`.

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
        """
        Set the label for the x-axis.

        Parameters
        ----------
        xlabel : str
            The label text.

        labelpad : float, default: :rc:`axes.labelpad`
            Spacing in points from the Axes bounding box including ticks
            and tick labels.  If None, the previous value is left as is.

        loc : {'left', 'center', 'right'}, default: :rc:`xaxis.labellocation`
            The label position. This is a high-level alternative for passing
            parameters *x* and *horizontalalignment*.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.text.Text` properties
            `.Text` properties control the appearance of the label.

        See Also
        --------
        text : Documents the properties supported by `.Text`.
        """
        return self.xaxis._set_axes_label(xlabel, fontdict, labelpad, loc,
                                          kwargs)

    def get_xlabel(self):
        """
        Get the xlabel text string.
        """
        return _axes.get_xlabel(self._native)

    def set_ylabel(self, ylabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """
        Set the label for the y-axis.

        Parameters
        ----------
        ylabel : str
            The label text.

        labelpad : float, default: :rc:`axes.labelpad`
            Spacing in points from the Axes bounding box including ticks
            and tick labels.  If None, the previous value is left as is.

        loc : {'bottom', 'center', 'top'}, default: :rc:`yaxis.labellocation`
            The label position. This is a high-level alternative for passing
            parameters *y* and *horizontalalignment*.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.text.Text` properties
            `.Text` properties control the appearance of the label.

        See Also
        --------
        text : Documents the properties supported by `.Text`.
        """
        return self.yaxis._set_axes_label(ylabel, fontdict, labelpad, loc,
                                          kwargs)

    def get_ylabel(self):
        """
        Get the ylabel text string.
        """
        return _axes.get_ylabel(self._native)

    def _display_to_data(self, xy):
        """The data coordinates of the display point *xy*
        (``self.transData.inverted().transform(xy)``)."""
        x, y = xy
        # numpy floats, as matplotlib's transform gives them.
        return _values.float64s(_axes.display_to_data(self._native, x, y))

    def get_xlim(self):
        """
        Return the x-axis view limits.

        Returns
        -------
        left, right : (float, float)
            The current x-axis limits in data coordinates.

        See Also
        --------
        .Axes.set_xlim
        .Axes.set_xbound, .Axes.get_xbound
        .Axes.invert_xaxis, .Axes.xaxis_inverted

        Notes
        -----
        The x-axis may be inverted, in which case the *left* value will
        be greater than the *right* value.
        """
        return _values.float64s(_axes.get_xlim(self._native))

    def get_ylim(self):
        """
        Return the y-axis view limits.

        Returns
        -------
        bottom, top : (float, float)
            The current y-axis limits in data coordinates.

        See Also
        --------
        .Axes.set_ylim
        .Axes.set_ybound, .Axes.get_ybound
        .Axes.invert_yaxis, .Axes.yaxis_inverted

        Notes
        -----
        The y-axis may be inverted, in which case the *bottom* value
        will be greater than the *top* value.
        """
        return _values.float64s(_axes.get_ylim(self._native))

    @property
    def lines(self):
        return ArtistList._of("lines", lambda: _handles.wrap_all(
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

    @_docstring.interpd
    def legend(self, *args, **kwargs):
        """
        Place a legend on the Axes.

        Call signatures::

            legend()
            legend(handles, labels)
            legend(handles=handles)
            legend(labels)

        The call signatures correspond to the following different ways to use
        this method:

        **1. Automatic detection of elements to be shown in the legend**

        The elements to be added to the legend are automatically determined,
        when you do not pass in any extra arguments.

        In this case, the labels are taken from the artist. You can specify
        them either at artist creation or by calling the
        :meth:`~.Artist.set_label` method on the artist::

            ax.plot([1, 2, 3], label='Inline label')
            ax.legend()

        or::

            line, = ax.plot([1, 2, 3])
            line.set_label('Label via method')
            ax.legend()

        .. note::
            Specific artists can be excluded from the automatic legend element
            selection by using a label starting with an underscore, "_".
            A string starting with an underscore is the default label for all
            artists, so calling `.Axes.legend` without any arguments and
            without setting the labels manually will result in a ``UserWarning``
            and an empty legend being drawn.


        **2. Explicitly listing the artists and labels in the legend**

        For full control of which artists have a legend entry, it is possible
        to pass an iterable of legend artists followed by an iterable of
        legend labels respectively::

            ax.legend([line1, line2, line3], ['label1', 'label2', 'label3'])


        **3. Explicitly listing the artists in the legend**

        This is similar to 2, but the labels are taken from the artists'
        label properties. Example::

            line1, = ax.plot([1, 2, 3], label='label1')
            line2, = ax.plot([1, 2, 3], label='label2')
            ax.legend(handles=[line1, line2])


        **4. Labeling existing plot elements**

        .. admonition:: Discouraged

            This call signature is discouraged, because the relation between
            plot elements and labels is only implicit by their order and can
            easily be mixed up.

        To make a legend for all artists on an Axes, call this function with
        an iterable of strings, one for each legend item. For example::

            ax.plot([1, 2, 3])
            ax.plot([5, 6, 7])
            ax.legend(['First line', 'Second line'])


        Parameters
        ----------
        handles : list of (`.Artist` or tuple of `.Artist`), optional
            A list of Artists (lines, patches) to be added to the legend.
            Use this together with *labels*, if you need full control on what
            is shown in the legend and the automatic mechanism described above
            is not sufficient.

            The length of handles and labels should be the same in this
            case. If they are not, they are truncated to the smaller length.

            If an entry contains a tuple, then the legend handler for all Artists in the
            tuple will be placed alongside a single label.

        labels : list of str, optional
            A list of labels to show next to the artists.
            Use this together with *handles*, if you need full control on what
            is shown in the legend and the automatic mechanism described above
            is not sufficient.

        Returns
        -------
        `~matplotlib.legend.Legend`

        Other Parameters
        ----------------
        %(_legend_kw_axes)s

        See Also
        --------
        .Figure.legend

        Notes
        -----
        Some artists are not supported by this function.  See
        :ref:`legend_guide` for details.

        Examples
        --------
        .. plot:: gallery/text_labels_and_annotations/legend.py
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
        # matplotlib's attribute: None takes the legend off the Axes, a
        # legend made for the Axes becomes its legend again.
        current = self.get_legend()
        if legend is current:
            return
        if legend is None:
            current.remove()
            return
        from .._native import legend as _legend
        if not _legend.set_axes_legend(self._native, legend._native):
            raise ValueError("legend_ must be a Legend made for this Axes")


# matplotlib defines the methods of the mixins on Axes itself.
cbook._own_mixin_methods(Axes, _AxesBase)
