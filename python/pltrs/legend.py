"""
The legend module defines the Legend class, which is responsible for
drawing legends associated with Axes and/or figures (matplotlib's
``legend``).

pltrs (Rust) lays out and draws legends (``pltrs::legend``). A `Legend`
object wraps the native legend of its axes or figure; the arguments
matplotlib resolves in Python (the positional forms of ``legend()``, the
labels, the handler maps keyed by handles or types) are resolved here, as
matplotlib 3.11.2 resolves them (under the Matplotlib License), and the
legend receives its entries and keyword arguments explicitly.
"""

import numbers
import time

import numpy as np

from . import _program_classes
from ._program_classes import takes_from_program
from . import _docstring
from . import _api, _handles, _lazy_imports, _live_transforms, _values, colors, rcParams
from . import _transform_trees, _val_or_rc
from ._native import artist as _artist, legend as _legend
from ._native import register as _register
from ._native import text as _text
from .artist import Artist
from .cbook import silent_list
from .collections import (CircleCollection, Collection, LineCollection,
                          PatchCollection, PathCollection, PolyCollection,
                          RegularPolyCollection)
from .container import BarContainer, Container, ErrorbarContainer, StemContainer
from .font_manager import FontManager, FontProperties
from .legend_handler import (HandlerCircleCollection, HandlerErrorbar,
                             HandlerLine2D, HandlerLineCollection, HandlerPatch,
                             HandlerPathCollection, HandlerPolyCollection,
                             HandlerRegularPolyCollection, HandlerStem,
                             HandlerStepPatch, HandlerTuple,
                             update_from_first_child)
from .lines import Line2D
from .offsetbox import DraggableOffsetBox
from .patches import FancyBboxPatch, Patch, Rectangle, StepPatch
from .text import Text
from .transforms import (Bbox, BboxBase, BboxTransformFrom, IdentityTransform,
                         TransformedBbox)

_legend_checks = _lazy_imports.module(globals(), "._legend_checks")
_legend_handlers = _lazy_imports.module(globals(), "._legend_handlers")
_legend_label_colors = _lazy_imports.module(globals(), "._legend_label_colors")

# pltrs (Rust) times the search for the best location with Python's
# performance counter, looked up at every reading as matplotlib's
# ``_find_best_position`` does.
_register("perf_counter", lambda: time.perf_counter())

#: The Python class of each kind of artist a legend can show
#: (``_native.legend.candidates``).
_ARTIST_CLASSES = {"Line2D": Line2D, "Collection": Collection,
                   "LineCollection": LineCollection, "Text": Text,
                   "Rectangle": Rectangle, "Artist": Artist}

#: The Python class of each kind of artist a legend draws: the
#: collections its handlers make are path collections.
_DRAWN_CLASSES = {**_ARTIST_CLASSES, "Collection": PathCollection}


class DraggableLegend(DraggableOffsetBox):
    def __init__(self, legend, use_blit=False, update="loc"):
        """
        Wrapper around a `.Legend` to support mouse dragging.

        Parameters
        ----------
        legend : `.Legend`
            The `.Legend` instance to wrap.
        use_blit : bool, optional
            Use blitting for faster image composition. For details see
            :ref:`func-animation`.
        update : {'loc', 'bbox'}, optional
            If "loc", update the *loc* parameter of the legend upon finalizing.
            If "bbox", update the *bbox_to_anchor* parameter.
        """
        self.legend = legend

        _api.check_in_list(["loc", "bbox"], update=update)
        self._update = update

        super().__init__(legend, legend._legend_box, use_blit=use_blit)

    def finalize_offset(self):
        if self._update == "loc":
            self._update_loc(self.get_loc_in_canvas())
        elif self._update == "bbox":
            self._update_bbox_to_anchor(self.get_loc_in_canvas())

    def _update_loc(self, loc_in_canvas):
        bbox = self.legend.get_bbox_to_anchor()
        # if bbox has zero width or height, the transformation is
        # ill-defined. Fall back to the default bbox_to_anchor.
        if bbox.width == 0 or bbox.height == 0:
            self.legend.set_bbox_to_anchor(None)
            bbox = self.legend.get_bbox_to_anchor()
        _bbox_transform = BboxTransformFrom(bbox)
        self.legend._loc = tuple(_bbox_transform.transform(loc_in_canvas))

    def _update_bbox_to_anchor(self, loc_in_canvas):
        loc_in_bbox = self.legend.axes.transAxes.transform(loc_in_canvas)
        self.legend.set_bbox_to_anchor(loc_in_bbox)


