"""
Ready-made anchored boxes (matplotlib's
``mpl_toolkits.axes_grid1.anchored_artists``, from matplotlib 3.11.2, under
the Matplotlib License): a drawing area, a box of artists in data
coordinates, a scale bar and direction arrows. Each keeps its arguments,
and pltrs (Rust) builds the box when an axes adds it
(``ax.add_artist(box)``, ``_native.axes_grid1``).
"""

from pltrs import _handles, _live_transforms, cbook
from pltrs.font_manager import FontManager
from pltrs.offsetbox import AnchoredOffsetbox, AuxTransformBox, DrawingArea
from pltrs.patches import Patch
from pltrs._native import axes_grid1 as _axes_grid1

__all__ = ['AnchoredDrawingArea', 'AnchoredAuxTransformBox',
           'AnchoredSizeBar', 'AnchoredDirectionArrows']


def _native_font(prop):
    """The native font properties of *prop* (None stays None)."""
    if prop is None:
        return None
    from pltrs.font_manager import FontProperties
    return FontManager._props(FontProperties._from_any(prop))


# An anchored box pltrs (Rust) builds when an axes adds it (each subclass's
# ``_add_native`` adds it and returns its handle). No docstring, so that
# ``help`` shows `.AnchoredOffsetbox`'s for the subclasses, as matplotlib's.
class _NativeAnchoredBox(AnchoredOffsetbox):

    def _add_to_axes(self, axes, kind, autolim=True):
        return _handles.adopt(self, self._add_native(axes), axes.figure)

    def _frame(self):
        """The paddings and frame of the box, as pltrs (Rust) takes them."""
        return (float(self.pad), float(self.borderpad), bool(self.frameon))

    def get_children(self):
        """
        Return the list of children: the child box, which a size bar and
        direction arrows do not have (pltrs (Rust) builds and draws their
        boxes whole).
        """
        return [] if self._child is None else [self._child]


class AnchoredDrawingArea(_NativeAnchoredBox):
    def __init__(self, width, height, xdescent, ydescent,
                 loc, pad=0.4, borderpad=0.5, prop=None, frameon=True,
                 **kwargs):
        """
        An anchored container with a fixed size and fillable `.DrawingArea`.

        Artists added to the *drawing_area* will have their coordinates
        interpreted as pixels. Any transformations set on the artists will be
        overridden.

        Parameters
        ----------
        width, height : float
            Width and height of the container, in pixels.
        xdescent, ydescent : float
            Descent of the container in the x- and y- direction, in pixels.
        loc : str
            Location of this artist.  Valid locations are
            'upper left', 'upper center', 'upper right',
            'center left', 'center', 'center right',
            'lower left', 'lower center', 'lower right'.
            For backward compatibility, numeric values are accepted as well.
            See the parameter *loc* of `.Legend` for details.
        pad : float, default: 0.4
            Padding around the child objects, in fraction of the font size.
        borderpad : float, default: 0.5
            Border padding, in fraction of the font size.
        prop : `~matplotlib.font_manager.FontProperties`, optional
            Font property used as a reference for paddings.
        frameon : bool, default: True
            If True, draw a box around this artist.
        **kwargs
            Keyword arguments forwarded to `.AnchoredOffsetbox`.

        Attributes
        ----------
        drawing_area : `~matplotlib.offsetbox.DrawingArea`
            A container for artists to display.

        Examples
        --------
        To display blue and red circles of different sizes in the upper right
        of an Axes *ax*:

        >>> ada = AnchoredDrawingArea(20, 20, 0, 0,
        ...                           loc='upper right', frameon=False)
        >>> ada.drawing_area.add_artist(Circle((10, 10), 10, fc="b"))
        >>> ada.drawing_area.add_artist(Circle((30, 10), 5, fc="r"))
        >>> ax.add_artist(ada)
        """
        self.da = DrawingArea(width, height, xdescent, ydescent)
        self.drawing_area = self.da
        self._size = tuple(map(float, (width, height, xdescent, ydescent)))
        super().__init__(
            loc, pad=pad, borderpad=borderpad, child=self.da, prop=None,
            frameon=frameon, **kwargs
        )

    def _add_native(self, axes):
        return _axes_grid1.add_drawing_area(
            axes._native, (self._size, self.loc), self._frame(),
            [child._native for child in self.da.get_children()])


