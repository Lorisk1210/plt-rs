"""
The `Axes` class: plotting methods on a pltrs (Rust) axes (matplotlib's
``axes._axes``).

Each plotting method turns matplotlib's signature (positional forms,
aliases, defaults) into the explicit arguments of a native function of
``_native.axes``, and wraps the artists it returns (see `.._handles`).
"""

from .. import _docstring
from .. import cbook
from . import _failed_calls
from ._artist_list import ArtistList
from ._base import _AxesBase
from ..axis import XAxis, YAxis
from ..legend import Legend
from ._imaging import ImagingMixin
from ._insets import InsetsMixin
from ._plotting import PlottingMixin
from ._plotting.bars import _GroupedBarReturn  # noqa: F401  (matplotlib defines it here)
from ._titles import TitlesMixin
from .._text_methods import AxesText


@_docstring.interpd
class Axes(PlottingMixin, ImagingMixin, InsetsMixin, AxesText, TitlesMixin,
           _AxesBase):
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
        ax._connect_unit_change_handler()
        return ax

    def _init_axes_state(self):
        super()._init_axes_state()
        self.xaxis = XAxis(self)
        self.yaxis = YAxis(self)

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
        legend = mlegend.Legend(self, handles, labels, **kwargs)
        if isinstance(legend, Legend):  # (A program's stand-in is not drawn.)
            self.legend_ = legend
        return legend


# matplotlib defines the methods of the mixins on Axes itself.
cbook._own_mixin_methods(Axes, _AxesBase)
_failed_calls.guard(Axes)


def __getattr__(name):
    # matplotlib's parser of format strings, a private function of this
    # module that libraries call (healpy's projection Axes).
    if name == "_process_plot_format":
        from ._plotting._events_reference import _process_plot_format
        return _process_plot_format
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
