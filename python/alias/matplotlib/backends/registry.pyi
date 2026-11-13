# The type stub of matplotlib.backends.registry: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from enum import Enum
import importlib

class BackendFilter(Enum):
    INTERACTIVE = 0
    NON_INTERACTIVE = 1

class BackendRegistry:
    _BUILTIN_BACKEND_TO_GUI_FRAMEWORK = {'gtk3agg': 'gtk3', 'gtk3cairo': 'gtk3', 'gtk4agg': 'gtk4', 'gtk4cairo': 'gtk4', 'macosx': 'macosx', 'nbagg': 'nbagg', 'notebook': 'nbagg', 'qtagg': 'qt', 'qtcairo': 'qt', 'qt5agg': 'qt5', 'qt5cairo': 'qt5', 'tkagg': 'tk', 'tkcairo': 'tk', 'webagg': 'webagg', 'wx': 'wx', 'wxagg': 'wx', 'wxcairo': 'wx', 'agg': 'headless', 'cairo': 'headless', 'pdf': 'headless', 'pgf': 'headless', 'ps': 'headless', 'svg': 'headless', 'template': 'headless'}
    _GUI_FRAMEWORK_TO_BACKEND = {'gtk3': 'gtk3agg', 'gtk4': 'gtk4agg', 'headless': 'agg', 'macosx': 'macosx', 'qt': 'qtagg', 'qt5': 'qt5agg', 'qt6': 'qtagg', 'tk': 'tkagg', 'wx': 'wxagg'}

    def __init__(self):
        self._loaded_entry_points = False
        self._backend_to_gui_framework = {}
        self._name_to_module = {'notebook': 'nbagg'}

    def _backend_module_name(self, backend):
        ...

    def _clear(self):
        ...

    def _ensure_entry_points_loaded(self):
        self._loaded_entry_points = True

    def _get_gui_framework_by_loading(self, backend):
        ...

    def _read_entry_points(self):
        ...

    def _validate_and_store_entry_points(self, entries):
        self._name_to_module[name] = 'module://' + module
        self._backend_to_gui_framework[name] = 'unknown'

    def backend_for_gui_framework(self, framework):
        ...

    def is_valid_backend(self, backend):
        self._backend_to_gui_framework[backend] = 'unknown'

    def list_all(self):
        ...

    def list_builtin(self, filter_=None):
        ...

    def list_gui_frameworks(self):
        ...

    def load_backend_module(self, backend):
        ...

    def resolve_backend(self, backend):
        self._backend_to_gui_framework[backend] = gui

    def resolve_gui_or_backend(self, gui_or_backend):
        ...
backend_registry = BackendRegistry()
__all__ = ['Enum', 'importlib', 'BackendFilter', 'BackendRegistry', 'backend_registry']
