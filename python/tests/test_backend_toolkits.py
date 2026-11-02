"""
Smoke tests of the GUI toolkit backends ported from matplotlib (Qt, GTK 3
and 4, wx, Tk, and their Cairo variants, which draw with pltrs (Rust)'s
raster): without their toolkits they fail to import as matplotlib's do;
with stand-ins for GTK, cairo and wx (no system libraries here) their
modules load over pltrs' canvases. Qt and Tk run for real under
matplotlib's own tests (``test_backend_qt.py``, ``test_backend_tk.py``,
``test_backends_interactive.py``) where a display and the toolkit exist.
"""

import importlib.util
import inspect
import os
import subprocess
import sys
import textwrap

import pytest


@pytest.mark.skipif(any(importlib.util.find_spec(binding) for binding in
                        ["PyQt6", "PySide6", "PyQt5", "PySide2"]),
                    reason="a Qt binding is installed")
@pytest.mark.parametrize("module, bindings", [
    ("backend_qtagg", "PyQt6, PySide6, PyQt5, PySide2"),
    ("backend_qtcairo", "PyQt6, PySide6, PyQt5, PySide2"),
    ("backend_qt5agg", "PyQt5, PySide2"),
])
def test_qt_backends_need_a_qt_binding(module, bindings):
    """matplotlib's message, naming the bindings tried."""
    with pytest.raises(ImportError,
                       match=f"Failed to import any of the following Qt "
                             f"binding modules: {bindings}$"):
        importlib.import_module(f"pltrs.backends.{module}")


@pytest.mark.skipif(importlib.util.find_spec("gi") is not None,
                    reason="PyGObject is installed")
@pytest.mark.parametrize("module", ["backend_gtk3agg", "backend_gtk4cairo",
                                    "backend_wxagg"])
def test_gtk_and_wx_backends_need_their_toolkit(module):
    with pytest.raises(ImportError):
        importlib.import_module(f"pltrs.backends.{module}")


def _stand_ins():
    """Install stand-ins for PyGObject's GTK, pycairo and wxPython."""
    import sys
    import types
    from unittest.mock import MagicMock

    class Anything(type):
        """A class whose missing attributes are mocks (GTK's enums...)."""
        def __getattr__(cls, name):
            return MagicMock(name=name)

    def namespace(name):
        module = types.ModuleType(name)
        classes = {}

        def getattr_(attr):
            if attr.startswith("__"):
                raise AttributeError(attr)
            if attr[:1].isupper():
                return classes.setdefault(
                    attr, Anything(attr, (), {"__init__": lambda *a, **k: None}))
            return MagicMock(name=attr)

        module.__getattr__ = getattr_
        return module

    gi = types.ModuleType("gi")
    gi.version_info = (3, 50, 0)
    gi.require_version = lambda *args: None
    gi.require_foreign = lambda *args: None
    repository = types.ModuleType("gi.repository")
    gi.repository = repository
    sys.modules["gi"], sys.modules["gi.repository"] = gi, repository
    for name in ["Gtk", "Gdk", "Gio", "GLib", "GObject", "GdkPixbuf"]:
        module = namespace(f"gi.repository.{name}")
        setattr(repository, name, module)
        sys.modules[f"gi.repository.{name}"] = module
    sys.modules["gi.repository.Gtk"].MAJOR_VERSION = 3
    sys.modules["cairo"] = namespace("cairo")
    wx = namespace("wx")
    sys.modules["wx"] = wx
    sys.modules["wx.svg"] = namespace("wx.svg")
    sys.modules["wx.lib"] = namespace("wx.lib")


def _gtk3():
    _stand_ins()  # noqa: F821  (defined in the same program)
    from pltrs.backends import backend_gtk3agg, backend_gtk3cairo
    from pltrs.backends.backend_agg import FigureCanvasAgg
    from pltrs.backends.backend_gtk3 import FigureCanvasGTK3

    agg = backend_gtk3agg.FigureCanvasGTK3Agg
    assert issubclass(agg, FigureCanvasAgg) and issubclass(agg, FigureCanvasGTK3)
    assert agg.required_interactive_framework == "gtk3"
    assert issubclass(backend_gtk3cairo.FigureCanvasGTK3Cairo, agg)
    assert backend_gtk3agg.FigureCanvas is agg
    assert backend_gtk3cairo.FigureCanvas is backend_gtk3cairo.FigureCanvasGTK3Cairo
    print("gtk3")


