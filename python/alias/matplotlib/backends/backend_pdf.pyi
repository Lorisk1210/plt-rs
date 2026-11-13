# The type stub of matplotlib.backends.backend_pdf: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import codecs
from datetime import timezone
from datetime import datetime
from enum import Enum
from functools import total_ordering
from io import BytesIO
import itertools
import logging
import math
import os
import string
import struct
import sys
import time
import types
import zlib
import numpy as np
from PIL import Image
import matplotlib as mpl
from matplotlib import _api, _text_helpers, _type1font, cbook, dviread
from matplotlib._pylab_helpers import Gcf
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, GraphicsContextBase, RendererBase
from matplotlib.backends.backend_mixed import MixedModeRenderer
from matplotlib.figure import Figure
from matplotlib.font_manager import FontPath, get_font, fontManager as _fontManager
from matplotlib._afm import AFM
from matplotlib.ft2font import FT2Font, FaceFlags, LoadFlags, StyleFlags
from matplotlib.transforms import Affine2D, BboxBase
from matplotlib.path import Path
from matplotlib.dates import UTC
from matplotlib import _path
from . import _backend_pdf_ps
_log = logging.getLogger(__name__)

def _fill(strings, linelen=75):
    ...

def _create_pdf_info_dict(backend, metadata):
    ...

def _datetime_to_pdf(d):
    ...

def _calculate_quad_point_coordinates(x, y, width, height, angle=0):
    ...

def _get_coordinates_of_block(x, y, width, height, angle=0):
    ...

def _get_link_annotation(gc, x, y, width, height, angle=0):
    ...
_str_escapes = str.maketrans({'\\': '\\\\', '(': '\\(', ')': '\\)', '\n': '\\n', '\r': '\\r'})

def pdfRepr(obj):
    ...

class Reference:

    def __init__(self, id):
        self.id = id

    def __repr__(self):
        ...

    def pdfRepr(self):
        ...

    def write(self, contents, file):
        ...

@total_ordering
class Name:
    __slots__ = ('name',)
    _hexify = {c: '#%02x' % c for c in {*range(256)} - {*range(ord('!'), ord('~') + 1)}}

    def __init__(self, name):
        self.name = name.name
        self.name = name.translate(self._hexify).encode('ascii')

    def __repr__(self):
        ...

    def __str__(self):
        ...

    def __eq__(self, other):
        ...

    def __lt__(self, other):
        ...

    def __hash__(self):
        ...

    def pdfRepr(self):
        ...

class Verbatim:

    def __init__(self, x):
        self._x = x

    def pdfRepr(self):
        ...

class Op(Enum):
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
        ...

    @classmethod
    def paint_path(cls, fill, stroke):
        ...

class Stream:
    __slots__ = ('id', 'len', 'pdfFile', 'file', 'compressobj', 'extra', 'pos')

    def __init__(self, id, len, file, extra=None, png=None):
        self.id = id
        self.len = len
        self.pdfFile = file
        self.file = file.fh
        self.compressobj = None
        self.extra = dict()
        self.extra = extra.copy()
        self.compressobj = zlib.compressobj(mpl.rcParams['pdf.compression'])
        self.file = BytesIO()
        self.pos = self.file.tell()

    def _writeHeader(self):
        ...

    def end(self):
        self.len = len(contents)
        self.file = self.pdfFile.fh

    def write(self, data):
        ...

    def _flush(self):
        self.compressobj = None

def _get_pdf_charprocs(font_path, glyph_indices):
    ...

