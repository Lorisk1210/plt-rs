# The type stub of matplotlib.backends.backend_wx: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import functools
import logging
import math
import pathlib
import sys
import weakref
import numpy as np
import PIL.Image
import matplotlib as mpl
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, GraphicsContextBase, MouseButton, NavigationToolbar2, RendererBase, TimerBase, ToolContainerBase, cursors, CloseEvent, KeyEvent, LocationEvent, MouseEvent, ResizeEvent
from matplotlib import _api, cbook, backend_tools, _c_internal_utils
from matplotlib._pylab_helpers import Gcf
from matplotlib.path import Path
from matplotlib.transforms import Affine2D
import wx
import wx.svg
_log = logging.getLogger(__name__)
PIXELS_PER_INCH = 75

@functools.lru_cache(1)
def _create_wxapp():
    ...

class TimerWx(TimerBase):

    def __init__(self, *args, **kwargs):
        self._timer = wx.Timer()
        self._timer.Notify = self._on_timer

    def _timer_start(self):
        ...

    def _timer_stop(self):
        ...

    def _timer_set_interval(self):
        ...

@_api.deprecated('2.0', name='wx', obj_type='backend', removal='the future', alternative='wxagg', addendum='See the Matplotlib usage FAQ for more info on backends.')
class RendererWx(RendererBase):
    fontweights = {100: wx.FONTWEIGHT_LIGHT, 200: wx.FONTWEIGHT_LIGHT, 300: wx.FONTWEIGHT_LIGHT, 400: wx.FONTWEIGHT_NORMAL, 500: wx.FONTWEIGHT_NORMAL, 600: wx.FONTWEIGHT_NORMAL, 700: wx.FONTWEIGHT_BOLD, 800: wx.FONTWEIGHT_BOLD, 900: wx.FONTWEIGHT_BOLD, 'ultralight': wx.FONTWEIGHT_LIGHT, 'light': wx.FONTWEIGHT_LIGHT, 'normal': wx.FONTWEIGHT_NORMAL, 'medium': wx.FONTWEIGHT_NORMAL, 'semibold': wx.FONTWEIGHT_NORMAL, 'bold': wx.FONTWEIGHT_BOLD, 'heavy': wx.FONTWEIGHT_BOLD, 'ultrabold': wx.FONTWEIGHT_BOLD, 'black': wx.FONTWEIGHT_BOLD}
    fontangles = {'italic': wx.FONTSTYLE_ITALIC, 'normal': wx.FONTSTYLE_NORMAL, 'oblique': wx.FONTSTYLE_SLANT}
    fontnames = {'Sans': wx.FONTFAMILY_SWISS, 'Roman': wx.FONTFAMILY_ROMAN, 'Script': wx.FONTFAMILY_SCRIPT, 'Decorative': wx.FONTFAMILY_DECORATIVE, 'Modern': wx.FONTFAMILY_MODERN, 'Courier': wx.FONTFAMILY_MODERN, 'courier': wx.FONTFAMILY_MODERN}

    def __init__(self, bitmap, dpi):
        self.width = bitmap.GetWidth()
        self.height = bitmap.GetHeight()
        self.bitmap = bitmap
        self.fontd = {}
        self.dpi = dpi
        self.gc = None

    def flipy(self):
        ...

    def get_text_width_height_descent(self, s, prop, ismath):
        ...

    def get_canvas_width_height(self):
        ...

    def handle_clip_rectangle(self, gc):
        ...

    @staticmethod
    def convert_path(gfx_ctx, path, transform):
        ...

    def draw_path(self, gc, path, transform, rgbFace=None):
        ...

    def draw_image(self, gc, x, y, im):
        ...

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        ...

    def new_gc(self):
        self.gc = GraphicsContextWx(self.bitmap, self)

    def get_wx_font(self, s, prop):
        self.fontd[key] = font = wx.Font(pointSize=round(size), family=self.fontnames.get(prop.get_name(), wx.ROMAN), style=self.fontangles[prop.get_style()], weight=self.fontweights[prop.get_weight()])

    def points_to_pixels(self, points):
        ...

