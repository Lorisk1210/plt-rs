# The type stub of matplotlib.backends._backend_tk: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import uuid
import weakref
from contextlib import contextmanager
import logging
import math
import os.path
import pathlib
import sys
import tkinter as tk
import tkinter.filedialog
import tkinter.font
import tkinter.messagebox
from tkinter.simpledialog import SimpleDialog
import numpy as np
from PIL import Image, ImageTk
import matplotlib as mpl
from matplotlib import _api, backend_tools, cbook, _c_internal_utils
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, NavigationToolbar2, TimerBase, ToolContainerBase, cursors, _Mode, MouseButton, CloseEvent, KeyEvent, LocationEvent, MouseEvent, ResizeEvent
from matplotlib._pylab_helpers import Gcf
try:
    from . import _tkagg
    from ._tkagg import TK_PHOTO_COMPOSITE_OVERLAY, TK_PHOTO_COMPOSITE_SET
except ImportError as e:
    cause1 = getattr(e, '__cause__', None)
    cause2 = getattr(cause1, '__cause__', None)
    if isinstance(cause1, ImportError) and isinstance(cause2, AttributeError) and ("'_tkinter' has no attribute '__file__'" in str(cause2)):
        is_uv_python = '/uv/python' in os.path.realpath(sys.executable)
        if is_uv_python:
            raise ImportError("Failed to import tkagg backend. You appear to be using an outdated version of uv's managed Python distribution which is not compatible with Tk. Please upgrade to the latest uv version, then update Python with: `uv python upgrade --reinstall`") from e
        else:
            raise ImportError('Failed to import tkagg backend. This is likely caused by using a Python executable based on python-build-standalone, which is not compatible with Tk. Recent versions of python-build-standalone should be compatible with Tk. Please update your python version or select another backend.') from e
    else:
        raise
_log = logging.getLogger(__name__)
cursord = {cursors.MOVE: 'fleur', cursors.HAND: 'hand2', cursors.POINTER: 'arrow', cursors.SELECT_REGION: 'crosshair', cursors.WAIT: 'watch', cursors.RESIZE_HORIZONTAL: 'sb_h_double_arrow', cursors.RESIZE_VERTICAL: 'sb_v_double_arrow'}

@contextmanager
def _restore_foreground_window_at_end():
    ...
_blit_args = {}
_blit_tcl_name = 'mpl_blit_' + uuid.uuid4().hex

def _blit(argsid):
    ...

def blit(photoimage, aggimage, offsets, bbox=None):
    ...

class TimerTk(TimerBase):

    def __init__(self, parent, *args, **kwargs):
        self._timer = None
        self.parent = parent

    def _timer_start(self):
        self._timer = self.parent.after(self._interval, self._on_timer)

    def _timer_stop(self):
        self._timer = None

    def _on_timer(self):
        self._timer = None
        self._timer = self.parent.after(self._interval, self._on_timer)
        self._timer = self.parent.after_idle(lambda: self.parent.after(self._interval, self._on_timer))

