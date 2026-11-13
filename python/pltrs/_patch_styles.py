"""
The patch styles of matplotlib's ``patches``: `BoxStyle`,
`ConnectionStyle` and `ArrowStyle`.

Each is a registry of style classes (the ``_Style`` machinery is
matplotlib's); a style instance keeps its parameters, and pltrs (Rust)
draws it from matplotlib's description string (``"round,pad=0.2"``,
see ``_native.patch_styles``). A style of the program's (a subclass of
``_Base`` it defines and may `~_Style.register`, or a callable) is called
by pltrs (Rust) where it draws with it, as matplotlib calls it (see
`_native_spec`).
"""

import functools
import inspect
from types import SimpleNamespace
import textwrap
import weakref

import numpy as np

from . import _api, _docstring, _patch_style_docs as _style_docs, _qualnames
from ._native import patch_styles as _styles
from ._program_classes import takes_from_program
from .bezier import inside_circle, split_path_inout
from .path import Path


class _Style:
    """
    A base class for the Styles. It is meant to be a container class,
    where actual styles are declared as subclass of it, and it
    provides some helper functions.
    """

    def __new__(cls, stylename, **kwargs):
        """Return the instance of the subclass with the given style name."""
        # The "class" should have the _style_list attribute, which is a mapping
        # of style names to style classes.
        _list = stylename.replace(" ", "").split(",")
        _name = _list[0].lower()
        try:
            _cls = cls._style_list[_name]
        except KeyError as err:
            raise ValueError(f"Unknown style: {stylename!r}") from err
        try:
            _args_pair = [cs.split("=") for cs in _list[1:]]
            _args = {k: float(v) for k, v in _args_pair}
        except ValueError as err:
            raise ValueError(
                f"Incorrect style argument: {stylename!r}") from err
        return _cls(**{**_args, **kwargs})

    @classmethod
    def get_styles(cls):
        """Return a dictionary of available styles."""
        return cls._style_list

    @classmethod
    @_api.deprecated(
        '3.10',
        message="This method is never used internally.",
        alternative="No replacement.  Please open an issue if you use this.")
    def register(cls, name, style):
        """Register a new style."""
        if not issubclass(style, cls._Base):
            raise ValueError(f"{style} must be a subclass of {cls._Base}")
        cls._style_list[name] = style

    @classmethod
    def pprint_styles(cls):
        """Return the available styles as pretty-printed string."""
        table = [('Class', 'Name', 'Parameters'),
                 *[(cls.__name__,
                    # Add backquotes, as - and | have special meaning in reST.
                    f'``{name}``',
                    # [1:-1] drops the surrounding parentheses.
                    str(inspect.signature(cls))[1:-1] or 'None')
                   for name, cls in cls._style_list.items()]]
        # Convert to rst table.
        col_len = [max(len(cell) for cell in column) for column in zip(*table)]
        table_formatstr = '  '.join('=' * cl for cl in col_len)
        rst_table = '\n'.join([
            '',
            table_formatstr,
            '  '.join(cell.ljust(cl) for cell, cl in zip(table[0], col_len)),
            table_formatstr,
            *['  '.join(cell.ljust(cl) for cell, cl in zip(row, col_len))
              for row in table[1:]],
            table_formatstr,
        ])
        return textwrap.indent(rst_table, prefix=' ' * 4)


def _register_style(style_list, cls=None, *, name=None):
    """Class decorator that stashes a class in a (style) dictionary."""
    if cls is None:
        return functools.partial(_register_style, style_list, name=name)
    style_list[name or cls.__name__.lower()] = cls
    return cls


