# The type stub of matplotlib.backends.backend_qt: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import functools
import os
import sys
import traceback
import matplotlib as mpl
from matplotlib import _api, backend_tools, cbook
from matplotlib._pylab_helpers import Gcf
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, NavigationToolbar2, TimerBase, cursors, ToolContainerBase, MouseButton, CloseEvent, KeyEvent, LocationEvent, MouseEvent, ResizeEvent, _allow_interrupt
import matplotlib.backends.qt_editor.figureoptions as figureoptions
from . import qt_compat
from .qt_compat import QtCore, QtGui, QtWidgets, QtSvg, __version__, QT_API, _to_int, _isdeleted
SPECIAL_KEYS = {_to_int(getattr(QtCore.Qt.Key, k)): v for k, v in [('Key_Escape', 'escape'), ('Key_Tab', 'tab'), ('Key_Backspace', 'backspace'), ('Key_Return', 'enter'), ('Key_Enter', 'enter'), ('Key_Insert', 'insert'), ('Key_Delete', 'delete'), ('Key_Pause', 'pause'), ('Key_SysReq', 'sysreq'), ('Key_Clear', 'clear'), ('Key_Home', 'home'), ('Key_End', 'end'), ('Key_Left', 'left'), ('Key_Up', 'up'), ('Key_Right', 'right'), ('Key_Down', 'down'), ('Key_PageUp', 'pageup'), ('Key_PageDown', 'pagedown'), ('Key_Shift', 'shift'), ('Key_Control', 'control' if sys.platform != 'darwin' else 'cmd'), ('Key_Meta', 'meta' if sys.platform != 'darwin' else 'control'), ('Key_Alt', 'alt'), ('Key_CapsLock', 'caps_lock'), ('Key_F1', 'f1'), ('Key_F2', 'f2'), ('Key_F3', 'f3'), ('Key_F4', 'f4'), ('Key_F5', 'f5'), ('Key_F6', 'f6'), ('Key_F7', 'f7'), ('Key_F8', 'f8'), ('Key_F9', 'f9'), ('Key_F10', 'f10'), ('Key_F11', 'f11'), ('Key_F12', 'f12'), ('Key_Super_L', 'super'), ('Key_Super_R', 'super')]}
_MODIFIER_KEYS = [(_to_int(getattr(QtCore.Qt.KeyboardModifier, mod)), _to_int(getattr(QtCore.Qt.Key, key))) for mod, key in [('ControlModifier', 'Key_Control'), ('AltModifier', 'Key_Alt'), ('ShiftModifier', 'Key_Shift'), ('MetaModifier', 'Key_Meta')]]
cursord = {k: getattr(QtCore.Qt.CursorShape, v) for k, v in [(cursors.MOVE, 'SizeAllCursor'), (cursors.HAND, 'PointingHandCursor'), (cursors.POINTER, 'ArrowCursor'), (cursors.SELECT_REGION, 'CrossCursor'), (cursors.WAIT, 'WaitCursor'), (cursors.RESIZE_HORIZONTAL, 'SizeHorCursor'), (cursors.RESIZE_VERTICAL, 'SizeVerCursor')]}

@functools.lru_cache(1)
def _create_qApp():
    ...

def _allow_interrupt_qt(qapp_or_eventloop):
    ...

class TimerQT(TimerBase):

    def __init__(self, *args, **kwargs):
        self._timer = QtCore.QTimer()

    def __del__(self):
        ...

    def _timer_set_single_shot(self):
        ...

    def _timer_set_interval(self):
        ...

    def _timer_start(self):
        ...

    def _timer_stop(self):
        ...

class FigureCanvasQT(FigureCanvasBase, QtWidgets.QWidget):
    required_interactive_framework = 'qt'
    _timer_cls = TimerQT
    manager_class = _api.classproperty(lambda cls: FigureManagerQT)
    buttond = {getattr(QtCore.Qt.MouseButton, k): v for k, v in [('LeftButton', MouseButton.LEFT), ('RightButton', MouseButton.RIGHT), ('MiddleButton', MouseButton.MIDDLE), ('XButton1', MouseButton.BACK), ('XButton2', MouseButton.FORWARD)]}

    def __init__(self, figure=None):
        self._draw_pending = False
        self._is_drawing = False
        self._draw_rect_callback = lambda painter: None
        self._in_resize_event = False

    @QtCore.Slot()
    def _update_pixel_ratio(self):
        ...

    @QtCore.Slot(QtGui.QScreen)
    def _update_screen(self, screen):
        ...

    def eventFilter(self, source, event):
        ...

    def showEvent(self, event):
        ...

    def set_cursor(self, cursor):
        ...

    def mouseEventCoords(self, pos=None):
        ...

    def enterEvent(self, event):
        ...

    def leaveEvent(self, event):
        ...

    def mousePressEvent(self, event):
        ...

    def mouseDoubleClickEvent(self, event):
        ...

    def mouseMoveEvent(self, event):
        ...

    def mouseReleaseEvent(self, event):
        ...

    def wheelEvent(self, event):
        ...

    def keyPressEvent(self, event):
        ...

    def keyReleaseEvent(self, event):
        ...

    def resizeEvent(self, event):
        self._in_resize_event = True
        self._in_resize_event = False

    def sizeHint(self):
        ...

    def minimumSizeHint(self):
        ...

    @staticmethod
    def _mpl_buttons(buttons):
        ...

    @staticmethod
    def _mpl_modifiers(modifiers=None, *, exclude=None):
        ...

    def _get_key(self, event):
        ...

    def flush_events(self):
        ...

    def start_event_loop(self, timeout=0):
        self._event_loop = event_loop = QtCore.QEventLoop()

    def stop_event_loop(self, event=None):
        ...

    def draw(self):
        ...

    def draw_idle(self):
        self._draw_pending = True

    def blit(self, bbox=None):
        ...

    def _draw_idle(self):
        self._draw_pending = False

    def drawRectangle(self, rect):
        self._draw_rect_callback = _draw_rect_callback