class AnchoredAuxTransformBox(_NativeAnchoredBox):
    def __init__(self, transform, loc,
                 pad=0.4, borderpad=0.5, prop=None, frameon=True, **kwargs):
        """
        An anchored container with transformed coordinates.

        Artists added to the *drawing_area* are scaled according to the
        coordinates of the transformation used. The dimensions of this artist
        will scale to contain the artists added.

        Parameters
        ----------
        transform : `~matplotlib.transforms.Transform`
            The transformation object for the coordinate system in use, i.e.,
            :attr:`!matplotlib.axes.Axes.transData`.
        loc : str
            Location of this artist.  Valid locations are
            'upper left', 'upper center', 'upper right',
            'center left', 'center', 'center right',
            'lower left', 'lower center', 'lower right'.
            For backward compatibility, numeric values are accepted as well.
            See the parameter *loc* of `.Legend` for details.
        pad : float, default: 0.4
            Padding around the child objects, in fraction of the font size.
        borderpad : float, default: 0.5
            Border padding, in fraction of the font size.
        prop : `~matplotlib.font_manager.FontProperties`, optional
            Font property used as a reference for paddings.
        frameon : bool, default: True
            If True, draw a box around this artist.
        **kwargs
            Keyword arguments forwarded to `.AnchoredOffsetbox`.

        Attributes
        ----------
        drawing_area : `~matplotlib.offsetbox.AuxTransformBox`
            A container for artists to display.

        Examples
        --------
        To display an ellipse in the upper left, with a width of 0.1 and
        height of 0.4 in data coordinates:

        >>> box = AnchoredAuxTransformBox(ax.transData, loc='upper left')
        >>> el = Ellipse((0, 0), width=0.1, height=0.4, angle=30)
        >>> box.drawing_area.add_artist(el)
        >>> ax.add_artist(box)
        """
        self.drawing_area = AuxTransformBox(transform)
        self._prop_given = prop
        super().__init__(loc, pad=pad, borderpad=borderpad,
                         child=self.drawing_area, prop=prop, frameon=frameon,
                         **kwargs)

    def _add_native(self, axes):
        return _axes_grid1.add_aux_transform_box(
            axes._native,
            (_live_transforms.to_native(self.drawing_area.aux_transform),
             self.loc, _native_font(self._prop_given)),
            self._frame(),
            [child._native for child in self.drawing_area.get_children()])