def _register_tables(style):
    """
    Register the docstring snippets listing the styles of the container
    *style* once they are defined (``%(BoxStyle:table)s`` and
    ``%(BoxStyle:table_and_accepts)s``, ...), as matplotlib's
    ``_Style.__init_subclass__`` registers them, and interpolate the
    container's docstring with them, as its ``@_docstring.interpd`` does.
    """
    table = style.pprint_styles()
    _docstring.interpd.register(**{
        f"{style.__name__}:table": table,
        f"{style.__name__}:table_and_accepts": (
            table
            + "\n\n    .. ACCEPTS: ["
            + "|".join(map(" '{}' ".format, style._style_list))
            + "]")
    })
    _docstring.interpd(style)


class _Described:
    """
    A style whose parameters are its constructor's arguments: it describes
    itself as matplotlib's style string for the native side.
    """

    #: The name pltrs (Rust) knows the style by.
    _spec_name = None

    def __init__(self, **params):
        self._params = params
        for key, value in params.items():
            setattr(self, key, value)

    def _spec(self):
        """The style as ``"name,key=value,..."``, unset parameters left out."""
        parts = [self._spec_name]
        parts += [f"{key}={getattr(self, key)}" for key in self._params
                  if getattr(self, key) is not None]
        return ",".join(parts)


def _described(name, qualname, check=None, **defaults):
    """
    A `_Described` style class named *name* whose constructor takes the
    keyword parameters *defaults* (with their default values), as the
    constructor matplotlib defines in its class *qualname*
    (``"BoxStyle.Square"``), so that Python words the errors of a call as
    it words matplotlib's. *check*, given the parameters, raises
    matplotlib's errors for values its constructor refuses.
    """
    parameters = ", ".join(f"{key}=_defaults[{key!r}]" for key in defaults)
    arguments = ", ".join(f"{key}={key}" for key in defaults)
    namespace = {"_defaults": defaults, "_check": check, "_Described": _Described}
    exec(_docstring.compiled(textwrap.dedent(f"""
        def __init__(self, {parameters}):
            if _check is not None:
                _check({arguments})
            _Described.__init__(self, {arguments})
    """), "<string>"), namespace)
    init = _qualnames.named_as(f"{qualname}.__init__")(namespace["__init__"])
    return type(name, (_Described,), {"__init__": init})


# What the methods of matplotlib's styles compute, which pltrs (Rust) does
# from the style's description (each style class has them as methods of its
# own, `._qualnames.own_method`): a box style's path around a rectangle, a connection
# style's path between two points, and an arrow style's paths along a path
# with whether each is filled (one path and a bool for a one-piece arrow).

def _box_path(self, x0, y0, width, height, mutation_size):
    return Path(*_styles.box_path(
        self._spec(), (float(x0), float(y0), float(width), float(height)),
        float(mutation_size)))


def _connection_path(self, posA, posB):
    return Path(*_styles.connection_path(
        self._spec(), tuple(map(float, posA)), tuple(map(float, posB))))


def _arrow_paths(self, path, mutation_size, linewidth):
    pieces = _styles.arrow_paths(self._spec(), path, float(mutation_size),
                                 float(linewidth), 1.0)
    paths = [Path(*arrays) for arrays, _ in pieces]
    fillables = [filled for _, filled in pieces]
    if len(paths) == 1:
        return paths[0], fillables[0]
    return paths, fillables


def _check_rarrow(pad, head_width, head_angle):
    """matplotlib's checks of the head of `BoxStyle.RArrow`."""
    if head_width < 0:
        raise ValueError("'head_width' must be nonnegative")
    if head_angle % 360 == 0:
        raise ValueError("'head_angle' must be nonzero")


class BoxStyle(_Style):
    """
    `BoxStyle` is a container class which defines several
    boxstyle classes, which are used for `FancyBboxPatch`.

    A style object can be created as::

           BoxStyle.Round(pad=0.2)

    or::

           BoxStyle("Round", pad=0.2)

    or::

           BoxStyle("Round, pad=0.2")

    The following boxstyle classes are defined.

    %(BoxStyle:table)s

    An instance of a boxstyle class is a callable object, with the signature ::

       __call__(self, x0, y0, width, height, mutation_size) -> Path

    *x0*, *y0*, *width* and *height* specify the location and size of the box
    to be drawn; *mutation_size* scales the outline properties such as padding.
    """

    _style_list = {}

    class _Base(_Described):
        pass