class PdfFile:

    def __init__(self, filename, metadata=None):
        self._object_seq = itertools.count(1)
        self.xrefTable = [[0, 65535, 'the zero object']]
        self.passed_in_file_object = False
        self.original_file_like = None
        self.tell_base = 0
        self.fh = fh
        self.currentstream = None
        self.rootObject = self.reserveObject('root')
        self.pagesObject = self.reserveObject('pages')
        self.pageList = []
        self.fontObject = self.reserveObject('fonts')
        self._extGStateObject = self.reserveObject('extended graphics states')
        self.hatchObject = self.reserveObject('tiling patterns')
        self.gouraudObject = self.reserveObject('Gouraud triangles')
        self.XObjectObject = self.reserveObject('external objects')
        self.resourceObject = self.reserveObject('resources')
        self.infoDict = _create_pdf_info_dict('pdf', metadata or {})
        self._internal_font_seq = (Name(f'F{i}') for i in itertools.count(1))
        self._fontNames = {}
        self._dviFontInfo = {}
        self._character_tracker = _backend_pdf_ps.CharacterTracker(_backend_pdf_ps._FONT_MAX_GLYPH.get(mpl.rcParams['ps.fonttype'], 0))
        self.alphaStates = {}
        self._alpha_state_seq = (Name(f'A{i}') for i in itertools.count(1))
        self._soft_mask_states = {}
        self._soft_mask_seq = (Name(f'SM{i}') for i in itertools.count(1))
        self._soft_mask_groups = []
        self._hatch_patterns = {}
        self._hatch_pattern_seq = (Name(f'H{i}') for i in itertools.count(1))
        self.gouraudTriangles = []
        self._images = {}
        self._image_seq = (Name(f'I{i}') for i in itertools.count(1))
        self.markers = {}
        self.paths = []
        self._annotations = []
        self.pageAnnotations = []
        self.tell_base = filename.tell()
        self.passed_in_file_object = True
        self.original_file_like = filename
    fontNames = _api.deprecated('3.11')(property(lambda self: self._fontNames))
    multi_byte_charprocs = _api.deprecated('3.11')(property(lambda _: {}))
    type1Descriptors = _api.deprecated('3.11')(property(lambda _: {}))

    @_api.deprecated('3.11')
    @property
    def dviFontInfo(self):
        ...

    def newPage(self, width, height):
        self.width, self.height = (width, height)
        self.pageAnnotations = []

    def newTextnote(self, text, positionRect=[-100, -100, 0, 0]):
        ...

    @staticmethod
    def _get_subset_prefix(charset):
        ...

    @staticmethod
    def _get_subsetted_psname(ps_name, charmap):
        ...

    def finalize(self):
        ...

    def close(self):
        ...

    def write(self, data):
        ...

    def output(self, *data):
        ...

    def beginStream(self, id, len, extra=None, png=None):
        self.currentstream = Stream(id, len, self, extra, png)

    def endStream(self):
        self.currentstream = None

    def outputStream(self, ref, data, *, extra=None):
        ...

    def _write_annotations(self):
        ...

    def fontName(self, fontprop, subset=0):
        self._fontNames[fname, subset] = Fx

    def dviFontName(self, dvifont):
        self._dviFontInfo[pdfname] = dvifont

    def writeFonts(self):
        ...

    def _write_afm_font(self, filename):
        ...

    def _embedTeXFont(self, dvifont):
        ...

    def _generate_encoding(self, encoding):
        ...

    @_api.delete_parameter('3.11', 'fontfile')
    def createType1Descriptor(self, t1font, fontfile=None):
        ...
    _identityToUnicodeCMap = b'/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo\n<< /Registry (Adobe)\n   /Ordering (UCS)\n   /Supplement 0\n>> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <ffff>\nendcodespacerange\n%d beginbfrange\n%s\nendbfrange\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend'

    def embedTTF(self, filename, subset_index, charmap):
        ...

    def alphaState(self, alpha):
        self.alphaStates[alpha] = (name, {'Type': Name('ExtGState'), 'CA': alpha[0], 'ca': alpha[1]})

    def _soft_mask_state(self, smask):
        self._soft_mask_states[smask] = (name, {'Type': Name('ExtGState'), 'AIS': False, 'SMask': {'Type': Name('Mask'), 'S': Name('Luminosity'), 'BC': [1], 'G': groupOb}})

    def writeExtGSTates(self):
        ...

    def _write_soft_mask_groups(self):
        ...

    def hatchPattern(self, hatch_style):
        self._hatch_patterns[hatch_style] = name
    hatchPatterns = _api.deprecated('3.10')(property(lambda self: {k: (e, f, h) for k, (e, f, h, l) in self._hatch_patterns.items()}))

    def writeHatches(self):
        ...

    def addGouraudTriangles(self, points, colors):
        ...

    def writeGouraudTriangles(self):
        ...

    def imageObject(self, image):
        self._images[id(image)] = (image, name, ob)

    def _unpack(self, im):
        ...

    def _writePng(self, img):
        ...

    def _writeImg(self, data, id, smask=None):
        ...

    def writeImages(self):
        ...

    def markerObject(self, path, trans, fill, stroke, lw, joinstyle, capstyle):
        self.markers[key] = [name, ob, bbox, lw]

    def writeMarkers(self):
        ...

    def pathCollectionObject(self, gc, path, trans, padding, filled, stroked):
        ...

    def writePathCollectionTemplates(self):
        ...

    @staticmethod
    def pathOperations(path, transform, clip=None, simplify=None, sketch=None):
        ...

    def writePath(self, path, transform, clip=False, sketch=None):
        ...

    def reserveObject(self, name=''):
        ...

    def recordXref(self, id):
        self.xrefTable[id][0] = self.fh.tell() - self.tell_base

    def writeObject(self, object, contents):
        ...

    def writeXref(self):
        self.startxref = self.fh.tell() - self.tell_base

    def writeInfoDict(self):
        self.infoObject = self.reserveObject('info')

    def writeTrailer(self):
        ...

