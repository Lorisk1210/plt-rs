"""
The PDF backend (matplotlib's ``backend_pdf``): `FigureCanvasPdf`, whose
``print_pdf`` writes the figure as a PDF document, `PdfPages`, a document
of several figures, `RendererPdf`, with which the figure is laid out for
its document, and the objects of matplotlib's PDF writer (`PdfFile`,
`pdfRepr`, `Name`, `Op`, `Reference`, `Stream`, `Verbatim`,
`GraphicsContextPdf`). pltrs (Rust) writes the documents.

The document information helpers (`_create_pdf_info_dict`,
`_datetime_to_pdf`), which the PGF backend shares, the `Op` table and
`GraphicsContextPdf` are matplotlib's (from matplotlib 3.11.2, under the
Matplotlib License).
"""

import datetime
from enum import Enum
from functools import total_ordering
import logging
import os
import time
import types

import numpy as np

from .. import __version__, _api, _path, cbook, dviread, rcParams
from ..colors import to_rgba
from .._native import backends as _backends, text_overrides as _text_overrides
from .._pylab_helpers import Gcf
from ..backend_bases import (
    _Backend, _write_bytes, DrawEvent, FigureCanvasBase, FigureManagerBase,
    GraphicsContextBase)
from ..dates import UTC
from ..figure import Figure
from ..font_manager import (
    _data_font_path, FontManager, FontPath, fontManager as _fontManager)
from ..text import _measuring_with
from ..transforms import BboxBase
from ._backend_pdf_ps import RendererPDFPSBase
from ._native_drawing import DocumentDrawingMixin, PdfPsDrawingMixin, native_gc
from .backend_mixed import MixedModeRenderer

_log = logging.getLogger(__name__)


def _create_pdf_info_dict(backend, metadata):
    """
    Create a PDF infoDict based on user-supplied metadata.

    A default ``Creator``, ``Producer``, and ``CreationDate`` are added, though
    the user metadata may override it. The date may be the current time, or a
    time set by the ``SOURCE_DATE_EPOCH`` environment variable.

    Metadata is verified to have the correct keys and their expected types. Any
    unknown keys/types will raise a warning.

    Parameters
    ----------
    backend : str
        The name of the backend to use in the Producer value.

    metadata : dict[str, Union[str, datetime]]
        A dictionary of metadata supplied by the user with information
        following the PDF specification. If any value is *None*, then the key
        will be removed. This can be used to remove any pre-defined values.

    Returns
    -------
    dict[str, Union[str, datetime]]
        A validated dictionary of metadata.
    """
    source_date_epoch = os.getenv("SOURCE_DATE_EPOCH")
    if source_date_epoch:
        source_date = datetime.datetime.fromtimestamp(
            int(source_date_epoch), datetime.timezone.utc)
    else:
        source_date = datetime.datetime.today()

    creator = _backends.default_creator()
    info = {
        'Creator': f'{creator}, matplotlib {__version__} API',
        'Producer': f'{creator} {backend} backend',
        'CreationDate': source_date,
        **metadata
    }
    info = {k: v for (k, v) in info.items() if v is not None}
    _check_info(info)
    return info


def _is_string_like(x):
    return isinstance(x, str)


_is_string_like.text_for_warning = "an instance of str"


def _is_date(x):
    return isinstance(x, datetime.datetime)


_is_date.text_for_warning = "an instance of datetime.datetime"


def _check_trapped(x):
    return x in ('True', 'False', 'Unknown')


_check_trapped.text_for_warning = 'one of {"True", "False", "Unknown"}'

#: The checks of the keys of a document information dictionary.
_INFO_CHECKS = {
    'Title': _is_string_like,
    'Author': _is_string_like,
    'Subject': _is_string_like,
    'Keywords': _is_string_like,
    'Creator': _is_string_like,
    'Producer': _is_string_like,
    'CreationDate': _is_date,
    'ModDate': _is_date,
    'Trapped': _check_trapped,
}


def _check_info(info):
    """Warn about the unknown keys and the values of the wrong type of the
    information dictionary *info*; return the keys that pass."""
    valid = []
    for k in info:
        if k not in _INFO_CHECKS:
            _api.warn_external(f'Unknown infodict keyword: {k!r}. '
                               f'Must be one of {set(_INFO_CHECKS)!r}.')
        elif not _INFO_CHECKS[k](info[k]):
            _api.warn_external(f'Bad value for infodict keyword {k}. '
                               f'Got {info[k]!r} which is not '
                               f'{_INFO_CHECKS[k].text_for_warning}.')
        else:
            valid.append(k)
    return valid


def _native_info(metadata):
    """The document *metadata* of a PDF document for pltrs (Rust), checked
    as matplotlib checks it: the entries it warns about are left out, dates
    are seconds since the epoch and None removes an entry."""
    if metadata is None:
        return None
    given = {k: v for k, v in metadata.items() if v is not None}
    native = {k: None for k, v in metadata.items() if v is None}
    for key in _check_info(given):
        value = given[key]
        native[key] = (int(value.timestamp())
                       if isinstance(value, datetime.datetime)
                       else value)
    return native


