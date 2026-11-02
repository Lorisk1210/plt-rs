"""
Tables of text cells drawn in or next to an Axes (matplotlib's ``table``).

Use the factory function `~matplotlib.table.table` to create a ready-made
table from texts. If you need more control, use the `.Table` class and its
methods to add cells one by one.

pltrs (Rust) lays the table out and draws it (``_native.table``). Its cells
are part of the table's native artist, not artists of their own: a `Cell`
is the cell at its ``(row, column)`` key in its table. A cell made by its
constructor lives in a table of its own, outside any figure, until it is
put in a table (``table[row, col] = cell``), which copies it there. The
signatures and checks are matplotlib's (3.11.2, under the Matplotlib
License).
"""

import numpy as np

from . import _api, _handles, _live_transforms, cbook, colors as mcolors
from ._artist_children import ChildrenSearchMixin
from ._axis_artists import _pushing
from ._native import table as _table
from .artist import Artist
from .patches import Rectangle
from .path import Path
from .text import Text
from .transforms import BboxBase, Bbox


class _CellHolder:
    """The native table holding a cell made outside any table, and the
    figure given to the cell (`Cell.set_figure`)."""

    def __init__(self):
        self._native = _table.new(None, "bottom", None)
        self.figure = None


@_pushing
class _CellText(Text):
    """The text of the cell ``_owner`` as a text artist (`Cell.get_text`):
    its string, colour, font and horizontal alignment are the cell's."""

    _owner = None

    def _push(self):
        _table.restyle_text(self._owner._ref(), self._native)


def _measured(value):
    """*value*, measured in the figure of a cell's table; None, for a cell
    in no table's figure, fails as matplotlib's measuring of a text in no
    figure does."""
    if value is None:
        raise AttributeError("'NoneType' object has no attribute 'dpi'")
    return value


class Cell(ChildrenSearchMixin):
    """
    A cell is a rectangle with some associated text.

    As a user, you'll most likely not creates cells yourself. Instead, you
    should use either the `~matplotlib.table.table` factory function or
    `.Table.add_cell`.

    pltrs: a cell is the cell at its key in a table's native artist (see
    the module documentation), not a `.Rectangle` artist of its own.
    """

    PAD = 0.1
    """Padding between text and rectangle."""

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
            The position of the bottom left corner of the cell (pltrs: the
            table places its cells when it is drawn).
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
        if not fill:
            raise NotImplementedError("pltrs: Cell(fill=False)")
        self._table = _CellHolder()
        self._key = (0, 0)
        _table.add_cell(self._table._native, 0, 0, float(width), float(height))
        self.set_edgecolor(edgecolor)
        self.set_facecolor(facecolor)
        self.visible_edges = visible_edges
        self._loc = loc
        _table.set_text_halign(self._ref(), loc)
        _table.set_text(self._ref(), str(text))
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
        return cell

    def _ref(self):
        """The cell as ``_native.table`` takes it."""
        return (self._table._native, *self._key)

    @property
    def _parent_figure(self):
        """The `.Figure` or `.SubFigure` of the cell's table (the one given
        to `set_figure` for a cell in no table)."""
        if isinstance(self._table, _CellHolder):
            return self._table.figure
        return self._table.get_figure(root=False)

    def set(self, **kwargs):
        """
        Set multiple properties at once, each keyword naming a setter of
        the cell (``set_<keyword>``), the aliases of `.Rectangle` allowed;
        return the setters' results.
        """
        ret = []
        for k, v in cbook.normalize_kwargs(kwargs, Rectangle).items():
            func = getattr(self, f"set_{k}", None)
            if not callable(func):
                raise AttributeError(
                    f"{type(self).__name__}.set() got an unexpected keyword "
                    f"argument {k!r}", name=k)
            ret.append(func(v))
        return ret

    def set_transform(self, t):
        """
        Set the artist transform: the cell, its place in its table being in
        the coordinates of *t* (pltrs: its text is drawn there too).

        Parameters
        ----------
        t : `~matplotlib.transforms.Transform`
        """
        _table.set_cell_transform(self._ref(), _live_transforms.to_native(t))

    def set_figure(self, fig):
        """
        Set the `.Figure` or `.SubFigure` instance the cell belongs to.

        pltrs: a cell in a table belongs to its table's figure; one made by
        its constructor keeps *fig* until a table takes it.
        """
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
        """
        Return the cell `.Text` instance.

        pltrs: the text has the cell's string, colour, font and horizontal
        alignment, and changing them changes the cell; the table places
        the text when it is drawn.
        """
        native = _table.text_artist(self._ref())
        text = self.__dict__.get("_text_mirror")
        if text is None:
            text = self._text_mirror = _CellText._from_native(native, None)
            text._owner = self
        else:
            text._native = native
        return text

    def set_fontsize(self, size):
        """Set the text fontsize."""
        _table.set_cell_fontsize(self._ref(), float(size))

    def get_fontsize(self):
        """Return the cell fontsize."""
        return _table.get_cell_fontsize(self._ref())

    def auto_set_font_size(self, renderer):
        """
        Shrink font size until the text fits into the cell width.

        pltrs: measured in the figure's current layout at its resolution.
        """
        if isinstance(self._table, _CellHolder):
            return _measured(None)
        return _table.fit_font_size(self._ref())

    def draw(self, renderer):
        """
        Draw the cell alone with *renderer*: its rectangle and its text,
        where its table places it in its figure's last layout.
        """
        draw_cell = getattr(renderer, "_draw_cell", None)
        if draw_cell is None:
            raise NotImplementedError(
                "pltrs: Cell.draw(renderer) with a renderer pltrs (Rust) is "
                "not drawing with")
        draw_cell(self)

    def get_text_bounds(self, renderer):
        """
        Return the text bounds as *(x, y, width, height)* in table
        coordinates.

        pltrs: measured where the table draws the text in the figure's
        current layout, at the figure's resolution.
        """
        return _measured(_table.text_bounds(self._ref()))

    def get_required_width(self, renderer):
        """Return the minimal required width for the cell."""
        return _measured(_table.required_width(self._ref()))

    def set_text_props(self, **kwargs):
        """
        Update the text properties: its *color*, horizontal alignment
        (*horizontalalignment*, *ha*) and *fontsize*.
        """
        for key, value in kwargs.items():
            if key in ("color", "c"):
                _table.set_text_color(self._ref(), mcolors.to_rgba(value))
            elif key in ("horizontalalignment", "ha"):
                _table.set_text_halign(self._ref(), value)
            elif key in ("fontsize", "size"):
                self.set_fontsize(value)
            else:
                raise NotImplementedError(f"pltrs: Cell.set_text_props({key}=...)")

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
        _table.set_visible_edges(self._ref(), 'closed' if value is None else value)

    def get_path(self):
        """Return a `.Path` for the `.visible_edges`."""
        vertices, codes = _table.get_path(self._ref())
        return Path(vertices, codes, readonly=True)

    def get_width(self):
        """Return the width of the cell."""
        return _table.get_size(self._ref())[0]

    def get_height(self):
        """Return the height of the cell."""
        return _table.get_size(self._ref())[1]

    def set_width(self, w):
        """Set the width of the cell."""
        _table.set_width(self._ref(), float(w))

    def set_height(self, h):
        """Set the height of the cell."""
        _table.set_height(self._ref(), float(h))

    def set_facecolor(self, color):
        """Set the cell facecolor."""
        _table.set_facecolor(self._ref(), mcolors.to_rgba(color))

    def set_edgecolor(self, color):
        """Set the color of the cell border."""
        _table.set_edgecolor(self._ref(), mcolors.to_rgba(color))

    def set_linewidth(self, w):
        """Set the width of the cell border in points."""
        _table.set_linewidth(self._ref(), float(w))

    def set_alpha(self, alpha):
        """Set the opacity of the cell background and border."""
        _table.set_alpha(self._ref(), None if alpha is None else float(alpha))


