"""
The resources of a `.PdfFile` (marker and path forms, shadings, images,
hatch patterns, alpha states, fonts) as matplotlib's keeps them: the
values the program gives, unchecked, until the document writes them.

pltrs (Rust) holds and writes the resources it can draw. One it cannot (a
line width that is no number, a join style that is no name, triangles that
are no arrays, a font file it cannot read) holds a stand-in under the name
the document gives it. Writing the resources first runs matplotlib's
arithmetic over every one given, in order (`check_markers`,
`check_templates`, `check_shadings`, `check_images`, `check_hatches`,
`check_fonts`), so that the document fails where matplotlib's does, with
its error and its numpy warnings. Ported from matplotlib 3.11.2's
``backends/backend_pdf.py`` under the Matplotlib License.
"""

from io import BytesIO
from numbers import Integral, Real

import numpy as np

from .. import _pybind, rcParams
from ..font_manager import get_font
from ..ft2font import LoadFlags
from ..path import Path
from ..transforms import Affine2D
from ._pdf_arguments import check_finite

#: The join and cap styles as matplotlib's `GraphicsContextPdf` numbers them.
JOINSTYLES = {'miter': 0, 'round': 1, 'bevel': 2}
CAPSTYLES = {'butt': 0, 'round': 1, 'projecting': 2}


class Given:
    """What the program gave a `.PdfFile`'s resource methods, in order."""

    def __init__(self):
        #: ``(bbox, lw, joinstyle, capstyle)`` of each marker.
        self.markers = []
        #: ``(path, trans, padding, joinstyle, capstyle)`` of each path
        #: template.
        self.templates = []
        #: ``(points, colors)`` of each shading.
        self.shadings = []
        #: The name of each alpha state, by the value given.
        self.alpha_states = {}
        #: The name of each hatch pattern, by the style given.
        self.hatches = {}
        #: The name of each font pltrs (Rust) cannot embed, by its
        #: ``(FontPath, subset)``.
        self.fonts = {}
        #: Whether the program named fonts itself (``fontName``).
        self.named_fonts = False


def _plain_trans(trans):
    """Whether pltrs (Rust) draws under *trans*: None (the identity) or
    a 3x3 matrix."""
    try:
        return _pybind.trans_affine(trans)
    except ValueError:
        return False


def plain_marker(trans, lw, joinstyle, capstyle):
    """Whether pltrs (Rust) holds a marker of these values."""
    return (_plain_trans(trans) and isinstance(lw, Real)
            and _style(joinstyle, JOINSTYLES) and _style(capstyle, CAPSTYLES))


def plain_template(path, trans, padding):
    """Whether pltrs (Rust) holds a path template of these values."""
    return (isinstance(path, Path) and _plain_trans(trans)
            and isinstance(padding, Real))


def plain_shading(points, colors):
    """Whether pltrs (Rust) holds the shading of *points* and *colors*:
    arrays of numbers of shapes ``(n, 3, 2)`` and ``(n, 3, 1 or 4)``."""
    return (all(isinstance(array, np.ndarray) and array.dtype.kind in 'biuf'
                for array in (points, colors))
            and points.ndim == colors.ndim == 3 and len(points) == len(colors) > 0
            and points.shape[1:] == (3, 2) and colors.shape[1] == 3
            and colors.shape[2] in (1, 4))


def plain_image(image):
    """Whether pltrs (Rust) holds *image*: bytes of grey, RGB or RGBA."""
    return (isinstance(image, np.ndarray) and image.dtype == np.uint8
            and image.size > 0
            and (image.ndim == 2 or (image.ndim == 3 and image.shape[2] in (3, 4))))


def plain_hatch(style):
    """Whether pltrs (Rust) holds the hatch pattern *style*: its colours
    three or four numbers each (or None for the face), its hatch a string
    and its line width a number."""
    edge, face, hatch, lw = style
    return (_color(edge) and (face is None or _color(face))
            and isinstance(hatch, str) and isinstance(lw, Real))


def plain_alphas(alpha):
    """Whether pltrs (Rust) holds the alpha state *alpha*: its first two
    values numbers."""
    return isinstance(alpha[0], Real) and isinstance(alpha[1], Real)


def plain_subset(subset):
    """The subset *subset* as pltrs (Rust) holds it (a count; a whole
    number stands for the same count, as matplotlib's dictionary of names
    finds it), else None."""
    if isinstance(subset, bool) or not isinstance(subset, Real):
        return None
    if isinstance(subset, Integral):
        return int(subset) if subset >= 0 else None
    return int(subset) if np.isfinite(subset) and subset == int(subset) >= 0 else None


def _color(color):
    """Whether *color* is three or four numbers."""
    return (isinstance(color, (tuple, list, np.ndarray)) and len(color) in (3, 4)
            and all(isinstance(value, Real) for value in color))


def _style(style, styles):
    """Whether *style* names one of *styles*."""
    try:
        return style in styles
    except TypeError:  # an unhashable style
        return False


def check_markers(file, markers):
    """matplotlib's ``writeMarkers`` over the *markers* given, short of
    writing them."""
    for bbox, lw, joinstyle, capstyle in markers:
        bbox = bbox.padded(lw * 5)
        assert file.currentstream is None
        check_finite(*bbox.extents)
        JOINSTYLES[joinstyle]
        CAPSTYLES[capstyle]


