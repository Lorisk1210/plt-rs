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

import numpy as np

from . import _api, _handles, _live_transforms, colors, rcParams
from ._native import legend as _legend
from ._native import text as _text
from .artist import Artist
from .collections import (CircleCollection, Collection, LineCollection,
                          PatchCollection, PathCollection, PolyCollection,
                          RegularPolyCollection)
from .container import BarContainer, Container, ErrorbarContainer, StemContainer
from .font_manager import FontProperties
from .legend_handler import (HandlerCircleCollection, HandlerErrorbar,
                             HandlerLine2D, HandlerLineCollection, HandlerPatch,
                             HandlerPathCollection, HandlerPolyCollection,
                             HandlerRegularPolyCollection, HandlerStem,
                             HandlerStepPatch, HandlerTuple,
                             update_from_first_child)
from .lines import Line2D
from .patches import FancyBboxPatch, Patch, Rectangle, StepPatch
from .text import Text

#: The Python class of each kind of artist a legend can show
#: (``_native.legend.candidates``).
_ARTIST_CLASSES = {"Line2D": Line2D, "Collection": Collection, "Text": Text,
                   "Rectangle": Rectangle, "Artist": Artist}

#: The Python class of each kind of artist a legend draws: the
#: collections its handlers make are path collections.
_DRAWN_CLASSES = {**_ARTIST_CLASSES, "Collection": PathCollection}


