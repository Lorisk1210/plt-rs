"""
Tables of text cells drawn in or next to an Axes (matplotlib's ``table``).

Use the factory function `~matplotlib.table.table` to create a ready-made
table from texts. If you need more control, use the `.Table` class and its
methods to add cells one by one.

pltrs (Rust) lays the table out and draws it (``_native.table``). Its cells
are part of the table's native artist, not artists of their own: a `Cell`
is the cell at its ``(row, column)`` key in its table. A cell made by its
constructor lives in a table of its own, outside any figure, until it is
put in a table (``table[row, col] = cell``), which copies it there. As
matplotlib's, a `Cell` is a `.Rectangle`: a rectangle outside any figure
with the cell's look, from which each setter restyles the cell. A table
holding a cell whose class (or the cell itself) has a ``draw`` of the
program's draws each cell by its ``draw``, as matplotlib's does. The
signatures and checks are matplotlib's (3.11.2, under the Matplotlib
License).
"""

from numbers import Real

import numpy as np

from . import _docstring
from . import (_api, _artist_clipping, _handles, _live_transforms, _mirrors,
               _python_drawing, _unusable, _values, cbook, colors as mcolors)
from ._drawn_with import DrawnWith
from ._program_classes import takes_from_program
from ._native import artist as _artist, table as _table
from .artist import Artist, allow_rasterization
from .patches import Patch, Rectangle
from .path import Path
from .text import Text
from .transforms import BboxBase, Bbox


class _CellHolder:
    """The native table holding a cell made outside any table, and the
    figure given to the cell (`Cell.set_figure`)."""

    #: A cell in no table keeps the corner it was made with.
    _placements = 0

    def __init__(self):
        self._native = _table.new(None, "bottom", None)
        self.figure = None


class _CellText(_mirrors.Mirroring):
    """The text of *cell*, which a `.Text` stands for (`Cell.get_text`):
    its string, colour, font, alignments, rotation and opacity are the
    cell's."""

    def __init__(self, cell):
        self.cell = cell

    def push(self, artist):
        _table.restyle_text(self.cell._ref(), artist._native)


def _points(size):
    """A font size in points: a number, or a size name (``'large'``) as a
    text's font properties take it."""
    if isinstance(size, str):
        from .font_manager import FontProperties
        return FontProperties(size=size).get_size_in_points()
    return float(size)


def _measured(value):
    """*value*, measured in the figure of a cell's table; None, for a cell
    in no table's figure, fails as matplotlib's measuring of a text in no
    figure does."""
    if value is None:
        raise AttributeError("'NoneType' object has no attribute 'dpi'")
    return value


class _CellLook(_mirrors.Mirroring):
    """The look of a cell, which the cell as a `.Rectangle` stands for (see
    `Cell`)."""

    def push(self, artist):
        _table.restyle_cell(artist._ref(), artist._native)


