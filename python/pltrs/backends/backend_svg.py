"""
The SVG backend (matplotlib's ``backend_svg``): `FigureCanvasSVG`, whose
``print_svg`` and ``print_svgz`` write the figure as an SVG document,
plain or gzip-compressed. pltrs (Rust) writes the documents.

The checks of the metadata and the escaping functions of the XML writer
are matplotlib's (from matplotlib 3.11.2, under the Matplotlib License).
"""

import datetime
import re
from datetime import UTC  # noqa: F401  (matplotlib's backend_svg has it)

import numpy as np

from .. import __version__, _api, _named, rcParams
from .._native import backends as _backends, text_overrides as _text_overrides
from .. import _print_targets
from ..backend_bases import (_Backend, FigureCanvasBase, FigureManagerBase, RendererBase,
                             _write_bytes)
from ..font_manager import FontManager
from ..path import Path
from ..text import _measuring_with
from ._native_drawing import DocumentDrawingMixin
from ._svg_arguments import SvgArgumentsMixin, short_float
from .backend_mixed import MixedModeRenderer

#: The XML declaration and doctype every SVG document starts with.
svgProlog = _backends.svg_prolog()


class _XMLWriter:
    """
    Parameters
    ----------
    file : writable text file-like object
    """
    # pltrs (Rust) writes the XML (``pltrs_render::XmlWriter``); the writer
    # hands the file each piece as matplotlib's does. What matplotlib's
    # writer computes with in Python is computed here first, so that what
    # it cannot write fails as there: the escaped tags, attributes and
    # comments, the character data kept until an element is written (and
    # joined then), the open elements closed one by one.

    def __init__(self, file):
        self.__write = file.write
        if hasattr(file, "flush"):
            self.flush = file.flush
        self.__native = _backends.xml_new()
        self.__data = []

    def __written(self, result=None):
        """Write what pltrs (Rust) wrote to the file; return *result*."""
        text = _backends.xml_take(self.__native)
        if text:
            self.__write(text)
        return result

    def __flush(self):
        """Hand pltrs (Rust) the character data kept (`data`), joined."""
        if self.__data:
            text = ''.join(self.__data)
            self.__data = []
            _backends.xml_data(self.__native, text)

    def start(self, tag, attrib=None, **extra):
        """
        Open a new element.  Attributes can be given as keyword
        arguments, or as a string/string dictionary. The method returns
        an opaque identifier that can be passed to the :meth:`close`
        method, to close all open elements up to and including this one.

        Parameters
        ----------
        tag
            Element tag.
        attrib
            Attribute dictionary.  Alternatively, attributes can be given as
            keyword arguments.

        Returns
        -------
        An element identifier.
        """
        if attrib is None:
            attrib = {}
        self.__flush()
        _escape_cdata(tag)
        attributes = []
        try:
            for k, v in {**attrib, **extra}.items():
                if v:
                    _escape_cdata(k)
                    _quote_escape_attrib(v)
                    attributes.append((k, v))
        except Exception:
            # matplotlib's writer opened the element before it failed on
            # its attributes.
            self.__written(_backends.xml_start(self.__native, tag, []))
            raise
        return self.__written(_backends.xml_start(self.__native, tag, attributes))

    def comment(self, comment):
        """
        Add a comment to the output stream.

        Parameters
        ----------
        comment : str
            Comment text.
        """
        self.__flush()
        _escape_comment(comment)
        _backends.xml_comment(self.__native, comment)
        self.__written()

    def data(self, text):
        """
        Add character data to the output stream.

        Parameters
        ----------
        text : str
            Character data.
        """
        self.__data.append(text)

    def end(self, tag=None, indent=True):
        """
        Close the current element (opened by the most recent call to
        :meth:`start`).

        Parameters
        ----------
        tag
            Element tag.  If given, the tag must match the start tag.  If
            omitted, the current element is closed.
        indent : bool, default: True
        """
        tags = _backends.xml_tags(self.__native)
        if tag:
            assert tags, f"unbalanced end({tag})"
            assert _escape_cdata(tag) == tags[-1], \
                f"expected end({tags[-1]}), got {tag}"
        else:
            assert tags, "unbalanced end()"
        try:
            self.__flush()
        except TypeError:
            # matplotlib's writer takes the element off before it writes
            # the data, which it then fails to join: nothing is written.
            _backends.xml_end(self.__native, bool(indent))
            _backends.xml_take(self.__native)
            raise
        _backends.xml_end(self.__native, bool(indent))
        self.__written()

    def close(self, id):
        """
        Close open elements, up to (and including) the element identified
        by the given identifier.

        Parameters
        ----------
        id
            Element identifier, as returned by the :meth:`start` method.
        """
        while len(_backends.xml_tags(self.__native)) > id:
            self.end()

    def element(self, tag, text=None, attrib=None, **extra):
        """
        Add an entire element.  This is the same as calling :meth:`start`,
        :meth:`data`, and :meth:`end` in sequence. The *text* argument can be
        omitted.
        """
        if attrib is None:
            attrib = {}
        self.start(tag, attrib, **extra)
        if text:
            self.data(text)
        self.end(indent=False)

    def flush(self):
        """Flush the output stream."""
        pass  # replaced by the constructor


