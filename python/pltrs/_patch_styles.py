"""
The patch styles of matplotlib's ``patches``: `BoxStyle`,
`ConnectionStyle` and `ArrowStyle`.

Each is a registry of style classes (the ``_Style`` machinery is
matplotlib's); a style instance keeps its parameters, and pltrs (Rust)
draws it from matplotlib's description string (``"round,pad=0.2"``,
see ``_native.patch_styles``).
"""

import functools
import inspect
import textwrap

from ._native import patch_styles as _styles
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


def _described(name, **defaults):
    """
    A `_Described` style class named *name* whose constructor takes the
    keyword parameters *defaults* (with their default values).
    """
    def __init__(self, *args, **kwargs):
        bound = signature.bind(self, *args, **kwargs)
        bound.apply_defaults()
        params = dict(bound.arguments)
        params.pop("self")
        _Described.__init__(self, **params)

    signature = inspect.Signature(
        [inspect.Parameter("self", inspect.Parameter.POSITIONAL_OR_KEYWORD)]
        + [inspect.Parameter(key, inspect.Parameter.POSITIONAL_OR_KEYWORD,
                             default=value) for key, value in defaults.items()])
    __init__.__signature__ = signature
    return type(name, (_Described,), {"__init__": __init__})


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

    An instance of a box style is callable: ``style(x0, y0, width,
    height, mutation_size)`` returns the `.Path` of the box.
    """

    _style_list = {}

    class _Base(_Described):
        def __call__(self, x0, y0, width, height, mutation_size):
            """Return the path of the box around the given rectangle."""
            return Path(*_styles.box_path(
                self._spec(), (float(x0), float(y0), float(width), float(height)),
                float(mutation_size)))


for _name, _params in [
        ("Square", dict(pad=0.3)),
        ("Circle", dict(pad=0.3)),
        ("Ellipse", dict(pad=0.3)),
        ("LArrow", dict(pad=0.3, head_width=1.5, head_angle=90)),
        ("RArrow", dict(pad=0.3, head_width=1.5, head_angle=90)),
        ("DArrow", dict(pad=0.3, head_width=1.5, head_angle=90)),
        ("Round", dict(pad=0.3, rounding_size=None)),
        ("Round4", dict(pad=0.3, rounding_size=None)),
        ("Sawtooth", dict(pad=0.3, tooth_size=None)),
        ("Roundtooth", dict(pad=0.3, tooth_size=None))]:
    _style = type(_name, (BoxStyle._Base, _described(_name, **_params)),
                  {"_spec_name": _name.lower(), "__qualname__": f"BoxStyle.{_name}"})
    setattr(BoxStyle, _name, _register_style(BoxStyle._style_list, _style))


class ConnectionStyle(_Style):
    """
    `ConnectionStyle` is a container class which defines several
    connectionstyle classes, which is used to create a path between two
    points.  These are mainly used with `FancyArrowPatch`.

    An instance of a connection style is callable: ``style(posA, posB)``
    returns the `.Path` connecting the points.
    """

    _style_list = {}

    class _Base(_Described):
        def __call__(self, posA, posB, shrinkA=2., shrinkB=2., patchA=None,
                     patchB=None):
            """Return the path connecting *posA* and *posB*."""
            return Path(*_styles.connection_path(
                self._spec(), tuple(map(float, posA)), tuple(map(float, posB))))


for _name, _params in [
        ("Arc3", dict(rad=0.)),
        ("Angle3", dict(angleA=90, angleB=0)),
        ("Angle", dict(angleA=90, angleB=0, rad=0.)),
        ("Arc", dict(angleA=0, angleB=0, armA=None, armB=None, rad=0.)),
        ("Bar", dict(armA=0., armB=0., fraction=0.3, angle=None))]:
    _style = type(_name, (ConnectionStyle._Base, _described(_name, **_params)),
                  {"_spec_name": _name.lower(),
                   "__qualname__": f"ConnectionStyle.{_name}"})
    setattr(ConnectionStyle, _name,
            _register_style(ConnectionStyle._style_list, _style))


class ArrowStyle(_Style):
    """
    `ArrowStyle` is a container class which defines several
    arrowstyle classes, which is used to create an arrow path along a
    given path.  These are mainly used with `FancyArrowPatch`.

    An instance of an arrow style is callable: ``style(path,
    mutation_size, linewidth, aspect_ratio)`` returns the arrow's path and
    whether it is filled.
    """

    _style_list = {}

    class _Base(_Described):
        def __call__(self, path, mutation_size, linewidth,
                     aspect_ratio=1.):
            """
            Return the paths of the arrow along *path* and whether each is
            filled (one path and a bool for a one-piece arrow).
            """
            pieces = _styles.arrow_paths(self._spec(), path, float(mutation_size),
                                         float(linewidth), float(aspect_ratio))
            paths = [Path(*arrays) for arrays, _ in pieces]
            fillables = [filled for _, filled in pieces]
            if len(paths) == 1:
                return paths[0], fillables[0]
            return paths, fillables


_CURVE = dict(head_length=.4, head_width=.2)
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
    _style = type(_name, (ArrowStyle._Base, _described(_name, **_params)),
                  {"_spec_name": _arrow, "arrow": _arrow,
                   "__qualname__": f"ArrowStyle.{_name}"})
    setattr(ArrowStyle, _name, _register_style(ArrowStyle._style_list, _style,
                                               name=_arrow))
del _name, _arrow, _params, _style


def _native_spec(style, registry):
    """
    The description string of *style* for the native side: a style
    instance of *registry*, or a style string (validated by *registry*).
    """
    if isinstance(style, str):
        return registry(style)._spec()
    if isinstance(style, _Described):
        return style._spec()
    raise TypeError(f"{style!r} is not a {registry.__name__}")


__all__ = ["ArrowStyle", "BoxStyle", "ConnectionStyle"]