class MainWindow(QtWidgets.QMainWindow):
    closing = QtCore.Signal()

    def closeEvent(self, event):
        ...

class FigureManagerQT(FigureManagerBase):

    def __init__(self, canvas, num):
        self.window = MainWindow()
        self.window._destroying = False

    def full_screen_toggle(self):
        ...

    def _widgetclosed(self):
        self.window._destroying = True

    def resize(self, width, height):
        ...

    @classmethod
    def start_main_loop(cls):
        ...

    def show(self):
        self.window._destroying = False

    def destroy(self, *args):
        self.window._destroying = True

    def get_window_title(self):
        ...

    def set_window_title(self, title):
        ...

class _IconEngine(QtGui.QIconEngine):

    def __init__(self, image_path, toolbar=None):
        self.image_path = image_path
        self.toolbar = toolbar

    def _is_dark_mode(self):
        ...

    def paint(self, painter, rect, mode, state):
        ...

    def pixmap(self, size, mode, state):
        ...

    def _devicePixelRatio(self):
        ...

    def _create_pixmap_from_svg(self, svg_path, size):
        ...

    def _create_pixmap_from_png(self, base_path, size):
        ...

class NavigationToolbar2QT(NavigationToolbar2, QtWidgets.QToolBar):
    toolitems = [*NavigationToolbar2.toolitems]
    toolitems.insert([name for name, *_ in toolitems].index('Subplots') + 1, ('Customize', 'Edit axis, curve and image parameters', 'qt4_editor_options', 'edit_parameters'))

    def __init__(self, canvas, parent=None, coordinates=True):
        self.coordinates = coordinates
        self._actions = {}
        self._subplot_dialog = None
        self.locLabel = QtWidgets.QLabel('', self)
        self._actions[callback] = a

    def _icon(self, name):
        ...

    def edit_parameters(self):
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
        ...

    def remove_rubberband(self):
        ...

    def configure_subplots(self):
        self._subplot_dialog = SubplotToolQt(self.canvas.figure, self.canvas.parent())

    def save_figure(self, *args):
        ...

    def set_history_buttons(self):
        ...

class SubplotToolQt(QtWidgets.QDialog):

    def __init__(self, targetfig, parent):
        self._spinboxes = {}
        self._figure = targetfig
        self._defaults = {}
        self._export_values_dialog = None
        self._spinboxes[name] = spinbox = QtWidgets.QDoubleSpinBox()

    def update_from_current_subplotpars(self):
        self._defaults = {spinbox: getattr(self._figure.subplotpars, name) for name, spinbox in self._spinboxes.items()}

    def _export_values(self):
        self._export_values_dialog = QtWidgets.QDialog()

    def _on_value_changed(self):
        ...

    def _tight_layout(self):
        ...

    def _reset(self):
        ...

class ToolbarQt(ToolContainerBase, QtWidgets.QToolBar):

    def __init__(self, toolmanager, parent=None):
        self._message_action = self.addWidget(message_label)
        self._toolitems = {}
        self._groups = {}

    def add_toolitem(self, name, group, position, image_file, description, toggle):
        ...

    def _add_to_group(self, group, name, button, position):
        self._groups[group] = gr

    def toggle_toolitem(self, name, toggled):
        ...

    def remove_toolitem(self, name):
        ...

    def set_message(self, s):
        ...

@backend_tools._register_tool_class(FigureCanvasQT)
class ConfigureSubplotsQt(backend_tools.ConfigureSubplotsBase):

    def __init__(self, *args, **kwargs):
        self._subplot_dialog = None

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(FigureCanvasQT)
class SaveFigureQt(backend_tools.SaveFigureBase):

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(FigureCanvasQT)
class RubberbandQt(backend_tools.RubberbandBase):

    def draw_rubberband(self, x0, y0, x1, y1):
        ...

    def remove_rubberband(self):
        ...

@backend_tools._register_tool_class(FigureCanvasQT)
class HelpQt(backend_tools.ToolHelpBase):

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(FigureCanvasQT)
class ToolCopyToClipboardQT(backend_tools.ToolCopyToClipboardBase):

    def trigger(self, *args, **kwargs):
        ...
FigureManagerQT._toolbar2_class = NavigationToolbar2QT
FigureManagerQT._toolmanager_toolbar_class = ToolbarQt

@_Backend.export
class _BackendQT(_Backend):
    backend_version = __version__
    FigureCanvas = FigureCanvasQT
    FigureManager = FigureManagerQT
    mainloop = FigureManagerQT.start_main_loop
__all__ = ['functools', 'os', 'sys', 'traceback', 'mpl', 'backend_tools', 'cbook', 'Gcf', 'FigureCanvasBase', 'FigureManagerBase', 'NavigationToolbar2', 'TimerBase', 'cursors', 'ToolContainerBase', 'MouseButton', 'CloseEvent', 'KeyEvent', 'LocationEvent', 'MouseEvent', 'ResizeEvent', 'figureoptions', 'qt_compat', 'QtCore', 'QtGui', 'QtWidgets', 'QtSvg', 'QT_API', 'SPECIAL_KEYS', 'cursord', 'TimerQT', 'FigureCanvasQT', 'MainWindow', 'FigureManagerQT', 'NavigationToolbar2QT', 'SubplotToolQt', 'ToolbarQt', 'ConfigureSubplotsQt', 'SaveFigureQt', 'RubberbandQt', 'HelpQt', 'ToolCopyToClipboardQT']