class AnchoredSizeBar(_NativeAnchoredBox):
    def __init__(self, transform, size, label, loc,
                 pad=0.1, borderpad=0.1, sep=2,
                 frameon=True, size_vertical=0, color='black',
                 label_top=False, fontproperties=None, fill_bar=None,
                 **kwargs):
        """
        Draw a horizontal scale bar with a center-aligned label underneath.

        Parameters
        ----------
        transform : `~matplotlib.transforms.Transform`
            The transformation object for the coordinate system in use, i.e.,
            :attr:`!matplotlib.axes.Axes.transData`.
        size : float
            Horizontal length of the size bar, given in coordinates of
            *transform*.
        label : str
            Label to display.
        loc : str
            Location of the size bar.  Valid locations are
            'upper left', 'upper center', 'upper right',
            'center left', 'center', 'center right',
            'lower left', 'lower center', 'lower right'.
            For backward compatibility, numeric values are accepted as well.
            See the parameter *loc* of `.Legend` for details.
        pad : float, default: 0.1
            Padding around the label and size bar, in fraction of the font
            size.
        borderpad : float, default: 0.1
            Border padding, in fraction of the font size.
        sep : float, default: 2
            Separation between the label and the size bar, in points.
        frameon : bool, default: True
            If True, draw a box around the horizontal bar and label.
        size_vertical : float, default: 0
            Vertical length of the size bar, given in coordinates of
            *transform*.
        color : str, default: 'black'
            Color for the size bar and label.
        label_top : bool, default: False
            If True, the label will be over the size bar.
        fontproperties : `~matplotlib.font_manager.FontProperties`, optional
            Font properties for the label text.
        fill_bar : bool, optional
            If True and if *size_vertical* is nonzero, the size bar will
            be filled in with the color specified by the size bar.
            Defaults to True if *size_vertical* is greater than
            zero and False otherwise.
        **kwargs
            Keyword arguments forwarded to `.AnchoredOffsetbox`.

        Attributes
        ----------
        size_bar : `~matplotlib.offsetbox.AuxTransformBox`
            Container for the size bar.
        txt_label : `~matplotlib.offsetbox.TextArea`
            Container for the label of the size bar.

        Notes
        -----
        If *prop* is passed as a keyword argument, but *fontproperties* is
        not, then *prop* is assumed to be the intended *fontproperties*.
        Using both *prop* and *fontproperties* is not supported.

        Examples
        --------
        >>> import matplotlib.pyplot as plt
        >>> import numpy as np
        >>> from mpl_toolkits.axes_grid1.anchored_artists import (
        ...     AnchoredSizeBar)
        >>> fig, ax = plt.subplots()
        >>> ax.imshow(np.random.random((10, 10)))
        >>> bar = AnchoredSizeBar(ax.transData, 3, '3 data units', 4)
        >>> ax.add_artist(bar)
        >>> fig.show()

        Using all the optional parameters

        >>> import matplotlib.font_manager as fm
        >>> fontprops = fm.FontProperties(size=14, family='monospace')
        >>> bar = AnchoredSizeBar(ax.transData, 3, '3 units', 4, pad=0.5,
        ...                       sep=5, borderpad=0.5, frameon=False,
        ...                       size_vertical=0.5, color='white',
        ...                       fontproperties=fontprops)
        """
        if fontproperties is None and 'prop' in kwargs:
            fontproperties = kwargs.pop('prop')
        self._bar = (transform, float(size), str(label), float(sep),
                     float(size_vertical), color, bool(label_top),
                     fontproperties, fill_bar)
        super().__init__(loc, pad=pad, borderpad=borderpad, child=None,
                         prop=fontproperties, frameon=frameon, **kwargs)

    def _add_native(self, axes):
        (transform, size, label, sep, size_vertical, color, label_top,
         fontproperties, fill_bar) = self._bar
        return _axes_grid1.add_size_bar(
            axes._native,
            (_live_transforms.to_native(transform), size, label, self.loc),
            (self._frame(), sep, size_vertical),
            (color, label_top, _native_font(fontproperties),
             None if fill_bar is None else bool(fill_bar)))