CustomCell = Cell  # Backcompat. alias.


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


def _extents(bbox):
    """The extents of *bbox*, a `.Bbox` or ``[xmin, ymin, width, height]``."""
    if bbox is None:
        return None
    if not isinstance(bbox, BboxBase):
        bbox = Bbox.from_bounds(*bbox)
    return tuple(float(v) for v in bbox.extents)


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
        self._native = _table.new(ax._native, _loc_name(loc), _extents(bbox))
        super().__init__()
        self._axes = ax
        self._edges = None
        self._cells = {}
        self._internal_update(kwargs)

    @classmethod
    def _from_native(cls, native, figure):
        table = super()._from_native(native, figure)
        table._edges = None
        table._cells = {}
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
        _table.put_cell(self._native, int(row), int(col), cell._ref())
        cell._table = self
        cell._key = (row, col)
        self._cells[row, col] = cell

    def __getitem__(self, position):
        """Retrieve a custom cell from a given position."""
        cell = self._cells.get(position)
        if cell is None:
            row, col = position
            if not _table.has_cell(self._native, int(row), int(col)):
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
        _table.auto_set_font_size(self._native, bool(value))

    def scale(self, xscale, yscale):
        """Scale column widths by *xscale* and row heights by *yscale*."""
        _table.scale(self._native, float(xscale), float(yscale))

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
        _table.set_fontsize(self._native, float(size))

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
        return {key: self[key] for key in _table.cell_keys(self._native)}


def table(ax,
          cellText=None, cellColours=None,
          cellLoc='right', colWidths=None,
          rowLabels=None, rowColours=None, rowLoc='left',
          colLabels=None, colColours=None, colLoc='center',
          loc='bottom', bbox=None, edges='closed',
          **kwargs):
    """
    Add a table to an `~.axes.Axes`.

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

    fontsize = kwargs.pop("fontsize", None)
    native = _table.axes_table(
        ax._native,
        ([texts(row) for row in cellText],
         None if cellColours is None else [colours(row) for row in cellColours],
         cellLoc, None if colWidths is None else [float(w) for w in colWidths]),
        (texts(rowLabels), colours(rowColours), rowLoc),
        (texts(colLabels), colours(colColours), colLoc),
        (_loc_name(loc), _extents(bbox), edges,
         None if fontsize is None else float(fontsize)))
    table = _handles.wrap(native, Table, ax.figure)
    table.edges = edges
    table._internal_update(kwargs)
    return table