# Each style with its parameters, or the style it derives from (and takes
# the parameters of), as matplotlib's classes do.
for _name, _params in [
        ("Square", dict(pad=0.3)),
        ("Circle", dict(pad=0.3)),
        ("Ellipse", dict(pad=0.3)),
        ("RArrow", dict(pad=0.3, head_width=1.5, head_angle=90, check=_check_rarrow)),
        ("LArrow", "RArrow"),
        ("DArrow", "RArrow"),
        ("Round", dict(pad=0.3, rounding_size=None)),
        ("Round4", dict(pad=0.3, rounding_size=None)),
        ("Sawtooth", dict(pad=0.3, tooth_size=None)),
        ("Roundtooth", "Sawtooth")]:
    _bases = ((getattr(BoxStyle, _params),) if isinstance(_params, str)
              else (BoxStyle._Base, _described(_name, f"BoxStyle.{_name}", **_params)))
    _style = type(_name, _bases,
                  {"_spec_name": _name.lower(), "__qualname__": f"BoxStyle.{_name}"})
    _qualnames.own_method(_style, "__call__", _box_path)
    _style_docs.document(_style, BoxStyle)
    setattr(BoxStyle, _name, _register_style(BoxStyle._style_list, _style))
_register_tables(BoxStyle)


class ConnectionStyle(_Style):
    """
    `ConnectionStyle` is a container class which defines
    several connectionstyle classes, which is used to create a path
    between two points.  These are mainly used with `FancyArrowPatch`.

    A connectionstyle object can be either created as::

           ConnectionStyle.Arc3(rad=0.2)

    or::

           ConnectionStyle("Arc3", rad=0.2)

    or::

           ConnectionStyle("Arc3, rad=0.2")

    The following classes are defined

    %(ConnectionStyle:table)s

    An instance of any connection style class is a callable object,
    whose call signature is::

        __call__(self, posA, posB,
                 patchA=None, patchB=None,
                 shrinkA=2., shrinkB=2.)

    and it returns a `.Path` instance. *posA* and *posB* are
    tuples of (x, y) coordinates of the two points to be
    connected. *patchA* (or *patchB*) is given, the returned path is
    clipped so that it start (or end) from the boundary of the
    patch. The path is further shrunk by *shrinkA* (or *shrinkB*)
    which is given in points.
    """

    _style_list = {}

    class _Base(_Described):
        """
        A base class for connectionstyle classes. The subclass needs
        to implement a *connect* method whose call signature is::

          connect(posA, posB)

        where posA and posB are tuples of x, y coordinates to be
        connected.  The method needs to return a path connecting two
        points. This base class defines a __call__ method, and a few
        helper methods.
        """

        def _in_patch(self, patch):
            """
            Return a predicate function testing whether a point *xy* is
            contained in *patch*.
            """
            return lambda xy: patch.contains(
                SimpleNamespace(x=xy[0], y=xy[1]))[0]

        def _clip(self, path, in_start, in_stop):
            """
            Clip *path* at its start by the region where *in_start* returns
            True, and at its stop by the region where *in_stop* returns True.

            The original path is assumed to start in the *in_start* region and
            to stop in the *in_stop* region.
            """
            if in_start:
                try:
                    _, path = split_path_inout(path, in_start)
                except ValueError:
                    pass
            if in_stop:
                try:
                    path, _ = split_path_inout(path, in_stop)
                except ValueError:
                    pass
            return path

        def __call__(self, posA, posB,
                     shrinkA=2., shrinkB=2., patchA=None, patchB=None):
            """
            Call the *connect* method to create a path between *posA* and
            *posB*; then clip and shrink the path.
            """
            path = self.connect(posA, posB)
            path = self._clip(
                path,
                self._in_patch(patchA) if patchA else None,
                self._in_patch(patchB) if patchB else None,
            )
            path = self._clip(
                path,
                inside_circle(*path.vertices[0], shrinkA) if shrinkA else None,
                inside_circle(*path.vertices[-1], shrinkB) if shrinkB else None
            )
            return path