@_mirrors.pushing
class Cell(DrawnWith, Rectangle):
    """
    A cell is a `.Rectangle` with some associated `.Text`.

    As a user, you'll most likely not creates cells yourself. Instead, you
    should use either the `~matplotlib.table.table` factory function or
    `.Table.add_cell`.
    """
    # pltrs: a cell is the cell at its key in a table's native artist (see the
    # module documentation). As a `.Rectangle` it is a rectangle outside any
    # figure with the cell's look, from which every setter restyles the cell;
    # its table sizes and places it.

    PAD = 0.1
    """Padding between text and rectangle."""

    _default_colors = {"edgecolor": "k", "facecolor": "w"}

    _edges = 'BRTL'
    _edge_aliases = {'open':         '',
                     'closed':       _edges,  # default
                     'horizontal':   'BT',
                     'vertical':     'RL'
                     }

    def __init__(self, xy, width, height, *,
                 edgecolor='k', facecolor='w',
                 fill=True,
                 text='',
                 loc='right',
                 fontproperties=None,
                 visible_edges='closed',
                 ):
        """
        Parameters
        ----------
        xy : 2-tuple
            The position of the bottom left corner of the cell.
        width : float
            The cell width.
        height : float
            The cell height.
        edgecolor : :mpltype:`color`, default: 'k'
            The color of the cell border.
        facecolor : :mpltype:`color`, default: 'w'
            The cell facecolor.
        fill : bool, default: True
            Whether the cell background is filled.
        text : str, optional
            The cell text.
        loc : {'right', 'center', 'left'}
            The alignment of the text within the cell.
        fontproperties : dict, optional
            A dict defining the font properties of the text. Supported keys and
            values are the keyword arguments accepted by `.FontProperties`.
        visible_edges : {'closed', 'open', 'horizontal', 'vertical'} or \
substring of 'BRTL'
            The cell edges to be drawn with a line: a substring of 'BRTL'
            (bottom, right, top, left), or one of 'open' (no edges drawn),
            'closed' (all edges drawn), 'horizontal' (bottom and top),
            'vertical' (right and left).
        """
        # matplotlib's `Rectangle` adds the size to the corner first.
        xy[0] + width, xy[1] + height  # noqa: B018
        self._table = _CellHolder()
        self._key = (0, 0)
        _table.add_cell(self._table._native, 0, 0, float(width), float(height))
        self._mirror = _CellLook()
        # Set up text properties
        super().__init__(xy, width=width, height=height, fill=fill,
                         edgecolor=edgecolor, facecolor=facecolor)
        self.set_clip_on(False)
        self.visible_edges = visible_edges
        self._loc = loc
        # The text's alignment, checked as matplotlib's `Text` checks it.
        _api.check_in_list(['center', 'right', 'left'], align=loc)
        _table.set_text_halign(self._ref(), loc)
        _table.set_text(self._ref(), '' if text is None else str(text))
        if fontproperties is not None:
            from .font_manager import FontManager, FontProperties
            _table.set_font(self._ref(), FontManager._props(
                FontProperties._from_any(fontproperties)))

    @classmethod
    def _of(cls, table, key):
        """The cell at *key* of the `.Table` *table*."""
        cell = cls.__new__(cls)
        cell._table = table
        cell._key = key
        cell._figure = None
        cell._native = _table.cell_rectangle(cell._ref())
        # Where matplotlib's tables make their cells (`add_cell`).
        cell._given_xy = ((0, 0), 0)
        return _mirrors.attach(cell, _CellLook())

    def _ref(self):
        """The cell as ``_native.table`` takes it."""
        return (self._table._native, *self._key)

    def _drawer(self):
        # Its table draws the cell (see `._drawn_with`).
        table = self.__dict__.get("_table")
        return table if isinstance(table, Table) else None

    def __setattr__(self, name, value):
        super().__setattr__(name, value)
        if name == "draw" and isinstance(self.__dict__.get("_table"), Table):
            # Its table draws each cell by its draw from now on.
            _python_drawing.paint(self._table)

    @property
    def _parent_figure(self):
        """The `.Figure` or `.SubFigure` of the cell's table (the one given
        to `set_figure` for a cell in no table)."""
        if isinstance(self._table, _CellHolder):
            return self._table.figure
        return self._table.get_figure(root=False)

    def get_figure(self, root=False):
        """Return the `.Figure` or `.SubFigure` the cell's table is on."""
        figure = self._parent_figure
        if figure is None or not root:
            return figure
        return figure.get_figure(root=True)

    def set_transform(self, t):
        # Set the artist transform: the cell, its place in its table being in
        # the coordinates of *t* (pltrs: its text is drawn there too).
        self._cell_transform = t
        _table.set_cell_transform(self._ref(), _live_transforms.to_native(t))

    def is_transform_set(self):
        """Return whether the cell was given its transform: a table gives
        each cell of it one (`set_transform`)."""
        return ("_cell_transform" in self.__dict__
                or not isinstance(self._table, _CellHolder))

    def get_data_transform(self):
        """Return the transform the cell is placed in: the one set, else
        the one its table gave it when it was put there, the fractions of
        the table's Axes unless the table was given another transform
        (`Table.set_transform` leaves its cells theirs)."""
        given = self.__dict__.get("_cell_transform")
        if given is not None:
            return given
        if isinstance(self._table, _CellHolder):
            return super().get_data_transform()
        axes = self._table.axes
        return self._table.get_transform() if axes is None else axes.transAxes

    def get_transform(self):
        """Return the `~.transforms.Transform` applied to the cell."""
        return self.get_patch_transform() + self.get_data_transform()

    def set_figure(self, fig):
        # pltrs: a cell in a table belongs to its table's figure; one made by
        # its constructor keeps *fig* until a table takes it.
        if self._parent_figure is fig:
            return
        if self._parent_figure is not None:
            raise RuntimeError("Can not put single artist in "
                               "more than one figure")
        if isinstance(self._table, _CellHolder):
            self._table.figure = fig
        else:
            self._table.set_figure(fig)

    def get_text(self):
        """Return the cell `.Text` instance."""
        # pltrs: the text has the cell's string, colour, font, alignments,
        # rotation and opacity, and changing them changes the cell; the table
        # places the text when it is drawn.
        native = _table.text_artist(self._ref())
        text = self.__dict__.get("_text_mirror")
        if text is None:
            text = self._text_mirror = _mirrors.attach(
                Text._from_native(native, None), _CellText(self))
        else:
            text._native = native
        return text

    def set_fontsize(self, size):
        """Set the text fontsize."""
        _table.set_cell_fontsize(self._ref(), _points(size))
        # A size given stays until the table places the cell again.
        self._fontsize_given_at = self._table._placements
        self.stale = True

    def get_fontsize(self):
        """Return the cell fontsize."""
        table = self._table
        given_at = max(self.__dict__.get("_fontsize_given_at", -1),
                       table.__dict__.get("_fontsize_given_at", -1))
        if table._placements > max(given_at, 0):
            placed = _table.cell_placed_fontsize(self._ref())
            if placed is not None:
                return placed
        return _table.get_cell_fontsize(self._ref())

    def auto_set_font_size(self, renderer):
        """Shrink font size until the text fits into the cell width."""
        # pltrs: measured in the figure's current layout at its resolution.
        if isinstance(self._table, _CellHolder):
            return _measured(None)
        return _table.fit_font_size(self._ref())

    @allow_rasterization
    def draw(self, renderer):
        if not self.get_visible():
            return
        draw_cell = getattr(renderer, "_draw_cell", None)
        if draw_cell is not None:
            draw_cell(self)
        else:
            _python_drawing.draw_cell_through(renderer, self)
        self.stale = False

    def get_text_bounds(self, renderer):
        """
        Return the text bounds as *(x, y, width, height)* in table coordinates.
        """
        return tuple(map(np.float64, _measured(_table.text_bounds(self._ref()))))

    def get_required_width(self, renderer):
        """Return the minimal required width for the cell."""
        l, b, w, h = self.get_text_bounds(renderer)
        return w * (1.0 + (2.0 * self.PAD))

    @_docstring.interpd
    def set_text_props(self, **kwargs):
        """
        Update the text properties.

        Valid keyword arguments are:

        %(Text:kwdoc)s
        """
        self.get_text()._internal_update(kwargs)
        self.stale = True

    @property
    def visible_edges(self):
        """
        The cell edges to be drawn with a line.

        Reading this property returns a substring of 'BRTL' (bottom, right,
        top, left').

        When setting this property, you can use a substring of 'BRTL' or one
        of {'open', 'closed', 'horizontal', 'vertical'}.
        """
        return _table.get_visible_edges(self._ref())

    @visible_edges.setter
    def visible_edges(self, value):
        _table.set_visible_edges(self._ref(), _edges_of(value))
        self.stale = True

    def get_path(self):
        """Return a `.Path` for the `.visible_edges`."""
        vertices, codes = _table.get_path(self._ref())
        return Path(vertices, codes, readonly=True)

    def get_xy(self):
        """Return the left and bottom coords of the cell as a tuple: the
        corner it was made with or given since its table last placed it,
        else where its table placed it when drawn or measured (pltrs: in
        the figure's current layout)."""
        given, placements = self.__dict__.get("_given_xy", (None, 0))
        if given is not None and placements == self._table._placements:
            return given
        corner = _table.cell_corner(self._ref())
        if corner is None or not self._table._placements:
            return super().get_xy()
        # The numpy floats matplotlib's table computes the corners as.
        return tuple(map(np.float64, corner))

    xy = property(get_xy, Rectangle.set_xy)

    def _set_fields(self, x=_values.NONE_GIVEN, y=_values.NONE_GIVEN, *args, **kwargs):
        # A corner given stays until the table places the cell again.
        absent = _values.NONE_GIVEN
        if x is not absent or y is not absent:
            corner = self.get_xy()
            self._given_xy = ((corner[0] if x is absent else x,
                               corner[1] if y is absent else y),
                              self._table._placements)
        super()._set_fields(x, y, *args, **kwargs)
        if (kwargs.get("angle", absent) is not absent
                or kwargs.get("rotation_point", absent) is not absent or len(args) > 2):
            # The ``angle`` and ``rotation_point`` properties rotate the cell
            # as `set_angle` does.
            self._mirror.push(self)

    def get_bbox(self):
        """Return the `.Bbox` of the cell: at its corner (`get_xy`), as
        large as it is."""
        x, y = self.get_xy()
        return Bbox.from_extents(x, y, x + self.get_width(), y + self.get_height())

    def get_x(self):
        """Return the left coordinate of the cell."""
        return self.get_xy()[0]

    def get_y(self):
        """Return the bottom coordinate of the cell."""
        return self.get_xy()[1]

    def get_width(self):
        """Return the width of the cell."""
        return _values.given(self, "width", _table.get_size(self._ref())[0])

    def get_height(self):
        """Return the height of the cell: as given, else the numpy float of
        the text height matplotlib's tables size their rows with."""
        native = _table.get_size(self._ref())[1]
        return _values.given(self, "height", native, np.float64(native))

    def set_width(self, w):
        """Set the width of the cell."""
        super().set_width(w)
        _table.set_width(self._ref(), float(w))

    def set_height(self, h):
        """Set the height of the cell."""
        super().set_height(h)
        _table.set_height(self._ref(), float(h))

    def set_bounds(self, *args):
        # The bounds of the cell as *left*, *bottom*, *width*, *height*: its
        # table places it, so its size counts.
        super().set_bounds(*args)
        _table.set_width(self._ref(), float(super().get_width()))
        _table.set_height(self._ref(), float(super().get_height()))