@_api.deprecated("3.11")
class XMLWriter(_XMLWriter):
    """
    An XML writer class.

    :meta private:
    """


def _escape_cdata(s):
    """*s* with the characters XML character data cannot hold escaped."""
    s = s.replace("&", "&amp;")
    s = s.replace("<", "&lt;")
    s = s.replace(">", "&gt;")
    return s


_escape_xml_comment = re.compile(r'-(?=-)')


def _escape_comment(s):
    s = _escape_cdata(s)
    return _escape_xml_comment.sub('- ', s)


def _escape_attrib(s):
    s = s.replace("&", "&amp;")
    s = s.replace("'", "&apos;")
    s = s.replace('"', "&quot;")
    s = s.replace("<", "&lt;")
    s = s.replace(">", "&gt;")
    return s


def _quote_escape_attrib(s):
    return ('"' + _escape_cdata(s) + '"' if '"' not in s else
            "'" + _escape_cdata(s) + "'" if "'" not in s else
            '"' + _escape_attrib(s) + '"')

#: The metadata keys an SVG document holds.
_METADATA_KEYS = frozenset([
    'Title', 'Coverage', 'Date', 'Description', 'Format', 'Identifier',
    'Language', 'Relation', 'Source', 'Type', 'Creator', 'Contributor',
    'Publisher', 'Rights', 'Keywords'])


def _check_is_str(info, key):
    if not isinstance(info, str):
        raise TypeError(f'Invalid type for {key} metadata. Expected str, not '
                        f'{type(info)}.')


def _check_is_iterable_of_str(infos, key):
    if np.iterable(infos):
        for info in infos:
            if not isinstance(info, str):
                raise TypeError(f'Invalid type for {key} metadata. Expected '
                                f'iterable of str, not {type(info)}.')
    else:
        raise TypeError(f'Invalid type for {key} metadata. Expected str or '
                        f'iterable of str, not {type(infos)}.')


def _date_text(date):
    """The ``Date`` metadata *date* as matplotlib writes it: ISO 8601,
    several dates joined by slashes."""
    if isinstance(date, str):
        return date
    if isinstance(date, (datetime.datetime, datetime.date)):
        return date.isoformat()
    if np.iterable(date):
        dates = []
        for d in date:
            if isinstance(d, str):
                dates.append(d)
            elif isinstance(d, (datetime.datetime, datetime.date)):
                dates.append(d.isoformat())
            else:
                raise TypeError(
                    f'Invalid type for Date metadata. '
                    f'Expected iterable of str, date, or datetime, '
                    f'not {type(d)}.')
        return '/'.join(dates)
    raise TypeError(f'Invalid type for Date metadata. '
                    f'Expected str, date, datetime, or iterable '
                    f'of the same, not {type(date)}.')