class GraphicsContextWx(GraphicsContextBase):
    _capd = {'butt': wx.CAP_BUTT, 'projecting': wx.CAP_PROJECTING, 'round': wx.CAP_ROUND}
    _joind = {'bevel': wx.JOIN_BEVEL, 'miter': wx.JOIN_MITER, 'round': wx.JOIN_ROUND}
    _cache = weakref.WeakKeyDictionary()

    def __init__(self, bitmap, renderer):
        self.bitmap = bitmap
        self.dc = dc
        self.gfx_ctx = gfx_ctx
        self._pen = wx.Pen('BLACK', 1, wx.SOLID)
        self.renderer = renderer
        self._cache[bitmap] = (dc, gfx_ctx)

    def select(self):
        self.IsSelected = True

    def unselect(self):
        self.IsSelected = False

    def set_foreground(self, fg, isRGBA=None):
        ...

    def set_linewidth(self, w):
        ...

    def set_capstyle(self, cs):
        ...

    def set_joinstyle(self, js):
        ...

    def get_wxcolour(self, color):
        ...

class _FigureCanvasWxBase(FigureCanvasBase, wx.Panel):
    required_interactive_framework = 'wx'
    _timer_cls = TimerWx
    manager_class = _api.classproperty(lambda cls: FigureManagerWx)
    keyvald = {wx.WXK_CONTROL: 'control', wx.WXK_SHIFT: 'shift', wx.WXK_ALT: 'alt', wx.WXK_CAPITAL: 'caps_lock', wx.WXK_LEFT: 'left', wx.WXK_UP: 'up', wx.WXK_RIGHT: 'right', wx.WXK_DOWN: 'down', wx.WXK_ESCAPE: 'escape', wx.WXK_F1: 'f1', wx.WXK_F2: 'f2', wx.WXK_F3: 'f3', wx.WXK_F4: 'f4', wx.WXK_F5: 'f5', wx.WXK_F6: 'f6', wx.WXK_F7: 'f7', wx.WXK_F8: 'f8', wx.WXK_F9: 'f9', wx.WXK_F10: 'f10', wx.WXK_F11: 'f11', wx.WXK_F12: 'f12', wx.WXK_SCROLL: 'scroll_lock', wx.WXK_PAUSE: 'break', wx.WXK_BACK: 'backspace', wx.WXK_RETURN: 'enter', wx.WXK_INSERT: 'insert', wx.WXK_DELETE: 'delete', wx.WXK_HOME: 'home', wx.WXK_END: 'end', wx.WXK_PAGEUP: 'pageup', wx.WXK_PAGEDOWN: 'pagedown', wx.WXK_NUMPAD0: '0', wx.WXK_NUMPAD1: '1', wx.WXK_NUMPAD2: '2', wx.WXK_NUMPAD3: '3', wx.WXK_NUMPAD4: '4', wx.WXK_NUMPAD5: '5', wx.WXK_NUMPAD6: '6', wx.WXK_NUMPAD7: '7', wx.WXK_NUMPAD8: '8', wx.WXK_NUMPAD9: '9', wx.WXK_NUMPAD_ADD: '+', wx.WXK_NUMPAD_SUBTRACT: '-', wx.WXK_NUMPAD_MULTIPLY: '*', wx.WXK_NUMPAD_DIVIDE: '/', wx.WXK_NUMPAD_DECIMAL: 'dec', wx.WXK_NUMPAD_ENTER: 'enter', wx.WXK_NUMPAD_UP: 'up', wx.WXK_NUMPAD_RIGHT: 'right', wx.WXK_NUMPAD_DOWN: 'down', wx.WXK_NUMPAD_LEFT: 'left', wx.WXK_NUMPAD_PAGEUP: 'pageup', wx.WXK_NUMPAD_PAGEDOWN: 'pagedown', wx.WXK_NUMPAD_HOME: 'home', wx.WXK_NUMPAD_END: 'end', wx.WXK_NUMPAD_INSERT: 'insert', wx.WXK_NUMPAD_DELETE: 'delete'}

    def __init__(self, parent, id, figure=None):
        self.bitmap = None
        self._isDrawn = False
        self._rubberband_rect = None
        self._rubberband_pen_black = wx.Pen('BLACK', 1, wx.PENSTYLE_SHORT_DASH)
        self._rubberband_pen_white = wx.Pen('WHITE', 1, wx.PENSTYLE_SOLID)

    def Copy_to_Clipboard(self, event=None):
        ...

    def _update_device_pixel_ratio(self, *args, **kwargs):
        ...

    def draw_idle(self):
        self._isDrawn = False

    def flush_events(self):
        ...

    def start_event_loop(self, timeout=0):
        self._event_loop = wx.GUIEventLoop()

    def stop_event_loop(self, event=None):
        ...

    def _get_imagesave_wildcards(self):
        ...

    def gui_repaint(self, drawDC=None):
        ...
    filetypes = {**FigureCanvasBase.filetypes, 'bmp': 'Windows bitmap', 'jpeg': 'JPEG', 'jpg': 'JPEG', 'pcx': 'PCX', 'png': 'Portable Network Graphics', 'tif': 'Tagged Image Format File', 'tiff': 'Tagged Image Format File', 'xpm': 'X pixmap'}

    def _on_paint(self, event):
        ...

    def _on_size(self, event):
        self._width, self._height = size
        self._isDrawn = False

    @staticmethod
    def _mpl_buttons():
        ...

    @staticmethod
    def _mpl_modifiers(event=None, *, exclude=None):
        ...

    def _get_key(self, event):
        ...

    def _mpl_coords(self, pos=None):
        ...

    def _on_key_down(self, event):
        ...

    def _on_key_up(self, event):
        ...

    def set_cursor(self, cursor):
        ...

    def _set_capture(self, capture=True):
        ...

    def _on_capture_lost(self, event):
        ...

    def _on_mouse_button(self, event):
        ...

    def _on_mouse_wheel(self, event):
        self._skipwheelevent = True
        self._skipwheelevent = False
        self._skipwheelevent = True

    def _on_motion(self, event):
        ...

    def _on_enter(self, event):
        ...

    def _on_leave(self, event):
        ...