_legend_kw_doc_base = """
bbox_to_anchor : `.BboxBase`, 2-tuple, or 4-tuple of floats
    Box that is used to position the legend in conjunction with *loc*.
    This is an advanced option for free placement of the legend. For
    most use cases, *loc* alone is sufficient.

    Defaults to ``axes.bbox`` (if called as a method to `.Axes.legend`) or
    ``figure.bbox`` (if ``figure.legend``).

    Bbox coordinates are interpreted in the coordinate system given by
    *bbox_transform*, with the default transform
    Axes or Figure coordinates, depending on which ``legend`` is called.

    If a 4-tuple or `.BboxBase` is given, then it specifies the bbox
    ``(x, y, width, height)`` that the legend is placed in.
    To put the legend in the best location in the bottom right
    quadrant of the Axes (or figure)::

        loc='best', bbox_to_anchor=(0.5, 0., 0.5, 0.5)

    A 2-tuple ``(x, y)`` places the corner of the legend specified by *loc* at
    x, y.  For example, to put the legend's upper right-hand corner in the
    center of the Axes (or figure) the following keywords can be used::

        loc='upper right', bbox_to_anchor=(0.5, 0.5)

    For more details on legend positioning, see the
    :ref:`legend_guide`.

ncols : int, default: 1
    The number of columns that the legend has.

    For backward compatibility, the spelling *ncol* is also supported
    but it is discouraged. If both are given, *ncols* takes precedence.

prop : None or `~matplotlib.font_manager.FontProperties` or dict
    The font properties of the legend. If None (default), the current
    :data:`matplotlib.rcParams` will be used.

fontsize : int or {'xx-small', 'x-small', 'small', 'medium', 'large', \
'x-large', 'xx-large'}
    The font size of the legend. If the value is numeric the size will be the
    absolute font size in points. String values are relative to the current
    default font size. This argument is only used if *prop* is not specified.

labelcolor : str or list, default: :rc:`legend.labelcolor`
    The color of the text in the legend. Either a valid color string
    (for example, 'red'), or a list of color strings. The labelcolor can
    also be made to match the color of the line or marker using 'linecolor',
    'markerfacecolor' (or 'mfc'), or 'markeredgecolor' (or 'mec').

    Labelcolor can be set globally using :rc:`legend.labelcolor`. If None,
    use :rc:`text.color`.

numpoints : int, default: :rc:`legend.numpoints`
    The number of marker points in the legend when creating a legend
    entry for a `.Line2D` (line).

scatterpoints : int, default: :rc:`legend.scatterpoints`
    The number of marker points in the legend when creating
    a legend entry for a `.PathCollection` (scatter plot).

scatteryoffsets : iterable of floats, default: ``[0.375, 0.5, 0.3125]``
    The vertical offset (relative to the font size) for the markers
    created for a scatter plot legend entry. 0.0 is at the base the
    legend text, and 1.0 is at the top. To draw all markers at the
    same height, set to ``[0.5]``.

markerscale : float, default: :rc:`legend.markerscale`
    The relative size of legend markers compared to the originally drawn ones.

markerfirst : bool, default: True
    If *True*, legend marker is placed to the left of the legend label.
    If *False*, legend marker is placed to the right of the legend label.

reverse : bool, default: False
    If *True*, the legend labels are displayed in reverse order from the input.
    If *False*, the legend labels are displayed in the same order as the input.

    .. versionadded:: 3.7

frameon : bool, default: :rc:`legend.frameon`
    Whether the legend should be drawn on a patch (frame).

fancybox : bool, default: :rc:`legend.fancybox`
    Whether round edges should be enabled around the `.FancyBboxPatch` which
    makes up the legend's background.

shadow : None, bool or dict, default: :rc:`legend.shadow`
    Whether to draw a shadow behind the legend.
    The shadow can be configured using `.Patch` keywords.
    Customization via :rc:`legend.shadow` is currently not supported.

framealpha : float, default: :rc:`legend.framealpha`
    The alpha transparency of the legend's background.
    If *shadow* is activated and *framealpha* is ``None``, the default value is
    ignored.

facecolor : "inherit" or color, default: :rc:`legend.facecolor`
    The legend's background color.
    If ``"inherit"``, use :rc:`axes.facecolor`.

edgecolor : "inherit" or color, default: :rc:`legend.edgecolor`
    The legend's background patch edge color.
    If ``"inherit"``, use :rc:`axes.edgecolor`.

linewidth : float or None, default: :rc:`legend.linewidth`
    The legend's background patch edge linewidth.
    If ``None``, use :rc:`patch.linewidth`.

    .. versionadded:: 3.11

mode : {"expand", None}
    If *mode* is set to ``"expand"`` the legend will be horizontally
    expanded to fill the Axes area (or *bbox_to_anchor* if defines
    the legend's size).

bbox_transform : None or `~matplotlib.transforms.Transform`
    The transform for the bounding box (*bbox_to_anchor*). For a value
    of ``None`` (default) the Axes'
    :data:`!matplotlib.axes.Axes.transAxes` transform will be used.

title : str or None
    The legend's title. Default is no title (``None``).

title_fontproperties : None or `~matplotlib.font_manager.FontProperties` or dict
    The font properties of the legend's title. If None (default), the
    *title_fontsize* argument will be used if present; if *title_fontsize* is
    also None, the current :rc:`legend.title_fontsize` will be used.

title_fontsize : int or {'xx-small', 'x-small', 'small', 'medium', 'large', \
'x-large', 'xx-large'}, default: :rc:`legend.title_fontsize`
    The font size of the legend's title.
    Note: This cannot be combined with *title_fontproperties*. If you want
    to set the fontsize alongside other font properties, use the *size*
    parameter in *title_fontproperties*.

alignment : {'center', 'left', 'right'}, default: 'center'
    The alignment of the legend title and the box of entries. The entries
    are aligned as a single block, so that markers always lined up.

borderpad : float, default: :rc:`legend.borderpad`
    The fractional whitespace inside the legend border, in font-size units.

labelspacing : float, default: :rc:`legend.labelspacing`
    The vertical space between the legend entries, in font-size units.

handlelength : float, default: :rc:`legend.handlelength`
    The length of the legend handles, in font-size units.

handleheight : float, default: :rc:`legend.handleheight`
    The height of the legend handles, in font-size units.

handletextpad : float, default: :rc:`legend.handletextpad`
    The pad between the legend handle and text, in font-size units.

borderaxespad : float, default: :rc:`legend.borderaxespad`
    The pad between the Axes and legend border, in font-size units.

columnspacing : float, default: :rc:`legend.columnspacing`
    The spacing between columns, in font-size units.

handler_map : dict or None
    The custom dictionary mapping instances or types to a legend
    handler. This *handler_map* updates the default handler map
    found at `matplotlib.legend.Legend.get_legend_handler_map`.

draggable : bool, default: False
    Whether the legend can be dragged with the mouse.
"""

_loc_doc_base = """
loc : str or pair of floats, default: {default}
    The location of the legend.

    The string locations place the legend at the corresponding position
    within the bounding box, which by default is the full {parent} area.
    The bounding box can be changed via *bbox_to_anchor*.

    The positions are visualized below::

        +--------------+--------------+---------------+
        | 'upper left' |'upper center'| 'upper right' |
        +--------------+--------------+---------------+
        |'center left' |   'center'   |'center right' |
        +--------------+--------------+---------------+
        | 'lower left' |'lower center'| 'lower right' |
        +--------------+--------------+---------------+
{best}
    The location can also be a 2-tuple giving the coordinates of the lower-left
    corner of the legend in {parent} coordinates (in which case *bbox_to_anchor*
    will be ignored).

    For back-compatibility, ``'center right'`` (but no other location) can also
    be spelled ``'right'``, and each "string" location can also be given as a
    numeric value:

    ==================   =============
    Location String      Location Code
    ==================   =============
    'best' (Axes only)   0
    'upper right'        1
    'upper left'         2
    'lower left'         3
    'lower right'        4
    'right'              5
    'center left'        6
    'center right'       7
    'lower center'       8
    'upper center'       9
    'center'             10
    ==================   =============
    {outside}"""

_loc_doc_best = """
    The string ``'best'`` places the legend at the location, among the nine
    locations defined so far, with the minimum overlap with other drawn
    artists.  This currently takes into account most, but not all, artists
    added to the Axes via plotting functions. In particular it does not consider
    inset axes, titles, or axis labels.

    The computation of the best position can be expensive for plots with large
    amounts of data. If speed becomes a concern, you may may benefit from
    providing a specific location.
"""

_legend_kw_axes_st = (
    _loc_doc_base.format(parent='axes', default=':rc:`legend.loc`',
                         best=_loc_doc_best, outside='') +
    _legend_kw_doc_base)

_docstring.interpd.register(_legend_kw_axes=_legend_kw_axes_st)

_outside_doc = """
    If a figure is using the constrained layout manager, the string codes
    of the *loc* keyword argument can get better layout behaviour using the
    prefix 'outside'. There is ambiguity at the corners, so 'outside
    upper right' will make space for the legend above the rest of the
    axes in the layout, and 'outside right upper' will make space on the
    right side of the layout.  In addition to the values of *loc*
    listed above, we have 'outside right upper', 'outside right lower',
    'outside left upper', and 'outside left lower'.  See
    :ref:`legend_guide` for more details.
"""