def _svg_metadata(metadata):
    """*metadata* checked as matplotlib's SVG writer checks it, with
    pltrs (Python) as default creator and the dates written out; the
    entries left to pltrs (Rust)'s defaults (format, type, date) are not
    added."""
    from .._native import backends as _backends
    metadata = {
        'Creator': f'{_backends.default_creator()}, matplotlib {__version__} API',
        **({} if metadata is None else metadata),
    }
    unknown = [key for key in metadata if key not in _METADATA_KEYS]
    if unknown:
        raise ValueError('Unknown metadata key(s) passed to SVG writer: ' +
                         ','.join(unknown))
    for key, value in metadata.items():
        if value is None:
            continue
        if key == 'Date':
            metadata[key] = _date_text(value)
        elif key in ('Creator', 'Contributor', 'Publisher', 'Rights',
                     'Keywords'):
            if not isinstance(value, str):
                _check_is_iterable_of_str(value, key)
                metadata[key] = list(value)
        else:
            _check_is_str(value, key)
    return metadata


class RendererSVG(SvgArgumentsMixin, DocumentDrawingMixin, RendererBase):
    # pltrs (Rust) writes the documents itself: this is the renderer the
    # program's drawing receives while a figure is saved as SVG
    # (`FigureCanvasSVG._program_renderer`), which draws into the document
    # pltrs (Rust) writes. One the program makes around an *svgwriter* of its
    # own draws into a document of its own, which `finalize` writes there.

    def __init__(self, width, height, svgwriter, basename=None, image_dpi=72,
                 *, metadata=None):
        super().__init__()
        self.width = width
        self.height = height
        self.image_dpi = image_dpi
        self.basename = basename
        # matplotlib's renderer makes its XML writer, then writes the size
        # of the page and the metadata.
        svgwriter.write  # noqa: B018
        short_float(height), short_float(width)
        self._metadata = _svg_metadata(metadata)
        self._file = svgwriter

    @classmethod
    def _of_save(cls, width, height, image_dpi):
        """The renderer the program's drawing receives while pltrs (Rust)
        saves a figure of *width* × *height* points as SVG (images at
        *image_dpi*), drawing into the document pltrs (Rust) writes."""
        renderer = cls.__new__(cls)
        RendererBase.__init__(renderer)
        renderer.width = width
        renderer.height = height
        renderer.image_dpi = image_dpi
        renderer.basename = None
        return renderer

    def _new_page(self):
        return _backends.svg_page_new(float(self.width), float(self.height),
                                      float(self.image_dpi), self._metadata)

    def finalize(self):
        # Write the document drawn to the *svgwriter* (the renderers of
        # pltrs's saves have no document of their own: pltrs (Rust) writes
        # it).
        if self._file is not None:
            self._file.write(_backends.page_finish(self._document_page()))
            if hasattr(self._file, "flush"):
                self._file.flush()

    def get_canvas_width_height(self):
        # docstring inherited
        return self.width, self.height

    def get_text_width_height_descent(self, s, prop, ismath):
        # docstring inherited
        _api.check_in_list(["TeX", True, False], ismath=ismath)
        if ismath is True:
            # As matplotlib's text-to-path measures math: parsed at its font
            # scale, numpy's floats scaled back.
            return self._text2path.get_text_width_height_descent(s, prop, ismath)
        return _text_overrides.measure_svg(s, FontManager._props(prop), ismath,
                                           72.0)

    def option_image_nocomposite(self):
        # docstring inherited
        return not rcParams['image.composite_image']

    def option_scale_image(self):
        # docstring inherited
        return True

    def get_image_magnification(self):
        # docstring inherited
        return self.image_dpi / 72.0

    # The drawing methods of `._native_drawing.DocumentDrawingMixin`, which
    # pltrs (Rust) draws, as the renderer's own (matplotlib's renderers each
    # define them).

    def open_group(self, s, gid=None):
        # docstring inherited
        if gid:
            # The id matplotlib's writer writes.
            _quote_escape_attrib(gid)
        else:
            # matplotlib counts the groups of each name.
            {}.get(s)
        super().open_group(s, gid)

    def close_group(self, s):
        # docstring inherited
        super().close_group(s)

    def draw_path(self, gc, path, transform, rgbFace=None):
        # docstring inherited
        self._path_arguments(gc, path, transform, rgbFace)
        super().draw_path(gc, path, transform, rgbFace)

    def draw_markers(self, gc, marker_path, marker_trans, path, trans,
                     rgbFace=None):
        # docstring inherited
        if self._markers_arguments(gc, marker_path, marker_trans, path, trans,
                                   rgbFace):
            if marker_path is None:
                # matplotlib's extension writes no marker path as an empty one.
                marker_path = Path(np.zeros((0, 2)))
            super().draw_markers(gc, marker_path, marker_trans, path, trans,
                                 rgbFace)

    def draw_path_collection(self, gc, master_transform, paths, all_transforms,
                             offsets, offset_trans, facecolors, edgecolors,
                             linewidths, linestyles, antialiaseds, urls,
                             offset_position, *, hatchcolors=None):
        # Is the optimization worth it? Rough calculation:
        # cost of emitting a path in-line is
        #    (len_path + 5) * uses_per_path
        # cost of definition+use is
        #    (len_path + 3) + 9 * uses_per_path
        len_path, uses_per_path = self._collection_uses(
            paths, all_transforms, offsets, facecolors, edgecolors)
        should_do_optimization = \
            len_path + 9 * uses_per_path + 3 < (len_path + 5) * uses_per_path
        self._draw_collection(
            should_do_optimization, gc, master_transform, paths, all_transforms,
            offsets, offset_trans, facecolors, edgecolors, linewidths, linestyles,
            antialiaseds, urls, offset_position, hatchcolors)

    def draw_gouraud_triangles(self, gc, triangles_array, colors_array,
                               transform):
        # docstring inherited
        self._gouraud_arguments(gc, triangles_array, colors_array, transform)
        super().draw_gouraud_triangles(gc, triangles_array, colors_array, transform)

    def draw_image(self, gc, x, y, im, transform=None):
        # docstring inherited
        if self._image_arguments(gc, x, y, im, transform):
            super().draw_image(gc, x, y, im, transform)

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        # docstring inherited
        self._text_arguments(gc, x, y, s, prop, angle, mtext)
        # Any true *ismath* but "TeX" typesets mathtext, as matplotlib's.
        super().draw_text(gc, x, y, s, prop, angle,
                          ismath if ismath == "TeX" else bool(ismath), mtext)

    def flipy(self):
        # docstring inherited
        return super().flipy()


