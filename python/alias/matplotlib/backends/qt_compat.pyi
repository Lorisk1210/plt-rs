# The type stub of matplotlib.backends.qt_compat: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import operator
import os
import platform
import sys
from packaging.version import parse as parse_version
import matplotlib as mpl
from . import _QT_FORCE_QT5_BINDING
QT_API_PYQT6 = 'PyQt6'
QT_API_PYSIDE6 = 'PySide6'
QT_API_PYQT5 = 'PyQt5'
QT_API_PYSIDE2 = 'PySide2'
QT_API_ENV = os.environ.get('QT_API')
if QT_API_ENV is not None:
    QT_API_ENV = QT_API_ENV.lower()
_ETS = {'pyqt6': QT_API_PYQT6, 'pyside6': QT_API_PYSIDE6, 'pyqt5': QT_API_PYQT5, 'pyside2': QT_API_PYSIDE2}
if sys.modules.get('PyQt6.QtCore'):
    QT_API = QT_API_PYQT6
elif sys.modules.get('PySide6.QtCore'):
    QT_API = QT_API_PYSIDE6
elif sys.modules.get('PyQt5.QtCore'):
    QT_API = QT_API_PYQT5
elif sys.modules.get('PySide2.QtCore'):
    QT_API = QT_API_PYSIDE2
elif (mpl.rcParams._get_backend_or_none() or '').lower().startswith('qt5'):
    if QT_API_ENV in ['pyqt5', 'pyside2']:
        QT_API = _ETS[QT_API_ENV]
    else:
        _QT_FORCE_QT5_BINDING = True
        QT_API = None
elif QT_API_ENV is None:
    QT_API = None
elif QT_API_ENV in _ETS:
    QT_API = _ETS[QT_API_ENV]
else:
    raise RuntimeError('The environment variable QT_API has the unrecognized value {!r}; valid values are {}'.format(QT_API_ENV, ', '.join(_ETS)))

def _setup_pyqt5plus():
    ...
if QT_API in [QT_API_PYQT6, QT_API_PYQT5, QT_API_PYSIDE6, QT_API_PYSIDE2]:
    _setup_pyqt5plus()
elif QT_API is None:
    if _QT_FORCE_QT5_BINDING:
        _candidates = [(_setup_pyqt5plus, QT_API_PYQT5), (_setup_pyqt5plus, QT_API_PYSIDE2)]
    else:
        _candidates = [(_setup_pyqt5plus, QT_API_PYQT6), (_setup_pyqt5plus, QT_API_PYSIDE6), (_setup_pyqt5plus, QT_API_PYQT5), (_setup_pyqt5plus, QT_API_PYSIDE2)]
    for _setup, QT_API in _candidates:
        try:
            _setup()
        except ImportError:
            continue
        break
    else:
        raise ImportError('Failed to import any of the following Qt binding modules: {}'.format(', '.join([QT_API for _, QT_API in _candidates])))
else:
    raise AssertionError(f'Unexpected QT_API: {QT_API}')
_version_info = tuple(QtCore.QLibraryInfo.version().segments())
if _version_info < (5, 12):
    raise ImportError(f'The Qt version imported is {QtCore.QLibraryInfo.version().toString()} but Matplotlib requires Qt>=5.12')
if sys.platform == 'darwin' and parse_version(platform.mac_ver()[0]) >= parse_version('10.16') and (_version_info < (5, 15, 2)):
    os.environ.setdefault('QT_MAC_WANTS_LAYER', '1')

def _exec(obj):
    ...
__all__ = ['operator', 'os', 'platform', 'sys', 'parse_version', 'mpl', 'QT_API_PYQT6', 'QT_API_PYSIDE6', 'QT_API_PYQT5', 'QT_API_PYSIDE2', 'QT_API_ENV', 'QT_API']