# The methods matplotlib's cell inherits.
cbook._inherited_methods(Cell, Artist, ["get_figure", "is_transform_set"])
cbook._inherited_methods(Cell, Patch, ["get_data_transform", "get_transform"])
cbook._inherited_methods(Cell, Rectangle, [
    "get_xy", "get_bbox", "get_x", "get_y", "get_width", "get_height",
    "set_width", "set_height", "set_bounds"])

CustomCell = Cell  # Backcompat. alias.


def _edges_of(value):
    """The visible edges *value* gives a cell, checked as matplotlib's
    `Cell.visible_edges` checks them: a name of the cell's edge aliases, a
    sequence of its edges, or None for all of them; as pltrs (Rust) takes
    them (a string)."""
    if value is None:
        return 'closed'
    if value in Cell._edge_aliases:
        return value
    if any(edge not in Cell._edges for edge in value):
        raise ValueError('Invalid edge param {}, must only be one of '
                         '{} or string of {}'.format(
                             value,
                             ", ".join(Cell._edge_aliases),
                             ", ".join(Cell._edges)))
    return ''.join(value)


def _loc_name(loc):
    """The name of the location *loc*, a name or a code of `Table.codes`
    (pltrs: without one, a table goes below the Axes, at ``'bottom'``)."""
    if loc is None:
        return 'bottom'
    if isinstance(loc, str):
        if loc not in Table.codes:
            raise ValueError(
                "Unrecognized location {!r}. Valid locations are\n\t{}"
                .format(loc, '\n\t'.join(Table.codes)))
        return loc
    return next(name for name, code in Table.codes.items() if code == loc)