for _name, _params in [
        ("Arc3", dict(rad=0.)),
        ("Angle3", dict(angleA=90, angleB=0)),
        ("Angle", dict(angleA=90, angleB=0, rad=0.)),
        ("Arc", dict(angleA=0, angleB=0, armA=None, armB=None, rad=0.)),
        ("Bar", dict(armA=0., armB=0., fraction=0.3, angle=None))]:
    _style = type(_name, (ConnectionStyle._Base,
                          _described(_name, f"ConnectionStyle.{_name}", **_params)),
                  {"_spec_name": _name.lower(),
                   "__qualname__": f"ConnectionStyle.{_name}"})
    _qualnames.own_method(_style, "connect", _connection_path)
    _style_docs.document(_style, ConnectionStyle)
    setattr(ConnectionStyle, _name,
            _register_style(ConnectionStyle._style_list, _style))
_register_tables(ConnectionStyle)


class ArrowStyle(_Style):
    """
    `ArrowStyle` is a container class which defines several
    arrowstyle classes, which is used to create an arrow path along a
    given path.  These are mainly used with `FancyArrowPatch`.

    An arrowstyle object can be either created as::

           ArrowStyle.Fancy(head_length=.4, head_width=.4, tail_width=.4)

    or::

           ArrowStyle("Fancy", head_length=.4, head_width=.4, tail_width=.4)

    or::

           ArrowStyle("Fancy, head_length=.4, head_width=.4, tail_width=.4")

    The following classes are defined

    %(ArrowStyle:table)s

    For an overview of the visual appearance, see
    :doc:`/gallery/text_labels_and_annotations/fancyarrow_demo`.

    An instance of any arrow style class is a callable object,
    whose call signature is::

        __call__(self, path, mutation_size, linewidth, aspect_ratio=1.)

    and it returns a tuple of a `.Path` instance and a boolean
    value. *path* is a `.Path` instance along which the arrow
    will be drawn. *mutation_size* and *aspect_ratio* have the same
    meaning as in `BoxStyle`. *linewidth* is a line width to be
    stroked. This is meant to be used to correct the location of the
    head so that it does not overshoot the destination point, but not all
    classes support it.

    Notes
    -----
    *angleA* and *angleB* specify the orientation of the bracket, as either a
    clockwise or counterclockwise angle depending on the arrow type. 0 degrees
    means perpendicular to the line connecting the arrow's head and tail.

    .. plot:: gallery/text_labels_and_annotations/angles_on_bracket_arrows.py
    """

    _style_list = {}

    class _Base(_Described):
        """
        Arrow Transmuter Base class

        ArrowTransmuterBase and its derivatives are used to make a fancy
        arrow around a given path. The __call__ method returns a path
        (which will be used to create a PathPatch instance) and a boolean
        value indicating the path is open therefore is not fillable.  This
        class is not an artist and actual drawing of the fancy arrow is
        done by the FancyArrowPatch class.
        """

        # matplotlib's, as it is.
        @staticmethod
        def ensure_quadratic_bezier(path):
            """
            Some ArrowStyle classes only works with a simple quadratic
            Bézier curve (created with `.ConnectionStyle.Arc3` or
            `.ConnectionStyle.Angle3`). This static method checks if the
            provided path is a simple quadratic Bézier curve and returns its
            control points if true.
            """
            segments = list(path.iter_segments())
            if (len(segments) != 2 or segments[0][1] != Path.MOVETO or
                    segments[1][1] != Path.CURVE3):
                raise ValueError(
                    "'path' is not a valid quadratic Bezier curve")
            return [*segments[0][0], *segments[1][0]]

        def transmute(self, path, mutation_size, linewidth):
            """
            The transmute method is the very core of the ArrowStyle class and
            must be overridden in the subclasses. It receives the *path*
            object along which the arrow will be drawn, and the
            *mutation_size*, with which the arrow head etc. will be scaled.
            The *linewidth* may be used to adjust the path so that it does not
            pass beyond the given points. It returns a tuple of a `.Path`
            instance and a boolean. The boolean value indicate whether the
            path can be filled or not. The return value can also be a list of
            paths and list of booleans of the same length.
            """
            raise NotImplementedError('Derived must override')

        def __call__(self, path, mutation_size, linewidth,
                     aspect_ratio=1.):
            """
            The __call__ method is a thin wrapper around the transmute method
            and takes care of the aspect ratio.
            """
            if aspect_ratio is not None:
                # Squeeze the given height by the aspect_ratio
                vertices = path.vertices / [1, aspect_ratio]
                path_shrunk = Path(vertices, path.codes)
                # call transmute method with squeezed height.
                path_mutated, fillable = self.transmute(path_shrunk,
                                                        mutation_size,
                                                        linewidth)
                if np.iterable(fillable):
                    # Restore the height
                    path_list = [Path(p.vertices * [1, aspect_ratio], p.codes)
                                 for p in path_mutated]
                    return path_list, fillable
                else:
                    return path_mutated, fillable
            else:
                return self.transmute(path, mutation_size, linewidth)

    class _Curve(_Base):
        """
        A simple arrow which will work with any path instance. The
        returned path is the concatenation of the original path, and at
        most two paths representing the arrow head or bracket at the start
        point and at the end point. The arrow heads can be either open
        or closed.
        """