class FigureCanvasSVG(FigureCanvasBase):
    filetypes = {'svg': 'Scalable Vector Graphics',
                 'svgz': 'Scalable Vector Graphics'}

    fixed_dpi = 72

    def print_svg(self, filename, *, bbox_inches_restore=None, metadata=None):
        """
        Parameters
        ----------
        filename : str or path-like or file-like
            Output target; if a string, a file will be opened for writing.

        metadata : dict[str, Any], optional
            Metadata in the SVG file defined as key-value pairs of strings,
            datetimes, or lists of strings, e.g., ``{'Creator': 'My software',
            'Contributor': ['Me', 'My Friend'], 'Title': 'Awesome'}``.

            The standard keys and their value types are:

            * *str*: ``'Coverage'``, ``'Description'``, ``'Format'``,
              ``'Identifier'``, ``'Language'``, ``'Relation'``, ``'Source'``,
              ``'Title'``, and ``'Type'``.
            * *str* or *list of str*: ``'Contributor'``, ``'Creator'``,
              ``'Keywords'``, ``'Publisher'``, and ``'Rights'``.
            * *str*, *date*, *datetime*, or *tuple* of same: ``'Date'``. If a
              non-*str*, then it will be formatted as ISO 8601.

            Values have been predefined for ``'Creator'``, ``'Date'``,
            ``'Format'``, and ``'Type'``. They can be removed by setting them
            to `None`.

            Information is encoded as `Dublin Core Metadata`__.

            .. _DC: https://www.dublincore.org/specifications/dublin-core/

            __ DC_
        """
        self._print_svg("svg", filename, bbox_inches_restore=bbox_inches_restore,
                        metadata=metadata)

    def print_svgz(self, filename, **kwargs):
        self._print_svg("svgz", filename, **kwargs)

    @_named.named("FigureCanvasSVG.print_svg")
    def _print_svg(self, fmt, filename, *, bbox_inches_restore=None, metadata=None):
        """Write the figure as *fmt*, ``svg`` or ``svgz``, its texts measured
        through the metrics cache of a renderer of the print's own, as
        matplotlib's ``print_svg`` draws with a new `RendererSVG` (named
        so in the errors of its calls, as ``print_svgz`` calls it)."""
        width, height = self.figure.get_size_inches()
        renderer = RendererSVG._of_save(width * 72, height * 72, self.figure.dpi)
        with _measuring_with(renderer):
            if fmt == "svg" and not rcParams["svg.image_inline"]:
                self._print_svg_with_files(filename, metadata=_svg_metadata(metadata),
                                           bbox_inches_restore=bbox_inches_restore)
            else:
                self._print_native(fmt, filename, metadata=_svg_metadata(metadata),
                                   bbox_inches_restore=bbox_inches_restore)

    def _print_svg_with_files(self, filename, **kwargs):
        """Write the figure as SVG to *filename* with its images as PNG
        files beside it (:rc:`svg.image_inline` off), named after the name
        of the file written to as matplotlib's ``RendererSVG`` names them
        (``<name>.image<N>.png``; a stream without a name: ``.image<N>.png``
        in the working directory)."""
        _print_targets.check("svg", filename)
        stem = (os.fspath(filename) if isinstance(filename, (str, os.PathLike))
                else getattr(filename, "name", ""))
        with self._program_drawing(kwargs.get("bbox_inches_restore")):
            data, images = _backends.print_svg(
                self.figure._native, self._print_settings(**kwargs),
                stem if isinstance(stem, str) else "", self._draw_event_sender())
        for name, png in images:
            logging.getLogger(__name__).info('Writing image file for inclusion: %s', name)
            with open(name, "wb") as file:
                file.write(png)
        _write_bytes(data, filename, "svg")

    def get_default_filetype(self):
        return 'svg'

    def _program_renderer(self, dpi, bbox_inches_restore):
        """The renderer the program's drawing receives while the figure is
        saved at *dpi* (matplotlib's ``print_svg`` draws the figure with it):
        a `MixedModeRenderer` around a `RendererSVG`, the figure drawn at
        72 dpi meanwhile."""
        width, height = self.figure.get_size_inches()
        return MixedModeRenderer(
            self.figure, width, height, dpi,
            RendererSVG._of_save(width * 72, height * 72, dpi),
            bbox_inches_restore=bbox_inches_restore)

    def draw(self):
        self.figure.draw_without_rendering()
        return super().draw()


FigureManagerSVG = FigureManagerBase


@_Backend.export
class _BackendSVG(_Backend):
    backend_version = __version__
    FigureCanvas = FigureCanvasSVG


# What matplotlib's module also has from its imports, which programs
# import from it.
import base64  # noqa: E402, F401
import codecs  # noqa: E402, F401
import gzip  # noqa: E402, F401
import hashlib  # noqa: E402, F401
import itertools  # noqa: E402, F401
import logging  # noqa: E402, F401
import os  # noqa: E402, F401
import uuid  # noqa: E402, F401
from PIL import Image  # noqa: E402, F401
from io import BytesIO  # noqa: E402, F401
from .. import cbook, font_manager as fm  # noqa: E402, F401
from ..colors import to_hex as rgb2hex  # noqa: E402, F401
from ..transforms import Affine2D, Affine2DBase  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