class FigureCanvasTk(FigureCanvasBase):
    required_interactive_framework = 'tk'
    manager_class = _api.classproperty(lambda cls: FigureManagerTk)

    def __init__(self, figure=None, master=None):
        self._idle_draw_id = None
        self._event_loop_id = None
        self._tkcanvas = tk.Canvas(master=master, background='white', width=w, height=h, borderwidth=0, highlightthickness=0)
        self._tkphoto = tk.PhotoImage(master=self._tkcanvas, width=w, height=h)
        self._tkcanvas_image_region = self._tkcanvas.create_image(w // 2, h // 2, image=self._tkphoto)
        self._rubberband_rect_black = None
        self._rubberband_rect_white = None

    def _update_device_pixel_ratio(self, event=None):
        ...

    def resize(self, event):
        ...

    def _resize_figure_for_canvas_size(self, width, height):
        self._tkcanvas_image_region = self._tkcanvas.create_image(int(width / 2), int(height / 2), image=self._tkphoto)

    def draw_idle(self):
        self._idle_draw_id = self._tkcanvas.after_idle(idle_draw)

    def get_tk_widget(self):
        ...

    def _event_mpl_coords(self, event):
        ...

    def motion_notify_event(self, event):
        ...

    def enter_notify_event(self, event):
        ...

    def leave_notify_event(self, event):
        ...

    def button_press_event(self, event, dblclick=False):
        ...

    def button_dblclick_event(self, event):
        ...

    def button_release_event(self, event):
        ...

    def scroll_event(self, event):
        ...

    def scroll_event_windows(self, event):
        ...

    @staticmethod
    def _mpl_buttons(event):
        ...

    @staticmethod
    def _mpl_modifiers(event, *, exclude=None):
        ...

    def _get_key(self, event):
        ...

    def key_press(self, event):
        ...

    def key_release(self, event):
        ...

    def new_timer(self, *args, **kwargs):
        ...

    def flush_events(self):
        ...

    def start_event_loop(self, timeout=0):
        self._event_loop_id = self._tkcanvas.after(milliseconds, self.stop_event_loop)
        self._event_loop_id = self._tkcanvas.after_idle(self.stop_event_loop)

    def stop_event_loop(self):
        self._event_loop_id = None

    def set_cursor(self, cursor):
        ...

class FigureManagerTk(FigureManagerBase):
    _owns_mainloop = False

    def __init__(self, canvas, num, window):
        self.window = window
        self._window_dpi = tk.IntVar(master=window, value=96, name=f'window_dpi{window_frame}')
        self._window_dpi_cbname = ''
        self._shown = False
        self._window_dpi_cbname = self._window_dpi.trace_add('write', self._update_window_dpi)

    @classmethod
    def create_with_canvas(cls, canvas_class, figure, num):
        ...

    @classmethod
    def start_main_loop(cls):
        ...

    def _update_window_dpi(self, *args):
        ...

    def resize(self, width, height):
        ...

    def show(self):
        self._shown = True

    def destroy(self, *args):
        ...

    def get_window_title(self):
        ...

    def set_window_title(self, title):
        ...

    def full_screen_toggle(self):
        ...

class NavigationToolbar2Tk(NavigationToolbar2, tk.Frame):

    def __init__(self, canvas, window=None, *, pack_toolbar=True):
        self._buttons = {}
        self._label_font = tkinter.font.Font(root=window, size=10)
        self.message = tk.StringVar(master=self)
        self._message_label = tk.Label(master=self, font=self._label_font, textvariable=self.message, justify=tk.RIGHT)
        self._buttons[text] = button = self._Button(text, str(cbook._get_data_path(f'images/{image_file}.png')), toggle=callback in ['zoom', 'pan'], command=getattr(self, callback))

    def _rescale(self):
        ...

    def _update_buttons_checked(self):
        ...

    def pan(self, *args):
        ...

    def zoom(self, *args):
        ...

    def set_message(self, s):
        ...

    def draw_rubberband(self, event, x0, y0, x1, y1):
        self.canvas._rubberband_rect_black = self.canvas._tkcanvas.create_rectangle(x0, y0, x1, y1, outline='black')
        self.canvas._rubberband_rect_white = self.canvas._tkcanvas.create_rectangle(x0, y0, x1, y1, outline='white', dash=(3, 3))

    def remove_rubberband(self):
        self.canvas._rubberband_rect_white = None
        self.canvas._rubberband_rect_black = None

    def _set_image_for_button(self, button):
        ...

    def _Button(self, text, image_file, toggle, command):
        ...

    def _Spacer(self):
        ...

    def save_figure(self, *args):
        ...

    def set_history_buttons(self):
        self._buttons['Back']['state'] = state_map[can_back]
        self._buttons['Forward']['state'] = state_map[can_forward]

def add_tooltip(widget, text):
    ...

@backend_tools._register_tool_class(FigureCanvasTk)
class RubberbandTk(backend_tools.RubberbandBase):

    def draw_rubberband(self, x0, y0, x1, y1):
        ...

    def remove_rubberband(self):
        ...

class ToolbarTk(ToolContainerBase, tk.Frame):

    def __init__(self, toolmanager, window=None):
        self._label_font = tkinter.font.Font(size=10)
        self._message = tk.StringVar(master=self)
        self._message_label = tk.Label(master=self, font=self._label_font, textvariable=self._message)
        self._toolitems = {}
        self._groups = {}

    def _rescale(self):
        ...

    def add_toolitem(self, name, group, position, image_file, description, toggle):
        ...

    def _get_groupframe(self, group):
        self._groups[group] = frame

    def _add_separator(self):
        ...

    def _button_click(self, name):
        ...

    def toggle_toolitem(self, name, toggled):
        ...

    def remove_toolitem(self, name):
        ...

    def set_message(self, s):
        ...

@backend_tools._register_tool_class(FigureCanvasTk)
class SaveFigureTk(backend_tools.SaveFigureBase):

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(FigureCanvasTk)
class ConfigureSubplotsTk(backend_tools.ConfigureSubplotsBase):

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(FigureCanvasTk)
class HelpTk(backend_tools.ToolHelpBase):

    def trigger(self, *args):
        ...
Toolbar = ToolbarTk
FigureManagerTk._toolbar2_class = NavigationToolbar2Tk
FigureManagerTk._toolmanager_toolbar_class = ToolbarTk

@_Backend.export
class _BackendTk(_Backend):
    backend_version = tk.TkVersion
    FigureCanvas = FigureCanvasTk
    FigureManager = FigureManagerTk
    mainloop = FigureManagerTk.start_main_loop
__all__ = ['uuid', 'weakref', 'contextmanager', 'logging', 'math', 'os', 'pathlib', 'sys', 'tk', 'tkinter', 'SimpleDialog', 'np', 'Image', 'ImageTk', 'mpl', 'backend_tools', 'cbook', 'FigureCanvasBase', 'FigureManagerBase', 'NavigationToolbar2', 'TimerBase', 'ToolContainerBase', 'cursors', 'MouseButton', 'CloseEvent', 'KeyEvent', 'LocationEvent', 'MouseEvent', 'ResizeEvent', 'Gcf', 'cursord', 'blit', 'TimerTk', 'FigureCanvasTk', 'FigureManagerTk', 'NavigationToolbar2Tk', 'add_tooltip', 'RubberbandTk', 'ToolbarTk', 'SaveFigureTk', 'ConfigureSubplotsTk', 'HelpTk', 'Toolbar']
