"""
Smoke tests of the GTK 3, GTK 4 and wx backends with their real toolkits
(``python/tools/toolkits.sh`` installs them without root), and of the Cairo
backend (`pltrs.backends.backend_cairo`).

The canvas widget of every Agg and Cairo variant shows the pixels pltrs
(Rust) drew for the figure, as matplotlib's Agg canvases do: what each
toolkit captures of the widget is the canvas's Agg buffer. The expected
values of the Cairo backend are real matplotlib 3.11's, with pycairo 1.29.
Each test skips where its toolkit, pycairo or a display is missing.
"""

import importlib.util
import inspect
import io
import os
import subprocess
import sys
import textwrap

import numpy as np
import pytest

import pltrs
from pltrs import _c_internal_utils, cbook
from pltrs.backend_bases import NavigationToolbar2
from pltrs.figure import Figure

#: How long a body may run, in seconds.
TIMEOUT = 60


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield


def _gtk_versions():
    """The GTK versions PyGObject can load (none without it)."""
    try:
        import gi
    except ImportError:
        return []
    return gi.Repository.get_default().enumerate_versions("Gtk")


def _skip_reason(backend):
    """Why *backend* cannot run here, or ``None``."""
    if not _c_internal_utils.display_is_valid():
        return "no display"
    if backend.startswith("wx"):
        if importlib.util.find_spec("wx") is None:
            return "no wxPython"
        return None if _c_internal_utils.xdisplay_is_valid() else "no X display"
    if importlib.util.find_spec("cairo") is None:
        return "no pycairo"
    version = f"{backend[3]}.0"
    return None if version in _gtk_versions() else f"no GTK {version}"


def _run(body, *args):
    """Run the function *body* with *args* in a fresh interpreter, in
    matplotlib's look; its standard output."""
    source = textwrap.dedent(inspect.getsource(body)) + \
        f"\n{body.__name__}(*{args!r})\n"
    proc = subprocess.run([sys.executable, "-c", source],
                          env={**os.environ, "PLTRS_LOOK": "matplotlib"},
                          capture_output=True, text=True, timeout=TIMEOUT)
    assert proc.returncode == 0, proc.stderr
    return proc.stdout


def _show_and_capture(backend):
    """Show a figure with *backend*, capture its canvas widget through the
    toolkit once drawn, and print whether it shows the Agg buffer."""
    import numpy as np

    import pltrs
    pltrs.use(backend)
    pltrs.rcParams["toolbar"] = "None"
    import pltrs.pyplot as plt

    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    x = np.linspace(0, 2 * np.pi, 50)
    ax.plot(x, np.sin(x), "o-", label="sin")
    ax.fill_between(x, np.cos(x), alpha=0.4, hatch="//")
    ax.imshow(np.arange(12).reshape(3, 4), extent=(0, 3, 0, 1), aspect="auto")
    ax.set_title(r"Title $\alpha^2$")
    ax.legend()
    canvas = fig.canvas

    def gtk3():
        from gi.repository import Gdk
        window = canvas.get_window()
        pixbuf = Gdk.pixbuf_get_from_window(
            window, 0, 0, window.get_width(), window.get_height())
        rows = np.frombuffer(pixbuf.get_pixels(), np.uint8).reshape(
            pixbuf.get_height(), pixbuf.get_rowstride())
        width = pixbuf.get_width() * pixbuf.get_n_channels()
        return rows[:, :width].reshape(pixbuf.get_height(), -1,
                                       pixbuf.get_n_channels())[..., :3]

    def gtk4():
        from gi.repository import Gdk, Gtk
        snapshot = Gtk.Snapshot.new()
        Gtk.WidgetPaintable.new(canvas).snapshot(
            snapshot, canvas.get_width(), canvas.get_height())
        texture = canvas.get_native().get_renderer().render_texture(
            snapshot.to_node(), None)
        downloader = Gdk.TextureDownloader.new(texture)
        downloader.set_format(Gdk.MemoryFormat.R8G8B8)
        data, stride = downloader.download_bytes()
        rows = np.frombuffer(data.get_data(), np.uint8).reshape(
            texture.get_height(), stride)
        return rows[:, :texture.get_width() * 3].reshape(
            texture.get_height(), -1, 3)

    def wx_():
        import wx
        width, height = canvas.GetClientSize()
        bitmap = wx.Bitmap(width, height)
        memory = wx.MemoryDC(bitmap)
        memory.Blit(0, 0, width, height, wx.ClientDC(canvas), 0, 0)
        memory.SelectObject(wx.NullBitmap)
        image = bitmap.ConvertToImage()
        return np.frombuffer(bytes(image.GetData()), np.uint8).reshape(
            height, width, 3)

    grab = {"gtk3": gtk3, "gtk4": gtk4}.get(backend[:4], wx_)

    def capture():
        shown = grab()
        drawn = np.asarray(canvas.buffer_rgba())[..., :3]
        print(shown.shape == drawn.shape and (shown == drawn).all())
        plt.close(fig)

    timer = canvas.new_timer(1000)
    timer.single_shot = True
    timer.add_callback(capture)
    timer.start()
    plt.show()
    print(type(canvas).__name__)