def _integer(index):
    """Whether *index*, a row or column, is an integer, at which pltrs
    (Rust) places a cell (a float with an integer value keys the same cell
    in matplotlib's dict of cells)."""
    return (isinstance(index, (Real, np.bool_)) and np.isfinite(index)
            and index == int(index))


def _extents(bbox):
    """The extents of *bbox*, a `.Bbox` or ``[xmin, ymin, width, height]``;
    None for none, or for one pltrs (Rust) cannot place a table in, which
    the table keeps (`Table._keep_bbox`)."""
    if bbox is None:
        return None
    try:
        if not isinstance(bbox, BboxBase):
            bbox = Bbox.from_bounds(*bbox)
        return tuple(float(v) for v in bbox.extents)
    except (TypeError, ValueError):
        return None


class Table(Artist):
    """
    A table of cells.

    .. note::

        ``table()`` has some fundamental design limitations and will not be
        developed further. If you need more functionality, consider
        `blume <https://github.com/swfiua/blume>`__.


    The table consists of a grid of cells, which are indexed by (row, column).

    For a simple table, you'll have a full grid of cells with indices from
    (0, 0) to (num_rows-1, num_cols-1), in which the cell (0, 0) is positioned
    at the top left. However, you can also add cells with negative indices.
    You don't have to add a cell to every grid position, so you can create
    tables that have holes.

    *Note*: You'll usually not create an empty table from scratch. Instead use
    `~matplotlib.table.table` to create a table from data.
    """

    def get_window_extent(self, renderer=None):
        # Return the table's box in display space, where pltrs (Rust) lays it
        # out in the figure as it is (matplotlib computes it from the table's
        # layout, placing its cells).
        self._check_figure_renderer(renderer)
        _unusable.check(self, measuring=True)
        self._place_cells()
        return self._extent_as_is()

    def _kept_order(self):
        """The order in which matplotlib's table computes with what it
        keeps when it places its cells (``_update_positions``)."""
        return ["autoFontsize", "cells", "bbox"]

    def _keep_bbox(self, bbox):
        """Keep *bbox*, the box given to place the table in, if pltrs (Rust)
        cannot place the table in it (`_extents`): matplotlib keeps it
        unchecked, and computes with it when it places the cells."""
        if bbox is not None and _extents(bbox) is None:
            _unusable.keep(self, "bbox", bbox, self._place_in)

    @staticmethod
    def _place_in(bbox):
        """Compute with *bbox* as matplotlib's table does when it places its
        cells in it (``_update_positions``: the cells scaled to its size and
        moved to its corner, from the box of the grid, of numpy floats), for
        its errors."""
        rl, rb, rw, rh = bbox.bounds if isinstance(bbox, BboxBase) else bbox
        unit = np.float64(1.0)
        rw / unit, rh / unit, rl - unit, rb - unit  # noqa: B018

    def _align_cells(self, kept):
        """Align the cells as matplotlib's ``_do_cell_alignment`` does
        before it places them, with the positions *kept* by the table
        alone, for its errors: their rows and columns sorted, which fails
        for positions that do not compare (pltrs (Rust) places the others)."""
        widths = {}
        heights = {}
        for (row, col), cell in self.get_celld().items():
            height = heights.setdefault(row, 0.0)
            heights[row] = max(height, cell.get_height())
            width = widths.setdefault(col, 0.0)
            widths[col] = max(width, cell.get_width())
        sorted(widths)
        sorted(heights, reverse=True)

    def _place_cells(self):
        """Count a placing of the cells outside the figure's draws."""
        self._measures = self.__dict__.get("_measures", 0) + 1

    @property
    def _placements(self):
        """How many times the table placed its cells, which matplotlib's
        table does when it is drawn or measured (``_update_positions``):
        until then they keep the corner they were made with, and a corner
        given to a cell stays until the next time."""
        drawn = 0 if self._figure is None else _artist.draw_count(self._native)
        return self.__dict__.get("_measures", 0) + drawn

    codes = {'best': 0,
             'upper right':  1,  # default
             'upper left':   2,
             'lower left':   3,
             'lower right':  4,
             'center left':  5,
             'center right': 6,
             'lower center': 7,
             'upper center': 8,
             'center':       9,
             'top right':    10,
             'top left':     11,
             'bottom left':  12,
             'bottom right': 13,
             'right':        14,
             'left':         15,
             'top':          16,
             'bottom':       17,
             }
    """Possible values where to place the table relative to the Axes."""

    FONTSIZE = 10

    AXESPAD = 0.02
    """The border between the Axes and the table edge in Axes units."""

    def __init__(self, ax, loc=None, bbox=None, **kwargs):
        """
        Parameters
        ----------
        ax : `~matplotlib.axes.Axes`
            The `~.axes.Axes` to plot the table into.
        loc : str, optional
            The position of the cell with respect to *ax*. This must be one of
            the `~.Table.codes`.
        bbox : `.Bbox` or [xmin, ymin, width, height], optional
            A bounding box to draw the table into. If this is not *None*, this
            overrides *loc*.

        Other Parameters
        ----------------
        **kwargs
            `.Artist` properties.
        """
        loc = _loc_name(loc)
        # matplotlib's table takes the figure of its Axes first.
        ax.get_figure(root=False)
        self._native = _table.new(ax._native, loc, _extents(bbox))
        super().__init__()
        self._axes = ax
        self._edges = None
        self._cells = {}
        self._kept_cells = {}
        self._keep_bbox(bbox)
        self._internal_update(kwargs)

    @classmethod
    def _from_native(cls, native, figure):
        table = super()._from_native(native, figure)
        table._edges = None
        table._cells = {}
        table._kept_cells = {}
        return table

    def add_cell(self, row, col, *args, **kwargs):
        """
        Create a cell and add it to the table.

        Parameters
        ----------
        row : int
            Row index.
        col : int
            Column index.
        *args, **kwargs
            All other parameters are passed on to `Cell`.

        Returns
        -------
        `.Cell`
            The created cell.

        """
        xy = (0, 0)
        cell = Cell(xy, visible_edges=self.edges, *args, **kwargs)
        self[row, col] = cell
        return cell

    def __setitem__(self, position, cell):
        """
        Set a custom cell in a given position.
        """
        _api.check_isinstance(Cell, cell=cell)
        try:
            row, col = position[0], position[1]
        except Exception as err:
            raise KeyError('Only tuples length 2 are accepted as '
                           'coordinates') from err
        cell.set_figure(self.get_figure(root=False))
        if not (_integer(row) and _integer(col)):
            # matplotlib keys its cells by any position, and fails when it
            # places them by positions that do not compare. pltrs (Rust)
            # places cells at integer positions: this one stays with the
            # table alone (drawn nowhere), which fails as matplotlib's.
            cell.set_transform(self.get_transform())
            cell.set_clip_on(False)
            self._cells[row, col] = self._kept_cells[row, col] = cell
            _unusable.keep(self, "cells", self._kept_cells, self._align_cells)
            self.stale = True
            return
        _table.put_cell(self._native, int(row), int(col), cell._ref())
        cell._table = self
        cell._key = (row, col)
        # The cell takes the table's transform, as matplotlib's
        # ``cell.set_transform(self.get_transform())``.
        cell.__dict__.pop("_cell_transform", None)
        if "_given_transform" in self.__dict__:
            cell._cell_transform = self._given_transform
        self._cells[row, col] = cell
        if takes_from_program(type(cell), "draw") or "draw" in vars(cell):
            # matplotlib's table draws each cell by its own draw.
            _python_drawing.paint(self)

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        if not self.get_visible():
            return
        self._place_cells()  # Drawing places the cells first.
        _unusable.check(self)
        renderer.open_group('table', gid=self.get_gid())
        for key in sorted(_table.cell_keys(self._native)):
            self[key].draw(renderer)
        renderer.close_group('table')
        self.stale = False

    def __getitem__(self, position):
        """Retrieve a custom cell from a given position."""
        cell = self._cells.get(position)
        if cell is None:
            # matplotlib looks the position up in a dict of its cells.
            try:
                row, col = position
                found = ((int(row), int(col)) == (row, col)
                         and _table.has_cell(self._native, int(row), int(col)))
            except (TypeError, ValueError):
                found = False
            if not found:
                raise KeyError(position)
            cell = self._cells[position] = Cell._of(self, (row, col))
        return cell

    @property
    def edges(self):
        """
        The default value of `~.Cell.visible_edges` for newly added
        cells using `.add_cell`.

        Notes
        -----
        This setting does currently only affect newly created cells using
        `.add_cell`.

        To change existing cells, you have to set their edges explicitly::

            for c in tab.get_celld().values():
                c.visible_edges = 'horizontal'

        """
        return self._edges

    @edges.setter
    def edges(self, value):
        self._edges = value

    def get_children(self):
        """Return the Artists contained by the table."""
        return list(self.get_celld().values())

    def contains(self, mouseevent):
        # docstring inherited
        if self._kept_cells and not self._different_canvas(mouseevent):
            # matplotlib measures the cells at rows and columns from 0 on,
            # comparing every position with 0.
            [key for key in self.get_celld() if key[0] >= 0 and key[1] >= 0]
        return super().contains(mouseevent)

    def auto_set_column_width(self, col):
        """
        Automatically set the widths of given columns to optimal sizes.

        Parameters
        ----------
        col : int or sequence of ints
            The indices of the columns to auto-scale.
        """
        col1d = np.atleast_1d(col)
        if not np.issubdtype(col1d.dtype, np.integer):
            raise TypeError("col must be an int or sequence of ints.")
        _table.auto_set_column_width(self._native, [int(c) for c in col1d])

    def auto_set_font_size(self, value=True):
        """Automatically set font size."""
        try:
            fits = bool(value)
        except ValueError:
            # matplotlib keeps it, and tests it when it draws the table (an
            # array of more than one value, whose truth is ambiguous).
            _unusable.keep(self, "autoFontsize", value, bool, measured=False)
            return
        _unusable.drop(self, "autoFontsize")
        _table.auto_set_font_size(self._native, fits)

    def scale(self, xscale, yscale):
        """Scale column widths by *xscale* and row heights by *yscale*."""
        for c in self.get_celld().values():
            c.set_width(c.get_width() * xscale)
            c.set_height(c.get_height() * yscale)

    def set_fontsize(self, size):
        """
        Set the font size, in points, of the cell text.

        Parameters
        ----------
        size : float

        Notes
        -----
        As long as auto font size has not been disabled, the value will be
        clipped such that the text fits horizontally into the cell.

        You can disable this behavior using `.auto_set_font_size`.

        >>> the_table.auto_set_font_size(False)
        >>> the_table.set_fontsize(20)

        However, there is no automatic scaling of the row height so that the
        text may exceed the cell boundary.
        """
        _table.set_fontsize(self._native, _points(size))
        # The size given to every cell stays until the table places them.
        self._fontsize_given_at = self._placements
        self.stale = True

    def get_celld(self):
        r"""
        Return a dict of cells in the table mapping *(row, column)* to
        `.Cell`\s.

        Notes
        -----
        You can also directly index into the Table object to access individual
        cells::

            cell = table[row, col]

        """
        cells = {key: self[key] for key in _table.cell_keys(self._native)}
        return {**cells, **self._kept_cells}