def check_templates(file, templates):
    """matplotlib's ``writePathCollectionTemplates`` over the path
    *templates* given, short of writing them."""
    for path, trans, padding, joinstyle, capstyle in templates:
        file.pathOperations(path, trans, simplify=False)
        bbox = path.get_extents(trans)
        if not np.all(np.isfinite(bbox.extents)):
            extents = [0, 0, 0, 0]
        else:
            extents = list(bbox.padded(padding).extents)
        assert file.currentstream is None
        check_finite(*extents)
        JOINSTYLES[joinstyle]
        CAPSTYLES[capstyle]


def check_shadings(file, shadings):
    """matplotlib's ``writeGouraudTriangles`` over the *shadings* given,
    short of writing them (its numpy warnings issued)."""
    for points, colors in shadings:
        shape = points.shape
        flat_points = points.reshape((shape[0] * shape[1], 2))
        colordim = colors.shape[2]
        assert colordim in (1, 4)
        flat_colors = colors.reshape((shape[0] * shape[1], colordim))
        if colordim == 4:
            # strip the alpha channel
            colordim = 3
        points_min = np.min(flat_points, axis=0) - (1 << 8)
        points_max = np.max(flat_points, axis=0) + (1 << 8)
        factor = 0xffffffff / (points_max - points_min)
        assert file.currentstream is None
        check_finite(points_min[0], points_max[0], points_min[1], points_max[1])
        streamarr = np.empty(
            (shape[0] * shape[1],),
            dtype=[('flags', 'u1'), ('points', '>u4', (2,)),
                   ('colors', 'u1', (colordim,))])
        streamarr['points'] = (flat_points - points_min) * factor
        streamarr['colors'] = flat_colors[:, :colordim] * 255.0


def check_images(file, images):
    """matplotlib's ``writeImages`` over the *images* given, short of
    writing them."""
    for image in images:
        data, adata = _unpack(image)
        if adata is not None:
            _check_image(file, adata)
        _check_image(file, data)


def _unpack(im):
    """matplotlib's ``_unpack``: the image *im* as ``(data, alpha)``."""
    im = im[::-1]
    if im.ndim == 2:
        return im, None
    rgb = np.array(im[:, :, :3], order='C')
    if im.shape[2] == 4:
        alpha = im[:, :, 3][..., None]
        alpha = None if np.all(alpha == 255) else np.array(alpha, order='C')
    else:
        alpha = None
    return rgb, alpha


def _check_image(file, data):
    """matplotlib's ``_writeImg`` of *data*, short of writing it: its
    shape and colour space read and, compressed, its pixels encoded by
    Pillow."""
    _height, _width, color_channels = data.shape
    {1: 'DeviceGray', 3: 'DeviceRGB'}[color_channels]  # noqa: B018
    if rcParams['pdf.compression']:
        from PIL import Image
        if data.shape[-1] == 1:
            data = data.squeeze(axis=-1)
        img = Image.fromarray(data)
        img_colors = img.getcolors(maxcolors=256)
        if color_channels == 3 and img_colors is not None:
            palette = np.array([comp for _, color in img_colors for comp in color],
                               dtype=np.uint8)
            palette24 = ((palette[0::3].astype(np.uint32) << 16) |
                         (palette[1::3].astype(np.uint32) << 8) |
                         palette[2::3])
            rgb24 = ((data[:, :, 0].astype(np.uint32) << 16) |
                     (data[:, :, 1].astype(np.uint32) << 8) |
                     data[:, :, 2])
            indices = np.argsort(palette24).astype(np.uint8)
            rgb8 = indices[np.searchsorted(palette24, rgb24, sorter=indices)]
            img = Image.fromarray(rgb8).convert("P")
            img.putpalette(palette)
        img.save(BytesIO(), format="png")
    assert file.currentstream is None


def check_hatches(file, hatches):
    """matplotlib's ``writeHatches`` over the hatch styles given, short of
    writing them."""
    from .backend_pdf import _pdf_object
    sidelen = 72.0
    for stroke_rgb, fill_rgb, hatch, lw in hatches:
        assert file.currentstream is None
        _pdf_object(file.height * 72)
        for value in (stroke_rgb[0], stroke_rgb[1], stroke_rgb[2]):
            _pdf_object(value)
        if fill_rgb is not None:
            for value in (fill_rgb[0], fill_rgb[1], fill_rgb[2]):
                _pdf_object(value)
        _pdf_object(lw)
        file.pathOperations(Path.hatch(hatch), Affine2D().scale(sidelen),
                            simplify=False)


def check_embedding(file, filename, charmap):
    """What matplotlib's ``embedTTF`` reads of the font *filename* and the
    character map *charmap* before it writes its first stream (a stream
    already open fails there)."""
    font = get_font(filename)
    frozenset(charmap.values())  # the subset's name hashes the glyphs
    if rcParams['pdf.fonttype'] == 3:
        firstchar, lastchar = min(charmap), max(charmap)
        for charcode in range(firstchar, lastchar + 1):
            font.load_glyph(charmap.get(charcode, 0),
                            flags=LoadFlags.NO_SCALE | LoadFlags.NO_HINTING)
        sorted([(ccode, font.get_glyph_name(gind)) for ccode, gind in charmap.items()])
    assert file.currentstream is None


def check_fonts(names, tracks_font):
    """matplotlib's ``writeFonts`` over the fonts *names* holds (``(FontPath,
    subset)`` to name), short of embedding them: each TrueType font is
    embedded with the characters its tracker holds for it (*tracks_font*
    tells whether it holds any), which fails for a font never drawn."""
    for (filename, subset), _name in sorted(names.items()):
        if filename.path.endswith('.afm'):
            continue
        if not tracks_font(filename):
            raise KeyError(filename)
        if plain_subset(subset) is None:
            raise KeyError(subset)
