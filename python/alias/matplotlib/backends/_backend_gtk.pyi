# The type stub of matplotlib.backends._backend_gtk: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import logging
import sys
import matplotlib as mpl
from matplotlib import _api, backend_tools, cbook
from matplotlib._pylab_helpers import Gcf
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, NavigationToolbar2, TimerBase
from matplotlib.backend_tools import Cursors
import gi
from gi.repository import Gdk, Gio, GLib, Gtk
try:
    gi.require_foreign('cairo')
except ImportError as e:
    raise ImportError('Gtk-based backends require cairo') from e
_log = logging.getLogger(__name__)
_application = None

def _shutdown_application(app):
    ...

def _create_application():
    ...

def mpl_to_gtk_cursor_name(mpl_cursor):
    ...

class TimerGTK(TimerBase):

    def __init__(self, *args, **kwargs):
        self._timer = None

    def _timer_start(self):
        self._timer = GLib.timeout_add(self._interval, self._on_timer)

    def _timer_stop(self):
        self._timer = None

    def _timer_set_interval(self):
        ...

    def _on_timer(self):
        self._timer = None

class _FigureCanvasGTK(FigureCanvasBase):
    _timer_cls = TimerGTK

class _FigureManagerGTK(FigureManagerBase):

    def __init__(self, canvas, num):
        self._gtk_ver = gtk_ver = Gtk.get_major_version()
        self.window = Gtk.Window()
        self.vbox = Gtk.Box()
        self._destroying = False

    def destroy(self, *args):
        self._destroying = True

    @classmethod
    def start_main_loop(cls):
        ...

    def show(self):
        ...

    def full_screen_toggle(self):
        ...

    def get_window_title(self):
        ...

    def set_window_title(self, title):
        ...

    def resize(self, width, height):
        ...

class _NavigationToolbar2GTK(NavigationToolbar2):

    def set_message(self, s):
        ...

    def draw_rubberband(self, event, x0, y0, x1, y1):
        ...

    def remove_rubberband(self):
        ...

    def _update_buttons_checked(self):
        ...

    def pan(self, *args):
        ...

    def zoom(self, *args):
        ...

    def set_history_buttons(self):
        ...

class RubberbandGTK(backend_tools.RubberbandBase):

    def draw_rubberband(self, x0, y0, x1, y1):
        ...

    def remove_rubberband(self):
        ...

class ConfigureSubplotsGTK(backend_tools.ConfigureSubplotsBase):

    def trigger(self, *args):
        ...

class _BackendGTK(_Backend):
    backend_version = '{}.{}.{}'.format(Gtk.get_major_version(), Gtk.get_minor_version(), Gtk.get_micro_version())
    mainloop = _FigureManagerGTK.start_main_loop
__all__ = ['logging', 'sys', 'mpl', 'backend_tools', 'cbook', 'Gcf', 'FigureCanvasBase', 'FigureManagerBase', 'NavigationToolbar2', 'TimerBase', 'Cursors', 'gi', 'Gdk', 'Gio', 'GLib', 'Gtk', 'mpl_to_gtk_cursor_name', 'TimerGTK', 'RubberbandGTK', 'ConfigureSubplotsGTK']
