# The type stub of matplotlib.backends.backend_ps: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import bisect
import codecs
import datetime
from enum import Enum
import functools
from io import StringIO
import itertools
import logging
import math
import os
import pathlib
import shutil
import struct
from tempfile import TemporaryDirectory
import textwrap
import time
import fontTools
import numpy as np
import matplotlib as mpl
from matplotlib import _api, cbook, _path, _text_helpers
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, RendererBase
from matplotlib.cbook import is_writable_file_like, file_requires_unicode
from matplotlib.font_manager import get_font
from matplotlib.ft2font import LoadFlags
from matplotlib._mathtext_data import uni2type1
from matplotlib.path import Path
from matplotlib.texmanager import TexManager
from matplotlib.transforms import Affine2D
from matplotlib.backends.backend_mixed import MixedModeRenderer
from . import _backend_pdf_ps
_log = logging.getLogger(__name__)
debugPS = False
papersize = {'letter': (8.5, 11), 'legal': (8.5, 14), 'ledger': (11, 17), 'a0': (33.11, 46.81), 'a1': (23.39, 33.11), 'a2': (16.54, 23.39), 'a3': (11.69, 16.54), 'a4': (8.27, 11.69), 'a5': (5.83, 8.27), 'a6': (4.13, 5.83), 'a7': (2.91, 4.13), 'a8': (2.05, 2.91), 'a9': (1.46, 2.05), 'a10': (1.02, 1.46), 'b0': (40.55, 57.32), 'b1': (28.66, 40.55), 'b2': (20.27, 28.66), 'b3': (14.33, 20.27), 'b4': (10.11, 14.33), 'b5': (7.16, 10.11), 'b6': (5.04, 7.16), 'b7': (3.58, 5.04), 'b8': (2.51, 3.58), 'b9': (1.76, 2.51), 'b10': (1.26, 1.76)}

def _nums_to_str(*args, sep=' '):
    ...

def _move_path_to_path_or_stream(src, dst):
    ...

def _font_to_ps_type3(font_path, subset_index, glyph_indices):
    ...

def _font_to_ps_type42(font_path, subset_index, glyph_indices, fh):
    ...

def _serialize_type42(font, subset_index, subset, fontdata):
    ...

def _version_and_breakpoints(loca, fontdata):
    ...

def _bounds(font):
    ...

def _generate_charstrings(font):
    ...

def _generate_sfnts(fontdata, font, breakpoints):
    ...

def _log_if_debug_on(meth):
    ...

class RendererPS(_backend_pdf_ps.RendererPDFPSBase):
    _afm_font_dir = cbook._get_data_path('fonts/afm')
    _use_afm_rc_name = 'ps.useafm'

    def __init__(self, width, height, pswriter, imagedpi=72):
        self._pswriter = pswriter
        self.imagedpi = imagedpi
        self.color = None
        self.linewidth = None
        self.linejoin = None
        self.linecap = None
        self.linedash = None
        self.fontname = None
        self.fontsize = None
        self._hatches = {}
        self.image_magnification = imagedpi / 72
        self._clip_paths = {}
        self._path_collection_id = 0
        self._character_tracker = _backend_pdf_ps.CharacterTracker(_backend_pdf_ps._FONT_MAX_GLYPH.get(mpl.rcParams['ps.fonttype'], 0))
        self._logwarn_once = functools.cache(_log.warning)
        self.textcnt = 0
        self.psfrag = []

    def _is_transparent(self, rgb_or_rgba):
        ...

    def set_color(self, r, g, b, store=True):
        self.color = (r, g, b)

    def set_linewidth(self, linewidth, store=True):
        self.linewidth = linewidth

    @staticmethod
    def _linejoin_cmd(linejoin):
        ...

    def set_linejoin(self, linejoin, store=True):
        self.linejoin = linejoin

    @staticmethod
    def _linecap_cmd(linecap):
        ...

    def set_linecap(self, linecap, store=True):
        self.linecap = linecap

    def set_linedash(self, offset, seq, store=True):
        self.linedash = (offset, seq)

    def set_font(self, fontname, fontsize, store=True):
        self.fontname = fontname
        self.fontsize = fontsize

    def create_hatch(self, hatch, linewidth):
        self._hatches[hatch] = name

    def get_image_magnification(self):
        ...

    def _convert_path(self, path, transform, clip=False, simplify=None):
        ...

    def _get_clip_cmd(self, gc):
        self._clip_paths[key] = custom_clip_cmd

    @_log_if_debug_on
    def draw_image(self, gc, x, y, im, transform=None):
        ...

    @_log_if_debug_on
    def draw_path(self, gc, path, transform, rgbFace=None):
        ...

    @_log_if_debug_on
    def draw_markers(self, gc, marker_path, marker_trans, path, trans, rgbFace=None):
        ...

    @_log_if_debug_on
    def draw_path_collection(self, gc, master_transform, paths, all_transforms, offsets, offset_trans, facecolors, edgecolors, linewidths, linestyles, antialiaseds, urls, offset_position, *, hatchcolors=None):
        ...

    @_log_if_debug_on
    def draw_tex(self, gc, x, y, s, prop, angle, *, mtext=None):
        ...

    @_log_if_debug_on
    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        ...

    @_log_if_debug_on
    def draw_mathtext(self, gc, x, y, s, prop, angle):
        ...

    @_log_if_debug_on
    def draw_gouraud_triangles(self, gc, points, colors, trans):
        ...

    def _draw_ps(self, ps, gc, rgbFace, *, fill=True, stroke=True):
        ...