_legend_kw_figure_st = (
    _loc_doc_base.format(parent='figure', default="'upper right'",
                         best='', outside=_outside_doc) +
    _legend_kw_doc_base)

_docstring.interpd.register(_legend_kw_figure=_legend_kw_figure_st)

_legend_kw_both_st = (
    _loc_doc_base.format(parent='axes/figure',
                         default=":rc:`legend.loc` for Axes, 'upper right' for Figure",
                         best=_loc_doc_best, outside=_outside_doc) +
    _legend_kw_doc_base)

_docstring.interpd.register(_legend_kw_doc=_legend_kw_both_st)

_legend_kw_set_loc_st = (
    _loc_doc_base.format(parent='axes/figure',
                         default=":rc:`legend.loc` for Axes, 'upper right' for Figure",
                         best=_loc_doc_best, outside=_outside_doc))

_docstring.interpd.register(_legend_kw_set_loc_doc=_legend_kw_set_loc_st)


class Legend(Artist):
    """
    Place a legend on the figure/axes.
    """

    #: The `.DraggableLegend` dragging the legend, while it is draggable.
    _draggable = None

    def get_window_extent(self, renderer=None):
        # Return the legend's box in display space, where pltrs (Rust) lays it
        # out in the figure as it is (matplotlib computes it from the legend's
        # layout, the view limits not autoscaled first).
        self._check_figure_renderer(renderer)
        self._measure_box_with(renderer)
        _transform_trees.measured(self)
        return self._extent_as_is(renderer)

    @staticmethod
    def _measure_box_with(renderer):
        """Fail as matplotlib's legend box fails to be measured with
        *renderer* when it is no renderer: its packers ask it the size of a
        point first."""
        if renderer is not None:
            renderer.points_to_pixels(1.)

    # 'best' is only implemented for Axes legends
    codes = {'best': 0, **{
        'upper right': 1,
        'upper left': 2,
        'lower left': 3,
        'lower right': 4,
        'right': 5,
        'center left': 6,
        'center right': 7,
        'lower center': 8,
        'upper center': 9,
        'center': 10,
    }}

    _default_handler_map = {
        StemContainer: HandlerStem(),
        ErrorbarContainer: HandlerErrorbar(),
        Line2D: HandlerLine2D(),
        Patch: HandlerPatch(),
        StepPatch: HandlerStepPatch(),
        LineCollection: HandlerLineCollection(),
        RegularPolyCollection: HandlerRegularPolyCollection(),
        CircleCollection: HandlerCircleCollection(),
        BarContainer: HandlerPatch(update_func=update_from_first_child),
        tuple: HandlerTuple(),
        PatchCollection: HandlerPolyCollection(),
        PathCollection: HandlerPathCollection(),
        PolyCollection: HandlerPolyCollection(),
    }

    def __str__(self):
        return "Legend"

    @_docstring.interpd
    def __init__(
        self, parent, handles, labels,
        *,
        loc=None,
        numpoints=None,      # number of points in the legend line
        markerscale=None,    # relative size of legend markers vs. original
        markerfirst=True,    # left/right ordering of legend marker and label
        reverse=False,       # reverse ordering of legend marker and label
        scatterpoints=None,  # number of scatter points
        scatteryoffsets=None,
        prop=None,           # properties for the legend texts
        fontsize=None,       # keyword to set font size directly
        labelcolor=None,     # keyword to set the text color
        # spacing & pad defined as a fraction of the font-size
        borderpad=None,      # whitespace inside the legend border
        labelspacing=None,   # vertical space between the legend entries
        handlelength=None,   # length of the legend handles
        handleheight=None,   # height of the legend handles
        handletextpad=None,  # pad between the legend handle and text
        borderaxespad=None,  # pad between the Axes and legend border
        columnspacing=None,  # spacing between columns
        ncols=1,     # number of columns
        mode=None,  # horizontal distribution of columns: None or "expand"
        fancybox=None,  # True: fancy box, False: rounded box, None: rcParam
        shadow=None,
        title=None,           # legend title
        title_fontsize=None,  # legend title font size
        framealpha=None,      # set frame alpha
        edgecolor=None,       # frame patch edgecolor
        facecolor=None,       # frame patch facecolor
        linewidth=None,       # frame patch linewidth
        bbox_to_anchor=None,  # bbox to which the legend will be anchored
        bbox_transform=None,  # transform for the bbox
        frameon=None,         # draw frame
        handler_map=None,
        title_fontproperties=None,  # properties for the legend title
        alignment="center",       # control the alignment within the legend box
        ncol=1,  # synonym for ncols (backward compatibility)
        draggable=False  # whether the legend can be dragged with the mouse
    ):
        """
        Parameters
        ----------
        parent : `~matplotlib.axes.Axes` or `.Figure`
            The artist that contains the legend.

        handles : list of (`.Artist` or tuple of `.Artist`)
            A list of Artists (lines, patches) to be added to the legend.

        labels : list of str
            A list of labels to show next to the artists. The length of handles
            and labels should be the same. If they are not, they are truncated
            to the length of the shorter list.

        Other Parameters
        ----------------
        %(_legend_kw_doc)s

        Attributes
        ----------
        legend_handles
            List of `.Artist` objects added as legend entries.

            .. versionadded:: 3.7
        """
        from .axes._base import _AxesBase
        from .figure import FigureBase
        # matplotlib's constructor computes with its arguments in this order
        # (see `._legend_checks`).
        if prop is None:
            self.prop = FontProperties(size=_val_or_rc(fontsize, "legend.fontsize"))
        else:
            self.prop = FontProperties._from_any(prop)
            if isinstance(prop, dict) and "size" not in prop:
                self.prop.set_size(rcParams["legend.fontsize"])
        self._fontsize = self.prop.get_size_in_points()
        self._custom_handler_map = handler_map
        self.numpoints = _val_or_rc(numpoints, 'legend.numpoints')
        self.markerscale = _val_or_rc(markerscale, 'legend.markerscale')
        self.scatterpoints = _val_or_rc(scatterpoints, 'legend.scatterpoints')
        # The paddings legend handlers read, as fractions of the font size.
        self.borderpad = _val_or_rc(borderpad, 'legend.borderpad')
        self.labelspacing = _val_or_rc(labelspacing, 'legend.labelspacing')
        self.handlelength = _val_or_rc(handlelength, 'legend.handlelength')
        self.handleheight = _val_or_rc(handleheight, 'legend.handleheight')
        self.handletextpad = _val_or_rc(handletextpad, 'legend.handletextpad')
        self.borderaxespad = _val_or_rc(borderaxespad, 'legend.borderaxespad')
        self.columnspacing = _val_or_rc(columnspacing, 'legend.columnspacing')
        if framealpha is None and not shadow:
            # matplotlib's frame is opaque by default for the *shadow* given,
            # not for :rc:`legend.shadow`.
            framealpha = rcParams["legend.framealpha"]
        shadow = _val_or_rc(shadow, 'legend.shadow')
        handles = list(handles)
        if len(handles) < 2:
            ncols = 1
        ncols = ncols if ncols != 1 else ncol
        if self.numpoints <= 0:
            raise ValueError("numpoints must be > 0; it was %d" % numpoints)
        if scatteryoffsets is None:
            self._scatteryoffsets = np.array([3. / 8., 4. / 8., 2.5 / 8.])
        else:
            self._scatteryoffsets = np.asarray(scatteryoffsets)
        reps = self.scatterpoints // len(self._scatteryoffsets) + 1
        self._scatteryoffsets = np.tile(self._scatteryoffsets,
                                        reps)[:self.scatterpoints]
        if isinstance(parent, _AxesBase):
            self.isaxes = True
        elif isinstance(parent, FigureBase):
            self.isaxes = False
        else:
            raise TypeError(
                "Legend needs either Axes or FigureBase as parent"
            )
        self.parent = parent
        self._mode = mode
        _legend_checks.check_anchor(bbox_to_anchor, parent)
        if not isinstance(shadow, dict):
            if shadow not in (0, 1, True, False):
                raise ValueError(
                    'Legend shadow must be a dict or bool, not '
                    f'{shadow!r} of type {type(shadow)}.')
            shadow = bool(shadow)
        width = _legend_checks.check_frame(facecolor, edgecolor, linewidth, framealpha)
        if linewidth is not None:
            linewidth = width  # As the frame keeps it.
        _api.check_in_list(["center", "left", "right"], alignment=alignment)
        self.shadow = True if isinstance(shadow, dict) else shadow
        # What the handlers running in Python read before the legend is made.
        self._figure = None
        self._axes = parent if self.isaxes else None
        handler_map = self.get_legend_handler_map()
        _legend_checks.check_entries(self, handles, handler_map, ncols)
        entries, kept, kept_handles = [], [], []
        made = _legend_handlers.MadeEntries()
        for handle, label in zip(handles, labels):
            handler = self.get_legend_handler(handler_map, handle)
            if handler is None:
                _api.warn_external(
                    "Legend does not support handles for "
                    f"{type(handle).__name__} "
                    "instances.\nA proxy artist may be used "
                    "instead.\nSee: https://matplotlib.org/"
                    "stable/users/explain/axes/legend_guide.html"
                    "#controlling-the-legend-entries")
                continue
            if _legend_handlers.runs_in_python(self, handler_map, handler,
                                               handle):
                entries.append(made.entry(self, len(kept), handler, handle))
            else:
                entries.append(self._entry(handle, handler_map))
            kept.append(str(label))
            kept_handles.append(handle)
        given_loc = loc
        loc = self._validated_loc(loc)
        _legend_checks.check_title(title_fontsize, title_fontproperties)
        _legend_checks.check_labelcolor(labelcolor)
        if (labelcolor is not None and not isinstance(labelcolor, str)
                and colors.is_color_like(labelcolor)):
            # One colour, as the cycle of one that gives it to every label.
            labelcolor = [colors.to_rgba(labelcolor)]
        # pltrs (Rust) makes the legend with the numbers of a tuple; a box
        # (`.BboxBase`) anchors it once it is made, as `set_bbox_to_anchor`
        # does, which matplotlib's constructor calls for either.
        anchor_box = bbox_to_anchor if isinstance(bbox_to_anchor, BboxBase) else None
        anchor = None if anchor_box is not None else bbox_to_anchor
        kwargs = dict(
            # Entries of fewer than two points show one marker.
            loc=loc, numpoints=max(1, int(self.numpoints)),
            markerscale=self.markerscale,
            markerfirst=markerfirst, reverse=reverse,
            scatterpoints=self.scatterpoints, scatteryoffsets=scatteryoffsets,
            prop=None if prop is None else FontManager._props(self.prop),
            fontsize=fontsize, labelcolor=labelcolor, borderpad=borderpad,
            labelspacing=labelspacing, handlelength=handlelength,
            handleheight=handleheight, handletextpad=handletextpad,
            borderaxespad=_legend_checks.held_pad(borderaxespad),
            columnspacing=columnspacing,
            ncols=1 if len(kept) < 2 else ncols,
            mode=_legend_checks.native_mode(mode),
            fancybox=fancybox, shadow=shadow,
            title=title, title_fontsize=title_fontsize,
            framealpha=framealpha, edgecolor=edgecolor, facecolor=facecolor,
            linewidth=linewidth, bbox_to_anchor=anchor,
            bbox_transform=(None if anchor is None
                            else _live_transforms.given_to_native(
                                bbox_transform, holder=_live_transforms.root_figure(parent))),
            frameon=frameon, alignment=alignment)
        self._ncols = ncols
        self._draggable = None
        # matplotlib's constructor makes the legend for the Axes without
        # making it their legend (`Axes.legend` does, ``legend_``), which
        # pltrs (Rust) does as it makes it: the Axes keep the legend they
        # had. A legend made for a figure is drawn once it is put in the
        # figure's ``legends`` (`.Figure.legend` puts it there).
        own_axes = self.isaxes and parent.name != "3d"
        previous = _legend.axes_legend(parent._native) if own_axes else None
        native = _legend.new(parent._native, entries, kept, kwargs)
        _handles.adopt(self, native, parent.figure)
        if own_axes and previous is not None:
            _legend.set_axes_legend(parent._native, previous)
        elif own_axes or not self.isaxes:
            _artist.remove(native)
        made.adopt(self)
        if type(self) is not Legend:
            # A class of the program's drawing or measuring legends itself.
            _python_drawing.attach(self)
        overridden = self._program_setters
        if overridden:
            _program_classes.replay_setters(self, [
                ("set_figure", (parent.get_figure(root=False) if self.isaxes else parent,)),
                ("set_bbox_to_anchor", (bbox_to_anchor, bbox_transform))])
        if anchor_box is not None and "set_bbox_to_anchor" not in overridden:
            self.set_bbox_to_anchor(anchor_box, bbox_transform)
        _legend_checks.keep_borderaxespad(self, borderaxespad)
        if takes_from_program(type(self), "set_loc"):
            # matplotlib's constructor places the legend through it.
            self.set_loc(given_loc)
        self._made_handles = made.returned
        self._entry_handles = kept_handles
        _legend_label_colors.remember(self, labelcolor)
        if draggable and "set_draggable" not in overridden:
            self.set_draggable(True)
        if title_fontproperties is not None:
            title_prop_fp = FontProperties._from_any(title_fontproperties)
            if (isinstance(title_fontproperties, dict)
                    and "size" not in title_fontproperties):
                title_prop_fp.set_size(rcParams["legend.title_fontsize"])
            self.get_title().set_fontproperties(title_prop_fp)
        if overridden:
            # matplotlib's constructor titles the legend, then makes it
            # draggable or not, through these.
            _program_classes.replay_setters(self, [
                ("set_title", (title, self.get_title().get_fontproperties())),
                ("set_draggable", (draggable,))])

    def _add_to_axes(self, axes, kind, autolim=True):
        """Keep the legend of *axes* as an ordinary artist of it
        (``ax.add_artist(legend)``), so that the next ``legend()`` adds a
        legend beside it; any other artist goes as `.Artist` adds it."""
        if self.parent is not axes:
            return super()._add_to_axes(axes, kind, autolim)
        _legend.keep(axes._native, self._native)
        # Back in the Axes if it was removed from it.
        return _handles.adopt(self, self._native, axes.figure)

    def _remove_again(self):
        # The removal method of an Axes' legend forgets the Axes' legend,
        # again without fail; a figure's takes it out of its legends.
        if not self.isaxes:
            super()._remove_again()

    def _validated_loc(self, loc):
        """*loc* as pltrs (Rust) takes it (a name, a code or a point), or
        None for :rc:`legend.loc`, raising matplotlib's errors."""
        loc0 = loc
        if loc is None:
            if self.isaxes:
                return None
            loc = rcParams["legend.loc"]
            if loc in [0, 'best']:
                return 'upper right'
        type_err_message = ("loc must be string, coordinate tuple, or"
                            f" an integer 0-10, not {loc!r}")
        if isinstance(loc, str):
            outside = loc.split()[0] == 'outside'
            if outside and self.isaxes:
                raise ValueError(
                    f"'outside' option for loc='{loc0}' keyword argument only "
                    "works for figure legends")
            name = loc.split('outside ')[1] if outside else loc
            if not outside:
                _api.getitem_checked(self.codes, loc=name)
        elif np.iterable(loc):
            loc = tuple(loc)
            if len(loc) != 2 or not all(isinstance(e, numbers.Real) for e in loc):
                raise ValueError(type_err_message)
            return tuple(map(float, loc))
        elif isinstance(loc, int):
            if loc < 0 or loc > 10:
                raise ValueError(type_err_message)
        else:
            raise ValueError(type_err_message)
        if not self.isaxes and loc in (0, 'best'):
            raise ValueError(
                "Automatic legend placement (loc='best') not implemented for "
                "figure legend")
        return loc

    def _entry(self, handle, handler_map):
        """The entry pltrs (Rust) draws for *handle*: its native handle,
        the native handle the axes found it as, or a tuple entry."""
        native = getattr(handle, "_legend_entry", None)
        if native is not None:
            return native
        if isinstance(handle, Container):
            if getattr(handle, "_native", None) is not None:  # Of an axes.
                return _legend.container_entry(handle._native)
            first = next(iter(handle.get_children()), None)
            return self._entry(first, handler_map)
        if isinstance(handle, tuple):
            handler = self.get_legend_handler(handler_map, handle)
            ndivide, pad = ((handler._ndivide, handler._pad)
                            if isinstance(handler, HandlerTuple) else (1, None))
            return ("tuple", [self._entry(h, handler_map) for h in handle],
                    ndivide, pad)
        return handle._native

    @classmethod
    def get_default_handler_map(cls):
        """Return the global default handler map, shared by all legends."""
        return cls._default_handler_map

    @classmethod
    def set_default_handler_map(cls, handler_map):
        """Set the global default handler map, shared by all legends."""
        cls._default_handler_map = handler_map

    @classmethod
    def update_default_handler_map(cls, handler_map):
        """Update the global default handler map, shared by all legends."""
        cls._default_handler_map.update(handler_map)

    def get_legend_handler_map(self):
        """Return this legend instance's handler map."""
        default_handler_map = self.get_default_handler_map()
        return ({**default_handler_map, **self._custom_handler_map}
                if self._custom_handler_map else default_handler_map)

    @staticmethod
    def get_legend_handler(legend_handler_map, orig_handle):
        """
        Return a legend handler from *legend_handler_map* that
        corresponds to *orig_handler*.

        *legend_handler_map* should be a dictionary object (that is
        returned by the get_legend_handler_map method).

        It first checks if the *orig_handle* itself is a key in the
        *legend_handler_map* and return the associated value.
        Otherwise, it checks for each of the classes in its
        method-resolution-order. If no matching key is found, it
        returns ``None``.
        """
        try:
            return legend_handler_map[orig_handle]
        except (TypeError, KeyError):  # TypeError if unhashable.
            pass
        for handle_type in type(orig_handle).mro():
            try:
                return legend_handler_map[handle_type]
            except KeyError:
                pass
        return None

    def get_texts(self):
        r"""Return the list of `~.text.Text`\s in the legend."""
        return silent_list('Text', self.texts)

    # matplotlib's legend lists its labels and handles when made; after, only
    # `get_texts`, `get_lines` and `get_patches` read the lists, which a
    # program may replace.
    @_values.assignable
    def texts(self):
        """The `~.text.Text` labels of the legend."""
        return _handles.wrap_all(_legend.get_texts(self._native), Text, self.figure)

    def get_title(self):
        """Return the `.Text` instance for the legend title."""
        return _handles.wrap(_legend.get_title(self._native), Text, self.figure)

    def set_title(self, title, prop=None):
        """
        Set legend title and title style.

        Parameters
        ----------
        title : str
            The legend title.

        prop : `.font_manager.FontProperties` or `str` or `pathlib.Path`
            The font properties of the legend title.
            If a `str`, it is interpreted as a fontconfig pattern parsed by
            `.FontProperties`.  If a `pathlib.Path`, it is interpreted as the
            absolute path to a font file.

        """
        # As `Text.set_text` takes it: None is no text.
        _legend.set_title(self._native, "" if title is None else str(title))
        if prop is not None:
            self.get_title().set_fontproperties(FontProperties._from_any(prop))

    @_values.assignable
    def legend_handles(self):
        """
        The artists drawn as the handles of the entries, in their order:
        the first one each entry's handler draws, None for the entries no
        handler draws.
        """
        made = self.__dict__.get("_made_handles", {})
        return [made[index] if index in made else
                None if handle is None else
                self._drawn_handle(index, handle)
                for index, handle in enumerate(_legend.legend_handles(self._native))]

    def _drawn_handle(self, index, handle):
        """The artist of the native *handle* drawn for the entry *index*,
        reading back what the entry's handle was given, as matplotlib's
        handlers copy it (``update_from``)."""
        drawn = _handles.wrap(handle[0], _DRAWN_CLASSES[handle[1]], self.figure)
        entry_handles = self.__dict__.get("_entry_handles", [])
        if index < len(entry_handles):
            _values.inherit(drawn, entry_handles[index])
            _copy_unheld(drawn, entry_handles[index])
        return drawn

    def get_lines(self):
        r"""Return the list of `~.lines.Line2D`\s in the legend."""
        return [h for h in self.legend_handles if isinstance(h, Line2D)]

    def get_patches(self):
        r"""Return the list of `~.patches.Patch`\s in the legend."""
        return silent_list('Patch',
                           [h for h in self.legend_handles if isinstance(h, Patch)])

    @property
    def _legend_box(self):
        """The `.VPacker` of the title and the columns of entries, as the
        legend lays them out (made from pltrs (Rust)'s box when first
        asked for), or the box a program gave in its place."""
        given = self.__dict__.get("_given_legend_box")
        return self._laid_out_legend_box() if given is None else given

    @_legend_box.setter
    def _legend_box(self, box):
        # matplotlib's draw and measures go through the box: a packer of
        # the legend's own title and entries (legendkit's title beside or
        # below them) is what pltrs (Rust) lays out from then on.
        from .offsetbox import HPacker, VPacker
        title, entries = self._box_parts()
        children = box.get_children() if isinstance(box, (VPacker, HPacker)) else []
        if len(children) == 2 and {id(child) for child in children} == {id(title), id(entries)}:
            _legend.repack(self._native, (isinstance(box, VPacker), children[0] is title),
                           (float(box.pad or 0.), float(box.sep or 0.)), box.align)
        box._laid_out_for = self
        self.__dict__["_given_legend_box"] = box
        self.stale = True

    def _laid_out_legend_box(self):
        """The legend's box as pltrs (Rust) laid it out (see `_legend_box`)."""
        box = self.__dict__.get("_legend_box_object")
        if box is None:
            from .offsetbox import _laid_out_box
            box = _laid_out_box(_legend.legend_box(self._native), self.figure,
                                _DRAWN_CLASSES)
            # The box is on the legend's figure and Axes, placed by the
            # legend's location (``_findoffset``) unless pinned somewhere.
            pinned = _legend.box_offset(self._native)
            box._offset = (self._findoffset if pinned is None
                           else _values.float64s(pinned))
            box._laid_out_for = self
            self._legend_box_object = box
        if not self.stale:
            # Drawn with the legend, as matplotlib's draw draws its box.
            _fresh(box)
        return box

    def _made_boxes(self):
        """The legend's boxes made so far: the laid-out one, and the one a
        program gave in its place."""
        return [box for box in (self.__dict__.get("_legend_box_object"),
                                self.__dict__.get("_given_legend_box")) if box is not None]

    def _box_parts(self):
        """The boxes of the title and of the columns of entries, as the
        legend first laid them out (kept as matplotlib keeps its
        ``_legend_title_box`` and ``_legend_handle_box``, whatever packs
        them since)."""
        parts = self.__dict__.get("_legend_box_parts")
        if parts is None:
            parts = self._legend_box_parts = tuple(self._laid_out_legend_box().get_children()[:2])
        return parts

    # matplotlib's boxes of the title and of the columns of entries, which
    # programs restyle and pack anew (legendkit), and the alignment given
    # (``_alignment``).
    _legend_title_box = property(lambda self: self._box_parts()[0])
    _legend_handle_box = property(lambda self: self._box_parts()[1])

    @_values.assignable
    def _alignment(self):
        # matplotlib's field, kept as a program assigns it (legendkit's title
        # layout assigns its packer's: ``'top'`` for a title on the left).
        return self.get_alignment()

    def _laid_out_box_moved(self, xy):
        """Pin the legend's box at *xy*, display coordinates the program
        moved it to (``_legend_box.set_offset``, as `.DraggableLegend` does
        while the legend is dragged), or place it by its location again
        for a callable *xy* (the legend's ``_findoffset``)."""
        _legend.set_box_offset(self._native,
                               None if callable(xy) else tuple(map(float, xy)))

    def _laid_out_box_changed(self, box):
        """Align the title and the entries as the program aligned the
        legend's box (``legend._legend_box.align = ...``), which
        matplotlib's draw reads: a `.VPacker` aligns its children
        horizontally, on the left for ``left``, ``bottom`` and
        ``baseline``, on the right for ``right`` and ``top``."""
        alignment = {"left": "left", "bottom": "left", "baseline": "left",
                     "center": "center", "right": "right", "top": "right"}.get(box.align)
        if alignment is not None:
            _legend.set_alignment(self._native, alignment)

    def get_children(self):
        # docstring inherited
        return [self._legend_box, self.get_frame()]

    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    def get_tightbbox(self, renderer=None):
        # docstring inherited
        # matplotlib's is the extent of the legend's box, which keeps its
        # figure (and renderer) once the legend is removed: where the legend
        # was then.
        self._measure_box_with(renderer)
        kept = self.__dict__.get("_kept_extent")
        if kept is not None and self.__dict__.get("_decoupled"):
            return Bbox(kept.get_points())
        _transform_trees.measured(self)
        return self._extent_as_is()

    def _keep_drawn_state(self):
        # docstring inherited: the extent of the legend's box.
        self._kept_extent = self._extent_as_is()

    def contains(self, mouseevent):
        # (pltrs (Rust) hit-tests the legend's frame.)
        return super().contains(mouseevent)

    def get_frame(self):
        """Return the `~.patches.Rectangle` used to frame the legend."""
        return _handles.wrap(_legend.get_frame(self._native), FancyBboxPatch,
                             self.figure)

    def _set_frame(self, patch):
        # matplotlib's draw sizes the patch assigned to the legend's box and
        # draws it (and `get_frame`, `contains` read it): the frame pltrs
        # (Rust) draws takes its look, and the patch becomes the frame.
        frame = self.get_frame()
        frame.set_boxstyle(patch.get_boxstyle())
        frame.set_mutation_aspect(patch.get_mutation_aspect())
        for prop in ("facecolor", "edgecolor", "linewidth", "linestyle", "hatch", "fill",
                     "alpha", "visible", "capstyle", "joinstyle", "path_effects",
                     # (matplotlib's own frame snaps; a patch made alone does not.)
                     "snap", "antialiased"):
            getattr(frame, f"set_{prop}")(getattr(patch, f"get_{prop}")())
        _handles.adopt(patch, frame._native, self.figure)

    legendPatch = property(get_frame, _set_frame, doc="The `.FancyBboxPatch` framing the legend.")

    def get_frame_on(self):
        """Get whether the legend box patch is drawn."""
        return self.legendPatch.get_visible()

    def set_frame_on(self, b):
        """
        Set whether the legend box patch is drawn.

        Parameters
        ----------
        b : bool
        """
        # matplotlib's sets the visibility of the frame, which keeps the
        # object it is given (``1``, ``'False'``) when it changes.
        patch = self.legendPatch
        if b != patch.get_visible():
            _legend.set_frame_on(self._native, bool(b))
            _values.remember(patch, "visible", b)
        self.stale = True

    draw_frame = set_frame_on  # Backcompat alias.

    @_docstring.interpd
    def set_loc(self, loc=None):
        """
        Set the location of the legend.

        .. versionadded:: 3.8

        Parameters
        ----------
        %(_legend_kw_set_loc_doc)s
        """
        _legend.set_loc(self._native, self._validated_loc(loc))

    def _set_loc(self, loc):
        """Place the legend at *loc*, a location code or a point in its
        anchor box, as the program's choice; the legend box is placed by
        its location again (``_legend_box.set_offset(self._findoffset)``)."""
        _legend.set_loc(self._native, loc if isinstance(loc, (int, str))
                        else tuple(map(float, loc)))
        _values.remember(self, "loc", loc)
        for box in self._made_boxes():
            box._offset = self._findoffset
        self.stale = True

    def _get_loc(self):
        """The location code, or the point the legend is placed at."""
        code, point = _legend.loc(self._native)
        return _values.given(self, "loc", point if code is None else code)

    _loc = property(_get_loc, _set_loc)

    def _findoffset(self, width, height, xdescent, ydescent, renderer):
        """Helper function to locate the legend."""
        # pltrs (Rust) locates the legend box, the size of which it lays
        # out as matplotlib's (``width``, ``height`` and the descents).
        offset = _legend.located_box_offset(self._native)
        if offset is None:
            raise ValueError("the legend is not in its figure: it has no location")
        # matplotlib computes the offset with numpy.
        return _values.float64s(offset)

    def get_alignment(self):
        """Get the alignment value of the legend box"""
        return _legend.get_alignment(self._native)

    def set_alignment(self, alignment):
        """
        Set the alignment of the legend title and the box of entries.

        The entries are aligned as a single block, so that markers always
        lined up.

        Parameters
        ----------
        alignment : {'center', 'left', 'right'}.

        """
        _api.check_in_list(["center", "left", "right"], alignment=alignment)
        _legend.set_alignment(self._native, alignment)
        for box in self._made_boxes():
            box.align = alignment

    def set_draggable(self, state, use_blit=False, update='loc'):
        """
        Enable or disable mouse dragging support of the legend.

        Parameters
        ----------
        state : bool
            Whether mouse dragging is enabled.
        use_blit : bool, optional
            Use blitting for faster image composition. For details see
            :ref:`func-animation`.
        update : {'loc', 'bbox'}, optional
            The legend parameter to be changed when dragged:

            - 'loc': update the *loc* parameter of the legend
            - 'bbox': update the *bbox_to_anchor* parameter of the legend

        Returns
        -------
        `.DraggableLegend` or *None*
            If *state* is ``True`` this returns the `.DraggableLegend` helper
            instance. Otherwise this returns *None*.
        """
        if state:
            if self._draggable is None:
                self._draggable = DraggableLegend(self,
                                                  use_blit,
                                                  update=update)
        else:
            if self._draggable is not None:
                self._draggable.disconnect()
            self._draggable = None
        return self._draggable

    def get_draggable(self):
        """Return ``True`` if the legend is draggable, ``False`` otherwise."""
        return self._draggable is not None

    def set_ncols(self, ncols):
        """Set the number of columns."""
        self._ncols = ncols

    @property
    def axes(self):
        """The `~.axes.Axes` the legend resides in, or *None*: the Axes
        whose clear forgot the legend too, as matplotlib's clear only drops
        its ``legend_``, and none once removed."""
        if "_native" in self.__dict__ and self._figure is None:
            return None
        axes = Artist.axes.fget(self)
        return self.__dict__.get("_cleared_from") if axes is None else axes

    @axes.setter
    def axes(self, new_axes):
        Artist.axes.fset(self, new_axes)

    def _get_figure(self, root):
        # docstring inherited
        if "_native" not in self.__dict__:
            # Being made, by the handlers of its entries: on its parent's.
            figure = self.parent.get_figure(root=False) if self.isaxes else self.parent
            return figure.get_figure(root=True) if root else figure
        return super()._get_figure(root)

    def get_transform(self):
        # docstring inherited
        if "_native" not in self.__dict__:
            # Being made, by the handlers of its entries: no transform yet.
            return IdentityTransform()
        return super().get_transform()

    def _set_artist_props(self, a):
        """Set the boilerplate props for artists added to Axes."""
        a.set_figure(self.get_figure(root=False))
        if self.isaxes:
            a.axes = self.axes
        a.set_transform(self.get_transform())

    def _fractions(self):
        """The transform of the parent's fractions, onto its box as it is
        now (matplotlib's ``BboxTransformTo(parent.bbox)``: a root figure's
        own box, which a tight save does not move)."""
        return BboxTransformTo(self.parent.bbox)

    def get_bbox_to_anchor(self):
        """Return the bbox that the legend will be anchored to."""
        anchor = _legend.get_bbox_to_anchor(self._native)
        if anchor is None:
            return self.parent.bbox
        bounds, native = anchor
        given = self.__dict__.get("_anchored")
        if given is not None and given[1] == bounds:
            return given[0]
        bbox = TransformedBbox(Bbox.from_bounds(*bounds),
                               _live_transforms.from_native(self.figure, native))
        self._anchored = (bbox, bounds)
        return bbox

    @property
    def _bbox_to_anchor(self):
        # matplotlib's attribute: the box the legend is anchored to, None
        # for its parent's (which exporters read).
        if _legend.get_bbox_to_anchor(self._native) is None:
            return None
        return self.get_bbox_to_anchor()

    def set_bbox_to_anchor(self, bbox, transform=None):
        """
        Set the bbox that the legend will be anchored to.

        Parameters
        ----------
        bbox : `~matplotlib.transforms.BboxBase` or tuple
            The bounding box can be specified in the following ways:

            - A `.BboxBase` instance
            - A tuple of ``(left, bottom, width, height)`` in the given
              transform (normalized axes coordinate if None)
            - A tuple of ``(left, bottom)`` where the width and height will be
              assumed to be zero.
            - *None*, to remove the bbox anchoring, and use the parent bbox.

        transform : `~matplotlib.transforms.Transform`, optional
            A transform to apply to the bounding box. If not specified, this
            will use a transform to the bounding box of the parent.
        """
        if bbox is None:
            self.__dict__.pop("_anchored", None)
            _legend.set_bbox_to_anchor(self._native, None, None)
            return
        elif not isinstance(bbox, BboxBase):
            try:
                l = len(bbox)
            except TypeError as err:
                raise ValueError(f"Invalid bbox: {bbox}") from err

            if l == 2:
                bbox = [bbox[0], bbox[1], 0, 0]

            bbox = Bbox.from_bounds(*bbox)

        if transform is None:
            transform = self._fractions()

        try:
            anchored = TransformedBbox(bbox, transform)
        except (TypeError, ValueError):
            # matplotlib's legend holds the box itself by then, in display
            # coordinates.
            self._anchor_to(bbox, bbox, IdentityTransform())
            raise
        self._anchor_to(anchored, bbox, transform)
        self.stale = True

    def _anchor_to(self, anchored, bbox, transform):
        """Anchor the legend to *bbox* in the coordinates of *transform*,
        *anchored* being the box `get_bbox_to_anchor` returns."""
        spanning = getattr(bbox, "_spanning", None)
        if spanning is not None:
            bbox, transform = Bbox.unit(), spanning + transform
        elif isinstance(bbox, TransformedBbox):
            bbox, transform = bbox._bbox, bbox._transform + transform
        bounds = tuple(map(float, bbox.bounds))
        _legend.set_bbox_to_anchor(self._native, bounds,
                                   _live_transforms.to_native(transform))
        self._anchored = (anchored, bounds)


