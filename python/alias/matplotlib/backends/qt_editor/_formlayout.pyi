# The type stub of matplotlib.backends.qt_editor._formlayout: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

__version__ = '1.0.10'
__license__ = __doc__
from ast import literal_eval
import copy
import datetime
import logging
from numbers import Integral, Real
from matplotlib import _api, colors as mcolors
from matplotlib.backends.qt_compat import _to_int, QtGui, QtWidgets, QtCore
_log = logging.getLogger(__name__)
BLACKLIST = {'title', 'label'}

class ColorButton(QtWidgets.QPushButton):
    colorChanged = QtCore.Signal(QtGui.QColor)

    def __init__(self, parent=None):
        self._color = QtGui.QColor()

    def choose_color(self):
        ...

    def get_color(self):
        ...

    @QtCore.Slot(QtGui.QColor)
    def set_color(self, color):
        self._color = color
    color = QtCore.Property(QtGui.QColor, get_color, set_color)

def to_qcolor(color):
    ...

class ColorLayout(QtWidgets.QHBoxLayout):

    def __init__(self, color, parent=None):
        self.lineedit = QtWidgets.QLineEdit(mcolors.to_hex(color.getRgbF(), keep_alpha=True), parent)
        self.colorbtn = ColorButton(parent)
        self.colorbtn.color = color

    def update_color(self):
        self.colorbtn.color = qcolor

    def update_text(self, color):
        ...

    def text(self):
        ...

def font_is_installed(font):
    ...

def tuple_to_qfont(tup):
    ...

def qfont_to_tuple(font):
    ...

class FontLayout(QtWidgets.QGridLayout):

    def __init__(self, value, parent=None):
        self.family = QtWidgets.QFontComboBox(parent)
        self.size = QtWidgets.QComboBox(parent)
        self.italic = QtWidgets.QCheckBox(self.tr('Italic'), parent)
        self.bold = QtWidgets.QCheckBox(self.tr('Bold'), parent)

    def get_font(self):
        ...

def is_edit_valid(edit):
    ...

class FormWidget(QtWidgets.QWidget):
    update_buttons = QtCore.Signal()

    def __init__(self, data, comment='', with_margin=False, parent=None):
        self.data = copy.deepcopy(data)
        self.widgets = []
        self.formlayout = QtWidgets.QFormLayout(self)

    def get_dialog(self):
        ...

    def setup(self):
        ...

    def get(self):
        ...

class FormComboWidget(QtWidgets.QWidget):
    update_buttons = QtCore.Signal()

    def __init__(self, datalist, comment='', parent=None):
        self.combobox = QtWidgets.QComboBox()
        self.stackwidget = QtWidgets.QStackedWidget(self)
        self.widgetlist = []

    def setup(self):
        ...

    def get(self):
        ...

class FormTabWidget(QtWidgets.QWidget):
    update_buttons = QtCore.Signal()

    def __init__(self, datalist, comment='', parent=None):
        self.tabwidget = QtWidgets.QTabWidget()
        self.widgetlist = []

    def setup(self):
        ...

    def get(self):
        ...

class FormDialog(QtWidgets.QDialog):

    def __init__(self, data, title='', comment='', icon=None, parent=None, apply=None):
        self.apply_callback = apply
        self.float_fields = []
        self.bbox = bbox = QtWidgets.QDialogButtonBox(QtWidgets.QDialogButtonBox.StandardButton(_to_int(QtWidgets.QDialogButtonBox.StandardButton.Ok) | _to_int(QtWidgets.QDialogButtonBox.StandardButton.Cancel)))
        self.formwidget = FormTabWidget(data, comment=comment, parent=self)
        self.formwidget = FormComboWidget(data, comment=comment, parent=self)
        self.formwidget = FormWidget(data, comment=comment, parent=self)

    def register_float_field(self, field):
        ...

    def update_buttons(self):
        ...

    def accept(self):
        self.data = self.formwidget.get()

    def reject(self):
        self.data = None

    def apply(self):
        ...

    def get(self):
        ...

def fedit(data, title='', comment='', icon=None, parent=None, apply=None):
    ...
if __name__ == '__main__':
    _app = QtWidgets.QApplication([])

    def create_datalist_example():
        ...

    def create_datagroup_example():
        ...
    datalist = create_datalist_example()

    def apply_test(data):
        ...
    fedit(datalist, title='Example', comment='This is just an <b>example</b>.', apply=apply_test)
    _app.exec()
    datagroup = create_datagroup_example()
    fedit(datagroup, 'Global title', apply=apply_test)
    _app.exec()
    datalist = create_datalist_example()
    datagroup = create_datagroup_example()
    fedit(((datagroup, 'Title 1', 'Tab 1 comment'), (datalist, 'Title 2', 'Tab 2 comment'), (datalist, 'Title 3', 'Tab 3 comment')), 'Global title', apply=apply_test)
    _app.exec()
__all__ = ['literal_eval', 'copy', 'datetime', 'logging', 'Integral', 'Real', 'mcolors', 'QtGui', 'QtWidgets', 'QtCore', 'BLACKLIST', 'ColorButton', 'to_qcolor', 'ColorLayout', 'font_is_installed', 'tuple_to_qfont', 'qfont_to_tuple', 'FontLayout', 'is_edit_valid', 'FormWidget', 'FormComboWidget', 'FormTabWidget', 'FormDialog', 'fedit', 'create_datalist_example', 'create_datagroup_example', 'datalist', 'apply_test', 'datagroup']