def _datetime_to_pdf(d):
    """
    Convert a datetime to a PDF string representing it.

    Used for PDF and PGF.
    """
    r = d.strftime('D:%Y%m%d%H%M%S')
    z = d.utcoffset()
    if z is not None:
        z = z.seconds
    else:
        if time.daylight:
            z = time.altzone
        else:
            z = time.timezone
    if z == 0:
        r += 'Z'
    elif z < 0:
        r += "+%02d'%02d'" % ((-z) // 3600, (-z) % 3600)
    else:
        r += "-%02d'%02d'" % (z // 3600, z % 3600)
    return r


def _pdf_object(obj):
    """
    *obj* as pltrs (Rust) takes PDF objects: ``(kind, value)``, the kinds
    of the Python values `pdfRepr` writes.
    """
    if isinstance(obj, Reference):
        return ('ref', obj.id)
    elif isinstance(obj, Name):
        return ('name', obj.name.decode('ascii'))
    elif hasattr(obj, 'pdfRepr'):
        return ('raw', obj.pdfRepr())
    elif isinstance(obj, (float, np.floating)):
        return ('real', float(obj))
    elif isinstance(obj, bool):
        return ('bool', obj)
    elif isinstance(obj, (int, np.integer)):
        return ('int', int(obj))
    elif isinstance(obj, str):
        return ('text', obj)
    elif isinstance(obj, bytes):
        return ('bytes', obj)
    elif isinstance(obj, dict):
        return ('dict', _pdf_entries(obj))
    elif isinstance(obj, (list, tuple)):
        return ('array', [_pdf_object(val) for val in obj])
    elif obj is None:
        return ('null', None)
    elif isinstance(obj, datetime.datetime):
        return ('text', _datetime_to_pdf(obj))
    elif isinstance(obj, BboxBase):
        return ('seq', [_pdf_object(val) for val in obj.bounds])
    else:
        raise TypeError(f"Don't know a PDF representation for {type(obj)} "
                        "objects")


def _pdf_entries(mapping):
    """The entries of the dictionary *mapping*, its keys as PDF names."""
    return [(Name(k).name.decode('ascii'), _pdf_object(v))
            for k, v in mapping.items()]


def pdfRepr(obj):
    """Map Python objects to PDF syntax."""
    return _backends.pdf_repr(_pdf_object(obj))


class Reference:
    """
    PDF reference object.

    Use PdfFile.reserveObject() to create References.
    """

    def __init__(self, id):
        self.id = id

    def __repr__(self):
        return "<Reference %d>" % self.id

    def pdfRepr(self):
        return pdfRepr(self)

    def write(self, contents, file):
        write = file.write
        write(b"%d 0 obj\n" % self.id)
        write(pdfRepr(contents))
        write(b"\nendobj\n")


@total_ordering
class Name:
    """PDF name object."""
    __slots__ = ('name',)

    def __init__(self, name):
        if isinstance(name, Name):
            self.name = name.name
        else:
            if isinstance(name, bytes):
                name = name.decode('ascii')
            # pltrs (Rust) writes the bytes outside ! to ~ as #xx.
            self.name = _backends.pdf_repr(('name', name))[1:]

    def __repr__(self):
        return "<Name %s>" % self.name

    def __str__(self):
        return '/' + self.name.decode('ascii')

    def __eq__(self, other):
        return isinstance(other, Name) and self.name == other.name

    def __lt__(self, other):
        return isinstance(other, Name) and self.name < other.name

    def __hash__(self):
        return hash(self.name)

    def pdfRepr(self):
        return b'/' + self.name


class Verbatim:
    """Store verbatim PDF command content for later inclusion in the stream."""
    def __init__(self, x):
        self._x = x

    def pdfRepr(self):
        return self._x


class Op(Enum):
    """PDF operators (not an exhaustive list)."""

    close_fill_stroke = b'b'
    fill_stroke = b'B'
    fill = b'f'
    closepath = b'h'
    close_stroke = b's'
    stroke = b'S'
    endpath = b'n'
    begin_text = b'BT'
    end_text = b'ET'
    curveto = b'c'
    rectangle = b're'
    lineto = b'l'
    moveto = b'm'
    concat_matrix = b'cm'
    use_xobject = b'Do'
    setgray_stroke = b'G'
    setgray_nonstroke = b'g'
    setrgb_stroke = b'RG'
    setrgb_nonstroke = b'rg'
    setcolorspace_stroke = b'CS'
    setcolorspace_nonstroke = b'cs'
    setcolor_stroke = b'SCN'
    setcolor_nonstroke = b'scn'
    setdash = b'd'
    setlinejoin = b'j'
    setlinecap = b'J'
    setgstate = b'gs'
    gsave = b'q'
    grestore = b'Q'
    textpos = b'Td'
    selectfont = b'Tf'
    textmatrix = b'Tm'
    textrise = b'Ts'
    show = b'Tj'
    showkern = b'TJ'
    setlinewidth = b'w'
    clip = b'W'
    shading = b'sh'

    def pdfRepr(self):
        return self.value

    @classmethod
    def paint_path(cls, fill, stroke):
        """
        Return the PDF operator to paint a path.

        Parameters
        ----------
        fill : bool
            Fill the path with the fill color.
        stroke : bool
            Stroke the outline of the path with the line color.
        """
        if stroke:
            if fill:
                return cls.fill_stroke
            else:
                return cls.stroke
        else:
            if fill:
                return cls.fill
            else:
                return cls.endpath


class Stream:
    """
    PDF stream object.

    This has no pdfRepr method. Instead, call begin(), then output the
    contents of the stream by calling write(), and finally call end().

    pltrs (Rust) keeps the stream's data and writes the stream whole when it
    ends, compressed at :rc:`pdf.compression`: *file* is the `PdfFile` the
    data goes to, and the stream keeps no compression object or position of
    its own (``compressobj`` and ``pos`` are None).
    """
    __slots__ = ('id', 'len', 'pdfFile', 'file', 'compressobj', 'extra', 'pos')

    def __init__(self, id, len, file, extra=None, png=None):
        """
        Parameters
        ----------
        id : int
            Object id of the stream.
        len : Reference or None
            An unused Reference object for the length of the stream;
            None means to use a memory buffer so the length can be inlined.
        file : PdfFile
            The underlying object to write the stream to.
        extra : dict from Name to anything, or None
            Extra key-value pairs to include in the stream header.
        png : dict or None
            If the data is already png encoded, the decode parameters.
        """
        self._set(id, len, file, extra, png)
        _backends.pdf_file_begin_stream(
            file._native, id, None if len is None else len.id,
            _pdf_entries(extra or {}),
            None if png is None else _pdf_object(png))

    @classmethod
    def _open(cls, id, file):
        """The stream *id* of *file*, open already."""
        stream = cls.__new__(cls)
        stream._set(id, None, file, None, None)
        return stream

    def _set(self, id, len, file, extra, png):
        """Set the attributes of the stream."""
        self.id = id
        self.len = len
        self.pdfFile = file
        self.file = file
        self.compressobj = None
        self.extra = {} if extra is None else extra.copy()
        if png is not None:
            self.extra.update({'Filter': Name('FlateDecode'),
                               'DecodeParms': png})
        self.pos = None

    def end(self):
        """Finalize stream."""
        _backends.pdf_file_end_stream(self.pdfFile._native)

    def write(self, data):
        """Write some data on the stream."""
        _backends.pdf_file_write(self.pdfFile._native, data)


def _info_of(entries):
    """
    The document information pltrs (Rust) writes, as ``PdfFile.infoDict``
    holds it: text, and the dates in UTC.
    """
    return {key: text if date is None else
            datetime.datetime.fromtimestamp(date, datetime.timezone.utc)
            for key, text, date in entries}


def _native_info_of(info):
    """
    The entries of the information dictionary *info* pltrs (Rust) writes:
    text, names (``Trapped``) and dates (as seconds since the epoch).
    """
    native = {}
    for key, value in info.items():
        if isinstance(value, datetime.datetime):
            native[key] = int(value.timestamp())
        elif isinstance(value, Name):
            native[key] = value.name.decode('ascii')
        elif isinstance(value, str):
            native[key] = value
    return native


def _rgba_pixels(image):
    """The pixels of *image* (grey, RGB or RGBA bytes) as RGBA bytes."""
    image = np.asarray(image)
    if image.ndim == 2:
        image = np.repeat(image[..., None], 3, axis=2)
    if image.shape[2] == 3:
        opaque = np.full(image.shape[:2] + (1,), 255, image.dtype)
        image = np.concatenate([image, opaque], axis=2)
    return np.ascontiguousarray(image, np.uint8)


def _style_name(style):
    """The name of a join or cap style (an enum member or a string)."""
    return style.name if isinstance(style, Enum) else str(style)


class PdfFile:
    """
    PDF file object.

    pltrs (Rust) writes the document in memory, its objects and streams as
    the methods ask (and its pages as `PdfPages.savefig` draws them);
    *filename* receives it when the file is closed.
    """

    def __init__(self, filename, metadata=None):
        """
        Parameters
        ----------
        filename : str or path-like or file-like
            Output target; if a string, a file will be opened for writing.

        metadata : dict from strings to strings and dates
            Information dictionary object (see PDF reference section 10.2.1
            'Document Information Dictionary'), e.g.:
            ``{'Creator': 'My software', 'Author': 'Me', 'Title': 'Awesome'}``.

            The standard keys are 'Title', 'Author', 'Subject', 'Keywords',
            'Creator', 'Producer', 'CreationDate', 'ModDate', and
            'Trapped'. Values have been predefined for 'Creator', 'Producer'
            and 'CreationDate'. They can be removed by setting them to `None`.
        """
        super().__init__()
        self._filename = filename
        self._native = _backends.pdf_pages_new(_native_info(metadata or {}))
        self.infoDict = _info_of(_backends.pdf_file_info(self._native))
        self._dviFontInfo = {}   # maps pdf names to dvifonts
        self._image_objects = {}  # images by id: (image, name)
        # The names of the objects, for the error of an unwritten one.
        self._object_names = dict(enumerate([
            'the zero object', 'root', 'pages', 'fonts',
            'extended graphics states', 'tiling patterns', 'Gouraud triangles',
            'external objects', 'resources']))

    fontNames = _api.deprecated("3.11")(property(lambda self: self._fontNames))
    multi_byte_charprocs = _api.deprecated("3.11")(property(lambda _: {}))
    type1Descriptors = _api.deprecated("3.11")(property(lambda _: {}))

    @property
    def _fontNames(self):
        """The fonts named so far: ``(FontPath, subset)`` to `Name`."""
        return {
            (FontPath(_data_font_path(fname, 'afm' if core else 'ttf'), index),
             subset): Name(name)
            for fname, index, subset, core, name
            in _backends.pdf_file_font_names(self._native)}

    @property
    def _images(self):
        """The names of the images the document holds so far, one per image
        written (matplotlib keeps them by the images drawn)."""
        return _backends.pdf_pages_images(self._native)

    @_api.deprecated("3.11")
    @property
    def dviFontInfo(self):
        d = {}
        tex_font_map = dviread.PsfontsMap(dviread.find_tex_file('pdftex.map'))
        for pdfname, dvifont in self._dviFontInfo.items():
            psfont = tex_font_map[dvifont.texname]
            if psfont.filename is None:
                raise ValueError(
                    "No usable font file found for {} (TeX: {}); "
                    "the font may lack a Type-1 version"
                    .format(psfont.psname, dvifont.texname))
            d[dvifont.texname] = types.SimpleNamespace(
                dvifont=dvifont,
                pdfname=pdfname,
                fontfile=psfont.filename,
                basefont=psfont.psname,
                encodingfile=psfont.encoding,
                effects=psfont.effects,
            )
        return d

    def newPage(self, width, height):
        self.endStream()
        self.width, self.height = width, height
        _backends.pdf_file_new_page(self._native, width, height)

    def newTextnote(self, text, positionRect=[-100, -100, 0, 0]):
        # Create a new annotation of type text
        x0, y0, x1, y1 = map(float, positionRect)
        _backends.pdf_pages_attach_note(self._native, str(text),
                                        (x0, y0, x1, y1))

    def finalize(self):
        """Write out the various deferred objects and the pdf end matter."""
        _backends.pdf_file_set_info(self._native,
                                    _native_info_of(self.infoDict))
        _backends.pdf_file_finalize(self._native)

    def close(self):
        """Flush all buffers and free all resources."""
        _write_bytes(_backends.pdf_file_close(self._native), self._filename)

    def write(self, data):
        _backends.pdf_file_write(self._native, data)

    def output(self, *data):
        _backends.pdf_file_output(self._native, [_pdf_object(x) for x in data])

    @property
    def currentstream(self):
        """The `Stream` being written, if any."""
        id = _backends.pdf_file_open_stream(self._native)
        return None if id is None else Stream._open(id, self)

    def beginStream(self, id, len, extra=None, png=None):
        assert self.currentstream is None
        Stream(id, len, self, extra, png)

    def endStream(self):
        _backends.pdf_file_end_stream(self._native)

    def outputStream(self, ref, data, *, extra=None):
        self.beginStream(ref.id, None, extra)
        self.currentstream.write(data)
        self.endStream()

    def fontName(self, fontprop, subset=0):
        """
        Select a font based on fontprop and return a name suitable for
        ``Op.selectfont``. If fontprop is a string, it will be interpreted
        as the filename of the font.
        """

        if isinstance(fontprop, FontPath):
            filenames = [fontprop]
        elif isinstance(fontprop, str):
            filenames = [FontPath(fontprop, 0)]
        elif rcParams['pdf.use14corefonts']:
            filenames = _fontManager._find_fonts_by_props(
                fontprop, fontext='afm',
                directory=cbook._get_data_path('fonts/pdfcorefonts'))
        else:
            filenames = _fontManager._find_fonts_by_props(fontprop)
        names = [self._font_name(fname, subset) for fname in filenames]
        # find_fontsprop's first value always adheres to
        # findfont's value, so technically no behaviour change
        return names[0] if names else None

    def _font_name(self, fname, subset):
        """The name of *subset* of the font file *fname*."""
        if str(fname).endswith('.afm'):
            name = _backends.pdf_file_core_font_name(
                self._native, os.path.basename(fname))
        else:
            path = fname if isinstance(fname, FontPath) else FontPath(fname, 0)
            name = _backends.pdf_file_font_name(
                self._native, path.path, path.face_index, subset)
        _log.debug('Assigning font %s (subset %d) = %r', name, subset, fname)
        return Name(name)

    def dviFontName(self, dvifont):
        """
        Given a dvi font object, return a name suitable for Op.selectfont.

        Register the font internally (in ``_dviFontInfo``) if not yet registered.
        """
        pdfname = Name(_backends.pdf_file_dvi_font_name(self._native,
                                                        dvifont._native))
        _log.debug('Assigning font %s = %s (dvi)', pdfname, dvifont.texname)
        self._dviFontInfo[pdfname] = dvifont
        return Name(pdfname)

    def writeFonts(self):
        _backends.pdf_file_write_fonts(self._native)

    @_api.delete_parameter("3.11", "fontfile")
    def createType1Descriptor(self, t1font, fontfile=None):
        # Create and write the font descriptor and the font file
        # of a Type-1 font
        return Reference(
            _backends.pdf_file_type1_descriptor(self._native, t1font._native))

    def embedTTF(self, filename, subset_index, charmap):
        """Embed the TTF font from the named file into the document."""
        path = filename if isinstance(filename, FontPath) else FontPath(filename, 0)
        return Reference(_backends.pdf_file_embed_ttf(
            self._native, path.path, path.face_index, subset_index,
            [(int(code), int(glyph)) for code, glyph in charmap.items()]))

    def alphaState(self, alpha):
        """Return name of an ExtGState that sets alpha to the given value."""
        stroke, fill = alpha
        return Name(_backends.pdf_file_alpha_state(
            self._native, (float(stroke), float(fill))))

    def writeExtGSTates(self):
        _backends.pdf_file_write_ext_gstates(self._native)

    def hatchPattern(self, hatch_style):
        edge, face, hatch, lw = hatch_style
        colors = [None if c is None else [float(v) for v in c]
                  for c in (edge, face)]
        return Name(_backends.pdf_file_hatch_pattern(
            self._native, (*colors, hatch, float(lw))))

    # The patterns by (edge, face, hatch), as matplotlib deprecated them
    # (matplotlib 3.11's unpacks the names of the patterns, and raises
    # TypeError once there is one).
    hatchPatterns = _api.deprecated("3.10")(property(lambda self: {
        (e if e is None else tuple(e), f if f is None else tuple(f), h): Name(n)
        for e, f, h, n in _backends.pdf_file_hatch_patterns(self._native)
    }))

    def writeHatches(self):
        _backends.pdf_file_write_hatches(self._native)

    def addGouraudTriangles(self, points, colors):
        """
        Add a Gouraud triangle shading.

        Parameters
        ----------
        points : np.ndarray
            Triangle vertices, shape (n, 3, 2)
            where n = number of triangles, 3 = vertices, 2 = x, y.
        colors : np.ndarray
            Vertex colors, shape (n, 3, 1) or (n, 3, 4)
            as with points, but last dimension is either (gray,)
            or (r, g, b, alpha).

        Returns
        -------
        Name, Reference
        """
        name, ob = _backends.pdf_file_gouraud(
            self._native, np.asarray(points, float).reshape((-1, 3, 2)),
            np.asarray(colors, float))
        return Name(name), Reference(ob)

    def writeGouraudTriangles(self):
        _backends.pdf_file_write_gouraud(self._native)

    def imageObject(self, image):
        """Return name of an image XObject representing the given image."""

        entry = self._image_objects.get(id(image), None)
        if entry is not None:
            return entry[1]

        name = Name(_backends.pdf_file_image(self._native, _rgba_pixels(image)))
        self._image_objects[id(image)] = (image, name)
        return name

    def writeImages(self):
        _backends.pdf_file_write_images(self._native)

    def markerObject(self, path, trans, fill, stroke, lw, joinstyle,
                     capstyle):
        """Return name of a marker XObject representing the given path."""
        return Name(_backends.pdf_file_marker(
            self._native, path, np.asarray(trans, float), bool(fill),
            bool(stroke), float(lw), _style_name(joinstyle),
            _style_name(capstyle)))

    def writeMarkers(self):
        _backends.pdf_file_write_markers(self._native)

    def pathCollectionObject(self, gc, path, trans, padding, filled, stroked):
        return Name(_backends.pdf_file_path_collection(
            self._native, path, np.asarray(trans, float), float(padding),
            bool(filled), bool(stroked), _style_name(gc.get_joinstyle()),
            _style_name(gc.get_capstyle())))

    def writePathCollectionTemplates(self):
        _backends.pdf_file_write_templates(self._native)

    @staticmethod
    def pathOperations(path, transform, clip=None, simplify=None, sketch=None):
        return [Verbatim(_path.convert_to_string(
            path, transform, clip, simplify, sketch,
            6,
            [Op.moveto.value, Op.lineto.value, b'', Op.curveto.value,
             Op.closepath.value],
            True))]

    def writePath(self, path, transform, clip=False, sketch=None):
        if clip:
            clip = (0.0, 0.0, self.width * 72, self.height * 72)
            simplify = path.should_simplify
        else:
            clip = None
            simplify = False
        cmds = self.pathOperations(path, transform, clip, simplify=simplify,
                                   sketch=sketch)
        self.output(*cmds)

    def reserveObject(self, name=''):
        """
        Reserve an ID for an indirect object.

        The name is used for debugging in case we forget to print out
        the object with writeObject.
        """
        id = _backends.pdf_file_reserve(self._native)
        self._object_names[id] = name
        return Reference(id)

    def recordXref(self, id):
        _backends.pdf_file_record_xref(self._native, id)

    def writeObject(self, object, contents):
        _backends.pdf_file_write_object(self._native, object.id,
                                        _pdf_object(contents))

    def writeXref(self):
        """Write out the xref table."""
        unwritten = _backends.pdf_file_unwritten(self._native)
        if unwritten is not None:
            raise AssertionError('No offset for object %d (%s)'
                                 % (unwritten, self._object_names.get(unwritten, '')))
        _backends.pdf_file_write_xref(self._native)

    def writeInfoDict(self):
        """Write out the info dictionary, checking it for good form"""
        _backends.pdf_file_set_info(self._native,
                                    _native_info_of(self.infoDict))
        self.infoObject = Reference(
            _backends.pdf_file_write_info_dict(self._native))

    def writeTrailer(self):
        """Write out the PDF trailer."""
        _backends.pdf_file_write_trailer(self._native)


class PdfPages:
    """
    A multi-page PDF file.

    Examples
    --------
    >>> import matplotlib.pyplot as plt
    >>> # Initialize:
    >>> with PdfPages('foo.pdf') as pdf:
    ...     # As many times as you like, create a figure fig and save it:
    ...     fig = plt.figure()
    ...     pdf.savefig(fig)
    ...     # When no figure is specified the current figure is saved
    ...     pdf.savefig()
    """

    @_api.delete_parameter("3.10", "keep_empty",
                           addendum="This parameter does nothing.")
    def __init__(self, filename, keep_empty=None, metadata=None):
        """
        Create a new PdfPages object.

        Parameters
        ----------
        filename : str or path-like or file-like
            Plots using `PdfPages.savefig` will be written to a file at this location.
            The file is opened when a figure is saved for the first time (overwriting
            any older file with the same name).
        metadata : dict, optional
            Information dictionary object (see PDF reference section 10.2.1
            'Document Information Dictionary').
        """
        self._filename = filename
        self._metadata = metadata
        self._file = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

    def _ensure_file(self):
        """The document, started on first use."""
        if self._file is None:
            self._file = PdfFile(self._filename, metadata=self._metadata)
        return self._file

    def close(self):
        """
        Finalize this object, making the underlying file a complete
        PDF file.
        """
        if self._file is not None:
            self._file.finalize()
            self._file.close()
            self._file = None

    def infodict(self):
        """
        Return a modifiable information dictionary object
        (see PDF reference section 10.2.1 'Document Information
        Dictionary').
        """
        return self._ensure_file().infoDict

    def savefig(self, figure=None, **kwargs):
        """
        Save a `.Figure` to this file as a new page.

        Any other keyword arguments are passed to `~.Figure.savefig`.

        Parameters
        ----------
        figure : `.Figure` or int, default: the active figure
            The figure, or index of the figure, that is saved to the file.
        """
        if not isinstance(figure, Figure):
            if figure is None:
                manager = Gcf.get_active()
            else:
                manager = Gcf.get_fig_manager(figure)
            if manager is None:
                raise ValueError(f"No figure {figure}")
            figure = manager.canvas.figure
        # Force use of pdf backend, as PdfPages is tightly coupled with it.
        figure.savefig(self, format="pdf", backend="pdf", **kwargs)

    def get_pagecount(self):
        """Return the current number of pages in the multipage pdf file."""
        return _backends.pdf_pages_count(self._ensure_file()._native)

    def attach_note(self, text, positionRect=[-100, -100, 0, 0]):
        """
        Add a new text note to the page to be saved next. The note is
        placed outside the page, so that it is invisible on printouts.
        """
        self._ensure_file().newTextnote(text, positionRect)


class RendererPdf(PdfPsDrawingMixin, DocumentDrawingMixin, RendererPDFPSBase):
    """
    The renderer of a PDF page of *width* × *height* inches, drawn in
    points, its images at *image_dpi*, into the `PdfFile` *file*.

    pltrs (Rust) writes the documents itself: the renderers pltrs hands
    Python are the one the figure is laid out with for its document (the
    renderer text measurement receives while a figure under the PDF canvas
    is laid out or saved), which measures text as the PDF backend does,
    and the one the program's drawing receives while the figure is saved
    (`FigureCanvasPdf._program_renderer`), which draws into the document
    pltrs (Rust) writes (*file* is None for both).
    """

    _use_afm_rc_name = "pdf.use14corefonts"

    def __init__(self, file, image_dpi, height, width):
        super().__init__(width, height)
        self.file = file
        self.gc = self.new_gc()
        self.image_dpi = image_dpi

    def get_text_width_height_descent(self, s, prop, ismath):
        # docstring inherited
        _api.check_in_list(["TeX", True, False], ismath=ismath)
        return _text_overrides.measure_pdf(s, FontManager._props(prop), ismath,
                                           72.0)

    def get_image_magnification(self):
        # docstring inherited
        return self.image_dpi / 72.0

    def _file_native(self):
        """The native document of the renderer's `PdfFile`; refused for the
        renderers of pltrs's saves, whose document pltrs (Rust) writes."""
        if self.file is None:
            raise RuntimeError(
                "RendererPdf draws into the document pltrs writes while it "
                "saves a figure: save the figure through its canvas "
                "(savefig), or make the renderer with a PdfFile")
        return self.file._native

    def _draw_now(self, drawing):
        """Draw *drawing* on the current page of the renderer's `PdfFile`."""
        _backends.pdf_file_draw(self._file_native(), drawing)

    def check_gc(self, gc, fillcolor=None):
        """Change the graphics state of the page into the one *gc* and the
        fill *fillcolor* need."""
        fill = None if fillcolor is None else tuple(map(float, to_rgba(fillcolor)))
        _backends.pdf_file_check_gc(self._file_native(), native_gc(gc), fill)

    def finalize(self):
        """Restore every graphics state the page saved."""
        _backends.pdf_file_restore_gc(self._file_native())

    def encode_string(self, s, fonttype):
        """*s* encoded as a PDF string shows it in a font of *fonttype*: in
        cp1252 for Type 1 and Type 3 fonts, else in UTF-16BE."""
        if fonttype in (1, 3):
            return s.encode('cp1252', 'replace')
        return s.encode('utf-16be', 'replace')


class GraphicsContextPdf(GraphicsContextBase):
    """
    A graphics context that knows the PDF operators changing the graphics
    state of a page into its own (matplotlib's, which ``RendererPdf.check_gc``
    keeps the page's state in); the `PdfFile` *file* names its alpha states
    and hatch patterns.
    """

    def __init__(self, file):
        super().__init__()
        self._fillcolor = (0.0, 0.0, 0.0)
        self._effective_alphas = (1.0, 1.0)
        self.file = file
        self.parent = None

    def __repr__(self):
        d = dict(self.__dict__)
        del d['file']
        del d['parent']
        return repr(d)

    def stroke(self):
        """
        Predicate: does the path need to be stroked (its outline drawn)?
        This tests for the various conditions that disable stroking
        the path, in which case it would presumably be filled.
        """
        # _linewidth > 0: in pdf a line of width 0 is drawn at minimum
        #   possible device width, but e.g., agg doesn't draw at all
        return (self._linewidth > 0 and self._alpha > 0 and
                (len(self._rgb) <= 3 or self._rgb[3] != 0.0))

    def fill(self, *args):
        """
        Predicate: does the path need to be filled?

        An optional argument can be used to specify an alternative
        _fillcolor, as needed by RendererPdf.draw_markers.
        """
        if len(args):
            _fillcolor = args[0]
        else:
            _fillcolor = self._fillcolor
        return (self._hatch or
                (_fillcolor is not None and
                 (len(_fillcolor) <= 3 or _fillcolor[3] != 0.0)))

    def paint(self):
        """
        Return the appropriate pdf operator to cause the path to be
        stroked, filled, or both.
        """
        return Op.paint_path(self.fill(), self.stroke())

    capstyles = {'butt': 0, 'round': 1, 'projecting': 2}
    joinstyles = {'miter': 0, 'round': 1, 'bevel': 2}

    def capstyle_cmd(self, style):
        return [self.capstyles[style], Op.setlinecap]

    def joinstyle_cmd(self, style):
        return [self.joinstyles[style], Op.setlinejoin]

    def linewidth_cmd(self, width):
        return [width, Op.setlinewidth]

    def dash_cmd(self, dashes):
        offset, dash = dashes
        if dash is None:
            dash = []
            offset = 0
        return [list(dash), offset, Op.setdash]

    def alpha_cmd(self, alpha, forced, effective_alphas):
        name = self.file.alphaState(effective_alphas)
        return [name, Op.setgstate]

    def hatch_cmd(self, hatch, hatch_color, hatch_linewidth):
        if not hatch:
            if self._fillcolor is not None:
                return self.fillcolor_cmd(self._fillcolor)
            else:
                return [Name('DeviceRGB'), Op.setcolorspace_nonstroke]
        else:
            hatch_style = (hatch_color, self._fillcolor, hatch, hatch_linewidth)
            name = self.file.hatchPattern(hatch_style)
            return [Name('Pattern'), Op.setcolorspace_nonstroke,
                    name, Op.setcolor_nonstroke]

    def rgb_cmd(self, rgb):
        if rcParams['pdf.inheritcolor']:
            return []
        if rgb[0] == rgb[1] == rgb[2]:
            return [rgb[0], Op.setgray_stroke]
        else:
            return [*rgb[:3], Op.setrgb_stroke]

    def fillcolor_cmd(self, rgb):
        if rgb is None or rcParams['pdf.inheritcolor']:
            return []
        elif rgb[0] == rgb[1] == rgb[2]:
            return [rgb[0], Op.setgray_nonstroke]
        else:
            return [*rgb[:3], Op.setrgb_nonstroke]

    def push(self):
        parent = GraphicsContextPdf(self.file)
        parent.copy_properties(self)
        parent.parent = self.parent
        self.parent = parent
        return [Op.gsave]

    def pop(self):
        assert self.parent is not None
        self.copy_properties(self.parent)
        self.parent = self.parent.parent
        return [Op.grestore]

    def clip_cmd(self, cliprect, clippath):
        """Set clip rectangle. Calls `.pop()` and `.push()`."""
        cmds = []
        # Pop graphics state until we hit the right one or the stack is empty
        while ((self._cliprect, self._clippath) != (cliprect, clippath)
                and self.parent is not None):
            cmds.extend(self.pop())
        # Unless we hit the right one, set the clip polygon
        if ((self._cliprect, self._clippath) != (cliprect, clippath) or
                self.parent is None):
            cmds.extend(self.push())
            if self._cliprect != cliprect:
                cmds.extend([cliprect, Op.rectangle, Op.clip, Op.endpath])
            if self._clippath != clippath:
                path, affine = clippath.get_transformed_path_and_affine()
                cmds.extend(
                    PdfFile.pathOperations(path, affine, simplify=False) +
                    [Op.clip, Op.endpath])
        return cmds

    commands = (
        # must come first since may pop
        (('_cliprect', '_clippath'), clip_cmd),
        (('_alpha', '_forced_alpha', '_effective_alphas'), alpha_cmd),
        (('_capstyle',), capstyle_cmd),
        (('_fillcolor',), fillcolor_cmd),
        (('_joinstyle',), joinstyle_cmd),
        (('_linewidth',), linewidth_cmd),
        (('_dashes',), dash_cmd),
        (('_rgb',), rgb_cmd),
        # must come after fillcolor and rgb
        (('_hatch', '_hatch_color', '_hatch_linewidth'), hatch_cmd),
    )

    def delta(self, other):
        """
        Copy properties of other into self and return PDF commands
        needed to transform *self* into *other*.
        """
        cmds = []
        fill_performed = False
        for params, cmd in self.commands:
            different = False
            for p in params:
                ours = getattr(self, p)
                theirs = getattr(other, p)
                try:
                    if ours is None or theirs is None:
                        different = ours is not theirs
                    else:
                        different = bool(ours != theirs)
                except ValueError:
                    ours = np.asarray(ours)
                    theirs = np.asarray(theirs)
                    different = (ours.shape != theirs.shape or
                                 np.any(ours != theirs))
                if different:
                    break

            # Need to update hatching if we also updated fillcolor
            if cmd.__name__ == 'hatch_cmd' and fill_performed:
                different = True

            if different:
                if cmd.__name__ == 'fillcolor_cmd':
                    fill_performed = True
                theirs = [getattr(other, p) for p in params]
                cmds.extend(cmd(self, *theirs))
                for p in params:
                    setattr(self, p, getattr(other, p))
        return cmds

    def copy_properties(self, other):
        """
        Copy properties of other into self.
        """
        super().copy_properties(other)
        fillcolor = getattr(other, '_fillcolor', self._fillcolor)
        effective_alphas = getattr(other, '_effective_alphas',
                                   self._effective_alphas)
        self._fillcolor = fillcolor
        self._effective_alphas = effective_alphas

    def finalize(self):
        """
        Make sure every pushed graphics state is popped.
        """
        cmds = []
        while self.parent is not None:
            cmds.extend(self.pop())
        return cmds


class FigureCanvasPdf(FigureCanvasBase):
    # docstring inherited

    fixed_dpi = 72
    filetypes = {'pdf': 'Portable Document Format'}

    def get_default_filetype(self):
        return 'pdf'

    def _print_renderer(self):
        """The renderer the figure is laid out with for its PDF document
        (the one matplotlib's ``print_pdf`` draws with)."""
        width, height = self.figure.get_size_inches()
        return RendererPdf(None, self.figure.dpi, height, width)

    def _program_renderer(self, dpi, bbox_inches_restore):
        """The renderer the program's drawing receives while the figure is
        saved at *dpi* (matplotlib's ``print_pdf`` draws the figure with
        it): a `MixedModeRenderer` around a `RendererPdf`, the figure drawn
        at 72 dpi meanwhile."""
        width, height = self.figure.get_size_inches()
        return MixedModeRenderer(
            self.figure, width, height, dpi,
            RendererPdf(None, dpi, height, width),
            bbox_inches_restore=bbox_inches_restore)

    def draw(self):
        self.figure.draw_without_rendering()
        return super().draw()

    def print_pdf(self, filename, *, bbox_inches_restore=None, metadata=None,
                  facecolor=None, edgecolor=None):
        """Write the figure as a PDF document to *filename*, with the
        document *metadata*, or as a new page of the `PdfPages` *filename*
        (whose document has its own metadata)."""
        settings = dict(bbox_inches_restore=bbox_inches_restore,
                        facecolor=facecolor, edgecolor=edgecolor)
        with _measuring_with(self._print_renderer()):
            if isinstance(filename, PdfPages):
                with self._program_drawing(bbox_inches_restore):
                    _backends.pdf_pages_savefig(
                        filename._ensure_file()._native, self.figure._native,
                        self._print_settings(**settings))
            else:
                self._print_native("pdf", filename,
                                   metadata=_native_info(metadata), **settings)
        if isinstance(filename, PdfPages):
            DrawEvent("draw_event", self, None)._process()


FigureManagerPdf = FigureManagerBase


@_Backend.export
class _BackendPdf(_Backend):
    FigureCanvas = FigureCanvasPdf