def _fresh(box):
    """Mark the laid-out *box* and the boxes in it as drawn."""
    from .offsetbox import OffsetBox
    _artist.set_stale(box._native, False)
    for child in box.get_children():
        if isinstance(child, OffsetBox):
            _fresh(child)


#: What matplotlib's ``update_from`` copies that pltrs (Rust) does not
#: hold: the path effects as given, and an alpha per item.
_UNHELD = ("_path_effects", "_array_alpha")


def _copy_unheld(drawn, entry):
    """Give the handle *drawn* what matplotlib's handler copies from the
    entry's handle *entry* with ``update_from`` (`_UNHELD`): from the
    artist itself when the handle is of its class, from the first patch
    of a bar container."""
    from .container import BarContainer
    source = (entry.patches[0] if isinstance(entry, BarContainer) and entry.patches
              else entry)
    if type(source) is not type(drawn):
        return
    for name in _UNHELD:
        if name in getattr(source, "__dict__", {}):
            drawn.__dict__[name] = source.__dict__[name]


def _get_legend_handles(axs, legend_handler_map=None):
    """Yield artists that can be used as handles in a legend."""
    handler_map = {**Legend.get_default_handler_map(),
                   **(legend_handler_map or {})}
    for ax in axs:
        for native, artist, label in _legend.candidates(ax._native):
            if artist is None:
                yield _container(ax, native, label)
                continue
            raw, kind = artist
            yield from _handled(
                _handles.wrap(raw, _ARTIST_CLASSES[kind], ax.figure), label,
                handler_map)
        # Texts are candidates too, which no default handler draws.
        for raw in _text.axes_texts(ax._native):
            text = _handles.wrap(raw, Text, ax.figure)
            yield from _handled(text, text.get_label(), handler_map)