class FigureCanvasWx(_FigureCanvasWxBase):

    def draw(self, drawDC=None):
        self.renderer = RendererWx(self.bitmap, self.figure.dpi)
        self._isDrawn = True

    def _print_image(self, filetype, filename):
        ...
    print_bmp = functools.partialmethod(_print_image, wx.BITMAP_TYPE_BMP)
    print_jpeg = print_jpg = functools.partialmethod(_print_image, wx.BITMAP_TYPE_JPEG)
    print_pcx = functools.partialmethod(_print_image, wx.BITMAP_TYPE_PCX)
    print_png = functools.partialmethod(_print_image, wx.BITMAP_TYPE_PNG)
    print_tiff = print_tif = functools.partialmethod(_print_image, wx.BITMAP_TYPE_TIF)
    print_xpm = functools.partialmethod(_print_image, wx.BITMAP_TYPE_XPM)

class FigureFrameWx(wx.Frame):

    def __init__(self, num, fig, *, canvas_class):
        self.canvas = canvas_class(self, -1, fig)

    def _on_close(self, event):
        self.canvas.manager.frame = None

class FigureManagerWx(FigureManagerBase):

    def __init__(self, canvas, num, frame):
        self.frame = self.window = frame

    @classmethod
    def create_with_canvas(cls, canvas_class, figure, num):
        ...

    @classmethod
    def start_main_loop(cls):
        ...

    def show(self):
        ...

    def destroy(self, *args):
        ...

    def full_screen_toggle(self):
        ...

    def get_window_title(self):
        ...

    def set_window_title(self, title):
        ...

    def resize(self, width, height):
        ...

def _load_bitmap(filename):
    ...

def _set_frame_icon(frame):
    ...