_qualnames.own_method(ArrowStyle._Curve, "transmute", _arrow_paths)
_CURVE = dict(head_length=.4, head_width=.2, widthA=1., widthB=1.,
              lengthA=0.2, lengthB=0.2, angleA=0, angleB=0, scaleA=None,
              scaleB=None)
# matplotlib's constructor of `_Curve`, which its curves with heads inherit.
ArrowStyle._Curve.__init__ = _described("_Curve", "ArrowStyle._Curve", **_CURVE).__init__
for _name, _arrow, _params in [
        ("Curve", "-", {}),
        ("CurveA", "<-", _CURVE),
        ("CurveB", "->", _CURVE),
        ("CurveAB", "<->", _CURVE),
        ("CurveFilledA", "<|-", _CURVE),
        ("CurveFilledB", "-|>", _CURVE),
        ("CurveFilledAB", "<|-|>", _CURVE),
        ("BracketA", "]-", dict(widthA=1., lengthA=0.2, angleA=0)),
        ("BracketB", "-[", dict(widthB=1., lengthB=0.2, angleB=0)),
        ("BracketAB", "]-[", dict(widthA=1., lengthA=0.2, angleA=0,
                                  widthB=1., lengthB=0.2, angleB=0)),
        ("BarAB", "|-|", dict(widthA=1., angleA=0, widthB=1., angleB=0)),
        ("BracketCurve", "]->", dict(widthA=1., lengthA=0.2, angleA=None)),
        ("CurveBracket", "<-[", dict(widthB=1., lengthB=0.2, angleB=None)),
        ("Simple", "simple", dict(head_length=.5, head_width=.5, tail_width=.2)),
        ("Fancy", "fancy", dict(head_length=.4, head_width=.4, tail_width=.4)),
        ("Wedge", "wedge", dict(tail_width=.3, shrink_factor=0.5))]:
    _base = (ArrowStyle._Base if _name in ("Simple", "Fancy", "Wedge")
             else ArrowStyle._Curve)
    # The curves with heads take the parameters of matplotlib's `_Curve`
    # (its constructor, below); the others have constructors of their own.
    # matplotlib's curves name their ends in ``arrow``; its other styles do not.
    _ends = {"arrow": _arrow} if _base is ArrowStyle._Curve else {}
    _own = {"_spec_name": _arrow, **_ends, "__qualname__": f"ArrowStyle.{_name}"}
    _parameters = _described(_name, f"ArrowStyle.{_name}", **_params)
    if _params is not _CURVE:
        _own["__init__"] = _parameters.__init__
    _style = type(_name, (_base, _parameters), _own)
    if _base is ArrowStyle._Base:
        _qualnames.own_method(_style, "transmute", _arrow_paths)
    _style_docs.document(_style, ArrowStyle)
    setattr(ArrowStyle, _name, _register_style(ArrowStyle._style_list, _style,
                                               name=_arrow))