def _container(ax, native, label):
    """The container of *ax* the legend handle *native* shows (one standing
    for it, labelled *label*, when the axes has no such container object)."""
    for container in ax.containers:
        if (getattr(container, "_native", None) is not None
                and _legend.shows_container(native, container._native)):
            return container
    container = Container([], label=label)
    container._legend_entry = native
    return container


def _handled(handle, label, handler_map):
    """Yield *handle*, labelled *label*, if *handler_map* has a handler for
    it; else warn that a shown label cannot have an entry."""
    if Legend.get_legend_handler(handler_map, handle) is not None:
        yield handle
    elif label and not label.startswith('_'):
        _api.warn_external(
            "Legend does not support handles for "
            f"{type(handle).__name__} "
            "instances.\nSee: https://matplotlib.org/stable/"
            "tutorials/intermediate/legend_guide.html"
            "#implementing-a-custom-legend-handler")


def _get_legend_handles_labels(axs, legend_handler_map=None):
    """Return handles and labels for legend."""
    handles = []
    labels = []
    for handle in _get_legend_handles(axs, legend_handler_map):
        label = handle.get_label()
        if label and not label.startswith('_'):
            handles.append(handle)
            labels.append(label)
    return handles, labels


def _parse_legend_args(axs, *args, handles=None, labels=None, **kwargs):
    """
    Get the handles and labels from the calls to either ``figure.legend``
    or ``axes.legend``.

    The parser is a bit involved because we support::

        legend()
        legend(labels)
        legend(handles, labels)
        legend(labels=labels)
        legend(handles=handles)
        legend(handles=handles, labels=labels)

    The behavior for a mixture of positional and keyword handles and labels
    is undefined and raises an error.

    Returns
    -------
    handles : list of (`.Artist` or tuple of `.Artist`)
        The legend handles.
    labels : list of str
        The legend labels.
    kwargs : dict
        *kwargs* with keywords handles and labels removed.
    """
    handlers = kwargs.get('handler_map')

    if (handles is not None or labels is not None) and args:
        raise TypeError("When passing handles and labels, they must both be "
                        "passed positionally or both as keywords.")

    if (hasattr(handles, "__len__") and
            hasattr(labels, "__len__") and
            len(handles) != len(labels)):
        _api.warn_external(f"Mismatched number of handles and labels: "
                           f"len(handles) = {len(handles)} "
                           f"len(labels) = {len(labels)}")
    # if got both handles and labels as kwargs, make same length
    if handles is not None and labels is not None:
        handles, labels = zip(*zip(handles, labels))

    elif handles is not None and labels is None:
        labels = [handle.get_label() for handle in handles]

    elif labels is not None and handles is None:
        # Get as many handles as there are labels.
        handles = [handle for handle, label
                   in zip(_get_legend_handles(axs, handlers), labels)]

    elif len(args) == 0:  # 0 args: automatically detect labels and handles.
        handles, labels = _get_legend_handles_labels(axs, handlers)
        if not handles:
            _api.warn_external(
                "No artists with labels found to put in legend.  Note that "
                "artists whose label start with an underscore are ignored "
                "when legend() is called with no argument.")

    elif len(args) == 1:  # 1 arg: user defined labels, automatic handle detection.
        labels, = args
        if any(isinstance(l, Artist) for l in labels):
            raise TypeError("A single argument passed to legend() must be a "
                            "list of labels, but found an Artist in there.")

        # Get as many handles as there are labels.
        handles = [handle for handle, label
                   in zip(_get_legend_handles(axs, handlers), labels)]

    elif len(args) == 2:  # 2 args: user defined handles and labels.
        handles, labels = args[:2]
        if (hasattr(handles, "__len__") and hasattr(labels, "__len__")
                and len(handles) != len(labels)):
            _api.warn_external(f"Mismatched number of handles and labels: "
                               f"len(handles) = {len(handles)} "
                               f"len(labels) = {len(labels)}")

    else:
        raise _api.nargs_error('legend', '0-2', len(args))

    return handles, labels, kwargs


# What matplotlib's module also has from its imports, which programs
# import from it.
import itertools  # noqa: E402, F401
import logging  # noqa: E402, F401
from . import _python_drawing, cbook, legend_handler, offsetbox  # noqa: E402, F401
from .artist import allow_rasterization  # noqa: E402, F401
from .offsetbox import (  # noqa: E402, F401
    AnchoredOffsetbox, DrawingArea, HPacker, TextArea, VPacker)
from .patches import Shadow  # noqa: E402, F401
from .transforms import BboxTransformTo  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