class NavigationToolbar2Wx(NavigationToolbar2, wx.ToolBar):

    def __init__(self, canvas, coordinates=True, *, style=wx.TB_BOTTOM):
        self.wx_ids = {}
        self._coordinates = coordinates
        self.wx_ids[text] = self.AddTool(-1, bitmap=self._icon(f'{image_file}.svg'), bmpDisabled=wx.NullBitmap, label=text, shortHelp=tooltip_text, kind=wx.ITEM_CHECK if text in ['Pan', 'Zoom'] else wx.ITEM_NORMAL).Id
        self._label_text = wx.StaticText(self, style=wx.ALIGN_RIGHT)

    @staticmethod
    def _icon(name):
        ...

    def _update_buttons_checked(self):
        ...

    def zoom(self, *args):
        ...

    def pan(self, *args):
        ...

    def save_figure(self, *args):
        ...

    def draw_rubberband(self, event, x0, y0, x1, y1):
        self.canvas._rubberband_rect = (x0 / sf, (height - y0) / sf, x1 / sf, (height - y1) / sf)

    def remove_rubberband(self):
        self.canvas._rubberband_rect = None

    def set_message(self, s):
        ...

    def set_history_buttons(self):
        ...

class ToolbarWx(ToolContainerBase, wx.ToolBar):
    _icon_extension = '.svg'

    def __init__(self, toolmanager, parent=None, style=wx.TB_BOTTOM):
        self._space = self.AddStretchableSpace()
        self._label_text = wx.StaticText(self, style=wx.ALIGN_RIGHT)
        self._toolitems = {}
        self._groups = {}

    def _get_tool_pos(self, tool):
        ...

    def add_toolitem(self, name, group, position, image_file, description, toggle):
        self._groups[group] = self.InsertSeparator(self._get_tool_pos(self._space))

    def toggle_toolitem(self, name, toggled):
        ...

    def remove_toolitem(self, name):
        ...

    def set_message(self, s):
        ...

@backend_tools._register_tool_class(_FigureCanvasWxBase)
class ConfigureSubplotsWx(backend_tools.ConfigureSubplotsBase):

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(_FigureCanvasWxBase)
class SaveFigureWx(backend_tools.SaveFigureBase):

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(_FigureCanvasWxBase)
class RubberbandWx(backend_tools.RubberbandBase):

    def draw_rubberband(self, x0, y0, x1, y1):
        ...

    def remove_rubberband(self):
        ...

class _HelpDialog(wx.Dialog):
    _instance = None
    headers = [('Action', 'Shortcuts', 'Description')]
    widths = [100, 140, 300]

    def __init__(self, parent, help_entries):
        ...

    def _on_close(self, event):
        ...

    @classmethod
    def show(cls, parent, help_entries):
        cls._instance = cls(parent, help_entries)

@backend_tools._register_tool_class(_FigureCanvasWxBase)
class HelpWx(backend_tools.ToolHelpBase):

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(_FigureCanvasWxBase)
class ToolCopyToClipboardWx(backend_tools.ToolCopyToClipboardBase):

    def trigger(self, *args, **kwargs):
        ...
FigureManagerWx._toolbar2_class = NavigationToolbar2Wx
FigureManagerWx._toolmanager_toolbar_class = ToolbarWx

@_Backend.export
class _BackendWx(_Backend):
    FigureCanvas = FigureCanvasWx
    FigureManager = FigureManagerWx
    mainloop = FigureManagerWx.start_main_loop
__all__ = ['functools', 'logging', 'math', 'pathlib', 'sys', 'weakref', 'np', 'PIL', 'mpl', 'FigureCanvasBase', 'FigureManagerBase', 'GraphicsContextBase', 'MouseButton', 'NavigationToolbar2', 'RendererBase', 'TimerBase', 'ToolContainerBase', 'cursors', 'CloseEvent', 'KeyEvent', 'LocationEvent', 'MouseEvent', 'ResizeEvent', 'cbook', 'backend_tools', 'Gcf', 'Path', 'Affine2D', 'wx', 'PIXELS_PER_INCH', 'TimerWx', 'RendererWx', 'GraphicsContextWx', 'FigureCanvasWx', 'FigureFrameWx', 'FigureManagerWx', 'NavigationToolbar2Wx', 'ToolbarWx', 'ConfigureSubplotsWx', 'SaveFigureWx', 'RubberbandWx', 'HelpWx', 'ToolCopyToClipboardWx']