_register_tables(ArrowStyle)
del _name, _arrow, _params, _base, _style, _ends, _own, _parameters


class _PlainRoute:
    """
    A connection style of the program's that is a plain callable, as
    pltrs (Rust) calls it (a route, see ``_native.patch_styles``): with the
    patches at the ends of the arrow drawn with it and the shrinking in
    pixels, as matplotlib's arrows call it, its path drawn as it is
    (networkx's self loops ignore the shrinking). The arrow is held weakly
    and not pickled: the style of an arrow read back gets no patches.
    """

    def __init__(self, style, arrow=None):
        self.style = style
        self._arrow = None if arrow is None else weakref.ref(arrow)

    def __getstate__(self):
        return {"style": self.style, "_arrow": None}

    def __call__(self, posA, posB, shrinkA, shrinkB):
        arrow = None if self._arrow is None else self._arrow()
        return self.style(posA, posB,
                          patchA=getattr(arrow, "patchA", None),
                          patchB=getattr(arrow, "patchB", None),
                          shrinkA=shrinkA, shrinkB=shrinkB)


def _arrow_pieces(style, vertices, codes, mutation_size, linewidth, aspect):
    """The pieces of the arrow an arrow style of the program's makes along
    the path of *vertices* and *codes*: ``(path, filled)`` pairs."""
    paths, fillable = style(Path(vertices, codes), mutation_size, linewidth,
                            aspect)
    if np.iterable(fillable):
        return [(path, bool(filled)) for path, filled in zip(paths, fillable)]
    return [(paths, bool(fillable))]


#: The methods matplotlib's fancy patches call on their styles to draw
#: (a box style's and an arrow style's ``__call__``, which calls the arrow
#: style's ``transmute``, a connection style's ``connect``).
_DRAWING_METHODS = ("__call__", "connect", "transmute")


def calls_program(style):
    """Whether pltrs (Rust) calls the program to draw with *style*, a style
    object: whether it is the program's (or a plain callable), or a style
    of matplotlib's whose class the program derived to draw its own way."""
    if not (isinstance(style, _Described) and type(style)._spec_name is not None):
        return True
    return any(takes_from_program(type(style), name) for name in _DRAWING_METHODS)


def _native_spec(style, registry, owner=None):
    """
    *style* of *registry* (a name, which *registry* validates, or a style
    object) as the native side takes it: a style of matplotlib's as its
    description string; a style of the program's as what pltrs (Rust)
    calls where it draws with it: a box style itself, a connection
    style's ``connect`` (a plain callable as a `_PlainRoute` of the arrow
    *owner*, ``("route", route)``), an arrow style through
    `_arrow_pieces`.
    """
    if isinstance(style, str):
        style = registry(style)
    if not calls_program(style):
        return style._spec()
    if not callable(style):
        raise TypeError(f"{style!r} is not a {registry.__name__}")
    if registry is ConnectionStyle:
        connect = getattr(style, "connect", None)
        return connect if connect is not None else ("route", _PlainRoute(style, owner))
    if registry is ArrowStyle:
        return functools.partial(_arrow_pieces, style)
    return style


__all__ = ["ArrowStyle", "BoxStyle", "ConnectionStyle"]