class RendererPdf(_backend_pdf_ps.RendererPDFPSBase):
    _afm_font_dir = cbook._get_data_path('fonts/pdfcorefonts')
    _use_afm_rc_name = 'pdf.use14corefonts'

    def __init__(self, file, image_dpi, height, width):
        self.file = file
        self.gc = self.new_gc()
        self.image_dpi = image_dpi

    def finalize(self):
        ...

    def check_gc(self, gc, fillcolor=None):
        ...

    def get_image_magnification(self):
        ...

    def draw_image(self, gc, x, y, im, transform=None):
        ...

    def draw_path(self, gc, path, transform, rgbFace=None):
        ...

    def draw_path_collection(self, gc, master_transform, paths, all_transforms, offsets, offset_trans, facecolors, edgecolors, linewidths, linestyles, antialiaseds, urls, offset_position, *, hatchcolors=None):
        ...

    def draw_markers(self, gc, marker_path, marker_trans, path, trans, rgbFace=None):
        ...

    def draw_gouraud_triangles(self, gc, points, colors, trans):
        ...

    def _setup_textpos(self, x, y, angle, oldx=0, oldy=0, oldangle=0):
        ...

    def draw_mathtext(self, gc, x, y, s, prop, angle):
        ...

    def draw_tex(self, gc, x, y, s, prop, angle, *, mtext=None):
        ...

    def _encode_glyphs(self, subset, fonttype):
        ...

    def encode_string(self, s, fonttype):
        ...

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        ...

    def new_gc(self):
        ...

class GraphicsContextPdf(GraphicsContextBase):

    def __init__(self, file):
        self._fillcolor = (0.0, 0.0, 0.0)
        self._effective_alphas = (1.0, 1.0)
        self.file = file
        self.parent = None

    def __repr__(self):
        ...

    def stroke(self):
        ...

    def fill(self, *args):
        ...

    def paint(self):
        ...
    capstyles = {'butt': 0, 'round': 1, 'projecting': 2}
    joinstyles = {'miter': 0, 'round': 1, 'bevel': 2}

    def capstyle_cmd(self, style):
        ...

    def joinstyle_cmd(self, style):
        ...

    def linewidth_cmd(self, width):
        ...

    def dash_cmd(self, dashes):
        ...

    def alpha_cmd(self, alpha, forced, effective_alphas):
        ...

    def hatch_cmd(self, hatch, hatch_color, hatch_linewidth):
        ...

    def rgb_cmd(self, rgb):
        ...

    def fillcolor_cmd(self, rgb):
        ...

    def push(self):
        self.parent = parent

    def pop(self):
        self.parent = self.parent.parent

    def clip_cmd(self, cliprect, clippath):
        ...
    commands = ((('_cliprect', '_clippath'), clip_cmd), (('_alpha', '_forced_alpha', '_effective_alphas'), alpha_cmd), (('_capstyle',), capstyle_cmd), (('_fillcolor',), fillcolor_cmd), (('_joinstyle',), joinstyle_cmd), (('_linewidth',), linewidth_cmd), (('_dashes',), dash_cmd), (('_rgb',), rgb_cmd), (('_hatch', '_hatch_color', '_hatch_linewidth'), hatch_cmd))

    def delta(self, other):
        ...

    def copy_properties(self, other):
        self._fillcolor = fillcolor
        self._effective_alphas = effective_alphas

    def finalize(self):
        ...

class PdfPages:

    @_api.delete_parameter('3.10', 'keep_empty', addendum='This parameter does nothing.')
    def __init__(self, filename, keep_empty=None, metadata=None):
        self._filename = filename
        self._metadata = metadata
        self._file = None

    def __enter__(self):
        ...

    def __exit__(self, exc_type, exc_val, exc_tb):
        ...

    def _ensure_file(self):
        self._file = PdfFile(self._filename, metadata=self._metadata)

    def close(self):
        self._file = None

    def infodict(self):
        ...

    def savefig(self, figure=None, **kwargs):
        ...

    def get_pagecount(self):
        ...

    def attach_note(self, text, positionRect=[-100, -100, 0, 0]):
        ...

class FigureCanvasPdf(FigureCanvasBase):
    fixed_dpi = 72
    filetypes = {'pdf': 'Portable Document Format'}

    def get_default_filetype(self):
        ...

    def print_pdf(self, filename, *, bbox_inches_restore=None, metadata=None):
        self.figure.dpi = 72

    def draw(self):
        ...
FigureManagerPdf = FigureManagerBase

@_Backend.export
class _BackendPdf(_Backend):
    FigureCanvas = FigureCanvasPdf
__all__ = ['codecs', 'timezone', 'datetime', 'Enum', 'total_ordering', 'BytesIO', 'itertools', 'logging', 'math', 'os', 'string', 'struct', 'sys', 'time', 'types', 'zlib', 'np', 'Image', 'mpl', 'cbook', 'dviread', 'Gcf', 'FigureCanvasBase', 'FigureManagerBase', 'GraphicsContextBase', 'RendererBase', 'MixedModeRenderer', 'Figure', 'FontPath', 'get_font', 'AFM', 'FT2Font', 'FaceFlags', 'LoadFlags', 'StyleFlags', 'Affine2D', 'BboxBase', 'Path', 'UTC', 'pdfRepr', 'Reference', 'Name', 'Verbatim', 'Op', 'Stream', 'PdfFile', 'RendererPdf', 'GraphicsContextPdf', 'PdfPages', 'FigureCanvasPdf', 'FigureManagerPdf']