class Legend(Artist):
    """
    Place a legend on the figure/axes.
    """

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
        """
        from .axes._base import _AxesBase
        from .figure import FigureBase
        if prop is not None:
            raise NotImplementedError("pltrs: Legend(prop=...)")
        if shadow is None:
            shadow = rcParams["legend.shadow"]
        if not isinstance(shadow, dict):
            if shadow not in (0, 1, True, False):
                raise ValueError(
                    'Legend shadow must be a dict or bool, not '
                    f'{shadow!r} of type {type(shadow)}.')
            shadow = bool(shadow)
        if title_fontsize is not None and title_fontproperties is not None:
            raise ValueError(
                "title_fontsize and title_fontproperties can't be specified "
                "at the same time. Only use one of them. ")
        if isinstance(parent, _AxesBase):
            self.isaxes = True
        elif isinstance(parent, FigureBase):
            self.isaxes = False
        else:
            raise TypeError(
                "Legend needs either Axes or FigureBase as parent"
            )
        self.parent = parent
        self._custom_handler_map = handler_map
        self.numpoints = rcParams['legend.numpoints'] if numpoints is None else numpoints
        self.markerscale = (rcParams['legend.markerscale'] if markerscale is None
                            else markerscale)
        self.scatterpoints = (rcParams['legend.scatterpoints']
                              if scatterpoints is None else scatterpoints)
        handler_map = self.get_legend_handler_map()
        entries, kept = [], []
        for handle, label in zip(handles, labels):
            if self.get_legend_handler(handler_map, handle) is None:
                _api.warn_external(
                    "Legend does not support handles for "
                    f"{type(handle).__name__} "
                    "instances.\nA proxy artist may be used "
                    "instead.\nSee: https://matplotlib.org/"
                    "stable/users/explain/axes/legend_guide.html"
                    "#controlling-the-legend-entries")
                continue
            entries.append(self._entry(handle, handler_map))
            kept.append(str(label))
        loc = self._validated_loc(loc)
        ncols = ncols if ncols != 1 else ncol
        if (labelcolor is not None and not isinstance(labelcolor, str)
                and colors.is_color_like(labelcolor)):
            # One colour, as the cycle of one that gives it to every label.
            labelcolor = [colors.to_rgba(labelcolor)]
        kwargs = dict(
            # Entries of fewer than two points show one marker.
            loc=loc, numpoints=max(1, int(self.numpoints)),
            markerscale=self.markerscale,
            markerfirst=markerfirst, reverse=reverse,
            scatterpoints=self.scatterpoints, scatteryoffsets=scatteryoffsets,
            fontsize=fontsize, labelcolor=labelcolor, borderpad=borderpad,
            labelspacing=labelspacing, handlelength=handlelength,
            handleheight=handleheight, handletextpad=handletextpad,
            borderaxespad=borderaxespad, columnspacing=columnspacing,
            ncols=1 if len(kept) < 2 else ncols, mode=mode,
            fancybox=fancybox, shadow=shadow,
            title=title, title_fontsize=title_fontsize,
            framealpha=framealpha, edgecolor=edgecolor, facecolor=facecolor,
            linewidth=linewidth, bbox_to_anchor=bbox_to_anchor,
            bbox_transform=_live_transforms.given_to_native(bbox_transform),
            frameon=frameon, alignment=alignment, draggable=draggable)
        _api.check_in_list(["center", "left", "right"], alignment=alignment)
        native = _legend.new(parent._native, entries, kept, kwargs)
        _handles.adopt(self, native, parent.figure)
        if title_fontproperties is not None:
            title_prop_fp = FontProperties._from_any(title_fontproperties)
            if (isinstance(title_fontproperties, dict)
                    and "size" not in title_fontproperties):
                title_prop_fp.set_size(rcParams["legend.title_fontsize"])
            self.get_title().set_fontproperties(title_prop_fp)

    def _add_to_axes(self, axes, kind, autolim=True):
        """Keep the legend of *axes* as an ordinary artist of it
        (``ax.add_artist(legend)``), so that the next ``legend()`` adds a
        legend beside it; any other artist goes as `.Artist` adds it."""
        if self.parent is not axes:
            return super()._add_to_axes(axes, kind, autolim)
        _legend.keep(axes._native, self._native)
        return self

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
        return _handles.wrap_all(_legend.get_texts(self._native), Text, self.figure)

    texts = property(get_texts, doc="The `~.text.Text` labels of the legend.")

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
        """
        _legend.set_title(self._native, str(title))
        if prop is not None:
            self.get_title().set_fontproperties(FontProperties._from_any(prop))

    @property
    def legend_handles(self):
        """
        The artists drawn as the handles of the entries, in their order:
        the first one each entry's handler draws, None for the entries no
        handler draws.
        """
        return [None if handle is None else
                _handles.wrap(handle[0], _DRAWN_CLASSES[handle[1]], self.figure)
                for handle in _legend.legend_handles(self._native)]

    def get_lines(self):
        r"""Return the list of `~.lines.Line2D`\s in the legend."""
        return [h for h in self.legend_handles if isinstance(h, Line2D)]

    def get_patches(self):
        r"""Return the list of `~.patches.Patch`\s in the legend."""
        return [h for h in self.legend_handles if isinstance(h, Patch)]

    @property
    def _legend_box(self):
        """The `.VPacker` of the title and the columns of entries, as the
        legend lays them out (made from pltrs (Rust)'s box when first
        asked for)."""
        box = self.__dict__.get("_legend_box_object")
        if box is None:
            from .offsetbox import _laid_out_box
            box = _laid_out_box(_legend.legend_box(self._native), self.figure,
                                _DRAWN_CLASSES)
            self._legend_box_object = box
        return box

    def get_children(self):
        # docstring inherited
        return [self._legend_box, self.get_frame()]

    def get_frame(self):
        """Return the `~.patches.Rectangle` used to frame the legend."""
        return _handles.wrap(_legend.get_frame(self._native), FancyBboxPatch,
                             self.figure)

    legendPatch = property(get_frame, doc="The `.FancyBboxPatch` framing the legend.")

    def get_frame_on(self):
        """Get whether the legend box patch is drawn."""
        return _legend.get_frame_on(self._native)

    def set_frame_on(self, b):
        """
        Set whether the legend box patch is drawn.

        Parameters
        ----------
        b : bool
        """
        _legend.set_frame_on(self._native, bool(b))

    draw_frame = set_frame_on  # Backcompat alias.

    def set_loc(self, loc=None):
        """
        Set the location of the legend.

        Parameters
        ----------
        loc : str or pair of floats, default: :rc:`legend.loc`
            The location of the legend.
        """
        loc = self._validated_loc(loc)
        _legend.set_loc(self._native,
                        rcParams["legend.loc"] if loc is None else loc)

    def _get_loc(self):
        """The location code, or the point the legend is placed at."""
        code, point = _legend.loc(self._native)
        return point if code is None else code

    _loc = property(_get_loc)

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
        if "_legend_box_object" in self.__dict__:
            self._legend_box_object.align = alignment

    def set_draggable(self, state, use_blit=False, update='loc'):
        """
        Enable or disable mouse dragging support of the legend.

        Parameters
        ----------
        state : bool
            Whether mouse dragging is enabled.
        use_blit : bool, optional
            Use blitting for faster image composition.
        update : {'loc', 'bbox'}, optional
            The legend parameter to be changed when dragged.
        """
        _api.check_in_list(["loc", "bbox"], update=update)
        _legend.set_draggable(self._native, bool(state))

    def get_draggable(self):
        """Return ``True`` if the legend is draggable, ``False`` otherwise."""
        return _legend.get_draggable(self._native)


def _get_legend_handles(axs, legend_handler_map=None):
    """Yield artists that can be used as handles in a legend."""
    handler_map = {**Legend.get_default_handler_map(),
                   **(legend_handler_map or {})}
    for ax in axs:
        for native, artist, label in _legend.candidates(ax._native):
            if artist is None:
                container = Container([], label=label)
                container._legend_entry = native
                yield container
                continue
            raw, kind = artist
            yield from _handled(
                _handles.wrap(raw, _ARTIST_CLASSES[kind], ax.figure), label,
                handler_map)
        # Texts are candidates too, which no default handler draws.
        for raw in _text.axes_texts(ax._native):
            text = _handles.wrap(raw, Text, ax.figure)
            yield from _handled(text, text.get_label(), handler_map)


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