@pytest.mark.parametrize("backend, canvas", [
    ("gtk3agg", "FigureCanvasGTK3Agg"), ("gtk3cairo", "FigureCanvasGTK3Cairo"),
    ("gtk4agg", "FigureCanvasGTK4Agg"), ("gtk4cairo", "FigureCanvasGTK4Cairo"),
    ("wxagg", "FigureCanvasWxAgg"), ("wxcairo", "FigureCanvasWxCairo"),
])
def test_canvas_widget_shows_the_drawn_figure(backend, canvas):
    reason = _skip_reason(backend)
    if reason:
        pytest.skip(reason)
    assert _run(_show_and_capture, backend).split() == ["True", canvas]


def _change_from_a_worker(backend, loop):
    """While the toolkit's loop runs (``plt.show()`` or ``plt.pause()``), a
    worker thread changes the figure; print what the main thread sees."""
    import threading
    import time

    import pltrs
    pltrs.use(backend)
    import pltrs.pyplot as plt

    fig, ax = plt.subplots()
    (line,) = ax.plot([0, 1, 2], [0, 1, 2])
    done = []

    def work():
        time.sleep(0.5)
        line.set_ydata([2, 1, 0])
        ax.set_title("from the worker")
        fig.canvas.draw_idle()
        done.append(True)

    threading.Thread(target=work).start()
    if loop == "show":
        timer = fig.canvas.new_timer(1500)
        timer.single_shot = True
        timer.add_callback(plt.close, fig)
        timer.start()
        plt.show()
    else:
        plt.pause(1.5)
        plt.close(fig)
    print(done, [float(y) for y in line.get_ydata()], ax.get_title())


@pytest.mark.parametrize("loop", ["show", "pause"])
@pytest.mark.parametrize("backend", ["gtk3agg", "gtk4agg", "wxagg"])
def test_other_threads_change_a_shown_figure(backend, loop):
    """The toolkit's main and event loops run the calls of other threads,
    as matplotlib's figures take them."""
    reason = _skip_reason(backend)
    if reason:
        pytest.skip(reason)
    assert _run(_change_from_a_worker, backend, loop).strip() == (
        "[True] [2.0, 1.0, 0.0] from the worker")


def test_toolbar_images_are_shipped():
    """Every toolbar image the GUI backends load: ``<name>.svg`` (wx),
    ``<name>-symbolic.svg`` (GTK), ``<name>.png`` and ``<name>_large.png``
    (Tk, Qt), as matplotlib's ``mpl-data/images`` has them."""
    names = [item[2] for item in NavigationToolbar2.toolitems if item[2]]
    assert names == ["home", "back", "forward", "move", "zoom_to_rect",
                     "subplots", "filesave"]
    for name in [*names, "help"]:
        for suffix in [".svg", "-symbolic.svg", ".png", "_large.png"]:
            assert cbook._get_data_path("images", name + suffix).is_file()


