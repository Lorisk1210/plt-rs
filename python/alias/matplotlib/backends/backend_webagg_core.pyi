# The type stub of matplotlib.backends.backend_webagg_core: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import asyncio
import datetime
from io import BytesIO, StringIO
import json
import logging
import os
from pathlib import Path
import numpy as np
from PIL import Image
from matplotlib import _api, backend_bases, backend_tools
from matplotlib.backends import backend_agg
from matplotlib.backend_bases import _Backend, MouseButton, KeyEvent, LocationEvent, MouseEvent, ResizeEvent
_log = logging.getLogger(__name__)
_SPECIAL_KEYS_LUT = {'Alt': 'alt', 'AltGraph': 'alt', 'CapsLock': 'caps_lock', 'Control': 'control', 'Meta': 'meta', 'NumLock': 'num_lock', 'ScrollLock': 'scroll_lock', 'Shift': 'shift', 'Super': 'super', 'Enter': 'enter', 'Tab': 'tab', 'ArrowDown': 'down', 'ArrowLeft': 'left', 'ArrowRight': 'right', 'ArrowUp': 'up', 'End': 'end', 'Home': 'home', 'PageDown': 'pagedown', 'PageUp': 'pageup', 'Backspace': 'backspace', 'Delete': 'delete', 'Insert': 'insert', 'Escape': 'escape', 'Pause': 'pause', 'Select': 'select', 'Dead': 'dead', 'F1': 'f1', 'F2': 'f2', 'F3': 'f3', 'F4': 'f4', 'F5': 'f5', 'F6': 'f6', 'F7': 'f7', 'F8': 'f8', 'F9': 'f9', 'F10': 'f10', 'F11': 'f11', 'F12': 'f12'}

def _handle_key(key):
    ...

class TimerTornado(backend_bases.TimerBase):

    def __init__(self, *args, **kwargs):
        self._timer = None

    def _timer_start(self):
        self._timer = ioloop.add_timeout(datetime.timedelta(milliseconds=self.interval), self._on_timer)
        self._timer = tornado.ioloop.PeriodicCallback(self._on_timer, max(self.interval, 1e-06))

    def _timer_stop(self):
        self._timer = None

    def _timer_set_interval(self):
        ...

class TimerAsyncio(backend_bases.TimerBase):

    def __init__(self, *args, **kwargs):
        self._task = None

    async def _timer_task(self, interval):
        ...

    def _timer_start(self):
        self._task = asyncio.ensure_future(self._timer_task(max(self.interval / 1000.0, 1e-06)))

    def _timer_stop(self):
        self._task = None

    def _timer_set_interval(self):
        ...

class FigureCanvasWebAggCore(backend_agg.FigureCanvasAgg):
    manager_class = _api.classproperty(lambda cls: FigureManagerWebAgg)
    _timer_cls = TimerAsyncio
    supports_blit = False

    def __init__(self, *args, **kwargs):
        self._png_is_old = True
        self._force_full = True
        self._last_buff = np.empty((0, 0))
        self._current_image_mode = 'full'
        self._last_mouse_xy = (None, None)
        self._capture_scroll = False

    def show(self):
        ...

    def draw(self):
        self._png_is_old = True

    def blit(self, bbox=None):
        self._png_is_old = True

    def draw_idle(self):
        ...

    def set_cursor(self, cursor):
        ...

    def set_image_mode(self, mode):
        self._current_image_mode = mode

    def set_capture_scroll(self, capture):
        self._capture_scroll = capture

    def get_capture_scroll(self):
        ...

    def get_diff_image(self):
        self._last_buff = buff.copy()
        self._force_full = False
        self._png_is_old = False

    def handle_event(self, event):
        ...

    def handle_unknown_event(self, event):
        ...

    def handle_ack(self, event):
        ...

    def handle_draw(self, event):
        ...

    def _handle_mouse(self, event):
        self._last_mouse_xy = (x, y)
    handle_button_press = handle_button_release = handle_dblclick = handle_figure_enter = handle_figure_leave = handle_motion_notify = handle_scroll = _handle_mouse

    def _handle_key(self, event):
        ...
    handle_key_press = handle_key_release = _handle_key

    def handle_toolbar_button(self, event):
        ...

    def handle_refresh(self, event):
        self._force_full = True

    def handle_resize(self, event):
        self._png_is_old = True

    def handle_send_image_mode(self, event):
        ...

    def handle_set_device_pixel_ratio(self, event):
        ...

    def handle_set_dpi_ratio(self, event):
        ...

    def _handle_set_device_pixel_ratio(self, device_pixel_ratio):
        self._force_full = True

    def send_event(self, event_type, **kwargs):
        ...
_ALLOWED_TOOL_ITEMS = {'home', 'back', 'forward', 'pan', 'zoom', 'download', None}

class NavigationToolbar2WebAgg(backend_bases.NavigationToolbar2):
    toolitems = [(text, tooltip_text, image_file, name_of_method) for text, tooltip_text, image_file, name_of_method in (*backend_bases.NavigationToolbar2.toolitems, ('Download', 'Download plot', 'filesave', 'download')) if name_of_method in _ALLOWED_TOOL_ITEMS]

    def __init__(self, canvas):
        self.message = ''

    def set_message(self, message):
        self.message = message

    def draw_rubberband(self, event, x0, y0, x1, y1):
        ...

    def remove_rubberband(self):
        ...

    def save_figure(self, *args):
        ...

    def pan(self):
        ...

    def zoom(self):
        ...

    def set_history_buttons(self):
        ...

class FigureManagerWebAgg(backend_bases.FigureManagerBase):
    _toolbar2_class = None
    ToolbarCls = NavigationToolbar2WebAgg
    _window_title = 'Matplotlib'

    def __init__(self, canvas, num):
        self.web_sockets = set()

    def show(self):
        ...

    def resize(self, w, h, forward=True):
        ...

    def set_window_title(self, title):
        self._window_title = title

    def get_window_title(self):
        ...

    def add_web_socket(self, web_socket):
        ...

    def remove_web_socket(self, web_socket):
        ...

    def handle_json(self, content):
        ...

    def refresh_all(self):
        ...

    @classmethod
    def get_javascript(cls, stream=None):
        ...

    @classmethod
    def get_static_file_path(cls):
        ...

    def _send_event(self, event_type, **kwargs):
        ...

@_Backend.export
class _BackendWebAggCoreAgg(_Backend):
    FigureCanvas = FigureCanvasWebAggCore
    FigureManager = FigureManagerWebAgg
__all__ = ['asyncio', 'datetime', 'BytesIO', 'StringIO', 'json', 'logging', 'os', 'Path', 'np', 'Image', 'backend_bases', 'backend_tools', 'backend_agg', 'MouseButton', 'KeyEvent', 'LocationEvent', 'MouseEvent', 'ResizeEvent', 'TimerTornado', 'TimerAsyncio', 'FigureCanvasWebAggCore', 'NavigationToolbar2WebAgg', 'FigureManagerWebAgg']