def _gtk4():
    _stand_ins()  # noqa: F821
    import sys
    sys.modules["gi.repository.Gtk"].MAJOR_VERSION = 4
    from pltrs.backends import backend_gtk4agg, backend_gtk4cairo
    from pltrs.backends.backend_agg import FigureCanvasAgg

    agg = backend_gtk4agg.FigureCanvasGTK4Agg
    assert issubclass(agg, FigureCanvasAgg)
    assert agg.required_interactive_framework == "gtk4"
    assert issubclass(backend_gtk4cairo.FigureCanvasGTK4Cairo, agg)
    print("gtk4")


def _wx():
    _stand_ins()  # noqa: F821
    from pltrs.backends import backend_wx, backend_wxagg, backend_wxcairo
    from pltrs.backends.backend_agg import FigureCanvasAgg

    agg = backend_wxagg.FigureCanvasWxAgg
    assert issubclass(agg, FigureCanvasAgg)
    assert agg.required_interactive_framework == "wx"
    assert issubclass(backend_wxcairo.FigureCanvasWxCairo, agg)
    assert backend_wx.FigureCanvas is backend_wx.FigureCanvasWx
    print("wx")


@pytest.mark.parametrize("body", [_gtk3, _gtk4, _wx])
def test_gtk_and_wx_modules_load_over_pltrs_canvases(body):
    """Each body runs in a fresh interpreter, where the stand-ins stay."""
    stand_ins = textwrap.dedent(inspect.getsource(_stand_ins))
    source = stand_ins + textwrap.dedent(inspect.getsource(body)) + \
        f"\n{body.__name__}()\n"
    proc = subprocess.run([sys.executable, "-c", source],
                          env={**os.environ, "PLTRS_LOOK": "matplotlib"},
                          capture_output=True, text=True, timeout=60)
    assert proc.returncode == 0, proc.stderr
    assert proc.stdout.split() == [body.__name__.strip("_")]


@pytest.mark.skipif(importlib.util.find_spec("tkinter") is None,
                    reason="no tkinter")
def test_tkcairo_draws_as_tkagg():
    from pltrs.backends import backend_tkcairo
    from pltrs.backends.backend_tkagg import FigureCanvasTkAgg

    assert issubclass(backend_tkcairo.FigureCanvasTkCairo, FigureCanvasTkAgg)
    assert backend_tkcairo.FigureCanvas is backend_tkcairo.FigureCanvasTkCairo


def test_tkagg_blit_helper_checks_its_box_as_matplotlibs():
    """matplotlib's ``_tkagg.blit`` refuses boxes out of the data's bounds
    and unknown interpreters (its ``test_blit``)."""
    import numpy as np

    tkagg = pytest.importorskip("pltrs.backends._tkagg")
    data = np.ones((4, 4, 4), dtype=np.uint8)
    with pytest.raises(ValueError, match="Failed to extract Tk_PhotoHandle"):
        tkagg.blit(0, "photo", data, tkagg.TK_PHOTO_COMPOSITE_OVERLAY,
                   (0, 1, 2, 3), (0, 2, 0, 2))
    assert tkagg.enable_dpi_awareness(0, 0) is None


def test_macosx_backend_is_macos_only():
    if sys.platform == "darwin":
        pytest.skip("macOS has the backend")
    assert importlib.util.find_spec("pltrs.backends._macosx") is None
    with pytest.raises(ImportError):
        importlib.import_module("pltrs.backends.backend_macosx")


def test_buffer_regions_expose_their_pixels():
    """matplotlib's ``BufferRegion`` buffer, which the Qt and GTK canvases
    paint from: ``(height, width, 4)`` bytes, transparent off the canvas."""
    import numpy as np

    from pltrs.backends.backend_agg import FigureCanvasAgg
    from pltrs.figure import Figure
    from pltrs.transforms import Bbox

    fig = Figure(figsize=(2, 1), dpi=50)
    canvas = FigureCanvasAgg(fig)
    canvas.draw()
    region = canvas.copy_from_bbox(Bbox([[-10, 0], [20, 30]]))
    pixels = np.asarray(region)
    assert pixels.shape == memoryview(region).shape == (30, 30, 4)
    assert (pixels[:, :10] == 0).all()
    assert (pixels[:, 10:] == np.asarray(canvas.buffer_rgba())[20:, :20]).all()
