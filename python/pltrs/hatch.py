"""
Contains classes for generating hatch patterns (matplotlib's ``hatch``).

pltrs (Rust) generates the hatch paths (``pltrs_core::hatch``, through
``_native.path.hatch``); the validation of patterns and the structure of
the pattern classes are matplotlib's, each class's vertices made by
pltrs (Rust) (``_native.hatch``).
"""

import numpy as np

from . import _api
from ._native import hatch as _hatch
from .path import Path


def _write(piece, vertices, codes):
    """Write a piece of hatch made by pltrs (Rust) into the caller's
    *vertices* and *codes* arrays, as ``set_vertices_and_codes`` does."""
    piece_vertices, piece_codes = piece
    vertices[...] = piece_vertices
    codes[...] = piece_codes


class HatchPatternBase:
    """The base class for a hatch pattern."""
    pass


class _Lines(HatchPatternBase):
    """
    Straight hatch lines, the family ``_family`` of pltrs (Rust)'s
    ``HatchLines``, for the pattern characters ``_characters``.
    """

    _family = None
    _characters = ()

    def __init__(self, hatch, density):
        self.num_lines = int(sum(hatch.count(c) for c in self._characters)
                             * density)
        self.num_vertices = _hatch.line_vertex_count(self._family, self.num_lines)

    def set_vertices_and_codes(self, vertices, codes):
        _write(_hatch.line_piece(self._family, self.num_lines), vertices, codes)


class HorizontalHatch(_Lines):
    _family = "horizontal"
    _characters = ('-', '+')


class VerticalHatch(_Lines):
    _family = "vertical"
    _characters = ('|', '+')


class NorthEastHatch(_Lines):
    _family = "north_east"
    _characters = ('/', 'x', 'X')


class SouthEastHatch(_Lines):
    _family = "south_east"
    _characters = ('\\', 'x', 'X')


class Shapes(HatchPatternBase):
    filled = False

    def __init__(self, hatch, density):
        if self.num_rows == 0:
            self.num_shapes = 0
            self.num_vertices = 0
        else:
            self.num_shapes = _hatch.shape_count(self.num_rows)
            self.num_vertices = (self.num_shapes *
                                 len(self.shape_vertices) *
                                 (1 if self.filled else 2))

    def set_vertices_and_codes(self, vertices, codes):
        shape = Path(self.shape_vertices, self.shape_codes)
        _write(_hatch.shape_piece(shape, (float(self.size), bool(self.filled)),
                                  self.num_rows),
               vertices, codes)


class Circles(Shapes):
    def __init__(self, hatch, density):
        path = Path.unit_circle()
        self.shape_vertices = path.vertices
        self.shape_codes = path.codes
        super().__init__(hatch, density)


class SmallCircles(Circles):
    size = 0.2

    def __init__(self, hatch, density):
        self.num_rows = (hatch.count('o')) * density
        super().__init__(hatch, density)


class LargeCircles(Circles):
    size = 0.35

    def __init__(self, hatch, density):
        self.num_rows = (hatch.count('O')) * density
        super().__init__(hatch, density)


class SmallFilledCircles(Circles):
    size = 0.1
    filled = True

    def __init__(self, hatch, density):
        self.num_rows = (hatch.count('.')) * density
        super().__init__(hatch, density)


class Stars(Shapes):
    size = 1.0 / 3.0
    filled = True

    def __init__(self, hatch, density):
        self.num_rows = (hatch.count('*')) * density
        path = Path.unit_regular_star(5)
        self.shape_vertices = path.vertices
        self.shape_codes = np.full(len(self.shape_vertices), Path.LINETO,
                                   dtype=Path.code_type)
        self.shape_codes[0] = Path.MOVETO
        self.shape_codes[-1] = Path.CLOSEPOLY
        super().__init__(hatch, density)


_hatch_types = [
    HorizontalHatch,
    VerticalHatch,
    NorthEastHatch,
    SouthEastHatch,
    SmallCircles,
    LargeCircles,
    SmallFilledCircles,
    Stars
    ]


def _validate_hatch_pattern(hatch):
    valid_hatch_patterns = set(r'-+|/\xXoO.*')
    if hatch is not None:
        invalids = set(hatch).difference(valid_hatch_patterns)
        if invalids:
            valid = ''.join(sorted(valid_hatch_patterns))
            invalids = ''.join(sorted(invalids))
            _api.warn_deprecated(
                '3.4',
                removal='3.13',  # one release after custom hatches (#20690)
                message=f'hatch must consist of a string of "{valid}" or '
                        'None, but found the following invalid values '
                        f'"{invalids}". Passing invalid values is deprecated '
                        'since %(since)s and will become an error in %(removal)s.'
            )


def get_path(hatchpattern, density=6):
    """
    Given a hatch specifier, *hatchpattern*, generates Path to render
    the hatch in a unit square.  *density* is the number of lines per
    unit square.
    """
    return Path.hatch(hatchpattern, density)