def _labels(labels, colours, align, count):
    """The *count* labels of a table's columns or rows, their colours and
    alignment, as matplotlib's `table` fills them in (empty labels for
    colours alone, white for labels alone), or None for none."""
    if labels is None and colours is None:
        return None
    return ([''] * count if labels is None else labels,
            'w' * count if colours is None else colours, align)


def _check_table_cells(ax, loc, widths, cols, edges, body, col_labels, row_labels):
    """
    Fail as matplotlib's `table` fails making the table (`Table`) on the
    Axes *ax* at *loc*, then adding its cells one by one (each a `Cell`,
    *widths* wide, else evenly, with the visible *edges*): the first row
    of the body, the column labels, the row labels. *body* holds the
    colours of the body's first row (None for white) and its alignment,
    *col_labels* and *row_labels* the labels, colours and alignment of
    the labels, or None for none (their texts and colours are indexed by
    column and row, as matplotlib's are).
    """
    _loc_name(loc)
    ax.get_figure(root=False)
    widths = [1.0 / cols] * cols if widths is None else widths

    def check(colour, width, align):
        mcolors.to_rgba(colour)
        0 + width  # noqa: B018  (the corner plus the width)
        _edges_of(edges)
        _api.check_in_list(['center', 'right', 'left'], align=align)

    colours, align = body
    for col in range(cols):
        check('w' if colours is None else colours[col], widths[col], align)
    if col_labels is not None:
        labels, colours, align = col_labels
        for col in range(cols):
            labels[col]  # noqa: B018
            check(colours[col], widths[col], align)
    if row_labels is not None:
        labels, colours, align = row_labels
        for row in range(len(labels)):
            labels[row]  # noqa: B018
            check(colours[row], 1e-15, align)