def test_cairo_backend_needs_pycairo_or_cairocffi():
    """matplotlib's message when neither binding imports."""
    def body():
        import sys
        sys.modules["cairo"] = sys.modules["cairocffi"] = None
        try:
            import pltrs.backends.backend_cairo  # noqa: F401
        except ImportError as err:
            print(err)

    assert _run(body).strip() == (
        "cairo backend requires that pycairo>=1.14.0 or cairocffi is installed")


@pytest.fixture
def cairo():
    return pytest.importorskip("cairo")


def test_cairo_canvas_saves_every_format(cairo, tmp_path):
    """matplotlib's ``cairo`` canvas: PNG and raw RGBA, and PDF, PostScript
    and SVG documents drawn at 72 dpi; pltrs (Rust) draws them all, the
    raster ones as the Agg canvas does."""
    from pltrs.backends import backend_agg, backend_cairo

    assert backend_cairo._BackendCairo.backend_version == cairo.version
    assert backend_cairo.FigureCanvas is backend_cairo.FigureCanvasCairo
    assert backend_cairo._BackendCairo.FigureManager.__name__ == "FigureManagerBase"
    fig = Figure(figsize=(4, 3), dpi=100)
    fig.add_subplot().plot([1, 2])
    canvas = backend_cairo.FigureCanvasCairo(fig)
    assert canvas.get_default_filetype() == "png"
    assert {"png", "pdf", "ps", "svg", "svgz", "raw", "rgba"} <= set(
        canvas.get_supported_filetypes())
    rgba = io.BytesIO()
    canvas.print_rgba(rgba)
    assert len(rgba.getvalue()) == 400 * 300 * 4
    signatures = {"pdf": b"%PDF-", "ps": b"%!PS-", "svg": b"<?xml",
                  "svgz": b"\x1f\x8b", "png": b"\x89PNG\r"}
    for fmt, signature in signatures.items():
        out = io.BytesIO()
        getattr(canvas, f"print_{fmt}")(out)
        assert out.getvalue().startswith(signature)
        # matplotlib's print methods leave the figure at 72 dpi.
        assert fig.dpi == 72
    fig.dpi = 100
    fig.savefig(tmp_path / "cairo.png", backend="cairo")
    fig.savefig(tmp_path / "agg.png", backend="agg")
    assert (tmp_path / "cairo.png").read_bytes() == (tmp_path / "agg.png").read_bytes()
    assert isinstance(canvas.get_renderer(), backend_agg.RendererAgg)


def test_renderer_cairo_draws_onto_a_context(cairo):
    """matplotlib's ``RendererCairo``: sized by the context's surface, its
    one graphics context setting the context, a figure drawn through it."""
    from pltrs.backends import backend_cairo
    from pltrs.font_manager import FontProperties

    renderer = backend_cairo.RendererCairo(100)
    assert (renderer.width, renderer.height) == (None, None)
    assert renderer.points_to_pixels(72) == 100
    assert renderer.gc.renderer is renderer
    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 400, 300)
    renderer.set_context(cairo.Context(surface))
    assert renderer.get_canvas_width_height() == (400, 300)

    gc = renderer.new_gc()
    gc.set_linewidth(2)
    gc.set_capstyle("round")
    gc.set_joinstyle("bevel")
    gc.set_foreground((1, 0, 0))
    assert gc.get_linewidth() == 2
    assert gc.ctx.get_line_width() == pytest.approx(2 * 100 / 72)
    assert gc.ctx.get_line_cap() == cairo.LINE_CAP_ROUND
    assert gc.ctx.get_line_join() == cairo.LINE_JOIN_BEVEL
    assert gc.get_rgb() == (1, 0, 0)
    gc.restore()
    assert renderer.get_text_width_height_descent(
        "$x^2$", FontProperties(size=12), True) == (19, 16, 0)
    assert backend_cairo._cairo_font_args_from_font_prop(
        FontProperties(family="DejaVu Sans", weight="bold", style="italic")
    ) == ("DejaVu Sans", cairo.FONT_SLANT_ITALIC, cairo.FONT_WEIGHT_BOLD)

    fig = Figure(figsize=(4, 3), dpi=100)
    fig.add_subplot().plot([1, 2])
    fig.draw(renderer)
    pixels = np.frombuffer(surface.get_data(), np.uint8)
    assert pixels.any()
