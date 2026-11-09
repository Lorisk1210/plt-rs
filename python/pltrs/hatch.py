"""
Contains classes for generating hatch patterns (matplotlib's ``hatch``).

pltrs (Rust) generates the hatch paths `.Path.hatch` draws
(``pltrs_core::hatch``, through ``_native.path.hatch``); the validation
of patterns, the structure of the pattern classes and their counts of
lines and vertices are matplotlib's, each class's vertices made by
pltrs (Rust) (``_native.hatch``), and `get_path` assembles them as
matplotlib's does.
"""

import numpy as np

from . import _api
from ._native import hatch as _hatch, path as _native_path
from .path import Path


def _write_lines(piece, vertices, codes):
    """Write hatch lines made by pltrs (Rust) into the caller's *vertices*
    and *codes* arrays as matplotlib's ``set_vertices_and_codes`` does,
    each line's start and end apart (so that arrays of another size fail
    as they fail there)."""
    piece_vertices, piece_codes = piece
    for end in (0, 1):
        vertices[end::2, 0] = piece_vertices[end::2, 0]
        vertices[end::2, 1] = piece_vertices[end::2, 1]
    for end in (0, 1):
        codes[end::2] = piece_codes[end::2]


def _write_shapes(piece, vertices, codes):
    """Write hatch shapes made by pltrs (Rust) into the caller's
    *vertices* and *codes* arrays as matplotlib's
    ``set_vertices_and_codes`` does (``np.concatenate`` into them)."""
    piece_vertices, piece_codes = piece
    np.concatenate([piece_vertices], out=vertices)
    np.concatenate([piece_codes], out=codes)


class HatchPatternBase:
    """The base class for a hatch pattern."""
    pass


def _count_lines(hatch, characters, density):
    """The number of lines of the line family the pattern *hatch* draws
    for each of its *characters*, *density* lines each."""
    return int(sum(hatch.count(c) for c in characters) * density)


def _diagonal_vertices(num_lines):
    """The number of vertices of *num_lines* diagonals, which run through
    both corners (one line more), as matplotlib counts them."""
    return (num_lines + 1) * 2 if num_lines else 0


def _write_family(family, num_lines, vertices, codes):
    """Write the *num_lines* lines of the *family* (of pltrs (Rust)'s
    ``HatchLines``) into *vertices* and *codes*."""
    if num_lines < 0:
        # matplotlib spaces the lines (the diagonals' ends, one more) with
        # numpy, which refuses a negative count.
        np.linspace(0.0, 1.0, num_lines + (family in ("north_east", "south_east")))
    _write_lines(_hatch.line_piece(family, num_lines), vertices, codes)


class HorizontalHatch(HatchPatternBase):
    def __init__(self, hatch, density):
        self.num_lines = _count_lines(hatch, ('-', '+'), density)
        self.num_vertices = self.num_lines * 2

    def set_vertices_and_codes(self, vertices, codes):
        _write_family("horizontal", self.num_lines, vertices, codes)


class VerticalHatch(HatchPatternBase):
    def __init__(self, hatch, density):
        self.num_lines = _count_lines(hatch, ('|', '+'), density)
        self.num_vertices = self.num_lines * 2

    def set_vertices_and_codes(self, vertices, codes):
        _write_family("vertical", self.num_lines, vertices, codes)


class NorthEastHatch(HatchPatternBase):
    def __init__(self, hatch, density):
        self.num_lines = _count_lines(hatch, ('/', 'x', 'X'), density)
        self.num_vertices = _diagonal_vertices(self.num_lines)

    def set_vertices_and_codes(self, vertices, codes):
        _write_family("north_east", self.num_lines, vertices, codes)


class SouthEastHatch(HatchPatternBase):
    def __init__(self, hatch, density):
        self.num_lines = _count_lines(hatch, ('\\', 'x', 'X'), density)
        self.num_vertices = _diagonal_vertices(self.num_lines)

    def set_vertices_and_codes(self, vertices, codes):
        _write_family("south_east", self.num_lines, vertices, codes)


class Shapes(HatchPatternBase):
    filled = False

    def __init__(self, hatch, density):
        if self.num_rows == 0:
            self.num_shapes = 0
            self.num_vertices = 0
        else:
            self.num_shapes = ((self.num_rows // 2 + 1) * (self.num_rows + 1) +
                               (self.num_rows // 2) * self.num_rows)
            self.num_vertices = (self.num_shapes *
                                 len(self.shape_vertices) *
                                 (1 if self.filled else 2))

    def set_vertices_and_codes(self, vertices, codes):
        if self.num_rows < 0:
            # No rows: matplotlib concatenates none.
            np.concatenate([], out=vertices)
        shape = Path(self.shape_vertices, self.shape_codes)
        _write_shapes(
            _hatch.shape_piece(shape, (float(self.size), bool(self.filled)),
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
    density = int(density)
    if isinstance(hatchpattern, str) and density >= 0:
        return Path(*_native_path.hatch(hatchpattern, density))
    # matplotlib's assembly, which fails as it does on what is no pattern
    # or on a negative density.
    patterns = [hatch_type(hatchpattern, density)
                for hatch_type in _hatch_types]
    num_vertices = sum([pattern.num_vertices for pattern in patterns])

    if num_vertices == 0:
        return Path(np.empty((0, 2)))

    vertices = np.empty((num_vertices, 2))
    codes = np.empty(num_vertices, Path.code_type)

    cursor = 0
    for pattern in patterns:
        if pattern.num_vertices != 0:
            vertices_chunk = vertices[cursor:cursor + pattern.num_vertices]
            codes_chunk = codes[cursor:cursor + pattern.num_vertices]
            pattern.set_vertices_and_codes(vertices_chunk, codes_chunk)
            cursor += pattern.num_vertices

    return Path(vertices, codes)