class _Orientation(Enum):
    portrait, landscape = range(2)

    def swap_if_landscape(self, shape):
        ...

class FigureCanvasPS(FigureCanvasBase):
    fixed_dpi = 72
    filetypes = {'ps': 'Postscript', 'eps': 'Encapsulated Postscript'}

    def get_default_filetype(self):
        ...

    def _print_ps(self, fmt, outfile, *, metadata=None, papertype=None, orientation='portrait', bbox_inches_restore=None, **kwargs):
        self.figure.dpi = 72

    def _print_figure(self, fmt, outfile, *, dpi, dsc_comments, orientation, papertype, bbox_inches_restore=None):
        self._pswriter = StringIO()

    def _print_figure_tex(self, fmt, outfile, *, dpi, dsc_comments, orientation, papertype, bbox_inches_restore=None):
        self._pswriter = StringIO()
    print_ps = functools.partialmethod(_print_ps, 'ps')
    print_eps = functools.partialmethod(_print_ps, 'eps')

    def draw(self):
        ...

def _convert_psfrags(tmppath, psfrags, paper_width, paper_height, orientation):
    ...

def _try_distill(func, tmppath, *args, **kwargs):
    ...

def gs_distill(tmpfile, eps=False, ptype='letter', bbox=None, rotated=False):
    ...

def xpdf_distill(tmpfile, eps=False, ptype='letter', bbox=None, rotated=False):
    ...

def _get_bbox_header(lbrt):
    ...

def _get_rotate_command(lbrt):
    ...

def pstoeps(tmpfile, bbox=None, rotated=False):
    ...
FigureManagerPS = FigureManagerBase
_psDefs = ['/_d { bind def } bind def', '/m { moveto } _d', '/l { lineto } _d', '/r { rlineto } _d', '/c { curveto } _d', '/cl { closepath } _d', '/ce { closepath eofill } _d', '/sc { setcachedevice } _d']

@_Backend.export
class _BackendPS(_Backend):
    backend_version = 'Level II'
    FigureCanvas = FigureCanvasPS
__all__ = ['bisect', 'codecs', 'datetime', 'Enum', 'functools', 'StringIO', 'itertools', 'logging', 'math', 'os', 'pathlib', 'shutil', 'struct', 'TemporaryDirectory', 'textwrap', 'time', 'fontTools', 'np', 'mpl', 'cbook', 'FigureCanvasBase', 'FigureManagerBase', 'RendererBase', 'is_writable_file_like', 'file_requires_unicode', 'get_font', 'LoadFlags', 'uni2type1', 'Path', 'TexManager', 'Affine2D', 'MixedModeRenderer', 'debugPS', 'papersize', 'RendererPS', 'FigureCanvasPS', 'gs_distill', 'xpdf_distill', 'pstoeps', 'FigureManagerPS']
