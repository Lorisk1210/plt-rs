# The type stub of matplotlib.backends.backend_svg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import base64
import codecs
import datetime
import gzip
import hashlib
from io import BytesIO
import itertools
import logging
import os
import re
import uuid
import numpy as np
from PIL import Image
import matplotlib as mpl
from matplotlib import cbook, font_manager as fm
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, RendererBase
from matplotlib.backends.backend_mixed import MixedModeRenderer
from matplotlib.colors import rgb2hex
from matplotlib.dates import UTC
from matplotlib.path import Path
from matplotlib import _path
from matplotlib.transforms import Affine2D, Affine2DBase
_log = logging.getLogger(__name__)

def _escape_cdata(s):
    ...
_escape_xml_comment = re.compile('-(?=-)')

def _escape_comment(s):
    ...

def _escape_attrib(s):
    ...

def _quote_escape_attrib(s):
    ...

def _short_float_fmt(x):
    ...

class _XMLWriter:

    def __init__(self, file):
        self.__write = file.write
        self.__open = 0
        self.__tags = []
        self.__data = []
        self.__indentation = ' ' * 64
        self.flush = file.flush

    def __flush(self, indent=True):
        self.__open = 0
        self.__data = []

    def start(self, tag, attrib=None, **extra):
        self.__data = []
        self.__open = 1

    def comment(self, comment):
        ...

    def data(self, text):
        ...

    def end(self, tag=None, indent=True):
        self.__open = 0

    def close(self, id):
        ...

    def element(self, tag, text=None, attrib=None, **extra):
        ...

    def flush(self):
        ...

@mpl._api.deprecated('3.11')
class XMLWriter(_XMLWriter):
    ...

def _generate_transform(transform_list):
    ...

def _generate_css(attrib):
    ...
_capstyle_d = {'projecting': 'square', 'butt': 'butt', 'round': 'round'}

def _check_is_str(info, key):
    ...

def _check_is_iterable_of_str(infos, key):
    ...

class RendererSVG(RendererBase):

    def __init__(self, width, height, svgwriter, basename=None, image_dpi=72, *, metadata=None):
        self.width = width
        self.height = height
        self.writer = _XMLWriter(svgwriter)
        self.image_dpi = image_dpi
        self.basename = basename
        self._groupd = {}
        self._image_counter = itertools.count()
        self._clip_path_ids = {}
        self._clipd = {}
        self._markers = {}
        self._path_collection_id = 0
        self._hatchd = {}
        self._has_gouraud = False
        self._n_gradients = 0
        self._glyph_map = dict()
        self._start_id = self.writer.start('svg', width=f'{str_width}pt', height=f'{str_height}pt', viewBox=f'0 0 {str_width} {str_height}', xmlns='http://www.w3.org/2000/svg', version='1.1', id=mpl.rcParams['svg.id'], attrib={'xmlns:xlink': 'http://www.w3.org/1999/xlink'})

    def _get_clippath_id(self, clippath):
        self._clip_path_ids[clippath] = len(self._clip_path_ids)

    def finalize(self):
        ...

    def _write_metadata(self, metadata):
        ...

    def _write_default_style(self):
        ...

    def _make_id(self, type, content):
        ...

    def _make_flip_transform(self, transform):
        ...

    def _get_hatch(self, gc, rgbFace):
        self._hatchd[dictkey] = ((gc.get_hatch_path(), rgbFace, edge, lw), oid)

    def _write_hatches(self):
        ...

    def _get_style_dict(self, gc, rgbFace):
        ...

    def _get_style(self, gc, rgbFace):
        ...

    def _get_clip_attrs(self, gc):
        self._clipd[dictkey] = ((clippath, clippath_trans), oid)
        self._clipd[dictkey] = (dictkey, oid)

    def _write_clips(self):
        ...

    def open_group(self, s, gid=None):
        self._groupd[s] = self._groupd.get(s, 0) + 1

    def close_group(self, s):
        ...

    def option_image_nocomposite(self):
        ...

    def _convert_path(self, path, transform=None, clip=None, simplify=None, sketch=None):
        ...

    def draw_path(self, gc, path, transform, rgbFace=None):
        ...

    def draw_markers(self, gc, marker_path, marker_trans, path, trans, rgbFace=None):
        self._markers[dictkey] = oid

    def draw_path_collection(self, gc, master_transform, paths, all_transforms, offsets, offset_trans, facecolors, edgecolors, linewidths, linestyles, antialiaseds, urls, offset_position, *, hatchcolors=None):
        ...

    def _draw_gouraud_triangle(self, transformed_points, colors):
        ...

    def draw_gouraud_triangles(self, gc, triangles_array, colors_array, transform):
        self._has_gouraud = True

    def option_scale_image(self):
        ...

    def get_image_magnification(self):
        ...

    def draw_image(self, gc, x, y, im, transform=None):
        ...

    def _update_glyph_map_defs(self, glyph_map_new):
        ...

    def _adjust_glyph_repr(self, glyph_repr):
        ...

    def _draw_text_as_path(self, gc, x, y, s, prop, angle, ismath, mtext=None):
        ...

    def _draw_text_as_text(self, gc, x, y, s, prop, angle, ismath, mtext=None):
        ...

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        ...

    def flipy(self):
        ...

    def get_canvas_width_height(self):
        ...

    def get_text_width_height_descent(self, s, prop, ismath):
        ...

class FigureCanvasSVG(FigureCanvasBase):
    filetypes = {'svg': 'Scalable Vector Graphics', 'svgz': 'Scalable Vector Graphics'}
    fixed_dpi = 72

    def print_svg(self, filename, *, bbox_inches_restore=None, metadata=None):
        self.figure.dpi = 72

    def print_svgz(self, filename, **kwargs):
        ...

    def get_default_filetype(self):
        ...

    def draw(self):
        ...
FigureManagerSVG = FigureManagerBase
svgProlog = '<?xml version="1.0" encoding="utf-8" standalone="no"?>\n<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN"\n  "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">\n'

@_Backend.export
class _BackendSVG(_Backend):
    backend_version = mpl.__version__
    FigureCanvas = FigureCanvasSVG
__all__ = ['base64', 'codecs', 'datetime', 'gzip', 'hashlib', 'BytesIO', 'itertools', 'logging', 'os', 're', 'uuid', 'np', 'Image', 'mpl', 'cbook', 'fm', 'FigureCanvasBase', 'FigureManagerBase', 'RendererBase', 'MixedModeRenderer', 'rgb2hex', 'UTC', 'Path', 'Affine2D', 'Affine2DBase', 'XMLWriter', 'RendererSVG', 'FigureCanvasSVG', 'FigureManagerSVG', 'svgProlog']
