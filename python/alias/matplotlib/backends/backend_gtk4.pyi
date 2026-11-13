# The type stub of matplotlib.backends.backend_gtk4: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import functools
import io
import os
import matplotlib as mpl
from matplotlib import _api, backend_tools, cbook
from matplotlib.backend_bases import ToolContainerBase, MouseButton, KeyEvent, LocationEvent, MouseEvent, ResizeEvent, CloseEvent
try:
    from gi import require_version as gi_require_version
except ImportError as err:
    raise ImportError('The GTK4 backends require PyGObject') from err
try:
    gi_require_version('Gtk', '4.0')
    gi_require_version('Gdk', '4.0')
    gi_require_version('GdkPixbuf', '2.0')
except ValueError as e:
    raise ImportError(e) from e
import gi
from gi.repository import Gio, GLib, Gtk, Gdk, GdkPixbuf
from . import _backend_gtk
from ._backend_gtk import _BackendGTK, _FigureCanvasGTK, _FigureManagerGTK, _NavigationToolbar2GTK, TimerGTK as TimerGTK4
_GOBJECT_GE_3_47 = gi.version_info >= (3, 47, 0)
_GTK_GE_4_12 = Gtk.check_version(4, 12, 0) is None

class FigureCanvasGTK4(_FigureCanvasGTK, Gtk.DrawingArea):
    required_interactive_framework = 'gtk4'
    supports_blit = False
    manager_class = _api.classproperty(lambda cls: FigureManagerGTK4)

    def __init__(self, figure=None):
        self._idle_draw_id = 0
        self._rubberband_rect = None

    def destroy(self):
        ...

    def set_cursor(self, cursor):
        ...

    def _mpl_coords(self, xy=None):
        ...

    def scroll_event(self, controller, dx, dy):
        ...

    def button_press_event(self, controller, n_press, x, y):
        ...

    def button_release_event(self, controller, n_press, x, y):
        ...

    def key_press_event(self, controller, keyval, keycode, state):
        ...

    def key_release_event(self, controller, keyval, keycode, state):
        ...

    def motion_notify_event(self, controller, x, y):
        ...

    def enter_notify_event(self, controller, x, y):
        ...

    def leave_notify_event(self, controller):
        ...

    def resize_event(self, area, width, height):
        ...

    def _mpl_buttons(self, controller):
        ...

    def _mpl_modifiers(self, controller=None):
        ...

    def _get_key(self, keyval, keycode, state):
        ...

    def _realize_event(self, obj):
        ...

    def _update_device_pixel_ratio(self, *args, **kwargs):
        ...

    def _draw_rubberband(self, rect):
        self._rubberband_rect = rect

    def _draw_func(self, drawing_area, ctx, width, height):
        ...

    def _post_draw(self, widget, ctx):
        ...

    def on_draw_event(self, widget, ctx):
        ...

    def draw(self):
        ...

    def draw_idle(self):
        self._idle_draw_id = GLib.idle_add(idle_draw)

    def flush_events(self):
        ...

class NavigationToolbar2GTK4(_NavigationToolbar2GTK, Gtk.Box):

    def __init__(self, canvas):
        self._gtk_ids = {}
        self.message = Gtk.Label()
        self._gtk_ids[text] = button = Gtk.ToggleButton() if callback in ['zoom', 'pan'] else Gtk.Button()

    def save_figure(self, *args):
        self._save_dialog = dialog

class ToolbarGTK4(ToolContainerBase, Gtk.Box):
    _icon_extension = '-symbolic.svg'

    def __init__(self, toolmanager):
        self._tool_box = Gtk.Box()
        self._groups = {}
        self._toolitems = {}
        self._message = Gtk.Label()

    def add_toolitem(self, name, group, position, image_file, description, toggle):
        ...

    def _find_child_at_position(self, group, position):
        ...

    def _add_button(self, button, group, position):
        self._groups[group] = group_box

    def _call_tool(self, btn, name):
        ...

    def toggle_toolitem(self, name, toggled):
        ...

    def remove_toolitem(self, name):
        ...

    def _add_separator(self):
        ...

    def set_message(self, s):
        ...

@backend_tools._register_tool_class(FigureCanvasGTK4)
class SaveFigureGTK4(backend_tools.SaveFigureBase):

    def trigger(self, *args, **kwargs):
        ...

@backend_tools._register_tool_class(FigureCanvasGTK4)
class HelpGTK4(backend_tools.ToolHelpBase):

    def _normalize_shortcut(self, key):
        ...

    def _is_valid_shortcut(self, key):
        ...

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(FigureCanvasGTK4)
class ToolCopyToClipboardGTK4(backend_tools.ToolCopyToClipboardBase):

    def trigger(self, *args, **kwargs):
        ...
backend_tools._register_tool_class(FigureCanvasGTK4, _backend_gtk.ConfigureSubplotsGTK)
backend_tools._register_tool_class(FigureCanvasGTK4, _backend_gtk.RubberbandGTK)
Toolbar = ToolbarGTK4

class FigureManagerGTK4(_FigureManagerGTK):
    _toolbar2_class = NavigationToolbar2GTK4
    _toolmanager_toolbar_class = ToolbarGTK4

@_BackendGTK.export
class _BackendGTK4(_BackendGTK):
    FigureCanvas = FigureCanvasGTK4
    FigureManager = FigureManagerGTK4
__all__ = ['functools', 'io', 'os', 'mpl', 'backend_tools', 'cbook', 'ToolContainerBase', 'MouseButton', 'KeyEvent', 'LocationEvent', 'MouseEvent', 'ResizeEvent', 'CloseEvent', 'gi', 'Gio', 'GLib', 'Gtk', 'Gdk', 'GdkPixbuf', 'TimerGTK4', 'FigureCanvasGTK4', 'NavigationToolbar2GTK4', 'ToolbarGTK4', 'SaveFigureGTK4', 'HelpGTK4', 'ToolCopyToClipboardGTK4', 'Toolbar', 'FigureManagerGTK4']