class AnchoredDirectionArrows(_NativeAnchoredBox):
    def __init__(self, transform, label_x, label_y, length=0.15,
                 fontsize=0.08, loc='upper left', angle=0, aspect_ratio=1,
                 pad=0.4, borderpad=0.4, frameon=False, color='w', alpha=1,
                 sep_x=0.01, sep_y=0, fontproperties=None, back_length=0.15,
                 head_width=10, head_length=15, tail_width=2,
                 text_props=None, arrow_props=None,
                 **kwargs):
        """
        Draw two perpendicular arrows to indicate directions.

        Parameters
        ----------
        transform : `~matplotlib.transforms.Transform`
            The transformation object for the coordinate system in use, i.e.,
            :attr:`!matplotlib.axes.Axes.transAxes`.
        label_x, label_y : str
            Label text for the x and y arrows
        length : float, default: 0.15
            Length of the arrow, given in coordinates of *transform*.
        fontsize : float, default: 0.08
            Size of label strings, given in coordinates of *transform*.
        loc : str, default: 'upper left'
            Location of the arrow.  Valid locations are
            'upper left', 'upper center', 'upper right',
            'center left', 'center', 'center right',
            'lower left', 'lower center', 'lower right'.
            For backward compatibility, numeric values are accepted as well.
            See the parameter *loc* of `.Legend` for details.
        angle : float, default: 0
            The angle of the arrows in degrees.
        aspect_ratio : float, default: 1
            The ratio of the length of arrow_x and arrow_y.
            Negative numbers can be used to change the direction.
        pad : float, default: 0.4
            Padding around the labels and arrows, in fraction of the font size.
        borderpad : float, default: 0.4
            Border padding, in fraction of the font size.
        frameon : bool, default: False
            If True, draw a box around the arrows and labels.
        color : str, default: 'white'
            Color for the arrows and labels.
        alpha : float, default: 1
            Alpha values of the arrows and labels
        sep_x, sep_y : float, default: 0.01 and 0 respectively
            Separation between the arrows and labels in coordinates of
            *transform*.
        fontproperties : `~matplotlib.font_manager.FontProperties`, optional
            Font properties for the label text.
        back_length : float, default: 0.15
            Fraction of the arrow behind the arrow crossing.
        head_width : float, default: 10
            Width of arrow head, sent to `.ArrowStyle`.
        head_length : float, default: 15
            Length of arrow head, sent to `.ArrowStyle`.
        tail_width : float, default: 2
            Width of arrow tail, sent to `.ArrowStyle`.
        text_props, arrow_props : dict
            Properties of the text and arrows, passed to `.TextPath` and
            `.FancyArrowPatch`.
        **kwargs
            Keyword arguments forwarded to `.AnchoredOffsetbox`.

        Attributes
        ----------
        arrow_x, arrow_y : `~matplotlib.patches.FancyArrowPatch`
            Arrow x and y
        text_path_x, text_path_y : `~matplotlib.text.TextPath`
            Path for arrow labels
        p_x, p_y : `~matplotlib.patches.PathPatch`
            Patch for arrow labels
        box : `~matplotlib.offsetbox.AuxTransformBox`
            Container for the arrows and labels.

        Notes
        -----
        If *prop* is passed as a keyword argument, but *fontproperties* is
        not, then *prop* is assumed to be the intended *fontproperties*.
        Using both *prop* and *fontproperties* is not supported.

        Examples
        --------
        >>> import matplotlib.pyplot as plt
        >>> import numpy as np
        >>> from mpl_toolkits.axes_grid1.anchored_artists import (
        ...     AnchoredDirectionArrows)
        >>> fig, ax = plt.subplots()
        >>> ax.imshow(np.random.random((10, 10)))
        >>> arrows = AnchoredDirectionArrows(ax.transAxes, '111', '110')
        >>> ax.add_artist(arrows)
        >>> fig.show()

        Using several of the optional parameters, creating downward pointing
        arrow and high contrast text labels.

        >>> import matplotlib.font_manager as fm
        >>> fontprops = fm.FontProperties(family='monospace')
        >>> arrows = AnchoredDirectionArrows(ax.transAxes, 'East', 'South',
        ...                                  loc='lower left', color='k',
        ...                                  aspect_ratio=-1, sep_x=0.02,
        ...                                  sep_y=-0.01,
        ...                                  text_props={'ec':'w', 'fc':'k'},
        ...                                  fontproperties=fontprops)
        """
        if fontproperties is None and 'prop' in kwargs:
            fontproperties = kwargs.pop('prop')
        self._arrows = (
            transform, (str(label_x), str(label_y)),
            tuple(map(float, (length, fontsize, angle, aspect_ratio, sep_x,
                              sep_y, back_length))),
            tuple(map(float, (head_width, head_length, tail_width))),
            color, float(alpha), fontproperties,
            cbook.normalize_kwargs(text_props or {}, Patch),
            cbook.normalize_kwargs(arrow_props or {}, Patch))
        super().__init__(loc, pad=pad, borderpad=borderpad, child=None,
                         frameon=frameon, **kwargs)

    def _add_native(self, axes):
        (transform, labels, geometry, heads, color, alpha, fontproperties,
         text_props, arrow_props) = self._arrows
        return _axes_grid1.add_direction_arrows(
            axes._native,
            (_live_transforms.to_native(transform), labels, self.loc,
             self._frame()),
            (geometry, heads),
            (color, alpha, _native_font(fontproperties)),
            (text_props, arrow_props))


# What matplotlib's module also has from its imports, which programs
# import from it.
from pltrs import transforms  # noqa: E402, F401
from pltrs.offsetbox import TextArea, VPacker  # noqa: E402, F401
from pltrs.patches import ArrowStyle, FancyArrowPatch, PathPatch, Rectangle  # noqa: E402, F401
from pltrs.textpath import TextPath  # noqa: E402, F401