@_docstring.interpd
def table(ax,
          cellText=None, cellColours=None,
          cellLoc='right', colWidths=None,
          rowLabels=None, rowColours=None, rowLoc='left',
          colLabels=None, colColours=None, colLoc='center',
          loc='bottom', bbox=None, edges='closed',
          **kwargs):
    """
    Add a table to an `~.axes.Axes`.

    .. note::

        ``table()`` has some fundamental design limitations and will not be
        developed further. If you need more functionality, consider
        `blume <https://github.com/swfiua/blume>`__.

    At least one of *cellText* or *cellColours* must be specified. These
    parameters must be 2D lists, in which the outer lists define the rows and
    the inner list define the column values per row. Each row must have the
    same number of elements.

    The table can optionally have row and column headers, which are configured
    using *rowLabels*, *rowColours*, *rowLoc* and *colLabels*, *colColours*,
    *colLoc* respectively.

    For finer grained control over tables, use the `.Table` class and add it to
    the Axes with `.Axes.add_table`.

    Parameters
    ----------
    cellText : 2D list of str or pandas.DataFrame, optional
        The texts to place into the table cells.

        *Note*: Line breaks in the strings are currently not accounted for and
        will result in the text exceeding the cell boundaries.

    cellColours : 2D list of :mpltype:`color`, optional
        The background colors of the cells.

    cellLoc : {'right', 'center', 'left'}
        The alignment of the text within the cells.

    colWidths : list of float, optional
        The column widths in units of the axes. If not given, all columns will
        have a width of *1 / ncols*.

    rowLabels : list of str, optional
        The text of the row header cells.

    rowColours : list of :mpltype:`color`, optional
        The colors of the row header cells.

    rowLoc : {'left', 'center', 'right'}
        The text alignment of the row header cells.

    colLabels : list of str, optional
        The text of the column header cells.

    colColours : list of :mpltype:`color`, optional
        The colors of the column header cells.

    colLoc : {'center', 'left', 'right'}
        The text alignment of the column header cells.

    loc : str, default: 'bottom'
        The position of the cell with respect to *ax*. This must be one of
        the `~.Table.codes`.

    bbox : `.Bbox` or [xmin, ymin, width, height], optional
        A bounding box to draw the table into. If this is not *None*, this
        overrides *loc*.

    edges : {'closed', 'open', 'horizontal', 'vertical'} or substring of 'BRTL'
        The cell edges to be drawn with a line. See also
        `~.Cell.visible_edges`.

    Returns
    -------
    `~matplotlib.table.Table`
        The created table.

    Other Parameters
    ----------------
    **kwargs
        `.Table` properties.

    %(Table:kwdoc)s
    """
    if cellColours is None and cellText is None:
        raise ValueError('At least one argument from "cellColours" or '
                         '"cellText" must be provided to create a table.')

    # Check we have some cellText
    if cellText is None:
        # assume just colours are needed
        rows = len(cellColours)
        cols = len(cellColours[0])
        cellText = [[''] * cols] * rows

    # Check if we have a Pandas DataFrame
    if cbook._is_pandas_dataframe(cellText):
        # if rowLabels/colLabels are empty, use DataFrame entries.
        # Otherwise, throw an error.
        if rowLabels is None:
            rowLabels = cellText.index
        else:
            raise ValueError("rowLabels cannot be used alongside Pandas DataFrame")
        if colLabels is None:
            colLabels = cellText.columns
        else:
            raise ValueError("colLabels cannot be used alongside Pandas DataFrame")
        # Update cellText with only values
        cellText = cellText.values

    rows = len(cellText)
    cols = len(cellText[0])
    for row in cellText:
        if len(row) != cols:
            raise ValueError(f"Each row in 'cellText' must have {cols} "
                             "columns")

    if cellColours is not None:
        if len(cellColours) != rows:
            raise ValueError(f"'cellColours' must have {rows} rows")
        for row in cellColours:
            if len(row) != cols:
                raise ValueError("Each row in 'cellColours' must have "
                                 f"{cols} columns")

    if rowLabels is not None and len(rowLabels) != rows:
        raise ValueError(f"'rowLabels' must be of length {rows}")

    def texts(values):
        return None if values is None else [str(value) for value in values]

    def colours(values):
        return None if values is None else [mcolors.to_rgba(c) for c in values]

    _check_table_cells(
        ax, loc, colWidths, cols, edges,
        (None if cellColours is None else cellColours[0], cellLoc),
        _labels(colLabels, colColours, colLoc, cols),
        _labels(rowLabels, rowColours, rowLoc, rows))
    fontsize = kwargs.pop("fontsize", None)
    native = _table.axes_table(
        ax._native,
        ([texts(row) for row in cellText],
         None if cellColours is None else [colours(row) for row in cellColours],
         cellLoc, None if colWidths is None else [float(w) for w in colWidths]),
        # (The alignment of labels there are none of is never checked.)
        (texts(rowLabels), colours(rowColours),
         rowLoc if rowLabels is not None else 'left'),
        (texts(colLabels), colours(colColours),
         colLoc if colLabels is not None else 'center'),
        (_loc_name(loc), _extents(bbox), _edges_of(edges),
         None if fontsize is None else float(fontsize)))
    table = _handles.wrap(native, Table, ax.figure)
    table._keep_bbox(bbox)
    # matplotlib's ``table`` adds the table with ``add_table``.
    _artist_clipping.take_parent_defaults(table, ax)
    table.edges = edges
    table._internal_update(kwargs)
    return table
